//! Compact read-only renderers for the five panes promoted from overlays
//! (bloc U2): Map, Comms, Sector, Missions, Storage. The four original panes
//! (Probe, Inventory, Scanner, Mannies) reuse their existing renderers.
//!
//! These show a terse summary sized for a 1/3 grid cell; the rich detail and
//! actions land in the zoom view (U3) and menus (U5).

use crate::api::types::MissionStatus;
use crate::app::{AppState, Pane};
use crate::ui::theme::{gauge_color, object_icon, panel_block};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

const DIM: Style = Style::new().fg(Color::DarkGray);

/// Style for the selected row: highlighted only while the pane is active.
fn row_style(active: bool, selected: bool) -> Style {
    if active && selected {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    }
}

fn cursor(state: &AppState, pane: Pane) -> usize {
    state.pane_nav[pane.index()].cursor
}

fn render_body(frame: &mut Frame, area: Rect, title: &str, active: bool, lines: Vec<Line>) {
    let block = panel_block(title, active);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(lines), inner);
}

pub fn render_map(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
    let mut lines = Vec::new();
    match state.probe_sector_coords() {
        Some((x, y, z)) => lines.push(Line::from(format!("sector ({x}, {y}, {z})"))),
        None => lines.push(Line::styled("sector unknown", DIM)),
    }
    lines.push(Line::from(format!("visited: {}", state.visited_sectors.len())));
    let nets = state.scut_coverage();
    if nets.is_empty() {
        lines.push(Line::styled("SCUT: no coverage", DIM));
    } else {
        lines.push(Line::styled(
            format!("≣ SCUT: {} network(s)", nets.len()),
            Style::default().fg(Color::LightBlue),
        ));
    }
    lines.push(Line::styled("[z] zoom for full map", DIM));
    render_body(frame, area, " MAP ", active, lines);
}

pub fn render_comms(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
    let unread_alerts = state.unread_alert_count();
    let unread_msgs = state.unread_message_count();
    let mut lines = vec![
        Line::from(vec![
            Span::raw(format!("alerts {} ", state.alerts.len())),
            Span::styled(format!("({unread_alerts} unread)"), DIM),
            Span::raw(format!("  warn {}", state.damage_warnings.len())),
        ]),
        Line::from(vec![
            Span::raw(format!("messages {} ", state.messages.len())),
            Span::styled(format!("({unread_msgs} unread)"), DIM),
        ]),
        Line::raw(""),
    ];
    let cur = cursor(state, Pane::Comms);
    if state.messages.is_empty() {
        lines.push(Line::styled("no messages", DIM));
    } else {
        for (i, m) in state.messages.iter().enumerate() {
            let unread = m.status == crate::api::types::MessageStatus::Unread;
            let mark = if unread { "✉" } else { "·" };
            let body: String = m.body.chars().take(18).collect();
            let span = Span::styled(
                format!("{mark} {}: {}", m.sender.name, body),
                row_style(active, i == cur),
            );
            lines.push(Line::from(span));
        }
    }
    render_body(frame, area, " COMMS ", active, lines);
}

pub fn render_sector(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
    let mut lines = Vec::new();
    match state.current_sector() {
        Some(s) => {
            let v = &s.relative_coordinates;
            lines.push(Line::from(format!(
                "({}, {}, {})  d{}",
                v.x as i32, v.y as i32, v.z as i32, s.distance
            )));
            let objs = state.scanner_objects();
            lines.push(Line::styled(format!("{} object(s)", objs.len()), DIM));
            let cur = cursor(state, Pane::Sector);
            for (i, e) in objs.iter().enumerate() {
                let (icon, color) = object_icon(&e.object_type);
                let name: String = e.name.chars().take(20).collect();
                lines.push(Line::from(vec![
                    Span::styled(format!("{icon} "), Style::default().fg(color)),
                    Span::styled(name, row_style(active, i == cur)),
                ]));
            }
        }
        None => lines.push(Line::styled("no sector scan yet", DIM)),
    }
    render_body(frame, area, " SECTOR ", active, lines);
}

pub fn render_missions(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
    let mut lines = Vec::new();
    if state.missions.is_empty() {
        lines.push(Line::styled("no active missions", DIM));
    } else {
        let cur = cursor(state, Pane::Missions);
        for (i, m) in state.missions.iter().enumerate() {
            let color = match m.status {
                MissionStatus::Active => Color::Cyan,
                MissionStatus::Completed => Color::Green,
                MissionStatus::Failed | MissionStatus::Abandoned => Color::Red,
                MissionStatus::Unknown => Color::DarkGray,
            };
            let done = m.steps.iter().filter(|s| {
                matches!(s.status, crate::api::types::MissionStepStatus::Completed)
            }).count();
            let title: String = m.title.chars().take(22).collect();
            lines.push(Line::from(vec![
                Span::styled("▸ ", Style::default().fg(color)),
                Span::styled(title, row_style(active, i == cur)),
                Span::styled(format!(" {done}/{}", m.steps.len()), DIM),
            ]));
        }
    }
    render_body(frame, area, " MISSIONS ", active, lines);
}

pub fn render_storage(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
    let mut lines = Vec::new();
    if state.storage_containers.is_empty() {
        lines.push(Line::styled("no containers ([C] loads)", DIM));
    } else {
        let cur = cursor(state, Pane::Storage);
        for (i, c) in state.storage_containers.iter().enumerate() {
            let ratio = if c.capacity > 0.0 {
                c.used_capacity / c.capacity
            } else {
                0.0
            };
            let label: String = c.label.chars().take(16).collect();
            lines.push(Line::from(vec![
                Span::styled(label, row_style(active, i == cur)),
                Span::styled(
                    format!(" {:.0}/{:.0}", c.used_capacity, c.capacity),
                    Style::default().fg(gauge_color(1.0 - ratio)),
                ),
            ]));
        }
    }
    render_body(frame, area, " STORAGE ", active, lines);
}
