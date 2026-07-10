use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::{fetch_rename_container, fetch_update_container_rules};
use crate::app::{ApiMessage, AppState, ContainerRulesInput, LogEvent, RenameContainerInput};

use super::geometry::list_nav;

/// Move a type to the next/previous routing list:
/// none → priority → exclusion → strict → none.
fn cycle_assignment(
    ty: &str,
    priority: &mut Vec<String>,
    exclusion: &mut Vec<String>,
    strict: &mut Vec<String>,
    backward: bool,
) {
    let cur = if priority.iter().any(|t| t == ty) {
        1
    } else if exclusion.iter().any(|t| t == ty) {
        2
    } else if strict.iter().any(|t| t == ty) {
        3
    } else {
        0
    };
    priority.retain(|t| t != ty);
    exclusion.retain(|t| t != ty);
    strict.retain(|t| t != ty);
    let next = if backward { (cur + 3) % 4 } else { (cur + 1) % 4 };
    match next {
        1 => priority.push(ty.to_string()),
        2 => exclusion.push(ty.to_string()),
        3 => strict.push(ty.to_string()),
        _ => {}
    }
}

pub(super) fn handle_rename_container_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    if !matches!(state.rename_container, RenameContainerInput::Typing { .. }) {
        return;
    }
    match code {
        KeyCode::Esc => state.rename_container = RenameContainerInput::Inactive,
        KeyCode::Backspace => {
            if let RenameContainerInput::Typing { buf, .. } = &mut state.rename_container {
                buf.pop();
            }
        }
        KeyCode::Enter => {
            let (id, label) = {
                let RenameContainerInput::Typing { container_id, buf, .. } = &state.rename_container
                else {
                    return;
                };
                (container_id.clone(), buf.trim().to_string())
            };
            if label.is_empty() {
                state.set_rename_container_error("label cannot be empty".into());
                return;
            }
            let new_label = label.clone();
            fetch_rename_container(id, label, client.clone(), tx.clone());
            state.log_event(LogEvent::rename_container(&new_label, state.active_probe_id));
        }
        KeyCode::Char(c) => {
            if let RenameContainerInput::Typing { buf, error, .. } = &mut state.rename_container {
                if buf.chars().count() < 80 {
                    buf.push(c);
                    *error = None;
                }
            }
        }
        _ => {}
    }
}

pub(super) fn handle_container_rules_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    let ContainerRulesInput::Editing { selection, types, .. } = &state.container_rules else {
        return;
    };
    let sel = *selection;
    let count = types.len();
    match code {
        KeyCode::Esc => state.container_rules = ContainerRulesInput::Inactive,
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(ns) = list_nav(code, sel, count) {
                if let ContainerRulesInput::Editing { selection, .. } = &mut state.container_rules {
                    *selection = ns;
                }
            }
        }
        KeyCode::Char(' ') | KeyCode::Right | KeyCode::Char('l') => {
            cycle_selected(state, false);
        }
        KeyCode::Left | KeyCode::Char('h') => {
            cycle_selected(state, true);
        }
        KeyCode::Delete | KeyCode::Backspace => {
            // Clear the selected type back to "none".
            if let ContainerRulesInput::Editing {
                types, priority, exclusion, strict_exclusion, selection, ..
            } = &mut state.container_rules
            {
                if let Some(ty) = types.get(*selection).cloned() {
                    priority.retain(|t| t != &ty);
                    exclusion.retain(|t| t != &ty);
                    strict_exclusion.retain(|t| t != &ty);
                }
            }
        }
        KeyCode::Enter => {
            let (id, p, e, s, label) = {
                let ContainerRulesInput::Editing {
                    container_id, priority, exclusion, strict_exclusion, container_label, ..
                } = &state.container_rules
                else {
                    return;
                };
                (
                    container_id.clone(),
                    priority.clone(),
                    exclusion.clone(),
                    strict_exclusion.clone(),
                    container_label.clone(),
                )
            };
            fetch_update_container_rules(id, p, e, s, client.clone(), tx.clone());
            state.log_event(LogEvent::container_rules(&label, state.active_probe_id));
        }
        _ => {}
    }
}

fn cycle_selected(state: &mut AppState, backward: bool) {
    if let ContainerRulesInput::Editing {
        types, priority, exclusion, strict_exclusion, selection, ..
    } = &mut state.container_rules
    {
        if let Some(ty) = types.get(*selection).cloned() {
            cycle_assignment(&ty, priority, exclusion, strict_exclusion, backward);
        }
    }
}
