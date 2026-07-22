//! Local SQLite persistence.
//!
//! Scope today: the sector scan history, keyed by coordinates, migrated off the
//! non-atomic whole-file JSON write (`scan_history.json`). A single writer
//! thread owns the connection, so writes are serialized and atomic by
//! construction — matching the app's single-owner, message-passing design.
//!
//! Schema shape: the stable top-level fields of an observation are promoted to
//! typed columns (queryable/indexable), and the **full API payload** is kept
//! verbatim in `data` as JSON. This keeps the row faithful to what the API
//! returns and forward-compatible with API drift (new fields ride along in the
//! payload, exactly like the serde `Unknown` fallbacks elsewhere) while still
//! giving clean columns to query and sort on. The `events` table holds the
//! ship's log — an append-only action/event journal kept in full for long-term
//! stats; only the most recent [`JOURNAL_WINDOW`] rows are loaded into memory.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::api::types::SectorObservation;
use crate::app::{LogEvent, TelemetrySample};

/// Table + index DDL, shared by `open` and the tests.
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS sector_observations (
    x                 INTEGER NOT NULL,
    y                 INTEGER NOT NULL,
    z                 INTEGER NOT NULL,
    distance          INTEGER,
    knowledge_level   TEXT,
    confidence        REAL,
    navigational_risk TEXT,
    message           TEXT,
    object_count      INTEGER,
    observed_by       INTEGER,
    scanned_at        TEXT,
    data              TEXT NOT NULL,
    PRIMARY KEY (x, y, z)
);
CREATE INDEX IF NOT EXISTS idx_sector_scanned_at
    ON sector_observations (scanned_at);

CREATE TABLE IF NOT EXISTS events (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    occurred_at  TEXT NOT NULL,
    kind         TEXT NOT NULL,
    probe_id     INTEGER,
    summary      TEXT NOT NULL,
    data         TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_events_occurred_at
    ON events (occurred_at);

CREATE TABLE IF NOT EXISTS telemetry (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    occurred_at TEXT NOT NULL,
    probe_id    INTEGER,
    fuel        REAL NOT NULL,
    integrity   REAL NOT NULL,
    cargo       REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_telemetry_probe
    ON telemetry (probe_id, id);
";

/// How many recent ship's-log entries the cockpit loads into memory and keeps
/// in `AppState::journal`. The `events` table itself is never trimmed — the
/// full history is retained for long-term stats; this only bounds the in-memory
/// working set and what the pane can scroll through.
pub const JOURNAL_WINDOW: usize = 1000;

/// How many recent telemetry samples are loaded into memory at boot (across all
/// probes, newest first then reversed to chronological order). The `telemetry`
/// table itself is append-only and never trimmed; this only bounds the working
/// set the sparklines draw from.
pub const TELEMETRY_WINDOW: usize = 512;

/// Additive column migrations for databases that predate a column. `CREATE
/// TABLE IF NOT EXISTS` never alters an existing table, so a new promoted
/// column must be back-filled with `ALTER TABLE ADD COLUMN`. Each ALTER is
/// best-effort: it errors with "duplicate column" once the column exists
/// (fresh DBs get it from `SCHEMA`), which we deliberately ignore.
fn ensure_columns(conn: &Connection) {
    // observed_by — scan provenance (API v81 multi-probe).
    let _ = conn.execute("ALTER TABLE sector_observations ADD COLUMN observed_by INTEGER", []);
}

/// Messages accepted by the persistence writer thread.
pub enum PersistMsg {
    /// Upsert the latest observation for a sector (keyed by coordinates).
    UpsertObservation(SectorObservation),
    /// Append a ship's-log entry (append-only, trimmed to the retention cap).
    AppendEvent(LogEvent),
    /// Append a telemetry sample (append-only vital-ratio time series).
    AppendTelemetry(TelemetrySample),
}

/// What `migrate_legacy_json` did, so the boot preflight can report it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationOutcome {
    /// The table already had rows — no import, JSON left untouched.
    AlreadyMigrated,
    /// No legacy `scan_history.json` to import (fresh install or already gone).
    NoLegacyFile,
    /// Imported N observations from the JSON, then removed the file.
    Imported(usize),
}

/// Open (creating if needed) the cockpit database and ensure the schema.
pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(path)?;
    // WAL keeps the single-writer / startup-reader pair snappy and durable.
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.execute_batch(SCHEMA)?;
    ensure_columns(&conn);
    Ok(conn)
}

