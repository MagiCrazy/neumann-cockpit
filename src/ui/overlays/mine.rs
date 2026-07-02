use crate::app::{AppState, MineInput, RESOURCE_LABELS, RESOURCE_TYPES};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui::theme::format_duration;
use super::{centered_rect, render_pick_list};
pub(crate) fn estimate_mine_duration(target_amount: f64) -> (i64, i64) {
    const CARGO_CAP: f64 = 0.30;
    const TRAVEL_SECS: i64 = 1800; // 900s each way
    const TICK_AMOUNT: f64 = 0.01;
    const TICK_SECS: i64 = 300;
    let trips = (target_amount / CARGO_CAP).ceil() as i64;
    let mut remaining = target_amount;
    let mut total_secs: i64 = 0;
    for _ in 0..trips {
        let trip = remaining.min(CARGO_CAP);
        let ticks = (trip / TICK_AMOUNT).ceil() as i64;
        total_secs += TRAVEL_SECS + ticks * TICK_SECS;
        remaining -= trip;
    }
    (trips, total_secs)
}

pub(crate) fn render_mine_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    match &state.mine {
        MineInput::PickAsteroid { manny_name, candidates, selection, .. } => {
            let names: Vec<&str> = candidates.iter().map(|(_, n)| n.as_str()).collect();
            let height = (candidates.len() as u16 + 6).min(16);
            render_pick_list(frame, area, &format!(" MINE — {manny_name} "), 50, height,
                Some("Select mining target:"), &names, *selection, None, "confirm");
        }

        MineInput::Configure { manny_name, object_name, object_id, resources, amount_buf, amount_mode, target_container, error, .. } => {
            let reserves = state.minable_target_reserves(object_id);
            let popup = centered_rect(52, 15, area);
            frame.render_widget(Clear, popup);

            let manny_short = if manny_name.len() > 10 { &manny_name[..10] } else { manny_name };
            let obj_short = if object_name.len() > 12 { &object_name[..12] } else { object_name };
            let title = format!(" MINE — {manny_short} → {obj_short} ");
            let block = Block::default()
                .title(title)
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let mut lines: Vec<Line> = Vec::new();

            // Resources section
            let res_header_color = if *amount_mode { Color::DarkGray } else { Color::Cyan };
            lines.push(Line::from(vec![
                Span::styled("Resources", Style::default().fg(res_header_color)),
                Span::styled(
                    if *amount_mode { "  (Tab to edit)" } else { "  [1-4 to toggle]" },
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
            for (i, (&label, &_type_str)) in RESOURCE_LABELS.iter().zip(RESOURCE_TYPES.iter()).enumerate() {
                let present = reserves.map(|(f, _)| f[i]).unwrap_or(true);
                let reserve = reserves.map(|(_, r)| r[i]);
                let (checkbox, checked_color, label_color) = if !present {
                    ("[·]", Color::DarkGray, Color::DarkGray)
                } else if resources[i] {
                    ("[✓]", Color::Green, Color::White)
                } else {
                    ("[ ]", Color::DarkGray, Color::Gray)
                };
                let mut spans = vec![
                    Span::styled(format!("  {checkbox} "), Style::default().fg(checked_color)),
                    Span::styled(format!("{} ", i + 1), Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{label:<9}"), Style::default().fg(label_color)),
                ];
                // Remaining reserve (ECE) when the target exposes it.
                match (present, reserve) {
                    (true, Some(r)) if r > 0.0 => spans.push(Span::styled(
                        format!(" {r:.2}"),
                        Style::default().fg(Color::DarkGray),
                    )),
                    (false, _) => spans.push(Span::styled(" —", Style::default().fg(Color::DarkGray))),
                    _ => {}
                }
                lines.push(Line::from(spans));
            }

            lines.push(Line::default());

            // Amount section
            let amt_header_color = if *amount_mode { Color::Cyan } else { Color::DarkGray };
            if *amount_mode {
                lines.push(Line::from(vec![
                    Span::styled("Amount: ", Style::default().fg(amt_header_color)),
                    Span::raw(amount_buf.as_str()),
                    Span::styled("█", Style::default().fg(Color::Cyan)),
                    Span::styled(" ECE", Style::default().fg(Color::DarkGray)),
                    Span::raw("  "),
                    Span::styled("[M]", Style::default().fg(Color::Yellow)),
                    Span::styled(" max", Style::default().fg(Color::DarkGray)),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("Amount: ", Style::default().fg(amt_header_color)),
                    Span::styled(amount_buf.as_str(), Style::default().fg(Color::DarkGray)),
                    Span::styled(" ECE", Style::default().fg(Color::DarkGray)),
                    Span::styled("  [Tab to edit]", Style::default().fg(Color::DarkGray)),
                ]));
            }

            // Time estimate
            if let Ok(amount) = amount_buf.parse::<f64>() {
                if amount > 0.0 {
                    let (trips, secs) = estimate_mine_duration(amount);
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("{trips} trip{}  •  ~{}", if trips == 1 { "" } else { "s" }, format_duration(secs)),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                } else {
                    lines.push(Line::default());
                }
            } else {
                lines.push(Line::default());
            }

            // Optional target container (only when the sector has detached ones)
            let has_containers = !state.collect_detached_containers().is_empty();
            if has_containers {
                let target_label = match target_container {
                    Some((_, name)) => name.as_str(),
                    None => "probe (none)",
                };
                lines.push(Line::from(vec![
                    Span::styled("Store in: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(target_label, Style::default().fg(Color::Cyan)),
                    Span::styled("  [c] cycle", Style::default().fg(Color::DarkGray)),
                ]));
            }

            // Error
            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    format!("✗ {err}"),
                    Style::default().fg(Color::Red),
                )));
            }

            frame.render_widget(Paragraph::new(lines), rows[0]);

            // Hint bar
            let any_resource = resources.iter().any(|&r| r);
            let valid_amount = amount_buf.parse::<f64>().ok().filter(|&v| v > 0.0).is_some();
            let can_send = any_resource && valid_amount;
            let hint = if *amount_mode {
                Line::from(vec![
                    Span::styled("[Tab]", Style::default().fg(Color::Cyan)),
                    Span::raw(" resources  "),
                    Span::styled("[Enter]", if can_send { Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) }),
                    Span::raw(" send  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])
            } else {
                Line::from(vec![
                    Span::styled("[1-4]", Style::default().fg(Color::Cyan)),
                    Span::raw(" toggle  "),
                    Span::styled("[Tab]", Style::default().fg(Color::Cyan)),
                    Span::raw(" amount  "),
                    Span::styled("[Enter]", if can_send { Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) }),
                    Span::raw(" send  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])
            };
            frame.render_widget(Paragraph::new(hint), rows[1]);
        }

        MineInput::Inactive => {}
    }
}

