//! Driving a motorized asteroid (API v108–v116, issue #308).
//!
//! Reading the system landed with #301 phase 1 — a motorized asteroid already
//! renders its trajectory in the Sector pane. This is the other half: giving an
//! asteroid an engine, refuelling it, and aiming it.
//!
//! Three rules from the spec shape everything here, and each one is a
//! constraint the cockpit enforces rather than discovers through a 422:
//!
//! 1. A **sector transfer** takes a *direct FCC-grid neighbour*, and that
//!    neighbour is a **heading**, not a destination — the asteroid keeps going,
//!    one sector every 24 h, until something captures it.
//! 2. A **system impact** takes a local body and a speed in `(0, 0.5] c`, and
//!    the server forbids aiming at a black hole
//!    (`system_impact_black_hole_forbidden`).
//! 3. Motorization **replaces the asteroid's opaque id**, so anything holding
//!    the old one has to re-resolve.

use crate::api::types::{SectorObject, SectorObjectType};

use super::{AppState, ScannerObjectEntry};

/// The twelve nearest neighbours of an FCC lattice point.
///
/// The sector grid is face-centred cubic — which is why travel refuses an odd
/// coordinate sum — so a *direct* neighbour is one face-diagonal step: two axes
/// move by ±1 and the third stays put. Twelve of them, never six; a cockpit
/// that offered six would silently hide half the headings a pilot may take.
pub const FCC_NEIGHBOURS: [(i64, i64, i64); 12] = [
    (1, 1, 0),
    (1, -1, 0),
    (-1, 1, 0),
    (-1, -1, 0),
    (1, 0, 1),
    (1, 0, -1),
    (-1, 0, 1),
    (-1, 0, -1),
    (0, 1, 1),
    (0, 1, -1),
    (0, -1, 1),
    (0, -1, -1),
];

/// The upper bound the server puts on `targetSpeedC`, in fractions of c.
pub const MAX_TARGET_SPEED_C: f64 = 0.5;

/// A heading offered by the sector-transfer step: the neighbour's relative
/// coordinates, plus whatever the fleet already knows about that sector.
#[derive(Debug, Clone, PartialEq)]
pub struct Heading {
    pub x: i64,
    pub y: i64,
    pub z: i64,
    /// `true` when the scan history holds an observation of that sector — the
    /// difference between aiming into the dark and aiming somewhere seen.
    pub visited: bool,
}

impl Heading {
    /// The offset from the probe, which is what the pilot reasons about.
    pub fn delta(&self, from: (i64, i64, i64)) -> (i64, i64, i64) {
        (self.x - from.0, self.y - from.1, self.z - from.2)
    }
}

/// A candidate impact target: a local body the server will accept.
#[derive(Debug, Clone, PartialEq)]
pub struct ImpactTarget {
    pub id: String,
    pub name: String,
    pub object_type: SectorObjectType,
}

impl AppState {
    /// The motorized asteroid behind a scanner entry, if the current sector
    /// scan holds it.
    pub fn motorized_asteroid(&self, id: &str) -> Option<&SectorObject> {
        self.probe_current_sector_scan()
            .and_then(|s| s.objects.as_ref())
            .and_then(|objects| objects.iter().find(|o| o.id.as_deref() == Some(id)))
            .filter(|o| o.motorized == Some(true))
    }

    /// Whether an asteroid is motorized, fuelled and free of a running
    /// trajectory — the exact state the launch endpoint accepts.
    pub fn asteroid_ready_to_launch(&self, id: &str) -> bool {
        self.motorized_asteroid(id).is_some_and(|o| {
            o.motor_fuel_status == Some(crate::api::types::MotorFuelStatus::Full) && !trajectory_active(o)
        })
    }

    /// Whether an asteroid can be refuelled: motorized, empty, and not already
    /// under way. The server refuses a refuel while a trajectory runs.
    pub fn asteroid_needs_fuel(&self, id: &str) -> bool {
        self.motorized_asteroid(id).is_some_and(|o| {
            o.motor_fuel_status == Some(crate::api::types::MotorFuelStatus::Empty) && !trajectory_active(o)
        })
    }

    /// The running trajectory of a local asteroid, if any.
    pub fn asteroid_trajectory_id(&self, id: &str) -> Option<String> {
        self.motorized_asteroid(id)
            .and_then(|o| o.trajectory.as_ref())
            .filter(|t| is_running(t.status))
            .map(|t| t.id.clone())
    }

    /// Whether the pilot knows the Distributed Thrust Anchoring blueprint.
    ///
    /// Motorization needs it, and a menu entry that always answers
    /// `distributed_thrust_anchoring_unavailable` teaches nothing.
    pub fn thrust_anchoring_known(&self) -> bool {
        self.probe_improvements
            .iter()
            .any(|i| i.id == THRUST_ANCHORING_ID && (i.available || i.done))
    }

