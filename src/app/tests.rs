use super::*;

fn make_sector(x: f64, y: f64, z: f64) -> SectorObservation {
    serde_json::from_str(&format!(r#"{{
        "relativeCoordinates": {{"x": {x}, "y": {y}, "z": {z}}},
        "distance": 1,
        "knowledgeLevel": "detailed",
        "confidence": 1.0,
        "objects": null,
        "probes": null,
        "possibleObjects": null,
        "estimatedObjects": null,
        "navigationalRisk": null,
        "message": null,
        "sensorMode": null,
        "dataFreshness": null,
        "scan": {{
            "currentSectorResidenceSeconds": 60,
            "requiredResidenceSeconds": 60,
            "scanQuality": 1.0
        }}
    }}"#)).unwrap()
}

fn make_probe(free_capacity: f64, sector_x: f64, sector_y: f64, sector_z: f64) -> Probe {
    serde_json::from_str(&format!(r#"{{
        "id": 1,
        "name": "test",
        "status": "idle",
        "fuel": {{"deuterium": 100.0}},
        "sensorMode": "normal",
        "sector": {{"relative": {{"x": {sector_x}, "y": {sector_y}, "z": {sector_z}}}}},
        "movement": null,
        "systems": null,
        "inventory": {{
            "capacity": 10.0,
            "usedCapacity": {used},
            "freeCapacity": {free_capacity},
            "items": [],
            "resourceStocks": [],
            "externalTanks": [],
            "containers": []
        }}
    }}"#, used = 10.0 - free_capacity)).unwrap()
}

// ── parse_scan_coords ─────────────────────────────────────────────────────

#[test]
fn parse_scan_coords_valid() {
    let mut state = AppState::default();
    state.scan_mode = ScanMode::Input("2 0 -2".into());
    assert_eq!(state.parse_scan_coords(), Some((2, 0, -2)));
}

#[test]
fn parse_scan_coords_negative_values() {
    let mut state = AppState::default();
    state.scan_mode = ScanMode::Input("-4 2 -6".into());
    assert_eq!(state.parse_scan_coords(), Some((-4, 2, -6)));
}

#[test]
fn parse_scan_coords_not_in_input_mode() {
    let state = AppState::default();
    assert_eq!(state.parse_scan_coords(), None);
}

#[test]
fn parse_scan_coords_only_two_parts() {
    let mut state = AppState::default();
    state.scan_mode = ScanMode::Input("1 2".into());
    assert_eq!(state.parse_scan_coords(), None);
}

#[test]
fn parse_scan_coords_non_numeric() {
    let mut state = AppState::default();
    state.scan_mode = ScanMode::Input("a b c".into());
    assert_eq!(state.parse_scan_coords(), None);
}

// ── probe_sector_coords ───────────────────────────────────────────────────

#[test]
fn probe_sector_coords_no_probe() {
    let state = AppState::default();
    assert_eq!(state.probe_sector_coords(), None);
}

#[test]
fn probe_sector_coords_exact() {
    let mut state = AppState::default();
    state.probe = Some(make_probe(7.0, 2.0, 0.0, -2.0));
    assert_eq!(state.probe_sector_coords(), Some((2, 0, -2)));
}

#[test]
fn probe_sector_coords_rounds_floats() {
    let mut state = AppState::default();
    state.probe = Some(make_probe(7.0, 2.7, 0.2, -2.8));
    assert_eq!(state.probe_sector_coords(), Some((3, 0, -3)));
}

// ── travel_submit ─────────────────────────────────────────────────────────

#[test]
fn travel_submit_even_sum_no_error() {
    let mut state = AppState::default();
    state.travel = TravelInput::Typing("2 0 -2".into());
    state.travel_submit();
    if let TravelInput::Confirming { x, y, z, error, .. } = &state.travel {
        assert_eq!((*x, *y, *z), (2, 0, -2));
        assert!(error.is_none(), "expected no error, got {error:?}");
    } else {
        panic!("expected Confirming variant");
    }
}

#[test]
fn travel_submit_odd_sum_sets_error() {
    let mut state = AppState::default();
    state.travel = TravelInput::Typing("1 0 0".into());
    state.travel_submit();
    if let TravelInput::Confirming { error, .. } = &state.travel {
        assert!(error.is_some(), "expected parity error");
        assert!(error.as_ref().unwrap().contains("even"));
    } else {
        panic!("expected Confirming variant");
    }
}

#[test]
fn travel_submit_not_typing_is_noop() {
    let mut state = AppState::default();
    state.travel_submit();
    assert!(matches!(state.travel, TravelInput::Inactive));
}

#[test]
fn travel_submit_invalid_input_is_noop() {
    let mut state = AppState::default();
    state.travel = TravelInput::Typing("abc".into());
    state.travel_submit();
    assert!(matches!(state.travel, TravelInput::Typing(_)));
}

#[test]
fn travel_relative_input_resolves_from_probe_position() {
    let mut state = AppState::default();
    state.probe = Some(make_probe(7.0, 2.0, 0.0, -2.0));
    state.travel = TravelInput::Typing("+2 0 -2".into());
    assert_eq!(state.resolve_travel_target(), Some((4, 0, -4)));
    state.travel_submit();
    if let TravelInput::Confirming { x, y, z, error, .. } = &state.travel {
        assert_eq!((*x, *y, *z), (4, 0, -4));
        assert!(error.is_none());
    } else {
        panic!("expected Confirming variant");
    }
}

#[test]
fn travel_relative_without_probe_position_is_noop() {
    let mut state = AppState::default();
    state.travel = TravelInput::Typing("+2 0 0".into());
    assert_eq!(state.resolve_travel_target(), None);
    state.travel_submit();
    assert!(matches!(state.travel, TravelInput::Typing(_)));
}

#[test]
fn travel_plus_only_accepted_as_first_char() {
    let mut state = AppState::default();
    state.travel = TravelInput::Typing(String::new());
    state.travel_type_char('+');
    state.travel_type_char('2');
    state.travel_type_char('+'); // rejected mid-buffer
    if let TravelInput::Typing(ref buf) = state.travel {
        assert_eq!(buf, "+2");
    } else {
        panic!("expected Typing variant");
    }
}

// ── scan_hist_next / scan_hist_prev ───────────────────────────────────────

