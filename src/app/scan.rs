use super::*;
use crate::api::types::{DangerLevel, ScutRelayStatus, SectorObjectType, SectorObservation};
use chrono::Utc;

/// Reserves (presence flags + amounts, indexed as [`RESOURCE_TYPES`]) paired
/// with the danger level for a sector object — the inputs a pick-row label needs.
pub type ObjectPickInfo = (Option<([bool; 4], [f64; 4])>, Option<DangerLevel>);

/// Cyclic filter applied to the scan history list ([f] in the scanner).
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum ScanFilter {
    #[default]
    All,
    Objects,
    Minable,
    Danger,
}

impl ScanFilter {
    pub fn next(self) -> Self {
        match self {
            ScanFilter::All => ScanFilter::Objects,
            ScanFilter::Objects => ScanFilter::Minable,
            ScanFilter::Minable => ScanFilter::Danger,
            ScanFilter::Danger => ScanFilter::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ScanFilter::All => "all",
            ScanFilter::Objects => "objects",
            ScanFilter::Minable => "minable",
            ScanFilter::Danger => "danger",
        }
    }
}

pub fn sector_matches_filter(s: &SectorObservation, f: ScanFilter) -> bool {
    match f {
        ScanFilter::All => true,
        ScanFilter::Objects => s.objects.as_ref().is_some_and(|o| !o.is_empty()),
        ScanFilter::Minable => s
            .objects
            .iter()
            .flatten()
            .any(|o| o.minable_targets.as_ref().is_some_and(|t| !t.is_empty())),
        ScanFilter::Danger => {
            let observed = s.objects.iter().flatten().any(|o| {
                matches!(o.object_type, crate::api::types::SectorObjectType::BlackHole)
                    || matches!(o.danger_level, Some(crate::api::types::DangerLevel::Extreme))
            });
            let estimated = s.estimated_objects.as_ref().is_some_and(|e| {
                matches!(e.danger_estimate, Some(crate::api::types::DangerLevel::Extreme))
                    || e.black_hole_probability.unwrap_or(0.0) > 0.5
            });
            observed || estimated
        }
    }
}

/// Action applicable to a sector object from the scanner panel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjectAction {
    Mine,
    Inspect,
    Salvage,
    Recover,
    DeployWaypoint,
    TurnOnRelay,
    InstallTransitBeacon,
}

impl ObjectAction {
    pub fn label(&self) -> &'static str {
        match self {
            ObjectAction::Mine => "mine",
            ObjectAction::Inspect => "inspect",
            ObjectAction::Salvage => "salvage",
            ObjectAction::Recover => "recover",
            ObjectAction::DeployWaypoint => "deploy waypoint",
            ObjectAction::TurnOnRelay => "turn on relay",
            ObjectAction::InstallTransitBeacon => "install transit beacon",
        }
    }
}

/// Where a scanner object entry comes from; determines which actions apply
/// (mirrors the candidate sets of the manny-first flows).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjectProvenance {
    TopLevel,
    MinableTarget,
    BookmarkTarget,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScannerObjectEntry {
    pub id: String,
    pub name: String,
    pub object_type: SectorObjectType,
    pub provenance: ObjectProvenance,
    /// True for a detached container hidden on a listed asteroid: the Sector
    /// pane renders it indented under its host (set during hierarchy ordering).
    pub attached: bool,
}

impl AppState {
    pub fn update_sector(&mut self, mut sector: SectorObservation) {
        sector.scanned_at = Some(Utc::now());
        // Provenance: which probe produced this scan (fleet knowledge is
        // shared, so the history stays a single store — see store.rs).
        sector.observed_by = self.active_probe_id.or(self.default_probe_id);
        let key = (
            sector.relative_coordinates.x as i64,
            sector.relative_coordinates.y as i64,
            sector.relative_coordinates.z as i64,
        );
        self.scan_history.retain(|s| {
            let k = (
                s.relative_coordinates.x as i64,
                s.relative_coordinates.y as i64,
                s.relative_coordinates.z as i64,
            );
            k != key
        });
        self.scan_history.insert(0, sector);
        self.scan_history_idx = 0;
        self.scan_detail_scroll = 0;
        self.scan_loading = false;
        self.scan_error = None;
        self.scan_mode = ScanMode::Current;
        // Object list may have changed — leave browsing mode.
        self.scanner_obj_selection = None;
    }

