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

Copy `config.example.toml` to that path and fill in the API key (generated once via the web UI). **First run needs no manual file**: if the config is missing or has no real key, the boot preflight prompts for an API key and writes `config.toml` for you (see Boot preflight).

Scan history is persisted across runs in a local SQLite database (`cockpit.db`, under the XDG state dir); the legacy `scan_history.json` is migrated into it once, then removed (`src/store.rs`, issue #134).

The same database holds the **ship's log** — an append-only `events` table recording pilot actions (travel, mine, deploy, drop/detach container, storage move…) as narrated captain's-log entries. Actions stage a pre-rendered line into `AppState::pending_journal` (mirroring `pending_fire`), which the event loop drains to persist and prepend to `AppState::journal`. The table is never trimmed (full history kept for long-term stats); only the most recent `store::JOURNAL_WINDOW` are loaded into memory at boot.

## Architecture

### Boot preflight (`src/preflight.rs` + `src/ui/preflight.rs`)

`main()` enters the alternate screen **before** any fallible startup, then runs `preflight::run()`, which draws the boot grid with the real check-list **inside the centre Probe pane** (the eight surrounding subsystems stay dark until the link comes up) and returns a `preflight::Ready` (config, `ApiClient`, DB connection, scan history, api_version, link_ok) or `Outcome::Quit`. This is the Windows first-run fix: `Config::load()` used to error out before the terminal existed, so a double-clicked binary flashed a console and vanished — now every failure has an in-TUI outcome. Steps:

- **CONFIG** — `Config::load_status()` returns `Ready` / `NeedsKey` / `Invalid` (a lenient parse so a keyless file doesn't error). On `NeedsKey`/`Invalid`, an onboarding prompt in the Probe pane collects an API key and `config::write_config()` writes `config.toml` (base URL defaulted to `DEFAULT_BASE_URL`); Esc/Ctrl-C quits cleanly.
- **ARCHIVE** — `store::open` + `migrate_legacy_json` (reports `MigrationOutcome`: imported N / already migrated / none) + `load_observations`.
- **REMOTE LINK** — `get_api_version()` under an 8 s timeout, retried interactively: a bad key or outage shows in the Probe pane with actions — `[R]` retry · `[K]` re-enter key (re-runs onboarding) · `[Enter]` continue offline. Continuing enters **degraded mode** (an error toast; `F5` retries), per the API-KO decision.

Once the link is up (or the pilot continues offline), it hands off to `run()`, which builds `AppState`, spawns the persistence writer from the preflight's connection, and plays the cosmetic boot animation that lights the eight subsystems centre-out (see UI › Boot).

### Event loop (`src/main.rs`)

Single `tokio::select!` loop over three sources:

- **crossterm `EventStream`** — keyboard / mouse / resize events
- **`mpsc::Receiver<ApiMessage>`** — results from spawned API tasks
- **`tokio::time::sleep_until(deadline)`** — auto-refresh timer

The timer deadline is set to `movement.arrival_at` (ISO 8601) converted to a `tokio::time::Instant`. When no movement is in progress the deadline is 24 h away — no polling.

Two more `select!` branches are timers: a **short-lived ~90 ms boot tick** (guarded by `state.booting`, runs only during the startup boot sequence — see UI › Boot — then stops) and a **steady-state 1 s `ui_tick`** that redraws so time-derived values (progress bars, ETAs, sync age) advance and fires the periodic auto-refresh when one is due.

`fetch_all()` spawns **seven** independent `tokio::spawn` tasks: probe, mannies, sector, visited sectors, alerts, damage warnings, and missions. All but probe are non-fatal.

All other API calls (move, repair, mine, craft, storage container CRUD, storage moves, etc.) are also spawned tasks that send results back via the `mpsc::Sender<ApiMessage>`. Keyboard handlers live in `src/input/` (`mod.rs` holds `handle_event`, which runs the shared wizard/overlay handlers then dispatches navigation to `cockpit.rs`; one module per wizard handler — `pickers.rs` groups the manny pick-list/confirm ones, `containers.rs` the storage-container ones, `storage_move.rs`, `alerts.rs`, `geometry.rs` for the scan offset helpers); fetch spawners live in `src/api/tasks.rs`. `main.rs` only contains the select loop and the `ApiMessage` dispatch (which also sets the success toasts).

### State (`src/app/`)

`AppState` is the single source of truth passed to the renderer. Split by domain: `mod.rs` (struct `AppState`, core impl — updates, toasts, refresh deadline — and `pub use` re-exports keeping `crate::app::*` paths stable), `grid.rs` (`Pane` — the 9 cockpit panes — + `PaneNav` per-pane cursor/drill state + grid navigation helpers), `mode.rs` (`InputMode`, `ContextMenu`, `MenuAction`, `MenuItem`), `boot.rs` (startup self-check schedule), `color.rs` (`ColorMode`), `inputs.rs` (all wizard input enums + constants), `scan.rs`, `travel.rs`, `inventory.rs`, `mannies.rs`, `containers.rs` (storage-container/move helpers), `map.rs`, `waypoints.rs`, `message.rs` (`ApiMessage`), `journal.rs` (`LogEvent` — the ship's-log entry type + narrative constructors), `tests.rs` (unit tests). Key design choices:

- **Cockpit v2 state**: `active_pane: Pane`, `zoomed: bool`, `mode: InputMode` (`Normal` / `Menu` / `Command`), `pane_nav: [PaneNav; 9]` (cursor + drill-in stack per pane), `hints_visible`, `color_mode`, and `booting` / `boot_frame` for the startup sequence. `build_context_menu()` produces the `Enter` menu for the active pane; menu items map to `MenuAction`s that launch the existing wizards.
- The four reused panel renderers (Probe/Inventory/Scanner/Mannies) take an `active: bool` to mark the active pane; the old `Panel` enum + `focused` state and the classic single-key action handlers are gone.

- Each interactive action (travel, repair, mine, fabricate, jettison, salvage, recall, rename, deploy, inspect, recover, detach, object actions, waypoints, alerts, storage containers + rename + routing rules, storage moves, drop cargo) has its own input state enum (`TravelInput`, `RepairInput`, `ObjectActionInput`, `AlertsInput`, `ContainersInput`, `ContainerRulesInput`, `StorageMoveInput`, etc.) with variants for each wizard step. All start as `Inactive`.
- **Fabrication is unified** (`FabricationInput`): one item-first catalog spanning both fabricators, replacing the old separate `CraftInput` (Mannies pane) + `AtomicPrinterCraftInput` (Inventory pane). `fabrication_recipes()` returns every recipe paired with its `Fabricator` (atomic printer recipes first, then Manny), sectioned in the overlay. `PickRecipe` browses the catalog; committing an atomic recipe fires `atomic-printer/craft` (the printer auto-reserves a Manny), a Manny recipe either uses a pre-chosen builder (when opened from the Mannies pane on an orderable Manny), auto-picks the sole idle onboard Manny, or advances to `PickBuilder`. Reachable from the Inventory pane menu, the Mannies pane menu (pre-fills that Manny), and `:craft`.
- `update_probe()` extracts `movement_arrival` from the response and stores it separately so the event loop can compute the next deadline without re-reading the full probe struct.
- Scan history is cached in `AppState::scan_history` (a `Vec<SectorObservation>`) and persisted to disk asynchronously after each sector fetch. Each observation is stamped on receipt with a local `scanned_at` and an `observed_by` probe id (both serde-defaulted, so old history files load). The history is a **unified fleet-wide store** — SCUT shares sector knowledge across probes, so switching the active probe never partitions or clears it; `observed_by` is provenance only. The SQLite `observed_by` column is back-filled on existing DBs via `store::ensure_columns` (`ALTER TABLE ADD COLUMN`).
- Panel cursors: `mannies_selection`, `inventory_selection` (rows built by `inventory_rows()` — stocks, active items, passive groups), `scan_history_idx` (moves within `filtered_history_indices()` when a `ScanFilter` is active), `scanner_obj_selection` (object-browsing mode, entries from `scanner_objects()`).
- `jettison_for_selected()` builds the jettison wizard from the selected inventory row; `actions_for_object()` maps a `ScannerObjectEntry` to its available `ObjectAction`s, mirroring the manny-first candidate sets (`collect_*_candidates`).
- Transient success toasts: `set_toast()` / `active_toast()` (5 s expiry, dismissed by any keypress).
- `RESOURCE_TYPES`, `MOVE_RESOURCE_TYPES`, and `DETACH_MODES` constants live here (not in `main.rs` or the UI).

### API layer (`src/api/`)

- `types.rs` — all OpenAPI types. Structs use `#[serde(rename_all = "camelCase")]`; enums use `#[serde(rename_all = "snake_case")]` with `#[serde(other)] Unknown` fallbacks. `#![allow(dead_code)]` suppresses warnings on fields not yet consumed by the UI.
- `client.rs` — `ApiClient` (cloneable, wraps `reqwest::Client`). Each endpoint has a typed wrapper with an inline `struct Resp` that deserializes the envelope and returns the inner value. HTTP errors extract `error.message` from the JSON body; 401 produces a specific "check your api_key" message.
  - **Multi-probe seam (API v81)**: `ApiClient` carries an `active_probe_id: Option<u64>`; per-probe wrappers build their path via `probe_path(suffix)` — `None` → `/api/probe{suffix}` (default probe, pre-v81 behaviour), `Some(id)` → `/api/probe/{id}{suffix}` (the `{probeId}` mirrors). `with_active_probe(id)` returns a retargeted clone (used by the cockpit to switch probes without touching the server default). Player-level endpoints (`/api/version`, `/api/sector`, `/api/crafting-recipes`, `/api/probes`, `/api/probe/missions*`, `/api/probe/messages/sent`, `/api/probe/mind-snapshot/reassign`) have **no** `{probeId}` mirror and stay literal.

### Theme & colours (`src/ui/theme.rs`)

One unified phosphor theme (there is no classic/retro split any more). `theme.rs` holds the shared helpers: `Palette` + `palette(ColorMode)` (accent / dim / text / good / warn / crit per color mode), `pane_block(title, active, palette)` — the double-line (`BorderType::Double`) pane frame used by every pane, coloured accent when active and dim-accent otherwise — plus icons, labels, gauges (`make_line_gauge`, `gauge_color`), and `format_duration` / `format_age`. Color modes: `mono-green` (default), `mono-amber`, `phosphor-semantic`, `modern-16`; `F2` cycles them.

### UI (`src/ui/`)

`ui::render` → `cockpit_v2::render(frame, state)` is the single render entry point. Module layout: `cockpit_v2/` (`mod.rs` entry point — grid layout, status bar, boot screen; `grid.rs` responsive window; `panes.rs` compact renderers for the five promoted panes; `menu.rs` contextual-menu popup), `panels/` (the four original panel renderers, reused by the grid: `probe`, `inventory`, `scanner`, `mannies`), `overlays/` (one file per wizard overlay; `pickers.rs` groups the manny pick-list/confirm ones, `containers.rs` the storage-container ones, plus `alerts.rs` / `storage_move.rs`; `mod.rs` hosts `centered_rect` + `render_pick_list`), `theme.rs`, `sigil.rs`.

**Probe sigil** (`src/ui/sigil.rs`, API v81) — `probe_sigil(id)` builds a deterministic 7×7 mirror-symmetric identicon (FNV-1a over the id, stable across runs — **not** the randomized default hasher; 7×4 = 28 free bits ≈ 268M patterns). `sigil_lines(id, palette, indent)` renders it as 4 half-block (`▀▄█`) lines in the accent colour. The Probe pane pins the **active probe's** sigil to its top-right corner, always visible in every mode/theme, so probes/drones are told apart at a glance (`panels/probe.rs`).

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
 NAV  COCKPIT › MANNIES        ⟳ · ≣ SCUT · ! 2 · API vN · 14:09
 ↑↓ move · hl drill · z zoom · Enter act · ertdfgcvb pane · F1 hints
```

**Keys** — `e r t d f g c v b` activate a pane · `j`/`k` (`↑`/`↓`) move the cursor · `l`/`→` drill in, `h`/`←` drill out (Missions → steps, Comms → message) · `Enter` opens the contextual action menu · `z` zooms the active pane full-screen · `Tab`/`Shift+Tab` cycle panes · `:` command mode · `F1` toggle hints · `F2` cycle color mode · `F5` refresh · `?` help · `q` quit · `Esc` closes menu / leaves zoom / drills up.

**Contextual menu** (`Enter`) — built per active pane + selection (`build_context_menu` → `Vec<MenuItem>`, disabled items shown with a reason). Firing an item launches the existing wizard (`MenuAction` → the matching `*Input`). Panes with rich wizards (Missions, Comms, Storage, Sector objects) reuse their legacy overlays instead of the popup.

**Responsive** — `grid::visible_panes` fits `rows × cols` whole panes (each 1..=3 from a minimum cell size) and slides the window to keep the active pane visible: 3×3 on a large terminal, 2×2 on a half-screen, a single row on a short wide split, one pane when tiny. A position mini-map in the status bar (the nine keys in three groups) shows where the active pane sits whenever the grid is reduced.

**Status bar** — `[MODE]` tag (NAV / MENU / CMD, or ZOOM) · breadcrumb (`COCKPIT › PANE › …`) · transient error (crit) or success toast · right-aligned meta (`⟳` while loading, active probe `⏻ name` when the fleet has >1 probe or the pilot is off the default — accented when non-default, `⚠` when unreachable, `≣ SCUT`, unread `! n`, `API vN`, clock). A second **hints line** (toggle `F1`) shows the keys valid for the active pane.

**Boot** (`src/app/boot.rs` + `cockpit_v2::render_boot`) — on startup the probe core boots first (centre pane self-check), then the eight subsystems come online centre-out, each typing a themed teletype self-check (SUDDAR array, SCUT link, autofactory, manny bay…). Once done it holds on `ANY KEY TO CONTINUE` in the centre pane; any key drops into the live cockpit (or skips the animation). Driven by the bounded boot tick.

The four reused panels (Probe / Inventory / Scanner / Mannies) keep their internal content colours; gauge colors: green > 50 %, yellow 25–50 %, red < 25 %. Movement progress is derived from `started_at` / `arrival_at` client-side. Scanner history shows symbol + coords + distance and scrolls with the selection.

**Overlays** (wizards, rendered on top of the grid; launched from the contextual menu or the reused panels):
- **Mannies pane menu** (`Enter`) — Fabricate, Mine, Repair, Salvage, Inspect, Recover/Detach container, Refill deuterium, Drop cargo, **Assemble probe…**, Recall/Abandon, Rename. **Assemble probe…** (API v81; enabled on an orderable Manny with ≥2 empty additional containers) opens the assembly wizard (`AssembleProbeInput`): a single multi-select of exactly two empty containers with the fixed component bill shown; `Space` toggles, `Enter` fires the ~3h `assemble-probe` task that spawns a new drone in the sector (`fetch_assemble_probe`). Each launches its wizard (`*Input`). Fabricate opens the unified catalog with the selected Manny pre-chosen as builder. Inspect (`inspect-sector-object`, API v65) targets any inspectable object — asteroid, dormant construct, or detached container (`collect_inspectable_candidates`). Remote mine (SCUT-reachable manny) fetches the manny's sector first, then picks asteroid → resources/amount → mandatory detached container. Recall on a SCUT-remote manny is labelled **abandon**. A mining Manny that turns up a hidden container flags **⚠ hidden container found** in its detail (`manny_artificial_detection` reads `artificialObjectDetected` from the task payload).
- **Inventory pane menu** (`Enter`) — Fabricate, Move stock, Jettison, and **Deploy waypoint…** (only when a `waypoint_bookmark` is held; picks the installing Manny → sector target → name, firing `install-bookmark`). Fabricate opens the unified catalog with no builder pre-chosen.
- **Missions pane** — a root with two categories (`DrillLevel::MissionsCat`, drilled like Comms): **Missions** (active-mission list with steps/status; `Enter` → abandon confirmation) and **Ship's log** (the captain's-log flow — newest-first narrated entries, dimmed timestamp, entities in accent, server events in warn; lines truncate to the pane width when compact and read out in full when zoomed). `l`/`Enter` at the root enters a category, `h`/`Esc` backs out. Parked in the Missions pane for now — modular so it can graduate to its own pane later.
- **Comms pane** — a category root (`DrillLevel::CommsCat`): **Messages**, **Alerts**, **Warnings**, each with its unread count and a one-line preview of its most recent entry. `l`/`Enter` on a category: **Messages** opens the messaging overlay (inbox/sent tabs, `Enter` reads a message full-screen — body + emission sector `ProbeMessage.sector`, API v71 —, `c` composes to a probe/planet recipient); **Alerts** / **Warnings** drill into an in-pane list rendered right in the pane, where `Enter` acknowledges the selected entry. `h`/`Esc` backs out to the root.
- **Storage pane** (`Enter`) — container browser with capacity bars; content view, rename, routing-rules editor (none → priority → exclusion → strict).
- **Sector pane** (`Enter` on an object) — object-action picker (mine / inspect / salvage / recover / deploy waypoint); inspect is offered on asteroids, **dormant constructs**, and detached containers; an inactive `scut_relay` offers **turn on relay** (needs a star + integrated_circuit) and salvage.
- **Map pane** — compact summary; `z` opens the full isometric map (pan `hjkl`, `g` travel to the centred sector, `c` coordinate center). `Enter` menu: open map, **Travel to coordinates…**, **Jump to visited sector…** (picker over `visited_sectors`), **Waypoints…** (picker over bookmarks/stars/mineable targets). Scanner `Enter` also offers **Travel here** to the selected observation.
- **Travel** wizard — coordinate input (absolute, or relative with a leading `+`), live parity check, fuel/ETA preview + confirmation. Launched from Map/Scanner or `:travel`.
- **Probe pane menu** (`Enter`) — **Switch probe…** (multi-probe, API v81; enabled with >1 probe, opens the fleet picker), **Set as default probe** (`PATCH /api/probe/{id}` `isDefault`; shown when the active probe isn't default, disabled + reasoned when it's out of SCUT range), **Rename probe…** (`PATCH /api/probe/{id}` `name`, text-entry wizard `RenameProbeInput`; renames the piloted probe, available even with a single probe), **Inspect SCUT network…** (enabled when a SCUT relay covers the current sector; auto-views the sole network or picks among several via `scut-network/{id}`), **Improve probe…** (enabled when an unlocked, not-yet-done improvement exists; two-panel catalog → resolve the installing Manny → `improve-probe`), plus **Reassign mind snapshot** only when the probe is dead or trapped by a black hole (`probe.alert`); reassigns the snapshot to a fresh probe. Improvements are fetched in `fetch_all` (`ProbeImprovement`).
  - **Fleet switching (API v81)**: the active probe is client-side (`AppState::active_probe_id`, `None` = default). The picker (`ProbeSwitchInput`, `render_probe_switch_overlay`) lists the roster with default (★) / active (▸) markers, status, and SCUT reachability; `Enter` pilots a reachable probe (refuses an unreachable one with a toast). Switching only sets `active_probe_id`; the event loop in `main.rs` reconciles the `ApiClient` (`with_active_probe`) and refetches. Also reachable via `:probe <id|name>`. The Probe panel (`panels/probe.rs`) flags a piloted non-default drone (▸ accented name), shows a compact `fleet N probes` line, and unfolds the full roster (default ★ / active ▸ / SCUT reach) when zoomed (`z`) — a fleet cockpit. All fleet UI is gated on `fleet.len() > 1`, so single-probe play is unchanged.
- **Command mode** (`:`) — `focus <pane>` · `travel <x y z|+dx dy dz>` · `goto <x y z>` · `filter <all|objects|minable|danger>` · `craft [recipe]` · `mine [res[,res]] [amount] [by <manny>] [at <asteroid>] [to <container>]` · `probe <id|name>` · `refresh` · `theme <mode>` · `zoom` · `help` · `q`. `Tab` completes and cycles both the verb and enumerable arguments (`AppState::command_completions`; candidate list / usage ghost-text shown on the command line), `↑`/`↓` browse history (`AppState::command_history`), the recognised verb's argument usage comes from `command_usage`; verbs live in `AppState::run_command` (`app/command.rs`). `craft`/`mine` are **hybrid**: bare opens the wizard, a full line fires directly. Since `run_command` owns no `ApiClient`/sender, a one-shot stages a `CommandFire` in `AppState::pending_fire` that the input layer (`input/command.rs`) drains and spawns (`fetch_craft` / `fetch_atomic_printer_craft` / `fetch_mine`). `:mine` resolves the builder/asteroid from context (sole idle onboard Manny, sole mineable object), overridable via `by`/`at`, destination defaulting to the probe (`to <container>` to redirect); local mining only.
- Shared bits: `EndpointId` is an untagged int|string (probe id | planet object id). `Manny.taskVisibility` (`local` / `scut_network` / `too_far`) drives remote display (`≣ via SCUT` / `too far`).

**All wizards are now wired to a cockpit launcher** — Travel, the full isometric Map (`z` on the Map pane), Waypoints, mind-snapshot reassign, drop-storage-container, SCUT-network inspect (Probe pane), deploy-waypoint (Inventory pane), and command mode (`:`).

## Implemented API endpoints

<!-- The live server version shows as "API vN" in the status bar; the versioned
     contract lives in api-specs/. Keep this table, but not a hardcoded version. -->


| Endpoint | Method | Status |
|---|---|---|
| `/api/version` | GET | ✓ |
| `/api/probes` | GET | ✓ (fleet roster, API v81) |
| `/api/probe` | GET | ✓ |
| `/api/probe/{probeId}` | PATCH | ✓ (rename / set default, API v81) |
| `/api/probe/mannies/{id}/assemble-probe` | POST | ✓ (assemble a drone, API v81) |
| `/api/probe/mannies` | GET | ✓ |
| `/api/probe/sector` | GET | ✓ |
| `/api/probe/mind-snapshot/reassign` | POST | ✓ |
| `/api/probe/move` | POST | ✓ |
| `/api/probe/mannies/{id}/repair` | POST | ✓ |
| `/api/probe/mannies/{id}/mine` | POST | ✓ |
| `/api/probe/mannies/{id}/craft` | POST | ✓ |
| `/api/probe/mannies/{id}/improve-probe` | POST | ✓ |
| `/api/probe/probe-improvements-available` | GET | ✓ |
| `/api/probe/mannies/{id}/salvage` | POST | ✓ |
| `/api/probe/mannies/{id}/recall` | POST | ✓ |
| `/api/probe/mannies/{id}` | PATCH | ✓ (rename) |
| `/api/probe/mannies/{id}/install-bookmark` | POST | ✓ |
| `/api/probe/mannies/{id}/inspect-sector-object` | POST | ✓ |
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
