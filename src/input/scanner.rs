use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::{
    fetch_inspect,
    fetch_recover,
};
use crate::app::{
    ApiMessage, AppState, DeployInput, MineInput, ObjectAction, ObjectActionInput, SalvageInput,
    WaypointsInput,
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
    state.object_action = ObjectActionInput::Inactive;
    state.scanner_obj_selection = None;
    match action {
        ObjectAction::Mine => {
            state.mine = MineInput::Configure {
                manny_id,
                manny_name,
                object_id,
                object_name,
                resources: [false, true, false, false],
                amount_buf: "0.30".into(),
                amount_mode: false,
                error: None,
            };
        }
        ObjectAction::Inspect => {
            fetch_inspect(manny_id, object_id, client.clone(), tx.clone());
        }
        ObjectAction::Salvage => {
            state.salvage = SalvageInput::Confirm {
                manny_id,
                manny_name,
                object_id,
                object_name,
                error: None,
            };
        }
        ObjectAction::Recover => {
            fetch_recover(manny_id, object_id, client.clone(), tx.clone());
        }
        ObjectAction::DeployWaypoint => {
            state.deploy = DeployInput::EnterName {
                manny_id,
                object_id,
                object_name,
                name_buf: String::new(),
                error: None,
            };
        }
    }
}

pub(super) fn handle_waypoints_event(code: KeyCode, state: &mut AppState) {
    let WaypointsInput::Browsing { ref entries, selection } = state.waypoints else { return };
    let count = entries.len();
    match code {
        KeyCode::Esc | KeyCode::Char('w') => state.waypoints = WaypointsInput::Inactive,
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(new_sel) = list_nav(code, selection, count) {
                if let WaypointsInput::Browsing { ref mut selection, .. } = state.waypoints {
                    *selection = new_sel;
                }
            }
        }
        KeyCode::Enter => {
            let (x, y, z) = {
                let WaypointsInput::Browsing { ref entries, selection } = state.waypoints else { return };
                let e = &entries[selection];
                (e.x, e.y, e.z)
            };
            state.waypoints = WaypointsInput::Inactive;
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
    match &state.object_action {
        ObjectActionInput::PickAction { selection, actions, .. } => {
            let sel = *selection;
            let count = actions.len();
            match code {
                KeyCode::Esc => state.object_action = ObjectActionInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let ObjectActionInput::PickAction { ref mut selection, .. } = state.object_action {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (object_id, object_name, action) = {
                        let ObjectActionInput::PickAction { ref object_id, ref object_name, ref actions, selection } = state.object_action else { return };
                        (object_id.clone(), object_name.clone(), actions[selection])
                    };
                    let mannies = state.collect_idle_onboard_mannies();
                    match mannies.len() {
                        0 => {
                            state.object_action = ObjectActionInput::Inactive;
                            state.error = Some("no idle Manny on board".into());
                        }
                        1 => {
                            let manny = mannies.into_iter().next().unwrap();
                            dispatch_object_action(state, client, tx, action, (object_id, object_name), manny);
                        }
                        _ => {
                            state.object_action = ObjectActionInput::PickManny {
                                object_id,
                                object_name,
                                action,
                                mannies,
                                selection: 0,
                            };
                        }
                    }
                }
                _ => {}
            }
        }
        ObjectActionInput::PickManny { selection, mannies, .. } => {
            let sel = *selection;
            let count = mannies.len();
            match code {
                KeyCode::Esc => state.object_action = ObjectActionInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let ObjectActionInput::PickManny { ref mut selection, .. } = state.object_action {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (object, action, manny) = {
                        let ObjectActionInput::PickManny { ref object_id, ref object_name, action, ref mannies, selection } = state.object_action else { return };
                        ((object_id.clone(), object_name.clone()), action, mannies[selection].clone())
                    };
                    dispatch_object_action(state, client, tx, action, object, manny);
                }
                _ => {}
            }
        }
        ObjectActionInput::Inactive => {}
    }
}
