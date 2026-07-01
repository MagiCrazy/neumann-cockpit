# Contributing to neumann-cockpit

Thanks for your interest! This is a hobby project — a terminal UI cockpit for
[Von Neumann Game](https://github.com/gnieark/Von-Neumann-Game). Bug reports,
ideas, and pull requests are all welcome.

By contributing, you agree that your contributions are licensed under the
project's [GPL-3.0](LICENSE) license.

## Ways to contribute

- **Report a bug** — open an issue with steps to reproduce, what you expected,
  and what happened. A screenshot of the TUI helps a lot.
- **Suggest a feature** — open an issue describing the idea and the gameplay it
  supports. For anything non-trivial, it's worth agreeing on the approach in the
  issue before writing code.
- **Send a pull request** — see below.

## Development setup

You need a stable Rust toolchain (via [rustup](https://rustup.rs/)).

```bash
git clone https://github.com/MagiCrazy/neumann-cockpit
cd neumann-cockpit

# Configure: copy the example and fill in your API key
mkdir -p ~/.config/neumann-cockpit
cp config.example.toml ~/.config/neumann-cockpit/config.toml
# then edit ~/.config/neumann-cockpit/config.toml (base_url + api_key)

cargo run        # launch the TUI
```

An API key is generated once from the web UI on
[neumann-probe.net](https://neumann-probe.net) (Settings → API key).

## Before you open a PR

CI runs these four checks — please run them locally first:

```bash
cargo check
cargo clippy -- -D warnings   # warnings are treated as errors
cargo test
cargo build --release
```

- **Add tests** for the behavior you change. Pure logic lives in unit tests
  (`src/app/tests.rs`, `src/input/…`); API types are covered by serde fixtures in
  `tests/serde_types.rs`. When you add a new deserialized type, add a serde test
  with a payload copied from the OpenAPI spec. Network I/O and TUI rendering are
  not covered (they need a live terminal and endpoint). Optional helpers:
  `cargo nextest run` for richer output, `cargo tarpaulin --out Html` for a
  coverage report.
- **Match the surrounding style** — the code favors small, per-domain modules and
  one input-state enum per interactive action. `CLAUDE.md` documents the
  architecture (event loop, `AppState`, API layer, UI panels/overlays) and is the
  best map of the codebase.
- **API changes** are typed against the vendored OpenAPI spec in `api-specs/`
  (e.g. `api-specs/v63.yaml`). Read the spec before adding or changing types.

## Commit messages

This repo uses [Conventional Commits](https://www.conventionalcommits.org/) —
[release-please](https://github.com/googleapis/release-please) reads them to
compute versions and generate the changelog, so the prefix matters:

| Prefix | Effect |
|---|---|
| `feat:` | new feature → **minor** bump, shown under *Features* |
| `fix:` | bug fix → **patch** bump, shown under *Bug Fixes* |
| `perf:` / `revert:` / `docs:` / `deps:` | shown in the changelog |
| `chore:` / `ci:` / `refactor:` / `style:` / `test:` / `build:` | hidden from the changelog |

Use a scope when it helps, e.g. `feat(scut): …`, `fix(input): …`. A breaking
change needs a `!` (`feat!:`) or a `BREAKING CHANGE:` footer.

Do **not** bump the version or edit the changelog by hand — release-please owns
both through its release PR.

## Pull request workflow

1. Branch off `main` (`git switch -c feat/my-thing`).
2. Make your change, with tests, and keep the four checks above green.
3. Open a PR against `main`. Never push directly to `main`.
4. Keep PRs focused — one logical concern per PR is easier to review and to
   release.

Once merged, release-please rolls your commit into the pending release PR; a
maintainer cuts the release from there.

Happy hacking! 🛰️
