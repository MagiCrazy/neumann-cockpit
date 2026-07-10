use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::tasks::fetch_abandon_mission;
use crate::api::types::MissionStatus;
use crate::app::{ActiveWizard, ApiMessage, AppState, LogEvent, MissionsInput};
use crate::api::client::ApiClient;

use super::geometry::list_nav;

pub(super) fn handle_missions_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match state.active_wizard {
        ActiveWizard::Missions(MissionsInput::Browsing { selection }) => {
            let count = state.missions.len();
            match code {
                KeyCode::Esc | KeyCode::Char('O') | KeyCode::Char('q') => {
                    state.close_wizard();
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, selection, count) {
                        state.active_wizard = ActiveWizard::Missions(MissionsInput::Browsing { selection: new_sel });
                    }
                }
                KeyCode::Char('a') => {
                    if let Some(m) = state.missions.get(selection) {
                        if m.status == MissionStatus::Active {
                            state.active_wizard = ActiveWizard::Missions(MissionsInput::ConfirmAbandon {
                                mission_id: m.id.clone(),
                                mission_title: m.title.clone(),
                                selection,
                                error: None,
                            });
                        } else {
                            state.error = Some("only active missions can be abandoned".into());
                        }
                    }
                }
                _ => {}
            }
        }
        ActiveWizard::Missions(MissionsInput::ConfirmAbandon { selection, .. }) => match code {
            KeyCode::Esc | KeyCode::Char('n') => {
                state.active_wizard = ActiveWizard::Missions(MissionsInput::Browsing { selection });
            }
            KeyCode::Enter | KeyCode::Char('y') => {
                let (mission_id, mission_title) = {
                    let ActiveWizard::Missions(MissionsInput::ConfirmAbandon { ref mission_id, ref mission_title, .. }) = state.active_wizard
                    else {
                        return;
                    };
                    (mission_id.clone(), mission_title.clone())
                };
                fetch_abandon_mission(mission_id, client.clone(), tx.clone());
                state.log_event(LogEvent::mission_abandon(&mission_title, state.active_probe_id));
            }
            _ => {}
        },
        _ => {}
    }
}
