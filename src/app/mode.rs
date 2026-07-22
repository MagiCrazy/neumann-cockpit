//! Input mode state machine for the Cockpit v2 interface (bloc U1).
//!
//! `InputMode` is the top-level state the input router will dispatch on in
//! later blocs. U1 only wires the state and its default (`Normal`); the
//! `Menu`/`Command` payloads are populated by blocs U5/U6. The `Prompt`
//! variant (wrapping the existing `*Input` wizards) is added in U5.

/// An action a contextual menu item can fire. Each maps to launching one of
/// the existing wizards (bloc U5 wires the Mannies pane; more panes follow).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Mine,
    /// Open the unified fabrication catalog (atomic printer + Manny craft).
    Fabricate,
    Repair,
    Salvage,
    Inspect,
    Recover,
    Detach,
    Refuel,
    TransferDeuterium,
    TransferProbe,
    DropCargo,
    Recall,
    Rename,
    // Inventory pane
    Jettison,
    MoveStock,
    /// Deploy a waypoint bookmark from inventory onto a sector object.
    Deploy,
    // Probe pane
    MindSnapshot,
    /// Inspect the SCUT relay network covering the current sector.
    ScutInspect,
    /// Install a probe improvement with an idle Manny.
    Improve,
    /// Open the fleet picker to switch the piloted probe (API v81).
    SwitchProbe,
    /// Promote the active probe to the player's default (PATCH isDefault).
    SetDefaultProbe,
    /// Assemble a new drone probe with the selected Manny (API v81).
    AssembleProbe,
    /// Rename the piloted probe (`PATCH /api/probe/{id}` name, API v81).
    RenameProbe,
    // Mannies pane (extra)
    DropStorageContainer,
    // Storage pane
    RenameContainer,
    EditContainerRules,
    // Scanner pane
    ScanAround,
    ScanDirection,
    ScanObserve,
    ScanFilter,
    ScanTravel,
    // Map pane
    OpenMap,
    Travel,
    GotoVisited,
    Waypoints,
}

/// A single entry in a contextual action menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    pub action: MenuAction,
    pub label: String,
    pub enabled: bool,
    /// Reason shown (dimmed) when the item is disabled — teaches the rules
    /// instead of hiding the action.
    pub disabled_reason: Option<String>,
}

/// The contextual action menu opened with `Enter` on a selection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContextMenu {
    pub title: String,
    pub items: Vec<MenuItem>,
    pub cursor: usize,
}

/// The `:` command line being typed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandLine {
    pub input: String,
    /// Caret position within `input`.
    pub cursor: usize,
    /// Tab-completion cycling state. `None` until the first `Tab`; reset to
    /// `None` on any edit so the next `Tab` recomputes candidates from scratch.
    pub completion: Option<CompletionState>,
    /// History browse position: `None` while editing the live line, `Some(i)`
    /// while stepping through `AppState::command_history` with the arrow keys.
    pub history_idx: Option<usize>,
}

/// Tab-completion cycling state for the command line: the candidate list for the
/// token under the caret and which one is currently applied.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompletionState {
    /// Candidates matching the token, in stable order.
    pub candidates: Vec<String>,
    /// Index of the candidate currently written into `input`.
    pub index: usize,
    /// Byte offset in `input` where the completed token starts.
    pub token_start: usize,
}

/// Top-level interaction mode. The input router dispatches on this.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum InputMode {
    /// Grid navigation + cursor + drill-in.
    #[default]
    Normal,
    /// A contextual action menu is open.
    Menu(ContextMenu),
    /// The command line is being typed.
    Command(CommandLine),
}

impl InputMode {
    /// Short uppercase tag shown in the status bar (`NAV` / `MENU` / `CMD`).
    pub fn tag(&self) -> &'static str {
        match self {
            InputMode::Normal => "NAV",
            InputMode::Menu(_) => "MENU",
            InputMode::Command(_) => "CMD",
        }
    }

    pub fn is_text_entry(&self) -> bool {
        matches!(self, InputMode::Command(_))
    }
}

