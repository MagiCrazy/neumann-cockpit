use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tokio::sync::mpsc;

mod api;
mod app;
mod config;
mod ui;

use api::client::ApiClient;
use api::types::SectorObjectType;
use app::{ApiMessage, AppState, CraftInput, DeployInput, JettisonInput, MineInput, Panel, RecallInput, RenameMannyInput, RepairInput, SalvageInput, ScanMode, TravelInput, RESOURCE_TYPES};

fn neighbors_d1() -> Vec<(i32, i32, i32)> {
    let mut out = Vec::new();
    for a in -1i32..=1 {
        for b in -1i32..=1 {
            for c in -1i32..=1 {
                if a.abs().max(b.abs()).max(c.abs()) == 1 && (a + b + c) % 2 == 0 {
                    out.push((a, b, c));
                }
            }
        }
    }
    out
}

fn face_d2(axis: u8) -> Vec<(i32, i32, i32)> {
    let mut out = Vec::new();
    for face in [-2i32, 2] {
        for u in -2i32..=2 {
            for v in -2i32..=2 {
                let coords = match axis {
                    b'x' => (face, u, v),
                    b'y' => (u, face, v),
                    _    => (u, v, face),
                };
                if (coords.0 + coords.1 + coords.2) % 2 == 0 {
                    out.push(coords);
                }
            }
        }
    }
    out
}

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
                    ApiMessage::MoveStarted(mv) => state.apply_movement(mv),
                    ApiMessage::MoveError(e) => state.set_travel_error(e),
                    ApiMessage::RepairStarted => {
                        state.repair = RepairInput::Inactive;
                        fetch_mannies(client.clone(), tx.clone());
                    }
                    ApiMessage::RepairError(e) => state.set_repair_error(e),
                    ApiMessage::MineStarted => {
                        state.mine = MineInput::Inactive;
                        fetch_mannies(client.clone(), tx.clone());
                    }
                    ApiMessage::MineError(e) => state.set_mine_error(e),
                    ApiMessage::JettisonDone(inv) => {
                        state.update_inventory(inv);
                        state.jettison = JettisonInput::Inactive;
                        fetch_mannies(client.clone(), tx.clone());
                    }
                    ApiMessage::JettisonError(e) => state.set_jettison_error(e),
                    ApiMessage::CraftStarted => {
                        state.craft = CraftInput::Inactive;
                        fetch_mannies(client.clone(), tx.clone());
                    }
                    ApiMessage::CraftError(e) => state.set_craft_error(e),
                    ApiMessage::SalvageStarted => {
                        state.salvage = SalvageInput::Inactive;
                        fetch_mannies(client.clone(), tx.clone());
                    }
                    ApiMessage::SalvageError(e) => state.set_salvage_error(e),
                    ApiMessage::RecallStarted => {
                        state.recall = RecallInput::Inactive;
                        fetch_mannies(client.clone(), tx.clone());
                    }
                    ApiMessage::RecallError(e) => state.set_recall_error(e),
                    ApiMessage::DeployStarted => {
                        state.deploy = DeployInput::Inactive;
                        fetch_all(client.clone(), tx.clone());
                    }
                    ApiMessage::DeployError(e) => state.set_deploy_error(e),
                    ApiMessage::RenameMannyDone(manny) => {
                        if let Some(ref mut mannies) = state.mannies {
                            if let Some(m) = mannies.iter_mut().find(|m| m.id == manny.id) {
                                *m = manny;
                            }
                        }
                        state.rename_manny = RenameMannyInput::Inactive;
                    }
                    ApiMessage::RenameMannyError(e) => state.set_rename_manny_error(e),
                    ApiMessage::VersionFetched(v) => state.api_version = Some(v),
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

        if state_requests_quit(&state) {
            break;
        }
    }

    Ok(())
}

