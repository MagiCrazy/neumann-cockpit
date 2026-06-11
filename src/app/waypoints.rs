use crate::api::types::SectorObjectType;
use super::*;

/// Category of a known destination shown in the waypoints overlay.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaypointKind {
    Bookmark,
    Star,
    Minable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WaypointEntry {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub distance: i64,
    pub label: String,
    pub kind: WaypointKind,
}

impl AppState {
    /// Known destinations aggregated from scan history: deployed waypoint
    /// bookmarks first, then sectors with a star, then sectors with minable
    /// targets. One entry per (sector, category); bookmarks listed per name.
    pub fn collect_waypoints(&self) -> Vec<WaypointEntry> {
        let mut bookmarks: Vec<WaypointEntry> = Vec::new();
        let mut stars: Vec<WaypointEntry> = Vec::new();
        let mut minables: Vec<WaypointEntry> = Vec::new();

        for s in &self.scan_history {
            let (x, y, z) = (
                s.relative_coordinates.x.round() as i32,
                s.relative_coordinates.y.round() as i32,
                s.relative_coordinates.z.round() as i32,
            );
            let Some(objects) = &s.objects else { continue };

            for o in objects {
                let obj_name = o.name.clone().unwrap_or_else(|| "object".into());
                for wb in &o.waypoint_bookmarks {
                    bookmarks.push(WaypointEntry {
                        x, y, z,
                        distance: s.distance,
                        label: format!("{} @ {}", wb.name, obj_name),
                        kind: WaypointKind::Bookmark,
                    });
                }
                for t in &o.bookmark_targets {
                    let t_name = t.name.clone().unwrap_or_else(|| "object".into());
                    for wb in &t.waypoint_bookmarks {
                        bookmarks.push(WaypointEntry {
                            x, y, z,
                            distance: s.distance,
                            label: format!("{} @ {}", wb.name, t_name),
                            kind: WaypointKind::Bookmark,
                        });
                    }
                }
            }

            let has_star = objects.iter().any(|o| {
                matches!(o.object_type, SectorObjectType::Star | SectorObjectType::SolarSystem)
            });
            if has_star {
                stars.push(WaypointEntry {
                    x, y, z,
                    distance: s.distance,
                    label: "star".into(),
                    kind: WaypointKind::Star,
                });
            }

            let has_minable = objects.iter().any(|o| {
                o.minable_targets.as_ref().is_some_and(|t| !t.is_empty())
            });
            if has_minable {
                minables.push(WaypointEntry {
                    x, y, z,
                    distance: s.distance,
                    label: "minable resources".into(),
                    kind: WaypointKind::Minable,
                });
            }
        }

        stars.sort_by_key(|e| e.distance);
        minables.sort_by_key(|e| e.distance);
        let mut out = bookmarks;
        out.extend(stars);
        out.extend(minables);
        out
    }
}
