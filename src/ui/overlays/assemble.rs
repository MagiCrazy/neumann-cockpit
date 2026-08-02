use crate::api::types::ProbeModel;
use crate::app::{
    assembly_bill, model_blurb, model_label, ActiveWizard, AppState, AssembleProbeInput, ASSEMBLABLE_MODELS,
};
use crate::ui::theme::palette;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::{centered_rect, render_footer, FooterKey};

pub(crate) fn render_assemble_probe_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    match &state.active_wizard {
        ActiveWizard::AssembleProbe(AssembleProbeInput::PickModel { .. }) => render_model_step(frame, area, state),
        ActiveWizard::AssembleProbe(AssembleProbeInput::PickContainers { .. }) => {
            render_container_step(frame, area, state)
        }
        _ => {}
    }
}

/// Step 1 — hull model, with the bill each one costs (API v104).
fn render_model_step(frame: &mut Frame, area: Rect, state: &AppState) {
    let ActiveWizard::AssembleProbe(AssembleProbeInput::PickModel { manny_name, cursor, .. }) = &state.active_wizard
    else {
        return;
    };
    let p = palette(state.color_mode);
    let selected_model = ASSEMBLABLE_MODELS[(*cursor).min(ASSEMBLABLE_MODELS.len() - 1)];
    let bill = assembly_bill(selected_model);
    let height = (bill.len() as u16 + ASSEMBLABLE_MODELS.len() as u16 * 2 + 8).clamp(14, 26);
    let popup = centered_rect(58, height, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" ASSEMBLE DRONE PROBE · MODEL ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Builder  ", Style::default().fg(p.dim)),
            Span::styled(manny_name.clone(), Style::default().fg(p.text)),
        ]),
        Line::from(Span::styled("Hull model", Style::default().fg(p.accent))),
    ];
    for (i, model) in ASSEMBLABLE_MODELS.iter().enumerate() {
        let style = if i == *cursor {
            Style::default().fg(p.accent).add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(p.text)
        };
        lines.push(Line::from(Span::styled(
            format!(" {} {}", if i == *cursor { "▸" } else { " " }, model_label(*model)),
            style,
        )));
        lines.push(Line::from(Span::styled(
            format!("    {}", model_blurb(*model)),
            Style::default().fg(p.dim),
        )));
    }
    lines.push(Line::default());
    lines.push(Line::from(Span::styled("Consumes", Style::default().fg(p.dim))));
    lines.push(Line::from(Span::styled(
        "  · 2× empty additional container".to_string(),
        Style::default().fg(p.dim),
    )));
    for (item, qty) in &bill {
        lines.push(Line::from(Span::styled(
            format!("  · {qty:.0}× {}", item.replace('_', " ")),
            Style::default().fg(p.dim),
        )));
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);
    render_footer(
        frame,
        rows[1],
        p,
        &[
            FooterKey::nav("[↑↓]", "model"),
            FooterKey::commit("[Enter]", "containers"),
            FooterKey::nav("[Esc]", "cancel"),
        ],
    );
}

/// Step 2 — the two empty containers, with the chosen model's bill in view.
fn render_container_step(frame: &mut Frame, area: Rect, state: &AppState) {
    let ActiveWizard::AssembleProbe(AssembleProbeInput::PickContainers {
        manny_name,
        model,
        containers,
        selected,
        cursor,
        error,
        ..
    }) = &state.active_wizard
    else {
        return;
    };
    let p = palette(state.color_mode);
    let bill = assembly_bill(*model);
    let height = (containers.len() as u16 + bill.len() as u16 + 10).clamp(14, 26);
    let popup = centered_rect(58, height, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" ASSEMBLE DRONE PROBE ")
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
        Span::styled("Builder  ", Style::default().fg(p.dim)),
        Span::styled(manny_name.clone(), Style::default().fg(p.text)),
        Span::styled("   Model  ", Style::default().fg(p.dim)),
        Span::styled(model_label(*model), Style::default().fg(p.accent)),
    ]));
    lines.push(Line::from(Span::styled(
        format!("Select two empty containers  ({}/2)", selected.len()),
        Style::default().fg(p.accent),
    )));
    for (i, (_, label)) in containers.iter().enumerate() {
        let checked = selected.contains(&i);
        let mark = if checked { "[x]" } else { "[ ]" };
        let style = if i == *cursor {
            Style::default().fg(p.accent).add_modifier(Modifier::REVERSED)
        } else if checked {
            Style::default().fg(p.text)
        } else {
            Style::default().fg(p.dim)
        };
        lines.push(Line::from(Span::styled(format!(" {mark} {label}"), style)));
    }
    lines.push(Line::default());
    lines.push(Line::from(Span::styled("Also consumes", Style::default().fg(p.dim))));
    for (item, qty) in &bill {
        lines.push(Line::from(Span::styled(
            format!("  · {qty:.0}× {}", item.replace('_', " ")),
            Style::default().fg(p.dim),
        )));
    }
    let tank = if *model == ProbeModel::DeuteriumTanker {
        400
    } else {
        100
    };
    lines.push(Line::from(Span::styled(
        format!("  → new drone in this sector, {tank}-point tank (~3h task)"),
        Style::default().fg(p.dim),
    )));
    if let Some(err) = error {
        lines.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(p.crit),
        )));
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);
    render_footer(
        frame,
        rows[1],
        p,
        &[
            FooterKey::nav("[Space]", "select"),
            FooterKey::commit("[Enter]", "ASSEMBLE"),
            FooterKey::nav("[Esc]", "back"),
        ],
    );
}
