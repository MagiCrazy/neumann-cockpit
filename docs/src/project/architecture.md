# Architecture & contributing

This guide is for **pilots**. If you want to hack on the cockpit itself:

- **Architecture** — the codebase is documented in depth in [`CLAUDE.md`](https://github.com/MagiCrazy/neumann-cockpit/blob/main/CLAUDE.md): the boot preflight, the `tokio::select!` event loop, the `AppState` domain split, the API layer, the theme/UI layout, and every implemented endpoint. It is the single source of truth for how the code is put together — this book intentionally does not duplicate it.
- **Contributing** — [`CONTRIBUTING.md`](https://github.com/MagiCrazy/neumann-cockpit/blob/main/CONTRIBUTING.md) covers the dev setup, the checks CI runs (`cargo test`, `clippy`, `rustfmt`), and the commit/PR conventions.
- **Changelog** — [`CHANGELOG.md`](https://github.com/MagiCrazy/neumann-cockpit/blob/main/CHANGELOG.md) is generated from Conventional Commits.

Bug reports, ideas, and pull requests are welcome on [GitHub](https://github.com/MagiCrazy/neumann-cockpit).
