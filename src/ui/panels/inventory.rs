use crate::app::{is_active_item, AppState, Panel};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::theme::{gauge_color, item_icon, make_line_gauge, panel_block};
// ── Inventory panel ───────────────────────────────────────────────────────────

pub(crate) fn render_inventory_panel(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
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
            Span::styled("[↑↓]", Style::default().fg(Color::Cyan)),
            Span::raw(" select  "),
            Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
            Span::raw(" detail  "),
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
    let containers_rows = containers_row_count(inv, focused);
    let tanks_rows = tanks_row_count(inv, focused);
    let n_rows = 1 + inv.resource_stocks.len() + items_rows + containers_rows + tanks_rows;

    let mut sections: Vec<Constraint> = (0..n_rows).map(|_| Constraint::Length(1)).collect();
    sections.push(Constraint::Min(0));

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(sections)
        .split(inner);

    let mut row = 0;
    // Index into the navigable rows (stocks, active items, passive groups),
    // must advance in the same order as AppState::inventory_rows().
    let mut nav_idx: usize = 0;
    let sel_prefix = |selected: bool| {
        if selected {
            Span::styled("▶ ", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("  ")
        }
    };
    let name_style = |selected: bool, dim: bool| {
        if selected {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else if dim {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
        }
    };

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
        let selected = focused && nav_idx == state.inventory_selection;
        nav_idx += 1;
        let (icon, color, label) = match stock.stock_type.as_str() {
            "metals" => ("◆", Color::White, "Metals"),
            "ice" => ("❄", Color::Cyan, "Ice"),
            "carbon_compounds" => ("◇", Color::Green, "Carbon"),
            _ => ("·", Color::DarkGray, stock.stock_type.as_str()),
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                sel_prefix(selected),
                Span::styled(format!("{icon} "), Style::default().fg(color)),
                Span::styled(format!("{:<11}", label), name_style(selected, false)),
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
            let selected = focused && nav_idx == state.inventory_selection;
            nav_idx += 1;
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
                    sel_prefix(selected),
                    Span::styled(format!("{icon} "), Style::default().fg(icon_color)),
                    Span::styled(format!("{:<14}", item.name), name_style(selected, false)),
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
            let selected = focused && nav_idx == state.inventory_selection;
            nav_idx += 1;
            let count = inv.items.iter().filter(|i| i.item_type == item.item_type).count();
            let (icon, icon_color) = item_icon(&item.item_type);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    sel_prefix(selected),
                    Span::styled(format!("{icon} "), Style::default().fg(icon_color)),
                    Span::styled(format!("{:<14}", item.name), name_style(selected, false)),
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

    // ── Containers ── (display only, expanded view)
    if containers_rows > 0 {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "── containers ──",
                Style::default().fg(Color::DarkGray),
            ))),
            rows[row],
        );
        row += 1;

        let mut containers: Vec<_> = inv.containers.iter().collect();
        containers.sort_by_key(|c| c.sort_order);
        for c in containers {
            let name = if c.kind == "probe" {
                c.label.clone()
            } else {
                format!("{} ({})", c.label, c.kind)
            };
            let ratio = if c.capacity > 0.0 {
                (c.used_capacity / c.capacity).clamp(0.0, 1.0)
            } else {
                0.0
            };
            frame.render_widget(
                make_line_gauge(
                    &format!("  {:<18}{:.2} / {:.2}", name, c.used_capacity, c.capacity),
                    ratio,
                    Color::Blue,
                ),
                rows[row],
            );
            row += 1;
        }
    }

    // ── External tanks ── (display only, expanded view)
    if tanks_rows > 0 {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "── tanks ──",
                Style::default().fg(Color::DarkGray),
            ))),
            rows[row],
        );
        row += 1;

        for tank in &inv.external_tanks {
            let ratio = (tank.fill_percent / 100.0).clamp(0.0, 1.0);
            frame.render_widget(
                make_line_gauge(
                    &format!("  {:<18}{:.1}%", tank.name, tank.fill_percent),
                    ratio,
                    gauge_color(ratio),
                ),
                rows[row],
            );
            row += 1;
        }
    }

    let _ = row;
    let _ = nav_idx;
}

pub(crate) fn inventory_panel_height(state: &AppState) -> u16 {
    let probe = match &state.probe {
        Some(p) => p,
        None => return 3,
    };
    let inv = &probe.inventory;
    let focused = state.focused == Some(Panel::Inventory);
    let n_stocks = inv.resource_stocks.len() as u16;
    let expanded = focused && !inv.items.is_empty();
    let items_rows = items_row_count(&inv.items, expanded) as u16;
    let containers_rows = containers_row_count(inv, focused) as u16;
    let tanks_rows = tanks_row_count(inv, focused) as u16;
    let hint_row = if focused { 1 } else { 0 };
    1 + n_stocks + items_rows + containers_rows + tanks_rows + hint_row + 2
}

pub(crate) fn containers_row_count(inv: &crate::api::types::ProbeInventory, focused: bool) -> usize {
    if focused && !inv.containers.is_empty() {
        1 + inv.containers.len()
    } else {
        0
    }
}

pub(crate) fn tanks_row_count(inv: &crate::api::types::ProbeInventory, focused: bool) -> usize {
    if focused && !inv.external_tanks.is_empty() {
        1 + inv.external_tanks.len()
    } else {
        0
    }
}

pub(crate) fn items_row_count(items: &[crate::api::types::ProbeInventoryItem], expanded: bool) -> usize {
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
