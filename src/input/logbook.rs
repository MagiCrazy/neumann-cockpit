//! Writing the probe's server logbook (API v90, issue #254).
//!
//! The pages are prose the pilot writes, so the editor is two plain text
//! buffers — title, then body — rather than a pick-list. Both are bounded
//! client-side against the server's own limits, so an over-long page is
//! refused here instead of spending a 400 to find out.

use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::{fetch_create_logbook_page, fetch_delete_logbook_page, fetch_update_logbook_page};
use crate::api::types::{LOGBOOK_CONTENT_MAX, LOGBOOK_TITLE_MAX};
use crate::app::{ActiveWizard, ApiMessage, AppState, LogbookInput};

pub(super) fn handle_logbook_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.active_wizard {
        ActiveWizard::Logbook(LogbookInput::Title { .. }) => handle_title(code, state),
        ActiveWizard::Logbook(LogbookInput::Content { .. }) => handle_content(code, state),
        ActiveWizard::Logbook(LogbookInput::ConfirmDelete { .. }) => handle_confirm_delete(code, state, client, tx),
        _ => {}
    }
}

fn handle_title(code: KeyCode, state: &mut AppState) {
    let ActiveWizard::Logbook(LogbookInput::Title {
        page_id,
        title,
        content,
        error,
    }) = &mut state.active_wizard
    else {
        return;
    };
    match code {
        KeyCode::Esc => state.close_wizard(),
        KeyCode::Backspace => {
            title.pop();
            *error = None;
        }
        KeyCode::Enter => {
            if title.trim().is_empty() {
                *error = Some("a page needs a title".into());
                return;
            }
            let (page_id, title, content) = (*page_id, title.clone(), content.clone());
            state.active_wizard = ActiveWizard::Logbook(LogbookInput::Content {
                page_id,
                title,
                content,
                error: None,
            });
        }
        KeyCode::Char(c) => {
            if title.chars().count() < LOGBOOK_TITLE_MAX {
                title.push(c);
                *error = None;
            } else {
                *error = Some(format!("titles stop at {LOGBOOK_TITLE_MAX} characters"));
            }
        }
        _ => {}
    }
}

fn handle_content(code: KeyCode, state: &mut AppState) {
    // `Enter` inserts a newline — this is prose, and a page of it needs
    // paragraphs — so committing is `Ctrl-S`, and `Esc` steps back to the
    // title rather than throwing the draft away.
    let ActiveWizard::Logbook(LogbookInput::Content {
        page_id,
        title,
        content,
        error,
    }) = &mut state.active_wizard
    else {
        return;
    };
    match code {
        KeyCode::Esc => {
            let (page_id, title, content) = (*page_id, title.clone(), content.clone());
            state.active_wizard = ActiveWizard::Logbook(LogbookInput::Title {
                page_id,
                title,
                content,
                error: None,
            });
        }
        KeyCode::Backspace => {
            content.pop();
            *error = None;
        }
        KeyCode::Enter => {
            if content.chars().count() < LOGBOOK_CONTENT_MAX {
                content.push('\n');
            }
        }
        KeyCode::Char(c) => {
            if content.chars().count() < LOGBOOK_CONTENT_MAX {
                content.push(c);
                *error = None;
            } else {
                *error = Some(format!("pages stop at {LOGBOOK_CONTENT_MAX} characters"));
            }
        }
        _ => {}
    }
}

/// Fire the create or update behind the open editor. Called from the input
/// layer's Ctrl-S arm, which is resolved before the wizard registry because a
/// modifier does not reach these handlers.
pub(super) fn submit_logbook_page(state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    let ActiveWizard::Logbook(LogbookInput::Content {
        page_id,
        title,
        content,
        ..
    }) = &state.active_wizard
    else {
        return;
    };
    let (page_id, title, content) = (*page_id, title.clone(), content.clone());
    if content.trim().is_empty() {
        if let ActiveWizard::Logbook(LogbookInput::Content { error, .. }) = &mut state.active_wizard {
            *error = Some("an empty page is not a page".into());
        }
        return;
    }
    let Some(probe_id) = state.probe_id() else {
        state.set_toast("waiting for a probe sync");
        return;
    };
    match page_id {
        Some(id) => fetch_update_logbook_page(probe_id, id, title, content, client.clone(), tx.clone()),
        None => fetch_create_logbook_page(probe_id, title, content, client.clone(), tx.clone()),
    }
    state.close_wizard();
}

fn handle_confirm_delete(code: KeyCode, state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    let ActiveWizard::Logbook(LogbookInput::ConfirmDelete { page_id, .. }) = &state.active_wizard else {
        return;
    };
    let page_id = *page_id;
    match code {
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(probe_id) = state.probe_id() {
                fetch_delete_logbook_page(probe_id, page_id, client.clone(), tx.clone());
            }
            state.close_wizard();
        }
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => state.close_wizard(),
        _ => {}
    }
}
