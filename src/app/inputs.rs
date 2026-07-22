use super::*;
use crate::api::types::EndpointId;

#[derive(Default)]
pub enum ScanMode {
    #[default]
    Current,
    Input(String),
    DirectionPick,
}

pub enum RepairInput {
    Typing {
        manny_id: String,
        manny_name: String,
        buf: String,
        error: Option<String>,
    },
}

pub enum TravelInput {
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

/// Picker over the probe's visited sectors; selecting one launches the travel
/// wizard (confirm step) for its coordinates.
#[derive(Default)]
pub enum GotoVisitedInput {
    #[default]
    Inactive,
    Picking {
        selection: usize,
    },
}

/// Picker over the player's fleet (API v81 multi-probe). Selecting a reachable
/// probe switches the cockpit's active probe (client-side, no server change);
/// an unreachable one is refused with a toast.
#[derive(Default)]
pub enum ProbeSwitchInput {
    #[default]
    Inactive,
    Picking {
        selection: usize,
    },
}

/// Rename-probe wizard (API v81): text entry that renames the piloted probe
/// via `PATCH /api/probe/{id}`.
pub enum RenameProbeInput {
    Typing {
        probe_id: u64,
        current_name: String,
        buf: String,
        error: Option<String>,
    },
}

/// Assemble-a-drone wizard (API v81): a chosen Manny plus exactly two empty
/// additional containers. Single step — the container multi-select and the
/// (always-visible) ingredient bill live together; `Enter` with two selected
/// fires the 3-hour task.
pub enum AssembleProbeInput {
    PickContainers {
        manny_id: String,
        manny_name: String,
        /// Empty additional containers, `(id, label)`.
        containers: Vec<(String, String)>,
        /// Indices into `containers` currently ticked (at most two).
        selected: Vec<usize>,
        cursor: usize,
        error: Option<String>,
    },
}

pub const RESOURCE_TYPES: [&str; 4] = ["deuterium", "metals", "ice", "carbon_compounds"];

pub const RESOURCE_LABELS: [&str; 4] = ["deuterium", "metals", "ice", "carbon"];

pub const DETACH_MODES: [(&str, &str); 2] = [
    ("drifting", "drifting — leave in sector"),
    ("hidden_on_asteroid", "hidden — attach to asteroid"),
];

pub enum ObjectActionInput {
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

pub enum AlertsInput {
    /// `show_warnings` selects the Warnings tab; otherwise the Alerts tab.
    /// The entries themselves live in `AppState::alerts` / `damage_warnings`.
    Browsing { selection: usize, show_warnings: bool },
}

pub enum RenameContainerInput {
    Typing {
        container_id: String,
        current_label: String,
        buf: String,
        error: Option<String>,
    },
}

pub enum ContainerRulesInput {
    /// Each routable type in `types` is assigned to at most one of the three
    /// lists; `selection` cursors `types`, cycled none → priority → exclusion
    /// → strict via Space.
    Editing {
        container_id: String,
        container_label: String,
        types: Vec<String>,
        priority: Vec<String>,
        exclusion: Vec<String>,
        strict_exclusion: Vec<String>,
        selection: usize,
        error: Option<String>,
    },
}

/// Resource types movable between containers — deuterium lives in the tank,
/// not in storage containers, so it is excluded (matches the v44 schema).
pub const MOVE_RESOURCE_TYPES: [&str; 3] = ["metals", "ice", "carbon_compounds"];

pub enum StorageMoveInput {
    PickManny {
        mannies: Vec<(String, String)>,
        selection: usize,
    },
    PickKind {
        actor_manny_id: String,
        actor_manny_name: String,
        selection: usize, // 0 = resource, 1 = item
    },
    ConfigureResource {
        actor_manny_id: String,
        actor_manny_name: String,
        containers: Vec<(String, String)>,
        resource_idx: usize,
        from_sel: usize,
        to_sel: usize,
        amount_buf: String,
        field: u8, // 0 resource, 1 from, 2 to, 3 amount
        error: Option<String>,
    },
    ConfigureItem {
        actor_manny_id: String,
        actor_manny_name: String,
        containers: Vec<(String, String)>,
        items: Vec<(String, String, bool)>, // (id, label, selected)
        to_sel: usize,
        item_cursor: usize,
        field: u8, // 0 items list, 1 destination
        error: Option<String>,
    },
}

pub enum DropStorageContainerInput {
    PickContainer {
        manny_id: String,
        manny_name: String,
        containers: Vec<(String, String)>,
        selection: usize,
    },
    PickPlanet {
        manny_id: String,
        manny_name: String,
        container_id: String,
        container_name: String,
        planets: Vec<(String, String)>,
        selection: usize,
        error: Option<String>,
    },
}

pub enum DropCargoInput {
    /// Confirmation for the irreversible cargo drop (resources are lost).
    Confirm {
        manny_id: String,
        manny_name: String,
        error: Option<String>,
    },
}

pub enum WaypointsInput {
    Browsing {
        entries: Vec<WaypointEntry>,
        selection: usize,
    },
}

/// Which fabricator produces a recipe, derived from its `craftable_by`. Drives
/// both the section a recipe is shown under and which endpoint a craft fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fabricator {
    /// `atomic_3d_printer` — the printer auto-reserves a Manny as assistant.
    AtomicPrinter,
    /// `manny` — the craft is assigned to a specific idle onboard Manny.
    Manny,
}

/// Probe-improvement wizard (API v67+): pick an improvement, then resolve which
/// idle onboard Manny installs it (auto when a single one, else `PickBuilder`).
pub enum ImproveInput {
    PickImprovement {
        selection: usize,
        error: Option<String>,
    },
    PickBuilder {
        improvement_id: String,
        improvement_name: String,
        mannies: Vec<(String, String)>, // (id, name)
        selection: usize,
        error: Option<String>,
    },
}

/// Unified fabrication wizard: a single item-first catalog spanning both the
/// atomic printer and Manny craft. `PickRecipe` lists every recipe sectioned by
/// fabricator; selecting a Manny recipe with no pre-chosen builder advances to
/// `PickBuilder`, atomic recipes fire straight away.
/// Which panel of the fabrication console has the keyboard focus.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FabFocus {
    /// The recipe catalog (adjust qty, Enter to queue).
    Catalog,
    /// The live production queue (move the cursor, remove/clear steps).
    Queue,
}

