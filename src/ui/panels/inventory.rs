use crate::app::{is_active_item, AppState};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::theme::{block_gauge_line, item_icon, palette, pane_block, ratio_color};
// ── Inventory panel ───────────────────────────────────────────────────────────

pub(crate) fn render_inventory_panel(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let p = palette(state.color_mode);
    let block = pane_block(" INVENTORY ", focused, p);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(probe) = &state.probe else {
        frame.render_widget(Paragraph::new("No data").style(Style::default().fg(p.dim)), inner);
        return;
    };

    let inv = &probe.inventory;

    let cargo_ratio = if inv.capacity > 0.0 {
        (inv.used_capacity / inv.capacity).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let items_expanded = focused && !inv.items.is_empty();
    let containers_rows = containers_row_count(inv, focused);
    let tanks_rows = tanks_row_count(inv, focused);

    // Every row is one Line, collected then rendered as a single scrolled
    // Paragraph. Laying them out as fixed 1-row rects (as this pane used to)
    // silently drops everything past the pane height — ratatui hands the
    // overflow zero-height rects — cursor included (issue #292).
    let mut lines: Vec<Line> = Vec::new();
    let mut sel_line: Option<usize> = None;
    // Index into the navigable rows (stocks, active items, passive groups),
    // must advance in the same order as AppState::inventory_rows().
    let mut nav_idx: usize = 0;
    let sel_prefix = |selected: bool| {
        if selected {
            Span::styled("▶ ", Style::default().fg(p.accent))
        } else {
            Span::raw("  ")
        }
    };
    let name_style = |selected: bool, dim: bool| {
        if selected {
            Style::default().fg(p.text).add_modifier(Modifier::BOLD)
        } else if dim {
            Style::default().fg(p.dim)
        } else {
            Style::default().fg(p.text)
        }
    };

    lines.push(block_gauge_line(
        "CARGO",
        cargo_ratio,
        &format!("{:.1}/{:.1}", inv.used_capacity, inv.capacity),
        p.accent,
        p,
    ));

    for stock in &inv.resource_stocks {
        let selected = focused && nav_idx == state.inventory_selection;
        nav_idx += 1;
        if selected {
            sel_line = Some(lines.len());
        }
        let (icon, label) = match stock.stock_type.as_str() {
            "metals" => ("◆", "Metals"),
            "ice" => ("❄", "Ice"),
            "carbon_compounds" => ("◇", "Carbon"),
            _ => ("·", stock.stock_type.as_str()),
        };
        lines.push(Line::from(vec![
            sel_prefix(selected),
            Span::styled(format!("{icon} "), Style::default().fg(p.accent)),
            Span::styled(format!("{label:<11}"), name_style(selected, false)),
            Span::styled(format!("{:.3}", stock.amount), Style::default().fg(p.text)),
            Span::styled(" ECE", Style::default().fg(p.dim)),
        ]));
    }

    // ── Items ──
    if items_expanded {
        lines.push(Line::from(Span::styled("── items ──", Style::default().fg(p.dim))));

        // Active items: manny and atomic_3d_printer — show individually with task state
        for item in inv.items.iter().filter(|i| is_active_item(&i.item_type)) {
            let selected = focused && nav_idx == state.inventory_selection;
            nav_idx += 1;
            if selected {
                sel_line = Some(lines.len());
            }
            let icon = item_icon(&item.item_type).0;
            let (task_span, progress) = match item.current_task.as_deref() {
                None => (Span::styled("idle", Style::default().fg(p.dim)), String::new()),
                Some(t) => (
                    Span::styled(t.to_string(), Style::default().fg(p.warn)),
                    // Omitted on Mannies since API v104 — the Mannies pane owns
                    // their live progress.
                    item.task_progress_percent
                        .map(|pct| format!(" {pct:3.0}%"))
                        .unwrap_or_default(),
                ),
            };
            lines.push(Line::from(vec![
                sel_prefix(selected),
                Span::styled(format!("{icon} "), Style::default().fg(p.accent)),
                Span::styled(format!("{:<14}", item.name), name_style(selected, false)),
                task_span,
                Span::styled(progress, Style::default().fg(p.dim)),
            ]));
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
            if selected {
                sel_line = Some(lines.len());
            }
            let count = inv.items.iter().filter(|i| i.item_type == item.item_type).count();
            let icon = item_icon(&item.item_type).0;
            lines.push(Line::from(vec![
                sel_prefix(selected),
                Span::styled(format!("{icon} "), Style::default().fg(p.accent)),
                Span::styled(format!("{:<14}", item.name), name_style(selected, false)),
                Span::styled(format!("× {count}"), Style::default().fg(p.text)),
            ]));
        }
    } else if !inv.items.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  items  ", Style::default().fg(p.dim)),
            Span::styled(format!("{}", inv.items.len()), Style::default().fg(p.text)),
            Span::styled("  (focus to expand)", Style::default().fg(p.dim)),
        ]));
    }

    // ── Containers ── (display only, expanded view)
    if containers_rows > 0 {
        lines.push(Line::from(Span::styled("── containers ──", Style::default().fg(p.dim))));

        let mut containers: Vec<_> = inv.containers.iter().collect();
        containers.sort_by_key(|c| c.sort_order);
        for c in containers {
            let name: String = c.label.chars().take(9).collect();
            let ratio = if c.capacity > 0.0 {
                (c.used_capacity / c.capacity).clamp(0.0, 1.0)
            } else {
                0.0
            };
            lines.push(block_gauge_line(
                &name,
                ratio,
                &format!("{:.1}/{:.1}", c.used_capacity, c.capacity),
                p.accent,
                p,
            ));
        }
    }

    // ── External tanks ── (display only, expanded view)
    if tanks_rows > 0 {
        lines.push(Line::from(Span::styled("── tanks ──", Style::default().fg(p.dim))));

        for tank in &inv.external_tanks {
            let ratio = (tank.fill_percent / 100.0).clamp(0.0, 1.0);
            let name: String = tank.name.chars().take(9).collect();
            lines.push(block_gauge_line(
                &name,
                ratio,
                &format!("{:.0}%", tank.fill_percent),
                ratio_color(ratio, p),
                p,
            ));
        }
    }

    let _ = nav_idx;
    let offset = sel_line
        .map(|c| crate::ui::cockpit_v2::scroll_offset((c, c), lines.len(), inner.height as usize))
        .unwrap_or(0);
    let total = lines.len();
    frame.render_widget(Paragraph::new(lines).scroll((offset, 0)), inner);
    crate::ui::theme::scroll_markers(frame, area, offset, total, focused, p);
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
