//! Release-update check (issue #339).
//!
//! A pilot who installed from the releases page — an archive plus its
//! `.sha256`, which is what the README tells them to do — has no way to learn
//! a newer version exists short of going back to look. There is no package
//! manager in that path and nothing in the cockpit ever mentions its own
//! version.
//!
//! Two things make this more than an HTTP call:
//!
//! - **It is the first request outside `base_url`.** Everything else the
//!   cockpit sends goes to the probe server the pilot configured. Asking
//!   GitHub tells a third party — one they never entered in `config.toml` —
//!   that this machine runs the cockpit, along with its IP. So it is opt-in,
//!   asked once at first run and written to the config, and a pilot who
//!   declines is never asked again.
//! - **The tag is not the crate version.** release-please tags this repository
//!   `neumann-cockpit-v104.4.0`, so the prefix has to come off before the
//!   string is a version at all. And the crate **major tracks the API
//!   version**, so a major bump is not a breaking change: the comparison is
//!   "is the published version greater", never "the major differs, beware".

use serde::Deserialize;
use std::time::Duration;

/// How long to leave between checks. A cockpit relaunched all day must not ask
/// all day, and the unauthenticated GitHub API allows 60 requests an hour.
pub const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Bounded like every other startup probe: an unreachable GitHub costs a few
/// seconds in a background task, never the boot.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/MagiCrazy/neumann-cockpit/releases/latest";

/// The tag prefix release-please puts in front of the version.
const TAG_PREFIX: &str = "neumann-cockpit-v";

/// What the pilot decided about the check.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum UpdatePref {
    /// Never asked yet — the first run asks.
    #[default]
    Unset,
    Enabled,
    Disabled,
}

impl UpdatePref {
    pub fn from_label(label: &str) -> Option<Self> {
        match label.trim().to_ascii_lowercase().as_str() {
            "true" | "on" | "yes" => Some(UpdatePref::Enabled),
            "false" | "off" | "no" => Some(UpdatePref::Disabled),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            UpdatePref::Enabled => "true",
            // An unset preference writes as `false`: a pilot who never answered
            // has not consented, and the absence of an answer is not a yes.
            UpdatePref::Unset | UpdatePref::Disabled => "false",
        }
    }

    pub fn enabled(self) -> bool {
        self == UpdatePref::Enabled
    }
}

/// Parse a release tag into comparable version numbers.
///
/// Returns `None` for anything that is not this project's tag shape, so a
/// renamed or hand-made tag is ignored rather than mis-compared.
pub fn parse_tag(tag: &str) -> Option<(u64, u64, u64)> {
    let rest = tag.strip_prefix(TAG_PREFIX).or_else(|| tag.strip_prefix('v'))?;
    let mut parts = rest.split('.');
    let mut number = || parts.next()?.parse::<u64>().ok();
    let version = (number()?, number()?, number()?);
    parts.next().is_none().then_some(version)
}

/// This build's version.
pub fn current_version() -> (u64, u64, u64) {
    parse_tag(&format!("v{}", env!("CARGO_PKG_VERSION"))).unwrap_or((0, 0, 0))
}

/// Whether `tag` names a release newer than this build.
///
/// A plain "greater than" — the crate major tracks the API version, so a major
/// bump carries no warning of its own.
pub fn is_newer(tag: &str, current: (u64, u64, u64)) -> bool {
    parse_tag(tag).is_some_and(|published| published > current)
}

/// Whether a check is due, given when the last one ran.
///
/// The gate the issue's acceptance asks for: a cockpit relaunched all day must
/// not ask all day. An unreadable or absent stamp means "never checked", which
/// is the reading that lets a first run proceed.
pub fn check_due(last_checked: Option<&str>, now: chrono::DateTime<chrono::Utc>) -> bool {
    let Some(stamp) = last_checked else { return true };
    let Ok(last) = chrono::DateTime::parse_from_rfc3339(stamp) else {
        return true;
    };
    (now - last.with_timezone(&chrono::Utc))
        .to_std()
        .map(|d| d >= CHECK_INTERVAL)
        .unwrap_or(true)
}

#[derive(Deserialize)]
struct LatestRelease {
    tag_name: String,
}

