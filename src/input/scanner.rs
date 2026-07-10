use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::{
    fetch_inspect,
    fetch_recover,
    fetch_scut_network,
    fetch_turn_on_relay,
};
use crate::app::{
    ActiveWizard, ApiMessage, AppState, DeployInput, LogEvent, MineInput, ObjectAction, ObjectActionInput, SalvageInput,
    ScutNetworkInput, ScutRelayInput, WaypointsInput,
};
use super::geometry::list_nav;
/// Send the chosen object action, reusing the existing wizards/endpoints.
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
    }
}

pub(super) fn handle_scut_relay_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    let ActiveWizard::ScutRelay(ScutRelayInput::EnterNetworkName { .. }) = &state.active_wizard else { return };
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
                let ActiveWizard::ScutRelay(ScutRelayInput::EnterNetworkName { manny_id, relay_id, buf, .. }) =
                    &state.active_wizard
                else {
                    return;
                };
                let name = if buf.trim().is_empty() { None } else { Some(buf.trim().to_string()) };
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
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, selection, count) {
                        if let ActiveWizard::ScutNetwork(ScutNetworkInput::Picking { selection, .. }) = &mut state.active_wizard {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let id = {
                        let ActiveWizard::ScutNetwork(ScutNetworkInput::Picking { networks, .. }) = &state.active_wizard else { return };
                        networks[selection].0
                    };
                    state.active_wizard = ActiveWizard::ScutNetwork(ScutNetworkInput::Viewing { error: None });
                    state.scut_network_view = None;
                    fetch_scut_network(id, client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        ActiveWizard::ScutNetwork(ScutNetworkInput::Viewing { .. })
            if code == KeyCode::Esc => {
                state.close_wizard();
                state.scut_network_view = None;
            }
        _ => {}
    }
}

pub(super) fn handle_waypoints_event(code: KeyCode, state: &mut AppState) {
    let ActiveWizard::Waypoints(WaypointsInput::Browsing { entries, selection }) = &state.active_wizard else { return };
    let count = entries.len();
    let selection = *selection;
    match code {
        KeyCode::Esc | KeyCode::Char('w') => state.close_wizard(),
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(new_sel) = list_nav(code, selection, count) {
                if let ActiveWizard::Waypoints(WaypointsInput::Browsing { selection, .. }) = &mut state.active_wizard {
                    *selection = new_sel;
                }
            }
        }
        KeyCode::Enter => {
            let (x, y, z) = {
                let ActiveWizard::Waypoints(WaypointsInput::Browsing { entries, .. }) = &state.active_wizard else { return };
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
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let ActiveWizard::ObjectAction(ObjectActionInput::PickAction { selection, .. }) = &mut state.active_wizard {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (object_id, object_name, action) = {
                        let ActiveWizard::ObjectAction(ObjectActionInput::PickAction { object_id, object_name, actions, selection }) = &state.active_wizard else { return };
                        (object_id.clone(), object_name.clone(), actions[*selection])
                    };
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
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let ActiveWizard::ObjectAction(ObjectActionInput::PickManny { selection, .. }) = &mut state.active_wizard {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (object, action, manny) = {
                        let ActiveWizard::ObjectAction(ObjectActionInput::PickManny { object_id, object_name, action, mannies, selection }) = &state.active_wizard else { return };
                        ((object_id.clone(), object_name.clone()), *action, mannies[*selection].clone())
                    };
                    dispatch_object_action(state, client, tx, action, object, manny);
                }
                _ => {}
            }
        }
        _ => {}
    }
}
