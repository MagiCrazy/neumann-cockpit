//! Tech-tree cost engine (#200).
//!
//! Recipes form a dependency DAG: a craftable item lists ingredients that are
//! themselves either craftable items or raw resources. This module walks that
//! DAG for a target item and rolls it up to the four base resources
//! (`metals`, `ice`, `carbon_compounds`, `deuterium`), counting the
//! intermediate craft operations and summing their durations along the way.
//!
//! Kept as pure functions over `&[CraftingRecipe]` so the roll-up is unit-tested
//! without a live `AppState`; `AppState::recipe_rollup` is the thin wrapper the
//! `:tree` overlay calls against `self.recipes`.

use std::collections::BTreeMap;

use super::AppState;
use crate::api::types::CraftingRecipe;

/// The raw resources every recipe chain bottoms out in. Anything not craftable
/// by a known recipe is a leaf; the four below are the ones the game mines.
pub const BASE_RESOURCES: [&str; 4] = ["metals", "ice", "carbon_compounds", "deuterium"];

/// Depth guard against a malformed (cyclic) recipe graph. Real recipe chains are
/// only a handful deep; this is a backstop so a server-side cycle can never spin
/// the roll-up forever.
const MAX_DEPTH: usize = 64;

/// The rolled-up cost of building some quantity of a target item.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rollup {
    /// Base resource → total amount (in the ingredient's own unit; the four
    /// `BASE_RESOURCES` are all `earth_container_equivalent`). Also collects any
    /// leaf ingredient that has no known recipe, so an unmodelled dependency
    /// surfaces visibly rather than silently vanishing.
    pub base: BTreeMap<String, f64>,
    /// Craftable item id → number of craft operations needed (the target item
    /// itself included), i.e. how many times each fabricator run fires.
    pub crafts: BTreeMap<String, f64>,
    /// Sum of every craft operation's `durationSeconds`. This is cumulative
    /// fabricator time, not wall-clock (lanes run in parallel).
    pub duration_seconds: f64,
}

impl Rollup {
    /// Total base-resource amount across the four tracked resources.
    pub fn base_total(&self) -> f64 {
        BASE_RESOURCES.iter().filter_map(|r| self.base.get(*r)).sum()
    }

    /// Total craft operations (every fabricator firing, target included).
    pub fn craft_ops(&self) -> f64 {
        self.crafts.values().sum()
    }
}

fn recipe_by_id<'a>(recipes: &'a [CraftingRecipe], id: &str) -> Option<&'a CraftingRecipe> {
    recipes.iter().find(|r| r.id == id || r.output.output_type == id)
}

fn accumulate(recipes: &[CraftingRecipe], item: &str, qty: f64, depth: usize, out: &mut Rollup) {
    match recipe_by_id(recipes, item) {
        // A craftable item: count `qty` fabricator runs, add its duration, and
        // recurse into its ingredients scaled by `qty`.
        Some(recipe) if depth < MAX_DEPTH => {
            *out.crafts.entry(item.to_string()).or_insert(0.0) += qty;
            out.duration_seconds += recipe.duration_seconds as f64 * qty;
            for ing in &recipe.ingredients {
                accumulate(recipes, &ing.ingredient_type, ing.quantity * qty, depth + 1, out);
            }
        }
        // A raw resource, an unmodelled item, or a cycle backstop: leaf.
        _ => {
            *out.base.entry(item.to_string()).or_insert(0.0) += qty;
        }
    }
}

/// Roll `qty` of `item` up to its base-resource cost, craft-op count, and
/// cumulative fabricator duration. An unknown item (no recipe) rolls up to
/// itself as a single leaf.
pub fn recipe_rollup(recipes: &[CraftingRecipe], item: &str, qty: f64) -> Rollup {
    let mut out = Rollup::default();
    accumulate(recipes, item, qty, 0, &mut out);
    out
}

impl AppState {
    /// Roll a target item up to base resources against the live recipe set.
    pub fn recipe_rollup(&self, item: &str, qty: f64) -> Rollup {
        recipe_rollup(&self.recipes, item, qty)
    }
}
