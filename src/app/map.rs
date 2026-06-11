use super::*;

#[derive(Default)]
pub struct MapView {
    pub open: bool,
    pub center_x: i32,
    pub center_z: i32,
    pub y_layer: i32,
    /// Some(buffer) while typing target coordinates ([c] on the map).
    pub coord_input: Option<String>,
}

impl AppState {
    pub fn open_map(&mut self) {
        self.map_recenter_on_probe();
        self.map.open = true;
    }

    pub fn map_recenter_on_probe(&mut self) {
        if let Some((x, y, z)) = self.probe_sector_coords() {
            self.map.center_x = x;
            self.map.center_z = z;
            self.map.y_layer = y;
        }
    }

    /// Chebyshev distance from the probe to the map center, when known.
    pub fn map_center_distance(&self) -> Option<i64> {
        let (px, py, pz) = self.probe_sector_coords()?;
        let dx = (self.map.center_x - px).abs() as i64;
        let dy = (self.map.y_layer - py).abs() as i64;
        let dz = (self.map.center_z - pz).abs() as i64;
        Some(dx.max(dy).max(dz))
    }

    // Move to y±1 while preserving cx+y+cz (no drift on round-trips).
    pub fn map_move_y(&mut self, dy: i32) {
        self.map.y_layer += dy;
        self.map.center_z -= dy;
    }
}
