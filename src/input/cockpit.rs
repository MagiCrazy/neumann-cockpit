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
use crate::api::tasks::{fetch_all, fetch_inspect, fetch_recover, fetch_sector};
use crate::api::types::{MannyTask, MannyTaskVisibility};
use crate::app::{
    ApiMessage, AppState, CraftInput, DetachInput, DropCargoInput, InputMode, InspectInput,
    MenuAction, MineInput, Pane, RecallInput, RecoverInput, RefuelInput, RemoteMineInput,
    RenameMannyInput, RepairInput, SalvageInput,
};

pub fn handle_cockpit_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    // A context menu, when open, captures all input.
    if matches!(state.mode, InputMode::Menu(_)) {
        handle_menu_key(code, state, client, tx);
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

fn handle_menu_key(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
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
            let action = if let InputMode::Menu(m) = &state.mode {
                m.items.get(m.cursor).filter(|i| i.enabled).map(|i| i.action)
            } else {
                None
            };
            if let Some(action) = action {
                state.mode = InputMode::Normal;
                fire_menu_action(action, state, client, tx);
            }
        }
        _ => {}
    }
}

/// Launch the wizard behind a menu action for the selected Manny. Mirrors the
/// classic single-key launches; the shared wizard handlers take over next.
fn fire_menu_action(
    action: MenuAction,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    let Some(m) = state.mannies.as_ref().and_then(|v| v.get(state.mannies_selection)) else {
        return;
    };
    let id = m.id.clone();
    let name = m.name.clone();
    let can = m.can_receive_orders;
    let has_task = m.current_task.is_some();
    let waiting_space = m.current_task == Some(MannyTask::WaitingForSpace);
    let remote_recall = matches!(m.task_visibility, Some(MannyTaskVisibility::ScutNetwork));
    let remote_minable = state.manny_remote_minable(m);
    let coords = state.manny_sector_coords(m).unwrap_or((0, 0, 0));

    match action {
        MenuAction::Repair if can => {
            state.repair = RepairInput::Typing { manny_id: id, manny_name: name, buf: String::new(), error: None };
        }
        MenuAction::Craft if can => {
            if state.manny_craft_recipes().is_empty() {
                state.error = Some("recipes not loaded yet — F5 to refresh".into());
            } else {
                state.craft = CraftInput::PickRecipe { manny_id: id, manny_name: name, selection: 0, error: None };
            }
        }
        MenuAction::Mine => {
            if remote_minable {
                state.remote_mine = RemoteMineInput::Loading {
                    manny_id: id,
                    manny_name: name,
                    x: coords.0,
                    y: coords.1,
                    z: coords.2,
                };
                state.set_toast("fetching remote sector…");
                fetch_sector(Some(coords), client.clone(), tx.clone());
            } else if can {
                let candidates = state.collect_mineable_candidates();
                match candidates.len() {
                    0 => state.error = Some("no mineable objects in current sector — scan first".into()),
                    1 => {
                        let (object_id, object_name) = candidates.into_iter().next().unwrap();
                        state.mine = MineInput::Configure {
                            manny_id: id,
                            manny_name: name,
                            object_id,
                            object_name,
                            resources: [false, true, false, false],
                            amount_buf: "0.30".into(),
                            amount_mode: false,
                            target_container: None,
                            error: None,
                        };
                    }
                    _ => state.mine = MineInput::PickAsteroid { manny_id: id, manny_name: name, candidates, selection: 0 },
                }
            }
        }
        MenuAction::Salvage if can => {
            let candidates = state.collect_salvage_candidates();
            match candidates.len() {
                0 => state.error = Some("no salvageable objects in current sector — scan first".into()),
                1 => {
                    let (object_id, object_name) = candidates.into_iter().next().unwrap();
                    state.salvage = SalvageInput::Confirm { manny_id: id, manny_name: name, object_id, object_name, error: None };
                }
                _ => state.salvage = SalvageInput::PickTarget { manny_id: id, manny_name: name, candidates, selection: 0 },
            }
        }
        MenuAction::Inspect if can => {
            let candidates = state.collect_asteroid_candidates();
            match candidates.len() {
                0 => state.error = Some("no asteroids in current sector — scan first".into()),
                1 => {
                    let (object_id, _) = candidates.into_iter().next().unwrap();
                    fetch_inspect(id, object_id, client.clone(), tx.clone());
                }
                _ => state.inspect = InspectInput::PickAsteroid { manny_id: id, manny_name: name, candidates, selection: 0, error: None },
            }
        }
        MenuAction::Recover if can => {
            let candidates = state.collect_detached_containers();
            match candidates.len() {
                0 => state.error = Some("no detached containers in current sector — scan first".into()),
                1 => {
                    let (object_id, _) = candidates.into_iter().next().unwrap();
                    fetch_recover(id, object_id, client.clone(), tx.clone());
                }
                _ => state.recover = RecoverInput::PickContainer { manny_id: id, manny_name: name, candidates, selection: 0, error: None },
            }
        }
        MenuAction::Detach if can => {
            let containers = state.collect_detachable_containers();
            match containers.len() {
                0 => state.error = Some("no detachable containers in inventory".into()),
                1 => {
                    let (container_id, container_name) = containers.into_iter().next().unwrap();
                    state.detach = DetachInput::PickMode { manny_id: id, manny_name: name, container_id, container_name, selection: 0, error: None };
                }
                _ => state.detach = DetachInput::PickContainer { manny_id: id, manny_name: name, containers, selection: 0 },
            }
        }
        MenuAction::Refuel if can => {
            if state.deuterium_station_in_current_sector() {
                state.refuel = RefuelInput::Confirm { manny_id: id, manny_name: name, error: None };
            } else {
                state.error = Some("no deuterium refuel station in this sector".into());
            }
        }
        MenuAction::DropCargo if waiting_space => {
            state.drop_cargo = DropCargoInput::Confirm { manny_id: id, manny_name: name, error: None };
        }
        MenuAction::Recall if !can && has_task => {
            state.recall = RecallInput::Confirm { manny_id: id, manny_name: name, remote: remote_recall, error: None };
        }
        MenuAction::Rename => {
            state.rename_manny = RenameMannyInput::Typing { manny_id: id, manny_name: name.clone(), buf: name, error: None };
        }
        // Guard mismatch (state changed since the menu was built): no-op.
        _ => {}
    }
}
