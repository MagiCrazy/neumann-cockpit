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
theme    = "mono-green"   # color mode (optional)
hints    = true           # show the contextual hints line (optional)
```

- `theme` — cockpit color mode: `mono-green` (default), `mono-amber`, `phosphor-semantic` (green base + green/yellow/red status), or `modern-16` (named ANSI for terminals without truecolor). `F2` cycles it at runtime.
- `hints` — show the contextual hints line at the bottom (`F1` toggles at runtime). Defaults `true`.

Unknown keys are ignored, so legacy configs (`ui`, `phosphor`, `animations`, `theme = "retro"`) still load.

Copy `config.example.toml` to that path and fill in the API key (generated once via the web UI).

Scan history is persisted across runs to `~/.config/neumann-cockpit/scan_history.json`.

## Architecture

### Event loop (`src/main.rs`)

Single `tokio::select!` loop over three sources:

- **crossterm `EventStream`** — keyboard / mouse / resize events
- **`mpsc::Receiver<ApiMessage>`** — results from spawned API tasks
- **`tokio::time::sleep_until(deadline)`** — auto-refresh timer

The timer deadline is set to `movement.arrival_at` (ISO 8601) converted to a `tokio::time::Instant`. When no movement is in progress the deadline is 24 h away — no polling.

A fourth `select!` branch is a **short-lived ~90 ms boot tick**, guarded by `state.booting`: it only runs during the startup boot sequence (see UI › Boot), then stops. Steady state stays fully event-driven — no tick.

`fetch_all()` spawns **seven** independent `tokio::spawn` tasks: probe, mannies, sector, visited sectors, alerts, damage warnings, and missions. All but probe are non-fatal.

All other API calls (move, repair, mine, craft, storage container CRUD, storage moves, etc.) are also spawned tasks that send results back via the `mpsc::Sender<ApiMessage>`. Keyboard handlers live in `src/input/` (`mod.rs` holds `handle_event`, which runs the shared wizard/overlay handlers then dispatches navigation to `cockpit.rs`; one module per wizard handler — `pickers.rs` groups the manny pick-list/confirm ones, `containers.rs` the storage-container ones, `storage_move.rs`, `alerts.rs`, `geometry.rs` for the scan offset helpers); fetch spawners live in `src/api/tasks.rs`. `main.rs` only contains the select loop and the `ApiMessage` dispatch (which also sets the success toasts).

### State (`src/app/`)

`AppState` is the single source of truth passed to the renderer. Split by domain: `mod.rs` (struct `AppState`, core impl — updates, toasts, refresh deadline — and `pub use` re-exports keeping `crate::app::*` paths stable), `grid.rs` (`Pane` — the 9 cockpit panes — + `PaneNav` per-pane cursor/drill state + grid navigation helpers), `mode.rs` (`InputMode`, `ContextMenu`, `MenuAction`, `MenuItem`), `boot.rs` (startup self-check schedule), `color.rs` (`ColorMode`), `inputs.rs` (all wizard input enums + constants), `scan.rs`, `travel.rs`, `inventory.rs`, `mannies.rs`, `containers.rs` (storage-container/move helpers), `map.rs`, `waypoints.rs`, `message.rs` (`ApiMessage`), `tests.rs` (unit tests). Key design choices:

- **Cockpit v2 state**: `active_pane: Pane`, `zoomed: bool`, `mode: InputMode` (`Normal` / `Menu` / `Command`), `pane_nav: [PaneNav; 9]` (cursor + drill-in stack per pane), `hints_visible`, `color_mode`, and `booting` / `boot_frame` for the startup sequence. `build_context_menu()` produces the `Enter` menu for the active pane; menu items map to `MenuAction`s that launch the existing wizards.
- The legacy `Panel` enum + `focused` field remain, still used by the four reused panel renderers to mark the active pane; the classic single-key action handlers are gone.

- Each interactive action (travel, repair, mine, craft, jettison, salvage, recall, rename, deploy, inspect, recover, detach, atomic printer craft, object actions, waypoints, alerts, storage containers + rename + routing rules, storage moves, drop cargo) has its own input state enum (`TravelInput`, `RepairInput`, `ObjectActionInput`, `AlertsInput`, `ContainersInput`, `ContainerRulesInput`, `StorageMoveInput`, etc.) with variants for each wizard step. All start as `Inactive`.
- `update_probe()` extracts `movement_arrival` from the response and stores it separately so the event loop can compute the next deadline without re-reading the full probe struct.
- Scan history is cached in `AppState::scan_history` (a `Vec<SectorObservation>`) and persisted to disk asynchronously after each sector fetch. Each observation is stamped with a local `scanned_at` on receipt (serde-defaulted, so old history files load).
- Panel cursors: `mannies_selection`, `inventory_selection` (rows built by `inventory_rows()` — stocks, active items, passive groups), `scan_history_idx` (moves within `filtered_history_indices()` when a `ScanFilter` is active), `scanner_obj_selection` (object-browsing mode, entries from `scanner_objects()`).
- `jettison_for_selected()` builds the jettison wizard from the selected inventory row; `actions_for_object()` maps a `ScannerObjectEntry` to its available `ObjectAction`s, mirroring the manny-first candidate sets (`collect_*_candidates`).
- Transient success toasts: `set_toast()` / `active_toast()` (5 s expiry, dismissed by any keypress).
- `RESOURCE_TYPES`, `MOVE_RESOURCE_TYPES`, and `DETACH_MODES` constants live here (not in `main.rs` or the UI).

### API layer (`src/api/`)

- `types.rs` — all OpenAPI types. Structs use `#[serde(rename_all = "camelCase")]`; enums use `#[serde(rename_all = "snake_case")]` with `#[serde(other)] Unknown` fallbacks. `#![allow(dead_code)]` suppresses warnings on fields not yet consumed by the UI.
- `client.rs` — `ApiClient` (cloneable, wraps `reqwest::Client`). Each endpoint has a typed wrapper with an inline `struct Resp` that deserializes the envelope and returns the inner value. HTTP errors extract `error.message` from the JSON body; 401 produces a specific "check your api_key" message.

