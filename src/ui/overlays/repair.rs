use crate::app::{AppState, RepairInput};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui::theme::{format_duration, palette};
use super::{centered_rect, render_footer, FooterKey, KeyTone};
pub(crate) fn render_repair_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
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
        .border_style(Style::default().fg(p.accent));
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
        Span::styled("Restore: ", Style::default().fg(p.accent)),
        Span::raw(buf.as_str()),
        Span::styled("█", Style::default().fg(p.accent)),
        Span::styled("%", Style::default().fg(p.dim)),
    ]));

    lines.push(Line::default());

    // MAX hint
    lines.push(Line::from(vec![
        Span::styled("MAX  ", Style::default().fg(p.dim)),
        Span::styled(format!("{max_pct:.2}%"), Style::default().fg(p.text)),
        Span::raw("   "),
        Span::styled("[M]", Style::default().fg(p.warn)),
        Span::styled(" fill", Style::default().fg(p.dim)),
    ]));

    lines.push(Line::default());

    // Cost preview (only when input is parseable)
    if let (Some(metals), Some(secs)) = (metals_cost, duration_secs) {
        let metals_color = if insufficient { p.crit } else { p.text };
        lines.push(Line::from(vec![
            Span::styled("Metals  ", Style::default().fg(p.dim)),
            Span::styled(format!("{metals:.4}"), Style::default().fg(metals_color)),
            Span::styled(" ECE", Style::default().fg(p.dim)),
            if insufficient {
                Span::styled(
                    format!("  (have {metals_stock:.4})"),
                    Style::default().fg(p.crit),
                )
            } else {
                Span::raw("")
            },
        ]));
        lines.push(Line::from(vec![
            Span::styled("Time    ", Style::default().fg(p.dim)),
            Span::styled(format_duration(secs), Style::default().fg(p.warn)),
        ]));
        if let Some(eff) = effective {
            if parsed.is_some_and(|v| v > max_pct + 0.001) {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  → capped at {eff:.2}% (probe already at max above)"),
                        Style::default().fg(p.dim),
                    ),
                ]));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "type a value to see cost",
            Style::default().fg(p.dim),
        )));
    }

    // API error
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(p.crit),
        )));
    }

    frame.render_widget(Paragraph::new(lines), body);

    // Hint bar
    let can_send = parsed.is_some() && !insufficient;
    let repair_key = if can_send {
        FooterKey::commit("[Enter]", "REPAIR")
    } else {
        FooterKey { key: "[Enter]", label: "REPAIR", tone: KeyTone::Disabled }
    };
    render_footer(frame, hint_area, p, &[repair_key, FooterKey::nav("[Esc]", "cancel")]);
}

