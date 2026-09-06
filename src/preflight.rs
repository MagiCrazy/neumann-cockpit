//! Boot preflight: runs the real startup checks — config presence, local
//! archive (SQLite migration), and the remote API link — inside the boot grid's
//! centre Probe pane, and onboards a first-run API key without ever crashing to
//! a console.
//!
//! The eight surrounding subsystems stay dark until the link comes up (the
//! cosmetic boot animation in `run()` lights them centre-out afterwards). This
//! runs before the cockpit event loop and hands `run()` a ready set of
//! resources. The Windows first-run failure it fixes: `Config::load()` used to
//! error out before the terminal was set up, so a double-clicked binary flashed
//! a console and vanished. Now the screen is up first and every failure has an
//! in-TUI outcome.

use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use rusqlite::Connection;
use tokio::time::timeout;

use crate::api::client::ApiClient;
use crate::api::ratelimit::RateLimited;
use crate::api::types::SectorObservation;
use crate::app::LogEvent;
use crate::app::{ColorMode, Polarity};
use crate::config::{self, Config, ConfigStatus, DEFAULT_BASE_URL};
use crate::store;

/// How long to wait on the remote link check before declaring it down.
const LINK_TIMEOUT: Duration = Duration::from_secs(8);

/// Status of one preflight check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Pending,
    Ok(String),
    Warn(String),
    Fail(String),
}

/// One line of the boot check-list.
#[derive(Debug, Clone)]
pub struct Step {
    pub label: &'static str,
    pub status: Status,
}

/// The ordered check-list, with the last entry being the step currently running.
#[derive(Debug, Default)]
pub struct BootLog {
    pub steps: Vec<Step>,
}

impl BootLog {
    /// Append a new step in the `Pending` state and make it current.
    fn begin(&mut self, label: &'static str) {
        self.steps.push(Step {
            label,
            status: Status::Pending,
        });
    }
    /// Set the status of the current (last) step.
    fn set(&mut self, status: Status) {
        if let Some(last) = self.steps.last_mut() {
            last.status = status;
        }
    }
}

/// Everything the cockpit needs, produced by a successful preflight.
pub struct Ready {
    pub config: Config,
    pub client: ApiClient,
    pub conn: Option<Connection>,
    pub scan_history: Vec<SectorObservation>,
    pub journal: Vec<LogEvent>,
    pub telemetry: Vec<crate::app::TelemetrySample>,
    pub api_version: Option<u32>,
    pub link_ok: bool,
    /// Terminal ground resolved at boot: detected via OSC 11, or forced by the
    /// config's `polarity` key (issue #233).
    pub polarity: Polarity,
}

/// The result of the preflight: either resources to run, or a clean quit
/// (the pilot pressed Esc/Ctrl-C at the onboarding prompt).
pub enum Outcome {
    Ready(Box<Ready>),
    Quit,
}

/// What the pilot chose at the "remote link down" prompt.
enum LinkAction {
    Retry,
    ReenterKey,
    Continue,
}

type Term = Terminal<CrosstermBackend<io::Stdout>>;

// ── Step decisions ────────────────────────────────────────────────────────
//
// The preflight is an async loop around a terminal, which is why none of it
// was testable (issue #348). These are the *decisions* it makes, lifted out of
// the drawing: each is a pure function of what the step returned, so the shell
// below stays a shell and the reasoning can be asserted without a terminal, a
// network or a real config directory.

/// What the CONFIG step should do with the file it found.
#[derive(Debug)]
pub enum ConfigStep {
    /// A usable config: carry on.
    Use(Box<Config>),
    /// Ask the pilot for a key, logging `reason` first.
    Onboard { reason: Status },
}

/// Decide the CONFIG step. A file that exists but has no usable key is not an
/// error — it is the first-run path, and the reason it took it is what the boot
/// log shows.
pub fn config_step(status: ConfigStatus) -> ConfigStep {
    match status {
        ConfigStatus::Ready(c) => ConfigStep::Use(Box::new(c)),
        ConfigStatus::NeedsKey => ConfigStep::Onboard {
            reason: Status::Pending,
        },
        ConfigStatus::Invalid(msg) => ConfigStep::Onboard {
            reason: Status::Warn(format!("invalid: {msg}")),
        },
    }
}

