use crate::api::types::{
    DangerLevel, DataFreshness, KnowledgeLevel, Manny, MannyLocationType, MannyTask,
    MovementPhase, ProbeStatus, SectorObject, SectorObjectType, SectorObservation, SensorMode,
};
use crate::app::{AppState, AtomicPrinterCraftInput, CraftInput, DeployInput, DetachInput, InspectInput, JettisonInput, MineInput, Panel, RecallInput, RecoverInput, RenameMannyInput, RepairInput, SalvageInput, ScanMode, TravelInput, RESOURCE_LABELS, RESOURCE_TYPES};
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
    if !matches!(state.repair, RepairInput::Inactive) {
        render_repair_overlay(frame, area, state);
    }
    if !matches!(state.mine, MineInput::Inactive) {
        render_mine_overlay(frame, area, state);
    }
    if state.map.open {
        render_map_overlay(frame, area, state);
    }
    if !matches!(state.jettison, JettisonInput::Inactive) {
        render_jettison_overlay(frame, area, state);
    }
    if !matches!(state.craft, CraftInput::Inactive) {
        render_craft_overlay(frame, area, state);
    }
    if !matches!(state.atomic_printer_craft, AtomicPrinterCraftInput::Inactive) {
        render_atomic_printer_craft_overlay(frame, area, state);
    }
    if !matches!(state.salvage, SalvageInput::Inactive) {
        render_salvage_overlay(frame, area, state);
    }
    if !matches!(state.recall, RecallInput::Inactive) {
        render_recall_overlay(frame, area, state);
    }
    if !matches!(state.deploy, DeployInput::Inactive) {
        render_deploy_overlay(frame, area, state);
    }
    if !matches!(state.rename_manny, RenameMannyInput::Inactive) {
        render_rename_manny_overlay(frame, area, state);
    }
    if !matches!(state.inspect, InspectInput::Inactive) {
        render_inspect_overlay(frame, area, state);
    }
    if !matches!(state.recover, RecoverInput::Inactive) {
        render_recover_overlay(frame, area, state);
    }
    if !matches!(state.detach, DetachInput::Inactive) {
        render_detach_overlay(frame, area, state);
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

    let show_sector = active_movement.is_none()
        && probe.sector.as_ref().and_then(|s| s.relative.as_ref()).is_some();

    let mut sections: Vec<Constraint> = vec![
        Constraint::Length(1), // name + status
    ];
    if show_sector {
        sections.push(Constraint::Length(1)); // current sector coords
    }
    if active_movement.is_some() {
        sections.push(Constraint::Length(1)); // coords + distance
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

    // ── Current sector ──
    if show_sector {
        if let Some(rel) = probe.sector.as_ref().and_then(|s| s.relative.as_ref()) {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("@ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("({},{},{})", rel.x as i64, rel.y as i64, rel.z as i64),
                        Style::default().fg(Color::White),
                    ),
                ])),
                rows[row],
            );
            row += 1;
        }
    }

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
                    "({},{},{}) → ({},{},{})  d:{}",
                    mv.origin.x as i64, mv.origin.y as i64, mv.origin.z as i64,
                    mv.target.x as i64, mv.target.y as i64, mv.target.z as i64,
                    mv.distance,
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
    let fuel_ratio = probe.fuel.deuterium
        .map(|d| (d / 100.0).clamp(0.0, 1.0))
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
    let full_inner = block.inner(area);
    frame.render_widget(block, area);

    let (inner, hint_area_opt) = if focused {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(full_inner);
        (split[0], Some(split[1]))
    } else {
        (full_inner, None)
    };

    if let Some(hint_area) = hint_area_opt {
        let mut hint_spans = vec![
            Span::styled("[j]", Style::default().fg(Color::Cyan)),
            Span::raw(" jettison"),
        ];
        if state.inventory_waypoint_bookmark_id().is_some() {
            hint_spans.push(Span::raw("  "));
            hint_spans.push(Span::styled("[d]", Style::default().fg(Color::Cyan)));
            hint_spans.push(Span::raw(" deploy"));
        }
        if state.has_atomic_printer() {
            hint_spans.push(Span::raw("  "));
            hint_spans.push(Span::styled("[a]", Style::default().fg(Color::Cyan)));
            hint_spans.push(Span::raw(" atomic craft"));
        }
        frame.render_widget(
            Paragraph::new(Line::from(hint_spans)),
            hint_area,
        );
    }

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

    let items_expanded = focused && !inv.items.is_empty();
    let items_rows: usize = items_row_count(&inv.items, items_expanded);
    let n_rows = 1 + inv.resource_stocks.len() + items_rows;

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
            "ice" => ("❄", Color::Cyan, "Ice"),
            "carbon_compounds" => ("◇", Color::Green, "Carbon"),
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

    // ── Items ──
    if items_expanded {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "── items ──",
                Style::default().fg(Color::DarkGray),
            ))),
            rows[row],
        );
        row += 1;

        // Active items: manny and atomic_3d_printer — show individually with task state
        for item in inv.items.iter().filter(|i| is_active_item(&i.item_type)) {
            let (icon, icon_color) = item_icon(&item.item_type);
            let (task_span, progress) = match item.current_task.as_deref() {
                None => (
                    Span::styled("idle", Style::default().fg(Color::DarkGray)),
                    String::new(),
                ),
                Some(t) => (
                    Span::styled(t.to_string(), Style::default().fg(Color::Yellow)),
                    format!(" {:3.0}%", item.task_progress_percent),
                ),
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(format!("  {icon} "), Style::default().fg(icon_color)),
                    Span::raw(format!("{:<14}", item.name)),
                    task_span,
                    Span::styled(progress, Style::default().fg(Color::DarkGray)),
                ])),
                rows[row],
            );
            row += 1;
        }

        // Passive items: group by type, show count
        let mut seen_types: Vec<&str> = Vec::new();
        for item in inv.items.iter().filter(|i| !is_active_item(&i.item_type)) {
            if seen_types.contains(&item.item_type.as_str()) {
                continue;
            }
            seen_types.push(&item.item_type);
            let count = inv.items.iter().filter(|i| i.item_type == item.item_type).count();
            let (icon, icon_color) = item_icon(&item.item_type);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(format!("  {icon} "), Style::default().fg(icon_color)),
                    Span::raw(format!("{:<14}", item.name)),
                    Span::styled(format!("× {count}"), Style::default().fg(Color::White)),
                ])),
                rows[row],
            );
            row += 1;
        }
    } else if !inv.items.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  items  ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{}", inv.items.len()), Style::default().fg(Color::White)),
                Span::styled("  (focus to expand)", Style::default().fg(Color::DarkGray)),
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

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    let list_area = rows[0];
    let hint_area = rows[1];

    // Hint bar
    if focused {
        let selected_manny = state.mannies.as_ref()
            .and_then(|m| m.get(state.mannies_selection));
        let can_order = selected_manny.map(|m| m.can_receive_orders).unwrap_or(false);
        let is_busy = selected_manny.map(|m| !m.can_receive_orders && m.current_task.is_some()).unwrap_or(false);
        if can_order {
            let has_detachable = !state.collect_detachable_containers().is_empty();
            let has_detached = !state.collect_detached_containers().is_empty();
            let mut spans = vec![
                Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
                Span::raw(" repair  "),
                Span::styled("[e]", Style::default().fg(Color::Cyan)),
                Span::raw(" mine  "),
                Span::styled("[c]", Style::default().fg(Color::Cyan)),
                Span::raw(" craft  "),
                Span::styled("[s]", Style::default().fg(Color::Cyan)),
                Span::raw(" salvage  "),
                Span::styled("[x]", Style::default().fg(Color::Cyan)),
                Span::raw(" inspect"),
            ];
            if has_detachable {
                spans.push(Span::raw("  "));
                spans.push(Span::styled("[D]", Style::default().fg(Color::Cyan)));
                spans.push(Span::raw(" detach"));
            }
            if has_detached {
                spans.push(Span::raw("  "));
                spans.push(Span::styled("[v]", Style::default().fg(Color::Cyan)));
                spans.push(Span::raw(" recover"));
            }
            spans.push(Span::raw("  "));
            spans.push(Span::styled("[n]", Style::default().fg(Color::Cyan)));
            spans.push(Span::raw(" rename"));
            if is_busy {
                spans.push(Span::raw("  "));
                spans.push(Span::styled("[R]", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" recall"));
            }
            frame.render_widget(Paragraph::new(Line::from(spans)), hint_area);
        } else if is_busy {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("busy  ", Style::default().fg(Color::DarkGray)),
                    Span::styled("[R]", Style::default().fg(Color::Yellow)),
                    Span::raw(" recall  "),
                    Span::styled("[n]", Style::default().fg(Color::Cyan)),
                    Span::raw(" rename"),
                ])),
                hint_area,
            );
        } else {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("busy — cannot receive orders  ", Style::default().fg(Color::DarkGray)),
                    Span::styled("[n]", Style::default().fg(Color::Cyan)),
                    Span::raw(" rename"),
                ])),
                hint_area,
            );
        }
    }

    let Some(mannies) = &state.mannies else {
        frame.render_widget(
            Paragraph::new("No data").style(Style::default().fg(Color::DarkGray)),
            list_area,
        );
        return;
    };

    if mannies.is_empty() {
        frame.render_widget(
            Paragraph::new("No mannies aboard").style(Style::default().fg(Color::DarkGray)),
            list_area,
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

    frame.render_stateful_widget(list, list_area, &mut list_state);
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
        Some(MannyTask::Crafting) => Span::styled("crafting", Style::default().fg(Color::Cyan)),
        Some(MannyTask::AssistingAtomicPrinter) => Span::styled("assisting printer", Style::default().fg(Color::Cyan)),
        Some(MannyTask::Salvage) => Span::styled("salvage", Style::default().fg(Color::Yellow)),
        Some(MannyTask::InstallingWaypointBookmark) => Span::styled("installing waypoint", Style::default().fg(Color::Green)),
        Some(MannyTask::DetachingStorageContainer) => Span::styled("detaching container", Style::default().fg(Color::Yellow)),
        Some(MannyTask::InspectingAsteroid) => Span::styled("inspecting", Style::default().fg(Color::Yellow)),
        Some(MannyTask::Returning) => Span::styled("returning", Style::default().fg(Color::Blue)),
        Some(MannyTask::WaitingForSpace) => Span::styled("waiting", Style::default().fg(Color::Magenta)),
        Some(MannyTask::MovingStockage) => Span::styled("moving cargo", Style::default().fg(Color::Blue)),
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
                    s.push(Span::raw("  "));
                    s.push(Span::styled("[J/K]", Style::default().fg(Color::Cyan)));
                    s.push(Span::raw(" scroll"));
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

fn estimate_mine_duration(target_amount: f64) -> (i64, i64) {
    const CARGO_CAP: f64 = 0.30;
    const TRAVEL_SECS: i64 = 1800; // 900s each way
    const TICK_AMOUNT: f64 = 0.01;
    const TICK_SECS: i64 = 300;
    let trips = (target_amount / CARGO_CAP).ceil() as i64;
    let mut remaining = target_amount;
    let mut total_secs: i64 = 0;
    for _ in 0..trips {
        let trip = remaining.min(CARGO_CAP);
        let ticks = (trip / TICK_AMOUNT).ceil() as i64;
        total_secs += TRAVEL_SECS + ticks * TICK_SECS;
        remaining -= trip;
    }
    (trips, total_secs)
}

fn render_mine_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    match &state.mine {
        MineInput::PickAsteroid { manny_name, candidates, selection, .. } => {
            let height = (candidates.len() as u16 + 6).min(16);
            let popup = centered_rect(50, height, area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(format!(" MINE — {manny_name} "))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let mut lines: Vec<Line> = vec![
                Line::from(Span::styled("Select mining target:", Style::default().fg(Color::Cyan))),
                Line::default(),
            ];
            for (i, (_, name)) in candidates.iter().enumerate() {
                let selected = i == *selection;
                if selected {
                    lines.push(Line::from(vec![
                        Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                        Span::styled(name.as_str(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(name.as_str(), Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("[↑/↓]", Style::default().fg(Color::Cyan)),
                    Span::raw(" select  "),
                    Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw(" confirm  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])),
                rows[1],
            );
        }

        MineInput::Configure { manny_name, object_name, resources, amount_buf, amount_mode, error, .. } => {
            let popup = centered_rect(52, 14, area);
            frame.render_widget(Clear, popup);

            let manny_short = if manny_name.len() > 10 { &manny_name[..10] } else { manny_name };
            let obj_short = if object_name.len() > 12 { &object_name[..12] } else { object_name };
            let title = format!(" MINE — {manny_short} → {obj_short} ");
            let block = Block::default()
                .title(title)
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let mut lines: Vec<Line> = Vec::new();

            // Resources section
            let res_header_color = if *amount_mode { Color::DarkGray } else { Color::Cyan };
            lines.push(Line::from(vec![
                Span::styled("Resources", Style::default().fg(res_header_color)),
                Span::styled(
                    if *amount_mode { "  (Tab to edit)" } else { "  [1-4 to toggle]" },
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
            for (i, (&label, &_type_str)) in RESOURCE_LABELS.iter().zip(RESOURCE_TYPES.iter()).enumerate() {
                let checked = if resources[i] { "[✓]" } else { "[ ]" };
                let (checked_color, label_color) = if resources[i] {
                    (Color::Green, Color::White)
                } else {
                    (Color::DarkGray, Color::DarkGray)
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {checked} "), Style::default().fg(checked_color)),
                    Span::styled(format!("{} ", i + 1), Style::default().fg(Color::DarkGray)),
                    Span::styled(label, Style::default().fg(label_color)),
                ]));
            }

            lines.push(Line::default());

            // Amount section
            let amt_header_color = if *amount_mode { Color::Cyan } else { Color::DarkGray };
            if *amount_mode {
                lines.push(Line::from(vec![
                    Span::styled("Amount: ", Style::default().fg(amt_header_color)),
                    Span::raw(amount_buf.as_str()),
                    Span::styled("█", Style::default().fg(Color::Cyan)),
                    Span::styled(" ECE", Style::default().fg(Color::DarkGray)),
                    Span::raw("  "),
                    Span::styled("[M]", Style::default().fg(Color::Yellow)),
                    Span::styled(" max", Style::default().fg(Color::DarkGray)),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("Amount: ", Style::default().fg(amt_header_color)),
                    Span::styled(amount_buf.as_str(), Style::default().fg(Color::DarkGray)),
                    Span::styled(" ECE", Style::default().fg(Color::DarkGray)),
                    Span::styled("  [Tab to edit]", Style::default().fg(Color::DarkGray)),
                ]));
            }

            // Time estimate
            if let Ok(amount) = amount_buf.parse::<f64>() {
                if amount > 0.0 {
                    let (trips, secs) = estimate_mine_duration(amount);
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("{trips} trip{}  •  ~{}", if trips == 1 { "" } else { "s" }, format_duration(secs)),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                } else {
                    lines.push(Line::default());
                }
            } else {
                lines.push(Line::default());
            }

            // Error
            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    format!("✗ {err}"),
                    Style::default().fg(Color::Red),
                )));
            }

            frame.render_widget(Paragraph::new(lines), rows[0]);

            // Hint bar
            let any_resource = resources.iter().any(|&r| r);
            let valid_amount = amount_buf.parse::<f64>().ok().filter(|&v| v > 0.0).is_some();
            let can_send = any_resource && valid_amount;
            let hint = if *amount_mode {
                Line::from(vec![
                    Span::styled("[Tab]", Style::default().fg(Color::Cyan)),
                    Span::raw(" resources  "),
                    Span::styled("[Enter]", if can_send { Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) }),
                    Span::raw(" send  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])
            } else {
                Line::from(vec![
                    Span::styled("[1-4]", Style::default().fg(Color::Cyan)),
                    Span::raw(" toggle  "),
                    Span::styled("[Tab]", Style::default().fg(Color::Cyan)),
                    Span::raw(" amount  "),
                    Span::styled("[Enter]", if can_send { Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) }),
                    Span::raw(" send  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])
            };
            frame.render_widget(Paragraph::new(hint), rows[1]);
        }

        MineInput::Inactive => {}
    }
}

fn render_repair_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let RepairInput::Typing { ref manny_name, ref buf, ref error, .. } = state.repair else { return };

    let max_pct = state.repair_max_percent();
    let metals_stock = state.repair_metals_stock();

    let parsed = buf.parse::<f64>().ok().filter(|&v| v > 0.0);
    let effective = parsed.map(|v| v.min(max_pct));
    let metals_cost = effective.map(|v| v * 0.01);
    let duration_secs = effective.map(|v| (v * 600.0) as i64);
    let insufficient = metals_cost.is_some_and(|c| c > metals_stock + 1e-6);

    let popup = centered_rect(46, 12, area);
    frame.render_widget(Clear, popup);

    let title = format!(" REPAIR — {manny_name} ");
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    let body = rows[0];
    let hint_area = rows[1];

    let mut lines: Vec<Line> = Vec::new();

    // Input line
    lines.push(Line::from(vec![
        Span::styled("Restore: ", Style::default().fg(Color::Cyan)),
        Span::raw(buf.as_str()),
        Span::styled("█", Style::default().fg(Color::Cyan)),
        Span::styled("%", Style::default().fg(Color::DarkGray)),
    ]));

    lines.push(Line::default());

    // MAX hint
    lines.push(Line::from(vec![
        Span::styled("MAX  ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{max_pct:.2}%"), Style::default().fg(Color::White)),
        Span::raw("   "),
        Span::styled("[M]", Style::default().fg(Color::Yellow)),
        Span::styled(" fill", Style::default().fg(Color::DarkGray)),
    ]));

    lines.push(Line::default());

    // Cost preview (only when input is parseable)
    if let (Some(metals), Some(secs)) = (metals_cost, duration_secs) {
        let metals_color = if insufficient { Color::Red } else { Color::White };
        lines.push(Line::from(vec![
            Span::styled("Metals  ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{metals:.4}"), Style::default().fg(metals_color)),
            Span::styled(" ECE", Style::default().fg(Color::DarkGray)),
            if insufficient {
                Span::styled(
                    format!("  (have {metals_stock:.4})"),
                    Style::default().fg(Color::Red),
                )
            } else {
                Span::raw("")
            },
        ]));
        lines.push(Line::from(vec![
            Span::styled("Time    ", Style::default().fg(Color::DarkGray)),
            Span::styled(format_duration(secs), Style::default().fg(Color::Yellow)),
        ]));
        if let Some(eff) = effective {
            if parsed.is_some_and(|v| v > max_pct + 0.001) {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  → capped at {eff:.2}% (probe already at max above)"),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "type a value to see cost",
            Style::default().fg(Color::DarkGray),
        )));
    }

    // API error
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(Color::Red),
        )));
    }

    frame.render_widget(Paragraph::new(lines), body);

    // Hint bar
    let can_send = parsed.is_some() && !insufficient;
    let hint = if can_send {
        Line::from(vec![
            Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" send  "),
            Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ])
    } else {
        Line::from(vec![
            Span::styled("[Enter]", Style::default().fg(Color::DarkGray)),
            Span::raw(" send  "),
            Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ])
    };
    frame.render_widget(Paragraph::new(hint), hint_area);
}

fn render_jettison_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    match &state.jettison {
        JettisonInput::PickItem { items, selection } => {
            let height = (items.len() as u16 + 6).min(20);
            let popup = centered_rect(50, height, area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(" JETTISON ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let mut lines: Vec<Line> = Vec::new();
            for (i, (id, label, is_manny)) in items.iter().enumerate() {
                let selected = i == *selection;
                let (icon, icon_color) = if *is_manny {
                    ("♟", Color::Green)
                } else if id.contains("metals") {
                    ("◆", Color::White)
                } else if id.contains("ice") {
                    ("❄", Color::Cyan)
                } else if id.contains("carbon") {
                    ("◇", Color::Green)
                } else {
                    ("◆", Color::White)
                };
                if selected {
                    lines.push(Line::from(vec![
                        Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                        Span::styled(icon, Style::default().fg(icon_color)),
                        Span::raw(" "),
                        Span::styled(label.as_str(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(icon, Style::default().fg(icon_color)),
                        Span::raw(" "),
                        Span::styled(label.as_str(), Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("[↑/↓]", Style::default().fg(Color::Cyan)),
                    Span::raw(" select  "),
                    Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw(" confirm  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])),
                rows[1],
            );
        }

        JettisonInput::ConfirmManny { manny_name, error, .. } => {
            let popup = centered_rect(48, 8, area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(format!(" JETTISON — {manny_name} "))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let mut lines: Vec<Line> = Vec::new();
            lines.push(Line::from(Span::styled(
                "Eject manny into the sector?",
                Style::default().fg(Color::Red),
            )));
            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    format!("✗ {err}"),
                    Style::default().fg(Color::Red),
                )));
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("[Enter]", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    Span::raw(" EJECT  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])),
                rows[1],
            );
        }

        JettisonInput::EnterAmount { item_name, max_amount, buf, error, .. } => {
            let popup = centered_rect(46, 8, area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(format!(" JETTISON — {item_name} "))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let mut lines: Vec<Line> = Vec::new();
            lines.push(Line::from(vec![
                Span::styled("Amount: ", Style::default().fg(Color::Cyan)),
                Span::raw(buf.as_str()),
                Span::styled("█", Style::default().fg(Color::Cyan)),
                Span::styled(" ECE", Style::default().fg(Color::DarkGray)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("MAX  ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{max_amount:.3}"), Style::default().fg(Color::White)),
                Span::styled("  [empty = all]", Style::default().fg(Color::DarkGray)),
            ]));
            if let Some(err) = error {
                lines.push(Line::from(Span::styled(
                    format!("✗ {err}"),
                    Style::default().fg(Color::Red),
                )));
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw(" confirm  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])),
                rows[1],
            );
        }

        JettisonInput::Inactive => {}
    }
}

fn render_craft_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let CraftInput::PickRecipe { ref manny_name, selection, ref error, .. } = state.craft else { return };

    let recipes = state.manny_craft_recipes();
    let height = (recipes.len() as u16 + 6).min(16);
    let popup = centered_rect(58, height, area);
    frame.render_widget(Clear, popup);

    let title = format!(" CRAFT — {manny_name} ");
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::new();
    if recipes.is_empty() {
        lines.push(Line::from(Span::styled(
            "loading recipes…",
            Style::default().fg(Color::DarkGray),
        )));
    }
    for (i, recipe) in recipes.iter().enumerate() {
        let selected = i == selection;
        let duration_min = recipe.duration_seconds / 60;
        let ingredients: String = recipe.ingredients.iter().map(|ing| {
            if ing.unit == "item" {
                format!("{} × {}", ing.quantity as u32, ing.ingredient_type)
            } else {
                format!("{:.2} ECE {}", ing.quantity, ing.ingredient_type)
            }
        }).collect::<Vec<_>>().join(", ");
        let detail = format!("  {}m  {}", duration_min, ingredients);
        if selected {
            lines.push(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                Span::styled(recipe.name.as_str(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled(detail, Style::default().fg(Color::DarkGray)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(recipe.name.as_str(), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(Color::Red),
        )));
    }

    frame.render_widget(Paragraph::new(lines), rows[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[↑/↓]", Style::default().fg(Color::Cyan)),
            Span::raw(" select  "),
            Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" start  "),
            Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ])),
        rows[1],
    );
}

fn render_atomic_printer_craft_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let AtomicPrinterCraftInput::PickRecipe { selection, ref error } = state.atomic_printer_craft else { return };

    let recipes = state.atomic_printer_recipes();
    let height = (recipes.len() as u16 + 6).min(16);
    let popup = centered_rect(58, height, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" ATOMIC PRINTER — SELECT RECIPE ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::new();
    if recipes.is_empty() {
        lines.push(Line::from(Span::styled(
            "loading recipes…",
            Style::default().fg(Color::DarkGray),
        )));
    }
    for (i, recipe) in recipes.iter().enumerate() {
        let selected = i == selection;
        let duration_min = recipe.duration_seconds / 60;
        let ingredients: String = recipe.ingredients.iter().map(|ing| {
            if ing.unit == "item" {
                format!("{} × {}", ing.quantity as u32, ing.ingredient_type)
            } else {
                format!("{:.2} ECE {}", ing.quantity, ing.ingredient_type)
            }
        }).collect::<Vec<_>>().join(", ");
        let detail = format!("  {}m  {}", duration_min, ingredients);
        if selected {
            lines.push(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                Span::styled(recipe.name.as_str(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled(detail, Style::default().fg(Color::DarkGray)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(recipe.name.as_str(), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(Color::Red),
        )));
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[↑/↓]", Style::default().fg(Color::Cyan)),
            Span::raw(" select  "),
            Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" start  "),
            Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ])),
        rows[1],
    );
}

fn render_salvage_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    match &state.salvage {
        SalvageInput::PickTarget { manny_name, candidates, selection, .. } => {
            let height = (candidates.len() as u16 + 6).min(16);
            let popup = centered_rect(50, height, area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(format!(" SALVAGE — {manny_name} "))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let mut lines: Vec<Line> = vec![
                Line::from(Span::styled("Select salvage target:", Style::default().fg(Color::Cyan))),
                Line::default(),
            ];
            for (i, (_, name)) in candidates.iter().enumerate() {
                let selected = i == *selection;
                if selected {
                    lines.push(Line::from(vec![
                        Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                        Span::styled(name.as_str(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(name.as_str(), Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("[↑/↓]", Style::default().fg(Color::Cyan)),
                    Span::raw(" select  "),
                    Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw(" confirm  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])),
                rows[1],
            );
        }

        SalvageInput::Confirm { manny_name, object_name, error, .. } => {
            let popup = centered_rect(50, 8, area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(format!(" SALVAGE — {manny_name} "))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let mut lines: Vec<Line> = Vec::new();
            lines.push(Line::from(vec![
                Span::styled("Target: ", Style::default().fg(Color::DarkGray)),
                Span::styled(object_name.as_str(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]));
            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    format!("✗ {err}"),
                    Style::default().fg(Color::Red),
                )));
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw(" SALVAGE  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])),
                rows[1],
            );
        }

        SalvageInput::Inactive => {}
    }
}

fn render_recall_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let RecallInput::Confirm { ref manny_name, ref error, .. } = state.recall else { return };

    let popup = centered_rect(46, 7, area);
    frame.render_widget(Clear, popup);

    let title = format!(" RECALL — {manny_name} ");
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        "Send recall order?",
        Style::default().fg(Color::White),
    )));
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(Color::Red),
        )));
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[Enter]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" RECALL  "),
            Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ])),
        rows[1],
    );
}

fn render_rename_manny_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let RenameMannyInput::Typing { ref manny_name, ref buf, ref error, .. } = state.rename_manny else { return };

    let popup = centered_rect(46, 7, area);
    frame.render_widget(Clear, popup);

    let title = format!(" RENAME — {manny_name} ");
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("Name: ", Style::default().fg(Color::Cyan)),
        Span::raw(buf.as_str()),
        Span::styled("█", Style::default().fg(Color::Cyan)),
    ]));
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(Color::Red),
        )));
    }

    frame.render_widget(Paragraph::new(lines), rows[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" rename  "),
            Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ])),
        rows[1],
    );
}

