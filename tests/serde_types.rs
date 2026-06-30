use neumann_cockpit::api::types::{
    AlertPhase, AlertStatus, AlertType, ContainerInventory, CraftingRecipe, DamageWarningRule,
    DataFreshness, KnowledgeLevel, Manny, MannyLocationType, MannyTask, Mission, MissionStatus,
    MissionStepStatus, MovementPhase, Probe,
    ProbeAlert, ProbeInventory, ProbeMovement, ProbeStatus, SectorObjectType, SectorObservation,
    SensorMode, StorageContainer,
};

// ── helpers ───────────────────────────────────────────────────────────────────

fn deser<'de, T: serde::Deserialize<'de>>(json: &'de str) -> T {
    serde_json::from_str(json).expect("deserialization failed")
}

// ── Probe ─────────────────────────────────────────────────────────────────────

const PROBE_JSON: &str = r#"{
  "id": 42,
  "name": "Von Neumann #42",
  "status": "idle",
  "fuel": { "deuterium": 85.5 },
  "sensorMode": "normal",
  "sector": { "relative": { "x": 2.0, "y": 0.0, "z": -2.0 } },
  "movement": null,
  "systems": {
    "integrityPercent": 95.0,
    "damagePercent": 5.0,
    "energyStored": 100.0,
    "internalClockRate": 1.0,
    "currentTask": null
  },
  "inventory": {
    "capacity": 10.0,
    "usedCapacity": 3.0,
    "freeCapacity": 7.0,
    "items": [],
    "resourceStocks": [],
    "externalTanks": [],
    "containers": []
  }
}"#;

#[test]
fn probe_basic_fields() {
    let probe: Probe = deser(PROBE_JSON);
    assert_eq!(probe.id, 42);
    assert_eq!(probe.name, "Von Neumann #42");
    assert_eq!(probe.status, ProbeStatus::Idle);
    assert_eq!(probe.sensor_mode, SensorMode::Normal);
    assert_eq!(probe.fuel.deuterium, Some(85.5));
    assert!(probe.movement.is_none());
}

#[test]
fn probe_sector_coords_parsed() {
    let probe: Probe = deser(PROBE_JSON);
    let rel = probe.sector.unwrap().relative.unwrap();
    assert_eq!(rel.x, 2.0);
    assert_eq!(rel.y, 0.0);
    assert_eq!(rel.z, -2.0);
}

#[test]
fn probe_inventory_capacity() {
    let probe: Probe = deser(PROBE_JSON);
    assert_eq!(probe.inventory.capacity, 10.0);
    assert_eq!(probe.inventory.free_capacity, 7.0);
    assert!(probe.inventory.items.is_empty());
}

#[test]
fn probe_unknown_status_fallback() {
    let json = PROBE_JSON.replace("\"idle\"", "\"warp_speed\"");
    let probe: Probe = deser(&json);
    assert_eq!(probe.status, ProbeStatus::Unknown);
}

// ── ProbeMovement ─────────────────────────────────────────────────────────────

const MOVEMENT_JSON: &str = r#"{
  "status": "cruising",
  "origin": { "x": 0.0, "y": 0.0, "z": 0.0 },
  "target": { "x": 2.0, "y": 0.0, "z": -2.0 },
  "distance": 3,
  "fuelCostDeuterium": 2.5,
  "startedAt": "2024-01-01T12:00:00Z",
  "arrivalAt": "2024-01-01T13:00:00Z",
  "phase": "cruising",
  "secondsRemaining": 3600,
  "sensorMode": "degraded",
  "estimatedVelocityC": 0.1
}"#;

#[test]
fn movement_basic_fields() {
    let mv: ProbeMovement = deser(MOVEMENT_JSON);
    assert_eq!(mv.status, MovementPhase::Cruising);
    assert_eq!(mv.distance, 3);
    assert_eq!(mv.fuel_cost_deuterium, 2.5);
    assert_eq!(mv.seconds_remaining, Some(3600));
    assert_eq!(mv.sensor_mode, Some(SensorMode::Degraded));
    assert_eq!(mv.estimated_velocity_c, Some(0.1));
}

#[test]
fn movement_target_coords() {
    let mv: ProbeMovement = deser(MOVEMENT_JSON);
    assert_eq!(mv.target.x, 2.0);
    assert_eq!(mv.target.y, 0.0);
    assert_eq!(mv.target.z, -2.0);
}

