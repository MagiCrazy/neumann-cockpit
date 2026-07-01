use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub base_url: String,
    pub api_key: String,
    /// Layout: "classic" (default), "retro", or "cockpit" (unified preview,
    /// bloc U-series). Takes precedence over `theme` when set.
    #[serde(default)]
    pub ui: Option<String>,
    /// UI theme: "classic" (default) or "retro".
    #[serde(default)]
    pub theme: Option<String>,
    /// Retro phosphor tint: "green" (default) or "amber".
    #[serde(default)]
    pub phosphor: Option<String>,
    /// Retro idle animations (render tick only, never API polling).
    #[serde(default = "default_true")]
    pub animations: bool,
}

fn default_true() -> bool {
    true
}

impl Config {
    pub fn ui_theme(&self) -> crate::app::UiTheme {
        use crate::app::UiTheme;
        // `ui` (layout) wins over the legacy `theme` key when set.
        match self.ui.as_deref() {
            Some("cockpit") => return UiTheme::Cockpit,
            Some("retro") => return UiTheme::Retro,
            Some("classic") => return UiTheme::Classic,
            _ => {}
        }
        match self.theme.as_deref() {
            Some("retro") => UiTheme::Retro,
            _ => UiTheme::Classic,
        }
    }

    pub fn ui_phosphor(&self) -> crate::app::Phosphor {
        match self.phosphor.as_deref() {
            Some("amber") => crate::app::Phosphor::Amber,
            _ => crate::app::Phosphor::Green,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path();
        let content = std::fs::read_to_string(&path).with_context(|| {
            format!(
                "Config not found: {}\n\nCreate it:\n  mkdir -p {}\n  cp config.example.toml {}",
                path.display(),
                path.parent().unwrap_or(&path).display(),
                path.display()
            )
        })?;
        toml::from_str(&content).context("Invalid config.toml")
    }
}

pub fn config_path() -> PathBuf {
    ProjectDirs::from("net", "neumann", "neumann-cockpit")
        .map(|d| d.config_dir().join("config.toml"))
        .unwrap_or_else(|| PathBuf::from("config.toml"))
}

pub fn history_path() -> PathBuf {
    ProjectDirs::from("net", "neumann", "neumann-cockpit")
        .map(|d| d.config_dir().join("scan_history.json"))
        .unwrap_or_else(|| PathBuf::from("scan_history.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::UiTheme;

    fn cfg(ui: Option<&str>, theme: Option<&str>) -> Config {
        Config {
            base_url: "x".into(),
            api_key: "x".into(),
            ui: ui.map(String::from),
            theme: theme.map(String::from),
            phosphor: None,
            animations: true,
        }
    }

    #[test]
    fn ui_key_wins_over_theme() {
        assert_eq!(cfg(Some("cockpit"), Some("retro")).ui_theme(), UiTheme::Cockpit);
        assert_eq!(cfg(Some("retro"), None).ui_theme(), UiTheme::Retro);
        assert_eq!(cfg(Some("classic"), Some("retro")).ui_theme(), UiTheme::Classic);
    }

    #[test]
    fn falls_back_to_theme_when_ui_absent() {
        assert_eq!(cfg(None, Some("retro")).ui_theme(), UiTheme::Retro);
        assert_eq!(cfg(None, None).ui_theme(), UiTheme::Classic);
        assert_eq!(cfg(None, Some("bogus")).ui_theme(), UiTheme::Classic);
    }
}