impl super::AppState {
    /// Build the contextual action menu for the active pane and selection,
    /// or `None` when the pane has no actions yet (bloc U5: Mannies only).
    pub fn build_context_menu(&self) -> Option<ContextMenu> {
        match self.active_pane {
            super::Pane::Mannies => self.mannies_context_menu(),
            super::Pane::Inventory => Some(self.inventory_context_menu()),
            super::Pane::Probe => self.probe_context_menu(),
            super::Pane::Storage => self.storage_context_menu(),
            super::Pane::Scanner => Some(self.scanner_context_menu()),
            super::Pane::Map => Some(self.map_context_menu()),
            _ => None,
        }
    }

    fn map_context_menu(&self) -> ContextMenu {
        let has_visited = !self.visited_sectors.is_empty();
        let items = vec![
            MenuItem {
                action: MenuAction::OpenMap,
                label: "Open map".into(),
                enabled: true,
                disabled_reason: None,
            },
            MenuItem {
                action: MenuAction::Travel,
                label: "Travel to coordinates…".into(),
                enabled: true,
                disabled_reason: None,
            },
            MenuItem {
                action: MenuAction::GotoVisited,
                label: "Jump to visited sector…".into(),
                enabled: has_visited,
                disabled_reason: (!has_visited).then(|| "none visited".to_string()),
            },
            {
                let has_wp = !self.collect_waypoints().is_empty();
                MenuItem {
                    action: MenuAction::Waypoints,
                    label: "Waypoints…".into(),
                    enabled: has_wp,
                    disabled_reason: (!has_wp).then(|| "no waypoints".to_string()),
                }
            },
        ];
        ContextMenu {
            title: "MAP".into(),
            items,
            cursor: 0,
        }
    }

    fn scanner_context_menu(&self) -> ContextMenu {
        // Batch scans need a known probe position to offset from.
        let has_pos = self.probe_sector_coords().is_some();
        let pos_reason = (!has_pos).then(|| "unknown position".to_string());
        // Travel to the observation under the history cursor — unless it is the
        // sector we are already in.
        let has_selection = self.current_sector().is_some();
        let here = self.viewing_probe_sector();
        let items = vec![
            MenuItem {
                action: MenuAction::ScanAround,
                label: "Scan around (neighbors)".into(),
                enabled: has_pos,
                disabled_reason: pos_reason.clone(),
            },
            MenuItem {
                action: MenuAction::ScanDirection,
                label: "Scan a direction (×2)…".into(),
                enabled: has_pos,
                disabled_reason: pos_reason,
            },
            MenuItem {
                action: MenuAction::ScanObserve,
                label: "Observe coordinates…".into(),
                enabled: true,
                disabled_reason: None,
            },
            MenuItem {
                action: MenuAction::ScanFilter,
                label: format!("Cycle filter (now: {})", self.scan_filter.label()),
                enabled: !self.scan_history.is_empty(),
                disabled_reason: self.scan_history.is_empty().then(|| "no history".to_string()),
            },
            // Travel is the terminal action — kept last, below the scan verbs.
            MenuItem {
                action: MenuAction::ScanTravel,
                label: "Travel here".into(),
                enabled: has_selection && !here,
                disabled_reason: if !has_selection {
                    Some("no sector selected".to_string())
                } else if here {
                    Some("already here".to_string())
                } else {
                    None
                },
            },
        ];
        let cursor = items.iter().position(|i| i.enabled).unwrap_or(0);
        ContextMenu {
            title: "SCANNER".into(),
            items,
            cursor,
        }
    }

    fn storage_context_menu(&self) -> Option<ContextMenu> {
        let cur = self.pane_nav[super::Pane::Storage.index()].cursor;
        let c = self.probe.as_ref()?.inventory.containers.get(cur)?;
        Some(ContextMenu {
            title: c.label.clone(),
            items: vec![
                MenuItem {
                    action: MenuAction::RenameContainer,
                    label: "Rename…".into(),
                    enabled: true,
                    disabled_reason: None,
                },
                MenuItem {
                    action: MenuAction::EditContainerRules,
                    label: "Edit routing rules…".into(),
                    enabled: true,
                    disabled_reason: None,
                },
            ],
            cursor: 0,
        })
    }

