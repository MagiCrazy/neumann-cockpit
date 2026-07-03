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

fn remote_manny(task_visibility: &str, task: &str) -> crate::api::types::Manny {
    serde_json::from_str(&format!(r#"{{
        "id": "mny_remote", "name": "manny-r",
        "location": {{ "type": "sector", "sector": {{ "relative": {{"x": 2, "y": 0, "z": -2}} }} }},
        "currentTask": {task},
        "taskProgressPercent": 0,
        "taskVisibility": "{task_visibility}",
        "cargo": {{ "capacity": 0.05, "deuterium": 0, "metals": 0, "ice": 0, "organicCompounds": 0, "capacityUnit": "earth_container_equivalent" }},
        "canReceiveOrders": false,
        "taskEstimatedEndTime": null
    }}"#)).unwrap()
}

#[test]
fn idle_scut_remote_manny_is_remote_minable() {
    let state = AppState::default();
    let manny = remote_manny("scut_network", "null");
    assert!(state.manny_remote_minable(&manny));
    assert_eq!(state.manny_sector_coords(&manny), Some((2, 0, -2)));
}

#[test]
fn busy_or_too_far_manny_not_remote_minable() {
    let state = AppState::default();
    assert!(!state.manny_remote_minable(&remote_manny("scut_network", "\"mining\"")));
    assert!(!state.manny_remote_minable(&remote_manny("too_far", "null")));
}

