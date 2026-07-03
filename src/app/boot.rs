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
/// Frame by which the whole self-check has typed out. Past this the boot is
/// "complete" and shows the "any key to continue" prompt.
const BOOT_SEQUENCE_END: u64 = 34;
/// Frame at which the boot auto-continues on its own (~2 s after the self-check
/// finishes, at the 90 ms boot tick), so a relaunch under tmux/ssh never sits
/// on the prompt forever. A keypress still continues immediately.
const BOOT_AUTO_CONTINUE: u64 = BOOT_SEQUENCE_END + 22;

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
    /// Advance the boot animation one frame. It plays the self-check, shows the
    /// "any key to continue" prompt, then auto-continues a couple seconds later
    /// (a keypress via `skip_boot` continues immediately).
    pub fn boot_tick(&mut self) {
        self.boot_frame = self.boot_frame.saturating_add(1);
        if self.boot_frame >= BOOT_AUTO_CONTINUE {
            self.booting = false;
        }
    }

    /// Leave the boot screen — either skipping the animation or continuing
    /// once it has finished. Any key triggers this.
    pub fn skip_boot(&mut self) {
        self.booting = false;
    }

    /// Whether the self-check has fully typed out (drives the "any key to
    /// continue" prompt).
    pub fn boot_complete(&self) -> bool {
        self.boot_frame >= BOOT_SEQUENCE_END
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
                ("MATRIX", s("OK")),
                ("REACTOR", s("NOMINAL")),
                ("SURGE DRIVE", s("ONLINE")),
                ("VR CORE", s("LOADED")),
                ("DEUTERIUM", s("SYNC")),
                ("GUPPI", s("READY")),
                ("CLOCK", s("2337")),
            ],
            Pane::Scanner => vec![
                ("SUDDAR ARRAY", s("6/6")),
                ("SUBSPACE PING", s("OK")),
                ("RANGE", s("MAX")),
                ("ARCHIVE", format!("{} SEC", self.scan_history.len())),
                ("RESOLUTION", s("OK")),
            ],
            Pane::Map => vec![
                ("STAR CHART", s("LOCKED")),
                ("SUDDAR MAP", s("OK")),
                ("WAYPOINTS", s("SYNC")),
                ("SYSTEMS", format!("{}", self.visited_sectors.len())),
                ("GRID", s("ALIGNED")),
            ],
            Pane::Comms => vec![
                (
                    "SCUT LINK",
                    self.api_version.map(|v| format!("v{v}")).unwrap_or_else(|| s("SYNC")),
                ),
                ("SUBSPACE NET", s("SCAN")),
                ("INBOX", s("SYNC")),
                ("RELAY", s("OK")),
                ("CRYPTO", s("OK")),
            ],
            Pane::Sector => vec![
                ("SUDDAR SWEEP", s("OK")),
                ("GRAV WELL", s("CLEAR")),
                ("HAZARDS", s("NONE")),
                ("CONTACTS", s("SCAN")),
                ("LOCK", s("OK")),
            ],
            Pane::Missions => vec![
                ("DIRECTIVES", s("SYNCED")),
                ("PROJECTS", s("OK")),
                ("STEPS", s("OK")),
                ("PRIORITY", s("SET")),
                ("LOG", s("OK")),
            ],
            Pane::Inventory => vec![
                ("AUTOFACTORY", s("IDLE")),
                ("FEEDSTOCK", s("SYNC")),
                ("CARGO", s("SYNC")),
                ("MATTER PRINTER", s("OK")),
                ("MANIFEST", s("SEALED")),
            ],
            Pane::Storage => vec![
                ("HOLDS", s("INDEXED")),
                ("BINS", s("SYNC")),
                ("ROUTING", s("OK")),
                ("CAPACITY", s("OK")),
                ("SEALS", s("OK")),
            ],
            Pane::Mannies => vec![
                ("MANNY BAY", s("UNLOCKED")),
                ("ROSTER", s("SYNC")),
                ("ROAMERS", s("OK")),
                ("INTERLOCKS", s("OK")),
                ("POWER", s("OK")),
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
    fn boot_completes_then_auto_continues() {
        let mut s = AppState { booting: true, ..Default::default() };
        assert!(!s.boot_complete());
        for _ in 0..BOOT_SEQUENCE_END {
            s.boot_tick();
        }
        assert!(s.boot_complete());
        // Just-completed: still holds on the "any key to continue" prompt.
        assert!(s.booting, "holds on the prompt right after the self-check");
        // ...then auto-continues on its own so a relaunch is never stuck.
        for _ in 0..(BOOT_AUTO_CONTINUE - BOOT_SEQUENCE_END + 1) {
            s.boot_tick();
        }
        assert!(!s.booting, "auto-continues shortly after completion");
    }

    #[test]
    fn any_key_leaves_the_boot() {
        let mut s = AppState { booting: true, ..Default::default() };
        s.skip_boot();
        assert!(!s.booting);
    }
}
