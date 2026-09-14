//! Incoming-missile threat state (issue #362).
//!
//! Since v122 the sector scan carries every missile in flight, and one field on
//! it changes what the cockpit owes the pilot: `targetsCurrentProbe`, true only
//! when the observing probe is the **declared** target. That is not another
//! object in a list — it is a countdown to being hit, and `impactAt` says when.
//!
//! The cockpit's whole visual grammar is quiet: the quota chip stays silent
//! until half spent, the update notice is a dim chip, ambiance can be switched
//! off. This is the one case where that grammar is wrong, and the way out of
//! the contradiction is **scarcity**: the banner exists only while a missile is
//! actually aimed at the piloted probe, and it is the only thing in the cockpit
//! that gets a row of its own. A warning that cannot fire spuriously is a
//! warning nobody learns to ignore.
//!
//! A missile in the sector aimed at *something else* deliberately gets none of
//! this — it is an object in the Sector pane, in crit, with its target and its
//! impact. Visible without being alarming, which is the distinction the server
//! itself draws between the `weapon` and `weapon_targeted` alert phases.

use chrono::{DateTime, Utc};

use super::*;
use crate::api::types::{MissileLauncherKind, SectorObject, SectorObjectType};

/// A missile whose declared target is the piloted probe.
#[derive(Debug, Clone, PartialEq)]
pub struct IncomingMissile {
    /// Estimated resolution. `None` on a scan that saw the missile but could
    /// not time it — the threat is still real, so the banner says so without
    /// inventing a countdown.
    pub impact_at: Option<DateTime<Utc>>,
    pub launcher: Option<MissileLauncherKind>,
}

impl IncomingMissile {
    /// Seconds until impact, or `None` when unknown. Clamped at zero: a
    /// missile whose estimate has passed is overdue, not early.
    pub fn seconds_to_impact(&self) -> Option<i64> {
        self.impact_at.map(|t| (t - Utc::now()).num_seconds().max(0))
    }

    /// Who fired, in words, when the scan says.
    pub fn launcher_label(&self) -> Option<&'static str> {
        match self.launcher? {
            MissileLauncherKind::Probe => Some("probe"),
            MissileLauncherKind::OthersShip => Some("Others ship"),
            MissileLauncherKind::Unknown => None,
        }
    }
}

/// Whether this scanned object is a missile in flight.
pub fn is_missile(o: &SectorObject) -> bool {
    o.object_type == SectorObjectType::Missile
}

impl AppState {
    /// The missile aimed at the piloted probe, soonest impact first.
    ///
    /// Only `targetsCurrentProbe == Some(true)` counts. An absent field is not
    /// a threat: the cockpit must never invent one, because the whole value of
    /// the banner is that it cannot fire spuriously.
    pub fn incoming_missile(&self) -> Option<IncomingMissile> {
        let objects = self.probe_current_sector_scan()?.objects.as_ref()?;
        objects
            .iter()
            .filter(|o| is_missile(o) && o.targets_current_probe == Some(true))
            .map(|o| IncomingMissile {
                impact_at: o.impact_at,
                launcher: o.launcher_kind,
            })
            // Soonest first; an untimed missile sorts last, since a known
            // countdown is the more urgent thing to show.
            .min_by_key(|m| m.impact_at.map_or(i64::MAX, |t| t.timestamp()))
    }

    /// Hull integrity as a whole percent, for the banner. The number a pilot
    /// wants next to a countdown is how much hull is left to spend on it.
    pub fn hull_percent(&self) -> Option<i64> {
        self.probe_integrity().map(|i| i.round() as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sector_with(objects: &str) -> crate::api::types::SectorObservation {
        serde_json::from_str(&format!(
            r#"{{"relativeCoordinates": {{"x": 0.0, "y": 0.0, "z": 0.0}}, "distance": 0,
                 "knowledgeLevel": "detailed", "confidence": 1.0,
                 "objects": [{objects}],
                 "scan": {{"currentSectorResidenceSeconds": 60,
                           "requiredResidenceSeconds": 60, "scanQuality": 1.0}}}}"#
        ))
        .unwrap()
    }

