//! Discarding a Comms entry for good (API v112, issue #366).
//!
//! The Comms lists deliberately keep acknowledged entries — that is what makes
//! them a log rather than an inbox — but a list that only ever grows stops
//! being readable. Deleting is the second act the pilot needs, and it is a
//! different one: acknowledging says "seen", this says "gone", permanently and
//! server-side. So it asks first, every time.

use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::fetch_delete_comms_entry;
use crate::app::{ActiveWizard, ApiMessage, AppState, DiscardCommsInput};

pub(super) fn handle_discard_comms_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    let ActiveWizard::DiscardComms(input) = &state.active_wizard else {
        return;
    };
    // `y` as well as Enter: a destructive prompt is the one place where the
    // habitual key is worth honouring, and Esc still backs out of both.
    if !matches!(code, KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y')) {
        state.close_wizard();
        return;
    }
    // The probe id is needed either way — both paths are mirror-only.
    let Some(probe_id) = state.probe_id() else {
        state.set_error("waiting for a probe sync".to_string());
        state.close_wizard();
        return;
    };
    let (warnings, ids) = match input {
        DiscardCommsInput::Inactive => return,
        DiscardCommsInput::One { warnings, id, .. } => (*warnings, vec![*id]),
        DiscardCommsInput::Acknowledged { warnings, ids, .. } => (*warnings, ids.clone()),
    };
    for id in &ids {
        fetch_delete_comms_entry(warnings, probe_id, *id, client.clone(), tx.clone());
    }
    let what = if warnings { "warning" } else { "alert" };
    let plural = if ids.len() > 1 { "s" } else { "" };
    state.set_toast(format!("discarding {} {what}{plural}", ids.len()));
    state.close_wizard();
}
