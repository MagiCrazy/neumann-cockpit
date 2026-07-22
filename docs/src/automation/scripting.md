# Action scripting

Compose a **sequence** of heterogeneous pilot actions — "detach a container → travel → mine → bring it back" — that runs **strictly one step at a time**: a step fires only once the previous one has completed.

Open the console with `:script`.

## The editor (vim-style)

The console is a modal editor over the current script:

- **Insert mode** — type a command line; `Enter` validates and appends it (a syntax error shows inline), `Esc` switches to Normal mode.
- **Normal mode** — `j`/`k` move the selection, `x` removes a step, `c` clears the script, `i` re-enters Insert, `R` **runs** the script, `p` pauses/resumes, `Esc` closes.

Unlike the auto-running production queue, a script is **composed, then explicitly run** with `R`. A status-bar chip `≡ done/total` shows progress with the console closed.

## Command lines

Each line is one action. The MVP verbs:

| Verb | Grammar |
|------|---------|
| `travel` | `<x y z \| +dx dy dz>` |
| `mine` | `[res[,res]] [amount] [by <manny\|all\|A,B>] [at <asteroid>] [to <container>]` |
| `repair` | `[percent] [by <manny>]` |
| `salvage` | `[by <manny>] [at <wreck>]` |
| `detach` | `<container> [by <manny>] [mode <drifting\|hidden_on_asteroid>] [at <asteroid>]` |
| `recover` | `[by <manny>] [at <container>]` |
| `craft` | `<recipe[,recipe…]> [by <manny\|all\|A,B>]` |

Targets are matched by **exact id** first, then a **case-insensitive substring** of a name (manny, asteroid, container). Where a target is unambiguous — the sole idle manny, the only asteroid in the sector — you can omit it.

**Quote a name with spaces or a keyword.** Wrap it in double quotes: `at "Metal C #1"`. A quoted span is one atomic token, so a name may even contain a keyword — `to "Ready to Go"` keeps the inner `to` as part of the name. Without quotes, a bare `to`/`by`/`at`/`mode` inside a name is read as a delimiter and corrupts it.

> Nested asteroids (bodies of a solar system) are often **unnamed**, so a name match can't single one out. Zoom the **Sector** pane (`z`): each body shows its `id`. Copy it into `at <id>` to target that exact rock — e.g. `mine metals 0.20 by all at c4d44bd03a7ccc94b37b`.

## Two ideas that make it work

### Late binding

A line is checked for **syntax** when you add it, but its **targets are resolved against live state only when the step fires** — not when you type it. This is what lets a `mine` follow a `travel` into a not-yet-scanned sector: the asteroid only exists once the probe *arrives*, and the step binds to it then.

### Fork/join

A single step can **fan out** to several builders and acts as a **barrier**. A `mine … by all` (every idle manny) or `by Alpha,Bravo` (named) fires one mine per manny in parallel, and the step is complete only once **every** one of them is idle again. The next step waits for that join.

`craft` fans out the same way, but over a **recipe list**: `craft steel_plate,steel_bar,steel_bar by all` assigns **one recipe per builder** (distinct mannies), fires them together, and joins on all — so the parts are produced in parallel, then the next line assembles them.

This is the canonical resupply run:

```text
detach box by Alpha
mine metals 900 by all to box
recover box by Alpha
```

1. Alpha detaches the container into the sector.
2. **All** idle mannies mine into it, in parallel — the step holds until the last finishes.
3. Only then does the recover run.

## Two worked examples

### Fabricating a complex item

`craft <recipe>` fabricates one recipe. A recipe is matched by name (case-insensitive) or id; a manny recipe binds to a builder (`by <manny>`, else the sole idle onboard manny), an atomic-printer recipe needs no builder. The step waits for that craft to finish before the next line runs — so a **multi-step item is just its parts crafted in order, then the final assembly**:

```text
# an additional_container is built from a steel plate + a steel bar,
# each itself crafted from raw metals — so craft the parts first.
craft steel_plate by Alpha
craft steel_bar by Alpha
craft additional_container by Alpha
```

Each line fires only once Alpha is idle again, so the parts exist in the probe inventory by the time the container is assembled. (You need enough mined `metals` stocked for the intermediate crafts.) There is no auto-expansion of sub-components — you list them, which keeps what runs explicit and predictable.

To go faster, fan the parts out over several mannies, then assemble once they have all finished:

```text
# 3 parts in parallel (one manny each), barrier, then the motor
craft steel_plate,steel_bar,steel_bar by all
craft electric_motor by manny-8
```

> The script sequences **manny** and **atomic-printer** recipes alike. A printer craft has no named builder; the step completes when no manny is left assisting the printer.

### A mining resupply run

The canonical fan-out (detach a container, mine it full with the whole fleet, bring it back) — see [Fork/join](#forkjoin) above:

```text
detach box by Alpha
mine metals 900 by all to box
recover box by Alpha
```

## When a step fails

If a target can't be resolved, or an action errors while in flight, the script **halts**: the step is marked `✗` with the reason and the script pauses. Fix it (edit the line, or wait) and press `R` to resume — failed steps are retried.

Scripts are capped at 32 steps and are **session-only**. To run one unattended, from a file, see the [headless runner](headless.md).

> `by all` grabs *every* idle onboard manny at fire time — including the one that just finished the preceding step. To keep some aside, name them explicitly: `by Bravo,Charlie`.