fn coords(obs: &SectorObservation) -> (i64, i64, i64) {
    (
        obs.relative_coordinates.x as i64,
        obs.relative_coordinates.y as i64,
        obs.relative_coordinates.z as i64,
    )
}

/// Serialize a simple serde enum to its string form (e.g. `KnowledgeLevel` →
/// `"detailed"`), for storing in a text column.
fn enum_text<T: serde::Serialize>(v: &T) -> Option<String> {
    serde_json::to_value(v)
        .ok()
        .and_then(|j| j.as_str().map(str::to_string))
}

/// Insert or replace the observation for its sector (keyed by coordinates,
/// mirroring the in-memory dedupe in `AppState::update_sector`). Promotes the
/// stable fields to columns and stores the full API payload in `data`.
pub fn upsert_observation(conn: &Connection, obs: &SectorObservation) -> rusqlite::Result<()> {
    let (x, y, z) = coords(obs);
    let data = serde_json::to_string(obs).unwrap_or_default();
    conn.execute(
        "INSERT OR REPLACE INTO sector_observations
            (x, y, z, distance, knowledge_level, confidence,
             navigational_risk, message, object_count, observed_by, scanned_at, data)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        rusqlite::params![
            x,
            y,
            z,
            obs.distance,
            enum_text(&obs.knowledge_level),
            obs.confidence,
            &obs.navigational_risk,
            &obs.message,
            obs.objects.as_ref().map(|o| o.len() as i64),
            obs.observed_by.map(|v| v as i64),
            obs.scanned_at.map(|t| t.to_rfc3339()),
            data,
        ],
    )?;
    Ok(())
}

/// Load all observations, most-recently-scanned first — matching the in-memory
/// `scan_history` ordering (newest at index 0). The full observation is
/// reconstructed from the `data` payload; the columns are for querying only.
/// Best-effort: any error yields an empty history, like the old corrupt-JSON
/// path.
pub fn load_observations(conn: &Connection) -> Vec<SectorObservation> {
    let Ok(mut stmt) = conn.prepare("SELECT data FROM sector_observations ORDER BY scanned_at DESC") else {
        return Vec::new();
    };
    let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) else {
        return Vec::new();
    };
    rows.filter_map(Result::ok)
        .filter_map(|json| serde_json::from_str::<SectorObservation>(&json).ok())
        .collect()
}

/// Append a ship's-log entry. The journal is append-only and never trimmed —
/// the full history is kept for long-term stats (queried straight off the table).
pub fn append_event(conn: &Connection, ev: &LogEvent) -> rusqlite::Result<()> {
    let data = serde_json::to_string(&ev.data).unwrap_or_else(|_| "null".to_string());
    conn.execute(
        "INSERT INTO events (occurred_at, kind, probe_id, summary, data)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            ev.occurred_at.to_rfc3339(),
            ev.kind,
            ev.probe_id.map(|v| v as i64),
            ev.summary,
            data,
        ],
    )?;
    Ok(())
}

/// Load the most recent [`JOURNAL_WINDOW`] ship's-log entries, newest first —
/// matching the in-memory `AppState::journal` ordering. The stored RFC 3339
/// timestamp is reparsed; best-effort, any error yields an empty log.
pub fn load_events(conn: &Connection) -> Vec<LogEvent> {
    let Ok(mut stmt) = conn.prepare(
        "SELECT occurred_at, kind, probe_id, summary, data
         FROM events ORDER BY id DESC LIMIT ?1",
    ) else {
        return Vec::new();
    };
    let Ok(rows) = stmt.query_map(rusqlite::params![JOURNAL_WINDOW as i64], |row| {
        let occurred_at = row
            .get::<_, String>(0)?
            .parse::<DateTime<Utc>>()
            .unwrap_or_else(|_| Utc::now());
        Ok(LogEvent {
            occurred_at,
            kind: row.get(1)?,
            probe_id: row.get::<_, Option<i64>>(2)?.map(|v| v as u64),
            summary: row.get(3)?,
            data: row
                .get::<_, String>(4)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(serde_json::Value::Null),
        })
    }) else {
        return Vec::new();
    };
    rows.filter_map(Result::ok).collect()
}

