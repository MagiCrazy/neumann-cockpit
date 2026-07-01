//! Unified Cockpit v2 interface — the 3×3 tiling dashboard (bloc U2).
//!
//! U2 delivers the responsive grid and read-only navigation: `ertdfgcvb`
//! selects a pane, `jk` moves the cursor within it. Drill-in, zoom,
//! contextual menus and command mode follow in later blocs. The four
//! original panes reuse their existing renderers; the five promoted panes
//! (Map, Comms, Sector, Missions, Storage) use the compact renderers in
//! [`panes`].

mod grid;
mod panes;

use crate::app::{AppState, Pane};
use crate::ui::panels::{
    render_inventory_panel, render_mannies_panel, render_probe_panel, render_scanner_panel,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

const AMBER: Color = Color::Rgb(0xff, 0xb2, 0x4a);
const GREEN: Color = Color::Rgb(0x5e, 0xf0, 0x8f);
const DIM: Color = Color::Rgb(0x6f, 0x8c, 0x7d);

pub fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    for (pane, rect) in grid::visible_panes(rows[0], state.active_pane) {
        render_pane(frame, rect, pane, state, pane == state.active_pane);
    }
    render_status(frame, rows[1], state);
}

fn render_pane(frame: &mut Frame, area: Rect, pane: Pane, state: &AppState, active: bool) {
    match pane {
        Pane::Probe => render_probe_panel(frame, area, state, active),
        Pane::Inventory => render_inventory_panel(frame, area, state, active),
        Pane::Scanner => render_scanner_panel(frame, area, state, active),
        Pane::Mannies => render_mannies_panel(frame, area, state, active),
        Pane::Map => panes::render_map(frame, area, state, active),
        Pane::Comms => panes::render_comms(frame, area, state, active),
        Pane::Sector => panes::render_sector(frame, area, state, active),
        Pane::Missions => panes::render_missions(frame, area, state, active),
        Pane::Storage => panes::render_storage(frame, area, state, active),
    }
}

fn render_status(frame: &mut Frame, area: Rect, state: &AppState) {
    let line = Line::from(vec![
        Span::styled(
            format!(" {} ", state.mode.tag()),
            Style::default().fg(Color::Black).bg(AMBER).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {} ", state.active_pane.label()),
            Style::default().fg(GREEN),
        ),
        Span::styled(
            "· ertdfgcvb select · jk move · Tab cycle · F5 refresh · q quit",
            Style::default().fg(DIM),
        ),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}