fn render_deploy_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    match &state.deploy {
        DeployInput::PickManny { mannies, selection } => {
            let height = (mannies.len() as u16 + 6).min(18);
            let popup = centered_rect(52, height, area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(" DEPLOY WAYPOINT — SELECT MANNY ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let mut lines: Vec<Line> = Vec::new();
            for (i, (_, name)) in mannies.iter().enumerate() {
                let selected = i == *selection;
                if selected {
                    lines.push(Line::from(vec![
                        Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                        Span::styled(name.as_str(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(name.as_str(), Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("[↑/↓]", Style::default().fg(Color::Cyan)),
                    Span::raw(" select  "),
                    Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw(" confirm  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])),
                rows[1],
            );
        }

        DeployInput::PickObject { candidates, selection, .. } => {
            let height = (candidates.len() as u16 + 6).min(18);
            let popup = centered_rect(52, height, area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(" DEPLOY WAYPOINT ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let mut lines: Vec<Line> = Vec::new();
            for (i, (_, name)) in candidates.iter().enumerate() {
                let selected = i == *selection;
                if selected {
                    lines.push(Line::from(vec![
                        Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                        Span::styled(name.as_str(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(name.as_str(), Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("[↑/↓]", Style::default().fg(Color::Cyan)),
                    Span::raw(" select  "),
                    Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw(" confirm  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])),
                rows[1],
            );
        }

        DeployInput::EnterName { object_name, name_buf, error, .. } => {
            let popup = centered_rect(52, 8, area);
            frame.render_widget(Clear, popup);
            let title = format!(" DEPLOY WAYPOINT — {object_name} ");
            let block = Block::default()
                .title(title)
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let mut lines: Vec<Line> = Vec::new();
            lines.push(Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Color::Cyan)),
                Span::raw(name_buf.as_str()),
                Span::styled("█", Style::default().fg(Color::Cyan)),
            ]));
            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    format!("✗ {err}"),
                    Style::default().fg(Color::Red),
                )));
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw(" DEPLOY  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])),
                rows[1],
            );
        }

        DeployInput::Inactive => {}
    }
}

fn render_inspect_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let InspectInput::PickAsteroid { ref manny_name, ref candidates, selection, .. } = state.inspect else { return };

    let height = (candidates.len() as u16 + 6).min(16);
    let popup = centered_rect(52, height, area);
    frame.render_widget(Clear, popup);

    let title = format!(" INSPECT — {manny_name} ");
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled("Select asteroid to inspect:", Style::default().fg(Color::Cyan))),
        Line::default(),
    ];
    for (i, (_, name)) in candidates.iter().enumerate() {
        let selected = i == selection;
        if selected {
            lines.push(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                Span::styled(name.as_str(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(name.as_str(), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[↑/↓]", Style::default().fg(Color::Cyan)),
            Span::raw(" select  "),
            Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" inspect  "),
            Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ])),
        rows[1],
    );
}

fn render_recover_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let RecoverInput::PickContainer { ref manny_name, ref candidates, selection, .. } = state.recover else { return };

    let height = (candidates.len() as u16 + 6).min(16);
    let popup = centered_rect(52, height, area);
    frame.render_widget(Clear, popup);

    let title = format!(" RECOVER — {manny_name} ");
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled("Select container to recover:", Style::default().fg(Color::Cyan))),
        Line::default(),
    ];
    for (i, (_, name)) in candidates.iter().enumerate() {
        let selected = i == selection;
        if selected {
            lines.push(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                Span::styled(name.as_str(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(name.as_str(), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[↑/↓]", Style::default().fg(Color::Cyan)),
            Span::raw(" select  "),
            Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" recover  "),
            Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ])),
        rows[1],
    );
}

fn render_detach_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    match &state.detach {
        DetachInput::PickContainer { manny_name, containers, selection, .. } => {
            let height = (containers.len() as u16 + 6).min(16);
            let popup = centered_rect(52, height, area);
            frame.render_widget(Clear, popup);
            let title = format!(" DETACH — {manny_name} ");
            let block = Block::default()
                .title(title).title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);
            let rows = Layout::default().direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)]).split(inner);
            let mut lines: Vec<Line> = vec![
                Line::from(Span::styled("Select container to detach:", Style::default().fg(Color::Cyan))),
                Line::default(),
            ];
            for (i, (_, name)) in containers.iter().enumerate() {
                if i == *selection {
                    lines.push(Line::from(vec![
                        Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                        Span::styled(name.as_str(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]));
                } else {
                    lines.push(Line::from(vec![Span::raw("  "), Span::styled(name.as_str(), Style::default().fg(Color::DarkGray))]));
                }
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            frame.render_widget(Paragraph::new(Line::from(vec![
                Span::styled("[↑/↓]", Style::default().fg(Color::Cyan)), Span::raw(" select  "),
                Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)), Span::raw(" next  "),
                Span::styled("[Esc]", Style::default().fg(Color::Cyan)), Span::raw(" cancel"),
            ])), rows[1]);
        }

        DetachInput::PickMode { manny_name, container_name, selection, error, .. } => {
            let popup = centered_rect(52, 10, area);
            frame.render_widget(Clear, popup);
            let title = format!(" DETACH — {container_name} ");
            let block = Block::default()
                .title(title).title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);
            let rows = Layout::default().direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)]).split(inner);

            let mut lines: Vec<Line> = vec![
                Line::from(Span::styled(format!("Detach mode  (manny: {manny_name})"), Style::default().fg(Color::Cyan))),
                Line::default(),
            ];
            for (i, (_, label)) in crate::DETACH_MODES.iter().enumerate() {
                if i == *selection {
                    lines.push(Line::from(vec![
                        Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                        Span::styled(*label, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]));
                } else {
                    lines.push(Line::from(vec![Span::raw("  "), Span::styled(*label, Style::default().fg(Color::DarkGray))]));
                }
            }
            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(format!("✗ {err}"), Style::default().fg(Color::Red))));
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            frame.render_widget(Paragraph::new(Line::from(vec![
                Span::styled("[↑/↓]", Style::default().fg(Color::Cyan)), Span::raw(" select  "),
                Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)), Span::raw(" confirm  "),
                Span::styled("[Esc]", Style::default().fg(Color::Cyan)), Span::raw(" cancel"),
            ])), rows[1]);
        }

        DetachInput::PickAsteroid { manny_name, container_name, asteroids, selection, error, .. } => {
            let height = (asteroids.len() as u16 + 8).min(18);
            let popup = centered_rect(52, height, area);
            frame.render_widget(Clear, popup);
            let title = format!(" DETACH — hide {container_name} ");
            let block = Block::default()
                .title(title).title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);
            let rows = Layout::default().direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)]).split(inner);

            let mut lines: Vec<Line> = vec![
                Line::from(Span::styled(format!("Attach to asteroid  (manny: {manny_name})"), Style::default().fg(Color::Cyan))),
                Line::default(),
            ];
            for (i, (_, name)) in asteroids.iter().enumerate() {
                if i == *selection {
                    lines.push(Line::from(vec![
                        Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                        Span::styled(name.as_str(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]));
                } else {
                    lines.push(Line::from(vec![Span::raw("  "), Span::styled(name.as_str(), Style::default().fg(Color::DarkGray))]));
                }
            }
            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(format!("✗ {err}"), Style::default().fg(Color::Red))));
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            frame.render_widget(Paragraph::new(Line::from(vec![
                Span::styled("[↑/↓]", Style::default().fg(Color::Cyan)), Span::raw(" select  "),
                Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)), Span::raw(" hide here  "),
                Span::styled("[Esc]", Style::default().fg(Color::Cyan)), Span::raw(" cancel"),
            ])), rows[1]);
        }

        DetachInput::Inactive => {}
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

    if s.possible_objects.as_ref().is_some_and(|p| !p.is_empty()) {
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

    let salvageable = obj.salvageable.unwrap_or(false);

    let mut main_spans = vec![
        Span::styled(icon, Style::default().fg(color)),
        Span::raw(" "),
        Span::styled(estimated, Style::default().fg(Color::DarkGray)),
        Span::raw(format!("{name}{danger}")),
    ];
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
            let mut spans = vec![
                Span::styled("  ", Style::default().fg(Color::DarkGray)),
                Span::styled(icon, Style::default().fg(color)),
                Span::raw(" "),
                Span::raw(name.to_string()),
            ];
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
        let mut spans = vec![
            Span::styled("  ", Style::default().fg(Color::DarkGray)),
            Span::styled(icon, Style::default().fg(color)),
            Span::raw(" "),
            Span::raw(name.to_string()),
        ];
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

// ── Map overlay ───────────────────────────────────────────────────────────────

fn map_cell_symbol(s: &SectorObservation) -> (&'static str, Style) {
    if let Some(objects) = &s.objects {
        for obj in objects {
            if matches!(obj.object_type, SectorObjectType::BlackHole) {
                return ("◉", Style::default().fg(Color::Magenta));
            }
            if matches!(obj.danger_level, Some(DangerLevel::Extreme)) {
                return ("!", Style::default().fg(Color::Red));
            }
            if matches!(obj.object_type, SectorObjectType::Star | SectorObjectType::SolarSystem) {
                let has_minable = obj.minable_targets.as_ref()
                    .is_some_and(|t| !t.is_empty());
                return if has_minable {
                    ("★", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                } else {
                    ("★", Style::default().fg(Color::Yellow))
                };
            }
        }
        return ("●", Style::default().fg(Color::Green));
    }
    ("·", Style::default().fg(Color::White))
}

fn render_map_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    use std::collections::HashMap;

    let popup = centered_rect(66, 24, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(Span::styled(
            format!(
                " MAP  ({},{},{})  y:{} ",
                state.map.center_x, state.map.y_layer, state.map.center_z,
                state.map.y_layer,
            ),
            Style::default().fg(Color::Cyan),
        ))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let map_inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(map_inner);
    let map_area = rows_layout[0];
    let hint_area = rows_layout[1];

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[hjkl]", Style::default().fg(Color::Cyan)),
            Span::raw(" pan  "),
            Span::styled("[u/d]", Style::default().fg(Color::Cyan)),
            Span::raw(" y±1  "),
            Span::styled("[g]", Style::default().fg(Color::Yellow)),
            Span::raw(" travel  "),
            Span::styled("[b/Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" close"),
        ])),
        hint_area,
    );

    // Fast lookup: (x,y,z) → sector
    let sector_lookup: HashMap<(i32, i32, i32), &SectorObservation> = state
        .scan_history
        .iter()
        .map(|s| {
            (
                (
                    s.relative_coordinates.x as i32,
                    s.relative_coordinates.y as i32,
                    s.relative_coordinates.z as i32,
                ),
                s,
            )
        })
        .collect();

    let probe_coords = state
        .probe
        .as_ref()
        .and_then(|p| p.sector.as_ref())
        .and_then(|s| s.relative.as_ref())
        .map(|r| (r.x as i32, r.y as i32, r.z as i32));

    let cx = state.map.center_x;
    let cz = state.map.center_z;
    let y = state.map.y_layer;
    let w = map_area.width as i32;
    let h = map_area.height as i32;
    let center_col = map_area.x as i32 + w / 2;
    let center_row = map_area.y as i32 + h / 2;

    // Projection:
    //   term_col = center_col + (dx - dz) * 2
    //   term_row = center_row - (dx + dz) / 2
    // Valid cells: (dx+dz) even, col_off multiple of 4.

    let buf = frame.buffer_mut();

    for tr in map_area.y..(map_area.y + map_area.height) {
        let row_off = tr as i32 - center_row;
        let dsum = -2 * row_off; // dx + dz

        let col_off_min = map_area.x as i32 - center_col;
        let rem = col_off_min.rem_euclid(4);
        let col_off_start = if rem == 0 { col_off_min } else { col_off_min + (4 - rem) };
        let col_off_max = (map_area.x as i32 + w - 1) - center_col;

        let mut col_off = col_off_start;
        while col_off <= col_off_max {
            let tc = (center_col + col_off) as u16;
            let ddiff = col_off / 2; // dx - dz
            let dx = (dsum + ddiff) / 2;
            let dz = (dsum - ddiff) / 2;
            let x = cx + dx;
            let z = cz + dz;

            let is_probe = probe_coords == Some((x, y, z));
            let is_center = dx == 0 && dz == 0;

            let (sym, style) = if is_probe {
                ("⊕", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            } else if let Some(sector) = sector_lookup.get(&(x, y, z)) {
                let (s, st) = map_cell_symbol(sector);
                if is_center {
                    (s, st.add_modifier(Modifier::REVERSED))
                } else {
                    (s, st)
                }
            } else if is_center {
                ("+", Style::default().fg(Color::DarkGray).add_modifier(Modifier::REVERSED))
            } else {
                ("·", Style::default().fg(Color::DarkGray))
            };

            buf.set_string(tc, tr, sym, style);
            col_off += 4;
        }
    }
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
        Span::styled("[b]", Style::default().fg(Color::Cyan)),
        Span::raw(" map  "),
        Span::styled("[q]", Style::default().fg(Color::Cyan)),
        Span::raw(" quit"),
        Span::styled(error_part, Style::default().fg(Color::Red)),
    ]);

    let app_version = env!("CARGO_PKG_VERSION");
    let api_version = state.api_version
        .map(|v| format!("API v{v}  "))
        .unwrap_or_default();
    let right_text = format!("v{app_version}  {api_version}⟳ {last}   next: {next}");
    let right_len = right_text.chars().count() as u16;
    let right = Paragraph::new(right_text)
        .alignment(Alignment::Right)
        .style(Style::default().fg(Color::DarkGray));

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(right_len)])
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
    let show_sector = !has_movement
        && probe.sector.as_ref().and_then(|s| s.relative.as_ref()).is_some();
    let content: u16 = 1  // name + status
        + if show_sector { 1 } else { 0 }   // current sector coords
        + if has_movement { 4 } else { 0 }  // coords + phase + travel + speed
        + 1  // fuel
        + if probe.systems.is_some() { 1 } else { 0 };  // integrity
    content + 2  // borders
}

