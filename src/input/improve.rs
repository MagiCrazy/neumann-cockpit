use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::fetch_improve_probe;
use crate::app::{ApiMessage, AppState, ImproveInput, LogEvent};
use super::geometry::list_nav;

/// Drive the probe-improvement wizard: pick an improvement, then resolve which
/// idle onboard Manny installs it (auto for a single one, else `PickBuilder`).
pub(super) fn handle_improve_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match state.improve {
        ImproveInput::PickImprovement { selection, .. } => {
            let count = state.probe_improvements.len();
            match code {
                KeyCode::Esc => state.improve = ImproveInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, selection, count) {
                        if let ImproveInput::PickImprovement { ref mut selection, .. } = state.improve {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => commit_improvement(selection, state, client, tx),
                _ => {}
            }
        }
        ImproveInput::PickBuilder { selection, ref mannies, .. } => {
            let count = mannies.len();
            match code {
                KeyCode::Esc => state.improve = ImproveInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, selection, count) {
                        if let ImproveInput::PickBuilder { ref mut selection, .. } = state.improve {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let picked = if let ImproveInput::PickBuilder { ref mannies, selection, ref improvement_id, ref improvement_name, .. } = state.improve {
                        mannies.get(selection).map(|(id, _)| (id.clone(), improvement_id.clone(), improvement_name.clone()))
                    } else {
                        None
                    };
                    if let Some((manny_id, improvement_id, improvement_name)) = picked {
                        fetch_improve_probe(manny_id, improvement_id, client.clone(), tx.clone());
                        state.log_event(LogEvent::improve(&improvement_name, state.active_probe_id));
                    }
                }
                _ => {}
            }
        }
        ImproveInput::Inactive => {}
    }
}

/// Validate the selected improvement, then fire it (auto-picking the sole idle
/// Manny) or advance to the builder-selection step.
fn commit_improvement(
    selection: usize,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    let Some((id, name, done, available)) = state.probe_improvements.get(selection)
        .map(|i| (i.id.clone(), i.name.clone(), i.done, i.available))
    else {
        return;
    };
    if done {
        state.set_improve_error("already installed".into());
        return;
    }
    if !available {
        state.set_improve_error("not unlocked yet".into());
        return;
    }
    let mannies = state.collect_idle_onboard_mannies();
    match mannies.len() {
        0 => state.set_improve_error("no idle Manny on board".into()),
        1 => {
            let (manny_id, _) = mannies.into_iter().next().unwrap();
            fetch_improve_probe(manny_id, id, client.clone(), tx.clone());
            state.log_event(LogEvent::improve(&name, state.active_probe_id));
        }
        _ => {
            state.improve = ImproveInput::PickBuilder {
                improvement_id: id,
                improvement_name: name,
                mannies,
                selection: 0,
                error: None,
            };
        }
    }
}
