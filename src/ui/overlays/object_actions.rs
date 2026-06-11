use crate::app::{AppState, ObjectActionInput};
use ratatui::{
    layout::Rect,
    Frame,
};

use super::render_pick_list;
pub(crate) fn render_object_action_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    match &state.object_action {
        ObjectActionInput::PickAction { object_name, actions, selection, .. } => {
            let labels: Vec<&str> = actions.iter().map(|a| a.label()).collect();
            let height = (actions.len() as u16 + 6).min(14);
            render_pick_list(frame, area, &format!(" {object_name} "), 46, height,
                Some("Action:"), &labels, *selection, None, "select");
        }
        ObjectActionInput::PickManny { object_name, action, mannies, selection, .. } => {
            let names: Vec<&str> = mannies.iter().map(|(_, n)| n.as_str()).collect();
            let height = (mannies.len() as u16 + 6).min(16);
            let prompt = format!("{} — select manny:", action.label());
            render_pick_list(frame, area, &format!(" {object_name} "), 46, height,
                Some(&prompt), &names, *selection, None, "confirm");
        }
        ObjectActionInput::Inactive => {}
    }
}

