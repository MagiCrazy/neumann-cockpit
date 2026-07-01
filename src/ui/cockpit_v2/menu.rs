//! Contextual action menu popup for the Cockpit v2 interface (bloc U5).
//!
//! Rendered on top of the grid when `InputMode::Menu` is active. Enabled
//! items are selectable; disabled ones stay visible with their reason, so
//! the menu teaches what is (not yet) possible rather than hiding it.

use crate::app::ContextMenu;
use crate::ui::overlays::centered_rect;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

const AMBER: Color = Color::Rgb(0xff, 0xb2, 0x4a);
const DIM: Color = Color::Rgb(0x6f, 0x8c, 0x7d);

pub fn render(frame: &mut Frame, area: Rect, menu: &ContextMenu) {
    let widest = menu
        .items
        .iter()
        .map(|i| {
            i.label.chars().count()
                + i.disabled_reason.as_ref().map_or(0, |r| r.chars().count() + 3)
        })
        .max()
        .unwrap_or(0)
        .max(menu.title.chars().count());
    let w = (widest as u16 + 4).clamp(18, 48);
    let h = menu.items.len() as u16 + 2;
    let rect = centered_rect(w, h, area);

    frame.render_widget(Clear, rect);
    let block = Block::default()
        .title(Span::styled(
            format!(" {} ", menu.title),
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(AMBER));
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
                    Span::styled(format!(" {}", item.label), Style::default().fg(DIM)),
                    Span::styled(format!(" ({reason})"), Style::default().fg(DIM)),
                ]);
            }
            let style = if i == menu.cursor {
                Style::default().fg(AMBER).add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            Line::from(Span::styled(format!(" {}", item.label), style))
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}
