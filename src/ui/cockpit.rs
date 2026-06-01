use crate::api::types::{
    DangerLevel, DataFreshness, KnowledgeLevel, Manny, MannyLocationType, MannyTask,
    MovementPhase, ProbeStatus, SectorObject, SectorObjectType, SensorMode,
};
use crate::app::{AppState, Panel, ScanMode, TravelInput};
use chrono::Utc;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, LineGauge, List, ListItem, ListState, Paragraph},
    Frame,
};

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    let outer = Block::default()
        .title(" NEUMANN COCKPIT ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let main_area = rows[0];
    let status_area = rows[1];

    let top_h = top_row_height(state);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(top_h), Constraint::Min(0)])
        .split(main_area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    render_probe_panel(frame, top[0], state, state.focused == Some(Panel::Probe));
    render_inventory_panel(frame, top[1], state, state.focused == Some(Panel::Inventory));
    render_scanner_panel(frame, bottom[0], state, state.focused == Some(Panel::Scanner));
    render_mannies_panel(frame, bottom[1], state, state.focused == Some(Panel::Mannies));
    render_status_bar(frame, status_area, state);
    if !matches!(state.travel, TravelInput::Inactive) {
        render_travel_overlay(frame, area, state);
    }
}

// ── Probe panel ───────────────────────────────────────────────────────────────

fn render_probe_panel(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let block = panel_block(" PROBE ", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if state.loading && state.probe.is_none() {
        frame.render_widget(
            Paragraph::new("Fetching…").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let Some(probe) = &state.probe else {
        frame.render_widget(
            Paragraph::new("No data — press r to refresh")
                .style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    };

    let active_movement = probe.movement.as_ref().filter(|mv| {
        !matches!(
            mv.phase.as_ref().unwrap_or(&mv.status),
            MovementPhase::Arrived | MovementPhase::Failed | MovementPhase::Destroyed | MovementPhase::Idle
        )
    });

    let mut sections: Vec<Constraint> = vec![
        Constraint::Length(1), // name + status
    ];
    if active_movement.is_some() {
        sections.push(Constraint::Length(1)); // coords
        sections.push(Constraint::Length(1)); // phase + ETA
        sections.push(Constraint::Length(1)); // progress gauge
        sections.push(Constraint::Length(1)); // speed gauge
    }
    sections.push(Constraint::Length(1)); // fuel gauge
    if probe.systems.is_some() {
        sections.push(Constraint::Length(1)); // integrity gauge
    }
    sections.push(Constraint::Min(0)); // padding

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(sections)
        .split(inner);

    let mut row = 0;

    // ── Header ──
    let spinner = if state.loading { " ⟳" } else { "" };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(&probe.name, Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(probe_status_label(&probe.status), probe_status_style(&probe.status)),
            Span::styled(spinner, Style::default().fg(Color::DarkGray)),
        ])),
        rows[row],
    );
    row += 1;

    // ── Movement ──
    if let Some(mv) = active_movement {
        let remaining = (mv.arrival_at - Utc::now()).num_seconds().max(0);
        let elapsed = (Utc::now() - mv.started_at).num_seconds().max(0);
        let total = (mv.arrival_at - mv.started_at).num_seconds().max(1);
        let progress = (elapsed as f64 / total as f64).clamp(0.0, 1.0);
        let phase_label = movement_phase_label(mv.phase.as_ref().unwrap_or(&mv.status));

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                Span::raw(format!(
                    "({},{},{}) → ({},{},{})",
                    mv.origin.x as i64, mv.origin.y as i64, mv.origin.z as i64,
                    mv.target.x as i64, mv.target.y as i64, mv.target.z as i64,
                )),
            ])),
            rows[row],
        );
        row += 1;

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(phase_label, Style::default().fg(Color::Yellow)),
                Span::raw(format!("  ETA: {}", format_duration(remaining))),
            ])),
            rows[row],
        );
        row += 1;

        frame.render_widget(
            make_line_gauge(&format!("{:<12}{:.0}%", "Travel", progress * 100.0), progress, Color::Yellow),
            rows[row],
        );
        row += 1;

        let velocity = mv.estimated_velocity_c.unwrap_or(0.0).clamp(0.0, 1.0);
        frame.render_widget(
            make_line_gauge(
                &format!("{:<12}{:.2}c", "Speed", velocity),
                velocity,
                Color::Yellow,
            ),
            rows[row],
        );
        row += 1;
    }

    // ── Fuel ──
    let fuel_ratio = probe
        .inventory
        .external_tanks
        .iter()
        .find(|t| t.tank_type == "deuterium")
        .map(|t| (t.fill_percent / 100.0).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    frame.render_widget(
        make_line_gauge(
            &format!("{:<12}{:.1}%", "Fuel", fuel_ratio * 100.0),
            fuel_ratio,
            gauge_color(fuel_ratio),
        ),
        rows[row],
    );
    row += 1;

    // ── Integrity ──
    if let Some(sys) = &probe.systems {
        let integrity = (sys.integrity_percent.unwrap_or(100.0) / 100.0).clamp(0.0, 1.0);
        frame.render_widget(
            make_line_gauge(
                &format!("{:<12}{:.1}%", "Integrity", integrity * 100.0),
                integrity,
                gauge_color(integrity),
            ),
            rows[row],
        );
        row += 1;
    }

    let _ = row;
}

