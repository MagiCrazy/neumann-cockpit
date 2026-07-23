mod boot;
mod color;
mod command;
mod containers;
mod grid;
mod inputs;
mod inventory;
mod journal;
mod lexicon;
mod mannies;
mod map;
mod message;
mod mode;
mod queue;
mod scan;
mod script;
mod telemetry;
#[cfg(test)]
mod tests;
mod travel;
mod tree;
mod waypoints;

pub use boot::{BOOT_CHARS_PER_FRAME, BOOT_LINE_STRIDE};
pub use color::*;
pub use command::{command_usage, CommandFire, COMMANDS};
pub use grid::*;
pub use inputs::*;
pub use inventory::*;
pub use journal::*;
pub use map::*;
pub use message::*;
pub use mode::*;
pub use queue::*;
pub use scan::*;
pub use script::*;
pub use telemetry::*;
pub use tree::*;
pub use waypoints::*;

use crate::api::types::{
    ContainerInventory, CraftingRecipe, DamageWarningRule, Manny, Mission, Probe, ProbeAlert, ProbeImprovement,
    ProbeInventory, ProbeMessage, ProbeSentMessage, ProbeSummary, ScutNetwork, SectorObservation, StorageContainer,
    VisitedSector,
};
use chrono::{DateTime, Local, Utc};
use tokio::time::Instant;

/// A short label for a Manny task worth a desktop notification when it
/// completes, or `None` for quick/uninteresting tasks (moving cargo, returning,
/// waiting, inspecting…) that would only add noise (issue #203).
fn long_task_note(task: &crate::api::types::MannyTask) -> Option<&'static str> {
    use crate::api::types::MannyTask;
    Some(match task {
        MannyTask::Mining => "mining",
        MannyTask::Crafting | MannyTask::AssistingAtomicPrinter => "crafting",
        MannyTask::Repair => "a repair",
        MannyTask::Salvage => "salvaging",
        MannyTask::ImprovingProbe => "a probe upgrade",
        MannyTask::InstallingWaypointBookmark => "installing a waypoint",
        MannyTask::RefillingDeuteriumTank => "refueling",
        MannyTask::TurningOnScutRelay => "activating a relay",
        MannyTask::TransferringDeuteriumToProbe => "a deuterium transfer",
        MannyTask::TransferringToProbe => "transferring to a probe",
        MannyTask::InstallingScutTransitBeacon => "installing a transit beacon",
        MannyTask::AssemblingProbe => "assembling a probe",
        _ => return None,
    })
}