    /// True when the displayed history entry is the sector the probe is in.
    pub fn viewing_probe_sector(&self) -> bool {
        match (self.current_sector(), self.probe_sector_coords()) {
            (Some(s), Some(pos)) => {
                (
                    s.relative_coordinates.x.round() as i32,
                    s.relative_coordinates.y.round() as i32,
                    s.relative_coordinates.z.round() as i32,
                ) == pos
            }
            _ => false,
        }
    }

    /// Actionable objects of the probe's current sector, in display order:
    /// for each top-level object, the object itself (if it has an id), then
    /// its minable targets, then its bookmark-target asteroids.
    /// Presence flags and remaining reserves (ECE) per mineable resource for a
    /// sector object, indexed as [`RESOURCE_TYPES`] (deuterium, metals, ice,
    /// carbon_compounds). Looks up a top-level object or a nested mining target
    /// by id. `None` when the object isn't in the current sector scan.
    pub fn minable_target_reserves(&self, object_id: &str) -> Option<([bool; 4], [f64; 4])> {
        Self::reserves_in(self.current_sector()?, object_id)
    }

    /// Same lookup as [`minable_target_reserves`] but against the probe's actual
    /// current sector rather than the displayed history entry — used by the mine
    /// wizard, which targets the probe's sector regardless of what's on screen.
    pub fn probe_minable_reserves(&self, object_id: &str) -> Option<([bool; 4], [f64; 4])> {
        Self::reserves_in(self.probe_current_sector_scan()?, object_id)
    }

    fn reserves_in(sector: &SectorObservation, object_id: &str) -> Option<([bool; 4], [f64; 4])> {
        use crate::api::types::ResourceShares;
        let objs = sector.objects.as_ref()?;
        let build = |types: &[String], amt: Option<&ResourceShares>| {
            let has = |t: &str| types.iter().any(|x| x == t);
            let flags = [has("deuterium"), has("metals"), has("ice"), has("carbon_compounds")];
            let res = amt
                .map(|a| [a.deuterium, a.metals, a.ice, a.carbon_compounds])
                .unwrap_or([0.0; 4]);
            (flags, res)
        };
        for o in objs {
            if o.id.as_deref() == Some(object_id) {
                return Some(build(&o.resource_types, o.resource_amounts.as_ref()));
            }
            if let Some(t) = o.minable_targets.iter().flatten().find(|t| t.id == object_id) {
                let types = t.resource_types.clone().unwrap_or_default();
                return Some(build(&types, t.resource_amounts.as_ref()));
            }
        }
        None
    }

    /// Danger level for an object in `sector`, matched by top-level id or by a
    /// nested mining-target id (which inherits its parent object's danger).
    /// `None` when the object is absent from the scan or carries no danger.
    fn danger_in(sector: &SectorObservation, object_id: &str) -> Option<DangerLevel> {
        let objs = sector.objects.as_ref()?;
        for o in objs {
            if o.id.as_deref() == Some(object_id) || o.minable_targets.iter().flatten().any(|t| t.id == object_id) {
                return o.danger_level.clone();
            }
        }
        None
    }

    /// Reserves + danger for an object id in the probe's current sector scan,
    /// used by the shared pick-row label. `(None, None)` when not scanned.
    pub fn probe_object_pick_info(&self, object_id: &str) -> ObjectPickInfo {
        match self.probe_current_sector_scan() {
            Some(s) => (Self::reserves_in(s, object_id), Self::danger_in(s, object_id)),
            None => (None, None),
        }
    }

    /// Same as [`probe_object_pick_info`] but for a specific sector by
    /// coordinates — used by the remote-mine picker (asteroid in a SCUT-reachable
    /// sector, not the probe's own).
    pub fn sector_object_pick_info(&self, x: i32, y: i32, z: i32, object_id: &str) -> ObjectPickInfo {
        match self.sector_observation_at(x, y, z) {
            Some(s) => (Self::reserves_in(s, object_id), Self::danger_in(s, object_id)),
            None => (None, None),
        }
    }

    /// Max sensible mining amount for the selected resources: the sum of their
    /// remaining reserves when known, else the probe's free cargo capacity.
    pub fn mine_reserve_max(&self, object_id: &str, resources: [bool; 4]) -> f64 {
        match self.minable_target_reserves(object_id) {
            Some((_, res)) => {
                let sum: f64 = res.iter().zip(resources).filter(|(_, sel)| *sel).map(|(r, _)| *r).sum();
                if sum > 0.0 {
                    (sum * 10000.0).round() / 10000.0
                } else {
                    self.mine_max_amount()
                }
            }
            None => self.mine_max_amount(),
        }
    }

