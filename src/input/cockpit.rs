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
    fetch_ack_alert, fetch_ack_damage_warning, fetch_alerts, fetch_all, fetch_damage_warnings, fetch_inspect,
    fetch_messages, fetch_recover, fetch_scut_network, fetch_sector, fetch_sent_messages, fetch_set_default_probe,
    fetch_storage_container_detail,
};
use crate::api::types::{MannyTask, MannyTaskVisibility};
use crate::app::{
    ActiveWizard, ApiMessage, AppState, AssembleProbeInput, CommandLine, CommsCategory, DeployInput, DetachInput,
    DrillLevel, DropCargoInput, DropStorageContainerInput, FabricationInput, GotoVisitedInput, ImproveInput, InputMode,
    InspectInput, LogEvent, MenuAction, MessagesInput, MindSnapshotInput, MineInput, MissionsCategory, MissionsInput,
    ObjectActionInput, Pane, ProbeSwitchInput, RecallInput, RecoverInput, RefuelInput, RemoteMineInput,
    RenameContainerInput, RenameMannyInput, RenameProbeInput, RepairInput, SalvageInput, ScanMode, ScutNetworkInput,
    StorageMoveInput, TransferDeuteriumInput, TravelInput, WaypointsInput,
};

pub fn handle_cockpit_event(code: KeyCode, state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
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
        // Jump to the next idle Manny (focuses the Mannies pane).
        KeyCode::Char('i') => state.cycle_to_next_idle_manny(),
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
        Pane::Missions => match state.missions_category() {
            // Root: Enter enters the selected category, like `l`.
            None => missions_activate(state),
            Some(MissionsCategory::ShipsLog) => state.set_toast("ship's log — read only"),
            Some(MissionsCategory::Missions) => {
                let in_detail = matches!(
                    state.pane_nav[Pane::Missions.index()].drill.last(),
                    Some(DrillLevel::Mission(_))
                );
                if in_detail {
                    // Viewing a mission's steps — no extra action.
                } else if state.missions.is_empty() {
                    state.set_toast("no missions");
                } else {
                    let selection = state.pane_nav[Pane::Missions.index()].cursor;
                    state.active_wizard = ActiveWizard::Missions(MissionsInput::Browsing { selection });
                }
            }
        },
        Pane::Comms => comms_activate(state, client, tx),
        Pane::Sector => open_sector_object_actions(state),
    }
}

/// Comms activation, shared by `Enter` and `l`: at the root, pick a category
/// (Messages opens its overlay; Alerts/Warnings drill into an in-pane list);
/// inside Alerts/Warnings, acknowledge the selected entry.
fn comms_activate(state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    let cursor = state.pane_nav[Pane::Comms.index()].cursor;
    match state.comms_drill() {
        None => match CommsCategory::ALL.get(cursor) {
            Some(CommsCategory::Messages) => {
                state.active_wizard = ActiveWizard::Messages(MessagesInput::Browsing {
                    sent_tab: false,
                    selection: 0,
                });
                fetch_messages(client.clone(), tx.clone());
                fetch_sent_messages(client.clone(), tx.clone());
            }
            Some(CommsCategory::Alerts) => {
                state.comms_enter_category(CommsCategory::Alerts);
                fetch_alerts(client.clone(), tx.clone());
            }
            Some(CommsCategory::Warnings) => {
                state.comms_enter_category(CommsCategory::Warnings);
                fetch_damage_warnings(client.clone(), tx.clone());
            }
            None => {}
        },
        Some(CommsCategory::Alerts) => {
            if let Some(id) = state.alerts.get(cursor).map(|a| a.id) {
                fetch_ack_alert(id, client.clone(), tx.clone());
            }
        }
        Some(CommsCategory::Warnings) => {
            if let Some(id) = state.damage_warnings.get(cursor).map(|w| w.id) {
                fetch_ack_damage_warning(id, client.clone(), tx.clone());
            }
        }
        Some(CommsCategory::Messages) => {}
    }
}

