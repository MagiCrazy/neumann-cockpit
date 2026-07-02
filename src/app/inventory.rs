use crate::api::types::{CraftingRecipe, CraftingRecipeIngredient, ProbeInventory};
use super::*;

/// Active items (manny, atomic printer) are listed individually in the
/// inventory panel; passive items are grouped by type.
pub fn is_active_item(item_type: &str) -> bool {
    matches!(item_type, "manny" | "atomic_3d_printer")
}

/// One navigable row of the inventory panel, in display order.
#[derive(Debug, Clone, PartialEq)]
pub enum InventoryRow {
    Stock { id: String },
    ActiveItem { id: String },
    PassiveGroup { item_type: String },
}

impl AppState {
    pub(crate) fn clamp_inventory_selection(&mut self) {
        let count = self.inventory_rows().len();
        self.inventory_selection = if count == 0 {
            0
        } else {
            self.inventory_selection.min(count - 1)
        };
    }

    pub fn mine_max_amount(&self) -> f64 {
        self.probe.as_ref()
            .map(|p| (p.inventory.free_capacity * 10000.0).round() / 10000.0)
            .unwrap_or(0.30)
            .max(0.0)
    }

    /// Navigable rows of the inventory panel, in display order:
    /// resource stocks, then active items, then passive groups.
    pub fn inventory_rows(&self) -> Vec<InventoryRow> {
        let Some(probe) = &self.probe else { return vec![] };
        let inv = &probe.inventory;
        let mut out: Vec<InventoryRow> = Vec::new();
        for stock in &inv.resource_stocks {
            out.push(InventoryRow::Stock { id: stock.id.clone() });
        }
        for item in inv.items.iter().filter(|i| is_active_item(&i.item_type)) {
            out.push(InventoryRow::ActiveItem { id: item.id.clone() });
        }
        let mut seen: Vec<&str> = Vec::new();
        for item in inv.items.iter().filter(|i| !is_active_item(&i.item_type)) {
            if !seen.contains(&item.item_type.as_str()) {
                seen.push(&item.item_type);
                out.push(InventoryRow::PassiveGroup { item_type: item.item_type.clone() });
            }
        }
        out
    }

    pub fn selected_inventory_row(&self) -> Option<InventoryRow> {
        self.inventory_rows().into_iter().nth(self.inventory_selection)
    }

    pub fn inventory_next(&mut self) {
        let count = self.inventory_rows().len();
        if count > 0 {
            self.inventory_selection = (self.inventory_selection + 1) % count;
        }
    }

    pub fn inventory_prev(&mut self) {
        let count = self.inventory_rows().len();
        if count > 0 {
            self.inventory_selection = self
                .inventory_selection
                .checked_sub(1)
                .unwrap_or(count - 1);
        }
    }

    /// Build the jettison wizard state for the currently selected inventory row.
    pub fn jettison_for_selected(&self) -> Result<JettisonInput, String> {
        let Some(probe) = &self.probe else { return Err("no probe data".into()) };
        match self.selected_inventory_row() {
            Some(InventoryRow::Stock { id }) => {
                let stock = probe.inventory.resource_stocks.iter()
                    .find(|s| s.id == id)
                    .ok_or_else(|| "stock not found".to_string())?;
                if stock.amount <= 0.0 {
                    return Err(format!("{} stock is empty", stock.name));
                }
                Ok(JettisonInput::EnterAmount {
                    item_id: stock.id.clone(),
                    item_name: stock.name.clone(),
                    max_amount: stock.amount,
                    buf: String::new(),
                    error: None,
                })
            }
            Some(InventoryRow::ActiveItem { id }) => {
                let item = probe.inventory.items.iter()
                    .find(|i| i.id == id)
                    .ok_or_else(|| "item not found".to_string())?;
                if item.item_type != "manny" {
                    return Err("only resource stocks and mannies can be jettisoned".into());
                }
                let in_probe = item.location.as_ref()
                    .map(|l| l.location_type == crate::api::types::MannyLocationType::Probe)
                    .unwrap_or(false);
                if !in_probe {
                    return Err(format!("{} is not aboard the probe", item.name));
                }
                if item.current_task.is_some() {
                    return Err(format!("{} is busy", item.name));
                }
                Ok(JettisonInput::ConfirmManny {
                    item_id: item.id.clone(),
                    manny_name: item.name.clone(),
                    error: None,
                })
            }
            Some(InventoryRow::PassiveGroup { item_type }) if item_type == "scut_relay" => {
                let item = probe.inventory.items.iter()
                    .find(|i| i.item_type == "scut_relay")
                    .ok_or_else(|| "no SCUT relay in inventory".to_string())?;
                Ok(JettisonInput::ConfirmRelay {
                    item_id: item.id.clone(),
                    error: None,
                })
            }
            Some(InventoryRow::PassiveGroup { .. }) => {
                Err("only resource stocks, mannies and SCUT relays can be jettisoned".into())
            }
            None => Err("inventory is empty".into()),
        }
    }

