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
```

**3. Run**

```bash
neumann-cockpit
```

## Features

- **Probe** — status, fuel, integrity, movement ETA and speed gauges
- **Inventory** — cargo stocks, onboard items; jettison resources or eject mannies; deploy waypoint bookmarks
- **Mannies** — per-manny status and progress; repair, mine, craft, salvage, recall, rename
- **Scanner** — scan current sector or arbitrary coordinates, neighbor sweep, deep scan; browsable history
- **Sector map** — isometric overview of scanned sectors, pan and travel from the map
- **Auto-travel** — fuel and ETA preview before committing a jump

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
