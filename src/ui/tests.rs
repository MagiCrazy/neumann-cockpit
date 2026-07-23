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
