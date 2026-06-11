pub(crate) mod craft;
pub(crate) mod help;
pub(crate) mod inventory_detail;
pub(crate) mod jettison;
pub(crate) mod map;
pub(crate) mod mine;
pub(crate) mod object_actions;
pub(crate) mod pickers;
pub(crate) mod repair;
pub(crate) mod travel;
pub(crate) mod waypoints;

pub(crate) use craft::{render_atomic_printer_craft_overlay, render_craft_overlay};
pub(crate) use help::render_help_overlay;
pub(crate) use inventory_detail::render_inventory_detail_overlay;
pub(crate) use jettison::render_jettison_overlay;
pub(crate) use map::render_map_overlay;
pub(crate) use mine::render_mine_overlay;
pub(crate) use object_actions::render_object_action_overlay;
pub(crate) use pickers::{
    render_deploy_overlay, render_detach_overlay, render_inspect_overlay, render_recall_overlay,
    render_recover_overlay, render_rename_manny_overlay, render_salvage_overlay,
};
pub(crate) use repair::render_repair_overlay;
pub(crate) use travel::render_travel_overlay;
pub(crate) use waypoints::render_waypoints_overlay;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub(crate) fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(r.width), height.min(r.height))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_pick_list(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    width: u16,
    height: u16,
    prompt: Option<&str>,
    items: &[&str],
    selection: usize,
    error: Option<&str>,
    action: &str,
) {
    let popup = centered_rect(width, height, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(title.to_owned())
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
    if let Some(p) = prompt {
        lines.push(Line::from(Span::styled(p.to_owned(), Style::default().fg(Color::Cyan))));
        lines.push(Line::default());
    }
    for (i, name) in items.iter().enumerate() {
        if i == selection {
            lines.push(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                Span::styled(name.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(name.to_string(), Style::default().fg(Color::DarkGray)),
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
            Span::raw(format!(" {action}  ")),
            Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ])),
        rows[1],
    );
}

