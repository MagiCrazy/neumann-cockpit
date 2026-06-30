use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::tasks::fetch_abandon_mission;
use crate::api::types::MissionStatus;
use crate::app::{ApiMessage, AppState, MissionsInput};
use crate::api::client::ApiClient;

use super::geometry::list_nav;

pub(super) fn handle_missions_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match state.missions_input {
        MissionsInput::Browsing { selection } => {
            let count = state.missions.len();
            match code {
                KeyCode::Esc | KeyCode::Char('O') | KeyCode::Char('q') => {
                    state.missions_input = MissionsInput::Inactive;
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, selection, count) {
                        state.missions_input = MissionsInput::Browsing { selection: new_sel };
                    }
                }
                KeyCode::Char('a') => {
                    if let Some(m) = state.missions.get(selection) {
                        if m.status == MissionStatus::Active {
                            state.missions_input = MissionsInput::ConfirmAbandon {
                                mission_id: m.id.clone(),
                                mission_title: m.title.clone(),
                                selection,
                                error: None,
                            };
                        } else {
                            state.error = Some("only active missions can be abandoned".into());
                        }
                    }
                }
                _ => {}
            }
        }
        MissionsInput::ConfirmAbandon { selection, .. } => match code {
            KeyCode::Esc | KeyCode::Char('n') => {
                state.missions_input = MissionsInput::Browsing { selection };
            }
            KeyCode::Enter | KeyCode::Char('y') => {
                let mission_id = {
                    let MissionsInput::ConfirmAbandon { ref mission_id, .. } = state.missions_input
                    else {
                        return;
                    };
                    mission_id.clone()
                };
                fetch_abandon_mission(mission_id, client.clone(), tx.clone());
            }
            _ => {}
        },
        MissionsInput::Inactive => {}
    }
}
