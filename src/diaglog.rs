//! Diagnostic log file (issue #309).
//!
//! A TUI owns the screen, so the cockpit reports failures the only way it can:
//! a toast that expires after five seconds, or a status-bar chip that vanishes
//! with the condition. Nothing survives. A pilot returning to a wedged cockpit
//! ten minutes later has no record of what the server said, and two bugs have
//! already had to be instructed backwards from screenshots because the rejected
//! calls left no trace.
//!
//! This is **not** the ship's log. That one (`store`'s `events` table) records
//! pilot *actions*, narrated, as a captain's log: it says "mining ordered", it
//! never says "the server refused". This file is the other half.
//!
//! Three properties shape the implementation:
//!
//! - **It never blocks the event loop.** Lines cross an unbounded channel to a
//!   writer thread, the same shape as `store::spawn_writer`. A cockpit that
//!   stutters because it is logging is worse than one that does not log.
//! - **It never leaks the API key.** Redaction is a property of the writer, not
//!   a discipline at each call site: every line is scrubbed on the way out. A
//!   log file is the artefact a pilot pastes into a bug report — that is
//!   exactly how a key leaks.
//! - **Off costs nothing.** The level is checked before the message is built,
//!   so a disabled logger does not format strings it will throw away.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;

/// Size at which the active file is rotated to `<name>.1`, in bytes. One
/// previous file is kept: enough to survive a session, bounded on a cockpit
/// left running for days.
pub const ROTATE_AT_BYTES: u64 = 1024 * 1024;

/// Environment override for a one-off debugging run, e.g.
/// `NEUMANN_COCKPIT_LOG=debug`. Takes precedence over the config key.
pub const LEVEL_ENV: &str = "NEUMANN_COCKPIT_LOG";

/// What the pilot wants recorded. Ordered: a logger set to `Info` also writes
/// `Error` lines.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Nothing at all; the writer thread is never even started.
    Off,
    /// Failures only — the default. It is what the incidents behind this
    /// feature actually needed, and it stays quiet on a healthy session.
    #[default]
    Error,
    /// Failures plus the shape of the session: boot, probe switches, the
    /// sequencers' decisions.
    Info,
    /// Every API outcome, successes included.
    Debug,
}

impl Level {
    pub fn from_label(label: &str) -> Option<Self> {
        match label.trim().to_ascii_lowercase().as_str() {
            "off" | "none" => Some(Level::Off),
            "error" => Some(Level::Error),
            "info" => Some(Level::Info),
            "debug" => Some(Level::Debug),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Level::Off => "off",
            Level::Error => "error",
            Level::Info => "info",
            Level::Debug => "debug",
        }
    }
}

/// A cheap-to-clone handle onto the writer thread. Cloned into the API client
/// and held by `AppState`, like the metrics ring and the rate-limit cell.
#[derive(Clone)]
pub struct Logger(Option<Arc<Inner>>);

struct Inner {
    level: Level,
    tx: Sender<String>,
}

impl Logger {
    /// A logger that discards everything and starts no thread.
    pub fn disabled() -> Self {
        Logger(None)
    }

    /// Open `path` for appending and start the writer thread. `secret`, when
    /// non-empty, is scrubbed from every line before it is written.
    ///
    /// A path that cannot be opened yields a disabled logger rather than an
    /// error: failing to start because logging failed would be absurd.
    pub fn open(path: &Path, level: Level, secret: &str) -> Self {
        if level == Level::Off {
            return Logger::disabled();
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
            return Logger::disabled();
        };
        let (tx, rx) = mpsc::channel::<String>();
        let path = path.to_path_buf();
        let secret = secret.to_string();
        std::thread::spawn(move || {
            for line in rx {
                let line = redact(&line, &secret);
                if writeln!(file, "{line}").is_err() {
                    continue; // a failing log must not become a second failure
                }
                let _ = file.flush();
                if let Some(rotated) = rotate_if_needed(&path, &file) {
                    file = rotated;
                }
            }
        });
        Logger(Some(Arc::new(Inner { level, tx })))
    }

    /// Whether `level` would be written. Call sites use it to skip building a
    /// message that would be discarded.
    pub fn enabled(&self, level: Level) -> bool {
        self.0.as_ref().is_some_and(|inner| level <= inner.level)
    }

    /// Record one line. The message is built **only** when the level passes.
    pub fn log(&self, level: Level, message: impl FnOnce() -> String) {
        if !self.enabled(level) {
            return;
        }
        let Some(inner) = self.0.as_ref() else { return };
        let stamp = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f");
        let _ = inner.tx.send(format!("{stamp} {:<5} {}", level.label(), message()));
    }

    pub fn error(&self, message: impl FnOnce() -> String) {
        self.log(Level::Error, message);
    }

    pub fn info(&self, message: impl FnOnce() -> String) {
        self.log(Level::Info, message);
    }

