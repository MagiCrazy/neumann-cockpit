use crate::ui::theme::palette;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{AppState, RemoteMineInput, RESOURCE_LABELS};
use super::{centered_rect, render_footer, render_pick_list, FooterKey};

pub(crate) fn render_remote_mine_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    match &state.remote_mine {
        RemoteMineInput::Loading { manny_name, x, y, z, .. } => {
            let popup = centered_rect(50, 5, area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(format!(" REMOTE MINE — {manny_name} "))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(p.accent));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    format!("scanning sector ({x},{y},{z}) via SCUT…"),
                    Style::default().fg(p.text),
                ))),
                rows[0],
            );
            render_footer(frame, rows[1], p, &[FooterKey::nav("[Esc]", "cancel")]);
        }

        RemoteMineInput::PickAsteroid { manny_name, candidates, selection, x, y, z, .. } => {
            let labels: Vec<String> = candidates.iter().enumerate()
                .map(|(i, (id, n))| super::sector_object_label(state, *x, *y, *z, i, id, n)).collect();
            let names: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
            let height = (candidates.len() as u16 + 6).min(16);
            render_pick_list(
                frame, area, palette(state.color_mode), &format!(" REMOTE MINE — {manny_name} "), 52, height,
                Some("Asteroid in the Manny's sector:"), &names, *selection, None, "next",
            );
        }

        RemoteMineInput::Configure { object_name, resources, amount_buf, amount_mode, error, .. } => {
            let popup = centered_rect(54, 13, area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(format!(" REMOTE MINE → {object_name} "))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(p.accent));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let mut lines: Vec<Line> = Vec::new();
            let res_color = if *amount_mode { p.dim } else { p.accent };
            lines.push(Line::from(Span::styled("Resources  [1-4 toggle]", Style::default().fg(res_color))));
            for (i, &label) in RESOURCE_LABELS.iter().enumerate() {
                let checked = if resources[i] { "[✓]" } else { "[ ]" };
                let color = if resources[i] { p.text } else { p.dim };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {checked} "), Style::default().fg(if resources[i] { p.good } else { p.dim })),
                    Span::styled(format!("{} {label}", i + 1), Style::default().fg(color)),
                ]));
            }
            lines.push(Line::default());
            let amt_color = if *amount_mode { p.accent } else { p.dim };
            lines.push(Line::from(vec![
                Span::styled("Amount: ", Style::default().fg(amt_color)),
                Span::styled(amount_buf.as_str(), Style::default().fg(if *amount_mode { p.text } else { p.dim })),
                Span::styled(if *amount_mode { "█ ECE" } else { " ECE  [Tab]" }, Style::default().fg(p.dim)),
            ]));
            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(format!("✗ {err}"), Style::default().fg(p.crit))));
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            render_footer(frame, rows[1], p, &[
                FooterKey::nav("[1-4]", "res"),
                FooterKey::nav("[Tab]", "amount"),
                FooterKey::nav("[Enter]", "container →"),
                FooterKey::nav("[Esc]", "cancel"),
            ]);
        }

        RemoteMineInput::PickContainer { containers, selection, .. } => {
            let names: Vec<&str> = containers.iter().map(|(_, n)| n.as_str()).collect();
            let height = (containers.len() as u16 + 6).min(16);
            render_pick_list(
                frame, area, palette(state.color_mode), " REMOTE MINE — store in ", 52, height,
                Some("Detached container (required):"), &names, *selection, None, "MINE",
            );
        }

        RemoteMineInput::Inactive => {}
    }
}
