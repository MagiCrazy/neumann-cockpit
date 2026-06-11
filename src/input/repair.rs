use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::fetch_repair;
use crate::app::{
    ApiMessage, AppState, RepairInput,
};
pub(super) fn handle_repair_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match code {
        KeyCode::Esc => state.repair = RepairInput::Inactive,
        KeyCode::Backspace => state.repair_backspace(),
        KeyCode::Char('m') | KeyCode::Char('M') => state.repair_fill_max(),
        KeyCode::Char(c) => state.repair_type_char(c),
        KeyCode::Enter => {
            let (manny_id, pct) = {
                let RepairInput::Typing { ref manny_id, ref buf, .. } = state.repair else { return };
                let Ok(pct) = buf.parse::<f64>() else { return };
                if pct <= 0.0 { return }
                (manny_id.clone(), pct)
            };
            fetch_repair(manny_id, pct, client.clone(), tx.clone());
        }
        _ => {}
    }
}
