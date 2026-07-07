use crate::ui::theme::palette;
use crate::app::{AppState, DeployInput, DetachInput, DropCargoInput, InspectInput, MindSnapshotInput, RecallInput, RecoverInput, RefuelInput, RenameMannyInput, SalvageInput, ScutRelayInput};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::{centered_rect, render_footer, render_pick_list, FooterKey};
pub(crate) fn render_salvage_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    match &state.salvage {
        SalvageInput::PickTarget { manny_name, candidates, selection, .. } => {
            let names: Vec<&str> = candidates.iter().map(|(_, n)| n.as_str()).collect();
            let height = (candidates.len() as u16 + 6).min(16);
            render_pick_list(frame, area, p, &format!(" SALVAGE — {manny_name} "), 50, height,
                Some("Select salvage target:"), &names, *selection, None, "confirm");
        }

        SalvageInput::Confirm { manny_name, object_name, error, .. } => {
            let popup = centered_rect(50, 8, area);
            frame.render_widget(Clear, popup);
            let block = Block::default()
                .title(format!(" SALVAGE — {manny_name} "))
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
                Span::styled("Target: ", Style::default().fg(p.dim)),
                Span::styled(object_name.as_str(), Style::default().fg(p.text).add_modifier(Modifier::BOLD)),
            ]));
            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    format!("✗ {err}"),
                    Style::default().fg(p.crit),
                )));
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            render_footer(frame, rows[1], p, &[
                FooterKey::commit("[Enter]", "SALVAGE"),
                FooterKey::nav("[Esc]", "cancel"),
            ]);
        }

        SalvageInput::Inactive => {}
    }
}

pub(crate) fn render_drop_cargo_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let DropCargoInput::Confirm { ref manny_name, ref error, .. } = state.drop_cargo else { return };

    let popup = centered_rect(54, 8, area);
    frame.render_widget(Clear, popup);

    let title = format!(" DROP CARGO — {manny_name} ");
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.crit));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "Drop cargo and retry docking?",
            Style::default().fg(p.text),
        )),
        Line::from(Span::styled(
            "Resource cargo is lost (objects return to sector).",
            Style::default().fg(p.dim),
        )),
    ];
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(format!("✗ {err}"), Style::default().fg(p.crit))));
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);
    render_footer(frame, rows[1], p, &[
        FooterKey::danger("[Enter/y]", "DROP"),
        FooterKey::nav("[Esc]", "cancel"),
    ]);
}

pub(crate) fn render_recall_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let RecallInput::Confirm { ref manny_name, remote, ref error, .. } = state.recall else { return };

    let popup = centered_rect(52, 8, area);
    frame.render_widget(Clear, popup);

    let title = if remote {
        format!(" ABANDON — {manny_name} ")
    } else {
        format!(" RECALL — {manny_name} ")
    };
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.warn));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        if remote { "Abandon this Manny's remote task?" } else { "Send recall order?" },
        Style::default().fg(p.text),
    )));
    if remote {
        lines.push(Line::from(Span::styled(
            "It will be left forgotten in its sector (no return).",
            Style::default().fg(p.dim),
        )));
    }
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(p.crit),
        )));
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);
    // Abandon (remote) is irreversible → Danger; a normal recall is a Commit.
    let recall_key = if remote {
        FooterKey::danger("[Enter]", "ABANDON")
    } else {
        FooterKey::commit("[Enter]", "RECALL")
    };
    render_footer(frame, rows[1], p, &[recall_key, FooterKey::nav("[Esc]", "cancel")]);
}

pub(crate) fn render_refuel_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let RefuelInput::Confirm { ref manny_name, ref error, .. } = state.refuel else { return };

    let popup = centered_rect(50, 7, area);
    frame.render_widget(Clear, popup);

    let title = format!(" REFUEL — {manny_name} ");
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.good));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        "Refill the probe deuterium tank?",
        Style::default().fg(p.text),
    )));
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(p.crit),
        )));
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);
    render_footer(frame, rows[1], p, &[
        FooterKey::commit("[Enter]", "REFUEL"),
        FooterKey::nav("[Esc]", "cancel"),
    ]);
}

pub(crate) fn render_scut_relay_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let ScutRelayInput::EnterNetworkName { ref manny_name, ref relay_name, ref buf, ref error, .. } =
        state.scut_relay
    else {
        return;
    };

    let popup = centered_rect(56, 9, area);
    frame.render_widget(Clear, popup);
    let title = format!(" TURN ON RELAY — {relay_name} ");
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

    let mut lines = vec![
        Line::from(Span::styled(
            format!("Manny: {manny_name}"),
            Style::default().fg(p.text),
        )),
        Line::default(),
        Line::from(vec![
            Span::styled("Network name (optional): ", Style::default().fg(p.text)),
            Span::styled(buf.clone(), Style::default().fg(p.accent)),
            Span::styled("▏", Style::default().fg(p.dim)),
        ]),
    ];
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(format!("✗ {err}"), Style::default().fg(p.crit))));
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);
    render_footer(frame, rows[1], p, &[
        FooterKey::commit("[Enter]", "TURN ON"),
        FooterKey::nav("[Esc]", "cancel"),
    ]);
}

