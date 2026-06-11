pub mod cockpit;
pub(crate) mod overlays;
pub(crate) mod panels;
pub mod retro;
pub(crate) mod theme;

use crate::app::{AppState, UiTheme};
use ratatui::Frame;

/// Theme dispatch: classic 4-panel cockpit or the retro phosphor console.
pub fn render(frame: &mut Frame, state: &AppState) {
    match state.ui_theme {
        UiTheme::Classic => cockpit::render(frame, state),
        UiTheme::Retro => retro::render(frame, state),
    }
}
