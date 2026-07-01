pub mod cockpit;
pub mod cockpit_v2;
pub(crate) mod overlays;
pub(crate) mod panels;
pub mod retro;
pub(crate) mod theme;

use crate::app::{AppState, UiTheme};
use ratatui::Frame;

/// Layout dispatch: classic 4-panel cockpit, retro phosphor console, or the
/// unified Cockpit v2 interface (opt-in preview during the U-series).
pub fn render(frame: &mut Frame, state: &AppState) {
    match state.ui_theme {
        UiTheme::Classic => cockpit::render(frame, state),
        UiTheme::Retro => retro::render(frame, state),
        UiTheme::Cockpit => cockpit_v2::render(frame, state),
    }
}
