use crate::app::{is_active_item, AppState};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::theme::{block_gauge_line, item_icon, pane_block, ratio_color};
// ── Inventory panel ───────────────────────────────────────────────────────────

/// The HOLD pane (issue #345): the probe's own cargo **and** its containers,
/// which used to be two panes answering the same question from two angles —
/// and one of which already drew the other's list, inert.
///
/// Compact is one flat list (stocks → containers → items → tanks); zoom splits
/// it in two so the selected container's routing rules are permanently visible
/// instead of needing a zoom *and* a scroll to reach.
pub(crate) fn render_inventory_panel(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let p = state.palette();
    // Drilled into a container: its live contents, fetched on drill-in.
    if let Some(crate::app::DrillLevel::Container(id)) = state.pane_nav[crate::app::Pane::Hold.index()].drill.last() {
        return crate::ui::cockpit_v2::render_container_contents(frame, area, state, id, focused, p);
    }
    let block = pane_block(" HOLD ", focused, p);
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

    // ── Containers ── selectable rows now, not decoration (#345).
    //
    // Zoomed, they move to a column of their own with the selected one's
    // routing rules under them: that is the question the old Storage pane
    // answered and the one a flat list buries. `container_lines` therefore
    // collects them separately, but the cursor indices stay in one order —
    // stocks, then containers, then items — matching `inventory_rows`.
    let containers = state.storage_containers_ordered();
    let mut container_lines: Vec<Line> = Vec::new();
    let mut container_sel: Option<usize> = None;
    let zoomed = state.zoomed;
    if !containers.is_empty() {
        let sink = if zoomed { &mut container_lines } else { &mut lines };
        sink.push(Line::from(Span::styled(
            if state.storage_sort_alpha {
                "── containers · a-z ──"
            } else {
                "── containers ──"
            },
            Style::default().fg(p.dim),
        )));
        for c in &containers {
            let selected = focused && nav_idx == state.inventory_selection;
            nav_idx += 1;
            let sink = if zoomed { &mut container_lines } else { &mut lines };
            if selected {
                if zoomed {
                    container_sel = Some(sink.len());
                } else {
                    sel_line = Some(sink.len());
                }
            }
            let ratio = if c.capacity > 0.0 {
                (c.used_capacity / c.capacity).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let name: String = c.label.chars().take(11).collect();
            let rules = &c.rules;
            let routed =
                !rules.priority.is_empty() || !rules.exclusion.is_empty() || !rules.strict_exclusion.is_empty();
            let mut spans = vec![sel_prefix(selected)];
            spans.extend(block_gauge_line(&name, ratio, &format!("{:.0}%", ratio * 100.0), p.accent, p).spans);
            if routed {
                spans.push(Span::styled(" ⚙", Style::default().fg(p.accent)));
            }
            sink.push(Line::from(spans));
            // Zoomed, the selected container states its routing in full — the
            // whole point of giving it a column.
            if zoomed && selected {
                let dim = Style::default().fg(p.dim);
                for (label, rule) in [
                    ("priority", &rules.priority),
                    ("exclude ", &rules.exclusion),
                    ("strict  ", &rules.strict_exclusion),
                ] {
                    if !rule.is_empty() {
                        sink.push(Line::styled(format!("    {label}: {}", rule.join(", ")), dim));
                    }
                }
                if !routed {
                    sink.push(Line::styled("    no routing rules", dim));
                }
                sink.push(Line::styled(
                    format!("    free {:.2} of {:.2}", c.free_capacity, c.capacity),
                    dim,
                ));
            }
        }
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
    if container_lines.is_empty() {
        frame.render_widget(Paragraph::new(lines).scroll((offset, 0)), inner);
        crate::ui::theme::scroll_markers(frame, area, offset, total, focused, p);
        return;
    }
    // Two columns, zoomed: the hold on the left, the containers and the
    // selected one's routing on the right.
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::widgets::{Block, Borders};
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);
    frame.render_widget(Paragraph::new(lines).scroll((offset, 0)), cols[0]);
    let divider = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(p.dim));
    let right = divider.inner(cols[1]);
    frame.render_widget(divider, cols[1]);
    let c_off = container_sel
        .map(|c| crate::ui::cockpit_v2::scroll_offset((c, c), container_lines.len(), right.height as usize))
        .unwrap_or(0);
    frame.render_widget(Paragraph::new(container_lines).scroll((c_off, 0)), right);
}

pub(crate) fn tanks_row_count(inv: &crate::api::types::ProbeInventory, focused: bool) -> usize {
    if focused && !inv.external_tanks.is_empty() {
        1 + inv.external_tanks.len()
    } else {
        0
    }
}
