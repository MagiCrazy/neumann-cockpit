use crate::app::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::palette::Pal;

/// Frames between two boot lines appearing (~10 fps tick).
const LINE_STRIDE: u64 = 4;
/// Characters revealed per frame on the active line (teletype effect).
const CHARS_PER_FRAME: usize = 4;

fn boot_lines(state: &AppState) -> Vec<(String, String)> {
    let api = state
        .api_version
        .map(|v| format!("v{v} OK"))
        .unwrap_or_else(|| "PENDING".into());
    vec![
        ("ROM CHECK".into(), "OK".into()),
        ("DEUTERIUM FLOW REGULATOR".into(), "OK".into()),
        ("SENSOR PHASED ARRAY".into(), "6/6 NOMINAL".into()),
        ("MANNY BAY INTERLOCKS".into(), "OK".into()),
        ("UPLINK HANDSHAKE".into(), api),
        (
            "RESTORING SCAN ARCHIVE".into(),
            format!("{} SECTORS", state.scan_history.len()),
        ),
    ]
}

pub(super) fn render_boot(frame: &mut Frame, area: Rect, state: &AppState, p: &Pal) {
    let bf = state.anim.boot_frame;
    let lines_data = boot_lines(state);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(28),
            Constraint::Min(10),
            Constraint::Percentage(20),
        ])
        .split(area);
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(18),
            Constraint::Min(40),
            Constraint::Percentage(18),
        ])
        .split(rows[1])[1];

    let mut out: Vec<Line> = vec![
        Line::from(Span::styled(
            "███ NEUMANN SYSTEMS UNIFIED PROBE COMPUTER ███",
            p.bold(),
        )),
        Line::default(),
    ];

    for (i, (label, result)) in lines_data.iter().enumerate() {
        let start = i as u64 * LINE_STRIDE;
        if bf < start {
            break;
        }
        let revealed = ((bf - start) as usize) * CHARS_PER_FRAME;
        let dotted = format!("> {label} {}", ".".repeat(34_usize.saturating_sub(label.len())));
        if revealed >= dotted.chars().count() {
            out.push(Line::from(vec![
                Span::styled(dotted, p.norm()),
                Span::styled(format!(" {result}"), p.bright()),
            ]));
        } else {
            let partial: String = dotted.chars().take(revealed).collect();
            out.push(Line::from(Span::styled(partial, p.norm())));
        }
    }

    let all_done = bf >= lines_data.len() as u64 * LINE_STRIDE + 6;
    if all_done {
        out.push(Line::default());
        out.push(Line::from(Span::styled(
            "INTERFACE 2337 READY FOR INQUIRY",
            p.bold(),
        )));
    }
    // Blinking cursor on its own line.
    out.push(Line::default());
    out.push(Line::from(Span::styled(
        if (bf / 3).is_multiple_of(2) { "▌" } else { " " },
        p.bright(),
    )));

    frame.render_widget(Paragraph::new(out), body);

    let hint = Paragraph::new(Line::from(Span::styled(
        "  ANY KEY TO SKIP",
        p.dim(),
    )));
    frame.render_widget(hint, rows[2]);
}