fn handle_event(
    event: Event,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    let Event::Key(k) = event else { return };
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let in_scan_input = matches!(state.scan_mode, ScanMode::Input(_));
    let in_direction_pick = matches!(state.scan_mode, ScanMode::DirectionPick);
    let in_travel = !matches!(state.travel, TravelInput::Inactive);
    let in_repair = !matches!(state.repair, RepairInput::Inactive);
    let in_jettison = !matches!(state.jettison, JettisonInput::Inactive);
    let in_craft = !matches!(state.craft, CraftInput::Inactive);
    let in_salvage = !matches!(state.salvage, SalvageInput::Inactive);
    let in_recall = !matches!(state.recall, RecallInput::Inactive);
    let in_rename_manny = !matches!(state.rename_manny, RenameMannyInput::Inactive);
    let in_deploy = !matches!(state.deploy, DeployInput::Inactive);

    if ctrl && k.code == KeyCode::Char('c') {
        state.set_quit();
        return;
    }

    if state.map.open {
        handle_map_event(k.code, state);
        return;
    }

    if in_jettison {
        handle_jettison_event(k.code, state, client, tx);
        return;
    }

    if in_craft {
        handle_craft_event(k.code, state, client, tx);
        return;
    }

    if in_salvage {
        handle_salvage_event(k.code, state, client, tx);
        return;
    }

    if in_recall {
        handle_recall_event(k.code, state, client, tx);
        return;
    }

    if in_rename_manny {
        handle_rename_manny_event(k.code, state, client, tx);
        return;
    }

    if in_deploy {
        handle_deploy_event(k.code, state, client, tx);
        return;
    }

    if in_travel {
        handle_travel_event(k.code, state, client, tx);
        return;
    }

    if in_repair {
        handle_repair_event(k.code, state, client, tx);
        return;
    }

    let in_mine = !matches!(state.mine, MineInput::Inactive);
    if in_mine {
        handle_mine_event(k.code, state, client, tx);
        return;
    }

    if in_direction_pick {
        match k.code {
            KeyCode::Esc => state.scan_mode = ScanMode::Current,
            KeyCode::Char(axis @ ('x' | 'y' | 'z')) => {
                if let Some(pos) = state.probe_sector_coords() {
                    let offsets = face_d2(axis as u8);
                    let n = offsets.len();
                    state.start_batch(n);
                    state.scan_mode = ScanMode::Current;
                    for (dx, dy, dz) in offsets {
                        fetch_sector(Some((pos.0 + dx, pos.1 + dy, pos.2 + dz)), client.clone(), tx.clone());
                    }
                }
            }
            _ => {}
        }
        return;
    }

    if in_scan_input {
        match k.code {
            KeyCode::Esc => state.scan_mode = ScanMode::Current,
            KeyCode::Backspace => state.scan_backspace(),
            KeyCode::Enter => {
                if let Some(coords) = state.parse_scan_coords() {
                    state.scan_loading = true;
                    state.scan_error = None;
                    fetch_sector(Some(coords), client.clone(), tx.clone());
                }
            }
            KeyCode::Char(c) => state.scan_type_char(c),
            _ => {}
        }
        return;
    }

    match k.code {
        KeyCode::Char('q') => state.set_quit(),
        KeyCode::Char('b') => state.open_map(),
        KeyCode::Esc => state.focused = None,
        KeyCode::Char('t') => {
            state.travel = TravelInput::Typing(String::new());
        }
        KeyCode::Char('g') if state.focused == Some(Panel::Scanner) => {
            if let Some(sector) = state.current_sector() {
                let x = sector.relative_coordinates.x as i32;
                let y = sector.relative_coordinates.y as i32;
                let z = sector.relative_coordinates.z as i32;
                let dist = Some(sector.distance);
                state.travel_go_sector(x, y, z, dist);
            }
        }
        KeyCode::Char('r') if !state.loading => {
            state.clear_error();
            state.loading = true;
            fetch_all(client.clone(), tx.clone());
        }
        KeyCode::Char('p') => state.toggle_focus(Panel::Probe),
        KeyCode::Char('i') => state.toggle_focus(Panel::Inventory),
        KeyCode::Char('m') => state.toggle_focus(Panel::Mannies),
        KeyCode::Char('j') if state.focused == Some(Panel::Inventory) => {
            let items = state.build_jettison_items();
            if !items.is_empty() {
                state.jettison = JettisonInput::PickItem { items, selection: 0 };
            }
        }
        KeyCode::Char('d') if state.focused == Some(Panel::Inventory) => {
            if state.inventory_waypoint_bookmark_id().is_none() {
                state.error = Some("no waypoint bookmark in inventory — craft one first".into());
            } else {
                let mannies = state.collect_idle_onboard_mannies();
                if mannies.is_empty() {
                    state.error = Some("no idle Manny on board".into());
                } else {
                    let candidates = state.collect_deploy_candidates();
                    if candidates.is_empty() {
                        state.error = Some("no targets in current sector — scan first".into());
                    } else if mannies.len() == 1 {
                        let (manny_id, _) = mannies.into_iter().next().unwrap();
                        if candidates.len() == 1 {
                            let (object_id, object_name) = candidates.into_iter().next().unwrap();
                            state.deploy = DeployInput::EnterName {
                                manny_id,
                                object_id,
                                object_name,
                                name_buf: String::new(),
                                error: None,
                            };
                        } else {
                            state.deploy = DeployInput::PickObject { manny_id, candidates, selection: 0 };
                        }
                    } else {
                        state.deploy = DeployInput::PickManny { mannies, selection: 0 };
                    }
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') if state.focused == Some(Panel::Mannies) => {
            state.manny_next();
        }
        KeyCode::Up | KeyCode::Char('k') if state.focused == Some(Panel::Mannies) => {
            state.manny_prev();
        }
        KeyCode::Down | KeyCode::Char('j') if state.focused == Some(Panel::Scanner) => {
            state.scan_hist_next();
        }
        KeyCode::Up | KeyCode::Char('k') if state.focused == Some(Panel::Scanner) => {
            state.scan_hist_prev();
        }
        KeyCode::Enter if state.focused == Some(Panel::Mannies) => {
            if let Some(mannies) = &state.mannies {
                if let Some(manny) = mannies.get(state.mannies_selection) {
                    if manny.can_receive_orders {
                        state.repair = RepairInput::Typing {
                            manny_id: manny.id.clone(),
                            manny_name: manny.name.clone(),
                            buf: String::new(),
                            error: None,
                        };
                    }
                }
            }
        }
        KeyCode::Char('e') if state.focused == Some(Panel::Mannies) => {
            if let Some(mannies) = &state.mannies {
                if let Some(manny) = mannies.get(state.mannies_selection) {
                    if manny.can_receive_orders {
                        let manny_id = manny.id.clone();
                        let manny_name = manny.name.clone();
                        let candidates = collect_mineable_candidates(state);
                        match candidates.len() {
                            0 => state.error = Some("no mineable objects in current sector — scan first".into()),
                            1 => {
                                let (object_id, object_name) = candidates.into_iter().next().unwrap();
                                state.mine = MineInput::Configure {
                                    manny_id, manny_name, object_id, object_name,
                                    resources: [false, true, false, false],
                                    amount_buf: "0.30".into(),
                                    amount_mode: false,
                                    error: None,
                                };
                            }
                            _ => {
                                state.mine = MineInput::PickAsteroid {
                                    manny_id, manny_name, candidates, selection: 0,
                                };
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Char('c') if state.focused == Some(Panel::Mannies) => {
            if let Some(mannies) = &state.mannies {
                if let Some(manny) = mannies.get(state.mannies_selection) {
                    if manny.can_receive_orders {
                        state.craft = CraftInput::Confirm {
                            manny_id: manny.id.clone(),
                            manny_name: manny.name.clone(),
                            error: None,
                        };
                    }
                }
            }
        }
        KeyCode::Char('s') if state.focused == Some(Panel::Mannies) => {
            if let Some(mannies) = &state.mannies {
                if let Some(manny) = mannies.get(state.mannies_selection) {
                    if manny.can_receive_orders {
                        let manny_id = manny.id.clone();
                        let manny_name = manny.name.clone();
                        let candidates = state.collect_salvage_candidates();
                        match candidates.len() {
                            0 => state.error = Some("no salvageable objects in current sector — scan first".into()),
                            1 => {
                                let (object_id, object_name) = candidates.into_iter().next().unwrap();
                                state.salvage = SalvageInput::Confirm {
                                    manny_id, manny_name, object_id, object_name, error: None,
                                };
                            }
                            _ => {
                                state.salvage = SalvageInput::PickTarget {
                                    manny_id, manny_name, candidates, selection: 0,
                                };
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Char('R') if state.focused == Some(Panel::Mannies) => {
            if let Some(mannies) = &state.mannies {
                if let Some(manny) = mannies.get(state.mannies_selection) {
                    if !manny.can_receive_orders && manny.current_task.is_some() {
                        state.recall = RecallInput::Confirm {
                            manny_id: manny.id.clone(),
                            manny_name: manny.name.clone(),
                            error: None,
                        };
                    }
                }
            }
        }
        KeyCode::Char('n') if state.focused == Some(Panel::Mannies) => {
            if let Some(mannies) = &state.mannies {
                if let Some(manny) = mannies.get(state.mannies_selection) {
                    state.rename_manny = RenameMannyInput::Typing {
                        manny_id: manny.id.clone(),
                        manny_name: manny.name.clone(),
                        buf: manny.name.clone(),
                        error: None,
                    };
                }
            }
        }
        KeyCode::Enter if state.focused == Some(Panel::Scanner) && !state.scan_loading => {
            state.scan_loading = true;
            state.scan_error = None;
            fetch_sector(None, client.clone(), tx.clone());
        }
        KeyCode::Char('c') if state.focused == Some(Panel::Scanner) => {
            state.scan_mode = ScanMode::Input(String::new());
        }
        KeyCode::Char('n') if state.focused == Some(Panel::Scanner) && state.scan_batch.is_none() => {
            if let Some(pos) = state.probe_sector_coords() {
                let offsets = neighbors_d1();
                let n = offsets.len();
                state.start_batch(n);
                for (dx, dy, dz) in offsets {
                    fetch_sector(Some((pos.0 + dx, pos.1 + dy, pos.2 + dz)), client.clone(), tx.clone());
                }
            }
        }
        KeyCode::Char('d') if state.focused == Some(Panel::Scanner) && state.scan_batch.is_none() => {
            state.scan_mode = ScanMode::DirectionPick;
        }
        KeyCode::Char('s') => state.toggle_focus(Panel::Scanner),
        _ => {}
    }
}

fn state_requests_quit(state: &AppState) -> bool {
    state.quit
}

fn fetch_api_version(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        if let Ok(v) = client.get_api_version().await {
            let _ = tx.send(ApiMessage::VersionFetched(v)).await;
        }
    });
}

fn fetch_all(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    let c1 = client.clone();
    let tx1 = tx.clone();
    tokio::spawn(async move {
        let msg = match c1.get_probe().await {
            Ok(p) => ApiMessage::ProbeUpdated(p),
            Err(e) => ApiMessage::Error(e.to_string()),
        };
        let _ = tx1.send(msg).await;
    });

    let c2 = client.clone();
    let tx2 = tx.clone();
    tokio::spawn(async move {
        if let Ok(m) = c2.get_mannies().await {
            let _ = tx2.send(ApiMessage::ManniesUpdated(m)).await;
        }
    });

    let c3 = client;
    let tx3 = tx;
    tokio::spawn(async move {
        if let Ok(s) = c3.get_probe_sector().await {
            let _ = tx3.send(ApiMessage::SectorUpdated(s)).await;
        }
    });
}

fn handle_travel_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.travel {
        TravelInput::Typing(_) => match code {
            KeyCode::Esc => state.travel = TravelInput::Inactive,
            KeyCode::Backspace => state.travel_backspace(),
            KeyCode::Enter => state.travel_submit(),
            KeyCode::Char(c) => state.travel_type_char(c),
            _ => {}
        },
        TravelInput::Confirming { x, y, z, error, .. } => {
            let (x, y, z, has_error) = (*x, *y, *z, error.is_some());
            match code {
                KeyCode::Esc => state.travel = TravelInput::Inactive,
                KeyCode::Enter if !has_error => {
                    fetch_move(x, y, z, client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        TravelInput::Inactive => {}
    }
}

fn handle_repair_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match code {
        KeyCode::Esc => state.repair = RepairInput::Inactive,
        KeyCode::Backspace => state.repair_backspace(),
        KeyCode::Char('m') | KeyCode::Char('M') => state.repair_fill_max(),
        KeyCode::Char(c) => state.repair_type_char(c),
        KeyCode::Enter => {
            let (manny_id, pct) = {
                let RepairInput::Typing { ref manny_id, ref buf, .. } = state.repair else { return };
                let Ok(pct) = buf.parse::<f64>() else { return };
                if pct <= 0.0 { return }
                (manny_id.clone(), pct)
            };
            fetch_repair(manny_id, pct, client.clone(), tx.clone());
        }
        _ => {}
    }
}

fn handle_mine_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.mine {
        MineInput::PickAsteroid { selection, candidates, .. } => {
            let sel = *selection;
            let count = candidates.len();
            match code {
                KeyCode::Esc => state.mine = MineInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') => {
                    if let MineInput::PickAsteroid { ref mut selection, .. } = state.mine {
                        *selection = sel.checked_sub(1).unwrap_or(count - 1);
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let MineInput::PickAsteroid { ref mut selection, .. } = state.mine {
                        *selection = (sel + 1) % count;
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, manny_name, object_id, object_name) = {
                        let MineInput::PickAsteroid { ref manny_id, ref manny_name, ref candidates, selection } = state.mine else { return };
                        let (id, name) = candidates[selection].clone();
                        (manny_id.clone(), manny_name.clone(), id, name)
                    };
                    state.mine = MineInput::Configure {
                        manny_id, manny_name, object_id, object_name,
                        resources: [false, true, false, false],
                        amount_buf: "0.30".into(),
                        amount_mode: false,
                        error: None,
                    };
                }
                _ => {}
            }
        }
        MineInput::Configure { amount_mode, .. } => {
            let am = *amount_mode;
            match code {
                KeyCode::Esc => state.mine = MineInput::Inactive,
                KeyCode::Tab => {
                    if let MineInput::Configure { ref mut amount_mode, ref mut error, .. } = state.mine {
                        *amount_mode = !am;
                        *error = None;
                    }
                }
                // Toggle resources (only in resource mode)
                KeyCode::Char(c @ '1'..='4') if !am => {
                    let idx = (c as u8 - b'1') as usize;
                    if let MineInput::Configure { ref mut resources, ref mut error, .. } = state.mine {
                        resources[idx] = !resources[idx];
                        *error = None;
                    }
                }
                // Amount editing (only in amount mode)
                KeyCode::Char('m') | KeyCode::Char('M') if am => {
                    let max = state.mine_max_amount();
                    if let MineInput::Configure { ref mut amount_buf, ref mut error, .. } = state.mine {
                        *amount_buf = format!("{:.4}", max);
                        *error = None;
                    }
                }
                KeyCode::Char(c) if am && (c.is_ascii_digit() || c == '.') => {
                    if let MineInput::Configure { ref mut amount_buf, ref mut error, .. } = state.mine {
                        if !(c == '.' && amount_buf.contains('.')) {
                            amount_buf.push(c);
                            *error = None;
                        }
                    }
                }
                KeyCode::Backspace if am => {
                    if let MineInput::Configure { ref mut amount_buf, .. } = state.mine {
                        amount_buf.pop();
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, object_id, selected_resources, amount) = {
                        let MineInput::Configure { ref manny_id, ref object_id, resources, ref amount_buf, .. } = state.mine else { return };
                        let selected: Vec<String> = RESOURCE_TYPES.iter().enumerate()
                            .filter(|(i, _)| resources[*i])
                            .map(|(_, &t)| t.to_string())
                            .collect();
                        if selected.is_empty() { return }
                        let Ok(amount) = amount_buf.parse::<f64>() else { return };
                        if amount <= 0.0 { return }
                        (manny_id.clone(), object_id.clone(), selected, amount)
                    };
                    fetch_mine(manny_id, object_id, selected_resources, amount, client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        MineInput::Inactive => {}
    }
}

fn fetch_mine(
    manny_id: String,
    object_id: String,
    resources: Vec<String>,
    target_amount: f64,
    client: ApiClient,
    tx: mpsc::Sender<ApiMessage>,
) {
    tokio::spawn(async move {
        let msg = match client.mine_manny(&manny_id, &object_id, resources, target_amount).await {
            Ok(_) => ApiMessage::MineStarted,
            Err(e) => ApiMessage::MineError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

fn collect_mineable_candidates(state: &AppState) -> Vec<(String, String)> {
    // Find the scan for the probe's current sector
    let current_pos = state.probe.as_ref()
        .and_then(|p| p.sector.as_ref())
        .and_then(|s| s.relative.as_ref())
        .map(|r| (r.x as i64, r.y as i64, r.z as i64));

    let sector = if let Some(pos) = current_pos {
        state.scan_history.iter().find(|s| {
            (s.relative_coordinates.x as i64, s.relative_coordinates.y as i64, s.relative_coordinates.z as i64) == pos
        })
    } else {
        state.scan_history.first()
    };

    sector
        .and_then(|s| s.objects.as_ref())
        .map(|objects| {
            objects.iter()
                .flat_map(|o| o.minable_targets.iter().flatten())
                .filter(|t| matches!(t.object_type, SectorObjectType::Asteroid))
                .map(|t| {
                    let name = t.name.clone().unwrap_or_else(|| "unnamed".into());
                    (t.id.clone(), name)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn handle_jettison_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.jettison {
        JettisonInput::PickItem { selection, items, .. } => {
            let sel = *selection;
            let count = items.len();
            match code {
                KeyCode::Esc => state.jettison = JettisonInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') => {
                    if let JettisonInput::PickItem { ref mut selection, .. } = state.jettison {
                        *selection = sel.checked_sub(1).unwrap_or(count - 1);
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let JettisonInput::PickItem { ref mut selection, .. } = state.jettison {
                        *selection = (sel + 1) % count;
                    }
                }
                KeyCode::Enter => {
                    let (item_id, is_manny) = {
                        let JettisonInput::PickItem { ref items, selection, .. } = state.jettison else { return };
                        let (id, _, manny) = &items[selection];
                        (id.clone(), *manny)
                    };
                    if is_manny {
                        let manny_name = state.probe.as_ref()
                            .and_then(|p| p.inventory.items.iter().find(|i| i.id == item_id))
                            .map(|i| i.name.clone())
                            .unwrap_or_else(|| item_id.clone());
                        state.jettison = JettisonInput::ConfirmManny { item_id, manny_name, error: None };
                    } else {
                        let (item_name, max_amount) = state.probe.as_ref()
                            .and_then(|p| p.inventory.resource_stocks.iter().find(|s| s.id == item_id))
                            .map(|s| (s.name.clone(), s.amount))
                            .unwrap_or_else(|| (item_id.clone(), 0.0));
                        state.jettison = JettisonInput::EnterAmount {
                            item_id,
                            item_name,
                            max_amount,
                            buf: String::new(),
                            error: None,
                        };
                    }
                }
                _ => {}
            }
        }
        JettisonInput::ConfirmManny { .. } => {
            match code {
                KeyCode::Esc => state.jettison = JettisonInput::Inactive,
                KeyCode::Enter => {
                    let item_id = {
                        let JettisonInput::ConfirmManny { ref item_id, .. } = state.jettison else { return };
                        item_id.clone()
                    };
                    fetch_jettison(item_id, None, client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        JettisonInput::EnterAmount { .. } => {
            match code {
                KeyCode::Esc => state.jettison = JettisonInput::Inactive,
                KeyCode::Backspace => state.jettison_backspace(),
                KeyCode::Char(c) => state.jettison_type_char(c),
                KeyCode::Enter => {
                    let (item_id, amount) = {
                        let JettisonInput::EnterAmount { ref item_id, ref buf, .. } = state.jettison else { return };
                        let amount = if buf.is_empty() {
                            None
                        } else {
                            let Ok(v) = buf.parse::<f64>() else { return };
                            if v <= 0.0 { return }
                            Some(v)
                        };
                        (item_id.clone(), amount)
                    };
                    fetch_jettison(item_id, amount, client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        JettisonInput::Inactive => {}
    }
}

fn fetch_jettison(item_id: String, amount: Option<f64>, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.jettison_inventory(&item_id, amount).await {
            Ok(inv) => ApiMessage::JettisonDone(inv),
            Err(e) => ApiMessage::JettisonError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

fn fetch_repair(manny_id: String, integrity_percent: f64, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.repair_manny(&manny_id, integrity_percent).await {
            Ok(_) => ApiMessage::RepairStarted,
            Err(e) => ApiMessage::RepairError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

fn fetch_mannies(client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        if let Ok(m) = client.get_mannies().await {
            let _ = tx.send(ApiMessage::ManniesUpdated(m)).await;
        }
    });
}

fn fetch_move(x: i32, y: i32, z: i32, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.move_probe(x, y, z).await {
            Ok(mv) => ApiMessage::MoveStarted(mv),
            Err(e) => ApiMessage::MoveError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

fn handle_map_event(code: KeyCode, state: &mut AppState) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('b') => state.map.open = false,
        KeyCode::Char('h') | KeyCode::Left  => state.map.center_x -= 2,
        KeyCode::Char('l') | KeyCode::Right => state.map.center_x += 2,
        KeyCode::Char('k') | KeyCode::Up    => state.map.center_z -= 2,
        KeyCode::Char('j') | KeyCode::Down  => state.map.center_z += 2,
        KeyCode::Char('u') => state.map_move_y(1),
        KeyCode::Char('d') => state.map_move_y(-1),
        KeyCode::Char('g') => {
            let (cx, y, cz) = (state.map.center_x, state.map.y_layer, state.map.center_z);
            state.map.open = false;
            state.travel_go_sector(cx, y, cz, None);
        }
        _ => {}
    }
}

fn handle_craft_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match code {
        KeyCode::Esc => state.craft = CraftInput::Inactive,
        KeyCode::Enter => {
            let manny_id = {
                let CraftInput::Confirm { ref manny_id, .. } = state.craft else { return };
                manny_id.clone()
            };
            fetch_craft(manny_id, "waypoint_bookmark", client.clone(), tx.clone());
        }
        _ => {}
    }
}

fn fetch_craft(manny_id: String, recipe: &'static str, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.craft_manny(&manny_id, recipe).await {
            Ok(_) => ApiMessage::CraftStarted,
            Err(e) => ApiMessage::CraftError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

fn handle_salvage_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.salvage {
        SalvageInput::PickTarget { selection, candidates, .. } => {
            let sel = *selection;
            let count = candidates.len();
            match code {
                KeyCode::Esc => state.salvage = SalvageInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') => {
                    if let SalvageInput::PickTarget { ref mut selection, .. } = state.salvage {
                        *selection = sel.checked_sub(1).unwrap_or(count - 1);
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let SalvageInput::PickTarget { ref mut selection, .. } = state.salvage {
                        *selection = (sel + 1) % count;
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, manny_name, object_id, object_name) = {
                        let SalvageInput::PickTarget { ref manny_id, ref manny_name, ref candidates, selection } = state.salvage else { return };
                        let (id, name) = candidates[selection].clone();
                        (manny_id.clone(), manny_name.clone(), id, name)
                    };
                    state.salvage = SalvageInput::Confirm {
                        manny_id, manny_name, object_id, object_name, error: None,
                    };
                }
                _ => {}
            }
        }
        SalvageInput::Confirm { .. } => {
            match code {
                KeyCode::Esc => state.salvage = SalvageInput::Inactive,
                KeyCode::Enter => {
                    let (manny_id, object_id) = {
                        let SalvageInput::Confirm { ref manny_id, ref object_id, .. } = state.salvage else { return };
                        (manny_id.clone(), object_id.clone())
                    };
                    fetch_salvage(manny_id, object_id, client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        SalvageInput::Inactive => {}
    }
}

fn handle_recall_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match code {
        KeyCode::Esc => state.recall = RecallInput::Inactive,
        KeyCode::Enter => {
            let manny_id = {
                let RecallInput::Confirm { ref manny_id, .. } = state.recall else { return };
                manny_id.clone()
            };
            fetch_recall(manny_id, client.clone(), tx.clone());
        }
        _ => {}
    }
}

fn fetch_salvage(manny_id: String, object_id: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.salvage_manny(&manny_id, &object_id).await {
            Ok(_) => ApiMessage::SalvageStarted,
            Err(e) => ApiMessage::SalvageError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

fn fetch_recall(manny_id: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.recall_manny(&manny_id).await {
            Ok(_) => ApiMessage::RecallStarted,
            Err(e) => ApiMessage::RecallError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

fn fetch_sector(coords: Option<(i32, i32, i32)>, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let result = match coords {
            None => client.get_probe_sector().await,
            Some((x, y, z)) => client.get_sector(x, y, z).await,
        };
        let msg = match result {
            Ok(s) => ApiMessage::SectorUpdated(s),
            Err(e) => ApiMessage::ScanError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

fn handle_deploy_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.deploy {
        DeployInput::PickManny { selection, mannies } => {
            let sel = *selection;
            let count = mannies.len();
            match code {
                KeyCode::Esc => state.deploy = DeployInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') => {
                    if let DeployInput::PickManny { ref mut selection, .. } = state.deploy {
                        *selection = sel.checked_sub(1).unwrap_or(count - 1);
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let DeployInput::PickManny { ref mut selection, .. } = state.deploy {
                        *selection = (sel + 1) % count;
                    }
                }
                KeyCode::Enter => {
                    let manny_id = {
                        let DeployInput::PickManny { ref mannies, selection } = state.deploy else { return };
                        mannies[selection].0.clone()
                    };
                    let candidates = state.collect_deploy_candidates();
                    if candidates.is_empty() {
                        state.deploy = DeployInput::Inactive;
                        state.error = Some("no targets in current sector".into());
                    } else if candidates.len() == 1 {
                        let (object_id, object_name) = candidates.into_iter().next().unwrap();
                        state.deploy = DeployInput::EnterName { manny_id, object_id, object_name, name_buf: String::new(), error: None };
                    } else {
                        state.deploy = DeployInput::PickObject { manny_id, candidates, selection: 0 };
                    }
                }
                _ => {}
            }
        }
        DeployInput::PickObject { selection, candidates, .. } => {
            let sel = *selection;
            let count = candidates.len();
            match code {
                KeyCode::Esc => state.deploy = DeployInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') => {
                    if let DeployInput::PickObject { ref mut selection, .. } = state.deploy {
                        *selection = sel.checked_sub(1).unwrap_or(count - 1);
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let DeployInput::PickObject { ref mut selection, .. } = state.deploy {
                        *selection = (sel + 1) % count;
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, object_id, object_name) = {
                        let DeployInput::PickObject { ref manny_id, ref candidates, selection } = state.deploy else { return };
                        let (id, name) = candidates[selection].clone();
                        (manny_id.clone(), id, name)
                    };
                    state.deploy = DeployInput::EnterName {
                        manny_id,
                        object_id,
                        object_name,
                        name_buf: String::new(),
                        error: None,
                    };
                }
                _ => {}
            }
        }
        DeployInput::EnterName { .. } => {
            match code {
                KeyCode::Esc => state.deploy = DeployInput::Inactive,
                KeyCode::Backspace => state.deploy_backspace(),
                KeyCode::Char(c) => state.deploy_type_char(c),
                KeyCode::Enter => {
                    let (manny_id, object_id, name) = {
                        let DeployInput::EnterName { ref manny_id, ref object_id, ref name_buf, .. } = state.deploy else { return };
                        if name_buf.is_empty() { return }
                        (manny_id.clone(), object_id.clone(), name_buf.clone())
                    };
                    fetch_deploy(manny_id, object_id, name, client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        DeployInput::Inactive => {}
    }
}

fn fetch_deploy(manny_id: String, object_id: String, name: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.install_bookmark_manny(&manny_id, &object_id, &name).await {
            Ok(_) => ApiMessage::DeployStarted,
            Err(e) => ApiMessage::DeployError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}

fn handle_rename_manny_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match code {
        KeyCode::Esc => state.rename_manny = RenameMannyInput::Inactive,
        KeyCode::Backspace => state.rename_manny_backspace(),
        KeyCode::Char(c) => state.rename_manny_type_char(c),
        KeyCode::Enter => {
            let (manny_id, name) = {
                let RenameMannyInput::Typing { ref manny_id, ref buf, .. } = state.rename_manny else { return };
                if buf.is_empty() { return }
                (manny_id.clone(), buf.clone())
            };
            fetch_rename_manny(manny_id, name, client.clone(), tx.clone());
        }
        _ => {}
    }
}

fn fetch_rename_manny(manny_id: String, name: String, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.rename_manny(&manny_id, &name).await {
            Ok(manny) => ApiMessage::RenameMannyDone(manny),
            Err(e) => ApiMessage::RenameMannyError(e.to_string()),
        };
        let _ = tx.send(msg).await;
    });
}