/// The follow-up refresh a successful action needs. Staged into
/// `AppState::pending_refetch` by `finish_action` and dispatched once by the
/// event loop (which owns the `ApiClient` + sender), mirroring how `pending_fire`
/// and `pending_journal` defer effects the state layer cannot spawn itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refetch {
    /// Full `fetch_all` — the action changed the probe, sector, or roster.
    All,
    /// Just the manny roster — the action only changed a manny's task.
    Mannies,
    /// The active-mission list.
    Missions,
    /// Both message boxes (inbox + sent).
    Messages,
}

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
    /// The SQLite writer raised a write failure (disk full, corruption…), so
    /// history is no longer being saved. Read from the writer's shared flag each
    /// tick and surfaced as a status-bar warning (issue #216).
    pub persistence_degraded: bool,
    pub loading: bool,
    pub quit: bool,
    pub mannies_selection: usize,
    pub inventory_selection: usize,
    pub scan_history: Vec<SectorObservation>,
    /// Ship's log — most-recent-first, the recent `store::JOURNAL_WINDOW` window
    /// (the `events` table keeps the full history for stats).
    pub journal: Vec<LogEvent>,
    /// Log entries staged by action handlers this tick, drained by the event
    /// loop to persist + prepend to `journal` (mirrors `pending_fire`).
    pub pending_journal: Vec<LogEvent>,
    /// Vital-ratio time series (fuel / integrity / cargo), chronological, capped
    /// at `store::TELEMETRY_WINDOW`; the zoomed Probe pane draws sparklines from
    /// the active probe's tail (issue #201).
    pub telemetry: Vec<TelemetrySample>,
    /// Telemetry samples staged by `update_probe`, drained by the event loop to
    /// persist (mirrors `pending_journal`); already appended to `telemetry`.
    pub pending_telemetry: Vec<TelemetrySample>,
    /// Desktop-notification messages staged when a long task completes (travel
    /// arrival, a Manny finishing a long task), drained by the event loop and
    /// emitted via `notify::desktop_notify` when `config.notifications` is on
    /// (issue #203).
    pub pending_notifications: Vec<String>,
    /// Monotonic seed cycling the naming-ceremony lexicon (see
    /// `next_name_suggestion`); bumped each time a rename wizard suggests a name.
    pub name_suggestion_seed: usize,
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
    /// Persistent alerts and damage warnings (fetched in `fetch_all`).
    pub alerts: Vec<ProbeAlert>,
    pub damage_warnings: Vec<ProbeAlert>,
    pub damage_warning_rule: Option<DamageWarningRule>,
    /// Storage containers fetched on demand when the containers overlay opens.
    pub storage_containers: Vec<StorageContainer>,
    pub storage_container_detail: Option<(StorageContainer, ContainerInventory)>,
    /// Error from the last container-detail fetch, shown in the drill-in instead
    /// of a "fetching…" line that would otherwise hang forever.
    pub storage_container_detail_error: Option<String>,
    pub help_open: bool,
    /// Vertical scroll offset of the help overlay (rows). Reset when it closes.
    pub help_scroll: u16,
    /// Read-only detail popup for the selected inventory row.
    pub inventory_detail_open: bool,
    /// Transient success message shown in the status bar.
    pub toast: Option<(String, DateTime<Local>)>,
    /// Sectors already visited by the probe (server-side history).
    pub visited_sectors: Vec<VisitedSector>,
    pub goto_visited: GotoVisitedInput,
    pub probe_switch: ProbeSwitchInput,
    pub scut_network_view: Option<ScutNetwork>,
    pub missions: Vec<Mission>,
    pub messages: Vec<ProbeMessage>,
    pub sent_messages: Vec<ProbeSentMessage>,
    pub map: MapView,
    /// The full-screen tech-tree browser (`:tree`, #200).
    pub tree: TreeView,
    /// The single modal wizard currently open (or `None`). Replaces the 31
    /// mutually-exclusive `*Input` fields — only one can be held at a time, so
    /// two wizards open at once is unrepresentable.
    pub active_wizard: ActiveWizard,
    pub api_version: Option<u32>,
    pub recipes: Vec<CraftingRecipe>,
    pub probe_improvements: Vec<ProbeImprovement>,
    // ── Multi-probe fleet (API v81) ─────────────────────────────────────
    /// The player's probes (`GET /api/probes`), refreshed every `fetch_all`.
    pub fleet: Vec<ProbeSummary>,
    /// Server-side default probe id (the one `/api/probe` targets).
    pub default_probe_id: Option<u64>,
    /// Probe the cockpit currently pilots. `None` = the default probe and the
    /// pre-v81 endpoints; `Some(id)` retargets every per-probe call to that
    /// probe. Set only by an explicit switch, never reset by a refresh.
    pub active_probe_id: Option<u64>,
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
    /// Ring of previously-run `:` command lines, most-recent last. Browsed with
    /// ↑/↓ while the command line is open (`app/command.rs`). Session-only.
    pub command_history: Vec<String>,
    /// A task a `:` command staged but cannot spawn itself (no client/sender in
    /// `run_command`); the input layer drains it after running the command.
    pub pending_fire: Option<CommandFire>,
    /// Follow-up refresh a completed action requested via `finish_action`,
    /// drained + dispatched by the event loop (which owns the client + sender).
    pub pending_refetch: Option<Refetch>,
    // ── Production queue (#197) ─────────────────────────────────────────
    /// The crafting queue: sequential steps, one running at a time. Auto-runs
    /// (drains as steps complete) unless paused.
    pub craft_queue: Vec<QueuedCraft>,
    /// Whether the executor is paused. Default `false` — the queue runs itself.
    pub queue_paused: bool,
    /// Crafts the executor wants spawned this tick (one per free lane); drained
    /// by the event loop (mirrors `pending_fire`, since the state layer owns no
    /// client/sender).
    pub queue_fire: Vec<CraftFire>,
    // ── Action scripting (#198) ─────────────────────────────────────────
    /// The action script: a linear sequence of heterogeneous steps run strictly
    /// one at a time. Session-only.
    pub script: Vec<ScriptStep>,
    /// Whether the script is running. Default `false` — unlike the auto-running
    /// crafting queue, a script is composed then explicitly run (`R`).
    pub script_running: bool,
    /// Resolved actions the executor wants spawned this tick; drained by the
    /// event loop (mirrors `queue_fire` / `pending_fire`).
    pub script_fire: Vec<ScriptAction>,
}