// ── Inventory panel ───────────────────────────────────────────────────────────

fn render_inventory_panel(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let block = panel_block(" INVENTORY ", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(probe) = &state.probe else {
        frame.render_widget(
            Paragraph::new("No data").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    };

    let inv = &probe.inventory;

    let cargo_ratio = if inv.capacity > 0.0 {
        (inv.used_capacity / inv.capacity).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let n_rows = 1 + inv.resource_stocks.len();

    let mut sections: Vec<Constraint> = (0..n_rows).map(|_| Constraint::Length(1)).collect();
    sections.push(Constraint::Min(0));

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(sections)
        .split(inner);

    let mut row = 0;

    frame.render_widget(
        make_line_gauge(
            &format!("{:<12}{:.2} / {:.2}", "Cargo", inv.used_capacity, inv.capacity),
            cargo_ratio,
            Color::Blue,
        ),
        rows[row],
    );
    row += 1;

    for stock in &inv.resource_stocks {
        let (icon, color, label) = match stock.stock_type.as_str() {
            "metals" => ("◆", Color::White, "Metals"),
            "other" => ("◇", Color::Cyan, "Non-metals"),
            _ => ("·", Color::DarkGray, stock.stock_type.as_str()),
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("  {icon} "), Style::default().fg(color)),
                Span::raw(format!("{:<11}", label)),
                Span::styled(format!("{:.3}", stock.amount), Style::default().fg(Color::White)),
                Span::styled(" ECE", Style::default().fg(Color::DarkGray)),
            ])),
            rows[row],
        );
        row += 1;
    }

    let _ = row;
}

// ── Mannies panel ─────────────────────────────────────────────────────────────

