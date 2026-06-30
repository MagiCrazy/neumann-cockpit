use super::*;

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

pub const RESOURCE_TYPES: [&str; 4] = ["deuterium", "metals", "ice", "carbon_compounds"];

pub const RESOURCE_LABELS: [&str; 4] = ["deuterium", "metals", "ice", "carbon"];

pub const DETACH_MODES: [(&str, &str); 2] = [
    ("drifting", "drifting — leave in sector"),
    ("hidden_on_asteroid", "hidden — attach to asteroid"),
];

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

#[derive(Default)]
pub enum AlertsInput {
    #[default]
    Inactive,
    /// `show_warnings` selects the Warnings tab; otherwise the Alerts tab.
    /// The entries themselves live in `AppState::alerts` / `damage_warnings`.
    Browsing {
        selection: usize,
        show_warnings: bool,
    },
}

#[derive(Default)]
pub enum ContainersInput {
    #[default]
    Inactive,
    /// Browsing the storage-container list (entries live in
    /// `AppState::storage_containers`).
    Browsing { selection: usize },
}

#[derive(Default)]
pub enum RenameContainerInput {
    #[default]
    Inactive,
    Typing {
        container_id: String,
        current_label: String,
        buf: String,
        error: Option<String>,
    },
}

#[derive(Default)]
pub enum ContainerRulesInput {
    #[default]
    Inactive,
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

#[derive(Default)]
pub enum StorageMoveInput {
    #[default]
    Inactive,
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

#[derive(Default)]
pub enum DropStorageContainerInput {
    #[default]
    Inactive,
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

#[derive(Default)]
pub enum DropCargoInput {
    #[default]
    Inactive,
    /// Confirmation for the irreversible cargo drop (resources are lost).
    Confirm {
        manny_id: String,
        manny_name: String,
        error: Option<String>,
    },
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
        /// True when the Manny is in a remote sector reachable via SCUT: the
        /// recall cancels its task and leaves it forgotten (it does not return).
        remote: bool,
        error: Option<String>,
    },
}

#[derive(Default)]
pub enum RefuelInput {
    #[default]
    Inactive,
    /// Confirmation to send a Manny to refill the probe deuterium tank.
    Confirm {
        manny_id: String,
        manny_name: String,
        error: Option<String>,
    },
}

#[derive(Default)]
pub enum MindSnapshotInput {
    #[default]
    Inactive,
    /// Confirmation for the irreversible mind-snapshot reassignment to a fresh
    /// probe (only offered when the probe is dead or trapped by a black hole).
    Confirm { error: Option<String> },
}

#[derive(Default)]
pub enum ScutRelayInput {
    #[default]
    Inactive,
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

#[derive(Default)]
pub enum ScutNetworkInput {
    #[default]
    Inactive,
    /// Several networks cover the sector — pick which one to inspect.
    Picking {
        networks: Vec<(i64, String)>, // (network id, name)
        selection: usize,
    },
    /// Inspecting a network; details live in `AppState::scut_network_view`
    /// (None while the fetch is in flight).
    Viewing { error: Option<String> },
}

#[derive(Default)]
pub enum MissionsInput {
    #[default]
    Inactive,
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
    /// Confirmation for deploying a scut_relay item as an inactive relay.
    ConfirmRelay {
        item_id: String,
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
