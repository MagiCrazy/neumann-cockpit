use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::fetch_mine;
use crate::app::{
    ActiveWizard, ApiMessage, AppState, LogEvent, MineInput, RemoteMineInput, RESOURCE_TYPES,
};
use super::geometry::list_nav;

/// Cycle the optional mining target container: None → first → … → last → None.
pub(crate) fn next_target_container(
    current: Option<&(String, String)>,
    containers: &[(String, String)],
) -> Option<(String, String)> {
    match current {
        None => containers.first().cloned(),
        Some((id, _)) => match containers.iter().position(|(cid, _)| cid == id) {
            Some(i) => containers.get(i + 1).cloned(),
            None => containers.first().cloned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::next_target_container;

    fn c(id: &str) -> (String, String) {
        (id.into(), format!("container {id}"))
    }

    #[test]
    fn cycles_none_through_containers_back_to_none() {
        let list = vec![c("a"), c("b")];
        let s0 = next_target_container(None, &list);
        assert_eq!(s0.as_ref().map(|(id, _)| id.as_str()), Some("a"));
        let s1 = next_target_container(s0.as_ref(), &list);
        assert_eq!(s1.as_ref().map(|(id, _)| id.as_str()), Some("b"));
        let s2 = next_target_container(s1.as_ref(), &list);
        assert_eq!(s2, None);
    }

    #[test]
    fn no_containers_stays_none() {
        assert_eq!(next_target_container(None, &[]), None);
    }

    #[test]
    fn stale_selection_resets_to_first() {
        let list = vec![c("a")];
        let stale = c("gone");
        assert_eq!(
            next_target_container(Some(&stale), &list).as_ref().map(|(id, _)| id.as_str()),
            Some("a")
        );
    }
}

pub(super) fn handle_mine_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.active_wizard {
        ActiveWizard::Mine(MineInput::PickAsteroid { selection, candidates, .. }) => {
            let sel = *selection;
            let count = candidates.len();
            match code {
                KeyCode::Esc => state.close_wizard(),
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let ActiveWizard::Mine(MineInput::PickAsteroid { selection, .. }) = &mut state.active_wizard {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, manny_name, object_id, object_name) = {
                        let ActiveWizard::Mine(MineInput::PickAsteroid { manny_id, manny_name, candidates, selection }) = &state.active_wizard else { return };
                        let (id, name) = candidates[*selection].clone();
                        (manny_id.clone(), manny_name.clone(), id, name)
                    };
                    // Preselect the resources the target actually holds; fall
                    // back to metals when the reserves are unknown.
                    let resources = state
                        .minable_target_reserves(&object_id)
                        .map(|(flags, _)| flags)
                        .filter(|f| f.iter().any(|&x| x))
                        .unwrap_or([false, true, false, false]);
                    state.active_wizard = ActiveWizard::Mine(MineInput::Configure {
                        manny_id, manny_name, object_id, object_name,
                        resources,
                        amount_buf: "0.30".into(),
                        amount_mode: false,
                        target_container: None,
                        error: None,
                    });
                }
                _ => {}
            }
        }
        ActiveWizard::Mine(MineInput::Configure { amount_mode, object_id, resources, .. }) => {
            let am = *amount_mode;
            let obj_id = object_id.clone();
            let res = *resources;
            match code {
                KeyCode::Esc => state.close_wizard(),
                KeyCode::Tab => {
                    if let ActiveWizard::Mine(MineInput::Configure { amount_mode, error, .. }) = &mut state.active_wizard {
                        *amount_mode = !am;
                        *error = None;
                    }
                }
                KeyCode::Char(c @ '1'..='4') if !am => {
                    let idx = (c as u8 - b'1') as usize;
                    // Only toggle resources the target actually holds.
                    let present = state
                        .minable_target_reserves(&obj_id)
                        .map(|(flags, _)| flags[idx])
                        .unwrap_or(true);
                    if present {
                        if let ActiveWizard::Mine(MineInput::Configure { resources, error, .. }) = &mut state.active_wizard {
                            resources[idx] = !resources[idx];
                            *error = None;
                        }
                    }
                }
                KeyCode::Char('m') | KeyCode::Char('M') if am => {
                    let max = state.mine_reserve_max(&obj_id, res);
                    if let ActiveWizard::Mine(MineInput::Configure { amount_buf, error, .. }) = &mut state.active_wizard {
                        *amount_buf = format!("{:.4}", max);
                        *error = None;
                    }
                }
                KeyCode::Char(c) if am && (c.is_ascii_digit() || c == '.') => {
                    if let ActiveWizard::Mine(MineInput::Configure { amount_buf, error, .. }) = &mut state.active_wizard {
                        if !(c == '.' && amount_buf.contains('.')) {
                            amount_buf.push(c);
                            *error = None;
                        }
                    }
                }
                KeyCode::Backspace if am => {
                    if let ActiveWizard::Mine(MineInput::Configure { amount_buf, .. }) = &mut state.active_wizard {
                        amount_buf.pop();
                    }
                }
                KeyCode::Char('c') => {
                    let containers = state.collect_detached_containers();
                    if let ActiveWizard::Mine(MineInput::Configure { target_container, error, .. }) = &mut state.active_wizard {
                        *target_container = next_target_container(target_container.as_ref(), &containers);
                        *error = None;
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, object_id, selected_resources, amount, container_id, destination) = {
                        let ActiveWizard::Mine(MineInput::Configure { manny_id, object_id, resources, amount_buf, target_container, .. }) = &state.active_wizard else { return };
                        let selected: Vec<String> = RESOURCE_TYPES.iter().enumerate()
                            .filter(|(i, _)| resources[*i])
                            .map(|(_, &t)| t.to_string())
                            .collect();
                        if selected.is_empty() { return }
                        let Ok(amount) = amount_buf.parse::<f64>() else { return };
                        if amount <= 0.0 { return }
                        let destination = target_container
                            .as_ref()
                            .map(|(_, n)| n.clone())
                            .unwrap_or_else(|| "the probe".to_string());
                        (manny_id.clone(), object_id.clone(), selected, amount, target_container.as_ref().map(|(id, _)| id.clone()), destination)
                    };
                    let resources_label = selected_resources.join(", ");
                    fetch_mine(manny_id, object_id, selected_resources, amount, container_id, client.clone(), tx.clone());
                    state.log_event(LogEvent::mine(&resources_label, amount, &destination, state.active_probe_id));
                }
                _ => {}
            }
        }
        _ => {}
    }
}

