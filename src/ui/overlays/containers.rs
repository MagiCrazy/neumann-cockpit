use crate::app::{AppState, ContainerRulesInput, RenameContainerInput};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::centered_rect;

pub(crate) fn render_rename_container_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let RenameContainerInput::Typing { ref current_label, ref buf, ref error, .. } =
        state.rename_container
    else {
        return;
    };

    let popup = centered_rect(50, 7, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(format!(" RENAME — {current_label} "))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = vec![Line::from(vec![
        Span::styled("Label: ", Style::default().fg(Color::Cyan)),
        Span::raw(buf.as_str()),
        Span::styled("█", Style::default().fg(Color::Cyan)),
    ])];
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(format!("✗ {err}"), Style::default().fg(Color::Red))));
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

pub(crate) fn render_container_rules_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let ContainerRulesInput::Editing {
        ref container_label,
        ref types,
        ref priority,
        ref exclusion,
        ref strict_exclusion,
        selection,
        ref error,
        ..
    } = state.container_rules
    else {
        return;
    };

    let height = (types.len() as u16 + 8).clamp(10, 24);
    let popup = centered_rect(58, height, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(format!(" ROUTING RULES — {container_label} "))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("P", Style::default().fg(Color::Green)),
            Span::raw(" priority  "),
            Span::styled("E", Style::default().fg(Color::Yellow)),
            Span::raw(" exclude  "),
            Span::styled("S", Style::default().fg(Color::Red)),
            Span::raw(" strict  "),
            Span::styled("·", Style::default().fg(Color::DarkGray)),
            Span::raw(" none"),
        ])),
        rows[0],
    );

    let items: Vec<ListItem> = types
        .iter()
        .map(|ty| {
            let (tag, color) = if priority.iter().any(|t| t == ty) {
                ("[P]", Color::Green)
            } else if exclusion.iter().any(|t| t == ty) {
                ("[E]", Color::Yellow)
            } else if strict_exclusion.iter().any(|t| t == ty) {
                ("[S]", Color::Red)
            } else {
                ("[ ]", Color::DarkGray)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{tag} "), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Span::styled(ty.clone(), Style::default().fg(Color::White)),
            ]))
        })
        .collect();
    let list = List::new(items)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");
    let mut ls = ListState::default();
    if !types.is_empty() {
        ls.select(Some(selection.min(types.len() - 1)));
    }
    frame.render_stateful_widget(list, rows[1], &mut ls);

    let footer = if let Some(err) = error {
        Line::from(Span::styled(format!("✗ {err}"), Style::default().fg(Color::Red)))
    } else {
        Line::from(vec![
            Span::styled("[Space]", Style::default().fg(Color::Cyan)),
            Span::raw(" cycle  "),
            Span::styled("[Del]", Style::default().fg(Color::Cyan)),
            Span::raw(" clear  "),
            Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" save  "),
            Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ])
    };
    frame.render_widget(Paragraph::new(footer), rows[2]);
}
