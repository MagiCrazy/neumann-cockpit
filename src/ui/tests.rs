//! Render-level regression tests (issue #214).
//!
//! `ratatui::TestBackend` renders whole surfaces against a fixed `AppState`
//! fixture; assertions target buffer *content* (text that must appear) and
//! *cell styling* (gauge coloring) rather than full-buffer snapshots — a
//! regression net on layout math and coloring that survives cosmetic churn.

use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

use crate::api::types::Probe;
use crate::app::{
    ActiveWizard, AppState, ColorMode, ContainerRulesInput, DetachInput, Pane, Polarity, TransferProbeInput,
};
use crate::ui::theme::{palette, ratio_color};

/// Flatten a rendered buffer to text, one line per row, for `contains` checks.
fn buffer_text(buf: &Buffer) -> String {
    let area = buf.area;
    let mut out = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

/// Render the whole cockpit at a given terminal size and return the buffer.
fn render_cockpit(state: &AppState, w: u16, h: u16) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    term.draw(|f| crate::ui::render(f, state)).unwrap();
    term.backend().buffer().clone()
}

/// A probe fixture with a chosen deuterium level (tank max 100), 80 % integrity,
/// and light cargo — enough to exercise the vital gauges.
fn probe(deuterium: f64) -> Probe {
    serde_json::from_str(&format!(
        r#"{{
        "id": 1, "name": "t", "status": "idle",
        "fuel": {{"deuterium": {deuterium}, "maxDeuterium": 100.0}}, "sensorMode": "normal",
        "sector": null, "movement": null, "systems": {{"integrityPercent": 80.0}},
        "inventory": {{"capacity": 10.0, "usedCapacity": 2.0, "freeCapacity": 8.0,
            "items": [], "resourceStocks": [], "externalTanks": [], "containers": []}}
    }}"#
    ))
    .unwrap()
}

#[test]
fn grid_renders_at_three_sizes_without_panicking() {
    let mut state = AppState::default();
    state.active_pane = Pane::Probe;
    // Large: the full 3×3 grid — several pane titles are present.
    let large = buffer_text(&render_cockpit(&state, 120, 40));
    for title in ["PROBE", "SCANNER", "MANNIES", "MAP"] {
        assert!(large.contains(title), "large grid should show {title}");
    }
    // Medium half-screen and a tiny split: no panic, and the active pane shows.
    let medium = buffer_text(&render_cockpit(&state, 60, 24));
    assert!(medium.contains("PROBE"), "active pane visible when the grid shrinks");
    // Tiny: the responsive window narrows to the active pane; must not panic.
    let _tiny = render_cockpit(&state, 24, 8);
}

#[test]
fn probe_gauge_color_tracks_the_fuel_ratio() {
    // Semantic palette so good (>50 %) and crit (<25 %) are distinct colours.
    let mode = ColorMode::PhosphorSemantic;
    let p = palette(mode, Polarity::Dark);

    let fill_color = |deuterium: f64| -> ratatui::style::Color {
        let mut state = AppState::default();
        state.color_mode = mode;
        state.probe = Some(probe(deuterium));
        let mut term = Terminal::new(TestBackend::new(48, 20)).unwrap();
        term.draw(|f| {
            let area = f.area();
            crate::ui::panels::probe::render_probe_panel(f, area, &state, true);
        })
        .unwrap();
        let buf = term.backend().buffer().clone();
        gauge_fill_color(&buf, "FUEL").expect("FUEL gauge rendered with a filled cell")
    };

    // Low fuel → crit; full fuel → good; and each matches ratio_color exactly.
    let low = fill_color(10.0);
    let full = fill_color(100.0);
    assert_eq!(low, ratio_color(0.1, p), "low-fuel gauge uses the crit colour");
    assert_eq!(full, ratio_color(1.0, p), "full-fuel gauge uses the good colour");
    assert_ne!(low, full, "gauge colour must change with the ratio");
}

/// The foreground colour of the first filled gauge glyph (`▓`) on the row
/// carrying `label`, or `None` if the gauge is not present/filled.
fn gauge_fill_color(buf: &Buffer, label: &str) -> Option<ratatui::style::Color> {
    let area = buf.area;
    for y in 0..area.height {
        let row: String = (0..area.width).map(|x| buf[(x, y)].symbol()).collect();
        if !row.contains(label) {
            continue;
        }
        for x in 0..area.width {
            let cell = &buf[(x, y)];
            if cell.symbol() == "▓" {
                return Some(cell.fg);
            }
        }
    }
    None
}

#[test]
fn container_rules_overlay_shows_directional_wording() {
    // The routing-rules editor legend must read directionally (issue #234);
    // this pins that wording at the render level.
    let mut state = AppState::default();
    state.active_wizard = ActiveWizard::ContainerRules(ContainerRulesInput::Editing {
        container_id: "c1".into(),
        container_label: "hold".into(),
        types: vec!["metals".into(), "ice".into()],
        priority: vec!["ice".into()],
        exclusion: vec![],
        strict_exclusion: vec!["metals".into()],
        selection: 0,
        error: None,
    });
    let text = buffer_text(&render_cockpit(&state, 90, 24));
    assert!(text.contains("prefer here"), "legend spells out [P]");
    assert!(
        text.contains("never here"),
        "legend spells out [S] as exclusion, not whitelist"
    );
    assert!(
        text.contains("never placed here"),
        "per-type effect shown in plain language"
    );
}

#[test]
fn transfer_probe_overlay_lists_targets() {
    // Manny transfer wizard (API v93): the picker shows the title and the
    // candidate destination probes.
    let mut state = AppState::default();
    state.active_wizard = ActiveWizard::TransferProbe(TransferProbeInput::PickTarget {
        manny_id: "m1".into(),
        manny_name: "Grey Area".into(),
        targets: vec![(2, "Falling Outside".into()), (3, "Sleeper Service".into())],
        selection: 0,
        error: None,
    });
    let text = buffer_text(&render_cockpit(&state, 90, 24));
    assert!(text.contains("TRANSFER MANNY"), "overlay titled");
    assert!(text.contains("Grey Area"), "source manny named");
    assert!(
        text.contains("Falling Outside") && text.contains("Sleeper Service"),
        "targets listed"
    );
}

