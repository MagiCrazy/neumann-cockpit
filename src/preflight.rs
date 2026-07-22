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
use crate::api::types::SectorObservation;
use crate::app::ColorMode;
use crate::app::LogEvent;
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

/// Run the preflight sequence, drawing each step in the Probe pane as it
/// completes. Returns once the link is up, or the pilot chooses to continue in
/// degraded mode, or quits.
pub async fn run(terminal: &mut Term, color: ColorMode) -> Result<Outcome> {
    let mut log = BootLog::default();
    let mut events = EventStream::new();

    // ── CONFIG ──────────────────────────────────────────────────────────
    log.begin("CONFIG");
    redraw(terminal, &log, None, None, color)?;
    let mut config = match Config::load_status() {
        ConfigStatus::Ready(c) => {
            log.set(Status::Ok("loaded".into()));
            c
        }
        ConfigStatus::Invalid(msg) => {
            log.set(Status::Warn(format!("invalid: {msg}")));
            match onboard(terminal, &log, &mut events, color).await? {
                Some(c) => {
                    log.set(Status::Ok("configured".into()));
                    c
                }
                None => return Ok(Outcome::Quit),
            }
        }
        ConfigStatus::NeedsKey => match onboard(terminal, &log, &mut events, color).await? {
            Some(c) => {
                log.set(Status::Ok("configured".into()));
                c
            }
            None => return Ok(Outcome::Quit),
        },
    };

    // ── ARCHIVE (local SQLite store) ────────────────────────────────────
    log.begin("ARCHIVE");
    redraw(terminal, &log, None, None, color)?;
    let (conn, scan_history, journal, telemetry) = match store::open(&config::db_path()) {
        Ok(mut conn) => {
            let outcome = store::migrate_legacy_json(&mut conn, &config::history_path())
                .unwrap_or(store::MigrationOutcome::NoLegacyFile);
            let history = store::load_observations(&conn);
            let journal = store::load_events(&conn);
            let telemetry = store::load_telemetry(&conn);
            let msg = match outcome {
                store::MigrationOutcome::Imported(n) => {
                    format!("{} sectors · migrated {n}", history.len())
                }
                _ => format!("{} sectors · {} log", history.len(), journal.len()),
            };
            log.set(Status::Ok(msg));
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
        redraw(terminal, &log, None, None, color)?;
        match timeout(LINK_TIMEOUT, client.get_api_version()).await {
            Ok(Ok(v)) => {
                log.set(Status::Ok(format!("online · v{v}")));
                break (true, Some(v));
            }
            Ok(Err(e)) => log.set(Status::Fail(short_err(&e))),
            Err(_) => log.set(Status::Fail("timeout".into())),
        }
        redraw(
            terminal,
            &log,
            None,
            Some("[R]etry   [K] re-enter key\n[Enter] continue offline"),
            color,
        )?;
        match wait_action(&mut events).await {
            LinkAction::Retry => continue,
            LinkAction::Continue => break (false, None),
            LinkAction::ReenterKey => match onboard(terminal, &log, &mut events, color).await? {
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
) -> Result<Option<Config>> {
    let mut buf = String::new();
    let mut error: Option<String> = None;
    loop {
        redraw(terminal, log, Some(&buf), error.as_deref(), color)?;
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
                match config::write_config(DEFAULT_BASE_URL, &key) {
                    Ok(_) => {
                        if let ConfigStatus::Ready(c) = Config::load_status() {
                            return Ok(Some(c));
                        }
                        // We just wrote a valid key; fall back to a direct build.
                        return Ok(Some(Config {
                            base_url: DEFAULT_BASE_URL.into(),
                            api_key: key,
                            theme: None,
                            hints: true,
                            boot: true,
                        }));
                    }
                    Err(e) => error = Some(format!("write failed: {e}")),
                }
            }
            KeyCode::Char(c) => buf.push(c),
            _ => {}
        }
    }
}

fn redraw(terminal: &mut Term, log: &BootLog, entry: Option<&str>, note: Option<&str>, color: ColorMode) -> Result<()> {
    terminal.draw(|f| crate::ui::preflight::render(f, f.area(), &log.steps, entry, note, color))?;
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
