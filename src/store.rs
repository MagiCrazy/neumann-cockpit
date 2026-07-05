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
//! giving clean columns to query and sort on. Room is left for an
//! `action_audit` table later.

use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};

use rusqlite::Connection;

use crate::api::types::SectorObservation;

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
    scanned_at        TEXT,
    data              TEXT NOT NULL,
    PRIMARY KEY (x, y, z)
);
CREATE INDEX IF NOT EXISTS idx_sector_scanned_at
    ON sector_observations (scanned_at);
";

/// Messages accepted by the persistence writer thread.
pub enum PersistMsg {
    /// Upsert the latest observation for a sector (keyed by coordinates).
    UpsertObservation(SectorObservation),
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
    serde_json::to_value(v).ok().and_then(|j| j.as_str().map(str::to_string))
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
             navigational_risk, message, object_count, scanned_at, data)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
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
    let Ok(mut stmt) =
        conn.prepare("SELECT data FROM sector_observations ORDER BY scanned_at DESC")
    else {
        return Vec::new();
    };
    let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) else {
        return Vec::new();
    };
    rows.filter_map(Result::ok)
        .filter_map(|json| serde_json::from_str::<SectorObservation>(&json).ok())
        .collect()
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
    let Ok(data) = std::fs::read(json_path) else { return Ok(MigrationOutcome::NoLegacyFile) };
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
/// channel to send persistence messages on. `rusqlite` is synchronous, so the
/// writes run off the tokio runtime on this dedicated thread. Write errors are
/// best-effort (dropped) until the tracing work lands.
pub fn spawn_writer(conn: Connection) -> Sender<PersistMsg> {
    let (tx, rx): (Sender<PersistMsg>, Receiver<PersistMsg>) = mpsc::channel();
    std::thread::spawn(move || {
        for msg in rx {
            match msg {
                PersistMsg::UpsertObservation(obs) => {
                    let _ = upsert_observation(&conn, &obs);
                }
            }
        }
    });
    tx
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn migrate_imports_then_deletes_json() {
        let mut conn = mem();
        let path = std::env::temp_dir().join("nc_migrate_import_test.json");
        let history = vec![obs(1.0, 1.0, 1.0), obs(2.0, 2.0, 2.0)];
        std::fs::write(&path, serde_json::to_vec(&history).unwrap()).unwrap();
        assert_eq!(migrate_legacy_json(&mut conn, &path).unwrap(), MigrationOutcome::Imported(2));
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
        assert_eq!(migrate_legacy_json(&mut conn, &path).unwrap(), MigrationOutcome::AlreadyMigrated);
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
