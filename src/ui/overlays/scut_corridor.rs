//! Safe SCUT corridors (API v96, issue #257).
//!
//! A jump between two sectors that each hold an **active relay equipped with a
//! transit beacon**, in the **same** SCUT network, waives the probe-destruction
//! risk. Container-detachment risk is unaffected, and the overlay says so: a
//! corridor is safer, not free.

use ratatui::{layout::Rect, Frame};

use crate::app::{ActiveWizard, AppState, ScutCorridorInput};
use crate::ui::theme::palette;

use super::render_pick_list;

pub(crate) fn render_scut_corridor_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let ActiveWizard::ScutCorridor(corridor) = &state.active_wizard else {
        return;
    };
    match corridor {
        ScutCorridorInput::PickNetwork { networks, selection } => {
            let items: Vec<&str> = networks.iter().map(|(_, name)| name.as_str()).collect();
            let height = (items.len() as u16 + 6).min(16);
            render_pick_list(
                frame,
                area,
                p,
                " SAFE CORRIDORS — NETWORK ",
                56,
                height,
                Some("Several beacon relays here — pick a network"),
                &items,
                *selection,
                None,
                "OPEN",
            );
        }
        ScutCorridorInput::Loading { network_name } => {
            render_pick_list(
                frame,
                area,
                p,
                " SAFE CORRIDORS ",
                56,
                8,
                Some(&format!("Mapping {network_name}…")),
                &[],
                0,
                None,
                "…",
            );
        }
        ScutCorridorInput::Picking { selection, error } => {
            let destinations = state.corridor_destinations();
            let labels: Vec<String> = destinations
                .iter()
                .map(|d| {
                    let (x, y, z) = d.coords;
                    format!("({x}, {y}, {z})  ≣✓ {}  · {}", d.relay_name, d.network_name)
                })
                .collect();
            let items: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
            let height = (items.len() as u16 + 7).min(18);
            render_pick_list(
                frame,
                area,
                p,
                " SAFE CORRIDORS ",
                62,
                height,
                Some("No destruction risk · containers can still detach"),
                &items,
                *selection,
                error.as_deref(),
                "TRAVEL",
            );
        }
    }
}
