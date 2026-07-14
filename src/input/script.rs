use crossterm::event::KeyCode;

use crate::app::{ActiveWizard, AppState, ScriptInput};

/// Vim-style modal editor for the action script (#198). `Insert` types a command
/// line; `Normal` navigates and manages the step list. All effects are on state
/// only — the executor (`advance_script`) and the event loop own the firing.
pub(super) fn handle_script_event(code: KeyCode, state: &mut AppState) {
    match &state.active_wizard {
        ActiveWizard::Script(ScriptInput::Insert { .. }) => handle_insert(code, state),
        ActiveWizard::Script(ScriptInput::Normal { .. }) => handle_normal(code, state),
        _ => {}
    }
}

fn handle_insert(code: KeyCode, state: &mut AppState) {
    match code {
        // Esc leaves insert for the management (Normal) mode, keeping the console open.
        KeyCode::Esc => {
            let selection = state.script.len().saturating_sub(1);
            state.active_wizard = ActiveWizard::Script(ScriptInput::Normal { selection });
        }
        KeyCode::Enter => {
            let line = match &state.active_wizard {
                ActiveWizard::Script(ScriptInput::Insert { buf, .. }) => buf.clone(),
                _ => return,
            };
            if line.trim().is_empty() {
                return;
            }
            match state.enqueue_script_line(&line) {
                Ok(()) => {
                    if let ActiveWizard::Script(ScriptInput::Insert { buf, error }) = &mut state.active_wizard {
                        buf.clear();
                        *error = None;
                    }
                }
                Err(e) => {
                    if let ActiveWizard::Script(ScriptInput::Insert { error, .. }) = &mut state.active_wizard {
                        *error = Some(e);
                    }
                }
            }
        }
        KeyCode::Backspace => {
            if let ActiveWizard::Script(ScriptInput::Insert { buf, .. }) = &mut state.active_wizard {
                buf.pop();
            }
        }
        KeyCode::Char(c) => {
            if let ActiveWizard::Script(ScriptInput::Insert { buf, error }) = &mut state.active_wizard {
                buf.push(c);
                *error = None;
            }
        }
        _ => {}
    }
}

fn handle_normal(code: KeyCode, state: &mut AppState) {
    let len = state.script.len();
    let selection = match &state.active_wizard {
        ActiveWizard::Script(ScriptInput::Normal { selection }) => *selection,
        _ => return,
    };
    match code {
        KeyCode::Esc | KeyCode::Char('q') => state.close_wizard(),
        // Enter insert to append a new line.
        KeyCode::Char('i') | KeyCode::Char('a') | KeyCode::Char('o') => {
            state.active_wizard = ActiveWizard::Script(ScriptInput::editing());
        }
        KeyCode::Down | KeyCode::Char('j') => {
            set_selection(state, selection.saturating_add(1).min(len.saturating_sub(1)))
        }
        KeyCode::Up | KeyCode::Char('k') => set_selection(state, selection.saturating_sub(1)),
        KeyCode::Char('x') => {
            state.script_remove(selection);
            set_selection(state, selection.min(state.script.len().saturating_sub(1)));
        }
        KeyCode::Char('c') => state.script_clear(),
        // Shift-R runs (a bare `r` is free for a future "retry").
        KeyCode::Char('R') => state.script_run(),
        KeyCode::Char('p') => state.script_toggle_pause(),
        _ => {}
    }
}

fn set_selection(state: &mut AppState, sel: usize) {
    if let ActiveWizard::Script(ScriptInput::Normal { selection }) = &mut state.active_wizard {
        *selection = sel;
    }
}
