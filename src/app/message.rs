use crate::api::types::{
    ContainerInventory, CraftingRecipe, DamageWarningRule, Manny, MannyDetail, MannyRoster, Mission, Pagination, Probe,
    ProbeAlert, ProbeImprovement, ProbeInventory, ProbeListResponse, ProbeMessage, ProbeMovement, ProbeSentMessage,
    ScutNetwork, SectorObservation, StorageContainer, VisitedSector,
};

pub enum ApiMessage {
    ProbeUpdated(Probe),
    /// The player's fleet roster (`GET /api/probes`), fetched in `fetch_all`.
    /// Non-fatal. Drives the probe switcher; never resets the active probe.
    FleetFetched(ProbeListResponse),
    /// A `PATCH /api/probe/{id}` promoted a probe to default; carries the
    /// refreshed roster and the probe name for the toast. Failure (e.g. the
    /// 422 out-of-reach) arrives as `ActionError`.
    DefaultProbeSet(ProbeListResponse, String),
    /// A `PATCH /api/probe/{id}` renamed a probe; carries the refreshed roster
    /// and the new name for the toast. Failure arrives as `RenameProbeError`.
    ProbeRenamed(ProbeListResponse, String),
    RenameProbeError(String),
    /// The Manny roster plus the v104 polling hint. The `Option<u64>` is the
    /// probe the fetch targeted (`None` = the server default, i.e. the client's
    /// own `active_probe_id`): a roster in flight when the pilot switches probe
    /// lands *after* the switch, and a sequencer must never read it as if it
    /// described the newly piloted probe (issue #291).
    ManniesUpdated(Option<u64>, MannyRoster),
    /// A single Manny refreshed via `GET …/mannies/{id}` (API v104): the cheap
    /// poll used while waiting on one busy Manny. Merged into the roster.
    /// Carries the same probe tag as `ManniesUpdated`.
    MannyUpdated(Option<u64>, MannyDetail),
    SectorUpdated(SectorObservation),
    ScanError(String),
    MoveStarted(ProbeMovement),
    MoveError(String),
    RepairStarted,
    RepairError(String),
    MineStarted,
    MineError(String),
    VersionFetched(u32),
    VisitedSectorsFetched(Vec<VisitedSector>),
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
    ProbeImprovementsFetched(Vec<ProbeImprovement>),
    /// The full improvement catalog (locked entries included) for `:tree`.
    TreeImprovementsFetched(Vec<ProbeImprovement>),
    ImproveProbeStarted,
    ImproveProbeError(String),
    RenameMannyDone(Manny),
    RenameMannyError(String),
    InspectStarted,
    InspectError(String),
    RecoverStarted,
    RecoverError(String),
    DetachStarted,
    DetachError(String),
    DamageWarningsFetched(Vec<ProbeAlert>, DamageWarningRule),
    DamageWarningAcknowledged(ProbeAlert),
    AlertsFetched(Vec<ProbeAlert>),
    AlertAcknowledged(ProbeAlert),
    StorageContainersFetched(Vec<StorageContainer>),
    StorageContainerDetailFetched(StorageContainer, ContainerInventory),
    StorageContainerDetailError(String),
    RenameContainerDone(StorageContainer, ProbeInventory),
    RenameContainerError(String),
    UpdateContainerRulesDone(StorageContainer, ProbeInventory),
    UpdateContainerRulesError(String),
    StorageMoveDone(Manny, ProbeInventory),
    StorageMoveError(String),
    /// A drone-assembly task started (API v81): the updated builder Manny and
    /// probe inventory (two containers + components consumed).
    AssembleProbeStarted(Manny, ProbeInventory),
    AssembleProbeError(String),
    DropMannyCargoStarted(Manny),
    DropMannyCargoError(String),
    DeuteriumRefuelStarted,
    DeuteriumRefuelError(String),
    DeuteriumTransferStarted,
    DeuteriumTransferError(String),
    MannyTransferStarted,
    MannyTransferError(String),
    MindSnapshotReassigned(Probe),
    MindSnapshotReassignError(String),
    MissionsFetched(Vec<Mission>),
    MissionAbandoned(Mission),
    MissionAbandonError(String),
    ScutRelayTurnedOn,
    ScutRelayTurnOnError(String),
    TransitBeaconStarted,
    TransitBeaconError(String),
    ScutNetworkFetched(ScutNetwork),
    ScutNetworkError(String),
    /// The inbox page plus its pagination — the cockpit needs `has_more` to
    /// know whether the page holds every unread message (API v104).
    MessagesFetched(Vec<ProbeMessage>, Pagination),
    /// An atomic Manny task batch was accepted (API v104); carries the
    /// refreshed Mannies in request order.
    MannyTasksStarted(Vec<Manny>),
    /// The batch was rejected — nothing was applied.
    MannyTasksError(String),
    /// Exact unread count from the server's `status=unread` filter.
    UnreadMessagesFetched(usize),
    SentMessagesFetched(Vec<ProbeSentMessage>),
    MessageSent(ProbeMessage),
    MessageSendError(String),
    MessageMarkedRead(ProbeMessage),
    DropStorageContainerStarted(Manny),
    DropStorageContainerError(String),
    /// A direct user action (ack alert / damage warning, mark message read) that
    /// failed — surfaced in the status bar so the key does not feel dead.
    /// Distinct from `Error`, which is the fatal-probe channel that drives
    /// refresh backoff.
    ActionError(String),
    Error(String),
}
