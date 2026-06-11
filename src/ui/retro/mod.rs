mod banner;
mod boot;
mod drones;
mod palette;
mod radar;
mod systems;
mod ticker;

#[cfg(test)]
mod tests;

use crate::app::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use super::overlays::render_active_overlays;
use palette::{pal, Pal};

/// Section header used by every retro panel: `─── TITLE ───────`.
pub(crate) fn section_title(title: &str, focused: bool, p: &Pal) -> Line<'static> {
    let style = if focused { p.bold() } else { p.norm() };
    Line::from(vec![
        Span::styled(" ─── ", p.dim()),
        Span::styled(title.to_string(), style),
        Span::styled(" ", p.dim()),
        Span::styled("─".repeat(18_usize.saturating_sub(title.len())), p.dim()),
    ])
}

pub fn render(frame: &mut Frame, state: &AppState) {
    let p = pal(state.phosphor);
    let area = frame.area();

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(p.norm());
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    if state.anim.booting {
        boot::render_boot(frame, inner, state, &p);
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // banner
            Constraint::Length(1), // separator
            Constraint::Min(8),    // main columns
            Constraint::Length(1), // comms
            Constraint::Length(1), // tlm
            Constraint::Length(1), // hints
        ])
        .split(inner);

    banner::render_banner(frame, rows[0], state, &p);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "═".repeat(inner.width as usize),
            p.dim(),
        ))),
        rows[1],
    );

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(26),
            Constraint::Min(30),
            Constraint::Length(30),
        ])
        .split(rows[2]);

    systems::render_systems(frame, cols[0], state, &p);
    radar::render_radar(frame, cols[1], state, &p);
    drones::render_drones(frame, cols[2], state, &p);

    ticker::render_comms(frame, rows[3], state, &p);
    ticker::render_tlm(frame, rows[4], state, &p);
    render_hints(frame, rows[5], state, &p);

    render_active_overlays(frame, area, state);
}

fn render_hints(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState, p: &Pal) {
    let mut spans = vec![
        Span::styled(" [P][I][M][S]", p.bright()),
        Span::styled(" FOCUS ", p.dim()),
        Span::styled("[T]", p.bright()),
        Span::styled(" TRAVEL ", p.dim()),
        Span::styled("[B]", p.bright()),
        Span::styled(" NAV PLOT ", p.dim()),
        Span::styled("[W]", p.bright()),
        Span::styled(" WAYPOINTS ", p.dim()),
        Span::styled("[?]", p.bright()),
        Span::styled(" HELP ", p.dim()),
        Span::styled("[F2]", p.bright()),
        Span::styled(" CLASSIC ", p.dim()),
        Span::styled("[Q]", p.bright()),
        Span::styled(" QUIT", p.dim()),
    ];
    if let Some(next) = state.seconds_until_refresh() {
        spans.push(Span::styled(
            format!("   NEXT CONTACT T-{}", crate::ui::theme::format_duration(next)),
            p.dim(),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
