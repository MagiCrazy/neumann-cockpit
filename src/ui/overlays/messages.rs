use crate::ui::theme::palette;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::api::types::MessageStatus;
use crate::app::{AppState, MessagesInput};
use super::{centered_rect, render_pick_list};

fn preview(body: &str) -> String {
    let one = body.replace('\n', " ");
    if one.chars().count() > 48 {
        format!("{}…", one.chars().take(48).collect::<String>())
    } else {
        one
    }
}

pub(crate) fn render_messages_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    match &state.messages_input {
        MessagesInput::Browsing { sent_tab, selection } => {
            let popup = centered_rect(76, 80, area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(" MESSAGES ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            // Tabs
            let inbox_style = if *sent_tab { Style::default().fg(Color::DarkGray) } else { Style::default().fg(Color::White).add_modifier(Modifier::BOLD) };
            let sent_style = if *sent_tab { Style::default().fg(Color::White).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("Inbox", inbox_style),
                    Span::styled("   |   ", Style::default().fg(Color::DarkGray)),
                    Span::styled("Sent", sent_style),
                ])),
                rows[0],
            );

            let mut lines: Vec<Line> = Vec::new();
            if *sent_tab {
                if state.sent_messages.is_empty() {
                    lines.push(Line::from(Span::styled("No sent messages.", Style::default().fg(Color::DarkGray))));
                }
                for (i, m) in state.sent_messages.iter().enumerate() {
                    let marker = if i == *selection { "▸ " } else { "  " };
                    lines.push(Line::from(vec![
                        Span::styled(marker, Style::default().fg(Color::Cyan)),
                        Span::styled(format!("→ {} ", m.recipient.name), Style::default().fg(Color::White)),
                        Span::styled(preview(&m.body), Style::default().fg(Color::Gray)),
                    ]));
                }
            } else {
                if state.messages.is_empty() {
                    lines.push(Line::from(Span::styled("No messages.", Style::default().fg(Color::DarkGray))));
                }
                for (i, m) in state.messages.iter().enumerate() {
                    let marker = if i == *selection { "▸ " } else { "  " };
                    let unread = m.status == MessageStatus::Unread;
                    let dot = if unread { Span::styled("● ", Style::default().fg(Color::Cyan)) } else { Span::raw("  ") };
                    let name_style = if unread { Style::default().fg(Color::White).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Gray) };
                    lines.push(Line::from(vec![
                        Span::styled(marker, Style::default().fg(Color::Cyan)),
                        dot,
                        Span::styled(format!("{} ", m.sender.name), name_style),
                        Span::styled(preview(&m.body), Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }
            frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), rows[1]);

            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("[Tab]", Style::default().fg(Color::Cyan)),
                    Span::raw(" inbox/sent  "),
                    Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
                    Span::raw(" read  "),
                    Span::styled("[c]", Style::default().fg(Color::Cyan)),
                    Span::raw(" compose  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" close"),
                ])),
                rows[2],
            );
        }

        MessagesInput::PickRecipient { recipients, selection } => {
            let names: Vec<String> = recipients.iter()
                .map(|(kind, _, name)| format!("{name}  ({kind})"))
                .collect();
            let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            let height = (recipients.len() as u16 + 6).min(16);
            render_pick_list(
                frame, area, palette(state.color_mode), " NEW MESSAGE — recipient ", 54, height,
                Some("Send to:"), &refs, *selection, None, "compose",
            );
        }

        MessagesInput::Compose { recipient_name, body_buf, error, .. } => {
            let popup = centered_rect(60, 9, area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(format!(" MESSAGE → {recipient_name} "))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let mut lines = vec![Line::from(vec![
                Span::styled(body_buf.clone(), Style::default().fg(Color::White)),
                Span::styled("▏", Style::default().fg(Color::Cyan)),
            ])];
            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(format!("✗ {err}"), Style::default().fg(Color::Red))));
            }
            frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), rows[0]);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw(" send  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])),
                rows[1],
            );
        }

        MessagesInput::Inactive => {}
    }
}
