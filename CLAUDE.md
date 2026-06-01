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

Copy `config.example.toml` to that path and fill in the API key (generated once via the web UI, shown only at creation time).

## Architecture

### Event loop (`src/main.rs`)

Single `tokio::select!` loop over three sources:

- **crossterm `EventStream`** — keyboard / mouse / resize
- **`mpsc::Receiver<ApiMessage>`** — results from spawned API tasks
- **`tokio::time::sleep_until(deadline)`** — auto-refresh timer

The timer deadline is set to `movement.arrival_at` (ISO 8601 from the API) converted to a `tokio::time::Instant`. When no movement is in progress the deadline is 24 h away — no polling, no tick loop.

`fetch_all()` spawns two independent `tokio::spawn` tasks (probe + mannies); mannies failure is non-fatal.

### State (`src/app.rs`)

`AppState` is the single source of truth passed to the renderer. `update_probe()` extracts `movement_arrival` from the response and stores it separately so the event loop can compute the next deadline without re-reading the full probe struct.

### API layer (`src/api/`)

- `types.rs` — all OpenAPI types. Structs use `#[serde(rename_all = "camelCase")]`; enums use `#[serde(rename_all = "snake_case")]` with `#[serde(other)] Unknown` fallbacks. `#![allow(dead_code)]` suppresses warnings on fields not yet consumed by the UI.
- `client.rs` — `ApiClient` (cloneable, wraps `reqwest::Client`). Each endpoint has a typed wrapper that deserializes the envelope and returns the inner value.

### UI (`src/ui/cockpit.rs`)

`render(frame, state)` is the single render entry point called every loop iteration. Layout:

```
┌─ NEUMANN COCKPIT ──────────────────────────────────────────┐
│  ┌─ PROBE (45%) ──────────┐  ┌─ MANNIES (55%) ───────────┐ │
│  │ name + status          │  │ ● manny-1  idle           │ │
│  │ sensor mode dot        │  │ ◌ manny-2  mining   42%   │ │
│  │ movement + ETA gauge   │  │                           │ │
│  │ fuel gauge             │  │                           │ │
│  │ integrity gauge        │  │                           │ │
│  │ cargo gauge            │  │                           │ │
│  └────────────────────────┘  └───────────────────────────┘ │
│ [r] refresh  [q] quit          ⟳ HH:MM:SS   next: Xm Ys   │
└────────────────────────────────────────────────────────────┘
```

Gauge colors: green > 50 %, yellow 25–50 %, red < 25 %. Sensor mode and probe status are colour-coded. Movement progress is derived from `started_at` / `arrival_at` timestamps client-side (more accurate than the API's `secondsRemaining` snapshot).

The layout is designed to be extended: the right column is a single `Rect` today and will be split vertically to add a SECTOR panel above MANNIES.
