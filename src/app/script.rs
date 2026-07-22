//! Action scripting (#198) — compose a **linear sequence** of heterogeneous
//! pilot actions ("detach a container → travel → mine → recover it") that runs
//! **strictly one step at a time**: step N only fires once step N-1 has
//! completed. This is the second consumer of the sequencing primitives the
//! production queue (`queue.rs`) introduced — it reuses `StepState` and the
//! `pending_fire`/drain firing pattern — but with a different execution model
//! (one sequential lane, vs. the queue's parallel per-builder lanes).
//!
//! **Late binding.** A script is composed as `:`-style command lines and each
//! line is validated *syntactically* when added, but its targets (the builder
//! Manny, the asteroid, the container) are resolved against the **live** state
//! only when the step is about to fire. This is what makes the issue's own
//! example work: the asteroid a `mine` step targets exists only in the sector
//! the probe *arrives* in, after the preceding `travel` — so it cannot be bound
//! at compose time.
//!
//! Completion detection mirrors the queue: a Manny action is done on the
//! builder's busy→idle transition (`can_receive_orders`), a travel on the
//! probe's `movement` clearing (`movement_arrival`). The `observed_busy` guard
//! of `StepState::Running` covers the fire→busy lag the same way.

use super::command::{mine_buckets, mine_resource};
use super::*;

/// Cap on script length, mirroring `QUEUE_MAX` — a runaway script never
/// silently balloons into hundreds of API calls.
pub const SCRIPT_MAX: usize = 32;

/// The scriptable actions (MVP set, #198): the issue's example plus the other
/// long-running actions where sequencing is meaningful. Extensible verb by verb.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScriptVerb {
    Travel,
    Mine,
    Repair,
    Salvage,
    Detach,
    Recover,
    Craft,
}

impl ScriptVerb {
    pub fn parse(tok: &str) -> Option<Self> {
        Some(match tok.to_lowercase().as_str() {
            "travel" | "go" | "goto" => Self::Travel,
            "mine" => Self::Mine,
            "repair" => Self::Repair,
            "salvage" => Self::Salvage,
            "detach" => Self::Detach,
            "recover" => Self::Recover,
            "craft" | "fabricate" => Self::Craft,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Travel => "travel",
            Self::Mine => "mine",
            Self::Repair => "repair",
            Self::Salvage => "salvage",
            Self::Detach => "detach",
            Self::Recover => "recover",
            Self::Craft => "craft",
        }
    }

    /// One-line argument grammar, shown in the editor's help footer.
    pub fn usage(self) -> &'static str {
        match self {
            Self::Travel => "<x y z | +dx dy dz>",
            Self::Mine => "[res[,res]] [amount] [by <manny|all|A,B>] [at <asteroid>] [to <container>]",
            Self::Repair => "[percent] [by <manny>]",
            Self::Salvage => "[by <manny>] [at <wreck>]",
            Self::Detach => "<container> [by <manny>] [mode <drifting|hidden_on_asteroid>] [at <asteroid>]",
            Self::Recover => "[by <manny>] [at <container>]",
            Self::Craft => "<recipe> [by <manny>]",
        }
    }
}

/// A script line validated *syntactically* — verb + raw argument tokens. Target
/// resolution is deferred to `resolve` at fire time (late binding).
#[derive(Clone, Debug, PartialEq)]
pub struct ScriptCommand {
    pub verb: ScriptVerb,
    pub args: Vec<String>,
}

