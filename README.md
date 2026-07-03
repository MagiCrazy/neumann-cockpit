# neumann-cockpit

[![CI](https://github.com/MagiCrazy/neumann-cockpit/actions/workflows/ci.yml/badge.svg)](https://github.com/MagiCrazy/neumann-cockpit/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/MagiCrazy/neumann-cockpit)](https://github.com/MagiCrazy/neumann-cockpit/releases/latest)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)

A terminal UI cockpit for [Von Neumann Game](https://github.com/gnieark/Von-Neumann-Game) — control your probe and its mannies from the command line.

The official game instance runs at **[https://neumann-probe.net](https://neumann-probe.net)** (default endpoint).

## Install

**Prebuilt binaries** for Linux, macOS, and Windows are available on the [releases page](https://github.com/MagiCrazy/neumann-cockpit/releases/latest).

Every archive ships a matching `.sha256` — download, verify, then extract:

```bash
# Linux x86_64 example
base=https://github.com/MagiCrazy/neumann-cockpit/releases/latest/download
curl -sLO "$base/neumann-cockpit-linux-x86_64.tar.gz"
curl -sLO "$base/neumann-cockpit-linux-x86_64.tar.gz.sha256"
sha256sum -c neumann-cockpit-linux-x86_64.tar.gz.sha256
tar xzf neumann-cockpit-linux-x86_64.tar.gz
./neumann-cockpit
```

## Quickstart

**1. Get an API key** — create an account on [neumann-probe.net](https://neumann-probe.net), go to Settings and generate an API key. It is shown only once.

**2. Configure**

```bash
mkdir -p ~/.config/neumann-cockpit
cp config.example.toml ~/.config/neumann-cockpit/config.toml
# then edit the file and paste your API key
```

```toml
base_url = "https://neumann-probe.net"
api_key  = "vng_your_api_key_here"
# theme = "mono-green"   # color mode: mono-green | mono-amber | phosphor-semantic | modern-16
# hints = true           # show the contextual hints line
```

**3. Run**

```bash
neumann-cockpit
```

## Interface

A single **phosphor cockpit**: a 3×3 tiling dashboard of nine panes, keyboard-first, *navigate then act*.

- **Navigate** — `e r t / d f g / c v b` (a square on the keyboard) jump to a pane; `j`/`k` (or `↑`/`↓`) move the cursor; `l`/`h` drill in/out; `Tab`/`Shift+Tab` cycle panes.
- **Act** — `Enter` opens the pane's contextual action menu.
- **Command** — `:` opens a vim-style command line (`:travel`, `:goto`, `:filter`, `:theme`, `:refresh`…); `Tab` completes the verb.
- **Zoom** — `z` blows the active pane up to full screen.
- **Adapts** — the grid shrinks to 2×2 or a single pane on smaller terminals, following the active pane; a mini-map shows where you are.
- **`F1`** toggle hints · **`F2`** cycle color mode · **`F5`** refresh · **`?`** help · **`q`** quit.

Startup plays a GUPPI self-check that assembles the cockpit centre-out; any key continues.

## Features

- **Probe** — status, fuel, integrity, movement ETA and speed gauges
- **Scanner / Sector** — scan neighbors, a direction (distance 2), or arbitrary coordinates; filterable history; the Sector pane shows the current sector's objects with resources, planet class and habitability
- **Map** — isometric sector map (`z`); travel to a scanned/visited sector or a waypoint, with fuel/ETA preview
- **Mannies** — per-manny status and live task progress; repair, mine, craft, salvage, recall, rename, drop cargo
- **Inventory** — cargo stocks and onboard items; jettison, move stock, atomic-printer craft
- **Crafting** — recipes show what you can build right now (ingredients owned / required); mining shows the target's reserves
- **Comms** — inter-probe messaging (inbox / sent / compose), alerts and damage warnings
- **Missions & Storage** — track directives; browse storage containers with capacity and routing rules
- **Colour modes** — mono-green, mono-amber, phosphor-semantic, or a 16-colour fallback (`F2`)

## Live updates

Time-derived values (progress bars, percentages, ETAs, the clock) tick every second, so a manny's mining % or a movement's countdown advances on screen without any input. Data is re-fetched when an action needs it, when a movement completes (the timer follows the probe's `arrival_at`), and otherwise at most once a minute — the bottom-right `⟳` shows how long since the last sync.

## Build from source

Requires a stable Rust toolchain (`rustup` recommended).

```bash
git clone https://github.com/MagiCrazy/neumann-cockpit
cd neumann-cockpit
cargo build --release
./target/release/neumann-cockpit
```

## Contributing

Bug reports, ideas, and pull requests are welcome. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the dev setup, the checks CI runs
(`cargo test`, `clippy`, …), and the commit/PR conventions.
