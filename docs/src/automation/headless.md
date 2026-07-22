# Headless runner

Play an [action script](scripting.md) from a **file**, with no TUI, streaming the ship's-log to stdout. Good for cron jobs, unattended resupply runs, or scripting the cockpit from a shell.

```bash
neumann-cockpit --script run.ncs
# short form:
neumann-cockpit -s run.ncs
```

A bare launch (`neumann-cockpit`, no `--script`) is the interactive cockpit, unchanged.

## The script file

One command per line — the same grammar as the interactive console. Blank lines and `#` comments are ignored.

A **resupply run** — detach a container, mine it full with every idle manny, bring it back (quote a name with spaces or a keyword, e.g. `"Metal C #1"`):

```text
# resupply run
detach "Metal C #1" by Alpha mode hidden_on_asteroid at Rock
mine metals 900 by all to "Metal C #1"
recover by Alpha at "Metal C #1"
```

A **fabrication run** — build a complex item by crafting its parts first, then assembling (see [`craft`](scripting.md#fabricating-a-complex-item)):

```text
# additional_container = steel_plate + steel_bar (each from metals)
craft steel_plate by Alpha
craft steel_bar by Alpha
craft additional_container by Alpha
```

## Output

Ship's-log lines stream to stdout as the run progresses — a timestamp, the narrated captain's-log summary for each fired step, a `✓`/`✗` per step, and a final status line:

```text
14:09:02  » Detached a storage container, hiding it on an asteroid.
14:09:05    ✓ step 1/3 done
14:12:40  » Dispatched 3 mannies to mine «metals», hauling to a detached container.
14:16:55    ✓ step 2/3 done
...
14:20:11  ✓ script complete — 3/3
```

The same entries are **persisted** to the local ship's-log database, so a headless run also shows up in the next interactive session's ship's log.

## Exit codes

| Code | Meaning |
|------|---------|
| `0`  | the script completed |
| `1`  | the script halted on an error (a step failed to resolve or an action errored), or startup failed (bad config, unreadable file, no link) |

This makes it compose in shell scripts — chain runs with `&&`, or alert on a non-zero exit.

> The runner loads your config non-interactively: there is no key onboarding here. If the config is missing or the API key is invalid, it errors to stderr and exits — launch the interactive cockpit once to set your key first.
