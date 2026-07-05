use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::Deserialize;
use std::path::PathBuf;

/// The production probe server, pre-filled when onboarding writes a fresh config
/// so the pilot only ever has to paste an API key.
pub const DEFAULT_BASE_URL: &str = "https://neumann-probe.net";

/// The placeholder key shipped in `config.example.toml`; treated as "no key yet"
/// so a copied-but-unedited example still triggers onboarding.
const PLACEHOLDER_KEY: &str = "vng_your_api_key_here";

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

/// A lenient view of `config.toml` where every key is optional, so a file that
/// exists but lacks an API key parses cleanly (and drives onboarding) instead of
/// erroring out. Unknown keys are ignored, matching the tolerant load contract.
#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    base_url: Option<String>,
    api_key: Option<String>,
    theme: Option<String>,
    hints: Option<bool>,
    boot: Option<bool>,
}

/// The outcome of inspecting the on-disk config at boot.
pub enum ConfigStatus {
    /// A usable config with a real API key.
    Ready(Config),
    /// No file, or the file has no usable key yet — onboarding should collect one.
    NeedsKey,
    /// The file exists but is not valid TOML — surfaced so the pilot can fix it.
    Invalid(String),
}

impl Config {
    /// Inspect `config.toml` without failing on a missing/keyless file. This is
    /// the boot entry point: it never returns an error the caller must print to
    /// a vanishing console — every case maps to an in-TUI outcome.
    pub fn load_status() -> ConfigStatus {
        load_status_at(&config_path())
    }
}

/// Path-injectable core of `Config::load_status`, so tests never touch the real
/// user config.
fn load_status_at(path: &std::path::Path) -> ConfigStatus {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return ConfigStatus::NeedsKey,
    };
    let raw: RawConfig = match toml::from_str(&content) {
        Ok(r) => r,
        Err(e) => return ConfigStatus::Invalid(e.to_string()),
    };
    let key = raw.api_key.unwrap_or_default();
    if key.trim().is_empty() || key == PLACEHOLDER_KEY {
        return ConfigStatus::NeedsKey;
    }
    ConfigStatus::Ready(Config {
        base_url: raw.base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
        api_key: key,
        theme: raw.theme,
        hints: raw.hints.unwrap_or(true),
        boot: raw.boot.unwrap_or(true),
    })
}

/// Write a minimal `config.toml` (base URL + API key), creating the config
/// directory if needed. Returns the path written, for the boot log.
pub fn write_config(base_url: &str, api_key: &str) -> Result<PathBuf> {
    write_config_at(&config_path(), base_url, api_key)?;
    Ok(config_path())
}

/// Path-injectable core of `write_config`.
fn write_config_at(path: &std::path::Path, base_url: &str, api_key: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating config dir {}", parent.display()))?;
    }
    // `{:?}` emits a double-quoted, backslash-escaped string — valid TOML basic
    // string syntax, and API keys / URLs never contain anything exotic.
    let body = format!("base_url = {base_url:?}\napi_key  = {api_key:?}\n");
    std::fs::write(path, body).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
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

    fn tmp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("nc_cfg_test_{name}.toml"))
    }

    #[test]
    fn write_then_load_round_trips_with_default_base_url() {
        let path = tmp("roundtrip");
        let _ = std::fs::remove_file(&path);
        write_config_at(&path, DEFAULT_BASE_URL, "vng_realkey123").unwrap();
        match load_status_at(&path) {
            ConfigStatus::Ready(c) => {
                assert_eq!(c.api_key, "vng_realkey123");
                assert_eq!(c.base_url, DEFAULT_BASE_URL);
                assert!(c.hints && c.boot, "defaults applied");
            }
            _ => panic!("a freshly written key must load as Ready"),
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_file_needs_key() {
        let path = tmp("missing");
        let _ = std::fs::remove_file(&path);
        assert!(matches!(load_status_at(&path), ConfigStatus::NeedsKey));
    }

    #[test]
    fn placeholder_and_empty_key_need_key() {
        let path = tmp("placeholder");
        std::fs::write(&path, format!("api_key = {PLACEHOLDER_KEY:?}\n")).unwrap();
        assert!(matches!(load_status_at(&path), ConfigStatus::NeedsKey), "example key is not real");
        std::fs::write(&path, "api_key = \"\"\n").unwrap();
        assert!(matches!(load_status_at(&path), ConfigStatus::NeedsKey), "empty key");
        std::fs::write(&path, "base_url = \"https://x\"\n").unwrap();
        assert!(matches!(load_status_at(&path), ConfigStatus::NeedsKey), "no key at all");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn malformed_toml_is_invalid() {
        let path = tmp("malformed");
        std::fs::write(&path, "this is = = not toml\n").unwrap();
        assert!(matches!(load_status_at(&path), ConfigStatus::Invalid(_)));
        let _ = std::fs::remove_file(&path);
    }
}