pub enum FabricationInput {
    /// The production console: the recipe catalog on the left, the live queue on
    /// the right. `Enter` on the catalog adds the highlighted recipe ×`qty` to
    /// the queue; `Tab` moves focus to the queue to manage it.
    PickRecipe {
        /// A builder Manny (id, name) pre-chosen when the catalog was opened
        /// from the Mannies pane on an orderable Manny. Manny recipes skip the
        /// builder-selection step when this is set; atomic recipes ignore it.
        prefilled_manny: Option<(String, String)>,
        selection: usize,
        /// Quantity to enqueue for the highlighted recipe (`+`/`-`, `h`/`l`).
        qty: u32,
        focus: FabFocus,
        /// Cursor into the queue panel when `focus == Queue`.
        queue_sel: usize,
        error: Option<String>,
    },
    /// Choosing which idle onboard Manny builds the selected Manny recipe before
    /// it is enqueued (`qty` carries the chosen quantity through).
    PickBuilder {
        recipe_id: String,
        recipe_name: String,
        qty: u32,
        mannies: Vec<(String, String)>, // (id, name)
        selection: usize,
        error: Option<String>,
    },
}

impl FabricationInput {
    /// Open the console on the catalog with qty 1.
    pub fn pick_recipe(prefilled_manny: Option<(String, String)>) -> Self {
        FabricationInput::PickRecipe {
            prefilled_manny,
            selection: 0,
            qty: 1,
            focus: FabFocus::Catalog,
            queue_sel: 0,
            error: None,
        }
    }

    /// The builder-selection step for a Manny recipe, carrying its quantity.
    pub fn pick_builder(recipe_id: String, recipe_name: String, qty: u32, mannies: Vec<(String, String)>) -> Self {
        FabricationInput::PickBuilder {
            recipe_id,
            recipe_name,
            qty,
            mannies,
            selection: 0,
            error: None,
        }
    }
}

pub enum MineInput {
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
        /// Optional detached container in the current sector to deliver mined
        /// resources into ([c] cycles None → containers). `None` = probe.
        target_container: Option<(String, String)>, // (id, name)
        error: Option<String>,
    },
}

