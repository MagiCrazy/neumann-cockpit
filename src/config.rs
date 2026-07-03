use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub base_url: String,
    pub api_key: String,
    /// Cockpit color mode: "mono-green" (default), "mono-amber",
    /// "phosphor-semantic", or "modern-16". F2 cycles it at runtime.
    #[serde(default)]
    pub theme: Option<String>,
    /// Show the contextual hints line in the cockpit interface (F1 toggles).
    #[serde(default = "default_true")]
    pub hints: bool,
    /// Play the boot self-check animation on startup. Set `false` to drop
    /// straight into the live cockpit (handy over tmux/ssh or on relaunch).
    #[serde(default = "default_true")]
    pub boot: bool,
}

fn default_true() -> bool {
    true
}

impl Config {
    /// Cockpit color mode, read from the `theme` key.
    pub fn color_mode(&self) -> crate::app::ColorMode {
        use crate::app::ColorMode;
        match self.theme.as_deref() {
            Some("mono-amber") => ColorMode::MonoAmber,
            Some("phosphor-semantic") => ColorMode::PhosphorSemantic,
            Some("modern-16") => ColorMode::Modern16,
            _ => ColorMode::MonoGreen,
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

/// Path to the local SQLite database (scan history today; action audit later).
///
/// This is mutable state, not configuration, so it lives in the XDG state dir
/// (`~/.local/state/…`) rather than `~/.config`. State dir is Linux-only in the
/// spec, so fall back to the data dir elsewhere. The legacy `scan_history.json`
/// stays under `config_dir` (see `history_path`) purely as a one-time import
/// source.
pub fn db_path() -> PathBuf {
    ProjectDirs::from("net", "neumann", "neumann-cockpit")
        .map(|d| {
            d.state_dir()
                .unwrap_or_else(|| d.data_local_dir())
                .join("cockpit.db")
        })
        .unwrap_or_else(|| PathBuf::from("cockpit.db"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(theme: Option<&str>) -> Config {
        Config {
            base_url: "x".into(),
            api_key: "x".into(),
            theme: theme.map(String::from),
            hints: true,
            boot: true,
        }
    }

    #[test]
    fn color_mode_parses_from_theme() {
        use crate::app::ColorMode;
        assert_eq!(cfg(Some("mono-amber")).color_mode(), ColorMode::MonoAmber);
        assert_eq!(cfg(Some("phosphor-semantic")).color_mode(), ColorMode::PhosphorSemantic);
        assert_eq!(cfg(Some("modern-16")).color_mode(), ColorMode::Modern16);
        // Unknown/absent → default mono-green.
        assert_eq!(cfg(None).color_mode(), ColorMode::MonoGreen);
        assert_eq!(cfg(Some("bogus")).color_mode(), ColorMode::MonoGreen);
    }

    #[test]
    fn color_mode_cycles_through_all_four() {
        use crate::app::ColorMode;
        let m = ColorMode::MonoGreen;
        let m = m.cycle();
        assert_eq!(m, ColorMode::MonoAmber);
        let m = m.cycle().cycle();
        assert_eq!(m, ColorMode::Modern16);
        assert_eq!(m.cycle(), ColorMode::MonoGreen);
    }
}
