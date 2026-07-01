//! Input routing for the unified Cockpit v2 interface (bloc U2).
//!
//! Normal-mode navigation only: `ertdfgcvb` activates a pane, `jk`/arrows
//! move the cursor within it, `Tab` cycles panes, `F5` refreshes. Drill-in,
//! zoom, contextual menus (`Enter`) and command mode (`:`) land in later
//! blocs; unhandled keys are ignored.

use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::fetch_all;
use crate::app::{ApiMessage, AppState, Pane};

pub fn handle_cockpit_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match code {
        KeyCode::Char('q') => state.set_quit(),
        KeyCode::Char(c) if Pane::from_key(c).is_some() => {
            state.active_pane = Pane::from_key(c).unwrap();
        }
        KeyCode::Down | KeyCode::Char('j') => state.pane_cursor_down(),
        KeyCode::Up | KeyCode::Char('k') => state.pane_cursor_up(),
        KeyCode::Tab => state.cycle_pane(true),
        KeyCode::BackTab => state.cycle_pane(false),
        KeyCode::F(5) if !state.loading => {
            state.clear_error();
            state.loading = true;
            fetch_all(client.clone(), tx.clone());
        }
        _ => {}
    }
}