impl AppState {
    pub fn update_probe(&mut self, probe: Probe) {
        // A pending arrival that disappears this sync means the travel ended —
        // notify (issue #203). Captured before the deadline is recomputed.
        let was_traveling = self.movement_arrival.is_some();
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
        self.record_telemetry();
        if was_traveling && self.movement_arrival.is_none() {
            self.notify_travel_ended();
        }
    }

    /// Stage a desktop notification for a travel that just ended, reading the
    /// outcome (arrived / failed / destroyed) and destination from the probe's
    /// movement snapshot.
    fn notify_travel_ended(&mut self) {
        use crate::api::types::MovementPhase;
        let msg = match self.probe.as_ref().and_then(|p| p.movement.as_ref()) {
            Some(m) => match m.phase.as_ref().unwrap_or(&m.status) {
                MovementPhase::Failed => "Travel failed".to_string(),
                MovementPhase::Destroyed => "Probe destroyed in transit".to_string(),
                _ => format!(
                    "Probe arrived at ({}, {}, {})",
                    m.target.x as i64, m.target.y as i64, m.target.z as i64
                ),
            },
            None => "Probe arrived".to_string(),
        };
        self.pending_notifications.push(msg);
    }

    /// Sample the piloted probe's vital ratios into the telemetry series,
    /// dropping a sample identical to the last one for the same probe so an
    /// idle probe does not flood the series (issue #201). The kept sample is
    /// appended to the in-memory series (capped) and staged for persistence.
    fn record_telemetry(&mut self) {
        let Some(probe) = &self.probe else { return };
        let sample = TelemetrySample::from_probe(probe, self.active_probe_id);
        if self
            .telemetry
            .iter()
            .rev()
            .find(|s| s.probe_id == sample.probe_id)
            .is_some_and(|last| last.same_vitals(&sample))
        {
            return;
        }
        self.telemetry.push(sample.clone());
        let overflow = self.telemetry.len().saturating_sub(crate::store::TELEMETRY_WINDOW);
        if overflow > 0 {
            self.telemetry.drain(0..overflow);
        }
        self.pending_telemetry.push(sample);
    }

    /// Apply a fleet roster refresh. Records the roster and default id, but
    /// deliberately leaves `active_probe_id` alone so a periodic refresh never
    /// yanks the pilot back to the default — the active probe changes only via
    /// an explicit switch (`set_active_probe`).
    pub fn update_fleet(&mut self, list: crate::api::types::ProbeListResponse) {
        self.default_probe_id = list.default_probe_id;
        self.fleet = list.probes;
        // v94: a piloted (non-default) probe can be destroyed/trapped and removed
        // from the roster server-side. If the active probe is gone, revert to the
        // default so the client stops targeting a dead `{probeId}` (the event loop
        // reconciles the ApiClient from `active_probe_id`). The `probe_destroyed`
        // alert already surfaces the loss in the Comms feed.
        if let Some(active) = self.active_probe_id {
            if !self.fleet.iter().any(|p| p.id == active) {
                self.active_probe_id = None;
                self.set_toast("active probe lost — reverted to default");
            }
        }
    }

    /// The probe the cockpit is piloting, if it is present in the roster.
    pub fn active_probe_summary(&self) -> Option<&ProbeSummary> {
        let target = self.active_probe_id.or(self.default_probe_id)?;
        self.fleet.iter().find(|p| p.id == target)
    }

    /// `(id, name)` of the piloted probe — from the roster, falling back to the
    /// full probe struct so rename works even before the fleet has loaded.
    pub fn active_probe_identity(&self) -> Option<(u64, String)> {
        if let Some(s) = self.active_probe_summary() {
            return Some((s.id, s.name.clone()));
        }
        self.probe.as_ref().map(|p| (p.id as u64, p.name.clone()))
    }

