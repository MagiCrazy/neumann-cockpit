use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::fetch_rename_probe;
use crate::app::{ApiMessage, AppState, LogEvent, ProbeSwitchInput, RenameProbeInput};

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

/// Rename-probe wizard (API v81): text entry, `Enter` commits the new name via
/// `PATCH /api/probe/{id}`, `Esc` cancels. Empty input is ignored.
pub(super) fn handle_rename_probe_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match code {
        KeyCode::Esc => state.rename_probe = RenameProbeInput::Inactive,
        KeyCode::Tab => {
            let s = state.next_name_suggestion();
            if let RenameProbeInput::Typing { buf, .. } = &mut state.rename_probe {
                *buf = s;
            }
        }
        KeyCode::Backspace => {
            if let RenameProbeInput::Typing { buf, .. } = &mut state.rename_probe {
                buf.pop();
            }
        }
        KeyCode::Char(c) => {
            if let RenameProbeInput::Typing { buf, .. } = &mut state.rename_probe {
                buf.push(c);
            }
        }
        KeyCode::Enter => {
            let order = match &state.rename_probe {
                RenameProbeInput::Typing { probe_id, buf, .. } if !buf.trim().is_empty() => {
                    Some((*probe_id, buf.trim().to_string()))
                }
                _ => None,
            };
            if let Some((id, name)) = order {
                let new_name = name.clone();
                fetch_rename_probe(id, name, client.clone(), tx.clone());
                state.log_event(LogEvent::rename_probe(&new_name, Some(id)));
            }
        }
        _ => {}
    }
}
