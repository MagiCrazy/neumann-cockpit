# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo check          # type-check without linking
cargo build          # debug build
cargo build --release
cargo run            # run the TUI (requires config, see below)
cargo clippy         # lints
cargo test           # unit tests (app.rs, input.rs) + serde fixtures (tests/)
```

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

`fetch_all()` spawns **seven** independent `tokio::spawn` tasks: probe, mannies, sector, visited sectors, alerts, damage warnings, and missions. All but probe are non-fatal.

All other API calls (move, repair, mine, craft, storage container CRUD, storage moves, etc.) are also spawned tasks that send results back via the `mpsc::Sender<ApiMessage>`. Keyboard handlers live in `src/input/` (`mod.rs` holds `handle_event` — overlay dispatch + global key match; one module per wizard handler — `pickers.rs` groups the manny pick-list/confirm ones, `containers.rs` the storage-container ones, `storage_move.rs`, `alerts.rs`, `geometry.rs` for the scan offset helpers); fetch spawners live in `src/api/tasks.rs`. `main.rs` only contains the select loop and the `ApiMessage` dispatch (which also sets the success toasts).

### State (`src/app/`)

`AppState` is the single source of truth passed to the renderer. Split by domain: `mod.rs` (struct `Panel` + `AppState`, core impl — updates, focus, toasts, refresh deadline — and `pub use` re-exports keeping `crate::app::*` paths stable), `inputs.rs` (all wizard input enums + constants), `scan.rs`, `travel.rs`, `inventory.rs`, `mannies.rs`, `containers.rs` (storage-container/move helpers), `map.rs`, `waypoints.rs`, `message.rs` (`ApiMessage`), `tests.rs` (unit tests). Key design choices:

- Each interactive action (travel, repair, mine, craft, jettison, salvage, recall, rename, deploy, inspect, recover, detach, atomic printer craft, object actions, waypoints, alerts, storage containers + rename + routing rules, storage moves, drop cargo) has its own input state enum (`TravelInput`, `RepairInput`, `ObjectActionInput`, `AlertsInput`, `ContainersInput`, `ContainerRulesInput`, `StorageMoveInput`, etc.) with variants for each wizard step. All start as `Inactive`.
- `update_probe()` extracts `movement_arrival` from the response and stores it separately so the event loop can compute the next deadline without re-reading the full probe struct.
- Scan history is cached in `AppState::scan_history` (a `Vec<SectorObservation>`) and persisted to disk asynchronously after each sector fetch. Each observation is stamped with a local `scanned_at` on receipt (serde-defaulted, so old history files load).
- Panel cursors: `mannies_selection`, `inventory_selection` (rows built by `inventory_rows()` — stocks, active items, passive groups), `scan_history_idx` (moves within `filtered_history_indices()` when a `ScanFilter` is active), `scanner_obj_selection` (object-browsing mode, entries from `scanner_objects()`).
- `jettison_for_selected()` builds the jettison wizard from the selected inventory row; `actions_for_object()` maps a `ScannerObjectEntry` to its available `ObjectAction`s, mirroring the manny-first candidate sets (`collect_*_candidates`).
- Transient success toasts: `set_toast()` / `active_toast()` (5 s expiry, dismissed by any keypress).
- `RESOURCE_TYPES`, `MOVE_RESOURCE_TYPES`, and `DETACH_MODES` constants live here (not in `main.rs` or `cockpit.rs`).

### API layer (`src/api/`)

- `types.rs` — all OpenAPI types. Structs use `#[serde(rename_all = "camelCase")]`; enums use `#[serde(rename_all = "snake_case")]` with `#[serde(other)] Unknown` fallbacks. `#![allow(dead_code)]` suppresses warnings on fields not yet consumed by the UI.
- `client.rs` — `ApiClient` (cloneable, wraps `reqwest::Client`). Each endpoint has a typed wrapper with an inline `struct Resp` that deserializes the envelope and returns the inner value. HTTP errors extract `error.message` from the JSON body; 401 produces a specific "check your api_key" message.

### Retro theme (`src/ui/retro/`)

