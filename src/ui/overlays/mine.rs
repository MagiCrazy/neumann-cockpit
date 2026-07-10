use crate::app::{ActiveWizard, AppState, MineInput, RESOURCE_LABELS, RESOURCE_TYPES};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui::theme::{format_duration, palette};
use super::{centered_rect, render_footer, render_pick_list, FooterKey, KeyTone};
/// A mining-target picker row: `#n [name][ danger]  metals 1.20  ice 0.55 …`.
/// Thin wrapper over the shared [`super::probe_object_label`] so every
/// asteroid/object picker renders identically.
fn asteroid_label(state: &AppState, index: usize, id: &str, name: &str) -> String {
    super::probe_object_label(state, index, id, name)
}

/// A rough pre-launch estimate of a mining job: round-trip count and a ballpark
/// duration. Indicative only — the authoritative ETA comes from the server as
/// `task_estimated_end_time` once the task starts (rendered in the Mannies pane
/// via `manny_task_eta`). When `travel_deducted` is true the destination is a
/// container hidden on the very asteroid being mined, so the server charges no
/// per-trip travel (miningTravelSeconds = 0); otherwise a fixed travel estimate
/// applies per trip (the real distance isn't known client-side). The per-trip
/// cargo capacity is fixed at 0.05 ECE (per the API).
pub(crate) fn estimate_mine_duration(target_amount: f64, travel_deducted: bool) -> (i64, i64) {
    const CARGO_CAP: f64 = 0.05; // Manny cargo capacity per trip (ECE)
    const TRAVEL_SECS: i64 = 1800; // 900s each way
    const TICK_AMOUNT: f64 = 0.01;
    const TICK_SECS: i64 = 300;
    let trips = (target_amount / CARGO_CAP).ceil() as i64;
    let mut remaining = target_amount;
    let mut total_secs: i64 = 0;
    for _ in 0..trips {
        let trip = remaining.min(CARGO_CAP);
        let ticks = (trip / TICK_AMOUNT).ceil() as i64;
        total_secs += ticks * TICK_SECS;
        if !travel_deducted {
            total_secs += TRAVEL_SECS;
        }
        remaining -= trip;
    }
    (trips, total_secs)
}

