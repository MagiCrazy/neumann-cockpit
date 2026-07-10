//! Pane grid model for the unified Cockpit v2 interface (bloc U1).
//!
//! These types back the 3×3 tiling dashboard. They are consumed by later
//! blocs (navigation, drill-in, rendering); U1 only establishes the state
//! that `AppState` carries, so most of it is not read yet.

/// Rows moved by one PageUp/PageDown in a pane list.
const PANE_PAGE: usize = 10;
/// Safety cap on the step-until-stable jump to top/bottom (guards against a
/// hypothetical wrapping cursor looping forever).
const PANE_JUMP_CAP: usize = 4096;

/// The nine panes of the Cockpit v2 grid, laid out to match the
/// `e r t / d f g / c v b` navigation square (identical on AZERTY and
/// QWERTY). `Probe` is the centre — the `f` home key with the tactile bump.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Pane {
    Scanner, // e
    Map,     // r
    Comms,   // t
    Sector,  // d
    #[default]
    Probe, // f — centre of the square
    Missions, // g
    Inventory, // c
    Storage, // v
    Mannies, // b
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
    /// Which category the Comms pane is drilled into.
    CommsCat(CommsCategory),
    /// Which category the Missions pane is drilled into (Missions / Ship's log).
    MissionsCat(MissionsCategory),
}

/// The three sub-lists of the Comms pane, selectable at its root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommsCategory {
    Messages,
    Alerts,
    Warnings,
}

impl CommsCategory {
    pub fn label(self) -> &'static str {
        match self {
            CommsCategory::Messages => "Messages",
            CommsCategory::Alerts => "Alerts",
            CommsCategory::Warnings => "Warnings",
        }
    }

    /// Root-row order in the Comms pane.
    pub const ALL: [CommsCategory; 3] = [CommsCategory::Messages, CommsCategory::Alerts, CommsCategory::Warnings];
}

/// The two sub-views of the Missions pane, selectable at its root: the active
/// missions list and the ship's log (parked here until it earns its own pane).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionsCategory {
    Missions,
    ShipsLog,
}

impl MissionsCategory {
    pub fn label(self) -> &'static str {
        match self {
            MissionsCategory::Missions => "Missions",
            MissionsCategory::ShipsLog => "Ship's log",
        }
    }

    /// Root-row order in the Missions pane.
    pub const ALL: [MissionsCategory; 2] = [MissionsCategory::Missions, MissionsCategory::ShipsLog];
}

/// Per-pane navigation state: the cursor at the current level plus the
/// drill-in breadcrumb below the pane root.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaneNav {
    pub cursor: usize,
    pub drill: Vec<DrillLevel>,
}

impl super::AppState {
    /// Switch the active pane to the sequential next/previous one, wrapping
    /// around (fallback to the grid keys for `Tab`/`Shift+Tab`).
    pub fn cycle_pane(&mut self, forward: bool) {
        let n = Pane::ALL.len();
        let cur = Pane::ALL.iter().position(|p| *p == self.active_pane).unwrap_or(0);
        let next = if forward { (cur + 1) % n } else { (cur + n - 1) % n };
        self.active_pane = Pane::ALL[next];
    }

    /// Number of selectable rows in a pane. Panes reusing the classic
    /// renderers (Inventory/Scanner/Mannies) keep their own cursors, so they
    /// report 0 here; only the promoted panes drive `pane_nav`.
    pub fn pane_item_count(&self, pane: Pane) -> usize {
        let drill = self.pane_nav[pane.index()].drill.last();
        match pane {
            // Root: three categories. Drilled into Alerts/Warnings: that list.
            // (Messages open their own overlay, so no in-pane list there.)
            Pane::Comms => match drill {
                None => CommsCategory::ALL.len(),
                Some(DrillLevel::CommsCat(CommsCategory::Alerts)) => self.alerts.len(),
                Some(DrillLevel::CommsCat(CommsCategory::Warnings)) => self.damage_warnings.len(),
                _ => 0,
            },
            Pane::Sector => self.scanner_objects().len(),
            // Root: two categories. Ship's log: the journal. Drilled into a
            // mission: its steps. Otherwise (Missions category): the list.
            Pane::Missions => match drill {
                None => MissionsCategory::ALL.len(),
                Some(DrillLevel::MissionsCat(MissionsCategory::ShipsLog)) => self.ship_log_entries().len(),
                Some(DrillLevel::Mission(id)) => {
                    self.missions.iter().find(|m| &m.id == id).map_or(0, |m| m.steps.len())
                }
                _ => self.missions.len(),
            },
            // Drilled into a container, the cursor is frozen (contents are
            // rendered inline, read-only).
            Pane::Storage => match drill {
                Some(DrillLevel::Container(_)) => 0,
                _ => self.probe.as_ref().map_or(0, |p| p.inventory.containers.len()),
            },
            _ => 0,
        }
    }

