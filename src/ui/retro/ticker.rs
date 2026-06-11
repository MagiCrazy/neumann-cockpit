use crate::app::{anim_hash, AppState};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::palette::Pal;

/// One pseudo-telemetry segment, deterministic in `slot`.
pub(crate) fn tlm_segment(slot: u64) -> String {
    let h = anim_hash(slot.wrapping_mul(0x9e37_79b9));
    match h % 6 {
        0 => format!("{:02X}.{:02X}.{:04X}", h >> 8 & 0xff, h >> 16 & 0xff, h >> 24 & 0xffff),
        1 => format!("GYRO {:+.2} {:+.2} {:+.2}",
            ((h >> 8) % 40) as f64 / 100.0 - 0.2,
            ((h >> 16) % 40) as f64 / 100.0 - 0.2,
            ((h >> 24) % 40) as f64 / 100.0 - 0.2),
        2 => format!("FLOW 0.{:04}", (h >> 8) % 10000),
        3 => format!("MEM {:>3}/512", 40 + (h >> 8) % 200),
        4 => "SENS:NOM".to_string(),
        _ => "∿∿─╱╲─∿∿".to_string(),
    }
}

/// COMMS line: latest event (toast), error, or the all-clear.
pub(super) fn render_comms(frame: &mut Frame, area: Rect, state: &AppState, p: &Pal) {
    let line = if let Some(err) = &state.error {
        Line::from(vec![
            Span::styled(" COMMS ▸ ", p.dim()),
            Span::styled(format!("ALERT — {}", err.to_uppercase()), p.alert()),
        ])
    } else if let Some(toast) = state.active_toast() {
        Line::from(vec![
            Span::styled(" COMMS ▸ ", p.dim()),
            Span::styled(toast.to_uppercase(), p.bright()),
        ])
    } else {
        Line::from(vec![
            Span::styled(" COMMS ▸ ", p.dim()),
            Span::styled("ALL CHANNELS NOMINAL", p.norm()),
        ])
    };
    frame.render_widget(Paragraph::new(line), area);
}

/// TLM line: scrolling pseudo-telemetry. The scroll speed quadruples while
/// an API request is in flight — the probe is visibly "thinking".
pub(super) fn render_tlm(frame: &mut Frame, area: Rect, state: &AppState, p: &Pal) {
    let f = state.anim.frame;
    let busy = state.loading || state.scan_loading;
    let cursor = if busy { f * 4 } else { f };

    // Build a stream of segments and window it by character offset.
    let slot0 = cursor / 24;
    let offset = (cursor % 24) as usize;
    let mut stream = String::new();
    let mut k = 0u64;
    while stream.chars().count() < area.width as usize + 32 {
        stream.push_str(&tlm_segment(slot0 + k));
        stream.push_str(" ▍ ");
        k += 1;
    }
    let window: String = stream.chars().skip(offset).take(area.width.saturating_sub(8) as usize).collect();

    let label_style = if busy { p.bright() } else { p.dim() };
    let line = Line::from(vec![
        Span::styled(" TLM ▸ ", label_style),
        Span::styled(window, p.dim()),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}
