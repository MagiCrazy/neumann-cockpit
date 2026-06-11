use crate::app::{AppState, RepairInput};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui::theme::format_duration;
use super::centered_rect;
pub(crate) fn render_repair_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
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