/// The terminal's answer is only the default: a `polarity` key decides for it
/// in both directions (issue #233).
pub fn resolve_polarity(detected: Polarity, pref: crate::app::PolarityPref) -> Polarity {
    match pref {
        crate::app::PolarityPref::Auto => detected,
        crate::app::PolarityPref::Forced(forced) => forced,
    }
}

/// The ARCHIVE step's line: a one-time legacy import is worth saying out loud,
/// the steady state is a pair of counts.
pub fn archive_line(outcome: store::MigrationOutcome, sectors: usize, journal: usize) -> String {
    match outcome {
        store::MigrationOutcome::Imported(n) => format!("{sectors} sectors · migrated {n}"),
        _ => format!("{sectors} sectors · {journal} log"),
    }
}

/// What the REMOTE LINK probe found.
#[derive(Debug, PartialEq)]
pub enum LinkOutcome {
    Online(u32),
    /// The server answered, unhappily. `throttled` separates a spent quota from
    /// every other failure, because the two need different advice.
    Failed {
        message: String,
        throttled: bool,
    },
    /// No answer inside `LINK_TIMEOUT`.
    TimedOut,
}

impl LinkOutcome {
    /// Classify the probe's result. A 429 is a **healthy link with a spent
    /// quota**, not a bad key (API v104) — telling them apart here is what
    /// keeps the prompt below from sending a pilot to re-enter a good key.
    pub fn classify(result: std::result::Result<Result<u32>, tokio::time::error::Elapsed>) -> Self {
        match result {
            Ok(Ok(v)) => LinkOutcome::Online(v),
            Ok(Err(e)) => LinkOutcome::Failed {
                throttled: e.downcast_ref::<RateLimited>().is_some(),
                message: short_err(&e),
            },
            Err(_) => LinkOutcome::TimedOut,
        }
    }

    /// The boot-log line for this outcome.
    pub fn status(&self) -> Status {
        match self {
            LinkOutcome::Online(v) => Status::Ok(format!("online · v{v}")),
            LinkOutcome::Failed { message, .. } => Status::Fail(message.clone()),
            LinkOutcome::TimedOut => Status::Fail("timeout".into()),
        }
    }
}

/// The actions offered under a failed link. Offering "re-enter key" to a pilot
/// whose key is fine and whose quota is spent is the wrong advice, so the
/// throttled wording drops it.
pub fn link_prompt(throttled: bool) -> &'static str {
    if throttled {
        "rate limited — the key is fine\n[R]etry after the delay   [Enter] continue offline"
    } else {
        "[R]etry   [K] re-enter key\n[Enter] continue offline"
    }
}

/// Commit a freshly-entered API key: write it, then read the file back.
///
/// The re-read is what proves the file is usable; the fallback exists because a
/// key we just wrote successfully must not be lost to a subsequent read failure
/// — the pilot typed it, the cockpit should fly.
pub fn commit_key_at(path: &std::path::Path, base_url: &str, key: &str) -> Result<Config> {
    config::write_config_at(path, base_url, key)?;
    if let ConfigStatus::Ready(c) = config::load_status_at(path) {
        return Ok(c);
    }
    Ok(Config {
        base_url: base_url.to_string(),
        api_key: key.to_string(),
        theme: None,
        polarity: None,
        log: None,
        hints: true,
        boot: true,
        notifications: true,
    })
}

