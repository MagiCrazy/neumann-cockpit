//! Client-side rate-limit tracking (API v104).
//!
//! The server rate-limits every authenticated endpoint per bearer token using a
//! shared sliding window: successful responses carry `X-RateLimit-Limit`,
//! `X-RateLimit-Remaining` and `X-RateLimit-Reset`, and a token over its limit
//! gets **429** with `Retry-After`. `/api/version` and the auth endpoints are
//! exempt.
//!
//! Every `ApiClient` send path feeds the headers into a [`RateLimitState`]
//! shared across clones (behind `Arc<Mutex<_>>`, like the metrics ring). That
//! shared cell is how the *cockpit* learns about a 429 even when the failing
//! call was a silently-dropped background fetch: the event loop reads the state
//! each tick, holds the auto-refresh off until the window reopens, and shows a
//! status-bar chip.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Fallback pause when the server rejects a request without telling us how long
/// to wait (no `Retry-After`, no usable `X-RateLimit-Reset`).
pub const DEFAULT_RETRY_AFTER: Duration = Duration::from_secs(5);

/// A shared rate-limit cell: cheap to clone, so every `ApiClient` clone —
/// including `with_active_probe` — reads and writes the same state.
pub type RateLimitHandle = Arc<Mutex<RateLimitState>>;

/// Last known quota, plus the deadline of an in-force 429 back-off.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RateLimitState {
    /// `X-RateLimit-Limit`: accepted requests per window.
    pub limit: Option<u64>,
    /// `X-RateLimit-Remaining`: requests left in the current window.
    pub remaining: Option<u64>,
    /// `X-RateLimit-Reset`: unix timestamp when the oldest counted request
    /// expires.
    pub reset_unix: Option<i64>,
    /// Set on a 429: no request should be sent before this instant.
    blocked_until: Option<Instant>,
    /// How many 429s this session has taken. Sticky, unlike `blocked_until`: a
    /// short back-off can expire before the caller gets to look, and "we were
    /// throttled" is still the explanation it needs.
    pub throttles: u32,
}

impl RateLimitState {
    /// A shared, empty cell.
    pub fn handle() -> RateLimitHandle {
        Arc::new(Mutex::new(RateLimitState::default()))
    }

    /// Absorb the quota headers of a response.
    pub fn note_quota(&mut self, limit: Option<u64>, remaining: Option<u64>, reset_unix: Option<i64>) {
        if limit.is_some() {
            self.limit = limit;
        }
        if remaining.is_some() {
            self.remaining = remaining;
        }
        if reset_unix.is_some() {
            self.reset_unix = reset_unix;
        }
    }

    /// Record a 429 and hold requests off for `retry_after`.
    pub fn note_throttled(&mut self, retry_after: Duration) {
        self.remaining = Some(0);
        self.blocked_until = Some(Instant::now() + retry_after);
        self.throttles = self.throttles.saturating_add(1);
    }

    /// How long until the window reopens, or `None` when not throttled. Expires
    /// on its own once the deadline passes.
    pub fn retry_in(&self) -> Option<Duration> {
        let until = self.blocked_until?;
        let now = Instant::now();
        (until > now).then(|| until - now)
    }

    /// Whole seconds until the window reopens, **rounded up**: the deadline is
    /// already a fraction of a millisecond in the past by the time it is read,
    /// so truncating would report a 12 s back-off as 11 s and, worse, a
    /// sub-second one as `0s`.
    pub fn retry_in_secs(&self) -> Option<u64> {
        self.retry_in().map(|d| d.as_millis().div_ceil(1000).max(1) as u64)
    }

    /// Whether a 429 back-off is still in force.
    pub fn throttled(&self) -> bool {
        self.retry_in().is_some()
    }
}

/// The error returned by every send path on a 429, carrying the retry delay the
/// server asked for. Callers that need to react (rather than just display the
/// message) can `downcast_ref` it out of the `anyhow::Error`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimited {
    pub retry_after_secs: Option<u64>,
}

impl std::fmt::Display for RateLimited {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.retry_after_secs {
            Some(s) => write!(f, "Rate limited by the server — retry in {s}s"),
            None => write!(f, "Rate limited by the server — retry shortly"),
        }
    }
}

impl std::error::Error for RateLimited {}

/// Pick the back-off delay from a 429's headers: `Retry-After` (seconds) first,
/// else the distance to `X-RateLimit-Reset`, else [`DEFAULT_RETRY_AFTER`].
/// `now_unix` is passed in so the choice is testable.
pub fn retry_after_from(retry_after_header: Option<u64>, reset_unix: Option<i64>, now_unix: i64) -> Duration {
    if let Some(secs) = retry_after_header {
        return Duration::from_secs(secs.max(1));
    }
    if let Some(reset) = reset_unix {
        let delta = reset - now_unix;
        if delta > 0 {
            return Duration::from_secs(delta as u64);
        }
    }
    DEFAULT_RETRY_AFTER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_after_header_wins_over_reset() {
        assert_eq!(
            retry_after_from(Some(30), Some(1_000_100), 1_000_000),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn falls_back_to_reset_then_to_the_default() {
        assert_eq!(
            retry_after_from(None, Some(1_000_042), 1_000_000),
            Duration::from_secs(42)
        );
        // A reset already in the past tells us nothing useful.
        assert_eq!(retry_after_from(None, Some(999_000), 1_000_000), DEFAULT_RETRY_AFTER);
        assert_eq!(retry_after_from(None, None, 1_000_000), DEFAULT_RETRY_AFTER);
    }

    #[test]
    fn a_zero_retry_after_still_pauses_a_second() {
        assert_eq!(retry_after_from(Some(0), None, 0), Duration::from_secs(1));
    }

    #[test]
    fn throttling_expires_on_its_own() {
        let mut s = RateLimitState::default();
        assert!(!s.throttled());
        s.note_throttled(Duration::from_secs(10));
        assert!(s.throttled());
        assert_eq!(s.remaining, Some(0), "a 429 means the window is spent");
        assert_eq!(s.retry_in_secs(), Some(10));

        s.note_throttled(Duration::ZERO);
        assert!(!s.throttled(), "a deadline in the past no longer blocks");
        assert_eq!(s.retry_in_secs(), None);
        assert_eq!(s.throttles, 2, "the count is sticky once the back-off expires");
    }

    #[test]
    fn quota_keeps_the_last_known_values() {
        let mut s = RateLimitState::default();
        s.note_quota(Some(120), Some(119), Some(1_000_000));
        s.note_quota(None, Some(118), None);
        assert_eq!(s.limit, Some(120), "an absent header must not erase the quota");
        assert_eq!(s.remaining, Some(118));
        assert_eq!(s.reset_unix, Some(1_000_000));
    }
}
