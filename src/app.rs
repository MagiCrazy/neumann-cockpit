use crate::api::types::{CraftingRecipe, Manny, Probe, ProbeInventory, ProbeMovement, SectorObject, SectorObjectType, SectorObservation};
use chrono::{DateTime, Local, Utc};
use std::path::Path;
use tokio::time::Instant;

#[derive(Default)]
pub enum ScanMode {
    #[default]
    Current,
    Input(String),
    DirectionPick,
}

#[derive(Default)]
pub enum RepairInput {
    #[default]
    Inactive,
    Typing {
        manny_id: String,
        manny_name: String,
        buf: String,
        error: Option<String>,
    },
}

#[derive(Default)]
pub enum TravelInput {
    #[default]
    Inactive,
    Typing(String),
    Confirming {
        x: i32,
        y: i32,
        z: i32,
        sector_distance: Option<i64>,
        fuel_cost: Option<f64>,
        eta_minutes: Option<i64>,
        error: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Panel {
    Probe,
    Inventory,
    Mannies,
    Scanner,
}

pub const RESOURCE_TYPES: [&str; 4] = ["deuterium", "metals", "ice", "carbon_compounds"];
pub const RESOURCE_LABELS: [&str; 4] = ["deuterium", "metals", "ice", "carbon"];
pub const DETACH_MODES: [(&str, &str); 2] = [
    ("drifting", "drifting — leave in sector"),
    ("hidden_on_asteroid", "hidden — attach to asteroid"),
];

/// Action applicable to a sector object from the scanner panel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjectAction {
    Mine,
    Inspect,
    Salvage,
    Recover,
    DeployWaypoint,
}

impl ObjectAction {
    pub fn label(&self) -> &'static str {
        match self {
            ObjectAction::Mine => "mine",
            ObjectAction::Inspect => "inspect",
            ObjectAction::Salvage => "salvage",
            ObjectAction::Recover => "recover",
            ObjectAction::DeployWaypoint => "deploy waypoint",
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

#[derive(Default)]
pub enum ObjectActionInput {
    #[default]
    Inactive,
    PickAction {
        object_id: String,
        object_name: String,
        actions: Vec<ObjectAction>,
        selection: usize,
    },
    PickManny {
        object_id: String,
        object_name: String,
        action: ObjectAction,
        mannies: Vec<(String, String)>,
        selection: usize,
    },
}

/// Category of a known destination shown in the waypoints overlay.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaypointKind {
    Bookmark,
    Star,
    Minable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WaypointEntry {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub distance: i64,
    pub label: String,
    pub kind: WaypointKind,
}

#[derive(Default)]
pub enum WaypointsInput {
    #[default]
    Inactive,
    Browsing {
        entries: Vec<WaypointEntry>,
        selection: usize,
    },
}

#[derive(Default)]
pub enum AtomicPrinterCraftInput {
    #[default]
    Inactive,
    PickRecipe {
        selection: usize,
        error: Option<String>,
    },
}

#[derive(Default)]
pub struct MapView {
    pub open: bool,
    pub center_x: i32,
    pub center_z: i32,
    pub y_layer: i32,
    /// Some(buffer) while typing target coordinates ([c] on the map).
    pub coord_input: Option<String>,
}

#[derive(Default)]
pub enum MineInput {
    #[default]
    Inactive,
    PickAsteroid {
        manny_id: String,
        manny_name: String,
        candidates: Vec<(String, String)>, // (object_id, display_name)
        selection: usize,
    },
    Configure {
        manny_id: String,
        manny_name: String,
        object_id: String,
        object_name: String,
        resources: [bool; 4], // deuterium, metals, ice, carbon_compounds
        amount_buf: String,
        amount_mode: bool, // false = toggling resources, true = editing amount
        error: Option<String>,
    },
}

#[derive(Default)]
pub enum CraftInput {
    #[default]
    Inactive,
    PickRecipe {
        manny_id: String,
        manny_name: String,
        selection: usize,
        error: Option<String>,
    },
}

#[derive(Default)]
pub enum SalvageInput {
    #[default]
    Inactive,
    PickTarget {
        manny_id: String,
        manny_name: String,
        candidates: Vec<(String, String)>,
        selection: usize,
    },
    Confirm {
        manny_id: String,
        manny_name: String,
        object_id: String,
        object_name: String,
        error: Option<String>,
    },
}

#[derive(Default)]
pub enum RecallInput {
    #[default]
    Inactive,
    Confirm {
        manny_id: String,
        manny_name: String,
        error: Option<String>,
    },
}

#[derive(Default)]
pub enum RenameMannyInput {
    #[default]
    Inactive,
    Typing {
        manny_id: String,
        manny_name: String,
        buf: String,
        error: Option<String>,
    },
}

#[derive(Default)]
pub enum DeployInput {
    #[default]
    Inactive,
    PickManny {
        mannies: Vec<(String, String)>,
        selection: usize,
    },
    PickObject {
        manny_id: String,
        candidates: Vec<(String, String)>,
        selection: usize,
    },
    EnterName {
        manny_id: String,
        object_id: String,
        object_name: String,
        name_buf: String,
        error: Option<String>,
    },
}

#[derive(Default)]
pub enum JettisonInput {
    #[default]
    Inactive,
    ConfirmManny {
        item_id: String,
        manny_name: String,
        error: Option<String>,
    },
    EnterAmount {
        item_id: String,
        item_name: String,
        max_amount: f64,
        buf: String,
        error: Option<String>,
    },
}

#[derive(Default)]
pub enum InspectInput {
    #[default]
    Inactive,
    PickAsteroid {
        manny_id: String,
        manny_name: String,
        candidates: Vec<(String, String)>,
        selection: usize,
        error: Option<String>,
    },
}

#[derive(Default)]
pub enum RecoverInput {
    #[default]
    Inactive,
    PickContainer {
        manny_id: String,
        manny_name: String,
        candidates: Vec<(String, String)>,
        selection: usize,
        error: Option<String>,
    },
}

#[derive(Default)]
pub enum DetachInput {
    #[default]
    Inactive,
    PickContainer {
        manny_id: String,
        manny_name: String,
        containers: Vec<(String, String)>,
        selection: usize,
    },
    PickMode {
        manny_id: String,
        manny_name: String,
        container_id: String,
        container_name: String,
        selection: usize,
        error: Option<String>,
    },
    PickAsteroid {
        manny_id: String,
        manny_name: String,
        container_id: String,
        container_name: String,
        asteroids: Vec<(String, String)>,
        selection: usize,
        error: Option<String>,
    },
}

pub enum ApiMessage {
    ProbeUpdated(Probe),
    ManniesUpdated(Vec<Manny>),
    SectorUpdated(SectorObservation),
    ScanError(String),
    MoveStarted(ProbeMovement),
    MoveError(String),
    RepairStarted,
    RepairError(String),
    MineStarted,
    MineError(String),
    VersionFetched(u32),
    JettisonDone(ProbeInventory),
    JettisonError(String),
    CraftStarted,
    CraftError(String),
    SalvageStarted,
    SalvageError(String),
    RecallStarted,
    RecallError(String),
    DeployStarted,
    DeployError(String),
    AtomicPrinterCraftStarted,
    AtomicPrinterCraftError(String),
    RecipesFetched(Vec<CraftingRecipe>),
    RenameMannyDone(Manny),
    RenameMannyError(String),
    InspectStarted,
    InspectError(String),
    RecoverStarted,
    RecoverError(String),
    DetachStarted,
    DetachError(String),
    Error(String),
}

/// Active items (manny, atomic printer) are listed individually in the
/// inventory panel; passive items are grouped by type.
pub fn is_active_item(item_type: &str) -> bool {
    matches!(item_type, "manny" | "atomic_3d_printer")
}

/// One navigable row of the inventory panel, in display order.
#[derive(Debug, Clone, PartialEq)]
pub enum InventoryRow {
    Stock { id: String },
    ActiveItem { id: String },
    PassiveGroup { item_type: String },
}

#[derive(Default)]
pub struct AppState {
    pub probe: Option<Probe>,
    pub mannies: Option<Vec<Manny>>,
    pub last_update: Option<DateTime<Local>>,
    pub movement_arrival: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub loading: bool,
    pub quit: bool,
    pub focused: Option<Panel>,
    pub mannies_selection: usize,
    pub inventory_selection: usize,
    pub scan_history: Vec<SectorObservation>,
    pub scan_history_idx: usize,
    pub scan_loading: bool,
    pub scan_mode: ScanMode,
    pub scan_error: Option<String>,
    pub scan_batch: Option<usize>,
    pub scan_detail_scroll: usize,
    /// Some(idx) when the scanner panel is in object-browsing mode.
    pub scanner_obj_selection: Option<usize>,
    pub object_action: ObjectActionInput,
    pub waypoints: WaypointsInput,
    pub help_open: bool,
    pub travel: TravelInput,
    pub repair: RepairInput,
    pub mine: MineInput,
    pub jettison: JettisonInput,
    pub craft: CraftInput,
    pub atomic_printer_craft: AtomicPrinterCraftInput,
    pub salvage: SalvageInput,
    pub recall: RecallInput,
    pub deploy: DeployInput,
    pub rename_manny: RenameMannyInput,
    pub inspect: InspectInput,
    pub recover: RecoverInput,
    pub detach: DetachInput,
    pub map: MapView,
    pub api_version: Option<u32>,
    pub recipes: Vec<CraftingRecipe>,
}

impl AppState {
    pub fn update_probe(&mut self, probe: Probe) {
        self.movement_arrival = probe
            .movement
            .as_ref()
            .map(|m| m.arrival_at)
            .filter(|&a| a > Utc::now());
        self.probe = Some(probe);
        self.last_update = Some(Local::now());
        self.error = None;
        self.clamp_inventory_selection();
    }

    fn clamp_inventory_selection(&mut self) {
        let count = self.inventory_rows().len();
        self.inventory_selection = if count == 0 {
            0
        } else {
            self.inventory_selection.min(count - 1)
        };
    }

    pub fn update_mannies(&mut self, mannies: Vec<Manny>) {
        // Clamp selection in case list shrank.
        if !mannies.is_empty() {
            self.mannies_selection = self.mannies_selection.min(mannies.len() - 1);
        } else {
            self.mannies_selection = 0;
        }
        self.mannies = Some(mannies);
    }

    pub fn set_error(&mut self, msg: String) {
        self.error = Some(msg);
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }

    pub fn set_quit(&mut self) {
        self.quit = true;
    }

    /// Toggle focus on a panel; pressing the same shortcut again unfocuses.
    pub fn toggle_focus(&mut self, panel: Panel) {
        if self.focused == Some(panel) {
            self.focused = None;
        } else {
            self.focused = Some(panel);
        }
    }

    pub fn manny_next(&mut self) {
        if let Some(mannies) = &self.mannies {
            if !mannies.is_empty() {
                self.mannies_selection = (self.mannies_selection + 1) % mannies.len();
            }
        }
    }

    pub fn manny_prev(&mut self) {
        if let Some(mannies) = &self.mannies {
            if !mannies.is_empty() {
                self.mannies_selection = self
                    .mannies_selection
                    .checked_sub(1)
                    .unwrap_or(mannies.len() - 1);
            }
        }
    }

    pub fn update_sector(&mut self, sector: SectorObservation) {
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
                    name: o.name.clone().unwrap_or_else(|| format!("{:?}", o.object_type).to_lowercase()),
                    object_type: o.object_type.clone(),
                    provenance: ObjectProvenance::TopLevel,
                });
            }
            for t in o.minable_targets.iter().flatten() {
                push(&mut out, ScannerObjectEntry {
                    id: t.id.clone(),
                    name: t.name.clone().unwrap_or_else(|| "unnamed".into()),
                    object_type: t.object_type.clone(),
                    provenance: ObjectProvenance::MinableTarget,
                });
            }
            for t in &o.bookmark_targets {
                if matches!(t.object_type, SectorObjectType::Asteroid) {
                    push(&mut out, ScannerObjectEntry {
                        id: t.id.clone(),
                        name: t.name.clone().unwrap_or_else(|| "unnamed".into()),
                        object_type: t.object_type.clone(),
                        provenance: ObjectProvenance::BookmarkTarget,
                    });
                }
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
            _ => {}
        }
        if entry.provenance == ObjectProvenance::TopLevel
            && self.inventory_waypoint_bookmark_id().is_some()
        {
            actions.push(ObjectAction::DeployWaypoint);
        }
        actions
    }