pub(crate) fn render_mine_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let ActiveWizard::Mine(mine) = &state.active_wizard else { return };
    match mine {
        MineInput::PickAsteroid { manny_name, candidates, selection, .. } => {
            // Asteroids are usually unnamed, so label each by index + its known
            // resource content (like the Sector pane) instead of a bare name.
            let labels: Vec<String> = candidates
                .iter()
                .enumerate()
                .map(|(i, (id, name))| asteroid_label(state, i, id, name))
                .collect();
            let names: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
            let height = (candidates.len() as u16 + 6).min(16);
            render_pick_list(frame, area, palette(state.color_mode), &format!(" MINE — {manny_name} "), 62, height,
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
                .border_style(Style::default().fg(p.accent));
            let inner = block.inner(popup);
            frame.render_widget(block, popup);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);

            let mut lines: Vec<Line> = Vec::new();

            // Resources section
            let res_header_color = if *amount_mode { p.dim } else { p.accent };
            lines.push(Line::from(vec![
                Span::styled("Resources", Style::default().fg(res_header_color)),
                Span::styled(
                    if *amount_mode { "  (Tab to edit)" } else { "  [1-4 to toggle]" },
                    Style::default().fg(p.dim),
                ),
            ]));
            for (i, (&label, &_type_str)) in RESOURCE_LABELS.iter().zip(RESOURCE_TYPES.iter()).enumerate() {
                let present = reserves.map(|(f, _)| f[i]).unwrap_or(true);
                let reserve = reserves.map(|(_, r)| r[i]);
                let (checkbox, checked_color, label_color) = if !present {
                    ("[·]", p.dim, p.dim)
                } else if resources[i] {
                    ("[✓]", p.good, p.text)
                } else {
                    ("[ ]", p.dim, p.text)
                };
                let mut spans = vec![
                    Span::styled(format!("  {checkbox} "), Style::default().fg(checked_color)),
                    Span::styled(format!("{} ", i + 1), Style::default().fg(p.dim)),
                    Span::styled(format!("{label:<9}"), Style::default().fg(label_color)),
                ];
                // Remaining reserve (ECE) when the target exposes it.
                match (present, reserve) {
                    (true, Some(r)) if r > 0.0 => spans.push(Span::styled(
                        format!(" {r:.2}"),
                        Style::default().fg(p.dim),
                    )),
                    (false, _) => spans.push(Span::styled(" —", Style::default().fg(p.dim))),
                    _ => {}
                }
                lines.push(Line::from(spans));
            }

            lines.push(Line::default());

            // Amount section
            let amt_header_color = if *amount_mode { p.accent } else { p.dim };
            if *amount_mode {
                lines.push(Line::from(vec![
                    Span::styled("Amount: ", Style::default().fg(amt_header_color)),
                    Span::raw(amount_buf.as_str()),
                    Span::styled("█", Style::default().fg(p.accent)),
                    Span::styled(" ECE", Style::default().fg(p.dim)),
                    Span::raw("  "),
                    Span::styled("[M]", Style::default().fg(p.warn)),
                    Span::styled(" max", Style::default().fg(p.dim)),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("Amount: ", Style::default().fg(amt_header_color)),
                    Span::styled(amount_buf.as_str(), Style::default().fg(p.dim)),
                    Span::styled(" ECE", Style::default().fg(p.dim)),
                    Span::styled("  [Tab to edit]", Style::default().fg(p.dim)),
                ]));
            }

            // Time estimate. Travel is deducted when mining into a container
            // hidden on this very asteroid (no round trips); mining to the probe
            // or a container elsewhere keeps travel.
            if let Ok(amount) = amount_buf.parse::<f64>() {
                if amount > 0.0 {
                    let travel_deducted = target_container
                        .as_ref()
                        .is_some_and(|(cid, _)| state.mining_travel_deducted(object_id, cid));
                    let (trips, secs) = estimate_mine_duration(amount, travel_deducted);
                    let dest = if travel_deducted { "on-site" } else { "+ travel" };
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!(
                                "≈ {trips} trip{} · ~{} ({dest})",
                                if trips == 1 { "" } else { "s" },
                                format_duration(secs),
                            ),
                            Style::default().fg(p.dim),
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
                    Span::styled("Store in: ", Style::default().fg(p.dim)),
                    Span::styled(target_label, Style::default().fg(p.accent)),
                    Span::styled("  [c] cycle", Style::default().fg(p.dim)),
                ]));
            }

            // Error
            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    format!("✗ {err}"),
                    Style::default().fg(p.crit),
                )));
            }

            frame.render_widget(Paragraph::new(lines), rows[0]);

            // Hint bar
            let any_resource = resources.iter().any(|&r| r);
            let valid_amount = amount_buf.parse::<f64>().ok().filter(|&v| v > 0.0).is_some();
            let can_send = any_resource && valid_amount;
            let mine_key = if can_send {
                FooterKey::commit("[Enter]", "MINE")
            } else {
                FooterKey { key: "[Enter]", label: "MINE", tone: KeyTone::Disabled }
            };
            let keys = if *amount_mode {
                vec![FooterKey::nav("[Tab]", "resources"), mine_key, FooterKey::nav("[Esc]", "cancel")]
            } else {
                vec![
                    FooterKey::nav("[1-4]", "toggle"),
                    FooterKey::nav("[Tab]", "amount"),
                    mine_key,
                    FooterKey::nav("[Esc]", "cancel"),
                ]
            };
            render_footer(frame, rows[1], p, &keys);
        }

    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;

    fn state_with_targets() -> AppState {
        let mut state = AppState::default();
        state.probe = Some(serde_json::from_str(r#"{
            "id":1,"name":"t","status":"idle","fuel":{"deuterium":null},"sensorMode":"normal",
            "sector":{"relative":{"x":2,"y":0,"z":-2}},"movement":null,"systems":null,
            "inventory":{"capacity":10.0,"usedCapacity":1.0,"freeCapacity":9.0,"resourceStocks":[],"externalTanks":[],"containers":[],"items":[]}
        }"#).unwrap());
        state.scan_history = vec![serde_json::from_str(r#"{
            "relativeCoordinates":{"x":2,"y":0,"z":-2},"distance":0,"knowledgeLevel":"detailed","confidence":1.0,
            "objects":[{"id":"planet-1","type":"planet","name":"P1","minableTargets":[
                {"id":"ast-1","type":"asteroid","name":null,"resourceTypes":["deuterium","metals"],"resourceAmounts":{"deuterium":0.42,"metals":1.20,"ice":0.0,"carbonCompounds":0.0}},
                {"id":"ast-3","type":"asteroid","name":"Big Rock","resourceTypes":["metals"],"resourceAmounts":null}
            ]}],
            "scan":{"currentSectorResidenceSeconds":60,"requiredResidenceSeconds":60,"scanQuality":1.0}
        }"#).unwrap()];
        state
    }

    #[test]
    fn unnamed_asteroid_labelled_by_index_and_reserves() {
        let state = state_with_targets();
        let label = asteroid_label(&state, 0, "ast-1", "unnamed");
        assert_eq!(label, "#1  deuterium 0.42  metals 1.20", "index + named per-resource reserves, no 'unnamed'");
    }

    #[test]
    fn named_asteroid_keeps_its_name_and_lists_present_resources() {
        let state = state_with_targets();
        // No reserve amounts known → just the resource label.
        let label = asteroid_label(&state, 2, "ast-3", "Big Rock");
        assert_eq!(label, "#3 Big Rock  metals");
    }
}
