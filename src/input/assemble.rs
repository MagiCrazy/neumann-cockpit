use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::fetch_assemble_probe;
use crate::app::{ActiveWizard, ApiMessage, AppState, AssembleProbeInput, LogEvent, ASSEMBLABLE_MODELS};

use super::geometry::{is_list_nav_key, list_nav};

/// Assemble-probe wizard (API v81, per-model since v104): pick the hull model,
/// then exactly two empty additional containers. `Enter` advances then fires
/// the ~3-hour build task, `Space` toggles the container under the cursor
/// (capped at two), `Esc` backs out a step (and out of the wizard from the
/// first one).
pub(super) fn handle_assemble_probe_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match code {
        // Esc steps back to the model choice rather than throwing the whole
        // wizard away — the container multi-select is the fiddly part.
        KeyCode::Esc => match &state.active_wizard {
            ActiveWizard::AssembleProbe(AssembleProbeInput::PickContainers {
                manny_id,
                manny_name,
                containers,
                ..
            }) => {
                state.active_wizard = ActiveWizard::AssembleProbe(AssembleProbeInput::PickModel {
                    manny_id: manny_id.clone(),
                    manny_name: manny_name.clone(),
                    containers: containers.clone(),
                    cursor: 0,
                });
            }
            _ => state.close_wizard(),
        },
        _ if is_list_nav_key(code) => match &mut state.active_wizard {
            ActiveWizard::AssembleProbe(AssembleProbeInput::PickModel { cursor, .. }) => {
                if let Some(ns) = list_nav(code, *cursor, ASSEMBLABLE_MODELS.len()) {
                    *cursor = ns;
                }
            }
            ActiveWizard::AssembleProbe(AssembleProbeInput::PickContainers { containers, cursor, .. }) => {
                if let Some(ns) = list_nav(code, *cursor, containers.len()) {
                    *cursor = ns;
                }
            }
            _ => {}
        },
        KeyCode::Char(' ') => {
            if let ActiveWizard::AssembleProbe(AssembleProbeInput::PickContainers {
                selected,
                cursor,
                error,
                ..
            }) = &mut state.active_wizard
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
            // Model step: commit the choice and move on to the containers.
            if let ActiveWizard::AssembleProbe(AssembleProbeInput::PickModel {
                manny_id,
                manny_name,
                containers,
                cursor,
            }) = &state.active_wizard
            {
                let model = ASSEMBLABLE_MODELS[(*cursor).min(ASSEMBLABLE_MODELS.len() - 1)];
                state.active_wizard = ActiveWizard::AssembleProbe(AssembleProbeInput::PickContainers {
                    manny_id: manny_id.clone(),
                    manny_name: manny_name.clone(),
                    model,
                    containers: containers.clone(),
                    selected: Vec::new(),
                    cursor: 0,
                    error: None,
                });
                return;
            }
            // Extract the order without holding a borrow across the fire.
            let order = match &state.active_wizard {
                ActiveWizard::AssembleProbe(AssembleProbeInput::PickContainers {
                    manny_id,
                    model,
                    containers,
                    selected,
                    ..
                }) if selected.len() == 2 => {
                    let ids: Vec<String> = selected.iter().map(|&i| containers[i].0.clone()).collect();
                    Some((manny_id.clone(), *model, ids))
                }
                _ => None,
            };
            match order {
                Some((manny, model, ids)) => {
                    state.close_wizard();
                    fetch_assemble_probe(manny, model, ids, client.clone(), tx.clone());
                    state.log_event(LogEvent::assemble_probe(state.active_probe_id, model));
                }
                None => {
                    if let ActiveWizard::AssembleProbe(AssembleProbeInput::PickContainers { error, .. }) =
                        &mut state.active_wizard
                    {
                        *error = Some("select exactly two empty containers".into());
                    }
                }
            }
        }
        _ => {}
    }
}
