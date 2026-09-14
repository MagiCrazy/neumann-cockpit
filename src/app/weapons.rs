//! Firing a missile (API v125, issue #361).
//!
//! Every other wizard in the cockpit spends the pilot's own resources. This one
//! damages someone else's, and v121 caps damage at the target's remaining
//! integrity with any drop to 0 % marking it `dead` immediately. So the
//! confirmation is not ceremony: it names the target, says what it is, and says
//! plainly when it belongs to another player.
//!
//! The **target list is the design**. `targetId` is an opaque public id and the
//! server's `targetKind` enum says what it considers targetable — `probe`,
//! `manny`, `missile`, `motorized_asteroid`, `others_ship`, `others_auxiliary`
//! — a genuinely different candidate set from every other Manny action, which
//! targets rocks and containers in the local sector.
//!
//! What the cockpit can actually offer is narrower than that enum, and for a
//! structural reason: a detected **probe** is not a sector *object*. It arrives
//! in `SectorObservation.probes`, a separate list with no `SectorObjectType`,
//! so it cannot be an entry in the object menu this action hangs off. Firing at
//! another player's probe therefore waits for probes to become addressable
//! entries in the Sector pane — its own piece of work. Everything the server
//! calls targetable *and* the scan exposes as an object is offered here.

use super::*;
use crate::api::types::{ObservedClass, SectorObject, SectorObjectType};

/// What a missile would be aimed at, resolved from the scanned object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissileTarget {
    OthersShip,
    /// Another missile in flight: an interception attempt.
    Missile,
    MotorizedAsteroid,
    /// A Manny that is not on the piloted probe's roster.
    ForeignManny,
}

impl MissileTarget {
    pub fn label(&self) -> &'static str {
        match self {
            MissileTarget::OthersShip => "Others ship",
            MissileTarget::Missile => "missile in flight",
            MissileTarget::MotorizedAsteroid => "motorized asteroid",
            MissileTarget::ForeignManny => "Manny",
        }
    }

    /// Whether the target belongs to another **player** — which is what earns
    /// the reinforced warning. An Others ship is the NPC faction and ordinary
    /// play; a rock is a rock; a missile is already someone's mistake.
    pub fn is_another_players(&self) -> bool {
        matches!(self, MissileTarget::ForeignManny)
    }
}

/// An object carrying a ship classification is an Others ship, whatever its
/// `type` says.
///
/// The `SectorObjectType` enum has no ship value at all, while `observedClass`
/// is documented as "present on detected missiles and Others ships" — so the
/// classification is the evidence and the type is not. Reading it the other way
/// round would mean inventing a mapping the spec does not state.
fn is_others_ship(o: &SectorObject) -> bool {
    matches!(o.observed_class, Some(ObservedClass::Ship | ObservedClass::LargeShip))
}

impl AppState {
    /// The inventory item id of a missile ready to fire, if the probe holds
    /// one.
    ///
    /// `missileItemId` is optional on the endpoint — omitted, the server takes
    /// the first available missile — and the server exposes exactly one missile
    /// item type, so the cockpit never asks the pilot which. It only needs to
    /// know whether there *is* one, to disable the action with a reason rather
    /// than spend a request learning there is not. A second missile kind would
    /// turn this into a picker step.
    pub fn missile_in_inventory(&self) -> bool {
        self.probe
            .as_ref()
            .is_some_and(|p| p.inventory.items.iter().any(|i| i.item_type == "missile"))
    }

    /// What a missile fired at this object would be hitting, or `None` when the
    /// server would not consider it a target.
    pub fn missile_target_kind(&self, object_id: &str) -> Option<MissileTarget> {
        let objects = self.probe_current_sector_scan()?.objects.as_ref()?;
        let o = objects.iter().find(|o| o.id.as_deref() == Some(object_id))?;
        if is_others_ship(o) {
            return Some(MissileTarget::OthersShip);
        }
        match o.object_type {
            SectorObjectType::Missile => Some(MissileTarget::Missile),
            SectorObjectType::Asteroid if o.motorized == Some(true) => Some(MissileTarget::MotorizedAsteroid),
            // Our own crew is on the roster; a Manny that is not is someone
            // else's, and shooting one is an act against a player.
            SectorObjectType::Manny
                if !self
                    .mannies
                    .iter()
                    .flatten()
                    .any(|m| Some(m.id.as_str()) == o.id.as_deref()) =>
            {
                Some(MissileTarget::ForeignManny)
            }
            _ => None,
        }
    }

