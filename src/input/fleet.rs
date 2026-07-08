use crossterm::event::KeyCode;

use crate::app::{AppState, ProbeSwitchInput};

use super::geometry::list_nav;

/// Fleet picker (API v81 multi-probe): navigate the roster, `Enter` switches the
/// piloted probe, `Esc` cancels. Switching is client-side only — the event loop
/// reconciles the `ApiClient` and refetches. An unreachable probe is refused
/// with a toast: piloting it would return only limited telemetry.
pub(super) fn handle_probe_switch_event(code: KeyCode, state: &mut AppState) {
    let ProbeSwitchInput::Picking { selection } = state.probe_switch else { return };
    let count = state.fleet.len();
    match code {
        KeyCode::Esc => state.probe_switch = ProbeSwitchInput::Inactive,
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(ns) = list_nav(code, selection, count) {
                state.probe_switch = ProbeSwitchInput::Picking { selection: ns };
            }
        }
        KeyCode::Enter => {
            if let Some(p) = state.fleet.get(selection) {
                let (id, name, reachable) = (p.id, p.name.clone(), p.is_reachable);
                state.probe_switch = ProbeSwitchInput::Inactive;
                if !reachable {
                    state.set_toast(format!("{name} is out of SCUT range — cannot pilot"));
                } else if state.set_active_probe(id) {
                    state.set_toast(format!("piloting {name}"));
                }
            }
        }
        _ => {}
    }
}