// ── Manny ─────────────────────────────────────────────────────────────────────

const MANNY_IDLE_JSON: &str = r#"{
  "id": "manny-abc123",
  "name": "Manny-1",
  "location": { "type": "probe", "sector": null },
  "currentTask": null,
  "taskProgressPercent": 0.0,
  "cargo": {
    "capacity": 0.3,
    "deuterium": 0.0,
    "metals": 0.0,
    "ice": 0.0,
    "organicCompounds": 0.0
  },
  "canReceiveOrders": true,
  "taskEstimatedEndTime": null
}"#;

const MANNY_MINING_JSON: &str = r#"{
  "id": "manny-xyz789",
  "name": "Manny-2",
  "location": { "type": "sector", "sector": null },
  "currentTask": "mining",
  "taskProgressPercent": 42.0,
  "cargo": {
    "capacity": 0.3,
    "deuterium": 0.0,
    "metals": 0.15,
    "ice": 0.0,
    "organicCompounds": 0.0
  },
  "canReceiveOrders": false,
  "taskEstimatedEndTime": "2024-01-01T14:00:00Z"
}"#;

#[test]
fn manny_idle_in_probe() {
    let m: Manny = deser(MANNY_IDLE_JSON);
    assert_eq!(m.id, "manny-abc123");
    assert_eq!(m.location.location_type, MannyLocationType::Probe);
    assert!(m.current_task.is_none());
    assert!(m.can_receive_orders);
    assert_eq!(m.task_progress_percent, 0.0);
}

#[test]
fn manny_mining_in_sector() {
    let m: Manny = deser(MANNY_MINING_JSON);
    assert_eq!(m.location.location_type, MannyLocationType::Sector);
    assert_eq!(m.current_task, Some(MannyTask::Mining));
    assert!(!m.can_receive_orders);
    assert_eq!(m.task_progress_percent, 42.0);
    assert_eq!(m.cargo.metals, 0.15);
    assert!(m.task_estimated_end_time.is_some());
}

#[test]
fn manny_unknown_task_fallback() {
    let json = MANNY_MINING_JSON.replace("\"mining\"", "\"quantum_leap\"");
    let m: Manny = deser(&json);
    assert_eq!(m.current_task, Some(MannyTask::Unknown));
}

// ── ProbeInventory ────────────────────────────────────────────────────────────

const INVENTORY_JSON: &str = r#"{
  "capacity": 10.0,
  "usedCapacity": 2.5,
  "freeCapacity": 7.5,
  "items": [
    {
      "id": "item-1",
      "type": "manny",
      "name": "Manny-1",
      "containerSpace": 1.0,
      "currentTask": null,
      "taskProgressPercent": 0.0,
      "location": { "type": "probe", "sector": null },
      "cargo": null,
      "container": null
    }
  ],
  "resourceStocks": [
    {
      "id": "stock-metals",
      "type": "metals",
      "name": "Metals",
      "amount": 0.5,
      "containerSpace": 0.5,
      "containers": []
    }
  ],
  "externalTanks": [],
  "containers": []
}"#;

#[test]
fn inventory_items_and_stocks() {
    let inv: ProbeInventory = deser(INVENTORY_JSON);
    assert_eq!(inv.free_capacity, 7.5);
    assert_eq!(inv.items.len(), 1);
    assert_eq!(inv.items[0].item_type, "manny");
    assert_eq!(inv.resource_stocks.len(), 1);
    assert_eq!(inv.resource_stocks[0].stock_type, "metals");
    assert_eq!(inv.resource_stocks[0].amount, 0.5);
}

// ── CraftingRecipe ────────────────────────────────────────────────────────────

const RECIPE_MANNY_JSON: &str = r#"{
  "id": "storage-container",
  "name": "Storage Container",
  "craftableBy": ["manny"],
  "ingredients": [
    { "type": "resource", "quantity": 0.1, "unit": "ECE", "kind": "metals" }
  ],
  "durationSeconds": 300,
  "output": {
    "type": "storage_container",
    "name": "Storage Container",
    "containerSpace": 1.0,
    "containerSpaceUnit": "ECE",
    "capacityBonus": 0.5
  }
}"#;

