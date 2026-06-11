use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::{
    fetch_atomic_printer_craft, fetch_craft,
};
use crate::app::{
    ApiMessage, AppState, AtomicPrinterCraftInput, CraftInput,
};
use super::geometry::list_nav;
pub(super) fn handle_craft_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    let &CraftInput::PickRecipe { selection, .. } = &state.craft else { return };
    let count = state.manny_craft_recipes().len();
    if count == 0 { return; }
    let sel = selection;
    match code {
        KeyCode::Esc => state.craft = CraftInput::Inactive,
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(new_sel) = list_nav(code, sel, count) {
                if let CraftInput::PickRecipe { ref mut selection, .. } = state.craft {
                    *selection = new_sel;
                }
            }
        }
        KeyCode::Enter => {
            let (manny_id, recipe_id) = {
                let CraftInput::PickRecipe { ref manny_id, selection, .. } = state.craft else { return };
                let recipe_id = state.manny_craft_recipes()[selection].id.clone();
                (manny_id.clone(), recipe_id)
            };
            fetch_craft(manny_id, recipe_id, client.clone(), tx.clone());
        }
        _ => {}
    }
}

pub(super) fn handle_atomic_printer_craft_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    let &AtomicPrinterCraftInput::PickRecipe { selection, .. } = &state.atomic_printer_craft else { return };
    let count = state.atomic_printer_recipes().len();
    if count == 0 { return; }
    let sel = selection;
    match code {
        KeyCode::Esc => state.atomic_printer_craft = AtomicPrinterCraftInput::Inactive,
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(new_sel) = list_nav(code, sel, count) {
                if let AtomicPrinterCraftInput::PickRecipe { ref mut selection, .. } = state.atomic_printer_craft {
                    *selection = new_sel;
                }
            }
        }
        KeyCode::Enter => {
            let recipe_id = state.atomic_printer_recipes()[selection].id.clone();
            fetch_atomic_printer_craft(recipe_id, client.clone(), tx.clone());
        }
        _ => {}
    }
}
