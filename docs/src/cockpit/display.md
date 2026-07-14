# Themes & display

## Colour modes — `F2`

Four phosphor colour modes cycle with `F2` (or `:theme <mode>`, or the `theme` config key):

| Mode | Look |
|------|------|
| `mono-green` | classic green phosphor (default) |
| `mono-amber` | amber phosphor |
| `phosphor-semantic` | green base with green/yellow/red status |
| `modern-16` | named ANSI colours, for terminals without truecolor |

Gauges read green above 50 %, yellow 25–50 %, red below 25 %.

## Hints line — `F1`

A second status line lists the keys valid for the active pane. `F1` toggles it; the `hints` config key sets the default.

## Live updates

Time-derived values — progress bars, mining %, movement countdowns, the clock — tick **every second** without any input. Data is re-fetched when an action needs it, when a movement completes (the timer follows the probe's `arrival_at`), and otherwise at most once a minute. The bottom-right `⟳` shows how long since the last sync.

## Boot sequence

On startup the probe core boots first (the centre-pane preflight), then the eight surrounding subsystems come online centre-out, each typing a themed self-check (SUDDAR array, SCUT link, autofactory, manny bay…). Any key skips straight into the live cockpit.
