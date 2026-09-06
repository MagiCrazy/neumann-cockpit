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
    /// Whether to ask GitHub for the latest release at boot (issue #339).
    /// Absent means **not yet asked**: the first run asks once and writes the
    /// answer, because this is the only request the cockpit makes outside
    /// `base_url`.
    #[serde(default)]
    pub update_check: Option<bool>,
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

    /// What the pilot decided about the release check. Absent is `Unset`, not
    /// `false`: the difference is whether they have been asked.
    pub fn update_pref(&self) -> crate::update::UpdatePref {
        match self.update_check {
            None => crate::update::UpdatePref::Unset,
            Some(true) => crate::update::UpdatePref::Enabled,
            Some(false) => crate::update::UpdatePref::Disabled,
        }
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
    update_check: Option<bool>,
    hints: Option<bool>,
    boot: Option<bool>,
    notifications: Option<bool>,
}

/// The outcome of inspecting the on-disk config at boot.
#[derive(Debug)]
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
        update_check: raw.update_check,
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
    std::fs::write(path, generated_body(base_url, api_key)).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Write the optional keys as **commented defaults** alongside the two the
/// pilot must supply (issue #331).
///
/// First-run onboarding used to write exactly two lines, so a pilot who never
/// read the docs never learned `theme`, `hints`, `boot`, `notifications`,
/// `polarity` and `log` existed at all. That is a discoverability failure, not
/// a missing feature: the keys were always supported.
fn generated_body(base_url: &str, api_key: &str) -> String {
    // `{:?}` emits a double-quoted, backslash-escaped string — valid TOML basic
    // string syntax, and API keys / URLs never contain anything exotic.
    format!(
        "base_url = {base_url:?}\napi_key  = {api_key:?}\n\n\
         # Everything below is optional; the value shown is the default.\n\
         # Uncomment to change it, or press the key at runtime — the cockpit\n\
         # writes your choice back here, keeping your own comments.\n\
         #theme = \"mono-green\"      # F2 · mono-green mono-amber phosphor-semantic modern-16 culture deep-space rust-belt\n\
         #polarity = \"auto\"         # F3 · auto dark light\n\
         #hints = true              # F1 · the contextual hints line\n\
         #boot = true               # the startup self-check animation\n\
         #notifications = true      # desktop notification on a long task finishing\n\
         #log = \"error\"            # diagnostics: off error info debug\n\
         #update_check = false      # ask GitHub for the latest release at boot\n"
    )
}

/// Runtime settings the cockpit writes back when the pilot toggles them
/// (issue #331). A toggle that is forgotten on the next launch reads as a
/// setting that does not exist.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub theme: String,
    pub polarity: String,
    pub hints: bool,
    pub notifications: bool,
}

/// Record the pilot's answer about the release check, without touching
/// anything else in the file (issue #339). Separate from [`Settings`] because
/// it is asked once rather than toggled.
pub fn save_update_pref_at(path: &std::path::Path, enabled: bool) -> Result<()> {
    let body = std::fs::read_to_string(path).unwrap_or_default();
    let body = upsert_key(&body, "update_check", &enabled.to_string());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("creating config dir {}", parent.display()))?;
    }
    std::fs::write(path, body).with_context(|| format!("writing {}", path.display()))
}

/// Rewrite `path` so it carries `settings`, **preserving everything else**.
///
/// Deliberately an edit, not a regeneration: the file may be hand-written, with
/// comments and keys this build has never heard of. Regenerating from the
/// struct would silently eat both, and a cockpit that destroys your config the
/// first time you press F2 is worse than one that forgets your theme.
///
/// So each key is rewritten in place if present — commented or not — and
/// appended only when it is absent entirely.
pub fn save_settings_at(path: &std::path::Path, settings: &Settings) -> Result<()> {
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let mut body = existing;
    for (key, value) in [
        ("theme", format!("{:?}", settings.theme)),
        ("polarity", format!("{:?}", settings.polarity)),
        ("hints", settings.hints.to_string()),
        ("notifications", settings.notifications.to_string()),
    ] {
        body = upsert_key(&body, key, &value);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("creating config dir {}", parent.display()))?;
    }
    std::fs::write(path, body).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Set `key = value` in a TOML body, replacing the first line that assigns it —
