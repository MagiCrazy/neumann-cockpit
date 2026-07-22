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
        KeyCode::Char('d') => {
            // "Dedicate this container to the [P] types": strict-exclude every
            // other known type. This is the whitelist intent that [S] alone
            // does not express (issue #234).
            reserve_for_priority(state);
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

/// Reserve the container for its [P] (priority) types: every other known type
/// is moved to strict exclusion (and dropped from plain exclusion), so nothing
/// but the priority types can be auto-placed here — the "dedicate to X" intent
/// that a bare strict-exclusion cannot express (issue #234). Requires at least
/// one priority type; otherwise it sets a guiding error and changes nothing.
fn reserve_for_priority(state: &mut AppState) {
    if let ActiveWizard::ContainerRules(ContainerRulesInput::Editing {
        types,
        priority,
        exclusion,
        strict_exclusion,
        error,
        ..
    }) = &mut state.active_wizard
    {
        if priority.is_empty() {
            *error = Some("mark the wanted type(s) [P] first, then [d] to reserve".into());
            return;
        }
        for ty in types.iter() {
            if priority.iter().any(|t| t == ty) {
                continue;
            }
            exclusion.retain(|t| t != ty);
            if !strict_exclusion.iter().any(|t| t == ty) {
                strict_exclusion.push(ty.clone());
            }
        }
        *error = None;
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

#[cfg(test)]
mod tests {
    use super::reserve_for_priority;
    use crate::app::{ActiveWizard, AppState, ContainerRulesInput};

    fn editing(priority: &[&str], exclusion: &[&str], strict: &[&str]) -> AppState {
        let mut s = AppState::default();
        s.active_wizard = ActiveWizard::ContainerRules(ContainerRulesInput::Editing {
            container_id: "c1".into(),
            container_label: "hold".into(),
            types: vec!["metals".into(), "ice".into(), "carbon".into(), "deuterium".into()],
            priority: priority.iter().map(|s| s.to_string()).collect(),
            exclusion: exclusion.iter().map(|s| s.to_string()).collect(),
            strict_exclusion: strict.iter().map(|s| s.to_string()).collect(),
            selection: 0,
            error: None,
        });
        s
    }

    fn lists(s: &AppState) -> (Vec<String>, Vec<String>, Vec<String>, Option<String>) {
        let ActiveWizard::ContainerRules(ContainerRulesInput::Editing {
            priority,
            exclusion,
            strict_exclusion,
            error,
            ..
        }) = &s.active_wizard
        else {
            panic!("not editing");
        };
        (
            priority.clone(),
            exclusion.clone(),
            strict_exclusion.clone(),
            error.clone(),
        )
    }

    #[test]
    fn reserve_strict_excludes_every_non_priority_type() {
        // Dedicate to ice + carbon: metals and deuterium must be strict-excluded.
        let mut s = editing(&["ice", "carbon"], &["metals"], &[]);
        reserve_for_priority(&mut s);
        let (priority, exclusion, strict, error) = lists(&s);
        assert_eq!(priority, vec!["ice", "carbon"]);
        assert!(
            exclusion.is_empty(),
            "plain exclusion should be cleared for reserved types"
        );
        assert!(strict.contains(&"metals".to_string()));
        assert!(strict.contains(&"deuterium".to_string()));
        assert!(!strict.contains(&"ice".to_string()));
        assert!(!strict.contains(&"carbon".to_string()));
        assert!(error.is_none());
    }

    #[test]
    fn reserve_without_priority_sets_guiding_error_and_no_change() {
        let mut s = editing(&[], &[], &[]);
        reserve_for_priority(&mut s);
        let (priority, exclusion, strict, error) = lists(&s);
        assert!(priority.is_empty() && exclusion.is_empty() && strict.is_empty());
        assert!(error.unwrap().contains("[P]"));
    }
}
