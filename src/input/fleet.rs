use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::{fetch_rename_probe, fetch_transfer_deuterium};
use crate::app::{
    ApiMessage, AppState, LogEvent, ProbeSwitchInput, RenameProbeInput, TransferDeuteriumInput,
};

use super::geometry::list_nav;

/// Fleet picker (API v81 multi-probe): navigate the roster, `Enter` switches the
/// piloted probe, `Esc` cancels. Switching is client-side only — the event loop
/// reconciles the `ApiClient` and refetches. An unreachable probe is refused
/// with a toast: piloting it would return only limited telemetry.
pub(super) fn handle_probe_switch_event(code: KeyCode, state: &mut AppState) {
    let ProbeSwitchInput::Picking { selection } = state.probe_switch else { return };
    let count = state.fleet.len();
    match code {
        KeyCode::Esc => state.probe_switch = ProbeSwitchInput::Inactive,
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(ns) = list_nav(code, selection, count) {
                state.probe_switch = ProbeSwitchInput::Picking { selection: ns };
            }
        }
        KeyCode::Enter => {
            if let Some(p) = state.fleet.get(selection) {
                let (id, name, reachable) = (p.id, p.name.clone(), p.is_reachable);
                state.probe_switch = ProbeSwitchInput::Inactive;
                if !reachable {
                    state.set_toast(format!("{name} is out of SCUT range — cannot pilot"));
                } else if state.set_active_probe(id) {
                    state.set_toast(format!("piloting {name}"));
                }
            }
        }
        _ => {}
    }
}

/// Deuterium-transfer wizard (API v86): pick a destination probe from the
/// roster, then enter the percentage to ferry. `Enter` on `PickTarget` advances
/// to `EnterAmount`; `Enter` there parses the amount, fires the five-minute
/// Manny task, and logs it. `Esc` cancels the whole wizard. The same-sector
/// constraint is server-validated — a mismatch returns as an error toast in the
/// amount step.
pub(super) fn handle_transfer_deuterium_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    // Step 1 — destination picker.
    if let TransferDeuteriumInput::PickTarget { targets, selection, .. } = &state.transfer_deuterium {
        let (count, sel) = (targets.len(), *selection);
        match code {
            KeyCode::Esc => state.transfer_deuterium = TransferDeuteriumInput::Inactive,
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                if let (Some(ns), TransferDeuteriumInput::PickTarget { selection, .. }) =
                    (list_nav(code, sel, count), &mut state.transfer_deuterium)
                {
                    *selection = ns;
                }
            }
            KeyCode::Enter => {
                if let TransferDeuteriumInput::PickTarget {
                    manny_id, manny_name, targets, selection,
                } = &state.transfer_deuterium
                {
                    if let Some((target_id, target_name)) = targets.get(*selection).cloned() {
                        state.transfer_deuterium = TransferDeuteriumInput::EnterAmount {
                            manny_id: manny_id.clone(),
                            manny_name: manny_name.clone(),
                            target_id,
                            target_name,
                            buf: String::new(),
                            error: None,
                        };
                    }
                }
            }
            _ => {}
        }
        return;
    }

    // Step 2 — amount entry. Each arm takes its own borrow so reassigning the
    // wizard state never overlaps the mutable buffer borrow.
    if !matches!(state.transfer_deuterium, TransferDeuteriumInput::EnterAmount { .. }) {
        return;
    }
    match code {
        KeyCode::Esc => state.transfer_deuterium = TransferDeuteriumInput::Inactive,
        KeyCode::Backspace => {
            if let TransferDeuteriumInput::EnterAmount { buf, error, .. } =
                &mut state.transfer_deuterium
            {
                buf.pop();
                *error = None;
            }
        }
        KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
            if let TransferDeuteriumInput::EnterAmount { buf, error, .. } =
                &mut state.transfer_deuterium
            {
                if c == '.' && buf.contains('.') {
                    return;
                }
                buf.push(c);
                *error = None;
            }
        }
        KeyCode::Enter => {
            let TransferDeuteriumInput::EnterAmount {
                manny_id, target_id, target_name, buf, ..
            } = &state.transfer_deuterium else { return };
            match buf.trim().parse::<f64>() {
                Ok(amount) if amount > 0.0 => {
                    let (mid, tid, tname) = (manny_id.clone(), *target_id, target_name.clone());
                    fetch_transfer_deuterium(mid, tid, amount, client.clone(), tx.clone());
                    state.log_event(LogEvent::transfer_deuterium(
                        &tname,
                        amount,
                        state.active_probe_id,
                    ));
                }
                _ => state.set_transfer_deuterium_error(
                    "enter a positive deuterium percentage".to_string(),
                ),
            }
        }
        _ => {}
    }
}

/// Rename-probe wizard (API v81): text entry, `Enter` commits the new name via
/// `PATCH /api/probe/{id}`, `Esc` cancels. Empty input is ignored.
pub(super) fn handle_rename_probe_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match code {
        KeyCode::Esc => state.rename_probe = RenameProbeInput::Inactive,
        KeyCode::Tab => {
            let s = state.next_name_suggestion();
            if let RenameProbeInput::Typing { buf, .. } = &mut state.rename_probe {
                *buf = s;
            }
        }
        KeyCode::Backspace => {
            if let RenameProbeInput::Typing { buf, .. } = &mut state.rename_probe {
                buf.pop();
            }
        }
        KeyCode::Char(c) => {
            if let RenameProbeInput::Typing { buf, .. } = &mut state.rename_probe {
                buf.push(c);
            }
        }
        KeyCode::Enter => {
            let order = match &state.rename_probe {
                RenameProbeInput::Typing { probe_id, buf, .. } if !buf.trim().is_empty() => {
                    Some((*probe_id, buf.trim().to_string()))
                }
                _ => None,
            };
            if let Some((id, name)) = order {
                let new_name = name.clone();
                fetch_rename_probe(id, name, client.clone(), tx.clone());
                state.log_event(LogEvent::rename_probe(&new_name, Some(id)));
            }
        }
        _ => {}
    }
}