/// Append a telemetry sample. The series is append-only and never trimmed —
/// the full history is kept for long-term stats; only the recent window is
/// loaded into memory (see [`load_telemetry`]).
pub fn append_telemetry(conn: &Connection, s: &TelemetrySample) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO telemetry (occurred_at, probe_id, fuel, integrity, cargo)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            s.occurred_at.to_rfc3339(),
            s.probe_id.map(|v| v as i64),
            s.fuel,
            s.integrity,
            s.cargo,
        ],
    )?;
    Ok(())
}

/// Load the most recent [`TELEMETRY_WINDOW`] telemetry samples in chronological
/// order (oldest first), so the in-memory series can be appended to and the
/// sparklines read its tail. Best-effort: any error yields an empty series.
pub fn load_telemetry(conn: &Connection) -> Vec<TelemetrySample> {
    let Ok(mut stmt) = conn.prepare(
        "SELECT occurred_at, probe_id, fuel, integrity, cargo
         FROM telemetry ORDER BY id DESC LIMIT ?1",
    ) else {
        return Vec::new();
    };
    let Ok(rows) = stmt.query_map(rusqlite::params![TELEMETRY_WINDOW as i64], |row| {
        let occurred_at = row
            .get::<_, String>(0)?
            .parse::<DateTime<Utc>>()
            .unwrap_or_else(|_| Utc::now());
        Ok(TelemetrySample {
            occurred_at,
            probe_id: row.get::<_, Option<i64>>(1)?.map(|v| v as u64),
            fuel: row.get(2)?,
            integrity: row.get(3)?,
            cargo: row.get(4)?,
        })
    }) else {
        return Vec::new();
    };
    // Query is newest-first (by id); reverse to chronological (oldest-first).
    let mut samples: Vec<_> = rows.filter_map(Result::ok).collect();
    samples.reverse();
    samples
}

/// One-time migration of the legacy `scan_history.json`: while the table is
/// still empty, import every observation, then delete the JSON so it stops
/// lingering. The import runs in a transaction and the file is removed **only
/// after a fully successful commit** — a missing/corrupt file or a write error
/// leaves the JSON untouched (data-safe, retried next launch). A non-empty
/// table means we already migrated, so we never re-import or touch the file.
///
/// TEMPORARY: legacy migration, scheduled for removal — see issue #134.
/// The `legacy_migration_removal_reminder` test fails the build after
/// 2027-01-01 so this can't be forgotten.
pub fn migrate_legacy_json(conn: &mut Connection, json_path: &Path) -> rusqlite::Result<MigrationOutcome> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sector_observations", [], |r| r.get(0))
        .unwrap_or(0);
    if count > 0 {
        return Ok(MigrationOutcome::AlreadyMigrated);
    }
    let Ok(data) = std::fs::read(json_path) else {
        return Ok(MigrationOutcome::NoLegacyFile);
    };
    let Ok(history) = serde_json::from_slice::<Vec<SectorObservation>>(&data) else {
        return Ok(MigrationOutcome::NoLegacyFile);
    };
    let tx = conn.transaction()?;
    for obs in &history {
        upsert_observation(&tx, obs)?;
    }
    tx.commit()?;
    // Import committed — retire the legacy file. Failure to remove is harmless
    // (next launch sees a non-empty table and skips).
    let _ = std::fs::remove_file(json_path);
    Ok(MigrationOutcome::Imported(history.len()))
}