    /// Whether a mining job's per-trip travel is deducted: true only when the
    /// destination container is hidden on the very asteroid being mined (the
    /// server charges miningTravelSeconds = 0). Mining to the probe or to a
    /// container elsewhere keeps normal travel.
    pub fn mining_travel_deducted(&self, asteroid_id: &str, container_id: &str) -> bool {
        self.current_sector()
            .and_then(|s| s.objects.as_ref())
            .is_some_and(|objs| {
                objs.iter().any(|o| {
                    o.id.as_deref() == Some(container_id)
                        && matches!(o.object_type, SectorObjectType::DetachedContainer)
                        && o.mode.as_deref() == Some("hidden_on_asteroid")
                        && o.target_object_id.as_deref() == Some(asteroid_id)
                })
            })
    }

    pub fn scanner_objects(&self) -> Vec<ScannerObjectEntry> {
        if !self.viewing_probe_sector() {
            return vec![];
        }
        let Some(objects) = self.current_sector().and_then(|s| s.objects.as_ref()) else {
            return vec![];
        };
        let mut out: Vec<ScannerObjectEntry> = Vec::new();
        let push = |out: &mut Vec<ScannerObjectEntry>, entry: ScannerObjectEntry| {
            if !out.iter().any(|e| e.id == entry.id) {
                out.push(entry);
            }
        };
        for o in objects {
            if let Some(id) = &o.id {
                push(
                    &mut out,
                    ScannerObjectEntry {
                        id: id.clone(),
                        name: o.name.clone().unwrap_or_default(),
                        object_type: o.object_type.clone(),
                        provenance: ObjectProvenance::TopLevel,
                        attached: false,
                    },
                );
            }
            for t in o.minable_targets.iter().flatten() {
                push(
                    &mut out,
                    ScannerObjectEntry {
                        id: t.id.clone(),
                        name: t.name.clone().unwrap_or_default(),
                        object_type: t.object_type.clone(),
                        provenance: ObjectProvenance::MinableTarget,
                        attached: false,
                    },
                );
            }
            for t in &o.bookmark_targets {
                if matches!(t.object_type, SectorObjectType::Asteroid) {
                    push(
                        &mut out,
                        ScannerObjectEntry {
                            id: t.id.clone(),
                            name: t.name.clone().unwrap_or_default(),
                            object_type: t.object_type.clone(),
                            provenance: ObjectProvenance::BookmarkTarget,
                            attached: false,
                        },
                    );
                }
            }
        }
        // Synthesize a stable "type #n" label for objects the API left unnamed,
        // numbered per type in scan order.
        let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for e in out.iter_mut() {
            let label = crate::ui::theme::object_type_label(&e.object_type);
            let n = counts.entry(label).or_insert(0);
            *n += 1;
            if e.name.trim().is_empty() {
                e.name = format!("{label} #{n}");
            }
        }

        // Reorder into an asteroid→hosted-container hierarchy: each host keeps
        // its scan order and is immediately followed by the detached containers
        // hidden on it; drifting containers (and any whose host isn't listed)
        // sink to the bottom. `attached` marks the ones the pane indents.
        let listed: std::collections::HashSet<String> = out.iter().map(|e| e.id.clone()).collect();
        let host_of = |id: &str| -> Option<String> {
            objects
                .iter()
                .find(|o| {
                    o.id.as_deref() == Some(id)
                        && matches!(o.object_type, SectorObjectType::DetachedContainer)
                        && o.mode.as_deref() == Some("hidden_on_asteroid")
                })
                .and_then(|o| o.target_object_id.clone())
        };
        let is_container = |id: &str| -> bool {
            objects
                .iter()
                .any(|o| o.id.as_deref() == Some(id) && matches!(o.object_type, SectorObjectType::DetachedContainer))
        };
        let mut hosts: Vec<ScannerObjectEntry> = Vec::new();
        let mut hosted: std::collections::HashMap<String, Vec<ScannerObjectEntry>> = std::collections::HashMap::new();
        let mut drifting: Vec<ScannerObjectEntry> = Vec::new();
        for mut e in out.into_iter() {
            match host_of(&e.id) {
                Some(h) if listed.contains(&h) => {
                    e.attached = true;
                    hosted.entry(h).or_default().push(e);
                }
                // Hidden on an asteroid that isn't in this sector list, or a
                // free-floating container: both go to the bottom.
                Some(_) => drifting.push(e),
                None if is_container(&e.id) => drifting.push(e),
                None => hosts.push(e),
            }
        }
        let mut ordered: Vec<ScannerObjectEntry> = Vec::with_capacity(listed.len());
        for h in hosts {
            let kids = hosted.remove(&h.id);
            ordered.push(h);
            if let Some(kids) = kids {
                ordered.extend(kids);
            }
        }
        // Safety net for hosted entries whose host slipped past the guard, then
        // the drifting containers, all pinned to the bottom.
        for (_, kids) in hosted {
            ordered.extend(kids);
        }
        ordered.extend(drifting);
        ordered
    }

