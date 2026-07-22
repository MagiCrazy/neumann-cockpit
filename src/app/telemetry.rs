//! Telemetry time series — periodic samples of the probe's vital ratios
//! (fuel / integrity / cargo), persisted so the zoomed Probe pane can draw
//! sparklines of how they trend over a session (issue #201).
//!
//! Samples are taken on each probe refresh (`AppState::update_probe`) and
//! deduplicated against the previous one, so an idle probe does not flood the
//! series with identical points. They persist to the `telemetry` SQLite table
//! (append-only) and the most recent [`store::TELEMETRY_WINDOW`](crate::store)
//! are loaded into memory at boot, exactly like the ship's log.

use chrono::{DateTime, Utc};

use crate::api::types::Probe;

/// One telemetry sample: the three vital ratios (each `0.0..=1.0`) at a point
/// in time, tagged with the probe they belong to so a multi-probe series can
/// be filtered per probe at render time.
#[derive(Debug, Clone, PartialEq)]
pub struct TelemetrySample {
    pub occurred_at: DateTime<Utc>,
    pub probe_id: Option<u64>,
    pub fuel: f64,
    pub integrity: f64,
    pub cargo: f64,
}

impl TelemetrySample {
    /// Derive a sample from a probe snapshot, mirroring the gauge maths in
    /// `ui::panels::probe` so the sparkline tracks exactly what the gauges show.
    /// `probe_id` is the active-probe tag (which may differ from the default),
    /// so switching probes keeps each series distinct.
    pub fn from_probe(probe: &Probe, probe_id: Option<u64>) -> Self {
        let max_deuterium = probe.fuel.max_deuterium.unwrap_or(100.0);
        let fuel = probe
            .fuel
            .deuterium
            .map(|d| if max_deuterium > 0.0 { d / max_deuterium } else { 0.0 })
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        let integrity = probe
            .systems
            .as_ref()
            .and_then(|s| s.integrity_percent)
            .map(|i| i / 100.0)
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);
        let inv = &probe.inventory;
        let cargo = if inv.capacity > 0.0 {
            (inv.used_capacity / inv.capacity).clamp(0.0, 1.0)
        } else {
            0.0
        };
        Self {
            occurred_at: Utc::now(),
            probe_id,
            fuel,
            integrity,
            cargo,
        }
    }

    /// Whether the vital ratios match another sample (ignoring the timestamp),
    /// so consecutive identical samples can be dropped from the series.
    pub fn same_vitals(&self, other: &TelemetrySample) -> bool {
        self.probe_id == other.probe_id
            && self.fuel == other.fuel
            && self.integrity == other.integrity
            && self.cargo == other.cargo
    }
}
