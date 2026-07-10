use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::fetch_assemble_probe;
use crate::app::{ApiMessage, AppState, AssembleProbeInput, LogEvent};

use super::geometry::list_nav;

/// Assemble-probe wizard (API v81): pick exactly two empty additional
/// containers, `Enter` fires the 3-hour build task, `Esc` cancels. `Space`
/// toggles the container under the cursor (capped at two).
pub(super) fn handle_assemble_probe_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match code {
        KeyCode::Esc => state.assemble_probe = AssembleProbeInput::Inactive,
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let AssembleProbeInput::PickContainers { containers, cursor, .. } =
                &mut state.assemble_probe
            {
                if let Some(ns) = list_nav(code, *cursor, containers.len()) {
                    *cursor = ns;
                }
            }
        }
        KeyCode::Char(' ') => {
            if let AssembleProbeInput::PickContainers { selected, cursor, error, .. } =
                &mut state.assemble_probe
            {
                let cur = *cursor;
                if let Some(pos) = selected.iter().position(|&i| i == cur) {
                    selected.remove(pos);
                    *error = None;
                } else if selected.len() < 2 {
                    selected.push(cur);
                    *error = None;
                } else {
                    *error = Some("exactly two containers — deselect one first".into());
                }
            }
        }
        KeyCode::Enter => {
            // Extract the order without holding a borrow across the fire.
            let order = match &state.assemble_probe {
                AssembleProbeInput::PickContainers { manny_id, containers, selected, .. }
                    if selected.len() == 2 =>
                {
                    let ids: Vec<String> =
                        selected.iter().map(|&i| containers[i].0.clone()).collect();
                    Some((manny_id.clone(), ids))
                }
                _ => None,
            };
            match order {
                Some((manny, ids)) => {
                    state.assemble_probe = AssembleProbeInput::Inactive;
                    fetch_assemble_probe(manny, ids, client.clone(), tx.clone());
                    state.log_event(LogEvent::assemble_probe(state.active_probe_id));
                }
                None => {
                    if let AssembleProbeInput::PickContainers { error, .. } =
                        &mut state.assemble_probe
                    {
                        *error = Some("select exactly two empty containers".into());
                    }
                }
            }
        }
        _ => {}
    }
}
