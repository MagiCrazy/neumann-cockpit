use crate::app::{AppState, Panel};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::overlays::render_active_overlays;
use super::panels::{
    inventory_panel_height, probe_panel_height, render_inventory_panel, render_mannies_panel,
    render_probe_panel, render_scanner_panel,
};
use super::theme::format_duration;
// ── Entry point ───────────────────────────────────────────────────────────────

pub fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    let outer = Block::default()
        .title(" NEUMANN COCKPIT ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let main_area = rows[0];
    let status_area = rows[1];

    let top_h = top_row_height(state);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(top_h), Constraint::Min(0)])
        .split(main_area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    render_probe_panel(frame, top[0], state, state.focused == Some(Panel::Probe));
    render_inventory_panel(frame, top[1], state, state.focused == Some(Panel::Inventory));
    render_scanner_panel(frame, bottom[0], state, state.focused == Some(Panel::Scanner));
    render_mannies_panel(frame, bottom[1], state, state.focused == Some(Panel::Mannies));
    render_status_bar(frame, status_area, state);
    render_active_overlays(frame, area, state);
}

// ── Status bar ────────────────────────────────────────────────────────────────

pub(crate) fn render_status_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let last = state
        .last_update
        .map(|t| t.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "—".to_string());

    let next = state
        .seconds_until_refresh()
        .map(|s| format!("in {}", format_duration(s)))
        .unwrap_or_else(|| "∞".to_string());

    let error_part = if let Some(e) = &state.error {
        format!("  ERR: {e}")
    } else {
        String::new()
    };

    let toast_part = state
        .active_toast()
        .map(|t| format!("  ✓ {t}"))
        .unwrap_or_default();

    let left = Line::from(vec![
        Span::styled("[r]", Style::default().fg(Color::Cyan)),
        Span::raw(" refresh  "),
        Span::styled("[p]", Style::default().fg(Color::Cyan)),
        Span::raw(" probe  "),
        Span::styled("[i]", Style::default().fg(Color::Cyan)),
        Span::raw(" inventory  "),
        Span::styled("[m]", Style::default().fg(Color::Cyan)),
        Span::raw(" mannies  "),
        Span::styled("[s]", Style::default().fg(Color::Cyan)),
        Span::raw(" scanner  "),
        Span::styled("[t]", Style::default().fg(Color::Cyan)),
        Span::raw(" travel  "),
        Span::styled("[b]", Style::default().fg(Color::Cyan)),
        Span::raw(" map  "),
        Span::styled("[w]", Style::default().fg(Color::Cyan)),
        Span::raw(" waypoints  "),
        Span::styled(
            "[A]",
            Style::default().fg(if state.unread_alert_count() > 0 { Color::Red } else { Color::Cyan }),
        ),
        Span::raw(" alerts  "),
        Span::styled("[?]", Style::default().fg(Color::Cyan)),
        Span::raw(" help  "),
        Span::styled("[q]", Style::default().fg(Color::Cyan)),
        Span::raw(" quit"),
        Span::styled(toast_part, Style::default().fg(Color::Green)),
        Span::styled(error_part, Style::default().fg(Color::Red)),
    ]);

    let app_version = env!("CARGO_PKG_VERSION");
    let api_version = state.api_version
        .map(|v| format!("API v{v}  "))
        .unwrap_or_default();
    let right_text = format!("v{app_version}  {api_version}⟳ {last}   next: {next}");
    let right_len = right_text.chars().count() as u16;
    let right = Paragraph::new(right_text)
        .alignment(Alignment::Right)
        .style(Style::default().fg(Color::DarkGray));

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(right_len)])
        .split(area);

    frame.render_widget(Paragraph::new(left), cols[0]);
    frame.render_widget(right, cols[1]);
}

pub(crate) fn top_row_height(state: &AppState) -> u16 {
    probe_panel_height(state).max(inventory_panel_height(state))
}

