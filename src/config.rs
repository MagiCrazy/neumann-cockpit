use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub base_url: String,
    pub api_key: String,
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
        match self.theme.as_deref() {
            Some("retro") => crate::app::UiTheme::Retro,
            _ => crate::app::UiTheme::Classic,
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