/// **including a commented one**, so the documented default in a generated file
/// becomes the live setting rather than being shadowed by a duplicate below it.
/// Any trailing comment on that line is kept.
fn upsert_key(body: &str, key: &str, value: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut done = false;
    for line in body.lines() {
        let bare = line.trim_start().trim_start_matches('#').trim_start();
        let assigns = bare.split_once('=').is_some_and(|(name, _)| name.trim() == key);
        if assigns && !done {
            // Look for the trailing comment *after* the assignment, not from
            // the start of the line: on a commented default the first `#` is
            // the one commenting the key out, and taking it would swallow the
            // whole line instead of keeping its hint.
            let value_start = line.len() - bare.len();
            let trailing = bare
                .find('=')
                .and_then(|eq| bare[eq..].find('#').map(|i| &line[value_start + eq + i..]));
            out.push(match trailing {
                Some(comment) => format!("{key} = {value}  {comment}"),
                None => format!("{key} = {value}"),
            });
            done = true;
        } else {
            out.push(line.to_string());
        }
    }
    if !done {
        if !out.is_empty() && !out.last().is_some_and(|l| l.trim().is_empty()) {
            out.push(String::new());
        }
        out.push(format!("{key} = {value}"));
    }
    let mut body = out.join("\n");
    body.push('\n');
    body
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
            update_check: None,
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

    // ── Settings write-back (issue #331) ──────────────────────────────────

    fn settings(theme: &str, hints: bool) -> Settings {
        Settings {
            theme: theme.into(),
            polarity: "dark".into(),
            hints,
            notifications: true,
        }
    }

    #[test]
    fn saving_settings_keeps_hand_written_comments_and_unknown_keys() {
        // The reason this is an edit and not a regeneration: a cockpit that
        // eats your config the first time you press F2 is worse than one that
        // forgets your theme.
        let path = tmp("preserve");
        std::fs::write(
            &path,
            "# my own note, hands off\n\
             base_url = \"https://example.test\"\n\
             api_key  = \"vng_mine\"\n\
             future_key = 42  # a key this build has never heard of\n",
        )
        .unwrap();

        save_settings_at(&path, &settings("rust-belt", false)).unwrap();
        let body = std::fs::read_to_string(&path).unwrap();

        assert!(body.contains("# my own note, hands off"), "comment survived: {body}");
        assert!(body.contains("future_key = 42"), "unknown key survived: {body}");
        assert!(body.contains("api_key  = \"vng_mine\""), "the key is untouched: {body}");
        assert!(body.contains("theme = \"rust-belt\""), "and the setting landed: {body}");
        assert!(body.contains("hints = false"));

        // And it round-trips: the file we wrote is the file we read back.
        match load_status_at(&path) {
            ConfigStatus::Ready(c) => {
                assert_eq!(c.color_mode(), crate::app::ColorMode::RustBelt);
                assert!(!c.hints);
                assert_eq!(c.base_url, "https://example.test");
            }
            other => panic!("the written file must still parse: {other:?}"),
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn saving_settings_activates_a_commented_default_instead_of_duplicating_it() {
        // The generated file documents every key commented out. Appending a
        // second `theme =` below would leave the pilot staring at two, one of
        // which does nothing.
        let path = tmp("uncomment");
        let _ = std::fs::remove_file(&path);
        write_config_at(&path, DEFAULT_BASE_URL, "vng_k").unwrap();
        let generated = std::fs::read_to_string(&path).unwrap();
        assert!(generated.contains("#theme ="), "the default is documented: {generated}");

        save_settings_at(&path, &settings("culture", true)).unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        assert_eq!(body.matches("theme =").count(), 1, "exactly one theme line: {body}");
        assert!(body.contains("theme = \"culture\""), "{body}");
        assert!(
            !body.contains("#theme ="),
            "the commented one became the live one: {body}"
        );
        // The explanatory trailing comment on that line is worth keeping.
        assert!(body.contains("# F2 ·"), "the key's own hint survived: {body}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn saving_settings_appends_a_key_the_file_never_had() {
        let path = tmp("append");
        std::fs::write(&path, "base_url = \"x\"\napi_key = \"vng_k\"\n").unwrap();
        save_settings_at(&path, &settings("mono-amber", true)).unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("theme = \"mono-amber\""), "{body}");
        assert!(body.contains("polarity = \"dark\""), "{body}");
        assert!(body.contains("notifications = true"), "{body}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn the_generated_config_documents_every_optional_key() {
        // First-run onboarding used to write two lines, so a pilot who never
        // read the docs never learned the rest existed.
        let body = generated_body(DEFAULT_BASE_URL, "vng_k");
        for key in ["theme", "polarity", "hints", "boot", "notifications", "log"] {
            assert!(body.contains(&format!("#{key} =")), "{key} is not documented: {body}");
        }
        // Documented, but not active: the defaults must stay defaults.
        match load_status_at_str(&body) {
            ConfigStatus::Ready(c) => {
                assert_eq!(c.color_mode(), crate::app::ColorMode::MonoGreen);
                assert!(c.hints && c.boot && c.notifications);
            }
            other => panic!("the generated file must parse: {other:?}"),
        }
    }

    /// Parse a config body without touching the filesystem.
    fn load_status_at_str(body: &str) -> ConfigStatus {
        let path = tmp("inline");
        std::fs::write(&path, body).unwrap();
        let status = load_status_at(&path);
        let _ = std::fs::remove_file(&path);
        status
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