const RECIPE_PRINTER_JSON: &str = r#"{
  "id": "advanced-container",
  "name": "Advanced Container",
  "craftableBy": ["atomic_3d_printer"],
  "ingredients": [
    { "type": "resource", "quantity": 0.2, "unit": "ECE", "kind": "metals" },
    { "type": "resource", "quantity": 0.1, "unit": "ECE", "kind": "carbon_compounds" }
  ],
  "durationSeconds": 600,
  "output": {
    "type": "storage_container",
    "name": "Advanced Container",
    "containerSpace": 2.0,
    "containerSpaceUnit": "ECE",
    "capacityBonus": 1.0
  }
}"#;

#[test]
fn recipe_manny_craftable() {
    let r: CraftingRecipe = deser(RECIPE_MANNY_JSON);
    assert_eq!(r.id, "storage-container");
    assert!(r.craftable_by.contains(&"manny".to_string()));
    assert_eq!(r.ingredients.len(), 1);
    assert_eq!(r.ingredients[0].quantity, 0.1);
    assert_eq!(r.duration_seconds, 300);
    assert_eq!(r.output.container_space, 1.0);
    assert_eq!(r.output.capacity_bonus, Some(0.5));
}

#[test]
fn recipe_atomic_printer_craftable() {
    let r: CraftingRecipe = deser(RECIPE_PRINTER_JSON);
    assert!(r.craftable_by.contains(&"atomic_3d_printer".to_string()));
    assert_eq!(r.ingredients.len(), 2);
}

// ── SectorObservation ─────────────────────────────────────────────────────────

const SECTOR_JSON: &str = r#"{
  "relativeCoordinates": { "x": 2.0, "y": 0.0, "z": -2.0 },
  "distance": 3,
  "knowledgeLevel": "detailed",
  "confidence": 1.0,
  "objects": [
    {
      "id": "asteroid-1",
      "type": "asteroid",
      "name": "Rock Alpha",
      "estimated": null,
      "summary": null,
      "mass": 1.5e20,
      "massUnit": "kg",
      "radius": null,
      "radiusUnit": null,
      "dangerLevel": null,
      "salvageable": null,
      "mannyState": null,
      "mannyUid": null,
      "cargo": null,
      "itemType": null,
      "quantity": null,
      "containerSpace": null,
      "mode": null,
      "targetObjectId": null,
      "capacity": null,
      "capacityUnit": null,
      "minableTargets": [
        {
          "id": "asteroid-1",
          "type": "asteroid",
          "name": "Rock Alpha",
          "mass": 1.5e20,
          "resourceTypes": ["metals", "ice"]
        }
      ],
      "waypointBookmarks": [],
      "bookmarkTargets": []
    },
    {
      "id": "manny-field-1",
      "type": "manny",
      "name": "Manny-1",
      "estimated": null,
      "summary": null,
      "mass": null,
      "massUnit": null,
      "radius": null,
      "radiusUnit": null,
      "dangerLevel": null,
      "salvageable": true,
      "mannyState": "idle",
      "mannyUid": "manny-abc123",
      "cargo": null,
      "itemType": null,
      "quantity": null,
      "containerSpace": null,
      "mode": null,
      "targetObjectId": null,
      "capacity": null,
      "capacityUnit": null,
      "minableTargets": null,
      "waypointBookmarks": [],
      "bookmarkTargets": []
    }
  ],
  "probes": [{ "id": 1, "name": "Von Neumann #1", "moving": false }],
  "possibleObjects": null,
  "estimatedObjects": null,
  "navigationalRisk": null,
  "message": null,
  "sensorMode": "normal",
  "dataFreshness": "live",
  "scan": {
    "currentSectorResidenceSeconds": 120,
    "requiredResidenceSeconds": 60,
    "scanQuality": 1.0
  }
}"#;

#[test]
fn sector_coordinates_and_meta() {
    let s: SectorObservation = deser(SECTOR_JSON);
    assert_eq!(s.relative_coordinates.x, 2.0);
    assert_eq!(s.relative_coordinates.z, -2.0);
    assert_eq!(s.distance, 3);
    assert_eq!(s.knowledge_level, KnowledgeLevel::Detailed);
    assert_eq!(s.confidence, 1.0);
    assert_eq!(s.sensor_mode, Some(SensorMode::Normal));
    assert_eq!(s.data_freshness, Some(DataFreshness::Live));
}

