use crate::api::types::SensorMode;
use crate::app::AppState;
use crate::ui::theme::probe_status_label;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::palette::Pal;

const SPINNER: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

/// LINK heartbeat: a slow pulse showing the uplink is alive.
fn heartbeat(frame: u64) -> &'static str {
    match (frame / 5) % 4 {
        0 => "◉",
        1 => "◎",
        2 => "○",
        _ => "◎",
    }
}

pub(super) fn render_banner(frame: &mut Frame, area: Rect, state: &AppState, p: &Pal) {
    let f = state.anim.frame;

    // Line 1: system identity + clock
    let clock = chrono::Local::now().format("%H:%M:%S").to_string();
    let api = state
        .api_version
        .map(|v| format!("API.{v}"))
        .unwrap_or_else(|| "API.--".into());
    let line1 = Line::from(vec![
        Span::styled(
            format!(" NEUMANN/OS {} ", env!("CARGO_PKG_VERSION")),
            p.bold(),
        ),
        Span::styled("─── DEEP SPACE PROBE COMMAND ───", p.dim()),
        Span::styled(format!(" {api} "), p.norm()),
        Span::styled(format!(" T {clock} "), p.bright()),
    ]);

    // Line 2: probe identity + status lights
    let (name, status) = match &state.probe {
        Some(probe) => (probe.name.clone(), probe_status_label(&probe.status).to_uppercase()),
        None => ("--------".into(), "NO DATA".into()),
    };
    let sensors = state
        .probe
        .as_ref()
        .map(|pr| match pr.sensor_mode {
            SensorMode::Normal => ("▣▣▣▣▣▣", false),
            SensorMode::Degraded => ("▣▣▣▢▢▢", false),
            SensorMode::Blind => ("▢▢▢▢▢▢", true),
            SensorMode::Unknown => ("▢▢▢▢▢▢", false),
        })
        .unwrap_or(("▢▢▢▢▢▢", false));

    let busy = state.loading || state.scan_loading;
    let spinner = if busy {
        SPINNER[(f % SPINNER.len() as u64) as usize]
    } else {
        " "
    };

    let mut spans = vec![
        Span::styled("  PROBE: ", p.dim()),
        Span::styled(name, p.bold()),
        Span::styled("   MODE: ", p.dim()),
        Span::styled(status, p.bright()),
        Span::styled("   LINK ", p.dim()),
        Span::styled(heartbeat(f), p.bright()),
        Span::styled("   SENSORS ", p.dim()),
    ];
    spans.push(Span::styled(
        sensors.0,
        if sensors.1 { p.alert() } else { p.norm() },
    ));
    spans.push(Span::styled("   ", p.dim()));
    spans.push(Span::styled(spinner, p.bright()));

    let rows = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(area);
    frame.render_widget(Paragraph::new(line1), rows[0]);
    frame.render_widget(Paragraph::new(Line::from(spans)), rows[1]);
}
