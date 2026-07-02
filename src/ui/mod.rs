pub mod cockpit_v2;
pub(crate) mod overlays;
pub(crate) mod panels;
pub(crate) mod theme;

use crate::app::AppState;
use ratatui::Frame;

/// Single render entry point — the unified Cockpit interface.
pub fn render(frame: &mut Frame, state: &AppState) {
    cockpit_v2::render(frame, state);
}