#[test]
fn sector_object_zoom_shows_asteroid_id() {
    // The id must appear in the zoomed (non-compact) asteroid detail so it can
    // be copied into a script's `at <id>` (unnamed asteroids).
    let obj: crate::api::types::SectorObject = serde_json::from_str(
        r#"{"id":"rock-abc123","type":"asteroid","name":null,
            "estimated":false,"summary":"Wandering asteroid","resourceTypes":["metals"],
            "waypointBookmarks":[],"bookmarkTargets":[]}"#,
    )
    .unwrap();
    let p = palette(ColorMode::MonoGreen, Polarity::Dark);
    let lines = crate::ui::panels::scanner::sector_object_lines(&obj, false, p);
    let text: String = lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
        .collect::<Vec<_>>()
        .join("");
    assert!(
        text.contains("id rock-abc123"),
        "zoom detail shows the asteroid id: {text}"
    );
    // Compact view must NOT carry the id line (kept for the zoom only).
    let compact: String = crate::ui::panels::scanner::sector_object_lines(&obj, true, p)
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
        .collect();
    assert!(!compact.contains("id rock-abc123"), "compact line stays terse");
}

#[test]
fn detach_attach_to_probe_overlay_lists_target_probes() {
    // attach_to_probe detach mode (API v91): the target-probe picker renders.
    let mut state = AppState::default();
    state.active_wizard = ActiveWizard::Detach(DetachInput::PickTargetProbe {
        manny_id: "m1".into(),
        manny_name: "Grey Area".into(),
        container_id: "c1".into(),
        container_name: "cargo-hold-2".into(),
        probes: vec![(2, "Falling Outside".into())],
        selection: 0,
        error: None,
    });
    let text = buffer_text(&render_cockpit(&state, 90, 24));
    assert!(text.contains("cargo-hold-2"), "container named");
    assert!(text.contains("Attach to probe"), "prompt shown");
    assert!(text.contains("Falling Outside"), "target probe listed");
}

fn recipe_json(id: &str, name: &str, by: &str, ings: &str, dur: i64) -> crate::api::types::CraftingRecipe {
    serde_json::from_str(&format!(
        r#"{{"id":"{id}","name":"{name}","craftableBy":["{by}"],"ingredients":[{ings}],
        "durationSeconds":{dur},
        "output":{{"type":"{id}","name":"{name}","containerSpace":1.0,"containerSpaceUnit":"ECE","capacityBonus":null}}}}"#
    ))
    .unwrap()
}

#[test]
fn tree_overlay_renders_catalog_and_rollup() {
    let mut state = AppState::default();
    state.recipes = vec![
        recipe_json(
            "steel_plate",
            "Steel plate",
            "manny",
            r#"{"type":"metals","quantity":0.02,"unit":"earth_container_equivalent","kind":null}"#,
            300,
        ),
        recipe_json(
            "linear_actuator",
            "Linear actuator",
            "manny",
            r#"{"type":"steel_plate","quantity":2,"unit":"item","kind":null}"#,
            1200,
        ),
    ];
    state.open_tree();

    let text = buffer_text(&render_cockpit(&state, 90, 24));
    assert!(text.contains("TECH TREE"), "overlay title");
    assert!(text.contains("MANNY BAY"), "fabricator section header");
    assert!(text.contains("Steel plate"), "recipe listed");
    assert!(text.contains("ROLLED UP TO BASE"), "detail rollup panel");
    assert!(text.contains("metals"), "base resource shown");
}

#[test]
fn tree_overlay_expands_into_ingredients() {
    let mut state = AppState::default();
    state.recipes = vec![
        recipe_json(
            "steel_plate",
            "Steel plate",
            "manny",
            r#"{"type":"metals","quantity":0.02,"unit":"earth_container_equivalent","kind":null}"#,
            300,
        ),
        recipe_json(
            "linear_actuator",
            "Linear actuator",
            "manny",
            r#"{"type":"steel_plate","quantity":2,"unit":"item","kind":null}"#,
            1200,
        ),
    ];
    state.open_tree();
    // Land on the linear_actuator root and expand it.
    while state.tree_selected_item().as_deref() != Some("linear_actuator") {
        state.tree_move(1);
    }
    state.tree_expand();

    let rows = state.tree_rows();
    assert!(
        rows.iter().any(|r| r.item == "steel_plate" && r.depth == 1),
        "steel_plate appears indented under linear_actuator"
    );
}

#[test]
fn tree_overlay_shows_improvement_section() {
    let mut state = AppState::default();
    state.recipes = vec![recipe_json(
        "steel_plate",
        "Steel plate",
        "manny",
        r#"{"type":"metals","quantity":0.02,"unit":"earth_container_equivalent","kind":null}"#,
        300,
    )];
    state.tree_improvements = vec![serde_json::from_str(
        r#"{"id":"deuterium_compression","name":"Deuterium compression",
        "description":"Bigger tank.","available":true,"done":false,"durationSeconds":300,
        "ingredients":[{"type":"steel_plate","quantity":2,"unit":"item","kind":"item"}],"effects":null}"#,
    )
    .unwrap()];
    state.open_tree();

    let text = buffer_text(&render_cockpit(&state, 90, 24));
    assert!(text.contains("PROBE IMPROVEMENTS"), "improvement section header");
    assert!(text.contains("Deuterium compression"), "improvement listed");
}

#[test]
fn rate_limit_chip_shows_the_remaining_backoff() {
    let mut state = AppState::default();
    assert!(
        !buffer_text(&render_cockpit(&state, 120, 30)).contains("rate limit"),
        "no chip while the quota is healthy"
    );

    // A 429 back-off is mirrored from the client each tick (API v104).
    state.rate_limited_secs = Some(47);
    let text = buffer_text(&render_cockpit(&state, 120, 30));
    assert!(
        text.contains("rate limit 47s"),
        "the status bar should count the back-off down, got:\n{text}"
    );
}