    /// Move the cursor down within the active pane, routing to the pane's
    /// backing cursor (classic panels keep their existing selection state).
    pub fn pane_cursor_down(&mut self) {
        match self.active_pane {
            Pane::Inventory => self.inventory_next(),
            Pane::Scanner => self.scan_hist_next(),
            // Frozen while viewing a single manny's detail (drilled in).
            Pane::Mannies if self.pane_nav[Pane::Mannies.index()].drill.is_empty() => self.manny_next(),
            Pane::Mannies => {}
            Pane::Probe | Pane::Map => {}
            pane => {
                let n = self.pane_item_count(pane);
                if n > 0 {
                    let nav = &mut self.pane_nav[pane.index()];
                    nav.cursor = (nav.cursor + 1).min(n - 1);
                }
            }
        }
    }

    /// Move the cursor up within the active pane.
    pub fn pane_cursor_up(&mut self) {
        match self.active_pane {
            Pane::Inventory => self.inventory_prev(),
            Pane::Scanner => self.scan_hist_prev(),
            Pane::Mannies if self.pane_nav[Pane::Mannies.index()].drill.is_empty() => self.manny_prev(),
            Pane::Mannies => {}
            Pane::Probe | Pane::Map => {}
            pane => {
                let nav = &mut self.pane_nav[pane.index()];
                nav.cursor = nav.cursor.saturating_sub(1);
            }
        }
    }

    /// The active pane's current cursor position, whichever backing field it
    /// uses — so paging/jumping can detect when it has reached a bound.
    fn pane_cursor_pos(&self) -> usize {
        match self.active_pane {
            Pane::Inventory => self.inventory_selection,
            Pane::Scanner => self.scan_history_idx,
            Pane::Mannies => self.mannies_selection,
            pane => self.pane_nav[pane.index()].cursor,
        }
    }

    /// Move the cursor a page (10 rows) down/up, reusing the per-pane routing.
    /// Useful on lists that grow over a session (scan history, messages).
    pub fn pane_cursor_page_down(&mut self) {
        for _ in 0..PANE_PAGE {
            self.pane_cursor_down();
        }
    }

    pub fn pane_cursor_page_up(&mut self) {
        for _ in 0..PANE_PAGE {
            self.pane_cursor_up();
        }
    }

    /// Jump to the first/last row: step until the cursor stops moving (capped so
    /// a wrapping cursor can never loop forever).
    pub fn pane_cursor_top(&mut self) {
        let mut last = self.pane_cursor_pos();
        for _ in 0..PANE_JUMP_CAP {
            self.pane_cursor_up();
            let now = self.pane_cursor_pos();
            if now == last {
                break;
            }
            last = now;
        }
    }

    pub fn pane_cursor_bottom(&mut self) {
        let mut last = self.pane_cursor_pos();
        for _ in 0..PANE_JUMP_CAP {
            self.pane_cursor_down();
            let now = self.pane_cursor_pos();
            if now == last {
                break;
            }
            last = now;
        }
    }

    /// Toggle full-screen zoom of the active pane.
    pub fn toggle_zoom(&mut self) {
        self.zoomed = !self.zoomed;
    }

    /// Descend into the selected element of the active pane (drill-in).
    /// U3 supports one level, for panes whose detail is already in state:
    /// Missions (→ steps) and Comms (→ message thread). Other panes are
    /// no-ops until their detail views land.
    pub fn pane_drill_in(&mut self) {
        let idx = self.active_pane.index();
        // Only one level deep for now.
        if !self.pane_nav[idx].drill.is_empty() {
            return;
        }
        let cursor = self.pane_nav[idx].cursor;
        let level = match self.active_pane {
            // Missions and Comms drive their own drill (categories) via
            // `missions_activate` / `comms_activate`.
            Pane::Storage => self
                .probe
                .as_ref()
                .and_then(|p| p.inventory.containers.get(cursor))
                .map(|c| DrillLevel::Container(c.id.clone())),
            // Mannies uses its own selection cursor, not `pane_nav.cursor`.
            Pane::Mannies => self
                .mannies
                .as_ref()
                .and_then(|v| v.get(self.mannies_selection))
                .map(|m| DrillLevel::Manny(m.id.clone())),
            _ => None,
        };
        if let Some(level) = level {
            let nav = &mut self.pane_nav[idx];
            nav.drill.push(level);
            nav.cursor = 0;
        }
    }

    /// The Comms category currently drilled into, if any.
    pub fn comms_drill(&self) -> Option<CommsCategory> {
        match self.pane_nav[Pane::Comms.index()].drill.last() {
            Some(DrillLevel::CommsCat(c)) => Some(*c),
            _ => None,
        }
    }

