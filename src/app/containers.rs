use super::*;
use crate::api::types::StorageContainerRules;
use std::collections::BTreeSet;

impl AppState {
    /// All storage containers as (id, label) pairs.
    pub fn collect_renameable_containers(&self) -> Vec<(String, String)> {
        self.storage_containers
            .iter()
            .map(|c| (c.id.clone(), c.label.clone()))
            .collect()
    }

    /// Type names selectable in the routing-rules editor: the four resource
    /// types, every item/stock type currently in inventory, and any type
    /// already referenced by the container's rules. Sorted and de-duplicated.
    pub fn routable_types(&self, rules: &StorageContainerRules) -> Vec<String> {
        let mut set: BTreeSet<String> = RESOURCE_TYPES.iter().map(|s| s.to_string()).collect();
        if let Some(probe) = &self.probe {
            for it in &probe.inventory.items {
                set.insert(it.item_type.clone());
            }
            for st in &probe.inventory.resource_stocks {
                set.insert(st.stock_type.clone());
            }
        }
        for t in rules
            .priority
            .iter()
            .chain(&rules.exclusion)
            .chain(&rules.strict_exclusion)
        {
            set.insert(t.clone());
        }
        set.into_iter().collect()
    }

    /// Build the routing-rules editor for a container (by id), seeded from its
    /// current rules. Returns `None` if the container is not in the list.
    pub fn rules_editor_for(&self, container_id: &str) -> Option<ContainerRulesInput> {
        let c = self.storage_containers.iter().find(|c| c.id == container_id)?;
        let types = self.routable_types(&c.rules);
        Some(ContainerRulesInput::Editing {
            container_id: c.id.clone(),
            container_label: c.label.clone(),
            types,
            priority: c.rules.priority.clone(),
            exclusion: c.rules.exclusion.clone(),
            strict_exclusion: c.rules.strict_exclusion.clone(),
            selection: 0,
            error: None,
        })
    }
}