Optional phosphor-CRT skin (Alien-style), selected with `theme = "retro"` in config or toggled at runtime with `F2`. Entirely self-contained: `ui::render` dispatches on `AppState::ui_theme` between `cockpit::render` (classic, default) and `retro::render`. Components: `palette.rs` (4-intensity monochrome, green or amber via `phosphor`), `banner.rs`, `systems.rs` (block gauges + probe schematic LEDs), `radar.rs` (sweeping dial, blips for `scanner_objects()`), `drones.rs`, `ticker.rs` (COMMS + pseudo-telemetry), `boot.rs` (teletype boot sequence, any key skips).

Animations are driven by a 100 ms render tick in the main `select!`, guarded by `anim_tick_active()` — it only advances `AnimState::frame` and redraws, **never** triggers API calls; with the classic theme or `animations = false` the branch is disabled and behaviour is exactly the pre-existing event-driven one. All animations are pure functions of the frame counter (`app/anim.rs`, `anim_hash` for deterministic noise). Wizard overlays are shared between themes via `overlays::render_active_overlays`.

### UI (`src/ui/`)

`cockpit::render(frame, state)` is the single render entry point called every loop iteration. Module layout: `cockpit.rs` (entry point, 4-panel layout, status bar), `panels/` (one file per panel, each with its height helper), `overlays/` (one file per wizard overlay; `pickers.rs` groups the manny pick-list/confirm ones, `containers.rs` the storage-container ones, plus `alerts.rs` / `storage_move.rs`; `mod.rs` hosts `centered_rect` + `render_pick_list`), `theme.rs` (colours, icons, labels, `format_duration`/`format_age`). Layout — two rows, each split into two columns:

```
┌─ NEUMANN COCKPIT ─────────────────────────────────────────────┐
│  ┌─ PROBE ─────────────────┐  ┌─ INVENTORY ─────────────────┐ │
│  │ name · status · sector  │  │ capacity gauge              │ │
│  │ movement phase + ETA    │  │ resource stocks (cursor)    │ │
│  │ progress gauge          │  │ items list (expandable)     │ │
│  │ speed gauge             │  │ containers + tanks gauges   │ │
│  │ fuel gauge              │  │ [↑↓] select [Enter] detail  │ │
│  │ integrity gauge         │  │ [j] jettison [d] deploy     │ │
│  │ [!] alert badge         │  │ [a] atomic [C] containers   │ │
│  └─────────────────────────┘  │ [M] move stock              │ │
│                               └─────────────────────────────┘ │
│  ┌─ SCANNER ───────────────┐  ┌─ MANNIES ───────────────────┐ │
│  │ sector detail │ history │  │ ● manny-1  idle             │ │
│  │ [↑↓/jk] history [JK]    │  │ ◌ manny-2  mining   42%     │ │
│  │ [Enter] rescan [c] coord│  │ [Enter] repair [e] mine     │ │
│  │ [n] neighbors [d] deep  │  │ [c] craft  [s] salvage      │ │
│  │ [f] filter  [o] objects │  │ [x] inspect [D] detach      │ │
│  │ [g] go to sector        │  │ [v] recover [n] rename      │ │
│  └─────────────────────────┘  │ [R] recall [X] drop cargo   │ │
│                               └─────────────────────────────┘ │
│ [r] refresh [p][i][m][s]/Tab focus [t] travel [b] map         │
│ [w] waypoints [A] alerts [?] help [q] quit  v23.x  API v44    │
└───────────────────────────────────────────────────────────────┘
```

Top row height is dynamic: `max(probe_panel_height, inventory_panel_height)`. The focused panel gets a white border; others are dimmed.

Gauge colors: green > 50 %, yellow 25–50 %, red < 25 %. Probe status and sensor mode are colour-coded.

