use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::{fetch_storage_move, StorageMoveArgs};
use crate::app::{ActiveWizard, ApiMessage, AppState, LogEvent, StorageMoveInput, MOVE_RESOURCE_TYPES};

use super::geometry::list_nav;

fn wrap_prev(i: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        (i + len - 1) % len
    }
}
fn wrap_next(i: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        (i + 1) % len
    }
}

pub(super) fn handle_storage_move_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.active_wizard {
        ActiveWizard::StorageMove(StorageMoveInput::PickManny { selection, mannies }) => {
            let (sel, count) = (*selection, mannies.len());
            match code {
                KeyCode::Esc => state.close_wizard(),
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(ns) = list_nav(code, sel, count) {
                        if let ActiveWizard::StorageMove(StorageMoveInput::PickManny { selection, .. }) =
                            &mut state.active_wizard
                        {
                            *selection = ns;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (id, name) = {
                        let ActiveWizard::StorageMove(StorageMoveInput::PickManny { mannies, selection }) =
                            &state.active_wizard
                        else {
                            return;
                        };
                        mannies[*selection].clone()
                    };
                    state.active_wizard = ActiveWizard::StorageMove(StorageMoveInput::PickKind {
                        actor_manny_id: id,
                        actor_manny_name: name,
                        selection: 0,
                    });
                }
                _ => {}
            }
        }
        ActiveWizard::StorageMove(StorageMoveInput::PickKind { selection, .. }) => {
            let sel = *selection;
            match code {
                KeyCode::Esc => state.close_wizard(),
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(ns) = list_nav(code, sel, 2) {
                        if let ActiveWizard::StorageMove(StorageMoveInput::PickKind { selection, .. }) =
                            &mut state.active_wizard
                        {
                            *selection = ns;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (id, name) = {
                        let ActiveWizard::StorageMove(StorageMoveInput::PickKind {
                            actor_manny_id,
                            actor_manny_name,
                            ..
                        }) = &state.active_wizard
                        else {
                            return;
                        };
                        (actor_manny_id.clone(), actor_manny_name.clone())
                    };
                    if sel == 0 {
                        enter_resource(state, id, name);
                    } else {
                        enter_item(state, id, name);
                    }
                }
                _ => {}
            }
        }
        ActiveWizard::StorageMove(StorageMoveInput::ConfigureResource { .. }) => {
            handle_resource(code, state, client, tx)
        }
        ActiveWizard::StorageMove(StorageMoveInput::ConfigureItem { .. }) => handle_item(code, state, client, tx),
        _ => {}
    }
}

fn enter_resource(state: &mut AppState, actor_manny_id: String, actor_manny_name: String) {
    let containers = state.collect_move_containers();
    if containers.len() < 2 {
        state.close_wizard();
        state.error = Some("need at least two containers to move resources".into());
        return;
    }
    state.active_wizard = ActiveWizard::StorageMove(StorageMoveInput::ConfigureResource {
        actor_manny_id,
        actor_manny_name,
        containers,
        resource_idx: 0,
        from_sel: 0,
        to_sel: 1,
        amount_buf: "0.10".into(),
        field: 0,
        error: None,
    });
}

fn enter_item(state: &mut AppState, actor_manny_id: String, actor_manny_name: String) {
    let containers = state.collect_move_containers();
    if containers.is_empty() {
        state.close_wizard();
        state.error = Some("no containers available".into());
        return;
    }
    let items: Vec<(String, String, bool)> = state
        .collect_movable_items()
        .into_iter()
        .map(|(id, label)| (id, label, false))
        .collect();
    if items.is_empty() {
        state.close_wizard();
        state.error = Some("no movable items in inventory".into());
        return;
    }
    state.active_wizard = ActiveWizard::StorageMove(StorageMoveInput::ConfigureItem {
        actor_manny_id,
        actor_manny_name,
        containers,
        items,
        to_sel: 0,
        item_cursor: 0,
        field: 0,
        error: None,
    });
}

fn handle_resource(code: KeyCode, state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    let ActiveWizard::StorageMove(StorageMoveInput::ConfigureResource {
        actor_manny_id,
        containers,
        resource_idx,
        from_sel,
        to_sel,
        amount_buf,
        field,
        ..
    }) = &mut state.active_wizard
    else {
        return;
    };
    let clen = containers.len();
    match code {
        KeyCode::Esc => state.close_wizard(),
        KeyCode::Up | KeyCode::BackTab => *field = wrap_prev(*field as usize, 4) as u8,
        KeyCode::Down | KeyCode::Tab => *field = wrap_next(*field as usize, 4) as u8,
        KeyCode::Left | KeyCode::Char('h') => match field {
            0 => *resource_idx = wrap_prev(*resource_idx, MOVE_RESOURCE_TYPES.len()),
            1 => *from_sel = wrap_prev(*from_sel, clen),
            2 => *to_sel = wrap_prev(*to_sel, clen),
            _ => {}
        },
        KeyCode::Right | KeyCode::Char('l') => match field {
            0 => *resource_idx = wrap_next(*resource_idx, MOVE_RESOURCE_TYPES.len()),
            1 => *from_sel = wrap_next(*from_sel, clen),
            2 => *to_sel = wrap_next(*to_sel, clen),
            _ => {}
        },
        KeyCode::Char(c) if *field == 3 && (c.is_ascii_digit() || c == '.') => {
            if amount_buf.chars().count() < 10 {
                amount_buf.push(c);
            }
        }
        KeyCode::Backspace if *field == 3 => {
            amount_buf.pop();
        }
        KeyCode::Enter => {
            if from_sel == to_sel {
                state.set_storage_move_error("source and destination must differ".into());
                return;
            }
            let amount: f64 = match amount_buf.trim().parse() {
                Ok(a) if a > 0.0 => a,
                _ => {
                    state.set_storage_move_error("amount must be a positive number".into());
                    return;
                }
            };
            let actor = actor_manny_id.clone();
            let resource = MOVE_RESOURCE_TYPES[*resource_idx].to_string();
            let from_id = containers[*from_sel].0.clone();
            let from_name = containers[*from_sel].1.clone();
            let to_id = containers[*to_sel].0.clone();
            let to_name = containers[*to_sel].1.clone();
            fetch_storage_move(
                StorageMoveArgs {
                    actor_manny_id: actor,
                    kind: "resource".into(),
                    to_container_id: to_id,
                    from_container_id: Some(from_id),
                    resource_type: Some(resource.clone()),
                    amount: Some(amount),
                    item_ids: None,
                },
                client.clone(),
                tx.clone(),
            );
            state.log_event(LogEvent::storage_move_resource(
                amount,
                &resource,
                &from_name,
                &to_name,
                state.active_probe_id,
            ));
        }
        _ => {}
    }
}

fn handle_item(code: KeyCode, state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    let ActiveWizard::StorageMove(StorageMoveInput::ConfigureItem {
        actor_manny_id,
        containers,
        items,
        to_sel,
        item_cursor,
        field,
        ..
    }) = &mut state.active_wizard
    else {
        return;
    };
    let clen = containers.len();
    let ilen = items.len();
    match code {
        KeyCode::Esc => state.close_wizard(),
        KeyCode::Tab => *field = if *field == 0 { 1 } else { 0 },
        KeyCode::Up | KeyCode::Char('k') if *field == 0 => *item_cursor = wrap_prev(*item_cursor, ilen),
        KeyCode::Down | KeyCode::Char('j') if *field == 0 => *item_cursor = wrap_next(*item_cursor, ilen),
        KeyCode::Char(' ') if *field == 0 => {
            if let Some(it) = items.get_mut(*item_cursor) {
                it.2 = !it.2;
            }
        }
        KeyCode::Left | KeyCode::Char('h') if *field == 1 => *to_sel = wrap_prev(*to_sel, clen),
        KeyCode::Right | KeyCode::Char('l') if *field == 1 => *to_sel = wrap_next(*to_sel, clen),
        KeyCode::Enter => {
            let ids: Vec<String> = items
                .iter()
                .filter(|(_, _, sel)| *sel)
                .map(|(id, _, _)| id.clone())
                .collect();
            if ids.is_empty() {
                state.set_storage_move_error("select at least one item with Space".into());
                return;
            }
            let actor = actor_manny_id.clone();
            let to_id = containers[*to_sel].0.clone();
            let to_name = containers[*to_sel].1.clone();
            let count = ids.len();
            fetch_storage_move(
                StorageMoveArgs {
                    actor_manny_id: actor,
                    kind: "item".into(),
                    to_container_id: to_id,
                    from_container_id: None,
                    resource_type: None,
                    amount: None,
                    item_ids: Some(ids),
                },
                client.clone(),
                tx.clone(),
            );
            state.log_event(LogEvent::storage_move_items(count, &to_name, state.active_probe_id));
        }
        _ => {}
    }
}
