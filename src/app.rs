use crate::api::types::{Manny, Probe, ProbeMovement, SectorObservation};
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

pub enum ApiMessage {
    ProbeUpdated(Probe),
    ManniesUpdated(Vec<Manny>),
    SectorUpdated(SectorObservation),
    ScanError(String),
    MoveStarted(ProbeMovement),
    MoveError(String),
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
    pub travel: TravelInput,
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
        self.scan_loading = false;
        self.scan_error = None;
        self.scan_mode = ScanMode::Current;
    }

    pub fn current_sector(&self) -> Option<&SectorObservation> {
        self.scan_history.get(self.scan_history_idx)
    }

    pub fn scan_hist_next(&mut self) {
        if !self.scan_history.is_empty() {
            self.scan_history_idx =
                (self.scan_history_idx + 1).min(self.scan_history.len() - 1);
        }
    }

    pub fn scan_hist_prev(&mut self) {
        if self.scan_history_idx > 0 {
            self.scan_history_idx -= 1;
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

    pub fn travel_go_sector(&mut self, x: i32, y: i32, z: i32, _dist_hint: Option<i64>) {
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
}
