use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use super::geometry::{is_list_nav_key, list_nav};
use crate::api::client::ApiClient;
use crate::api::tasks::{
    fetch_ignite_missile, fetch_inspect, fetch_install_beacon, fetch_motorize_asteroid, fetch_recover,
    fetch_refuel_asteroid, fetch_scut_network, fetch_trajectory, fetch_turn_on_relay,
};
use crate::app::{
    ActiveWizard, AimAsteroidInput, ApiMessage, AppState, DeployInput, FireMissileInput, LogEvent, MineInput,
    ObjectAction, ObjectActionInput, SalvageInput, ScutCorridorInput, ScutNetworkInput, ScutRelayInput, WaypointsInput,
    LIST_PAGE,
};
/// Send the chosen object action, reusing the existing wizards/endpoints.
/// The fire confirmation (issue #361). One key, and it is `Enter` — the
/// reinforced wording carries the weight for a player target, not a different
/// gesture, because a pilot who reached this screen chose the target and the
/// Manny deliberately two steps ago.
pub(super) fn handle_fire_missile_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    let ActiveWizard::FireMissile(FireMissileInput::Confirm { .. }) = &state.active_wizard else {
        return;
    };
    match code {
        KeyCode::Esc => state.close_wizard(),
        KeyCode::Enter => {
            let ActiveWizard::FireMissile(FireMissileInput::Confirm {
                manny_id,
                object_id,
                object_name,
                target,
                ..
            }) = &state.active_wizard
            else {
                return;
            };
            let (manny_id, object_id, object_name, target) =
                (manny_id.clone(), object_id.clone(), object_name.clone(), *target);
            // Mirror-only endpoint, like the single-Manny GET and the task
            // batch: without a probe sync there is no path to send to.
            let Some(probe_id) = state.probe_id() else {
                state.set_wizard_error("no probe sync yet".into());
                return;
            };
            state.close_wizard();
            fetch_ignite_missile(probe_id, manny_id, object_id, client.clone(), tx.clone());
            state.log_event(LogEvent::fire_missile(
                &object_name,
                target.label(),
                state.active_probe_id,
            ));
        }
        _ => {}
    }
}