    fn probe_context_menu(&self) -> Option<ContextMenu> {
        let mut items = vec![];
        // Fleet switching (API v81) — only meaningful with more than one probe.
        let multi = self.fleet.len() > 1;
        items.push(MenuItem {
            action: MenuAction::SwitchProbe,
            label: "Switch probe…".into(),
            enabled: multi,
            disabled_reason: (!multi).then(|| "single probe".to_string()),
        });
        // Promote the active probe to default — only when it isn't already, and
        // only when it is reachable (the server refuses an out-of-reach target).
        if let Some(active) = self.active_probe_summary() {
            if !active.is_default {
                items.push(MenuItem {
                    action: MenuAction::SetDefaultProbe,
                    label: "Set as default probe".into(),
                    enabled: active.is_reachable,
                    disabled_reason: (!active.is_reachable).then(|| "out of SCUT range".to_string()),
                });
            }
        }
        // Rename the piloted probe — available whenever we know its id.
        let can_rename = self.active_probe_identity().is_some();
        items.push(MenuItem {
            action: MenuAction::RenameProbe,
            label: "Rename probe…".into(),
            enabled: can_rename,
            disabled_reason: (!can_rename).then(|| "no probe".to_string()),
        });
        let has_scut = !self.scut_coverage().is_empty();
        items.push(MenuItem {
            action: MenuAction::ScutInspect,
            label: "Inspect SCUT network…".into(),
            enabled: has_scut,
            disabled_reason: (!has_scut).then(|| "no SCUT network here".to_string()),
        });
        // Probe improvements — installable when one is unlocked and not done.
        let can_improve = self.has_orderable_improvement();
        items.push(MenuItem {
            action: MenuAction::Improve,
            label: "Improve probe…".into(),
            enabled: can_improve,
            disabled_reason: (!can_improve).then(|| "no improvement available".to_string()),
        });
        // Mind-snapshot recovery only applies to a dead/trapped probe.
        if self.probe_terminal_alert().is_some() {
            items.push(MenuItem {
                action: MenuAction::MindSnapshot,
                label: "Reassign mind snapshot".into(),
                enabled: true,
                disabled_reason: None,
            });
        }
        let cursor = items.iter().position(|i| i.enabled).unwrap_or(0);
        Some(ContextMenu {
            title: "PROBE".into(),
            items,
            cursor,
        })
    }

    fn inventory_context_menu(&self) -> ContextMenu {
        let has_row = self.selected_inventory_row().is_some();
        let has_recipes = !self.recipes.is_empty();
        let idle_manny = !self.collect_idle_onboard_mannies().is_empty();
        let mut items = vec![
            MenuItem {
                action: MenuAction::Fabricate,
                label: "Fabricate…".into(),
                enabled: has_recipes,
                disabled_reason: (!has_recipes).then(|| "recipes loading".to_string()),
            },
            MenuItem {
                action: MenuAction::MoveStock,
                label: "Move stock…".into(),
                enabled: idle_manny,
                disabled_reason: (!idle_manny).then(|| "no idle manny".to_string()),
            },
            MenuItem {
                action: MenuAction::Jettison,
                label: "Jettison…".into(),
                enabled: has_row,
                disabled_reason: (!has_row).then(|| "no item selected".to_string()),
            },
        ];
        // Deploy waypoint — only when a bookmark is actually held; it needs an
        // idle Manny to install it and a target object in the current sector.
        if self.inventory_waypoint_bookmark_id().is_some() {
            let has_target = !self.collect_deploy_candidates().is_empty();
            items.push(MenuItem {
                action: MenuAction::Deploy,
                label: "Deploy waypoint…".into(),
                enabled: idle_manny && has_target,
                disabled_reason: if !idle_manny {
                    Some("no idle manny".to_string())
                } else if !has_target {
                    Some("no target in sector".to_string())
                } else {
                    None
                },
            });
        }
        let cursor = items.iter().position(|i| i.enabled).unwrap_or(0);
        ContextMenu {
            title: "INVENTORY".into(),
            items,
            cursor,
        }
    }

