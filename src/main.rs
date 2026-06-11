use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, EventStream},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tokio::sync::mpsc;

use neumann_cockpit::api::client::ApiClient;
use neumann_cockpit::api::tasks::{fetch_all, fetch_api_version, fetch_crafting_recipes, fetch_mannies};
use neumann_cockpit::app::{
    ApiMessage, AppState, AtomicPrinterCraftInput, CraftInput, DeployInput, DetachInput,
    InspectInput, JettisonInput, MineInput, RecallInput, RecoverInput, RenameMannyInput,
    RepairInput, SalvageInput,
};
use neumann_cockpit::config;
use neumann_cockpit::input::handle_event;
use neumann_cockpit::ui;

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = config::Config::load()?;
    let client = ApiClient::new(cfg.base_url, cfg.api_key)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&client, &mut terminal).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run(
    client: &ApiClient,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    let (tx, mut rx) = mpsc::channel::<ApiMessage>(32);
    let mut state = AppState::default();
    let scan_history_path = config::history_path();
    state.load_scan_history(&scan_history_path);
    let mut events = EventStream::new();

    // Initial data fetch
    fetch_all(client.clone(), tx.clone());
    fetch_api_version(client.clone(), tx.clone());
    fetch_crafting_recipes(client.clone(), tx.clone());
    state.loading = true;

    loop {
        terminal.draw(|f| ui::cockpit::render(f, &state))?;

        let deadline = state.next_refresh_instant();

        tokio::select! {
            Some(event) = events.next() => {
                handle_event(event?, &mut state, client, &tx);
            }

            Some(msg) = rx.recv() => {
                state.loading = false;
                match msg {
                    ApiMessage::ProbeUpdated(probe) => state.update_probe(probe),
                    ApiMessage::ManniesUpdated(mannies) => state.update_mannies(mannies),
                    ApiMessage::SectorUpdated(sector) => {
                        state.update_sector(sector);
                        state.batch_tick();
                        let history = state.scan_history.clone();
                        let path = scan_history_path.clone();
                        tokio::spawn(async move {
                            if let Ok(json) = serde_json::to_string(&history) {
                                let _ = tokio::fs::write(path, json).await;
                            }
                        });
                    }
                    ApiMessage::ScanError(e) => {
                        if state.scan_batch.is_some() {
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
                        state.set_toast("mining order sent");
                        fetch_mannies(client.clone(), tx.clone());
                    }
                    ApiMessage::MineError(e) => state.set_mine_error(e),
                    ApiMessage::JettisonDone(inv) => {
                        state.update_inventory(inv);
                        state.jettison = JettisonInput::Inactive;
                        state.set_toast("jettisoned");
                        fetch_mannies(client.clone(), tx.clone());
                    }
                    ApiMessage::JettisonError(e) => state.set_jettison_error(e),
                    ApiMessage::CraftStarted => {
                        state.craft = CraftInput::Inactive;
                        state.set_toast("craft order sent");
                        fetch_mannies(client.clone(), tx.clone());
                    }
                    ApiMessage::CraftError(e) => state.set_craft_error(e),
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
                    ApiMessage::DeployStarted => {
                        state.deploy = DeployInput::Inactive;
                        state.set_toast("waypoint deploy order sent");
                        fetch_all(client.clone(), tx.clone());
                    }
                    ApiMessage::DeployError(e) => state.set_deploy_error(e),
                    ApiMessage::AtomicPrinterCraftStarted => {
                        state.atomic_printer_craft = AtomicPrinterCraftInput::Inactive;
                        state.set_toast("atomic printer craft started");
                        fetch_all(client.clone(), tx.clone());
                    }
                    ApiMessage::AtomicPrinterCraftError(e) => state.set_atomic_printer_craft_error(e),
                    ApiMessage::RecipesFetched(recipes) => state.recipes = recipes,
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
                    ApiMessage::Error(e) => state.set_error(e),
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