    /// Known destinations aggregated from scan history: deployed waypoint
    /// bookmarks first, then sectors with a star, then sectors with minable
    /// targets. One entry per (sector, category); bookmarks listed per name.
    pub fn collect_waypoints(&self) -> Vec<WaypointEntry> {
        let mut bookmarks: Vec<WaypointEntry> = Vec::new();
        let mut stars: Vec<WaypointEntry> = Vec::new();
        let mut minables: Vec<WaypointEntry> = Vec::new();

        for s in &self.scan_history {
            let (x, y, z) = (
                s.relative_coordinates.x.round() as i32,
                s.relative_coordinates.y.round() as i32,
                s.relative_coordinates.z.round() as i32,
            );
            let Some(objects) = &s.objects else { continue };

            for o in objects {
                let obj_name = o.name.clone().unwrap_or_else(|| "object".into());
                for wb in &o.waypoint_bookmarks {
                    bookmarks.push(WaypointEntry {
                        x, y, z,
                        distance: s.distance,
                        label: format!("{} @ {}", wb.name, obj_name),
                        kind: WaypointKind::Bookmark,
                    });
                }
                for t in &o.bookmark_targets {
                    let t_name = t.name.clone().unwrap_or_else(|| "object".into());
                    for wb in &t.waypoint_bookmarks {
                        bookmarks.push(WaypointEntry {
                            x, y, z,
                            distance: s.distance,
                            label: format!("{} @ {}", wb.name, t_name),
                            kind: WaypointKind::Bookmark,
                        });
                    }
                }
            }

            let has_star = objects.iter().any(|o| {
                matches!(o.object_type, SectorObjectType::Star | SectorObjectType::SolarSystem)
            });
            if has_star {
                stars.push(WaypointEntry {
                    x, y, z,
                    distance: s.distance,
                    label: "star".into(),
                    kind: WaypointKind::Star,
                });
            }

            let has_minable = objects.iter().any(|o| {
                o.minable_targets.as_ref().is_some_and(|t| !t.is_empty())
            });
            if has_minable {
                minables.push(WaypointEntry {
                    x, y, z,
                    distance: s.distance,
                    label: "minable resources".into(),
                    kind: WaypointKind::Minable,
                });
            }
        }

        stars.sort_by_key(|e| e.distance);
        minables.sort_by_key(|e| e.distance);
        let mut out = bookmarks;
        out.extend(stars);
        out.extend(minables);
        out
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

    fn probe_current_sector_scan(&self) -> Option<&SectorObservation> {
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

    pub fn scan_hist_next(&mut self) {
        if !self.scan_history.is_empty() {
            let new = (self.scan_history_idx + 1).min(self.scan_history.len() - 1);
            if new != self.scan_history_idx {
                self.scan_history_idx = new;
                self.scan_detail_scroll = 0;
            }
        }
    }

    pub fn scan_hist_prev(&mut self) {
        if self.scan_history_idx > 0 {
            self.scan_history_idx -= 1;
            self.scan_detail_scroll = 0;
        }
    }

    pub fn set_scan_error(&mut self, msg: String) {
        self.scan_error = Some(msg);
        self.scan_loading = false;
    }

    pub fn start_batch(&mut self, n: usize) {
        self.scan_batch = Some(n);
        self.scan_error = None;
    }

    pub fn batch_tick(&mut self) {
        if let Some(ref mut n) = self.scan_batch {
            *n = n.saturating_sub(1);
            if *n == 0 { self.scan_batch = None; }
        }
    }

    pub fn probe_sector_coords(&self) -> Option<(i32, i32, i32)> {
        let rel = self.probe.as_ref()?.sector.as_ref()?.relative.as_ref()?;
        Some((rel.x.round() as i32, rel.y.round() as i32, rel.z.round() as i32))
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

    pub fn next_refresh_instant(&self) -> Instant {
        match self.movement_arrival {
            Some(arrival) => {
                let remaining = (arrival - Utc::now())
                    .to_std()
                    .unwrap_or(std::time::Duration::ZERO);
                Instant::now() + remaining
            }
            None => Instant::now() + std::time::Duration::from_secs(86400),
        }
    }

    pub fn travel_type_char(&mut self, c: char) {
        if let TravelInput::Typing(ref mut buf) = self.travel {
            if c == '-' || c == ' ' || c.is_ascii_digit() {
                buf.push(c);
            }
        }
    }

    pub fn travel_backspace(&mut self) {
        if let TravelInput::Typing(ref mut buf) = self.travel {
            buf.pop();
        }
    }

    pub fn travel_submit(&mut self) {
        let (x, y, z) = {
            let TravelInput::Typing(ref buf) = self.travel else { return };
            let parts: Vec<&str> = buf.split_whitespace().collect();
            if parts.len() != 3 { return }
            let Ok(x) = parts[0].parse::<i32>() else { return };
            let Ok(y) = parts[1].parse::<i32>() else { return };
            let Ok(z) = parts[2].parse::<i32>() else { return };
            (x, y, z)
        };
        let error = if (x + y + z) % 2 != 0 {
            Some("x+y+z must be even".to_string())
        } else {
            None
        };
        let (sector_distance, fuel_cost, eta_minutes) = self.travel_preview(x, y, z);
        self.travel = TravelInput::Confirming { x, y, z, sector_distance, fuel_cost, eta_minutes, error };
    }

    pub fn travel_go_sector(&mut self, x: i32, y: i32, z: i32) {
        let (sector_distance, fuel_cost, eta_minutes) = self.travel_preview(x, y, z);
        self.travel = TravelInput::Confirming { x, y, z, sector_distance, fuel_cost, eta_minutes, error: None };
    }

    fn travel_preview(&self, x: i32, y: i32, z: i32) -> (Option<i64>, Option<f64>, Option<i64>) {
        let sector_distance = self.distance_to(x, y, z)
            .or_else(|| {
                self.scan_history.iter()
                    .find(|s| {
                        s.relative_coordinates.x as i32 == x
                            && s.relative_coordinates.y as i32 == y
                            && s.relative_coordinates.z as i32 == z
                    })
                    .map(|s| s.distance)
            });
        let fuel_cost = self.probe.as_ref()
            .and_then(|p| p.fuel.deuterium)
            .map(|d| (d * 0.02 * 10000.0).round() / 10000.0);
        let eta_minutes = sector_distance.map(|d| 5 + 35 * d);
        (sector_distance, fuel_cost, eta_minutes)
    }

    fn distance_to(&self, x: i32, y: i32, z: i32) -> Option<i64> {
        let pos = self.probe.as_ref()?.sector.as_ref()?.relative.as_ref()?;
        let dx = ((x as f64) - pos.x).abs().round() as i64;
        let dy = ((y as f64) - pos.y).abs().round() as i64;
        let dz = ((z as f64) - pos.z).abs().round() as i64;
        Some(dx.max(dy).max(dz))
    }

    pub fn set_travel_error(&mut self, msg: String) {
        if let TravelInput::Confirming { ref mut error, .. } = self.travel {
            *error = Some(format!("API: {msg}"));
        }
    }

    pub fn apply_movement(&mut self, mv: ProbeMovement) {
        self.movement_arrival = Some(mv.arrival_at).filter(|&a| a > Utc::now());
        if let Some(ref mut probe) = self.probe {
            probe.movement = Some(mv);
        }
        self.travel = TravelInput::Inactive;
    }

    pub fn load_scan_history(&mut self, path: &Path) {
        let Ok(data) = std::fs::read(path) else { return };
        if let Ok(history) = serde_json::from_slice::<Vec<SectorObservation>>(&data) {
            self.scan_history = history;
        }
    }

    pub fn seconds_until_refresh(&self) -> Option<i64> {
        self.movement_arrival
            .map(|a| (a - Utc::now()).num_seconds().max(0))
    }

    pub fn repair_max_percent(&self) -> f64 {
        self.probe.as_ref()
            .and_then(|p| p.systems.as_ref())
            .and_then(|s| s.integrity_percent)
            .map(|i| (100.0_f64 - i).max(0.0))
            .unwrap_or(0.0)
    }

    pub fn repair_metals_stock(&self) -> f64 {
        self.probe.as_ref()
            .map(|p| {
                p.inventory.resource_stocks.iter()
                    .find(|s| s.stock_type == "metals")
                    .map(|s| s.amount)
                    .unwrap_or(0.0)
            })
            .unwrap_or(0.0)
    }

    pub fn repair_type_char(&mut self, c: char) {
        if let RepairInput::Typing { ref mut buf, ref mut error, .. } = self.repair {
            if c.is_ascii_digit() || (c == '.' && !buf.contains('.')) {
                buf.push(c);
                *error = None;
            }
        }
    }

    pub fn repair_backspace(&mut self) {
        if let RepairInput::Typing { ref mut buf, .. } = self.repair {
            buf.pop();
        }
    }

    pub fn repair_fill_max(&mut self) {
        let max = self.repair_max_percent();
        if let RepairInput::Typing { ref mut buf, ref mut error, .. } = self.repair {
            *buf = format!("{:.2}", max);
            *error = None;
        }
    }

    pub fn set_repair_error(&mut self, msg: String) {
        if let RepairInput::Typing { ref mut error, .. } = self.repair {
            *error = Some(msg);
        }
    }

    pub fn mine_max_amount(&self) -> f64 {
        self.probe.as_ref()
            .map(|p| (p.inventory.free_capacity * 10000.0).round() / 10000.0)
            .unwrap_or(0.30)
            .max(0.0)
    }

    pub fn set_mine_error(&mut self, msg: String) {
        if let MineInput::Configure { ref mut error, .. } = self.mine {
            *error = Some(msg);
        }
    }

    pub fn open_map(&mut self) {
        self.map_recenter_on_probe();
        self.map.open = true;
    }

    pub fn map_recenter_on_probe(&mut self) {
        if let Some((x, y, z)) = self.probe_sector_coords() {
            self.map.center_x = x;
            self.map.center_z = z;
            self.map.y_layer = y;
        }
    }

    /// Chebyshev distance from the probe to the map center, when known.
    pub fn map_center_distance(&self) -> Option<i64> {
        let (px, py, pz) = self.probe_sector_coords()?;
        let dx = (self.map.center_x - px).abs() as i64;
        let dy = (self.map.y_layer - py).abs() as i64;
        let dz = (self.map.center_z - pz).abs() as i64;
        Some(dx.max(dy).max(dz))
    }

    // Move to y±1 while preserving cx+y+cz (no drift on round-trips).
    pub fn map_move_y(&mut self, dy: i32) {
        self.map.y_layer += dy;
        self.map.center_z -= dy;
    }

    /// Navigable rows of the inventory panel, in display order:
    /// resource stocks, then active items, then passive groups.
    pub fn inventory_rows(&self) -> Vec<InventoryRow> {
        let Some(probe) = &self.probe else { return vec![] };
        let inv = &probe.inventory;
        let mut out: Vec<InventoryRow> = Vec::new();
        for stock in &inv.resource_stocks {
            out.push(InventoryRow::Stock { id: stock.id.clone() });
        }
        for item in inv.items.iter().filter(|i| is_active_item(&i.item_type)) {
            out.push(InventoryRow::ActiveItem { id: item.id.clone() });
        }
        let mut seen: Vec<&str> = Vec::new();
        for item in inv.items.iter().filter(|i| !is_active_item(&i.item_type)) {
            if !seen.contains(&item.item_type.as_str()) {
                seen.push(&item.item_type);
                out.push(InventoryRow::PassiveGroup { item_type: item.item_type.clone() });
            }
        }
        out
    }

    pub fn selected_inventory_row(&self) -> Option<InventoryRow> {
        self.inventory_rows().into_iter().nth(self.inventory_selection)
    }

    pub fn inventory_next(&mut self) {
        let count = self.inventory_rows().len();
        if count > 0 {
            self.inventory_selection = (self.inventory_selection + 1) % count;
        }
    }

    pub fn inventory_prev(&mut self) {
        let count = self.inventory_rows().len();
        if count > 0 {
            self.inventory_selection = self
                .inventory_selection
                .checked_sub(1)
                .unwrap_or(count - 1);
        }
    }

    /// Build the jettison wizard state for the currently selected inventory row.
    pub fn jettison_for_selected(&self) -> Result<JettisonInput, String> {
        let Some(probe) = &self.probe else { return Err("no probe data".into()) };
        match self.selected_inventory_row() {
            Some(InventoryRow::Stock { id }) => {
                let stock = probe.inventory.resource_stocks.iter()
                    .find(|s| s.id == id)
                    .ok_or_else(|| "stock not found".to_string())?;
                if stock.amount <= 0.0 {
                    return Err(format!("{} stock is empty", stock.name));
                }
                Ok(JettisonInput::EnterAmount {
                    item_id: stock.id.clone(),
                    item_name: stock.name.clone(),
                    max_amount: stock.amount,
                    buf: String::new(),
                    error: None,
                })
            }
            Some(InventoryRow::ActiveItem { id }) => {
                let item = probe.inventory.items.iter()
                    .find(|i| i.id == id)
                    .ok_or_else(|| "item not found".to_string())?;
                if item.item_type != "manny" {
                    return Err("only resource stocks and mannies can be jettisoned".into());
                }
                let in_probe = item.location.as_ref()
                    .map(|l| l.location_type == crate::api::types::MannyLocationType::Probe)
                    .unwrap_or(false);
                if !in_probe {
                    return Err(format!("{} is not aboard the probe", item.name));
                }
                if item.current_task.is_some() {
                    return Err(format!("{} is busy", item.name));
                }
                Ok(JettisonInput::ConfirmManny {
                    item_id: item.id.clone(),
                    manny_name: item.name.clone(),
                    error: None,
                })
            }
            Some(InventoryRow::PassiveGroup { .. }) => {
                Err("only resource stocks and mannies can be jettisoned".into())
            }
            None => Err("inventory is empty".into()),
        }
    }

    pub fn update_inventory(&mut self, inv: ProbeInventory) {
        if let Some(ref mut probe) = self.probe {
            probe.inventory = inv;
        }
        self.clamp_inventory_selection();
    }

    pub fn jettison_type_char(&mut self, c: char) {
        if let JettisonInput::EnterAmount { ref mut buf, .. } = self.jettison {
            if c.is_ascii_digit() || (c == '.' && !buf.contains('.')) {
                buf.push(c);
            }
        }
    }

    pub fn jettison_backspace(&mut self) {
        if let JettisonInput::EnterAmount { ref mut buf, .. } = self.jettison {
            buf.pop();
        }
    }

    pub fn set_craft_error(&mut self, msg: String) {
        if let CraftInput::PickRecipe { ref mut error, .. } = self.craft {
            *error = Some(msg);
        }
    }

    pub fn set_atomic_printer_craft_error(&mut self, msg: String) {
        if let AtomicPrinterCraftInput::PickRecipe { ref mut error, .. } = self.atomic_printer_craft {
            *error = Some(msg);
        }
    }

    pub fn has_atomic_printer(&self) -> bool {
        self.probe.as_ref()
            .map(|p| p.inventory.items.iter().any(|i| i.item_type == "atomic_3d_printer"))
            .unwrap_or(false)
    }

    pub fn set_salvage_error(&mut self, msg: String) {
        if let SalvageInput::Confirm { ref mut error, .. } = self.salvage {
            *error = Some(msg);
        }
    }

    pub fn set_recall_error(&mut self, msg: String) {
        if let RecallInput::Confirm { ref mut error, .. } = self.recall {
            *error = Some(msg);
        }
    }

    pub fn collect_mineable_candidates(&self) -> Vec<(String, String)> {
        let sector = self.probe_current_sector_scan();
        sector
            .and_then(|s| s.objects.as_ref())
            .map(|objects| {
                objects.iter()
                    .flat_map(|o| o.minable_targets.iter().flatten())
                    .filter(|t| matches!(t.object_type, SectorObjectType::Asteroid))
                    .map(|t| (t.id.clone(), t.name.clone().unwrap_or_else(|| "unnamed".into())))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn collect_asteroid_candidates(&self) -> Vec<(String, String)> {
        let sector = self.probe_current_sector_scan();
        sector
            .and_then(|s| s.objects.as_ref())
            .map(|objects| {
                objects.iter()
                    .flat_map(|o| {
                        let direct = if matches!(o.object_type, SectorObjectType::Asteroid) {
                            o.id.as_ref().map(|id| vec![(id.clone(), o.name.clone().unwrap_or_else(|| "unnamed".into()))])
                                .unwrap_or_default()
                        } else { vec![] };
                        let nested: Vec<(String, String)> = o.bookmark_targets.iter()
                            .filter(|t| matches!(t.object_type, SectorObjectType::Asteroid))
                            .map(|t| (t.id.clone(), t.name.clone().unwrap_or_else(|| "unnamed".into())))
                            .collect();
                        [direct, nested].concat()
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn collect_salvage_candidates(&self) -> Vec<(String, String)> {
        self.probe_current_sector_scan()
            .and_then(|s| s.objects.as_ref())
            .map(|objects| {
                objects.iter()
                    .filter(|o| matches!(o.object_type, SectorObjectType::Manny))
                    .map(|o| {
                        let id = o.id.clone().unwrap_or_default();
                        let name = o.name.clone().unwrap_or_else(|| "unknown manny".into());
                        (id, name)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn set_jettison_error(&mut self, msg: String) {
        match self.jettison {
            JettisonInput::ConfirmManny { ref mut error, .. } => *error = Some(msg),
            JettisonInput::EnterAmount { ref mut error, .. } => *error = Some(msg),
            _ => {}
        }
    }

    pub fn set_deploy_error(&mut self, msg: String) {
        if let DeployInput::EnterName { ref mut error, .. } = self.deploy {
            *error = Some(msg);
        }
    }

    pub fn set_rename_manny_error(&mut self, msg: String) {
        if let RenameMannyInput::Typing { ref mut error, .. } = self.rename_manny {
            *error = Some(msg);
        }
    }

    pub fn rename_manny_type_char(&mut self, c: char) {
        if let RenameMannyInput::Typing { ref mut buf, .. } = self.rename_manny {
            if buf.len() < 40 {
                buf.push(c);
            }
        }
    }

    pub fn rename_manny_backspace(&mut self) {
        if let RenameMannyInput::Typing { ref mut buf, .. } = self.rename_manny {
            buf.pop();
        }
    }

    pub fn deploy_type_char(&mut self, c: char) {
        if let DeployInput::EnterName { ref mut name_buf, .. } = self.deploy {
            if name_buf.len() < 80 {
                name_buf.push(c);
            }
        }
    }

    pub fn deploy_backspace(&mut self) {
        if let DeployInput::EnterName { ref mut name_buf, .. } = self.deploy {
            name_buf.pop();
        }
    }

    pub fn collect_idle_onboard_mannies(&self) -> Vec<(String, String)> {
        self.mannies.as_ref()
            .map(|ms| {
                ms.iter()
                    .filter(|m| {
                        m.location.location_type == crate::api::types::MannyLocationType::Probe
                            && m.can_receive_orders
                    })
                    .map(|m| (m.id.clone(), m.name.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn collect_deploy_candidates(&self) -> Vec<(String, String)> {
        self.probe_current_sector_scan()
            .and_then(|s| s.objects.as_ref())
            .map(|objects| {
                objects.iter()
                    .filter(|o| o.id.is_some())
                    .map(|o: &SectorObject| {
                        let id = o.id.clone().unwrap();
                        let name = o.name.clone().unwrap_or_else(|| format!("{:?}", o.object_type).to_lowercase());
                        (id, name)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn set_inspect_error(&mut self, msg: String) {
        if let InspectInput::PickAsteroid { ref mut error, .. } = self.inspect {
            *error = Some(msg);
        } else {
            // Inspect was dispatched without the picker overlay (single
            // candidate or object-first flow) — surface in the status bar.
            self.error = Some(format!("inspect: {msg}"));
        }
    }

    pub fn set_recover_error(&mut self, msg: String) {
        if let RecoverInput::PickContainer { ref mut error, .. } = self.recover {
            *error = Some(msg);
        } else {
            self.error = Some(format!("recover: {msg}"));
        }
    }

    pub fn set_detach_error(&mut self, msg: String) {
        match self.detach {
            DetachInput::PickMode { ref mut error, .. } => *error = Some(msg),
            DetachInput::PickAsteroid { ref mut error, .. } => *error = Some(msg),
            _ => {}
        }
    }

    pub fn collect_detachable_containers(&self) -> Vec<(String, String)> {
        self.probe.as_ref()
            .map(|p| {
                p.inventory.containers.iter()
                    .filter(|c| c.kind != "probe")
                    .map(|c| (c.id.clone(), c.label.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn collect_detached_containers(&self) -> Vec<(String, String)> {
        self.probe_current_sector_scan()
            .and_then(|s| s.objects.as_ref())
            .map(|objects| {
                objects.iter()
                    .filter(|o| matches!(o.object_type, SectorObjectType::DetachedContainer))
                    .map(|o| {
                        let id = o.id.clone().unwrap_or_default();
                        let name = o.name.clone().unwrap_or_else(|| "unnamed container".into());
                        (id, name)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn atomic_printer_recipes(&self) -> Vec<&CraftingRecipe> {
        self.recipes.iter()
            .filter(|r| r.craftable_by.iter().any(|c| c == "atomic_3d_printer"))
            .collect()
    }

    pub fn manny_craft_recipes(&self) -> Vec<&CraftingRecipe> {
        self.recipes.iter()
            .filter(|r| r.craftable_by.iter().any(|c| c == "manny"))
            .collect()
    }

    pub fn inventory_waypoint_bookmark_id(&self) -> Option<String> {
        self.probe.as_ref()?.inventory.items.iter()
            .find(|i| i.item_type == "waypoint_bookmark")
            .map(|i| i.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sector(x: f64, y: f64, z: f64) -> SectorObservation {
        serde_json::from_str(&format!(r#"{{
            "relativeCoordinates": {{"x": {x}, "y": {y}, "z": {z}}},
            "distance": 1,
            "knowledgeLevel": "detailed",
            "confidence": 1.0,
            "objects": null,
            "probes": null,
            "possibleObjects": null,
            "estimatedObjects": null,
            "navigationalRisk": null,
            "message": null,
            "sensorMode": null,
            "dataFreshness": null,
            "scan": {{
                "currentSectorResidenceSeconds": 60,
                "requiredResidenceSeconds": 60,
                "scanQuality": 1.0
            }}
        }}"#)).unwrap()
    }

    fn make_probe(free_capacity: f64, sector_x: f64, sector_y: f64, sector_z: f64) -> Probe {
        serde_json::from_str(&format!(r#"{{
            "id": 1,
            "name": "test",
            "status": "idle",
            "fuel": {{"deuterium": 100.0}},
            "sensorMode": "normal",
            "sector": {{"relative": {{"x": {sector_x}, "y": {sector_y}, "z": {sector_z}}}}},
            "movement": null,
            "systems": null,
            "inventory": {{
                "capacity": 10.0,
                "usedCapacity": {used},
                "freeCapacity": {free_capacity},
                "items": [],
                "resourceStocks": [],
                "externalTanks": [],
                "containers": []
            }}
        }}"#, used = 10.0 - free_capacity)).unwrap()
    }

    // ── parse_scan_coords ─────────────────────────────────────────────────────

    #[test]
    fn parse_scan_coords_valid() {
        let mut state = AppState::default();
        state.scan_mode = ScanMode::Input("2 0 -2".into());
        assert_eq!(state.parse_scan_coords(), Some((2, 0, -2)));
    }

    #[test]
    fn parse_scan_coords_negative_values() {
        let mut state = AppState::default();
        state.scan_mode = ScanMode::Input("-4 2 -6".into());
        assert_eq!(state.parse_scan_coords(), Some((-4, 2, -6)));
    }

    #[test]
    fn parse_scan_coords_not_in_input_mode() {
        let state = AppState::default();
        assert_eq!(state.parse_scan_coords(), None);
    }

    #[test]
    fn parse_scan_coords_only_two_parts() {
        let mut state = AppState::default();
        state.scan_mode = ScanMode::Input("1 2".into());
        assert_eq!(state.parse_scan_coords(), None);
    }

    #[test]
    fn parse_scan_coords_non_numeric() {
        let mut state = AppState::default();
        state.scan_mode = ScanMode::Input("a b c".into());
        assert_eq!(state.parse_scan_coords(), None);
    }

    // ── probe_sector_coords ───────────────────────────────────────────────────

    #[test]
    fn probe_sector_coords_no_probe() {
        let state = AppState::default();
        assert_eq!(state.probe_sector_coords(), None);
    }

    #[test]
    fn probe_sector_coords_exact() {
        let mut state = AppState::default();
        state.probe = Some(make_probe(7.0, 2.0, 0.0, -2.0));
        assert_eq!(state.probe_sector_coords(), Some((2, 0, -2)));
    }

    #[test]
    fn probe_sector_coords_rounds_floats() {
        let mut state = AppState::default();
        state.probe = Some(make_probe(7.0, 2.7, 0.2, -2.8));
        assert_eq!(state.probe_sector_coords(), Some((3, 0, -3)));
    }

    // ── travel_submit ─────────────────────────────────────────────────────────

    #[test]
    fn travel_submit_even_sum_no_error() {
        let mut state = AppState::default();
        state.travel = TravelInput::Typing("2 0 -2".into());
        state.travel_submit();
        if let TravelInput::Confirming { x, y, z, error, .. } = &state.travel {
            assert_eq!((*x, *y, *z), (2, 0, -2));
            assert!(error.is_none(), "expected no error, got {error:?}");
        } else {
            panic!("expected Confirming variant");
        }
    }

    #[test]
    fn travel_submit_odd_sum_sets_error() {
        let mut state = AppState::default();
        state.travel = TravelInput::Typing("1 0 0".into());
        state.travel_submit();
        if let TravelInput::Confirming { error, .. } = &state.travel {
            assert!(error.is_some(), "expected parity error");
            assert!(error.as_ref().unwrap().contains("even"));
        } else {
            panic!("expected Confirming variant");
        }
    }

    #[test]
    fn travel_submit_not_typing_is_noop() {
        let mut state = AppState::default();
        state.travel_submit();
        assert!(matches!(state.travel, TravelInput::Inactive));
    }

    #[test]
    fn travel_submit_invalid_input_is_noop() {
        let mut state = AppState::default();
        state.travel = TravelInput::Typing("abc".into());
        state.travel_submit();
        assert!(matches!(state.travel, TravelInput::Typing(_)));
    }

    // ── scan_hist_next / scan_hist_prev ───────────────────────────────────────

    #[test]
    fn scan_hist_nav_empty_is_noop() {
        let mut state = AppState::default();
        state.scan_hist_next();
        assert_eq!(state.scan_history_idx, 0);
        state.scan_hist_prev();
        assert_eq!(state.scan_history_idx, 0);
    }

    #[test]
    fn scan_hist_next_advances_index() {
        let mut state = AppState::default();
        state.scan_history = vec![make_sector(0., 0., 0.), make_sector(2., 0., 0.), make_sector(4., 0., 0.)];
        state.scan_hist_next();
        assert_eq!(state.scan_history_idx, 1);
        state.scan_hist_next();
        assert_eq!(state.scan_history_idx, 2);
    }

    #[test]
    fn scan_hist_next_clamps_at_end() {
        let mut state = AppState::default();
        state.scan_history = vec![make_sector(0., 0., 0.), make_sector(2., 0., 0.)];
        state.scan_history_idx = 1;
        state.scan_hist_next();
        assert_eq!(state.scan_history_idx, 1);
    }

    #[test]
    fn scan_hist_prev_decrements_index() {
        let mut state = AppState::default();
        state.scan_history = vec![make_sector(0., 0., 0.), make_sector(2., 0., 0.)];
        state.scan_history_idx = 1;
        state.scan_hist_prev();
        assert_eq!(state.scan_history_idx, 0);
    }

    #[test]
    fn scan_hist_prev_clamps_at_zero() {
        let mut state = AppState::default();
        state.scan_history = vec![make_sector(0., 0., 0.)];
        state.scan_hist_prev();
        assert_eq!(state.scan_history_idx, 0);
    }

    // ── mine_max_amount ───────────────────────────────────────────────────────

    #[test]
    fn mine_max_amount_no_probe_returns_default() {
        let state = AppState::default();
        assert_eq!(state.mine_max_amount(), 0.30);
    }

    #[test]
    fn mine_max_amount_returns_free_capacity() {
        let mut state = AppState::default();
        state.probe = Some(make_probe(0.5, 0., 0., 0.));
        assert_eq!(state.mine_max_amount(), 0.5);
    }

    #[test]
    fn mine_max_amount_clamps_to_zero() {
        let mut state = AppState::default();
        state.probe = Some(make_probe(0.0, 0., 0., 0.));
        assert_eq!(state.mine_max_amount(), 0.0);
    }

    // ── inventory_rows / jettison_for_selected ────────────────────────────────

    fn probe_with_inventory(items_json: &str, stocks_json: &str) -> Probe {
        serde_json::from_str(&format!(r#"{{
            "id": 1, "name": "test", "status": "idle",
            "fuel": {{"deuterium": 100.0}}, "sensorMode": "normal",
            "sector": null, "movement": null, "systems": null,
            "inventory": {{
                "capacity": 10.0, "usedCapacity": 1.0, "freeCapacity": 9.0,
                "items": {items_json},
                "resourceStocks": {stocks_json},
                "externalTanks": [], "containers": []
            }}
        }}"#)).unwrap()
    }

    const STOCK_METALS: &str = r#"[{
        "id": "stock-metals", "type": "metals", "name": "Metals",
        "amount": 0.5, "containerSpace": 0.5, "containers": []
    }]"#;

    fn manny_item(task: Option<&str>) -> String {
        let task_json = task.map(|t| format!("\"{t}\"")).unwrap_or("null".into());
        format!(r#"{{
            "id": "manny-1", "type": "manny", "name": "Manny-1",
            "containerSpace": 1.0,
            "currentTask": {task_json},
            "taskProgressPercent": 0.0,
            "location": {{"type": "probe", "sector": null}},
            "cargo": null,
            "container": null
        }}"#)
    }

    #[test]
    fn inventory_rows_no_probe_is_empty() {
        let state = AppState::default();
        assert!(state.inventory_rows().is_empty());
        assert_eq!(state.selected_inventory_row(), None);
    }

    #[test]
    fn inventory_rows_order_stocks_active_passive() {
        let mut state = AppState::default();
        let items = format!(r#"[
            {{"id": "wb-1", "type": "waypoint_bookmark", "name": "Bookmark",
              "containerSpace": 0.1, "currentTask": null, "taskProgressPercent": 0.0,
              "location": null, "cargo": null, "container": null}},
            {{"id": "wb-2", "type": "waypoint_bookmark", "name": "Bookmark",
              "containerSpace": 0.1, "currentTask": null, "taskProgressPercent": 0.0,
              "location": null, "cargo": null, "container": null}},
            {}
        ]"#, manny_item(None));
        state.probe = Some(probe_with_inventory(&items, STOCK_METALS));
        let rows = state.inventory_rows();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], InventoryRow::Stock { id: "stock-metals".into() });
        assert_eq!(rows[1], InventoryRow::ActiveItem { id: "manny-1".into() });
        assert_eq!(rows[2], InventoryRow::PassiveGroup { item_type: "waypoint_bookmark".into() });
    }

    #[test]
    fn inventory_nav_wraps() {
        let mut state = AppState::default();
        state.probe = Some(probe_with_inventory(&format!("[{}]", manny_item(None)), STOCK_METALS));
        assert_eq!(state.inventory_rows().len(), 2);
        state.inventory_next();
        assert_eq!(state.inventory_selection, 1);
        state.inventory_next();
        assert_eq!(state.inventory_selection, 0);
        state.inventory_prev();
        assert_eq!(state.inventory_selection, 1);
    }

    #[test]
    fn jettison_for_selected_stock_enters_amount() {
        let mut state = AppState::default();
        state.probe = Some(probe_with_inventory("[]", STOCK_METALS));
        match state.jettison_for_selected() {
            Ok(JettisonInput::EnterAmount { item_id, item_name, max_amount, .. }) => {
                assert_eq!(item_id, "stock-metals");
                assert_eq!(item_name, "Metals");
                assert_eq!(max_amount, 0.5);
            }
            other => panic!("expected EnterAmount, got {:?}", std::mem::discriminant(&other.unwrap_or_default())),
        }
    }

    #[test]
    fn jettison_for_selected_idle_manny_confirms() {
        let mut state = AppState::default();
        state.probe = Some(probe_with_inventory(&format!("[{}]", manny_item(None)), "[]"));
        match state.jettison_for_selected() {
            Ok(JettisonInput::ConfirmManny { item_id, manny_name, .. }) => {
                assert_eq!(item_id, "manny-1");
                assert_eq!(manny_name, "Manny-1");
            }
            _ => panic!("expected ConfirmManny"),
        }
    }

    #[test]
    fn jettison_for_selected_busy_manny_errors() {
        let mut state = AppState::default();
        state.probe = Some(probe_with_inventory(&format!("[{}]", manny_item(Some("mining"))), "[]"));
        let err = state.jettison_for_selected().err().expect("busy manny should error");
        assert!(err.contains("busy"), "unexpected error: {err}");
    }

    #[test]
    fn jettison_for_selected_passive_group_errors() {
        let mut state = AppState::default();
        let items = r#"[{"id": "wb-1", "type": "waypoint_bookmark", "name": "Bookmark",
            "containerSpace": 0.1, "currentTask": null, "taskProgressPercent": 0.0,
            "location": null, "cargo": null, "container": null}]"#;
        state.probe = Some(probe_with_inventory(items, "[]"));
        assert!(state.jettison_for_selected().is_err());
    }

    #[test]
    fn update_probe_clamps_inventory_selection() {
        let mut state = AppState::default();
        state.inventory_selection = 5;
        state.update_probe(probe_with_inventory("[]", STOCK_METALS));
        assert_eq!(state.inventory_selection, 0);
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    fn make_manny(id: &str, location_type: &str, can_receive_orders: bool, task: Option<&str>) -> Manny {
        let task_json = match task {
            Some(t) => format!("\"{}\"", t),
            None => "null".into(),
        };
        serde_json::from_str(&format!(r#"{{
            "id": "{id}",
            "name": "{id}",
            "location": {{"type": "{location_type}", "sector": null}},
            "currentTask": {task_json},
            "taskProgressPercent": 0.0,
            "cargo": {{"capacity": 0.3, "deuterium": 0.0, "metals": 0.0, "ice": 0.0, "organicCompounds": 0.0}},
            "canReceiveOrders": {can_receive_orders},
            "taskEstimatedEndTime": null
        }}"#)).unwrap()
    }

    fn make_sector_with_objects(x: f64, y: f64, z: f64, objects_json: &str) -> SectorObservation {
        serde_json::from_str(&format!(r#"{{
            "relativeCoordinates": {{"x": {x}, "y": {y}, "z": {z}}},
            "distance": 1,
            "knowledgeLevel": "detailed",
            "confidence": 1.0,
            "objects": {objects_json},
            "probes": null,
            "possibleObjects": null,
            "estimatedObjects": null,
            "navigationalRisk": null,
            "message": null,
            "sensorMode": null,
            "dataFreshness": null,
            "scan": {{"currentSectorResidenceSeconds": 60, "requiredResidenceSeconds": 60, "scanQuality": 1.0}}
        }}"#)).unwrap()
    }

    fn probe_at(x: f64, y: f64, z: f64) -> Probe {
        make_probe(7.0, x, y, z)
    }

    // ── toggle_focus ──────────────────────────────────────────────────────────

    #[test]
    fn toggle_focus_sets_panel() {
        let mut state = AppState::default();
        state.toggle_focus(Panel::Probe);
        assert_eq!(state.focused, Some(Panel::Probe));
    }

    #[test]
    fn toggle_focus_same_panel_clears() {
        let mut state = AppState::default();
        state.toggle_focus(Panel::Scanner);
        state.toggle_focus(Panel::Scanner);
        assert_eq!(state.focused, None);
    }

    #[test]
    fn toggle_focus_different_panel_switches() {
        let mut state = AppState::default();
        state.toggle_focus(Panel::Probe);
        state.toggle_focus(Panel::Mannies);
        assert_eq!(state.focused, Some(Panel::Mannies));
    }

    // ── manny_next / manny_prev ───────────────────────────────────────────────

    #[test]
    fn manny_next_advances() {
        let mut state = AppState::default();
        state.mannies = Some(vec![
            make_manny("m1", "probe", true, None),
            make_manny("m2", "probe", true, None),
        ]);
        state.manny_next();
        assert_eq!(state.mannies_selection, 1);
    }

    #[test]
    fn manny_next_wraps() {
        let mut state = AppState::default();
        state.mannies = Some(vec![
            make_manny("m1", "probe", true, None),
            make_manny("m2", "probe", true, None),
        ]);
        state.mannies_selection = 1;
        state.manny_next();
        assert_eq!(state.mannies_selection, 0);
    }

    #[test]
    fn manny_prev_decrements() {
        let mut state = AppState::default();
        state.mannies = Some(vec![
            make_manny("m1", "probe", true, None),
            make_manny("m2", "probe", true, None),
        ]);
        state.mannies_selection = 1;
        state.manny_prev();
        assert_eq!(state.mannies_selection, 0);
    }

    #[test]
    fn manny_prev_wraps() {
        let mut state = AppState::default();
        state.mannies = Some(vec![
            make_manny("m1", "probe", true, None),
            make_manny("m2", "probe", true, None),
        ]);
        state.manny_prev();
        assert_eq!(state.mannies_selection, 1);
    }

    #[test]
    fn manny_nav_no_mannies_is_noop() {
        let mut state = AppState::default();
        state.manny_next();
        state.manny_prev();
        assert_eq!(state.mannies_selection, 0);
    }

    // ── repair_max_percent / repair_metals_stock ──────────────────────────────

    #[test]
    fn repair_max_percent_no_probe() {
        assert_eq!(AppState::default().repair_max_percent(), 0.0);
    }

    #[test]
    fn repair_max_percent_full_integrity() {
        let mut state = AppState::default();
        state.probe = Some(serde_json::from_str(r#"{
            "id": 1, "name": "t", "status": "idle",
            "fuel": {"deuterium": null}, "sensorMode": "normal",
            "sector": null, "movement": null,
            "systems": {"integrityPercent": 100.0, "damagePercent": 0.0,
                        "energyStored": null, "internalClockRate": null, "currentTask": null},
            "inventory": {"capacity": 1.0, "usedCapacity": 0.0, "freeCapacity": 1.0,
                          "items": [], "resourceStocks": [], "externalTanks": [], "containers": []}
        }"#).unwrap());
        assert_eq!(state.repair_max_percent(), 0.0);
    }

    #[test]
    fn repair_max_percent_damaged() {
        let mut state = AppState::default();
        state.probe = Some(serde_json::from_str(r#"{
            "id": 1, "name": "t", "status": "idle",
            "fuel": {"deuterium": null}, "sensorMode": "normal",
            "sector": null, "movement": null,
            "systems": {"integrityPercent": 60.0, "damagePercent": 40.0,
                        "energyStored": null, "internalClockRate": null, "currentTask": null},
            "inventory": {"capacity": 1.0, "usedCapacity": 0.0, "freeCapacity": 1.0,
                          "items": [], "resourceStocks": [], "externalTanks": [], "containers": []}
        }"#).unwrap());
        assert_eq!(state.repair_max_percent(), 40.0);
    }

    #[test]
    fn repair_metals_stock_no_probe() {
        assert_eq!(AppState::default().repair_metals_stock(), 0.0);
    }

    #[test]
    fn repair_metals_stock_returns_metals() {
        let mut state = AppState::default();
        state.probe = Some(serde_json::from_str(r#"{
            "id": 1, "name": "t", "status": "idle",
            "fuel": {"deuterium": null}, "sensorMode": "normal",
            "sector": null, "movement": null, "systems": null,
            "inventory": {"capacity": 1.0, "usedCapacity": 0.25, "freeCapacity": 0.75,
                "items": [], "externalTanks": [], "containers": [],
                "resourceStocks": [
                    {"id": "s-metals", "type": "metals", "name": "Metals", "amount": 0.25, "containerSpace": 0.25, "containers": []},
                    {"id": "s-ice", "type": "ice", "name": "Ice", "amount": 0.10, "containerSpace": 0.10, "containers": []}
                ]}
        }"#).unwrap());
        assert_eq!(state.repair_metals_stock(), 0.25);
    }

    // ── batch_tick ────────────────────────────────────────────────────────────

    #[test]
    fn batch_tick_decrements() {
        let mut state = AppState::default();
        state.scan_batch = Some(3);
        state.batch_tick();
        assert_eq!(state.scan_batch, Some(2));
    }

    #[test]
    fn batch_tick_clears_at_zero() {
        let mut state = AppState::default();
        state.scan_batch = Some(1);
        state.batch_tick();
        assert_eq!(state.scan_batch, None);
    }

    #[test]
    fn batch_tick_no_batch_is_noop() {
        let mut state = AppState::default();
        state.batch_tick();
        assert_eq!(state.scan_batch, None);
    }

    // ── update_sector (deduplication) ─────────────────────────────────────────

    #[test]
    fn update_sector_inserts_at_front() {
        let mut state = AppState::default();
        state.update_sector(make_sector(2., 0., -2.));
        assert_eq!(state.scan_history.len(), 1);
        assert_eq!(state.scan_history_idx, 0);
    }

    #[test]
    fn update_sector_deduplicates_by_coords() {
        let mut state = AppState::default();
        state.update_sector(make_sector(2., 0., -2.));
        state.update_sector(make_sector(4., 0., -4.));
        state.update_sector(make_sector(2., 0., -2.)); // duplicate
        assert_eq!(state.scan_history.len(), 2);
        // the re-scanned sector is now at the front
        assert_eq!(state.scan_history[0].relative_coordinates.x as i32, 2);
    }

    #[test]
    fn update_sector_resets_scroll_and_idx() {
        let mut state = AppState::default();
        state.update_sector(make_sector(0., 0., 0.));
        state.update_sector(make_sector(2., 0., 0.));
        state.scan_history_idx = 1;
        state.scan_detail_scroll = 5;
        state.update_sector(make_sector(4., 0., 0.));
        assert_eq!(state.scan_history_idx, 0);
        assert_eq!(state.scan_detail_scroll, 0);
    }

    // ── manny_craft_recipes / atomic_printer_recipes ──────────────────────────

    #[test]
    fn manny_craft_recipes_filters_correctly() {
        let mut state = AppState::default();
        state.recipes = vec![
            serde_json::from_str(r#"{"id":"r1","name":"R1","craftableBy":["manny"],
                "ingredients":[],"durationSeconds":60,
                "output":{"type":"x","name":"X","containerSpace":1.0,"containerSpaceUnit":"ECE","capacityBonus":null}}"#).unwrap(),
            serde_json::from_str(r#"{"id":"r2","name":"R2","craftableBy":["atomic_3d_printer"],
                "ingredients":[],"durationSeconds":60,
                "output":{"type":"x","name":"X","containerSpace":1.0,"containerSpaceUnit":"ECE","capacityBonus":null}}"#).unwrap(),
            serde_json::from_str(r#"{"id":"r3","name":"R3","craftableBy":["manny","atomic_3d_printer"],
                "ingredients":[],"durationSeconds":60,
                "output":{"type":"x","name":"X","containerSpace":1.0,"containerSpaceUnit":"ECE","capacityBonus":null}}"#).unwrap(),
        ];
        let manny = state.manny_craft_recipes();
        assert_eq!(manny.len(), 2);
        assert!(manny.iter().any(|r| r.id == "r1"));
        assert!(manny.iter().any(|r| r.id == "r3"));
    }

    #[test]
    fn atomic_printer_recipes_filters_correctly() {
        let mut state = AppState::default();
        state.recipes = vec![
            serde_json::from_str(r#"{"id":"r1","name":"R1","craftableBy":["manny"],
                "ingredients":[],"durationSeconds":60,
                "output":{"type":"x","name":"X","containerSpace":1.0,"containerSpaceUnit":"ECE","capacityBonus":null}}"#).unwrap(),
            serde_json::from_str(r#"{"id":"r2","name":"R2","craftableBy":["atomic_3d_printer"],
                "ingredients":[],"durationSeconds":60,
                "output":{"type":"x","name":"X","containerSpace":1.0,"containerSpaceUnit":"ECE","capacityBonus":null}}"#).unwrap(),
        ];
        let printer = state.atomic_printer_recipes();
        assert_eq!(printer.len(), 1);
        assert_eq!(printer[0].id, "r2");
    }

    // ── has_atomic_printer ────────────────────────────────────────────────────

    #[test]
    fn has_atomic_printer_no_probe() {
        assert!(!AppState::default().has_atomic_printer());
    }

    #[test]
    fn has_atomic_printer_absent() {
        let mut state = AppState::default();
        state.probe = Some(make_probe(7.0, 0., 0., 0.));
        assert!(!state.has_atomic_printer());
    }

    #[test]
    fn has_atomic_printer_present() {
        let mut state = AppState::default();
        state.probe = Some(serde_json::from_str(r#"{
            "id": 1, "name": "t", "status": "idle",
            "fuel": {"deuterium": null}, "sensorMode": "normal",
            "sector": null, "movement": null, "systems": null,
            "inventory": {"capacity": 10.0, "usedCapacity": 1.0, "freeCapacity": 9.0,
                "resourceStocks": [], "externalTanks": [], "containers": [],
                "items": [{"id": "ap-1", "type": "atomic_3d_printer", "name": "Atomic Printer",
                    "containerSpace": 1.0, "currentTask": null, "taskProgressPercent": 0.0,
                    "location": {"type": "probe", "sector": null}, "cargo": null, "container": null}]}
        }"#).unwrap());
        assert!(state.has_atomic_printer());
    }

    // ── collect_mineable_candidates ───────────────────────────────────────────

    #[test]
    fn collect_mineable_candidates_empty_when_no_sector() {
        assert!(AppState::default().collect_mineable_candidates().is_empty());
    }

    #[test]
    fn collect_mineable_candidates_returns_asteroid_targets() {
        let mut state = AppState::default();
        state.probe = Some(probe_at(2., 0., -2.));
        state.scan_history = vec![make_sector_with_objects(2., 0., -2., r#"[
            {
                "id": "planet-1", "type": "planet", "name": "P1",
                "estimated": null, "summary": null, "mass": null, "massUnit": null,
                "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
                "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
                "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
                "capacity": null, "capacityUnit": null,
                "minableTargets": [
                    {"id": "ast-1", "type": "asteroid", "name": "Rock A", "mass": null, "resourceTypes": ["metals"]},
                    {"id": "ast-2", "type": "asteroid", "name": "Rock B", "mass": null, "resourceTypes": null}
                ],
                "waypointBookmarks": [], "bookmarkTargets": []
            }
        ]"#)];
        let candidates = state.collect_mineable_candidates();
        assert_eq!(candidates.len(), 2);
        assert!(candidates.iter().any(|(id, _)| id == "ast-1"));
        assert!(candidates.iter().any(|(id, _)| id == "ast-2"));
    }

    #[test]
    fn collect_mineable_candidates_unnamed_fallback() {
        let mut state = AppState::default();
        state.probe = Some(probe_at(0., 0., 0.));
        state.scan_history = vec![make_sector_with_objects(0., 0., 0., r#"[
            {
                "id": "planet-1", "type": "planet", "name": null,
                "estimated": null, "summary": null, "mass": null, "massUnit": null,
                "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
                "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
                "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
                "capacity": null, "capacityUnit": null,
                "minableTargets": [{"id": "ast-x", "type": "asteroid", "name": null, "mass": null, "resourceTypes": null}],
                "waypointBookmarks": [], "bookmarkTargets": []
            }
        ]"#)];
        let candidates = state.collect_mineable_candidates();
        assert_eq!(candidates[0].1, "unnamed");
    }

    // ── collect_idle_onboard_mannies ──────────────────────────────────────────

    #[test]
    fn collect_idle_onboard_mannies_empty_when_no_mannies() {
        assert!(AppState::default().collect_idle_onboard_mannies().is_empty());
    }

    #[test]
    fn collect_idle_onboard_mannies_filters_correctly() {
        let mut state = AppState::default();
        state.mannies = Some(vec![
            make_manny("m1", "probe", true, None),           // included
            make_manny("m2", "probe", false, Some("mining")), // busy — excluded
            make_manny("m3", "sector", true, None),           // in sector — excluded
        ]);
        let result = state.collect_idle_onboard_mannies();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "m1");
    }

    // ── collect_detachable_containers ─────────────────────────────────────────

    #[test]
    fn collect_detachable_containers_empty_when_no_probe() {
        assert!(AppState::default().collect_detachable_containers().is_empty());
    }

    #[test]
    fn collect_detachable_containers_excludes_probe_container() {
        let mut state = AppState::default();
        state.probe = Some(serde_json::from_str(r#"{
            "id": 1, "name": "t", "status": "idle",
            "fuel": {"deuterium": null}, "sensorMode": "normal",
            "sector": null, "movement": null, "systems": null,
            "inventory": {"capacity": 10.0, "usedCapacity": 0.0, "freeCapacity": 10.0,
                "items": [], "resourceStocks": [], "externalTanks": [],
                "containers": [
                    {"id": "c-probe", "kind": "probe", "label": "Main Hold", "sortOrder": 0,
                     "capacity": 5.0, "usedCapacity": 0.0, "freeCapacity": 5.0,
                     "rules": {"priority": [], "exclusion": [], "strictExclusion": []}},
                    {"id": "c-ext", "kind": "external", "label": "Ext Container", "sortOrder": 1,
                     "capacity": 2.0, "usedCapacity": 0.0, "freeCapacity": 2.0,
                     "rules": {"priority": [], "exclusion": [], "strictExclusion": []}}
                ]}
        }"#).unwrap());
        let result = state.collect_detachable_containers();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "c-ext");
        assert_eq!(result[0].1, "Ext Container");
    }

    // ── collect_detached_containers ───────────────────────────────────────────

    #[test]
    fn collect_detached_containers_empty_when_no_scan() {
        assert!(AppState::default().collect_detached_containers().is_empty());
    }

    #[test]
    fn collect_detached_containers_returns_only_detached_type() {
        let mut state = AppState::default();
        state.probe = Some(probe_at(0., 0., 0.));
        state.scan_history = vec![make_sector_with_objects(0., 0., 0., r#"[
            {
                "id": "dc-1", "type": "detached_container", "name": "Floater",
                "estimated": null, "summary": null, "mass": null, "massUnit": null,
                "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
                "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
                "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
                "capacity": null, "capacityUnit": null, "minableTargets": null,
                "waypointBookmarks": [], "bookmarkTargets": []
            },
            {
                "id": "ast-1", "type": "asteroid", "name": "Rock",
                "estimated": null, "summary": null, "mass": null, "massUnit": null,
                "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
                "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
                "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
                "capacity": null, "capacityUnit": null, "minableTargets": null,
                "waypointBookmarks": [], "bookmarkTargets": []
            }
        ]"#)];
        let result = state.collect_detached_containers();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "dc-1");
        assert_eq!(result[0].1, "Floater");
    }

    #[test]
    fn collect_detached_containers_unnamed_fallback() {
        let mut state = AppState::default();
        state.probe = Some(probe_at(0., 0., 0.));
        state.scan_history = vec![make_sector_with_objects(0., 0., 0., r#"[
            {
                "id": "dc-2", "type": "detached_container", "name": null,
                "estimated": null, "summary": null, "mass": null, "massUnit": null,
                "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
                "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
                "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
                "capacity": null, "capacityUnit": null, "minableTargets": null,
                "waypointBookmarks": [], "bookmarkTargets": []
            }
        ]"#)];
        let result = state.collect_detached_containers();
        assert_eq!(result[0].1, "unnamed container");
    }

    // ── scanner_objects / actions_for_object ─────────────────────────────────

    const MIXED_OBJECTS: &str = r#"[
        {
            "id": "planet-1", "type": "planet", "name": "P1",
            "estimated": null, "summary": null, "mass": null, "massUnit": null,
            "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
            "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
            "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
            "capacity": null, "capacityUnit": null,
            "minableTargets": [
                {"id": "ast-1", "type": "asteroid", "name": "Rock A", "mass": null, "resourceTypes": ["metals"]}
            ],
            "waypointBookmarks": [], "bookmarkTargets": []
        },
        {
            "id": "wreck-1", "type": "manny", "name": "Lost Manny",
            "estimated": null, "summary": null, "mass": null, "massUnit": null,
            "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": true,
            "mannyState": "wreck", "mannyUid": null, "cargo": null, "itemType": null,
            "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
            "capacity": null, "capacityUnit": null, "minableTargets": null,
            "waypointBookmarks": [], "bookmarkTargets": []
        },
        {
            "id": "dc-1", "type": "detached_container", "name": "Floater",
            "estimated": null, "summary": null, "mass": null, "massUnit": null,
            "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
            "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
            "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
            "capacity": null, "capacityUnit": null, "minableTargets": null,
            "waypointBookmarks": [], "bookmarkTargets": []
        }
    ]"#;

    #[test]
    fn scanner_objects_empty_when_not_in_probe_sector() {
        let mut state = AppState::default();
        state.probe = Some(probe_at(4., 0., 0.));
        state.scan_history = vec![make_sector_with_objects(0., 0., 0., MIXED_OBJECTS)];
        assert!(!state.viewing_probe_sector());
        assert!(state.scanner_objects().is_empty());
    }

    #[test]
    fn scanner_objects_order_top_level_then_nested() {
        let mut state = AppState::default();
        state.probe = Some(probe_at(0., 0., 0.));
        state.scan_history = vec![make_sector_with_objects(0., 0., 0., MIXED_OBJECTS)];
        assert!(state.viewing_probe_sector());
        let entries = state.scanner_objects();
        let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["planet-1", "ast-1", "wreck-1", "dc-1"]);
        assert_eq!(entries[1].provenance, ObjectProvenance::MinableTarget);
        assert_eq!(entries[2].provenance, ObjectProvenance::TopLevel);
    }

    #[test]
    fn actions_for_object_by_kind() {
        let mut state = AppState::default();
        state.probe = Some(probe_at(0., 0., 0.));
        state.scan_history = vec![make_sector_with_objects(0., 0., 0., MIXED_OBJECTS)];
        let entries = state.scanner_objects();

        // planet (top-level, no bookmark in inventory): no actions
        assert!(state.actions_for_object(&entries[0]).is_empty());
        // minable asteroid: mine
        assert_eq!(state.actions_for_object(&entries[1]), vec![ObjectAction::Mine]);
        // manny wreck: salvage
        assert_eq!(state.actions_for_object(&entries[2]), vec![ObjectAction::Salvage]);
        // detached container: recover
        assert_eq!(state.actions_for_object(&entries[3]), vec![ObjectAction::Recover]);
    }

    #[test]
    fn actions_include_deploy_when_bookmark_in_inventory() {
        let mut state = AppState::default();
        let items = r#"[{"id": "wb-1", "type": "waypoint_bookmark", "name": "Bookmark",
            "containerSpace": 0.1, "currentTask": null, "taskProgressPercent": 0.0,
            "location": null, "cargo": null, "container": null}]"#;
        state.probe = Some(probe_with_inventory(items, "[]"));
        // probe_with_inventory has no sector — give it one at origin
        if let Some(ref mut p) = state.probe {
            p.sector = Some(serde_json::from_str(r#"{"relative": {"x": 0.0, "y": 0.0, "z": 0.0}}"#).unwrap());
        }
        state.scan_history = vec![make_sector_with_objects(0., 0., 0., MIXED_OBJECTS)];
        let entries = state.scanner_objects();
        // top-level planet gains deploy; nested minable target does not
        assert_eq!(state.actions_for_object(&entries[0]), vec![ObjectAction::DeployWaypoint]);
        assert_eq!(state.actions_for_object(&entries[1]), vec![ObjectAction::Mine]);
    }

    // ── collect_waypoints ─────────────────────────────────────────────────────

    #[test]
    fn collect_waypoints_empty_history() {
        assert!(AppState::default().collect_waypoints().is_empty());
    }

    #[test]
    fn collect_waypoints_bookmarks_first_then_sorted_by_distance() {
        let mut state = AppState::default();
        let star_far: SectorObservation = serde_json::from_str(r#"{
            "relativeCoordinates": {"x": 6.0, "y": 0.0, "z": 0.0},
            "distance": 6, "knowledgeLevel": "detailed", "confidence": 1.0,
            "objects": [{
                "id": "star-1", "type": "star", "name": "Sun",
                "estimated": null, "summary": null, "mass": null, "massUnit": null,
                "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
                "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
                "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
                "capacity": null, "capacityUnit": null, "minableTargets": null,
                "waypointBookmarks": [], "bookmarkTargets": []
            }],
            "probes": null, "possibleObjects": null, "estimatedObjects": null,
            "navigationalRisk": null, "message": null, "sensorMode": null, "dataFreshness": null,
            "scan": {"currentSectorResidenceSeconds": 0, "requiredResidenceSeconds": 0, "scanQuality": 1.0}
        }"#).unwrap();
        let bookmark_near: SectorObservation = serde_json::from_str(r#"{
            "relativeCoordinates": {"x": 2.0, "y": 0.0, "z": 0.0},
            "distance": 2, "knowledgeLevel": "detailed", "confidence": 1.0,
            "objects": [{
                "id": "planet-1", "type": "planet", "name": "P1",
                "estimated": null, "summary": null, "mass": null, "massUnit": null,
                "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
                "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
                "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
                "capacity": null, "capacityUnit": null,
                "minableTargets": [{"id": "ast-1", "type": "asteroid", "name": "Rock", "mass": null, "resourceTypes": null}],
                "waypointBookmarks": [{"name": "home", "playerId": 1, "playerName": "me", "createdAt": "2026-01-01T00:00:00Z"}],
                "bookmarkTargets": []
            }],
            "probes": null, "possibleObjects": null, "estimatedObjects": null,
            "navigationalRisk": null, "message": null, "sensorMode": null, "dataFreshness": null,
            "scan": {"currentSectorResidenceSeconds": 0, "requiredResidenceSeconds": 0, "scanQuality": 1.0}
        }"#).unwrap();
        // far star scanned first (more recent in history)
        state.scan_history = vec![star_far, bookmark_near];

        let entries = state.collect_waypoints();
        assert_eq!(entries.len(), 3);
        // bookmark first regardless of history order
        assert_eq!(entries[0].kind, WaypointKind::Bookmark);
        assert!(entries[0].label.contains("home"));
        assert_eq!((entries[0].x, entries[0].y, entries[0].z), (2, 0, 0));
        // then star, then minable
        assert_eq!(entries[1].kind, WaypointKind::Star);
        assert_eq!(entries[1].distance, 6);
        assert_eq!(entries[2].kind, WaypointKind::Minable);
        assert_eq!(entries[2].distance, 2);
    }

    #[test]
    fn scanner_obj_nav_wraps() {
        let mut state = AppState::default();
        state.probe = Some(probe_at(0., 0., 0.));
        state.scan_history = vec![make_sector_with_objects(0., 0., 0., MIXED_OBJECTS)];
        state.scanner_obj_selection = Some(0);
        state.scanner_obj_prev();
        assert_eq!(state.scanner_obj_selection, Some(3));
        state.scanner_obj_next();
        assert_eq!(state.scanner_obj_selection, Some(0));
    }
}