#[test]
fn scan_hist_nav_empty_is_noop() {
    let mut state = AppState::default();
    state.scan_hist_next();
    assert_eq!(state.scan_history_idx, 0);
    state.scan_hist_prev();
    assert_eq!(state.scan_history_idx, 0);
}

#[test]
fn scan_hist_next_advances_index() {
    let mut state = AppState::default();
    state.scan_history = vec![make_sector(0., 0., 0.), make_sector(2., 0., 0.), make_sector(4., 0., 0.)];
    state.scan_hist_next();
    assert_eq!(state.scan_history_idx, 1);
    state.scan_hist_next();
    assert_eq!(state.scan_history_idx, 2);
}

#[test]
fn scan_hist_next_clamps_at_end() {
    let mut state = AppState::default();
    state.scan_history = vec![make_sector(0., 0., 0.), make_sector(2., 0., 0.)];
    state.scan_history_idx = 1;
    state.scan_hist_next();
    assert_eq!(state.scan_history_idx, 1);
}

#[test]
fn scan_hist_prev_decrements_index() {
    let mut state = AppState::default();
    state.scan_history = vec![make_sector(0., 0., 0.), make_sector(2., 0., 0.)];
    state.scan_history_idx = 1;
    state.scan_hist_prev();
    assert_eq!(state.scan_history_idx, 0);
}

#[test]
fn scan_hist_prev_clamps_at_zero() {
    let mut state = AppState::default();
    state.scan_history = vec![make_sector(0., 0., 0.)];
    state.scan_hist_prev();
    assert_eq!(state.scan_history_idx, 0);
}

// ── mine_max_amount ───────────────────────────────────────────────────────

#[test]
fn mine_max_amount_no_probe_returns_default() {
    let state = AppState::default();
    assert_eq!(state.mine_max_amount(), 0.30);
}

#[test]
fn mine_max_amount_returns_free_capacity() {
    let mut state = AppState::default();
    state.probe = Some(make_probe(0.5, 0., 0., 0.));
    assert_eq!(state.mine_max_amount(), 0.5);
}

#[test]
fn mine_max_amount_clamps_to_zero() {
    let mut state = AppState::default();
    state.probe = Some(make_probe(0.0, 0., 0., 0.));
    assert_eq!(state.mine_max_amount(), 0.0);
}

// ── inventory_rows / jettison_for_selected ────────────────────────────────

fn probe_with_inventory(items_json: &str, stocks_json: &str) -> Probe {
    serde_json::from_str(&format!(r#"{{
        "id": 1, "name": "test", "status": "idle",
        "fuel": {{"deuterium": 100.0}}, "sensorMode": "normal",
        "sector": null, "movement": null, "systems": null,
        "inventory": {{
            "capacity": 10.0, "usedCapacity": 1.0, "freeCapacity": 9.0,
            "items": {items_json},
            "resourceStocks": {stocks_json},
            "externalTanks": [], "containers": []
        }}
    }}"#)).unwrap()
}

const STOCK_METALS: &str = r#"[{
    "id": "stock-metals", "type": "metals", "name": "Metals",
    "amount": 0.5, "containerSpace": 0.5, "containers": []
}]"#;

fn manny_item(task: Option<&str>) -> String {
    let task_json = task.map(|t| format!("\"{t}\"")).unwrap_or("null".into());
    format!(r#"{{
        "id": "manny-1", "type": "manny", "name": "Manny-1",
        "containerSpace": 1.0,
        "currentTask": {task_json},
        "taskProgressPercent": 0.0,
        "location": {{"type": "probe", "sector": null}},
        "cargo": null,
        "container": null
    }}"#)
}

#[test]
fn inventory_rows_no_probe_is_empty() {
    let state = AppState::default();
    assert!(state.inventory_rows().is_empty());
    assert_eq!(state.selected_inventory_row(), None);
}

#[test]
fn inventory_rows_order_stocks_active_passive() {
    let mut state = AppState::default();
    let items = format!(r#"[
        {{"id": "wb-1", "type": "waypoint_bookmark", "name": "Bookmark",
          "containerSpace": 0.1, "currentTask": null, "taskProgressPercent": 0.0,
          "location": null, "cargo": null, "container": null}},
        {{"id": "wb-2", "type": "waypoint_bookmark", "name": "Bookmark",
          "containerSpace": 0.1, "currentTask": null, "taskProgressPercent": 0.0,
          "location": null, "cargo": null, "container": null}},
        {}
    ]"#, manny_item(None));
    state.probe = Some(probe_with_inventory(&items, STOCK_METALS));
    let rows = state.inventory_rows();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0], InventoryRow::Stock { id: "stock-metals".into() });
    assert_eq!(rows[1], InventoryRow::ActiveItem { id: "manny-1".into() });
    assert_eq!(rows[2], InventoryRow::PassiveGroup { item_type: "waypoint_bookmark".into() });
}

#[test]
fn inventory_nav_wraps() {
    let mut state = AppState::default();
    state.probe = Some(probe_with_inventory(&format!("[{}]", manny_item(None)), STOCK_METALS));
    assert_eq!(state.inventory_rows().len(), 2);
    state.inventory_next();
    assert_eq!(state.inventory_selection, 1);
    state.inventory_next();
    assert_eq!(state.inventory_selection, 0);
    state.inventory_prev();
    assert_eq!(state.inventory_selection, 1);
}

#[test]
fn jettison_for_selected_stock_enters_amount() {
    let mut state = AppState::default();
    state.probe = Some(probe_with_inventory("[]", STOCK_METALS));
    match state.jettison_for_selected() {
        Ok(JettisonInput::EnterAmount { item_id, item_name, max_amount, .. }) => {
            assert_eq!(item_id, "stock-metals");
            assert_eq!(item_name, "Metals");
            assert_eq!(max_amount, 0.5);
        }
        other => panic!("expected EnterAmount, got {:?}", std::mem::discriminant(&other.unwrap_or_default())),
    }
}

#[test]
fn jettison_for_selected_idle_manny_confirms() {
    let mut state = AppState::default();
    state.probe = Some(probe_with_inventory(&format!("[{}]", manny_item(None)), "[]"));
    match state.jettison_for_selected() {
        Ok(JettisonInput::ConfirmManny { item_id, manny_name, .. }) => {
            assert_eq!(item_id, "manny-1");
            assert_eq!(manny_name, "Manny-1");
        }
        _ => panic!("expected ConfirmManny"),
    }
}

