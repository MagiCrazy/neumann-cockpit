//! Unified Cockpit v2 interface — the 3×3 tiling dashboard (blocs U2–U3).
//!
//! Responsive grid + read-only navigation: `ertdfgcvb` selects a pane, `jk`
//! moves the cursor, `l`/`h` drill in/out, `z` zooms the active pane full
//! screen. Contextual menus and command mode follow in later blocs. The four
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
    layout::{Alignment, Constraint, Direction, Layout, Rect},
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
    // Status bar is one line, plus a second hints line when enabled.
    let status_h = if state.hints_visible { 2 } else { 1 };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(status_h)])
        .split(area);

    if state.zoomed {
        // Zoom: the active pane takes the whole content area.
        render_pane(frame, rows[0], state.active_pane, state, true);
    } else {
        for (pane, rect) in grid::visible_panes(rows[0], state.active_pane) {
            render_pane(frame, rect, pane, state, pane == state.active_pane);
        }
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
    // Split off the hints line (bottom) when enabled.
    let (bar, hints) = if state.hints_visible && area.height >= 2 {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);
        (rows[0], Some(rows[1]))
    } else {
        (area, None)
    };

    render_status_line(frame, bar, state);
    if let Some(hints_area) = hints {
        let line = Line::from(Span::styled(
            format!(" {}", state.pane_hints()),
            Style::default().fg(DIM),
        ));
        frame.render_widget(Paragraph::new(line), hints_area);
    }
}

fn render_status_line(frame: &mut Frame, area: Rect, state: &AppState) {
    let (tag, tag_bg) = if state.zoomed {
        ("ZOOM", GREEN)
    } else {
        (state.mode.tag(), AMBER)
    };

    // Left: mode tag · breadcrumb · transient toast.
    let mut left = vec![
        Span::styled(
            format!(" {tag} "),
            Style::default().fg(Color::Black).bg(tag_bg).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("  {}", state.breadcrumb().join(" › ")), Style::default().fg(GREEN)),
    ];
    if let Some(toast) = state.active_toast() {
        left.push(Span::styled(format!("  ✓ {toast}"), Style::default().fg(Color::Green)));
    }

    // Right: SCUT coverage · unread · API version · clock.
    let mut meta = Vec::new();
    if !state.scut_coverage().is_empty() {
        meta.push("≣ SCUT".to_string());
    }
    let unread = state.unread_alert_count();
    if unread > 0 {
        meta.push(format!("! {unread}"));
    }
    if let Some(v) = state.api_version {
        meta.push(format!("API v{v}"));
    }
    if let Some(t) = state.last_update {
        meta.push(t.format("%H:%M:%S").to_string());
    }
    let meta = meta.join(" · ");
    let meta_len = meta.chars().count() as u16;

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(meta_len + 1)])
        .split(area);
    frame.render_widget(Paragraph::new(Line::from(left)), cols[0]);
    frame.render_widget(
        Paragraph::new(meta).alignment(Alignment::Right).style(Style::default().fg(DIM)),
        cols[1],
    );
}
