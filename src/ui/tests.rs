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
use crate::app::{ActiveWizard, AppState, ColorMode, ContainerRulesInput, DetachInput, Pane, TransferProbeInput};
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
    let p = palette(mode);

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
    let p = palette(ColorMode::MonoGreen);
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
    let p = palette(ColorMode::MonoGreen);

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
    let p = palette(ColorMode::MonoGreen);
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
