//! Production queue (#197) — a single sequential queue of **crafts** (Manny or
//! atomic-printer). One step runs at a time; the next fires when the previous
//! completes, detected by polling (the server has no push): a Manny craft is
//! done when its builder is idle again, an atomic-printer craft when no onboard
//! Manny is still assisting the printer. It auto-runs (drains as steps complete)
//! but every step is a real API call the pilot added, so it halts (pauses) on
//! the first failure and is capped.
//!
//! The `repeat`/executor shape is a primitive #198 (scripting) and #199 (rules)
//! can reuse, but only the crafting surface is built here.

use super::*;
use crate::api::types::{MannyLocationType, MannyTask};

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
    pub fn enqueue_craft(&mut self, craft: QueuedCraft) {
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

    /// Advance the queue: evaluate the running step's completion, then fire the
    /// next iteration or the next step. Stages a `CraftFire` in `queue_fire` for
    /// the event loop to spawn. Cheap and idempotent — called every loop tick.
    pub fn advance_queue(&mut self) {
        if self.queue_paused {
            return;
        }
        // A step is running: watch its target for the busy→idle completion.
        if let Some(idx) = self.craft_queue.iter().position(|s| s.is_running()) {
            let busy = self.craft_target_busy(&self.craft_queue[idx]);
            let mut fire = None;
            {
                let step = &mut self.craft_queue[idx];
                if let StepState::Running { observed_busy } = &mut step.state {
                    if !*observed_busy {
                        // Waiting for the target to pick up the order.
                        if busy {
                            *observed_busy = true;
                        }
                    } else if !busy {
                        // This iteration finished.
                        step.completed += 1;
                        if step.completed >= step.repeat {
                            step.state = StepState::Done;
                        } else {
                            step.state = StepState::Running { observed_busy: false };
                            fire = Some((fire_of(step), log_of(step)));
                        }
                    }
                }
            }
            if let Some((f, (name, atomic))) = fire {
                self.queue_fire = Some(f);
                self.log_event(LogEvent::craft(&name, atomic, self.active_probe_id));
            }
            return;
        }
        // Nothing running: start the next pending step (idle if drained).
        if let Some(idx) = self
            .craft_queue
            .iter()
            .position(|s| matches!(s.state, StepState::Pending))
        {
            let (f, (name, atomic)) = {
                let step = &mut self.craft_queue[idx];
                step.state = StepState::Running { observed_busy: false };
                (fire_of(step), log_of(step))
            };
            self.queue_fire = Some(f);
            self.log_event(LogEvent::craft(&name, atomic, self.active_probe_id));
        }
    }

    /// Halt on a craft failure: the running step records the error and keeps its
    /// `completed` counter; the queue pauses so nothing else fires until the
    /// pilot fixes the cause and resumes.
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
