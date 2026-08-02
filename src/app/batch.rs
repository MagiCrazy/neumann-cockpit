//! Atomic Manny task batching (`POST …/mannies/tasks`, API v104).
//!
//! Both sequencers fire *groups* of Manny orders in one tick: the script's
//! fan-out (`mine … by all`, the #258 craft fan-out) and a round of production
//! queue lanes. Before v104 that meant N independent requests that could half
//! succeed — three builders dispatched, the fourth rejected, the step already
//! in flight. The batch endpoint applies the whole group in order or none of
//! it, in one round-trip.
//!
//! This module is the pure translation layer: it turns the sequencers' own
//! action types into wire payloads identical to the individual endpoints', and
//! decides when batching applies at all. The event loop keeps the dispatch.

use serde_json::json;

use super::{CraftFire, Fabricator, ScriptAction};
use crate::api::types::MannyTaskRequest;

/// Below this, a batch buys nothing over the direct call it would replace.
const MIN_BATCH: usize = 2;

impl ScriptAction {
    /// This action as a batch entry, or `None` when it is not a Manny task
    /// (travel, an atomic-printer craft) and so cannot ride in a batch.
    pub fn as_batch_task(&self) -> Option<MannyTaskRequest> {
        let (manny_id, task, payload) = match self {
            ScriptAction::Travel { .. } => return None,
            ScriptAction::Mine {
                manny_id,
                object_id,
                resources,
                amount,
                container_id,
            } => {
                let mut payload = json!({
                    "objectId": object_id,
                    "resources": resources,
                    "targetAmount": amount,
                });
                // Mirrors the individual endpoint, which skips the field rather
                // than sending null when the destination is the probe itself.
                if let Some(c) = container_id {
                    payload["targetContainerId"] = json!(c);
                }
                (manny_id, "mine", payload)
            }
            ScriptAction::Repair {
                manny_id,
                integrity_percent,
            } => (manny_id, "repair", json!({ "integrityPercent": integrity_percent })),
            ScriptAction::Salvage { manny_id, object_id } => (manny_id, "salvage", json!({ "objectId": object_id })),
            ScriptAction::Detach {
                manny_id,
                container_id,
                mode,
                object_id,
            } => {
                let mut payload = json!({ "containerId": container_id, "mode": mode });
                if let Some(o) = object_id {
                    payload["objectId"] = json!(o);
                }
                (manny_id, "detach-storage-container", payload)
            }
            ScriptAction::Recover { manny_id, object_id } => {
                (manny_id, "recover-storage-container", json!({ "objectId": object_id }))
            }
            ScriptAction::Craft {
                fabricator: Fabricator::Manny,
                manny_id: Some(manny_id),
                recipe_id,
            } => (manny_id, "craft", json!({ "recipe": recipe_id })),
            // An atomic-printer craft is a probe-level endpoint, not a Manny
            // task, and a builderless Manny craft cannot be addressed at all.
            ScriptAction::Craft { .. } => return None,
        };
        Some(MannyTaskRequest {
            manny_id: manny_id.clone(),
            task,
            payload,
        })
    }
}

impl CraftFire {
    /// This queued craft as a batch entry, or `None` for the atomic printer
    /// (a probe-level endpoint) or a craft with no resolved builder.
    pub fn as_batch_task(&self) -> Option<MannyTaskRequest> {
        match (self.fabricator, &self.builder_manny_id) {
            (Fabricator::Manny, Some(manny_id)) => Some(MannyTaskRequest {
                manny_id: manny_id.clone(),
                task: "craft",
                payload: json!({ "recipe": self.recipe_id }),
            }),
            _ => None,
        }
    }
}

