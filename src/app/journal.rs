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
        let placement = if hidden_on_asteroid {
            "hidden on an asteroid"
        } else {
            "set adrift"
        };
        Self::action(
            kind::CONTAINER,
            format!("Detached container «{container}», {placement}."),
            probe_id,
        )
    }

    /// "Installed waypoint «name»."
    pub fn deploy_waypoint(name: &str, probe_id: Option<u64>) -> Self {
        Self::action(kind::WAYPOINT, format!("Installed waypoint «{name}»."), probe_id)
    }

    pub fn repair(pct: f64, probe_id: Option<u64>) -> Self {
        Self::action(kind::REPAIR, format!("Ordered a hull repair to {pct:.0}%."), probe_id)
    }

    /// Fabrication order, on the atomic printer or at the manny bay.
    pub fn craft(recipe: &str, atomic_printer: bool, probe_id: Option<u64>) -> Self {
        let bay = if atomic_printer {
            "on the atomic printer"
        } else {
            "at the manny bay"
        };
        Self::action(kind::CRAFT, format!("Queued «{recipe}» {bay}."), probe_id)
    }

    pub fn salvage(target: &str, probe_id: Option<u64>) -> Self {
        Self::action(kind::SALVAGE, format!("Sent a manny to salvage «{target}»."), probe_id)
    }

    pub fn recall(manny: &str, abandon: bool, probe_id: Option<u64>) -> Self {
        let s = if abandon {
            format!("Abandoned «{manny}» in a remote sector.")
        } else {
            format!("Recalled «{manny}».")
        };
        Self::action(kind::RECALL, s, probe_id)
    }

    pub fn recover(container: &str, probe_id: Option<u64>) -> Self {
        Self::action(
            kind::RECOVER,
            format!("Sent a manny to recover container «{container}»."),
            probe_id,
        )
    }

    pub fn rename_manny(old: &str, new: &str, probe_id: Option<u64>) -> Self {
        Self::action(kind::RENAME, format!("Renamed manny «{old}» to «{new}»."), probe_id)
    }

    pub fn rename_container(new: &str, probe_id: Option<u64>) -> Self {
        Self::action(kind::RENAME, format!("Renamed a container to «{new}»."), probe_id)
    }

    pub fn rename_probe(new: &str, probe_id: Option<u64>) -> Self {
        Self::action(kind::RENAME, format!("Renamed the probe to «{new}»."), probe_id)
    }

    pub fn set_default_probe(name: &str, probe_id: Option<u64>) -> Self {
        Self::action(kind::RENAME, format!("Set «{name}» as the default probe."), probe_id)
    }

    pub fn improve(improvement: &str, probe_id: Option<u64>) -> Self {
        Self::action(kind::IMPROVE, format!("Began installing «{improvement}»."), probe_id)
    }

    pub fn inspect(target: &str, probe_id: Option<u64>) -> Self {
        Self::action(kind::INSPECT, format!("Sent a manny to inspect «{target}»."), probe_id)
    }

    pub fn refuel(probe_id: Option<u64>) -> Self {
        Self::action(
            kind::REFUEL,
            "Sent a manny to refill the deuterium tank.".to_string(),
            probe_id,
        )
    }

    /// "Dispatched a manny to ferry {amount}% deuterium to «target»."
    pub fn transfer_deuterium(target: &str, amount: f64, probe_id: Option<u64>) -> Self {
        Self::action(
            kind::TRANSFER,
            format!("Dispatched a manny to ferry {amount:.0}% deuterium to «{target}»."),
            probe_id,
        )
    }

    pub fn transfer_manny(manny: &str, target: &str, probe_id: Option<u64>) -> Self {
        Self::action(
            kind::TRANSFER,
            format!("Dispatched «{manny}» to transfer to probe «{target}»."),
            probe_id,
        )
    }

    pub fn assemble_probe(probe_id: Option<u64>) -> Self {
        Self::action(
            kind::ASSEMBLE,
            "Began assembling a new drone (~3h).".to_string(),
            probe_id,
        )
    }

    pub fn drop_cargo(probe_id: Option<u64>) -> Self {
        Self::action(
            kind::DROP_CARGO,
            "Dumped a manny's onboard cargo.".to_string(),
            probe_id,
        )
    }

    pub fn mind_snapshot(probe_id: Option<u64>) -> Self {
        Self::action(
            kind::MIND_SNAPSHOT,
            "Reassigned the mind snapshot to a fresh probe.".to_string(),
            probe_id,
        )
    }

    pub fn mission_abandon(title: &str, probe_id: Option<u64>) -> Self {
        Self::action(kind::MISSION, format!("Abandoned mission «{title}»."), probe_id)
    }

    pub fn relay_on(network: Option<&str>, probe_id: Option<u64>) -> Self {
        let s = match network {
            Some(n) if !n.is_empty() => format!("Turned on SCUT relay «{n}»."),
            _ => "Turned on a SCUT relay.".to_string(),
        };
        Self::action(kind::RELAY, s, probe_id)
    }

    pub fn message_sent(recipient: &str, probe_id: Option<u64>) -> Self {
        Self::action(kind::MESSAGE, format!("Sent a message to «{recipient}»."), probe_id)
    }

    pub fn container_rules(container: &str, probe_id: Option<u64>) -> Self {
        Self::action(
            kind::RULES,
            format!("Updated routing rules for «{container}»."),
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
    pub const REPAIR: &str = "repair";
    pub const SALVAGE: &str = "salvage";
    pub const RECALL: &str = "recall";
    pub const RECOVER: &str = "recover";
    pub const RENAME: &str = "rename";
    pub const IMPROVE: &str = "improve";
    pub const INSPECT: &str = "inspect";
    pub const REFUEL: &str = "refuel";
    pub const TRANSFER: &str = "transfer";
    pub const ASSEMBLE: &str = "assemble";
    pub const DROP_CARGO: &str = "drop_cargo";
    pub const MIND_SNAPSHOT: &str = "mind_snapshot";
    pub const MISSION: &str = "mission";
    pub const RELAY: &str = "relay";
    pub const MESSAGE: &str = "message";
    pub const RULES: &str = "rules";
    /// Reconstructed server-side event (alert / damage warning).
    pub const ALERT: &str = "alert";
}
