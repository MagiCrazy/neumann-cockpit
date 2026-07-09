//! Ship's log — the local action/event journal shown in the Missions pane.
//!
//! Each entry is a captain's-log line: a pre-rendered `summary` sentence where
//! notable entities (container/waypoint names, sectors, probes) are wrapped in
//! `«…»` so the renderer can emphasize them without re-parsing structured data.
//! Entries are persisted append-only in the `events` SQLite table (see `store`)
//! and kept in full there for long-term stats; only the most recent
//! [`store::JOURNAL_WINDOW`](crate::store) are held in memory.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single ship's-log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    /// When the action was issued / the event occurred (local clock).
    pub occurred_at: DateTime<Utc>,
    /// Coarse category, drives the per-entry emphasis colour (see [`kind`]).
    pub kind: String,
    /// Probe the action targeted (provenance), if any.
    pub probe_id: Option<u64>,
    /// Pre-rendered captain's-log sentence; `«…»` marks emphasized entities.
    pub summary: String,
    /// Structured detail (verbatim args), reserved for future filtering.
    #[serde(default)]
    pub data: serde_json::Value,
}

impl LogEvent {
    /// A log entry stamped now, carrying no structured payload.
    pub fn action(kind: impl Into<String>, summary: impl Into<String>, probe_id: Option<u64>) -> Self {
        Self {
            occurred_at: Utc::now(),
            kind: kind.into(),
            probe_id,
            summary: summary.into(),
            data: serde_json::Value::Null,
        }
    }

    // ── Narrative constructors ──────────────────────────────────────────────
    // Each renders a captain's-log sentence; `«…»` wraps entities the log view
    // paints in the accent colour.

    /// "Laid in a course for sector «(x, y, z)»."
    pub fn travel(x: i32, y: i32, z: i32, probe_id: Option<u64>) -> Self {
        Self::action(
            kind::TRAVEL,
            format!("Laid in a course for sector «({x}, {y}, {z})»."),
            probe_id,
        )
    }

    /// "Dispatched a manny to mine {amount} «resources», hauling to «dest»."
    pub fn mine(resources: &str, amount: f64, destination: &str, probe_id: Option<u64>) -> Self {
        Self::action(
            kind::MINE,
            format!("Dispatched a manny to mine {amount:.2} «{resources}», hauling to «{destination}»."),
            probe_id,
        )
    }

    /// "Rerouted {amount} «resource» from «from» to «to»."
    pub fn storage_move_resource(amount: f64, resource: &str, from: &str, to: &str, probe_id: Option<u64>) -> Self {
        Self::action(
            kind::STORAGE_MOVE,
            format!("Rerouted {amount:.2} «{resource}» from «{from}» to «{to}»."),
            probe_id,
        )
    }

    /// "Moved N item(s) to «to»."
    pub fn storage_move_items(count: usize, to: &str, probe_id: Option<u64>) -> Self {
        let plural = if count == 1 { "" } else { "s" };
        Self::action(
            kind::STORAGE_MOVE,
            format!("Moved {count} item{plural} to «{to}»."),
            probe_id,
        )
    }

    /// "Dropped container «container» onto «planet»."
    pub fn drop_container(container: &str, planet: &str, probe_id: Option<u64>) -> Self {
        Self::action(
            kind::CONTAINER,
            format!("Dropped container «{container}» onto «{planet}»."),
            probe_id,
        )
    }

    /// "Detached container «container», {set adrift | hidden on an asteroid}."
    pub fn detach_container(container: &str, hidden_on_asteroid: bool, probe_id: Option<u64>) -> Self {
        let placement = if hidden_on_asteroid { "hidden on an asteroid" } else { "set adrift" };
        Self::action(
            kind::CONTAINER,
            format!("Detached container «{container}», {placement}."),
            probe_id,
        )
    }

    /// "Installed waypoint «name»."
    pub fn deploy_waypoint(name: &str, probe_id: Option<u64>) -> Self {
        Self::action(
            kind::WAYPOINT,
            format!("Installed waypoint «{name}»."),
            probe_id,
        )
    }
}

/// Event-kind tags. Local actions and reconstructed server events share this
/// space; the log view colours entries by category (server events warn/crit).
pub mod kind {
    pub const TRAVEL: &str = "travel";
    pub const MINE: &str = "mine";
    pub const CRAFT: &str = "craft";
    pub const STORAGE_MOVE: &str = "storage_move";
    pub const CONTAINER: &str = "container";
    pub const WAYPOINT: &str = "waypoint";
    /// Reconstructed server-side event (alert / damage warning).
    pub const ALERT: &str = "alert";
}
