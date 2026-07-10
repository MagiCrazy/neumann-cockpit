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
    fetch_all, fetch_api_version, fetch_crafting_recipes, fetch_mannies, fetch_messages,
    fetch_missions, fetch_sent_messages,
};
use neumann_cockpit::app::{
    ApiMessage, AppState, AssembleProbeInput, ColorMode, ContainerRulesInput, FabricationInput,
    ImproveInput,
    DeployInput,
    DetachInput, DropCargoInput, DropStorageContainerInput, InspectInput, JettisonInput,
    MessagesInput, MindSnapshotInput, MineInput, MissionsInput, RecallInput, RecoverInput,
    RefuelInput,
    RemoteMineInput,
    RenameContainerInput, RenameMannyInput, RenameProbeInput, RepairInput, SalvageInput,
    ScutNetworkInput, ScutRelayInput, StorageMoveInput,
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

async fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ready: preflight::Ready,
) -> Result<()> {
    let preflight::Ready { config, client, conn, scan_history, journal, api_version, link_ok } = ready;
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
        api_version,
        ..Default::default()
    };
    // The remote link was already probed in the preflight; surface a down link
    // straight away so the pilot sees why data is missing (F5 retries).
    if !link_ok {
        state.set_error("remote link down — press F5 to retry".into());
    }
    // The persistence writer takes the connection opened during preflight; on a
    // DB error there we run without persistence (history already empty).
    let persist_tx = conn.map(store::spawn_writer);
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

        terminal.draw(|f| ui::render(f, &state))?;

        let deadline = state.next_refresh_instant();

        tokio::select! {
            Some(event) = events.next() => {
                handle_event(event?, &mut state, &client, &tx);
                // A probe switch only sets `state.active_probe_id`; reconcile the
                // client so all later calls target it, then pull fresh data for
                // the new probe. Source of truth is the state; the client's
                // wired target trails it.
                if client.active_probe_id() != state.active_probe_id {
                    client = client.with_active_probe(state.active_probe_id);
                    if !state.loading {
                        fetch_all(client.clone(), tx.clone());
                        state.loading = true;
                    }
                }
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
                        state.rename_probe = RenameProbeInput::Inactive;
                        state.set_toast(format!("probe renamed to {name}"));
                        // Refresh so the Probe pane identity picks up the new name.
                        fetch_all(client.clone(), tx.clone());
                    }
                    ApiMessage::RenameProbeError(e) => state.set_rename_probe_error(e),
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
                            let _ = tx.send(store::PersistMsg::UpsertObservation(obs.clone()));
                        }
                    }
                    ApiMessage::ScanError(e) => {
                        if matches!(state.remote_mine, RemoteMineInput::Loading { .. }) {
                            // The remote-mine sector fetch failed — don't leave the
                            // wizard hung on "fetching…". Abort and surface why.
                            state.remote_mine = RemoteMineInput::Inactive;
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
                    ApiMessage::MoveError(e) => state.set_travel_error(e),
                    ApiMessage::RepairStarted => {
                        state.repair = RepairInput::Inactive;
                        state.set_toast("repair order sent");
                        fetch_mannies(client.clone(), tx.clone());
                    }
                    ApiMessage::RepairError(e) => state.set_repair_error(e),
                    ApiMessage::MineStarted => {
                        state.mine = MineInput::Inactive;
                        state.remote_mine = RemoteMineInput::Inactive;
                        state.set_toast("mining order sent");
                        fetch_mannies(client.clone(), tx.clone());
                    }
                    ApiMessage::MineError(e) => {
                        state.set_mine_error(e.clone());
                        state.set_remote_mine_error(e);
                    }
                    ApiMessage::JettisonDone(inv) => {
                        state.update_inventory(inv);
                        state.jettison = JettisonInput::Inactive;
                        state.set_toast("jettisoned");
                        // Jettison always adds an object to the sector (ejected manny,
                        // drifting item, or deployed SCUT relay) — refresh everything.
                        fetch_all(client.clone(), tx.clone());
                    }
                    ApiMessage::JettisonError(e) => state.set_jettison_error(e),
                    ApiMessage::CraftStarted => {
                        state.fabrication = FabricationInput::Inactive;
                        state.set_toast("craft order sent");
                        fetch_mannies(client.clone(), tx.clone());
                    }
                    ApiMessage::CraftError(e) => state.set_fabrication_error(e),
                    ApiMessage::SalvageStarted => {
                        state.salvage = SalvageInput::Inactive;
                        state.set_toast("salvage order sent");
                        fetch_mannies(client.clone(), tx.clone());
                    }
                    ApiMessage::SalvageError(e) => state.set_salvage_error(e),
                    ApiMessage::RecallStarted => {
                        state.recall = RecallInput::Inactive;
                        state.set_toast("recall order sent");
                        fetch_mannies(client.clone(), tx.clone());
                    }
                    ApiMessage::RecallError(e) => state.set_recall_error(e),
                    ApiMessage::DeuteriumRefuelStarted => {
                        state.refuel = RefuelInput::Inactive;
                        state.set_toast("refuel order sent");
                        fetch_all(client.clone(), tx.clone());
                    }
                    ApiMessage::DeuteriumRefuelError(e) => state.set_refuel_error(e),
                    ApiMessage::MindSnapshotReassigned(probe) => {
                        state.mind_snapshot = MindSnapshotInput::Inactive;
                        state.update_probe(probe);
                        state.set_toast("mind snapshot reassigned");
                        fetch_all(client.clone(), tx.clone());
                    }
                    ApiMessage::MindSnapshotReassignError(e) => state.set_mind_snapshot_error(e),
                    ApiMessage::MissionsFetched(missions) => state.missions = missions,
                    ApiMessage::MissionAbandoned(_) => {
                        state.missions_input = MissionsInput::Browsing { selection: 0 };
                        state.set_toast("mission abandoned");
                        fetch_missions(client.clone(), tx.clone());
                    }
                    ApiMessage::MissionAbandonError(e) => state.set_mission_abandon_error(e),
                    ApiMessage::ScutRelayTurnedOn => {
                        state.scut_relay = ScutRelayInput::Inactive;
                        state.set_toast("relay turn-on order sent");
                        fetch_all(client.clone(), tx.clone());
                    }
                    ApiMessage::ScutRelayTurnOnError(e) => state.set_scut_relay_error(e),
                    ApiMessage::ScutNetworkFetched(network) => {
                        if matches!(state.scut_network, ScutNetworkInput::Viewing { .. }) {
                            state.scut_network_view = Some(network);
                        }
                    }
                    ApiMessage::MessagesFetched(m) => state.messages = m,
                    ApiMessage::SentMessagesFetched(m) => state.sent_messages = m,
                    ApiMessage::MessageSent(_) => {
                        state.messages_input = MessagesInput::Browsing { sent_tab: false, selection: 0 };
                        state.set_toast("message sent");
                        fetch_messages(client.clone(), tx.clone());
                        fetch_sent_messages(client.clone(), tx.clone());
                    }
                    ApiMessage::MessageSendError(e) => state.set_message_send_error(e),
                    ApiMessage::MessageMarkedRead(m) => {
                        if let Some(slot) = state.messages.iter_mut().find(|x| x.id == m.id) {
                            *slot = m;
                        }
                    }
                    ApiMessage::ScutNetworkError(e) => {
                        if matches!(state.scut_network, ScutNetworkInput::Viewing { .. }) {
                            state.scut_network = ScutNetworkInput::Viewing { error: Some(e) };
                        }
                    }
                    ApiMessage::DeployStarted => {
                        state.deploy = DeployInput::Inactive;
                        state.set_toast("waypoint deploy order sent");
                        fetch_all(client.clone(), tx.clone());
                    }
                    ApiMessage::DeployError(e) => state.set_deploy_error(e),
                    ApiMessage::AtomicPrinterCraftStarted => {
                        state.fabrication = FabricationInput::Inactive;
                        state.set_toast("atomic printer craft started");
                        fetch_all(client.clone(), tx.clone());
                    }
                    ApiMessage::AtomicPrinterCraftError(e) => state.set_fabrication_error(e),
                    ApiMessage::RecipesFetched(recipes) => state.recipes = recipes,
                    ApiMessage::ProbeImprovementsFetched(improvements) => {
                        state.probe_improvements = improvements;
                    }
                    ApiMessage::ImproveProbeStarted => {
                        state.improve = ImproveInput::Inactive;
                        state.set_toast("probe improvement started");
                        fetch_all(client.clone(), tx.clone());
                    }
                    ApiMessage::ImproveProbeError(e) => state.set_improve_error(e),
                    ApiMessage::InspectStarted => {
                        state.inspect = InspectInput::Inactive;
                        state.set_toast("inspect order sent");
                        fetch_mannies(client.clone(), tx.clone());
                    }
                    ApiMessage::InspectError(e) => state.set_inspect_error(e),
                    ApiMessage::RecoverStarted => {
                        state.recover = RecoverInput::Inactive;
                        state.set_toast("recover order sent");
                        fetch_all(client.clone(), tx.clone());
                    }
                    ApiMessage::RecoverError(e) => state.set_recover_error(e),
                    ApiMessage::DetachStarted => {
                        state.detach = DetachInput::Inactive;
                        state.set_toast("detach order sent");
                        fetch_all(client.clone(), tx.clone());
                    }
                    ApiMessage::DetachError(e) => state.set_detach_error(e),
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
                        state.rename_container = RenameContainerInput::Inactive;
                        state.set_toast("container renamed");
                    }
                    ApiMessage::RenameContainerError(e) => state.set_rename_container_error(e),
                    ApiMessage::UpdateContainerRulesDone(c, inv) => {
                        state.apply_container_update(c, inv);
                        state.container_rules = ContainerRulesInput::Inactive;
                        state.set_toast("routing rules updated");
                    }
                    ApiMessage::UpdateContainerRulesError(e) => state.set_container_rules_error(e),
                    ApiMessage::StorageMoveDone(manny, inv) => {
                        if let Some(ref mut mannies) = state.mannies {
                            if let Some(m) = mannies.iter_mut().find(|m| m.id == manny.id) {
                                *m = manny;
                            }
                        }
                        state.update_inventory(inv);
                        state.storage_move = StorageMoveInput::Inactive;
                        state.set_toast("storage move order sent");
                    }
                    ApiMessage::StorageMoveError(e) => state.set_storage_move_error(e),
                    ApiMessage::AssembleProbeStarted(manny, inv) => {
                        if let Some(ref mut mannies) = state.mannies {
                            if let Some(m) = mannies.iter_mut().find(|m| m.id == manny.id) {
                                *m = manny;
                            }
                        }
                        state.update_inventory(inv);
                        state.assemble_probe = AssembleProbeInput::Inactive;
                        state.set_toast("drone assembly started (~3h)");
                        // The new drone appears in the roster once assembled.
                        fetch_all(client.clone(), tx.clone());
                    }
                    ApiMessage::AssembleProbeError(e) => state.set_assemble_probe_error(e),
                    ApiMessage::DropMannyCargoStarted(manny) => {
                        if let Some(ref mut mannies) = state.mannies {
                            if let Some(m) = mannies.iter_mut().find(|m| m.id == manny.id) {
                                *m = manny;
                            }
                        }
                        state.drop_cargo = DropCargoInput::Inactive;
                        state.set_toast("cargo dropped");
                        // Recoverable objects may reappear in the sector.
                        fetch_all(client.clone(), tx.clone());
                    }
                    ApiMessage::DropMannyCargoError(e) => state.set_drop_cargo_error(e),
                    ApiMessage::DropStorageContainerStarted(manny) => {
                        if let Some(ref mut mannies) = state.mannies {
                            if let Some(m) = mannies.iter_mut().find(|m| m.id == manny.id) {
                                *m = manny;
                            }
                        }
                        state.drop_container = DropStorageContainerInput::Inactive;
                        state.set_toast("drop container order sent");
                        // Container + drop kit leave the inventory.
                        fetch_all(client.clone(), tx.clone());
                    }
                    ApiMessage::DropStorageContainerError(e) => state.set_drop_container_error(e),
                    ApiMessage::RenameMannyDone(manny) => {
                        if let Some(ref mut mannies) = state.mannies {
                            if let Some(m) = mannies.iter_mut().find(|m| m.id == manny.id) {
                                *m = manny;
                            }
                        }
                        state.rename_manny = RenameMannyInput::Inactive;
                        state.set_toast("manny renamed");
                    }
                    ApiMessage::RenameMannyError(e) => state.set_rename_manny_error(e),
                    ApiMessage::VersionFetched(v) => state.api_version = Some(v),
                    ApiMessage::VisitedSectorsFetched(v) => state.visited_sectors = v,
                    ApiMessage::ActionError(e) => state.set_error(e),
                    ApiMessage::Error(e) => {
                        state.note_refresh_failure();
                        state.set_error(e);
                    }
                }
            }

            _ = tokio::time::sleep_until(deadline) => {
                if !state.loading {
                    fetch_all(client.clone(), tx.clone());
                    state.loading = true;
                }
            }
        }

        if state.quit {
            break;
        }
    }

    Ok(())
}
