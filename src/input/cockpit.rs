//! Input routing for the unified Cockpit v2 interface (blocs U2–U3).
//!
//! Normal-mode navigation: `ertdfgcvb` activates a pane, `jk`/arrows move the
//! cursor within it, `l`/`h` drill in/out, `z` zooms, `Tab` cycles panes,
//! `F5` refreshes. Contextual menus (`Enter`) and command mode (`:`) land in
//! later blocs; unhandled keys are ignored.

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
        KeyCode::Right | KeyCode::Char('l') => state.pane_drill_in(),
        KeyCode::Left | KeyCode::Char('h') => {
            state.pane_drill_out();
        }
        KeyCode::Char('z') => state.toggle_zoom(),
        // Esc backs out one step: leave zoom first, then drill up.
        KeyCode::Esc => {
            if state.zoomed {
                state.zoomed = false;
            } else {
                state.pane_drill_out();
            }
        }
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