### Theme & colours (`src/ui/theme.rs`)

One unified phosphor theme (there is no classic/retro split any more). `theme.rs` holds the shared helpers: `Palette` + `palette(ColorMode)` (accent / dim / text / good / warn / crit per color mode), `pane_block(title, active, palette)` — the double-line (`BorderType::Double`) pane frame used by every pane, coloured accent when active and dim-accent otherwise — plus icons, labels, gauges (`make_line_gauge`, `gauge_color`), and `format_duration` / `format_age`. Color modes: `mono-green` (default), `mono-amber`, `phosphor-semantic`, `modern-16`; `F2` cycles them.

### UI (`src/ui/`)

`ui::render` → `cockpit_v2::render(frame, state)` is the single render entry point. Module layout: `cockpit_v2/` (`mod.rs` entry point — grid layout, status bar, boot screen; `grid.rs` responsive window; `panes.rs` compact renderers for the five promoted panes; `menu.rs` contextual-menu popup), `panels/` (the four original panel renderers, reused by the grid: `probe`, `inventory`, `scanner`, `mannies`), `overlays/` (one file per wizard overlay; `pickers.rs` groups the manny pick-list/confirm ones, `containers.rs` the storage-container ones, plus `alerts.rs` / `storage_move.rs`; `mod.rs` hosts `centered_rect` + `render_pick_list`), `theme.rs`.

**The grid** — a 3×3 tiling dashboard of nine panes, each addressable by a key in the `e r t / d f g / c v b` square (identical on AZERTY and QWERTY; centre `f` = Probe). Model: *navigate then act*.

```
┌═ SCANNER ═════┐┌═ MAP ═════════┐┌═ COMMS ═══════┐
│ scan history  ││ sector coords ││ alerts / msgs │
│ + distances   ││ ≣ SCUT        ││ unread count  │
└═══════════════┘└═══════════════┘└═══════════════┘
┌═ SECTOR ══════┐┌═ PROBE ═══════┐┌═ MISSIONS ════┐
│ objects here  ││ status · fuel ││ active list   │
│ (drill → obj) ││ integrity · ETA││ (drill → steps)│
└═══════════════┘└═══════════════┘└═══════════════┘
┌═ INVENTORY ═══┐┌═ STORAGE ═════┐┌═ MANNIES ═════┐
│ cargo · stocks││ containers    ││ ● manny list  │
│ items         ││ + capacity    ││ task + %      │
└═══════════════┘└═══════════════┘└═══════════════┘
 NAV  COCKPIT › MANNIES        ⟳ · ≣ SCUT · ! 2 · API v63 · 14:09
 ↑↓ move · hl drill · z zoom · Enter act · ertdfgcvb pane · F1 hints
```

**Keys** — `e r t d f g c v b` activate a pane · `j`/`k` (`↑`/`↓`) move the cursor · `l`/`→` drill in, `h`/`←` drill out (Missions → steps, Comms → message) · `Enter` opens the contextual action menu · `z` zooms the active pane full-screen · `Tab`/`Shift+Tab` cycle panes · `:` command mode (planned) · `F1` toggle hints · `F2` cycle color mode · `F5` refresh · `?` help · `q` quit · `Esc` closes menu / leaves zoom / drills up.

