use crate::app::{AppState, DeployInput, DetachInput, InspectInput, RecallInput, RecoverInput, RenameMannyInput, SalvageInput};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::{centered_rect, render_pick_list};
pub(crate) fn render_salvage_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    match &state.salvage {
        SalvageInput::PickTarget { manny_name, candidates, selection, .. } => {
            let names: Vec<&str> = candidates.iter().map(|(_, n)| n.as_str()).collect();
            let height = (candidates.len() as u16 + 6).min(16);
            render_pick_list(frame, area, &format!(" SALVAGE — {manny_name} "), 50, height,
                Some("Select salvage target:"), &names, *selection, None, "confirm");
        }

        SalvageInput::Confirm { manny_name, object_name, error, .. } => {
            let popup = centered_rect(50, 8, area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(format!(" SALVAGE — {manny_name} "))
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
            lines.push(Line::from(vec![
                Span::styled("Target: ", Style::default().fg(Color::DarkGray)),
                Span::styled(object_name.as_str(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]));
            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    format!("✗ {err}"),
                    Style::default().fg(Color::Red),
                )));
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw(" SALVAGE  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])),
                rows[1],
            );
        }

        SalvageInput::Inactive => {}
    }
}

pub(crate) fn render_recall_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let RecallInput::Confirm { ref manny_name, ref error, .. } = state.recall else { return };

    let popup = centered_rect(46, 7, area);
    frame.render_widget(Clear, popup);

    let title = format!(" RECALL — {manny_name} ");
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        "Send recall order?",
        Style::default().fg(Color::White),
    )));
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(Color::Red),
        )));
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[Enter]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" RECALL  "),
            Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ])),
        rows[1],
    );
}

pub(crate) fn render_rename_manny_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let RenameMannyInput::Typing { ref manny_name, ref buf, ref error, .. } = state.rename_manny else { return };

    let popup = centered_rect(46, 7, area);
    frame.render_widget(Clear, popup);

    let title = format!(" RENAME — {manny_name} ");
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
    lines.push(Line::from(vec![
        Span::styled("Name: ", Style::default().fg(Color::Cyan)),
        Span::raw(buf.as_str()),
        Span::styled("█", Style::default().fg(Color::Cyan)),
    ]));
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(Color::Red),
        )));
    }

    frame.render_widget(Paragraph::new(lines), rows[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" rename  "),
            Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ])),
        rows[1],
    );
}

pub(crate) fn render_deploy_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    match &state.deploy {
        DeployInput::PickManny { mannies, selection } => {
            let names: Vec<&str> = mannies.iter().map(|(_, n)| n.as_str()).collect();
            let height = (mannies.len() as u16 + 6).min(18);
            render_pick_list(frame, area, " DEPLOY WAYPOINT — SELECT MANNY ", 52, height,
                None, &names, *selection, None, "confirm");
        }

        DeployInput::PickObject { candidates, selection, .. } => {
            let names: Vec<&str> = candidates.iter().map(|(_, n)| n.as_str()).collect();
            let height = (candidates.len() as u16 + 6).min(18);
            render_pick_list(frame, area, " DEPLOY WAYPOINT ", 52, height,
                None, &names, *selection, None, "confirm");
        }

        DeployInput::EnterName { object_name, name_buf, error, .. } => {
            let popup = centered_rect(52, 8, area);
            frame.render_widget(Clear, popup);
            let title = format!(" DEPLOY WAYPOINT — {object_name} ");
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
            lines.push(Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Color::Cyan)),
                Span::raw(name_buf.as_str()),
                Span::styled("█", Style::default().fg(Color::Cyan)),
            ]));
            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    format!("✗ {err}"),
                    Style::default().fg(Color::Red),
                )));
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw(" DEPLOY  "),
                    Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
                    Span::raw(" cancel"),
                ])),
                rows[1],
            );
        }

        DeployInput::Inactive => {}
    }
}

pub(crate) fn render_inspect_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let InspectInput::PickAsteroid { ref manny_name, ref candidates, selection, ref error, .. } = state.inspect else { return };
    let names: Vec<&str> = candidates.iter().map(|(_, n)| n.as_str()).collect();
    let error_lines = if error.is_some() { 2u16 } else { 0 };
    let height = (candidates.len() as u16 + 6 + error_lines).min(18);
    render_pick_list(frame, area, &format!(" INSPECT — {manny_name} "), 52, height,
        Some("Select asteroid to inspect:"), &names, selection, error.as_deref(), "inspect");
}

pub(crate) fn render_recover_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let RecoverInput::PickContainer { ref manny_name, ref candidates, selection, ref error, .. } = state.recover else { return };
    let names: Vec<&str> = candidates.iter().map(|(_, n)| n.as_str()).collect();
    let error_lines = if error.is_some() { 2u16 } else { 0 };
    let height = (candidates.len() as u16 + 6 + error_lines).min(18);
    render_pick_list(frame, area, &format!(" RECOVER — {manny_name} "), 52, height,
        Some("Select container to recover:"), &names, selection, error.as_deref(), "recover");
}

pub(crate) fn render_detach_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    match &state.detach {
        DetachInput::PickContainer { manny_name, containers, selection, .. } => {
            let names: Vec<&str> = containers.iter().map(|(_, n)| n.as_str()).collect();
            let height = (containers.len() as u16 + 6).min(16);
            render_pick_list(frame, area, &format!(" DETACH — {manny_name} "), 52, height,
                Some("Select container to detach:"), &names, *selection, None, "next");
        }

        DetachInput::PickMode { manny_name, container_name, selection, error, .. } => {
            let names: Vec<&str> = crate::app::DETACH_MODES.iter().map(|(_, l)| *l).collect();
            let prompt = format!("Detach mode  (manny: {manny_name})");
            render_pick_list(frame, area, &format!(" DETACH — {container_name} "), 52, 10,
                Some(&prompt), &names, *selection, error.as_deref(), "confirm");
        }

        DetachInput::PickAsteroid { manny_name, container_name, asteroids, selection, error, .. } => {
            let names: Vec<&str> = asteroids.iter().map(|(_, n)| n.as_str()).collect();
            let height = (asteroids.len() as u16 + 8).min(18);
            let prompt = format!("Attach to asteroid  (manny: {manny_name})");
            render_pick_list(frame, area, &format!(" DETACH — hide {container_name} "), 52, height,
                Some(&prompt), &names, *selection, error.as_deref(), "hide here");
        }

        DetachInput::Inactive => {}
    }
}