#[test]
fn assembly_wizard_shows_the_per_model_bill() {
    use crate::api::types::ProbeModel;

    let mut state = AppState::default();
    state.active_wizard = ActiveWizard::AssembleProbe(crate::app::AssembleProbeInput::PickModel {
        manny_id: "m1".into(),
        manny_name: "Alpha".into(),
        containers: vec![("c1".into(), "Box 1".into()), ("c2".into(), "Box 2".into())],
        cursor: 0,
    });
    let generic = buffer_text(&render_cockpit(&state, 100, 30));
    assert!(generic.contains("deuterium tanker"), "both models are offered");
    assert!(generic.contains("solar panel"), "the generic bill is shown");
    assert!(
        !generic.contains("linear actuator"),
        "the tanker extras are not part of the generic bill"
    );

    // Selecting the tanker swaps in its heavier bill.
    state.active_wizard = ActiveWizard::AssembleProbe(crate::app::AssembleProbeInput::PickModel {
        manny_id: "m1".into(),
        manny_name: "Alpha".into(),
        containers: vec![("c1".into(), "Box 1".into()), ("c2".into(), "Box 2".into())],
        cursor: 1,
    });
    let tanker = buffer_text(&render_cockpit(&state, 100, 30));
    assert!(tanker.contains("linear actuator"), "tanker extras listed");
    assert!(tanker.contains("400-point"), "the bigger tank is the reason to pick it");

    // The container step keeps the chosen model and its bill in view.
    state.active_wizard = ActiveWizard::AssembleProbe(crate::app::AssembleProbeInput::PickContainers {
        manny_id: "m1".into(),
        manny_name: "Alpha".into(),
        model: ProbeModel::DeuteriumTanker,
        containers: vec![("c1".into(), "Box 1".into()), ("c2".into(), "Box 2".into())],
        selected: vec![0],
        cursor: 0,
        error: None,
    });
    let step2 = buffer_text(&render_cockpit(&state, 100, 30));
    assert!(step2.contains("deuterium tanker"), "model recalled in step two");
    assert!(step2.contains("steel plate"), "tanker bill still in view");
}

#[test]
fn fleet_roster_flags_a_tanker() {
    let mut state = AppState::default();
    state.probe = Some(probe(80.0));
    state.zoomed = true;
    state.active_pane = crate::app::Pane::Probe;
    state.fleet = vec![
        serde_json::from_str(
            r#"{"id":5,"name":"Main","model":"generic","status":"idle",
                "isDefault":true,"isReachable":true}"#,
        )
        .unwrap(),
        serde_json::from_str(
            r#"{"id":9,"name":"Bunker","model":"deuterium_tanker","status":"idle",
                "isDefault":false,"isReachable":true}"#,
        )
        .unwrap(),
    ];
    let text = buffer_text(&render_cockpit(&state, 120, 40));
    assert!(text.contains("Bunker"), "roster lists the drone");
    assert!(text.contains("tanker"), "a tanker is flagged in the roster");
}

#[test]
fn inventory_pane_scrolls_to_the_selected_stock() {
    // Issue #292: the pane laid its rows out as fixed 1-row rects, so anything
    // past the pane height got a zero-height rect and vanished — the ▶ cursor
    // included, which is exactly how a stock the pilot could see in the web UI
    // was missing from the cockpit.
    let mut state = AppState::default();
    let stocks: Vec<String> = (0..24)
        .map(|i| {
            format!(
                r#"{{"id": "s{i:02}", "type": "res{i:02}", "name": "res{i:02}",
                     "amount": {i}.0, "containerSpace": 1.0}}"#
            )
        })
        .collect();
    state.probe = Some(
        serde_json::from_str(&format!(
            r#"{{
        "id": 1, "name": "t", "status": "idle",
        "fuel": {{"deuterium": 50.0, "maxDeuterium": 100.0}}, "sensorMode": "normal",
        "sector": null, "movement": null, "systems": {{"integrityPercent": 80.0}},
        "inventory": {{"capacity": 10.0, "usedCapacity": 2.0, "freeCapacity": 8.0,
            "items": [], "resourceStocks": [{}], "externalTanks": [], "containers": []}}
    }}"#,
            stocks.join(", ")
        ))
        .unwrap(),
    );
    // Cursor on the last stock, in a pane far too short to hold 24 of them.
    state.inventory_selection = 23;
    let mut term = Terminal::new(TestBackend::new(48, 12)).unwrap();
    term.draw(|f| {
        let area = f.area();
        crate::ui::panels::inventory::render_inventory_panel(f, area, &state, true);
    })
    .unwrap();
    let text = buffer_text(term.backend().buffer());
    assert!(text.contains("▶ "), "the cursor is drawn somewhere on screen");
    assert!(text.contains("res23"), "the selected stock is in view");
    assert!(!text.contains("res00"), "the top of the list scrolled away");
}

#[test]
fn zoomed_storage_shows_the_last_containers_free_line() {
    // Issue #293: in zoom a container is a block (header + free capacity), but
    // the scroller anchored the *header* to the bottom row, so the free line of
    // the last container fell one row past the edge.
    let mut state = AppState::default();
    let containers: Vec<String> = (0..12)
        .map(|i| {
            format!(
                r#"{{"id": "c{i:02}", "kind": "storage", "label": "Container {i:02}",
                     "sortOrder": {i}, "capacity": 100.0, "usedCapacity": {i}.0,
                     "freeCapacity": 9{i}.0, "rules": {{}}}}"#
            )
        })
        .collect();
    state.probe = Some(
        serde_json::from_str(&format!(
            r#"{{
        "id": 1, "name": "t", "status": "idle",
        "fuel": {{"deuterium": 50.0, "maxDeuterium": 100.0}}, "sensorMode": "normal",
        "sector": null, "movement": null, "systems": {{"integrityPercent": 80.0}},
        "inventory": {{"capacity": 10.0, "usedCapacity": 2.0, "freeCapacity": 8.0,
            "items": [], "resourceStocks": [], "externalTanks": [], "containers": [{}]}}
    }}"#,
            containers.join(", ")
        ))
        .unwrap(),
    );
    state.active_pane = Pane::Storage;
    state.zoomed = true;
    // Cursor on the last container — the reported case.
    state.pane_nav[Pane::Storage.index()].cursor = 11;

    let text = buffer_text(&render_cockpit(&state, 80, 20));
    assert!(text.contains("Container 11"), "the selected container is in view");
    assert!(text.contains("free 911.00"), "and so is its free-capacity line: {text}");
}

