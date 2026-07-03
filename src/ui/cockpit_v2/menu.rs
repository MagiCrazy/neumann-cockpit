//! Contextual action menu popup for the Cockpit v2 interface (blocs U5, U7).
//!
//! Rendered on top of the grid when `InputMode::Menu` is active. Enabled
//! items are selectable; disabled ones stay visible with their reason, so
//! the menu teaches what is (not yet) possible rather than hiding it.

use crate::app::ContextMenu;
use crate::ui::overlays::{centered_rect, render_footer, FooterKey};
use crate::ui::theme::Palette;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, menu: &ContextMenu, p: Palette) {
    let widest = menu
        .items
        .iter()
        .map(|i| {
            i.label.chars().count()
                + i.disabled_reason.as_ref().map_or(0, |r| r.chars().count() + 3)
        })
        .max()
        .unwrap_or(0)
        .max(menu.title.chars().count())
        // Keep the popup wide enough for the footer hint line.
        .max(FOOTER_WIDTH);
    let w = (widest as u16 + 4).clamp(18, 48);
    // items + footer + two border rows
    let h = menu.items.len() as u16 + 3;
    let rect = centered_rect(w, h, area);

    frame.render_widget(Clear, rect);
    let block = Block::default()
        .title(Span::styled(
            format!(" {} ", menu.title),
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(p.accent));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let lines: Vec<Line> = menu
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            if !item.enabled {
                let reason = item.disabled_reason.as_deref().unwrap_or("unavailable");
                return Line::from(vec![
                    Span::styled(format!(" {}", item.label), Style::default().fg(p.dim)),
                    Span::styled(format!(" ({reason})"), Style::default().fg(p.dim)),
                ]);
            }
            let style = if i == menu.cursor {
                Style::default().fg(p.accent).add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(p.text)
            };
            Line::from(Span::styled(format!(" {}", item.label), style))
        })
        .collect();

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    frame.render_widget(Paragraph::new(lines), rows[0]);
    render_footer(
        frame,
        rows[1],
        p,
        &[
            FooterKey::nav("[↑/↓]", "move"),
            FooterKey::nav("[Enter]", "select"),
            FooterKey::nav("[Esc]", "close"),
        ],
    );
}

/// Character width of the footer hint line (`[↑/↓] move  [Enter] select  [Esc]
/// close`), used to size the popup.
const FOOTER_WIDTH: usize = 39;
