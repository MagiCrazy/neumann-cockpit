# Automation

Three ways to let the cockpit work for you, from least to most hands-off:

- **[Production queue](queue.md)** — queue up crafts; the executor runs them in parallel across your builders and the atomic printer, auto-draining as each finishes.
- **[Action scripting](scripting.md)** — compose a sequence of heterogeneous actions ("detach a container → travel → mine → recover it") that runs strictly one step at a time, with fan-out/join for parallel mining.
- **[Headless runner](headless.md)** — play a script from a file with no TUI, streaming the ship's-log to stdout, for cron jobs and unattended runs.

All three share one sequencing engine, so they behave consistently: a step fires, completion is polled (a manny going idle again, a movement arriving), and progress shows in the status bar even with the console closed.