fn render_mannies_panel(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let block = panel_block(" MANNIES ", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(mannies) = &state.mannies else {
        frame.render_widget(
            Paragraph::new("No data").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    };

    if mannies.is_empty() {
        frame.render_widget(
            Paragraph::new("No mannies aboard").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let items: Vec<ListItem> = mannies.iter().map(|m| manny_list_item(m)).collect();

    let highlight_style = if focused {
        Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let list = List::new(items)
        .highlight_style(highlight_style)
        .highlight_symbol("▶ ");

    let mut list_state = ListState::default();
    if focused {
        list_state.select(Some(state.mannies_selection));
    }

    frame.render_stateful_widget(list, inner, &mut list_state);
}

fn manny_list_item(m: &Manny) -> ListItem<'_> {
    let loc_icon = match m.location.location_type {
        MannyLocationType::Probe => Span::styled("●", Style::default().fg(Color::Green)),
        MannyLocationType::Sector => Span::styled("◌", Style::default().fg(Color::Yellow)),
        MannyLocationType::Unknown => Span::styled("?", Style::default().fg(Color::DarkGray)),
    };

    let task_text = match &m.current_task {
        None => Span::styled("idle", Style::default().fg(Color::DarkGray)),
        Some(MannyTask::Repair) => Span::styled("repair", Style::default().fg(Color::Cyan)),
        Some(MannyTask::Mining) => Span::styled("mining", Style::default().fg(Color::Yellow)),
        Some(MannyTask::Returning) => Span::styled("returning", Style::default().fg(Color::Blue)),
        Some(MannyTask::WaitingForSpace) => {
            Span::styled("waiting", Style::default().fg(Color::Magenta))
        }
        Some(MannyTask::Unknown) => Span::styled("?", Style::default().fg(Color::DarkGray)),
    };

    let progress = if m.current_task.is_some() {
        format!(" {:3.0}%", m.task_progress_percent)
    } else {
        String::new()
    };

    let name = format!("{:<14}", m.name);

    ListItem::new(Line::from(vec![
        loc_icon,
        Span::raw(" "),
        Span::raw(name),
        task_text,
        Span::styled(progress, Style::default().fg(Color::DarkGray)),
    ]))
}

// ── Scanner panel ─────────────────────────────────────────────────────────────

fn render_scanner_panel(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
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

    // Outer vertical split: content row + hint bar
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    let content_area = rows[0];
    let hint_area = rows[1];

    let is_blind = probe.sensor_mode == SensorMode::Blind;

    // Hint bar
    match &state.scan_mode {
        ScanMode::Input(buf) => {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("coords (x y z): ", Style::default().fg(Color::Cyan)),
                    Span::raw(buf.as_str()),
                    Span::styled("█", Style::default().fg(Color::Cyan)),
                ])),
                hint_area,
            );
        }
        ScanMode::DirectionPick => {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("deep scan axis: ", Style::default().fg(Color::Cyan)),
                    Span::styled("[x]", Style::default().fg(Color::Yellow)),
                    Span::raw("  "),
                    Span::styled("[y]", Style::default().fg(Color::Yellow)),
                    Span::raw("  "),
                    Span::styled("[z]", Style::default().fg(Color::Yellow)),
                    Span::raw("  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])),
                hint_area,
            );
        }
        ScanMode::Current => {
            if let Some(remaining) = state.scan_batch {
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::styled("⟳ scanning  ", Style::default().fg(Color::Yellow)),
                        Span::styled(
                            format!("{remaining} remaining"),
                            Style::default().fg(Color::White),
                        ),
                    ])),
                    hint_area,
                );
            } else if focused {
                let spans = if is_blind {
                    vec![
                        Span::styled("● SENSORS BLIND", Style::default().fg(Color::Red)),
                        Span::raw("  "),
                        Span::styled("[↑/↓]", Style::default().fg(Color::Cyan)),
                        Span::raw(" history"),
                    ]
                } else {
                    let mut s = vec![
                        Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
                        Span::raw(" rescan  "),
                        Span::styled("[c]", Style::default().fg(Color::Cyan)),
                        Span::raw(" custom  "),
                        Span::styled("[n]", Style::default().fg(Color::Cyan)),
                        Span::raw(" neighbors  "),
                        Span::styled("[d]", Style::default().fg(Color::Cyan)),
                        Span::raw(" deep  "),
                        Span::styled("[↑/↓]", Style::default().fg(Color::Cyan)),
                        Span::raw(" history"),
                    ];
                    if state.current_sector().is_some() {
                        s.push(Span::raw("  "));
                        s.push(Span::styled("[g]", Style::default().fg(Color::Yellow)));
                        s.push(Span::raw(" go"));
                    }
                    s
                };
                frame.render_widget(Paragraph::new(Line::from(spans)), hint_area);
            }
        }
    }

    // Horizontal split: detail on left, history list on right
    let history_len = state.scan_history.len();
    let history_width: u16 = if history_len > 0 { 14 } else { 0 };
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(history_width)])
        .split(content_area);
    let detail_area = cols[0];
    let history_area = cols[1];

    // History list (always rendered when non-empty)
    if history_len > 0 {
        let hist_block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(Color::DarkGray));
        let hist_inner = hist_block.inner(history_area);
        frame.render_widget(hist_block, history_area);

        let items: Vec<Line> = state
            .scan_history
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let c = &s.relative_coordinates;
                let label = format!("{},{},{}", c.x as i64, c.y as i64, c.z as i64);
                let color = sector_interest_color(s);
                let selected = i == state.scan_history_idx;
                if selected {
                    Line::from(vec![
                        Span::styled("▶ ", Style::default().fg(Color::White)),
                        Span::styled(label, Style::default().fg(color).add_modifier(Modifier::BOLD)),
                    ])
                } else {
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled(label, Style::default().fg(color)),
                    ])
                }
            })
            .collect();
        frame.render_widget(Paragraph::new(items), hist_inner);
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
            ("Press [Enter] to scan current sector", Color::DarkGray)
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
    lines.push(Line::from(vec![
        Span::styled(freshness_str, Style::default().fg(freshness_color)),
        Span::raw("  quality: "),
        Span::styled(
            format!("{:.0}%", scan_q * 100.0),
            Style::default().fg(gauge_color(scan_q)),
        ),
        Span::raw("  sensors: "),
        Span::styled(sensor_dot(&sensor), sensor_style(&sensor)),
    ]));

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

    if let Some(objects) = &sector.objects {
        if !objects.is_empty() {
            lines.push(Line::from(Span::styled(
                "── objects ──",
                Style::default().fg(Color::DarkGray),
            )));
            for obj in objects {
                lines.extend(sector_object_lines(obj));
            }
        }
    }

    let has_objects = sector.objects.as_ref().map_or(false, |o| !o.is_empty());
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

    frame.render_widget(Paragraph::new(lines), detail_area);
}