#[test]
fn a_motorized_asteroid_reads_as_one() {
    // API v116: propulsion state and the live trajectory must be visible on the
    // object line — a rock aimed at the system you are sitting in cannot look
    // like an ordinary rock.
    use crate::api::types::SectorObject;
    let obj: SectorObject = serde_json::from_str(
        r#"{"id":"ast-1","type":"asteroid","name":"Metal 8f1a","motorized":true,
            "motorFuelStatus":"empty","distinctiveFeature":"Sculpted in the shape of a duck",
            "trajectory":{"id":"atr_1","asteroidId":"ast-1","mode":"system_impact",
                "status":"accelerating","startedAt":null,"nextTransitionAt":null,
                "targetObjectId":"planet-3","currentSpeedC":0.25,
                "plannedRevolutions":3,"completedRevolutions":1}}"#,
    )
    .unwrap();
    let p = palette(ColorMode::MonoGreen, Polarity::Dark);

    let compact: String = crate::ui::panels::scanner::sector_object_lines(&obj, true, p)
        .iter()
        .flat_map(|l| l.spans.iter())
        .map(|s| s.content.as_ref())
        .collect();
    assert!(compact.contains("motorized"), "propulsion marker: {compact}");
    assert!(compact.contains("(dry)"), "an empty engine is worth knowing about");
    assert!(compact.contains("impact: accelerating"), "mode and status: {compact}");
    assert!(compact.contains("0.25c"), "current speed");

    let zoomed: String = crate::ui::panels::scanner::sector_object_lines(&obj, false, p)
        .iter()
        .flat_map(|l| l.spans.iter())
        .map(|s| s.content.as_ref())
        .collect();
    assert!(zoomed.contains("→ planet-3"), "the target, in zoom: {zoomed}");
    assert!(zoomed.contains("rev 1/3"), "revolutions, in zoom");
    assert!(
        zoomed.contains("Sculpted in the shape of a duck"),
        "the feature is quoted verbatim"
    );
}

#[test]
fn the_travel_confirm_marks_a_safe_corridor_without_overselling_it() {
    // Issue #257: the marker must state what is waived (destruction) *and* what
    // is not (container detachment). A pilot reading "safe" as "free" is exactly
    // the failure this feature could cause.
    use crate::app::TravelInput;
    let mut state = AppState::default();
    state.probe = Some(probe(50.0));
    if let Some(pr) = state.probe.as_mut() {
        pr.sector = serde_json::from_str(r#"{"relative":{"x":0,"y":0,"z":0}}"#).unwrap();
    }
    state.scan_history = vec![serde_json::from_str(
        r#"{"relativeCoordinates":{"x":0,"y":0,"z":0},"distance":0,
            "knowledgeLevel":"detailed","confidence":1.0,
            "objects":[{"type":"scut_relay","id":"r1","name":"Alpha relay","status":"on",
                        "isTransitBeacon":true,"network":{"id":7,"name":"Alpha"}}],
            "scan":{"currentSectorResidenceSeconds":60,"requiredResidenceSeconds":60,"scanQuality":1.0}}"#,
    )
    .unwrap()];
    state.scut_network_view = Some(
        serde_json::from_str(
            r#"{"id":7,"name":"Alpha","relayCount":2,"coveredSectorCount":4,
            "relays":[{"id":2,"name":"Beta relay","status":"on","isTransitBeacon":true,
                       "sector":{"relative":{"x":3,"y":0,"z":0}},"coverageRadiusSectors":2}],
            "probes":[]}"#,
        )
        .unwrap(),
    );
    state.active_wizard = ActiveWizard::Travel(TravelInput::Confirming {
        x: 3,
        y: 0,
        z: 0,
        sector_distance: Some(3),
        fuel_cost: Some(0.3),
        eta_minutes: Some(12),
        error: None,
    });
    let text = buffer_text(&render_cockpit(&state, 100, 30));
    assert!(text.contains("safe corridor"), "the corridor is announced: {text}");
    assert!(text.contains("no destruction risk"), "what it waives");
    assert!(text.contains("containers can still detach"), "what it does not");

    // A destination that is not a corridor says nothing at all.
    state.active_wizard = ActiveWizard::Travel(TravelInput::Confirming {
        x: 9,
        y: 9,
        z: 9,
        sector_distance: Some(27),
        fuel_cost: Some(2.7),
        eta_minutes: Some(90),
        error: None,
    });
    let text = buffer_text(&render_cockpit(&state, 100, 30));
    assert!(!text.contains("safe corridor"), "no claim on an ordinary jump");
}

#[test]
fn the_menu_cursor_is_visible_on_a_disabled_row() {
    // Reverse video alone lost the cursor on a dimmed row, and a disabled row
    // never got it at all (issue #325): column 1 now carries it explicitly.
    use crate::app::{ContextMenu, InputMode, MenuAction, MenuItem};
    let mut state = AppState::default();
    state.mode = InputMode::Menu(ContextMenu {
        title: "TEST".into(),
        items: vec![
            MenuItem {
                action: MenuAction::Travel,
                label: "Travel…".into(),
                enabled: false,
                disabled_reason: Some("no fuel".into()),
            },
            MenuItem {
                action: MenuAction::Mine,
                label: "Mine…".into(),
                enabled: true,
                disabled_reason: None,
            },
        ],
        cursor: 0,
    });

    let text = buffer_text(&render_cockpit(&state, 80, 24));
    let row = text
        .lines()
        .find(|l| l.contains("Travel…"))
        .expect("the disabled item is rendered");
    assert!(row.contains("▶"), "the cursor marks the disabled row it sits on: {row}");
    assert!(row.contains("no fuel"), "and the row still states why: {row}");

    let other = text
        .lines()
        .find(|l| l.contains("Mine…"))
        .expect("the enabled item is rendered");
    assert!(!other.contains("▶"), "only the cursor row is marked: {other}");
}

// ── overflow markers (issue #326) ─────────────────────────────────────────

/// Render just the markers over a pane-sized rect and return the buffer.
fn markers_buffer(offset: u16, total: usize, w: u16, h: u16) -> Buffer {
    use crate::ui::theme::{pane_block, scroll_markers};
    let p = palette(ColorMode::MonoGreen, Polarity::Dark);
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    term.draw(|f| {
        let area = f.area();
        f.render_widget(pane_block(" TEST ", true, p), area);
        scroll_markers(f, area, offset, total, true, p);
    })
    .unwrap();
    term.backend().buffer().clone()
}

#[test]
fn overflow_markers_sit_on_the_border_and_only_when_needed() {
    let (w, h) = (12u16, 6u16);
    let at = |buf: &Buffer, y: u16| buf[(w - 1, y)].symbol().to_string();

    // Viewport is 4 rows (h minus the two border rows) out of 20 lines.
    let top = markers_buffer(0, 20, w, h);
    assert_eq!(at(&top, h - 2), "▼", "more below → marker above the bottom corner");
    assert_ne!(at(&top, 1), "▲", "nothing above yet");

    let middle = markers_buffer(5, 20, w, h);
    assert_eq!(at(&middle, 1), "▲", "scrolled → marker below the top corner");
    assert_eq!(at(&middle, h - 2), "▼");

    let bottom = markers_buffer(16, 20, w, h);
    assert_eq!(at(&bottom, 1), "▲");
    assert_ne!(at(&bottom, h - 2), "▼", "at the end, nothing more below");

    let fits = markers_buffer(0, 3, w, h);
    assert_ne!(at(&fits, 1), "▲", "a list that fits cries no wolf");
    assert_ne!(at(&fits, h - 2), "▼");
}