#[test]
fn remote_mine_advances_to_pick_asteroid_when_sector_loads() {
    let mut state = AppState::default();
    state.remote_mine = RemoteMineInput::Loading {
        manny_id: "mny_remote".into(),
        manny_name: "manny-r".into(),
        x: 2, y: 0, z: -2,
    };
    state.scan_history = vec![make_sector_with_objects(2., 0., -2., r#"[
        {
            "id": "ast-9", "type": "asteroid", "name": "Rock", "summary": null,
            "estimated": null, "mass": null, "massUnit": null, "radius": null, "radiusUnit": null,
            "dangerLevel": null, "salvageable": null, "mannyState": null, "mannyUid": null,
            "cargo": null, "itemType": null, "quantity": null, "containerSpace": null,
            "mode": null, "targetObjectId": null, "capacity": null, "capacityUnit": null,
            "minableTargets": null, "waypointBookmarks": [], "bookmarkTargets": []
        }
    ]"#)];
    state.remote_mine_sector_loaded(2, 0, -2);
    assert!(matches!(state.remote_mine, RemoteMineInput::PickAsteroid { .. }));
}

#[test]
fn unread_message_count_counts_only_unread() {
    let mut state = AppState::default();
    let mk = |id: i64, status: &str| -> crate::api::types::ProbeMessage {
        serde_json::from_str(&format!(r#"{{
            "id": {id},
            "sender": {{ "type": "probe", "id": 2, "name": "nova" }},
            "recipient": {{ "type": "probe", "id": 1, "name": "me" }},
            "sector": {{ "relative": {{"x":0,"y":0,"z":0}} }},
            "body": "hi", "status": "{status}", "readAt": null,
            "createdAt": "2026-06-06T12:00:00+00:00", "updatedAt": "2026-06-06T12:00:00+00:00"
        }}"#)).unwrap()
    };
    state.messages = vec![mk(1, "unread"), mk(2, "read"), mk(3, "unread")];
    assert_eq!(state.unread_message_count(), 2);
}

#[test]
fn scut_coverage_read_from_sector() {
    let mut state = AppState::default();
    state.probe = Some(probe_at(0., 0., 0.));
    let mut sector = make_sector_with_objects(0., 0., 0., "[]");
    sector.scut_networks = vec![
        serde_json::from_str(r#"{"id": 7, "name": "Delta SCUT"}"#).unwrap(),
    ];
    state.scan_history = vec![sector];
    let cov = state.scut_coverage();
    assert_eq!(cov.len(), 1);
    assert_eq!(cov[0], (7, "Delta SCUT".to_string()));
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

// ── cockpit color mode ─────────────────────────────────────────────────────

#[test]
fn color_mode_cycles_and_defaults_green() {
    assert_eq!(ColorMode::default(), ColorMode::MonoGreen);
    let mut m = ColorMode::default();
    for _ in 0..4 {
        m = m.cycle();
    }
    assert_eq!(m, ColorMode::MonoGreen, "cycles back after four steps");
}

// ── cockpit contextual menu ────────────────────────────────────────────────

fn menu_item(menu: &ContextMenu, action: MenuAction) -> &MenuItem {
    menu.items.iter().find(|i| i.action == action).expect("menu item")
}

#[test]
fn mannies_context_menu_reflects_manny_state() {
    let mut state = AppState::default();
    state.active_pane = Pane::Mannies;

    // Idle manny: order-requiring actions enabled, recall disabled, and
    // conditional actions (refuel/drop-cargo) disabled without their context.
    state.mannies = Some(vec![make_manny("m1", "probe_rack", true, None)]);
    let menu = state.build_context_menu().expect("mannies menu");
    assert!(menu_item(&menu, MenuAction::Mine).enabled);
    assert!(menu_item(&menu, MenuAction::Repair).enabled);
    assert!(menu_item(&menu, MenuAction::Craft).enabled);
    assert!(menu_item(&menu, MenuAction::Salvage).enabled);
    assert!(menu_item(&menu, MenuAction::Inspect).enabled);
    assert!(menu_item(&menu, MenuAction::Rename).enabled);
    assert!(!menu_item(&menu, MenuAction::Recall).enabled);
    assert!(!menu_item(&menu, MenuAction::Refuel).enabled); // no station
    assert!(!menu_item(&menu, MenuAction::DropCargo).enabled); // not waiting
    // Cursor lands on the first enabled item.
    assert!(menu.items[menu.cursor].enabled);

    // Busy manny with a task: order-requiring actions disabled, recall enabled.
    state.mannies = Some(vec![make_manny("m2", "sector", false, Some("mining"))]);
    let menu = state.build_context_menu().expect("mannies menu");
    assert!(!menu_item(&menu, MenuAction::Repair).enabled);
    assert!(!menu_item(&menu, MenuAction::Salvage).enabled);
    assert!(menu_item(&menu, MenuAction::Recall).enabled);
}

#[test]
fn context_menu_none_for_pane_without_actions() {
    let mut state = AppState::default();
    state.active_pane = Pane::Probe;
    assert!(state.build_context_menu().is_none());
}

#[test]
fn inventory_context_menu_present_but_disabled_when_empty() {
    let mut state = AppState::default();
    state.active_pane = Pane::Inventory;
    let menu = state.build_context_menu().expect("inventory menu");
    assert_eq!(menu.items.len(), 3);
    // Nothing loaded → every action disabled with a reason.
    assert!(menu.items.iter().all(|i| !i.enabled && i.disabled_reason.is_some()));
}

// ── periodic auto-refresh gating ──────────────────────────────────────────

#[test]
fn seconds_since_sync_none_before_first_sync() {
    let state = AppState::default();
    assert_eq!(state.seconds_since_sync(), None);
}

#[test]
fn periodic_refresh_not_due_without_prior_sync() {
    // Never synced → not due (avoids spin-retry on a failed initial fetch).
    let state = AppState::default();
    assert!(!state.periodic_refresh_due());
}

#[test]
fn periodic_refresh_due_after_60s_when_idle() {
    let mut state = AppState::default();
    state.last_update = Some(chrono::Local::now() - chrono::Duration::seconds(90));
    assert!(state.periodic_refresh_due());
}

#[test]
fn periodic_refresh_not_due_when_recent_or_loading() {
    let mut state = AppState::default();
    state.last_update = Some(chrono::Local::now() - chrono::Duration::seconds(10));
    assert!(!state.periodic_refresh_due(), "10s is within the 60s window");
    state.last_update = Some(chrono::Local::now() - chrono::Duration::seconds(90));
    state.loading = true;
    assert!(!state.periodic_refresh_due(), "a fetch already in flight");
}

#[test]
fn refresh_backoff_grows_then_caps_at_60() {
    let mut state = AppState::default();
    assert_eq!(state.refresh_backoff_secs(), 60, "healthy cadence is 60s");
    for (failures, expected) in [(1, 5), (2, 10), (3, 20), (4, 40), (5, 60), (6, 60), (99, 60)] {
        state.consecutive_failures = failures;
        assert_eq!(state.refresh_backoff_secs(), expected, "after {failures} failures");
    }
}

#[test]
fn periodic_refresh_backs_off_after_failure() {
    let mut state = AppState::default();
    // Stale data (never re-synced), one recent failed attempt.
    state.last_update = Some(chrono::Local::now() - chrono::Duration::seconds(300));
    state.consecutive_failures = 1; // backoff = 5s
    state.last_attempt = Some(chrono::Local::now() - chrono::Duration::seconds(2));
    assert!(!state.periodic_refresh_due(), "2s < 5s backoff → not due yet");
    state.last_attempt = Some(chrono::Local::now() - chrono::Duration::seconds(6));
    assert!(state.periodic_refresh_due(), "6s ≥ 5s backoff → due");
}

#[test]
fn successful_probe_sync_clears_backoff() {
    let mut state = AppState::default();
    state.consecutive_failures = 4;
    state.note_refresh_failure();
    assert_eq!(state.consecutive_failures, 5);
    state.update_probe(make_probe(1.0, 0.0, 0.0, 0.0));
    assert_eq!(state.consecutive_failures, 0, "a successful sync resets the backoff");
}

// ── manny task progress interpolation ─────────────────────────────────────

#[test]
fn manny_task_progress_falls_back_to_snapshot_without_timestamps() {
    use crate::ui::panels::mannies::manny_task_progress;
    let mut m = make_manny("m1", "probe", false, Some("mining"));
    m.task_progress_percent = 42.0; // no observed_at / end → static
    assert!((manny_task_progress(&m) - 0.42).abs() < 1e-9);
}

#[test]
fn manny_task_progress_interpolates_forward_between_fetches() {
    use crate::ui::panels::mannies::manny_task_progress;
    let mut m = make_manny("m1", "sector", false, Some("mining"));
    // Snapshot: 20% when observed 30 s ago, 30 s left → total 75 s, now ~60%.
    m.task_progress_percent = 20.0;
    m.observed_at = Some(chrono::Utc::now() - chrono::Duration::seconds(30));
    m.task_estimated_end_time = Some(chrono::Utc::now() + chrono::Duration::seconds(30));
    let prog = manny_task_progress(&m);
    assert!(prog > 0.20, "progress advanced past the snapshot: {prog}");
    assert!(prog < 1.0, "not complete yet: {prog}");
}

#[test]
fn manny_task_progress_complete_when_past_end() {
    use crate::ui::panels::mannies::manny_task_progress;
    let mut m = make_manny("m1", "sector", false, Some("mining"));
    m.task_progress_percent = 80.0;
    m.observed_at = Some(chrono::Utc::now() - chrono::Duration::seconds(120));
    m.task_estimated_end_time = Some(chrono::Utc::now() - chrono::Duration::seconds(10));
    assert_eq!(manny_task_progress(&m), 1.0);
}

// ── new v63 planet/asteroid fields ────────────────────────────────────────

#[test]
fn sector_object_planet_science_fields_deserialize() {
    use crate::api::types::SectorObject;
    let planet: SectorObject = serde_json::from_str(r#"{
        "type": "planet",
        "name": "Kepler-relative-1",
        "summary": "temperate ocean world",
        "category": "ocean",
        "habitabilityScore": 0.82,
        "mannyMineable": true,
        "resourceTypes": ["metals", "ice"],
        "resourceComposition": {"deuterium": 0.0, "metals": 0.6, "ice": 0.4, "carbon_compounds": 0.0}
    }"#).unwrap();
    assert_eq!(planet.category.as_deref(), Some("ocean"));
    assert_eq!(planet.habitability_score, Some(0.82));
    assert_eq!(planet.manny_mineable, Some(true));
    assert_eq!(planet.resource_types, vec!["metals", "ice"]);
    let comp = planet.resource_composition.expect("composition");
    assert!((comp.metals - 0.6).abs() < 1e-9 && (comp.ice - 0.4).abs() < 1e-9);
}

#[test]
fn sector_object_asteroid_reserves_deserialize() {
    use crate::api::types::SectorObject;
    let ast: SectorObject = serde_json::from_str(r#"{
        "type": "asteroid",
        "name": "AX-12",
        "summary": "carbonaceous",
        "composition": "carbonaceous",
        "resourceAmounts": {"deuterium": 0.0, "metals": 1.25, "ice": 0.0, "carbon_compounds": 3.5}
    }"#).unwrap();
    assert_eq!(ast.composition.as_deref(), Some("carbonaceous"));
    let amt = ast.resource_amounts.expect("amounts");
    assert!((amt.carbon_compounds - 3.5).abs() < 1e-9);
    // Absent fields stay None / empty.
    assert!(ast.category.is_none() && ast.resource_types.is_empty());
}

#[test]
fn scanner_objects_number_unnamed_by_type() {
    let mut state = AppState::default();
    state.probe = Some(probe_at(0., 0., 0.));
    // Two unnamed top-level asteroids + one named planet.
    state.scan_history = vec![make_sector_with_objects(0., 0., 0., r#"[
        {"type": "asteroid", "id": "a1", "name": null, "summary": ""},
        {"type": "planet", "id": "p1", "name": "Vulcan", "summary": ""},
        {"type": "asteroid", "id": "a2", "name": null, "summary": ""}
    ]"#)];
    let entries = state.scanner_objects();
    let by_id = |id: &str| entries.iter().find(|e| e.id == id).unwrap().name.clone();
    assert_eq!(by_id("a1"), "asteroid #1");
    assert_eq!(by_id("a2"), "asteroid #2");
    assert_eq!(by_id("p1"), "Vulcan"); // real names kept
}

// ── mining target reserves ────────────────────────────────────────────────

#[test]
fn minable_target_reserves_reads_types_and_amounts() {
    let mut state = AppState::default();
    state.scan_history = vec![make_sector_with_objects(0., 0., 0., r#"[
        {"type": "asteroid", "id": "ast-1", "name": "AX", "summary": "",
         "resourceTypes": ["metals", "ice"],
         "resourceAmounts": {"deuterium": 0.0, "metals": 2.0, "ice": 1.0, "carbon_compounds": 0.0}}
    ]"#)];
    let (flags, res) = state.minable_target_reserves("ast-1").expect("reserves");
    assert_eq!(flags, [false, true, true, false]);
    assert_eq!(res, [0.0, 2.0, 1.0, 0.0]);
    // Sum of selected present reserves (metals+ice).
    assert_eq!(state.mine_reserve_max("ast-1", [false, true, true, false]), 3.0);
    // Unknown object → None.
    assert!(state.minable_target_reserves("nope").is_none());
}

#[test]
fn mine_reserve_max_falls_back_to_free_capacity_without_reserves() {
    let mut state = AppState::default();
    state.probe = Some(make_probe(0.5, 0., 0., 0.));
    state.scan_history = vec![make_sector_with_objects(0., 0., 0., r#"[
        {"type": "asteroid", "id": "ast-1", "name": "AX", "summary": "", "resourceTypes": ["metals"]}
    ]"#)];
    // No resourceAmounts → reserves are 0 → fall back to free capacity (0.5).
    assert_eq!(state.mine_reserve_max("ast-1", [false, true, false, false]), 0.5);
}

// ── map & travel context menus ────────────────────────────────────────────

#[test]
fn map_context_menu_goto_disabled_without_visited() {
    let mut state = AppState::default();
    state.active_pane = Pane::Map;
    let menu = state.build_context_menu().expect("map menu");
    assert_eq!(menu.items.len(), 4);
    let goto = menu.items.iter().find(|i| i.action == MenuAction::GotoVisited).unwrap();
    assert!(!goto.enabled, "no visited sectors → jump disabled");
    // Open map / travel are always available.
    assert!(menu.items.iter().find(|i| i.action == MenuAction::Travel).unwrap().enabled);
}

#[test]
fn scanner_travel_here_enabled_for_remote_selection_only() {
    let mut state = AppState::default();
    state.active_pane = Pane::Scanner;
    state.probe = Some(probe_at(0., 0., 0.));
    // Remote observation selected → Travel here enabled.
    state.scan_history = vec![make_sector(2., 0., 0.)];
    let travel = state.build_context_menu().unwrap().items.into_iter()
        .find(|i| i.action == MenuAction::ScanTravel).unwrap();
    assert!(travel.enabled);
    // Current sector selected → disabled ("already here").
    state.scan_history = vec![make_sector(0., 0., 0.)];
    let travel = state.build_context_menu().unwrap().items.into_iter()
        .find(|i| i.action == MenuAction::ScanTravel).unwrap();
    assert!(!travel.enabled);
}

#[test]
fn map_menu_has_waypoints_disabled_when_empty() {
    let mut state = AppState::default();
    state.active_pane = Pane::Map;
    let menu = state.build_context_menu().expect("map menu");
    assert_eq!(menu.items.len(), 4);
    let wp = menu.items.iter().find(|i| i.action == MenuAction::Waypoints).unwrap();
    assert!(!wp.enabled, "no waypoints → disabled");
}

// ── recipe affordability ──────────────────────────────────────────────────

#[test]
fn recipe_affordable_checks_stocks_and_items() {
    use crate::api::types::CraftingRecipe;
    let mut state = AppState::default();
    state.probe = Some(probe_with_inventory(
        r#"[{"id":"i1","type":"integrated_circuit","name":"IC","containerSpace":0.0,"taskProgressPercent":0.0}]"#,
        r#"[{"id":"s1","type":"metals","name":"Metals","amount":5.0,"containerSpace":0.0}]"#,
    ));
    let recipe: CraftingRecipe = serde_json::from_str(r#"{
        "id":"r","name":"Steel plate","craftableBy":["manny"],
        "ingredients":[{"type":"metals","quantity":2.0,"unit":"ece","kind":null}],
        "durationSeconds":600,
        "output":{"type":"steel_plate","name":"Steel plate","containerSpace":0.1,"containerSpaceUnit":"ece","capacityBonus":null}
    }"#).unwrap();
    assert!(state.recipe_affordable(&recipe), "have 5.0 metals ≥ 2.0");

    // Needs 2 integrated circuits but only 1 on hand.
    let hungry: CraftingRecipe = serde_json::from_str(r#"{
        "id":"r2","name":"Board","craftableBy":["manny"],
        "ingredients":[{"type":"integrated_circuit","quantity":2.0,"unit":"item","kind":null}],
        "durationSeconds":600,
        "output":{"type":"board","name":"Board","containerSpace":0.1,"containerSpaceUnit":"ece","capacityBonus":null}
    }"#).unwrap();
    assert!(!state.recipe_affordable(&hungry));
    assert_eq!(state.recipe_ingredient_have(&hungry.ingredients[0]), 1.0);
}

// ── command mode (:) ──────────────────────────────────────────────────────

#[test]
fn run_command_focus_zoom_theme_filter() {
    let mut state = AppState::default();
    assert!(!state.run_command("focus mannies"));
    assert_eq!(state.active_pane, Pane::Mannies);
    assert!(state.zoomed);

    state.run_command("zoom"); // toggles off
    assert!(!state.zoomed);

    state.run_command("theme mono-amber");
    assert_eq!(state.color_mode, ColorMode::MonoAmber);

    state.run_command("filter minable");
    assert_eq!(state.scan_filter, ScanFilter::Minable);
}

#[test]
fn run_command_refresh_signals_fetch() {
    let mut state = AppState::default();
    assert!(state.run_command("refresh"), "refresh asks the caller to fetch_all");
    assert!(!state.run_command("zoom"), "other commands do not");
}

#[test]
fn run_command_travel_and_goto() {
    let mut state = AppState::default();
    state.probe = Some(probe_at(0., 0., 0.));
    // even-sum target → travel confirm
    state.run_command("travel 2 0 -2");
    assert!(matches!(state.travel, TravelInput::Confirming { x: 2, y: 0, z: -2, .. }));

    state.run_command("goto 1 1 0");
    assert!(state.map.open);
    assert_eq!((state.map.center_x, state.map.y_layer, state.map.center_z), (1, 1, 0));
}

#[test]
fn run_command_unknown_sets_toast() {
    let mut state = AppState::default();
    assert!(!state.run_command("frobnicate"));
    assert!(state.active_toast().is_some());
}
