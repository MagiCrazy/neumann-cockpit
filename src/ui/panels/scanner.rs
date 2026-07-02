use crate::api::types::{DangerLevel, ResourceShares, SectorObject, SectorObjectType, SensorMode};
use crate::app::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::ui::theme::{
    knowledge_color, knowledge_label, map_cell_style, object_color, object_icon, palette,
    pane_block, ratio_color, Palette,
};
// ── Scanner panel ─────────────────────────────────────────────────────────────
//
// The SECTOR pane owns the current sector's full detail. The Scanner is the
// cartography station: it scans *elsewhere* (neighbors, remote coordinates) and
// keeps the history of everything observed. So the history list is always
// present, and the detail area focuses on the selected — usually remote —
// observation. When the cursor lands on the current sector, we redirect to the
// SECTOR pane instead of duplicating it. All colours come from the active
// [`Palette`] so the pane matches the rest of the phosphor cockpit.

pub(crate) fn render_scanner_panel(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let p = palette(state.color_mode);
    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);

    let block = pane_block(" SCANNER ", focused, p);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(probe) = &state.probe else {
        frame.render_widget(Paragraph::new("No data").style(dim), inner);
        return;
    };

    let is_blind = probe.sensor_mode == SensorMode::Blind;

    // History list is the core of the pane — always shown when non-empty.
    let filtered = state.filtered_history_indices();
    let history_len = filtered.len();
    let history_width: u16 = if history_len > 0 { 22 } else { 0 };
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(history_width)])
        .split(inner);
    let detail_area = cols[0];
    let history_area = cols[1];

    if history_len > 0 {
        let hist_block = Block::default().borders(Borders::LEFT).border_style(dim);
        let hist_inner = hist_block.inner(history_area);
        frame.render_widget(hist_block, history_area);

        let items: Vec<ListItem> = filtered
            .iter()
            .map(|&i| {
                let s = &state.scan_history[i];
                let c = &s.relative_coordinates;
                let label = format!("{},{},{}", c.x as i64, c.y as i64, c.z as i64);
                let color = sector_interest_color(s, p);
                let (sym, sym_style) = map_cell_style(s, p);
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{sym} "), sym_style),
                    Span::styled(format!("{label:<9}"), Style::default().fg(color)),
                    Span::styled(format!("d:{}", s.distance), dim),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ ");

        let mut list_state = ListState::default();
        list_state.select(filtered.iter().position(|&i| i == state.scan_history_idx));
        frame.render_stateful_widget(list, hist_inner, &mut list_state);
    }

    // ── Detail area ──
    if state.scan_loading {
        frame.render_widget(Paragraph::new("Scanning…").style(dim), detail_area);
        return;
    }
    if let Some(err) = &state.scan_error {
        frame.render_widget(
            Paragraph::new(format!("ERR: {err}")).style(Style::default().fg(p.crit)),
            detail_area,
        );
        return;
    }
    let Some(sector) = state.current_sector() else {
        let (msg, color) = if is_blind {
            ("● sensors blind — history available →", p.crit)
        } else if focused {
            ("No scan data — Enter to observe", p.dim)
        } else {
            ("No scan data", p.dim)
        };
        frame.render_widget(Paragraph::new(msg).style(Style::default().fg(color)), detail_area);
        return;
    };

    let coords = &sector.relative_coordinates;
    let (cx, cy, cz) = (coords.x as i64, coords.y as i64, coords.z as i64);
    let mut lines: Vec<Line> = Vec::new();

    // Current sector: redirect to SECTOR, stay focused on remote scanning.
    if state.viewing_probe_sector() {
        lines.push(Line::from(vec![
            Span::styled(format!("({cx},{cy},{cz})"), text),
            Span::styled("  current sector", dim),
        ]));
        lines.push(Line::default());
        lines.push(Line::from(Span::styled("→ details in the SECTOR pane", Style::default().fg(p.accent))));
        let obj_count = sector.objects.as_ref().map(|o| o.len()).unwrap_or(0);
        let summary = if obj_count > 0 {
            format!("{obj_count} object(s) here")
        } else {
            "empty sector".to_string()
        };
        lines.push(Line::from(Span::styled(summary, dim)));
        lines.push(Line::default());
        lines.push(Line::from(Span::styled("z zoom → neighbor map", dim)));
        frame.render_widget(Paragraph::new(lines), detail_area);
        return;
    }

    // ── Remote / neighbor observation ──
    // Header: coords · distance · knowledge level. Sensors live in PROBE; the
    // scan-quality % is relegated to the zoom view.
    lines.push(Line::from(vec![
        Span::styled(format!("({cx},{cy},{cz})"), text),
        Span::raw("  "),
        Span::styled(format!("d:{}", sector.distance), dim),
        Span::raw("  "),
        Span::styled(
            knowledge_label(&sector.knowledge_level),
            Style::default().fg(knowledge_color(&sector.knowledge_level, p)),
        ),
    ]));

    // Single confidence gauge — the one trust signal kept in compact.
    let conf = sector.confidence;
    lines.push(Line::from(vec![
        Span::styled("confidence ", dim),
        Span::styled(
            format!("{:.0}%", conf * 100.0),
            Style::default().fg(ratio_color(conf, p)).add_modifier(Modifier::BOLD),
        ),
    ]));

    // Navigational risk (safety-critical — always kept).
    if let Some(risk) = &sector.navigational_risk {
        if !risk.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("risk  ", dim),
                Span::styled(risk.as_str(), Style::default().fg(p.warn)),
            ]));
        }
    }

    // Confirmed objects (decluttered: no mass/radius/uid — see zoom).
    if let Some(objects) = &sector.objects {
        if !objects.is_empty() {
            lines.push(Line::from(Span::styled("── objects ──", dim)));
            for obj in objects {
                lines.extend(sector_object_lines(obj, true, p));
            }
        }
    }

    if let Some(probes) = &sector.probes {
        if !probes.is_empty() {
            lines.push(Line::from(Span::styled("── other probes ──", dim)));
            for pr in probes {
                let moving = if pr.moving { " moving" } else { " idle" };
                lines.push(Line::from(vec![
                    Span::styled("⊕ ", Style::default().fg(p.accent)),
                    Span::styled(pr.name.as_str(), text),
                    Span::styled(moving, dim),
                ]));
            }
        }
    }

    // The value of a neighbor scan: what the sector *probably* holds.
    let has_objects = sector.objects.as_ref().is_some_and(|o| !o.is_empty());
    let confirmed_empty = sector.objects.as_ref().is_some_and(|o| o.is_empty())
        && sector.possible_objects.as_ref().is_none_or(|o| o.is_empty())
        && sector.estimated_objects.is_none();
    if confirmed_empty {
        lines.push(Line::from(Span::styled("empty sector", dim)));
    }
    if !has_objects {
        if let Some(possible) = &sector.possible_objects {
            if !possible.is_empty() {
                lines.push(Line::from(Span::styled("── possible ──", dim)));
                for pobj in possible {
                    lines.push(Line::from(vec![Span::styled("? ", dim), Span::styled(pobj.as_str(), text)]));
                }
            }
        }
        if let Some(est) = &sector.estimated_objects {
            lines.push(Line::from(Span::styled("── estimated ──", dim)));
            if est.star == Some(true) {
                lines.push(Line::from(vec![
                    Span::styled("★ ", Style::default().fg(p.warn)),
                    Span::styled("star", text),
                ]));
            }
            let pmin = est.planet_count_min.unwrap_or(0);
            let pmax = est.planet_count_max.unwrap_or(0);
            if pmax > 0 {
                lines.push(Line::from(vec![
                    Span::styled("● ", Style::default().fg(p.accent)),
                    Span::styled(
                        if pmin == pmax {
                            format!("{pmin} planet(s)")
                        } else {
                            format!("{pmin}–{pmax} planet(s)")
                        },
                        text,
                    ),
                ]));
            }
            if let Some(bh) = est.black_hole_probability {
                if bh > 0.0 {
                    lines.push(Line::from(vec![
                        Span::styled("◉ ", Style::default().fg(p.crit)),
                        Span::styled(format!("black hole {:.0}%", bh * 100.0), text),
                    ]));
                }
            }
            if let Some(danger) = &est.danger_estimate {
                let (label, color) = match danger {
                    DangerLevel::Low => ("low", p.good),
                    DangerLevel::Moderate => ("moderate", p.warn),
                    DangerLevel::Extreme => ("extreme", p.crit),
                    DangerLevel::Unknown => ("?", p.dim),
                };
                lines.push(Line::from(vec![
                    Span::styled("danger  ", dim),
                    Span::styled(label, Style::default().fg(color)),
                ]));
            }
        }
    }

    frame.render_widget(
        Paragraph::new(lines).scroll((state.scan_detail_scroll as u16, 0)),
        detail_area,
    );
}

