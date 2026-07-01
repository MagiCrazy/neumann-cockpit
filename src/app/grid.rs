//! Pane grid model for the unified Cockpit v2 interface (bloc U1).
//!
//! These types back the 3×3 tiling dashboard. They are consumed by later
//! blocs (navigation, drill-in, rendering); U1 only establishes the state
//! that `AppState` carries, so most of it is not read yet.
#![allow(dead_code)]

/// The nine panes of the Cockpit v2 grid, laid out to match the
/// `e r t / d f g / c v b` navigation square (identical on AZERTY and
/// QWERTY). `Probe` is the centre — the `f` home key with the tactile bump.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Pane {
    Scanner,   // e
    Map,       // r
    Comms,     // t
    Sector,    // d
    #[default]
    Probe,     // f — centre of the square
    Missions,  // g
    Inventory, // c
    Storage,   // v
    Mannies,   // b
}

impl Pane {
    /// All panes in grid order (row-major: e r t / d f g / c v b).
    pub const ALL: [Pane; 9] = [
        Pane::Scanner,
        Pane::Map,
        Pane::Comms,
        Pane::Sector,
        Pane::Probe,
        Pane::Missions,
        Pane::Inventory,
        Pane::Storage,
        Pane::Mannies,
    ];

    /// Map a bare (lowercase, unmodified) navigation key to its pane.
    pub fn from_key(c: char) -> Option<Pane> {
        Some(match c {
            'e' => Pane::Scanner,
            'r' => Pane::Map,
            't' => Pane::Comms,
            'd' => Pane::Sector,
            'f' => Pane::Probe,
            'g' => Pane::Missions,
            'c' => Pane::Inventory,
            'v' => Pane::Storage,
            'b' => Pane::Mannies,
            _ => return None,
        })
    }

    /// The lowercase key that activates this pane (matched at input time).
    pub fn key(self) -> char {
        match self {
            Pane::Scanner => 'e',
            Pane::Map => 'r',
            Pane::Comms => 't',
            Pane::Sector => 'd',
            Pane::Probe => 'f',
            Pane::Missions => 'g',
            Pane::Inventory => 'c',
            Pane::Storage => 'v',
            Pane::Mannies => 'b',
        }
    }

    /// Uppercase key for display (keycaps, hints, menus).
    pub fn key_label(self) -> char {
        self.key().to_ascii_uppercase()
    }

    pub fn label(self) -> &'static str {
        match self {
            Pane::Scanner => "SCANNER",
            Pane::Map => "MAP",
            Pane::Comms => "COMMS",
            Pane::Sector => "SECTOR",
            Pane::Probe => "PROBE",
            Pane::Missions => "MISSIONS",
            Pane::Inventory => "INVENTORY",
            Pane::Storage => "STORAGE",
            Pane::Mannies => "MANNIES",
        }
    }

    /// `(row, col)` position in the 3×3 grid, both in `0..3`.
    pub fn grid_pos(self) -> (u8, u8) {
        match self {
            Pane::Scanner => (0, 0),
            Pane::Map => (0, 1),
            Pane::Comms => (0, 2),
            Pane::Sector => (1, 0),
            Pane::Probe => (1, 1),
            Pane::Missions => (1, 2),
            Pane::Inventory => (2, 0),
            Pane::Storage => (2, 1),
            Pane::Mannies => (2, 2),
        }
    }

    /// Stable index `0..9`, used to key `AppState::pane_nav`. Matches the
    /// order of [`Pane::ALL`].
    pub fn index(self) -> usize {
        let (row, col) = self.grid_pos();
        row as usize * 3 + col as usize
    }
}

/// A level pushed onto a pane's drill-in stack. Bloc U3 consumes these; U1
/// only establishes the type so the per-pane stack has a payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrillLevel {
    Container(String),
    Mission(String),
    ItemGroup(String),
    Manny(String),
    SectorObject(usize),
    MessageThread(String),
}

/// Per-pane navigation state: the cursor at the current level plus the
/// drill-in breadcrumb below the pane root.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaneNav {
    pub cursor: usize,
    pub drill: Vec<DrillLevel>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pane_is_probe_centre() {
        assert_eq!(Pane::default(), Pane::Probe);
        assert_eq!(Pane::Probe.grid_pos(), (1, 1));
        assert_eq!(Pane::Probe.key(), 'f');
    }

    #[test]
    fn key_round_trips_for_every_pane() {
        for pane in Pane::ALL {
            assert_eq!(Pane::from_key(pane.key()), Some(pane));
            assert_eq!(pane.key_label(), pane.key().to_ascii_uppercase());
        }
    }

    #[test]
    fn non_grid_key_maps_to_none() {
        for c in ['a', 'z', 'q', 'j', 'k', 'h', 'l', '1', ' '] {
            assert_eq!(Pane::from_key(c), None);
        }
    }

    #[test]
    fn index_is_unique_and_matches_all_order() {
        for (i, pane) in Pane::ALL.into_iter().enumerate() {
            assert_eq!(pane.index(), i, "{pane:?} index");
        }
    }

    #[test]
    fn grid_positions_are_distinct() {
        let mut seen = std::collections::HashSet::new();
        for pane in Pane::ALL {
            assert!(seen.insert(pane.grid_pos()), "duplicate pos for {pane:?}");
        }
        assert_eq!(seen.len(), 9);
    }
}
