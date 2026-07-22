use anyhow::Result;
use crossterm::{
    cursor::Show,
    event::EventStream,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tokio::sync::mpsc;

use neumann_cockpit::api::tasks::{
    fetch_all, fetch_api_version, fetch_atomic_printer_craft, fetch_craft, fetch_crafting_recipes, fetch_detach,
    fetch_mannies, fetch_messages, fetch_mine, fetch_missions, fetch_move, fetch_recover, fetch_repair, fetch_salvage,
    fetch_sent_messages,
};
use neumann_cockpit::app::{
    ActiveWizard, ApiMessage, AppState, ColorMode, Fabricator, MessagesInput, MissionsInput, Refetch, RemoteMineInput,
    ScriptAction, ScutNetworkInput,
};
use neumann_cockpit::input::handle_event;
use neumann_cockpit::preflight;
use neumann_cockpit::store;
use neumann_cockpit::ui;

/// Best-effort restoration of the terminal to its cooked state. Writes the
/// leave sequences straight to `stdout` so it can run from a panic hook, where
/// the `Terminal` value is out of reach.
fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, Show)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Headless script runner: `--script <file>` plays a script with no TUI and
    // streams the ship's-log to stdout (see `headless`). A bare launch is the
    // interactive cockpit, unchanged.
    let args: Vec<String> = std::env::args().collect();
    if let Some(path) = neumann_cockpit::headless::script_arg(&args) {
        let code = neumann_cockpit::headless::run(std::path::Path::new(&path)).await?;
        std::process::exit(code);
    }

    // Enter the alternate screen FIRST — before any fallible startup. A missing
    // or keyless config used to error out of `main` before the terminal was set
    // up, which on a double-clicked Windows binary flashed a console and
    // vanished. Now the preflight screen is up first and every failure (and the
    // first-run key onboarding) has an in-TUI outcome.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    // No mouse capture: the cockpit handles no mouse events, and capturing would
    // steal the terminal's native selection/copy and scroll-wheel.
    execute!(stdout, EnterAlternateScreen)?;

    // A panic anywhere in the ~16k lines of render/input unwinds past the
    // teardown below, leaving the shell in raw mode and the panic message hidden
    // in the alternate screen. Restore the terminal first, then let the original
    // hook print the report to the real screen.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        original_hook(info);
    }));

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Preflight: config check + first-run onboarding, local archive migration,
    // and the remote link check — all drawn in-screen.
    let ready = match preflight::run(&mut terminal, ColorMode::default()).await {
        Ok(preflight::Outcome::Ready(r)) => *r,
        Ok(preflight::Outcome::Quit) => {
            restore_terminal()?;
            return Ok(());
        }
        Err(e) => {
            restore_terminal()?;
            return Err(e);
        }
    };

    let result = run(&mut terminal, ready).await;

    restore_terminal()?;

    result
}

async fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, ready: preflight::Ready) -> Result<()> {
    let preflight::Ready {
        config,
        client,
        conn,
        scan_history,
        journal,
        telemetry,
        api_version,
        link_ok,
    } = ready;
    // Mutable so a probe switch can retarget every subsequent call (auto-refresh
    // + actions) at the newly-active probe — see the reconcile after handle_event.
    let mut client = client;
    let (tx, mut rx) = mpsc::channel::<ApiMessage>(32);
    let mut state = AppState {
        hints_visible: config.hints,
        color_mode: config.color_mode(),
        booting: config.boot,
        scan_history,
        journal,
        telemetry,
        api_version,
        ..Default::default()
    };
    // The remote link was already probed in the preflight; surface a down link
    // straight away so the pilot sees why data is missing (F5 retries).
    if !link_ok {
        state.set_error("remote link down — press F5 to retry".into());
    }
    // The persistence writer takes the connection opened during preflight; on a
    // DB error there we run without persistence (history already empty). It also
    // hands back a `degraded` flag it raises if a write ever fails (issue #216).
    let (persist_tx, persist_degraded) = match conn.map(store::spawn_writer) {
        Some((tx, degraded)) => (Some(tx), Some(degraded)),
        None => (None, None),
    };
    let mut events = EventStream::new();

    // Short-lived tick that drives the boot assembly; runs only while booting.
    let mut boot_tick = tokio::time::interval(std::time::Duration::from_millis(90));
    boot_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // Steady-state 1 s tick: redraws so time-derived values (progress bars,
    // percentages, ETAs, sync age) advance live, and triggers the periodic
    // ≤60 s auto-refresh when one is due.
    let mut ui_tick = tokio::time::interval(std::time::Duration::from_secs(1));
    ui_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // Initial data fetch
    fetch_all(client.clone(), tx.clone());
    fetch_api_version(client.clone(), tx.clone());
    fetch_crafting_recipes(client.clone(), tx.clone());
    state.loading = true;

    loop {
        // Surface a persistence failure raised by the writer thread (#216).
        if let Some(degraded) = &persist_degraded {
            state.persistence_degraded = degraded.load(std::sync::atomic::Ordering::Relaxed);
        }

        // Drain ship's-log entries staged by the previous tick's handlers:
        // persist each and prepend to the in-memory journal (newest first,
        // capped), mirroring how sector observations are persisted.
        if !state.pending_journal.is_empty() {
            let staged: Vec<_> = state.pending_journal.drain(..).collect();
            for ev in staged {
                if let Some(tx) = &persist_tx {
                    let _ = tx.send(store::PersistMsg::AppendEvent(ev.clone()));
                }
                state.journal.insert(0, ev);
            }
            state.journal.truncate(store::JOURNAL_WINDOW);
        }

        // Persist telemetry samples staged by update_probe (already appended to
        // the in-memory series; the DB keeps the full history for stats).
        if !state.pending_telemetry.is_empty() {
            let staged: Vec<_> = state.pending_telemetry.drain(..).collect();
            if let Some(tx) = &persist_tx {
                for s in staged {
                    let _ = tx.send(store::PersistMsg::AppendTelemetry(s));
                }
            }
        }

        // Desktop notifications staged when a long task completed. Always drain
        // (so they never accumulate); emit only when the pilot enabled them.
        if !state.pending_notifications.is_empty() {
            let staged: Vec<_> = state.pending_notifications.drain(..).collect();
            if config.notifications {
                for msg in staged {
                    neumann_cockpit::notify::desktop_notify(&msg);
                }
            }
        }

        // Production queue: advance the executor, then spawn any craft it staged
        // (the event loop owns the client + sender). A fresh mannies fetch lets
        // the next tick detect the craft going busy, then idle → complete.
        state.advance_queue();
        if !state.queue_fire.is_empty() {
            for fire in state.queue_fire.drain(..) {
                match fire.fabricator {
                    Fabricator::Manny => {
                        if let Some(builder) = fire.builder_manny_id {
                            fetch_craft(builder, fire.recipe_id, client.clone(), tx.clone());
                        }
                    }
                    Fabricator::AtomicPrinter => {
                        fetch_atomic_printer_craft(fire.recipe_id, client.clone(), tx.clone());
                    }
                }
            }
            // One roster refresh lets the next tick see the builders go busy.
            fetch_mannies(client.clone(), tx.clone());
            state.loading = true;
        }

        // Action script (#198): the sequential executor, then spawn the single
        // action it staged this tick. Same drain pattern as the queue; a roster
        // refresh primes busy→idle completion detection.
        state.advance_script();
        if !state.script_fire.is_empty() {
            for action in state.script_fire.drain(..) {
                match action {
                    ScriptAction::Travel { x, y, z } => fetch_move(x, y, z, client.clone(), tx.clone()),
                    ScriptAction::Mine {
                        manny_id,
                        object_id,
                        resources,
                        amount,
                        container_id,
                    } => fetch_mine(
                        manny_id,
                        object_id,
                        resources,
                        amount,
                        container_id,
                        client.clone(),
                        tx.clone(),
                    ),
                    ScriptAction::Repair {
                        manny_id,
                        integrity_percent,
                    } => fetch_repair(manny_id, integrity_percent, client.clone(), tx.clone()),
                    ScriptAction::Salvage { manny_id, object_id } => {
                        fetch_salvage(manny_id, object_id, client.clone(), tx.clone())
                    }
                    ScriptAction::Detach {
                        manny_id,
                        container_id,
                        mode,
                        object_id,
                    } => fetch_detach(manny_id, container_id, mode, object_id, client.clone(), tx.clone()),
                    ScriptAction::Recover { manny_id, object_id } => {
                        fetch_recover(manny_id, object_id, client.clone(), tx.clone())
                    }
                    ScriptAction::Craft {
                        fabricator,
                        manny_id,
                        recipe_id,
                    } => match fabricator {
                        Fabricator::Manny => {
                            if let Some(builder) = manny_id {
                                fetch_craft(builder, recipe_id, client.clone(), tx.clone());
                            }
                        }
                        Fabricator::AtomicPrinter => {
                            fetch_atomic_printer_craft(recipe_id, client.clone(), tx.clone());
                        }
                    },
                }
            }
            fetch_mannies(client.clone(), tx.clone());
            state.loading = true;
        }

        terminal.draw(|f| ui::render(f, &state))?;

        let deadline = state.next_refresh_instant();

        tokio::select! {
            Some(event) = events.next() => {
                handle_event(event?, &mut state, &client, &tx);
            }

            _ = boot_tick.tick(), if state.booting => {
                state.boot_tick();
            }

            _ = ui_tick.tick() => {
                // The redraw at the loop top makes live values tick; here we
                // only fire the periodic refresh when it is due.
                if !state.booting && state.periodic_refresh_due() {
                    fetch_all(client.clone(), tx.clone());
                    state.note_refresh_attempt();
                    state.loading = true;
                }
            }

            Some(msg) = rx.recv() => {
                state.loading = false;
                match msg {
                    ApiMessage::ProbeUpdated(probe) => state.update_probe(probe),
                    ApiMessage::FleetFetched(list) => state.update_fleet(list),
                    ApiMessage::DefaultProbeSet(list, name) => {
                        state.update_fleet(list);
                        state.set_toast(format!("{name} is now the default probe"));
                    }
                    ApiMessage::ProbeRenamed(list, name) => {
                        state.update_fleet(list);
                        state.close_wizard();
                        // Refresh so the Probe pane identity picks up the new name.
                        state.finish_action(format!("probe renamed to {name}"), Refetch::All);
                    }
                    ApiMessage::RenameProbeError(e) => state.set_wizard_error(e),
                    ApiMessage::ManniesUpdated(mannies) => state.update_mannies(mannies),
                    ApiMessage::SectorUpdated(sector) => {
                        let (sx, sy, sz) = (
                            sector.relative_coordinates.x as i32,
                            sector.relative_coordinates.y as i32,
                            sector.relative_coordinates.z as i32,
                        );
                        state.update_sector(sector);
                        state.remote_mine_sector_loaded(sx, sy, sz);
                        state.batch_tick();
                        // Persist just the observation that changed (upsert by
                        // coordinates), via the single writer thread.
                        if let (Some(tx), Some(obs)) = (&persist_tx, state.scan_history.first()) {
                            let _ = tx.send(store::PersistMsg::UpsertObservation(Box::new(obs.clone())));
                        }
                    }
                    ApiMessage::ScanError(e) => {
                        if matches!(state.active_wizard, ActiveWizard::RemoteMine(RemoteMineInput::Loading { .. })) {
                            // The remote-mine sector fetch failed — don't leave the
                            // wizard hung on "fetching…". Abort and surface why.
                            state.close_wizard();
                            state.set_error(e);
                        } else if state.scan_batch.is_some() {
                            state.batch_tick();
                        } else {
                            state.set_scan_error(e);
                        }
                    }
                    ApiMessage::MoveStarted(mv) => {
                        state.apply_movement(mv);
                        state.set_toast("travel order sent");
                    }
                    ApiMessage::MoveError(e) => {
                        state.script_note_error(&e);
                        state.set_travel_error(e);
                    }
                    ApiMessage::RepairStarted => {
                        state.close_wizard();
                        state.finish_action("repair order sent", Refetch::Mannies);
                    }
                    ApiMessage::RepairError(e) => state.set_wizard_error(e),
                    ApiMessage::MineStarted => {
                        state.close_wizard();
                        state.finish_action("mining order sent", Refetch::Mannies);
                    }
                    // Either the local or remote-mine wizard is open; the single
                    // set_wizard_error targets whichever it is.
                    ApiMessage::MineError(e) => state.set_wizard_error(e),
                    ApiMessage::JettisonDone(inv) => {
                        state.update_inventory(inv);
                        state.close_wizard();
                        // Jettison always adds an object to the sector (ejected manny,
                        // drifting item, or deployed SCUT relay) — refresh everything.
                        state.finish_action("jettisoned", Refetch::All);
                    }
                    ApiMessage::JettisonError(e) => state.set_wizard_error(e),
                    // Every craft now flows through the production queue, so a
                    // start is quiet (the fire path already refreshed mannies)
                    // and an error halts the queue.
                    ApiMessage::CraftStarted => {}
                    // A craft error can belong to the queue or a scripted craft;
                    // route to both (each no-ops if it wasn't the one that fired).
                    ApiMessage::CraftError(e) => {
                        state.script_note_error(&e);
                        state.fail_queue(e);
                    }
                    ApiMessage::SalvageStarted => {
                        state.close_wizard();
                        state.finish_action("salvage order sent", Refetch::Mannies);
                    }
                    ApiMessage::SalvageError(e) => state.set_wizard_error(e),
                    ApiMessage::RecallStarted => {
                        state.close_wizard();
                        state.finish_action("recall order sent", Refetch::Mannies);
                    }
                    ApiMessage::RecallError(e) => state.set_wizard_error(e),
                    ApiMessage::DeuteriumRefuelStarted => {
                        state.close_wizard();
                        state.finish_action("refuel order sent", Refetch::All);
                    }
                    ApiMessage::DeuteriumRefuelError(e) => state.set_wizard_error(e),
                    ApiMessage::DeuteriumTransferStarted => {
                        state.close_wizard();
                        state.finish_action("deuterium transfer order sent", Refetch::All);
                    }
                    ApiMessage::DeuteriumTransferError(e) => state.set_wizard_error(e),
                    ApiMessage::MannyTransferStarted => {
                        state.close_wizard();
                        state.finish_action("manny transfer order sent", Refetch::All);
                    }
                    ApiMessage::MannyTransferError(e) => state.set_wizard_error(e),
                    ApiMessage::MindSnapshotReassigned(probe) => {
                        state.close_wizard();
                        state.update_probe(probe);
                        state.finish_action("mind snapshot reassigned", Refetch::All);
                    }
                    ApiMessage::MindSnapshotReassignError(e) => state.set_wizard_error(e),
                    ApiMessage::MissionsFetched(missions) => state.missions = missions,
                    ApiMessage::MissionAbandoned(_) => {
                        state.active_wizard = ActiveWizard::Missions(MissionsInput::Browsing { selection: 0 });
                        state.finish_action("mission abandoned", Refetch::Missions);
                    }
                    ApiMessage::MissionAbandonError(e) => state.set_wizard_error(e),
                    ApiMessage::ScutRelayTurnedOn => {
                        state.close_wizard();
                        state.finish_action("relay turn-on order sent", Refetch::All);
                    }
                    ApiMessage::ScutRelayTurnOnError(e) => state.set_wizard_error(e),
                    ApiMessage::TransitBeaconStarted => {
                        state.finish_action("transit beacon install order sent", Refetch::All);
                    }
                    // Fired directly from the object-action picker (no wizard open),
                    // so the failure surfaces as a status-bar error, not a wizard one.
                    ApiMessage::TransitBeaconError(e) => state.set_error(e),
                    ApiMessage::ScutNetworkFetched(network) => {
                        if matches!(state.active_wizard, ActiveWizard::ScutNetwork(ScutNetworkInput::Viewing { .. })) {
                            state.scut_network_view = Some(network);
                        }
                    }
                    ApiMessage::MessagesFetched(m) => state.messages = m,
                    ApiMessage::SentMessagesFetched(m) => state.sent_messages = m,
                    ApiMessage::MessageSent(_) => {
                        state.active_wizard = ActiveWizard::Messages(MessagesInput::Browsing { sent_tab: false, selection: 0 });
                        state.finish_action("message sent", Refetch::Messages);
                    }
                    ApiMessage::MessageSendError(e) => state.set_wizard_error(e),
                    ApiMessage::MessageMarkedRead(m) => {
                        if let Some(slot) = state.messages.iter_mut().find(|x| x.id == m.id) {
                            *slot = m;
                        }
                    }
                    ApiMessage::ScutNetworkError(e) => {
                        if matches!(state.active_wizard, ActiveWizard::ScutNetwork(ScutNetworkInput::Viewing { .. })) {
                            state.active_wizard = ActiveWizard::ScutNetwork(ScutNetworkInput::Viewing { error: Some(e) });
                        }
                    }
                    ApiMessage::DeployStarted => {
                        state.close_wizard();
                        state.finish_action("waypoint deploy order sent", Refetch::All);
                    }
                    ApiMessage::DeployError(e) => state.set_wizard_error(e),
                    ApiMessage::AtomicPrinterCraftStarted => {}
                    ApiMessage::AtomicPrinterCraftError(e) => {
                        state.script_note_error(&e);
                        state.fail_queue(e);
                    }
                    ApiMessage::RecipesFetched(recipes) => state.recipes = recipes,
                    ApiMessage::ProbeImprovementsFetched(improvements) => {
                        state.probe_improvements = improvements;
                    }
                    ApiMessage::ImproveProbeStarted => {
                        state.close_wizard();
                        state.finish_action("probe improvement started", Refetch::All);
                    }
                    ApiMessage::ImproveProbeError(e) => state.set_wizard_error(e),
                    ApiMessage::InspectStarted => {
                        state.close_wizard();
                        state.finish_action("inspect order sent", Refetch::Mannies);
                    }
                    ApiMessage::InspectError(e) => state.set_inspect_error(e),
                    ApiMessage::RecoverStarted => {
                        state.close_wizard();
                        state.finish_action("recover order sent", Refetch::All);
                    }
                    ApiMessage::RecoverError(e) => state.set_recover_error(e),
                    ApiMessage::DetachStarted => {
                        state.close_wizard();
                        state.finish_action("detach order sent", Refetch::All);
                    }
                    ApiMessage::DetachError(e) => state.set_wizard_error(e),
                    ApiMessage::AlertsFetched(a) => state.alerts = a,
                    ApiMessage::DamageWarningsFetched(w, rule) => {
                        state.damage_warnings = w;
                        state.damage_warning_rule = Some(rule);
                    }
                    ApiMessage::AlertAcknowledged(a) => {
                        state.replace_alert(a);
                        state.set_toast("alert acknowledged");
                    }
                    ApiMessage::DamageWarningAcknowledged(w) => {
                        state.replace_damage_warning(w);
                        state.set_toast("warning acknowledged");
                    }
                    ApiMessage::StorageContainersFetched(c) => state.storage_containers = c,
                    ApiMessage::StorageContainerDetailFetched(c, inv) => {
                        state.storage_container_detail = Some((c, inv));
                        state.storage_container_detail_error = None;
                    }
                    ApiMessage::StorageContainerDetailError(e) => {
                        state.storage_container_detail = None;
                        state.storage_container_detail_error = Some(e);
                    }
                    ApiMessage::RenameContainerDone(c, inv) => {
                        state.apply_container_update(c, inv);
                        state.close_wizard();
                        state.set_toast("container renamed");
                    }
                    ApiMessage::RenameContainerError(e) => state.set_wizard_error(e),
                    ApiMessage::UpdateContainerRulesDone(c, inv) => {
                        state.apply_container_update(c, inv);
                        state.close_wizard();
                        state.set_toast("routing rules updated");
                    }
                    ApiMessage::UpdateContainerRulesError(e) => state.set_wizard_error(e),
                    ApiMessage::StorageMoveDone(manny, inv) => {
                        if let Some(ref mut mannies) = state.mannies {
                            if let Some(m) = mannies.iter_mut().find(|m| m.id == manny.id) {
                                *m = manny;
                            }
                        }
                        state.update_inventory(inv);
                        state.close_wizard();
                        state.set_toast("storage move order sent");
                    }
                    ApiMessage::StorageMoveError(e) => state.set_wizard_error(e),
                    ApiMessage::AssembleProbeStarted(manny, inv) => {
                        if let Some(ref mut mannies) = state.mannies {
                            if let Some(m) = mannies.iter_mut().find(|m| m.id == manny.id) {
                                *m = manny;
                            }
                        }
                        state.update_inventory(inv);
                        state.close_wizard();
                        // The new drone appears in the roster once assembled.
                        state.finish_action("drone assembly started (~3h)", Refetch::All);
                    }
                    ApiMessage::AssembleProbeError(e) => state.set_wizard_error(e),
                    ApiMessage::DropMannyCargoStarted(manny) => {
                        if let Some(ref mut mannies) = state.mannies {
                            if let Some(m) = mannies.iter_mut().find(|m| m.id == manny.id) {
                                *m = manny;
                            }
                        }
                        state.close_wizard();
                        // Recoverable objects may reappear in the sector.
                        state.finish_action("cargo dropped", Refetch::All);
                    }
                    ApiMessage::DropMannyCargoError(e) => state.set_wizard_error(e),
                    ApiMessage::DropStorageContainerStarted(manny) => {
                        if let Some(ref mut mannies) = state.mannies {
                            if let Some(m) = mannies.iter_mut().find(|m| m.id == manny.id) {
                                *m = manny;
                            }
                        }
                        state.close_wizard();
                        // Container + drop kit leave the inventory.
                        state.finish_action("drop container order sent", Refetch::All);
                    }
                    ApiMessage::DropStorageContainerError(e) => state.set_wizard_error(e),
                    ApiMessage::RenameMannyDone(manny) => {
                        if let Some(ref mut mannies) = state.mannies {
                            if let Some(m) = mannies.iter_mut().find(|m| m.id == manny.id) {
                                *m = manny;
                            }
                        }
                        state.close_wizard();
                        state.set_toast("manny renamed");
                    }
                    ApiMessage::RenameMannyError(e) => state.set_wizard_error(e),
                    ApiMessage::VersionFetched(v) => state.api_version = Some(v),
                    ApiMessage::VisitedSectorsFetched(v) => state.visited_sectors = v,
                    ApiMessage::ActionError(e) => state.set_error(e),
                    ApiMessage::Error(e) => {
                        state.note_refresh_failure();
                        state.set_error(e);
                    }
                }
                // A completed action stages its follow-up refresh via
                // `finish_action`; dispatch it here — the one place that owns the
                // client + sender — instead of recabling a fetch in every arm.
                match state.pending_refetch.take() {
                    Some(Refetch::All) => fetch_all(client.clone(), tx.clone()),
                    Some(Refetch::Mannies) => fetch_mannies(client.clone(), tx.clone()),
                    Some(Refetch::Missions) => fetch_missions(client.clone(), tx.clone()),
                    Some(Refetch::Messages) => {
                        fetch_messages(client.clone(), tx.clone());
                        fetch_sent_messages(client.clone(), tx.clone());
                    }
                    None => {}
                }
            }

            _ = tokio::time::sleep_until(deadline) => {
                if !state.loading {
                    fetch_all(client.clone(), tx.clone());
                    state.loading = true;
                }
            }
        }

        // Reconcile the client's target with the state's active probe after any
        // select branch. A probe switch (keyboard) sets `active_probe_id`; a
        // fleet refresh may *reset* it when the piloted probe was destroyed
        // (v94, `update_fleet`). Either way the state is the source of truth and
        // the client's wired target trails it — retarget, then refetch.
        if client.active_probe_id() != state.active_probe_id {
            client = client.with_active_probe(state.active_probe_id);
            if !state.loading {
                fetch_all(client.clone(), tx.clone());
                state.loading = true;
            }
        }

        if state.quit {
            break;
        }
    }

    Ok(())
}