/// A fully-resolved action, ready to fire, produced by `resolve` against live
/// state. Each variant carries what its `fetch_*` spawner needs plus (for the
/// Manny actions) the builder id the executor polls for busy→idle completion.
#[derive(Clone, Debug, PartialEq)]
pub enum ScriptAction {
    Travel {
        x: i32,
        y: i32,
        z: i32,
    },
    Mine {
        manny_id: String,
        object_id: String,
        resources: Vec<String>,
        amount: f64,
        container_id: Option<String>,
    },
    Repair {
        manny_id: String,
        integrity_percent: f64,
    },
    Salvage {
        manny_id: String,
        object_id: String,
    },
    Detach {
        manny_id: String,
        container_id: String,
        mode: String,
        object_id: Option<String>,
    },
    Recover {
        manny_id: String,
        object_id: String,
    },
    /// Fabricate a recipe (#258). A Manny recipe carries its builder id (busy→idle
    /// completion); an atomic-printer recipe has no builder (`None`) and completes
    /// when no Manny is assisting the printer.
    Craft {
        fabricator: Fabricator,
        manny_id: Option<String>,
        recipe_id: String,
    },
}

impl ScriptAction {
    /// The builder Manny whose busy→idle transition marks this action complete,
    /// or `None` for travel (completion is the probe's movement clearing).
    fn manny_id(&self) -> Option<&str> {
        match self {
            Self::Travel { .. } => None,
            Self::Mine { manny_id, .. }
            | Self::Repair { manny_id, .. }
            | Self::Salvage { manny_id, .. }
            | Self::Detach { manny_id, .. }
            | Self::Recover { manny_id, .. } => Some(manny_id),
            // A Manny craft carries its builder; a printer craft has none (its
            // completion is polled separately, see `action_in_progress`).
            Self::Craft { manny_id, .. } => manny_id.as_deref(),
        }
    }
}

/// One step of a script: the raw line (for display), its parsed command, its
/// run state, and the **group** of actions it resolved to when it fired. Most
/// steps resolve to a single action; a fan-out `mine ... by all|A,B` resolves to
/// one action per builder, all fired together, and the step acts as a **join**
/// (barrier) — done only once every action in the group has completed.
#[derive(Clone)]
pub struct ScriptStep {
    pub raw: String,
    pub cmd: ScriptCommand,
    pub state: StepState,
    pub resolved: Vec<ScriptAction>,
}

impl ScriptStep {
    pub fn is_running(&self) -> bool {
        matches!(self.state, StepState::Running { .. })
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self.state, StepState::Done | StepState::Failed(_))
    }
}

/// Split an argument slice into the positional head and the `by` / `at` / `mode`
/// keyword buckets (values run to the next keyword, so names may contain
/// spaces) — the shared grammar of the Manny-targeted scripted actions.
fn split_kw<'a>(args: &[&'a str]) -> (Vec<&'a str>, Vec<&'a str>, Vec<&'a str>, Vec<&'a str>) {
    let (mut positional, mut by, mut at, mut mode) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut bucket = 0u8; // 0 positional · 1 by · 2 at · 3 mode
    for &tok in args {
        match tok {
            "by" => bucket = 1,
            "at" => bucket = 2,
            "mode" => bucket = 3,
            _ => match bucket {
                1 => by.push(tok),
                2 => at.push(tok),
                3 => mode.push(tok),
                _ => positional.push(tok),
            },
        }
    }
    (positional, by, at, mode)
}

/// Pick a single target from a candidate list: the sole one when `query` is
/// empty, else an **exact id** match (case-insensitive) — for targeting unnamed
/// objects by the id shown in the zoomed Sector view — falling back to a
/// case-insensitive **name substring**. `noun` shapes the error.
fn pick_one(cands: Vec<(String, String)>, query: &str, noun: &str) -> Result<(String, String), String> {
    if query.is_empty() {
        match cands.len() {
            0 => Err(format!("no {noun} in current sector")),
            1 => Ok(cands.into_iter().next().unwrap()),
            _ => Err(format!("multiple {noun}s — name one (or use its id)")),
        }
    } else {
        let q = query.to_lowercase();
        cands
            .iter()
            .find(|(id, _)| id.eq_ignore_ascii_case(&q))
            .or_else(|| cands.iter().find(|(_, n)| n.to_lowercase().contains(&q)))
            .cloned()
            .ok_or_else(|| format!("no {noun} matching \"{query}\""))
    }
}