    /// Drill the Comms pane into a category (Alerts/Warnings render in-pane).
    pub fn comms_enter_category(&mut self, cat: CommsCategory) {
        let nav = &mut self.pane_nav[Pane::Comms.index()];
        nav.drill.clear();
        nav.drill.push(DrillLevel::CommsCat(cat));
        nav.cursor = 0;
    }

    /// The Missions category currently drilled into, if any — found anywhere in
    /// the stack, so it holds while drilled deeper into a mission's steps.
    pub fn missions_category(&self) -> Option<MissionsCategory> {
        self.pane_nav[Pane::Missions.index()]
            .drill
            .iter()
            .find_map(|l| match l {
                DrillLevel::MissionsCat(c) => Some(*c),
                _ => None,
            })
    }

    /// Enter a Missions category from the root (missions list / ship's log).
    pub fn missions_enter_category(&mut self, cat: MissionsCategory) {
        let nav = &mut self.pane_nav[Pane::Missions.index()];
        nav.drill.clear();
        nav.drill.push(DrillLevel::MissionsCat(cat));
        nav.cursor = 0;
    }

    /// Drill from the missions list into the selected mission's steps.
    pub fn missions_drill_into(&mut self, id: String) {
        let nav = &mut self.pane_nav[Pane::Missions.index()];
        nav.drill.push(DrillLevel::Mission(id));
        nav.cursor = 0;
    }

    /// Ascend one drill level in the active pane. Returns true if a level was
    /// popped (so callers can distinguish "went up" from "already at root").
    pub fn pane_drill_out(&mut self) -> bool {
        let idx = self.active_pane.index();
        let popped = self.pane_nav[idx].drill.pop().is_some();
        if popped {
            self.pane_nav[idx].cursor = 0;
        }
        popped
    }

    /// Contextual key hints for the active pane and drill level (bloc U4).
    /// Reflects only what is actionable now; actions (`Enter`) arrive in U5.
    pub fn pane_hints(&self) -> String {
        let pane = self.active_pane;
        let drilled = !self.pane_nav[pane.index()].drill.is_empty();
        let mut parts: Vec<&str> = Vec::new();
        if drilled {
            parts.push("h back");
        }
        if !matches!(pane, Pane::Probe | Pane::Map) {
            parts.push("jk move");
        }
        if !drilled && matches!(pane, Pane::Missions | Pane::Comms | Pane::Storage | Pane::Mannies) {
            parts.push("l open");
        }
        // Panes that expose actions on Enter (menu or reused overlay). Probe
        // only acts when a recovery alert is present.
        let probe_recovery = pane == Pane::Probe && self.probe_terminal_alert().is_some();
        if probe_recovery
            || matches!(
                pane,
                Pane::Mannies
                    | Pane::Inventory
                    | Pane::Missions
                    | Pane::Comms
                    | Pane::Storage
                    | Pane::Sector
                    | Pane::Scanner
                    | Pane::Map
            )
        {
            parts.push("Enter act");
        }
        parts.push("z zoom");
        parts.push("ertdfgcvb pane");
        parts.push("F1 hints");
        parts.push("? help");
        parts.join(" · ")
    }

    /// Breadcrumb segments for the active pane: `COCKPIT › PANE [› detail…]`.
    pub fn breadcrumb(&self) -> Vec<String> {
        let mut segs = vec!["COCKPIT".to_string(), self.active_pane.label().to_string()];
        for level in &self.pane_nav[self.active_pane.index()].drill {
            segs.push(match level {
                DrillLevel::Mission(id) => self
                    .missions
                    .iter()
                    .find(|m| &m.id == id)
                    .map_or_else(|| "mission".to_string(), |m| m.title.clone()),
                DrillLevel::CommsCat(cat) => cat.label().to_string(),
                DrillLevel::MissionsCat(cat) => cat.label().to_string(),
                DrillLevel::Container(id) => self
                    .storage_container(id)
                    .map_or_else(|| id.clone(), |c| c.label.clone()),
                DrillLevel::ItemGroup(g) => g.clone(),
                DrillLevel::Manny(id) => self
                    .mannies
                    .as_ref()
                    .and_then(|v| v.iter().find(|m| &m.id == id))
                    .map_or_else(|| "manny".to_string(), |m| m.name.clone()),
                DrillLevel::SectorObject(i) => format!("object {i}"),
            });
        }
        segs
    }
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

    #[test]
    fn cycle_pane_wraps_both_ways() {
        let mut s = crate::app::AppState::default();
        s.active_pane = Pane::Mannies; // last in ALL
        s.cycle_pane(true);
        assert_eq!(s.active_pane, Pane::Scanner); // wrapped to first
        s.cycle_pane(false);
        assert_eq!(s.active_pane, Pane::Mannies); // wrapped back
    }

