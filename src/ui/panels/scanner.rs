use crate::api::types::{
    DangerLevel, SectorObject, SectorObjectType, SensorMode,
};
use crate::app::AppState;
use chrono::Utc;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::ui::theme::{format_age, format_duration, freshness_color, freshness_label, gauge_color, knowledge_color, knowledge_label, map_cell_symbol, object_icon, panel_block, sensor_dot, sensor_style};
// ── Scanner panel ─────────────────────────────────────────────────────────────

pub(crate) fn render_scanner_panel(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let block = panel_block(" SCANNER ", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(probe) = &state.probe else {
        frame.render_widget(
            Paragraph::new("No data").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    };

    let is_blind = probe.sensor_mode == SensorMode::Blind;

    // Action hints come from the cockpit's shared hints line (F1); the pane
    // uses its whole area for the sector detail + history.
    let filtered = state.filtered_history_indices();
    let history_len = filtered.len();
    let history_width: u16 = if history_len > 0 { 22 } else { 0 };
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(history_width)])
        .split(inner);
    let detail_area = cols[0];
    let history_area = cols[1];

    // History list (always rendered when non-empty)
    if history_len > 0 {
        let hist_block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(Color::DarkGray));
        let hist_inner = hist_block.inner(history_area);
        frame.render_widget(hist_block, history_area);

        let items: Vec<ListItem> = filtered
            .iter()
            .map(|&i| {
                let s = &state.scan_history[i];
                let c = &s.relative_coordinates;
                let label = format!("{},{},{}", c.x as i64, c.y as i64, c.z as i64);
                let color = sector_interest_color(s);
                let (sym, sym_style) = map_cell_symbol(s);
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{sym} "), sym_style),
                    Span::styled(format!("{label:<9}"), Style::default().fg(color)),
                    Span::styled(format!("d:{}", s.distance), Style::default().fg(Color::DarkGray)),
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

    // Detail area
    if state.scan_loading {
        frame.render_widget(
            Paragraph::new("Scanning…").style(Style::default().fg(Color::DarkGray)),
            detail_area,
        );
        return;
    }

    if let Some(err) = &state.scan_error {
        frame.render_widget(
            Paragraph::new(format!("ERR: {err}")).style(Style::default().fg(Color::Red)),
            detail_area,
        );
        return;
    }

    let Some(sector) = state.current_sector() else {
        let (msg, color) = if is_blind {
            ("● sensors blind — history available →", Color::Red)
        } else if focused {
            ("No scan data — F5 to refresh", Color::DarkGray)
        } else {
            ("No scan data", Color::DarkGray)
        };
        frame.render_widget(
            Paragraph::new(msg).style(Style::default().fg(color)),
            detail_area,
        );
        return;
    };

    let sensor = probe.sensor_mode.clone();
    let mut lines: Vec<Line> = Vec::new();

    let knowledge_str = knowledge_label(&sector.knowledge_level);
    let knowledge_color = knowledge_color(&sector.knowledge_level);
    let confidence_color = gauge_color(sector.confidence);
    let coords = &sector.relative_coordinates;
    lines.push(Line::from(vec![
        Span::styled(
            format!("({},{},{})", coords.x as i64, coords.y as i64, coords.z as i64),
            Style::default().fg(Color::White),
        ),
        Span::raw("  "),
        Span::styled(
            format!("d:{}", sector.distance),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("  "),
        Span::styled(knowledge_str, Style::default().fg(knowledge_color)),
        Span::raw("  "),
        Span::styled(
            format!("{:.0}%", sector.confidence * 100.0),
            Style::default().fg(confidence_color).add_modifier(Modifier::BOLD),
        ),
    ]));

    let freshness_str = sector.data_freshness.as_ref().map(freshness_label).unwrap_or("—");
    let freshness_color = sector.data_freshness.as_ref().map(freshness_color).unwrap_or(Color::DarkGray);
    let scan_q = sector.scan.scan_quality;
    let mut freshness_spans = vec![
        Span::styled(freshness_str, Style::default().fg(freshness_color)),
        Span::raw("  quality: "),
        Span::styled(
            format!("{:.0}%", scan_q * 100.0),
            Style::default().fg(gauge_color(scan_q)),
        ),
        Span::raw("  sensors: "),
        Span::styled(sensor_dot(&sensor), sensor_style(&sensor)),
    ];
    if let Some(scanned_at) = sector.scanned_at {
        let age_secs = (Utc::now() - scanned_at).num_seconds().max(0);
        freshness_spans.push(Span::styled(
            format!("  scanned {}", format_age(age_secs)),
            Style::default().fg(Color::DarkGray),
        ));
    }
    lines.push(Line::from(freshness_spans));

    let req = sector.scan.required_residence_seconds;
    let cur = sector.scan.current_sector_residence_seconds;
    if req > 0 {
        let ratio = (cur as f64 / req as f64).clamp(0.0, 1.0);
        let res_color = if ratio >= 1.0 { Color::Green } else { Color::Yellow };
        lines.push(Line::from(vec![
            Span::styled("residence  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} / {}", format_duration(cur), format_duration(req)),
                Style::default().fg(res_color),
            ),
        ]));
    }

    if let Some(risk) = &sector.navigational_risk {
        if !risk.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("risk  ", Style::default().fg(Color::DarkGray)),
                Span::styled(risk.as_str(), Style::default().fg(Color::Yellow)),
            ]));
        }
    }

    if let Some(msg) = &sector.message {
        if !msg.is_empty() {
            lines.push(Line::from(Span::styled(
                msg.as_str(),
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            )));
        }
    }

    let browsing = focused && state.scanner_obj_selection.is_some() && state.viewing_probe_sector();
    let selected_obj_id: Option<String> = if browsing {
        state
            .scanner_obj_selection
            .and_then(|i| state.scanner_objects().into_iter().nth(i))
            .map(|e| e.id)
    } else {
        None
    };

    if let Some(objects) = &sector.objects {
        if !objects.is_empty() {
            lines.push(Line::from(Span::styled(
                "── objects ──",
                Style::default().fg(Color::DarkGray),
            )));
            for obj in objects {
                lines.extend(sector_object_lines(obj, browsing, selected_obj_id.as_deref()));
            }
        }
    }

    if let Some(probes) = &sector.probes {
        if !probes.is_empty() {
            lines.push(Line::from(Span::styled(
                "── other probes ──",
                Style::default().fg(Color::DarkGray),
            )));
            for p in probes {
                let moving = if p.moving { " moving" } else { " idle" };
                lines.push(Line::from(vec![
                    Span::styled("⊕ ", Style::default().fg(Color::Cyan)),
                    Span::raw(p.name.as_str()),
                    Span::styled(moving, Style::default().fg(Color::DarkGray)),
                ]));
            }
        }
    }

    let has_objects = sector.objects.as_ref().is_some_and(|o| !o.is_empty());
    let confirmed_empty = sector.objects.as_ref().is_some_and(|o| o.is_empty())
        && sector.possible_objects.as_ref().is_none_or(|p| p.is_empty())
        && sector.estimated_objects.is_none();
    if confirmed_empty {
        lines.push(Line::from(Span::styled(
            "empty sector",
            Style::default().fg(Color::DarkGray),
        )));
    }
    if !has_objects {
        if let Some(possible) = &sector.possible_objects {
            if !possible.is_empty() {
                lines.push(Line::from(Span::styled(
                    "── possible ──",
                    Style::default().fg(Color::DarkGray),
                )));
                for p in possible {
                    lines.push(Line::from(vec![
                        Span::styled("? ", Style::default().fg(Color::DarkGray)),
                        Span::raw(p.as_str()),
                    ]));
                }
            }
        }
        if let Some(est) = &sector.estimated_objects {
            lines.push(Line::from(Span::styled(
                "── estimated ──",
                Style::default().fg(Color::DarkGray),
            )));
            if est.star == Some(true) {
                lines.push(Line::from(vec![
                    Span::styled("★ ", Style::default().fg(Color::Yellow)),
                    Span::raw("star"),
                ]));
            }
            let pmin = est.planet_count_min.unwrap_or(0);
            let pmax = est.planet_count_max.unwrap_or(0);
            if pmax > 0 {
                lines.push(Line::from(vec![
                    Span::styled("● ", Style::default().fg(Color::Cyan)),
                    Span::raw(if pmin == pmax {
                        format!("{pmin} planet(s)")
                    } else {
                        format!("{pmin}–{pmax} planet(s)")
                    }),
                ]));
            }
            if let Some(bh) = est.black_hole_probability {
                if bh > 0.0 {
                    lines.push(Line::from(vec![
                        Span::styled("◉ ", Style::default().fg(Color::Magenta)),
                        Span::raw(format!("black hole {:.0}%", bh * 100.0)),
                    ]));
                }
            }
            if let Some(danger) = &est.danger_estimate {
                let (label, color) = match danger {
                    DangerLevel::Low => ("low", Color::Green),
                    DangerLevel::Moderate => ("moderate", Color::Yellow),
                    DangerLevel::Extreme => ("extreme", Color::Red),
                    DangerLevel::Unknown => ("?", Color::DarkGray),
                };
                lines.push(Line::from(vec![
                    Span::styled("danger  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(label, Style::default().fg(color)),
                ]));
            }
            if let Some(age) = &est.signal_age {
                lines.push(Line::from(vec![
                    Span::styled("signal  ", Style::default().fg(Color::DarkGray)),
                    Span::raw(age.as_str()),
                ]));
            }
        }
    }

    frame.render_widget(
        Paragraph::new(lines).scroll((state.scan_detail_scroll as u16, 0)),
        detail_area,
    );
}