    pub fn debug(&self, message: impl FnOnce() -> String) {
        self.log(Level::Debug, message);
    }
}

impl std::fmt::Debug for Logger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.as_ref() {
            None => write!(f, "Logger(off)"),
            Some(inner) => write!(f, "Logger({})", inner.level.label()),
        }
    }
}

impl Default for Logger {
    fn default() -> Self {
        Logger::disabled()
    }
}

/// Remove the API key from a line, whatever shape it arrived in.
///
/// Two passes, because a key reaches a message either verbatim (an error
/// echoing a URL or a header) or behind `Bearer`. The second pass matters even
/// when `secret` is empty: a value we never learned still must not be written.
pub fn redact(line: &str, secret: &str) -> String {
    let mut out = if secret.is_empty() {
        line.to_string()
    } else {
        line.replace(secret, "<redacted>")
    };
    // `Bearer <token>` in any casing, up to the next whitespace.
    let lower = out.to_lowercase();
    if let Some(at) = lower.find("bearer ") {
        let start = at + "bearer ".len();
        let end = out[start..]
            .find(char::is_whitespace)
            .map(|i| start + i)
            .unwrap_or(out.len());
        if end > start {
            out.replace_range(start..end, "<redacted>");
        }
    }
    out
}

/// Rotate the active file to `<name>.1` once it passes [`ROTATE_AT_BYTES`],
/// returning the freshly opened replacement. Any failure leaves the current
/// file in place — logging must never be the thing that breaks.
fn rotate_if_needed(path: &PathBuf, file: &std::fs::File) -> Option<std::fs::File> {
    if file.metadata().ok()?.len() < ROTATE_AT_BYTES {
        return None;
    }
    let previous = path.with_extension("log.1");
    std::fs::rename(path, previous).ok()?;
    OpenOptions::new().create(true).append(true).open(path).ok()
}

/// Path of the diagnostic log: beside the database, under the state dir.
pub fn log_path() -> PathBuf {
    let mut path = crate::config::db_path();
    path.set_file_name("cockpit.log");
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_ordered_so_a_higher_one_includes_the_lower() {
        assert!(Level::Error < Level::Info);
        assert!(Level::Info < Level::Debug);
        assert!(Level::Off < Level::Error);
    }

    #[test]
    fn level_labels_round_trip() {
        for level in [Level::Off, Level::Error, Level::Info, Level::Debug] {
            assert_eq!(Level::from_label(level.label()), Some(level));
        }
        assert_eq!(Level::from_label("DEBUG"), Some(Level::Debug), "case-insensitive");
        assert_eq!(Level::from_label("chatty"), None);
    }

    #[test]
    fn the_api_key_never_reaches_the_file() {
        // The whole reason redaction lives in the writer: a call site that
        // forgets is not supposed to be able to leak.
        let key = "vng_supersecret123";
        let line = format!("GET /api/probe failed: 401 for key {key} (Bearer {key})");
        let clean = redact(&line, key);
        assert!(!clean.contains(key), "the key survived: {clean}");
        assert!(clean.contains("<redacted>"));
    }

    #[test]
    fn a_bearer_token_is_scrubbed_even_when_the_key_is_unknown() {
        let clean = redact("authorization: Bearer abc.def.ghi trailing", "");
        assert!(!clean.contains("abc.def.ghi"), "{clean}");
        assert!(clean.contains("trailing"), "only the token goes: {clean}");
    }

    #[test]
    fn a_disabled_logger_builds_nothing() {
        let logger = Logger::disabled();
        assert!(!logger.enabled(Level::Error));
        // The closure must not run: building the message is the cost we are
        // avoiding when logging is off.
        logger.error(|| panic!("a disabled logger must not build its message"));
    }

    #[test]
    fn a_level_below_the_threshold_builds_nothing() {
        let dir = std::env::temp_dir().join("nc_diaglog_level_test");
        let _ = std::fs::remove_dir_all(&dir);
        let logger = Logger::open(&dir.join("cockpit.log"), Level::Error, "k");
        assert!(logger.enabled(Level::Error));
        assert!(!logger.enabled(Level::Info));
        logger.info(|| panic!("below the threshold, so never built"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lines_reach_the_file_redacted() {
        let dir = std::env::temp_dir().join("nc_diaglog_write_test");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("cockpit.log");
        let logger = Logger::open(&path, Level::Info, "vng_key_abc");
        logger.error(|| "boom with vng_key_abc inside".to_string());
        drop(logger); // closes the channel; the writer drains and exits

        let mut body = String::new();
        for _ in 0..200 {
            body = std::fs::read_to_string(&path).unwrap_or_default();
            if !body.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(body.contains("boom with"), "the line was written: {body:?}");
        assert!(body.contains("error"), "the level is on the line: {body:?}");
        assert!(!body.contains("vng_key_abc"), "and the key is not: {body:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