pub(super) fn handle_remote_mine_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.active_wizard {
        ActiveWizard::RemoteMine(RemoteMineInput::Loading { .. }) => {
            if code == KeyCode::Esc {
                state.close_wizard();
            }
        }
        ActiveWizard::RemoteMine(RemoteMineInput::PickAsteroid { selection, candidates, .. }) => {
            let sel = *selection;
            let count = candidates.len();
            match code {
                KeyCode::Esc => state.close_wizard(),
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let ActiveWizard::RemoteMine(RemoteMineInput::PickAsteroid { selection, .. }) = &mut state.active_wizard {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let ActiveWizard::RemoteMine(RemoteMineInput::PickAsteroid {
                        manny_id, manny_name, x, y, z, candidates, selection,
                    }) = &state.active_wizard else { return };
                    let (object_id, object_name) = candidates[*selection].clone();
                    let (manny_id, manny_name, x, y, z) =
                        (manny_id.clone(), manny_name.clone(), *x, *y, *z);
                    state.active_wizard = ActiveWizard::RemoteMine(RemoteMineInput::Configure {
                        manny_id,
                        manny_name,
                        x, y, z,
                        object_id,
                        object_name,
                        resources: [false, true, false, false],
                        amount_buf: "0.30".into(),
                        amount_mode: false,
                        error: None,
                    });
                }
                _ => {}
            }
        }
        ActiveWizard::RemoteMine(RemoteMineInput::Configure { amount_mode, .. }) => {
            let am = *amount_mode;
            match code {
                KeyCode::Esc => state.close_wizard(),
                KeyCode::Tab => {
                    if let ActiveWizard::RemoteMine(RemoteMineInput::Configure { amount_mode, error, .. }) = &mut state.active_wizard {
                        *amount_mode = !am;
                        *error = None;
                    }
                }
                KeyCode::Char(c @ '1'..='4') if !am => {
                    let idx = (c as u8 - b'1') as usize;
                    if let ActiveWizard::RemoteMine(RemoteMineInput::Configure { resources, error, .. }) = &mut state.active_wizard {
                        resources[idx] = !resources[idx];
                        *error = None;
                    }
                }
                KeyCode::Char(c) if am && (c.is_ascii_digit() || c == '.') => {
                    if let ActiveWizard::RemoteMine(RemoteMineInput::Configure { amount_buf, error, .. }) = &mut state.active_wizard {
                        if !(c == '.' && amount_buf.contains('.')) {
                            amount_buf.push(c);
                            *error = None;
                        }
                    }
                }
                KeyCode::Backspace if am => {
                    if let ActiveWizard::RemoteMine(RemoteMineInput::Configure { amount_buf, .. }) = &mut state.active_wizard {
                        amount_buf.pop();
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, object_id, resources, amount, coords) = {
                        let ActiveWizard::RemoteMine(RemoteMineInput::Configure {
                            manny_id, object_id, resources, amount_buf, x, y, z, ..
                        }) = &state.active_wizard else { return };
                        if !resources.iter().any(|&r| r) { return }
                        let Ok(amount) = amount_buf.parse::<f64>() else { return };
                        if amount <= 0.0 { return }
                        (manny_id.clone(), object_id.clone(), *resources, amount, (*x, *y, *z))
                    };
                    let containers = state
                        .sector_observation_at(coords.0, coords.1, coords.2)
                        .map(|s| state.collect_detached_containers_in(s))
                        .unwrap_or_default();
                    if containers.is_empty() {
                        state.set_remote_mine_error(
                            "no detached container in the Manny's sector".into(),
                        );
                        return;
                    }
                    state.active_wizard = ActiveWizard::RemoteMine(RemoteMineInput::PickContainer {
                        manny_id,
                        object_id,
                        resources,
                        amount,
                        containers,
                        selection: 0,
                        error: None,
                    });
                }
                _ => {}
            }
        }
        ActiveWizard::RemoteMine(RemoteMineInput::PickContainer { selection, containers, .. }) => {
            let sel = *selection;
            let count = containers.len();
            match code {
                KeyCode::Esc => state.close_wizard(),
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let ActiveWizard::RemoteMine(RemoteMineInput::PickContainer { selection, .. }) = &mut state.active_wizard {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, object_id, selected_resources, amount, container_id, destination) = {
                        let ActiveWizard::RemoteMine(RemoteMineInput::PickContainer {
                            manny_id, object_id, resources, amount, containers, selection, ..
                        }) = &state.active_wizard else { return };
                        let selected: Vec<String> = RESOURCE_TYPES.iter().enumerate()
                            .filter(|(i, _)| resources[*i])
                            .map(|(_, &t)| t.to_string())
                            .collect();
                        (manny_id.clone(), object_id.clone(), selected, *amount, containers[*selection].0.clone(), containers[*selection].1.clone())
                    };
                    let resources_label = selected_resources.join(", ");
                    fetch_mine(manny_id, object_id, selected_resources, amount, Some(container_id), client.clone(), tx.clone());
                    state.log_event(LogEvent::mine(&resources_label, amount, &destination, state.active_probe_id));
                }
                _ => {}
            }
        }
        _ => {}
    }
}