pub(crate) fn sector_interest_color(s: &crate::api::types::SectorObservation) -> Color {
    use crate::api::types::{DangerLevel, KnowledgeLevel, SectorObjectType};

    // Known objects present
    if let Some(objects) = &s.objects {
        if !objects.is_empty() {
            let has_star = objects.iter().any(|o| {
                matches!(o.object_type, SectorObjectType::Star | SectorObjectType::SolarSystem)
            });
            let has_blackhole = objects
                .iter()
                .any(|o| matches!(o.object_type, SectorObjectType::BlackHole));
            let has_extreme = objects
                .iter()
                .any(|o| matches!(o.danger_level, Some(DangerLevel::Extreme)));
            if has_extreme || has_blackhole {
                return Color::Red;
            }
            if has_star {
                return Color::Yellow;
            }
            return Color::Green;
        }
    }

    // Estimated objects from neighbor scan
    if let Some(est) = &s.estimated_objects {
        if matches!(est.danger_estimate, Some(DangerLevel::Extreme)) {
            return Color::Red;
        }
        if est.black_hole_probability.unwrap_or(0.0) > 0.5 {
            return Color::Magenta;
        }
        if est.star == Some(true) {
            return Color::Yellow;
        }
        if est.planet_count_max.unwrap_or(0) > 0 {
            return Color::Cyan;
        }
    }

    if s.possible_objects.as_ref().is_some_and(|p| !p.is_empty()) {
        return Color::Cyan;
    }

    if s.knowledge_level == KnowledgeLevel::Detailed {
        return Color::White;
    }

    Color::DarkGray
}

