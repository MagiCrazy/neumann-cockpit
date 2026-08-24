//! Production queue (#197) — a queue of **crafts** (Manny or atomic-printer)
//! organised into **lanes**: one lane per builder Manny, plus one for the atomic
//! printer. A lane runs one craft at a time, but lanes run in **parallel**, so an
//! idle Manny starts its next craft while another is still working. Completion is
//! detected by polling (the server has no push): a Manny craft is done when its
//! builder is idle again, an atomic craft when no onboard Manny is still assisting
//! the printer. It auto-runs (drains as steps complete) but every step is a real
//! API call the pilot added, so it halts (pauses) on the first failure and is
//! capped.
//!
//! **One queue per probe** (issue #291). The live `craft_queue` belongs to the
//! piloted probe; switching probe parks it under its probe id and restores that
//! probe's own queue. Only the piloted probe's queue runs: completion is read
//! from `AppState::mannies`, which describes the piloted probe alone, so a queue
//! whose builders are elsewhere has no signal to advance on. Production is local
//! to a probe — probes drift far apart, and a shared queue across them would be
//! meaningless.
//!
//! The `repeat`/executor shape is a primitive #198 (scripting) and #199 (rules)
//! can reuse, but only the crafting surface is built here.

use super::*;
use crate::api::types::{MannyLocationType, MannyTask};
use std::time::Instant;

/// Cap on queue length; enqueuing past it is dropped with a toast so a runaway
/// `[Q]` never silently balloons into hundreds of API calls.
pub const QUEUE_MAX: usize = 32;

/// While the queue runs, the event loop polls at least this often (seconds) to
/// catch a craft finishing — the server offers no push, so completion is only
/// visible on the next fetch.
pub const QUEUE_POLL_SECS: u64 = 4;

#[derive(Clone, PartialEq)]
pub enum StepState {
    /// Not started.
    Pending,
    /// A repeat iteration is in flight. `observed_busy` guards the fire→busy
    /// lag: the builder reads idle for a beat after the order is accepted, so we
    /// only treat idle as *completion* once we have first seen it go busy.
    Running {
        observed_busy: bool,
    },
    Done,
    /// Halted here; carries the API error. The `completed` counter is kept so
    /// the overlay can show e.g. "✗ 3/10".
    Failed(String),
}

/// One crafting step: a recipe built `repeat` times by one target.
#[derive(Clone)]
pub struct QueuedCraft {
    pub fabricator: Fabricator,
    pub recipe_id: String,
    pub recipe_name: String,
    /// Builder Manny for a Manny craft; `None` for an atomic-printer craft
    /// (the printer auto-reserves a Manny).
    pub builder_manny_id: Option<String>,
    pub builder_manny_name: Option<String>,
    pub repeat: u32,
    pub completed: u32,
    pub state: StepState,
    /// The recipe's fabrication time, resolved at enqueue. Used as the fallback
    /// completion signal when the builder's busy window was never observed.
    pub duration_secs: u64,
    /// When the current iteration's order was staged. With `duration_secs` it
    /// bounds how long a `Running` step may wait for a busy it will never see.
    pub fired_at: Option<Instant>,
}

impl QueuedCraft {
    pub fn new(
        fabricator: Fabricator,
        recipe_id: String,
        recipe_name: String,
        builder_manny_id: Option<String>,
        builder_manny_name: Option<String>,
    ) -> Self {
        Self {
            fabricator,
            recipe_id,
            recipe_name,
            builder_manny_id,
            builder_manny_name,
            repeat: 1,
            completed: 0,
            state: StepState::Pending,
            duration_secs: 0,
            fired_at: None,
        }
    }

    /// Two steps merge when they are the same recipe by the same target — so
    /// consecutive `[Q]` presses on a base element stack into one `×N` step.
    pub fn coalesces_with(&self, o: &QueuedCraft) -> bool {
        self.fabricator == o.fabricator && self.recipe_id == o.recipe_id && self.builder_manny_id == o.builder_manny_id
    }

    pub fn is_running(&self) -> bool {
        matches!(self.state, StepState::Running { .. })
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self.state, StepState::Done | StepState::Failed(_))
    }

    /// Whether this iteration has been in flight for longer than the recipe
    /// takes to build. The API exposes no per-Manny task history, so when the
    /// builder's busy window was missed entirely — a short recipe between two
    /// polls, or a craft that ran while its probe was not piloted — the recipe's
    /// own duration is the only honest completion signal we have. Without it a
    /// step that was never seen busy waits forever (issue #291).
    fn ran_its_course(&self) -> bool {
        match (self.duration_secs, self.fired_at) {
            (0, _) | (_, None) => false,
            (secs, Some(t)) => t.elapsed().as_secs() >= secs,
        }
    }
}

/// A queue set aside because its probe is not the one being piloted. Kept whole
/// — steps, progress and pause state — so returning to that probe resumes
/// exactly where the pilot left off.
#[derive(Clone, Default)]
pub struct ParkedQueue {
    pub steps: Vec<QueuedCraft>,
    pub paused: bool,
}