/// Run the preflight sequence, drawing each step in the Probe pane as it
/// completes. Returns once the link is up, or the pilot chooses to continue in
/// degraded mode, or quits.
pub async fn run(terminal: &mut Term, color: ColorMode, polarity: Polarity) -> Result<Outcome> {
    // The detected ground is the default; a config that forces one wins, but
    // the file has not been read yet — so the first frames use the detection
    // and the CONFIG step below corrects it if the pilot decided.
    let mut polarity = polarity;
    let mut log = BootLog::default();
    let mut events = EventStream::new();

    // ── CONFIG ──────────────────────────────────────────────────────────
    log.begin("CONFIG");
    redraw(terminal, &log, None, None, color, polarity)?;
    let mut config = match config_step(Config::load_status()) {
        ConfigStep::Use(c) => {
            log.set(Status::Ok("loaded".into()));
            *c
        }
        ConfigStep::Onboard { reason } => {
            if !matches!(reason, Status::Pending) {
                log.set(reason);
            }
            match onboard(terminal, &log, &mut events, color, polarity).await? {
                Some(c) => {
                    log.set(Status::Ok("configured".into()));
                    c
                }
                None => return Ok(Outcome::Quit),
            }
        }
    };

    polarity = resolve_polarity(polarity, config.polarity_pref());

    // ── ARCHIVE (local SQLite store) ────────────────────────────────────
    log.begin("ARCHIVE");
    redraw(terminal, &log, None, None, color, polarity)?;
    let (conn, scan_history, journal, telemetry) = match store::open(&config::db_path()) {
        Ok(mut conn) => {
            let outcome = store::migrate_legacy_json(&mut conn, &config::history_path())
                .unwrap_or(store::MigrationOutcome::NoLegacyFile);
            let history = store::load_observations(&conn);
            let journal = store::load_events(&conn);
            let telemetry = store::load_telemetry(&conn);
            log.set(Status::Ok(archive_line(outcome, history.len(), journal.len())));
            (Some(conn), history, journal, telemetry)
        }
        Err(e) => {
            log.set(Status::Warn(format!("disabled: {e}")));
            (None, Vec::new(), Vec::new(), Vec::new())
        }
    };

    // ── REMOTE LINK ─────────────────────────────────────────────────────
    // Retried interactively: a bad key or an outage is shown in the Probe pane
    // with actions (retry / re-enter key / continue offline).
    log.begin("REMOTE LINK");
    let mut client = ApiClient::new(config.base_url.clone(), config.api_key.clone())?;
    let (link_ok, api_version) = loop {
        log.set(Status::Pending);
        redraw(terminal, &log, None, None, color, polarity)?;
        let outcome = LinkOutcome::classify(timeout(LINK_TIMEOUT, client.get_api_version()).await);
        log.set(outcome.status());
        if let LinkOutcome::Online(v) = outcome {
            break (true, Some(v));
        }
        let throttled = matches!(outcome, LinkOutcome::Failed { throttled: true, .. });
        redraw(terminal, &log, None, Some(link_prompt(throttled)), color, polarity)?;
        match wait_action(&mut events).await {
            LinkAction::Retry => continue,
            LinkAction::Continue => break (false, None),
            LinkAction::ReenterKey => match onboard(terminal, &log, &mut events, color, polarity).await? {
                Some(c) => {
                    config = c;
                    client = ApiClient::new(config.base_url.clone(), config.api_key.clone())?;
                }
                None => return Ok(Outcome::Quit),
            },
        }
    };

    Ok(Outcome::Ready(Box::new(Ready {
        config,
        client,
        conn,
        scan_history,
        journal,
        telemetry,
        api_version,
        link_ok,
        polarity,
    })))
}

/// Collect an API key from the pilot and write `config.toml`. Renders the
/// current check-list plus the onboarding prompt in the Probe pane; does not
/// touch step statuses (the caller owns those). Returns the ready `Config`, or
/// `None` if they pressed Esc/Ctrl-C to quit.
async fn onboard(
    terminal: &mut Term,
    log: &BootLog,
    events: &mut EventStream,
    color: ColorMode,
    polarity: Polarity,
) -> Result<Option<Config>> {
    let mut buf = String::new();
    let mut error: Option<String> = None;
    loop {
        redraw(terminal, log, Some(&buf), error.as_deref(), color, polarity)?;
        let Some(ev) = events.next().await else { return Ok(None) };
        let Ok(Event::Key(k)) = ev else { continue };
        if k.kind != KeyEventKind::Press {
            continue;
        }
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match k.code {
            KeyCode::Esc => return Ok(None),
            KeyCode::Char('c') if ctrl => return Ok(None),
            KeyCode::Backspace => {
                buf.pop();
            }
            KeyCode::Enter => {
                let key = buf.trim().to_string();
                if key.is_empty() {
                    error = Some("key can't be empty".into());
                    continue;
                }
                match commit_key_at(&config::config_path(), DEFAULT_BASE_URL, &key) {
                    Ok(c) => return Ok(Some(c)),
                    Err(e) => error = Some(format!("write failed: {e}")),
                }
            }
            KeyCode::Char(c) => buf.push(c),
            _ => {}
        }
    }
}