#[test]
fn a_pane_whose_list_overflows_says_so() {
    // End to end: a roster taller than its pane marks its own frame.
    let mut state = AppState::default();
    state.active_pane = Pane::Mannies;
    state.mannies = Some(
        (0..40)
            .map(|i| {
                serde_json::from_str(&format!(
                    r#"{{"id": "m{i}", "name": "m{i}",
                        "location": {{"type": "probe", "sector": null}},
                        "currentTask": null, "taskProgressPercent": 0.0,
                        "cargo": {{"capacity": 0.3, "deuterium": 0.0, "metals": 0.0,
                                   "ice": 0.0, "organicCompounds": 0.0}},
                        "canReceiveOrders": true, "taskEstimatedEndTime": null}}"#
                ))
                .unwrap()
            })
            .collect(),
    );

    let text = buffer_text(&render_cockpit(&state, 80, 24));
    assert!(text.contains("▼"), "the pane advertises that its list continues");
}

// ── drilled-in detail scrolling (issue #337) ──────────────────────────────

/// A Manny mining a long-named asteroid into a long-named container: the
/// detail block is taller than a 1/3 grid cell.
fn busy_manny() -> crate::api::types::Manny {
    serde_json::from_str(
        r#"{"id": "m1", "name": "Falling Outside The Normal Moral Constraints",
            "location": {"type": "sector", "sector": null},
            "currentTask": "mining", "taskProgressPercent": 42.0,
            "cargo": {"capacity": 0.3, "deuterium": 0.05, "metals": 0.12,
                      "ice": 0.01, "organicCompounds": 0.0},
            "canReceiveOrders": false, "taskEstimatedEndTime": null}"#,
    )
    .unwrap()
}

fn drilled_into_a_manny() -> AppState {
    let mut state = AppState::default();
    state.active_pane = Pane::Mannies;
    state.mannies = Some(vec![busy_manny()]);
    state.pane_drill_in();
    state
}

#[test]
fn the_manny_detail_scrolls_to_its_last_line() {
    // Its tail — the cargo figures — used to be unreachable: the view had no
    // viewport at all, and the pane cursor is frozen while drilled in (#337).
    let mut state = drilled_into_a_manny();
    assert!(state.detail_view_active(), "drilled into a Manny is a detail view");

    // A pane too short for the block: the tail falls outside the frame.
    let (w, h) = (40, 8);
    let top = buffer_text(&render_cockpit(&state, w, h));
    assert!(top.contains("mining"), "the head of the block is on screen");
    assert!(!top.contains("deut"), "and its tail is not: {top}");
    assert!(top.contains("▼"), "the pane says the block continues below: {top}");

    state.set_detail_scroll(20); // past the end; the renderer clamps
    let bottom = buffer_text(&render_cockpit(&state, w, h));
    assert!(bottom.contains("deut"), "the cargo tail is now reachable: {bottom}");
    assert!(bottom.contains("▲"), "and the pane says the block continues above");
}

#[test]
fn leaving_the_detail_forgets_its_scroll() {
    let mut state = drilled_into_a_manny();
    state.set_detail_scroll(4);
    state.pane_drill_out();
    assert_eq!(
        state.pane_nav[Pane::Mannies.index()].detail_scroll,
        0,
        "a fresh drill-in starts at the top"
    );
    assert!(!state.detail_view_active());
}

// ── rename prefill (issue #330) ───────────────────────────────────────────

#[test]
fn a_rename_opens_on_the_current_name_and_del_clears_it() {
    use crate::app::{RenameMannyInput, RenameProbeInput};

    // Manny: the wizard is prefilled, not seeded with a random suggestion.
    let mut state = AppState::default();
    state.active_wizard = ActiveWizard::RenameManny(RenameMannyInput::Typing {
        manny_id: "m1".into(),
        manny_name: "Grey Area".into(),
        buf: "Grey Area".into(),
        error: None,
    });
    let text = buffer_text(&render_cockpit(&state, 80, 24));
    assert!(text.contains("Grey Area"), "the field carries the current name");
    assert!(text.contains("[Del] clear"), "and says how to empty it: {text}");

    state.rename_manny_clear();
    let ActiveWizard::RenameManny(RenameMannyInput::Typing { buf, .. }) = &state.active_wizard else {
        panic!("wizard closed");
    };
    assert!(buf.is_empty(), "Del empties the field outright");

    // Probe: the suggestion stays one Tab away, so the ceremony is not lost.
    let mut state = AppState::default();
    state.active_wizard = ActiveWizard::RenameProbe(RenameProbeInput::Typing {
        probe_id: 1,
        current_name: "Sleeper Service".into(),
        buf: "Sleeper Service".into(),
        error: None,
    });
    let text = buffer_text(&render_cockpit(&state, 80, 24));
    assert!(text.contains("Sleeper Service"));
    assert!(text.contains("[Tab] suggest"));
}

// ── rate-limit quota chip (issue #332) ────────────────────────────────────

#[test]
fn the_quota_chip_stays_quiet_until_the_window_is_half_spent() {
    let mut state = AppState::default();

    state.rate_limit_quota = Some((119, 120));
    assert!(
        state.quota_chip().is_none(),
        "a healthy window says nothing — a permanent chip is noise"
    );
    assert!(!buffer_text(&render_cockpit(&state, 100, 24)).contains("quota"));

    state.rate_limit_quota = Some((50, 120));
    let (label, urgent) = state.quota_chip().expect("half spent → visible");
    assert_eq!(label, "⏳ quota 50/120");
    assert!(!urgent, "half a window left is information, not an alarm");
    assert!(buffer_text(&render_cockpit(&state, 100, 24)).contains("quota 50/120"));

    state.rate_limit_quota = Some((12, 120));
    let (_, urgent) = state.quota_chip().expect("nearly spent → visible");
    assert!(urgent, "the last quarter is worth an alarm");
}

#[test]
fn the_quota_chip_yields_to_the_back_off_countdown() {
    // Both would say the same thing, and `remaining` is 0 by construction
    // while a 429 back-off is in force.
    let mut state = AppState::default();
    state.rate_limit_quota = Some((0, 120));
    state.rate_limited_secs = Some(4);
    assert!(state.quota_chip().is_none());

    let text = buffer_text(&render_cockpit(&state, 100, 24));
    assert!(text.contains("rate limit 4s"));
    assert!(!text.contains("quota"));
}

