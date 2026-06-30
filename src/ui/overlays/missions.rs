use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::api::types::{Mission, MissionStatus, MissionStepStatus};
use crate::app::{AppState, MissionsInput};

use super::centered_rect;

fn mission_status_style(s: &MissionStatus) -> (&'static str, Style) {
    match s {
        MissionStatus::Active => ("active", Style::default().fg(Color::Cyan)),
        MissionStatus::Completed => ("done", Style::default().fg(Color::Green)),
        MissionStatus::Failed => ("failed", Style::default().fg(Color::Red)),
        MissionStatus::Abandoned => ("abandoned", Style::default().fg(Color::DarkGray)),
        MissionStatus::Unknown => ("?", Style::default().fg(Color::DarkGray)),
    }
}

fn step_mark(s: &MissionStepStatus) -> (&'static str, Color) {
    match s {
        MissionStepStatus::Pending => ("○", Color::Yellow),
        MissionStepStatus::Completed => ("✔", Color::Green),
        MissionStepStatus::Failed => ("✗", Color::Red),
        MissionStepStatus::Skipped => ("–", Color::DarkGray),
        MissionStepStatus::Unknown => ("?", Color::DarkGray),
    }
}

fn mission_lines(m: &Mission, selected: bool) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let (status_text, status_style) = mission_status_style(&m.status);
    let marker = if selected { "▸ " } else { "  " };
    let title_style = if selected {
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::BOLD)
    };
    lines.push(Line::from(vec![
        Span::styled(marker, Style::default().fg(Color::Cyan)),
        Span::styled(m.title.clone(), title_style),
        Span::raw("  "),
        Span::styled(format!("[{status_text}]"), status_style),
    ]));
    if let Some(desc) = &m.description {
        lines.push(Line::from(Span::styled(
            format!("    {desc}"),
            Style::default().fg(Color::Gray),
        )));
    }
    let mut steps: Vec<&_> = m.steps.iter().collect();
    steps.sort_by_key(|s| s.sort_order);
    for step in steps {
        let (mark, color) = step_mark(&step.status);
        lines.push(Line::from(vec![
            Span::styled(format!("    {mark} "), Style::default().fg(color)),
            Span::styled(step.title.clone(), Style::default().fg(Color::White)),
        ]));
    }
    lines
}

pub(crate) fn render_missions_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let selection = match state.missions_input {
        MissionsInput::Browsing { selection } => selection,
        MissionsInput::ConfirmAbandon { selection, .. } => selection,
        MissionsInput::Inactive => return,
    };

    let popup = centered_rect(78, 80, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" MISSIONS ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::new();
    if state.missions.is_empty() {
        lines.push(Line::from(Span::styled(
            "No active missions.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (i, m) in state.missions.iter().enumerate() {
            lines.extend(mission_lines(m, i == selection));
            lines.push(Line::default());
        }
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), rows[0]);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[↑↓]", Style::default().fg(Color::Cyan)),
            Span::raw(" select  "),
            Span::styled("[a]", Style::default().fg(Color::Yellow)),
            Span::raw(" abandon  "),
            Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" close"),
        ])),
        rows[1],
    );

    if let MissionsInput::ConfirmAbandon { ref mission_title, ref error, .. } = state.missions_input {
        render_abandon_confirm(frame, area, mission_title, error.as_deref());
    }
}

fn render_abandon_confirm(frame: &mut Frame, area: Rect, title: &str, error: Option<&str>) {
    let popup = centered_rect(50, 8, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" ABANDON MISSION ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines = vec![Line::from(Span::styled(
        format!("Abandon \"{title}\"?"),
        Style::default().fg(Color::White),
    ))];
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(Color::Red),
        )));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), rows[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[Enter]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" abandon  "),
            Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" keep"),
        ])),
        rows[1],
    );
}