    /// Why firing at this object is unavailable, if it is. `None` means it can
    /// be offered.
    pub fn fire_missile_block_reason(&self, object_id: &str) -> Option<&'static str> {
        self.missile_target_kind(object_id)?;
        if self.probe_id().is_none() {
            // The endpoint exists only on the {probeId} mirror.
            return Some("no probe sync yet");
        }
        (!self.missile_in_inventory()).then_some("no missile in inventory")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_seeing(objects: &str, items: &str) -> AppState {
        let mut s = AppState::default();
        s.probe = Some(
            serde_json::from_str(&format!(
                r#"{{"id": 1, "name": "t", "status": "idle",
                     "fuel": {{"deuterium": 50.0}}, "sensorMode": "normal",
                     "sector": {{"relative": {{"x": 0.0, "y": 0.0, "z": 0.0}}}}, "movement": null,
                     "systems": {{"integrityPercent": 100.0}},
                     "inventory": {{"capacity": 10.0, "usedCapacity": 1.0, "freeCapacity": 9.0,
                                    "items": [{items}], "resourceStocks": [],
                                    "externalTanks": [], "containers": []}}}}"#
            ))
            .unwrap(),
        );
        s.scan_history = vec![serde_json::from_str(&format!(
            r#"{{"relativeCoordinates": {{"x": 0.0, "y": 0.0, "z": 0.0}}, "distance": 0,
                 "knowledgeLevel": "detailed", "confidence": 1.0, "objects": [{objects}],
                 "scan": {{"currentSectorResidenceSeconds": 60,
                           "requiredResidenceSeconds": 60, "scanQuality": 1.0}}}}"#
        ))
        .unwrap()];
        s
    }

    const MISSILE_ITEM: &str = r#"{"id": "itm1", "type": "missile", "name": "Missile", "containerSpace": 0.05,
            "currentTask": null, "taskProgressPercent": null}"#;

    #[test]
    fn the_server_targetable_kinds_the_scan_exposes_are_offered() {
        let s = state_seeing(
            r#"{"id": "ship", "type": "manny", "name": "contact", "summary": "s",
                "observedClass": "large_ship"},
               {"id": "msl", "type": "missile", "name": "m", "summary": "s"},
               {"id": "rock", "type": "asteroid", "name": "r", "summary": "s", "motorized": true},
               {"id": "mny", "type": "manny", "name": "stranger", "summary": "s"}"#,
            MISSILE_ITEM,
        );
        assert_eq!(s.missile_target_kind("ship"), Some(MissileTarget::OthersShip));
        assert_eq!(s.missile_target_kind("msl"), Some(MissileTarget::Missile));
        assert_eq!(s.missile_target_kind("rock"), Some(MissileTarget::MotorizedAsteroid));
        assert_eq!(s.missile_target_kind("mny"), Some(MissileTarget::ForeignManny));
    }

    #[test]
    fn a_ship_classification_outranks_the_type_that_carries_it() {
        // `SectorObjectType` has no ship value at all, so `observedClass` is
        // the evidence; reading it the other way round would invent a mapping.
        let s = state_seeing(
            r#"{"id": "ship", "type": "manny", "name": "c", "summary": "s", "observedClass": "ship"}"#,
            MISSILE_ITEM,
        );
        assert_eq!(s.missile_target_kind("ship"), Some(MissileTarget::OthersShip));
    }

    #[test]
    fn an_ordinary_rock_is_not_a_target() {
        let s = state_seeing(
            r#"{"id": "rock", "type": "asteroid", "name": "r", "summary": "s"},
               {"id": "star", "type": "star", "name": "s", "summary": "s"}"#,
            MISSILE_ITEM,
        );
        assert!(s.missile_target_kind("rock").is_none(), "an unmotorized asteroid");
        assert!(s.missile_target_kind("star").is_none());
    }

    #[test]
    fn our_own_crew_is_never_a_target() {
        let mut s = state_seeing(
            r#"{"id": "m1", "type": "manny", "name": "ours", "summary": "s"}"#,
            MISSILE_ITEM,
        );
        s.mannies = Some(vec![serde_json::from_str(
            r#"{"id":"m1","name":"Manny-1","location":{"type":"probe","sector":null},
                "currentTask":null,"taskProgressPercent":0.0,
                "cargo":{"capacity":0.3,"deuterium":0.0,"metals":0.0,"ice":0.0,"organicCompounds":0.0},
                "canReceiveOrders":true,"taskEstimatedEndTime":null}"#,
        )
        .unwrap()]);
        assert!(s.missile_target_kind("m1").is_none(), "that is our Manny");
    }

    #[test]
    fn only_a_player_target_earns_the_reinforced_warning() {
        assert!(MissileTarget::ForeignManny.is_another_players());
        // The NPC faction is ordinary play; a rock is a rock; a missile in
        // flight is already someone's mistake.
        assert!(!MissileTarget::OthersShip.is_another_players());
        assert!(!MissileTarget::MotorizedAsteroid.is_another_players());
        assert!(!MissileTarget::Missile.is_another_players());
    }

    #[test]
    fn an_empty_magazine_is_a_reason_not_a_request() {
        let s = state_seeing(r#"{"id": "msl", "type": "missile", "name": "m", "summary": "s"}"#, "");
        assert_eq!(s.fire_missile_block_reason("msl"), Some("no missile in inventory"));
        // And an object the server would refuse is not offered at all, which is
        // a different thing from being offered and disabled.
        assert!(s.fire_missile_block_reason("nothing").is_none());
    }
}
