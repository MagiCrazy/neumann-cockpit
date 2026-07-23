//! Headless script runner (#198 extension) — play an action script from a file
//! against the live API with no TUI, streaming the ship's-log to stdout.
//!
//! Reuses the interactive engine wholesale: `parse_script_line` for the file
//! lines, `advance_script` for the single-lane sequential executor (including
//! `mine … by all` fork/join), and the same `fetch_*` spawners the cockpit's
//! event loop drains. Completion is polled exactly as in the TUI (Manny
//! busy→idle, probe movement). Ship's-log entries are printed AND persisted to
//! the SQLite `events` table, so a headless run shows up in the next TUI session.

use std::path::Path;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use chrono::Local;
use tokio::sync::mpsc;
use tokio::time::{interval, MissedTickBehavior};

use crate::api::client::ApiClient;
use crate::api::tasks::{
    fetch_all, fetch_atomic_printer_craft, fetch_craft, fetch_crafting_recipes, fetch_detach, fetch_mine, fetch_move,
    fetch_recover, fetch_repair, fetch_salvage,
};
use crate::app::{ApiMessage, AppState, Fabricator, LogEvent, ScriptAction, StepState};
use crate::config::{self, Config, ConfigStatus};
use crate::store;

/// The `--script <file>` / `-s <file>` argument, if present. Returns the path.
pub fn script_arg(args: &[String]) -> Option<String> {
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        if let Some(p) = a.strip_prefix("--script=") {
            return Some(p.to_string());
        }
        if a == "--script" || a == "-s" {
            return it.next().cloned();
        }
    }
    None
}

/// Default number of measurement rounds when `--diagnostic` is given no count.
const DEFAULT_DIAGNOSTIC_ROUNDS: u32 = 3;
/// p95 above this (ms) flags an endpoint as slow in the report.
const SLOW_THRESHOLD_MS: f64 = 1000.0;

/// The diagnostic mode, if requested: `--diagnostic` / `--diagnostic=N` /
/// `--status latency` (`--status=latency`). Returns the number of rounds to run
/// (defaulting when unspecified or unparseable), or `None` for other launches.
pub fn diagnostic_arg(args: &[String]) -> Option<u32> {
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        if let Some(n) = a.strip_prefix("--diagnostic=") {
            return Some(n.parse().unwrap_or(DEFAULT_DIAGNOSTIC_ROUNDS).max(1));
        }
        if a == "--diagnostic" {
            return Some(DEFAULT_DIAGNOSTIC_ROUNDS);
        }
        // `--status latency` (the #229 headless-status alias) or `--status=latency`.
        if a == "--status=latency" {
            return Some(DEFAULT_DIAGNOSTIC_ROUNDS);
        }
        if a == "--status" && it.next().map(|v| v == "latency").unwrap_or(false) {
            return Some(DEFAULT_DIAGNOSTIC_ROUNDS);
        }
    }
    None
}

/// Non-comment, non-blank lines of a script file (one command per line; `#`
/// starts a comment).
fn script_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect()
}

/// Flush stdout so a line reaches a redirected pipe/file immediately. Rust's
/// stdout is line-buffered on a tty but block-buffered otherwise, so without
/// this a long run's progress wouldn't appear until the buffer fills or the
/// process exits — defeating the "streams as it progresses" contract.
fn flush_stdout() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

/// `HH:MM:SS  {msg}` on stdout — the ship's-log line format.
fn log_line(msg: &str) {
    println!("{}  {msg}", Local::now().format("%H:%M:%S"));
    flush_stdout();
}

/// A staged ship's-log entry, stamped with its own occurrence time.
fn print_event(ev: &LogEvent) {
    println!(
        "{}  » {}",
        ev.occurred_at.with_timezone(&Local).format("%H:%M:%S"),
        ev.summary
    );
    flush_stdout();
}

