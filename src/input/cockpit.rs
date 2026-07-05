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
use crate::api::tasks::{
    fetch_all, fetch_inspect, fetch_messages, fetch_recover, fetch_scut_network, fetch_sector,
    fetch_sent_messages, fetch_storage_container_detail,
};
use crate::api::types::{MannyTask, MannyTaskVisibility};
use crate::app::{
    ApiMessage, AppState, DeployInput, FabricationInput, DetachInput, DropCargoInput,
    CommandLine, DropStorageContainerInput, DrillLevel, InputMode, InspectInput, MenuAction,
    MessagesInput,
    MindSnapshotInput, MineInput, MissionsInput, ObjectActionInput, Pane, RecallInput, RecoverInput,
    GotoVisitedInput, RefuelInput, RemoteMineInput, RenameContainerInput, RenameMannyInput,
    RepairInput, SalvageInput, ScanMode, ScutNetworkInput, StorageMoveInput, TravelInput,
    WaypointsInput,
};

pub fn handle_cockpit_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    // Cockpit keys (ertdfgcvb, jkhl, z, q, …) are all lowercase, but CapsLock —
    // or Shift — sends uppercase letters, and no cockpit binding uses those.
    // Normalize so the grid stays navigable regardless of CapsLock. Text entry
    // is handled by the wizard/command layers before this, so it is unaffected.
    let code = match code {
        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
        other => other,
    };

    // A context menu, when open, captures all input.
    if matches!(state.mode, InputMode::Menu(_)) {
        handle_menu_key(code, state, client, tx);
        return;
    }

    match code {
        KeyCode::Char('q') => state.set_quit(),
        KeyCode::Enter => open_actions(state, client, tx),
        KeyCode::Char(c) if Pane::from_key(c).is_some() => {
            state.active_pane = Pane::from_key(c).unwrap();
        }
        KeyCode::Down | KeyCode::Char('j') => state.pane_cursor_down(),
        KeyCode::Up | KeyCode::Char('k') => state.pane_cursor_up(),
        // Paging + jump to ends, for lists that grow over a session (scan
        // history, messages). `g`/`G` are pane keys, so use PageUp/Down/Home/End.
        KeyCode::PageDown => state.pane_cursor_page_down(),
        KeyCode::PageUp => state.pane_cursor_page_up(),
        KeyCode::Home => state.pane_cursor_top(),
        KeyCode::End => state.pane_cursor_bottom(),
        KeyCode::Right | KeyCode::Char('l') => drill_in(state, client, tx),
        KeyCode::Left | KeyCode::Char('h') => {
            drill_out(state);
        }
        // The Map pane has no in-pane zoom view — `z` opens the full isometric
        // map overlay (its own pan/travel controls take over).
        KeyCode::Char('z') if state.active_pane == Pane::Map => state.open_map(),
        KeyCode::Char('z') => state.toggle_zoom(),
        KeyCode::Char(':') => state.mode = InputMode::Command(CommandLine::default()),
        KeyCode::Char('?') => state.help_open = true,
        KeyCode::F(1) => state.hints_visible = !state.hints_visible,
        // Esc backs out one step: leave zoom first, then drill up.
        KeyCode::Esc => {
            if state.zoomed {
                state.zoomed = false;
            } else {
                drill_out(state);
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

/// `Enter` action for the active pane: panes with a discrete action set open
/// the contextual menu; panes backed by a rich wizard reuse its overlay.
fn open_actions(state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    match state.active_pane {
        Pane::Mannies | Pane::Inventory | Pane::Probe | Pane::Storage | Pane::Scanner | Pane::Map => {
            match state.build_context_menu() {
                Some(menu) if !menu.items.is_empty() => state.mode = InputMode::Menu(menu),
                _ => state.set_toast("no actions here"),
            }
        }
        Pane::Missions => {
            if state.missions.is_empty() {
                state.set_toast("no missions");
            } else {
                let selection = state.pane_nav[Pane::Missions.index()].cursor;
                state.missions_input = MissionsInput::Browsing { selection };
            }
        }
        Pane::Comms => {
            state.messages_input = MessagesInput::Browsing { sent_tab: false, selection: 0 };
            fetch_messages(client.clone(), tx.clone());
            fetch_sent_messages(client.clone(), tx.clone());
        }
        Pane::Sector => open_sector_object_actions(state),
    }
}

/// Drill into the selected element. For Storage, this fetches the container's
/// contents so they can be rendered inline (no legacy content modal).
fn drill_in(state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    state.pane_drill_in();
    if state.active_pane == Pane::Storage {
        if let Some(DrillLevel::Container(id)) =
            state.pane_nav[Pane::Storage.index()].drill.last().cloned()
        {
            state.storage_container_detail = None;
            state.storage_container_detail_error = None;
            fetch_storage_container_detail(id, client.clone(), tx.clone());
        }
    }
}

/// Drill out one level, clearing any transient detail loaded for the level.
fn drill_out(state: &mut AppState) {
    if state.active_pane == Pane::Storage {
        state.storage_container_detail = None;
        state.storage_container_detail_error = None;
    }
    state.pane_drill_out();
}

fn open_sector_object_actions(state: &mut AppState) {
    let entries = state.scanner_objects();
    let cur = state.pane_nav[Pane::Sector.index()].cursor;
    let Some(entry) = entries.get(cur) else {
        state.set_toast("no object selected");
        return;
    };
    let actions = state.actions_for_object(entry);
    if actions.is_empty() {
        state.set_toast(format!("no actions for {}", entry.name));
        return;
    }
    state.object_action = ObjectActionInput::PickAction {
        object_id: entry.id.clone(),
        object_name: entry.name.clone(),
        actions,
        selection: 0,
    };
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
        // 1-9 accelerators: fire the nth enabled item directly (one keystroke
        // instead of walking there with j/k). Disabled/out-of-range digits noop.
        KeyCode::Char(c @ '1'..='9') => {
            let idx = c as usize - '1' as usize;
            let action = if let InputMode::Menu(m) = &state.mode {
                m.items.get(idx).filter(|i| i.enabled).map(|i| i.action)
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
    // Inventory-pane actions operate on the selected inventory row, not a Manny.
    match action {
        MenuAction::Jettison => {
            match state.jettison_for_selected() {
                Ok(input) => state.jettison = input,
                Err(msg) => state.error = Some(msg),
            }
            return;
        }
        // The Inventory pane opens the catalog with no builder pre-chosen; the
        // Mannies-pane variant (with a builder) is handled further down.
        MenuAction::Fabricate if state.active_pane == Pane::Inventory => {
            if state.fabrication_recipes().is_empty() {
                state.error = Some("recipes not loaded yet — F5 to refresh".into());
            } else {
                state.fabrication = FabricationInput::PickRecipe {
                    prefilled_manny: None,
                    selection: 0,
                    error: None,
                };
            }
            return;
        }
        MenuAction::MoveStock => {
            let mannies = state.collect_idle_onboard_mannies();
            match mannies.len() {
                0 => state.error = Some("no idle Manny on board".into()),
                1 => {
                    let (id, name) = mannies.into_iter().next().unwrap();
                    state.storage_move = StorageMoveInput::PickKind {
                        actor_manny_id: id,
                        actor_manny_name: name,
                        selection: 0,
                    };
                }
                _ => state.storage_move = StorageMoveInput::PickManny { mannies, selection: 0 },
            }
            return;
        }
        MenuAction::Deploy => {
            // Deploy a held waypoint bookmark: pick the installing Manny, then a
            // target object in the current sector, then a name.
            let mannies = state.collect_idle_onboard_mannies();
            if mannies.is_empty() {
                state.error = Some("no idle Manny on board".into());
            } else if state.collect_deploy_candidates().is_empty() {
                state.error = Some("no target object in current sector — scan first".into());
            } else {
                state.deploy = DeployInput::PickManny { mannies, selection: 0 };
            }
            return;
        }
        MenuAction::ScutInspect => {
            let nets = state.scut_coverage();
            match nets.len() {
                0 => state.error = Some("no SCUT network covers this sector".into()),
                1 => {
                    state.scut_network = ScutNetworkInput::Viewing { error: None };
                    state.scut_network_view = None;
                    fetch_scut_network(nets[0].0, client.clone(), tx.clone());
                }
                _ => state.scut_network = ScutNetworkInput::Picking { networks: nets, selection: 0 },
            }
            return;
        }
        MenuAction::MindSnapshot => {
            if state.probe_terminal_alert().is_some() {
                state.mind_snapshot = MindSnapshotInput::Confirm { error: None };
            }
            return;
        }
        MenuAction::RenameContainer => {
            if let Some(id) = state.storage_selected_container_id() {
                if let Some((container_id, label)) =
                    state.storage_container(&id).map(|c| (c.id.clone(), c.label.clone()))
                {
                    state.rename_container = RenameContainerInput::Typing {
                        container_id,
                        current_label: label.clone(),
                        buf: label,
                        error: None,
                    };
                }
            }
            return;
        }
        MenuAction::EditContainerRules => {
            if let Some(id) = state.storage_selected_container_id() {
                if let Some(editor) = state.rules_editor_for(&id) {
                    state.container_rules = editor;
                }
            }
            return;
        }
        MenuAction::ScanAround => {
            if let Some((x, y, z)) = state.probe_sector_coords() {
                let offsets = super::geometry::neighbors_d1();
                state.start_batch(offsets.len());
                for (dx, dy, dz) in offsets {
                    fetch_sector(Some((x + dx, y + dy, z + dz)), client.clone(), tx.clone());
                }
            }
            return;
        }
        MenuAction::ScanDirection => {
            if state.probe_sector_coords().is_some() {
                state.scan_mode = ScanMode::DirectionPick;
            }
            return;
        }
        MenuAction::ScanObserve => {
            state.scan_mode = ScanMode::Input(String::new());
            return;
        }
        MenuAction::ScanFilter => {
            state.cycle_scan_filter();
            state.set_toast(format!("filter: {}", state.scan_filter.label()));
            return;
        }
        MenuAction::ScanTravel => {
            if let Some(s) = state.current_sector() {
                let c = &s.relative_coordinates;
                let (x, y, z) = (c.x.round() as i32, c.y.round() as i32, c.z.round() as i32);
                state.travel_go_sector(x, y, z);
            }
            return;
        }
        MenuAction::OpenMap => {
            state.open_map();
            return;
        }
        MenuAction::Travel => {
            state.travel = TravelInput::Typing(String::new());
            return;
        }
        MenuAction::GotoVisited => {
            if !state.visited_sectors.is_empty() {
                state.goto_visited = GotoVisitedInput::Picking { selection: 0 };
            }
            return;
        }
        MenuAction::Waypoints => {
            let entries = state.collect_waypoints();
            if !entries.is_empty() {
                state.waypoints = WaypointsInput::Browsing { entries, selection: 0 };
            }
            return;
        }
        _ => {}
    }

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
        MenuAction::Fabricate if can => {
            if state.fabrication_recipes().is_empty() {
                state.error = Some("recipes not loaded yet — F5 to refresh".into());
            } else {
                state.fabrication = FabricationInput::PickRecipe {
                    prefilled_manny: Some((id, name)),
                    selection: 0,
                    error: None,
                };
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
        MenuAction::DropStorageContainer if can => {
            let containers = state.collect_detachable_containers();
            if containers.is_empty() {
                state.error = Some("no detachable containers in inventory".into());
            } else if !state.has_atmospheric_drop_kit() {
                state.error = Some("no atmospheric_drop_kit in inventory".into());
            } else {
                let planets = state.collect_planet_candidates();
                if planets.is_empty() {
                    state.error = Some("no planet in current sector — scan first".into());
                } else if containers.len() == 1 {
                    let (container_id, container_name) = containers.into_iter().next().unwrap();
                    state.drop_container = DropStorageContainerInput::PickPlanet {
                        manny_id: id,
                        manny_name: name,
                        container_id,
                        container_name,
                        planets,
                        selection: 0,
                        error: None,
                    };
                } else {
                    state.drop_container = DropStorageContainerInput::PickContainer {
                        manny_id: id,
                        manny_name: name,
                        containers,
                        selection: 0,
                    };
                }
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
