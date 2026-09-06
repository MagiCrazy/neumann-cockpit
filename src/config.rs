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
    /// Emit a desktop notification (OSC 9 + terminal bell) when a long task
    /// finishes — travel arrival, a Manny completing a long task. Set `false`
    /// to stay silent (issue #203).
    #[serde(default = "default_true")]
    pub notifications: bool,
    /// Terminal ground: `"auto"` (default) asks the terminal at boot via
    /// OSC 11, `"dark"` / `"light"` decide for it. `F3` overrides at runtime
    /// either way (issue #233).
    #[serde(default)]
    pub polarity: Option<String>,
    /// Diagnostic log verbosity: `"off"`, `"error"` (default), `"info"` or
    /// `"debug"` (issue #309). `NEUMANN_COCKPIT_LOG` overrides it for a
    /// one-off debugging run.
    #[serde(default)]
    pub log: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Config {
    /// Cockpit color mode, read from the `theme` key. An unknown label falls
    /// back to the default rather than erroring: the load contract is tolerant,
    /// and a typo should not keep the pilot out of the cockpit.
    pub fn color_mode(&self) -> crate::app::ColorMode {
        self.theme
            .as_deref()
            .and_then(crate::app::ColorMode::from_label)
            .unwrap_or_default()
    }

    /// Diagnostic log level: the `NEUMANN_COCKPIT_LOG` environment variable
    /// first — so a pilot can raise it for one run without editing a file —
    /// then the `log` key, then the default. An unrecognised value is ignored
    /// rather than fatal, like every other key.
    pub fn log_level(&self) -> crate::diaglog::Level {
        std::env::var(crate::diaglog::LEVEL_ENV)
            .ok()
            .as_deref()
            .and_then(crate::diaglog::Level::from_label)
            .or_else(|| self.log.as_deref().and_then(crate::diaglog::Level::from_label))
            .unwrap_or_default()
    }

    /// What the `polarity` key asks for; `auto` when absent or unrecognised.
    pub fn polarity_pref(&self) -> crate::app::PolarityPref {
        self.polarity
            .as_deref()
            .and_then(crate::app::PolarityPref::from_label)
            .unwrap_or_default()
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
    polarity: Option<String>,
    log: Option<String>,
    hints: Option<bool>,
    boot: Option<bool>,
    notifications: Option<bool>,
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
pub(crate) fn load_status_at(path: &std::path::Path) -> ConfigStatus {
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
        polarity: raw.polarity,
        log: raw.log,
        hints: raw.hints.unwrap_or(true),
        boot: raw.boot.unwrap_or(true),
        notifications: raw.notifications.unwrap_or(true),
    })
}

/// Write a minimal `config.toml` (base URL + API key), creating the config
/// directory if needed. Returns the path written, for the boot log.
pub fn write_config(base_url: &str, api_key: &str) -> Result<PathBuf> {
    write_config_at(&config_path(), base_url, api_key)?;
    Ok(config_path())
}

/// Path-injectable core of `write_config`.
pub(crate) fn write_config_at(path: &std::path::Path, base_url: &str, api_key: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("creating config dir {}", parent.display()))?;
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
        .map(|d| d.state_dir().unwrap_or_else(|| d.data_local_dir()).join("cockpit.db"))
        .unwrap_or_else(|| PathBuf::from("cockpit.db"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(theme: Option<&str>) -> Config {
        Config {
            polarity: None,
            log: None,
            base_url: "x".into(),
            api_key: "x".into(),
            theme: theme.map(String::from),
            hints: true,
            boot: true,
            notifications: true,
        }
    }

    #[test]
    fn color_mode_parses_from_theme() {
        use crate::app::ColorMode;
        assert_eq!(cfg(Some("mono-amber")).color_mode(), ColorMode::MonoAmber);
        assert_eq!(cfg(Some("phosphor-semantic")).color_mode(), ColorMode::PhosphorSemantic);
        assert_eq!(cfg(Some("modern-16")).color_mode(), ColorMode::Modern16);
        assert_eq!(cfg(Some("culture")).color_mode(), ColorMode::Culture);
        assert_eq!(cfg(Some("deep-space")).color_mode(), ColorMode::DeepSpace);
        assert_eq!(cfg(Some("rust-belt")).color_mode(), ColorMode::RustBelt);
        // Unknown/absent → default mono-green.
        assert_eq!(cfg(None).color_mode(), ColorMode::MonoGreen);
        assert_eq!(cfg(Some("bogus")).color_mode(), ColorMode::MonoGreen);
    }

    #[test]
    fn every_color_mode_round_trips_through_its_label() {
        use crate::app::ColorMode;
        // Config label ⇄ mode, for every mode: a new variant that forgets its
        // `from_label` arm would be selectable by F2 but not by config.
        for mode in ColorMode::ALL {
            assert_eq!(cfg(Some(mode.label())).color_mode(), mode, "{}", mode.label());
        }
    }

    #[test]
    fn polarity_key_parses_and_defaults_to_auto() {
        use crate::app::{Polarity, PolarityPref};
        let with = |p: Option<&str>| Config {
            polarity: p.map(String::from),
            ..cfg(None)
        };
        assert_eq!(with(None).polarity_pref(), PolarityPref::Auto);
        assert_eq!(with(Some("auto")).polarity_pref(), PolarityPref::Auto);
        assert_eq!(with(Some("dark")).polarity_pref(), PolarityPref::Forced(Polarity::Dark));
        assert_eq!(
            with(Some("light")).polarity_pref(),
            PolarityPref::Forced(Polarity::Light)
        );
        // A typo must not keep the pilot out of the cockpit.
        assert_eq!(with(Some("bogus")).polarity_pref(), PolarityPref::Auto);
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
        assert!(
            matches!(load_status_at(&path), ConfigStatus::NeedsKey),
            "example key is not real"
        );
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
