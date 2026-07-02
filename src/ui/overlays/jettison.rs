use crate::ui::theme::palette;
use crate::app::{AppState, JettisonInput};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::centered_rect;
pub(crate) fn render_jettison_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    match &state.jettison {
        JettisonInput::ConfirmManny { manny_name, error, .. } => {
            let popup = centered_rect(48, 8, area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(format!(" JETTISON — {manny_name} "))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(p.crit));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let mut lines: Vec<Line> = Vec::new();
            lines.push(Line::from(Span::styled(
                "Eject manny into the sector?",
                Style::default().fg(p.crit),
            )));
            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    format!("✗ {err}"),
                    Style::default().fg(p.crit),
                )));
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("[Enter]", Style::default().fg(p.crit).add_modifier(Modifier::BOLD)),
                    Span::raw(" EJECT  "),
                    Span::styled("[Esc]", Style::default().fg(p.accent)),
                    Span::raw(" cancel"),
                ])),
                rows[1],
            );
        }

        JettisonInput::ConfirmRelay { error, .. } => {
            let popup = centered_rect(52, 8, area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(" DEPLOY SCUT RELAY ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(p.accent));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let mut lines = vec![Line::from(Span::styled(
                "Deploy an inactive SCUT relay into this sector?",
                Style::default().fg(p.text),
            ))];
            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    format!("✗ {err}"),
                    Style::default().fg(p.crit),
                )));
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("[Enter]", Style::default().fg(p.accent).add_modifier(Modifier::BOLD)),
                    Span::raw(" DEPLOY  "),
                    Span::styled("[Esc]", Style::default().fg(p.accent)),
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
                .border_style(Style::default().fg(p.warn));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let mut lines: Vec<Line> = Vec::new();
            lines.push(Line::from(vec![
                Span::styled("Amount: ", Style::default().fg(p.accent)),
                Span::raw(buf.as_str()),
                Span::styled("█", Style::default().fg(p.accent)),
                Span::styled(" ECE", Style::default().fg(p.dim)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("MAX  ", Style::default().fg(p.dim)),
                Span::styled(format!("{max_amount:.3}"), Style::default().fg(p.text)),
                Span::raw("   "),
                Span::styled("[M]", Style::default().fg(p.warn)),
                Span::styled(" fill", Style::default().fg(p.dim)),
                Span::styled("  [empty = all]", Style::default().fg(p.dim)),
            ]));
            if let Some(err) = error {
                lines.push(Line::from(Span::styled(
                    format!("✗ {err}"),
                    Style::default().fg(p.crit),
                )));
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("[Enter]", Style::default().fg(p.good).add_modifier(Modifier::BOLD)),
                    Span::raw(" confirm  "),
                    Span::styled("[Esc]", Style::default().fg(p.accent)),
                    Span::raw(" cancel"),
                ])),
                rows[1],
            );
        }

        JettisonInput::Inactive => {}
    }
}