**Contextual menu** (`Enter`) — built per active pane + selection (`build_context_menu` → `Vec<MenuItem>`, disabled items shown with a reason). Firing an item launches the existing wizard (`MenuAction` → the matching `*Input`). Panes with rich wizards (Missions, Comms, Storage, Sector objects) reuse their legacy overlays instead of the popup.

**Responsive** — `grid::visible_panes` fits `rows × cols` whole panes (each 1..=3 from a minimum cell size) and slides the window to keep the active pane visible: 3×3 on a large terminal, 2×2 on a half-screen, a single row on a short wide split, one pane when tiny. A position mini-map in the status bar (the nine keys in three groups) shows where the active pane sits whenever the grid is reduced.

**Status bar** — `[MODE]` tag (NAV / MENU / CMD, or ZOOM) · breadcrumb (`COCKPIT › PANE › …`) · transient error (crit) or success toast · right-aligned meta (`⟳` while loading, `≣ SCUT`, unread `! n`, `API vN`, clock). A second **hints line** (toggle `F1`) shows the keys valid for the active pane.

**Boot** (`src/app/boot.rs` + `cockpit_v2::render_boot`) — on startup the probe core boots first (centre pane self-check), then the eight subsystems come online centre-out, each typing a themed teletype self-check (SUDDAR array, SCUT link, autofactory, manny bay…). Once done it holds on `ANY KEY TO CONTINUE` in the centre pane; any key drops into the live cockpit (or skips the animation). Driven by the bounded boot tick.

The four reused panels (Probe / Inventory / Scanner / Mannies) keep their internal content colours; gauge colors: green > 50 %, yellow 25–50 %, red < 25 %. Movement progress is derived from `started_at` / `arrival_at` client-side. Scanner history shows symbol + coords + distance and scrolls with the selection.

**Overlays** (wizards, rendered on top of the grid; launched from the contextual menu or the reused panels):
- **Mannies pane menu** (`Enter`) — Repair, Mine, Craft, Salvage, Inspect, Recover/Detach container, Refill deuterium, Drop cargo, Recall/Abandon, Rename. Each launches its wizard (`*Input`). Remote mine (SCUT-reachable manny) fetches the manny's sector first, then picks asteroid → resources/amount → mandatory detached container. Recall on a SCUT-remote manny is labelled **abandon**.
- **Inventory pane menu** (`Enter`) — Jettison, Atomic printer craft, Move stock.
- **Missions pane** (`Enter`) — active-mission list with steps/status; abandon (confirmation).
- **Comms pane** (`Enter`) — messaging inbox/sent (mark read, compose to a probe/planet recipient); alerts + damage-warnings live in the same pane.
- **Storage pane** (`Enter`) — container browser with capacity bars; content view, rename, routing-rules editor (none → priority → exclusion → strict).
- **Sector pane** (`Enter` on an object) — object-action picker (mine / inspect / salvage / recover / deploy waypoint); an inactive `scut_relay` offers **turn on relay** (needs a star + integrated_circuit) and salvage.
- **Travel** wizard — coordinate input (absolute, or relative with a leading `+`), live parity check, fuel/ETA preview + confirmation.
- **Mind-snapshot reassign** — only when the probe is dead or trapped by a black hole (`probe.alert`); reassigns the snapshot to a fresh probe.
- Shared bits: `EndpointId` is an untagged int|string (probe id | planet object id). `Manny.taskVisibility` (`local` / `scut_network` / `too_far`) drives remote display (`≣ via SCUT` / `too far`).

**Not yet wired into the cockpit** (wizards that exist but have no launcher in the new interface — follow-ups): Travel, the full isometric Map overlay (the Map pane is a compact summary), Waypoints, SCUT-network inspect, mind-snapshot reassign, deploy-waypoint from Inventory, and drop-storage-container. Command mode (`:`) will cover several of these.

## Implemented API endpoints (API v63)

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
| `/api/probe/scut-network/{id}` | GET | ✓ |
| `/api/probe/missions` | GET | ✓ |
| `/api/probe/mission` | GET | ✓ (alias) |
| `/api/probe/missions/{id}/abandon` | POST | ✓ |
| `/api/probe/messages` | GET/POST | ✓ |
| `/api/probe/messages/sent` | GET | ✓ |
| `/api/probe/messages/{id}/read` | PATCH | ✓ |