    /// The bodies a system impact may be aimed at, from the current sector
    /// scan: stars, planets and asteroids — never the asteroid being launched,
    /// and never a black hole, which the server refuses outright
    /// (`system_impact_black_hole_forbidden`).
    ///
    /// `targetObjectId` also accepts a **probe**, and that answers the open
    /// design question about guarding it: a sector scan has no probe object
    /// type at all, so there is nothing here to offer. Aiming at a probe stays
    /// possible on the wire and impossible from this picker, which is the
    /// conservative end of a choice we would otherwise have to make.
    pub fn impact_targets(&self, launching: &str) -> Vec<ImpactTarget> {
        let Some(objects) = self.probe_current_sector_scan().and_then(|s| s.objects.as_ref()) else {
            return Vec::new();
        };
        objects
            .iter()
            .filter_map(|o| {
                let id = o.id.as_deref()?;
                if id == launching {
                    return None;
                }
                if !matches!(
                    o.object_type,
                    SectorObjectType::Star | SectorObjectType::Planet | SectorObjectType::Asteroid
                ) {
                    return None;
                }
                Some(ImpactTarget {
                    id: id.to_string(),
                    name: o.name.clone().unwrap_or_else(|| id.to_string()),
                    object_type: o.object_type.clone(),
                })
            })
            .collect()
    }

    /// The twelve headings a sector transfer may take, in the relative frame,
    /// each flagged with whether the fleet has ever seen that sector.
    pub fn transfer_headings(&self) -> Vec<Heading> {
        let Some((px, py, pz)) = self.probe_relative_coords() else {
            return Vec::new();
        };
        FCC_NEIGHBOURS
            .iter()
            .map(|(dx, dy, dz)| {
                let (x, y, z) = (px + dx, py + dy, pz + dz);
                Heading {
                    x,
                    y,
                    z,
                    visited: self.sector_observed(x, y, z),
                }
            })
            .collect()
    }

    /// The probe's own sector in the relative frame, rounded to the lattice.
    pub fn probe_relative_coords(&self) -> Option<(i64, i64, i64)> {
        let s = self.probe.as_ref()?.sector.as_ref()?;
        let r = s.relative.as_ref()?;
        Some((r.x.round() as i64, r.y.round() as i64, r.z.round() as i64))
    }

    /// Whether the scan history holds an observation of a relative sector.
    fn sector_observed(&self, x: i64, y: i64, z: i64) -> bool {
        self.scan_history.iter().any(|o| {
            let r = &o.relative_coordinates;
            r.x.round() as i64 == x && r.y.round() as i64 == y && r.z.round() as i64 == z
        })
    }
}

/// The improvement id the motorization endpoint requires.
pub const THRUST_ANCHORING_ID: &str = "distributed_thrust_anchoring";

/// Whether an asteroid already carries a trajectory the server would call
/// active (`asteroid_trajectory_already_active`).
pub fn trajectory_active(o: &SectorObject) -> bool {
    o.trajectory.as_ref().is_some_and(|t| is_running(t.status))
}

/// A trajectory is *running* while it still has somewhere to go. Everything
/// else — captured, missed, lost — is history the payload keeps reporting, the
/// same trap the probe's own `movement` sets (issue #229).
pub fn is_running(status: crate::api::types::AsteroidTrajectoryStatus) -> bool {
    use crate::api::types::AsteroidTrajectoryStatus as S;
    matches!(
        status,
        S::Accelerating | S::Coasting | S::CrossingSector | S::OrbitingBlackHole
    )
}

/// How the cockpit talks about capture odds — which is to say, carefully.
///
/// The spec states the rule (each empty sector crossed and each failed capture
/// costs ten points) but the payload does not say how many of `sectorsCrossed`
/// were empty. Computing a percentage from it would be inventing a number the
/// server never stated, the same reason `is_safe_corridor` only ever answers on
/// evidence. So the counters are reported as counters, and the rule as a rule.
pub fn crossing_summary(t: &crate::api::types::AsteroidTrajectory) -> Option<String> {
    let crossed = t.sectors_crossed?;
    match t.maximum_sector_crossings {
        Some(max) => Some(format!("{crossed}/{max} sectors crossed")),
        None => Some(format!("{crossed} sectors crossed")),
    }
}

impl ScannerObjectEntry {
    /// Whether this entry is an asteroid at all — the three motorized actions
    /// apply to nothing else.
    pub fn is_asteroid(&self) -> bool {
        self.object_type == SectorObjectType::Asteroid
    }
}

/// How a trajectory status reads in a toast — the pilot's words, not the
/// enum's. `Unknown` is a status this build has not heard of, which is a
/// perfectly ordinary thing for a server ahead of us to send.
pub fn trajectory_status_label(status: crate::api::types::AsteroidTrajectoryStatus) -> &'static str {
    use crate::api::types::AsteroidTrajectoryStatus as S;
    match status {
        S::Accelerating => "accelerating",
        S::Coasting => "coasting",
        S::CrossingSector => "crossing a sector",
        S::OrbitingBlackHole => "orbiting a black hole",
        S::Captured => "captured",
        S::Completed => "completed",
        S::Missed => "missed",
        S::NoEffect => "no effect",
        S::Destroyed => "destroyed",
        S::Lost => "lost",
        S::Failed => "failed",
        S::Unknown => "in an unrecognised state",
    }
}
