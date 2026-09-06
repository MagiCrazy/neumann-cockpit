//! The logbook page editor (API v90, issue #254).
//!
//! Two text buffers and a delete confirmation. Bigger than the other text
//! overlays on purpose: a page is prose, and an editor that shows three lines
//! of it invites writing three lines of it.

use crate::api::types::LOGBOOK_TITLE_MAX;
use crate::app::{ActiveWizard, AppState, LogbookInput};
use crate::ui::theme::Palette;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::{centered_rect, render_footer, FooterKey};

pub(crate) fn render_logbook_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = state.palette();
    let ActiveWizard::Logbook(input) = &state.active_wizard else {
        return;
    };
    match input {
        LogbookInput::Inactive => {}
        LogbookInput::ConfirmDelete { title, .. } => render_confirm(frame, area, p, title),
        LogbookInput::Title {
            page_id, title, error, ..
        } => render_editor(frame, area, p, page_id.is_some(), title, None, error.as_deref()),
        LogbookInput::Content {
            page_id,
            title,
            content,
            error,
        } => render_editor(
            frame,
            area,
            p,
            page_id.is_some(),
            title,
            Some(content),
            error.as_deref(),
        ),
    }
}

/// `content` is `Some` while the body has the focus; the title stays on screen
/// either way, so the pilot always sees what they are writing under.
fn render_editor(
    frame: &mut Frame,
    area: Rect,
    p: Palette,
    editing: bool,
    title: &str,
    content: Option<&str>,
    error: Option<&str>,
) {
    let popup = centered_rect(72, area.height.saturating_sub(4).clamp(12, 24), area);
    frame.render_widget(Clear, popup);
    let heading = if editing {
        " LOGBOOK — EDIT PAGE "
    } else {
        " LOGBOOK — NEW PAGE "
    };
    let block = Block::default()
        .title(heading)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let dim = Style::default().fg(p.dim);
    let on_title = content.is_none();
    let mut title_line = vec![Span::styled("Title  ", Style::default().fg(p.accent))];
    title_line.push(Span::styled(title.to_string(), Style::default().fg(p.text)));
    if on_title {
        title_line.push(Span::styled("█", Style::default().fg(p.accent)));
    }
    title_line.push(Span::styled(
        format!("  {}/{LOGBOOK_TITLE_MAX}", title.chars().count()),
        dim,
    ));
    frame.render_widget(Paragraph::new(Line::from(title_line)), rows[0]);

    let body = match content {
        None => Paragraph::new(Line::styled("Enter → write the page", dim)),
        Some(text) => {
            let mut lines: Vec<Line> = text
                .lines()
                .map(|l| Line::styled(l.to_string(), Style::default().fg(p.text)))
                .collect();
            // The caret sits on its own line when the body ends with a newline.
            match lines.last_mut() {
                Some(last) if !text.ends_with('\n') => {
                    last.spans.push(Span::styled("█", Style::default().fg(p.accent)))
                }
                _ => lines.push(Line::from(Span::styled("█", Style::default().fg(p.accent)))),
            }
            Paragraph::new(lines).wrap(Wrap { trim: false })
        }
    };
    frame.render_widget(body, rows[1]);

    if let Some(err) = error {
        frame.render_widget(
            Paragraph::new(Line::styled(format!("✗ {err}"), Style::default().fg(p.crit))),
            rows[1],
        );
    }

    let keys = if on_title {
        vec![FooterKey::nav("[Enter]", "body"), FooterKey::nav("[Esc]", "cancel")]
    } else {
        vec![
            FooterKey::commit("[Ctrl-S]", "save"),
            FooterKey::nav("[Enter]", "new line"),
            FooterKey::nav("[Esc]", "back to title"),
        ]
    };
    render_footer(frame, rows[2], p, &keys);
}

fn render_confirm(frame: &mut Frame, area: Rect, p: Palette, title: &str) {
    let popup = centered_rect(56, 7, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" DELETE PAGE ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.crit));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                title.to_string(),
                Style::default().fg(p.text).add_modifier(Modifier::BOLD),
            ),
            Line::default(),
            Line::styled(
                "deleted on the server — the ship's log is untouched",
                Style::default().fg(p.dim),
            ),
        ])
        .wrap(Wrap { trim: false }),
        rows[0],
    );
    render_footer(
        frame,
        rows[1],
        p,
        &[
            FooterKey::danger("[Enter/y]", "delete"),
            FooterKey::nav("[Esc/n]", "keep"),
        ],
    );
}