/// A craft the executor wants spawned, drained by the event loop (which owns the
/// `ApiClient` + sender) — mirrors `pending_fire`.
#[derive(Clone)]
pub struct CraftFire {
    pub fabricator: Fabricator,
    pub builder_manny_id: Option<String>,
    pub recipe_id: String,
}

fn fire_of(step: &QueuedCraft) -> CraftFire {
    CraftFire {
        fabricator: step.fabricator,
        builder_manny_id: step.builder_manny_id.clone(),
        recipe_id: step.recipe_id.clone(),
    }
}

/// `(recipe_name, is_atomic)` for the ship's-log entry when a craft fires.
fn log_of(step: &QueuedCraft) -> (String, bool) {
    (
        step.recipe_name.clone(),
        matches!(step.fabricator, Fabricator::AtomicPrinter),
    )
}

impl AppState {
    /// Add a craft to the queue: coalesce with the last step if identical
    /// (bumping its `repeat`), else push — unless the cap is hit.
    pub fn enqueue_craft(&mut self, mut craft: QueuedCraft) {
        // Bind the live queue to the piloted probe before touching it, so a
        // step is never appended to the queue of the probe we just left.
        self.sync_queue_probe();
        craft.duration_secs = self.recipe_duration_secs(&craft.recipe_id);
        if let Some(last) = self.craft_queue.last_mut() {
            if !last.is_terminal() && last.coalesces_with(&craft) {
                last.repeat += craft.repeat;
                let (name, n) = (last.recipe_name.clone(), last.repeat);
                self.set_toast(format!("queued {name} ×{n}"));
                return;
            }
        }
        if self.craft_queue.len() >= QUEUE_MAX {
            self.set_toast(format!("queue full ({QUEUE_MAX}) — step dropped"));
            return;
        }
        let name = craft.recipe_name.clone();
        self.craft_queue.push(craft);
        self.set_toast(format!("queued {name}"));
    }

    /// Pause or resume the queue. The queue auto-runs whenever it has work, so
    /// this is the only run control the pilot needs.
    pub fn queue_toggle_pause(&mut self) {
        self.queue_paused = !self.queue_paused;
        self.set_toast(if self.queue_paused {
            "queue paused"
        } else {
            "queue running"
        });
    }

    pub fn queue_remove(&mut self, idx: usize) {
        if idx < self.craft_queue.len() {
            self.craft_queue.remove(idx);
        }
    }

    pub fn queue_clear(&mut self) {
        self.craft_queue.clear();
    }

    /// Adjust a step's repeat count (never below what's already done, min 1).
    pub fn queue_bump(&mut self, idx: usize, delta: i32) {
        if let Some(s) = self.craft_queue.get_mut(idx) {
            let floor = s.completed.max(1) as i32;
            s.repeat = (s.repeat as i32 + delta).max(floor) as u32;
        }
    }

    /// The recipe's fabrication time, resolved from the catalog at enqueue so
    /// the executor keeps working off a self-contained step.
    fn recipe_duration_secs(&self, recipe_id: &str) -> u64 {
        self.fabrication_recipes()
            .iter()
            .find(|(_, r)| r.id == recipe_id)
            .map(|(_, r)| r.duration_seconds.max(0) as u64)
            .unwrap_or(0)
    }

    /// Bind the live queue to the piloted probe, parking the outgoing one and
    /// restoring that probe's own (issue #291). Cheap and idempotent — a no-op
    /// on every tick where the pilot has not switched probe.
    pub fn sync_queue_probe(&mut self) {
        let active = self.active_probe_id;
        if self.queue_probe == Some(active) {
            return;
        }
        if let Some(prev) = self.queue_probe {
            let steps = std::mem::take(&mut self.craft_queue);
            if steps.iter().any(|s| !s.is_terminal()) {
                self.parked_queues.insert(
                    prev,
                    ParkedQueue {
                        steps,
                        paused: self.queue_paused,
                    },
                );
            } else {
                // Nothing left to run: drop it rather than hoard finished steps
                // for a probe the pilot may never return to.
                self.parked_queues.remove(&prev);
            }
        }
        match self.parked_queues.remove(&active) {
            Some(q) => {
                self.craft_queue = q.steps;
                self.queue_paused = q.paused;
            }
            None => {
                self.craft_queue.clear();
                self.queue_paused = false;
            }
        }
        self.queue_probe = Some(active);
    }

    /// Forget the queue of a probe that no longer exists (destroyed or trapped,
    /// v94) — live or parked. Its builders are gone with it.
    pub fn forget_queue_for(&mut self, probe: Option<u64>) {
        self.parked_queues.remove(&probe);
        if self.queue_probe == Some(probe) {
            self.craft_queue.clear();
            self.queue_paused = false;
            self.queue_probe = None;
        }
    }

