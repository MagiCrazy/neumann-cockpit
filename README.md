# neumann-cockpit

[![CI](https://github.com/MagiCrazy/neumann-cockpit/actions/workflows/ci.yml/badge.svg)](https://github.com/MagiCrazy/neumann-cockpit/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/MagiCrazy/neumann-cockpit)](https://github.com/MagiCrazy/neumann-cockpit/releases/latest)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)

A terminal UI cockpit for [Von Neumann Game](https://github.com/gnieark/Von-Neumann-Game) — control your probe and its mannies from the command line.

The official game instance runs at **[https://neumann-probe.net](https://neumann-probe.net)** (default endpoint).

## Install

**Prebuilt binaries** for Linux, macOS, and Windows are available on the [releases page](https://github.com/MagiCrazy/neumann-cockpit/releases/latest).

```bash
# Linux x86_64 example
curl -sL https://github.com/MagiCrazy/neumann-cockpit/releases/latest/download/neumann-cockpit-linux-x86_64.tar.gz | tar xz
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

- **Navigate** — `e r t / d f g / c v b` (a square on the keyboard) jump to a pane; `j`/`k` (or `↑`/`↓`) move the cursor; `l`/`h` drill in/out.
- **Act** — `Enter` opens the pane's contextual action menu.
- **Zoom** — `z` blows the active pane up to full screen.
- **Adapts** — the grid shrinks to 2×2 or a single pane on smaller terminals, following the active pane; a mini-map shows where you are.
- **`F1`** toggle hints · **`F2`** cycle color mode · **`F5`** refresh · **`?`** help · **`q`** quit.

Startup plays a GUPPI self-check that assembles the cockpit centre-out; any key continues.

## Features

- **Probe** — status, fuel, integrity, movement ETA and speed gauges
- **Inventory** — cargo stocks, onboard items; jettison resources or eject mannies; deploy waypoint bookmarks
- **Mannies** — per-manny status and progress; repair, mine, craft, salvage, recall, rename
- **Scanner / Sector** — scan the current sector or arbitrary coordinates, neighbor sweep, deep scan; browsable history
- **Comms** — inter-probe messaging (inbox / sent / compose), alerts and damage warnings
- **Missions & Storage** — track directives; browse storage containers with capacity and routing rules
- **Colour modes** — mono-green, mono-amber, phosphor-semantic, or a 16-colour fallback

## Auto-refresh

The UI refreshes automatically when a movement completes — the timer is set to the probe's `arrival_at` timestamp. When idle the next deadline is 24 h away; no background polling.

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
