use crate::app::{AppState, JettisonInput};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::centered_rect;
pub(crate) fn render_jettison_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    match &state.jettison {
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

        JettisonInput::ConfirmRelay { error, .. } => {
            let popup = centered_rect(52, 8, area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(" DEPLOY SCUT RELAY ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::LightBlue));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let mut lines = vec![Line::from(Span::styled(
                "Deploy an inactive SCUT relay into this sector?",
                Style::default().fg(Color::White),
            ))];
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
                    Span::styled("[Enter]", Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD)),
                    Span::raw(" DEPLOY  "),
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
                Span::raw("   "),
                Span::styled("[M]", Style::default().fg(Color::Yellow)),
                Span::styled(" fill", Style::default().fg(Color::DarkGray)),
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

