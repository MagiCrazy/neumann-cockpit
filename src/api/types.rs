#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Primitives ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Vector {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProbeStatus {
    Idle,
    Preparing,
    Accelerating,
    Cruising,
    Decelerating,
    Orbiting,
    Disabled,
    Dead,
    TrappedByBlackHole,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SensorMode {
    Normal,
    Degraded,
    Blind,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MovementPhase {
    Idle,
    Preparing,
    Accelerating,
    Cruising,
    Decelerating,
    Arrived,
    Failed,
    Destroyed,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MannyTask {
    Repair,
    Mining,
    Crafting,
    AssistingAtomicPrinter,
    Salvage,
    InstallingWaypointBookmark,
    DetachingStorageContainer,
    InspectingAsteroid,
    Returning,
    WaitingForSpace,
    MovingStockage,
    DroppingStorageContainer,
    RefillingDeuteriumTank,
    TurningOnScutRelay,
    UnknownTooFar,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MannyLocationType {
    Probe,
    Sector,
    #[serde(other)]
    Unknown,
}

// ── Movement ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeMovement {
    pub status: MovementPhase,
    pub origin: Vector,
    pub target: Vector,
    pub distance: i64,
    pub fuel_cost_deuterium: f64,
    pub started_at: DateTime<Utc>,
    pub arrival_at: DateTime<Utc>,
    pub phase: Option<MovementPhase>,
    pub seconds_remaining: Option<i64>,
    pub sensor_mode: Option<SensorMode>,
    pub estimated_velocity_c: Option<f64>,
}

// ── Storage containers ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageContainerSummary {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageContainerRules {
    #[serde(default)]
    pub priority: Vec<String>,
    #[serde(default)]
    pub exclusion: Vec<String>,
    #[serde(default)]
    pub strict_exclusion: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageContainer {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub sort_order: i32,
    pub capacity: f64,
    pub used_capacity: f64,
    pub free_capacity: f64,
    pub capacity_unit: Option<String>,
    pub rules: StorageContainerRules,
}

/// Inner inventory of a single storage container
/// (`GET /api/probe/storage-containers/{id}`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerInventory {
    pub capacity_unit: Option<String>,
    #[serde(default)]
    pub items: Vec<ProbeInventoryItem>,
    #[serde(default)]
    pub resource_stocks: Vec<ProbeResourceStock>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceStockContainerLine {
    pub container: StorageContainerSummary,
    pub amount: f64,
    pub container_space: f64,
}

// ── Inventory ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MannyCargo {
    pub capacity: f64,
    pub deuterium: f64,
    pub metals: f64,
    pub ice: f64,
    pub organic_compounds: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MannyLocation {
    #[serde(rename = "type")]
    pub location_type: MannyLocationType,
    pub sector: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manny {
    pub id: String,
    pub name: String,
    pub location: MannyLocation,
    pub current_task: Option<MannyTask>,
    pub task_progress_percent: f64,
    #[serde(default)]
    pub task_visibility: Option<MannyTaskVisibility>,
    pub cargo: MannyCargo,
    pub can_receive_orders: bool,
    pub task_estimated_end_time: Option<DateTime<Utc>>,
    /// Current-task payload. For a mining task it carries the target asteroid,
    /// resource types, and destination container (else the probe). Kept as a
    /// raw value: the API's `anyOf` may also be an empty object/array, and
    /// remote/too-far mannies expose an empty payload.
    #[serde(default)]
    pub task: Option<serde_json::Value>,
    /// Client-side receipt timestamp (not an API field), stamped on
    /// `update_mannies`. Lets the UI interpolate `task_progress_percent`
    /// against `task_estimated_end_time` so progress ticks between fetches.
    #[serde(default)]
    pub observed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MannyTaskVisibility {
    /// Current sector or probe rack.
    Local,
    /// Different sector reachable via a shared SCUT network.
    ScutNetwork,
    /// Out of telemetry range; task details unavailable.
    TooFar,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeInventoryItem {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub name: String,
    pub container_space: f64,
    pub current_task: Option<String>,
    pub task_progress_percent: f64,
    pub location: Option<MannyLocation>,
    pub cargo: Option<MannyCargo>,
    pub container: Option<StorageContainerSummary>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResourceStock {
    pub id: String,
    #[serde(rename = "type")]
    pub stock_type: String,
    pub name: String,
    pub amount: f64,
    pub container_space: f64,
    #[serde(default)]
    pub containers: Vec<ResourceStockContainerLine>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeExternalTank {
    pub id: String,
    #[serde(rename = "type")]
    pub tank_type: String,
    pub name: String,
    pub fill_percent: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeInventory {
    pub capacity: f64,
    pub used_capacity: f64,
    pub free_capacity: f64,
    pub items: Vec<ProbeInventoryItem>,
    pub resource_stocks: Vec<ProbeResourceStock>,
    pub external_tanks: Vec<ProbeExternalTank>,
    #[serde(default)]
    pub containers: Vec<StorageContainer>,
}

// ── Alerts & damage warnings ──────────────────────────────────────────────────
//
// `/api/probe/alerts` and `/api/probe/damage-warnings` both return the same
// `ProbeDamageWarning` schema; we model both with `ProbeAlert`. Only the
// response envelope differs (`alerts` + keyed `rules` vs `damageWarnings` +
// single `rule`). Sub-objects are present only for specific alert types, hence
// `Option<T>`; enums carry an `Unknown` fallback for forward compatibility.

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    StorageContainerBreak,
    IntelligentLife,
    SectorObjectDetected,
    AnomalyDetected,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AlertStatus {
    Unread,
    Read,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AlertPhase {
    AccelerationEnd,
    DecelerationStart,
    Arrival,
    Detection,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertContainerRef {
    pub id: String,
    pub label: Option<String>,
    pub object_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertRisk {
    pub percent: i64,
    pub additional_container_count: i64,
    pub rule_starts_at_additional_containers: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertPlanetRef {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertObjectRef {
    pub id: String,
    #[serde(rename = "type")]
    pub object_type: String,
    pub label: Option<String>,
    #[serde(default)]
    pub resource_types: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertSector {
    pub relative: Option<Vector>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeAlert {
    pub id: i64,
    #[serde(rename = "type")]
    pub alert_type: AlertType,
    pub status: AlertStatus,
    pub message: String,
    pub phase: AlertPhase,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub sector: Option<AlertSector>,
    pub container: Option<AlertContainerRef>,
    pub risk: Option<AlertRisk>,
    pub planet: Option<AlertPlanetRef>,
    pub object: Option<AlertObjectRef>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub read_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl ProbeAlert {
    /// An alert still needs the operator's attention while unread and unresolved.
    pub fn is_unread(&self) -> bool {
        self.status == AlertStatus::Unread && self.resolved_at.is_none()
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DamageWarningRule {
    #[serde(rename = "type", default)]
    pub rule_type: String,
    pub starts_at_additional_containers: Option<i64>,
    pub risk_per_additional_container_after_four_percent: Option<i64>,
    pub maximum_risk_percent: Option<i64>,
    #[serde(default)]
    pub message: String,
}

// ── Visited sectors ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisitedSector {
    pub relative_coordinates: Vector,
    pub first_visited_at: DateTime<Utc>,
    pub last_visited_at: DateTime<Utc>,
    pub visit_count: i64,
}

// ── Systems ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeSystems {
    pub integrity_percent: Option<f64>,
    pub damage_percent: Option<f64>,
    pub energy_stored: Option<f64>,
    pub internal_clock_rate: Option<f64>,
    pub current_task: Option<String>,
}

// ── Probe ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeFuel {
    pub deuterium: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeSector {
    pub relative: Option<Vector>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Probe {
    pub id: i64,
    pub name: String,
    pub status: ProbeStatus,
    pub fuel: ProbeFuel,
    pub sensor_mode: SensorMode,
    pub sector: Option<ProbeSector>,
    pub movement: Option<ProbeMovement>,
    pub systems: Option<ProbeSystems>,
    pub inventory: ProbeInventory,
    /// Critical recovery alert raised when the probe is dead or trapped by a
    /// black hole, carrying the mind-snapshot reassignment action.
    #[serde(default)]
    pub alert: Option<ProbeTerminalAlert>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProbeTerminalAlert {
    #[serde(rename = "type")]
    pub alert_type: String,
    pub severity: String,
    pub title: String,
    pub message: String,
    pub action: ProbeTerminalAlertAction,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProbeTerminalAlertAction {
    pub label: String,
    pub method: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MissionStatus {
    Active,
    Completed,
    Failed,
    Abandoned,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MissionStepStatus {
    Pending,
    Completed,
    Failed,
    Skipped,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mission {
    pub id: String,
    #[serde(rename = "type")]
    pub mission_type: String,
    pub title: String,
    pub description: Option<String>,
    pub status: MissionStatus,
    #[serde(default)]
    pub steps: Vec<MissionStep>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum EndpointId {
    Probe(i64),
    Planet(String),
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    Unread,
    Read,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeMessageEndpoint {
    #[serde(rename = "type")]
    pub endpoint_type: String,
    pub id: EndpointId,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeMessage {
    pub id: i64,
    pub sender: ProbeMessageEndpoint,
    pub recipient: ProbeMessageEndpoint,
    pub body: String,
    pub status: MessageStatus,
    pub read_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeSentMessage {
    pub id: i64,
    pub sender: ProbeMessageEndpoint,
    pub recipient: ProbeMessageEndpoint,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub limit: i64,
    pub offset: i64,
    pub count: i64,
    pub total: i64,
    pub has_more: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionStep {
    pub id: String,
    pub sort_order: i64,
    pub title: String,
    pub description: Option<String>,
    pub status: MissionStepStatus,
}

// ── Mannies list ──────────────────────────────────────────────────────────────

// ── Crafting recipes ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CraftingRecipeIngredient {
    #[serde(rename = "type")]
    pub ingredient_type: String,
    pub quantity: f64,
    pub unit: String,
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CraftingRecipeOutput {
    #[serde(rename = "type")]
    pub output_type: String,
    pub name: String,
    pub container_space: f64,
    pub container_space_unit: String,
    pub capacity_bonus: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CraftingRecipe {
    pub id: String,
    pub name: String,
    pub craftable_by: Vec<String>,
    pub ingredients: Vec<CraftingRecipeIngredient>,
    pub duration_seconds: i64,
    pub output: CraftingRecipeOutput,
}

// ── Sector ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimatedObjects {
    pub star: Option<bool>,
    pub planet_count_min: Option<i32>,
    pub planet_count_max: Option<i32>,
    pub black_hole_probability: Option<f64>,
    pub danger_estimate: Option<DangerLevel>,
    pub signal_age: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeLevel {
    Detailed,
    NeighborScan,
    DistantScan,
    LongRangeEstimation,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DataFreshness {
    Live,
    DegradedLive,
    Historical,
    Unavailable,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SectorObjectType {
    Star,
    Planet,
    Asteroid,
    DustCloud,
    BlackHole,
    SolarSystem,
    Manny,
    DriftingItem,
    DetachedContainer,
    DeuteriumRefuelStation,
    ScutRelay,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DangerLevel {
    Low,
    Moderate,
    Extreme,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectorScan {
    pub current_sector_residence_seconds: i64,
    pub required_residence_seconds: i64,
    pub scan_quality: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WaypointBookmarkHistory {
    pub name: String,
    pub player_id: i64,
    pub player_name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WaypointBookmarkTarget {
    pub id: String,
    #[serde(rename = "type")]
    pub object_type: SectorObjectType,
    pub name: Option<String>,
    pub mass: Option<f64>,
    pub mass_unit: Option<String>,
    pub radius: Option<f64>,
    pub radius_unit: Option<String>,
    pub category: Option<String>,
    pub habitability_score: Option<f64>,
    #[serde(default)]
    pub waypoint_bookmarks: Vec<WaypointBookmarkHistory>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectorProbePresence {
    pub id: i64,
    pub name: String,
    pub moving: bool,
}

/// The four mineable-resource values shared by `resourceComposition`
/// (normalized shares) and `resourceAmounts` (remaining reserves). JSON keys
/// are snake_case, so this struct intentionally has no `rename_all`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResourceShares {
    #[serde(default)]
    pub deuterium: f64,
    #[serde(default)]
    pub metals: f64,
    #[serde(default)]
    pub ice: f64,
    #[serde(default)]
    pub carbon_compounds: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectorObject {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub object_type: SectorObjectType,
    pub name: Option<String>,
    pub estimated: Option<bool>,
    pub summary: Option<String>,
    pub mass: Option<f64>,
    pub mass_unit: Option<String>,
    pub radius: Option<f64>,
    pub radius_unit: Option<String>,
    pub danger_level: Option<DangerLevel>,
    pub habitability_score: Option<f64>,
    /// Planet classification (present on planets).
    pub category: Option<String>,
    /// Asteroid composition class (iron, silicate, carbonaceous, ice, …).
    pub composition: Option<String>,
    /// True when a Manny can mine this object (planets & asteroids).
    pub manny_mineable: Option<bool>,
    /// Raw sensor material hints (asteroids); prefer resource_types for logic.
    #[serde(default)]
    pub resources: Vec<String>,
    /// Mineable resource types present (planets, asteroids, stations).
    #[serde(default)]
    pub resource_types: Vec<String>,
    /// Normalized mineable-resource shares (sum ≈ 1).
    pub resource_composition: Option<ResourceShares>,
    /// Remaining reserves in equivalent earth containers (asteroids).
    pub resource_amounts: Option<ResourceShares>,
    /// Solar-system body counts.
    pub star_count: Option<i64>,
    pub planet_count: Option<i64>,
    pub orbital_body_count: Option<i64>,
    pub salvageable: Option<bool>,
    pub manny_state: Option<String>,
    pub manny_uid: Option<String>,
    pub cargo: Option<MannyCargo>,
    pub item_type: Option<String>,
    pub quantity: Option<i64>,
    pub container_space: Option<f64>,
    pub mode: Option<String>,
    pub target_object_id: Option<String>,
    pub capacity: Option<f64>,
    pub capacity_unit: Option<String>,
    pub minable_targets: Option<Vec<MinableTarget>>,
    // SCUT relay objects (present only when object_type == ScutRelay).
    pub status: Option<ScutRelayStatus>,
    pub coverage_radius_sectors: Option<i64>,
    pub created_by_probe_name: Option<String>,
    pub activated_at: Option<String>,
    pub network: Option<ScutNetworkReference>,
    #[serde(default)]
    pub waypoint_bookmarks: Vec<WaypointBookmarkHistory>,
    #[serde(default)]
    pub bookmark_targets: Vec<WaypointBookmarkTarget>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScutRelayStatus {
    Off,
    On,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScutNetworkReference {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScutRelay {
    pub id: i64,
    pub name: String,
    pub status: ScutRelayStatus,
    pub sector: ProbeSector,
    pub created_by_probe_name: Option<String>,
    pub coverage_radius_sectors: i64,
    pub activated_at: Option<String>,
    pub network: Option<ScutNetworkReference>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScutNetworkProbe {
    pub id: i64,
    pub name: String,
    pub sector: ProbeSector,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScutNetwork {
    pub id: i64,
    pub name: String,
    pub relay_count: i64,
    pub covered_sector_count: i64,
    #[serde(default)]
    pub relays: Vec<ScutRelay>,
    #[serde(default)]
    pub probes: Vec<ScutNetworkProbe>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MinableTarget {
    pub id: String,
    #[serde(rename = "type")]
    pub object_type: SectorObjectType,
    pub name: Option<String>,
    pub mass: Option<f64>,
    pub resource_types: Option<Vec<String>>,
    pub resource_amounts: Option<ResourceShares>,
    pub resource_composition: Option<ResourceShares>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectorObservation {
    pub relative_coordinates: Vector,
    pub distance: i64,
    pub knowledge_level: KnowledgeLevel,
    pub confidence: f64,
    pub objects: Option<Vec<SectorObject>>,
    pub probes: Option<Vec<SectorProbePresence>>,
    pub possible_objects: Option<Vec<String>>,
    pub estimated_objects: Option<EstimatedObjects>,
    pub navigational_risk: Option<String>,
    pub message: Option<String>,
    pub sensor_mode: Option<SensorMode>,
    pub data_freshness: Option<DataFreshness>,
    #[serde(default)]
    pub scut_networks: Vec<ScutNetworkReference>,
    pub scan: SectorScan,
    /// Local timestamp of when this observation was received (not an API
    /// field — stamped client-side and persisted in scan_history.json).
    #[serde(default)]
    pub scanned_at: Option<DateTime<Utc>>,
}
