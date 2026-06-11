use crate::api::types::MannyLocationType;
use crate::app::{AppState, InventoryRow, Panel};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::palette::{block_gauge, Pal};
use super::section_title;

const WORK_SPINNER: [&str; 4] = ["▖", "▘", "▝", "▗"];

pub(super) fn render_drones(frame: &mut Frame, area: Rect, state: &AppState, p: &Pal) {
    let f = state.anim.frame;
    let drones_focused = state.focused == Some(Panel::Mannies);
    let cargo_focused = state.focused == Some(Panel::Inventory);

    let mut lines: Vec<Line> = vec![section_title("DRONE BAY", drones_focused, p), Line::default()];

    match &state.mannies {
        None => lines.push(Line::from(Span::styled("  AWAITING TELEMETRY", p.dim()))),
        Some(mannies) if mannies.is_empty() => {
            lines.push(Line::from(Span::styled("  BAY EMPTY", p.dim())))
        }
        Some(mannies) => {
            for (i, m) in mannies.iter().enumerate() {
                let selected = drones_focused && i == state.mannies_selection;
                let cursor = if selected { "▸" } else { " " };
                let bay_led = match m.location.location_type {
                    MannyLocationType::Probe => "▣",
                    MannyLocationType::Sector => "▢",
                    MannyLocationType::Unknown => "?",
                };
                let task = m
                    .current_task
                    .as_ref()
                    .map(|t| format!("{t:?}").to_uppercase())
                    .unwrap_or_else(|| "IDLE".into());
                let name_style = if selected { p.bold() } else { p.bright() };
                lines.push(Line::from(vec![
                    Span::styled(format!(" {cursor}"), p.bright()),
                    Span::styled(format!("{bay_led} "), p.norm()),
                    Span::styled(format!("{:<12}", m.name.to_uppercase()), name_style),
                    Span::styled(task, if m.current_task.is_some() { p.norm() } else { p.dim() }),
                ]));
                if m.current_task.is_some() {
                    let spin = WORK_SPINNER[((f / 2) as usize + i) % WORK_SPINNER.len()];
                    let ratio = (m.task_progress_percent / 100.0).clamp(0.0, 1.0);
                    let mut spans = vec![
                        Span::styled("    ", p.dim()),
                        Span::styled(spin, p.bright()),
                        Span::styled(" ", p.dim()),
                    ];
                    for (cell, hot) in block_gauge(ratio, 8, f.wrapping_add(i as u64 * 5)) {
                        spans.push(Span::styled(cell, if hot { p.bright() } else { p.norm() }));
                    }
                    spans.push(Span::styled(
                        format!(" {:>3.0}%", m.task_progress_percent),
                        p.bright(),
                    ));
                    lines.push(Line::from(spans));
                }
            }
        }
    }

    // ── Cargo ──
    lines.push(Line::default());
    lines.push(section_title("CARGO", cargo_focused, p));
    lines.push(Line::default());

    if let Some(probe) = &state.probe {
        let inv = &probe.inventory;
        let ratio = if inv.capacity > 0.0 {
            (inv.used_capacity / inv.capacity).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let mut spans = vec![Span::styled("  HOLD ", p.dim())];
        for (cell, hot) in block_gauge(ratio, 10, f.wrapping_add(11)) {
            spans.push(Span::styled(cell, if hot { p.bright() } else { p.norm() }));
        }
        spans.push(Span::styled(
            format!(" {:.1}/{:.1}", inv.used_capacity, inv.capacity),
            p.bright(),
        ));
        lines.push(Line::from(spans));

        // Resource stocks follow inventory_rows() order so the existing
        // cursor ([Up]/[Down] + [j]/[Enter]) stays meaningful.
        let rows = state.inventory_rows();
        for (idx, row) in rows.iter().enumerate() {
            let selected = cargo_focused && idx == state.inventory_selection;
            let cursor = if selected { "▸" } else { " " };
            match row {
                InventoryRow::Stock { id } => {
                    if let Some(stock) = inv.resource_stocks.iter().find(|s| &s.id == id) {
                        lines.push(Line::from(vec![
                            Span::styled(format!(" {cursor} "), p.bright()),
                            Span::styled(
                                format!("{:<8}", stock.name.to_uppercase()),
                                if selected { p.bold() } else { p.norm() },
                            ),
                            Span::styled(format!("{:>8.3}", stock.amount), p.bright()),
                        ]));
                    }
                }
                InventoryRow::ActiveItem { id } => {
                    if let Some(item) = inv.items.iter().find(|i| &i.id == id) {
                        if item.item_type == "manny" {
                            continue; // already shown in the drone bay
                        }
                        lines.push(Line::from(vec![
                            Span::styled(format!(" {cursor} "), p.bright()),
                            Span::styled(
                                format!("{:<14}", item.name.to_uppercase()),
                                if selected { p.bold() } else { p.norm() },
                            ),
                        ]));
                    }
                }
                InventoryRow::PassiveGroup { item_type } => {
                    let count = inv.items.iter().filter(|i| &i.item_type == item_type).count();
                    if let Some(first) = inv.items.iter().find(|i| &i.item_type == item_type) {
                        lines.push(Line::from(vec![
                            Span::styled(format!(" {cursor} "), p.bright()),
                            Span::styled(
                                format!("{:<12}", first.name.to_uppercase()),
                                if selected { p.bold() } else { p.norm() },
                            ),
                            Span::styled(format!("×{count}"), p.bright()),
                        ]));
                    }
                }
            }
        }

        if !inv.external_tanks.is_empty() {
            for tank in &inv.external_tanks {
                let ratio = (tank.fill_percent / 100.0).clamp(0.0, 1.0);
                let mut spans = vec![Span::styled("   TANK ", p.dim())];
                for (cell, hot) in block_gauge(ratio, 6, f.wrapping_add(17)) {
                    spans.push(Span::styled(cell, if hot { p.bright() } else { p.norm() }));
                }
                spans.push(Span::styled(format!(" {:.0}%", tank.fill_percent), p.norm()));
                lines.push(Line::from(spans));
            }
        }
    } else {
        lines.push(Line::from(Span::styled("  AWAITING TELEMETRY", p.dim())));
    }

    frame.render_widget(Paragraph::new(lines), area);
}
