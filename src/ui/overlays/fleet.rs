use ratatui::{layout::Rect, Frame};

use crate::app::{AppState, ProbeSwitchInput};
use crate::ui::theme::{palette, probe_status_label};

use super::render_pick_list;

/// Fleet picker (API v81): one row per probe with a default (★) / active (▸)
/// marker, its status, and SCUT reachability. Selecting a reachable row pilots
/// that probe; an unreachable one is refused (see the input handler).
pub(crate) fn render_probe_switch_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let ProbeSwitchInput::Picking { selection } = state.probe_switch else {
        return;
    };
    let p = palette(state.color_mode);
    let active = state.active_probe_id.or(state.default_probe_id);
    let labels: Vec<String> = state
        .fleet
        .iter()
        .map(|pr| {
            let mark = if pr.is_default {
                "★"
            } else if Some(pr.id) == active {
                "▸"
            } else {
                " "
            };
            let reach = if pr.is_reachable { "" } else { "   ⚠ out of SCUT range" };
            format!("{mark} {}  ·  {}{reach}", pr.name, probe_status_label(&pr.status))
        })
        .collect();
    let refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
    let height = (refs.len() as u16 + 6).clamp(8, 20);
    render_pick_list(
        frame, area, p, " SWITCH PROBE ", 52, height,
        Some("Pilot:"), &refs, selection, None, "pilot",
    );
}
