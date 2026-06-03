# TODO — CI + Tests

## Context

Rust TUI app, currently no test suite. CI runs check + clippy + build only.

Layers to cover:
- `src/api/types.rs` — serde types, camelCase renames, Unknown fallbacks
- `src/api/client.rs` — reqwest HTTP client, 14 endpoints
- `src/app.rs` — AppState, 50+ methods, complex state machine
- `src/ui/cockpit.rs` — 39 ratatui render functions, 2300 lines

---

## Dev dependencies to add

```toml
[dev-dependencies]
wiremock  = "0.6"
insta     = { version = "1", features = ["json", "ron"] }
proptest  = "1"
```

Also install via cargo-binstall (not a dev-dep):
- `cargo-nextest` — faster test runner, better CI output
- `cargo-llvm-cov` — LLVM-based coverage

---

## File structure

```
tests/
  fixtures/
    probe_idle.json
    probe_moving.json
    probe_low_resources.json
    mannies_two.json
    sector_asteroid.json
    sector_empty.json
  api/
    client_test.rs
    types_test.rs
  app/
    state_test.rs
    coords_test.rs
  ui/
    snapshots/          ← insta auto-generated, committed
    cockpit_test.rs
```

---

## Step 1 — JSON fixtures

Create `tests/fixtures/` with realistic anonymised API responses.

| File | Content |
|---|---|
| `probe_idle.json` | Probe status=Idle, fuel 80%, integrity 100% |
| `probe_moving.json` | Probe with active movement, phase=Cruising |
| `probe_low_resources.json` | fuel 15%, integrity 20% (gauge color testing) |
| `mannies_two.json` | 2 mannies — one idle, one mining at 42% |
| `sector_asteroid.json` | SectorObservation with 3 minable asteroids |
| `sector_empty.json` | Empty sector, freshness=Unavailable |

---

## Step 2 — API types deserialization (`tests/api/types_test.rs`)

```rust
#[test]
fn probe_idle_deserializes() {
    let json = include_str!("../fixtures/probe_idle.json");
    let probe: Probe = serde_json::from_str(json).unwrap();
    assert_eq!(probe.status, ProbeStatus::Idle);
    assert!(probe.movement.is_none());
}

#[test]
fn unknown_probe_status_falls_back() {
    // inject unknown variant → ProbeStatus::Unknown, no panic
}

#[test]
fn probe_moving_has_movement() { ... }
#[test]
fn manny_task_unknown_falls_back() { ... }
#[test]
fn sector_observation_deserializes_objects() { ... }
```

Goal: cover all enums with `#[serde(other)] Unknown` + all camelCase structs.

---

## Step 3 — API client HTTP (`tests/api/client_test.rs`)

```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, body_json};

#[tokio::test]
async fn get_probe_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/probe"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_raw(include_str!("../fixtures/probe_idle.json"), "application/json"))
        .mount(&server)
        .await;

    let client = ApiClient::new(server.uri(), "test_key".into()).unwrap();
    let probe = client.get_probe().await.unwrap();
    assert_eq!(probe.name, "neumann");
}

#[tokio::test]
async fn unauthorized_returns_clear_error() {
    // 401 → anyhow error containing "check api_key"
}

#[tokio::test]
async fn move_probe_sends_correct_body() {
    // verify POST body contains { "target": { "x": 2, "y": 0, "z": 0 } }
    // via body_json matcher
}

#[tokio::test]
async fn server_error_propagates() {
    // 500 → error, no panic
}
```

Target: 14 endpoints × happy path + 401 + 500 ≈ 42 tests.

---

## Step 4 — AppState logic (`tests/app/state_test.rs`)

```rust
#[test]
fn parse_scan_coords_valid() {
    // (2,0,0) — sum even → ok
}

#[test]
fn parse_scan_coords_odd_sum_rejected() {
    // (1,0,0) — sum odd → err
}

#[test]
fn parse_scan_coords_non_numeric_rejected() { ... }

#[test]
fn neighbors_d1_returns_six_even_sum_coords() {
    let ns = AppState::neighbors_d1((0, 0, 0));
    assert_eq!(ns.len(), 6);
    for (x, y, z) in ns {
        assert_eq!((x + y + z) % 2, 0);
    }
}

#[test]
fn mine_max_amount_capped_by_free_cargo() {
    // free_capacity = 10, available = 100 → max = 10
}

#[test]
fn mine_max_amount_capped_by_available_resource() {
    // free_capacity = 100, available = 5 → max = 5
}

#[test]
fn travel_preview_computes_eta() {
    // inject Probe at (0,0,0), request move to (2,0,0)
    // verify fuel_cost and eta_minutes are nonzero and plausible
}

#[test]
fn next_refresh_instant_no_movement_is_far_future() {
    // no movement → deadline is ~24h away
}

#[test]
fn next_refresh_instant_with_arrival_is_bounded() {
    // arrival_at = now + 5min → deadline ≈ now + 5min
}
```

