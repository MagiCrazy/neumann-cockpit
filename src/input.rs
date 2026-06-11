use crossterm::event::{Event, KeyCode, KeyModifiers};
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::{
    fetch_all, fetch_atomic_printer_craft, fetch_craft, fetch_deploy, fetch_detach,
    fetch_inspect, fetch_jettison, fetch_mine, fetch_move, fetch_recall,
    fetch_recover, fetch_rename_manny, fetch_repair, fetch_salvage, fetch_sector,
};
use crate::app::{
    ApiMessage, AppState, AtomicPrinterCraftInput, CraftInput, DeployInput, DetachInput,
    InspectInput, JettisonInput, MineInput, ObjectAction, ObjectActionInput, Panel, RecallInput,
    RecoverInput, RenameMannyInput, RepairInput, SalvageInput, ScanMode, TravelInput,
    WaypointsInput, DETACH_MODES, RESOURCE_TYPES,
};

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

fn list_nav(code: KeyCode, sel: usize, count: usize) -> Option<usize> {
    match code {
        KeyCode::Up | KeyCode::Char('k') => Some(sel.checked_sub(1).unwrap_or(count.saturating_sub(1))),
        KeyCode::Down | KeyCode::Char('j') => Some((sel + 1) % count.max(1)),
        _ => None,
    }
}

