use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::{fetch_rename_container, fetch_update_container_rules};
use crate::app::{ActiveWizard, ApiMessage, AppState, ContainerRulesInput, LogEvent, RenameContainerInput};

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
    if !matches!(
        state.active_wizard,
        ActiveWizard::RenameContainer(RenameContainerInput::Typing { .. })
    ) {
        return;
    }
    match code {
        KeyCode::Esc => state.close_wizard(),
        KeyCode::Tab => {
            let s = state.next_name_suggestion();
            if let ActiveWizard::RenameContainer(RenameContainerInput::Typing { buf, .. }) = &mut state.active_wizard {
                *buf = s;
            }
        }
        KeyCode::Backspace => {
            if let ActiveWizard::RenameContainer(RenameContainerInput::Typing { buf, .. }) = &mut state.active_wizard {
                buf.pop();
            }
        }
        KeyCode::Enter => {
            let (id, label) = {
                let ActiveWizard::RenameContainer(RenameContainerInput::Typing { container_id, buf, .. }) =
                    &state.active_wizard
                else {
                    return;
                };
                (container_id.clone(), buf.trim().to_string())
            };
            if label.is_empty() {
                state.set_wizard_error("label cannot be empty".into());
                return;
            }
            let new_label = label.clone();
            fetch_rename_container(id, label, client.clone(), tx.clone());
            state.log_event(LogEvent::rename_container(&new_label, state.active_probe_id));
        }
        KeyCode::Char(c) => {
            if let ActiveWizard::RenameContainer(RenameContainerInput::Typing { buf, error, .. }) =
                &mut state.active_wizard
            {
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
    let ActiveWizard::ContainerRules(ContainerRulesInput::Editing { selection, types, .. }) = &state.active_wizard
    else {
        return;
    };
    let sel = *selection;
    let count = types.len();
    match code {
        KeyCode::Esc => state.close_wizard(),
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(ns) = list_nav(code, sel, count) {
                if let ActiveWizard::ContainerRules(ContainerRulesInput::Editing { selection, .. }) =
                    &mut state.active_wizard
                {
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
            if let ActiveWizard::ContainerRules(ContainerRulesInput::Editing {
                types,
                priority,
                exclusion,
                strict_exclusion,
                selection,
                ..
            }) = &mut state.active_wizard
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
                let ActiveWizard::ContainerRules(ContainerRulesInput::Editing {
                    container_id,
                    priority,
                    exclusion,
                    strict_exclusion,
                    container_label,
                    ..
                }) = &state.active_wizard
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
    if let ActiveWizard::ContainerRules(ContainerRulesInput::Editing {
        types,
        priority,
        exclusion,
        strict_exclusion,
        selection,
        ..
    }) = &mut state.active_wizard
    {
        if let Some(ty) = types.get(*selection).cloned() {
            cycle_assignment(&ty, priority, exclusion, strict_exclusion, backward);
        }
    }
}