/// Missions activation for `l` (and root `Enter`): at the root, enter the
/// selected category (missions list / ship's log); in the missions list, drill
/// into the selected mission's steps. The ship's log is read-only.
fn missions_activate(state: &mut AppState) {
    let cursor = state.pane_nav[Pane::Missions.index()].cursor;
    match state.missions_category() {
        None => {
            if let Some(&cat) = MissionsCategory::ALL.get(cursor) {
                state.missions_enter_category(cat);
            }
        }
        Some(MissionsCategory::Missions) => {
            let drilled = matches!(
                state.pane_nav[Pane::Missions.index()].drill.last(),
                Some(DrillLevel::Mission(_))
            );
            if !drilled {
                if let Some(id) = state.missions.get(cursor).map(|m| m.id.clone()) {
                    state.missions_drill_into(id);
                }
            }
        }
        Some(MissionsCategory::ShipsLog) => {}
    }
}

/// Drill into the selected element. For Storage, this fetches the container's
/// contents so they can be rendered inline (no legacy content modal).
fn drill_in(state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    // Comms drives its own drill (categories → in-pane alert/warning lists).
    if state.active_pane == Pane::Comms {
        comms_activate(state, client, tx);
        return;
    }
    // Missions likewise (categories → missions list → steps, or ship's log).
    if state.active_pane == Pane::Missions {
        missions_activate(state);
        return;
    }
    state.pane_drill_in();
    if state.active_pane == Pane::Storage {
        if let Some(DrillLevel::Container(id)) = state.pane_nav[Pane::Storage.index()].drill.last().cloned() {
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
    state.active_wizard = ActiveWizard::ObjectAction(ObjectActionInput::PickAction {
        object_id: entry.id.clone(),
        object_name: entry.name.clone(),
        actions,
        selection: 0,
    });
}

fn handle_menu_key(code: KeyCode, state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
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
fn fire_menu_action(action: MenuAction, state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    // Inventory-pane actions operate on the selected inventory row, not a Manny.
    match action {
        MenuAction::Jettison => {
            match state.jettison_for_selected() {
                Ok(input) => state.active_wizard = ActiveWizard::Jettison(input),
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
                state.active_wizard = ActiveWizard::Fabrication(FabricationInput::PickRecipe {
                    prefilled_manny: None,
                    selection: 0,
                    error: None,
                });
            }
            return;
        }
        MenuAction::MoveStock => {
            let mannies = state.collect_idle_onboard_mannies();
            match mannies.len() {
                0 => state.error = Some("no idle Manny on board".into()),
                1 => {
                    let (id, name) = mannies.into_iter().next().unwrap();
                    state.active_wizard = ActiveWizard::StorageMove(StorageMoveInput::PickKind {
                        actor_manny_id: id,
                        actor_manny_name: name,
                        selection: 0,
                    });
                }
                _ => {
                    state.active_wizard =
                        ActiveWizard::StorageMove(StorageMoveInput::PickManny { mannies, selection: 0 })
                }
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
                state.active_wizard = ActiveWizard::Deploy(DeployInput::PickManny { mannies, selection: 0 });
            }
            return;
        }
        MenuAction::ScutInspect => {
            let nets = state.scut_coverage();
            match nets.len() {
                0 => state.error = Some("no SCUT network covers this sector".into()),
                1 => {
                    state.active_wizard = ActiveWizard::ScutNetwork(ScutNetworkInput::Viewing { error: None });
                    state.scut_network_view = None;
                    fetch_scut_network(nets[0].0, client.clone(), tx.clone());
                }
                _ => {
                    state.active_wizard = ActiveWizard::ScutNetwork(ScutNetworkInput::Picking {
                        networks: nets,
                        selection: 0,
                    })
                }
            }
            return;
        }
        MenuAction::Improve => {
            if state.has_orderable_improvement() {
                state.active_wizard = ActiveWizard::Improve(ImproveInput::PickImprovement {
                    selection: 0,
                    error: None,
                });
            } else {
                state.error = Some("no probe improvement available".into());
            }
            return;
        }
        MenuAction::MindSnapshot => {
            if state.probe_terminal_alert().is_some() {
                state.active_wizard = ActiveWizard::MindSnapshot(MindSnapshotInput::Confirm { error: None });
            }
            return;
        }
        MenuAction::RenameContainer => {
            if let Some(id) = state.storage_selected_container_id() {
                if let Some((container_id, label)) =
                    state.storage_container(&id).map(|c| (c.id.clone(), c.label.clone()))
                {
                    let buf = state.next_name_suggestion();
                    state.active_wizard = ActiveWizard::RenameContainer(RenameContainerInput::Typing {
                        container_id,
                        current_label: label,
                        buf,
                        error: None,
                    });
                }
            }
            return;
        }
        MenuAction::EditContainerRules => {
            if let Some(id) = state.storage_selected_container_id() {
                if let Some(editor) = state.rules_editor_for(&id) {
                    state.active_wizard = ActiveWizard::ContainerRules(editor);
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
            state.active_wizard = ActiveWizard::Travel(TravelInput::Typing(String::new()));
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
                state.active_wizard = ActiveWizard::Waypoints(WaypointsInput::Browsing { entries, selection: 0 });
            }
            return;
        }
        MenuAction::SwitchProbe => {
            if state.fleet.len() > 1 {
                // Open the picker on the currently active probe.
                let active = state.active_probe_id.or(state.default_probe_id);
                let selection = state.fleet.iter().position(|p| Some(p.id) == active).unwrap_or(0);
                state.probe_switch = ProbeSwitchInput::Picking { selection };
            }
            return;
        }
        MenuAction::SetDefaultProbe => {
            if let Some(active) = state.active_probe_summary() {
                if !active.is_default && active.is_reachable {
                    let (id, name) = (active.id, active.name.clone());
                    fetch_set_default_probe(id, name.clone(), client.clone(), tx.clone());
                    state.log_event(LogEvent::set_default_probe(&name, Some(id)));
                }
            }
            return;
        }
        MenuAction::RenameProbe => {
            if let Some((id, name)) = state.active_probe_identity() {
                let buf = state.next_name_suggestion();
                state.active_wizard = ActiveWizard::RenameProbe(RenameProbeInput::Typing {
                    probe_id: id,
                    current_name: name,
                    buf,
                    error: None,
                });
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
            state.active_wizard = ActiveWizard::Repair(RepairInput::Typing {
                manny_id: id,
                manny_name: name,
                buf: String::new(),
                error: None,
            });
        }
        MenuAction::Fabricate if can => {
            if state.fabrication_recipes().is_empty() {
                state.error = Some("recipes not loaded yet — F5 to refresh".into());
            } else {
                state.active_wizard = ActiveWizard::Fabrication(FabricationInput::PickRecipe {
                    prefilled_manny: Some((id, name)),
                    selection: 0,
                    error: None,
                });
            }
        }
        MenuAction::AssembleProbe if can => {
            let containers = state.collect_empty_containers();
            if containers.len() < 2 {
                state.error = Some("need two empty additional containers".into());
            } else {
                state.active_wizard = ActiveWizard::AssembleProbe(AssembleProbeInput::PickContainers {
                    manny_id: id,
                    manny_name: name,
                    containers,
                    selected: Vec::new(),
                    cursor: 0,
                    error: None,
                });
            }
        }
        MenuAction::Mine => {
            if remote_minable {
                state.active_wizard = ActiveWizard::RemoteMine(RemoteMineInput::Loading {
                    manny_id: id,
                    manny_name: name,
                    x: coords.0,
                    y: coords.1,
                    z: coords.2,
                });
                state.set_toast("fetching remote sector…");
                fetch_sector(Some(coords), client.clone(), tx.clone());
            } else if can {
                let candidates = state.collect_mineable_candidates();
                match candidates.len() {
                    0 => state.error = Some("no mineable objects in current sector — scan first".into()),
                    1 => {
                        let (object_id, object_name) = candidates.into_iter().next().unwrap();
                        state.active_wizard = ActiveWizard::Mine(MineInput::Configure {
                            manny_id: id,
                            manny_name: name,
                            object_id,
                            object_name,
                            resources: [false, true, false, false],
                            amount_buf: "0.30".into(),
                            amount_mode: false,
                            target_container: None,
                            error: None,
                        });
                    }
                    _ => {
                        state.active_wizard = ActiveWizard::Mine(MineInput::PickAsteroid {
                            manny_id: id,
                            manny_name: name,
                            candidates,
                            selection: 0,
                        })
                    }
                }
            }
        }
        MenuAction::Salvage if can => {
            let candidates = state.collect_salvage_candidates();
            match candidates.len() {
                0 => state.error = Some("no salvageable objects in current sector — scan first".into()),
                1 => {
                    let (object_id, object_name) = candidates.into_iter().next().unwrap();
                    state.active_wizard = ActiveWizard::Salvage(SalvageInput::Confirm {
                        manny_id: id,
                        manny_name: name,
                        object_id,
                        object_name,
                        error: None,
                    });
                }
                _ => {
                    state.active_wizard = ActiveWizard::Salvage(SalvageInput::PickTarget {
                        manny_id: id,
                        manny_name: name,
                        candidates,
                        selection: 0,
                    })
                }
            }
        }
        MenuAction::Inspect if can => {
            let candidates = state.collect_inspectable_candidates();
            match candidates.len() {
                0 => state.error = Some("no inspectable objects in current sector — scan first".into()),
                1 => {
                    let (object_id, object_name) = candidates.into_iter().next().unwrap();
                    fetch_inspect(id, object_id, client.clone(), tx.clone());
                    state.log_event(LogEvent::inspect(&object_name, state.active_probe_id));
                }
                _ => {
                    state.active_wizard = ActiveWizard::Inspect(InspectInput::PickTarget {
                        manny_id: id,
                        manny_name: name,
                        candidates,
                        selection: 0,
                        error: None,
                    })
                }
            }
        }
        MenuAction::Recover if can => {
            let candidates = state.collect_detached_containers();
            match candidates.len() {
                0 => state.error = Some("no detached containers in current sector — scan first".into()),
                1 => {
                    let (object_id, container_name) = candidates.into_iter().next().unwrap();
                    fetch_recover(id, object_id, client.clone(), tx.clone());
                    state.log_event(LogEvent::recover(&container_name, state.active_probe_id));
                }
                _ => {
                    state.active_wizard = ActiveWizard::Recover(RecoverInput::PickContainer {
                        manny_id: id,
                        manny_name: name,
                        candidates,
                        selection: 0,
                        error: None,
                    })
                }
            }
        }
        MenuAction::Detach if can => {
            let containers = state.collect_detachable_containers();
            match containers.len() {
                0 => state.error = Some("no detachable containers in inventory".into()),
                1 => {
                    let (container_id, container_name) = containers.into_iter().next().unwrap();
                    state.active_wizard = ActiveWizard::Detach(DetachInput::PickMode {
                        manny_id: id,
                        manny_name: name,
                        container_id,
                        container_name,
                        selection: 0,
                        error: None,
                    });
                }
                _ => {
                    state.active_wizard = ActiveWizard::Detach(DetachInput::PickContainer {
                        manny_id: id,
                        manny_name: name,
                        containers,
                        selection: 0,
                    })
                }
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
                    state.active_wizard = ActiveWizard::DropContainer(DropStorageContainerInput::PickPlanet {
                        manny_id: id,
                        manny_name: name,
                        container_id,
                        container_name,
                        planets,
                        selection: 0,
                        error: None,
                    });
                } else {
                    state.active_wizard = ActiveWizard::DropContainer(DropStorageContainerInput::PickContainer {
                        manny_id: id,
                        manny_name: name,
                        containers,
                        selection: 0,
                    });
                }
            }
        }
        MenuAction::Refuel if can => {
            if state.deuterium_station_in_current_sector() {
                state.active_wizard = ActiveWizard::Refuel(RefuelInput::Confirm {
                    manny_id: id,
                    manny_name: name,
                    error: None,
                });
            } else {
                state.error = Some("no deuterium refuel station in this sector".into());
            }
        }
        MenuAction::TransferDeuterium if can => {
            let targets = state.transfer_deuterium_targets();
            if targets.is_empty() {
                state.error = Some("no other probe in the fleet".into());
            } else {
                state.active_wizard = ActiveWizard::TransferDeuterium(TransferDeuteriumInput::PickTarget {
                    manny_id: id,
                    manny_name: name,
                    targets,
                    selection: 0,
                });
            }
        }
        MenuAction::DropCargo if waiting_space => {
            state.active_wizard = ActiveWizard::DropCargo(DropCargoInput::Confirm {
                manny_id: id,
                manny_name: name,
                error: None,
            });
        }
        MenuAction::Recall if !can && has_task => {
            state.active_wizard = ActiveWizard::Recall(RecallInput::Confirm {
                manny_id: id,
                manny_name: name,
                remote: remote_recall,
                error: None,
            });
        }
        MenuAction::Rename => {
            let buf = state.next_name_suggestion();
            state.active_wizard = ActiveWizard::RenameManny(RenameMannyInput::Typing {
                manny_id: id,
                manny_name: name,
                buf,
                error: None,
            });
        }
        // Guard mismatch (state changed since the menu was built): no-op.
        _ => {}
    }
}
