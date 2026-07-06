use crate::ui::theme::palette;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::api::types::MessageStatus;
use crate::app::{AppState, MessagesInput};
use super::{centered_rect, render_footer, render_pick_list, FooterKey};

fn preview(body: &str) -> String {
    let one = body.replace('\n', " ");
    if one.chars().count() > 48 {
        format!("{}…", one.chars().take(48).collect::<String>())
    } else {
        one
    }
}

pub(crate) fn render_messages_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    match &state.messages_input {
        MessagesInput::Browsing { sent_tab, selection } => {
            let popup = centered_rect(76, 80, area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(" MESSAGES ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(p.text));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            // Tabs
            let inbox_style = if *sent_tab { Style::default().fg(p.dim) } else { Style::default().fg(p.text).add_modifier(Modifier::BOLD) };
            let sent_style = if *sent_tab { Style::default().fg(p.text).add_modifier(Modifier::BOLD) } else { Style::default().fg(p.dim) };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("Inbox", inbox_style),
                    Span::styled("   |   ", Style::default().fg(p.dim)),
                    Span::styled("Sent", sent_style),
                ])),
                rows[0],
            );

            let mut lines: Vec<Line> = Vec::new();
            if *sent_tab {
                if state.sent_messages.is_empty() {
                    lines.push(Line::from(Span::styled("No sent messages.", Style::default().fg(p.dim))));
                }
                for (i, m) in state.sent_messages.iter().enumerate() {
                    let marker = if i == *selection { "▸ " } else { "  " };
                    lines.push(Line::from(vec![
                        Span::styled(marker, Style::default().fg(p.accent)),
                        Span::styled(format!("→ {} ", m.recipient.name), Style::default().fg(p.text)),
                        Span::styled(preview(&m.body), Style::default().fg(p.text)),
                    ]));
                }
            } else {
                if state.messages.is_empty() {
                    lines.push(Line::from(Span::styled("No messages.", Style::default().fg(p.dim))));
                }
                for (i, m) in state.messages.iter().enumerate() {
                    let marker = if i == *selection { "▸ " } else { "  " };
                    let unread = m.status == MessageStatus::Unread;
                    let dot = if unread { Span::styled("● ", Style::default().fg(p.accent)) } else { Span::raw("  ") };
                    let name_style = if unread { Style::default().fg(p.text).add_modifier(Modifier::BOLD) } else { Style::default().fg(p.text) };
                    lines.push(Line::from(vec![
                        Span::styled(marker, Style::default().fg(p.accent)),
                        dot,
                        Span::styled(format!("{} ", m.sender.name), name_style),
                        Span::styled(preview(&m.body), Style::default().fg(p.dim)),
                    ]));
                }
            }
            frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), rows[1]);

            render_footer(frame, rows[2], p, &[
                FooterKey::nav("[Tab]", "inbox/sent"),
                FooterKey::nav("[Enter]", "read"),
                FooterKey::nav("[c]", "compose"),
                FooterKey::nav("[Esc]", "close"),
            ]);
        }

        MessagesInput::Reading { id, sent_tab } => {
            let msg = if *sent_tab {
                state.sent_messages.iter().find(|m| m.id == *id).map(|m| (&m.sender, &m.recipient, &m.body, &m.sector, &m.created_at))
            } else {
                state.messages.iter().find(|m| m.id == *id).map(|m| (&m.sender, &m.recipient, &m.body, &m.sector, &m.created_at))
            };
            let popup = centered_rect(64, 16, area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(" MESSAGE ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(p.text));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);
            let dim = Style::default().fg(p.dim);
            let text = Style::default().fg(p.text);
            let mut lines = Vec::new();
            if let Some((sender, recipient, body, sector, created)) = msg {
                lines.push(Line::from(vec![Span::styled("from ", dim), Span::styled(sender.name.clone(), text)]));
                lines.push(Line::from(vec![Span::styled("to   ", dim), Span::styled(recipient.name.clone(), text)]));
                if let Some(v) = sector.as_ref().and_then(|s| s.relative.as_ref()) {
                    lines.push(Line::from(vec![
                        Span::styled("at   ", dim),
                        Span::styled(format!("({}, {}, {})", v.x as i32, v.y as i32, v.z as i32), text),
                    ]));
                }
                lines.push(Line::styled(created.clone(), dim));
                lines.push(Line::default());
                lines.push(Line::styled(body.clone(), text));
            } else {
                lines.push(Line::styled("message not found", dim));
            }
            frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), rows[0]);
            render_footer(frame, rows[1], p, &[FooterKey::nav("[Esc]", "back")]);
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
                .border_style(Style::default().fg(p.accent));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let mut lines = vec![Line::from(vec![
                Span::styled(body_buf.clone(), Style::default().fg(p.text)),
                Span::styled("▏", Style::default().fg(p.accent)),
            ])];
            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(format!("✗ {err}"), Style::default().fg(p.crit))));
            }
            frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), rows[0]);
            render_footer(frame, rows[1], p, &[
                FooterKey::commit("[Enter]", "SEND"),
                FooterKey::nav("[Esc]", "cancel"),
            ]);
        }

        MessagesInput::Inactive => {}
    }
}