/// Palette-aware interest colour for a scanned sector (drives the history list
/// and the neighbor map).
pub(crate) fn sector_interest_color(s: &crate::api::types::SectorObservation, p: Palette) -> ratatui::style::Color {
    use crate::api::types::{DangerLevel, KnowledgeLevel, SectorObjectType};

    if let Some(objects) = &s.objects {
        if !objects.is_empty() {
            let has_star = objects
                .iter()
                .any(|o| matches!(o.object_type, SectorObjectType::Star | SectorObjectType::SolarSystem));
            let has_blackhole = objects.iter().any(|o| matches!(o.object_type, SectorObjectType::BlackHole));
            let has_extreme = objects.iter().any(|o| matches!(o.danger_level, Some(DangerLevel::Extreme)));
            if has_extreme || has_blackhole {
                return p.crit;
            }
            if has_star {
                return p.warn;
            }
            return p.good;
        }
    }

    if let Some(est) = &s.estimated_objects {
        if matches!(est.danger_estimate, Some(DangerLevel::Extreme)) {
            return p.crit;
        }
        if est.black_hole_probability.unwrap_or(0.0) > 0.5 {
            return p.crit;
        }
        if est.star == Some(true) {
            return p.warn;
        }
        if est.planet_count_max.unwrap_or(0) > 0 {
            return p.accent;
        }
    }

    if s.possible_objects.as_ref().is_some_and(|o| !o.is_empty()) {
        return p.accent;
    }
    if s.knowledge_level == KnowledgeLevel::Detailed {
        return p.text;
    }
    p.dim
}