    fn missile(targets_us: bool, impact_in_secs: i64) -> String {
        let at = Utc::now() + chrono::Duration::seconds(impact_in_secs);
        format!(
            r#"{{"id": "m{impact_in_secs}", "type": "missile", "name": "missile", "summary": "s",
                 "launcherKind": "others_ship", "targetsCurrentProbe": {targets_us},
                 "impactAt": "{}"}}"#,
            at.to_rfc3339()
        )
    }

    fn state_seeing(objects: &str) -> AppState {
        let mut s = AppState::default();
        s.probe = Some(
            serde_json::from_str(
                r#"{"id": 1, "name": "t", "status": "idle",
                    "fuel": {"deuterium": 50.0}, "sensorMode": "normal",
                    "sector": {"relative": {"x": 0.0, "y": 0.0, "z": 0.0}},
                    "movement": null,
                    "systems": {"integrityPercent": 84.0, "damagePercent": 16.0,
                                "energyStored": null, "internalClockRate": null,
                                "currentTask": null},
                    "inventory": {"capacity": 1.0, "usedCapacity": 0.0, "freeCapacity": 1.0,
                                  "items": [], "resourceStocks": [], "externalTanks": [],
                                  "containers": []}}"#,
            )
            .unwrap(),
        );
        s.scan_history = vec![sector_with(objects)];
        s
    }

    #[test]
    fn a_missile_aimed_at_us_is_the_incoming_one() {
        let s = state_seeing(&missile(true, 192));
        let m = s.incoming_missile().expect("aimed at the piloted probe");
        assert_eq!(m.launcher_label(), Some("Others ship"));
        let secs = m.seconds_to_impact().expect("timed");
        assert!((secs - 192).abs() <= 2, "got {secs}");
    }

    #[test]
    fn a_missile_aimed_elsewhere_raises_nothing() {
        // The distinction the server draws between `weapon` and
        // `weapon_targeted`, honoured: someone else's missile is an object in
        // the Sector pane and nothing more.
        let s = state_seeing(&missile(false, 60));
        assert!(s.incoming_missile().is_none());
    }

    #[test]
    fn an_absent_field_is_not_a_threat() {
        // The banner's whole value is that it cannot fire spuriously, so a
        // pre-v122 payload — or one that simply omits the field — invents
        // nothing.
        let s = state_seeing(r#"{"id": "m1", "type": "missile", "name": "m", "summary": "s"}"#);
        assert!(s.incoming_missile().is_none());
    }

    #[test]
    fn the_soonest_impact_wins_and_an_untimed_one_sorts_last() {
        let s = state_seeing(&format!(
            r#"{}, {}, {{"id": "mx", "type": "missile", "name": "m", "summary": "s",
                         "targetsCurrentProbe": true}}"#,
            missile(true, 600),
            missile(true, 90)
        ));
        let secs = s.incoming_missile().unwrap().seconds_to_impact().unwrap();
        assert!((secs - 90).abs() <= 2, "the nearest threat is the one shown: {secs}");
    }

    #[test]
    fn an_overdue_missile_counts_down_to_zero_not_below() {
        let s = state_seeing(&missile(true, -300));
        assert_eq!(s.incoming_missile().unwrap().seconds_to_impact(), Some(0));
    }

    #[test]
    fn an_incoming_missile_keeps_the_starfield_off() {
        // #206's guard was written for exactly this shape of problem, but it
        // only ever asked about *unread* alerts — so a pilot who read the alert
        // and walked away got a starfield over an incoming missile.
        let s = state_seeing(&missile(true, 120));
        assert!(!s.attract_allowed(), "a screensaver must not hide this");

        let calm = state_seeing(&missile(false, 120));
        assert!(calm.attract_allowed(), "someone else's missile is not our emergency");
    }
}
