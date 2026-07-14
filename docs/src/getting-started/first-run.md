# First run

Launch it:

```bash
neumann-cockpit
```

Startup runs a **preflight** inside the centre (Probe) pane before the live cockpit appears. It performs three checks:

1. **CONFIG** — if the config is missing or has no real key, an onboarding prompt appears right in the Probe pane: it tells you where to get a key, lets you paste it in, and writes `~/.config/neumann-cockpit/config.toml` for you. `Esc` / `Ctrl-C` quits cleanly.
2. **ARCHIVE** — opens the local SQLite database and loads your scan history.
3. **REMOTE LINK** — contacts the game API. On a bad key or an outage you get in-pane actions:
   - `[R]` retry the link
   - `[K]` re-enter the API key (re-runs onboarding)
   - `[Enter]` continue **offline** (degraded mode — an error toast; `F5` retries later)

Once the link is up, a short self-check animation assembles the cockpit centre-out. **Any key** drops you into the live cockpit.

From here, head to **[The cockpit](../cockpit/index.md)**.
