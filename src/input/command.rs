use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::fetch_all;
use crate::app::{ApiMessage, AppState, CommandLine, InputMode, COMMANDS};

/// Route a key while the `:` command line is open. Typed characters are literal
/// input; `Enter` runs the command, `Tab` completes the verb, `Esc` cancels.
pub(super) fn handle_command_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    let InputMode::Command(ref mut cmd) = state.mode else { return };
    match code {
        KeyCode::Esc => state.mode = InputMode::Normal,
        KeyCode::Enter => {
            let line = cmd.input.clone();
            state.mode = InputMode::Normal;
            if state.run_command(&line) {
                state.clear_error();
                state.loading = true;
                fetch_all(client.clone(), tx.clone());
            }
        }
        KeyCode::Tab => complete_verb(cmd),
        KeyCode::Backspace => {
            if cmd.cursor > 0 {
                cmd.cursor -= 1;
                cmd.input.remove(cmd.cursor);
            }
        }
        KeyCode::Left => cmd.cursor = cmd.cursor.saturating_sub(1),
        KeyCode::Right => cmd.cursor = (cmd.cursor + 1).min(cmd.input.chars().count()),
        KeyCode::Char(c) => {
            let byte = cmd.input.char_indices().nth(cmd.cursor).map_or(cmd.input.len(), |(b, _)| b);
            cmd.input.insert(byte, c);
            cmd.cursor += 1;
        }
        _ => {}
    }
}

/// Tab-complete the verb (first token) to the unique matching command.
fn complete_verb(cmd: &mut CommandLine) {
    // Only complete while still typing the verb (no space yet).
    if cmd.input.contains(' ') {
        return;
    }
    let prefix = cmd.input.as_str();
    let matches: Vec<&&str> = COMMANDS.iter().filter(|c| c.starts_with(prefix)).collect();
    if let [only] = matches.as_slice() {
        cmd.input = format!("{only} ");
        cmd.cursor = cmd.input.chars().count();
    }
}
