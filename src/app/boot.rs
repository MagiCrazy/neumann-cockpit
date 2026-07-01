//! Boot sequence for the cockpit interface.
//!
//! On startup the grid assembles centre-out (border trace), then each pane
//! fills from its own fetch as it returns. Driven by a short-lived animation
//! tick that only runs while `booting` — steady state stays event-driven.

use super::{AppState, Pane};

/// Frames (at the ~90 ms boot tick) between each pane's border tracing in.
const TRACE_STEP: u64 = 1;
/// Frame by which all nine panes have traced in (~0.8 s).
const TRACE_TOTAL: u64 = 9 * TRACE_STEP;
/// Safety cap: end the boot even if the initial probe fetch never returns.
const BOOT_MAX_FRAMES: u64 = 40; // ~3.6 s

/// Reveal order, centre-out: Probe (centre), then the axial neighbours, then
/// the corners.
const REVEAL: [Pane; 9] = [
    Pane::Probe,
    Pane::Map,
    Pane::Missions,
    Pane::Storage,
    Pane::Sector,
    Pane::Scanner,
    Pane::Comms,
    Pane::Mannies,
    Pane::Inventory,
];

/// Boot frame at which a pane's border traces in.
fn boot_reveal_frame(pane: Pane) -> u64 {
    REVEAL.iter().position(|p| *p == pane).unwrap_or(0) as u64 * TRACE_STEP
}

impl AppState {
    /// Advance the boot animation one frame. Ends the boot once the trace has
    /// played and the probe has loaded, or after the safety timeout.
    pub fn boot_tick(&mut self) {
        self.boot_frame = self.boot_frame.saturating_add(1);
        let trace_done = self.boot_frame >= TRACE_TOTAL;
        if (trace_done && self.probe.is_some()) || self.boot_frame >= BOOT_MAX_FRAMES {
            self.booting = false;
        }
    }

    /// Skip the boot screen (any key).
    pub fn skip_boot(&mut self) {
        self.booting = false;
    }

    /// Whether a pane's border has traced in yet this boot frame.
    pub fn boot_revealed(&self, pane: Pane) -> bool {
        self.boot_frame >= boot_reveal_frame(pane)
    }

    /// The pane tracing in on exactly this frame — the leading edge of the
    /// sweep, drawn with the bright accent.
    pub fn boot_leading(&self, pane: Pane) -> bool {
        self.boot_frame == boot_reveal_frame(pane)
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
    fn boot_waits_for_probe_after_trace() {
        let mut s = AppState { booting: true, ..Default::default() };
        // Play out the whole trace with no probe → still booting.
        for _ in 0..=TRACE_TOTAL {
            s.boot_tick();
        }
        assert!(s.booting, "trace done but no probe → keep waiting");
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
