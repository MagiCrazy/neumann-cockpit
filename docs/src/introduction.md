# neumann-cockpit

A terminal UI **cockpit** for the [Von Neumann Game](https://github.com/gnieark/Von-Neumann-Game) — pilot your probe and its mannies from the command line. The official game instance runs at **[neumann-probe.net](https://neumann-probe.net)** (the default endpoint).

It is a single **phosphor cockpit**: a 3×3 tiling dashboard of nine panes, keyboard-first, built around one idea — *navigate then act*. Jump to a pane, move the cursor, then press `Enter` for its contextual actions or `:` for a command line.

```text
┌═ SCANNER ═════┐┌═ MAP ═════════┐┌═ COMMS ═══════┐
│ scan history  ││ sector coords ││ alerts / msgs │
└═══════════════┘└═══════════════┘└═══════════════┘
┌═ SECTOR ══════┐┌═ PROBE ═══════┐┌═ MISSIONS ════┐
│ objects here  ││ status · fuel ││ active list   │
└═══════════════┘└═══════════════┘└═══════════════┘
┌═ INVENTORY ═══┐┌═ STORAGE ═════┐┌═ MANNIES ═════┐
│ cargo · stocks││ containers    ││ ● manny list  │
└═══════════════┘└═══════════════┘└═══════════════┘
```

## What this guide covers

- **[Getting started](getting-started/index.md)** — install, configure, and the first-run onboarding.
- **[The cockpit](cockpit/index.md)** — the grid, the nine panes, contextual actions, command mode, and display options.
- **[Automation](automation/index.md)** — the production queue, vim-style action scripting, and the headless script runner.
- **[Reference](reference/index.md)** — the keybinding cheat sheet and every config option.

Developers: the codebase architecture lives in [`CLAUDE.md`](https://github.com/MagiCrazy/neumann-cockpit/blob/main/CLAUDE.md); see **[Architecture & contributing](project/architecture.md)**.
