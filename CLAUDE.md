# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo check          # type-check without linking
cargo build          # debug build
cargo build --release
cargo run            # run the TUI (requires config, see below)
cargo clippy         # lints
```

No test suite yet.

## Config

The binary reads `~/.config/neumann-cockpit/config.toml` at startup:

```toml
base_url = "https://neumann-probe.net"
api_key  = "vng_..."
```

Copy `config.example.toml` to that path and fill in the API key (generated once via the web UI).

Scan history is persisted across runs to `~/.config/neumann-cockpit/scan_history.json`.

## Architecture

### Event loop (`src/main.rs`)

Single `tokio::select!` loop over three sources:

- **crossterm `EventStream`** — keyboard / mouse / resize events
- **`mpsc::Receiver<ApiMessage>`** — results from spawned API tasks
- **`tokio::time::sleep_until(deadline)`** — auto-refresh timer

The timer deadline is set to `movement.arrival_at` (ISO 8601) converted to a `tokio::time::Instant`. When no movement is in progress the deadline is 24 h away — no polling, no tick loop.

`fetch_all()` spawns **three** independent `tokio::spawn` tasks: probe, mannies, and sector. Mannies and sector failures are non-fatal.

All other API calls (move, repair, mine, craft, etc.) are also spawned tasks that send results back via the `mpsc::Sender<ApiMessage>`.

`main.rs` is currently large (~1600 lines) and mixes three concerns: the `tokio::select!` loop, all keyboard handlers (`handle_*_event`), and all `fetch_*` spawner functions. Planned refactor: extract handlers to `src/input.rs` and fetchers to `src/api/tasks.rs`.

### State (`src/app.rs`)

`AppState` is the single source of truth passed to the renderer. Key design choices:

- Each interactive action (travel, repair, mine, craft, jettison, salvage, recall, rename, deploy, inspect, recover, detach, atomic printer craft) has its own input state enum (`TravelInput`, `RepairInput`, etc.) with variants for each wizard step. All start as `Inactive`.
- `update_probe()` extracts `movement_arrival` from the response and stores it separately so the event loop can compute the next deadline without re-reading the full probe struct.
- Scan history is cached in `AppState::scan_history` (a `Vec<SectorObservation>`) and persisted to disk asynchronously after each sector fetch.
- `RESOURCE_TYPES` and `DETACH_MODES` constants live here (not in `main.rs` or `cockpit.rs`).

### API layer (`src/api/`)

- `types.rs` — all OpenAPI types. Structs use `#[serde(rename_all = "camelCase")]`; enums use `#[serde(rename_all = "snake_case")]` with `#[serde(other)] Unknown` fallbacks. `#![allow(dead_code)]` suppresses warnings on fields not yet consumed by the UI.
- `client.rs` — `ApiClient` (cloneable, wraps `reqwest::Client`). Each endpoint has a typed wrapper with an inline `struct Resp` that deserializes the envelope and returns the inner value. HTTP errors extract `error.message` from the JSON body; 401 produces a specific "check your api_key" message.

### UI (`src/ui/cockpit.rs`)

`render(frame, state)` is the single render entry point called every loop iteration. Layout — two rows, each split into two columns:

```
┌─ NEUMANN COCKPIT ─────────────────────────────────────────────┐
│  ┌─ PROBE ─────────────────┐  ┌─ INVENTORY ─────────────────┐ │
│  │ name · status · sector  │  │ capacity gauge              │ │
│  │ movement phase + ETA    │  │ resource stocks             │ │
│  │ progress gauge          │  │ items list (expandable)     │ │
│  │ speed gauge             │  │ [j] jettison  [d] deploy    │ │
│  │ fuel gauge              │  │ [a] atomic craft            │ │
│  │ integrity gauge         │  └─────────────────────────────┘ │
│  └─────────────────────────┘                                  │
│  ┌─ SCANNER ───────────────┐  ┌─ MANNIES ───────────────────┐ │
│  │ sector detail / history │  │ ● manny-1  idle             │ │
│  │ [↑/↓] scroll history   │  │ ◌ manny-2  mining   42%     │ │
│  │ [enter] drill-down      │  │ [c] craft  [n] mine         │ │
│  │ [f] batch scan          │  │ [x] repair [l] recall       │ │
│  └─────────────────────────┘  │ [v] salvage [e] rename      │ │
│                               └─────────────────────────────┘ │
│ [r] refresh [p][i][m][s] focus [t] travel [b] map [q] quit    │
│                                    v23.0.0  API v23  ⟳ HH:MM  │
└───────────────────────────────────────────────────────────────┘
```