pub(super) fn dispatch_object_action(
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
    action: ObjectAction,
    object: (String, String),
    manny: (String, String),
) {
    let (object_id, object_name) = object;
    let (manny_id, manny_name) = manny;
    // Close the object-action picker; each arm either fires immediately or
    // opens the next wizard (which replaces it in `active_wizard`).
    state.close_wizard();
    state.scanner_obj_selection = None;
    match action {
        ObjectAction::Mine => {
            state.active_wizard = ActiveWizard::Mine(MineInput::Configure {
                manny_id,
                manny_name,
                object_id,
                object_name,
                resources: [false, true, false, false],
                amount_buf: "0.30".into(),
                amount_mode: false,
                target_container: None,
                error: None,
            });
        }
        ObjectAction::Inspect => {
            fetch_inspect(manny_id, object_id, client.clone(), tx.clone());
            state.log_event(LogEvent::inspect(&object_name, state.active_probe_id));
        }
        // The target was chosen in the Sector pane and the Manny resolved by
        // the ordinary object-action flow; what is left is the decision (#361).
        ObjectAction::FireMissile => {
            let Some(target) = state.missile_target_kind(&object_id) else {
                state.error = Some("that object is not a valid missile target".into());
                return;
            };
            state.active_wizard = ActiveWizard::FireMissile(FireMissileInput::Confirm {
                manny_id,
                manny_name,
                object_id,
                object_name,
                target,
                error: None,
            });
        }
        ObjectAction::Salvage => {
            state.active_wizard = ActiveWizard::Salvage(SalvageInput::Confirm {
                manny_id,
                manny_name,
                object_id,
                object_name,
                error: None,
            });
        }
        ObjectAction::Recover => {
            fetch_recover(manny_id, object_id, client.clone(), tx.clone());
            state.log_event(LogEvent::recover(&object_name, state.active_probe_id));
        }
        ObjectAction::DeployWaypoint => {
            state.active_wizard = ActiveWizard::Deploy(DeployInput::EnterName {
                manny_id,
                object_id,
                object_name,
                name_buf: String::new(),
                error: None,
            });
        }
        ObjectAction::TurnOnRelay => match object_id.parse::<i64>() {
            Ok(relay_id) => {
                state.active_wizard = ActiveWizard::ScutRelay(ScutRelayInput::EnterNetworkName {
                    manny_id,
                    manny_name,
                    relay_id,
                    relay_name: object_name,
                    buf: String::new(),
                    error: None,
                });
            }
            Err(_) => {
                state.error = Some("relay has an unexpected id format".into());
            }
        },
        // ── Motorized asteroids (issue #308) ─────────────────────────────
        //
        // Motorizing and refuelling are ordinary Manny orders. Both need the
        // piloted probe's id: the endpoints are mirror-only.
        ObjectAction::MotorizeAsteroid | ObjectAction::RefuelAsteroid => {
            let Some(probe_id) = state.probe_id() else {
                state.error = Some("waiting for a probe sync".into());
                return;
            };
            if action == ObjectAction::MotorizeAsteroid {
                fetch_motorize_asteroid(probe_id, manny_id, object_id, client.clone(), tx.clone());
                state.log_event(LogEvent::motorize_asteroid(&object_name, state.active_probe_id));
            } else {
                fetch_refuel_asteroid(probe_id, manny_id, object_id, client.clone(), tx.clone());
                state.log_event(LogEvent::refuel_asteroid(&object_name, state.active_probe_id));
            }
        }
        // Aiming opens its own wizard: two modes, and each needs a target the
        // pilot has to choose.
        ObjectAction::AimAsteroid => {
            state.active_wizard = ActiveWizard::AimAsteroid(AimAsteroidInput::PickMode {
                asteroid_id: object_id,
                asteroid_name: object_name,
                selection: 0,
            });
        }
        ObjectAction::TrackTrajectory => match state.probe_id() {
            Some(probe_id) => match state.asteroid_trajectory_id(&object_id) {
                Some(trajectory_id) => fetch_trajectory(probe_id, trajectory_id, client.clone(), tx.clone()),
                None => state.error = Some("that asteroid is not under way".into()),
            },
            None => state.error = Some("waiting for a probe sync".into()),
        },
        ObjectAction::InstallTransitBeacon => match object_id.parse::<i64>() {
            Ok(relay_id) => {
                fetch_install_beacon(manny_id, relay_id, client.clone(), tx.clone());
                state.log_event(LogEvent::install_transit_beacon(&object_name, state.active_probe_id));
            }
            Err(_) => {
                state.error = Some("relay has an unexpected id format".into());
            }
        },
    }
}

pub(super) fn handle_scut_relay_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    let ActiveWizard::ScutRelay(ScutRelayInput::EnterNetworkName { .. }) = &state.active_wizard else {
        return;
    };
    match code {
        KeyCode::Esc => state.close_wizard(),
        KeyCode::Backspace => {
            if let ActiveWizard::ScutRelay(ScutRelayInput::EnterNetworkName { buf, .. }) = &mut state.active_wizard {
                buf.pop();
            }
        }
        KeyCode::Char(c) => {
            if let ActiveWizard::ScutRelay(ScutRelayInput::EnterNetworkName { buf, .. }) = &mut state.active_wizard {
                buf.push(c);
            }
        }
        KeyCode::Enter => {
            let (manny_id, relay_id, name) = {
                let ActiveWizard::ScutRelay(ScutRelayInput::EnterNetworkName {
                    manny_id,
                    relay_id,
                    buf,
                    ..
                }) = &state.active_wizard
                else {
                    return;
                };
                let name = if buf.trim().is_empty() {
                    None
                } else {
                    Some(buf.trim().to_string())
                };
                (manny_id.clone(), *relay_id, name)
            };
            let network = name.clone();
            fetch_turn_on_relay(manny_id, relay_id, name, client.clone(), tx.clone());
            state.log_event(LogEvent::relay_on(network.as_deref(), state.active_probe_id));
        }
        _ => {}
    }
}