/// A captain's-log entry for a fired step, narrated from its resolved action
/// group (a fan-out mine reports the builder count). Persisted to the ship's log
/// and printed by the headless runner, so both read the same. Entity names the
/// executor doesn't carry are left generic; the raw line keeps the specifics.
fn script_log_event(group: &[ScriptAction], probe_id: Option<u64>) -> LogEvent {
    use super::kind;
    let (k, summary): (&str, String) = match group.first() {
        Some(ScriptAction::Travel { x, y, z }) => {
            (kind::TRAVEL, format!("Laid in a course for sector «({x}, {y}, {z})»."))
        }
        Some(ScriptAction::Mine {
            resources,
            container_id,
            ..
        }) => {
            let res = resources.join(", ");
            let dest = if container_id.is_some() {
                "a detached container"
            } else {
                "the probe"
            };
            let who = if group.len() > 1 {
                format!("{} mannies", group.len())
            } else {
                "a manny".into()
            };
            (
                kind::MINE,
                format!("Dispatched {who} to mine «{res}», hauling to {dest}."),
            )
        }
        Some(ScriptAction::Repair { integrity_percent, .. }) => (
            kind::REPAIR,
            format!("Ordered hull repairs to «{integrity_percent:.0}%»."),
        ),
        Some(ScriptAction::Salvage { .. }) => (kind::SALVAGE, "Sent a manny to salvage a wreck.".into()),
        Some(ScriptAction::Detach { mode, .. }) => {
            let how = if mode == "hidden_on_asteroid" {
                "hiding it on an asteroid"
            } else {
                "leaving it adrift"
            };
            (kind::CONTAINER, format!("Detached a storage container, {how}."))
        }
        Some(ScriptAction::Recover { .. }) => (kind::CONTAINER, "Recovered a detached storage container.".into()),
        Some(ScriptAction::Craft { recipe_id, .. }) => {
            if group.len() > 1 {
                (
                    kind::CRAFT,
                    format!("Started fabricating {} parts in parallel.", group.len()),
                )
            } else {
                (kind::CRAFT, format!("Started fabricating «{recipe_id}»."))
            }
        }
        None => ("script", "Ran an empty step.".into()),
    };
    LogEvent::action(k, summary, probe_id)
}

impl AppState {
    // ── Composition ────────────────────────────────────────────────────────

    /// Validate a line syntactically and append it as a step. Returns the parse
    /// error (for inline display in the editor) without mutating on failure.
    pub fn enqueue_script_line(&mut self, line: &str) -> Result<(), String> {
        let cmd = self.parse_script_line(line)?;
        if self.script.len() >= SCRIPT_MAX {
            return Err(format!("script full ({SCRIPT_MAX})"));
        }
        self.script.push(ScriptStep {
            raw: line.trim().to_string(),
            cmd,
            state: StepState::Pending,
            resolved: Vec::new(),
        });
        Ok(())
    }

    pub fn script_remove(&mut self, idx: usize) {
        if idx < self.script.len() {
            self.script.remove(idx);
        }
    }

    pub fn script_clear(&mut self) {
        self.script.clear();
        self.script_running = false;
    }

    /// Start (or resume) the script: retry any failed step and run from there.
    pub fn script_run(&mut self) {
        if self.script.is_empty() {
            self.set_toast("script is empty");
            return;
        }
        if self.script.iter().all(|s| matches!(s.state, StepState::Done)) {
            self.set_toast("script already finished");
            return;
        }
        for s in &mut self.script {
            if matches!(s.state, StepState::Failed(_)) {
                s.state = StepState::Pending;
                s.resolved.clear();
            }
        }
        self.script_running = true;
        self.set_toast("script running");
    }

    /// Pause or resume the running script.
    pub fn script_toggle_pause(&mut self) {
        self.script_running = !self.script_running;
        self.set_toast(if self.script_running {
            "script running"
        } else {
            "script paused"
        });
    }