    /// Actions available for an entry, mirroring the manny-first candidate
    /// sets (mine ← minable targets; inspect ← top-level + bookmark-target
    /// asteroids; salvage ← sector mannies; recover ← detached containers;
    /// deploy ← any top-level object, when a bookmark is in inventory).
    pub fn actions_for_object(&self, entry: &ScannerObjectEntry) -> Vec<ObjectAction> {
        let mut actions: Vec<ObjectAction> = Vec::new();
        match (entry.provenance, &entry.object_type) {
            (ObjectProvenance::MinableTarget, SectorObjectType::Asteroid) => {
                actions.push(ObjectAction::Mine);
            }
            (ObjectProvenance::TopLevel | ObjectProvenance::BookmarkTarget, SectorObjectType::Asteroid) => {
                actions.push(ObjectAction::Inspect);
            }
            (ObjectProvenance::TopLevel, SectorObjectType::Manny) => {
                actions.push(ObjectAction::Salvage);
            }
            (ObjectProvenance::TopLevel, SectorObjectType::DetachedContainer) => {
                actions.push(ObjectAction::Recover);
                actions.push(ObjectAction::Inspect);
            }
            (ObjectProvenance::TopLevel, SectorObjectType::DormantConstruct) => {
                actions.push(ObjectAction::Inspect);
            }
            // An inactive relay (status != On) can be turned on or salvaged.
            (ObjectProvenance::TopLevel, SectorObjectType::ScutRelay)
                if self.sector_object_relay_status(&entry.id) != Some(ScutRelayStatus::On) =>
            {
                actions.push(ObjectAction::TurnOnRelay);
                actions.push(ObjectAction::Salvage);
            }
            // An active relay without a transit beacon can be equipped (v96).
            (ObjectProvenance::TopLevel, SectorObjectType::ScutRelay)
                if !self.sector_object_is_transit_beacon(&entry.id) =>
            {
                actions.push(ObjectAction::InstallTransitBeacon);
            }
            _ => {}
        }
        if entry.provenance == ObjectProvenance::TopLevel && self.inventory_waypoint_bookmark_id().is_some() {
            actions.push(ObjectAction::DeployWaypoint);
        }
        actions
    }

    /// SCUT networks covering the probe's current sector, as (id, name) pairs.
    pub fn scut_coverage(&self) -> Vec<(i64, String)> {
        self.probe_current_sector_scan()
            .map(|s| s.scut_networks.iter().map(|n| (n.id, n.name.clone())).collect())
            .unwrap_or_default()
    }

    /// Whether a SCUT relay object in the current sector already carries a
    /// transit beacon (API v96), by object id.
    pub fn sector_object_is_transit_beacon(&self, id: &str) -> bool {
        self.probe_current_sector_scan()
            .and_then(|s| s.objects.as_ref())
            .and_then(|objects| objects.iter().find(|o| o.id.as_deref() == Some(id)))
            .and_then(|o| o.is_transit_beacon)
            .unwrap_or(false)
    }

    /// Status of a SCUT relay object in the probe's current sector, by object id.
    pub fn sector_object_relay_status(&self, id: &str) -> Option<ScutRelayStatus> {
        self.probe_current_sector_scan()
            .and_then(|s| s.objects.as_ref())
            .and_then(|objects| {
                objects
                    .iter()
                    .find(|o| o.id.as_deref() == Some(id))
                    .and_then(|o| o.status.clone())
            })
    }

    pub fn scanner_obj_next(&mut self) {
        let count = self.scanner_objects().len();
        if let (Some(sel), true) = (self.scanner_obj_selection, count > 0) {
            self.scanner_obj_selection = Some((sel + 1) % count);
        }
    }

    pub fn scanner_obj_prev(&mut self) {
        let count = self.scanner_objects().len();
        if let (Some(sel), true) = (self.scanner_obj_selection, count > 0) {
            self.scanner_obj_selection = Some(sel.checked_sub(1).unwrap_or(count - 1));
        }
    }

