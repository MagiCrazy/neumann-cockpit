//! Input routing for the unified Cockpit v2 interface (blocs U2–U5).
//!
//! Normal-mode navigation: `ertdfgcvb` activates a pane, `jk`/arrows move the
//! cursor within it, `l`/`h` drill in/out, `z` zooms, `Tab` cycles panes,
//! `F1` toggles the hints line, `F5` refreshes. `Enter` opens the contextual
//! action menu, which launches the existing wizards. Command mode (`:`) lands
//! in a later bloc; unhandled keys are ignored.

use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::fetch_all;
use crate::api::types::MannyTaskVisibility;
use crate::app::{
    ApiMessage, AppState, CraftInput, InputMode, MenuAction, Pane, RecallInput, RenameMannyInput,
    RepairInput,
};

pub fn handle_cockpit_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    // A context menu, when open, captures all input.
    if matches!(state.mode, InputMode::Menu(_)) {
        handle_menu_key(code, state);
        return;
    }

    match code {
        KeyCode::Char('q') => state.set_quit(),
        KeyCode::Enter => open_context_menu(state),
        KeyCode::Char(c) if Pane::from_key(c).is_some() => {
            state.active_pane = Pane::from_key(c).unwrap();
        }
        KeyCode::Down | KeyCode::Char('j') => state.pane_cursor_down(),
        KeyCode::Up | KeyCode::Char('k') => state.pane_cursor_up(),
        KeyCode::Right | KeyCode::Char('l') => state.pane_drill_in(),
        KeyCode::Left | KeyCode::Char('h') => {
            state.pane_drill_out();
        }
        KeyCode::Char('z') => state.toggle_zoom(),
        KeyCode::F(1) => state.hints_visible = !state.hints_visible,
        // Esc backs out one step: leave zoom first, then drill up.
        KeyCode::Esc => {
            if state.zoomed {
                state.zoomed = false;
            } else {
                state.pane_drill_out();
            }
        }
        KeyCode::Tab => state.cycle_pane(true),
        KeyCode::BackTab => state.cycle_pane(false),
        KeyCode::F(5) if !state.loading => {
            state.clear_error();
            state.loading = true;
            fetch_all(client.clone(), tx.clone());
        }
        _ => {}
    }
}

fn open_context_menu(state: &mut AppState) {
    match state.build_context_menu() {
        Some(menu) if !menu.items.is_empty() => state.mode = InputMode::Menu(menu),
        _ => state.set_toast("no actions here"),
    }
}

fn handle_menu_key(code: KeyCode, state: &mut AppState) {
    match code {
        KeyCode::Esc => state.mode = InputMode::Normal,
        KeyCode::Up | KeyCode::Char('k') => {
            if let InputMode::Menu(m) = &mut state.mode {
                m.cursor = m.cursor.saturating_sub(1);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let InputMode::Menu(m) = &mut state.mode {
                if m.cursor + 1 < m.items.len() {
                    m.cursor += 1;
                }
            }
        }
        KeyCode::Enter => {
            // Read the selected action without holding the borrow across the fire.
            let action = if let InputMode::Menu(m) = &state.mode {
                m.items.get(m.cursor).filter(|i| i.enabled).map(|i| i.action)
            } else {
                None
            };
            if let Some(action) = action {
                state.mode = InputMode::Normal;
                fire_menu_action(action, state);
            }
        }
        _ => {}
    }
}

/// Launch the wizard behind a menu action for the selected Manny. The
/// existing wizard event handlers take over on subsequent keys.
fn fire_menu_action(action: MenuAction, state: &mut AppState) {
    let Some(m) = state.mannies.as_ref().and_then(|v| v.get(state.mannies_selection)) else {
        return;
    };
    let id = m.id.clone();
    let name = m.name.clone();
    let can = m.can_receive_orders;
    let has_task = m.current_task.is_some();
    let remote = matches!(m.task_visibility, Some(MannyTaskVisibility::ScutNetwork));

    match action {
        MenuAction::Repair if can => {
            state.repair = RepairInput::Typing {
                manny_id: id,
                manny_name: name,
                buf: String::new(),
                error: None,
            };
        }
        MenuAction::Craft if can => {
            if state.manny_craft_recipes().is_empty() {
                state.error = Some("recipes not loaded yet — F5 to refresh".into());
            } else {
                state.craft = CraftInput::PickRecipe {
                    manny_id: id,
                    manny_name: name,
                    selection: 0,
                    error: None,
                };
            }
        }
        MenuAction::Recall if !can && has_task => {
            state.recall = RecallInput::Confirm {
                manny_id: id,
                manny_name: name,
                remote,
                error: None,
            };
        }
        MenuAction::Rename => {
            state.rename_manny = RenameMannyInput::Typing {
                manny_id: id,
                manny_name: name.clone(),
                buf: name,
                error: None,
            };
        }
        // Guard mismatch (state changed since the menu was built): no-op.
        _ => {}
    }
}
