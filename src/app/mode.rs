//! Input mode state machine for the Cockpit v2 interface (bloc U1).
//!
//! `InputMode` is the top-level state the input router will dispatch on in
//! later blocs. U1 only wires the state and its default (`Normal`); the
//! `Menu`/`Command` payloads are populated by blocs U5/U6. The `Prompt`
//! variant (wrapping the existing `*Input` wizards) is added in U5.
#![allow(dead_code)]

/// An action a contextual menu item can fire. Each maps to launching one of
/// the existing wizards (bloc U5 wires the Mannies pane; more panes follow).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Mine,
    Craft,
    Repair,
    Salvage,
    Inspect,
    Recover,
    Detach,
    Refuel,
    DropCargo,
    Recall,
    Rename,
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
            _ => None,
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
            orders(MenuAction::Craft, "Craft…"),
            orders(MenuAction::Repair, "Repair"),
            orders(MenuAction::Salvage, "Salvage…"),
            orders(MenuAction::Inspect, "Inspect…"),
            orders(MenuAction::Recover, "Recover container…"),
            orders(MenuAction::Detach, "Detach container…"),
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
            MenuItem {
                action: MenuAction::DropCargo,
                label: "Drop cargo".into(),
                enabled: waiting_space,
                disabled_reason: (!waiting_space).then(|| "not waiting".to_string()),
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
