use crossterm::event::KeyCode;

use crate::app::AppState;
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
