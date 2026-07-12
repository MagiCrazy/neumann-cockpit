use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use super::geometry::list_nav;
use crate::api::client::ApiClient;
use crate::app::{ActiveWizard, ApiMessage, AppState, FabFocus, FabricationInput, Fabricator, QueuedCraft};

/// Upper bound on the per-add quantity (small batches only).
const QTY_MAX: u32 = 99;

/// Drive the production console. The catalog (left) sets a recipe + quantity and
/// `Enter` adds it to the queue; `Tab` moves to the queue panel to manage it. No
/// craft fires here — the executor drains the queue. `PickBuilder` is a brief
/// detour when a Manny recipe has several idle builders to choose from.
pub(super) fn handle_fabrication_event(
    code: KeyCode,
    state: &mut AppState,
    _client: &ApiClient,
    _tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.active_wizard {
        ActiveWizard::Fabrication(FabricationInput::PickRecipe {
            focus: FabFocus::Catalog,
            ..
        }) => handle_catalog(code, state),
        ActiveWizard::Fabrication(FabricationInput::PickRecipe {
            focus: FabFocus::Queue, ..
        }) => handle_queue_panel(code, state),
        ActiveWizard::Fabrication(FabricationInput::PickBuilder { .. }) => handle_builder(code, state),
        _ => {}
    }
}

fn handle_catalog(code: KeyCode, state: &mut AppState) {
    let count = state.fabrication_recipes().len();
    if count == 0 {
        if code == KeyCode::Esc {
            state.close_wizard();
        }
        return;
    }
    let (selection, qty) = match &state.active_wizard {
        ActiveWizard::Fabrication(FabricationInput::PickRecipe { selection, qty, .. }) => (*selection, *qty),
        _ => return,
    };
    match code {
        KeyCode::Esc => state.close_wizard(),
        KeyCode::Char('p') => state.queue_toggle_pause(),
        KeyCode::Tab => {
            if !state.craft_queue.is_empty() {
                set_focus(state, FabFocus::Queue);
            }
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(ns) = list_nav(code, selection, count) {
                mutate_recipe(state, |sel, _| *sel = ns);
            }
        }
        KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Right | KeyCode::Char('l') => {
            mutate_recipe(state, |_, q| *q = (*q + 1).min(QTY_MAX));
        }
        KeyCode::Char('-') | KeyCode::Char('_') | KeyCode::Left | KeyCode::Char('h') => {
            mutate_recipe(state, |_, q| *q = q.saturating_sub(1).max(1));
        }
        KeyCode::Enter => enqueue_selected(state, selection, qty),
        _ => {}
    }
}

fn handle_queue_panel(code: KeyCode, state: &mut AppState) {
    let count = state.craft_queue.len();
    let sel = match &state.active_wizard {
        ActiveWizard::Fabrication(FabricationInput::PickRecipe { queue_sel, .. }) => *queue_sel,
        _ => return,
    };
    match code {
        KeyCode::Esc => state.close_wizard(),
        KeyCode::Tab | KeyCode::Left | KeyCode::Char('h') => set_focus(state, FabFocus::Catalog),
        KeyCode::Char('p') => state.queue_toggle_pause(),
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(ns) = list_nav(code, sel, count) {
                set_queue_sel(state, ns);
            }
        }
        KeyCode::Char('+') | KeyCode::Char('=') => state.queue_bump(sel, 1),
        KeyCode::Char('-') | KeyCode::Char('_') => state.queue_bump(sel, -1),
        KeyCode::Char('x') => {
            state.queue_remove(sel);
            if state.craft_queue.is_empty() {
                set_focus(state, FabFocus::Catalog);
            } else {
                set_queue_sel(state, sel.min(state.craft_queue.len() - 1));
            }
        }
        KeyCode::Char('c') => {
            state.queue_clear();
            set_focus(state, FabFocus::Catalog);
        }
        _ => {}
    }
}

