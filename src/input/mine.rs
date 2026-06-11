use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::fetch_mine;
use crate::app::{
    ApiMessage, AppState, MineInput, RESOURCE_TYPES,
};
use super::geometry::list_nav;
pub(super) fn handle_mine_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.mine {
        MineInput::PickAsteroid { selection, candidates, .. } => {
            let sel = *selection;
            let count = candidates.len();
            match code {
                KeyCode::Esc => state.mine = MineInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let MineInput::PickAsteroid { ref mut selection, .. } = state.mine {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, manny_name, object_id, object_name) = {
                        let MineInput::PickAsteroid { ref manny_id, ref manny_name, ref candidates, selection } = state.mine else { return };
                        let (id, name) = candidates[selection].clone();
                        (manny_id.clone(), manny_name.clone(), id, name)
                    };
                    state.mine = MineInput::Configure {
                        manny_id, manny_name, object_id, object_name,
                        resources: [false, true, false, false],
                        amount_buf: "0.30".into(),
                        amount_mode: false,
                        error: None,
                    };
                }
                _ => {}
            }
        }
        MineInput::Configure { amount_mode, .. } => {
            let am = *amount_mode;
            match code {
                KeyCode::Esc => state.mine = MineInput::Inactive,
                KeyCode::Tab => {
                    if let MineInput::Configure { ref mut amount_mode, ref mut error, .. } = state.mine {
                        *amount_mode = !am;
                        *error = None;
                    }
                }
                KeyCode::Char(c @ '1'..='4') if !am => {
                    let idx = (c as u8 - b'1') as usize;
                    if let MineInput::Configure { ref mut resources, ref mut error, .. } = state.mine {
                        resources[idx] = !resources[idx];
                        *error = None;
                    }
                }
                KeyCode::Char('m') | KeyCode::Char('M') if am => {
                    let max = state.mine_max_amount();
                    if let MineInput::Configure { ref mut amount_buf, ref mut error, .. } = state.mine {
                        *amount_buf = format!("{:.4}", max);
                        *error = None;
                    }
                }
                KeyCode::Char(c) if am && (c.is_ascii_digit() || c == '.') => {
                    if let MineInput::Configure { ref mut amount_buf, ref mut error, .. } = state.mine {
                        if !(c == '.' && amount_buf.contains('.')) {
                            amount_buf.push(c);
                            *error = None;
                        }
                    }
                }
                KeyCode::Backspace if am => {
                    if let MineInput::Configure { ref mut amount_buf, .. } = state.mine {
                        amount_buf.pop();
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, object_id, selected_resources, amount) = {
                        let MineInput::Configure { ref manny_id, ref object_id, resources, ref amount_buf, .. } = state.mine else { return };
                        let selected: Vec<String> = RESOURCE_TYPES.iter().enumerate()
                            .filter(|(i, _)| resources[*i])
                            .map(|(_, &t)| t.to_string())
                            .collect();
                        if selected.is_empty() { return }
                        let Ok(amount) = amount_buf.parse::<f64>() else { return };
                        if amount <= 0.0 { return }
                        (manny_id.clone(), object_id.clone(), selected, amount)
                    };
                    fetch_mine(manny_id, object_id, selected_resources, amount, client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        MineInput::Inactive => {}
    }
}
