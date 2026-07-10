use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use super::geometry::list_nav;
use crate::api::client::ApiClient;
use crate::api::tasks::{fetch_atomic_printer_craft, fetch_craft};
use crate::app::{ActiveWizard, ApiMessage, AppState, FabricationInput, Fabricator, LogEvent, QueuedCraft};

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
    match state.active_wizard {
        ActiveWizard::Fabrication(FabricationInput::PickRecipe { selection, .. }) => {
            let count = state.fabrication_recipes().len();
            if count == 0 {
                if code == KeyCode::Esc {
                    state.close_wizard();
                }
                return;
            }
            match code {
                KeyCode::Esc => state.close_wizard(),
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, selection, count) {
                        if let ActiveWizard::Fabrication(FabricationInput::PickRecipe { ref mut selection, .. }) =
                            state.active_wizard
                        {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => commit_recipe(selection, state, client, tx),
                // Add the selected recipe to the production queue and stay open,
                // so repeated presses stack (they coalesce into one ×N step).
                KeyCode::Char('q') | KeyCode::Char('Q') => enqueue_recipe(selection, state),
                _ => {}
            }
        }
        ActiveWizard::Fabrication(FabricationInput::PickBuilder {
            selection, ref mannies, ..
        }) => {
            let count = mannies.len();
            match code {
                KeyCode::Esc => state.close_wizard(),
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, selection, count) {
                        if let ActiveWizard::Fabrication(FabricationInput::PickBuilder { ref mut selection, .. }) =
                            state.active_wizard
                        {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let picked = if let ActiveWizard::Fabrication(FabricationInput::PickBuilder {
                        ref mannies,
                        selection,
                        ref recipe_id,
                        ref recipe_name,
                        ..
                    }) = state.active_wizard
                    {
                        mannies
                            .get(selection)
                            .map(|(id, _)| (id.clone(), recipe_id.clone(), recipe_name.clone()))
                    } else {
                        None
                    };
                    if let Some((manny_id, recipe_id, recipe_name)) = picked {
                        fetch_craft(manny_id, recipe_id, client.clone(), tx.clone());
                        state.log_event(LogEvent::craft(&recipe_name, false, state.active_probe_id));
                    }
                }
                // Queue the recipe with the highlighted builder; stay open so
                // repeated presses stack into one ×N step.
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    let picked = if let ActiveWizard::Fabrication(FabricationInput::PickBuilder {
                        ref mannies,
                        selection,
                        ref recipe_id,
                        ref recipe_name,
                        ..
                    }) = state.active_wizard
                    {
                        mannies
                            .get(selection)
                            .map(|(id, name)| (id.clone(), name.clone(), recipe_id.clone(), recipe_name.clone()))
                    } else {
                        None
                    };
                    if let Some((builder_id, builder_name, recipe_id, recipe_name)) = picked {
                        state.enqueue_craft(QueuedCraft::new(
                            Fabricator::Manny,
                            recipe_id,
                            recipe_name,
                            Some(builder_id),
                            Some(builder_name),
                        ));
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

/// Add the recipe under the catalog cursor to the production queue, resolving
/// its builder the same way an immediate craft would. An atomic recipe needs a
/// printer; a Manny recipe uses the pre-chosen or sole idle builder, else falls
/// through to `PickBuilder` so the pilot names one (then `[Q]` there queues).
fn enqueue_recipe(selection: usize, state: &mut AppState) {
    let Some((fab, recipe_id, recipe_name)) = state
        .fabrication_recipes()
        .get(selection)
        .map(|(fab, r)| (*fab, r.id.clone(), r.name.clone()))
    else {
        return;
    };
    match fab {
        Fabricator::AtomicPrinter => {
            if state.has_atomic_printer() {
                state.enqueue_craft(QueuedCraft::new(fab, recipe_id, recipe_name, None, None));
            } else {
                state.set_wizard_error("no atomic printer in inventory".into());
            }
        }
        Fabricator::Manny => {
            let prefilled = if let ActiveWizard::Fabrication(FabricationInput::PickRecipe {
                ref prefilled_manny, ..
            }) = state.active_wizard
            {
                prefilled_manny.clone()
            } else {
                None
            };
            let mannies = state.collect_idle_onboard_mannies();
            let builder = prefilled.or_else(|| if mannies.len() == 1 { mannies.first().cloned() } else { None });
            match builder {
                Some((id, name)) => {
                    state.enqueue_craft(QueuedCraft::new(fab, recipe_id, recipe_name, Some(id), Some(name)));
                }
                None if mannies.is_empty() => state.set_wizard_error("no idle Manny on board".into()),
                None => {
                    state.active_wizard = ActiveWizard::Fabrication(FabricationInput::PickBuilder {
                        recipe_id,
                        recipe_name,
                        mannies,
                        selection: 0,
                        error: None,
                    });
                }
            }
        }
    }
}

/// Fire (or advance) the craft for the recipe under the catalog cursor.
fn commit_recipe(selection: usize, state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    let Some((fab, recipe_id, recipe_name)) = state
        .fabrication_recipes()
        .get(selection)
        .map(|(fab, r)| (*fab, r.id.clone(), r.name.clone()))
    else {
        return;
    };
    match fab {
        Fabricator::AtomicPrinter => {
            if state.has_atomic_printer() {
                fetch_atomic_printer_craft(recipe_id, client.clone(), tx.clone());
                state.log_event(LogEvent::craft(&recipe_name, true, state.active_probe_id));
            } else {
                state.set_wizard_error("no atomic printer in inventory".into());
            }
        }
        Fabricator::Manny => {
            let prefilled = if let ActiveWizard::Fabrication(FabricationInput::PickRecipe {
                ref prefilled_manny, ..
            }) = state.active_wizard
            {
                prefilled_manny.clone()
            } else {
                None
            };
            if let Some((manny_id, _)) = prefilled {
                fetch_craft(manny_id, recipe_id, client.clone(), tx.clone());
                state.log_event(LogEvent::craft(&recipe_name, false, state.active_probe_id));
                return;
            }
            let mannies = state.collect_idle_onboard_mannies();
            match mannies.len() {
                0 => state.set_wizard_error("no idle Manny on board".into()),
                1 => {
                    let (manny_id, _) = mannies.into_iter().next().unwrap();
                    fetch_craft(manny_id, recipe_id, client.clone(), tx.clone());
                    state.log_event(LogEvent::craft(&recipe_name, false, state.active_probe_id));
                }
                _ => {
                    state.active_wizard = ActiveWizard::Fabrication(FabricationInput::PickBuilder {
                        recipe_id,
                        recipe_name,
                        mannies,
                        selection: 0,
                        error: None,
                    });
                }
            }
        }
    }
}
