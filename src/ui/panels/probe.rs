use crate::api::types::MovementPhase;
use crate::app::AppState;
use chrono::Utc;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::theme::{format_duration, gauge_color, make_line_gauge, movement_phase_label, panel_block, probe_status_label, probe_status_style};
// ── Probe panel ───────────────────────────────────────────────────────────────

pub(crate) fn render_probe_panel(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let block = panel_block(" PROBE ", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if state.loading && state.probe.is_none() {
        frame.render_widget(
            Paragraph::new("Fetching…").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let Some(probe) = &state.probe else {
        frame.render_widget(
            Paragraph::new("No data — press r to refresh")
                .style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    };

    let active_movement = probe.movement.as_ref().filter(|mv| {
        !matches!(
            mv.phase.as_ref().unwrap_or(&mv.status),
            MovementPhase::Arrived | MovementPhase::Failed | MovementPhase::Destroyed | MovementPhase::Idle
        )
    });

    let show_sector = active_movement.is_none()
        && probe.sector.as_ref().and_then(|s| s.relative.as_ref()).is_some();

    let mut sections: Vec<Constraint> = vec![
        Constraint::Length(1), // name + status
    ];
    if show_sector {
        sections.push(Constraint::Length(1)); // current sector coords
    }
    if active_movement.is_some() {
        sections.push(Constraint::Length(1)); // coords + distance
        sections.push(Constraint::Length(1)); // phase + ETA
        sections.push(Constraint::Length(1)); // progress gauge
        sections.push(Constraint::Length(1)); // speed gauge
    }
    sections.push(Constraint::Length(1)); // fuel gauge
    if probe.systems.is_some() {
        sections.push(Constraint::Length(1)); // integrity gauge
    }
    sections.push(Constraint::Min(0)); // padding

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(sections)
        .split(inner);

    let mut row = 0;

    // ── Header ──
    let spinner = if state.loading { " ⟳" } else { "" };
    let unread = state.unread_alert_count();
    let alert_badge = if unread > 0 {
        Span::styled(
            format!(" [!{unread}]"),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD | Modifier::RAPID_BLINK),
        )
    } else {
        Span::raw("")
    };
    let mut status_spans = vec![
        Span::styled(&probe.name, Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(probe_status_label(&probe.status), probe_status_style(&probe.status)),
        alert_badge,
        Span::styled(spinner, Style::default().fg(Color::DarkGray)),
    ];
    if state.probe_terminal_alert().is_some() {
        status_spans.push(Span::styled(
            "  [^R reassign]",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    }
    if !state.scut_coverage().is_empty() {
        status_spans.push(Span::styled(
            "  ≣ SCUT",
            Style::default().fg(Color::LightBlue),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(status_spans)), rows[row]);
    row += 1;

    // ── Current sector ──
    if show_sector {
        if let Some(rel) = probe.sector.as_ref().and_then(|s| s.relative.as_ref()) {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("@ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("({},{},{})", rel.x as i64, rel.y as i64, rel.z as i64),
                        Style::default().fg(Color::White),
                    ),
                ])),
                rows[row],
            );
            row += 1;
        }
    }

    // ── Movement ──
    if let Some(mv) = active_movement {
        let remaining = (mv.arrival_at - Utc::now()).num_seconds().max(0);
        let elapsed = (Utc::now() - mv.started_at).num_seconds().max(0);
        let total = (mv.arrival_at - mv.started_at).num_seconds().max(1);
        let progress = (elapsed as f64 / total as f64).clamp(0.0, 1.0);
        let phase_label = movement_phase_label(mv.phase.as_ref().unwrap_or(&mv.status));

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                Span::raw(format!(
                    "({},{},{}) → ({},{},{})  d:{}",
                    mv.origin.x as i64, mv.origin.y as i64, mv.origin.z as i64,
                    mv.target.x as i64, mv.target.y as i64, mv.target.z as i64,
                    mv.distance,
                )),
            ])),
            rows[row],
        );
        row += 1;

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(phase_label, Style::default().fg(Color::Yellow)),
                Span::raw(format!("  ETA: {}", format_duration(remaining))),
            ])),
            rows[row],
        );
        row += 1;

        frame.render_widget(
            make_line_gauge(&format!("{:<12}{:.0}%", "Travel", progress * 100.0), progress, Color::Yellow),
            rows[row],
        );
        row += 1;

        let velocity = mv.estimated_velocity_c.unwrap_or(0.0).clamp(0.0, 1.0);
        frame.render_widget(
            make_line_gauge(
                &format!("{:<12}{:.2}c", "Speed", velocity),
                velocity,
                Color::Yellow,
            ),
            rows[row],
        );
        row += 1;
    }

    // ── Fuel ──
    let fuel_ratio = probe.fuel.deuterium
        .map(|d| (d / 100.0).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    frame.render_widget(
        make_line_gauge(
            &format!("{:<12}{:.1}%", "Fuel", fuel_ratio * 100.0),
            fuel_ratio,
            gauge_color(fuel_ratio),
        ),
        rows[row],
    );
    row += 1;

    // ── Integrity ──
    if let Some(sys) = &probe.systems {
        let integrity = (sys.integrity_percent.unwrap_or(100.0) / 100.0).clamp(0.0, 1.0);
        frame.render_widget(
            make_line_gauge(
                &format!("{:<12}{:.1}%", "Integrity", integrity * 100.0),
                integrity,
                gauge_color(integrity),
            ),
            rows[row],
        );
        row += 1;
    }

    let _ = row;
}