fn handle_builder(code: KeyCode, state: &mut AppState) {
    let count = match &state.active_wizard {
        ActiveWizard::Fabrication(FabricationInput::PickBuilder { mannies, .. }) => mannies.len(),
        _ => return,
    };
    match code {
        KeyCode::Esc => state.close_wizard(),
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let ActiveWizard::Fabrication(FabricationInput::PickBuilder { selection, .. }) = &mut state.active_wizard
            {
                if let Some(ns) = list_nav(code, *selection, count) {
                    *selection = ns;
                }
            }
        }
        KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('Q') => {
            let picked = if let ActiveWizard::Fabrication(FabricationInput::PickBuilder {
                mannies,
                selection,
                recipe_id,
                recipe_name,
                qty,
                ..
            }) = &state.active_wizard
            {
                mannies
                    .get(*selection)
                    .map(|(id, name)| (id.clone(), name.clone(), recipe_id.clone(), recipe_name.clone(), *qty))
            } else {
                None
            };
            if let Some((builder_id, builder_name, recipe_id, recipe_name, qty)) = picked {
                enqueue(
                    state,
                    Fabricator::Manny,
                    recipe_id,
                    recipe_name,
                    Some(builder_id),
                    Some(builder_name),
                    qty,
                );
                // Back to the console so the pilot sees the queue grow.
                state.active_wizard = ActiveWizard::Fabrication(FabricationInput::pick_recipe(None));
            }
        }
        _ => {}
    }
}

/// Add the recipe under the catalog cursor to the queue ×`qty`, resolving its
/// builder as an immediate craft would (pre-chosen / sole idle Manny, else the
/// builder picker). On success the catalog quantity resets to 1.
fn enqueue_selected(state: &mut AppState, selection: usize, qty: u32) {
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
                enqueue(state, fab, recipe_id, recipe_name, None, None, qty);
                mutate_recipe(state, |_, q| *q = 1);
            } else {
                state.set_wizard_error("no atomic printer in inventory".into());
            }
        }
        Fabricator::Manny => {
            let prefilled = if let ActiveWizard::Fabrication(FabricationInput::PickRecipe { prefilled_manny, .. }) =
                &state.active_wizard
            {
                prefilled_manny.clone()
            } else {
                None
            };
            let mannies = state.collect_idle_onboard_mannies();
            let builder = prefilled.or_else(|| {
                if mannies.len() == 1 {
                    mannies.first().cloned()
                } else {
                    None
                }
            });
            match builder {
                Some((id, name)) => {
                    enqueue(state, fab, recipe_id, recipe_name, Some(id), Some(name), qty);
                    mutate_recipe(state, |_, q| *q = 1);
                }
                None if mannies.is_empty() => state.set_wizard_error("no idle Manny on board".into()),
                None => {
                    state.active_wizard =
                        ActiveWizard::Fabrication(FabricationInput::pick_builder(recipe_id, recipe_name, qty, mannies));
                }
            }
        }
    }
}

fn enqueue(
    state: &mut AppState,
    fabricator: Fabricator,
    recipe_id: String,
    recipe_name: String,
    builder_id: Option<String>,
    builder_name: Option<String>,
    qty: u32,
) {
    let mut craft = QueuedCraft::new(fabricator, recipe_id, recipe_name, builder_id, builder_name);
    craft.repeat = qty.max(1);
    state.enqueue_craft(craft);
}

/// Mutate the `(selection, qty)` of the catalog step in place.
fn mutate_recipe(state: &mut AppState, f: impl FnOnce(&mut usize, &mut u32)) {
    if let ActiveWizard::Fabrication(FabricationInput::PickRecipe { selection, qty, .. }) = &mut state.active_wizard {
        f(selection, qty);
    }
}

fn set_focus(state: &mut AppState, to: FabFocus) {
    if let ActiveWizard::Fabrication(FabricationInput::PickRecipe { focus, queue_sel, .. }) = &mut state.active_wizard {
        *focus = to;
        if to == FabFocus::Queue {
            *queue_sel = 0;
        }
    }
}

fn set_queue_sel(state: &mut AppState, sel: usize) {
    if let ActiveWizard::Fabrication(FabricationInput::PickRecipe { queue_sel, .. }) = &mut state.active_wizard {
        *queue_sel = sel;
    }
}