#[test]
fn jettison_for_selected_busy_manny_errors() {
    let mut state = AppState::default();
    state.probe = Some(probe_with_inventory(&format!("[{}]", manny_item(Some("mining"))), "[]"));
    let err = state.jettison_for_selected().err().expect("busy manny should error");
    assert!(err.contains("busy"), "unexpected error: {err}");
}

#[test]
fn jettison_for_selected_passive_group_errors() {
    let mut state = AppState::default();
    let items = r#"[{"id": "wb-1", "type": "waypoint_bookmark", "name": "Bookmark",
        "containerSpace": 0.1, "currentTask": null, "taskProgressPercent": 0.0,
        "location": null, "cargo": null, "container": null}]"#;
    state.probe = Some(probe_with_inventory(items, "[]"));
    assert!(state.jettison_for_selected().is_err());
}

#[test]
fn jettison_fill_max_sets_buffer() {
    let mut state = AppState::default();
    state.jettison = JettisonInput::EnterAmount {
        item_id: "stock-metals".into(),
        item_name: "Metals".into(),
        max_amount: 0.5,
        buf: "0.1".into(),
        error: Some("previous".into()),
    };
    state.jettison_fill_max();
    if let JettisonInput::EnterAmount { ref buf, ref error, .. } = state.jettison {
        assert_eq!(buf, "0.5000");
        assert!(error.is_none());
    } else {
        panic!("expected EnterAmount");
    }
}

#[test]
fn visited_sector_parses_spec_example() {
    let v: Vec<VisitedSector> = serde_json::from_str(r#"[
        {"relativeCoordinates": {"x": 0, "y": 0, "z": 0},
         "firstVisitedAt": "2026-06-01T12:00:00+00:00",
         "lastVisitedAt": "2026-06-01T12:00:00+00:00",
         "visitCount": 1},
        {"relativeCoordinates": {"x": 1, "y": 1, "z": 0},
         "firstVisitedAt": "2026-06-01T13:15:00+00:00",
         "lastVisitedAt": "2026-06-01T15:45:00+00:00",
         "visitCount": 2}
    ]"#).unwrap();
    assert_eq!(v.len(), 2);
    assert_eq!(v[1].visit_count, 2);
    assert_eq!(v[1].relative_coordinates.x as i32, 1);
}

#[test]
fn toast_expires_after_five_seconds() {
    let mut state = AppState::default();
    state.set_toast("mining order sent");
    assert_eq!(state.active_toast(), Some("mining order sent"));
    // age the toast artificially
    if let Some((_, ref mut t)) = state.toast {
        *t -= chrono::Duration::seconds(6);
    }
    assert_eq!(state.active_toast(), None);
}

#[test]
fn update_probe_clamps_inventory_selection() {
    let mut state = AppState::default();
    state.inventory_selection = 5;
    state.update_probe(probe_with_inventory("[]", STOCK_METALS));
    assert_eq!(state.inventory_selection, 0);
}

// ── helpers ───────────────────────────────────────────────────────────────

fn make_manny(id: &str, location_type: &str, can_receive_orders: bool, task: Option<&str>) -> Manny {
    let task_json = match task {
        Some(t) => format!("\"{}\"", t),
        None => "null".into(),
    };
    serde_json::from_str(&format!(r#"{{
        "id": "{id}",
        "name": "{id}",
        "location": {{"type": "{location_type}", "sector": null}},
        "currentTask": {task_json},
        "taskProgressPercent": 0.0,
        "cargo": {{"capacity": 0.3, "deuterium": 0.0, "metals": 0.0, "ice": 0.0, "organicCompounds": 0.0}},
        "canReceiveOrders": {can_receive_orders},
        "taskEstimatedEndTime": null
    }}"#)).unwrap()
}

fn make_sector_with_objects(x: f64, y: f64, z: f64, objects_json: &str) -> SectorObservation {
    serde_json::from_str(&format!(r#"{{
        "relativeCoordinates": {{"x": {x}, "y": {y}, "z": {z}}},
        "distance": 1,
        "knowledgeLevel": "detailed",
        "confidence": 1.0,
        "objects": {objects_json},
        "probes": null,
        "possibleObjects": null,
        "estimatedObjects": null,
        "navigationalRisk": null,
        "message": null,
        "sensorMode": null,
        "dataFreshness": null,
        "scan": {{"currentSectorResidenceSeconds": 60, "requiredResidenceSeconds": 60, "scanQuality": 1.0}}
    }}"#)).unwrap()
}

fn probe_at(x: f64, y: f64, z: f64) -> Probe {
    make_probe(7.0, x, y, z)
}

// ── toggle_focus ──────────────────────────────────────────────────────────

#[test]
fn toggle_focus_sets_panel() {
    let mut state = AppState::default();
    state.toggle_focus(Panel::Probe);
    assert_eq!(state.focused, Some(Panel::Probe));
}

#[test]
fn toggle_focus_same_panel_clears() {
    let mut state = AppState::default();
    state.toggle_focus(Panel::Scanner);
    state.toggle_focus(Panel::Scanner);
    assert_eq!(state.focused, None);
}

#[test]
fn toggle_focus_different_panel_switches() {
    let mut state = AppState::default();
    state.toggle_focus(Panel::Probe);
    state.toggle_focus(Panel::Mannies);
    assert_eq!(state.focused, Some(Panel::Mannies));
}

#[test]
fn focus_next_panel_cycles_in_visual_order() {
    let mut state = AppState::default();
    state.focus_next_panel();
    assert_eq!(state.focused, Some(Panel::Probe));
    state.focus_next_panel();
    assert_eq!(state.focused, Some(Panel::Inventory));
    state.focus_next_panel();
    assert_eq!(state.focused, Some(Panel::Scanner));
    state.focus_next_panel();
    assert_eq!(state.focused, Some(Panel::Mannies));
    state.focus_next_panel();
    assert_eq!(state.focused, Some(Panel::Probe));
}

#[test]
fn focus_prev_panel_cycles_backwards() {
    let mut state = AppState::default();
    state.focus_prev_panel();
    assert_eq!(state.focused, Some(Panel::Mannies));
    state.focus_prev_panel();
    assert_eq!(state.focused, Some(Panel::Scanner));
}

