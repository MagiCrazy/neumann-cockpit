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

The status bar shows the current mode, a breadcrumb, transient errors/toasts, and right-aligned meta (loading, SCUT relay, unread alerts, API version, clock). The second **hints line** (toggle `F1`) lists the keys valid for the active pane.
