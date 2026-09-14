use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::api::types::{ProbeSector, ScutRelayStatus};
use crate::app::{ActiveWizard, AppState, ScutNetworkInput};

use super::{centered_rect, render_footer, render_pick_list, FooterKey};
use crate::ui::cockpit_v2::panes::truncate_spans;
use crate::ui::theme::{scroll_markers, Palette};

/// Geometry of the network-view popup. Shared by the renderer and the input
/// layer so the scroll bound is computed against the rect actually drawn.
fn viewing_popup(area: Rect) -> Rect {
    centered_rect(74, 80, area)
}

/// Inner width and content height of the network-view body, the footer row
/// already taken out. Read from the terminal, the way the pane viewports are
/// (`cockpit_v2::active_pane_inner_size`), because the input layer has no
/// frame to measure.
pub(crate) fn viewing_viewport() -> (u16, u16) {
    let (w, h) = crossterm::terminal::size().unwrap_or((80, 24));
    let popup = viewing_popup(Rect::new(0, 0, w, h));
    let inner = popup.width.saturating_sub(2);
    (inner, popup.height.saturating_sub(3))
}

/// Rendered height of the network-view body at `width`.
///
/// The bound is **exact**, the way the Scanner detail column's is (#347) and
/// unlike the Manny detail's estimate-plus-slack (#337): the body carries no
/// wrapping — a relay line too long for the popup is truncated with an
/// ellipsis, as the ship's log truncates its own — so its line count is its
/// rendered height, and `viewing_lines` is the very list the renderer draws.
pub(crate) fn viewing_height(state: &AppState, error: Option<&String>, width: u16) -> usize {
    viewing_lines(state, state.palette(), error, width).len()
}

fn rel(sector: &ProbeSector) -> String {
    match sector.relative.as_ref() {
        Some(v) => format!("({},{},{})", v.x as i64, v.y as i64, v.z as i64),
        None => "(?)".into(),
    }
}

pub(crate) fn render_scut_network_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = state.palette();
    let ActiveWizard::ScutNetwork(scut_network) = &state.active_wizard else {
        return;
    };
    match scut_network {
        ScutNetworkInput::Picking { networks, selection } => {
            let items: Vec<&str> = networks.iter().map(|(_, name)| name.as_str()).collect();
            let height = (items.len() as u16) + 4;
            render_pick_list(
                frame,
                area,
                state.palette(),
                " SCUT NETWORK ",
                52,
                height,
                Some("Pick a network to inspect"),
                &items,
                *selection,
                None,
                "INSPECT",
            );
        }
        ScutNetworkInput::Viewing { error, offset } => {
            let popup = viewing_popup(area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(" SCUT NETWORK ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(p.accent));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let lines = viewing_lines(state, p, error.as_ref(), rows[0].width);
            // Clamp here as well as in the handler: the terminal may have been
            // resized since the last keypress, and an offset past the end
            // would render an empty popup over a network that is all there.
            let total = lines.len();
            let max = total.saturating_sub(rows[0].height as usize);
            let offset = (*offset).min(max) as u16;
            frame.render_widget(Paragraph::new(lines).scroll((offset, 0)), rows[0]);
            // The markers go on the frame, so they are handed a rect whose
            // border rows are the popup's own and whose content height is the
            // body's — the footer row taken out (issue #326).
            let marker_area = Rect {
                height: popup.height.saturating_sub(1),
                ..popup
            };
            scroll_markers(frame, marker_area, offset, total, true, p);

            let mut keys = vec![FooterKey::nav("[Esc]", "close")];
            if total > rows[0].height as usize {
                keys.insert(0, FooterKey::nav("[jk]", "scroll"));
            }
            render_footer(frame, rows[1], p, &keys);
        }
    }
}

/// The network-view body, exactly as the renderer draws it — the input layer
/// measures this same list so the scroll bound cannot drift from the content.
fn viewing_lines(state: &AppState, p: Palette, error: Option<&String>, width: u16) -> Vec<Line<'static>> {
    let w = width as usize;
    let row = |spans: Vec<Span<'static>>| Line::from(truncate_spans(spans, w));
    let mut lines: Vec<Line> = Vec::new();
    if let Some(err) = error {
        lines.push(Line::from(Span::styled(
            format!("\u{2717} {err}"),
            Style::default().fg(p.crit),
        )));
        return lines;
    }
    let Some(net) = &state.scut_network_view else {
        lines.push(Line::from(Span::styled("loading\u{2026}", Style::default().fg(p.dim))));
        return lines;
    };
    lines.push(row(vec![
        Span::styled(
            net.name.clone(),
            Style::default().fg(p.text).add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled(
            format!(
                "{} relays \u{b7} {} sectors covered",
                net.relay_count, net.covered_sector_count
            ),
            Style::default().fg(p.text),
        ),
    ]));
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "RELAYS",
        Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
    )));
    for r in &net.relays {
        let (mark, color) = match r.status {
            ScutRelayStatus::On => ("\u{25cf}", p.good),
            ScutRelayStatus::Off => ("\u{25cb}", p.dim),
            ScutRelayStatus::Unknown => ("?", p.dim),
        };
        let by = r.created_by_probe_name.clone().unwrap_or_else(|| "\u{2014}".into());
        lines.push(row(vec![
            Span::styled(format!("  {mark} "), Style::default().fg(color)),
            Span::styled(rel(&r.sector), Style::default().fg(p.text)),
            Span::styled(
                format!("  r={}  by {by}", r.coverage_radius_sectors),
                Style::default().fg(p.dim),
            ),
        ]));
    }
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "PROBES",
        Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
    )));
    if net.probes.is_empty() {
        lines.push(Line::from(Span::styled("  none detected", Style::default().fg(p.dim))));
    } else {
        for probe in &net.probes {
            lines.push(Line::from(vec![
                Span::styled("  \u{25c6} ", Style::default().fg(p.accent)),
                Span::styled(probe.name.clone(), Style::default().fg(p.text)),
                Span::styled(format!("  {}", rel(&probe.sector)), Style::default().fg(p.dim)),
            ]));
        }
    }
    lines
}