#[test]
fn sector_objects_types() {
    let s: SectorObservation = deser(SECTOR_JSON);
    let objects = s.objects.as_ref().unwrap();
    assert_eq!(objects.len(), 2);
    assert_eq!(objects[0].object_type, SectorObjectType::Asteroid);
    assert_eq!(objects[1].object_type, SectorObjectType::Manny);
}

#[test]
fn sector_minable_targets() {
    let s: SectorObservation = deser(SECTOR_JSON);
    let objects = s.objects.as_ref().unwrap();
    let targets = objects[0].minable_targets.as_ref().unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].id, "asteroid-1");
    let resources = targets[0].resource_types.as_ref().unwrap();
    assert!(resources.contains(&"metals".to_string()));
    assert!(resources.contains(&"ice".to_string()));
}

#[test]
fn sector_probes_list() {
    let s: SectorObservation = deser(SECTOR_JSON);
    let probes = s.probes.as_ref().unwrap();
    assert_eq!(probes.len(), 1);
    assert_eq!(probes[0].id, 1);
    assert!(!probes[0].moving);
}

#[test]
fn sector_scan_quality() {
    let s: SectorObservation = deser(SECTOR_JSON);
    assert_eq!(s.scan.scan_quality, 1.0);
    assert_eq!(s.scan.current_sector_residence_seconds, 120);
}

#[test]
fn sector_unknown_knowledge_fallback() {
    let json = SECTOR_JSON.replace("\"detailed\"", "\"alien_tech\"");
    let s: SectorObservation = deser(&json);
    assert_eq!(s.knowledge_level, KnowledgeLevel::Unknown);
}

// ── Alerts & damage warnings ────────────────────────────────────────────────
// Payloads copied verbatim from the OpenAPI examples (api-specs/v44.yaml).

const ALERTS_JSON: &str = r#"{
  "alerts": [
    {
      "id": 7,
      "type": "storage_container_break",
      "status": "unread",
      "message": "movement stress can break one container link",
      "phase": "acceleration_end",
      "scheduledAt": "2026-06-12T12:03:00+00:00",
      "sector": { "relative": { "x": 0, "y": 0, "z": 0 } },
      "container": { "id": "cnt_extra_3", "label": "Additional container 3", "objectId": "detached-container-7" },
      "risk": { "percent": 30, "additionalContainerCount": 7, "ruleStartsAtAdditionalContainers": 5 },
      "createdAt": "2026-06-12T12:00:00+00:00",
      "updatedAt": "2026-06-12T12:00:00+00:00",
      "readAt": null,
      "resolvedAt": null
    },
    {
      "id": 8,
      "type": "intelligent_life",
      "status": "read",
      "message": "Intelligent life detected on Pale Signal.",
      "phase": "arrival",
      "scheduledAt": "2026-06-12T14:42:00+00:00",
      "sector": { "relative": { "x": 2, "y": 0, "z": 0 } },
      "planet": { "id": "planet_4", "name": "Pale Signal" },
      "createdAt": "2026-06-12T14:42:00+00:00",
      "updatedAt": "2026-06-12T14:42:00+00:00",
      "readAt": "2026-06-12T14:45:00+00:00",
      "resolvedAt": null
    },
    {
      "id": 9,
      "type": "sector_object_detected",
      "status": "unread",
      "message": "A new object was detected in the sector.",
      "phase": "detection",
      "scheduledAt": "2026-06-12T15:00:00+00:00",
      "sector": { "relative": { "x": 0, "y": 0, "z": 0 } },
      "object": { "id": "debug-deuterium-a9f0", "type": "asteroid", "label": "Deuterium asteroid", "resourceTypes": ["deuterium"] },
      "createdAt": "2026-06-12T15:00:00+00:00",
      "updatedAt": "2026-06-12T15:00:00+00:00",
      "readAt": null,
      "resolvedAt": null
    }
  ],
  "rules": {
    "storageContainerBreak": {
      "type": "storage_container_break",
      "startsAtAdditionalContainers": 5,
      "riskPerAdditionalContainerAfterFourPercent": 10,
      "maximumRiskPercent": 100,
      "message": "rule text"
    }
  }
}"#;