    pub fn update_inventory(&mut self, inv: ProbeInventory) {
        if let Some(ref mut probe) = self.probe {
            probe.inventory = inv;
        }
        self.clamp_inventory_selection();
    }

    pub fn jettison_type_char(&mut self, c: char) {
        if let JettisonInput::EnterAmount { ref mut buf, .. } = self.jettison {
            if c.is_ascii_digit() || (c == '.' && !buf.contains('.')) {
                buf.push(c);
            }
        }
    }

    pub fn jettison_backspace(&mut self) {
        if let JettisonInput::EnterAmount { ref mut buf, .. } = self.jettison {
            buf.pop();
        }
    }

    pub fn jettison_fill_max(&mut self) {
        if let JettisonInput::EnterAmount { ref mut buf, max_amount, ref mut error, .. } = self.jettison {
            *buf = format!("{max_amount:.4}");
            *error = None;
        }
    }

    pub fn has_atomic_printer(&self) -> bool {
        self.probe.as_ref()
            .map(|p| p.inventory.items.iter().any(|i| i.item_type == "atomic_3d_printer"))
            .unwrap_or(false)
    }

    pub fn set_jettison_error(&mut self, msg: String) {
        match self.jettison {
            JettisonInput::ConfirmManny { ref mut error, .. } => *error = Some(msg),
            JettisonInput::ConfirmRelay { ref mut error, .. } => *error = Some(msg),
            JettisonInput::EnterAmount { ref mut error, .. } => *error = Some(msg),
            _ => {}
        }
    }

    pub fn atomic_printer_recipes(&self) -> Vec<&CraftingRecipe> {
        self.recipes.iter()
            .filter(|r| r.craftable_by.iter().any(|c| c == "atomic_3d_printer"))
            .collect()
    }

    pub fn manny_craft_recipes(&self) -> Vec<&CraftingRecipe> {
        self.recipes.iter()
            .filter(|r| r.craftable_by.iter().any(|c| c == "manny"))
            .collect()
    }

    /// How much of a recipe ingredient the probe inventory holds: a unit count
    /// for `item` ingredients, or the resource stock amount (ECE) otherwise.
    pub fn recipe_ingredient_have(&self, ing: &CraftingRecipeIngredient) -> f64 {
        let Some(probe) = &self.probe else { return 0.0 };
        if ing.unit == "item" {
            probe.inventory.items.iter().filter(|it| it.item_type == ing.ingredient_type).count() as f64
        } else {
            probe
                .inventory
                .resource_stocks
                .iter()
                .find(|s| s.stock_type == ing.ingredient_type)
                .map_or(0.0, |s| s.amount)
        }
    }

    /// Whether every ingredient of a recipe is currently on hand.
    pub fn recipe_affordable(&self, recipe: &CraftingRecipe) -> bool {
        recipe.ingredients.iter().all(|ing| self.recipe_ingredient_have(ing) >= ing.quantity)
    }

    pub fn inventory_waypoint_bookmark_id(&self) -> Option<String> {
        self.probe.as_ref()?.inventory.items.iter()
            .find(|i| i.item_type == "waypoint_bookmark")
            .map(|i| i.id.clone())
    }
}