    fn mannies_context_menu(&self) -> Option<ContextMenu> {
        use crate::api::types::{MannyTask, MannyTaskVisibility};
        let manny = self.mannies.as_ref()?.get(self.mannies_selection)?;
        let can = manny.can_receive_orders;
        let busy = (!can).then(|| "busy".to_string());
        let has_task = manny.current_task.is_some();
        let remote = matches!(manny.task_visibility, Some(MannyTaskVisibility::ScutNetwork));
        let remote_minable = self.manny_remote_minable(manny);
        let waiting_space = manny.current_task == Some(MannyTask::WaitingForSpace);
        let has_station = self.deuterium_station_in_current_sector();

        // `orders`: an action needs the Manny to be idle/orderable, with a
        // shared "busy" reason when it isn't.
        let orders = |action, label: &str| MenuItem {
            action,
            label: label.into(),
            enabled: can,
            disabled_reason: busy.clone(),
        };

        let items = vec![
            MenuItem {
                action: MenuAction::Mine,
                label: "Mine…".into(),
                enabled: can || remote_minable,
                disabled_reason: (!can && !remote_minable).then(|| "busy".to_string()),
            },
            orders(MenuAction::Fabricate, "Fabricate…"),
            orders(MenuAction::Repair, "Repair"),
            orders(MenuAction::Salvage, "Salvage…"),
            orders(MenuAction::Inspect, "Inspect…"),
            orders(MenuAction::Recover, "Recover container…"),
            orders(MenuAction::Detach, "Detach container…"),
            {
                let has_kit = self.has_atmospheric_drop_kit();
                let has_container = !self.collect_detachable_containers().is_empty();
                let has_planet = !self.collect_planet_candidates().is_empty();
                MenuItem {
                    action: MenuAction::DropStorageContainer,
                    label: "Drop container on planet…".into(),
                    enabled: can && has_kit && has_container && has_planet,
                    disabled_reason: if !can {
                        Some("busy".to_string())
                    } else if !has_kit {
                        Some("no drop-kit".to_string())
                    } else if !has_container {
                        Some("no container".to_string())
                    } else if !has_planet {
                        Some("no planet".to_string())
                    } else {
                        None
                    },
                }
            },
            MenuItem {
                action: MenuAction::Refuel,
                label: "Refill deuterium".into(),
                enabled: can && has_station,
                disabled_reason: if !can {
                    Some("busy".to_string())
                } else {
                    (!has_station).then(|| "no station".to_string())
                },
            },
            {
                let has_targets = !self.other_fleet_probes().is_empty();
                MenuItem {
                    action: MenuAction::TransferDeuterium,
                    label: "Transfer deuterium…".into(),
                    enabled: can && has_targets,
                    disabled_reason: if !can {
                        busy.clone()
                    } else {
                        (!has_targets).then(|| "no other probe".to_string())
                    },
                }
            },
            {
                let has_targets = !self.other_fleet_probes().is_empty();
                MenuItem {
                    action: MenuAction::TransferProbe,
                    label: "Transfer Manny…".into(),
                    enabled: can && has_targets,
                    disabled_reason: if !can {
                        busy.clone()
                    } else {
                        (!has_targets).then(|| "no other probe".to_string())
                    },
                }
            },
            MenuItem {
                action: MenuAction::DropCargo,
                label: "Drop cargo".into(),
                enabled: waiting_space,
                disabled_reason: (!waiting_space).then(|| "not waiting".to_string()),
            },
            {
                // Assemble a drone (API v81): needs an orderable Manny and two
                // empty additional containers to consume.
                let empties = self.collect_empty_containers().len();
                MenuItem {
                    action: MenuAction::AssembleProbe,
                    label: "Assemble probe…".into(),
                    enabled: can && empties >= 2,
                    disabled_reason: if !can {
                        Some("busy".to_string())
                    } else {
                        (empties < 2).then(|| "need 2 empty containers".to_string())
                    },
                }
            },
            MenuItem {
                action: MenuAction::Recall,
                label: if remote { "Abandon".into() } else { "Recall".into() },
                enabled: !can && has_task,
                disabled_reason: (can || !has_task).then(|| "idle".to_string()),
            },
            MenuItem {
                action: MenuAction::Rename,
                label: "Rename…".into(),
                enabled: true,
                disabled_reason: None,
            },
        ];
        // Start the cursor on the first enabled item when there is one.
        let cursor = items.iter().position(|i| i.enabled).unwrap_or(0);
        Some(ContextMenu {
            title: manny.name.clone(),
            items,
            cursor,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_is_normal() {
        assert_eq!(InputMode::default(), InputMode::Normal);
        assert_eq!(InputMode::default().tag(), "NAV");
        assert!(!InputMode::default().is_text_entry());
    }

    #[test]
    fn command_mode_is_text_entry() {
        let m = InputMode::Command(CommandLine::default());
        assert!(m.is_text_entry());
        assert_eq!(m.tag(), "CMD");
    }
}
