use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::fetch_jettison;
use crate::app::{ActiveWizard, ApiMessage, AppState, JettisonInput};
pub(super) fn handle_jettison_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.active_wizard {
        ActiveWizard::Jettison(JettisonInput::ConfirmManny { .. }) => match code {
            KeyCode::Esc => state.close_wizard(),
            KeyCode::Enter => {
                let item_id = {
                    let ActiveWizard::Jettison(JettisonInput::ConfirmManny { ref item_id, .. }) = state.active_wizard
                    else {
                        return;
                    };
                    item_id.clone()
                };
                fetch_jettison(item_id, None, client.clone(), tx.clone());
            }
            _ => {}
        },
        ActiveWizard::Jettison(JettisonInput::ConfirmRelay { .. }) => match code {
            KeyCode::Esc => state.close_wizard(),
            KeyCode::Enter => {
                let item_id = {
                    let ActiveWizard::Jettison(JettisonInput::ConfirmRelay { ref item_id, .. }) = state.active_wizard
                    else {
                        return;
                    };
                    item_id.clone()
                };
                fetch_jettison(item_id, None, client.clone(), tx.clone());
            }
            _ => {}
        },
        ActiveWizard::Jettison(JettisonInput::EnterAmount { .. }) => {
            match code {
                KeyCode::Esc => state.close_wizard(),
                KeyCode::Backspace => state.jettison_backspace(),
                KeyCode::Char('m') | KeyCode::Char('M') => state.jettison_fill_max(),
                KeyCode::Char(c) => state.jettison_type_char(c),
                // Amount entered → move to an explicit confirmation before the
                // irreversible drop, rather than firing straight away.
                KeyCode::Enter => {
                    let next = {
                        let ActiveWizard::Jettison(JettisonInput::EnterAmount {
                            ref item_id,
                            ref item_name,
                            ref buf,
                            ..
                        }) = state.active_wizard
                        else {
                            return;
                        };
                        let amount = if buf.is_empty() {
                            None
                        } else {
                            let Ok(v) = buf.parse::<f64>() else { return };
                            if v <= 0.0 {
                                return;
                            }
                            Some(v)
                        };
                        JettisonInput::Confirm {
                            item_id: item_id.clone(),
                            item_name: item_name.clone(),
                            amount,
                            error: None,
                        }
                    };
                    state.active_wizard = ActiveWizard::Jettison(next);
                }
                _ => {}
            }
        }
        ActiveWizard::Jettison(JettisonInput::Confirm { .. }) => match code {
            KeyCode::Esc => state.close_wizard(),
            KeyCode::Enter => {
                let (item_id, amount) = {
                    let ActiveWizard::Jettison(JettisonInput::Confirm {
                        ref item_id, amount, ..
                    }) = state.active_wizard
                    else {
                        return;
                    };
                    (item_id.clone(), amount)
                };
                fetch_jettison(item_id, amount, client.clone(), tx.clone());
            }
            _ => {}
        },
        _ => {}
    }
}