// ── Travel overlay ────────────────────────────────────────────────────────────

fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(r.width), height.min(r.height))
}

fn render_travel_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let popup = centered_rect(46, 11, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" TRAVEL ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    let body = rows[0];
    let hint_area = rows[1];

    match &state.travel {
        TravelInput::Inactive => {}

        TravelInput::Typing(buf) => {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("Destination (x y z): ", Style::default().fg(Color::Cyan)),
                    Span::raw(buf.as_str()),
                    Span::styled("█", Style::default().fg(Color::Cyan)),
                ])),
                body,
            );
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
                    Span::raw(" preview  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])),
                hint_area,
            );
        }

        TravelInput::Confirming { x, y, z, sector_distance, fuel_cost, eta_minutes, error } => {
            let mut lines: Vec<Line> = Vec::new();

            lines.push(Line::from(vec![
                Span::styled("→  ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("({x}, {y}, {z})"),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
            ]));

            if let Some(dist) = sector_distance {
                lines.push(Line::from(vec![
                    Span::styled("   Distance  ", Style::default().fg(Color::DarkGray)),
                    Span::raw(format!("{dist} sector(s)")),
                ]));
            }

            if let Some(fuel) = fuel_cost {
                lines.push(Line::from(vec![
                    Span::styled("   Fuel      ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{fuel:.4}"), Style::default().fg(Color::Cyan)),
                    Span::styled(" units", Style::default().fg(Color::DarkGray)),
                ]));
            }

            if let Some(mins) = eta_minutes {
                lines.push(Line::from(vec![
                    Span::styled("   ETA       ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format_duration(mins * 60),
                        Style::default().fg(Color::Yellow),
                    ),
                ]));
            }

            if let Some(err) = error {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("   ✗ {err}"),
                    Style::default().fg(Color::Red),
                )));
            }

            frame.render_widget(Paragraph::new(lines), body);

            let hint = if error.is_some() {
                Line::from(vec![
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])
            } else {
                Line::from(vec![
                    Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw(" GO  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])
            };
            frame.render_widget(Paragraph::new(hint), hint_area);
        }
    }
}

fn sector_interest_color(s: &crate::api::types::SectorObservation) -> Color {
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

    if s.possible_objects.as_ref().map_or(false, |p| !p.is_empty()) {
        return Color::Cyan;
    }

    if s.knowledge_level == KnowledgeLevel::Detailed {
        return Color::White;
    }

    Color::DarkGray
}

fn sector_object_lines(obj: &SectorObject) -> Vec<Line<'_>> {
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

    let mut main_spans = vec![
        Span::styled(icon, Style::default().fg(color)),
        Span::raw(" "),
        Span::styled(estimated, Style::default().fg(Color::DarkGray)),
        Span::raw(format!("{name}{danger}")),
    ];
    if !manny_state.is_empty() {
        main_spans.push(Span::styled(
            format!("  [{manny_state}]"),
            Style::default().fg(Color::DarkGray),
        ));
    }
    main_spans.push(Span::styled(
        format!("  {}", obj.summary),
        Style::default().fg(Color::DarkGray),
    ));

    let mut lines = vec![Line::from(main_spans)];

    let has_mass = obj.mass.is_some();
    let has_radius = obj.radius.is_some();
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

    lines
}

