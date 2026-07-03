use crate::api::types::{ScutRelayStatus, SectorObjectType, SectorObservation};
use chrono::Utc;
use super::*;

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
        ScanFilter::Minable => s.objects.iter().flatten().any(|o| {
            o.minable_targets.as_ref().is_some_and(|t| !t.is_empty())
        }),
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
}

impl AppState {
    pub fn update_sector(&mut self, mut sector: SectorObservation) {
        sector.scanned_at = Some(Utc::now());
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
        use crate::api::types::ResourceShares;
        let objs = self.current_sector()?.objects.as_ref()?;
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
                push(&mut out, ScannerObjectEntry {
                    id: id.clone(),
                    name: o.name.clone().unwrap_or_default(),
                    object_type: o.object_type.clone(),
                    provenance: ObjectProvenance::TopLevel,
                });
            }
            for t in o.minable_targets.iter().flatten() {
                push(&mut out, ScannerObjectEntry {
                    id: t.id.clone(),
                    name: t.name.clone().unwrap_or_default(),
                    object_type: t.object_type.clone(),
                    provenance: ObjectProvenance::MinableTarget,
                });
            }
            for t in &o.bookmark_targets {
                if matches!(t.object_type, SectorObjectType::Asteroid) {
                    push(&mut out, ScannerObjectEntry {
                        id: t.id.clone(),
                        name: t.name.clone().unwrap_or_default(),
                        object_type: t.object_type.clone(),
                        provenance: ObjectProvenance::BookmarkTarget,
                    });
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
        out
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
            }
            // An inactive relay (status != On) can be turned on or salvaged.
            (ObjectProvenance::TopLevel, SectorObjectType::ScutRelay)
                if self.sector_object_relay_status(&entry.id) != Some(ScutRelayStatus::On) =>
            {
                actions.push(ObjectAction::TurnOnRelay);
                actions.push(ObjectAction::Salvage);
            }
            _ => {}
        }
        if entry.provenance == ObjectProvenance::TopLevel
            && self.inventory_waypoint_bookmark_id().is_some()
        {
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

    /// Status of a SCUT relay object in the probe's current sector, by object id.
    pub fn sector_object_relay_status(&self, id: &str) -> Option<ScutRelayStatus> {
        self.probe_current_sector_scan()
            .and_then(|s| s.objects.as_ref())
            .and_then(|objects| {
                objects.iter()
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
        let current_pos = self.probe.as_ref()
            .and_then(|p| p.sector.as_ref())
            .and_then(|s| s.relative.as_ref())
            .map(|r| (r.x as i64, r.y as i64, r.z as i64));
        if let Some(pos) = current_pos {
            self.scan_history.iter().find(|s| {
                (s.relative_coordinates.x as i64, s.relative_coordinates.y as i64, s.relative_coordinates.z as i64) == pos
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
            if *n == 0 { self.scan_batch = None; }
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