const DAMAGE_WARNINGS_JSON: &str = r#"{
  "damageWarnings": [
    {
      "id": 7,
      "type": "storage_container_break",
      "status": "unread",
      "message": "Risk is 30% for this jump.",
      "phase": "acceleration_end",
      "scheduledAt": "2026-06-12T12:03:00+00:00",
      "sector": { "relative": { "x": 0, "y": 0, "z": 0 } },
      "container": { "id": "cnt_extra_3", "label": "Additional container 3", "objectId": "detached-container-7" },
      "risk": { "percent": 30, "additionalContainerCount": 7 },
      "createdAt": "2026-06-12T12:00:00+00:00",
      "updatedAt": "2026-06-12T12:00:00+00:00",
      "readAt": null,
      "resolvedAt": null
    }
  ],
  "rule": {
    "type": "storage_container_break",
    "startsAtAdditionalContainers": 5,
    "riskPerAdditionalContainerAfterFourPercent": 10,
    "maximumRiskPercent": 100,
    "message": "rule text"
  }
}"#;

#[test]
fn alerts_response_deser() {
    let r: AlertsResponseProbe = deser(ALERTS_JSON);
    assert_eq!(r.alerts.len(), 3);
    assert_eq!(r.alerts[0].id, 7);
    assert_eq!(r.alerts[0].alert_type, AlertType::StorageContainerBreak);
    assert_eq!(r.alerts[0].status, AlertStatus::Unread);
    assert!(r.alerts[0].is_unread());
    assert_eq!(r.alerts[0].risk.as_ref().unwrap().percent, 30);
    assert_eq!(r.alerts[0].container.as_ref().unwrap().object_id.as_deref(), Some("detached-container-7"));
    // read alert is no longer unread
    assert_eq!(r.alerts[1].alert_type, AlertType::IntelligentLife);
    assert!(!r.alerts[1].is_unread());
    assert_eq!(r.alerts[1].planet.as_ref().unwrap().name.as_deref(), Some("Pale Signal"));
    // object-detected carries resource types
    assert_eq!(r.alerts[2].object.as_ref().unwrap().resource_types, vec!["deuterium"]);
}

#[test]
fn damage_warnings_response_deser() {
    let r: DamageWarningsResponseProbe = deser(DAMAGE_WARNINGS_JSON);
    assert_eq!(r.damage_warnings.len(), 1);
    assert_eq!(r.damage_warnings[0].phase, AlertPhase::AccelerationEnd);
    assert_eq!(r.rule.starts_at_additional_containers, Some(5));
    assert_eq!(r.rule.maximum_risk_percent, Some(100));
}

#[test]
fn alert_unknown_type_fallback() {
    let json = ALERTS_JSON.replace("storage_container_break\",\n      \"status\": \"unread", "supernova_imminent\",\n      \"status\": \"unread");
    let r: AlertsResponseProbe = deser(&json);
    assert_eq!(r.alerts[0].alert_type, AlertType::Unknown);
}

// Local mirrors of the private response envelopes (client.rs declares them
// inline), so the test can assert the public types deserialize the real shape.
#[derive(serde::Deserialize)]
struct AlertsResponseProbe {
    alerts: Vec<ProbeAlert>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DamageWarningsResponseProbe {
    damage_warnings: Vec<ProbeAlert>,
    rule: DamageWarningRule,
}

// ── Storage containers ──────────────────────────────────────────────────────
// Shape per api-specs/v44.yaml StorageContainerInventoryResponse + StorageContainer.

const CONTAINER_DETAIL_JSON: &str = r#"{
  "container": {
    "id": "container-itm_extra",
    "kind": "container",
    "label": "Soute metaux",
    "sortOrder": 2,
    "capacity": 1.0,
    "usedCapacity": 0.4,
    "freeCapacity": 0.6,
    "capacityUnit": "earth_container_equivalent",
    "rules": { "priority": ["metals"], "exclusion": ["ice"], "strictExclusion": ["manny"] }
  },
  "inventory": {
    "capacityUnit": "earth_container_equivalent",
    "items": [],
    "resourceStocks": [
      { "id": "stock-metals", "type": "metals", "name": "Metals", "amount": 0.4, "containerSpace": 0.4, "containers": [] }
    ]
  }
}"#;

#[test]
fn container_detail_deser() {
    #[derive(serde::Deserialize)]
    struct Resp {
        container: StorageContainer,
        inventory: ContainerInventory,
    }
    let r: Resp = deser(CONTAINER_DETAIL_JSON);
    assert_eq!(r.container.id, "container-itm_extra");
    assert_eq!(r.container.kind, "container");
    assert_eq!(r.container.capacity_unit.as_deref(), Some("earth_container_equivalent"));
    assert_eq!(r.container.rules.priority, vec!["metals"]);
    assert_eq!(r.container.rules.strict_exclusion, vec!["manny"]);
    assert_eq!(r.inventory.resource_stocks.len(), 1);
    assert_eq!(r.inventory.resource_stocks[0].amount, 0.4);
    assert!(r.inventory.items.is_empty());
}

