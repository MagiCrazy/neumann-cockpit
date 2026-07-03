use crate::ui::theme::{palette, Palette};
use crate::api::types::{AlertType, ProbeAlert};
use crate::app::{AlertsInput, AppState};
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
        AlertType::Unknown => p.text,
    }
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
    let label_color = if unread { type_color(&alert.alert_type, p) } else { p.dim };
    ListItem::new(Line::from(vec![
        Span::styled(marker, Style::default().fg(marker_color)),
        Span::styled(
            format!("{:<18}", alert_type_label(&alert.alert_type)),
            Style::default().fg(label_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(alert.message.clone(), text_style),
    ]))
}

pub(crate) fn render_alerts_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let p = palette(state.color_mode);
    let AlertsInput::Browsing { selection, show_warnings } = state.alerts_input else {
        return;
    };

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
            Style::default().fg(Color::Black).bg(p.accent).add_modifier(Modifier::BOLD)
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
        let label = if show_warnings { "no damage warnings" } else { "no active alerts" };
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
    render_footer(frame, rows[2], p, &[
        FooterKey::nav("[↑/↓]", "select"),
        FooterKey::nav("[Tab]", "switch"),
        FooterKey::commit("[Enter]", "ACK"),
        FooterKey::nav("[Esc]", "close"),
    ]);
}