// ── Status bar ────────────────────────────────────────────────────────────────

fn render_status_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let last = state
        .last_update
        .map(|t| t.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "—".to_string());

    let next = state
        .seconds_until_refresh()
        .map(|s| format!("in {}", format_duration(s)))
        .unwrap_or_else(|| "∞".to_string());

    let error_part = if let Some(e) = &state.error {
        format!("  ERR: {e}")
    } else {
        String::new()
    };

    let left = Line::from(vec![
        Span::styled("[r]", Style::default().fg(Color::Cyan)),
        Span::raw(" refresh  "),
        Span::styled("[p]", Style::default().fg(Color::Cyan)),
        Span::raw(" probe  "),
        Span::styled("[i]", Style::default().fg(Color::Cyan)),
        Span::raw(" inventory  "),
        Span::styled("[m]", Style::default().fg(Color::Cyan)),
        Span::raw(" mannies  "),
        Span::styled("[s]", Style::default().fg(Color::Cyan)),
        Span::raw(" scanner  "),
        Span::styled("[t]", Style::default().fg(Color::Cyan)),
        Span::raw(" travel  "),
        Span::styled("[q]", Style::default().fg(Color::Cyan)),
        Span::raw(" quit"),
        Span::styled(error_part, Style::default().fg(Color::Red)),
    ]);

    let right_text = format!("⟳ {last}   next: {next}");
    let right = Paragraph::new(right_text)
        .alignment(Alignment::Right)
        .style(Style::default().fg(Color::DarkGray));

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(35)])
        .split(area);

    frame.render_widget(Paragraph::new(left), cols[0]);
    frame.render_widget(right, cols[1]);
}

// ── Layout helpers ────────────────────────────────────────────────────────────

fn probe_panel_height(state: &AppState) -> u16 {
    let probe = match &state.probe {
        Some(p) => p,
        None => return 4,
    };
    let has_movement = probe.movement.as_ref().filter(|mv| {
        !matches!(
            mv.phase.as_ref().unwrap_or(&mv.status),
            MovementPhase::Arrived | MovementPhase::Failed | MovementPhase::Destroyed | MovementPhase::Idle
        )
    }).is_some();
    let content: u16 = 1  // name + status
        + if has_movement { 4 } else { 0 }  // coords + phase + travel + speed
        + 1  // fuel
        + if probe.systems.is_some() { 1 } else { 0 };  // integrity
    content + 2  // borders
}

fn inventory_panel_height(state: &AppState) -> u16 {
    let n_stocks = state.probe.as_ref()
        .map(|p| p.inventory.resource_stocks.len() as u16)
        .unwrap_or(0);
    1 + n_stocks + 2  // cargo gauge + stocks + borders
}

fn top_row_height(state: &AppState) -> u16 {
    probe_panel_height(state).max(inventory_panel_height(state))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn knowledge_label(k: &KnowledgeLevel) -> &'static str {
    match k {
        KnowledgeLevel::Detailed => "detailed",
        KnowledgeLevel::NeighborScan => "neighbor scan",
        KnowledgeLevel::DistantScan => "distant scan",
        KnowledgeLevel::LongRangeEstimation => "long range",
        KnowledgeLevel::Unknown => "?",
    }
}

fn knowledge_color(k: &KnowledgeLevel) -> Color {
    match k {
        KnowledgeLevel::Detailed => Color::Green,
        KnowledgeLevel::NeighborScan => Color::Cyan,
        KnowledgeLevel::DistantScan => Color::Yellow,
        KnowledgeLevel::LongRangeEstimation => Color::Red,
        KnowledgeLevel::Unknown => Color::DarkGray,
    }
}

fn freshness_label(f: &DataFreshness) -> &'static str {
    match f {
        DataFreshness::Live => "live",
        DataFreshness::DegradedLive => "degraded live",
        DataFreshness::Historical => "historical",
        DataFreshness::Unavailable => "unavailable",
        DataFreshness::Unknown => "?",
    }
}

fn freshness_color(f: &DataFreshness) -> Color {
    match f {
        DataFreshness::Live => Color::Green,
        DataFreshness::DegradedLive => Color::Yellow,
        DataFreshness::Historical => Color::DarkGray,
        DataFreshness::Unavailable => Color::Red,
        DataFreshness::Unknown => Color::DarkGray,
    }
}

