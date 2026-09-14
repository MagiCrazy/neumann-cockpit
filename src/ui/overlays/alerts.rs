use crate::api::types::{AlertPhase, AlertType, ProbeAlert};
use crate::app::{ActiveWizard, AlertsInput, AppState};
use crate::ui::theme::Palette;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::{centered_rect, render_footer, FooterKey};

fn alert_type_label(t: &AlertType) -> &'static str {
    match t {
        AlertType::StorageContainerBreak => "container break",
        AlertType::IntelligentLife => "intelligent life",
        AlertType::SectorObjectDetected => "object detected",
        AlertType::AnomalyDetected => "anomaly detected",
        AlertType::MannyReport => "manny report",
        AlertType::MindSnapshotTransferred => "mind snapshot moved",
        AlertType::ProbeDestroyed => "probe lost",
        AlertType::AsteroidTrajectory => "asteroid trajectory",
        AlertType::BlueprintShared => "blueprint shared",
        AlertType::OthersPresence => "OTHERS PRESENT",
        AlertType::OthersWeapon => "OTHERS WEAPON",
        AlertType::OthersHarvestTraces => "harvest traces",
        AlertType::Unknown => "alert",
    }
}

/// Colour-code the row by alert type severity (dimmed once read).
fn type_color(t: &AlertType, p: Palette) -> Color {
    match t {
        AlertType::StorageContainerBreak => p.crit,
        AlertType::IntelligentLife => p.accent,
        AlertType::SectorObjectDetected => p.warn,
        AlertType::AnomalyDetected => p.crit,
        AlertType::MannyReport => p.good,
        AlertType::MindSnapshotTransferred => p.warn,
        AlertType::ProbeDestroyed => p.crit,
        AlertType::AsteroidTrajectory => p.warn,
        AlertType::BlueprintShared => p.good,
        // A neighbour sighted is a warning; a neighbour armed is not.
        AlertType::OthersPresence => p.warn,
        AlertType::OthersWeapon => p.crit,
        AlertType::OthersHarvestTraces => p.warn,
        AlertType::Unknown => p.text,
    }
}

/// `weapon_targeted` is the one phase that is a countdown rather than a report:
/// this probe, or a Manny it owns reachable through the same SCUT network, is
/// the *declared* target of a missile (API v119/v120). Phase 1 names it so it
/// stops reading like any other detection; the full treatment — a chip, a
/// countdown to `impactAt` — is #362.
fn targeted(alert: &ProbeAlert) -> bool {
    alert.phase == AlertPhase::WeaponTargeted
}

fn alert_row(alert: &ProbeAlert, p: Palette) -> ListItem<'static> {
    let unread = alert.is_unread();
    let (marker, marker_color) = if unread {
        ("● ", type_color(&alert.alert_type, p))
    } else {
        ("○ ", p.dim)
    };
    let text_style = if unread {
        Style::default().fg(p.text)
    } else {
        Style::default().fg(p.dim)
    };
    let label_color = if unread {
        type_color(&alert.alert_type, p)
    } else {
        p.dim
    };
    let mut spans = vec![
        Span::styled(marker, Style::default().fg(marker_color)),
        Span::styled(
            format!("{:<18}", alert_type_label(&alert.alert_type)),
            Style::default().fg(label_color).add_modifier(Modifier::BOLD),
        ),
    ];
    // Being aimed at outranks the alert's own type in the row: it is the part
    // the pilot must not scroll past. It stays crit once read, because
    // acknowledging a missile does not stop it.
    if targeted(alert) {
        spans.push(Span::styled(
            "⊗ TARGETED  ",
            Style::default().fg(p.crit).add_modifier(Modifier::BOLD),
        ));
    }
    spans.push(Span::styled(alert.message.clone(), text_style));
    ListItem::new(Line::from(spans))
}

pub(crate) fn render_alerts_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = state.palette();
    let ActiveWizard::Alerts(AlertsInput::Browsing {
        selection,
        show_warnings,
    }) = &state.active_wizard
    else {
        return;
    };
    let (selection, show_warnings) = (*selection, *show_warnings);

    let entries: &[ProbeAlert] = if show_warnings {
        &state.damage_warnings
    } else {
        &state.alerts
    };

    let height = (entries.len() as u16 + 6).clamp(8, 22);
    let popup = centered_rect(72, height, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" ALERTS ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // tab bar
            Constraint::Min(1),    // list
            Constraint::Length(1), // footer
        ])
        .split(inner);

    // ── Tab bar ──
    let alerts_unread = state.alerts.iter().filter(|a| a.is_unread()).count();
    let warns_unread = state.damage_warnings.iter().filter(|w| w.is_unread()).count();
    let tab_style = |active: bool| {
        if active {
            Style::default()
                .fg(Color::Black)
                .bg(p.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.dim)
        }
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!(" Alerts ({alerts_unread}) "), tab_style(!show_warnings)),
            Span::raw("  "),
            Span::styled(format!(" Warnings ({warns_unread}) "), tab_style(show_warnings)),
        ])),
        rows[0],
    );

    // ── List ──
    if entries.is_empty() {
        let label = if show_warnings {
            "no damage warnings"
        } else {
            "no active alerts"
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(label, Style::default().fg(p.dim)))),
            rows[1],
        );
    } else {
        let items: Vec<ListItem> = entries.iter().map(|a| alert_row(a, p)).collect();
        let list = List::new(items)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ ");
        let mut list_state = ListState::default();
        list_state.select(Some(selection.min(entries.len() - 1)));
        frame.render_stateful_widget(list, rows[1], &mut list_state);
    }

    // ── Footer ──
    render_footer(
        frame,
        rows[2],
        p,
        &[
            FooterKey::nav("[↑/↓]", "select"),
            FooterKey::nav("[Tab]", "switch"),
            FooterKey::commit("[Enter]", "ACK"),
            FooterKey::nav("[Esc]", "close"),
        ],
    );
}