pub(crate) fn render_mind_snapshot_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let MindSnapshotInput::Confirm { ref error } = state.mind_snapshot else { return };
    let alert = state.probe_terminal_alert();

    let popup = centered_rect(60, 10, area);
    frame.render_widget(Clear, popup);

    let title = alert
        .map(|a| format!(" {} ", a.title))
        .unwrap_or_else(|| " MIND SNAPSHOT REASSIGN ".to_string());
    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.crit).add_modifier(Modifier::BOLD));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::new();
    if let Some(a) = alert {
        lines.push(Line::from(Span::styled(
            a.message.clone(),
            Style::default().fg(p.text),
        )));
        lines.push(Line::default());
    }
    lines.push(Line::from(Span::styled(
        "Reassign your mind snapshot to a fresh probe?",
        Style::default().fg(p.text),
    )));
    lines.push(Line::from(Span::styled(
        "The terminal probe is deleted and the local reference frame resets to 0,0,0.",
        Style::default().fg(p.dim),
    )));
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(p.crit),
        )));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), rows[0]);
    render_footer(frame, rows[1], p, &[
        FooterKey::danger("[Enter]", "REASSIGN"),
        FooterKey::nav("[Esc]", "cancel"),
    ]);
}

pub(crate) fn render_rename_manny_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let RenameMannyInput::Typing { ref manny_name, ref buf, ref error, .. } = state.rename_manny else { return };

    let popup = centered_rect(46, 7, area);
    frame.render_widget(Clear, popup);

    let title = format!(" RENAME — {manny_name} ");
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
    lines.push(Line::from(vec![
        Span::styled("Name: ", Style::default().fg(p.accent)),
        Span::raw(buf.as_str()),
        Span::styled("█", Style::default().fg(p.accent)),
    ]));
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(p.crit),
        )));
    }

    frame.render_widget(Paragraph::new(lines), rows[0]);
    render_footer(frame, rows[1], p, &[
        FooterKey::commit("[Enter]", "RENAME"),
        FooterKey::nav("[Esc]", "cancel"),
    ]);
}

pub(crate) fn render_deploy_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    match &state.deploy {
        DeployInput::PickManny { mannies, selection } => {
            let names: Vec<&str> = mannies.iter().map(|(_, n)| n.as_str()).collect();
            let height = (mannies.len() as u16 + 6).min(18);
            render_pick_list(frame, area, p, " DEPLOY WAYPOINT — SELECT MANNY ", 52, height,
                None, &names, *selection, None, "confirm");
        }

        DeployInput::PickObject { candidates, selection, .. } => {
            let labels: Vec<String> = candidates.iter().enumerate()
                .map(|(i, (id, n))| super::probe_object_label(state, i, id, n)).collect();
            let names: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
            let height = (candidates.len() as u16 + 6).min(18);
            render_pick_list(frame, area, p, " DEPLOY WAYPOINT ", 52, height,
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
                Span::raw(name_buf.as_str()),
                Span::styled("█", Style::default().fg(p.accent)),
            ]));
            if let Some(err) = error {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    format!("✗ {err}"),
                    Style::default().fg(p.crit),
                )));
            }
            frame.render_widget(Paragraph::new(lines), rows[0]);
            render_footer(frame, rows[1], p, &[
                FooterKey::commit("[Enter]", "DEPLOY"),
                FooterKey::nav("[Esc]", "cancel"),
            ]);
        }

        DeployInput::Inactive => {}
    }
}

pub(crate) fn render_inspect_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let InspectInput::PickTarget { ref manny_name, ref candidates, selection, ref error, .. } = state.inspect else { return };
    let labels: Vec<String> = candidates.iter().enumerate()
        .map(|(i, (id, n))| super::probe_object_label(state, i, id, n)).collect();
    let names: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
    let error_lines = if error.is_some() { 2u16 } else { 0 };
    let height = (candidates.len() as u16 + 6 + error_lines).min(18);
    render_pick_list(frame, area, p, &format!(" INSPECT — {manny_name} "), 52, height,
        Some("Select object to inspect:"), &names, selection, error.as_deref(), "INSPECT");
}

pub(crate) fn render_recover_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let RecoverInput::PickContainer { ref manny_name, ref candidates, selection, ref error, .. } = state.recover else { return };
    let names: Vec<&str> = candidates.iter().map(|(_, n)| n.as_str()).collect();
    let error_lines = if error.is_some() { 2u16 } else { 0 };
    let height = (candidates.len() as u16 + 6 + error_lines).min(18);
    render_pick_list(frame, area, p, &format!(" RECOVER — {manny_name} "), 52, height,
        Some("Select container to recover:"), &names, selection, error.as_deref(), "RECOVER");
}

pub(crate) fn render_detach_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    match &state.detach {
        DetachInput::PickContainer { manny_name, containers, selection, .. } => {
            let names: Vec<&str> = containers.iter().map(|(_, n)| n.as_str()).collect();
            let height = (containers.len() as u16 + 6).min(16);
            render_pick_list(frame, area, p, &format!(" DETACH — {manny_name} "), 52, height,
                Some("Select container to detach:"), &names, *selection, None, "next");
        }

        DetachInput::PickMode { manny_name, container_name, selection, error, .. } => {
            let names: Vec<&str> = crate::app::DETACH_MODES.iter().map(|(_, l)| *l).collect();
            let prompt = format!("Detach mode  (manny: {manny_name})");
            render_pick_list(frame, area, p, &format!(" DETACH — {container_name} "), 52, 10,
                Some(&prompt), &names, *selection, error.as_deref(), "confirm");
        }

        DetachInput::PickAsteroid { manny_name, container_name, asteroids, selection, error, .. } => {
            let labels: Vec<String> = asteroids.iter().enumerate()
                .map(|(i, (id, n))| super::probe_object_label(state, i, id, n)).collect();
            let names: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
            let height = (asteroids.len() as u16 + 8).min(18);
            let prompt = format!("Attach to asteroid  (manny: {manny_name})");
            render_pick_list(frame, area, p, &format!(" DETACH — hide {container_name} "), 52, height,
                Some(&prompt), &names, *selection, error.as_deref(), "HIDE HERE");
        }

        DetachInput::Inactive => {}
    }
}