pub fn handle_event(
    event: Event,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    let Event::Key(k) = event else { return };
    // Toasts are transient: any keypress dismisses the current one.
    state.toast = None;
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let in_scan_input = matches!(state.scan_mode, ScanMode::Input(_));
    let in_direction_pick = matches!(state.scan_mode, ScanMode::DirectionPick);
    let in_travel = !matches!(state.travel, TravelInput::Inactive);
    let in_repair = !matches!(state.repair, RepairInput::Inactive);
    let in_jettison = !matches!(state.jettison, JettisonInput::Inactive);
    let in_craft = !matches!(state.craft, CraftInput::Inactive);
    let in_atomic_craft = !matches!(state.atomic_printer_craft, AtomicPrinterCraftInput::Inactive);
    let in_salvage = !matches!(state.salvage, SalvageInput::Inactive);
    let in_recall = !matches!(state.recall, RecallInput::Inactive);
    let in_rename_manny = !matches!(state.rename_manny, RenameMannyInput::Inactive);
    let in_deploy = !matches!(state.deploy, DeployInput::Inactive);
    let in_inspect = !matches!(state.inspect, InspectInput::Inactive);
    let in_recover = !matches!(state.recover, RecoverInput::Inactive);
    let in_detach = !matches!(state.detach, DetachInput::Inactive);

    if ctrl && k.code == KeyCode::Char('c') {
        state.set_quit();
        return;
    }

    if state.help_open {
        if matches!(k.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')) {
            state.help_open = false;
        }
        return;
    }

    if state.inventory_detail_open {
        if matches!(k.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')) {
            state.inventory_detail_open = false;
        }
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

    if in_atomic_craft {
        handle_atomic_printer_craft_event(k.code, state, client, tx);
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

    if in_inspect {
        handle_inspect_event(k.code, state, client, tx);
        return;
    }

    if in_recover {
        handle_recover_event(k.code, state, client, tx);
        return;
    }

    if in_detach {
        handle_detach_event(k.code, state, client, tx);
        return;
    }

    if !matches!(state.object_action, ObjectActionInput::Inactive) {
        handle_object_action_event(k.code, state, client, tx);
        return;
    }

    if !matches!(state.waypoints, WaypointsInput::Inactive) {
        handle_waypoints_event(k.code, state);
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

    let in_object_mode =
        state.focused == Some(Panel::Scanner) && state.scanner_obj_selection.is_some();

    match k.code {
        KeyCode::Char('q') => state.set_quit(),
        KeyCode::Char('b') => state.open_map(),
        KeyCode::Char('?') => state.help_open = true,
        KeyCode::Char('w') => {
            let entries = state.collect_waypoints();
            if entries.is_empty() {
                state.error = Some("no known waypoints — scan sectors first".into());
            } else {
                state.waypoints = WaypointsInput::Browsing { entries, selection: 0 };
            }
        }
        KeyCode::Esc if in_object_mode => state.scanner_obj_selection = None,
        KeyCode::Esc => state.focused = None,
        KeyCode::Char('o') if state.focused == Some(Panel::Scanner) => {
            if state.scanner_obj_selection.is_some() {
                state.scanner_obj_selection = None;
            } else if !state.scanner_objects().is_empty() {
                state.scanner_obj_selection = Some(0);
            } else if state.viewing_probe_sector() {
                state.error = Some("no actionable objects in current sector".into());
            } else {
                state.error = Some("object actions only available in the probe's sector".into());
            }
        }
        KeyCode::Down | KeyCode::Char('j') if in_object_mode => state.scanner_obj_next(),
        KeyCode::Up | KeyCode::Char('k') if in_object_mode => state.scanner_obj_prev(),
        KeyCode::Enter if in_object_mode => {
            let entries = state.scanner_objects();
            if let Some(entry) = state.scanner_obj_selection.and_then(|i| entries.get(i)) {
                let actions = state.actions_for_object(entry);
                if actions.is_empty() {
                    state.error = Some(format!("no actions available for {}", entry.name));
                } else {
                    state.object_action = ObjectActionInput::PickAction {
                        object_id: entry.id.clone(),
                        object_name: entry.name.clone(),
                        actions,
                        selection: 0,
                    };
                }
            }
        }
        KeyCode::Char('t') => {
            state.travel = TravelInput::Typing(String::new());
        }
        KeyCode::Char('g') if state.focused == Some(Panel::Scanner) => {
            if let Some(sector) = state.current_sector() {
                let x = sector.relative_coordinates.x as i32;
                let y = sector.relative_coordinates.y as i32;
                let z = sector.relative_coordinates.z as i32;
                state.travel_go_sector(x, y, z);
            }
        }
        KeyCode::Char('r') if !state.loading => {
            state.clear_error();
            state.loading = true;
            fetch_all(client.clone(), tx.clone());
        }
        KeyCode::Tab => state.focus_next_panel(),
        KeyCode::BackTab => state.focus_prev_panel(),
        KeyCode::Char('p') => state.toggle_focus(Panel::Probe),
        KeyCode::Char('i') => state.toggle_focus(Panel::Inventory),
        KeyCode::Char('m') => state.toggle_focus(Panel::Mannies),
        KeyCode::Char('j') if state.focused == Some(Panel::Inventory) => {
            match state.jettison_for_selected() {
                Ok(input) => state.jettison = input,
                Err(msg) => state.error = Some(msg),
            }
        }
        KeyCode::Down if state.focused == Some(Panel::Inventory) => {
            state.inventory_next();
        }
        KeyCode::Up if state.focused == Some(Panel::Inventory) => {
            state.inventory_prev();
        }
        KeyCode::Enter if state.focused == Some(Panel::Inventory) => {
            if state.selected_inventory_row().is_some() {
                state.inventory_detail_open = true;
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
        KeyCode::Char('a') if state.focused == Some(Panel::Inventory) => {
            if !state.has_atomic_printer() {
                state.error = Some("no atomic printer in inventory".into());
            } else if state.atomic_printer_recipes().is_empty() {
                state.error = Some("recipes not loaded yet — press r to refresh".into());
            } else {
                state.atomic_printer_craft = AtomicPrinterCraftInput::PickRecipe {
                    selection: 0,
                    error: None,
                };
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
        KeyCode::Char('J') if state.focused == Some(Panel::Scanner) => {
            state.scan_detail_scroll = state.scan_detail_scroll.saturating_add(3);
        }
        KeyCode::Char('K') if state.focused == Some(Panel::Scanner) => {
            state.scan_detail_scroll = state.scan_detail_scroll.saturating_sub(3);
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
                        let candidates = state.collect_mineable_candidates();
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
                        if state.manny_craft_recipes().is_empty() {
                            state.error = Some("recipes not loaded yet — press r to refresh".into());
                        } else {
                            state.craft = CraftInput::PickRecipe {
                                manny_id: manny.id.clone(),
                                manny_name: manny.name.clone(),
                                selection: 0,
                                error: None,
                            };
                        }
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
        KeyCode::Char('x') if state.focused == Some(Panel::Mannies) => {
            if let Some(mannies) = &state.mannies {
                if let Some(manny) = mannies.get(state.mannies_selection) {
                    if manny.can_receive_orders {
                        let candidates = state.collect_asteroid_candidates();
                        match candidates.len() {
                            0 => state.error = Some("no asteroids in current sector — scan first".into()),
                            1 => {
                                let (object_id, _) = candidates.into_iter().next().unwrap();
                                fetch_inspect(manny.id.clone(), object_id, client.clone(), tx.clone());
                            }
                            _ => {
                                state.inspect = InspectInput::PickAsteroid {
                                    manny_id: manny.id.clone(),
                                    manny_name: manny.name.clone(),
                                    candidates,
                                    selection: 0,
                                    error: None,
                                };
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Char('v') if state.focused == Some(Panel::Mannies) => {
            if let Some(mannies) = &state.mannies {
                if let Some(manny) = mannies.get(state.mannies_selection) {
                    if manny.can_receive_orders {
                        let candidates = state.collect_detached_containers();
                        match candidates.len() {
                            0 => state.error = Some("no detached containers in current sector — scan first".into()),
                            1 => {
                                let (object_id, _) = candidates.into_iter().next().unwrap();
                                fetch_recover(manny.id.clone(), object_id, client.clone(), tx.clone());
                            }
                            _ => {
                                state.recover = RecoverInput::PickContainer {
                                    manny_id: manny.id.clone(),
                                    manny_name: manny.name.clone(),
                                    candidates,
                                    selection: 0,
                                    error: None,
                                };
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Char('D') if state.focused == Some(Panel::Mannies) => {
            if let Some(mannies) = &state.mannies {
                if let Some(manny) = mannies.get(state.mannies_selection) {
                    if manny.can_receive_orders {
                        let containers = state.collect_detachable_containers();
                        if containers.is_empty() {
                            state.error = Some("no detachable containers in inventory".into());
                        } else if containers.len() == 1 {
                            let (container_id, container_name) = containers.into_iter().next().unwrap();
                            state.detach = DetachInput::PickMode {
                                manny_id: manny.id.clone(),
                                manny_name: manny.name.clone(),
                                container_id,
                                container_name,
                                selection: 0,
                                error: None,
                            };
                        } else {
                            state.detach = DetachInput::PickContainer {
                                manny_id: manny.id.clone(),
                                manny_name: manny.name.clone(),
                                containers,
                                selection: 0,
                            };
                        }
                    }
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
        KeyCode::Char('f') if state.focused == Some(Panel::Scanner) => {
            state.cycle_scan_filter();
        }
        KeyCode::Char('s') => state.toggle_focus(Panel::Scanner),
        _ => {}
    }
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
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let MineInput::PickAsteroid { ref mut selection, .. } = state.mine {
                            *selection = new_sel;
                        }
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
                KeyCode::Char(c @ '1'..='4') if !am => {
                    let idx = (c as u8 - b'1') as usize;
                    if let MineInput::Configure { ref mut resources, ref mut error, .. } = state.mine {
                        resources[idx] = !resources[idx];
                        *error = None;
                    }
                }
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

fn handle_jettison_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.jettison {
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
                KeyCode::Char('m') | KeyCode::Char('M') => state.jettison_fill_max(),
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

fn handle_map_event(code: KeyCode, state: &mut AppState) {
    // Coordinate-input mode ([c]) captures keys first.
    if let Some(buf) = state.map.coord_input.as_mut() {
        match code {
            KeyCode::Esc => state.map.coord_input = None,
            KeyCode::Backspace => {
                buf.pop();
            }
            KeyCode::Enter => {
                let parts: Vec<&str> = buf.split_whitespace().collect();
                if parts.len() == 3 {
                    if let (Ok(x), Ok(y), Ok(z)) = (
                        parts[0].parse::<i32>(),
                        parts[1].parse::<i32>(),
                        parts[2].parse::<i32>(),
                    ) {
                        state.map.center_x = x;
                        state.map.y_layer = y;
                        state.map.center_z = z;
                        state.map.coord_input = None;
                    }
                }
            }
            KeyCode::Char(c) if c == '-' || c == ' ' || c.is_ascii_digit() => buf.push(c),
            _ => {}
        }
        return;
    }

    match code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('b') => state.map.open = false,
        KeyCode::Char('h') | KeyCode::Left  => state.map.center_x -= 2,
        KeyCode::Char('l') | KeyCode::Right => state.map.center_x += 2,
        KeyCode::Char('k') | KeyCode::Up    => state.map.center_z -= 2,
        KeyCode::Char('j') | KeyCode::Down  => state.map.center_z += 2,
        KeyCode::Char('u') => state.map_move_y(1),
        KeyCode::Char('d') => state.map_move_y(-1),
        KeyCode::Char('0') => state.map_recenter_on_probe(),
        KeyCode::Char('c') => state.map.coord_input = Some(String::new()),
        KeyCode::Char('g') => {
            let (cx, y, cz) = (state.map.center_x, state.map.y_layer, state.map.center_z);
            state.map.open = false;
            state.travel_go_sector(cx, y, cz);
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
    let &CraftInput::PickRecipe { selection, .. } = &state.craft else { return };
    let count = state.manny_craft_recipes().len();
    if count == 0 { return; }
    let sel = selection;
    match code {
        KeyCode::Esc => state.craft = CraftInput::Inactive,
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(new_sel) = list_nav(code, sel, count) {
                if let CraftInput::PickRecipe { ref mut selection, .. } = state.craft {
                    *selection = new_sel;
                }
            }
        }
        KeyCode::Enter => {
            let (manny_id, recipe_id) = {
                let CraftInput::PickRecipe { ref manny_id, selection, .. } = state.craft else { return };
                let recipe_id = state.manny_craft_recipes()[selection].id.clone();
                (manny_id.clone(), recipe_id)
            };
            fetch_craft(manny_id, recipe_id, client.clone(), tx.clone());
        }
        _ => {}
    }
}

fn handle_atomic_printer_craft_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    let &AtomicPrinterCraftInput::PickRecipe { selection, .. } = &state.atomic_printer_craft else { return };
    let count = state.atomic_printer_recipes().len();
    if count == 0 { return; }
    let sel = selection;
    match code {
        KeyCode::Esc => state.atomic_printer_craft = AtomicPrinterCraftInput::Inactive,
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(new_sel) = list_nav(code, sel, count) {
                if let AtomicPrinterCraftInput::PickRecipe { ref mut selection, .. } = state.atomic_printer_craft {
                    *selection = new_sel;
                }
            }
        }
        KeyCode::Enter => {
            let recipe_id = state.atomic_printer_recipes()[selection].id.clone();
            fetch_atomic_printer_craft(recipe_id, client.clone(), tx.clone());
        }
        _ => {}
    }
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
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let SalvageInput::PickTarget { ref mut selection, .. } = state.salvage {
                            *selection = new_sel;
                        }
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
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let DeployInput::PickManny { ref mut selection, .. } = state.deploy {
                            *selection = new_sel;
                        }
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
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let DeployInput::PickObject { ref mut selection, .. } = state.deploy {
                            *selection = new_sel;
                        }
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

fn handle_inspect_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    let &InspectInput::PickAsteroid { selection, ref candidates, .. } = &state.inspect else { return };
    let sel = selection;
    let count = candidates.len();
    match code {
        KeyCode::Esc => state.inspect = InspectInput::Inactive,
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(new_sel) = list_nav(code, sel, count) {
                if let InspectInput::PickAsteroid { ref mut selection, .. } = state.inspect {
                    *selection = new_sel;
                }
            }
        }
        KeyCode::Enter => {
            let (manny_id, object_id) = {
                let InspectInput::PickAsteroid { ref manny_id, ref candidates, selection, .. } = state.inspect else { return };
                (manny_id.clone(), candidates[selection].0.clone())
            };
            fetch_inspect(manny_id, object_id, client.clone(), tx.clone());
        }
        _ => {}
    }
}

fn handle_recover_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    let &RecoverInput::PickContainer { selection, ref candidates, .. } = &state.recover else { return };
    let sel = selection;
    let count = candidates.len();
    match code {
        KeyCode::Esc => state.recover = RecoverInput::Inactive,
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(new_sel) = list_nav(code, sel, count) {
                if let RecoverInput::PickContainer { ref mut selection, .. } = state.recover {
                    *selection = new_sel;
                }
            }
        }
        KeyCode::Enter => {
            let (manny_id, object_id) = {
                let RecoverInput::PickContainer { ref manny_id, ref candidates, selection, .. } = state.recover else { return };
                (manny_id.clone(), candidates[selection].0.clone())
            };
            fetch_recover(manny_id, object_id, client.clone(), tx.clone());
        }
        _ => {}
    }
}

fn handle_detach_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.detach {
        DetachInput::PickContainer { selection, containers, .. } => {
            let sel = *selection;
            let count = containers.len();
            match code {
                KeyCode::Esc => state.detach = DetachInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let DetachInput::PickContainer { ref mut selection, .. } = state.detach {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, manny_name, container_id, container_name) = {
                        let DetachInput::PickContainer { ref manny_id, ref manny_name, ref containers, selection } = state.detach else { return };
                        let (id, name) = containers[selection].clone();
                        (manny_id.clone(), manny_name.clone(), id, name)
                    };
                    state.detach = DetachInput::PickMode { manny_id, manny_name, container_id, container_name, selection: 0, error: None };
                }
                _ => {}
            }
        }
        DetachInput::PickMode { selection, .. } => {
            let sel = *selection;
            let count = DETACH_MODES.len();
            match code {
                KeyCode::Esc => state.detach = DetachInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let DetachInput::PickMode { ref mut selection, .. } = state.detach {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, manny_name, container_id, container_name, sel) = {
                        let DetachInput::PickMode { ref manny_id, ref manny_name, ref container_id, ref container_name, selection, .. } = state.detach else { return };
                        (manny_id.clone(), manny_name.clone(), container_id.clone(), container_name.clone(), selection)
                    };
                    let mode = DETACH_MODES[sel].0;
                    if mode == "hidden_on_asteroid" {
                        let asteroids = state.collect_asteroid_candidates();
                        if asteroids.is_empty() {
                            state.set_detach_error("no asteroids in current sector — scan first".into());
                        } else {
                            state.detach = DetachInput::PickAsteroid { manny_id, manny_name, container_id, container_name, asteroids, selection: 0, error: None };
                        }
                    } else {
                        fetch_detach(manny_id, container_id, "drifting".into(), None, client.clone(), tx.clone());
                    }
                }
                _ => {}
            }
        }
        DetachInput::PickAsteroid { selection, asteroids, .. } => {
            let sel = *selection;
            let count = asteroids.len();
            match code {
                KeyCode::Esc => state.detach = DetachInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let DetachInput::PickAsteroid { ref mut selection, .. } = state.detach {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, container_id, object_id) = {
                        let DetachInput::PickAsteroid { ref manny_id, ref container_id, ref asteroids, selection, .. } = state.detach else { return };
                        (manny_id.clone(), container_id.clone(), asteroids[selection].0.clone())
                    };
                    fetch_detach(manny_id, container_id, "hidden_on_asteroid".into(), Some(object_id), client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        DetachInput::Inactive => {}
    }
}

/// Send the chosen object action, reusing the existing wizards/endpoints.
fn dispatch_object_action(
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
    action: ObjectAction,
    object: (String, String),
    manny: (String, String),
) {
    let (object_id, object_name) = object;
    let (manny_id, manny_name) = manny;
    state.object_action = ObjectActionInput::Inactive;
    state.scanner_obj_selection = None;
    match action {
        ObjectAction::Mine => {
            state.mine = MineInput::Configure {
                manny_id,
                manny_name,
                object_id,
                object_name,
                resources: [false, true, false, false],
                amount_buf: "0.30".into(),
                amount_mode: false,
                error: None,
            };
        }
        ObjectAction::Inspect => {
            fetch_inspect(manny_id, object_id, client.clone(), tx.clone());
        }
        ObjectAction::Salvage => {
            state.salvage = SalvageInput::Confirm {
                manny_id,
                manny_name,
                object_id,
                object_name,
                error: None,
            };
        }
        ObjectAction::Recover => {
            fetch_recover(manny_id, object_id, client.clone(), tx.clone());
        }
        ObjectAction::DeployWaypoint => {
            state.deploy = DeployInput::EnterName {
                manny_id,
                object_id,
                object_name,
                name_buf: String::new(),
                error: None,
            };
        }
    }
}

fn handle_waypoints_event(code: KeyCode, state: &mut AppState) {
    let WaypointsInput::Browsing { ref entries, selection } = state.waypoints else { return };
    let count = entries.len();
    match code {
        KeyCode::Esc | KeyCode::Char('w') => state.waypoints = WaypointsInput::Inactive,
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(new_sel) = list_nav(code, selection, count) {
                if let WaypointsInput::Browsing { ref mut selection, .. } = state.waypoints {
                    *selection = new_sel;
                }
            }
        }
        KeyCode::Enter => {
            let (x, y, z) = {
                let WaypointsInput::Browsing { ref entries, selection } = state.waypoints else { return };
                let e = &entries[selection];
                (e.x, e.y, e.z)
            };
            state.waypoints = WaypointsInput::Inactive;
            state.travel_go_sector(x, y, z);
        }
        _ => {}
    }
}

fn handle_object_action_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.object_action {
        ObjectActionInput::PickAction { selection, actions, .. } => {
            let sel = *selection;
            let count = actions.len();
            match code {
                KeyCode::Esc => state.object_action = ObjectActionInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let ObjectActionInput::PickAction { ref mut selection, .. } = state.object_action {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (object_id, object_name, action) = {
                        let ObjectActionInput::PickAction { ref object_id, ref object_name, ref actions, selection } = state.object_action else { return };
                        (object_id.clone(), object_name.clone(), actions[selection])
                    };
                    let mannies = state.collect_idle_onboard_mannies();
                    match mannies.len() {
                        0 => {
                            state.object_action = ObjectActionInput::Inactive;
                            state.error = Some("no idle Manny on board".into());
                        }
                        1 => {
                            let manny = mannies.into_iter().next().unwrap();
                            dispatch_object_action(state, client, tx, action, (object_id, object_name), manny);
                        }
                        _ => {
                            state.object_action = ObjectActionInput::PickManny {
                                object_id,
                                object_name,
                                action,
                                mannies,
                                selection: 0,
                            };
                        }
                    }
                }
                _ => {}
            }
        }
        ObjectActionInput::PickManny { selection, mannies, .. } => {
            let sel = *selection;
            let count = mannies.len();
            match code {
                KeyCode::Esc => state.object_action = ObjectActionInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let ObjectActionInput::PickManny { ref mut selection, .. } = state.object_action {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (object, action, manny) = {
                        let ObjectActionInput::PickManny { ref object_id, ref object_name, action, ref mannies, selection } = state.object_action else { return };
                        ((object_id.clone(), object_name.clone()), action, mannies[selection].clone())
                    };
                    dispatch_object_action(state, client, tx, action, object, manny);
                }
                _ => {}
            }
        }
        ObjectActionInput::Inactive => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;

    // ── list_nav ──────────────────────────────────────────────────────────────

    #[test]
    fn list_nav_down_increments() {
        assert_eq!(list_nav(KeyCode::Down, 0, 3), Some(1));
        assert_eq!(list_nav(KeyCode::Down, 1, 3), Some(2));
        assert_eq!(list_nav(KeyCode::Char('j'), 0, 3), Some(1));
    }

    #[test]
    fn list_nav_down_wraps_at_end() {
        assert_eq!(list_nav(KeyCode::Down, 2, 3), Some(0));
        assert_eq!(list_nav(KeyCode::Char('j'), 2, 3), Some(0));
    }

    #[test]
    fn list_nav_up_decrements() {
        assert_eq!(list_nav(KeyCode::Up, 2, 3), Some(1));
        assert_eq!(list_nav(KeyCode::Char('k'), 2, 3), Some(1));
    }

    #[test]
    fn list_nav_up_wraps_at_zero() {
        assert_eq!(list_nav(KeyCode::Up, 0, 3), Some(2));
        assert_eq!(list_nav(KeyCode::Char('k'), 0, 3), Some(2));
    }

    #[test]
    fn list_nav_returns_none_for_other_keys() {
        assert_eq!(list_nav(KeyCode::Enter, 0, 3), None);
        assert_eq!(list_nav(KeyCode::Esc, 1, 3), None);
        assert_eq!(list_nav(KeyCode::Char('x'), 0, 3), None);
    }

    #[test]
    fn list_nav_empty_list_stays_at_zero() {
        assert_eq!(list_nav(KeyCode::Down, 0, 0), Some(0));
        assert_eq!(list_nav(KeyCode::Up, 0, 0), Some(0));
    }

    // ── neighbors_d1 ─────────────────────────────────────────────────────────

    #[test]
    fn neighbors_d1_count() {
        assert_eq!(neighbors_d1().len(), 12);
    }

    #[test]
    fn neighbors_d1_all_even_sum() {
        for (a, b, c) in neighbors_d1() {
            assert_eq!((a + b + c) % 2, 0, "odd sum at ({a},{b},{c})");
        }
    }

    #[test]
    fn neighbors_d1_all_at_distance_1() {
        for (a, b, c) in neighbors_d1() {
            let dist = a.abs().max(b.abs()).max(c.abs());
            assert_eq!(dist, 1, "distance != 1 at ({a},{b},{c})");
        }
    }

    #[test]
    fn neighbors_d1_no_duplicates() {
        let v = neighbors_d1();
        let mut seen = std::collections::HashSet::new();
        for coord in &v {
            assert!(seen.insert(coord), "duplicate at {coord:?}");
        }
    }

    // ── face_d2 ───────────────────────────────────────────────────────────────

    #[test]
    fn face_d2_all_even_sum() {
        for axis in [b'x', b'y', b'z'] {
            for (a, b, c) in face_d2(axis) {
                assert_eq!((a + b + c) % 2, 0, "odd sum at ({a},{b},{c}) axis={}", axis as char);
            }
        }
    }

    #[test]
    fn face_d2_x_face_coordinate_is_pm2() {
        for (a, _b, _c) in face_d2(b'x') {
            assert!(a == 2 || a == -2, "x coord {a} not ±2");
        }
    }

    #[test]
    fn face_d2_y_face_coordinate_is_pm2() {
        for (_a, b, _c) in face_d2(b'y') {
            assert!(b == 2 || b == -2, "y coord {b} not ±2");
        }
    }

    #[test]
    fn face_d2_z_face_coordinate_is_pm2() {
        for (_a, _b, c) in face_d2(b'z') {
            assert!(c == 2 || c == -2, "z coord {c} not ±2");
        }
    }

    #[test]
    fn face_d2_each_axis_has_same_count() {
        let cx = face_d2(b'x').len();
        let cy = face_d2(b'y').len();
        let cz = face_d2(b'z').len();
        assert_eq!(cx, cy);
        assert_eq!(cy, cz);
        assert!(cx > 0);
    }
}