    /// Syntactic validation only — no live-state lookups (those happen in
    /// `resolve` at fire time). Catches an unknown verb, malformed coordinates,
    /// unknown resource names, and a non-numeric repair percent.
    pub fn parse_script_line(&self, line: &str) -> Result<ScriptCommand, String> {
        let mut it = line.split_whitespace();
        let verb_tok = it.next().ok_or("empty line")?;
        let verb = ScriptVerb::parse(verb_tok)
            .ok_or_else(|| format!("unknown action \"{verb_tok}\" — travel/mine/repair/salvage/detach/recover"))?;
        let args: Vec<String> = it.map(String::from).collect();

        match verb {
            ScriptVerb::Travel => {
                let buf = args.join(" ");
                let rest = buf.trim().strip_prefix('+').unwrap_or(buf.trim());
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if parts.len() != 3 || parts.iter().any(|t| t.parse::<i32>().is_err()) {
                    return Err("travel: usage <x y z | +dx dy dz>".into());
                }
            }
            ScriptVerb::Mine => {
                for tok in &args {
                    if matches!(tok.as_str(), "by" | "at" | "to") {
                        break;
                    }
                    if tok.parse::<f64>().is_ok() {
                        continue;
                    }
                    for r in tok.split(',').filter(|s| !s.is_empty()) {
                        if mine_resource(r).is_none() {
                            return Err(format!("unknown resource \"{r}\""));
                        }
                    }
                }
            }
            ScriptVerb::Repair => {
                if let Some(first) = args.first() {
                    if first != "by" && first.parse::<f64>().is_err() {
                        return Err(format!("repair: bad percent \"{first}\""));
                    }
                }
            }
            ScriptVerb::Craft => {
                // Recipe token required; the recipe/builder resolve at fire time.
                if args.first().is_none_or(|t| t == "by") {
                    return Err("craft: usage <recipe> [by <manny>]".into());
                }
            }
            ScriptVerb::Salvage | ScriptVerb::Detach | ScriptVerb::Recover => {}
        }
        Ok(ScriptCommand { verb, args })
    }

    // ── Resolution (late binding) ────────────────────────────────────────────

