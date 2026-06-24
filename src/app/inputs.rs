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
