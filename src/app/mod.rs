mod anim;
mod inputs;
mod inventory;
mod mannies;
mod map;
mod message;
mod scan;
mod travel;
mod waypoints;
#[cfg(test)]
mod tests;

pub use anim::*;
pub use inputs::*;
pub use inventory::*;
pub use map::*;
pub use message::*;
pub use scan::*;
pub use waypoints::*;

use crate::api::types::{
    CraftingRecipe, DamageWarningRule, Manny, Probe, ProbeAlert, SectorObservation, VisitedSector,
};
use chrono::{DateTime, Local, Utc};
use tokio::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Panel {
    Probe,
    Inventory,
    Mannies,
    Scanner,
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
    /// Size of the batch in flight (for the progress gauge).
    pub scan_batch_total: usize,
    pub scan_detail_scroll: usize,
    pub scan_filter: ScanFilter,
    /// Some(idx) when the scanner panel is in object-browsing mode.
    pub scanner_obj_selection: Option<usize>,
    pub object_action: ObjectActionInput,
    pub waypoints: WaypointsInput,
    /// Persistent alerts and damage warnings (fetched in `fetch_all`).
    pub alerts: Vec<ProbeAlert>,
    pub damage_warnings: Vec<ProbeAlert>,
    pub damage_warning_rule: Option<DamageWarningRule>,
    pub alerts_input: AlertsInput,
    pub help_open: bool,
    /// Read-only detail popup for the selected inventory row.
    pub inventory_detail_open: bool,
    /// Transient success message shown in the status bar.
    pub toast: Option<(String, DateTime<Local>)>,
    /// Sectors already visited by the probe (server-side history).
    pub visited_sectors: Vec<VisitedSector>,
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
    pub ui_theme: UiTheme,
    pub phosphor: Phosphor,
    /// Render-tick animations for the retro theme (no I/O involved).
    pub animations_enabled: bool,
    pub anim: AnimState,
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

    /// Replace an acknowledged alert in place (matched by id).
    pub fn replace_alert(&mut self, alert: ProbeAlert) {
        if let Some(a) = self.alerts.iter_mut().find(|a| a.id == alert.id) {
            *a = alert;
        }
    }

    /// Replace an acknowledged damage warning in place (matched by id).
    pub fn replace_damage_warning(&mut self, warning: ProbeAlert) {
        if let Some(w) = self.damage_warnings.iter_mut().find(|w| w.id == warning.id) {
            *w = warning;
        }
    }

    /// Count of alerts still needing attention — drives the `[!]` badge.
    pub fn unread_alert_count(&self) -> usize {
        self.alerts
            .iter()
            .chain(self.damage_warnings.iter())
            .filter(|a| a.is_unread())
            .count()
    }

    pub fn set_toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Local::now()));
    }

    /// Toast message while fresh (< 5 s); expired toasts are not shown.
    pub fn active_toast(&self) -> Option<&str> {
        self.toast
            .as_ref()
            .filter(|(_, t)| (Local::now() - *t).num_seconds() < 5)
            .map(|(m, _)| m.as_str())
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

    /// Visual order for Tab cycling: top-left → top-right → bottom-left → bottom-right.
    const PANEL_ORDER: [Panel; 4] = [Panel::Probe, Panel::Inventory, Panel::Scanner, Panel::Mannies];

    pub fn focus_next_panel(&mut self) {
        self.focused = Some(match self.focused {
            None => Self::PANEL_ORDER[0],
            Some(p) => {
                let i = Self::PANEL_ORDER.iter().position(|&x| x == p).unwrap_or(0);
                Self::PANEL_ORDER[(i + 1) % Self::PANEL_ORDER.len()]
            }
        });
    }

    pub fn focus_prev_panel(&mut self) {
        self.focused = Some(match self.focused {
            None => Self::PANEL_ORDER[Self::PANEL_ORDER.len() - 1],
            Some(p) => {
                let i = Self::PANEL_ORDER.iter().position(|&x| x == p).unwrap_or(0);
                Self::PANEL_ORDER[(i + Self::PANEL_ORDER.len() - 1) % Self::PANEL_ORDER.len()]
            }
        });
    }

    pub fn probe_sector_coords(&self) -> Option<(i32, i32, i32)> {
        let rel = self.probe.as_ref()?.sector.as_ref()?.relative.as_ref()?;
        Some((rel.x.round() as i32, rel.y.round() as i32, rel.z.round() as i32))
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

    pub fn seconds_until_refresh(&self) -> Option<i64> {
        self.movement_arrival
            .map(|a| (a - Utc::now()).num_seconds().max(0))
    }
}
