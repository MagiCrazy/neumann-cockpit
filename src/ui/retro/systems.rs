use crate::api::types::MovementPhase;
use crate::app::{AppState, Panel};
use crate::ui::theme::format_duration;
use chrono::Utc;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::palette::{block_gauge, Pal};
use super::section_title;

fn gauge_line<'a>(label: &'a str, ratio: f64, pct: String, p: &Pal, frame: u64, alert: bool) -> Line<'a> {
    let mut spans = vec![Span::styled(format!("  {label:<5}"), p.dim())];
    for (cell, hot) in block_gauge(ratio, 10, frame) {
        let style = if alert {
            p.alert()
        } else if hot {
            p.bright()
        } else {
            p.norm()
        };
        spans.push(Span::styled(cell, style));
    }
    spans.push(Span::styled(format!(" {pct:>4}"), p.bright()));
    Line::from(spans)
}

/// Status LED with a slow pulse when `active`.
fn led(active: bool, frame: u64, phase: u64) -> &'static str {
    if !active {
        return "○";
    }
    match (frame / 4 + phase) % 3 {
        0 => "◉",
        1 => "◎",
        _ => "◉",
    }
}

pub(super) fn render_systems(frame: &mut Frame, area: Rect, state: &AppState, p: &Pal) {
    let f = state.anim.frame;
    let focused = state.focused == Some(Panel::Probe);
    let mut lines: Vec<Line> = vec![section_title("SYSTEMS", focused, p), Line::default()];

    let Some(probe) = &state.probe else {
        lines.push(Line::from(Span::styled("  AWAITING TELEMETRY", p.dim())));
        frame.render_widget(Paragraph::new(lines), area);
        return;
    };

    let fuel = probe.fuel.deuterium.map(|d| (d / 100.0).clamp(0.0, 1.0)).unwrap_or(0.0);
    lines.push(gauge_line("FUEL", fuel, format!("{:.0}%", fuel * 100.0), p, f, fuel < 0.25));

    if let Some(sys) = &probe.systems {
        let intg = (sys.integrity_percent.unwrap_or(100.0) / 100.0).clamp(0.0, 1.0);
        lines.push(gauge_line("INTG", intg, format!("{:.0}%", intg * 100.0), p, f.wrapping_add(7), intg < 0.25));
    }

    // Active burn: progress + ETA
    let active_mv = probe.movement.as_ref().filter(|mv| {
        !matches!(
            mv.phase.as_ref().unwrap_or(&mv.status),
            MovementPhase::Arrived | MovementPhase::Failed | MovementPhase::Destroyed | MovementPhase::Idle
        )
    });
    if let Some(mv) = active_mv {
        let elapsed = (Utc::now() - mv.started_at).num_seconds().max(0);
        let total = (mv.arrival_at - mv.started_at).num_seconds().max(1);
        let progress = (elapsed as f64 / total as f64).clamp(0.0, 1.0);
        let remaining = (mv.arrival_at - Utc::now()).num_seconds().max(0);
        lines.push(gauge_line("BURN", progress, format!("{:.0}%", progress * 100.0), p, f.wrapping_add(3), false));
        // Living ETA: the colon blinks once per second.
        let eta = format_duration(remaining);
        let eta = if (f / 5).is_multiple_of(2) { eta } else { eta.replace(':', " ") };
        lines.push(Line::from(vec![
            Span::styled("  ETA  ", p.dim()),
            Span::styled(eta, p.bright()),
            Span::styled(
                format!("  → ({},{},{})", mv.target.x as i64, mv.target.y as i64, mv.target.z as i64),
                p.norm(),
            ),
        ]));
    } else if let Some((x, y, z)) = state.probe_sector_coords() {
        lines.push(Line::from(vec![
            Span::styled("  POS  ", p.dim()),
            Span::styled(format!("({x},{y},{z})"), p.bright()),
            Span::styled("  HOLDING", p.dim()),
        ]));
    }

    // Probe schematic with status LEDs
    let busy = state.loading || state.scan_loading;
    let has_fuel = fuel > 0.0;
    lines.push(Line::default());
    lines.push(Line::from(Span::styled("      ┌──╥──┐", p.norm())));
    lines.push(Line::from(vec![
        Span::styled("     ═╡ ", p.norm()),
        Span::styled("CORE", p.bright()),
        Span::styled(" ╞═", p.norm()),
    ]));
    lines.push(Line::from(Span::styled("      └──╨──┘", p.norm())));
    lines.push(Line::from(vec![
        Span::styled("      ", p.dim()),
        Span::styled(led(active_mv.is_some(), f, 0), p.bright()),
        Span::styled("   ", p.dim()),
        Span::styled(led(has_fuel, f, 1), p.bright()),
        Span::styled("   ", p.dim()),
        Span::styled(led(busy, f, 2), if busy { p.bright() } else { p.dim() }),
    ]));
    lines.push(Line::from(Span::styled("     NAV  PWR  COM", p.dim())));

    frame.render_widget(Paragraph::new(lines), area);
}
