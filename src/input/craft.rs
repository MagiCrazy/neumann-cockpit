use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::{fetch_atomic_printer_craft, fetch_craft};
use crate::app::{ApiMessage, AppState, Fabricator, FabricationInput};
use super::geometry::list_nav;

/// Drive the unified fabrication wizard. `PickRecipe` browses the sectioned
/// catalog; committing an atomic recipe fires straight away, a Manny recipe
/// either uses the pre-chosen builder, auto-picks the sole idle Manny, or
/// advances to `PickBuilder` to choose among several.
pub(super) fn handle_fabrication_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match state.fabrication {
        FabricationInput::PickRecipe { selection, .. } => {
            let count = state.fabrication_recipes().len();
            if count == 0 {
                if code == KeyCode::Esc {
                    state.fabrication = FabricationInput::Inactive;
                }
                return;
            }
            match code {
                KeyCode::Esc => state.fabrication = FabricationInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, selection, count) {
                        if let FabricationInput::PickRecipe { ref mut selection, .. } = state.fabrication {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => commit_recipe(selection, state, client, tx),
                _ => {}
            }
        }
        FabricationInput::PickBuilder { selection, ref mannies, .. } => {
            let count = mannies.len();
            match code {
                KeyCode::Esc => state.fabrication = FabricationInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, selection, count) {
                        if let FabricationInput::PickBuilder { ref mut selection, .. } = state.fabrication {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let picked = if let FabricationInput::PickBuilder { ref mannies, selection, ref recipe_id, .. } = state.fabrication {
                        mannies.get(selection).map(|(id, _)| (id.clone(), recipe_id.clone()))
                    } else {
                        None
                    };
                    if let Some((manny_id, recipe_id)) = picked {
                        fetch_craft(manny_id, recipe_id, client.clone(), tx.clone());
                    }
                }
                _ => {}
            }
        }
        FabricationInput::Inactive => {}
    }
}

/// Fire (or advance) the craft for the recipe under the catalog cursor.
fn commit_recipe(
    selection: usize,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    let Some((fab, recipe_id, recipe_name)) = state.fabrication_recipes().get(selection)
        .map(|(fab, r)| (*fab, r.id.clone(), r.name.clone()))
    else {
        return;
    };
    match fab {
        Fabricator::AtomicPrinter => {
            if state.has_atomic_printer() {
                fetch_atomic_printer_craft(recipe_id, client.clone(), tx.clone());
            } else {
                state.set_fabrication_error("no atomic printer in inventory".into());
            }
        }
        Fabricator::Manny => {
            let prefilled = if let FabricationInput::PickRecipe { ref prefilled_manny, .. } = state.fabrication {
                prefilled_manny.clone()
            } else {
                None
            };
            if let Some((manny_id, _)) = prefilled {
                fetch_craft(manny_id, recipe_id, client.clone(), tx.clone());
                return;
            }
            let mannies = state.collect_idle_onboard_mannies();
            match mannies.len() {
                0 => state.set_fabrication_error("no idle Manny on board".into()),
                1 => {
                    let (manny_id, _) = mannies.into_iter().next().unwrap();
                    fetch_craft(manny_id, recipe_id, client.clone(), tx.clone());
                }
                _ => {
                    state.fabrication = FabricationInput::PickBuilder {
                        recipe_id,
                        recipe_name,
                        mannies,
                        selection: 0,
                        error: None,
                    };
                }
            }
        }
    }
}
