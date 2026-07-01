//! Compact renderers for the five panes promoted from overlays (blocs U2–U7):
//! Map, Comms, Sector, Missions, Storage. The four original panes (Probe,
//! Inventory, Scanner, Mannies) reuse their existing renderers.
//!
//! Each shows a terse summary sized for a 1/3 grid cell; drilling in (`l`)
//! swaps a pane to its detail view (Missions → steps, Comms → message).
//! Colours come from the active [`Palette`].

use crate::api::types::{MissionStatus, MissionStepStatus};
use crate::app::{AppState, DrillLevel, Pane};
use crate::ui::theme::{object_icon, pane_block, Palette};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

/// Style for the selected row: highlighted only while the pane is active.
fn row_style(active: bool, selected: bool) -> Style {
    if active && selected {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    }
}

/// Fill colour for a "how full" ratio, mapped through the palette so mono
/// modes stay single-hue.
fn fill_color(p: Palette, ratio: f64) -> ratatui::style::Color {
    if ratio > 0.5 {
        p.good
    } else if ratio > 0.25 {
        p.warn
    } else {
        p.crit
    }
}

fn cursor(state: &AppState, pane: Pane) -> usize {
    state.pane_nav[pane.index()].cursor
}

fn render_body(frame: &mut Frame, area: Rect, title: &str, active: bool, p: Palette, lines: Vec<Line>) {
    let block = pane_block(title, active, p);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(lines), inner);
}

pub fn render_map(frame: &mut Frame, area: Rect, state: &AppState, active: bool, p: Palette) {
    let dim = Style::default().fg(p.dim);
    let mut lines = Vec::new();
    match state.probe_sector_coords() {
        Some((x, y, z)) => lines.push(Line::styled(format!("sector ({x}, {y}, {z})"), Style::default().fg(p.text))),
        None => lines.push(Line::styled("sector unknown", dim)),
    }
    lines.push(Line::styled(format!("visited: {}", state.visited_sectors.len()), Style::default().fg(p.text)));
    let nets = state.scut_coverage();
    if nets.is_empty() {
        lines.push(Line::styled("SCUT: no coverage", dim));
    } else {
        lines.push(Line::styled(format!("≣ SCUT: {} network(s)", nets.len()), Style::default().fg(p.accent)));
    }
    lines.push(Line::styled("[z] zoom for full map", dim));
    render_body(frame, area, " MAP ", active, p, lines);
}

pub fn render_comms(frame: &mut Frame, area: Rect, state: &AppState, active: bool, p: Palette) {
    if let Some(DrillLevel::MessageThread(id)) = state.pane_nav[Pane::Comms.index()].drill.last() {
        return render_message_detail(frame, area, state, id, active, p);
    }
    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);
    let unread_alerts = state.unread_alert_count();
    let unread_msgs = state.unread_message_count();
    let mut lines = vec![
        Line::from(vec![
            Span::styled(format!("alerts {} ", state.alerts.len()), text),
            Span::styled(format!("({unread_alerts} unread)"), dim),
            Span::styled(format!("  warn {}", state.damage_warnings.len()), text),
        ]),
        Line::from(vec![
            Span::styled(format!("messages {} ", state.messages.len()), text),
            Span::styled(format!("({unread_msgs} unread)"), dim),
        ]),
        Line::raw(""),
    ];
    let cur = cursor(state, Pane::Comms);
    if state.messages.is_empty() {
        lines.push(Line::styled("no messages", dim));
    } else {
        for (i, m) in state.messages.iter().enumerate() {
            let unread = m.status == crate::api::types::MessageStatus::Unread;
            let mark = if unread { "✉" } else { "·" };
            let body: String = m.body.chars().take(18).collect();
            lines.push(Line::from(Span::styled(
                format!("{mark} {}: {}", m.sender.name, body),
                row_style(active, i == cur).patch(text),
            )));
        }
    }
    render_body(frame, area, " COMMS ", active, p, lines);
}