/// Run a script file headlessly. Returns the process exit code: 0 on
/// completion, 1 if the script halted on an error.
pub async fn run(path: &Path) -> Result<i32> {
    let text = std::fs::read_to_string(path).with_context(|| format!("cannot read script {}", path.display()))?;
    let lines = script_lines(&text);
    if lines.is_empty() {
        bail!("script {} has no commands", path.display());
    }

    let config = match Config::load_status() {
        ConfigStatus::Ready(c) => c,
        _ => bail!("no valid config — launch the cockpit once to set your API key"),
    };
    let client = ApiClient::new(config.base_url.clone(), config.api_key.clone())?;

    // Persist the ship's log to the same DB the TUI uses.
    let (conn, journal) = match store::open(&config::db_path()) {
        Ok(mut conn) => {
            let _ = store::migrate_legacy_json(&mut conn, &config::history_path());
            let journal = store::load_events(&conn);
            (Some(conn), journal)
        }
        Err(_) => (None, Vec::new()),
    };
    // Headless does not surface the degraded flag (no status bar); the writer
    // still survives failing writes. Keep only the sender.
    let persist_tx = conn.map(|c| store::spawn_writer(c).0);

    let (tx, mut rx) = mpsc::channel::<ApiMessage>(32);
    let mut state = AppState {
        journal,
        ..Default::default()
    };

    // Prime live state: the first step's targets resolve against it (late
    // binding), and completion polling needs a fresh roster.
    eprintln!("· linking to {} …", config.base_url);
    fetch_all(client.clone(), tx.clone());
    // The crafting catalog is a separate fetch (not part of fetch_all); a `craft`
    // step resolves against it, so it must be primed before such a step fires.
    fetch_crafting_recipes(client.clone(), tx.clone());
    if !wait_until_primed(&mut state, &mut rx, Duration::from_secs(15)).await {
        bail!("remote link failed — no probe data within 15s");
    }

    // Parse the file into steps; a syntactic error aborts before anything runs.
    for (i, line) in lines.iter().enumerate() {
        state
            .enqueue_script_line(line)
            .map_err(|e| anyhow::anyhow!("line {}: \"{line}\" — {e}", i + 1))?;
    }
    let total = state.script.len();
    // A craft step needs the recipe catalog; wait for it (bounded) so the step
    // doesn't fire against an empty catalog and fail spuriously.
    if state.script.iter().any(|s| s.cmd.verb == crate::app::ScriptVerb::Craft) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        while state.recipes.is_empty() {
            match tokio::time::timeout_at(deadline, rx.recv()).await {
                Ok(Some(msg)) => dispatch(&mut state, msg),
                _ => bail!("crafting recipes did not load within 15s"),
            }
        }
    }
    log_line(&format!("script loaded — {total} step(s)"));
    state.script_run();

    let mut ui_tick = interval(Duration::from_secs(1));
    ui_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    // Poll cadence for completion + late binding, matching the TUI's brisk poll.
    let mut poll = interval(Duration::from_secs(3));
    poll.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut reported = vec![false; total];

    loop {
        state.advance_script();
        for action in state.script_fire.drain(..) {
            spawn(action, &client, &tx);
        }
        for ev in state.pending_journal.drain(..) {
            print_event(&ev);
            if let Some(ptx) = &persist_tx {
                let _ = ptx.send(store::PersistMsg::AppendEvent(ev.clone()));
            }
            state.journal.insert(0, ev);
        }
        report_completions(&state, total, &mut reported);

        if !state.script_active() {
            break;
        }

        tokio::select! {
            _ = ui_tick.tick() => {}
            _ = poll.tick() => fetch_all(client.clone(), tx.clone()),
            Some(msg) = rx.recv() => dispatch(&mut state, msg),
        }
    }

    let (done, _) = state.script_progress();
    match state.script.iter().find_map(|s| match &s.state {
        StepState::Failed(e) => Some(e.clone()),
        _ => None,
    }) {
        Some(e) => {
            log_line(&format!("✗ script halted at {done}/{total} — {e}"));
            Ok(1)
        }
        None => {
            log_line(&format!("✓ script complete — {done}/{total}"));
            Ok(0)
        }
    }
}

/// Print a `✓`/`✗` marker for each step that has newly reached a terminal state.
fn report_completions(state: &AppState, total: usize, reported: &mut [bool]) {
    for (i, step) in state.script.iter().enumerate() {
        if i >= reported.len() || reported[i] {
            continue;
        }
        match &step.state {
            StepState::Done => {
                reported[i] = true;
                log_line(&format!("  ✓ step {}/{total} done", i + 1));
            }
            StepState::Failed(e) => {
                reported[i] = true;
                log_line(&format!("  ✗ step {}/{total} failed — {e}", i + 1));
            }
            _ => {}
        }
    }
}

/// Pump messages until the probe + mannies rosters are loaded (so the first
/// step can resolve), or the timeout elapses.
async fn wait_until_primed(state: &mut AppState, rx: &mut mpsc::Receiver<ApiMessage>, budget: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + budget;
    while state.probe.is_none() || state.mannies.is_none() {
        match tokio::time::timeout_at(deadline, rx.recv()).await {
            Ok(Some(msg)) => dispatch(state, msg),
            _ => return false,
        }
    }
    true
}

