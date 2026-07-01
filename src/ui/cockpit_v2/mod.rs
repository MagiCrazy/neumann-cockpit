//! Unified Cockpit v2 interface — the 3×3 tiling dashboard (blocs U2–U7).
//!
//! Responsive grid + navigation: `ertdfgcvb` selects a pane, `jk` moves the
//! cursor, `l`/`h` drill in/out, `z` zooms, `Enter` opens the contextual menu.
//! Colours come from the active [`palette`] (config `theme`, F2 cycles). The
//! four original panes reuse their existing renderers (still on classic
//! colours until they get cockpit-native renderers); the five promoted panes
//! use the compact renderers in [`panes`].

mod grid;
mod menu;
mod panes;

use crate::app::{AppState, Pane};
use crate::ui::panels::{
    render_inventory_panel, render_mannies_panel, render_probe_panel, render_scanner_panel,
};
use crate::ui::theme::{palette, Palette};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let p = palette(state.color_mode);
    let status_h = if state.hints_visible { 2 } else { 1 };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(status_h)])
        .split(area);

    let visible: Vec<Pane> = if state.zoomed {
        render_pane(frame, rows[0], state.active_pane, state, true, p);
        vec![state.active_pane]
    } else {
        let panes = grid::visible_panes(rows[0], state.active_pane);
        for (pane, rect) in &panes {
            render_pane(frame, *rect, *pane, state, *pane == state.active_pane, p);
        }
        panes.iter().map(|(pane, _)| *pane).collect()
    };
    render_status(frame, rows[1], state, p, &visible);

    // Contextual menu popup, then any active wizard overlay on top.
    if let crate::app::InputMode::Menu(m) = &state.mode {
        menu::render(frame, area, m, p);
    }
    crate::ui::overlays::render_active_overlays(frame, area, state);
}

fn render_pane(frame: &mut Frame, area: Rect, pane: Pane, state: &AppState, active: bool, p: Palette) {
    match pane {
        // Reused classic renderers keep their own colours for now.
        Pane::Probe => render_probe_panel(frame, area, state, active),
        Pane::Inventory => render_inventory_panel(frame, area, state, active),
        Pane::Scanner => render_scanner_panel(frame, area, state, active),
        Pane::Mannies => render_mannies_panel(frame, area, state, active),
        // Promoted panes are palette-aware.
        Pane::Map => panes::render_map(frame, area, state, active, p),
        Pane::Comms => panes::render_comms(frame, area, state, active, p),
        Pane::Sector => panes::render_sector(frame, area, state, active, p),
        Pane::Missions => panes::render_missions(frame, area, state, active, p),
        Pane::Storage => panes::render_storage(frame, area, state, active, p),
    }
}

fn render_status(frame: &mut Frame, area: Rect, state: &AppState, p: Palette, visible: &[Pane]) {
    let (bar, hints) = if state.hints_visible && area.height >= 2 {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);
        (rows[0], Some(rows[1]))
    } else {
        (area, None)
    };

    render_status_line(frame, bar, state, p, visible);
    if let Some(hints_area) = hints {
        let line = Line::from(Span::styled(
            format!(" {}", state.pane_hints()),
            Style::default().fg(p.dim),
        ));
        frame.render_widget(Paragraph::new(line), hints_area);
    }
}

fn render_status_line(frame: &mut Frame, area: Rect, state: &AppState, p: Palette, visible: &[Pane]) {
    let tag = if state.zoomed { "ZOOM" } else { state.mode.tag() };

    let mut left = vec![
        Span::styled(
            format!(" {tag} "),
            Style::default().fg(Color::Black).bg(p.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("  {}", state.breadcrumb().join(" › ")), Style::default().fg(p.accent)),
    ];
    // Position mini-map: shown only when the grid is reduced (not all 9 panes
    // visible), so you know where the active pane sits. Groups of three keys
    // evoke the 3×3 rows on a single line.
    if visible.len() < Pane::ALL.len() {
        left.push(Span::styled("   ", Style::default()));
        for (i, pane) in Pane::ALL.iter().enumerate() {
            if i > 0 && i % 3 == 0 {
                left.push(Span::styled(" ", Style::default().fg(p.dim)));
            }
            let key = pane.key_label().to_string();
            let style = if *pane == state.active_pane {
                Style::default().fg(Color::Black).bg(p.accent).add_modifier(Modifier::BOLD)
            } else if visible.contains(pane) {
                Style::default().fg(p.text)
            } else {
                Style::default().fg(p.dim)
            };
            left.push(Span::styled(key, style));
        }
    }
    // An error takes over the line until dismissed; otherwise a success toast.
    if let Some(err) = &state.error {
        left.push(Span::styled(format!("  ✗ {err}"), Style::default().fg(p.crit)));
    } else if let Some(toast) = state.active_toast() {
        left.push(Span::styled(format!("  ✓ {toast}"), Style::default().fg(p.good)));
    }

    let mut meta = Vec::new();
    if state.loading {
        meta.push(("⟳".to_string(), p.accent));
    }
    if !state.scut_coverage().is_empty() {
        meta.push(("≣ SCUT".to_string(), p.accent));
    }
    let unread = state.unread_alert_count();
    if unread > 0 {
        meta.push((format!("! {unread}"), p.crit));
    }
    if let Some(v) = state.api_version {
        meta.push((format!("API v{v}"), p.dim));
    }
    if let Some(t) = state.last_update {
        meta.push((t.format("%H:%M:%S").to_string(), p.dim));
    }
    let meta_len: usize = meta.iter().map(|(s, _)| s.chars().count() + 3).sum();
    let meta_spans: Vec<Span> = meta
        .iter()
        .enumerate()
        .flat_map(|(i, (s, c))| {
            let sep = if i == 0 { "" } else { " · " };
            [
                Span::styled(sep, Style::default().fg(p.dim)),
                Span::styled(s.clone(), Style::default().fg(*c)),
            ]
        })
        .collect();

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(meta_len as u16 + 1)])
        .split(area);
    frame.render_widget(Paragraph::new(Line::from(left)), cols[0]);
    frame.render_widget(
        Paragraph::new(Line::from(meta_spans)).alignment(Alignment::Right),
        cols[1],
    );
}