/// Spawn the writer thread, taking ownership of the connection, and return the
/// channel to send persistence messages on plus a shared `degraded` flag.
/// `rusqlite` is synchronous, so the writes run off the tokio runtime on this
/// dedicated thread.
///
/// A failing write (disk full, corruption, read-only fs) does **not** crash the
/// thread — it keeps draining so the app never blocks on a full channel — but it
/// sets the `degraded` flag so the cockpit can warn the pilot that history is no
/// longer being saved (issue #216). The flag is sticky: once set it stays set
/// for the session (a single failure means the DB is unhealthy).
pub fn spawn_writer(conn: Connection) -> (Sender<PersistMsg>, Arc<AtomicBool>) {
    let (tx, rx): (Sender<PersistMsg>, Receiver<PersistMsg>) = mpsc::channel();
    let degraded = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&degraded);
    std::thread::spawn(move || {
        for msg in rx {
            let result = match msg {
                PersistMsg::UpsertObservation(obs) => upsert_observation(&conn, &obs),
                PersistMsg::AppendEvent(ev) => append_event(&conn, &ev),
                PersistMsg::AppendTelemetry(s) => append_telemetry(&conn, &s),
            };
            if result.is_err() {
                flag.store(true, Ordering::Relaxed);
            }
        }
    });
    (tx, degraded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writer_flags_degraded_on_failure_and_keeps_draining() {
        // A schema-less connection: every INSERT hits a missing table and errors.
        let conn = Connection::open_in_memory().unwrap();
        let (tx, degraded) = spawn_writer(conn);

        tx.send(PersistMsg::AppendEvent(LogEvent::action("test", "one", None)))
            .unwrap();
        // Wait for the writer to process it and raise the flag (bounded poll).
        let mut set = false;
        for _ in 0..200 {
            if degraded.load(Ordering::Relaxed) {
                set = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(set, "a failing write must set the degraded flag");

        // The thread survived: it is still draining, so a further send succeeds.
        assert!(
            tx.send(PersistMsg::AppendEvent(LogEvent::action("test", "two", None)))
                .is_ok(),
            "writer thread must survive a failing write and keep draining"
        );
    }

    fn obs(x: f64, y: f64, z: f64) -> SectorObservation {
        serde_json::from_str(&format!(
            r#"{{"relativeCoordinates":{{"x":{x},"y":{y},"z":{z}}},"distance":1,
                "knowledgeLevel":"detailed","confidence":1.0,
                "scan":{{"currentSectorResidenceSeconds":60,
                        "requiredResidenceSeconds":60,"scanQuality":1.0}}}}"#
        ))
        .unwrap()
    }

    fn mem() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();
        conn
    }

    #[test]
    fn upsert_is_keyed_by_coordinates() {
        let conn = mem();
        upsert_observation(&conn, &obs(1.0, 2.0, 3.0)).unwrap();
        upsert_observation(&conn, &obs(1.0, 2.0, 3.0)).unwrap();
        upsert_observation(&conn, &obs(4.0, 5.0, 6.0)).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sector_observations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2, "same coords replace, not append");
    }

    #[test]
    fn promoted_columns_are_populated() {
        let conn = mem();
        upsert_observation(&conn, &obs(1.0, 2.0, 3.0)).unwrap();
        let (dist, level, conf): (i64, String, f64) = conn
            .query_row(
                "SELECT distance, knowledge_level, confidence FROM sector_observations",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(dist, 1);
        assert_eq!(level, "detailed", "enum stored as clean text, not JSON");
        assert!((conf - 1.0).abs() < 1e-9);
    }

    #[test]
    fn observed_by_persists_in_column_and_payload() {
        let conn = mem();
        let mut o = obs(1.0, 2.0, 3.0);
        o.observed_by = Some(5);
        upsert_observation(&conn, &o).unwrap();
        // Promoted column populated…
        let col: Option<i64> = conn
            .query_row("SELECT observed_by FROM sector_observations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(col, Some(5));
        // …and it round-trips through the JSON payload on load.
        assert_eq!(load_observations(&conn)[0].observed_by, Some(5));
    }

    #[test]
    fn ensure_columns_backfills_a_legacy_db() {
        // Simulate a pre-v81 DB: create the table without `observed_by`.
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE sector_observations (
                x INTEGER NOT NULL, y INTEGER NOT NULL, z INTEGER NOT NULL,
                distance INTEGER, knowledge_level TEXT, confidence REAL,
                navigational_risk TEXT, message TEXT, object_count INTEGER,
                scanned_at TEXT, data TEXT NOT NULL, PRIMARY KEY (x, y, z));",
        )
        .unwrap();
        ensure_columns(&conn); // adds observed_by
                               // A second call must not fail (column already present).
        ensure_columns(&conn);
        // The new column is now writable via the normal upsert.
        let mut o = obs(1.0, 1.0, 1.0);
        o.observed_by = Some(7);
        upsert_observation(&conn, &o).unwrap();
        let col: Option<i64> = conn
            .query_row("SELECT observed_by FROM sector_observations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(col, Some(7));
    }

    #[test]
    fn load_orders_newest_first() {
        let conn = mem();
        let mut a = obs(1.0, 1.0, 1.0);
        a.scanned_at = Some("2026-07-03T10:00:00+00:00".parse().unwrap());
        let mut b = obs(2.0, 2.0, 2.0);
        b.scanned_at = Some("2026-07-03T12:00:00+00:00".parse().unwrap());
        upsert_observation(&conn, &a).unwrap();
        upsert_observation(&conn, &b).unwrap();
        let loaded = load_observations(&conn);
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].relative_coordinates.x as i64, 2, "newest first");
    }

    #[test]
    fn events_are_kept_in_full_and_loaded_within_the_window() {
        let conn = mem();
        let n = JOURNAL_WINDOW + 5;
        for i in 0..n {
            append_event(&conn, &LogEvent::action("test", format!("entry {i}"), None)).unwrap();
        }
        // The table keeps the full history (append-only, never trimmed).
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0)).unwrap();
        assert_eq!(count as usize, n, "events are append-only, not trimmed");
        // Loading returns only the most recent window, newest first.
        let loaded = load_events(&conn);
        assert_eq!(loaded.len(), JOURNAL_WINDOW);
        assert_eq!(loaded[0].summary, format!("entry {}", n - 1), "newest first");
    }

    #[test]
    fn telemetry_round_trips_in_chronological_order() {
        let conn = mem();
        for i in 0..3 {
            let s = TelemetrySample {
                occurred_at: format!("2026-07-03T1{i}:00:00+00:00").parse().unwrap(),
                probe_id: Some(1),
                fuel: 1.0 - i as f64 * 0.1,
                integrity: 1.0,
                cargo: i as f64 * 0.2,
            };
            append_telemetry(&conn, &s).unwrap();
        }
        let loaded = load_telemetry(&conn);
        assert_eq!(loaded.len(), 3);
        // Oldest first: fuel decreases, cargo increases with insertion order.
        assert!((loaded[0].fuel - 1.0).abs() < 1e-9, "chronological: oldest first");
        assert!((loaded[2].cargo - 0.4).abs() < 1e-9);
    }

    #[test]
    fn migrate_imports_then_deletes_json() {
        let mut conn = mem();
        let path = std::env::temp_dir().join("nc_migrate_import_test.json");
        let history = vec![obs(1.0, 1.0, 1.0), obs(2.0, 2.0, 2.0)];
        std::fs::write(&path, serde_json::to_vec(&history).unwrap()).unwrap();
        assert_eq!(
            migrate_legacy_json(&mut conn, &path).unwrap(),
            MigrationOutcome::Imported(2)
        );
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sector_observations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2, "all observations imported");
        assert!(!path.exists(), "JSON deleted after a successful import");
    }

    #[test]
    fn migrate_skips_and_keeps_json_when_table_not_empty() {
        let mut conn = mem();
        upsert_observation(&conn, &obs(9.0, 9.0, 9.0)).unwrap();
        let path = std::env::temp_dir().join("nc_migrate_skip_test.json");
        std::fs::write(&path, serde_json::to_vec(&vec![obs(1.0, 1.0, 1.0)]).unwrap()).unwrap();
        assert_eq!(
            migrate_legacy_json(&mut conn, &path).unwrap(),
            MigrationOutcome::AlreadyMigrated
        );
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sector_observations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "did not import into an already-populated table");
        assert!(path.exists(), "JSON left in place when not migrating");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn legacy_migration_removal_reminder() {
        // The scan_history.json migration is one-time (issue #134). Once every
        // local install has migrated, delete `migrate_legacy_json`, its call in
        // main.rs, and this test. This assertion fails the build after the
        // horizon so the cleanup can't be forgotten.
        let today = chrono::Utc::now().date_naive();
        let horizon = chrono::NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        assert!(
            today < horizon,
            "Legacy JSON migration has expired — remove store::migrate_legacy_json \
             and this test (see issue #134)."
        );
    }
}