// ── v48–v62 telemetry catch-up enum variants ───────────────────────────────────

#[test]
fn new_manny_tasks_deserialize() {
    assert_eq!(
        deser::<MannyTask>(r#""refilling_deuterium_tank""#),
        MannyTask::RefillingDeuteriumTank
    );
    assert_eq!(
        deser::<MannyTask>(r#""turning_on_scut_relay""#),
        MannyTask::TurningOnScutRelay
    );
    assert_eq!(deser::<MannyTask>(r#""unknown_too_far""#), MannyTask::UnknownTooFar);
}

#[test]
fn anomaly_alert_type_deserializes() {
    assert_eq!(deser::<AlertType>(r#""anomaly_detected""#), AlertType::AnomalyDetected);
}

#[test]
fn new_sector_object_types_deserialize() {
    assert_eq!(
        deser::<SectorObjectType>(r#""deuterium_refuel_station""#),
        SectorObjectType::DeuteriumRefuelStation
    );
    assert_eq!(deser::<SectorObjectType>(r#""scut_relay""#), SectorObjectType::ScutRelay);
}

#[test]
fn trapped_probe_status_deserializes() {
    assert_eq!(
        deser::<ProbeStatus>(r#""trapped_by_black_hole""#),
        ProbeStatus::TrappedByBlackHole
    );
}

#[test]
fn dead_probe_with_terminal_alert_deserializes() {
    let json = r#"{
      "id": 7, "name": "Von Neumann #7", "status": "dead",
      "fuel": { "deuterium": 0.0 }, "sensorMode": "blind",
      "sector": null, "movement": null, "systems": null,
      "alert": {
        "type": "mind_snapshot_reassignment_available",
        "severity": "critical",
        "title": "Probe lost",
        "message": "Your probe is gone. Reassign your mind snapshot.",
        "action": { "label": "Reassign", "method": "POST", "endpoint": "/api/probe/mind-snapshot/reassign" }
      },
      "inventory": {
        "capacity": 10.0, "usedCapacity": 0.0, "freeCapacity": 10.0,
        "items": [], "resourceStocks": [], "externalTanks": [], "containers": []
      }
    }"#;
    let probe: Probe = deser(json);
    assert_eq!(probe.status, ProbeStatus::Dead);
    let alert = probe.alert.expect("terminal alert present");
    assert_eq!(alert.severity, "critical");
    assert_eq!(alert.action.endpoint, "/api/probe/mind-snapshot/reassign");
}

#[test]
fn probe_without_alert_defaults_to_none() {
    let probe: Probe = deser(PROBE_JSON);
    assert!(probe.alert.is_none());
}

#[test]
fn mission_with_steps_deserializes() {
    let json = r#"{
      "id": "mission-1",
      "type": "first_contact.return_to_space_program",
      "title": "First contact",
      "description": "Deliver materials to the inhabited planet.",
      "status": "active",
      "stepOrder": "sequential",
      "metadata": {},
      "startedAt": "2026-06-06T12:00:00+00:00",
      "createdAt": "2026-06-06T12:00:00+00:00",
      "updatedAt": "2026-06-06T12:00:00+00:00",
      "steps": [
        {"id": "s2", "sortOrder": 2, "title": "Deliver carbon", "status": "pending", "metadata": {}, "createdAt": "2026-06-06T12:00:00+00:00", "updatedAt": "2026-06-06T12:00:00+00:00"},
        {"id": "s1", "sortOrder": 1, "title": "Deliver metals", "status": "completed", "metadata": {}, "createdAt": "2026-06-06T12:00:00+00:00", "updatedAt": "2026-06-06T12:00:00+00:00"}
      ]
    }"#;
    let m: Mission = deser(json);
    assert_eq!(m.status, MissionStatus::Active);
    assert_eq!(m.mission_type, "first_contact.return_to_space_program");
    assert_eq!(m.steps.len(), 2);
    assert_eq!(m.steps[0].status, MissionStepStatus::Pending);
    assert_eq!(m.steps[1].status, MissionStepStatus::Completed);
}