Target: ~25 tests on pure-logic AppState methods.

---

## Step 5 — Property-based coords (`tests/app/coords_test.rs`)

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn valid_coords_accepted(x in -100i32..100, y in -100i32..100) {
        let z = if (x + y) % 2 == 0 { 0 } else { 1 };
        let input = format!("{x},{y},{z}");
        prop_assert!(state.parse_scan_coords(&input).is_ok());
    }

    #[test]
    fn odd_sum_coords_rejected(x in -100i32..100, y in -100i32..100) {
        let z = if (x + y) % 2 == 0 { 1 } else { 0 };
        let input = format!("{x},{y},{z}");
        prop_assert!(state.parse_scan_coords(&input).is_err());
    }
}
```

---

## Step 6 — UI snapshots (`tests/ui/cockpit_test.rs`)

```rust
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render_to_string(state: &AppState, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| cockpit::render(f, state)).unwrap();
    // read cells from buffer → String
    buffer_to_string(terminal.backend().buffer())
}

#[test]
fn probe_panel_idle() {
    let state = AppState { probe: Some(fixture_probe_idle()), ..Default::default() };
    insta::assert_snapshot!(render_to_string(&state, 120, 30));
}

#[test]
fn probe_panel_moving() { ... }

#[test]
fn probe_panel_low_resources_red_gauges() { ... }

#[test]
fn mannies_panel_two_mannies() { ... }

#[test]
fn scanner_panel_asteroid_sector() { ... }

#[test]
fn travel_overlay_typing_state() { ... }

#[test]
fn error_banner_displayed() { ... }

#[test]
fn loading_state_no_data() { ... }
```

Workflow: first run generates snapshots under `tests/ui/snapshots/`, committed to git.
Any visual regression fails CI. Review with `cargo insta review`.

---

## Step 7 — CI update (`ci.yml`)

Add after existing clippy step:

```yaml
- name: Install nextest + llvm-cov
  uses: taiki-e/install-action@v2
  with:
    tool: nextest,cargo-llvm-cov

- name: Run tests
  run: cargo nextest run --all-features

- name: Generate coverage
  run: cargo llvm-cov nextest --lcov --output-path lcov.info

- name: Upload to Codecov
  uses: codecov/codecov-action@v5
  with:
    files: lcov.info
    fail_ci_if_error: true
```

Also upgrade clippy step to catch dead code:
```yaml
run: cargo clippy -- -D warnings -D dead_code
```

Add weekly audit job:
```yaml
audit:
  runs-on: ubuntu-latest
  if: github.event_name == 'schedule'
  steps:
    - uses: actions/checkout@...
    - run: cargo install cargo-audit && cargo audit
```

---

## Metrics summary

| Metric | Tool | Integration |
|---|---|---|
| Code coverage | cargo-llvm-cov | Codecov, PR diff, badge |
| Snapshot regressions | insta | Fail on any visual drift |
| Property violations | proptest | Fail + shrunk counterexample |
| Security audit | cargo-audit | Weekly cron CI job |
| Dead code | clippy -D dead_code | Existing CI step upgrade |
| Build time | swatinem/rust-cache | Already in place |

Codecov thresholds to configure: 70% minimum to not block PRs, target 85%.

---

## Implementation order

| Step | Estimated time |
|---|---|
| 1. JSON fixtures | 1-2h |
| 2. Types deserialization | 2h |
| 3. Client wiremock | 3-4h |
| 4. AppState logic | 3h |
| 5. CI update (nextest + llvm-cov + codecov) | 1h |
| 6. UI snapshots | 4-5h |
| 7. Proptest coords | 1h |

**Total: ~2 days for a solid base with green CI.**
