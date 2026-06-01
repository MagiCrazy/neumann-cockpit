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
use app::{ApiMessage, AppState, Panel, ScanMode, TravelInput};

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

    if ctrl && k.code == KeyCode::Char('c') {
        state.set_quit();
        return;
    }

    if in_travel {
        handle_travel_event(k.code, state, client, tx);
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
        KeyCode::Char('s') => state.toggle_focus(Panel::Scanner),
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
        _ => {}
    }
}

fn state_requests_quit(state: &AppState) -> bool {
    state.quit
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
        match c2.get_mannies().await {
            Ok(m) => {
                let _ = tx2.send(ApiMessage::ManniesUpdated(m)).await;
            }
            Err(_) => {} // mannies failure is non-fatal
        }
    });

    let c3 = client;
    let tx3 = tx;
    tokio::spawn(async move {
        match c3.get_probe_sector().await {
            Ok(s) => { let _ = tx3.send(ApiMessage::SectorUpdated(s)).await; }
            Err(_) => {}
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

fn fetch_move(x: i32, y: i32, z: i32, client: ApiClient, tx: mpsc::Sender<ApiMessage>) {
    tokio::spawn(async move {
        let msg = match client.move_probe(x, y, z).await {
            Ok(mv) => ApiMessage::MoveStarted(mv),
            Err(e) => ApiMessage::MoveError(e.to_string()),
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