// -- production console (issue #328) --------------------------------------

fn console_state() -> AppState {
    let mut state = AppState::default();
    state.active_wizard = ActiveWizard::Fabrication(crate::app::FabricationInput::pick_recipe(None));
    state
}

#[test]
fn the_console_names_the_probe_whose_production_it_is() {
    // The queue belongs to the piloted probe and a switch parks it (#291), so
    // a console that did not say whose it was made a parked queue look lost.
    let mut state = console_state();
    state.probe = Some(probe(50.0));
    let text = buffer_text(&render_cockpit(&state, 110, 30));
    assert!(text.contains("PRODUCTION"), "the console is up");
    let title = text.lines().find(|l| l.contains("PRODUCTION")).unwrap();
    assert!(title.contains('\u{2014}'), "the title names a probe: {title}");
}

#[test]
fn the_console_resizes_and_stays_within_bounds() {
    use crate::app::{FAB_CONSOLE_MAX_WIDTH, FAB_CONSOLE_MIN_WIDTH, FAB_CONSOLE_WIDTH};
    let mut state = console_state();
    assert_eq!(state.fab_console_width(), FAB_CONSOLE_WIDTH);

    state.fab_console_resize(1);
    assert!(state.fab_console_width() > FAB_CONSOLE_WIDTH, "Shift-right widens it");
    for _ in 0..50 {
        state.fab_console_resize(1);
    }
    assert_eq!(state.fab_console_width(), FAB_CONSOLE_MAX_WIDTH, "and stops");
    for _ in 0..100 {
        state.fab_console_resize(-1);
    }
    assert_eq!(state.fab_console_width(), FAB_CONSOLE_MIN_WIDTH, "both ways");
}

// -- storage sort (issue #333) ---------------------------------------------

/// A probe holding three containers whose server order is not alphabetical.
fn probe_with_unsorted_containers() -> AppState {
    let mut state = AppState::default();
    state.probe = Some(
        serde_json::from_str(
            r#"{
        "id": 1, "name": "t", "status": "idle",
        "fuel": {"deuterium": 50.0, "maxDeuterium": 100.0}, "sensorMode": "normal",
        "sector": null, "movement": null, "systems": {"integrityPercent": 80.0},
        "inventory": {"capacity": 10.0, "usedCapacity": 2.0, "freeCapacity": 8.0,
            "items": [], "resourceStocks": [], "externalTanks": [], "containers": [
              {"id": "c1", "kind": "storage", "label": "Zulu", "sortOrder": 0,
               "capacity": 100.0, "usedCapacity": 1.0, "freeCapacity": 99.0, "rules": {}},
              {"id": "c2", "kind": "storage", "label": "alpha", "sortOrder": 1,
               "capacity": 100.0, "usedCapacity": 2.0, "freeCapacity": 98.0, "rules": {}},
              {"id": "c3", "kind": "storage", "label": "Mike", "sortOrder": 2,
               "capacity": 100.0, "usedCapacity": 3.0, "freeCapacity": 97.0, "rules": {}}]}
    }"#,
        )
        .unwrap(),
    );
    state.active_pane = Pane::Storage;
    state
}

#[test]
fn sorting_containers_is_case_insensitive_and_defaults_to_the_server_order() {
    let mut state = probe_with_unsorted_containers();
    let labels = |s: &AppState| {
        s.storage_containers_ordered()
            .iter()
            .map(|c| c.label.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(labels(&state), ["Zulu", "alpha", "Mike"], "server order by default");

    state.storage_toggle_sort();
    assert_eq!(
        labels(&state),
        ["alpha", "Mike", "Zulu"],
        "a-z ignores case, so a lowercase name does not sort last"
    );
    let title = buffer_text(&render_cockpit(&state, 80, 24));
    assert!(title.contains("STORAGE · a-z"), "the pane says how it is sorted");
}

#[test]
fn toggling_the_sort_keeps_the_cursor_on_the_same_container() {
    // The cursor indexes into the ordering, so re-sorting under it would
    // silently retarget every Storage action (issue #333).
    let mut state = probe_with_unsorted_containers();
    state.pane_nav[Pane::Storage.index()].cursor = 0; // "Zulu"
    assert_eq!(state.storage_selected_container_id().as_deref(), Some("c1"));

    state.storage_toggle_sort();
    assert_eq!(
        state.storage_selected_container_id().as_deref(),
        Some("c1"),
        "still on Zulu, now at the end of the list"
    );
    assert_eq!(state.pane_nav[Pane::Storage.index()].cursor, 2);
}

// -- polarity (issue #233) -------------------------------------------------

#[test]
fn the_cockpit_renders_in_every_mode_and_polarity() {
    // Fourteen palettes now reach the renderer; a mode wired into `ALL` but
    // missing a `palette` arm, or a light variant that panics, shows up here.
    for mode in ColorMode::ALL {
        for polarity in [Polarity::Dark, Polarity::Light] {
            let mut state = AppState::default();
            state.color_mode = mode;
            state.polarity = polarity;
            state.probe = Some(probe(50.0));
            let text = buffer_text(&render_cockpit(&state, 100, 30));
            assert!(
                text.contains("PROBE"),
                "{} / {} rendered nothing",
                mode.label(),
                polarity.label()
            );
        }
    }
}

#[test]
fn polarity_actually_changes_what_is_painted() {
    // The axis has to reach the buffer, not just the state: same mode, same
    // frame, different ink.
    let cell_colors = |polarity| {
        let mut state = AppState::default();
        state.polarity = polarity;
        state.probe = Some(probe(50.0));
        let buf = render_cockpit(&state, 100, 30);
        let area = buf.area;
        (0..area.height)
            .flat_map(|y| (0..area.width).map(move |x| (x, y)))
            .map(|(x, y)| buf[(x, y)].fg)
            .collect::<Vec<_>>()
    };
    assert_ne!(
        cell_colors(Polarity::Dark),
        cell_colors(Polarity::Light),
        "F3 must repaint the cockpit, not just flip a flag"
    );
}

// -- scanner detail scrolling (issue #347) ---------------------------------

/// A remote observation carrying enough objects that its detail overflows any
/// pane: `scan_detail_scroll` used to be pinned at zero, so the tail could not
/// be read at all.
fn state_with_a_long_observation() -> AppState {
    let objects: Vec<String> = (0..12)
        .map(|i| {
            format!(
                r#"{{"id":"ast-{i}","type":"asteroid","name":"Rock {i:02}",
                     "minableTargets":[{{"id":"mt-{i}","type":"asteroid",
                                         "resourceTypes":["metals"]}}]}}"#
            )
        })
        .collect();
    let mut state = AppState::default();
    state.active_pane = Pane::Scanner;
    state.scan_history = vec![serde_json::from_str(&format!(
        r#"{{"relativeCoordinates": {{"x": 8.0, "y": 0.0, "z": 0.0}}, "distance": 4,
             "knowledgeLevel": "detailed", "confidence": 1.0, "objects": [{}],
             "scan": {{"currentSectorResidenceSeconds": 60,
                       "requiredResidenceSeconds": 60, "scanQuality": 1.0}}}}"#,
        objects.join(", ")
    ))
    .unwrap()];
    state.probe = Some(probe(50.0));
    state
}