/// Spawn the `fetch_*` task for a resolved script action (mirrors the event
/// loop's `script_fire` drain).
fn spawn(action: ScriptAction, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    match action {
        ScriptAction::Travel { x, y, z } => fetch_move(x, y, z, client.clone(), tx.clone()),
        ScriptAction::Mine {
            manny_id,
            object_id,
            resources,
            amount,
            container_id,
        } => fetch_mine(
            manny_id,
            object_id,
            resources,
            amount,
            container_id,
            client.clone(),
            tx.clone(),
        ),
        ScriptAction::Repair {
            manny_id,
            integrity_percent,
        } => fetch_repair(manny_id, integrity_percent, client.clone(), tx.clone()),
        ScriptAction::Salvage { manny_id, object_id } => fetch_salvage(manny_id, object_id, client.clone(), tx.clone()),
        ScriptAction::Detach {
            manny_id,
            container_id,
            mode,
            object_id,
        } => fetch_detach(manny_id, container_id, mode, object_id, client.clone(), tx.clone()),
        ScriptAction::Recover { manny_id, object_id } => fetch_recover(manny_id, object_id, client.clone(), tx.clone()),
        ScriptAction::Craft {
            fabricator,
            manny_id,
            recipe_id,
        } => match fabricator {
            Fabricator::Manny => {
                if let Some(builder) = manny_id {
                    fetch_craft(builder, recipe_id, client.clone(), tx.clone());
                }
            }
            Fabricator::AtomicPrinter => fetch_atomic_printer_craft(recipe_id, client.clone(), tx.clone()),
        },
    }
}

/// Minimal message dispatch: refresh the state the executor polls, and route the
/// MVP verbs' errors to `script_note_error` so an API failure halts the script.
/// Everything else (success acks, unrelated fetches) is ignored — the executor
/// works off polled state, not the `*Started` messages.
fn dispatch(state: &mut AppState, msg: ApiMessage) {
    match msg {
        ApiMessage::ProbeUpdated(probe) => state.update_probe(probe),
        ApiMessage::ManniesUpdated(mannies) => state.update_mannies(mannies),
        ApiMessage::SectorUpdated(sector) => state.update_sector(sector),
        ApiMessage::RecipesFetched(recipes) => state.recipes = recipes,
        ApiMessage::MoveError(e)
        | ApiMessage::MineError(e)
        | ApiMessage::RepairError(e)
        | ApiMessage::SalvageError(e)
        | ApiMessage::DetachError(e)
        | ApiMessage::RecoverError(e)
        | ApiMessage::CraftError(e)
        | ApiMessage::AtomicPrinterCraftError(e) => state.script_note_error(&e),
        _ => {}
    }
}

/// Run the API diagnostic: fire `rounds` bursts of the read-only endpoints
/// against the live server, then print a per-endpoint latency/health report to
/// stdout (#247). Read-only: no mutating calls, so it is safe to run anytime.
/// Exit code 0 unless every request failed (or config/link is unusable → 1).
pub async fn run_diagnostic(rounds: u32) -> Result<i32> {
    let config = match Config::load_status() {
        ConfigStatus::Ready(c) => c,
        _ => bail!("no valid config — launch the cockpit once to set your API key"),
    };
    let client = ApiClient::new(config.base_url.clone(), config.api_key.clone())?;

    log_line(&format!("API diagnostic — {} · {rounds} round(s)", config.base_url));

    for round in 1..=rounds {
        probe_endpoints(&client).await;
        log_line(&format!("round {round}/{rounds} complete"));
    }

    let metrics = client.metrics();
    let metrics = metrics.lock().expect("metrics lock");
    print_diagnostic_report(&metrics, &config.base_url, rounds);

    // Success unless nothing got through at all.
    let all_failed = !metrics.is_empty() && metrics.total_errors() == metrics.len();
    Ok(if metrics.is_empty() || all_failed { 1 } else { 0 })
}

/// Fire one round of every read-only endpoint. Errors are swallowed here — each
/// call is already recorded in the metrics ring (status / timeout / latency),
/// which is what the report reads.
async fn probe_endpoints(c: &ApiClient) {
    let _ = c.get_api_version().await;
    let _ = c.get_probes().await;
    let _ = c.get_probe().await;
    let _ = c.get_mannies().await;
    let _ = c.get_probe_sector().await;
    let _ = c.get_alerts().await;
    let _ = c.get_damage_warnings().await;
    let _ = c.get_missions().await;
    let _ = c.get_probe_improvements(false).await;
    let _ = c.get_visited_sectors().await;
    let _ = c.get_crafting_recipes().await;
}