fn object_icon(t: &SectorObjectType) -> (&'static str, Color) {
    match t {
        SectorObjectType::Star => ("★", Color::Yellow),
        SectorObjectType::Planet => ("●", Color::Cyan),
        SectorObjectType::Asteroid => ("◆", Color::White),
        SectorObjectType::DustCloud => ("~", Color::DarkGray),
        SectorObjectType::BlackHole => ("◉", Color::Magenta),
        SectorObjectType::SolarSystem => ("⊙", Color::Yellow),
        SectorObjectType::Manny => ("♟", Color::Green),
        SectorObjectType::Unknown => ("?", Color::DarkGray),
    }
}

fn panel_block(title: &str, focused: bool) -> Block<'_> {
    let (border_color, title_modifier) = if focused {
        (Color::Cyan, Modifier::BOLD)
    } else {
        (Color::DarkGray, Modifier::empty())
    };
    Block::default()
        .title(Span::styled(title, Style::default().add_modifier(title_modifier)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
}

fn probe_status_label(s: &ProbeStatus) -> &'static str {
    match s {
        ProbeStatus::Idle => "idle",
        ProbeStatus::Preparing => "preparing",
        ProbeStatus::Accelerating => "accelerating",
        ProbeStatus::Cruising => "cruising",
        ProbeStatus::Decelerating => "decelerating",
        ProbeStatus::Orbiting => "orbiting",
        ProbeStatus::Disabled => "disabled",
        ProbeStatus::Dead => "DEAD",
        ProbeStatus::Unknown => "?",
    }
}

fn probe_status_style(s: &ProbeStatus) -> Style {
    match s {
        ProbeStatus::Idle | ProbeStatus::Orbiting => Style::default().fg(Color::White),
        ProbeStatus::Preparing | ProbeStatus::Decelerating => Style::default().fg(Color::Yellow),
        ProbeStatus::Accelerating => {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        }
        ProbeStatus::Cruising => Style::default().fg(Color::Cyan),
        ProbeStatus::Disabled => Style::default().fg(Color::Red),
        ProbeStatus::Dead => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ProbeStatus::Unknown => Style::default().fg(Color::DarkGray),
    }
}

fn sensor_dot(m: &SensorMode) -> &'static str {
    match m {
        SensorMode::Normal | SensorMode::Degraded | SensorMode::Blind | SensorMode::Unknown => "●",
    }
}


fn sensor_style(m: &SensorMode) -> Style {
    match m {
        SensorMode::Normal => Style::default().fg(Color::Green),
        SensorMode::Degraded => Style::default().fg(Color::Yellow),
        SensorMode::Blind => Style::default().fg(Color::Red),
        SensorMode::Unknown => Style::default().fg(Color::DarkGray),
    }
}

fn movement_phase_label(p: &MovementPhase) -> &'static str {
    match p {
        MovementPhase::Idle => "idle",
        MovementPhase::Preparing => "preparing",
        MovementPhase::Accelerating => "accelerating",
        MovementPhase::Cruising => "cruising",
        MovementPhase::Decelerating => "decelerating",
        MovementPhase::Arrived => "arrived",
        MovementPhase::Failed => "failed",
        MovementPhase::Destroyed => "destroyed",
        MovementPhase::Unknown => "?",
    }
}

fn make_line_gauge(label: &str, ratio: f64, color: Color) -> LineGauge<'_> {
    LineGauge::default()
        .label(Line::raw(label.to_owned()))
        .filled_style(Style::default().fg(color))
        .unfilled_style(Style::default().fg(Color::DarkGray))
        .ratio(ratio)
}

fn gauge_color(ratio: f64) -> Color {
    if ratio > 0.5 {
        Color::Green
    } else if ratio > 0.25 {
        Color::Yellow
    } else {
        Color::Red
    }
}

pub fn format_duration(secs: i64) -> String {
    if secs <= 0 {
        return "arriving…".to_string();
    }
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{}h {:02}m {:02}s", h, m, s)
    } else if m > 0 {
        format!("{}m {:02}s", m, s)
    } else {
        format!("{}s", s)
    }
}