#[test]
fn the_scanner_detail_can_be_read_to_its_end() {
    use crate::app::ScannerFocus;
    use crate::ui::panels::scanner::scanner_detail_lines;

    let mut state = state_with_a_long_observation();
    let p = state.palette();
    let total = scanner_detail_lines(&state, p).len();
    assert!(total > 10, "the fixture has to overflow to prove anything: {total}");

    let (w, h) = (46, 14);
    let top = buffer_text(&render_cockpit(&state, w, h));
    assert!(top.contains("Rock 00"), "the head of the detail is on screen");
    assert!(!top.contains("Rock 11"), "and its tail is not: {top}");

    // Focus the detail column, then walk to the end.
    state.scanner_focus = ScannerFocus::Detail;
    state.scan_detail_scroll = total; // past the end; the renderer clamps
    let bottom = buffer_text(&render_cockpit(&state, w, h));
    assert!(bottom.contains("Rock 11"), "the tail is now reachable: {bottom}");
    assert!(bottom.contains("\u{25b2}"), "and the pane says it continues above");
}

// -- notification mute indicator (issue #331) ------------------------------

#[test]
fn the_status_bar_shows_whether_the_cockpit_will_beep() {
    let mut state = AppState::default();
    state.probe = Some(probe(50.0));

    state.notifications_enabled = true;
    let on = buffer_text(&render_cockpit(&state, 110, 24));
    assert!(on.contains("\u{266a}"), "the note is shown when sound is on");
    assert!(!on.contains("off"), "and nothing more: {on}");

    state.notifications_enabled = false;
    let muted = buffer_text(&render_cockpit(&state, 110, 24));
    assert!(muted.contains("\u{266a} off"), "muted says so plainly: {muted}");
}

// -- update notice (issue #339) --------------------------------------------

#[test]
fn a_newer_release_is_a_quiet_chip_and_an_older_one_is_nothing() {
    let mut state = AppState::default();
    state.probe = Some(probe(50.0));

    // Older and equal tags must not raise anything: a false "update
    // available" is worse than none.
    state.note_latest_release("neumann-cockpit-v0.0.1");
    assert_eq!(state.update_available, None);
    assert!(!buffer_text(&render_cockpit(&state, 110, 24)).contains("\u{2b06}"));

    // A tag we cannot parse is not an update either.
    state.note_latest_release("some-other-project-1.2.3");
    assert_eq!(state.update_available, None);

    state.note_latest_release("neumann-cockpit-v999.0.0");
    assert_eq!(state.update_available.as_deref(), Some("999.0.0"));
    let text = buffer_text(&render_cockpit(&state, 110, 24));
    assert!(text.contains("999.0.0"), "the version is named: {text}");
}

// -- ambiance (issues #204, #205, #206) ------------------------------------

/// Tick until a cosmic ray is actually on screen, or give up. The generator is
/// deliberately sparse, so a test that assumed one every tick would flake.
fn with_a_glitch(state: &mut AppState) -> bool {
    for _ in 0..400 {
        state.ambiance.tick(false);
        if state.ambiance.glitch_at(20).is_some() {
            return true;
        }
    }
    false
}

#[test]
fn entropy_only_ever_lands_on_the_frame() {
    // The rule the whole effect rests on: a cockpit that garbles a fuel
    // reading for texture is a cockpit you stop trusting. So a flip may touch
    // the border row and nothing else — the buffers are compared cell by cell.
    let mut state = AppState::default();
    state.probe = Some(probe(50.0));
    let clean = render_cockpit(&state, 60, 16);

    state.ambiance.enabled = true;
    assert!(
        with_a_glitch(&mut state),
        "the generator produced no entropy in 400 ticks"
    );
    let dirty = render_cockpit(&state, 60, 16);

    let area = clean.area;
    let mut changed_rows: Vec<u16> = Vec::new();
    for y in 0..area.height {
        for x in 0..area.width {
            if clean[(x, y)].symbol() != dirty[(x, y)].symbol() && !changed_rows.contains(&y) {
                changed_rows.push(y);
            }
        }
    }
    assert!(!changed_rows.is_empty(), "the glitch did reach the screen");
    // Every changed cell sits on a pane's top border, never in its content.
    for y in &changed_rows {
        for x in 0..area.width {
            if clean[(x, *y)].symbol() != dirty[(x, *y)].symbol() {
                let was = clean[(x, *y)].symbol().to_string();
                assert!(
                    was.chars().all(|c| !c.is_alphanumeric()),
                    "entropy overwrote {was:?} at ({x},{y}) — that could have been data"
                );
            }
        }
    }
}

#[test]
fn switched_off_the_cockpit_renders_exactly_as_before() {
    // The promise of the config key: opting out gives back the cockpit that
    // existed before any of this was written.
    let mut state = AppState::default();
    state.probe = Some(probe(50.0));
    let before = buffer_text(&render_cockpit(&state, 60, 16));

    state.ambiance.enabled = true;
    assert!(with_a_glitch(&mut state));
    state.ambiance.enabled = false;
    state.ambiance.tick(true);
    state.ambiance.force_idle();
    state.ambiance.tick(true);

    let after = buffer_text(&render_cockpit(&state, 60, 16));
    assert_eq!(after, before, "no entropy, and no starfield however long it idles");
}

#[test]
fn the_attract_screen_names_the_probe_and_says_how_to_leave() {
    let mut state = AppState::default();
    state.probe = Some(probe(50.0));
    state.ambiance.enabled = true;
    state.ambiance.force_idle();
    state.ambiance.tick(true);
    assert!(state.ambiance.attract);

    let text = buffer_text(&render_cockpit(&state, 74, 18));
    assert!(text.contains("any key to resume"), "the way back is on screen: {text}");
    assert!(text.contains("T"), "and the probe is named");
    assert!(!text.contains("MISSIONS"), "the grid really is covered");
}

