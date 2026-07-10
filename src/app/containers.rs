use super::*;
use crate::api::types::{SectorObjectType, StorageContainerRules};
use std::collections::BTreeSet;

impl AppState {
    /// Planets in the probe's current sector, as (id, name) pairs — drop-target
    /// candidates for dropping a storage container.
    pub fn collect_planet_candidates(&self) -> Vec<(String, String)> {
        self.probe_current_sector_scan()
            .and_then(|s| s.objects.as_ref())
            .map(|objects| {
                objects
                    .iter()
                    .filter(|o| matches!(o.object_type, SectorObjectType::Planet) && o.id.is_some())
                    .map(|o| {
                        let id = o.id.clone().unwrap();
                        let name = o.name.clone().unwrap_or_else(|| "unnamed planet".into());
                        (id, name)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Whether the probe inventory holds an atmospheric drop kit (required to
    /// drop a container on a planet).
    pub fn has_atmospheric_drop_kit(&self) -> bool {
        self.probe
            .as_ref()
            .map(|p| {
                p.inventory
                    .items
                    .iter()
                    .any(|it| it.item_type == "atmospheric_drop_kit")
            })
            .unwrap_or(false)
    }

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

    /// Containers available as move source/destination, from the probe
    /// inventory (always loaded), as (id, label) pairs.
    pub fn collect_move_containers(&self) -> Vec<(String, String)> {
        match &self.probe {
            Some(p) => p
                .inventory
                .containers
                .iter()
                .map(|c| (c.id.clone(), c.label.clone()))
                .collect(),
            None => Vec::new(),
        }
    }

    /// Unit items movable between containers (excludes mannies, which use the
    /// dedicated `manny` move kind). Label shows the current container.
    pub fn collect_movable_items(&self) -> Vec<(String, String)> {
        let Some(p) = &self.probe else { return Vec::new() };
        p.inventory
            .items
            .iter()
            .filter(|it| it.item_type != "manny")
            .map(|it| {
                let loc = it
                    .container
                    .as_ref()
                    .map(|c| c.label.clone())
                    .unwrap_or_else(|| "—".to_string());
                (it.id.clone(), format!("{} [{}]", it.name, loc))
            })
            .collect()
    }

    /// Build the routing-rules editor for a container (by id), seeded from its
    /// current rules. Returns `None` if the container is not in the list.
    /// The storage container with this id, from the fetched list or the
    /// probe's inventory (the latter is available as soon as the probe loads).
    pub fn storage_container(&self, id: &str) -> Option<&crate::api::types::StorageContainer> {
        self.storage_containers
            .iter()
            .chain(self.probe.iter().flat_map(|p| p.inventory.containers.iter()))
            .find(|c| c.id == id)
    }

    /// Id of the container the Storage pane cursor is on (from the probe's
    /// inventory, which the pane renders).
    pub fn storage_selected_container_id(&self) -> Option<String> {
        let cur = self.pane_nav[crate::app::Pane::Storage.index()].cursor;
        self.probe.as_ref()?.inventory.containers.get(cur).map(|c| c.id.clone())
    }

    pub fn rules_editor_for(&self, container_id: &str) -> Option<ContainerRulesInput> {
        let c = self.storage_container(container_id)?;
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
