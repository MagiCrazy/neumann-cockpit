# The panes

Each pane shows a slice of state and offers actions via `Enter`. Move the cursor with `j`/`k`, drill with `l`/`h`, zoom with `z`.

## Scanner (`e`)
Scan history: symbol, coordinates, and distance for every sector you've seen, filterable. `Enter` offers **Travel here** to the selected observation. Scan neighbours, a direction (distance 2), or arbitrary coordinates.

## Map (`r`)
Compact sector summary. `z` opens the full **isometric map** (pan with `h j k l`, `g` travels to the centred sector, `c` re-centres on coordinates). `Enter` also offers **Travel to coordinates…**, **Jump to visited sector…**, and **Waypoints…** — each with a fuel/ETA preview.

## Comms (`t`)
A category root: **Messages**, **Alerts**, **Warnings**, each with an unread count and a preview. Drill in (`l`) to open the inbox/sent messaging overlay (read full-screen, `c` composes to a probe or planet), or an in-pane list where `Enter` acknowledges an alert/warning.

## Sector (`d`)
The objects in your current sector — asteroids (with reserves), detached containers, dormant constructs, planets (class + habitability). Drill into an object and `Enter` for its actions: **mine, inspect, salvage, recover, deploy waypoint**, or **turn on relay** for an inactive SCUT relay.

## Probe (`f`, centre)
Status, fuel, hull integrity, movement ETA and speed gauges. Its top-right corner pins the active probe's **sigil** (a deterministic identicon) so drones are told apart at a glance. `Enter`: switch probe (multi-probe fleets), set default, rename, inspect the SCUT network, install **probe improvements**, and — if the probe is dead or trapped — reassign its mind snapshot. Zoom (`z`) unfolds the full fleet roster.

## Missions (`g`)
Two categories: **Missions** (active directives with steps/status; `Enter` → abandon) and **Ship's log** (the captain's-log — newest-first narrated entries of your actions and server events).

## Inventory (`c`)
Cargo stocks and onboard items. `Enter`: **Fabricate**, **Move stock**, **Jettison**, and **Deploy waypoint…** (when a waypoint bookmark is held).

## Storage (`v`)
Storage containers with capacity bars. `Enter`: view contents, rename, and edit routing rules (none → priority → exclusion → strict).

## Mannies (`b`)
The manny roster with live task progress. `Enter` opens the richest menu: fabricate, mine, repair, salvage, inspect, recover/detach a container, refuel, **transfer deuterium** to another probe, drop cargo, **assemble a probe**, recall/abandon, rename. A mining manny that turns up a hidden container flags it in its detail.
