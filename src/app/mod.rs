mod boot;
mod color;
mod command;
mod containers;
mod grid;
mod inputs;
mod inventory;
mod mannies;
mod map;
mod message;
mod mode;
mod scan;
mod travel;
mod waypoints;
#[cfg(test)]
mod tests;

pub use boot::{BOOT_CHARS_PER_FRAME, BOOT_LINE_STRIDE};
pub use color::*;
pub use command::COMMANDS;
pub use grid::*;
pub use inputs::*;
pub use inventory::*;
pub use map::*;
pub use message::*;
pub use mode::*;
pub use scan::*;
pub use waypoints::*;

use crate::api::types::{
    ContainerInventory, CraftingRecipe, DamageWarningRule, Manny, Mission, Probe,
    ProbeAlert, ProbeInventory, ProbeMessage, ProbeSentMessage, ScutNetwork, SectorObservation,
    StorageContainer, VisitedSector,
};
use chrono::{DateTime, Local, Utc};
use tokio::time::Instant;

#[derive(Default)]
pub struct AppState {
    pub probe: Option<Probe>,
    pub mannies: Option<Vec<Manny>>,
    pub last_update: Option<DateTime<Local>>,
    /// When the last automatic (periodic) refresh was *fired*, as opposed to
    /// `last_update` (last *successful* sync). Lets the periodic refresh throttle
    /// by elapsed-since-attempt instead of firing every tick while the last sync
    /// stays stale during an outage.
    pub last_attempt: Option<DateTime<Local>>,
    /// Consecutive failed periodic refreshes, driving exponential backoff.
    pub consecutive_failures: u32,
    pub movement_arrival: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub loading: bool,
    pub quit: bool,
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
    /// Storage containers fetched on demand when the containers overlay opens.
    pub storage_containers: Vec<StorageContainer>,
    pub storage_container_detail: Option<(StorageContainer, ContainerInventory)>,
    /// Error from the last container-detail fetch, shown in the drill-in instead
    /// of a "fetching…" line that would otherwise hang forever.
    pub storage_container_detail_error: Option<String>,
    pub rename_container: RenameContainerInput,
    pub container_rules: ContainerRulesInput,
    pub storage_move: StorageMoveInput,
    pub drop_cargo: DropCargoInput,
    pub drop_container: DropStorageContainerInput,
    pub help_open: bool,
    /// Vertical scroll offset of the help overlay (rows). Reset when it closes.
    pub help_scroll: u16,
    /// Read-only detail popup for the selected inventory row.
    pub inventory_detail_open: bool,
    /// Transient success message shown in the status bar.
    pub toast: Option<(String, DateTime<Local>)>,
    /// Sectors already visited by the probe (server-side history).
    pub visited_sectors: Vec<VisitedSector>,
    pub travel: TravelInput,
    pub goto_visited: GotoVisitedInput,
    pub repair: RepairInput,
    pub mine: MineInput,
    pub remote_mine: RemoteMineInput,
    pub jettison: JettisonInput,
    pub fabrication: FabricationInput,
    pub salvage: SalvageInput,
    pub recall: RecallInput,
    pub refuel: RefuelInput,
    pub mind_snapshot: MindSnapshotInput,
    pub scut_relay: ScutRelayInput,
    pub scut_network: ScutNetworkInput,
    pub scut_network_view: Option<ScutNetwork>,
    pub missions: Vec<Mission>,
    pub missions_input: MissionsInput,
    pub messages: Vec<ProbeMessage>,
    pub sent_messages: Vec<ProbeSentMessage>,
    pub messages_input: MessagesInput,
    pub deploy: DeployInput,
    pub rename_manny: RenameMannyInput,
    pub inspect: InspectInput,
    pub recover: RecoverInput,
    pub detach: DetachInput,
    pub map: MapView,
    pub api_version: Option<u32>,
    pub recipes: Vec<CraftingRecipe>,
    // ── Cockpit v2 (bloc U1) ────────────────────────────────────────────
    /// Active pane in the 3×3 grid (defaults to `Probe`, the centre).
    pub active_pane: Pane,
    /// Whether the active pane is zoomed to full screen.
    pub zoomed: bool,
    /// Top-level interaction mode for the unified interface.
    pub mode: InputMode,
    /// Per-pane cursor + drill-in state, indexed by `Pane::index()`.
    pub pane_nav: [PaneNav; 9],
    /// Whether the contextual hints line is shown (config `hints`, F1 toggles).
    pub hints_visible: bool,
    /// Cockpit color mode (config `theme`, F2 cycles).
    pub color_mode: ColorMode,
    /// Boot sequence: true while the grid assembles on startup (see `boot.rs`).
    pub booting: bool,
    /// Frame counter for the boot trace, advanced by the boot tick.
    pub boot_frame: u64,
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
        // A successful probe sync clears the refresh backoff.
        self.consecutive_failures = 0;
        self.error = None;
        self.clamp_inventory_selection();
    }

    pub fn update_mannies(&mut self, mut mannies: Vec<Manny>) {
        // Stamp receipt time so the UI can interpolate task progress between
        // fetches (server sends a snapshot % + an estimated end time).
        let now = Utc::now();
        for m in &mut mannies {
            m.observed_at = Some(now);
        }
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

    /// Apply a rename/rules response: refresh the container in the list and the
    /// probe inventory in one shot.
    pub fn apply_container_update(&mut self, container: StorageContainer, inventory: ProbeInventory) {
        if let Some(c) = self.storage_containers.iter_mut().find(|c| c.id == container.id) {
            *c = container;
        }
        self.update_inventory(inventory);
    }

    pub fn set_rename_container_error(&mut self, msg: String) {
        if let RenameContainerInput::Typing { ref mut error, .. } = self.rename_container {
            *error = Some(msg);
        }
    }

    pub fn set_container_rules_error(&mut self, msg: String) {
        if let ContainerRulesInput::Editing { ref mut error, .. } = self.container_rules {
            *error = Some(msg);
        }
    }

    pub fn set_storage_move_error(&mut self, msg: String) {
        match &mut self.storage_move {
            StorageMoveInput::ConfigureResource { error, .. }
            | StorageMoveInput::ConfigureItem { error, .. } => *error = Some(msg),
            _ => {}
        }
    }

    pub fn set_drop_cargo_error(&mut self, msg: String) {
        if let DropCargoInput::Confirm { ref mut error, .. } = self.drop_cargo {
            *error = Some(msg);
        }
    }

    pub fn set_drop_container_error(&mut self, msg: String) {
        if let DropStorageContainerInput::PickPlanet { ref mut error, .. } = self.drop_container {
            *error = Some(msg);
        }
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

    pub fn probe_sector_coords(&self) -> Option<(i32, i32, i32)> {
        let rel = self.probe.as_ref()?.sector.as_ref()?.relative.as_ref()?;
        Some((rel.x.round() as i32, rel.y.round() as i32, rel.z.round() as i32))
    }

    /// Seconds since the last successful full sync (probe update), if any.
    /// `last_update` is reset on every `update_probe`, so any refresh — manual,
    /// event-driven, or periodic — restarts the clock.
    pub fn seconds_since_sync(&self) -> Option<i64> {
        self.last_update.map(|t| (Local::now() - t).num_seconds().max(0))
    }

    /// Interval the periodic refresh must respect: the normal 60 s cadence when
    /// healthy, otherwise exponential backoff (5→10→20→40→60 s) so a network
    /// outage does not trigger a request storm (7 spawns per tick).
    pub fn refresh_backoff_secs(&self) -> i64 {
        match self.consecutive_failures {
            0 => 60,
            n => (5_i64 << (n - 1).min(4)).min(60),
        }
    }

    /// Record that an automatic refresh was just fired.
    pub fn note_refresh_attempt(&mut self) {
        self.last_attempt = Some(Local::now());
    }

    /// Record a failed refresh (fatal probe error), growing the backoff.
    pub fn note_refresh_failure(&mut self) {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
    }

    /// Whether a periodic auto-refresh is due. Requires a prior successful sync
    /// (so a failed initial fetch does not spin-retry), the data to be stale
    /// (≥60 s), and — crucially — the backoff-adjusted interval to have elapsed
    /// since the last *attempt*, so consecutive failures back off instead of
    /// firing every 1 s tick while `last_update` stays stale.
    pub fn periodic_refresh_due(&self) -> bool {
        if self.loading || self.last_update.is_none() {
            return false;
        }
        if !matches!(self.seconds_since_sync(), Some(s) if s >= 60) {
            return false;
        }
        match self.last_attempt {
            None => true,
            Some(t) => (Local::now() - t).num_seconds().max(0) >= self.refresh_backoff_secs(),
        }
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
