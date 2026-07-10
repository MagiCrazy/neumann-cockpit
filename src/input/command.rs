use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::{fetch_all, fetch_atomic_printer_craft, fetch_craft, fetch_mine};
use crate::app::{
    command_usage, ApiMessage, AppState, CommandFire, CompletionState, InputMode, LogEvent,
};

/// Route a key while the `:` command line is open. Typed characters are literal
/// input; `Enter` runs the command, `Tab` completes/cycles the token under the
/// caret, `↑`/`↓` browse history, `Esc` cancels.
pub(super) fn handle_command_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match code {
        KeyCode::Esc => state.mode = InputMode::Normal,
        KeyCode::Enter => {
            let InputMode::Command(cmd) = &state.mode else { return };
            let line = cmd.input.clone();
            state.mode = InputMode::Normal;
            if state.run_command(&line) {
                state.clear_error();
                state.loading = true;
                fetch_all(client.clone(), tx.clone());
            }
            // Drain any task the command staged but could not spawn itself.
            if let Some(fire) = state.pending_fire.take() {
                match fire {
                    CommandFire::AtomicCraft { recipe_id } => {
                        let name = state.fabrication_recipes().iter()
                            .find(|(_, r)| r.id == recipe_id).map(|(_, r)| r.name.clone());
                        fetch_atomic_printer_craft(recipe_id, client.clone(), tx.clone());
                        if let Some(name) = name {
                            state.log_event(LogEvent::craft(&name, true, state.active_probe_id));
                        }
                    }
                    CommandFire::MannyCraft { manny_id, recipe_id } => {
                        let name = state.fabrication_recipes().iter()
                            .find(|(_, r)| r.id == recipe_id).map(|(_, r)| r.name.clone());
                        fetch_craft(manny_id, recipe_id, client.clone(), tx.clone());
                        if let Some(name) = name {
                            state.log_event(LogEvent::craft(&name, false, state.active_probe_id));
                        }
                    }
                    CommandFire::Mine { manny_id, object_id, resources, amount, container_id } => {
                        let resources_label = resources.join(", ");
                        let destination = if container_id.is_some() { "a container" } else { "the probe" };
                        fetch_mine(
                            manny_id,
                            object_id,
                            resources,
                            amount,
                            container_id,
                            client.clone(),
                            tx.clone(),
                        );
                        state.log_event(LogEvent::mine(&resources_label, amount, destination, state.active_probe_id));
                    }
                }
            }
        }
        KeyCode::Tab => cycle_completion(state),
        KeyCode::Up => history_step(state, -1),
        KeyCode::Down => history_step(state, 1),
        KeyCode::Backspace => {
            let InputMode::Command(cmd) = &mut state.mode else { return };
            if cmd.cursor > 0 {
                cmd.cursor -= 1;
                cmd.input.remove(byte_at(&cmd.input, cmd.cursor));
            }
            cmd.completion = None;
            cmd.history_idx = None;
        }
        KeyCode::Left => {
            let InputMode::Command(cmd) = &mut state.mode else { return };
            cmd.cursor = cmd.cursor.saturating_sub(1);
            cmd.completion = None;
        }
        KeyCode::Right => {
            let InputMode::Command(cmd) = &mut state.mode else { return };
            cmd.cursor = (cmd.cursor + 1).min(cmd.input.chars().count());
            cmd.completion = None;
        }
        KeyCode::Char(c) => {
            let InputMode::Command(cmd) = &mut state.mode else { return };
            let byte = byte_at(&cmd.input, cmd.cursor);
            cmd.input.insert(byte, c);
            cmd.cursor += 1;
            cmd.completion = None;
            cmd.history_idx = None;
        }
        _ => {}
    }
}

/// Byte offset of the character at char-index `idx` (or the end of the string).
fn byte_at(s: &str, idx: usize) -> usize {
    s.char_indices().nth(idx).map_or(s.len(), |(b, _)| b)
}

/// `Tab`: complete the token under the caret. On the first press it computes the
/// candidate set and applies the first match; subsequent presses cycle through
/// the alternatives. A unique verb match is committed with a trailing space,
/// ready for its argument, and the cycle state is cleared.
fn cycle_completion(state: &mut AppState) {
    let (input, existing) = {
        let InputMode::Command(cmd) = &state.mode else { return };
        (cmd.input.clone(), cmd.completion.clone())
    };

    // Cycling an active set, vs. computing a fresh one on the first Tab.
    let (candidates, index, token_start) = match existing {
        Some(c) if c.candidates.len() > 1 => {
            let index = (c.index + 1) % c.candidates.len();
            (c.candidates, index, c.token_start)
        }
        Some(c) => (c.candidates, c.index, c.token_start),
        None => {
            let cursor = match &state.mode {
                InputMode::Command(cmd) => cmd.cursor,
                _ => return,
            };
            match state.command_completions(&input, cursor) {
                Some((token_start, candidates)) => (candidates, 0, token_start),
                None => return,
            }
        }
    };

    let candidate = &candidates[index];
    let unique = candidates.len() == 1;
    // A verb sits at the start of the line (only whitespace before it); an
    // argument does not. A unique verb that takes an argument gets a trailing
    // space so the caret lands where the argument goes.
    let is_verb = input[..token_start].trim().is_empty();
    let mut new_input = String::with_capacity(token_start + candidate.len() + 1);
    new_input.push_str(&input[..token_start]);
    new_input.push_str(candidate);
    if unique && is_verb && command_usage(candidate).is_some() {
        new_input.push(' ');
    }
    let cursor = new_input.chars().count();

    let InputMode::Command(cmd) = &mut state.mode else { return };
    cmd.input = new_input;
    cmd.cursor = cursor;
    cmd.history_idx = None;
    cmd.completion =
        (!unique).then_some(CompletionState { candidates, index, token_start });
}

/// `↑`/`↓`: step through the command history. `dir` is `-1` (older) or `1`
/// (newer). Stepping down past the newest entry restores an empty live line.
fn history_step(state: &mut AppState, dir: i32) {
    let len = state.command_history.len();
    if len == 0 {
        return;
    }
    let cur = match &state.mode {
        InputMode::Command(cmd) => cmd.history_idx,
        _ => return,
    };

    let new_idx: Option<usize> = match (cur, dir) {
        // Not browsing yet: Up enters at the most recent; Down is a no-op.
        (None, d) if d < 0 => Some(len - 1),
        (None, _) => return,
        // Older, clamped at the oldest entry.
        (Some(i), d) if d < 0 => Some(i.saturating_sub(1)),
        // Newer, still within history.
        (Some(i), _) if i + 1 < len => Some(i + 1),
        // Newer past the end → drop back to the live (empty) line.
        (Some(_), _) => {
            let InputMode::Command(cmd) = &mut state.mode else { return };
            cmd.history_idx = None;
            cmd.input.clear();
            cmd.cursor = 0;
            cmd.completion = None;
            return;
        }
    };

    let line = state.command_history[new_idx.unwrap()].clone();
    let InputMode::Command(cmd) = &mut state.mode else { return };
    cmd.history_idx = new_idx;
    cmd.cursor = line.chars().count();
    cmd.input = line;
    cmd.completion = None;
}
