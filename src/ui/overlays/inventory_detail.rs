use crate::ui::theme::{palette, Palette};
use crate::app::{AppState, InventoryRow};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::centered_rect;
// ── Inventory detail overlay ──────────────────────────────────────────────────

pub(crate) fn detail_kv(p: Palette, key: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{key:<12}"), Style::default().fg(p.dim)),
        Span::raw(value),
    ])
}

pub(crate) fn render_inventory_detail_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let Some(probe) = &state.probe else { return };
    let Some(row) = state.selected_inventory_row() else { return };
    let inv = &probe.inventory;

    let (title, lines): (String, Vec<Line>) = match row {
        InventoryRow::Stock { id } => {
            let Some(stock) = inv.resource_stocks.iter().find(|s| s.id == id) else { return };
            let mut lines = vec![
                detail_kv(p, "Type", stock.stock_type.clone()),
                detail_kv(p, "Amount", format!("{:.4} ECE", stock.amount)),
                detail_kv(p, "Space", format!("{:.4}", stock.container_space)),
            ];
            if !stock.containers.is_empty() {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    "── containers ──",
                    Style::default().fg(p.dim),
                )));
                for line in &stock.containers {
                    lines.push(detail_kv(p, 
                        &line.container.label,
                        format!("{:.4} ECE  (space {:.4})", line.amount, line.container_space),
                    ));
                }
            }
            (stock.name.clone(), lines)
        }
        InventoryRow::ActiveItem { id } => {
            let Some(item) = inv.items.iter().find(|i| i.id == id) else { return };
            let mut lines = vec![
                detail_kv(p, "Type", item.item_type.clone()),
                detail_kv(p, "Space", format!("{:.4}", item.container_space)),
                detail_kv(p, 
                    "Task",
                    match item.current_task.as_deref() {
                        None => "idle".into(),
                        Some(t) => format!("{t}  {:.0}%", item.task_progress_percent),
                    },
                ),
            ];
            if let Some(loc) = &item.location {
                lines.push(detail_kv(p, "Location", format!("{:?}", loc.location_type).to_lowercase()));
            }
            if let Some(container) = &item.container {
                lines.push(detail_kv(p, "Container", container.label.clone()));
            }
            if let Some(cargo) = &item.cargo {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    "── cargo ──",
                    Style::default().fg(p.dim),
                )));
                lines.push(detail_kv(p, "Capacity", format!("{:.3}", cargo.capacity)));
                lines.push(detail_kv(p, "Deuterium", format!("{:.3}", cargo.deuterium)));
                lines.push(detail_kv(p, "Metals", format!("{:.3}", cargo.metals)));
                lines.push(detail_kv(p, "Ice", format!("{:.3}", cargo.ice)));
                lines.push(detail_kv(p, "Organic", format!("{:.3}", cargo.organic_compounds)));
            }
            (item.name.clone(), lines)
        }
        InventoryRow::PassiveGroup { item_type } => {
            let items: Vec<_> = inv.items.iter().filter(|i| i.item_type == item_type).collect();
            let Some(first) = items.first() else { return };
            let mut lines = vec![
                detail_kv(p, "Type", item_type.clone()),
                detail_kv(p, "Count", format!("{}", items.len())),
                detail_kv(p, 
                    "Space",
                    format!("{:.4} total", items.iter().map(|i| i.container_space).sum::<f64>()),
                ),
                Line::default(),
            ];
            for item in &items {
                let container = item.container.as_ref()
                    .map(|c| format!("  ({})", c.label))
                    .unwrap_or_default();
                lines.push(Line::from(vec![
                    Span::raw(format!("  {}", item.name)),
                    Span::styled(container, Style::default().fg(p.dim)),
                ]));
            }
            (first.name.clone(), lines)
        }
    };

    let height = (lines.len() as u16 + 4).min(20);
    let popup = centered_rect(50, height, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(format!(" {title} "))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    frame.render_widget(Paragraph::new(lines), rows[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[Esc]", Style::default().fg(p.accent)),
            Span::raw(" close"),
        ])),
        rows[1],
    );
}

