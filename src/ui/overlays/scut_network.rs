use crate::ui::theme::palette;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::api::types::{ProbeSector, ScutRelayStatus};
use crate::app::{ActiveWizard, AppState, ScutNetworkInput};

use super::{centered_rect, render_footer, render_pick_list, FooterKey};

fn rel(sector: &ProbeSector) -> String {
    match sector.relative.as_ref() {
        Some(v) => format!("({},{},{})", v.x as i64, v.y as i64, v.z as i64),
        None => "(?)".into(),
    }
}

pub(crate) fn render_scut_network_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let ActiveWizard::ScutNetwork(scut_network) = &state.active_wizard else { return };
    match scut_network {
        ScutNetworkInput::Picking { networks, selection } => {
            let items: Vec<&str> = networks.iter().map(|(_, name)| name.as_str()).collect();
            let height = (items.len() as u16) + 4;
            render_pick_list(
                frame, area, palette(state.color_mode), " SCUT NETWORK ",
                52,
                height,
                Some("Pick a network to inspect"),
                &items,
                *selection,
                None,
                "INSPECT",
            );
        }
        ScutNetworkInput::Viewing { error } => {
            let popup = centered_rect(74, 80, area);
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

            let mut lines: Vec<Line> = Vec::new();
            if let Some(err) = error {
                lines.push(Line::from(Span::styled(
                    format!("✗ {err}"),
                    Style::default().fg(p.crit),
                )));
            } else if let Some(net) = &state.scut_network_view {
                lines.push(Line::from(vec![
                    Span::styled(net.name.clone(), Style::default().fg(p.text).add_modifier(Modifier::BOLD)),
                    Span::raw("   "),
                    Span::styled(
                        format!("{} relays · {} sectors covered", net.relay_count, net.covered_sector_count),
                        Style::default().fg(p.text),
                    ),
                ]));
                lines.push(Line::default());
                lines.push(Line::from(Span::styled("RELAYS", Style::default().fg(p.accent).add_modifier(Modifier::BOLD))));
                for r in &net.relays {
                    let (mark, color) = match r.status {
                        ScutRelayStatus::On => ("●", p.good),
                        ScutRelayStatus::Off => ("○", p.dim),
                        ScutRelayStatus::Unknown => ("?", p.dim),
                    };
                    let by = r.created_by_probe_name.clone().unwrap_or_else(|| "—".into());
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {mark} "), Style::default().fg(color)),
                        Span::styled(rel(&r.sector), Style::default().fg(p.text)),
                        Span::styled(
                            format!("  r={}  by {by}", r.coverage_radius_sectors),
                            Style::default().fg(p.dim),
                        ),
                    ]));
                }
                lines.push(Line::default());
                lines.push(Line::from(Span::styled("PROBES", Style::default().fg(p.accent).add_modifier(Modifier::BOLD))));
                if net.probes.is_empty() {
                    lines.push(Line::from(Span::styled("  none detected", Style::default().fg(p.dim))));
                } else {
                    for probe in &net.probes {
                        lines.push(Line::from(vec![
                            Span::styled("  ◆ ", Style::default().fg(p.accent)),
                            Span::styled(probe.name.clone(), Style::default().fg(p.text)),
                            Span::styled(format!("  {}", rel(&probe.sector)), Style::default().fg(p.dim)),
                        ]));
                    }
                }
            } else {
                lines.push(Line::from(Span::styled("loading…", Style::default().fg(p.dim))));
            }
            frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), rows[0]);
            render_footer(frame, rows[1], p, &[FooterKey::nav("[Esc]", "close")]);
        }
    }
}
