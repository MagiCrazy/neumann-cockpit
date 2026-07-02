use crossterm::event::KeyCode;

use crate::app::{AppState, GotoVisitedInput};

use super::geometry::list_nav;

/// Picker over visited sectors: navigate the list, `Enter` launches the travel
/// confirm for the chosen sector, `Esc` cancels.
pub(super) fn handle_goto_visited_event(code: KeyCode, state: &mut AppState) {
    let GotoVisitedInput::Picking { selection } = state.goto_visited else { return };
    let count = state.visited_sectors.len();
    match code {
        KeyCode::Esc => state.goto_visited = GotoVisitedInput::Inactive,
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(ns) = list_nav(code, selection, count) {
                state.goto_visited = GotoVisitedInput::Picking { selection: ns };
            }
        }
        KeyCode::Enter => {
            if let Some(v) = state.visited_sectors.get(selection) {
                let c = &v.relative_coordinates;
                let (x, y, z) = (c.x.round() as i32, c.y.round() as i32, c.z.round() as i32);
                state.goto_visited = GotoVisitedInput::Inactive;
                state.travel_go_sector(x, y, z);
            }
        }
        _ => {}
    }
}

pub(super) fn handle_map_event(code: KeyCode, state: &mut AppState) {
    // Coordinate-input mode ([c]) captures keys first.
    if let Some(buf) = state.map.coord_input.as_mut() {
        match code {
            KeyCode::Esc => state.map.coord_input = None,
            KeyCode::Backspace => {
                buf.pop();
            }
            KeyCode::Enter => {
                let parts: Vec<&str> = buf.split_whitespace().collect();
                if parts.len() == 3 {
                    if let (Ok(x), Ok(y), Ok(z)) = (
                        parts[0].parse::<i32>(),
                        parts[1].parse::<i32>(),
                        parts[2].parse::<i32>(),
                    ) {
                        state.map.center_x = x;
                        state.map.y_layer = y;
                        state.map.center_z = z;
                        state.map.coord_input = None;
                    }
                }
            }
            KeyCode::Char(c) if c == '-' || c == ' ' || c.is_ascii_digit() => buf.push(c),
            _ => {}
        }
        return;
    }

    match code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('b') => state.map.open = false,
        KeyCode::Char('h') | KeyCode::Left  => state.map.center_x -= 2,
        KeyCode::Char('l') | KeyCode::Right => state.map.center_x += 2,
        KeyCode::Char('k') | KeyCode::Up    => state.map.center_z -= 2,
        KeyCode::Char('j') | KeyCode::Down  => state.map.center_z += 2,
        KeyCode::Char('u') => state.map_move_y(1),
        KeyCode::Char('d') => state.map_move_y(-1),
        KeyCode::Char('0') => state.map_recenter_on_probe(),
        KeyCode::Char('c') => state.map.coord_input = Some(String::new()),
        KeyCode::Char('g') => {
            let (cx, y, cz) = (state.map.center_x, state.map.y_layer, state.map.center_z);
            state.map.open = false;
            state.travel_go_sector(cx, y, cz);
        }
        _ => {}
    }
}
