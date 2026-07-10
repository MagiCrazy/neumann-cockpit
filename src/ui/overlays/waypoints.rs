use crate::app::{ActiveWizard, AppState, WaypointKind, WaypointsInput};
use crate::ui::theme::palette;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};

use super::{centered_rect, render_footer, FooterKey};
pub(crate) fn render_waypoints_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let ActiveWizard::Waypoints(WaypointsInput::Browsing { entries, selection }) = &state.active_wizard else {
        return;
    };
    let selection = *selection;

    let height = (entries.len() as u16 + 5).min(20);
    let popup = centered_rect(58, height, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" WAYPOINTS ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let items: Vec<ListItem> = entries
        .iter()
        .map(|e| {
            let (icon, color) = match e.kind {
                WaypointKind::Bookmark => ("◎", p.accent),
                WaypointKind::Star => ("★", p.warn),
                WaypointKind::Minable => ("◆", p.text),
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{icon} "), Style::default().fg(color)),
                Span::raw(format!("{:<28}", e.label)),
                Span::styled(format!("({},{},{})", e.x, e.y, e.z), Style::default().fg(p.text)),
                Span::styled(format!("  d:{}", e.distance), Style::default().fg(p.dim)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");
    let mut list_state = ListState::default();
    list_state.select(Some(selection));
    frame.render_stateful_widget(list, rows[0], &mut list_state);

    render_footer(
        frame,
        rows[1],
        p,
        &[
            FooterKey::nav("[↑/↓]", "select"),
            FooterKey::commit("[Enter]", "TRAVEL"),
            FooterKey::nav("[Esc]", "close"),
        ],
    );
}