    /// Steps still to run in the queues of probes that are not being piloted —
    /// the status-bar chip that keeps a parked queue from looking like a lost
    /// one.
    pub fn parked_pending(&self) -> usize {
        self.parked_queues
            .values()
            .flat_map(|q| q.steps.iter())
            .filter(|s| !s.is_terminal())
            .count()
    }

    /// Whether this craft's target is currently busy on the server — a Manny
    /// builder not accepting orders, or (atomic) any onboard Manny assisting the
    /// printer.
    fn craft_target_busy(&self, craft: &QueuedCraft) -> bool {
        let Some(ms) = &self.mannies else { return false };
        match &craft.builder_manny_id {
            Some(id) => ms.iter().any(|m| &m.id == id && !m.can_receive_orders),
            None => ms.iter().any(|m| {
                m.location.location_type == MannyLocationType::Probe
                    && matches!(m.current_task, Some(MannyTask::AssistingAtomicPrinter))
            }),
        }
    }

    /// Advance the queue. Each **lane** (a builder Manny, or the atomic printer)
    /// runs one craft at a time, but lanes run in **parallel** — an idle Manny
    /// starts its next craft while another is still working. Completion is the
    /// per-lane busy→idle transition; started crafts are staged in `queue_fire`.
    /// Cheap and idempotent — called every loop tick.
    pub fn advance_queue(&mut self) {
        self.sync_queue_probe();
        if self.queue_paused {
            return;
        }
        // Completion is read from the Manny roster, which describes the piloted
        // probe alone. A roster fetched for another probe reports this queue's
        // builders as absent, which `craft_target_busy` cannot distinguish from
        // idle — advancing on it completes steps that never ran and fires orders
        // at the wrong probe (issue #291). Wait for a roster we can trust.
        if !self.roster_matches_active() {
            return;
        }
        let mut fires: Vec<(CraftFire, String, bool)> = Vec::new();

        // A) Completion: advance every running step whose target went idle.
        let running: Vec<usize> = self
            .craft_queue
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_running())
            .map(|(i, _)| i)
            .collect();
        for idx in running {
            let busy = self.craft_target_busy(&self.craft_queue[idx]);
            let step = &mut self.craft_queue[idx];
            let StepState::Running { observed_busy } = &mut step.state else {
                continue;
            };
            let finished = if *observed_busy {
                !busy
            } else if busy {
                // The target picked the order up.
                *observed_busy = true;
                false
            } else {
                // Never seen busy. Either the order is still being picked up, or
                // the whole craft ran and finished unobserved — which is what a
                // probe switch used to turn into a permanent stall.
                step.ran_its_course()
            };
            if finished {
                step.completed += 1;
                if step.completed >= step.repeat {
                    step.state = StepState::Done;
                } else {
                    step.state = StepState::Running { observed_busy: false };
                    step.fired_at = Some(Instant::now());
                    fires.push((fire_of(step), step.recipe_name.clone(), log_of(step).1));
                }
            }
        }

        // B) Start the first pending step of every free lane: a lane is free
        // when it has no running step and its builder is idle (or the printer is
        // free). Different builders fire together → parallel execution.
        let mut i = 0;
        while i < self.craft_queue.len() {
            if matches!(self.craft_queue[i].state, StepState::Pending) {
                let lane = self.craft_queue[i].builder_manny_id.clone();
                let lane_running = self
                    .craft_queue
                    .iter()
                    .any(|s| s.is_running() && s.builder_manny_id == lane);
                let builder_free = !self.craft_target_busy(&self.craft_queue[i]);
                if !lane_running && builder_free {
                    let step = &mut self.craft_queue[i];
                    step.state = StepState::Running { observed_busy: false };
                    step.fired_at = Some(Instant::now());
                    fires.push((fire_of(step), step.recipe_name.clone(), log_of(step).1));
                }
            }
            i += 1;
        }

        for (f, name, atomic) in fires {
            self.queue_fire.push(f);
            self.log_event(LogEvent::craft(&name, atomic, self.active_probe_id));
        }
    }

    /// Halt on a craft failure. With lanes running in parallel the erroring step
    /// can't be attributed from the (generic) error message, so this pauses the
    /// whole queue and marks the oldest running step failed — the pilot inspects,
    /// fixes, and resumes.
    pub fn fail_queue(&mut self, msg: String) {
        if let Some(step) = self.craft_queue.iter_mut().find(|s| s.is_running()) {
            step.state = StepState::Failed(msg);
        }
        self.queue_paused = true;
    }

    /// Whether the queue is actively working (unpaused with a pending/running
    /// step) — drives the faster poll cadence and the status-bar indicator.
    pub fn queue_active(&self) -> bool {
        !self.queue_paused
            && self
                .craft_queue
                .iter()
                .any(|s| matches!(s.state, StepState::Pending | StepState::Running { .. }))
    }

    /// `(done, total)` step counts across the queue, for the status-bar chip.
    /// `done` counts terminal steps; `total` all steps.
    pub fn queue_progress(&self) -> (usize, usize) {
        let done = self.craft_queue.iter().filter(|s| s.is_terminal()).count();
        (done, self.craft_queue.len())
    }
}