Movement progress is derived from `started_at` / `arrival_at` timestamps client-side (more accurate than the API's `secondsRemaining` snapshot).

Scanner specifics: the history column shows symbol + coords + distance, scrolls with the selection (`List`/`ListState`), and `[f]` cycles a filter (all → objects → minable → danger). `[o]` enters object-browsing mode on the probe's current sector: `Enter` on an object opens a contextual action menu (mine / inspect / salvage / recover / deploy waypoint) that reuses the existing wizards.

**Overlays** (rendered on top of the 4-panel layout):
- Travel (`[t]`) — coordinate input (absolute, or relative with a leading `+`) with live parity check, fuel cost preview + confirmation
- Repair / Mine / Craft / Atomic printer craft — manny/target/recipe pickers
- Jettison / Salvage / Recall / Rename / Inspect / Recover / Detach — inventory/sector object pickers
- Deploy waypoint — 3-step wizard: pick manny → pick object → enter bookmark name
- Object actions — action picker for the selected scanner object (+ manny picker when several idle). An inactive `scut_relay` object offers **turn on relay** (pick manny → optional network name → `turn-on-relay`; needs a star in the sector + an integrated_circuit) and salvage.
- Jettison a `scut_relay` inventory item (Inventory `[j]` on the SCUT-relay group) — confirmation; deploys an inactive relay into the current sector
- Alerts (`[A]`) — tabbed Alerts / Damage-warnings list, `Tab` switches tab, `Enter` marks read; `[!]` badge on the probe panel + status bar when unread
- Missions (`[O]`) — active-mission list with steps and status; `[a]` abandons the selected active mission (confirmation sub-popup)
- Storage containers (`[C]` in Inventory) — container browser with capacity bars; `Enter` content view, `[n]` rename, `[e]` routing-rules editor (cycle each type none → priority → exclusion → strict)
- Storage move (`[M]` in Inventory) — pick actor manny → kind (resource / item) → source/destination + amount, or multi-select items + destination
- Drop cargo (`[X]` on a Manny waiting for space) — one-step confirmation (resource cargo is lost)
- Mind-snapshot reassign (`Ctrl+R`, only when the probe is dead or trapped by a black hole — `probe.alert` present) — confirmation; reassigns the mind snapshot to a fresh probe and resets the local frame to 0,0,0
- Waypoints (`[w]`) — known destinations from scan history (bookmarks, stars, minable), `Enter` → travel confirmation
- Inventory detail (`Enter` in inventory) — read-only detail of the selected row
- Map (`[b]`) — isometric sector overview: pan (`[hjkl/←↓↑→]`), `[u/d]` y±1, `[0]` recenter on probe, `[c]` jump to coords, `[g]` travel to center; info line (distance, ETA, sector summary) + legend; visited-but-unscanned sectors shown as `○`
- Help (`[?]`) — all keybindings grouped by context

## Implemented API endpoints (API v44)

| Endpoint | Method | Status |
|---|---|---|
| `/api/version` | GET | ✓ |
| `/api/probe` | GET | ✓ |
| `/api/probe/mannies` | GET | ✓ |
| `/api/probe/sector` | GET | ✓ |
| `/api/probe/mind-snapshot/reassign` | POST | ✓ |
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
| `/api/probe/mannies/{id}/drop-manny-cargo` | POST | ✓ |
| `/api/probe/inventory/{id}/jettison` | POST | ✓ |
| `/api/probe/atomic-printer/craft` | POST | ✓ |
| `/api/probe/alerts` | GET | ✓ |
| `/api/probe/alerts/{id}` | PATCH | ✓ (mark read) |
| `/api/probe/damage-warnings` | GET | ✓ |
| `/api/probe/damage-warnings/{id}` | PATCH | ✓ (mark read) |
| `/api/probe/storage-containers` | GET | ✓ |
| `/api/probe/storage-containers/{id}` | GET | ✓ |
| `/api/probe/storage-containers/{id}` | PATCH | ✓ (rename) |
| `/api/probe/storage-containers/{id}/rules` | PATCH | ✓ |
| `/api/probe/storage-moves` | POST | ✓ |
| `/api/crafting-recipes` | GET | ✓ |
| `/api/sector` | GET | ✓ |
| `/api/probe/visited-sectors` | GET | ✓ |
| `/api/probe/mannies/{id}/drop-storage-container` | POST | ✓ |
| `/api/probe/mannies/{id}/refill-deuterium-tank` | POST | ✓ |
| `/api/probe/mannies/{id}/turn-on-relay` | POST | ✓ |
| `/api/probe/missions` | GET | ✓ |
| `/api/probe/mission` | GET | ✓ (alias) |
| `/api/probe/missions/{id}/abandon` | POST | ✓ |
| `/api/probe/messages` | GET/POST | ✗ (not implemented) |