fn inventory_panel_height(state: &AppState) -> u16 {
    let probe = match &state.probe {
        Some(p) => p,
        None => return 3,
    };
    let inv = &probe.inventory;
    let focused = state.focused == Some(Panel::Inventory);
    let n_stocks = inv.resource_stocks.len() as u16;
    let expanded = focused && !inv.items.is_empty();
    let items_rows = items_row_count(&inv.items, expanded) as u16;
    1 + n_stocks + items_rows + 2
}

fn top_row_height(state: &AppState) -> u16 {
    probe_panel_height(state).max(inventory_panel_height(state))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn is_active_item(item_type: &str) -> bool {
    matches!(item_type, "manny" | "atomic_3d_printer")
}

fn item_icon(item_type: &str) -> (&'static str, Color) {
    match item_type {
        "manny" => ("♟", Color::Green),
        "atomic_3d_printer" => ("⚙", Color::Magenta),
        "additional_container" => ("□", Color::Cyan),
        "waypoint_bookmark" => ("◎", Color::Cyan),
        "micro_conductor" | "ceramic_insulator" | "crystal_substrate"
        | "dopant_matrix" | "integrated_circuit" => ("◈", Color::Yellow),
        _ => ("◈", Color::White),
    }
}

fn items_row_count(items: &[crate::api::types::ProbeInventoryItem], expanded: bool) -> usize {
    if items.is_empty() { return 0; }
    if !expanded { return 1; }
    let n_active = items.iter().filter(|i| is_active_item(&i.item_type)).count();
    let mut seen: Vec<&str> = Vec::new();
    for item in items.iter().filter(|i| !is_active_item(&i.item_type)) {
        if !seen.contains(&item.item_type.as_str()) {
            seen.push(&item.item_type);
        }
    }
    1 + n_active + seen.len()
}

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
        SectorObjectType::DriftingItem => ("◌", Color::White),
        SectorObjectType::DetachedContainer => ("□", Color::Cyan),
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