pub(crate) fn obj_sel_prefix(browsing: bool, selected: bool) -> Option<Span<'static>> {
    if !browsing {
        return None;
    }
    Some(if selected {
        Span::styled("▶ ", Style::default().fg(Color::Yellow))
    } else {
        Span::raw("  ")
    })
}

pub(crate) fn sector_object_lines<'a>(
    obj: &'a SectorObject,
    browsing: bool,
    selected_id: Option<&str>,
) -> Vec<Line<'a>> {
    let (icon, color) = object_icon(&obj.object_type);
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

    let main_selected = browsing && obj.id.is_some() && obj.id.as_deref() == selected_id;
    let name_style = if main_selected {
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let mut main_spans: Vec<Span> = Vec::new();
    if let Some(prefix) = obj_sel_prefix(browsing, main_selected) {
        main_spans.push(prefix);
    }
    main_spans.extend([
        Span::styled(icon, Style::default().fg(color)),
        Span::raw(" "),
        Span::styled(estimated, Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{name}{danger}"), name_style),
    ]);
    if salvageable {
        main_spans.push(Span::styled("  ⬡ salvageable", Style::default().fg(Color::Yellow)));
    }
    if !manny_state.is_empty() {
        main_spans.push(Span::styled(
            format!("  [{manny_state}]"),
            Style::default().fg(Color::DarkGray),
        ));
    }
    // drifting item: show type + quantity instead of summary
    if obj.object_type == SectorObjectType::DriftingItem {
        if let (Some(itype), Some(qty)) = (&obj.item_type, obj.quantity) {
            main_spans.push(Span::styled(
                format!("  {itype} × {qty}"),
                Style::default().fg(Color::DarkGray),
            ));
        }
    } else if obj.object_type == SectorObjectType::DetachedContainer {
        if let Some(cap) = obj.capacity {
            let mode = obj.mode.as_deref().unwrap_or("drifting");
            main_spans.push(Span::styled(
                format!("  {mode}  {cap:.2} ECE"),
                Style::default().fg(Color::DarkGray),
            ));
        }
    } else if let Some(summary) = &obj.summary {
        if !matches!(obj.object_type, SectorObjectType::SolarSystem) || obj.bookmark_targets.is_empty() {
            main_spans.push(Span::styled(
                format!("  {summary}"),
                Style::default().fg(Color::DarkGray),
            ));
        }
    }

    let mut lines = vec![Line::from(main_spans)];

    let skip_dimensions = matches!(obj.object_type, SectorObjectType::SolarSystem);
    let has_mass = obj.mass.is_some() && !skip_dimensions;
    let has_radius = obj.radius.is_some() && !skip_dimensions;
    let has_uid = obj.manny_uid.is_some();
    if has_mass || has_radius || has_uid {
        let mut detail_spans = vec![Span::raw("  ")];
        if let Some(m) = obj.mass {
            detail_spans.push(Span::styled("mass ", Style::default().fg(Color::DarkGray)));
            detail_spans.push(Span::styled(
                format!("{m:.3e}"),
                Style::default().fg(Color::White),
            ));
            if has_radius { detail_spans.push(Span::raw("  ")); }
        }
        if let Some(r) = obj.radius {
            detail_spans.push(Span::styled("radius ", Style::default().fg(Color::DarkGray)));
            detail_spans.push(Span::styled(
                format!("{r:.3e}"),
                Style::default().fg(Color::White),
            ));
        }
        if let Some(uid) = &obj.manny_uid {
            if has_mass || has_radius { detail_spans.push(Span::raw("  ")); }
            detail_spans.push(Span::styled("uid ", Style::default().fg(Color::DarkGray)));
            detail_spans.push(Span::styled(uid.as_str(), Style::default().fg(Color::White)));
        }
        lines.push(Line::from(detail_spans));
    }

    // Minable asteroid targets with resource types
    if let Some(targets) = &obj.minable_targets {
        for target in targets {
            let (icon, color) = object_icon(&target.object_type);
            let name = target.name.as_deref().unwrap_or("unnamed");
            let selected = browsing && selected_id == Some(target.id.as_str());
            let target_style = if selected {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let mut spans: Vec<Span> = Vec::new();
            if let Some(prefix) = obj_sel_prefix(browsing, selected) {
                spans.push(prefix);
            }
            spans.extend([
                Span::styled("  ", Style::default().fg(Color::DarkGray)),
                Span::styled(icon, Style::default().fg(color)),
                Span::raw(" "),
                Span::styled(name.to_string(), target_style),
            ]);
            if let Some(resources) = &target.resource_types {
                let res_str = resources.iter().map(|r| match r.as_str() {
                    "metals" => "metals",
                    "ice" => "ice",
                    "carbon_compounds" => "carbon",
                    other => other,
                }).collect::<Vec<_>>().join("  ");
                if !res_str.is_empty() {
                    spans.push(Span::styled(
                        format!("  {res_str}"),
                        Style::default().fg(Color::Yellow),
                    ));
                }
            }
            lines.push(Line::from(spans));
        }
    }

    // Nested bodies of a solar system
    for target in &obj.bookmark_targets {
        let (icon, color) = object_icon(&target.object_type);
        let name = target.name.as_deref().unwrap_or("unnamed");
        let selected = browsing && selected_id == Some(target.id.as_str());
        let target_style = if selected {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let mut spans: Vec<Span> = Vec::new();
        if let Some(prefix) = obj_sel_prefix(browsing, selected) {
            spans.push(prefix);
        }
        spans.extend([
            Span::styled("  ", Style::default().fg(Color::DarkGray)),
            Span::styled(icon, Style::default().fg(color)),
            Span::raw(" "),
            Span::styled(name.to_string(), target_style),
        ]);
        let mut extras: Vec<String> = Vec::new();
        if let Some(m) = target.mass {
            extras.push(format!("{m:.3e} {}", target.mass_unit.as_deref().unwrap_or("")));
        }
        if let Some(r) = target.radius {
            extras.push(format!("r {r:.3e} {}", target.radius_unit.as_deref().unwrap_or("")));
        }
        if !extras.is_empty() {
            spans.push(Span::styled(
                format!("  {}", extras.join("  ")),
                Style::default().fg(Color::DarkGray),
            ));
        }
        lines.push(Line::from(spans));
    }

    lines
}