    /// Resolve a validated command against the live roster/sector into the
    /// **group** of actions to fire. `Err` carries the reason a target could not
    /// be found (used to fail the step). Only `mine` fans out to more than one
    /// action (one per builder); every other verb resolves to a single action.
    fn resolve(&self, cmd: &ScriptCommand) -> Result<Vec<ScriptAction>, String> {
        let args: Vec<&str> = cmd.args.iter().map(String::as_str).collect();
        let one = |a: ScriptAction| Ok(vec![a]);
        match cmd.verb {
            ScriptVerb::Travel => {
                let buf = cmd.args.join(" ");
                match Self::parse_travel_buf(&buf, self.probe_sector_coords()) {
                    Some((x, y, z)) if (x + y + z) % 2 == 0 => one(ScriptAction::Travel { x, y, z }),
                    Some(_) => Err("travel: x+y+z must be even".into()),
                    None => Err("travel: usage <x y z | +dx dy dz>".into()),
                }
            }
            ScriptVerb::Mine => {
                // Fan-out: `by all` / `by A,B` mine the same asteroid+container in
                // parallel; the step joins on all of them (see `advance_script`).
                let (positional, by, at, to) = mine_buckets(&args);
                let target = self.resolve_mine_target(&positional, &at, &to)?;
                let builders = self.resolve_builders(&by)?;
                Ok(builders
                    .into_iter()
                    .map(|(manny_id, _)| ScriptAction::Mine {
                        manny_id,
                        object_id: target.object_id.clone(),
                        resources: target.resources.clone(),
                        amount: target.amount,
                        container_id: target.container_id.clone(),
                    })
                    .collect())
            }
            ScriptVerb::Repair => {
                let (pos, by, _, _) = split_kw(&args);
                let (manny_id, _) = self.resolve_builder(&by)?;
                let pct = match pos.first() {
                    Some(t) => t.parse::<f64>().map_err(|_| format!("repair: bad percent \"{t}\""))?,
                    None => 100.0,
                };
                if pct <= 0.0 {
                    return Err("repair: percent must be positive".into());
                }
                one(ScriptAction::Repair {
                    manny_id,
                    integrity_percent: pct,
                })
            }
            ScriptVerb::Salvage => {
                let (pos, by, at, _) = split_kw(&args);
                let (manny_id, _) = self.resolve_builder(&by)?;
                let query = if at.is_empty() { pos.join(" ") } else { at.join(" ") };
                let (object_id, _) = pick_one(self.collect_salvage_candidates(), &query, "salvageable wreck")?;
                one(ScriptAction::Salvage { manny_id, object_id })
            }
            ScriptVerb::Recover => {
                let (pos, by, at, _) = split_kw(&args);
                let (manny_id, _) = self.resolve_builder(&by)?;
                let query = if at.is_empty() { pos.join(" ") } else { at.join(" ") };
                let (object_id, _) = pick_one(self.collect_detached_containers(), &query, "detached container")?;
                one(ScriptAction::Recover { manny_id, object_id })
            }
            ScriptVerb::Detach => {
                let (pos, by, at, mode) = split_kw(&args);
                let (manny_id, _) = self.resolve_builder(&by)?;
                let (container_id, _) = pick_one(self.collect_detachable_containers(), &pos.join(" "), "container")?;
                let mode = if mode.is_empty() {
                    "drifting".to_string()
                } else {
                    mode.join("")
                };
                // Scripts support only the modes whose target resolves from the
                // line; attach_to_probe (v91) needs a probe picker, so it stays
                // interactive-only for now.
                if mode != "drifting" && mode != "hidden_on_asteroid" {
                    return Err("detach: mode must be drifting or hidden_on_asteroid".into());
                }
                let object_id = if mode == "hidden_on_asteroid" {
                    // Same candidate set as the interactive detach wizard — any
                    // asteroid in the sector, not only mineable ones.
                    let (id, _) = pick_one(self.collect_asteroid_candidates(), &at.join(" "), "asteroid")?;
                    Some(id)
                } else {
                    None
                };
                one(ScriptAction::Detach {
                    manny_id,
                    container_id,
                    mode,
                    object_id,
                })
            }
            ScriptVerb::Craft => {
                let (pos, by, _, _) = split_kw(&args);
                let joined = pos.join(" ");
                // Comma-separated recipe list: `craft A,B,C by all` fans out one
                // recipe per builder, in parallel, joining on all (like mine).
                let queries: Vec<String> = joined
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if queries.is_empty() {
                    return Err("craft: name a recipe".into());
                }
                // Resolve each query → (fabricator, recipe_id). Scoped so the
                // `recipes` borrow of self drops before the builder lookups.
                let resolved: Vec<(Fabricator, String)> = {
                    let recipes = self.fabrication_recipes();
                    let resolve_one = |q: &str| -> Result<(Fabricator, String), String> {
                        recipes
                            .iter()
                            .find(|(_, r)| r.name.eq_ignore_ascii_case(q) || r.id.eq_ignore_ascii_case(q))
                            .or_else(|| {
                                let ql = q.to_lowercase();
                                recipes.iter().find(|(_, r)| r.name.to_lowercase().contains(&ql))
                            })
                            .map(|(fab, r)| (*fab, r.id.clone()))
                            .ok_or_else(|| format!("craft: no recipe matching \"{q}\""))
                    };
                    queries.iter().map(|q| resolve_one(q)).collect::<Result<_, _>>()?
                };

                // Single recipe: keep the simple path (sole idle builder, or `by`).
                if resolved.len() == 1 {
                    let (fabricator, recipe_id) = resolved.into_iter().next().unwrap();
                    return match fabricator {
                        Fabricator::AtomicPrinter => {
                            if !self.has_atomic_printer() {
                                return Err("craft: no atomic printer in inventory".into());
                            }
                            one(ScriptAction::Craft {
                                fabricator,
                                manny_id: None,
                                recipe_id,
                            })
                        }
                        Fabricator::Manny => {
                            let (manny_id, _) = self.resolve_builder(&by)?;
                            one(ScriptAction::Craft {
                                fabricator,
                                manny_id: Some(manny_id),
                                recipe_id,
                            })
                        }
                    };
                }

                // Fan-out: one recipe per builder. Manny recipes each take a
                // distinct idle builder; a printer recipe uses the printer lane.
                let manny_count = resolved.iter().filter(|(f, _)| *f == Fabricator::Manny).count();
                let printer_count = resolved.len() - manny_count;
                if printer_count > 1 {
                    return Err("craft: at most one atomic-printer recipe per parallel step".into());
                }
                if printer_count == 1 && !self.has_atomic_printer() {
                    return Err("craft: no atomic printer in inventory".into());
                }
                let builders = self.resolve_builders(&by)?;
                if builders.len() < manny_count {
                    return Err(format!(
                        "craft: need {manny_count} idle Mannies for the parts, have {} — add by all|A,B",
                        builders.len()
                    ));
                }
                let mut actions = Vec::new();
                let mut bi = 0;
                for (fabricator, recipe_id) in resolved {
                    match fabricator {
                        Fabricator::Manny => {
                            let (manny_id, _) = builders[bi].clone();
                            bi += 1;
                            actions.push(ScriptAction::Craft {
                                fabricator,
                                manny_id: Some(manny_id),
                                recipe_id,
                            });
                        }
                        Fabricator::AtomicPrinter => actions.push(ScriptAction::Craft {
                            fabricator,
                            manny_id: None,
                            recipe_id,
                        }),
                    }
                }
                Ok(actions)
            }
        }
    }

