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
    pub rules: StorageContainerRules,
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
    pub cargo: MannyCargo,
    pub can_receive_orders: bool,
    pub task_estimated_end_time: Option<DateTime<Utc>>,
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
    #[serde(default)]
    pub waypoint_bookmarks: Vec<WaypointBookmarkHistory>,
    #[serde(default)]
    pub bookmark_targets: Vec<WaypointBookmarkTarget>,
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
    pub scan: SectorScan,
    /// Local timestamp of when this observation was received (not an API
    /// field — stamped client-side and persisted in scan_history.json).
    #[serde(default)]
    pub scanned_at: Option<DateTime<Utc>>,
}