fn redraw(
    terminal: &mut Term,
    log: &BootLog,
    entry: Option<&str>,
    note: Option<&str>,
    color: ColorMode,
    polarity: Polarity,
) -> Result<()> {
    terminal.draw(|f| crate::ui::preflight::render(f, f.area(), &log.steps, entry, note, color, polarity))?;
    Ok(())
}

/// Wait for the pilot's choice at the "remote link down" prompt.
async fn wait_action(events: &mut EventStream) -> LinkAction {
    loop {
        match events.next().await {
            Some(Ok(Event::Key(k))) if k.kind == KeyEventKind::Press => match k.code {
                KeyCode::Char('r') | KeyCode::Char('R') => return LinkAction::Retry,
                KeyCode::Char('k') | KeyCode::Char('K') => return LinkAction::ReenterKey,
                KeyCode::Enter | KeyCode::Esc => return LinkAction::Continue,
                _ => {}
            },
            Some(_) => {}
            None => return LinkAction::Continue,
        }
    }
}

/// The first line of an error, for a compact status column.
fn short_err(e: &anyhow::Error) -> String {
    e.to_string().lines().next().unwrap_or("error").to_string()
}

#[cfg(test)]
mod tests {
    //! The boot path's *decisions* (issue #348). None of these needs a
    //! terminal, a network or a real config directory — which is the whole
    //! point: this is the code a pilot meets before anything is on screen, and
    //! it exists because a Windows first run used to fail before the terminal
    //! did.
    use super::*;
    use crate::app::{Polarity, PolarityPref};