/// A one-line breakdown of the four mineable resources, skipping zeros.
/// `percent` renders shares as `40%`; otherwise reserves as `1.20`.
pub(crate) fn resource_shares_line<'a>(
    label: &'a str,
    shares: Option<&ResourceShares>,
    percent: bool,
    p: Palette,
) -> Option<Line<'a>> {
    let s = shares?;
    let parts = [
        ("metals", s.metals),
        ("ice", s.ice),
        ("carbon", s.carbon_compounds),
        ("deut", s.deuterium),
    ];
    let mut spans = vec![Span::styled(label, Style::default().fg(p.dim))];
    let mut any = false;
    for (name, v) in parts {
        if v <= 0.0 {
            continue;
        }
        any = true;
        let val = if percent { format!("{:.0}%", v * 100.0) } else { format!("{v:.2}") };
        spans.push(Span::styled(format!("{name} "), Style::default().fg(p.dim)));
        spans.push(Span::styled(format!("{val}  "), Style::default().fg(p.text)));
    }
    any.then_some(Line::from(spans))
}

/// Lines for one scanned object. `compact` drops the scientific detail
/// (mass / radius / uid and nested-body dimensions) shown only in the zoom view.
pub(crate) fn sector_object_lines<'a>(obj: &'a SectorObject, compact: bool, p: Palette) -> Vec<Line<'a>> {
    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);
    let glyph = object_icon(&obj.object_type).0;
    let color = object_color(&obj.object_type, p);
    let estimated = if obj.estimated.unwrap_or(false) { "~ " } else { "" };
    let name = obj.name.as_deref().unwrap_or("unnamed");
    let danger = obj
        .danger_level
        .as_ref()
        .map(|d| match d {
            DangerLevel::Moderate => " ⚠",
            DangerLevel::Extreme => " ☠",
            _ => "",
        })
        .unwrap_or("");

    let manny_state = obj.manny_state.as_deref().unwrap_or("");
    let salvageable = obj.salvageable.unwrap_or(false);

    let mut main_spans: Vec<Span> = vec![
        Span::styled(glyph, Style::default().fg(color)),
        Span::raw(" "),
        Span::styled(estimated, dim),
        Span::styled(format!("{name}{danger}"), text),
    ];
    if salvageable {
        main_spans.push(Span::styled("  ⬡ salvageable", Style::default().fg(p.warn)));
    }
    if !manny_state.is_empty() {
        main_spans.push(Span::styled(format!("  [{manny_state}]"), dim));
    }
    if obj.object_type == SectorObjectType::DriftingItem {
        if let (Some(itype), Some(qty)) = (&obj.item_type, obj.quantity) {
            main_spans.push(Span::styled(format!("  {itype} × {qty}"), dim));
        }
    } else if obj.object_type == SectorObjectType::DetachedContainer {
        if let Some(cap) = obj.capacity {
            let mode = obj.mode.as_deref().unwrap_or("drifting");
            main_spans.push(Span::styled(format!("  {mode}  {cap:.2} ECE"), dim));
        }
    } else if let Some(summary) = &obj.summary {
        if !matches!(obj.object_type, SectorObjectType::SolarSystem) || obj.bookmark_targets.is_empty() {
            main_spans.push(Span::styled(format!("  {summary}"), dim));
        }
    }

    let mut lines = vec![Line::from(main_spans)];

    // Scientific detail — zoom only.
    if !compact {
        let skip_dimensions = matches!(obj.object_type, SectorObjectType::SolarSystem);
        let has_mass = obj.mass.is_some() && !skip_dimensions;
        let has_radius = obj.radius.is_some() && !skip_dimensions;
        let has_uid = obj.manny_uid.is_some();
        if has_mass || has_radius || has_uid {
            let mut detail_spans = vec![Span::raw("  ")];
            if let Some(m) = obj.mass {
                detail_spans.push(Span::styled("mass ", dim));
                detail_spans.push(Span::styled(format!("{m:.3e}"), text));
                if has_radius {
                    detail_spans.push(Span::raw("  "));
                }
            }
            if let Some(r) = obj.radius {
                detail_spans.push(Span::styled("radius ", dim));
                detail_spans.push(Span::styled(format!("{r:.3e}"), text));
            }
            if let Some(uid) = &obj.manny_uid {
                if has_mass || has_radius {
                    detail_spans.push(Span::raw("  "));
                }
                detail_spans.push(Span::styled("uid ", dim));
                detail_spans.push(Span::styled(uid.as_str(), text));
            }
            lines.push(Line::from(detail_spans));
        }

        // Planet / asteroid science (v63 fields).
        if let Some(cat) = &obj.category {
            lines.push(Line::from(vec![Span::styled("  class ", dim), Span::styled(cat.as_str(), text)]));
        }
        if let Some(h) = obj.habitability_score {
            lines.push(Line::from(vec![
                Span::styled("  habitability ", dim),
                Span::styled(format!("{:.0}%", h * 100.0), Style::default().fg(ratio_color(h, p))),
            ]));
        }
        if let Some(comp) = &obj.composition {
            lines.push(Line::from(vec![Span::styled("  composition ", dim), Span::styled(comp.as_str(), text)]));
        }
        if obj.manny_mineable == Some(true) {
            lines.push(Line::from(Span::styled("  ⛏ mineable", Style::default().fg(p.warn))));
        }
        if let Some(line) = resource_shares_line("  shares ", obj.resource_composition.as_ref(), true, p) {
            lines.push(line);
        }
        if let Some(line) = resource_shares_line("  reserves ", obj.resource_amounts.as_ref(), false, p) {
            lines.push(line);
        }
    }

    // Minable asteroid targets with resource types (kept in both views —
    // that's the actionable payoff of a scan).
    if let Some(targets) = &obj.minable_targets {
        for target in targets {
            let tglyph = object_icon(&target.object_type).0;
            let tcolor = object_color(&target.object_type, p);
            let name = target.name.as_deref().unwrap_or("unnamed");
            let mut spans: Vec<Span> = vec![
                Span::raw("  "),
                Span::styled(tglyph, Style::default().fg(tcolor)),
                Span::raw(" "),
                Span::styled(name.to_string(), text),
            ];
            if let Some(resources) = &target.resource_types {
                let res_str = resources
                    .iter()
                    .map(|r| match r.as_str() {
                        "carbon_compounds" => "carbon",
                        other => other,
                    })
                    .collect::<Vec<_>>()
                    .join("  ");
                if !res_str.is_empty() {
                    spans.push(Span::styled(format!("  {res_str}"), Style::default().fg(p.warn)));
                }
            }
            lines.push(Line::from(spans));
        }
    }

    // Nested bodies of a solar system.
    for target in &obj.bookmark_targets {
        let tglyph = object_icon(&target.object_type).0;
        let tcolor = object_color(&target.object_type, p);
        let name = target.name.as_deref().unwrap_or("unnamed");
        let mut spans: Vec<Span> = vec![
            Span::raw("  "),
            Span::styled(tglyph, Style::default().fg(tcolor)),
            Span::raw(" "),
            Span::styled(name.to_string(), text),
        ];
        if !compact {
            let mut extras: Vec<String> = Vec::new();
            if let Some(m) = target.mass {
                extras.push(format!("{m:.3e} {}", target.mass_unit.as_deref().unwrap_or("")));
            }
            if let Some(r) = target.radius {
                extras.push(format!("r {r:.3e} {}", target.radius_unit.as_deref().unwrap_or("")));
            }
            if !extras.is_empty() {
                spans.push(Span::styled(format!("  {}", extras.join("  ")), dim));
            }
        }
        lines.push(Line::from(spans));
    }

    lines
}