/// Remote mining of an idle Manny in a SCUT-reachable sector (API v60).
/// Targets the Manny's sector; a detached container there is mandatory.
pub enum RemoteMineInput {
    /// Waiting for the Manny's sector scan to arrive.
    Loading {
        manny_id: String,
        manny_name: String,
        x: i32,
        y: i32,
        z: i32,
    },
    PickAsteroid {
        manny_id: String,
        manny_name: String,
        x: i32,
        y: i32,
        z: i32,
        candidates: Vec<(String, String)>,
        selection: usize,
    },
    Configure {
        manny_id: String,
        manny_name: String,
        x: i32,
        y: i32,
        z: i32,
        object_id: String,
        object_name: String,
        resources: [bool; 4],
        amount_buf: String,
        amount_mode: bool,
        error: Option<String>,
    },
    PickContainer {
        manny_id: String,
        object_id: String,
        resources: [bool; 4],
        amount: f64,
        containers: Vec<(String, String)>,
        selection: usize,
        error: Option<String>,
    },
}

pub enum SalvageInput {
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

pub enum RecallInput {
    Confirm {
        manny_id: String,
        manny_name: String,
        /// True when the Manny is in a remote sector reachable via SCUT: the
        /// recall cancels its task and leaves it forgotten (it does not return).
        remote: bool,
        error: Option<String>,
    },
}

pub enum RefuelInput {
    /// Confirmation to send a Manny to refill the probe deuterium tank.
    Confirm {
        manny_id: String,
        manny_name: String,
        error: Option<String>,
    },
}

/// Transfer a Manny to another owned probe (API v93). Single step: pick the
/// destination probe from the roster (the piloted source excluded). The
/// same-sector requirement is server-validated, so a wrong target surfaces as
/// a 422 in `error`.
#[derive(Debug, Clone, PartialEq)]
pub enum TransferProbeInput {
    PickTarget {
        manny_id: String,
        manny_name: String,
        targets: Vec<(u64, String)>,
        selection: usize,
        error: Option<String>,
    },
}

/// Deuterium-transfer wizard (API v86): a Manny ferries a reserved deuterium
/// amount from the current probe to another fleet probe in the same sector.
/// Two steps — pick the destination probe (the source is excluded), then enter
/// the percentage to transfer. The same-sector constraint is server-validated
/// (the roster carries no coordinates), so a wrong target surfaces as an error
/// in `EnterAmount`.
#[derive(Debug, Clone, PartialEq)]
pub enum TransferDeuteriumInput {
    /// Choose the destination probe from the roster (id, name pairs).
    PickTarget {
        manny_id: String,
        manny_name: String,
        targets: Vec<(u64, String)>,
        selection: usize,
    },
    /// Enter the deuterium percentage to reserve and transfer.
    EnterAmount {
        manny_id: String,
        manny_name: String,
        target_id: u64,
        target_name: String,
        buf: String,
        error: Option<String>,
    },
}

pub enum MindSnapshotInput {
    /// Confirmation for the irreversible mind-snapshot reassignment to a fresh
    /// probe (only offered when the probe is dead or trapped by a black hole).
    Confirm { error: Option<String> },
}

pub enum ScutRelayInput {
    /// Turn-on wizard for an inactive relay: optional network name then confirm.
    EnterNetworkName {
        manny_id: String,
        manny_name: String,
        relay_id: i64,
        relay_name: String,
        buf: String,
        error: Option<String>,
    },
}

pub enum MessagesInput {
    /// Browsing inbox (sent_tab=false) or sent (true); entries in AppState.
    Browsing { sent_tab: bool, selection: usize },
    /// Reading one message full-screen (its full body + emission sector).
    Reading { id: i64, sent_tab: bool },
    /// Picking a recipient for a new message (probes + planets in sector).
    PickRecipient {
        recipients: Vec<(String, EndpointId, String)>, // (kind, id, name)
        selection: usize,
    },
    /// Typing the message body for the chosen recipient.
    Compose {
        recipient_type: String,
        recipient_id: EndpointId,
        recipient_name: String,
        body_buf: String,
        error: Option<String>,
    },
}

pub enum ScutNetworkInput {
    /// Several networks cover the sector — pick which one to inspect.
    Picking {
        networks: Vec<(i64, String)>, // (network id, name)
        selection: usize,
    },
    /// Inspecting a network; details live in `AppState::scut_network_view`
    /// (None while the fetch is in flight).
    Viewing { error: Option<String> },
}

pub enum MissionsInput {
    /// Browsing the mission list; entries live in `AppState::missions`.
    Browsing { selection: usize },
    /// Confirmation for abandoning the selected active mission.
    ConfirmAbandon {
        mission_id: String,
        mission_title: String,
        selection: usize,
        error: Option<String>,
    },
}

pub enum RenameMannyInput {
    Typing {
        manny_id: String,
        manny_name: String,
        buf: String,
        error: Option<String>,
    },
}

pub enum DeployInput {
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

pub enum JettisonInput {
    ConfirmManny {
        item_id: String,
        manny_name: String,
        error: Option<String>,
    },
    /// Confirmation for deploying a scut_relay item as an inactive relay.
    ConfirmRelay { item_id: String, error: Option<String> },
    EnterAmount {
        item_id: String,
        item_name: String,
        max_amount: f64,
        buf: String,
        error: Option<String>,
    },
    /// Confirmation for the irreversible resource jettison (stock is lost).
    Confirm {
        item_id: String,
        item_name: String,
        amount: Option<f64>,
        error: Option<String>,
    },
}

pub enum InspectInput {
    PickTarget {
        manny_id: String,
        manny_name: String,
        candidates: Vec<(String, String)>,
        selection: usize,
        error: Option<String>,
    },
}

pub enum RecoverInput {
    PickContainer {
        manny_id: String,
        manny_name: String,
        candidates: Vec<(String, String)>,
        selection: usize,
        error: Option<String>,
    },
}

pub enum DetachInput {
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

/// The action-scripting console (#198). Vim-style modal editor over the session
/// script: `Insert` types a new command line (`Enter` commits it via
/// `parse_script_line`), `Normal` navigates/manages the step list (`j`/`k`,
/// `x` remove, `c` clear, `R` run, `p` pause).
pub enum ScriptInput {
    Normal { selection: usize },
    Insert { buf: String, error: Option<String> },
}

impl ScriptInput {
    /// Open ready to type the first line.
    pub fn editing() -> Self {
        ScriptInput::Insert {
            buf: String::new(),
            error: None,
        }
    }
}

/// The single modal wizard the cockpit currently has open — a sum type over the
/// mutually-exclusive wizards, replacing coexisting `*Input` fields on
/// `AppState`. Because only one variant can be held at a time, "two wizards open
/// at once" is now unrepresentable (the invariant is a compile-time fact, not a
/// convention). `None` is the sole idle state (it replaced every `*Input::Inactive`).
#[derive(Default)]
pub enum ActiveWizard {
    #[default]
    None,
    Repair(RepairInput),
    Travel(TravelInput),
    AssembleProbe(AssembleProbeInput),
    RenameProbe(RenameProbeInput),
    Jettison(JettisonInput),
    Fabrication(FabricationInput),
    Improve(ImproveInput),
    Salvage(SalvageInput),
    Recall(RecallInput),
    Refuel(RefuelInput),
    TransferDeuterium(TransferDeuteriumInput),
    TransferProbe(TransferProbeInput),
    MindSnapshot(MindSnapshotInput),
    ScutRelay(ScutRelayInput),
    ScutNetwork(ScutNetworkInput),
    Missions(MissionsInput),
    Messages(MessagesInput),
    RenameManny(RenameMannyInput),
    Deploy(DeployInput),
    Inspect(InspectInput),
    Recover(RecoverInput),
    Detach(DetachInput),
    Alerts(AlertsInput),
    RenameContainer(RenameContainerInput),
    ContainerRules(ContainerRulesInput),
    StorageMove(StorageMoveInput),
    DropCargo(DropCargoInput),
    DropContainer(DropStorageContainerInput),
    ObjectAction(ObjectActionInput),
    Waypoints(WaypointsInput),
    Mine(MineInput),
    RemoteMine(RemoteMineInput),
    Script(ScriptInput),
}