    fn tmp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("nc_preflight_{name}.toml"))
    }

    // ── CONFIG ────────────────────────────────────────────────────────────

    #[test]
    fn a_usable_config_is_used_as_is() {
        let path = tmp("ready");
        let _ = std::fs::remove_file(&path);
        config::write_config_at(&path, "https://example.test", "vng_realkey").unwrap();
        match config_step(config::load_status_at(&path)) {
            ConfigStep::Use(c) => {
                assert_eq!(c.api_key, "vng_realkey");
                assert_eq!(c.base_url, "https://example.test");
            }
            other => panic!("expected Use, got {other:?}"),
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_missing_key_is_the_first_run_path_not_an_error() {
        // The file may not exist at all, or exist with the placeholder key
        // from config.example.toml. Both mean "ask", and neither is a failure
        // worth colouring the boot log with.
        match config_step(config::load_status_at(&tmp("absent-nothing-here"))) {
            ConfigStep::Onboard { reason } => assert!(
                matches!(reason, Status::Pending),
                "a first run is not a warning: {reason:?}"
            ),
            other => panic!("expected Onboard, got {other:?}"),
        }
    }

    #[test]
    fn a_malformed_config_says_why_before_asking() {
        let path = tmp("malformed");
        std::fs::write(&path, "this is not toml = = =").unwrap();
        match config_step(config::load_status_at(&path)) {
            ConfigStep::Onboard { reason } => match reason {
                Status::Warn(msg) => assert!(msg.starts_with("invalid:"), "{msg}"),
                other => panic!("a broken file has to be explained, not swallowed: {other:?}"),
            },
            other => panic!("expected Onboard, got {other:?}"),
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_committed_key_is_read_back_from_the_file() {
        let path = tmp("commit");
        let _ = std::fs::remove_file(&path);
        let config = commit_key_at(&path, DEFAULT_BASE_URL, "vng_typed_by_the_pilot").unwrap();
        assert_eq!(config.api_key, "vng_typed_by_the_pilot");
        assert_eq!(config.base_url, DEFAULT_BASE_URL);
        // And it really is on disk, not just in the returned struct.
        assert!(matches!(config::load_status_at(&path), ConfigStatus::Ready(_)));
        let _ = std::fs::remove_file(&path);
    }

    // ── Polarity (#233) ───────────────────────────────────────────────────

    #[test]
    fn a_forced_polarity_outranks_the_terminals_answer() {
        // Both directions: the pilot may be correcting a terminal that
        // answered wrong, or one that never answered at all.
        assert_eq!(resolve_polarity(Polarity::Dark, PolarityPref::Auto), Polarity::Dark);
        assert_eq!(resolve_polarity(Polarity::Light, PolarityPref::Auto), Polarity::Light);
        assert_eq!(
            resolve_polarity(Polarity::Dark, PolarityPref::Forced(Polarity::Light)),
            Polarity::Light
        );
        assert_eq!(
            resolve_polarity(Polarity::Light, PolarityPref::Forced(Polarity::Dark)),
            Polarity::Dark
        );
    }

    // ── ARCHIVE ───────────────────────────────────────────────────────────

    #[test]
    fn a_one_time_import_is_reported_and_the_steady_state_is_counts() {
        assert_eq!(
            archive_line(store::MigrationOutcome::Imported(42), 100, 7),
            "100 sectors · migrated 42",
            "a legacy import happens once and is worth saying"
        );
        for quiet in [
            store::MigrationOutcome::AlreadyMigrated,
            store::MigrationOutcome::NoLegacyFile,
        ] {
            assert_eq!(archive_line(quiet, 100, 7), "100 sectors · 7 log");
        }
    }

    // ── REMOTE LINK ───────────────────────────────────────────────────────

    #[test]
    fn a_healthy_link_reports_its_api_version() {
        let outcome = LinkOutcome::classify(Ok(Ok(116)));
        assert_eq!(outcome, LinkOutcome::Online(116));
        assert_eq!(outcome.status(), Status::Ok("online · v116".into()));
    }

    #[test]
    fn a_spent_quota_is_told_apart_from_a_bad_key() {
        // The distinction that matters: a 429 is a healthy link whose quota is
        // gone, and offering "re-enter key" would send the pilot to change a
        // key that is perfectly fine (API v104).
        let throttled = LinkOutcome::classify(Ok(Err(anyhow::Error::new(RateLimited {
            retry_after_secs: Some(30),
        }))));
        assert!(matches!(throttled, LinkOutcome::Failed { throttled: true, .. }));
        assert!(link_prompt(true).contains("the key is fine"));
        assert!(
            !link_prompt(true).contains("re-enter key"),
            "wrong advice: {}",
            link_prompt(true)
        );

        let unauthorized = LinkOutcome::classify(Ok(Err(anyhow::anyhow!(
            "Unauthorized — check your api_key in config.toml"
        ))));
        assert!(matches!(unauthorized, LinkOutcome::Failed { throttled: false, .. }));
        assert!(link_prompt(false).contains("re-enter key"));
    }

    #[test]
    fn a_failure_keeps_only_its_first_line() {
        // anyhow chains contexts across lines; the boot grid has one row.
        let outcome = LinkOutcome::classify(Ok(Err(anyhow::anyhow!("first line\nsecond line"))));
        assert_eq!(
            outcome.status(),
            Status::Fail("first line".into()),
            "the prompt has one row to say it in"
        );
    }

    #[test]
    fn every_failure_offers_a_way_to_continue_offline() {
        // Degraded mode is the whole reason the preflight exists: no failure
        // may leave the pilot with nothing to press.
        for prompt in [link_prompt(true), link_prompt(false)] {
            assert!(prompt.contains("continue offline"), "{prompt}");
            assert!(prompt.contains("[R]etry"), "{prompt}");
        }
    }
}
