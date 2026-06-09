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
    PickItem {
        items: Vec<(String, String, bool)>,
        selection: usize,
    },
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
    pub scan_history: Vec<SectorObservation>,
    pub scan_history_idx: usize,
    pub scan_loading: bool,
    pub scan_mode: ScanMode,
    pub scan_error: Option<String>,
    pub scan_batch: Option<usize>,
    pub scan_detail_scroll: usize,
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
        if let Some((x, y, z)) = self.probe_sector_coords() {
            self.map.center_x = x;
            self.map.center_z = z;
            self.map.y_layer = y;
        }
        self.map.open = true;
    }

    // Move to y±1 while preserving cx+y+cz (no drift on round-trips).
    pub fn map_move_y(&mut self, dy: i32) {
        self.map.y_layer += dy;
        self.map.center_z -= dy;
    }

    pub fn build_jettison_items(&self) -> Vec<(String, String, bool)> {
        let Some(probe) = &self.probe else { return vec![] };
        let inv = &probe.inventory;
        let mut out: Vec<(String, String, bool)> = Vec::new();
        for stock in &inv.resource_stocks {
            if stock.amount > 0.0 {
                let label = format!("{} ({:.3} ECE)", stock.name, stock.amount);
                out.push((stock.id.clone(), label, false));
            }
        }
        for item in &inv.items {
            if item.item_type == "manny" {
                let in_probe = item.location.as_ref()
                    .map(|l| l.location_type == crate::api::types::MannyLocationType::Probe)
                    .unwrap_or(false);
                let idle = item.current_task.is_none();
                if in_probe && idle {
                    out.push((item.id.clone(), item.name.clone(), true));
                }
            }
        }
        out
    }

    pub fn update_inventory(&mut self, inv: ProbeInventory) {
        if let Some(ref mut probe) = self.probe {
            probe.inventory = inv;
        }
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
        }
    }

    pub fn set_recover_error(&mut self, msg: String) {
        if let RecoverInput::PickContainer { ref mut error, .. } = self.recover {
            *error = Some(msg);
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
