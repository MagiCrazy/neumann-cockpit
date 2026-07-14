# Installation

## Prebuilt binaries (recommended)

Binaries for **Linux, macOS, and Windows** are published on the [releases page](https://github.com/MagiCrazy/neumann-cockpit/releases/latest). Every archive ships a matching `.sha256` — download, verify, then extract:

```bash
# Linux x86_64 example
base=https://github.com/MagiCrazy/neumann-cockpit/releases/latest/download
curl -sLO "$base/neumann-cockpit-linux-x86_64.tar.gz"
curl -sLO "$base/neumann-cockpit-linux-x86_64.tar.gz.sha256"
sha256sum -c neumann-cockpit-linux-x86_64.tar.gz.sha256
tar xzf neumann-cockpit-linux-x86_64.tar.gz
./neumann-cockpit
```

On **Windows**, a double-clicked binary works: the boot screen comes up first and every startup failure (including the missing-key case) is handled in the TUI rather than flashing a console and vanishing.

## Build from source

Requires a stable Rust toolchain (`rustup` recommended).

```bash
git clone https://github.com/MagiCrazy/neumann-cockpit
cd neumann-cockpit
cargo build --release
./target/release/neumann-cockpit
```
