# Production queue

The **production console** is one unified crafting catalog spanning both fabricators — the atomic printer and manny crafting. Open it with `:craft`, `:queue`, or **Fabricate** from the Mannies/Inventory pane menu.

It shows three panels: the **recipe catalog**, the **selected-recipe detail** (ingredients owned vs required), and the live **production queue**.

## Queueing

- On the catalog, `+`/`-` (or `h`/`l`) set a **quantity**, and `Enter` adds the recipe ×qty to the queue. Identical trailing steps coalesce into one `×N`.
- A manny recipe resolves its builder like an immediate craft: pre-chosen from the Mannies pane, the sole idle onboard manny, or a builder picker.
- `Tab` focuses the queue panel to manage it: `+`/`-` a step's repeat, `x` remove, `c` clear.

## How it runs

The queue **auto-runs** — it drains as steps complete, no explicit "go". It is organised into **lanes**: one per builder manny plus one for the atomic printer. A lane runs one craft at a time, but lanes run in **parallel**, so an idle manny starts its next craft while another is still working.

It **pauses** with `p` (and automatically on a craft failure — inspect, fix, resume), is capped at 32 steps, and is **session-only** (not persisted). A status-bar chip `⛭ done/total` shows progress with the console closed.

> For a sequence of *different* action types (not just crafts) that must run in order — or to fabricate **unattended** from a file — use [action scripting](scripting.md) instead: its `craft` verb runs recipes one at a time (parts first, then assembly). The queue is the interactive, parallel-lane tool; a script is the sequential, headless-capable one.
