use crossterm::event::KeyCode;

use crate::app::{AppState, LIST_PAGE};

/// Key handling for the full-screen tech-tree browser (`:tree`, #200):
/// `j`/`k` move (`PgUp`/`PgDn`/`Home`/`End` page and jump, issue #325), `l`/`h`
/// expand/collapse the selected node, `+`/`-` scale the roll-up quantity,
/// `Esc`/`q` close.
pub(super) fn handle_tree_event(code: KeyCode, state: &mut AppState) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => state.tree.open = false,
        KeyCode::Down | KeyCode::Char('j') => state.tree_move(1),
        KeyCode::Up | KeyCode::Char('k') => state.tree_move(-1),
        KeyCode::PageDown => state.tree_move(LIST_PAGE as isize),
        KeyCode::PageUp => state.tree_move(-(LIST_PAGE as isize)),
        KeyCode::Right | KeyCode::Char('l') => state.tree_expand(),
        KeyCode::Left | KeyCode::Char('h') => state.tree_collapse(),
        KeyCode::Enter => state.tree_toggle(),
        KeyCode::Char('+') | KeyCode::Char('=') => state.tree_adjust_qty(1),
        KeyCode::Char('-') | KeyCode::Char('_') => state.tree_adjust_qty(-1),
        _ => {}
    }
}
