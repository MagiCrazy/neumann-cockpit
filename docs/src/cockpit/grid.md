# The grid & navigation

Nine panes are addressed by a **square of keys** on the keyboard — identical on AZERTY and QWERTY:

```text
 e  r  t     Scanner   Map      Comms
 d  f  g     Sector    Probe    Missions
 c  v  b     Inventory Storage  Mannies
```

The centre key `f` is always the **Probe** pane.

## Keys

| Key            | Action                                             |
|----------------|----------------------------------------------------|
| `e r t d f g c v b` | activate a pane                               |
| `j` / `k` (or `↑` / `↓`) | move the cursor in the active pane        |
| `l` / `→`      | drill **in** (e.g. Missions → steps, Comms → a message) |
| `h` / `←`      | drill **out**                                      |
| `Tab` / `Shift+Tab` | cycle panes forward / back                    |
| `z`            | **zoom** the active pane full-screen               |
| `Enter`        | open the pane's contextual action menu             |
| `:`            | command mode                                       |
| `i`            | jump to the next idle Manny (focuses Mannies)      |
| `F1` / `F2`    | toggle hints / cycle colour mode                   |
| `F5`           | refresh · `?` help · `q` quit                      |
| `Esc`          | close menu / leave zoom / drill up                 |

## Zoom

`z` blows the active pane up to full screen — useful for the Map, the ship's log, or a long list. `Esc` (or `z` again) returns to the grid.

## Responsive layout

The grid fits as many whole panes as your terminal allows: **3×3** on a large terminal, **2×2** on a half-screen, a single row on a short wide split, or one pane when tiny. It slides the visible window to keep the active pane on screen, and a **position mini-map** (the nine keys in three groups) appears in the status bar whenever the grid is reduced, so you always know where you are.