    pub fn current_sector(&self) -> Option<&SectorObservation> {
        self.scan_history.get(self.scan_history_idx)
    }

    /// Most recent observation of a sector at the given relative coordinates.
    pub fn sector_observation_at(&self, x: i32, y: i32, z: i32) -> Option<&SectorObservation> {
        self.scan_history.iter().rev().find(|s| {
            s.relative_coordinates.x as i32 == x
                && s.relative_coordinates.y as i32 == y
                && s.relative_coordinates.z as i32 == z
        })
    }

    pub(crate) fn probe_current_sector_scan(&self) -> Option<&SectorObservation> {
        let current_pos = self
            .probe
            .as_ref()
            .and_then(|p| p.sector.as_ref())
            .and_then(|s| s.relative.as_ref())
            .map(|r| (r.x as i64, r.y as i64, r.z as i64));
        if let Some(pos) = current_pos {
            self.scan_history.iter().find(|s| {
                (
                    s.relative_coordinates.x as i64,
                    s.relative_coordinates.y as i64,
                    s.relative_coordinates.z as i64,
                ) == pos
            })
        } else {
            self.scan_history.first()
        }
    }

    /// Indices into scan_history matching the active filter, in history order.
    pub fn filtered_history_indices(&self) -> Vec<usize> {
        self.scan_history
            .iter()
            .enumerate()
            .filter(|(_, s)| sector_matches_filter(s, self.scan_filter))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn cycle_scan_filter(&mut self) {
        self.set_scan_filter(self.scan_filter.next());
    }

    /// Set the scan filter and snap the history cursor onto the first entry it
    /// keeps visible.
    pub fn set_scan_filter(&mut self, filter: ScanFilter) {
        self.scan_filter = filter;
        let idxs = self.filtered_history_indices();
        if !idxs.contains(&self.scan_history_idx) {
            if let Some(&first) = idxs.first() {
                self.scan_history_idx = first;
                self.scan_detail_scroll = 0;
            }
        }
    }

    pub fn scan_hist_next(&mut self) {
        let idxs = self.filtered_history_indices();
        let Some(pos) = idxs.iter().position(|&i| i == self.scan_history_idx) else {
            if let Some(&first) = idxs.first() {
                self.scan_history_idx = first;
                self.scan_detail_scroll = 0;
            }
            return;
        };
        if pos + 1 < idxs.len() {
            self.scan_history_idx = idxs[pos + 1];
            self.scan_detail_scroll = 0;
        }
    }

    pub fn scan_hist_prev(&mut self) {
        let idxs = self.filtered_history_indices();
        let Some(pos) = idxs.iter().position(|&i| i == self.scan_history_idx) else {
            if let Some(&first) = idxs.first() {
                self.scan_history_idx = first;
                self.scan_detail_scroll = 0;
            }
            return;
        };
        if pos > 0 {
            self.scan_history_idx = idxs[pos - 1];
            self.scan_detail_scroll = 0;
        }
    }

    pub fn set_scan_error(&mut self, msg: String) {
        self.scan_error = Some(msg);
        self.scan_loading = false;
    }

    pub fn start_batch(&mut self, n: usize) {
        self.scan_batch = Some(n);
        self.scan_batch_total = n;
        self.scan_error = None;
    }

    pub fn batch_tick(&mut self) {
        if let Some(ref mut n) = self.scan_batch {
            *n = n.saturating_sub(1);
            if *n == 0 {
                self.scan_batch = None;
            }
        }
    }

    pub fn scan_type_char(&mut self, c: char) {
        if let ScanMode::Input(ref mut buf) = self.scan_mode {
            if c == '-' || c == ' ' || c.is_ascii_digit() {
                buf.push(c);
            }
        }
    }

    pub fn scan_backspace(&mut self) {
        if let ScanMode::Input(ref mut buf) = self.scan_mode {
            buf.pop();
        }
    }

    /// Parse "x y z" from the input buffer. Returns None if invalid.
    pub fn parse_scan_coords(&self) -> Option<(i32, i32, i32)> {
        if let ScanMode::Input(ref buf) = self.scan_mode {
            let parts: Vec<&str> = buf.split_whitespace().collect();
            if parts.len() == 3 {
                let x = parts[0].parse::<i32>().ok()?;
                let y = parts[1].parse::<i32>().ok()?;
                let z = parts[2].parse::<i32>().ok()?;
                return Some((x, y, z));
            }
        }
        None
    }
}