/// Print the aggregated per-endpoint report as a fixed-width table, copy-pasteable
/// into a bug report. Slowest endpoint first; a `⚠` flags a slow p95.
fn print_diagnostic_report(metrics: &crate::api::metrics::Metrics, base_url: &str, rounds: u32) {
    let agg = metrics.aggregate();
    println!();
    println!("═══ API DIAGNOSTIC REPORT ═══");
    println!("base_url : {base_url}");
    println!(
        "rounds   : {rounds}   requests: {}   errors: {}   timeouts: {}",
        metrics.len(),
        metrics.total_errors(),
        metrics.total_timeouts()
    );
    println!("slow flag: p95 > {} ms", SLOW_THRESHOLD_MS as i64);
    println!();
    println!(
        "{:<46} {:>4} {:>7} {:>7} {:>7} {:>4} {:>4}",
        "ENDPOINT", "n", "p50", "p95", "max", "err", "t/o"
    );
    println!("{}", "─".repeat(82));
    for s in &agg {
        let slow = if s.p95_ms > SLOW_THRESHOLD_MS { " ⚠" } else { "" };
        println!(
            "{:<46} {:>4} {:>7.0} {:>7.0} {:>7.0} {:>4} {:>4}{slow}",
            truncate(&s.label, 46),
            s.count,
            s.p50_ms,
            s.p95_ms,
            s.max_ms,
            s.errors,
            s.timeouts,
        );
    }
    if let Some(slowest) = agg.first() {
        println!();
        println!("slowest: {} (p95 {:.0} ms)", slowest.label, slowest.p95_ms);
    }
    flush_stdout();
}

/// Truncate a label to `width` columns with an ellipsis, so a long endpoint
/// path never breaks the table alignment.
fn truncate(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        s.to_string()
    } else {
        let keep = width.saturating_sub(1);
        format!("{}…", s.chars().take(keep).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_arg_recognises_flag_forms() {
        let a = |v: &[&str]| script_arg(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(a(&["nc", "--script", "run.ncs"]).as_deref(), Some("run.ncs"));
        assert_eq!(a(&["nc", "-s", "run.ncs"]).as_deref(), Some("run.ncs"));
        assert_eq!(a(&["nc", "--script=run.ncs"]).as_deref(), Some("run.ncs"));
        assert_eq!(a(&["nc"]), None, "bare launch stays interactive");
        assert_eq!(a(&["nc", "--script"]), None, "flag with no path");
    }

    #[test]
    fn diagnostic_arg_recognises_flag_forms() {
        let d = |v: &[&str]| diagnostic_arg(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(d(&["nc", "--diagnostic"]), Some(DEFAULT_DIAGNOSTIC_ROUNDS));
        assert_eq!(d(&["nc", "--diagnostic=5"]), Some(5));
        assert_eq!(d(&["nc", "--diagnostic=0"]), Some(1), "clamped to at least one round");
        assert_eq!(
            d(&["nc", "--diagnostic=abc"]),
            Some(DEFAULT_DIAGNOSTIC_ROUNDS),
            "unparseable falls back"
        );
        assert_eq!(d(&["nc", "--status", "latency"]), Some(DEFAULT_DIAGNOSTIC_ROUNDS));
        assert_eq!(d(&["nc", "--status=latency"]), Some(DEFAULT_DIAGNOSTIC_ROUNDS));
        assert_eq!(d(&["nc"]), None, "bare launch is not diagnostic");
        assert_eq!(d(&["nc", "--status"]), None, "status without latency arg");
    }

    #[test]
    fn truncate_keeps_short_labels_and_ellipsises_long_ones() {
        assert_eq!(truncate("GET /api/probe", 46), "GET /api/probe");
        let long = "GET /api/probe/:id/mannies/:id/some-very-long-endpoint-name";
        let t = truncate(long, 20);
        assert_eq!(t.chars().count(), 20);
        assert!(t.ends_with('…'));
    }

    #[test]
    fn script_lines_strips_comments_and_blanks() {
        let text = "# header\n\ntravel 2 0 0\n  mine metals 500  \n# trailing\nrecover box\n";
        assert_eq!(
            script_lines(text),
            vec!["travel 2 0 0", "mine metals 500", "recover box"]
        );
    }
}
