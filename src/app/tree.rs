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

use std::collections::{BTreeMap, BTreeSet};

use super::{AppState, Fabricator};
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

// ── :tree overlay state ──────────────────────────────────────────────────────

/// The full-screen tech-tree browser (`:tree`). A booted flag + its own state,
/// kept outside `ActiveWizard` like the isometric map — mutually exclusive with
/// the wizards in practice, but structurally independent of them.
#[derive(Default)]
pub struct TreeView {
    pub open: bool,
    /// Index into the currently visible rows (`tree_rows`); always lands on a
    /// selectable (non-header) row.
    pub cursor: usize,
    /// Node paths currently expanded. Keyed by full path (not bare item id) so
    /// the same component under two parents expands independently.
    pub expanded: BTreeSet<String>,
    /// Roll-up multiplier applied to the selected node (`+`/`-`).
    pub qty: u32,
}

/// One rendered row of the tech tree: either a section header or a node.
#[derive(Debug, Clone)]
pub struct TreeRow {
    /// Unique path from its section root (`atomic:integrated_circuit/micro_conductor`).
    pub path: String,
    /// Item id (`""` for a header).
    pub item: String,
    pub label: String,
    /// Indent depth (0 = a section's top-level recipe).
    pub depth: usize,
    /// Absolute count needed to build `TreeView::qty` of this branch's root.
    pub qty_abs: f64,
    pub fabricator: Option<Fabricator>,
    /// This node's own recipe duration in seconds (0 for base/unknown/header).
    pub duration_seconds: i64,
    pub is_header: bool,
    /// A base resource or an unmodelled leaf (no recipe).
    pub is_base: bool,
    pub expandable: bool,
    pub expanded: bool,
}

impl TreeRow {
    fn header(label: &str) -> Self {
        TreeRow {
            path: String::new(),
            item: String::new(),
            label: label.to_string(),
            depth: 0,
            qty_abs: 0.0,
            fabricator: None,
            duration_seconds: 0,
            is_header: true,
            is_base: false,
            expandable: false,
            expanded: false,
        }
    }
}

/// Which fabricator builds a recipe (atomic printer takes precedence when a
/// recipe lists both).
fn recipe_fabricator(recipe: &CraftingRecipe) -> Fabricator {
    if recipe.craftable_by.iter().any(|c| c == "atomic_3d_printer") {
        Fabricator::AtomicPrinter
    } else {
        Fabricator::Manny
    }
}

impl AppState {
    /// Open the tech-tree overlay, seeding the quantity and landing the cursor
    /// on the first selectable row.
    pub fn open_tree(&mut self) {
        self.tree.open = true;
        if self.tree.qty == 0 {
            self.tree.qty = 1;
        }
        let rows = self.tree_rows();
        self.tree.cursor = rows.iter().position(|r| !r.is_header).unwrap_or(0);
    }

    /// The visible rows of the tree given the current expansion set: two
    /// fabricator sections, each recipe a root that expands into its ingredient
    /// sub-tree.
    pub fn tree_rows(&self) -> Vec<TreeRow> {
        let mut rows = Vec::new();
        let qty = self.tree.qty.max(1) as f64;
        let sections = [
            ("ATOMIC PRINTER", self.atomic_printer_recipes()),
            ("MANNY BAY", self.manny_craft_recipes()),
        ];
        for (label, recipes) in sections {
            if recipes.is_empty() {
                continue;
            }
            rows.push(TreeRow::header(label));
            for recipe in recipes {
                let tag = match recipe_fabricator(recipe) {
                    Fabricator::AtomicPrinter => "atomic",
                    Fabricator::Manny => "manny",
                };
                let root_path = format!("{tag}:{}", recipe.id);
                self.push_tree_node(&mut rows, &root_path, &recipe.id, qty, 0);
            }
        }
        rows
    }

    /// Push `item` as a row and, when expanded, recurse into its ingredients.
    fn push_tree_node(&self, rows: &mut Vec<TreeRow>, path: &str, item: &str, qty_abs: f64, depth: usize) {
        let recipe = self.recipes.iter().find(|r| r.id == item || r.output.output_type == item);
        let expandable = recipe.map(|r| !r.ingredients.is_empty()).unwrap_or(false);
        let expanded = expandable && self.tree.expanded.contains(path);
        let label = recipe.map(|r| r.name.clone()).unwrap_or_else(|| item.to_string());
        rows.push(TreeRow {
            path: path.to_string(),
            item: item.to_string(),
            label,
            depth,
            qty_abs,
            fabricator: recipe.map(recipe_fabricator),
            duration_seconds: recipe.map(|r| r.duration_seconds).unwrap_or(0),
            is_header: false,
            is_base: recipe.is_none(),
            expandable,
            expanded,
        });
        if let (Some(recipe), true) = (recipe, expanded) {
            for ing in &recipe.ingredients {
                let child_path = format!("{path}/{}", ing.ingredient_type);
                self.push_tree_node(rows, &child_path, &ing.ingredient_type, qty_abs * ing.quantity, depth + 1);
            }
        }
    }

    /// Move the tree cursor by `delta`, skipping non-selectable header rows.
    pub fn tree_move(&mut self, delta: isize) {
        let rows = self.tree_rows();
        if rows.is_empty() {
            return;
        }
        let mut i = self.tree.cursor as isize;
        let step = delta.signum();
        loop {
            i += step;
            if i < 0 || i >= rows.len() as isize {
                return; // hit an edge — leave the cursor where it was
            }
            if !rows[i as usize].is_header {
                self.tree.cursor = i as usize;
                return;
            }
        }
    }

    /// Toggle expansion of the node under the cursor (`Enter`).
    pub fn tree_toggle(&mut self) {
        let rows = self.tree_rows();
        let Some(row) = rows.get(self.tree.cursor) else { return };
        if !row.expandable {
            return;
        }
        if row.expanded {
            self.tree.expanded.remove(&row.path);
        } else {
            self.tree.expanded.insert(row.path.clone());
        }
    }

    /// Expand the node under the cursor if it isn't already (`l`/→).
    pub fn tree_expand(&mut self) {
        let rows = self.tree_rows();
        let Some(row) = rows.get(self.tree.cursor) else { return };
        if row.expandable && !row.expanded {
            self.tree.expanded.insert(row.path.clone());
        }
    }

    /// Collapse the node under the cursor if it is expanded (`h`/←).
    pub fn tree_collapse(&mut self) {
        let rows = self.tree_rows();
        let Some(row) = rows.get(self.tree.cursor) else { return };
        if row.expanded {
            self.tree.expanded.remove(&row.path);
        }
    }

    /// Adjust the roll-up quantity (clamped to 1..=999).
    pub fn tree_adjust_qty(&mut self, delta: i32) {
        self.tree.qty = (self.tree.qty.max(1) as i32 + delta).clamp(1, 999) as u32;
    }

    /// The item id currently selected, if any (for the detail roll-up).
    pub fn tree_selected_item(&self) -> Option<String> {
        let rows = self.tree_rows();
        rows.get(self.tree.cursor).filter(|r| !r.is_header).map(|r| r.item.clone())
    }
}
