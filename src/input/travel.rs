use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::fetch_move;
use crate::app::{
    ActiveWizard, ApiMessage, AppState, LogEvent, TravelInput,
};
pub(super) fn handle_travel_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.active_wizard {
        ActiveWizard::Travel(TravelInput::Typing(_)) => match code {
            KeyCode::Esc => state.close_wizard(),
            KeyCode::Backspace => state.travel_backspace(),
            KeyCode::Enter => state.travel_submit(),
            KeyCode::Char(c) => state.travel_type_char(c),
            _ => {}
        },
        ActiveWizard::Travel(TravelInput::Confirming { x, y, z, error, .. }) => {
            let (x, y, z, has_error) = (*x, *y, *z, error.is_some());
            match code {
                KeyCode::Esc => state.close_wizard(),
                KeyCode::Enter if !has_error => {
                    fetch_move(x, y, z, client.clone(), tx.clone());
                    state.log_event(LogEvent::travel(x, y, z, state.active_probe_id));
                }
                _ => {}
            }
        }
        _ => {}
    }
}
