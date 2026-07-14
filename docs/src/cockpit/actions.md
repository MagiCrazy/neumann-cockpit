# Actions & command mode

Two ways to act: the **contextual menu** (`Enter`) and the **command line** (`:`).

## Contextual menu — `Enter`

`Enter` builds a menu from the active pane **and** the current selection. Unavailable items are shown greyed with the reason. Picking an item launches the matching wizard — a short guided flow (pick a target, confirm, fire). Panes with rich flows (Missions, Comms, Storage, Sector objects) open their dedicated overlay instead of the popup.

In a menu: `1`–`9` fire the nth item, `j`/`k` move, `Enter` fires the selection, `Esc` closes.

## Command mode — `:`

Press `:` for a vim-style command line. `Tab` completes and cycles the verb and its enumerable arguments; `↑`/`↓` browse history; ghost-text shows the argument usage.

| Command | Argument | Does |
|---------|----------|------|
| `focus` | `<pane>` | zoom that pane |
| `travel` | `<x y z \| +dx dy dz>` | travel to absolute or relative coordinates |
| `goto` | `<x y z>` | centre the map |
| `filter` | `<all\|objects\|minable\|danger>` | filter the scan history |
| `craft` | `[recipe]` | open the production console, or enqueue a recipe |
| `queue` | | open the production console |
| `script` | | open the [action-scripting](../automation/scripting.md) console |
| `mine` | `[res] [amt] [by/at/to …]` | open the mine wizard, or fire a mine directly |
| `probe` | `<id\|name>` | pilot a fleet probe |
| `theme` | `<mode>` | set the colour mode |
| `refresh` · `zoom` · `help` · `q` | | refresh · toggle zoom · help · quit |

`travel` accepts a leading `+` for relative moves (e.g. `travel +2 0 0`). `mine` is **hybrid**: bare `:mine` opens the wizard, a full line fires directly — resolving the builder manny and asteroid from context, overridable with `by`/`at`, destination defaulting to the probe (`to <container>` redirects).
