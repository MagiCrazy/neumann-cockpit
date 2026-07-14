# Configuration reference

## File location

`~/.config/neumann-cockpit/config.toml` (XDG config directory).

## Keys

```toml
base_url = "https://neumann-probe.net"
api_key  = "vng_your_api_key_here"
theme    = "mono-green"
hints    = true
```

| Key        | Type   | Default                       | Meaning |
|------------|--------|-------------------------------|---------|
| `base_url` | string | `https://neumann-probe.net`   | Game API endpoint. |
| `api_key`  | string | —                             | Your `vng_…` API key (generated once in the web UI). |
| `theme`    | string | `mono-green`                  | Colour mode: `mono-green`, `mono-amber`, `phosphor-semantic`, `modern-16`. `F2` cycles at runtime. |
| `hints`    | bool   | `true`                        | Show the contextual hints line. `F1` toggles at runtime. |

Unknown keys are ignored, so legacy config files keep loading.

## State (not config)

Scan history and the **ship's log** are persisted in a local SQLite database (`cockpit.db`) under your XDG **state** directory. It is created and managed automatically — you never edit it. A legacy `scan_history.json`, if present, is migrated into the database once and then removed.

## Command-line

| Flag | Effect |
|------|--------|
| _(none)_ | launch the interactive cockpit |
| `--script <file>`, `-s <file>` | [headless runner](../automation/headless.md): play a script, no TUI |
