# The cockpit

The cockpit is a **3×3 grid of nine panes**, keyboard-first, built on one model: **navigate then act**.

- **[The grid & navigation](grid.md)** — the pane keys, cursor movement, drilling, zoom, and how the layout adapts to your terminal size.
- **[The panes](panes.md)** — what each of the nine panes shows and does.
- **[Actions & command mode](actions.md)** — the `Enter` contextual menu and the `:` command line.
- **[Themes & display](display.md)** — colour modes, the hints line, live updates, and the boot sequence.

At a glance:

```text
 NAV  COCKPIT › MANNIES        ⟳ · ≣ SCUT · ! 2 · API vN · 14:09
 ↑↓ move · hl drill · z zoom · Enter act · ertdfgcvb pane · F1 hints
```

The status bar shows the current mode, a breadcrumb, transient errors/toasts, and right-aligned meta (loading, active probe, SCUT relay, idle mannies, unread alerts, API version, clock). The second **hints line** (toggle `F1`) lists the keys valid for the active pane.

A few meta chips only appear when they have something to say:

| Chip | Meaning |
|------|---------|
| `⛭ 3/8` | production queue progress, with the console closed |
| `≡ 2/5` | action-script progress (`≡‖` when paused) |
| `⚙ 4 idle` | idle mannies waiting for orders (`i` cycles to the next) |
| `! 3` | unread alerts and messages, counted across your whole mailbox |
| `⏳ rate limit 47s` | the server's request quota is spent — see below |
| `⚠ save failing` | the local database is refusing writes, so history is no longer saved |

## Rate limiting

The game server meters requests **per API key** over a sliding window. Go over it and it answers `429` with a retry delay; the cockpit then shows the `⏳ rate limit Ns` chip and **holds its automatic refreshes** until the window reopens, counting down each second. Nothing is lost — the next refresh picks everything up. It resolves on its own, and normal play does not come close to the ceiling.
