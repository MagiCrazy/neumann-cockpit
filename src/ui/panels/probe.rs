use crate::api::types::{MovementPhase, SensorMode};
use crate::app::AppState;
use chrono::Utc;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::theme::{
    block_gauge_line, format_duration, movement_phase_label, palette, pane_block, probe_status_label,
    probe_status_style, ratio_color,
};
// ── Probe panel ───────────────────────────────────────────────────────────────

pub(crate) fn render_probe_panel(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let p = palette(state.color_mode);
    let block = pane_block(" PROBE ", focused, p);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    // Breathing room: a one-column margin inside the border.
    let content = Rect {
        x: inner.x + 1,
        y: inner.y,
        width: inner.width.saturating_sub(2),
        height: inner.height,
    };

    let dim = Style::default().fg(p.dim);
    let text = Style::default().fg(p.text);
    // A dim, left-aligned label column so every row lines up.
    let label = |s: &str| Span::styled(format!("{s:<7} "), dim);

    let Some(probe) = &state.probe else {
        let msg = if state.loading { "fetching…" } else { "no data — F5 to refresh" };
        frame.render_widget(Paragraph::new(msg).style(dim), content);
        return;
    };

    let mut lines: Vec<Line> = Vec::new();

    // ── Identity ──
    lines.push(Line::styled(probe.name.clone(), text.add_modifier(Modifier::BOLD)));
    lines.push(Line::from(vec![
        label("status"),
        Span::styled(probe_status_label(&probe.status), probe_status_style(&probe.status)),
    ]));
    let (sensor_txt, sensor_col) = match probe.sensor_mode {
        SensorMode::Normal => ("normal", p.good),
        SensorMode::Degraded => ("degraded", p.warn),
        SensorMode::Blind => ("BLIND", p.crit),
        SensorMode::Unknown => ("?", p.dim),
    };
    lines.push(Line::from(vec![
        label("sensor"),
        Span::styled(sensor_txt, Style::default().fg(sensor_col)),
    ]));

    // ── Badges ──
    if state.probe_terminal_alert().is_some() {
        lines.push(Line::styled("⚠ RECOVER — Enter", p.crit_style()));
    }
    let unread = state.unread_alert_count();
    if unread > 0 {
        lines.push(Line::from(vec![
            label("alerts"),
            Span::styled(format!("{unread} unread"), p.crit_style()),
        ]));
    }
    if !state.scut_coverage().is_empty() {
        lines.push(Line::styled("≣ SCUT coverage", Style::default().fg(p.accent)));
    }

    // ── Vital gauges ──
    lines.push(Line::raw(""));
    let fuel = probe.fuel.deuterium.map(|d| (d / 100.0).clamp(0.0, 1.0)).unwrap_or(0.0);
    lines.push(block_gauge_line("FUEL", fuel, &format!("{:.0}%", fuel * 100.0), ratio_color(fuel, p), p));
    if let Some(sys) = &probe.systems {
        let integ = (sys.integrity_percent.unwrap_or(100.0) / 100.0).clamp(0.0, 1.0);
        lines.push(block_gauge_line("INTEG", integ, &format!("{:.0}%", integ * 100.0), ratio_color(integ, p), p));
    }
    let inv = &probe.inventory;
    let cargo = if inv.capacity > 0.0 { (inv.used_capacity / inv.capacity).clamp(0.0, 1.0) } else { 0.0 };
    lines.push(block_gauge_line("CARGO", cargo, &format!("{:.0}%", cargo * 100.0), p.accent, p));

    // ── Movement (active) or current sector ──
    let active = probe.movement.as_ref().filter(|mv| {
        !matches!(
            mv.phase.as_ref().unwrap_or(&mv.status),
            MovementPhase::Arrived | MovementPhase::Failed | MovementPhase::Destroyed | MovementPhase::Idle
        )
    });
    if let Some(mv) = active {
        let remaining = (mv.arrival_at - Utc::now()).num_seconds().max(0);
        let elapsed = (Utc::now() - mv.started_at).num_seconds().max(0);
        let total = (mv.arrival_at - mv.started_at).num_seconds().max(1);
        let progress = (elapsed as f64 / total as f64).clamp(0.0, 1.0);
        let velocity = mv.estimated_velocity_c.unwrap_or(0.0).clamp(0.0, 1.0);

        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            label("travel"),
            Span::styled(
                format!(
                    "({},{},{}) → ({},{},{})",
                    mv.origin.x as i64, mv.origin.y as i64, mv.origin.z as i64,
                    mv.target.x as i64, mv.target.y as i64, mv.target.z as i64,
                ),
                text,
            ),
        ]));
        lines.push(Line::from(vec![label("dist"), Span::styled(format!("{}", mv.distance), text)]));
        lines.push(Line::from(vec![
            label("phase"),
            Span::styled(movement_phase_label(mv.phase.as_ref().unwrap_or(&mv.status)), Style::default().fg(p.warn)),
        ]));
        lines.push(Line::from(vec![label("ETA"), Span::styled(format_duration(remaining), text)]));
        lines.push(block_gauge_line("BURN", progress, &format!("{:.0}%", progress * 100.0), p.accent, p));
        lines.push(block_gauge_line("SPEED", velocity, &format!("{velocity:.2}c"), p.accent, p));
        lines.push(Line::from(vec![
            label("cost"),
            Span::styled(format!("{:.1} deut", mv.fuel_cost_deuterium), dim),
        ]));
    } else if let Some(rel) = probe.sector.as_ref().and_then(|s| s.relative.as_ref()) {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            label("sector"),
            Span::styled(format!("({}, {}, {})", rel.x as i64, rel.y as i64, rel.z as i64), text),
        ]));
    }

    // ── Extras (shown when the pane is tall enough) ──
    if let Some(sys) = &probe.systems {
        lines.push(Line::raw(""));
        if let Some(e) = sys.energy_stored {
            lines.push(Line::from(vec![label("energy"), Span::styled(format!("{e:.0}"), text)]));
        }
        if let Some(rate) = sys.internal_clock_rate {
            lines.push(Line::from(vec![label("clock"), Span::styled(format!("{rate:.2}×"), text)]));
        }
    }
    lines.push(Line::from(vec![
        label("hold"),
        Span::styled(
            format!("{} items · {} stocks · {} tanks", inv.items.len(), inv.resource_stocks.len(), inv.external_tanks.len()),
            dim,
        ),
    ]));

    frame.render_widget(Paragraph::new(lines), content);
}
