//! Confirming a Comms discard (API v112, issue #366).
//!
//! Two prompts, one shape. The single discard shows the entry's own message so
//! the pilot recognises what they are about to lose, and says plainly when it
//! has never been read. The bulk discard names its bound instead: it only ever
//! sees acknowledged entries, and it says how many of them it is leaving.

use crate::app::{ActiveWizard, AppState, DiscardCommsInput};
use crate::ui::theme::Palette;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::{centered_rect, render_footer, FooterKey};

pub(crate) fn render_discard_comms_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = state.palette();
    let ActiveWizard::DiscardComms(input) = &state.active_wizard else {
        return;
    };
    let (heading, body) = match input {
        DiscardCommsInput::Inactive => return,
        DiscardCommsInput::One {
            warnings,
            message,
            unread,
            ..
        } => {
            let what = if *warnings { "WARNING" } else { "ALERT" };
            let mut body = vec![
                Line::styled(
                    message.clone(),
                    Style::default().fg(p.text).add_modifier(Modifier::BOLD),
                ),
                Line::default(),
            ];
            if *unread {
                body.push(Line::styled(
                    "⚠ still unread — nobody has seen this yet",
                    Style::default().fg(p.warn),
                ));
            }
            body.push(Line::styled(
                "deleted on the server, permanently",
                Style::default().fg(p.dim),
            ));
            (format!(" DISCARD {what} "), body)
        }
        DiscardCommsInput::Acknowledged { warnings, ids, total } => {
            let what = if *warnings { "WARNINGS" } else { "ALERTS" };
            let n = ids.len();
            let plural = if n > 1 { "entries" } else { "entry" };
            let mut body = vec![
                Line::styled(
                    format!("Discard {n} acknowledged {plural}, oldest first"),
                    Style::default().fg(p.text).add_modifier(Modifier::BOLD),
                ),
                Line::default(),
                Line::styled("nothing unread is touched", Style::default().fg(p.dim)),
            ];
            if *total > n {
                body.push(Line::styled(
                    format!("{} acknowledged remain — press X again", total - n),
                    Style::default().fg(p.dim),
                ));
            }
            (format!(" DISCARD ACKNOWLEDGED {what} "), body)
        }
    };
    render_prompt(frame, area, p, &heading, body);
}

fn render_prompt(frame: &mut Frame, area: Rect, p: Palette, heading: &str, body: Vec<Line>) {
    let popup = centered_rect(60, 9, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(heading.to_string())
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.crit));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    frame.render_widget(Paragraph::new(body).wrap(Wrap { trim: false }), rows[0]);
    render_footer(
        frame,
        rows[1],
        p,
        &[
            FooterKey::danger("[Enter/y]", "discard"),
            FooterKey::nav("[Esc]", "keep"),
        ],
    );
}
