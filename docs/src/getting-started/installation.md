# Installation

## Prebuilt binaries (recommended)

Binaries for **Linux, macOS, and Windows** are published on the [releases page](https://github.com/MagiCrazy/neumann-cockpit/releases/latest). Every archive ships a matching `.sha256` — download, verify, then extract:

```bash
# Linux x86_64 example. Archive names carry the version, so resolve the
# latest tag first — the redirect on /releases/latest names it.
repo=https://github.com/MagiCrazy/neumann-cockpit
tag=$(curl -sI "$repo/releases/latest" | sed -n 's#.*/tag/##p' | tr -d '\r')
version=${tag#neumann-cockpit-}
archive="neumann-cockpit-${version}-linux-x86_64.tar.gz"

curl -sLO "$repo/releases/download/$tag/$archive"
curl -sLO "$repo/releases/download/$tag/$archive.sha256"
sha256sum -c "$archive.sha256"
tar xzf "$archive"
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
