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
    Returning,
    WaitingForSpace,
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

// ── Inventory ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MannyCargo {
    pub capacity: f64,
    pub deuterium: f64,
    pub metals: f64,
    pub other: f64,
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

#[derive(Debug, Clone, Deserialize)]
pub struct ManniesResponse {
    pub mannies: Vec<Manny>,
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
pub struct SectorObject {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub object_type: SectorObjectType,
    pub name: Option<String>,
    pub estimated: Option<bool>,
    pub summary: String,
    pub mass: Option<f64>,
    pub radius: Option<f64>,
    pub danger_level: Option<DangerLevel>,
    pub manny_state: Option<String>,
    pub manny_uid: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectorObservation {
    pub relative_coordinates: Vector,
    pub distance: i64,
    pub knowledge_level: KnowledgeLevel,
    pub confidence: f64,
    pub objects: Option<Vec<SectorObject>>,
    pub possible_objects: Option<Vec<String>>,
    pub estimated_objects: Option<EstimatedObjects>,
    pub navigational_risk: Option<String>,
    pub message: Option<String>,
    pub sensor_mode: Option<SensorMode>,
    pub data_freshness: Option<DataFreshness>,
    pub scan: SectorScan,
}