// ── manny_next / manny_prev ───────────────────────────────────────────────

#[test]
fn manny_next_advances() {
    let mut state = AppState::default();
    state.mannies = Some(vec![
        make_manny("m1", "probe", true, None),
        make_manny("m2", "probe", true, None),
    ]);
    state.manny_next();
    assert_eq!(state.mannies_selection, 1);
}

#[test]
fn manny_next_wraps() {
    let mut state = AppState::default();
    state.mannies = Some(vec![
        make_manny("m1", "probe", true, None),
        make_manny("m2", "probe", true, None),
    ]);
    state.mannies_selection = 1;
    state.manny_next();
    assert_eq!(state.mannies_selection, 0);
}

#[test]
fn manny_prev_decrements() {
    let mut state = AppState::default();
    state.mannies = Some(vec![
        make_manny("m1", "probe", true, None),
        make_manny("m2", "probe", true, None),
    ]);
    state.mannies_selection = 1;
    state.manny_prev();
    assert_eq!(state.mannies_selection, 0);
}

#[test]
fn manny_prev_wraps() {
    let mut state = AppState::default();
    state.mannies = Some(vec![
        make_manny("m1", "probe", true, None),
        make_manny("m2", "probe", true, None),
    ]);
    state.manny_prev();
    assert_eq!(state.mannies_selection, 1);
}

#[test]
fn manny_nav_no_mannies_is_noop() {
    let mut state = AppState::default();
    state.manny_next();
    state.manny_prev();
    assert_eq!(state.mannies_selection, 0);
}

// ── repair_max_percent / repair_metals_stock ──────────────────────────────

#[test]
fn repair_max_percent_no_probe() {
    assert_eq!(AppState::default().repair_max_percent(), 0.0);
}