    #[test]
    fn toggle_zoom_flips() {
        let mut s = crate::app::AppState::default();
        assert!(!s.zoomed);
        s.toggle_zoom();
        assert!(s.zoomed);
        s.toggle_zoom();
        assert!(!s.zoomed);
    }

    #[test]
    fn drill_out_at_root_is_noop() {
        let mut s = crate::app::AppState::default();
        s.active_pane = Pane::Missions;
        assert!(!s.pane_drill_out());
        assert_eq!(s.breadcrumb(), vec!["COCKPIT", "MISSIONS"]);
    }

    #[test]
    fn mission_drill_in_out_updates_breadcrumb() {
        use crate::api::types::{Mission, MissionStatus};
        let mut s = crate::app::AppState::default();
        s.missions = vec![Mission {
            id: "m1".into(),
            mission_type: "survey".into(),
            title: "Survey the rim".into(),
            description: None,
            status: MissionStatus::Active,
            steps: vec![],
        }];
        s.active_pane = Pane::Missions;

        // Root → enter the Missions category → drill into the mission's steps.
        s.missions_enter_category(crate::app::MissionsCategory::Missions);
        assert_eq!(s.breadcrumb(), vec!["COCKPIT", "MISSIONS", "Missions"]);
        s.missions_drill_into("m1".into());
        assert_eq!(
            s.breadcrumb(),
            vec!["COCKPIT", "MISSIONS", "Missions", "Survey the rim"]
        );
        // The category holds while drilled into a mission's steps.
        assert_eq!(s.missions_category(), Some(crate::app::MissionsCategory::Missions));

        // Drill out step by step: mission → category → root.
        assert!(s.pane_drill_out());
        assert_eq!(s.breadcrumb(), vec!["COCKPIT", "MISSIONS", "Missions"]);
        assert!(s.pane_drill_out());
        assert_eq!(s.breadcrumb(), vec!["COCKPIT", "MISSIONS"]);
    }

    #[test]
    fn missions_root_categories_and_ship_log_counts_journal() {
        use crate::app::{LogEvent, MissionsCategory};
        let mut s = crate::app::AppState::default();
        s.active_pane = Pane::Missions;
        // Root exposes the two categories.
        assert_eq!(s.pane_item_count(Pane::Missions), MissionsCategory::ALL.len());
        // The Ship's log category lists the journal entries.
        s.journal = vec![LogEvent::action("test", "a", None), LogEvent::action("test", "b", None)];
        s.missions_enter_category(MissionsCategory::ShipsLog);
        assert_eq!(s.missions_category(), Some(MissionsCategory::ShipsLog));
        assert_eq!(s.pane_item_count(Pane::Missions), 2);
        assert_eq!(s.breadcrumb(), vec!["COCKPIT", "MISSIONS", "Ship's log"]);
    }

    #[test]
    fn ship_log_merges_actions_and_server_alerts_newest_first() {
        use crate::app::LogEvent;
        let mut s = crate::app::AppState::default();
        // A local action stamped now (2026+).
        s.journal = vec![LogEvent::action("test", "did a thing", None)];
        // A server alert dated far in the past.
        let alert: crate::api::types::ProbeAlert = serde_json::from_str(
            r#"{"id": 1, "type": "anomaly_detected", "status": "unread",
                "message": "anomaly detected", "phase": "detection",
                "createdAt": "2000-01-01T00:00:00+00:00"}"#,
        )
        .unwrap();
        s.alerts = vec![alert];
        let entries = s.ship_log_entries();
        assert_eq!(entries.len(), 2, "action + alert merged");
        assert_eq!(entries[0].summary, "did a thing", "newest (now) first");
        assert_eq!(entries[1].summary, "anomaly detected");
        assert_eq!(entries[1].kind, crate::app::kind::ALERT, "alert tagged as server event");
    }

    #[test]
    fn pane_hints_are_contextual() {
        let mut s = crate::app::AppState::default();

        // Probe has no list → no movement hint. Help is always advertised.
        s.active_pane = Pane::Probe;
        let probe_h = s.pane_hints();
        assert!(!probe_h.contains("jk move"));
        assert!(probe_h.contains("? help"), "the help key is always shown");

        // Missions at root offers drill-in, not "back".
        s.active_pane = Pane::Missions;
        let h = s.pane_hints();
        assert!(h.contains("l open"));
        assert!(!h.contains("h back"));

        // Drilled in, it offers "back" and drops "open".
        s.pane_nav[Pane::Missions.index()]
            .drill
            .push(DrillLevel::Mission("m1".into()));
        let h = s.pane_hints();
        assert!(h.contains("h back"));
        assert!(!h.contains("l open"));
    }
}
