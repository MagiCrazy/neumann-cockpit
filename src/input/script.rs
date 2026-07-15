use crossterm::event::KeyCode;

use crate::app::{ActiveWizard, AppState, CompletionState, ScriptInput};

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
                    if let ActiveWizard::Script(ScriptInput::Insert { buf, error, completion }) =
                        &mut state.active_wizard
                    {
                        buf.clear();
                        *error = None;
                        *completion = None;
                    }
                }
                Err(e) => {
                    if let ActiveWizard::Script(ScriptInput::Insert { error, completion, .. }) =
                        &mut state.active_wizard
                    {
                        *error = Some(e);
                        *completion = None;
                    }
                }
            }
        }
        KeyCode::Tab => cycle_script_completion(state),
        KeyCode::Backspace => {
            if let ActiveWizard::Script(ScriptInput::Insert { buf, completion, .. }) = &mut state.active_wizard {
                buf.pop();
                *completion = None;
            }
        }
        KeyCode::Char(c) => {
            if let ActiveWizard::Script(ScriptInput::Insert { buf, error, completion }) = &mut state.active_wizard {
                buf.push(c);
                *error = None;
                *completion = None;
            }
        }
        _ => {}
    }
}

/// `Tab`: complete the token at the caret (always the end of `buf`). First press
/// computes the candidate set and writes the first match; subsequent presses
/// cycle. Mirrors the command-line `cycle_completion`.
fn cycle_script_completion(state: &mut AppState) {
    let (buf, existing) = match &state.active_wizard {
        ActiveWizard::Script(ScriptInput::Insert { buf, completion, .. }) => (buf.clone(), completion.clone()),
        _ => return,
    };

    let (candidates, index, token_start) = match existing {
        Some(c) if c.candidates.len() > 1 => {
            let index = (c.index + 1) % c.candidates.len();
            (c.candidates, index, c.token_start)
        }
        Some(c) => (c.candidates, c.index, c.token_start),
        None => match state.script_completions(&buf) {
            Some((token_start, candidates)) => (candidates, 0, token_start),
            None => return,
        },
    };

    let candidate = &candidates[index];
    let unique = candidates.len() == 1;
    // A unique verb (only whitespace before it) gets a trailing space so the
    // caret lands where its arguments go.
    let is_verb = buf[..token_start].trim().is_empty();
    let mut new_buf = String::with_capacity(token_start + candidate.len() + 1);
    new_buf.push_str(&buf[..token_start]);
    new_buf.push_str(candidate);
    if unique && is_verb {
        new_buf.push(' ');
    }

    if let ActiveWizard::Script(ScriptInput::Insert { buf, completion, .. }) = &mut state.active_wizard {
        *buf = new_buf;
        *completion = (!unique).then_some(CompletionState {
            candidates,
            index,
            token_start,
        });
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
