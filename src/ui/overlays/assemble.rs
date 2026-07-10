use crate::app::{ActiveWizard, AppState, AssembleProbeInput};
use crate::ui::theme::palette;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::{centered_rect, render_footer, FooterKey};

/// The fixed component bill for one drone assembly (API v81), on top of the two
/// empty containers the pilot picks. Server-enforced; shown so the cost is
/// visible before committing the 3-hour task.
const ASSEMBLY_BILL: &[&str] = &[
    "1× deuterium engine",
    "1× SCUT relay",
    "5× electric motor",
    "2× atomic printer part",
    "4× solar panel",
];

pub(crate) fn render_assemble_probe_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let ActiveWizard::AssembleProbe(AssembleProbeInput::PickContainers { manny_name, containers, selected, cursor, error, .. }) =
        &state.active_wizard
    else {
        return;
    };
    let p = palette(state.color_mode);
    let height = (containers.len() as u16 + ASSEMBLY_BILL.len() as u16 + 9).clamp(14, 24);
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
    for item in ASSEMBLY_BILL {
        lines.push(Line::from(Span::styled(format!("  · {item}"), Style::default().fg(p.dim))));
    }
    lines.push(Line::from(Span::styled(
        "  → new drone in this sector (~3h task)",
        Style::default().fg(p.dim),
    )));
    if let Some(err) = error {
        lines.push(Line::from(Span::styled(format!("✗ {err}"), Style::default().fg(p.crit))));
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);
    render_footer(frame, rows[1], p, &[
        FooterKey::nav("[Space]", "select"),
        FooterKey::commit("[Enter]", "ASSEMBLE"),
        FooterKey::nav("[Esc]", "cancel"),
    ]);
}
