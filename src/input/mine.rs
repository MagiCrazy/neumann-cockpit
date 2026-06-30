use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::fetch_mine;
use crate::app::{
    ApiMessage, AppState, MineInput, RemoteMineInput, RESOURCE_TYPES,
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
    match &state.mine {
        MineInput::PickAsteroid { selection, candidates, .. } => {
            let sel = *selection;
            let count = candidates.len();
            match code {
                KeyCode::Esc => state.mine = MineInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let MineInput::PickAsteroid { ref mut selection, .. } = state.mine {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, manny_name, object_id, object_name) = {
                        let MineInput::PickAsteroid { ref manny_id, ref manny_name, ref candidates, selection } = state.mine else { return };
                        let (id, name) = candidates[selection].clone();
                        (manny_id.clone(), manny_name.clone(), id, name)
                    };
                    state.mine = MineInput::Configure {
                        manny_id, manny_name, object_id, object_name,
                        resources: [false, true, false, false],
                        amount_buf: "0.30".into(),
                        amount_mode: false,
                        target_container: None,
                        error: None,
                    };
                }
                _ => {}
            }
        }
        MineInput::Configure { amount_mode, .. } => {
            let am = *amount_mode;
            match code {
                KeyCode::Esc => state.mine = MineInput::Inactive,
                KeyCode::Tab => {
                    if let MineInput::Configure { ref mut amount_mode, ref mut error, .. } = state.mine {
                        *amount_mode = !am;
                        *error = None;
                    }
                }
                KeyCode::Char(c @ '1'..='4') if !am => {
                    let idx = (c as u8 - b'1') as usize;
                    if let MineInput::Configure { ref mut resources, ref mut error, .. } = state.mine {
                        resources[idx] = !resources[idx];
                        *error = None;
                    }
                }
                KeyCode::Char('m') | KeyCode::Char('M') if am => {
                    let max = state.mine_max_amount();
                    if let MineInput::Configure { ref mut amount_buf, ref mut error, .. } = state.mine {
                        *amount_buf = format!("{:.4}", max);
                        *error = None;
                    }
                }
                KeyCode::Char(c) if am && (c.is_ascii_digit() || c == '.') => {
                    if let MineInput::Configure { ref mut amount_buf, ref mut error, .. } = state.mine {
                        if !(c == '.' && amount_buf.contains('.')) {
                            amount_buf.push(c);
                            *error = None;
                        }
                    }
                }
                KeyCode::Backspace if am => {
                    if let MineInput::Configure { ref mut amount_buf, .. } = state.mine {
                        amount_buf.pop();
                    }
                }
                KeyCode::Char('c') => {
                    let containers = state.collect_detached_containers();
                    if let MineInput::Configure { ref mut target_container, ref mut error, .. } = state.mine {
                        *target_container = next_target_container(target_container.as_ref(), &containers);
                        *error = None;
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, object_id, selected_resources, amount, container_id) = {
                        let MineInput::Configure { ref manny_id, ref object_id, resources, ref amount_buf, ref target_container, .. } = state.mine else { return };
                        let selected: Vec<String> = RESOURCE_TYPES.iter().enumerate()
                            .filter(|(i, _)| resources[*i])
                            .map(|(_, &t)| t.to_string())
                            .collect();
                        if selected.is_empty() { return }
                        let Ok(amount) = amount_buf.parse::<f64>() else { return };
                        if amount <= 0.0 { return }
                        (manny_id.clone(), object_id.clone(), selected, amount, target_container.as_ref().map(|(id, _)| id.clone()))
                    };
                    fetch_mine(manny_id, object_id, selected_resources, amount, container_id, client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        MineInput::Inactive => {}
    }
}

pub(super) fn handle_remote_mine_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.remote_mine {
        RemoteMineInput::Loading { .. } => {
            if code == KeyCode::Esc {
                state.remote_mine = RemoteMineInput::Inactive;
            }
        }
        RemoteMineInput::PickAsteroid { selection, candidates, .. } => {
            let sel = *selection;
            let count = candidates.len();
            match code {
                KeyCode::Esc => state.remote_mine = RemoteMineInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let RemoteMineInput::PickAsteroid { ref mut selection, .. } = state.remote_mine {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let RemoteMineInput::PickAsteroid {
                        ref manny_id, ref manny_name, x, y, z, ref candidates, selection,
                    } = state.remote_mine else { return };
                    let (object_id, object_name) = candidates[selection].clone();
                    state.remote_mine = RemoteMineInput::Configure {
                        manny_id: manny_id.clone(),
                        manny_name: manny_name.clone(),
                        x, y, z,
                        object_id,
                        object_name,
                        resources: [false, true, false, false],
                        amount_buf: "0.30".into(),
                        amount_mode: false,
                        error: None,
                    };
                }
                _ => {}
            }
        }
        RemoteMineInput::Configure { amount_mode, .. } => {
            let am = *amount_mode;
            match code {
                KeyCode::Esc => state.remote_mine = RemoteMineInput::Inactive,
                KeyCode::Tab => {
                    if let RemoteMineInput::Configure { ref mut amount_mode, ref mut error, .. } = state.remote_mine {
                        *amount_mode = !am;
                        *error = None;
                    }
                }
                KeyCode::Char(c @ '1'..='4') if !am => {
                    let idx = (c as u8 - b'1') as usize;
                    if let RemoteMineInput::Configure { ref mut resources, ref mut error, .. } = state.remote_mine {
                        resources[idx] = !resources[idx];
                        *error = None;
                    }
                }
                KeyCode::Char(c) if am && (c.is_ascii_digit() || c == '.') => {
                    if let RemoteMineInput::Configure { ref mut amount_buf, ref mut error, .. } = state.remote_mine {
                        if !(c == '.' && amount_buf.contains('.')) {
                            amount_buf.push(c);
                            *error = None;
                        }
                    }
                }
                KeyCode::Backspace if am => {
                    if let RemoteMineInput::Configure { ref mut amount_buf, .. } = state.remote_mine {
                        amount_buf.pop();
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, object_id, resources, amount, coords) = {
                        let RemoteMineInput::Configure {
                            ref manny_id, ref object_id, resources, ref amount_buf, x, y, z, ..
                        } = state.remote_mine else { return };
                        if !resources.iter().any(|&r| r) { return }
                        let Ok(amount) = amount_buf.parse::<f64>() else { return };
                        if amount <= 0.0 { return }
                        (manny_id.clone(), object_id.clone(), resources, amount, (x, y, z))
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
                    state.remote_mine = RemoteMineInput::PickContainer {
                        manny_id,
                        object_id,
                        resources,
                        amount,
                        containers,
                        selection: 0,
                        error: None,
                    };
                }
                _ => {}
            }
        }
        RemoteMineInput::PickContainer { selection, containers, .. } => {
            let sel = *selection;
            let count = containers.len();
            match code {
                KeyCode::Esc => state.remote_mine = RemoteMineInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let RemoteMineInput::PickContainer { ref mut selection, .. } = state.remote_mine {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, object_id, selected_resources, amount, container_id) = {
                        let RemoteMineInput::PickContainer {
                            ref manny_id, ref object_id, resources, amount, ref containers, selection, ..
                        } = state.remote_mine else { return };
                        let selected: Vec<String> = RESOURCE_TYPES.iter().enumerate()
                            .filter(|(i, _)| resources[*i])
                            .map(|(_, &t)| t.to_string())
                            .collect();
                        (manny_id.clone(), object_id.clone(), selected, amount, containers[selection].0.clone())
                    };
                    fetch_mine(manny_id, object_id, selected_resources, amount, Some(container_id), client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        RemoteMineInput::Inactive => {}
    }
}
