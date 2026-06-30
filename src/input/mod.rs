use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::{
    fetch_all,
    fetch_inspect,
    fetch_recover, fetch_scut_network, fetch_sector, fetch_storage_containers,
};
use crate::api::types::MannyTask;
use crate::app::{
    AlertsInput, ApiMessage, AppState, AtomicPrinterCraftInput, ContainerRulesInput,
    ContainersInput, CraftInput, DeployInput, DetachInput, DropCargoInput,
    DropStorageContainerInput, InspectInput, JettisonInput, MindSnapshotInput, MineInput,
    MissionsInput, ObjectActionInput, Panel, RecallInput, RecoverInput, RefuelInput,
    RemoteMineInput, RenameContainerInput, RenameMannyInput, RepairInput, SalvageInput, ScanMode,
    ScutNetworkInput, ScutRelayInput, StorageMoveInput, TravelInput, WaypointsInput,
};
mod alerts;
mod containers;
mod craft;
mod geometry;
mod jettison;
mod map;
mod mine;
mod missions;
mod pickers;
mod repair;
mod scanner;
mod storage_move;
mod travel;

use alerts::handle_alerts_event;
use containers::{
    handle_container_rules_event, handle_containers_event, handle_rename_container_event,
};
use craft::{handle_atomic_printer_craft_event, handle_craft_event};
use geometry::{face_d2, neighbors_d1};
use jettison::handle_jettison_event;
use map::handle_map_event;
use mine::{handle_mine_event, handle_remote_mine_event};
use missions::handle_missions_event;
use pickers::{
    handle_deploy_event, handle_detach_event, handle_drop_cargo_event,
    handle_drop_container_event, handle_inspect_event, handle_mind_snapshot_event,
    handle_recall_event, handle_recover_event, handle_refuel_event, handle_rename_manny_event,
    handle_salvage_event,
};
use repair::handle_repair_event;
use scanner::{
    handle_object_action_event, handle_scut_network_event, handle_scut_relay_event,
    handle_waypoints_event,
};
use storage_move::handle_storage_move_event;
use travel::handle_travel_event;
pub fn handle_event(
    event: Event,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    let Event::Key(k) = event else { return };
    if k.kind != KeyEventKind::Press { return };
    // Toasts are transient: any keypress dismisses the current one.
    state.toast = None;
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let in_scan_input = matches!(state.scan_mode, ScanMode::Input(_));
    let in_direction_pick = matches!(state.scan_mode, ScanMode::DirectionPick);
    let in_travel = !matches!(state.travel, TravelInput::Inactive);
    let in_repair = !matches!(state.repair, RepairInput::Inactive);
    let in_jettison = !matches!(state.jettison, JettisonInput::Inactive);
    let in_craft = !matches!(state.craft, CraftInput::Inactive);
    let in_atomic_craft = !matches!(state.atomic_printer_craft, AtomicPrinterCraftInput::Inactive);
    let in_salvage = !matches!(state.salvage, SalvageInput::Inactive);
    let in_recall = !matches!(state.recall, RecallInput::Inactive);
    let in_refuel = !matches!(state.refuel, RefuelInput::Inactive);
    let in_mind_snapshot = !matches!(state.mind_snapshot, MindSnapshotInput::Inactive);
    let in_remote_mine = !matches!(state.remote_mine, RemoteMineInput::Inactive);
    let in_scut_relay = !matches!(state.scut_relay, ScutRelayInput::Inactive);
    let in_scut_network = !matches!(state.scut_network, ScutNetworkInput::Inactive);
    let in_missions = !matches!(state.missions_input, MissionsInput::Inactive);
    let in_rename_manny = !matches!(state.rename_manny, RenameMannyInput::Inactive);
    let in_deploy = !matches!(state.deploy, DeployInput::Inactive);
    let in_inspect = !matches!(state.inspect, InspectInput::Inactive);
    let in_recover = !matches!(state.recover, RecoverInput::Inactive);
    let in_detach = !matches!(state.detach, DetachInput::Inactive);
    let in_alerts = !matches!(state.alerts_input, AlertsInput::Inactive);
    let in_rename_container = !matches!(state.rename_container, RenameContainerInput::Inactive);
    let in_container_rules = !matches!(state.container_rules, ContainerRulesInput::Inactive);
    let in_containers = !matches!(state.containers_input, ContainersInput::Inactive);
    let in_storage_move = !matches!(state.storage_move, StorageMoveInput::Inactive);
    let in_drop_cargo = !matches!(state.drop_cargo, DropCargoInput::Inactive);
    let in_drop_container = !matches!(state.drop_container, DropStorageContainerInput::Inactive);

    if ctrl && k.code == KeyCode::Char('c') {
        state.set_quit();
        return;
    }

    // Retro boot sequence: any key skips it.
    if state.anim.booting {
        state.skip_boot();
        return;
    }

    if k.code == KeyCode::F(2) {
        state.toggle_theme();
        return;
    }

    if state.help_open {
        if matches!(k.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')) {
            state.help_open = false;
        }
        return;
    }

    if state.inventory_detail_open {
        if matches!(k.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')) {
            state.inventory_detail_open = false;
        }
        return;
    }

    if state.map.open {
        handle_map_event(k.code, state);
        return;
    }

    if in_jettison {
        handle_jettison_event(k.code, state, client, tx);
        return;
    }

    if in_craft {
        handle_craft_event(k.code, state, client, tx);
        return;
    }

    if in_atomic_craft {
        handle_atomic_printer_craft_event(k.code, state, client, tx);
        return;
    }

    if in_salvage {
        handle_salvage_event(k.code, state, client, tx);
        return;
    }

    if in_recall {
        handle_recall_event(k.code, state, client, tx);
        return;
    }

    if in_refuel {
        handle_refuel_event(k.code, state, client, tx);
        return;
    }

    if in_mind_snapshot {
        handle_mind_snapshot_event(k.code, state, client, tx);
        return;
    }

    if in_scut_relay {
        handle_scut_relay_event(k.code, state, client, tx);
        return;
    }

    if in_scut_network {
        handle_scut_network_event(k.code, state, client, tx);
        return;
    }

    if in_missions {
        handle_missions_event(k.code, state, client, tx);
        return;
    }

    if in_rename_manny {
        handle_rename_manny_event(k.code, state, client, tx);
        return;
    }

    if in_deploy {
        handle_deploy_event(k.code, state, client, tx);
        return;
    }

    if in_inspect {
        handle_inspect_event(k.code, state, client, tx);
        return;
    }

    if in_recover {
        handle_recover_event(k.code, state, client, tx);
        return;
    }

    if in_detach {
        handle_detach_event(k.code, state, client, tx);
        return;
    }

    if in_alerts {
        handle_alerts_event(k.code, state, client, tx);
        return;
    }

    if in_rename_container {
        handle_rename_container_event(k.code, state, client, tx);
        return;
    }

    if in_container_rules {
        handle_container_rules_event(k.code, state, client, tx);
        return;
    }

    if in_containers {
        handle_containers_event(k.code, state, client, tx);
        return;
    }

    if in_storage_move {
        handle_storage_move_event(k.code, state, client, tx);
        return;
    }

    if in_drop_cargo {
        handle_drop_cargo_event(k.code, state, client, tx);
        return;
    }

    if in_drop_container {
        handle_drop_container_event(k.code, state, client, tx);
        return;
    }

    if !matches!(state.object_action, ObjectActionInput::Inactive) {
        handle_object_action_event(k.code, state, client, tx);
        return;
    }

    if !matches!(state.waypoints, WaypointsInput::Inactive) {
        handle_waypoints_event(k.code, state);
        return;
    }

    if in_travel {
        handle_travel_event(k.code, state, client, tx);
        return;
    }

    if in_repair {
        handle_repair_event(k.code, state, client, tx);
        return;
    }

    let in_mine = !matches!(state.mine, MineInput::Inactive);
    if in_mine {
        handle_mine_event(k.code, state, client, tx);
        return;
    }

    if in_remote_mine {
        handle_remote_mine_event(k.code, state, client, tx);
        return;
    }

    if in_direction_pick {
        match k.code {
            KeyCode::Esc => state.scan_mode = ScanMode::Current,
            KeyCode::Char(axis @ ('x' | 'y' | 'z')) => {
                if let Some(pos) = state.probe_sector_coords() {
                    let offsets = face_d2(axis as u8);
                    let n = offsets.len();
                    state.start_batch(n);
                    state.scan_mode = ScanMode::Current;
                    for (dx, dy, dz) in offsets {
                        fetch_sector(Some((pos.0 + dx, pos.1 + dy, pos.2 + dz)), client.clone(), tx.clone());
                    }
                }
            }
            _ => {}
        }
        return;
    }

    if in_scan_input {
        match k.code {
            KeyCode::Esc => state.scan_mode = ScanMode::Current,
            KeyCode::Backspace => state.scan_backspace(),
            KeyCode::Enter => {
                if let Some(coords) = state.parse_scan_coords() {
                    state.scan_loading = true;
                    state.scan_error = None;
                    fetch_sector(Some(coords), client.clone(), tx.clone());
                }
            }
            KeyCode::Char(c) => state.scan_type_char(c),
            _ => {}
        }
        return;
    }

    let in_object_mode =
        state.focused == Some(Panel::Scanner) && state.scanner_obj_selection.is_some();

    match k.code {
        KeyCode::Char('q') => state.set_quit(),
        KeyCode::Char('r') if ctrl && state.probe_terminal_alert().is_some() => {
            state.mind_snapshot = MindSnapshotInput::Confirm { error: None };
        }
        KeyCode::Char('A') => {
            // Open on the tab that actually has entries (warnings if alerts empty).
            let show_warnings = state.alerts.is_empty() && !state.damage_warnings.is_empty();
            state.alerts_input = AlertsInput::Browsing { selection: 0, show_warnings };
        }
        KeyCode::Char('b') => state.open_map(),
        KeyCode::Char('O') => {
            state.missions_input = MissionsInput::Browsing { selection: 0 };
        }
        KeyCode::Char('N') => {
            let nets = state.scut_coverage();
            match nets.len() {
                0 => state.error = Some("no SCUT coverage in this sector".into()),
                1 => {
                    let id = nets[0].0;
                    state.scut_network = ScutNetworkInput::Viewing { error: None };
                    state.scut_network_view = None;
                    fetch_scut_network(id, client.clone(), tx.clone());
                }
                _ => state.scut_network = ScutNetworkInput::Picking { networks: nets, selection: 0 },
            }
        }
        KeyCode::Char('?') => state.help_open = true,
        KeyCode::Char('w') => {
            let entries = state.collect_waypoints();
            if entries.is_empty() {
                state.error = Some("no known waypoints — scan sectors first".into());
            } else {
                state.waypoints = WaypointsInput::Browsing { entries, selection: 0 };
            }
        }
        KeyCode::Esc if in_object_mode => state.scanner_obj_selection = None,
        KeyCode::Esc => state.focused = None,
        KeyCode::Char('o') if state.focused == Some(Panel::Scanner) => {
            if state.scanner_obj_selection.is_some() {
                state.scanner_obj_selection = None;
            } else if !state.scanner_objects().is_empty() {
                state.scanner_obj_selection = Some(0);
            } else if state.viewing_probe_sector() {
                state.error = Some("no actionable objects in current sector".into());
            } else {
                state.error = Some("object actions only available in the probe's sector".into());
            }
        }
        KeyCode::Down | KeyCode::Char('j') if in_object_mode => state.scanner_obj_next(),
        KeyCode::Up | KeyCode::Char('k') if in_object_mode => state.scanner_obj_prev(),
        KeyCode::Enter if in_object_mode => {
            let entries = state.scanner_objects();
            if let Some(entry) = state.scanner_obj_selection.and_then(|i| entries.get(i)) {
                let actions = state.actions_for_object(entry);
                if actions.is_empty() {
                    state.error = Some(format!("no actions available for {}", entry.name));
                } else {
                    state.object_action = ObjectActionInput::PickAction {
                        object_id: entry.id.clone(),
                        object_name: entry.name.clone(),
                        actions,
                        selection: 0,
                    };
                }
            }
        }
        KeyCode::Char('t') => {
            state.travel = TravelInput::Typing(String::new());
        }
        KeyCode::Char('g') if state.focused == Some(Panel::Scanner) => {
            if let Some(sector) = state.current_sector() {
                let x = sector.relative_coordinates.x as i32;
                let y = sector.relative_coordinates.y as i32;
                let z = sector.relative_coordinates.z as i32;
                state.travel_go_sector(x, y, z);
            }
        }
        KeyCode::Char('r') if !state.loading => {
            state.clear_error();
            state.loading = true;
            fetch_all(client.clone(), tx.clone());
        }
        KeyCode::Tab => state.focus_next_panel(),
        KeyCode::BackTab => state.focus_prev_panel(),
        KeyCode::Char('p') => state.toggle_focus(Panel::Probe),
        KeyCode::Char('i') => state.toggle_focus(Panel::Inventory),
        KeyCode::Char('m') => state.toggle_focus(Panel::Mannies),
        KeyCode::Char('j') if state.focused == Some(Panel::Inventory) => {
            match state.jettison_for_selected() {
                Ok(input) => state.jettison = input,
                Err(msg) => state.error = Some(msg),
            }
        }
        KeyCode::Down if state.focused == Some(Panel::Inventory) => {
            state.inventory_next();
        }
        KeyCode::Up if state.focused == Some(Panel::Inventory) => {
            state.inventory_prev();
        }
        KeyCode::Enter if state.focused == Some(Panel::Inventory) => {
            if state.selected_inventory_row().is_some() {
                state.inventory_detail_open = true;
            }
        }
        KeyCode::Char('d') if state.focused == Some(Panel::Inventory) => {
            if state.inventory_waypoint_bookmark_id().is_none() {
                state.error = Some("no waypoint bookmark in inventory — craft one first".into());
            } else {
                let mannies = state.collect_idle_onboard_mannies();
                if mannies.is_empty() {
                    state.error = Some("no idle Manny on board".into());
                } else {
                    let candidates = state.collect_deploy_candidates();
                    if candidates.is_empty() {
                        state.error = Some("no targets in current sector — scan first".into());
                    } else if mannies.len() == 1 {
                        let (manny_id, _) = mannies.into_iter().next().unwrap();
                        if candidates.len() == 1 {
                            let (object_id, object_name) = candidates.into_iter().next().unwrap();
                            state.deploy = DeployInput::EnterName {
                                manny_id,
                                object_id,
                                object_name,
                                name_buf: String::new(),
                                error: None,
                            };
                        } else {
                            state.deploy = DeployInput::PickObject { manny_id, candidates, selection: 0 };
                        }
                    } else {
                        state.deploy = DeployInput::PickManny { mannies, selection: 0 };
                    }
                }
            }
        }
        KeyCode::Char('a') if state.focused == Some(Panel::Inventory) => {
            if !state.has_atomic_printer() {
                state.error = Some("no atomic printer in inventory".into());
            } else if state.atomic_printer_recipes().is_empty() {
                state.error = Some("recipes not loaded yet — press r to refresh".into());
            } else {
                state.atomic_printer_craft = AtomicPrinterCraftInput::PickRecipe {
                    selection: 0,
                    error: None,
                };
            }
        }
        KeyCode::Char('C') if state.focused == Some(Panel::Inventory) => {
            fetch_storage_containers(client.clone(), tx.clone());
            state.containers_input = ContainersInput::Browsing { selection: 0 };
        }
        KeyCode::Char('M') if state.focused == Some(Panel::Inventory) => {
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
        }
        KeyCode::Down | KeyCode::Char('j') if state.focused == Some(Panel::Mannies) => {
            state.manny_next();
        }
        KeyCode::Up | KeyCode::Char('k') if state.focused == Some(Panel::Mannies) => {
            state.manny_prev();
        }
        KeyCode::Down | KeyCode::Char('j') if state.focused == Some(Panel::Scanner) => {
            state.scan_hist_next();
        }
        KeyCode::Up | KeyCode::Char('k') if state.focused == Some(Panel::Scanner) => {
            state.scan_hist_prev();
        }
        KeyCode::Char('J') if state.focused == Some(Panel::Scanner) => {
            state.scan_detail_scroll = state.scan_detail_scroll.saturating_add(3);
        }
        KeyCode::Char('K') if state.focused == Some(Panel::Scanner) => {
            state.scan_detail_scroll = state.scan_detail_scroll.saturating_sub(3);
        }
        KeyCode::Enter if state.focused == Some(Panel::Mannies) => {
            if let Some(mannies) = &state.mannies {
                if let Some(manny) = mannies.get(state.mannies_selection) {
                    if manny.can_receive_orders {
                        state.repair = RepairInput::Typing {
                            manny_id: manny.id.clone(),
                            manny_name: manny.name.clone(),
                            buf: String::new(),
                            error: None,
                        };
                    }
                }
            }
        }
        KeyCode::Char('e') if state.focused == Some(Panel::Mannies) => {
            if let Some(mannies) = &state.mannies {
                if let Some(manny) = mannies.get(state.mannies_selection) {
                    if state.manny_remote_minable(manny) {
                        let coords = state.manny_sector_coords(manny).unwrap_or((0, 0, 0));
                        let manny_id = manny.id.clone();
                        let manny_name = manny.name.clone();
                        state.remote_mine = RemoteMineInput::Loading {
                            manny_id,
                            manny_name,
                            x: coords.0,
                            y: coords.1,
                            z: coords.2,
                        };
                        state.set_toast("fetching remote sector…");
                        fetch_sector(Some(coords), client.clone(), tx.clone());
                    } else if manny.can_receive_orders {
                        let manny_id = manny.id.clone();
                        let manny_name = manny.name.clone();
                        let candidates = state.collect_mineable_candidates();
                        match candidates.len() {
                            0 => state.error = Some("no mineable objects in current sector — scan first".into()),
                            1 => {
                                let (object_id, object_name) = candidates.into_iter().next().unwrap();
                                state.mine = MineInput::Configure {
                                    manny_id, manny_name, object_id, object_name,
                                    resources: [false, true, false, false],
                                    amount_buf: "0.30".into(),
                                    amount_mode: false,
                                    target_container: None,
                                    error: None,
                                };
                            }
                            _ => {
                                state.mine = MineInput::PickAsteroid {
                                    manny_id, manny_name, candidates, selection: 0,
                                };
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Char('c') if state.focused == Some(Panel::Mannies) => {
            if let Some(mannies) = &state.mannies {
                if let Some(manny) = mannies.get(state.mannies_selection) {
                    if manny.can_receive_orders {
                        if state.manny_craft_recipes().is_empty() {
                            state.error = Some("recipes not loaded yet — press r to refresh".into());
                        } else {
                            state.craft = CraftInput::PickRecipe {
                                manny_id: manny.id.clone(),
                                manny_name: manny.name.clone(),
                                selection: 0,
                                error: None,
                            };
                        }
                    }
                }
            }
        }
        KeyCode::Char('s') if state.focused == Some(Panel::Mannies) => {
            if let Some(mannies) = &state.mannies {
                if let Some(manny) = mannies.get(state.mannies_selection) {
                    if manny.can_receive_orders {
                        let manny_id = manny.id.clone();
                        let manny_name = manny.name.clone();
                        let candidates = state.collect_salvage_candidates();
                        match candidates.len() {
                            0 => state.error = Some("no salvageable objects in current sector — scan first".into()),
                            1 => {
                                let (object_id, object_name) = candidates.into_iter().next().unwrap();
                                state.salvage = SalvageInput::Confirm {
                                    manny_id, manny_name, object_id, object_name, error: None,
                                };
                            }
                            _ => {
                                state.salvage = SalvageInput::PickTarget {
                                    manny_id, manny_name, candidates, selection: 0,
                                };
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Char('R') if state.focused == Some(Panel::Mannies) => {
            if let Some(mannies) = &state.mannies {
                if let Some(manny) = mannies.get(state.mannies_selection) {
                    if !manny.can_receive_orders && manny.current_task.is_some() {
                        let remote = matches!(
                            manny.task_visibility,
                            Some(crate::api::types::MannyTaskVisibility::ScutNetwork)
                        );
                        state.recall = RecallInput::Confirm {
                            manny_id: manny.id.clone(),
                            manny_name: manny.name.clone(),
                            remote,
                            error: None,
                        };
                    }
                }
            }
        }
        KeyCode::Char('F') if state.focused == Some(Panel::Mannies) => {
            if state.deuterium_station_in_current_sector() {
                if let Some(mannies) = &state.mannies {
                    if let Some(manny) = mannies.get(state.mannies_selection) {
                        if manny.can_receive_orders {
                            state.refuel = RefuelInput::Confirm {
                                manny_id: manny.id.clone(),
                                manny_name: manny.name.clone(),
                                error: None,
                            };
                        } else {
                            state.error = Some("manny is not available for orders".into());
                        }
                    }
                }
            } else {
                state.error = Some("no deuterium refuel station in this sector".into());
            }
        }
        KeyCode::Char('n') if state.focused == Some(Panel::Mannies) => {
            if let Some(mannies) = &state.mannies {
                if let Some(manny) = mannies.get(state.mannies_selection) {
                    state.rename_manny = RenameMannyInput::Typing {
                        manny_id: manny.id.clone(),
                        manny_name: manny.name.clone(),
                        buf: manny.name.clone(),
                        error: None,
                    };
                }
            }
        }
        KeyCode::Char('X') if state.focused == Some(Panel::Mannies) => {
            if let Some(mannies) = &state.mannies {
                if let Some(manny) = mannies.get(state.mannies_selection) {
                    if manny.current_task == Some(MannyTask::WaitingForSpace) {
                        state.drop_cargo = DropCargoInput::Confirm {
                            manny_id: manny.id.clone(),
                            manny_name: manny.name.clone(),
                            error: None,
                        };
                    } else {
                        state.error = Some("manny is not waiting for storage space".into());
                    }
                }
            }
        }
        KeyCode::Char('P') if state.focused == Some(Panel::Mannies) => {
            let manny = state.mannies.as_ref()
                .and_then(|m| m.get(state.mannies_selection))
                .filter(|m| m.can_receive_orders)
                .map(|m| (m.id.clone(), m.name.clone()));
            if let Some((manny_id, manny_name)) = manny {
                let containers = state.collect_detachable_containers();
                if containers.is_empty() {
                    state.error = Some("no additional container to drop".into());
                } else if !state.has_atmospheric_drop_kit() {
                    state.error = Some("no atmospheric_drop_kit in inventory".into());
                } else if state.collect_planet_candidates().is_empty() {
                    state.error = Some("no planet in current sector — scan first".into());
                } else if containers.len() == 1 {
                    let (container_id, container_name) = containers.into_iter().next().unwrap();
                    let planets = state.collect_planet_candidates();
                    state.drop_container = DropStorageContainerInput::PickPlanet {
                        manny_id, manny_name, container_id, container_name,
                        planets, selection: 0, error: None,
                    };
                } else {
                    state.drop_container = DropStorageContainerInput::PickContainer {
                        manny_id, manny_name, containers, selection: 0,
                    };
                }
            }
        }
        KeyCode::Char('x') if state.focused == Some(Panel::Mannies) => {
            if let Some(mannies) = &state.mannies {
                if let Some(manny) = mannies.get(state.mannies_selection) {
                    if manny.can_receive_orders {
                        let candidates = state.collect_asteroid_candidates();
                        match candidates.len() {
                            0 => state.error = Some("no asteroids in current sector — scan first".into()),
                            1 => {
                                let (object_id, _) = candidates.into_iter().next().unwrap();
                                fetch_inspect(manny.id.clone(), object_id, client.clone(), tx.clone());
                            }
                            _ => {
                                state.inspect = InspectInput::PickAsteroid {
                                    manny_id: manny.id.clone(),
                                    manny_name: manny.name.clone(),
                                    candidates,
                                    selection: 0,
                                    error: None,
                                };
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Char('v') if state.focused == Some(Panel::Mannies) => {
            if let Some(mannies) = &state.mannies {
                if let Some(manny) = mannies.get(state.mannies_selection) {
                    if manny.can_receive_orders {
                        let candidates = state.collect_detached_containers();
                        match candidates.len() {
                            0 => state.error = Some("no detached containers in current sector — scan first".into()),
                            1 => {
                                let (object_id, _) = candidates.into_iter().next().unwrap();
                                fetch_recover(manny.id.clone(), object_id, client.clone(), tx.clone());
                            }
                            _ => {
                                state.recover = RecoverInput::PickContainer {
                                    manny_id: manny.id.clone(),
                                    manny_name: manny.name.clone(),
                                    candidates,
                                    selection: 0,
                                    error: None,
                                };
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Char('D') if state.focused == Some(Panel::Mannies) => {
            if let Some(mannies) = &state.mannies {
                if let Some(manny) = mannies.get(state.mannies_selection) {
                    if manny.can_receive_orders {
                        let containers = state.collect_detachable_containers();
                        if containers.is_empty() {
                            state.error = Some("no detachable containers in inventory".into());
                        } else if containers.len() == 1 {
                            let (container_id, container_name) = containers.into_iter().next().unwrap();
                            state.detach = DetachInput::PickMode {
                                manny_id: manny.id.clone(),
                                manny_name: manny.name.clone(),
                                container_id,
                                container_name,
                                selection: 0,
                                error: None,
                            };
                        } else {
                            state.detach = DetachInput::PickContainer {
                                manny_id: manny.id.clone(),
                                manny_name: manny.name.clone(),
                                containers,
                                selection: 0,
                            };
                        }
                    }
                }
            }
        }
        KeyCode::Enter if state.focused == Some(Panel::Scanner) && !state.scan_loading => {
            state.scan_loading = true;
            state.scan_error = None;
            fetch_sector(None, client.clone(), tx.clone());
        }
        KeyCode::Char('c') if state.focused == Some(Panel::Scanner) => {
            state.scan_mode = ScanMode::Input(String::new());
        }
        KeyCode::Char('n') if state.focused == Some(Panel::Scanner) && state.scan_batch.is_none() => {
            if let Some(pos) = state.probe_sector_coords() {
                let offsets = neighbors_d1();
                let n = offsets.len();
                state.start_batch(n);
                for (dx, dy, dz) in offsets {
                    fetch_sector(Some((pos.0 + dx, pos.1 + dy, pos.2 + dz)), client.clone(), tx.clone());
                }
            }
        }
        KeyCode::Char('d') if state.focused == Some(Panel::Scanner) && state.scan_batch.is_none() => {
            state.scan_mode = ScanMode::DirectionPick;
        }
        KeyCode::Char('f') if state.focused == Some(Panel::Scanner) => {
            state.cycle_scan_filter();
        }
        KeyCode::Char('s') => state.toggle_focus(Panel::Scanner),
        _ => {}
    }
}