fn render_message_detail(frame: &mut Frame, area: Rect, state: &AppState, id: &str, active: bool, p: Palette) {
    let block = pane_block(" MESSAGE ", active, p);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let dim = Style::default().fg(p.dim);
    let Some(m) = state.messages.iter().find(|m| m.id.to_string() == id) else {
        frame.render_widget(Paragraph::new(Line::styled("message not found", dim)), inner);
        return;
    };
    let lines = vec![
        Line::from(vec![Span::styled("from ", dim), Span::styled(m.sender.name.clone(), Style::default().fg(p.text))]),
        Line::from(vec![Span::styled("to   ", dim), Span::styled(m.recipient.name.clone(), Style::default().fg(p.text))]),
        Line::styled(m.created_at.clone(), dim),
        Line::raw(""),
        Line::styled(m.body.clone(), Style::default().fg(p.text)),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

pub fn render_sector(frame: &mut Frame, area: Rect, state: &AppState, active: bool, p: Palette) {
    let dim = Style::default().fg(p.dim);
    let mut lines = Vec::new();
    match state.current_sector() {
        Some(s) => {
            let v = &s.relative_coordinates;
            lines.push(Line::styled(
                format!("({}, {}, {})  d{}", v.x as i32, v.y as i32, v.z as i32, s.distance),
                Style::default().fg(p.text),
            ));
            let objs = state.scanner_objects();
            lines.push(Line::styled(format!("{} object(s)", objs.len()), dim));
            let cur = cursor(state, Pane::Sector);
            for (i, e) in objs.iter().enumerate() {
                let (icon, color) = object_icon(&e.object_type);
                let name: String = e.name.chars().take(20).collect();
                lines.push(Line::from(vec![
                    Span::styled(format!("{icon} "), Style::default().fg(color)),
                    Span::styled(name, row_style(active, i == cur).patch(Style::default().fg(p.text))),
                ]));
            }
        }
        None => lines.push(Line::styled("no sector scan yet", dim)),
    }
    render_body(frame, area, " SECTOR ", active, p, lines);
}

pub fn render_missions(frame: &mut Frame, area: Rect, state: &AppState, active: bool, p: Palette) {
    if let Some(DrillLevel::Mission(id)) = state.pane_nav[Pane::Missions.index()].drill.last() {
        return render_mission_detail(frame, area, state, id, active, p);
    }
    let dim = Style::default().fg(p.dim);
    let mut lines = Vec::new();
    if state.missions.is_empty() {
        lines.push(Line::styled("no active missions", dim));
    } else {
        let cur = cursor(state, Pane::Missions);
        for (i, m) in state.missions.iter().enumerate() {
            let color = match m.status {
                MissionStatus::Active => p.accent,
                MissionStatus::Completed => p.good,
                MissionStatus::Failed | MissionStatus::Abandoned => p.crit,
                MissionStatus::Unknown => p.dim,
            };
            let done = m
                .steps
                .iter()
                .filter(|s| matches!(s.status, MissionStepStatus::Completed))
                .count();
            let title: String = m.title.chars().take(22).collect();
            lines.push(Line::from(vec![
                Span::styled("▸ ", Style::default().fg(color)),
                Span::styled(title, row_style(active, i == cur).patch(Style::default().fg(p.text))),
                Span::styled(format!(" {done}/{}", m.steps.len()), dim),
            ]));
        }
    }
    render_body(frame, area, " MISSIONS ", active, p, lines);
}

fn render_mission_detail(frame: &mut Frame, area: Rect, state: &AppState, id: &str, active: bool, p: Palette) {
    let block = pane_block(" MISSION ", active, p);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let dim = Style::default().fg(p.dim);
    let Some(m) = state.missions.iter().find(|m| m.id == id) else {
        frame.render_widget(Paragraph::new(Line::styled("mission not found", dim)), inner);
        return;
    };
    let mut lines = vec![Line::styled(
        m.title.clone(),
        Style::default().fg(p.text).add_modifier(Modifier::BOLD),
    )];
    if let Some(d) = &m.description {
        lines.push(Line::styled(d.clone(), dim));
    }
    lines.push(Line::raw(""));
    let cur = cursor(state, Pane::Missions);
    for (i, step) in m.steps.iter().enumerate() {
        let (mark, color) = match step.status {
            MissionStepStatus::Completed => ("✓", p.good),
            MissionStepStatus::Failed => ("✗", p.crit),
            MissionStepStatus::Skipped => ("–", p.dim),
            MissionStepStatus::Pending => ("·", p.accent),
            MissionStepStatus::Unknown => ("?", p.dim),
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{mark} "), Style::default().fg(color)),
            Span::styled(step.title.clone(), row_style(active, i == cur).patch(Style::default().fg(p.text))),
        ]));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

pub fn render_storage(frame: &mut Frame, area: Rect, state: &AppState, active: bool, p: Palette) {
    let dim = Style::default().fg(p.dim);
    let mut lines = Vec::new();
    if state.storage_containers.is_empty() {
        lines.push(Line::styled("no containers ([C] loads)", dim));
    } else {
        let cur = cursor(state, Pane::Storage);
        for (i, c) in state.storage_containers.iter().enumerate() {
            let ratio = if c.capacity > 0.0 { c.used_capacity / c.capacity } else { 0.0 };
            let label: String = c.label.chars().take(16).collect();
            lines.push(Line::from(vec![
                Span::styled(label, row_style(active, i == cur).patch(Style::default().fg(p.text))),
                Span::styled(
                    format!(" {:.0}/{:.0}", c.used_capacity, c.capacity),
                    Style::default().fg(fill_color(p, 1.0 - ratio)),
                ),
            ]));
        }
    }
    render_body(frame, area, " STORAGE ", active, p, lines);
}
