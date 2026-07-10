use crate::app::{ActiveWizard, AppState, DropStorageContainerInput};
use crate::ui::theme::palette;
use ratatui::{layout::Rect, Frame};

use super::render_pick_list;

pub(crate) fn render_drop_container_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let ActiveWizard::DropContainer(drop_container) = &state.active_wizard else {
        return;
    };
    match drop_container {
        DropStorageContainerInput::PickContainer {
            containers, selection, ..
        } => {
            let names: Vec<&str> = containers.iter().map(|(_, n)| n.as_str()).collect();
            let height = (containers.len() as u16 + 6).min(18);
            render_pick_list(
                frame,
                area,
                palette(state.color_mode),
                " DROP CONTAINER — SELECT CONTAINER ",
                54,
                height,
                None,
                &names,
                *selection,
                None,
                "select",
            );
        }
        DropStorageContainerInput::PickPlanet {
            container_name,
            planets,
            selection,
            error,
            ..
        } => {
            let names: Vec<&str> = planets.iter().map(|(_, n)| n.as_str()).collect();
            let height = (planets.len() as u16 + 7).min(18);
            let prompt = format!("Drop {container_name} on planet:");
            render_pick_list(
                frame,
                area,
                palette(state.color_mode),
                " DROP CONTAINER — SELECT PLANET ",
                54,
                height,
                Some(&prompt),
                &names,
                *selection,
                error.as_deref(),
                "DROP",
            );
        }
    }
}
