//! Blueprint sharing overlay (API v116, #301 phase 3).

use ratatui::{layout::Rect, Frame};

use crate::app::{ActiveWizard, AppState, ShareBlueprintInput};
use crate::ui::theme::palette;

use super::render_pick_list;

pub(crate) fn render_share_blueprint_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let ActiveWizard::ShareBlueprint(share) = &state.active_wizard else {
        return;
    };
    match share {
        ShareBlueprintInput::PickNetwork { networks, selection } => {
            let items: Vec<&str> = networks.iter().map(|(_, n)| n.as_str()).collect();
            let height = (items.len() as u16 + 6).min(16);
            render_pick_list(
                frame,
                area,
                p,
                " SHARE BLUEPRINT — NETWORK ",
                56,
                height,
                Some("Reach the recipient through which network?"),
                &items,
                *selection,
                None,
                "OPEN",
            );
        }
        ShareBlueprintInput::Loading { network_name } => {
            render_pick_list(
                frame,
                area,
                p,
                " SHARE BLUEPRINT ",
                56,
                8,
                Some(&format!("Listing probes on {network_name}…")),
                &[],
                0,
                None,
                "…",
            );
        }
        ShareBlueprintInput::PickRecipient {
            recipients,
            selection,
            error,
        } => {
            let labels: Vec<String> = recipients.iter().map(|(id, name)| format!("{name}  #{id}")).collect();
            let items: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
            let height = (items.len() as u16 + 7).min(18);
            render_pick_list(
                frame,
                area,
                p,
                " SHARE BLUEPRINT — RECIPIENT ",
                58,
                height,
                Some("Another player's probe · you keep the blueprint"),
                &items,
                *selection,
                error.as_deref(),
                "SELECT",
            );
        }
        ShareBlueprintInput::PickBlueprint {
            recipient_name,
            blueprints,
            selection,
            error,
            ..
        } => {
            let labels: Vec<&str> = blueprints.iter().map(|(_, name)| name.as_str()).collect();
            let height = (labels.len() as u16 + 7).min(16);
            render_pick_list(
                frame,
                area,
                p,
                " SHARE BLUEPRINT ",
                58,
                height,
                Some(&format!("Send to {recipient_name}")),
                &labels,
                *selection,
                error.as_deref(),
                "SHARE",
            );
        }
    }
}
