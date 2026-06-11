use crate::api::types::ProbeMovement;
use chrono::Utc;
use super::*;

impl AppState {
    pub fn travel_type_char(&mut self, c: char) {
        if let TravelInput::Typing(ref mut buf) = self.travel {
            if c == '-' || c == ' ' || c.is_ascii_digit() || (c == '+' && buf.is_empty()) {
                buf.push(c);
            }
        }
    }

    pub fn travel_backspace(&mut self) {
        if let TravelInput::Typing(ref mut buf) = self.travel {
            buf.pop();
        }
    }

    /// Parse a travel destination buffer: absolute "x y z", or relative
    /// "+dx dy dz" applied to `current`.
    fn parse_travel_buf(buf: &str, current: Option<(i32, i32, i32)>) -> Option<(i32, i32, i32)> {
        let trimmed = buf.trim();
        let (relative, rest) = match trimmed.strip_prefix('+') {
            Some(r) => (true, r),
            None => (false, trimmed),
        };
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if parts.len() != 3 {
            return None;
        }
        let x = parts[0].parse::<i32>().ok()?;
        let y = parts[1].parse::<i32>().ok()?;
        let z = parts[2].parse::<i32>().ok()?;
        if relative {
            let (cx, cy, cz) = current?;
            Some((cx + x, cy + y, cz + z))
        } else {
            Some((x, y, z))
        }
    }

    /// Destination currently typed in the travel overlay, resolved to
    /// absolute coordinates (None while incomplete or invalid).
    pub fn resolve_travel_target(&self) -> Option<(i32, i32, i32)> {
        let TravelInput::Typing(ref buf) = self.travel else { return None };
        Self::parse_travel_buf(buf, self.probe_sector_coords())
    }

    pub fn travel_submit(&mut self) {
        let Some((x, y, z)) = self.resolve_travel_target() else { return };
        let error = if (x + y + z) % 2 != 0 {
            Some("x+y+z must be even".to_string())
        } else {
            None
        };
        let (sector_distance, fuel_cost, eta_minutes) = self.travel_preview(x, y, z);
        self.travel = TravelInput::Confirming { x, y, z, sector_distance, fuel_cost, eta_minutes, error };
    }

    pub fn travel_go_sector(&mut self, x: i32, y: i32, z: i32) {
        let (sector_distance, fuel_cost, eta_minutes) = self.travel_preview(x, y, z);
        self.travel = TravelInput::Confirming { x, y, z, sector_distance, fuel_cost, eta_minutes, error: None };
    }

    fn travel_preview(&self, x: i32, y: i32, z: i32) -> (Option<i64>, Option<f64>, Option<i64>) {
        let sector_distance = self.distance_to(x, y, z)
            .or_else(|| {
                self.scan_history.iter()
                    .find(|s| {
                        s.relative_coordinates.x as i32 == x
                            && s.relative_coordinates.y as i32 == y
                            && s.relative_coordinates.z as i32 == z
                    })
                    .map(|s| s.distance)
            });
        let fuel_cost = self.probe.as_ref()
            .and_then(|p| p.fuel.deuterium)
            .map(|d| (d * 0.02 * 10000.0).round() / 10000.0);
        let eta_minutes = sector_distance.map(|d| 5 + 35 * d);
        (sector_distance, fuel_cost, eta_minutes)
    }

    fn distance_to(&self, x: i32, y: i32, z: i32) -> Option<i64> {
        let pos = self.probe.as_ref()?.sector.as_ref()?.relative.as_ref()?;
        let dx = ((x as f64) - pos.x).abs().round() as i64;
        let dy = ((y as f64) - pos.y).abs().round() as i64;
        let dz = ((z as f64) - pos.z).abs().round() as i64;
        Some(dx.max(dy).max(dz))
    }

    pub fn set_travel_error(&mut self, msg: String) {
        if let TravelInput::Confirming { ref mut error, .. } = self.travel {
            *error = Some(format!("API: {msg}"));
        }
    }

    pub fn apply_movement(&mut self, mv: ProbeMovement) {
        self.movement_arrival = Some(mv.arrival_at).filter(|&a| a > Utc::now());
        if let Some(ref mut probe) = self.probe {
            probe.movement = Some(mv);
        }
        self.travel = TravelInput::Inactive;
    }
}
