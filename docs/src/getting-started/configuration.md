# Configuration

The cockpit reads `~/.config/neumann-cockpit/config.toml` at startup.

## Get an API key

Create an account on [neumann-probe.net](https://neumann-probe.net), open **Settings**, and generate an API key. It is shown **only once** — copy it right away.

## The config file

You rarely need to write this by hand (see [First run](first-run.md)), but if you prefer:

```bash
mkdir -p ~/.config/neumann-cockpit
cp config.example.toml ~/.config/neumann-cockpit/config.toml
```

```toml
base_url = "https://neumann-probe.net"
api_key  = "vng_your_api_key_here"
# theme = "mono-green"   # mono-green | mono-amber | phosphor-semantic | modern-16
# hints = true           # show the contextual hints line
```

| Key        | Required | Meaning                                                        |
|------------|----------|----------------------------------------------------------------|
| `base_url` | yes      | Game API endpoint (defaults to `https://neumann-probe.net`).   |
| `api_key`  | yes      | Your `vng_…` API key.                                          |
| `theme`    | no       | Colour mode; `F2` cycles it at runtime.                        |
| `hints`    | no       | Show the contextual hints line (`F1` toggles). Defaults `true`.|

Unknown keys are ignored, so older config files keep loading. See the full [configuration reference](../reference/configuration.md).

## Where state lives

Scan history and the ship's log are kept in a local SQLite database under your XDG **state** directory — separate from the config. You never edit it by hand.
