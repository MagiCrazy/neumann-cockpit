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
use crate::api::tasks::{fetch_all, fetch_detach, fetch_mine, fetch_move, fetch_recover, fetch_repair, fetch_salvage};
use crate::app::{ApiMessage, AppState, LogEvent, ScriptAction, StepState};
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

/// Non-comment, non-blank lines of a script file (one command per line; `#`
/// starts a comment).
fn script_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect()
}

/// `HH:MM:SS  {msg}` on stdout — the ship's-log line format.
fn log_line(msg: &str) {
    println!("{}  {msg}", Local::now().format("%H:%M:%S"));
}

/// A staged ship's-log entry, stamped with its own occurrence time.
fn print_event(ev: &LogEvent) {
    println!(
        "{}  » {}",
        ev.occurred_at.with_timezone(&Local).format("%H:%M:%S"),
        ev.summary
    );
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
        ApiMessage::MoveError(e)
        | ApiMessage::MineError(e)
        | ApiMessage::RepairError(e)
        | ApiMessage::SalvageError(e)
        | ApiMessage::DetachError(e)
        | ApiMessage::RecoverError(e) => state.script_note_error(&e),
        _ => {}
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
    fn script_lines_strips_comments_and_blanks() {
        let text = "# header\n\ntravel 2 0 0\n  mine metals 500  \n# trailing\nrecover box\n";
        assert_eq!(
            script_lines(text),
            vec!["travel 2 0 0", "mine metals 500", "recover box"]
        );
    }
}
