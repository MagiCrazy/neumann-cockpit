use crate::app::{AppState, TravelInput};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui::theme::{format_duration, palette};
use super::{centered_rect, render_footer, FooterKey};
pub(crate) fn render_travel_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let popup = centered_rect(46, 11, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" TRAVEL ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.warn));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    let body = rows[0];
    let hint_area = rows[1];

    match &state.travel {
        TravelInput::Inactive => {}

        TravelInput::Typing(buf) => {
            let mut lines: Vec<Line> = Vec::new();

            if let Some((px, py, pz)) = state.probe_sector_coords() {
                lines.push(Line::from(vec![
                    Span::styled("From: ", Style::default().fg(p.dim)),
                    Span::styled(
                        format!("({px},{py},{pz})"),
                        Style::default().fg(p.text),
                    ),
                ]));
            }

            lines.push(Line::from(vec![
                Span::styled("Destination (x y z): ", Style::default().fg(p.accent)),
                Span::raw(buf.as_str()),
                Span::styled("█", Style::default().fg(p.accent)),
            ]));
            lines.push(Line::from(Span::styled(
                "prefix with + for relative (e.g. +2 0 -2)",
                Style::default().fg(p.dim),
            )));

            // Live resolution + parity check
            if let Some((x, y, z)) = state.resolve_travel_target() {
                let parity_ok = (x + y + z) % 2 == 0;
                let mut spans = vec![
                    Span::styled("→ ", Style::default().fg(p.warn)),
                    Span::styled(
                        format!("({x},{y},{z})"),
                        Style::default().fg(p.text).add_modifier(Modifier::BOLD),
                    ),
                ];
                if parity_ok {
                    spans.push(Span::styled("  ✓", Style::default().fg(p.good)));
                } else {
                    spans.push(Span::styled(
                        "  ✗ x+y+z must be even",
                        Style::default().fg(p.crit),
                    ));
                }
                lines.push(Line::default());
                lines.push(Line::from(spans));
            } else if buf.trim_start().starts_with('+') && state.probe_sector_coords().is_none() {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    "✗ relative input needs a known probe position",
                    Style::default().fg(p.crit),
                )));
            }

            frame.render_widget(Paragraph::new(lines), body);
            render_footer(frame, hint_area, p, &[
                FooterKey::nav("[Enter]", "preview"),
                FooterKey::nav("[Esc]", "cancel"),
            ]);
        }

        TravelInput::Confirming { x, y, z, sector_distance, fuel_cost, eta_minutes, error } => {
            let mut lines: Vec<Line> = Vec::new();

            lines.push(Line::from(vec![
                Span::styled("→  ", Style::default().fg(p.warn)),
                Span::styled(
                    format!("({x}, {y}, {z})"),
                    Style::default().fg(p.text).add_modifier(Modifier::BOLD),
                ),
            ]));

            if let Some(dist) = sector_distance {
                lines.push(Line::from(vec![
                    Span::styled("   Distance  ", Style::default().fg(p.dim)),
                    Span::raw(format!("{dist} sector(s)")),
                ]));
            }

            if let Some(fuel) = fuel_cost {
                lines.push(Line::from(vec![
                    Span::styled("   Fuel      ", Style::default().fg(p.dim)),
                    Span::styled(format!("{fuel:.4}"), Style::default().fg(p.accent)),
                    Span::styled(" units", Style::default().fg(p.dim)),
                ]));
            }

            if let Some(mins) = eta_minutes {
                lines.push(Line::from(vec![
                    Span::styled("   ETA       ", Style::default().fg(p.dim)),
                    Span::styled(
                        format_duration(mins * 60),
                        Style::default().fg(p.warn),
                    ),
                ]));
            }

            if let Some(err) = error {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("   ✗ {err}"),
                    Style::default().fg(p.crit),
                )));
            }

            frame.render_widget(Paragraph::new(lines), body);

            if error.is_some() {
                render_footer(frame, hint_area, p, &[FooterKey::nav("[Esc]", "cancel")]);
            } else {
                render_footer(frame, hint_area, p, &[
                    FooterKey::commit("[Enter]", "GO"),
                    FooterKey::nav("[Esc]", "cancel"),
                ]);
            }
        }
    }
}