Top row height is dynamic: `max(probe_panel_height, inventory_panel_height)`. The focused panel gets a white border; others are dimmed.

Gauge colors: green > 50 %, yellow 25–50 %, red < 25 %. Probe status and sensor mode are colour-coded.

Movement progress is derived from `started_at` / `arrival_at` timestamps client-side (more accurate than the API's `secondsRemaining` snapshot).

**Overlays** (rendered on top of the 4-panel layout):
- Travel — coordinate input + fuel cost preview + confirmation
- Repair / Mine / Craft / Atomic printer craft — manny/target/recipe pickers
- Jettison / Salvage / Recall / Rename / Inspect / Recover / Detach — inventory/sector object pickers
- Deploy waypoint — 3-step wizard: pick manny → pick object → enter bookmark name
- Map (`[b]`) — isometric sector overview with pan (`[↑↓←→]`) and layer selection

### Known issues / planned refactors

- `set_inspect_error` and `set_recover_error` in `app.rs` are no-ops: errors from the API on inspect/recover are silently dropped. Fix: add `error: Option<String>` to `InspectInput::PickAsteroid` and `RecoverInput::PickContainer`.
- `collect_mineable_candidates` and `collect_asteroid_candidates` are free functions in `main.rs`; the other `collect_*` functions are methods on `AppState`. Consolidate all into `AppState` methods.
- List navigation (Up/Down/Esc/Enter) is copy-pasted ~9× across handlers. Extract a `list_nav(code, sel, count) -> NavResult` helper.
- Selection overlays (mine, salvage, deploy, inspect, recover, detach) are structurally identical. Extract a `render_selection_overlay(...)` helper to remove ~300 lines of duplication.
- `ManniesResponse` in `types.rs` is unused — delete it.
- `travel_go_sector` has an unused `_dist_hint` parameter — remove it and the call-site calculation.

## Implemented API endpoints (v23)

| Endpoint | Method | Status |
|---|---|---|
| `/api/version` | GET | ✓ |
| `/api/probe` | GET | ✓ |
| `/api/probe/mannies` | GET | ✓ |
| `/api/probe/sector` | GET | ✓ |
| `/api/probe/move` | POST | ✓ |
| `/api/probe/mannies/{id}/repair` | POST | ✓ |
| `/api/probe/mannies/{id}/mine` | POST | ✓ |
| `/api/probe/mannies/{id}/craft` | POST | ✓ |
| `/api/probe/mannies/{id}/salvage` | POST | ✓ |
| `/api/probe/mannies/{id}/recall` | POST | ✓ |
| `/api/probe/mannies/{id}` | PATCH | ✓ (rename) |
| `/api/probe/mannies/{id}/install-bookmark` | POST | ✓ |
| `/api/probe/mannies/{id}/inspect-asteroid` | POST | ✓ |
| `/api/probe/mannies/{id}/recover-storage-container` | POST | ✓ |
| `/api/probe/mannies/{id}/detach-storage-container` | POST | ✓ |
| `/api/probe/inventory/{id}/jettison` | POST | ✓ |
| `/api/probe/atomic-printer/craft` | POST | ✓ |
| `/api/crafting-recipes` | GET | ✓ |
| `/api/sector` | GET | ✓ |
| `/api/probe/visited-sectors` | GET | ✗ (not implemented) |
| `/api/probe/messages` | GET/POST | ✗ (not implemented) |