/// Ask GitHub for the latest release tag. `None` on any failure — no network,
/// a rate-limited API, an unparseable body — because none of that is the
/// pilot's problem and none of it is worth a message.
pub async fn fetch_latest_tag() -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(format!("neumann-cockpit/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;
    let response = client.get(LATEST_RELEASE_URL).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    Some(response.json::<LatestRelease>().await.ok()?.tag_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_release_please_tag_shape_is_understood() {
        // The actual shape this repository publishes.
        assert_eq!(parse_tag("neumann-cockpit-v104.4.0"), Some((104, 4, 0)));
        // And the plain shape, in case the tagging ever simplifies.
        assert_eq!(parse_tag("v1.2.3"), Some((1, 2, 3)));
    }

    #[test]
    fn anything_that_is_not_our_tag_is_ignored_rather_than_guessed() {
        assert_eq!(parse_tag("104.4.0"), None, "no prefix at all");
        assert_eq!(parse_tag("neumann-cockpit-v104.4"), None, "not three parts");
        assert_eq!(parse_tag("neumann-cockpit-v104.4.0.1"), None, "four parts");
        assert_eq!(parse_tag("v1.2.x"), None, "not a number");
        assert_eq!(parse_tag(""), None);
    }

    #[test]
    fn newer_means_greater_and_nothing_more() {
        let current = (104, 4, 0);
        assert!(is_newer("neumann-cockpit-v104.4.1", current));
        assert!(is_newer("neumann-cockpit-v104.5.0", current));
        assert!(is_newer("neumann-cockpit-v105.0.0", current));
        assert!(!is_newer("neumann-cockpit-v104.4.0", current), "the same build");
        assert!(!is_newer("neumann-cockpit-v104.3.9", current), "an older one");
        assert!(!is_newer("garbage", current), "an unparseable tag is not an update");
    }

    #[test]
    fn a_major_bump_is_not_a_warning() {
        // The crate major tracks the API version (v104 → v116 and so on), so a
        // major step is an ordinary release, not a breaking change.
        assert!(is_newer("neumann-cockpit-v116.0.0", (104, 4, 0)));
        assert!(!is_newer("neumann-cockpit-v104.0.0", (116, 0, 0)));
    }

    #[test]
    fn silence_is_not_consent() {
        // A pilot who never answered has not agreed to a third-party request.
        assert!(!UpdatePref::default().enabled());
        assert!(!UpdatePref::Unset.enabled());
        assert_eq!(UpdatePref::Unset.label(), "false");
        assert!(UpdatePref::Enabled.enabled());
    }

    #[test]
    fn the_preference_round_trips_through_its_label() {
        assert_eq!(UpdatePref::from_label("true"), Some(UpdatePref::Enabled));
        assert_eq!(UpdatePref::from_label("YES"), Some(UpdatePref::Enabled));
        assert_eq!(UpdatePref::from_label("false"), Some(UpdatePref::Disabled));
        assert_eq!(UpdatePref::from_label("off"), Some(UpdatePref::Disabled));
        assert_eq!(UpdatePref::from_label("maybe"), None);
    }

    #[test]
    fn a_relaunch_does_not_re_ask_github() {
        use chrono::{Duration, Utc};
        let now = Utc::now();

        assert!(check_due(None, now), "never checked → check");
        assert!(
            !check_due(Some(&(now - Duration::hours(1)).to_rfc3339()), now),
            "an hour ago is far too soon"
        );
        assert!(
            check_due(Some(&(now - Duration::hours(25)).to_rfc3339()), now),
            "a day later it is due again"
        );
        assert!(
            check_due(Some("not a date"), now),
            "an unreadable stamp is not a lock-out"
        );
        // A stamp in the future means the clock moved backwards. Checking
        // (and rewriting the stamp) self-heals; treating it as "not due" would
        // disable the check until that future time actually arrives.
        assert!(check_due(Some(&(now + Duration::hours(2)).to_rfc3339()), now));
    }

    #[test]
    fn this_build_knows_its_own_version() {
        assert_ne!(current_version(), (0, 0, 0), "CARGO_PKG_VERSION must parse");
    }
}
