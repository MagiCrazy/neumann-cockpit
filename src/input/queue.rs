use crossterm::event::KeyCode;

use super::geometry::list_nav;
use crate::app::{ActiveWizard, AppState, QueueInput};

/// Drive the production-queue overlay (`:queue`): move the cursor, run/pause the
/// executor, and edit steps (repeat count, remove, clear). Read-only over the
/// crafting state — it never fires a craft itself (the executor does that).
pub(super) fn handle_queue_event(code: KeyCode, state: &mut AppState) {
    let ActiveWizard::Queue(QueueInput::Browsing { selection }) = state.active_wizard else {
        return;
    };
    let count = state.craft_queue.len();
    match code {
        KeyCode::Esc | KeyCode::Char('q') => state.close_wizard(),
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(ns) = list_nav(code, selection, count) {
                state.active_wizard = ActiveWizard::Queue(QueueInput::Browsing { selection: ns });
            }
        }
        KeyCode::Char('r') => state.queue_toggle_run(),
        KeyCode::Char('+') | KeyCode::Char('=') => state.queue_bump(selection, 1),
        KeyCode::Char('-') | KeyCode::Char('_') => state.queue_bump(selection, -1),
        KeyCode::Char('x') => {
            state.queue_remove(selection);
            let clamped = selection.min(state.craft_queue.len().saturating_sub(1));
            state.active_wizard = ActiveWizard::Queue(QueueInput::Browsing { selection: clamped });
        }
        KeyCode::Char('c') => {
            state.queue_clear();
            state.active_wizard = ActiveWizard::Queue(QueueInput::Browsing { selection: 0 });
        }
        _ => {}
    }
}
