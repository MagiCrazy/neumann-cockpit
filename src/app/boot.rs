//! Boot sequence for the cockpit interface.
//!
//! On startup the probe's core computer boots first (centre pane), then the
//! eight subsystems come online centre-out, each running its own themed
//! teletype self-check. Driven by a short-lived animation tick that only runs
//! while `booting` — steady state stays event-driven.

use super::{AppState, Pane};

/// Frames (at the ~90 ms boot tick) between each line of a pane's self-check.
pub const BOOT_LINE_STRIDE: u64 = 2;
/// Characters revealed per frame on the active line (teletype effect).
pub const BOOT_CHARS_PER_FRAME: usize = 3;

/// The probe (centre) boots first and takes its time; only after this lead do
/// the eight subsystems come online, centre-out, at the fast cadence.
const PROBE_LEAD: u64 = 12;
/// Minimum boot duration. Long enough for the full self-check to play out
/// (~frame 32) plus a ~1 s hold on the completed screen before the live
/// content loads in. The boot never ends before this.
const BOOT_MIN_FRAMES: u64 = 43;
/// Safety cap: end the boot even if the initial probe fetch never returns.
const BOOT_MAX_FRAMES: u64 = 70;

/// The eight non-probe panes in centre-out order: axial neighbours, then
/// corners.
const OTHER_ORDER: [Pane; 8] = [
    Pane::Map,
    Pane::Missions,
    Pane::Storage,
    Pane::Sector,
    Pane::Scanner,
    Pane::Comms,
    Pane::Mannies,
    Pane::Inventory,
];

/// Boot frame at which a pane comes online: the probe immediately, the rest
/// after the probe's lead, staggered centre-out.
fn boot_reveal_frame(pane: Pane) -> u64 {
    if pane == Pane::Probe {
        return 0;
    }
    let rank = OTHER_ORDER.iter().position(|p| *p == pane).unwrap_or(0) as u64;
    PROBE_LEAD + rank
}

impl AppState {
    /// Advance the boot animation one frame. Ends the boot once the trace has
    /// played and the probe has loaded, or after the safety timeout.
    pub fn boot_tick(&mut self) {
        self.boot_frame = self.boot_frame.saturating_add(1);
        let min_played = self.boot_frame >= BOOT_MIN_FRAMES;
        if (min_played && self.probe.is_some()) || self.boot_frame >= BOOT_MAX_FRAMES {
            self.booting = false;
        }
    }

    /// Overall boot progress in `0.0..=1.0` (for the global loading bar).
    pub fn boot_progress(&self) -> f64 {
        (self.boot_frame as f64 / BOOT_MIN_FRAMES as f64).clamp(0.0, 1.0)
    }

    /// Skip the boot screen (any key).
    pub fn skip_boot(&mut self) {
        self.booting = false;
    }

    /// Whether a pane's border has traced in yet this boot frame.
    pub fn boot_revealed(&self, pane: Pane) -> bool {
        self.boot_frame >= boot_reveal_frame(pane)
    }

    /// The pane coming online on exactly this frame — drawn with the bright
    /// accent as it lights up.
    pub fn boot_leading(&self, pane: Pane) -> bool {
        self.boot_frame == boot_reveal_frame(pane)
    }

    /// Frames elapsed since a pane came online (drives its teletype).
    pub fn boot_elapsed(&self, pane: Pane) -> u64 {
        self.boot_frame.saturating_sub(boot_reveal_frame(pane))
    }

    /// The themed self-check lines for a pane — `(label, result)`. Real values
    /// where we already have them, thematic placeholders otherwise.
    pub fn boot_check_lines(&self, pane: Pane) -> Vec<(&'static str, String)> {
        let s = |x: &str| x.to_string();
        match pane {
            Pane::Probe => vec![
                ("CPU", s("OK")),
                ("RAM 64K", s("OK")),
                ("REACTOR", s("NOMINAL")),
                ("MIND CORE", s("LOADED")),
                ("HULL", s("SYNC")),
                ("FUEL LINE", s("OK")),
                ("CLOCK", s("2337")),
            ],
            Pane::Scanner => vec![
                ("SENSOR ARRAY", s("6/6")),
                ("PHASE LOCK", s("OK")),
                ("RANGE", s("MAX")),
                ("ARCHIVE", format!("{} SEC", self.scan_history.len())),
                ("CALIBRATION", s("OK")),
            ],
            Pane::Map => vec![
                ("NAV CHART", s("LOCKED")),
                ("STELLAR IDX", s("OK")),
                ("WAYPOINTS", s("SYNC")),
                ("VISITED", format!("{}", self.visited_sectors.len())),
                ("GRID", s("ALIGNED")),
            ],
            Pane::Comms => vec![
                (
                    "COMMLINK",
                    self.api_version.map(|v| format!("v{v}")).unwrap_or_else(|| s("SYNC")),
                ),
                ("INBOX", s("SYNC")),
                ("RELAY NET", s("SCAN")),
                ("ANTENNA", s("OK")),
                ("CRYPTO", s("OK")),
            ],
            Pane::Sector => vec![
                ("LOCAL SWEEP", s("OK")),
                ("GRAV FIELD", s("CLEAR")),
                ("HAZARDS", s("NONE")),
                ("OBJECTS", s("SCAN")),
                ("LOCK", s("OK")),
            ],
            Pane::Missions => vec![
                ("DIRECTIVES", s("SYNCED")),
                ("ORDERS", s("OK")),
                ("STEPS", s("OK")),
                ("PRIORITY", s("SET")),
                ("LOG", s("OK")),
            ],
            Pane::Inventory => vec![
                ("MANIFEST", s("SEALED")),
                ("CARGO", s("SYNC")),
                ("PRINTER", s("IDLE")),
                ("RESOURCES", s("OK")),
                ("SEALS", s("OK")),
            ],
            Pane::Storage => vec![
                ("REGISTRY", s("INDEXED")),
                ("BINS", s("SYNC")),
                ("ROUTING", s("OK")),
                ("CAPACITY", s("OK")),
                ("LOCKS", s("OK")),
            ],
            Pane::Mannies => vec![
                ("MANNY BAY", s("UNLOCKED")),
                ("ROSTER", s("SYNC")),
                ("INTERLOCKS", s("OK")),
                ("POWER", s("OK")),
                ("DIAGNOSTICS", s("OK")),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_reveals_first_corners_last() {
        assert_eq!(boot_reveal_frame(Pane::Probe), 0);
        assert!(boot_reveal_frame(Pane::Inventory) > boot_reveal_frame(Pane::Map));
    }

    #[test]
    fn boot_progress_fills_over_min_duration() {
        let mut s = AppState { booting: true, ..Default::default() };
        assert_eq!(s.boot_progress(), 0.0);
        for _ in 0..BOOT_MIN_FRAMES {
            s.boot_tick();
        }
        assert_eq!(s.boot_progress(), 1.0);
        // No probe yet, so despite the min duration it keeps waiting.
        assert!(s.booting);
    }

    #[test]
    fn boot_times_out_without_a_probe() {
        let mut s = AppState { booting: true, ..Default::default() };
        for _ in 0..BOOT_MAX_FRAMES {
            s.boot_tick();
        }
        assert!(!s.booting, "safety timeout ends the boot");
    }
}