/// Turn a group of staged orders into one atomic batch, or `None` to fall back
/// to firing them individually.
///
/// Batching is refused unless *every* order qualifies: a partial batch plus
/// leftover single calls would give up the atomicity that is the whole point,
/// and leave a half-fired group behind on rejection. It also refuses a repeated
/// Manny, which the server rejects outright, and a single order, which gains
/// nothing.
pub fn batch_tasks<T>(orders: &[T], as_task: impl Fn(&T) -> Option<MannyTaskRequest>) -> Option<Vec<MannyTaskRequest>> {
    if orders.len() < MIN_BATCH {
        return None;
    }
    let tasks: Vec<MannyTaskRequest> = orders.iter().filter_map(&as_task).collect();
    if tasks.len() != orders.len() {
        return None;
    }
    let mut seen: Vec<&str> = Vec::with_capacity(tasks.len());
    for t in &tasks {
        if seen.contains(&t.manny_id.as_str()) {
            return None;
        }
        seen.push(&t.manny_id);
    }
    Some(tasks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mine(manny: &str, container: Option<&str>) -> ScriptAction {
        ScriptAction::Mine {
            manny_id: manny.into(),
            object_id: "ast-1".into(),
            resources: vec!["metals".into()],
            amount: 0.02,
            container_id: container.map(String::from),
        }
    }

    #[test]
    fn a_mine_payload_mirrors_the_individual_endpoint() {
        let t = mine("m1", None).as_batch_task().unwrap();
        assert_eq!(t.manny_id, "m1");
        assert_eq!(t.task, "mine");
        assert_eq!(t.payload["objectId"], "ast-1");
        assert_eq!(t.payload["targetAmount"], 0.02);
        assert!(
            t.payload.get("targetContainerId").is_none(),
            "omitted, not null, when hauling to the probe"
        );

        let with_container = mine("m1", Some("c-9")).as_batch_task().unwrap();
        assert_eq!(with_container.payload["targetContainerId"], "c-9");
    }

    #[test]
    fn travel_and_printer_crafts_never_batch() {
        assert!(ScriptAction::Travel { x: 1, y: 0, z: 1 }.as_batch_task().is_none());
        assert!(ScriptAction::Craft {
            fabricator: Fabricator::AtomicPrinter,
            manny_id: None,
            recipe_id: "integrated_circuit".into(),
        }
        .as_batch_task()
        .is_none());
    }

    #[test]
    fn a_fan_out_of_mannies_batches() {
        let group = vec![mine("m1", None), mine("m2", None), mine("m3", None)];
        let batch = batch_tasks(&group, ScriptAction::as_batch_task).expect("batchable");
        assert_eq!(batch.len(), 3);
        assert_eq!(batch[2].manny_id, "m3");
    }

    #[test]
    fn a_lone_order_is_not_worth_a_batch() {
        assert!(batch_tasks(&[mine("m1", None)], ScriptAction::as_batch_task).is_none());
    }

    #[test]
    fn a_group_with_one_unbatchable_order_falls_back_whole() {
        // Half a batch plus a stray single call would lose the atomicity the
        // batch exists for.
        let group = vec![
            mine("m1", None),
            ScriptAction::Craft {
                fabricator: Fabricator::AtomicPrinter,
                manny_id: None,
                recipe_id: "integrated_circuit".into(),
            },
        ];
        assert!(batch_tasks(&group, ScriptAction::as_batch_task).is_none());
    }

    #[test]
    fn a_repeated_manny_falls_back() {
        // The server rejects a Manny appearing twice in one batch, which would
        // fail the whole group — fire them individually instead.
        let group = vec![mine("m1", None), mine("m1", Some("c-2"))];
        assert!(batch_tasks(&group, ScriptAction::as_batch_task).is_none());
    }

    #[test]
    fn queued_manny_crafts_batch_but_the_printer_does_not() {
        let manny = |id: &str| CraftFire {
            fabricator: Fabricator::Manny,
            builder_manny_id: Some(id.into()),
            recipe_id: "steel_bar".into(),
        };
        let batch = batch_tasks(&[manny("m1"), manny("m2")], CraftFire::as_batch_task).expect("batchable");
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0].task, "craft");
        assert_eq!(batch[0].payload["recipe"], "steel_bar");

        let mixed = vec![
            manny("m1"),
            CraftFire {
                fabricator: Fabricator::AtomicPrinter,
                builder_manny_id: None,
                recipe_id: "micro_conductor".into(),
            },
        ];
        assert!(batch_tasks(&mixed, CraftFire::as_batch_task).is_none());
    }
}