    /// The single builder Manny for a non-fan-out action: the `by` override, else
    /// the sole idle onboard Manny. Rejects the ambiguous many-idle case.
    fn resolve_builder(&self, by: &[&str]) -> Result<(String, String), String> {
        if by.is_empty() {
            let mannies = self.collect_idle_onboard_mannies();
            match mannies.len() {
                0 => Err("no idle Manny on board".into()),
                1 => Ok(mannies.into_iter().next().unwrap()),
                _ => Err("multiple idle Mannies — add by <manny>".into()),
            }
        } else {
            self.resolve_idle_manny(&by.join(" "))
                .ok_or_else(|| format!("no idle Manny matching \"{}\"", by.join(" ")))
        }
    }

    /// Builders for a fan-out `mine`: `by all` → every idle onboard Manny; `by
    /// A,B` → those named (comma-separated); bare / a single name → exactly one
    /// (like `:mine`). Empty resolves are an error so the step fails loudly.
    fn resolve_builders(&self, by: &[&str]) -> Result<Vec<(String, String)>, String> {
        if by.is_empty() {
            return self.resolve_builder(by).map(|m| vec![m]);
        }
        if by.len() == 1 && by[0].eq_ignore_ascii_case("all") {
            let idle = self.collect_idle_onboard_mannies();
            return if idle.is_empty() {
                Err("no idle Manny on board".into())
            } else {
                Ok(idle)
            };
        }
        let joined = by.join(" ");
        let mut out: Vec<(String, String)> = Vec::new();
        for token in joined.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            match self.resolve_idle_manny(token) {
                Some(m) if !out.iter().any(|(id, _)| id == &m.0) => out.push(m),
                Some(_) => {} // already added
                None => return Err(format!("no idle Manny matching \"{token}\"")),
            }
        }
        if out.is_empty() {
            Err("no idle Manny resolved".into())
        } else {
            Ok(out)
        }
    }

    // ── Executor ─────────────────────────────────────────────────────────────

    /// Whether a resolved action is still in flight: a travel while the probe is
    /// moving, a Manny action while its builder is busy (not accepting orders).
    /// Mirrors `craft_target_busy`.
    fn action_in_progress(&self, action: &ScriptAction) -> bool {
        // An atomic-printer craft has no builder of its own; it is in flight while
        // any Manny is assisting the printer (mirrors the queue's printer lane).
        if let ScriptAction::Craft {
            fabricator: Fabricator::AtomicPrinter,
            ..
        } = action
        {
            return self.mannies.as_ref().is_some_and(|ms| {
                ms.iter()
                    .any(|m| m.current_task == Some(crate::api::types::MannyTask::AssistingAtomicPrinter))
            });
        }
        match action.manny_id() {
            None => self.movement_arrival.is_some(),
            Some(id) => self
                .mannies
                .as_ref()
                .is_some_and(|ms| ms.iter().any(|m| m.id == id && !m.can_receive_orders)),
        }
    }

    /// Barrier state of a fired step's group: still in flight while **any** of
    /// its actions is running; complete only once **all** have finished (join).
    fn group_in_progress(&self, group: &[ScriptAction]) -> bool {
        group.iter().any(|a| self.action_in_progress(a))
    }

    /// Advance the single-lane sequential executor. At most one step is in
    /// flight: a running step is polled to completion (busy→idle, or movement
    /// clearing) before the next pending step resolves + fires. Cheap and
    /// idempotent — called every loop tick. Staged fires land in `script_fire`,
    /// drained by the event loop.
    pub fn advance_script(&mut self) {
        if !self.script_running {
            return;
        }

        // A) A running step: poll its action group for the join (all done).
        if let Some(idx) = self.script.iter().position(ScriptStep::is_running) {
            let group = self.script[idx].resolved.clone();
            let in_progress = self.group_in_progress(&group);
            if let StepState::Running { observed_busy } = &mut self.script[idx].state {
                if !*observed_busy {
                    // Wait for the targets to pick the order up (fire→busy lag).
                    // For a fan-out group this fires the same tick and each craft
                    // runs for minutes, so "seen any busy" then "none busy" is a
                    // sound join; a builder that starts+finishes within one 4 s
                    // poll would be missed, which minute-scale work never does.
                    if in_progress {
                        *observed_busy = true;
                    }
                } else if !in_progress {
                    self.script[idx].state = StepState::Done;
                }
            }
            return; // strict: nothing else starts while a step runs.
        }

        // B) No step running → resolve + fire the first pending step's group.
        if let Some(idx) = self.script.iter().position(|s| matches!(s.state, StepState::Pending)) {
            match self.resolve(&self.script[idx].cmd) {
                Ok(group) => {
                    self.script_fire.extend(group.iter().cloned());
                    self.log_event(script_log_event(&group, self.active_probe_id));
                    self.script[idx].resolved = group;
                    self.script[idx].state = StepState::Running { observed_busy: false };
                }
                Err(msg) => self.fail_script(msg),
            }
        }

        // All steps terminal → stop running.
        if self.script.iter().all(ScriptStep::is_terminal) {
            self.script_running = false;
        }
    }

    /// Halt the script: mark the current (first non-terminal) step failed and
    /// pause. Mirrors `fail_queue`. Called on a resolve failure or when an API
    /// error arrives while a step is in flight (`script_note_error`).
    pub fn fail_script(&mut self, msg: String) {
        if let Some(step) = self.script.iter_mut().find(|s| !s.is_terminal()) {
            step.state = StepState::Failed(msg);
        }
        self.script_running = false;
    }

    /// If a scripted step is in flight, attribute an incoming action error to it
    /// (the strict single lane means it can only be this step) and halt.
    pub fn script_note_error(&mut self, msg: &str) {
        if self.script.iter().any(ScriptStep::is_running) {
            self.fail_script(msg.to_string());
        }
    }

    /// Whether the script is actively running (drives the brisk poll cadence and
    /// the status-bar chip).
    pub fn script_active(&self) -> bool {
        self.script_running
            && self
                .script
                .iter()
                .any(|s| matches!(s.state, StepState::Pending | StepState::Running { .. }))
    }

    /// `(done, total)` step counts, for the status-bar chip. `done` counts
    /// terminal steps.
    pub fn script_progress(&self) -> (usize, usize) {
        let done = self.script.iter().filter(|s| s.is_terminal()).count();
        (done, self.script.len())
    }
}