    /// Switch the piloted probe to `id`. Records the new active id (and clears
    /// it back to `None` when `id` is the server default, so the client falls
    /// back to the pre-v81 `/api/probe` paths). The event loop reconciles the
    /// `ApiClient` and refetches. Returns whether the active probe changed.
    pub fn set_active_probe(&mut self, id: u64) -> bool {
        let new = (Some(id) != self.default_probe_id).then_some(id);
        if new == self.active_probe_id {
            return false;
        }
        self.active_probe_id = new;
        true
    }

    pub fn update_mannies(&mut self, mut mannies: Vec<Manny>) {
        // Stamp receipt time so the UI can interpolate task progress between
        // fetches (server sends a snapshot % + an estimated end time).
        let now = Utc::now();
        for m in &mut mannies {
            m.observed_at = Some(now);
        }
        // Notify on a long task completing: a Manny that was running a notable
        // long task last sync and is now idle (issue #203). Comparing against
        // the previous roster (by id) before it is replaced; the fire→busy lag
        // is a None→Some transition, so it never produces a false completion.
        if let Some(prev) = &self.mannies {
            for m in &mannies {
                if m.current_task.is_some() {
                    continue;
                }
                let finished = prev
                    .iter()
                    .find(|p| p.id == m.id)
                    .and_then(|p| p.current_task.as_ref())
                    .and_then(long_task_note);
                if let Some(note) = finished {
                    self.pending_notifications.push(format!("{} finished {note}", m.name));
                }
            }
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

    pub fn set_toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Local::now()));
    }

    /// Close whatever modal wizard is open (back to the idle cockpit). Replaces
    /// the per-wizard `state.<field> = <Input>::Inactive` resets — with a single
    /// `active_wizard` field there is now one way to close any wizard.
    pub fn close_wizard(&mut self) {
        self.active_wizard = ActiveWizard::None;
    }

    /// Set the inline error on whichever wizard step is currently open — the
    /// active wizard is the one that just errored, since only one can be open.
    /// Replaces the two dozen near-identical `set_<wizard>_error` setters with a
    /// single slot-lookup. Travel, inspect and recover keep their own setters:
    /// travel prefixes the message, and inspect/recover fall back to the status
    /// bar when their picker step is not the active overlay.
    pub fn set_wizard_error(&mut self, msg: String) {
        let slot: Option<&mut Option<String>> = match &mut self.active_wizard {
            ActiveWizard::Repair(RepairInput::Typing { error, .. }) => Some(error),
            ActiveWizard::Mine(MineInput::Configure { error, .. }) => Some(error),
            ActiveWizard::Fabrication(
                FabricationInput::PickRecipe { error, .. } | FabricationInput::PickBuilder { error, .. },
            ) => Some(error),
            ActiveWizard::Improve(
                ImproveInput::PickImprovement { error, .. } | ImproveInput::PickBuilder { error, .. },
            ) => Some(error),
            ActiveWizard::Salvage(SalvageInput::Confirm { error, .. }) => Some(error),
            ActiveWizard::Recall(RecallInput::Confirm { error, .. }) => Some(error),
            ActiveWizard::Refuel(RefuelInput::Confirm { error, .. }) => Some(error),
            ActiveWizard::TransferDeuterium(TransferDeuteriumInput::EnterAmount { error, .. }) => Some(error),
            ActiveWizard::TransferProbe(TransferProbeInput::PickTarget { error, .. }) => Some(error),
            ActiveWizard::MindSnapshot(MindSnapshotInput::Confirm { error }) => Some(error),
            ActiveWizard::ScutRelay(ScutRelayInput::EnterNetworkName { error, .. }) => Some(error),
            ActiveWizard::Missions(MissionsInput::ConfirmAbandon { error, .. }) => Some(error),
            ActiveWizard::Messages(MessagesInput::Compose { error, .. }) => Some(error),
            ActiveWizard::Deploy(DeployInput::EnterName { error, .. }) => Some(error),
            ActiveWizard::RenameManny(RenameMannyInput::Typing { error, .. }) => Some(error),
            ActiveWizard::Detach(
                DetachInput::PickMode { error, .. }
                | DetachInput::PickAsteroid { error, .. }
                | DetachInput::PickTargetProbe { error, .. },
            ) => Some(error),
            ActiveWizard::RemoteMine(
                RemoteMineInput::Configure { error, .. } | RemoteMineInput::PickContainer { error, .. },
            ) => Some(error),
            ActiveWizard::RenameProbe(RenameProbeInput::Typing { error, .. }) => Some(error),
            ActiveWizard::RenameContainer(RenameContainerInput::Typing { error, .. }) => Some(error),
            ActiveWizard::ContainerRules(ContainerRulesInput::Editing { error, .. }) => Some(error),
            ActiveWizard::StorageMove(
                StorageMoveInput::ConfigureResource { error, .. } | StorageMoveInput::ConfigureItem { error, .. },
            ) => Some(error),
            ActiveWizard::DropCargo(DropCargoInput::Confirm { error, .. }) => Some(error),
            ActiveWizard::AssembleProbe(AssembleProbeInput::PickContainers { error, .. }) => Some(error),
            ActiveWizard::DropContainer(DropStorageContainerInput::PickPlanet { error, .. }) => Some(error),
            ActiveWizard::Jettison(
                JettisonInput::ConfirmManny { error, .. }
                | JettisonInput::ConfirmRelay { error, .. }
                | JettisonInput::EnterAmount { error, .. },
            ) => Some(error),
            _ => None,
        };
        if let Some(e) = slot {
            *e = Some(msg);
        }
    }

    /// Common tail of a successful action: show a confirmation toast and stage
    /// the follow-up refresh. The wizard reset stays at the call site — most
    /// arms `close_wizard()` first, but a few (mission abandon, message sent)
    /// return to a browsing state and keep their overlay open, so closing here
    /// would be wrong. The event loop drains `pending_refetch` and dispatches
    /// the actual fetch.
    pub fn finish_action(&mut self, toast: impl Into<String>, refetch: Refetch) {
        self.set_toast(toast);
        self.pending_refetch = Some(refetch);
    }

    /// Stage a ship's-log entry for this tick. The event loop drains
    /// `pending_journal` — persisting each entry and prepending it to `journal`
    /// (mirrors how `pending_fire` is drained by the input layer).
    pub fn log_event(&mut self, ev: LogEvent) {
        self.pending_journal.push(ev);
    }

    /// Advance the naming-ceremony seed and return the next Culture-style name
    /// suggestion, cycling through the lexicon. Used to pre-fill and regenerate
    /// the rename wizards' input.
    pub fn next_name_suggestion(&mut self) -> String {
        self.name_suggestion_seed = self.name_suggestion_seed.wrapping_add(1);
        lexicon::suggest(self.name_suggestion_seed).to_string()
    }

    /// The ship's-log flow shown in the pane: locally-captured actions merged
    /// with reconstructed server events (alerts + damage warnings), newest
    /// first, capped at the memory window. Server events live on the server and
    /// are re-sent on each fetch, so they're projected fresh here rather than
    /// persisted locally (no dedup needed — this recomputes per render).
    pub fn ship_log_entries(&self) -> Vec<LogEvent> {
        fn project(a: &ProbeAlert) -> LogEvent {
            LogEvent {
                occurred_at: a.scheduled_at.or(a.created_at).unwrap_or_else(Utc::now),
                kind: crate::app::kind::ALERT.to_string(),
                probe_id: None,
                summary: a.message.clone(),
                data: serde_json::Value::Null,
            }
        }
        let mut all = self.journal.clone();
        all.extend(self.alerts.iter().map(project));
        all.extend(self.damage_warnings.iter().map(project));
        all.sort_by_key(|e| std::cmp::Reverse(e.occurred_at));
        all.truncate(crate::store::JOURNAL_WINDOW);
        all
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
        let base = match self.movement_arrival {
            Some(arrival) => {
                let remaining = (arrival - Utc::now()).to_std().unwrap_or(std::time::Duration::ZERO);
                Instant::now() + remaining
            }
            None => Instant::now() + std::time::Duration::from_secs(86400),
        };
        // While the production queue or the action script is working, poll
        // briskly so a finished craft/step is detected within a few seconds (the
        // server has no push).
        if self.queue_active() || self.script_active() {
            return base.min(Instant::now() + std::time::Duration::from_secs(QUEUE_POLL_SECS));
        }
        base
    }

    pub fn seconds_until_refresh(&self) -> Option<i64> {
        self.movement_arrival.map(|a| (a - Utc::now()).num_seconds().max(0))
    }
}
