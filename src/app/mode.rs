//! Input mode state machine for the Cockpit v2 interface (bloc U1).
//!
//! `InputMode` is the top-level state the input router will dispatch on in
//! later blocs. U1 only wires the state and its default (`Normal`); the
//! `Menu`/`Command` payloads are populated by blocs U5/U6. The `Prompt`
//! variant (wrapping the existing `*Input` wizards) is added in U5.
#![allow(dead_code)]

/// A single entry in a contextual action menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
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
