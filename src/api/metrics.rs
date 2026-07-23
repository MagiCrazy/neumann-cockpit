//! Client-side request instrumentation (#247).
//!
//! Every `ApiClient` send path records a [`RequestSample`] into a bounded,
//! in-memory ring shared across clones (behind `Arc<Mutex<_>>`). [`Metrics`]
//! aggregates the ring into per-endpoint latency percentiles and error/timeout
//! counts — the data behind the headless `--diagnostic` report and, later, the
//! in-cockpit diagnostics overlay. Purely client-side: no server endpoint.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Bound on retained samples. A diagnostic burst is a few dozen requests; a
/// long cockpit session stays well under this. Oldest samples drop first.
const RING_CAP: usize = 1024;

/// A shared metrics ring: cheap to clone (an `Arc`), so every `ApiClient` clone
/// records into the same ring.
pub type MetricsHandle = Arc<Mutex<Metrics>>;

/// One recorded request: the normalized endpoint label, how long the `send()`
/// took, the HTTP status (absent when no response arrived), and whether the
/// failure was a timeout.
#[derive(Debug, Clone, PartialEq)]
pub struct RequestSample {
    pub label: String,
    pub elapsed_ms: f64,
    /// `None` when no response arrived (timeout or transport error).
    pub status: Option<u16>,
    pub timed_out: bool,
    /// A 2xx response.
    pub ok: bool,
}

/// Aggregated stats for one endpoint label over the retained samples.
#[derive(Debug, Clone, PartialEq)]
pub struct EndpointStats {
    pub label: String,
    pub count: usize,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub max_ms: f64,
    /// Requests that did not return a 2xx (HTTP errors + transport failures).
    pub errors: usize,
    pub timeouts: usize,
}

/// A bounded ring of request samples.
#[derive(Default)]
pub struct Metrics {
    ring: VecDeque<RequestSample>,
}

impl Metrics {
    /// A shared, empty ring.
    pub fn handle() -> MetricsHandle {
        Arc::new(Mutex::new(Metrics::default()))
    }

    /// Record one sample, evicting the oldest when full.
    pub fn record(&mut self, sample: RequestSample) {
        if self.ring.len() == RING_CAP {
            self.ring.pop_front();
        }
        self.ring.push_back(sample);
    }

    pub fn len(&self) -> usize {
        self.ring.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }

    /// Total requests that failed (no 2xx) across every endpoint.
    pub fn total_errors(&self) -> usize {
        self.ring.iter().filter(|s| !s.ok).count()
    }

    /// Total requests that timed out across every endpoint.
    pub fn total_timeouts(&self) -> usize {
        self.ring.iter().filter(|s| s.timed_out).count()
    }

    /// Per-endpoint aggregate, one entry per label, sorted by descending p95
    /// (slowest first — what a bug report leads with).
    pub fn aggregate(&self) -> Vec<EndpointStats> {
        let mut by_label: std::collections::BTreeMap<&str, Vec<&RequestSample>> = std::collections::BTreeMap::new();
        for s in &self.ring {
            by_label.entry(s.label.as_str()).or_default().push(s);
        }
        let mut stats: Vec<EndpointStats> = by_label
            .into_iter()
            .map(|(label, samples)| {
                let mut lat: Vec<f64> = samples.iter().map(|s| s.elapsed_ms).collect();
                lat.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                EndpointStats {
                    label: label.to_string(),
                    count: samples.len(),
                    p50_ms: percentile(&lat, 50.0),
                    p95_ms: percentile(&lat, 95.0),
                    max_ms: lat.last().copied().unwrap_or(0.0),
                    errors: samples.iter().filter(|s| !s.ok).count(),
                    timeouts: samples.iter().filter(|s| s.timed_out).count(),
                }
            })
            .collect();
        stats.sort_by(|a, b| b.p95_ms.partial_cmp(&a.p95_ms).unwrap_or(std::cmp::Ordering::Equal));
        stats
    }
}

/// Nearest-rank percentile over an already-sorted slice (empty → 0). `p` is a
/// percentage in `0..=100`.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = (p / 100.0 * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

/// Normalize a request into an aggregation label: `METHOD /path` with numeric
/// path segments collapsed to `:id` and any query string dropped, so requests
/// to the same endpoint with different ids aggregate together.
pub fn endpoint_label(method: &str, path: &str) -> String {
    let path = path.split('?').next().unwrap_or(path);
    let normalized: Vec<&str> = path
        .split('/')
        .map(|seg| {
            if !seg.is_empty() && seg.chars().all(|c| c.is_ascii_digit()) {
                ":id"
            } else {
                seg
            }
        })
        .collect();
    format!("{method} {}", normalized.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_collapses_numeric_segments_and_drops_query() {
        assert_eq!(
            endpoint_label("POST", "/api/probe/123/mannies/456/mine"),
            "POST /api/probe/:id/mannies/:id/mine"
        );
        assert_eq!(endpoint_label("GET", "/api/probe"), "GET /api/probe");
        assert_eq!(
            endpoint_label("GET", "/api/probe/probe-improvements-available?includeAll=1"),
            "GET /api/probe/probe-improvements-available"
        );
    }

    #[test]
    fn percentile_matches_nearest_rank() {
        let v = [10.0, 20.0, 30.0, 40.0, 50.0];
        assert_eq!(percentile(&v, 50.0), 30.0);
        assert_eq!(percentile(&v, 95.0), 50.0);
        assert_eq!(percentile(&v, 0.0), 10.0);
        assert_eq!(percentile(&[], 50.0), 0.0);
    }

    fn sample(label: &str, ms: f64, status: Option<u16>, timed_out: bool) -> RequestSample {
        RequestSample {
            label: label.to_string(),
            elapsed_ms: ms,
            status,
            ok: status.map(|s| (200..300).contains(&s)).unwrap_or(false),
            timed_out,
        }
    }

    #[test]
    fn aggregate_groups_and_counts_errors_and_timeouts() {
        let mut m = Metrics::default();
        m.record(sample("GET /a", 100.0, Some(200), false));
        m.record(sample("GET /a", 300.0, Some(500), false));
        m.record(sample("GET /a", 900.0, None, true));
        m.record(sample("GET /b", 50.0, Some(200), false));

        let agg = m.aggregate();
        // Sorted by p95 desc: /a (900) before /b (50).
        assert_eq!(agg[0].label, "GET /a");
        assert_eq!(agg[0].count, 3);
        assert_eq!(agg[0].max_ms, 900.0);
        assert_eq!(agg[0].errors, 2, "one 500 + one timeout");
        assert_eq!(agg[0].timeouts, 1);
        assert_eq!(agg[1].label, "GET /b");
        assert_eq!(m.total_errors(), 2);
        assert_eq!(m.total_timeouts(), 1);
    }

    #[test]
    fn ring_evicts_oldest_past_capacity() {
        let mut m = Metrics::default();
        for i in 0..(RING_CAP + 10) {
            m.record(sample("GET /x", i as f64, Some(200), false));
        }
        assert_eq!(m.len(), RING_CAP);
    }
}
