use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::{fetch_ack_alert, fetch_ack_damage_warning};
use crate::app::{AlertsInput, ApiMessage, AppState};

use super::geometry::list_nav;

/// Number of entries shown under the currently selected tab.
fn tab_len(state: &AppState, show_warnings: bool) -> usize {
    if show_warnings {
        state.damage_warnings.len()
    } else {
        state.alerts.len()
    }
}

pub(super) fn handle_alerts_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    let AlertsInput::Browsing { selection, show_warnings } = state.alerts_input else {
        return;
    };
    let count = tab_len(state, show_warnings);
    match code {
        KeyCode::Esc | KeyCode::Char('A') | KeyCode::Char('q') => {
            state.alerts_input = AlertsInput::Inactive;
        }
        KeyCode::Tab | KeyCode::Left | KeyCode::Right | KeyCode::Char('h') | KeyCode::Char('l') => {
            // Switch tab and clamp the selection to the new tab's length.
            let next = !show_warnings;
            let new_count = tab_len(state, next);
            state.alerts_input = AlertsInput::Browsing {
                selection: selection.min(new_count.saturating_sub(1)),
                show_warnings: next,
            };
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(new_sel) = list_nav(code, selection, count) {
                state.alerts_input = AlertsInput::Browsing { selection: new_sel, show_warnings };
            }
        }
        KeyCode::Enter => {
            let id = if show_warnings {
                state.damage_warnings.get(selection).map(|w| w.id)
            } else {
                state.alerts.get(selection).map(|a| a.id)
            };
            if let Some(id) = id {
                if show_warnings {
                    fetch_ack_damage_warning(id, client.clone(), tx.clone());
                } else {
                    fetch_ack_alert(id, client.clone(), tx.clone());
                }
            }
        }
        _ => {}
    }
}
