use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{AppState, ProbeSwitchInput, RenameProbeInput};
use crate::ui::theme::{palette, probe_status_label};

use super::{centered_rect, render_footer, render_pick_list, FooterKey};

/// Fleet picker (API v81): one row per probe with a default (★) / active (▸)
/// marker, its status, and SCUT reachability. Selecting a reachable row pilots
/// that probe; an unreachable one is refused (see the input handler).
pub(crate) fn render_probe_switch_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let ProbeSwitchInput::Picking { selection } = state.probe_switch else {
        return;
    };
    let p = palette(state.color_mode);
    let active = state.active_probe_id.or(state.default_probe_id);
    let labels: Vec<String> = state
        .fleet
        .iter()
        .map(|pr| {
            let mark = if pr.is_default {
                "★"
            } else if Some(pr.id) == active {
                "▸"
            } else {
                " "
            };
            let reach = if pr.is_reachable { "" } else { "   ⚠ out of SCUT range" };
            format!("{mark} {}  ·  {}{reach}", pr.name, probe_status_label(&pr.status))
        })
        .collect();
    let refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
    let height = (refs.len() as u16 + 6).clamp(8, 20);
    render_pick_list(
        frame, area, p, " SWITCH PROBE ", 52, height,
        Some("Pilot:"), &refs, selection, None, "pilot",
    );
}

/// Text-entry overlay to rename the piloted probe (API v81).
pub(crate) fn render_rename_probe_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let RenameProbeInput::Typing { current_name, buf, error, .. } = &state.rename_probe else {
        return;
    };
    let p = palette(state.color_mode);
    let popup = centered_rect(48, 7, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(format!(" RENAME PROBE — {current_name} "))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("Name: ", Style::default().fg(p.accent)),
        Span::raw(buf.as_str()),
        Span::styled("█", Style::default().fg(p.accent)),
    ]));
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(format!("✗ {err}"), Style::default().fg(p.crit))));
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);
    render_footer(frame, rows[1], p, &[
        FooterKey::commit("[Enter]", "RENAME"),
        FooterKey::nav("[Esc]", "cancel"),
    ]);
}
