use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::fetch_jettison;
use crate::app::{
    ApiMessage, AppState, JettisonInput,
};
pub(super) fn handle_jettison_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.jettison {
        JettisonInput::ConfirmManny { .. } => {
            match code {
                KeyCode::Esc => state.jettison = JettisonInput::Inactive,
                KeyCode::Enter => {
                    let item_id = {
                        let JettisonInput::ConfirmManny { ref item_id, .. } = state.jettison else { return };
                        item_id.clone()
                    };
                    fetch_jettison(item_id, None, client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        JettisonInput::ConfirmRelay { .. } => {
            match code {
                KeyCode::Esc => state.jettison = JettisonInput::Inactive,
                KeyCode::Enter => {
                    let item_id = {
                        let JettisonInput::ConfirmRelay { ref item_id, .. } = state.jettison else { return };
                        item_id.clone()
                    };
                    fetch_jettison(item_id, None, client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        JettisonInput::EnterAmount { .. } => {
            match code {
                KeyCode::Esc => state.jettison = JettisonInput::Inactive,
                KeyCode::Backspace => state.jettison_backspace(),
                KeyCode::Char('m') | KeyCode::Char('M') => state.jettison_fill_max(),
                KeyCode::Char(c) => state.jettison_type_char(c),
                KeyCode::Enter => {
                    let (item_id, amount) = {
                        let JettisonInput::EnterAmount { ref item_id, ref buf, .. } = state.jettison else { return };
                        let amount = if buf.is_empty() {
                            None
                        } else {
                            let Ok(v) = buf.parse::<f64>() else { return };
                            if v <= 0.0 { return }
                            Some(v)
                        };
                        (item_id.clone(), amount)
                    };
                    fetch_jettison(item_id, amount, client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        JettisonInput::Inactive => {}
    }
}