pub(super) fn handle_scut_network_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.active_wizard {
        ActiveWizard::ScutNetwork(ScutNetworkInput::Picking { networks, selection }) => {
            let count = networks.len();
            let selection = *selection;
            match code {
                KeyCode::Esc => state.close_wizard(),
                _ if is_list_nav_key(code) => {
                    if let Some(new_sel) = list_nav(code, selection, count) {
                        if let ActiveWizard::ScutNetwork(ScutNetworkInput::Picking { selection, .. }) =
                            &mut state.active_wizard
                        {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let id = {
                        let ActiveWizard::ScutNetwork(ScutNetworkInput::Picking { networks, .. }) =
                            &state.active_wizard
                        else {
                            return;
                        };
                        networks[selection].0
                    };
                    state.active_wizard =
                        ActiveWizard::ScutNetwork(ScutNetworkInput::Viewing { error: None, offset: 0 });
                    state.scut_network_view = None;
                    fetch_scut_network(id, client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        ActiveWizard::ScutNetwork(ScutNetworkInput::Viewing { .. }) if code == KeyCode::Esc => {
            state.close_wizard();
            state.scut_network_view = None;
        }
        // The body is a viewport, not a cursor, so it answers the shared
        // navigation keys (#325) without wrapping: a list that snapped back to
        // the top after its last relay would read as a glitch (issue #379).
        ActiveWizard::ScutNetwork(ScutNetworkInput::Viewing { error, offset }) => {
            let (width, height) = crate::ui::overlays::scut_network::viewing_viewport();
            let total = crate::ui::overlays::scut_network::viewing_height(state, error.as_ref(), width);
            let max = total.saturating_sub(height as usize);
            let cur = *offset;
            let next = match code {
                KeyCode::Down | KeyCode::Char('j') => (cur + 1).min(max),
                KeyCode::Up | KeyCode::Char('k') => cur.saturating_sub(1),
                KeyCode::PageDown => (cur + LIST_PAGE).min(max),
                KeyCode::PageUp => cur.saturating_sub(LIST_PAGE),
                KeyCode::Home => 0,
                KeyCode::End => max,
                _ => return,
            };
            if let ActiveWizard::ScutNetwork(ScutNetworkInput::Viewing { offset, .. }) = &mut state.active_wizard {
                *offset = next;
            }
        }
        _ => {}
    }
}

/// Safe SCUT corridors (API v96, issue #257). `Enter` on a destination hands
/// over to the ordinary travel confirm rather than jumping outright: waiving the
/// destruction risk does not waive the fuel bill, and the pilot still deserves
/// the ETA before committing.
pub(super) fn handle_scut_corridor_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.active_wizard {
        ActiveWizard::ScutCorridor(ScutCorridorInput::PickNetwork { networks, selection }) => {
            let (count, selection) = (networks.len(), *selection);
            match code {
                KeyCode::Esc => state.close_wizard(),
                _ if is_list_nav_key(code) => {
                    if let Some(new_sel) = list_nav(code, selection, count) {
                        if let ActiveWizard::ScutCorridor(ScutCorridorInput::PickNetwork { selection, .. }) =
                            &mut state.active_wizard
                        {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let ActiveWizard::ScutCorridor(ScutCorridorInput::PickNetwork { networks, .. }) =
                        &state.active_wizard
                    else {
                        return;
                    };
                    let (id, name) = networks[selection].clone();
                    state.scut_network_view = None;
                    state.active_wizard = ActiveWizard::ScutCorridor(ScutCorridorInput::Loading { network_name: name });
                    fetch_scut_network(id, client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        ActiveWizard::ScutCorridor(ScutCorridorInput::Loading { .. }) if code == KeyCode::Esc => {
            state.close_wizard();
            state.scut_network_view = None;
        }
        ActiveWizard::ScutCorridor(ScutCorridorInput::Picking { selection, .. }) => {
            let destinations = state.corridor_destinations();
            let (count, selection) = (destinations.len(), *selection);
            match code {
                KeyCode::Esc => {
                    state.close_wizard();
                    state.scut_network_view = None;
                }
                _ if is_list_nav_key(code) => {
                    if let Some(new_sel) = list_nav(code, selection, count) {
                        if let ActiveWizard::ScutCorridor(ScutCorridorInput::Picking { selection, .. }) =
                            &mut state.active_wizard
                        {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let Some(d) = destinations.get(selection) else {
                        return;
                    };
                    let (x, y, z) = d.coords;
                    // Keep the network detail: the travel confirm reads it back
                    // to mark the destination as a corridor.
                    state.active_wizard = ActiveWizard::None;
                    state.travel_go_sector(x, y, z);
                }
                _ => {}
            }
        }
        _ => {}
    }
}

pub(super) fn handle_waypoints_event(code: KeyCode, state: &mut AppState) {
    let ActiveWizard::Waypoints(WaypointsInput::Browsing { entries, selection }) = &state.active_wizard else {
        return;
    };
    let count = entries.len();
    let selection = *selection;
    match code {
        KeyCode::Esc | KeyCode::Char('w') => state.close_wizard(),
        _ if is_list_nav_key(code) => {
            if let Some(new_sel) = list_nav(code, selection, count) {
                if let ActiveWizard::Waypoints(WaypointsInput::Browsing { selection, .. }) = &mut state.active_wizard {
                    *selection = new_sel;
                }
            }
        }
        KeyCode::Enter => {
            let (x, y, z) = {
                let ActiveWizard::Waypoints(WaypointsInput::Browsing { entries, .. }) = &state.active_wizard else {
                    return;
                };
                let e = &entries[selection];
                (e.x, e.y, e.z)
            };
            state.close_wizard();
            state.travel_go_sector(x, y, z);
        }
        _ => {}
    }
}

pub(super) fn handle_object_action_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.active_wizard {
        ActiveWizard::ObjectAction(ObjectActionInput::PickAction { selection, actions, .. }) => {
            let sel = *selection;
            let count = actions.len();
            match code {
                KeyCode::Esc => state.close_wizard(),
                _ if is_list_nav_key(code) => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let ActiveWizard::ObjectAction(ObjectActionInput::PickAction { selection, .. }) =
                            &mut state.active_wizard
                        {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (object_id, object_name, action) = {
                        let ActiveWizard::ObjectAction(ObjectActionInput::PickAction {
                            object_id,
                            object_name,
                            actions,
                            selection,
                        }) = &state.active_wizard
                        else {
                            return;
                        };
                        (object_id.clone(), object_name.clone(), actions[*selection])
                    };
                    // The two asteroid actions that involve no crew skip the
                    // Manny step entirely (issue #308).
                    if !action.needs_manny() {
                        dispatch_object_action(
                            state,
                            client,
                            tx,
                            action,
                            (object_id, object_name),
                            (String::new(), String::new()),
                        );
                        return;
                    }
                    let mannies = state.collect_idle_onboard_mannies();
                    match mannies.len() {
                        0 => {
                            state.close_wizard();
                            state.error = Some("no idle Manny on board".into());
                        }
                        1 => {
                            let manny = mannies.into_iter().next().unwrap();
                            dispatch_object_action(state, client, tx, action, (object_id, object_name), manny);
                        }
                        _ => {
                            state.active_wizard = ActiveWizard::ObjectAction(ObjectActionInput::PickManny {
                                object_id,
                                object_name,
                                action,
                                mannies,
                                selection: 0,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        ActiveWizard::ObjectAction(ObjectActionInput::PickManny { selection, mannies, .. }) => {
            let sel = *selection;
            let count = mannies.len();
            match code {
                KeyCode::Esc => state.close_wizard(),
                _ if is_list_nav_key(code) => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let ActiveWizard::ObjectAction(ObjectActionInput::PickManny { selection, .. }) =
                            &mut state.active_wizard
                        {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (object, action, manny) = {
                        let ActiveWizard::ObjectAction(ObjectActionInput::PickManny {
                            object_id,
                            object_name,
                            action,
                            mannies,
                            selection,
                        }) = &state.active_wizard
                        else {
                            return;
                        };
                        (
                            (object_id.clone(), object_name.clone()),
                            *action,
                            mannies[*selection].clone(),
                        )
                    };
                    dispatch_object_action(state, client, tx, action, object, manny);
                }
                _ => {}
            }
        }
        _ => {}
    }
}