// -- the server logbook (issue #254) ---------------------------------------

fn state_with_logbook_pages() -> AppState {
    let mut state = AppState::default();
    state.probe = Some(probe(50.0));
    state.logbook_pages = Some(vec![serde_json::from_str(
        r#"{"id": 7, "probeId": 1, "title": "Premier relais SCUT", "sortOrder": 1,
             "createdAt": "2026-09-01T10:00:00Z", "updatedAt": "2026-09-02T11:30:00Z"}"#,
    )
    .unwrap()]);
    state.active_pane = Pane::Missions;
    state.missions_enter_category(crate::app::MissionsCategory::Logbook);
    state
}

#[test]
fn the_logbook_lists_pages_and_reads_one() {
    let mut state = state_with_logbook_pages();
    let list = buffer_text(&render_cockpit(&state, 90, 24));
    assert!(list.contains("LOGBOOK"), "the half is titled: {list}");
    assert!(list.contains("Premier relais SCUT"), "the page is listed: {list}");

    // Drilled into the page, its body reads out — prose is wrapped, not cut.
    state.pane_nav[Pane::Missions.index()]
        .drill
        .push(crate::app::DrillLevel::LogbookPage(7));
    state.logbook_page = Some(
        serde_json::from_str(
            r#"{"id": 7, "probeId": 1, "title": "Premier relais SCUT", "sortOrder": 1,
                 "createdAt": "2026-09-01T10:00:00Z", "updatedAt": "2026-09-02T11:30:00Z",
                 "content": "Le relais est stable."}"#,
        )
        .unwrap(),
    );
    let page = buffer_text(&render_cockpit(&state, 90, 24));
    assert!(page.contains("Le relais est stable"), "the body is on screen: {page}");
}

#[test]
fn the_logbook_says_it_is_still_fetching_rather_than_empty() {
    // `None` and "no pages" are different answers, and showing the second for
    // the first would have the pilot writing a page they already have.
    let mut state = AppState::default();
    state.active_pane = Pane::Missions;
    state.missions_enter_category(crate::app::MissionsCategory::Logbook);
    let text = buffer_text(&render_cockpit(&state, 90, 24));
    assert!(text.contains("fetching"), "{text}");

    state.logbook_pages = Some(Vec::new());
    let empty = buffer_text(&render_cockpit(&state, 90, 24));
    assert!(empty.contains("no pages yet"), "{empty}");
}

#[test]
fn the_logbook_editor_shows_both_fields_and_its_commit_key() {
    use crate::app::LogbookInput;
    let mut state = AppState::default();
    state.active_wizard = ActiveWizard::Logbook(LogbookInput::Content {
        page_id: Some(7),
        title: "Premier relais SCUT".into(),
        content: "Le relais est stable.".into(),
        error: None,
    });
    let text = buffer_text(&render_cockpit(&state, 90, 26));
    assert!(text.contains("EDIT PAGE"), "an existing page says so: {text}");
    assert!(text.contains("Premier relais SCUT"), "the title stays in view: {text}");
    assert!(
        text.contains("Le relais est stable"),
        "and the body is editable: {text}"
    );
    // Enter is a newline in prose, so saving needs a key of its own.
    assert!(text.contains("[Ctrl-S]"), "the commit key is advertised: {text}");
}

#[test]
fn deleting_a_page_asks_first_and_says_what_survives() {
    use crate::app::LogbookInput;
    let mut state = AppState::default();
    state.active_wizard = ActiveWizard::Logbook(LogbookInput::ConfirmDelete {
        page_id: 7,
        title: "Premier relais SCUT".into(),
    });
    let text = buffer_text(&render_cockpit(&state, 90, 24));
    assert!(text.contains("DELETE PAGE"));
    assert!(text.contains("Premier relais SCUT"), "it names what goes: {text}");
    assert!(
        text.contains("ship's log is untouched"),
        "the two journals are separate, and the pilot should know it: {text}"
    );
}

#[test]
fn discarding_an_unread_alert_says_so_before_it_goes() {
    // Acknowledging and discarding are different acts, and the prompt for the
    // destructive one has to admit when nobody has read the entry (#366).
    use crate::app::DiscardCommsInput;
    let mut state = AppState::default();
    state.active_wizard = ActiveWizard::DiscardComms(DiscardCommsInput::One {
        warnings: false,
        id: 3,
        message: "hull breach on deck two".into(),
        unread: true,
    });
    let text = buffer_text(&render_cockpit(&state, 90, 24));
    assert!(text.contains("DISCARD ALERT"));
    assert!(text.contains("hull breach"), "it names what goes: {text}");
    assert!(text.contains("still unread"), "and warns it was never seen: {text}");
    assert!(
        text.contains("permanently"),
        "and that it is not an acknowledge: {text}"
    );
}

#[test]
fn a_bulk_discard_says_what_it_leaves_behind() {
    use crate::app::DiscardCommsInput;
    let mut state = AppState::default();
    state.active_wizard = ActiveWizard::DiscardComms(DiscardCommsInput::Acknowledged {
        warnings: true,
        ids: (1..=20).collect(),
        total: 34,
    });
    let text = buffer_text(&render_cockpit(&state, 90, 24));
    assert!(text.contains("DISCARD ACKNOWLEDGED WARNINGS"));
    assert!(
        text.contains("nothing unread is touched"),
        "the bound is stated: {text}"
    );
    assert!(
        text.contains("14 acknowledged remain"),
        "and so is the remainder, or the pilot thinks the list is clear: {text}"
    );
}

#[test]
fn the_missions_root_offers_the_logbook_and_admits_it_has_not_looked() {
    // `None` (never fetched) and `0` (fetched, empty) are different answers,
    // and the root must not claim the second while meaning the first (#254).
    let mut state = AppState::default();
    state.active_pane = Pane::Missions;

    let text = buffer_text(&render_cockpit(&state, 70, 20));
    assert!(text.contains("Missions"), "the three categories are listed: {text}");
    assert!(text.contains("Ship's log"));
    assert!(text.contains("Logbook"));
    let row = text.lines().find(|l| l.contains("Logbook")).unwrap();
    assert!(
        row.contains('\u{2014}'),
        "an unfetched logbook shows a dash, not a zero: {row}"
    );

    state.logbook_pages = Some(Vec::new());
    let fetched = buffer_text(&render_cockpit(&state, 70, 20));
    let row = fetched.lines().find(|l| l.contains("Logbook")).unwrap();
    assert!(row.contains('0'), "once fetched and empty, it says zero: {row}");
}
