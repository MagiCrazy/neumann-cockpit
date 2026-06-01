# neumann-cockpit

Terminal UI for monitoring the Neumann probe and its mannies in real time.

## Prerequisites

- Rust toolchain (`rustup` recommended — stable is sufficient)
- An API key generated from the Neumann web UI (shown only at creation time)

## Setup

```bash
mkdir -p ~/.config/neumann-cockpit
cp config.example.toml ~/.config/neumann-cockpit/config.toml
```

Edit `~/.config/neumann-cockpit/config.toml` and fill in your API key:

```toml
base_url = "https://neumann-probe.net"
api_key  = "vng_your_api_key_here"
```

## Run

```bash
cargo run           # development (debug build)
cargo run --release # optimised build
```

Or build once and run the binary directly:

```bash
cargo build --release
./target/release/neumann-cockpit
```

## Keybindings

| Key | Action          |
|-----|-----------------|
| `r` | Manual refresh  |
| `q` | Quit            |

## Auto-refresh

The UI refreshes automatically when a movement completes: the timer is set to the probe's `arrival_at` timestamp. When no movement is in progress the next deadline is 24 h away — no background polling.