#[test]
fn repair_max_percent_full_integrity() {
    let mut state = AppState::default();
    state.probe = Some(serde_json::from_str(r#"{
        "id": 1, "name": "t", "status": "idle",
        "fuel": {"deuterium": null}, "sensorMode": "normal",
        "sector": null, "movement": null,
        "systems": {"integrityPercent": 100.0, "damagePercent": 0.0,
                    "energyStored": null, "internalClockRate": null, "currentTask": null},
        "inventory": {"capacity": 1.0, "usedCapacity": 0.0, "freeCapacity": 1.0,
                      "items": [], "resourceStocks": [], "externalTanks": [], "containers": []}
    }"#).unwrap());
    assert_eq!(state.repair_max_percent(), 0.0);
}

#[test]
fn repair_max_percent_damaged() {
    let mut state = AppState::default();
    state.probe = Some(serde_json::from_str(r#"{
        "id": 1, "name": "t", "status": "idle",
        "fuel": {"deuterium": null}, "sensorMode": "normal",
        "sector": null, "movement": null,
        "systems": {"integrityPercent": 60.0, "damagePercent": 40.0,
                    "energyStored": null, "internalClockRate": null, "currentTask": null},
        "inventory": {"capacity": 1.0, "usedCapacity": 0.0, "freeCapacity": 1.0,
                      "items": [], "resourceStocks": [], "externalTanks": [], "containers": []}
    }"#).unwrap());
    assert_eq!(state.repair_max_percent(), 40.0);
}

#[test]
fn repair_metals_stock_no_probe() {
    assert_eq!(AppState::default().repair_metals_stock(), 0.0);
}

#[test]
fn repair_metals_stock_returns_metals() {
    let mut state = AppState::default();
    state.probe = Some(serde_json::from_str(r#"{
        "id": 1, "name": "t", "status": "idle",
        "fuel": {"deuterium": null}, "sensorMode": "normal",
        "sector": null, "movement": null, "systems": null,
        "inventory": {"capacity": 1.0, "usedCapacity": 0.25, "freeCapacity": 0.75,
            "items": [], "externalTanks": [], "containers": [],
            "resourceStocks": [
                {"id": "s-metals", "type": "metals", "name": "Metals", "amount": 0.25, "containerSpace": 0.25, "containers": []},
                {"id": "s-ice", "type": "ice", "name": "Ice", "amount": 0.10, "containerSpace": 0.10, "containers": []}
            ]}
    }"#).unwrap());
    assert_eq!(state.repair_metals_stock(), 0.25);
}

// ── batch_tick ────────────────────────────────────────────────────────────

#[test]
fn batch_tick_decrements() {
    let mut state = AppState::default();
    state.scan_batch = Some(3);
    state.batch_tick();
    assert_eq!(state.scan_batch, Some(2));
}

#[test]
fn start_batch_records_total() {
    let mut state = AppState::default();
    state.start_batch(12);
    assert_eq!(state.scan_batch, Some(12));
    assert_eq!(state.scan_batch_total, 12);
    state.batch_tick();
    // total is preserved while remaining decreases
    assert_eq!(state.scan_batch, Some(11));
    assert_eq!(state.scan_batch_total, 12);
}

#[test]
fn batch_tick_clears_at_zero() {
    let mut state = AppState::default();
    state.scan_batch = Some(1);
    state.batch_tick();
    assert_eq!(state.scan_batch, None);
}

#[test]
fn batch_tick_no_batch_is_noop() {
    let mut state = AppState::default();
    state.batch_tick();
    assert_eq!(state.scan_batch, None);
}

// ── update_sector (deduplication) ─────────────────────────────────────────

#[test]
fn update_sector_inserts_at_front() {
    let mut state = AppState::default();
    state.update_sector(make_sector(2., 0., -2.));
    assert_eq!(state.scan_history.len(), 1);
    assert_eq!(state.scan_history_idx, 0);
}

#[test]
fn update_sector_stamps_scanned_at() {
    let mut state = AppState::default();
    // fixture JSON has no scannedAt field — defaults to None
    let sector = make_sector(2., 0., -2.);
    assert!(sector.scanned_at.is_none());
    state.update_sector(sector);
    assert!(state.scan_history[0].scanned_at.is_some());
}

#[test]
fn update_sector_deduplicates_by_coords() {
    let mut state = AppState::default();
    state.update_sector(make_sector(2., 0., -2.));
    state.update_sector(make_sector(4., 0., -4.));
    state.update_sector(make_sector(2., 0., -2.)); // duplicate
    assert_eq!(state.scan_history.len(), 2);
    // the re-scanned sector is now at the front
    assert_eq!(state.scan_history[0].relative_coordinates.x as i32, 2);
}

#[test]
fn update_sector_resets_scroll_and_idx() {
    let mut state = AppState::default();
    state.update_sector(make_sector(0., 0., 0.));
    state.update_sector(make_sector(2., 0., 0.));
    state.scan_history_idx = 1;
    state.scan_detail_scroll = 5;
    state.update_sector(make_sector(4., 0., 0.));
    assert_eq!(state.scan_history_idx, 0);
    assert_eq!(state.scan_detail_scroll, 0);
}

// ── manny_craft_recipes / atomic_printer_recipes ──────────────────────────

#[test]
fn manny_craft_recipes_filters_correctly() {
    let mut state = AppState::default();
    state.recipes = vec![
        serde_json::from_str(r#"{"id":"r1","name":"R1","craftableBy":["manny"],
            "ingredients":[],"durationSeconds":60,
            "output":{"type":"x","name":"X","containerSpace":1.0,"containerSpaceUnit":"ECE","capacityBonus":null}}"#).unwrap(),
        serde_json::from_str(r#"{"id":"r2","name":"R2","craftableBy":["atomic_3d_printer"],
            "ingredients":[],"durationSeconds":60,
            "output":{"type":"x","name":"X","containerSpace":1.0,"containerSpaceUnit":"ECE","capacityBonus":null}}"#).unwrap(),
        serde_json::from_str(r#"{"id":"r3","name":"R3","craftableBy":["manny","atomic_3d_printer"],
            "ingredients":[],"durationSeconds":60,
            "output":{"type":"x","name":"X","containerSpace":1.0,"containerSpaceUnit":"ECE","capacityBonus":null}}"#).unwrap(),
    ];
    let manny = state.manny_craft_recipes();
    assert_eq!(manny.len(), 2);
    assert!(manny.iter().any(|r| r.id == "r1"));
    assert!(manny.iter().any(|r| r.id == "r3"));
}

#[test]
fn atomic_printer_recipes_filters_correctly() {
    let mut state = AppState::default();
    state.recipes = vec![
        serde_json::from_str(r#"{"id":"r1","name":"R1","craftableBy":["manny"],
            "ingredients":[],"durationSeconds":60,
            "output":{"type":"x","name":"X","containerSpace":1.0,"containerSpaceUnit":"ECE","capacityBonus":null}}"#).unwrap(),
        serde_json::from_str(r#"{"id":"r2","name":"R2","craftableBy":["atomic_3d_printer"],
            "ingredients":[],"durationSeconds":60,
            "output":{"type":"x","name":"X","containerSpace":1.0,"containerSpaceUnit":"ECE","capacityBonus":null}}"#).unwrap(),
    ];
    let printer = state.atomic_printer_recipes();
    assert_eq!(printer.len(), 1);
    assert_eq!(printer[0].id, "r2");
}

// ── has_atomic_printer ────────────────────────────────────────────────────

#[test]
fn has_atomic_printer_no_probe() {
    assert!(!AppState::default().has_atomic_printer());
}

#[test]
fn has_atomic_printer_absent() {
    let mut state = AppState::default();
    state.probe = Some(make_probe(7.0, 0., 0., 0.));
    assert!(!state.has_atomic_printer());
}

#[test]
fn has_atomic_printer_present() {
    let mut state = AppState::default();
    state.probe = Some(serde_json::from_str(r#"{
        "id": 1, "name": "t", "status": "idle",
        "fuel": {"deuterium": null}, "sensorMode": "normal",
        "sector": null, "movement": null, "systems": null,
        "inventory": {"capacity": 10.0, "usedCapacity": 1.0, "freeCapacity": 9.0,
            "resourceStocks": [], "externalTanks": [], "containers": [],
            "items": [{"id": "ap-1", "type": "atomic_3d_printer", "name": "Atomic Printer",
                "containerSpace": 1.0, "currentTask": null, "taskProgressPercent": 0.0,
                "location": {"type": "probe", "sector": null}, "cargo": null, "container": null}]}
    }"#).unwrap());
    assert!(state.has_atomic_printer());
}

// ── collect_mineable_candidates ───────────────────────────────────────────

#[test]
fn collect_mineable_candidates_empty_when_no_sector() {
    assert!(AppState::default().collect_mineable_candidates().is_empty());
}

#[test]
fn collect_mineable_candidates_returns_asteroid_targets() {
    let mut state = AppState::default();
    state.probe = Some(probe_at(2., 0., -2.));
    state.scan_history = vec![make_sector_with_objects(2., 0., -2., r#"[
        {
            "id": "planet-1", "type": "planet", "name": "P1",
            "estimated": null, "summary": null, "mass": null, "massUnit": null,
            "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
            "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
            "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
            "capacity": null, "capacityUnit": null,
            "minableTargets": [
                {"id": "ast-1", "type": "asteroid", "name": "Rock A", "mass": null, "resourceTypes": ["metals"]},
                {"id": "ast-2", "type": "asteroid", "name": "Rock B", "mass": null, "resourceTypes": null}
            ],
            "waypointBookmarks": [], "bookmarkTargets": []
        }
    ]"#)];
    let candidates = state.collect_mineable_candidates();
    assert_eq!(candidates.len(), 2);
    assert!(candidates.iter().any(|(id, _)| id == "ast-1"));
    assert!(candidates.iter().any(|(id, _)| id == "ast-2"));
}

#[test]
fn deuterium_station_detected_in_current_sector() {
    let mut state = AppState::default();
    state.probe = Some(probe_at(2., 0., -2.));
    state.scan_history = vec![make_sector_with_objects(2., 0., -2., r#"[
        {
            "id": "station-1", "type": "deuterium_refuel_station", "name": "Refuel Stop",
            "estimated": null, "summary": null, "mass": null, "massUnit": null,
            "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
            "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
            "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
            "capacity": null, "capacityUnit": null,
            "minableTargets": null, "waypointBookmarks": [], "bookmarkTargets": []
        }
    ]"#)];
    assert!(state.deuterium_station_in_current_sector());
}

fn relay_sector(status: &str) -> SectorObservation {
    make_sector_with_objects(0., 0., 0., &format!(r#"[
        {{
            "id": "42", "type": "scut_relay", "name": "Relais SCUT",
            "estimated": false, "summary": "relay", "mass": 0.0, "massUnit": null,
            "radius": 0.0, "radiusUnit": null, "dangerLevel": "low", "salvageable": true,
            "status": "{status}", "coverageRadiusSectors": 10,
            "createdByProbeName": "Probe X", "activatedAt": null,
            "network": null,
            "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
            "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
            "capacity": null, "capacityUnit": null,
            "minableTargets": null, "waypointBookmarks": [], "bookmarkTargets": []
        }}
    ]"#))
}

#[test]
fn relay_status_read_from_sector_object() {
    let mut state = AppState::default();
    state.probe = Some(probe_at(0., 0., 0.));
    state.scan_history = vec![relay_sector("off")];
    assert_eq!(
        state.sector_object_relay_status("42"),
        Some(crate::api::types::ScutRelayStatus::Off)
    );
}

#[test]
fn inactive_relay_offers_turn_on_and_salvage() {
    let mut state = AppState::default();
    state.probe = Some(probe_at(0., 0., 0.));
    state.scan_history = vec![relay_sector("off")];
    let entry = state.scanner_objects().into_iter()
        .find(|e| matches!(e.object_type, crate::api::types::SectorObjectType::ScutRelay))
        .expect("relay entry present");
    let actions = state.actions_for_object(&entry);
    assert!(actions.contains(&ObjectAction::TurnOnRelay));
    assert!(actions.contains(&ObjectAction::Salvage));
}

#[test]
fn active_relay_offers_no_turn_on() {
    let mut state = AppState::default();
    state.probe = Some(probe_at(0., 0., 0.));
    state.scan_history = vec![relay_sector("on")];
    let entry = state.scanner_objects().into_iter()
        .find(|e| matches!(e.object_type, crate::api::types::SectorObjectType::ScutRelay))
        .expect("relay entry present");
    let actions = state.actions_for_object(&entry);
    assert!(!actions.contains(&ObjectAction::TurnOnRelay));
}

#[test]
fn deuterium_station_absent_in_current_sector() {
    let mut state = AppState::default();
    state.probe = Some(probe_at(0., 0., 0.));
    state.scan_history = vec![make_sector_with_objects(0., 0., 0., r#"[
        {
            "id": "planet-1", "type": "planet", "name": "P1",
            "estimated": null, "summary": null, "mass": null, "massUnit": null,
            "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
            "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
            "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
            "capacity": null, "capacityUnit": null,
            "minableTargets": null, "waypointBookmarks": [], "bookmarkTargets": []
        }
    ]"#)];
    assert!(!state.deuterium_station_in_current_sector());
}

#[test]
fn collect_mineable_candidates_unnamed_fallback() {
    let mut state = AppState::default();
    state.probe = Some(probe_at(0., 0., 0.));
    state.scan_history = vec![make_sector_with_objects(0., 0., 0., r#"[
        {
            "id": "planet-1", "type": "planet", "name": null,
            "estimated": null, "summary": null, "mass": null, "massUnit": null,
            "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
            "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
            "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
            "capacity": null, "capacityUnit": null,
            "minableTargets": [{"id": "ast-x", "type": "asteroid", "name": null, "mass": null, "resourceTypes": null}],
            "waypointBookmarks": [], "bookmarkTargets": []
        }
    ]"#)];
    let candidates = state.collect_mineable_candidates();
    assert_eq!(candidates[0].1, "unnamed");
}

// ── collect_idle_onboard_mannies ──────────────────────────────────────────

#[test]
fn collect_idle_onboard_mannies_empty_when_no_mannies() {
    assert!(AppState::default().collect_idle_onboard_mannies().is_empty());
}

#[test]
fn collect_idle_onboard_mannies_filters_correctly() {
    let mut state = AppState::default();
    state.mannies = Some(vec![
        make_manny("m1", "probe", true, None),           // included
        make_manny("m2", "probe", false, Some("mining")), // busy — excluded
        make_manny("m3", "sector", true, None),           // in sector — excluded
    ]);
    let result = state.collect_idle_onboard_mannies();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "m1");
}

// ── collect_detachable_containers ─────────────────────────────────────────

#[test]
fn collect_detachable_containers_empty_when_no_probe() {
    assert!(AppState::default().collect_detachable_containers().is_empty());
}

#[test]
fn collect_detachable_containers_excludes_probe_container() {
    let mut state = AppState::default();
    state.probe = Some(serde_json::from_str(r#"{
        "id": 1, "name": "t", "status": "idle",
        "fuel": {"deuterium": null}, "sensorMode": "normal",
        "sector": null, "movement": null, "systems": null,
        "inventory": {"capacity": 10.0, "usedCapacity": 0.0, "freeCapacity": 10.0,
            "items": [], "resourceStocks": [], "externalTanks": [],
            "containers": [
                {"id": "c-probe", "kind": "probe", "label": "Main Hold", "sortOrder": 0,
                 "capacity": 5.0, "usedCapacity": 0.0, "freeCapacity": 5.0,
                 "rules": {"priority": [], "exclusion": [], "strictExclusion": []}},
                {"id": "c-ext", "kind": "external", "label": "Ext Container", "sortOrder": 1,
                 "capacity": 2.0, "usedCapacity": 0.0, "freeCapacity": 2.0,
                 "rules": {"priority": [], "exclusion": [], "strictExclusion": []}}
            ]}
    }"#).unwrap());
    let result = state.collect_detachable_containers();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "c-ext");
    assert_eq!(result[0].1, "Ext Container");
}

// ── collect_detached_containers ───────────────────────────────────────────

#[test]
fn collect_detached_containers_empty_when_no_scan() {
    assert!(AppState::default().collect_detached_containers().is_empty());
}

#[test]
fn collect_detached_containers_returns_only_detached_type() {
    let mut state = AppState::default();
    state.probe = Some(probe_at(0., 0., 0.));
    state.scan_history = vec![make_sector_with_objects(0., 0., 0., r#"[
        {
            "id": "dc-1", "type": "detached_container", "name": "Floater",
            "estimated": null, "summary": null, "mass": null, "massUnit": null,
            "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
            "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
            "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
            "capacity": null, "capacityUnit": null, "minableTargets": null,
            "waypointBookmarks": [], "bookmarkTargets": []
        },
        {
            "id": "ast-1", "type": "asteroid", "name": "Rock",
            "estimated": null, "summary": null, "mass": null, "massUnit": null,
            "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
            "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
            "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
            "capacity": null, "capacityUnit": null, "minableTargets": null,
            "waypointBookmarks": [], "bookmarkTargets": []
        }
    ]"#)];
    let result = state.collect_detached_containers();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "dc-1");
    assert_eq!(result[0].1, "Floater");
}

#[test]
fn collect_detached_containers_unnamed_fallback() {
    let mut state = AppState::default();
    state.probe = Some(probe_at(0., 0., 0.));
    state.scan_history = vec![make_sector_with_objects(0., 0., 0., r#"[
        {
            "id": "dc-2", "type": "detached_container", "name": null,
            "estimated": null, "summary": null, "mass": null, "massUnit": null,
            "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
            "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
            "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
            "capacity": null, "capacityUnit": null, "minableTargets": null,
            "waypointBookmarks": [], "bookmarkTargets": []
        }
    ]"#)];
    let result = state.collect_detached_containers();
    assert_eq!(result[0].1, "unnamed container");
}

// ── scanner_objects / actions_for_object ─────────────────────────────────

const MIXED_OBJECTS: &str = r#"[
    {
        "id": "planet-1", "type": "planet", "name": "P1",
        "estimated": null, "summary": null, "mass": null, "massUnit": null,
        "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
        "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
        "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
        "capacity": null, "capacityUnit": null,
        "minableTargets": [
            {"id": "ast-1", "type": "asteroid", "name": "Rock A", "mass": null, "resourceTypes": ["metals"]}
        ],
        "waypointBookmarks": [], "bookmarkTargets": []
    },
    {
        "id": "wreck-1", "type": "manny", "name": "Lost Manny",
        "estimated": null, "summary": null, "mass": null, "massUnit": null,
        "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": true,
        "mannyState": "wreck", "mannyUid": null, "cargo": null, "itemType": null,
        "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
        "capacity": null, "capacityUnit": null, "minableTargets": null,
        "waypointBookmarks": [], "bookmarkTargets": []
    },
    {
        "id": "dc-1", "type": "detached_container", "name": "Floater",
        "estimated": null, "summary": null, "mass": null, "massUnit": null,
        "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
        "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
        "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
        "capacity": null, "capacityUnit": null, "minableTargets": null,
        "waypointBookmarks": [], "bookmarkTargets": []
    }
]"#;

#[test]
fn scanner_objects_empty_when_not_in_probe_sector() {
    let mut state = AppState::default();
    state.probe = Some(probe_at(4., 0., 0.));
    state.scan_history = vec![make_sector_with_objects(0., 0., 0., MIXED_OBJECTS)];
    assert!(!state.viewing_probe_sector());
    assert!(state.scanner_objects().is_empty());
}

#[test]
fn scanner_objects_order_top_level_then_nested() {
    let mut state = AppState::default();
    state.probe = Some(probe_at(0., 0., 0.));
    state.scan_history = vec![make_sector_with_objects(0., 0., 0., MIXED_OBJECTS)];
    assert!(state.viewing_probe_sector());
    let entries = state.scanner_objects();
    let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, vec!["planet-1", "ast-1", "wreck-1", "dc-1"]);
    assert_eq!(entries[1].provenance, ObjectProvenance::MinableTarget);
    assert_eq!(entries[2].provenance, ObjectProvenance::TopLevel);
}

#[test]
fn actions_for_object_by_kind() {
    let mut state = AppState::default();
    state.probe = Some(probe_at(0., 0., 0.));
    state.scan_history = vec![make_sector_with_objects(0., 0., 0., MIXED_OBJECTS)];
    let entries = state.scanner_objects();

    // planet (top-level, no bookmark in inventory): no actions
    assert!(state.actions_for_object(&entries[0]).is_empty());
    // minable asteroid: mine
    assert_eq!(state.actions_for_object(&entries[1]), vec![ObjectAction::Mine]);
    // manny wreck: salvage
    assert_eq!(state.actions_for_object(&entries[2]), vec![ObjectAction::Salvage]);
    // detached container: recover
    assert_eq!(state.actions_for_object(&entries[3]), vec![ObjectAction::Recover]);
}

#[test]
fn actions_include_deploy_when_bookmark_in_inventory() {
    let mut state = AppState::default();
    let items = r#"[{"id": "wb-1", "type": "waypoint_bookmark", "name": "Bookmark",
        "containerSpace": 0.1, "currentTask": null, "taskProgressPercent": 0.0,
        "location": null, "cargo": null, "container": null}]"#;
    state.probe = Some(probe_with_inventory(items, "[]"));
    // probe_with_inventory has no sector — give it one at origin
    if let Some(ref mut p) = state.probe {
        p.sector = Some(serde_json::from_str(r#"{"relative": {"x": 0.0, "y": 0.0, "z": 0.0}}"#).unwrap());
    }
    state.scan_history = vec![make_sector_with_objects(0., 0., 0., MIXED_OBJECTS)];
    let entries = state.scanner_objects();
    // top-level planet gains deploy; nested minable target does not
    assert_eq!(state.actions_for_object(&entries[0]), vec![ObjectAction::DeployWaypoint]);
    assert_eq!(state.actions_for_object(&entries[1]), vec![ObjectAction::Mine]);
}

// ── scan filter ───────────────────────────────────────────────────────────

#[test]
fn scan_filter_cycles() {
    assert_eq!(ScanFilter::All.next(), ScanFilter::Objects);
    assert_eq!(ScanFilter::Danger.next(), ScanFilter::All);
}

#[test]
fn filtered_history_respects_objects_filter() {
    let mut state = AppState::default();
    state.scan_history = vec![
        make_sector_with_objects(0., 0., 0., MIXED_OBJECTS), // has objects
        make_sector(2., 0., 0.),                              // objects: null
    ];
    state.scan_filter = ScanFilter::Objects;
    assert_eq!(state.filtered_history_indices(), vec![0]);
    state.scan_filter = ScanFilter::Minable;
    assert_eq!(state.filtered_history_indices(), vec![0]);
    state.scan_filter = ScanFilter::Danger;
    assert!(state.filtered_history_indices().is_empty());
}

#[test]
fn hist_nav_skips_filtered_out_entries() {
    let mut state = AppState::default();
    state.scan_history = vec![
        make_sector_with_objects(0., 0., 0., MIXED_OBJECTS),
        make_sector(2., 0., 0.),
        make_sector_with_objects(4., 0., 0., MIXED_OBJECTS),
    ];
    state.scan_filter = ScanFilter::Objects;
    state.scan_history_idx = 0;
    state.scan_hist_next();
    // skips index 1 (no objects)
    assert_eq!(state.scan_history_idx, 2);
    state.scan_hist_prev();
    assert_eq!(state.scan_history_idx, 0);
}

#[test]
fn cycle_filter_snaps_selection_into_filter() {
    let mut state = AppState::default();
    state.scan_history = vec![
        make_sector_with_objects(0., 0., 0., MIXED_OBJECTS),
        make_sector(2., 0., 0.),
    ];
    state.scan_history_idx = 1;
    state.cycle_scan_filter(); // All → Objects, idx 1 no longer visible
    assert_eq!(state.scan_history_idx, 0);
}

// ── collect_waypoints ─────────────────────────────────────────────────────

#[test]
fn collect_waypoints_empty_history() {
    assert!(AppState::default().collect_waypoints().is_empty());
}

#[test]
fn collect_waypoints_bookmarks_first_then_sorted_by_distance() {
    let mut state = AppState::default();
    let star_far: SectorObservation = serde_json::from_str(r#"{
        "relativeCoordinates": {"x": 6.0, "y": 0.0, "z": 0.0},
        "distance": 6, "knowledgeLevel": "detailed", "confidence": 1.0,
        "objects": [{
            "id": "star-1", "type": "star", "name": "Sun",
            "estimated": null, "summary": null, "mass": null, "massUnit": null,
            "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
            "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
            "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
            "capacity": null, "capacityUnit": null, "minableTargets": null,
            "waypointBookmarks": [], "bookmarkTargets": []
        }],
        "probes": null, "possibleObjects": null, "estimatedObjects": null,
        "navigationalRisk": null, "message": null, "sensorMode": null, "dataFreshness": null,
        "scan": {"currentSectorResidenceSeconds": 0, "requiredResidenceSeconds": 0, "scanQuality": 1.0}
    }"#).unwrap();
    let bookmark_near: SectorObservation = serde_json::from_str(r#"{
        "relativeCoordinates": {"x": 2.0, "y": 0.0, "z": 0.0},
        "distance": 2, "knowledgeLevel": "detailed", "confidence": 1.0,
        "objects": [{
            "id": "planet-1", "type": "planet", "name": "P1",
            "estimated": null, "summary": null, "mass": null, "massUnit": null,
            "radius": null, "radiusUnit": null, "dangerLevel": null, "salvageable": null,
            "mannyState": null, "mannyUid": null, "cargo": null, "itemType": null,
            "quantity": null, "containerSpace": null, "mode": null, "targetObjectId": null,
            "capacity": null, "capacityUnit": null,
            "minableTargets": [{"id": "ast-1", "type": "asteroid", "name": "Rock", "mass": null, "resourceTypes": null}],
            "waypointBookmarks": [{"name": "home", "playerId": 1, "playerName": "me", "createdAt": "2026-01-01T00:00:00Z"}],
            "bookmarkTargets": []
        }],
        "probes": null, "possibleObjects": null, "estimatedObjects": null,
        "navigationalRisk": null, "message": null, "sensorMode": null, "dataFreshness": null,
        "scan": {"currentSectorResidenceSeconds": 0, "requiredResidenceSeconds": 0, "scanQuality": 1.0}
    }"#).unwrap();
    // far star scanned first (more recent in history)
    state.scan_history = vec![star_far, bookmark_near];

    let entries = state.collect_waypoints();
    assert_eq!(entries.len(), 3);
    // bookmark first regardless of history order
    assert_eq!(entries[0].kind, WaypointKind::Bookmark);
    assert!(entries[0].label.contains("home"));
    assert_eq!((entries[0].x, entries[0].y, entries[0].z), (2, 0, 0));
    // then star, then minable
    assert_eq!(entries[1].kind, WaypointKind::Star);
    assert_eq!(entries[1].distance, 6);
    assert_eq!(entries[2].kind, WaypointKind::Minable);
    assert_eq!(entries[2].distance, 2);
}

#[test]
fn scanner_obj_nav_wraps() {
    let mut state = AppState::default();
    state.probe = Some(probe_at(0., 0., 0.));
    state.scan_history = vec![make_sector_with_objects(0., 0., 0., MIXED_OBJECTS)];
    state.scanner_obj_selection = Some(0);
    state.scanner_obj_prev();
    assert_eq!(state.scanner_obj_selection, Some(3));
    state.scanner_obj_next();
    assert_eq!(state.scanner_obj_selection, Some(0));
}

// ── anim / theme ──────────────────────────────────────────────────────────

#[test]
fn tick_anim_advances_and_finishes_boot() {
    let mut state = AppState::default();
    state.anim.booting = true;
    for _ in 0..BOOT_TOTAL_FRAMES {
        state.tick_anim();
    }
    assert!(!state.anim.booting, "boot should complete");
    assert_eq!(state.anim.frame, BOOT_TOTAL_FRAMES);
}

#[test]
fn toggle_theme_flips_between_classic_and_retro() {
    let mut state = AppState::default();
    assert_eq!(state.ui_theme, UiTheme::Classic);
    state.toggle_theme();
    assert_eq!(state.ui_theme, UiTheme::Retro);
    state.toggle_theme();
    assert_eq!(state.ui_theme, UiTheme::Classic);
}

#[test]
fn anim_tick_active_only_for_retro_or_boot() {
    let mut state = AppState::default();
    state.animations_enabled = true;
    assert!(!state.anim_tick_active(), "classic theme: no tick");
    state.ui_theme = UiTheme::Retro;
    assert!(state.anim_tick_active());
    state.animations_enabled = false;
    assert!(!state.anim_tick_active(), "retro without animations: no tick");
    state.anim.booting = true;
    assert!(state.anim_tick_active(), "boot still animates");
}

#[test]
fn anim_hash_is_deterministic_and_spreads() {
    assert_eq!(anim_hash(42), anim_hash(42));
    assert_ne!(anim_hash(1), anim_hash(2));
}
