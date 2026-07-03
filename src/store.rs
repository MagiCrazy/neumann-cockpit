//! Local SQLite persistence.
//!
//! Scope today: the sector scan history, keyed by coordinates, migrated off the
//! non-atomic whole-file JSON write (`scan_history.json`). A single writer
//! thread owns the connection, so writes are serialized and atomic by
//! construction — matching the app's single-owner, message-passing design.
//! The schema leaves room for an action-audit table later.

use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};

use rusqlite::Connection;

use crate::api::types::SectorObservation;

/// Messages accepted by the persistence writer thread.
pub enum PersistMsg {
    /// Upsert the latest observation for a sector (keyed by coordinates).
    UpsertObservation(SectorObservation),
}

/// Open (creating if needed) the cockpit database and ensure the schema.
pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(path)?;
    // WAL keeps the single-writer / startup-reader pair snappy and durable.
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sector_observations (
            x           INTEGER NOT NULL,
            y           INTEGER NOT NULL,
            z           INTEGER NOT NULL,
            scanned_at  TEXT,
            data        TEXT NOT NULL,
            PRIMARY KEY (x, y, z)
        )",
        [],
    )?;
    Ok(conn)
}

fn coords(obs: &SectorObservation) -> (i64, i64, i64) {
    (
        obs.relative_coordinates.x as i64,
        obs.relative_coordinates.y as i64,
        obs.relative_coordinates.z as i64,
    )
}

/// Insert or replace the observation for its sector (keyed by coordinates,
/// mirroring the in-memory dedupe in `AppState::update_sector`).
pub fn upsert_observation(conn: &Connection, obs: &SectorObservation) -> rusqlite::Result<()> {
    let (x, y, z) = coords(obs);
    let scanned_at = obs.scanned_at.map(|t| t.to_rfc3339());
    let data = serde_json::to_string(obs).unwrap_or_default();
    conn.execute(
        "INSERT OR REPLACE INTO sector_observations (x, y, z, scanned_at, data)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![x, y, z, scanned_at, data],
    )?;
    Ok(())
}

/// Load all observations, most-recently-scanned first — matching the in-memory
/// `scan_history` ordering (newest at index 0). Best-effort: any error yields
/// an empty history, like the old corrupt-JSON path.
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

/// One-time import of the legacy `scan_history.json`, only while the table is
/// still empty — so DB-native data is never clobbered.
pub fn import_legacy_json(conn: &Connection, json_path: &Path) {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sector_observations", [], |r| r.get(0))
        .unwrap_or(0);
    if count > 0 {
        return;
    }
    let Ok(data) = std::fs::read(json_path) else { return };
    let Ok(history) = serde_json::from_slice::<Vec<SectorObservation>>(&data) else { return };
    for obs in &history {
        let _ = upsert_observation(conn, obs);
    }
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
        conn.execute(
            "CREATE TABLE sector_observations (x INTEGER, y INTEGER, z INTEGER,
             scanned_at TEXT, data TEXT NOT NULL, PRIMARY KEY (x, y, z))",
            [],
        )
        .unwrap();
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
}
