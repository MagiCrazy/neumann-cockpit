use crate::app::{AppState, ContainerRulesInput, ContainersInput, RenameContainerInput};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::centered_rect;
use crate::ui::theme::gauge_color;

/// Fixed-width textual capacity bar.
fn bar(ratio: f64, width: usize) -> String {
    let filled = (ratio.clamp(0.0, 1.0) * width as f64).round() as usize;
    (0..width).map(|i| if i < filled { '█' } else { '░' }).collect()
}

pub(crate) fn render_containers_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let ContainersInput::Browsing { selection } = state.containers_input else {
        return;
    };
    let containers = &state.storage_containers;

    let height = (containers.len() as u16 + 6).clamp(8, 22);
    let popup = centered_rect(70, height, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" STORAGE CONTAINERS ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    if containers.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "no storage containers — loading…",
                Style::default().fg(Color::DarkGray),
            ))),
            rows[0],
        );
    } else {
        let items: Vec<ListItem> = containers
            .iter()
            .map(|c| {
                let ratio = if c.capacity > 0.0 { c.used_capacity / c.capacity } else { 0.0 };
                let core = if c.kind == "probe" { " ⌂" } else { "" };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{:<20}", c.label), Style::default().fg(Color::White)),
                    Span::styled(core, Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::styled(bar(ratio, 12), Style::default().fg(gauge_color(1.0 - ratio))),
                    Span::styled(
                        format!(" {:.2}/{:.2}", c.used_capacity, c.capacity),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
            })
            .collect();
        let list = List::new(items)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ ");
        let mut ls = ListState::default();
        ls.select(Some(selection.min(containers.len() - 1)));
        frame.render_stateful_widget(list, rows[0], &mut ls);
    }

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[↑/↓]", Style::default().fg(Color::Cyan)),
            Span::raw(" select  "),
            Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" content  "),
            Span::styled("[n]", Style::default().fg(Color::Cyan)),
            Span::raw(" rename  "),
            Span::styled("[e]", Style::default().fg(Color::Cyan)),
            Span::raw(" rules  "),
            Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" close"),
        ])),
        rows[1],
    );
}

pub(crate) fn render_container_detail_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some((container, inv)) = &state.storage_container_detail else {
        return;
    };

    let line_count = inv.resource_stocks.len() + inv.items.len() + 4;
    let height = (line_count as u16 + 4).clamp(8, 24);
    let popup = centered_rect(60, height, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(format!(" {} ", container.label))
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
    let ratio = if container.capacity > 0.0 { container.used_capacity / container.capacity } else { 0.0 };
    lines.push(Line::from(vec![
        Span::styled("capacity ", Style::default().fg(Color::Cyan)),
        Span::styled(bar(ratio, 16), Style::default().fg(gauge_color(1.0 - ratio))),
        Span::styled(
            format!(" {:.2}/{:.2}", container.used_capacity, container.capacity),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    lines.push(Line::default());

    lines.push(Line::from(Span::styled("RESOURCES", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
    if inv.resource_stocks.is_empty() {
        lines.push(Line::from(Span::styled("  —", Style::default().fg(Color::DarkGray))));
    } else {
        for st in &inv.resource_stocks {
            lines.push(Line::from(vec![
                Span::raw(format!("  {:<16}", st.name)),
                Span::styled(format!("{:.2}", st.amount), Style::default().fg(Color::White)),
            ]));
        }
    }
    lines.push(Line::default());

    lines.push(Line::from(Span::styled("ITEMS", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
    if inv.items.is_empty() {
        lines.push(Line::from(Span::styled("  —", Style::default().fg(Color::DarkGray))));
    } else {
        for it in &inv.items {
            lines.push(Line::from(Span::raw(format!("  {}", it.name))));
        }
    }

    frame.render_widget(Paragraph::new(lines), rows[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[Esc/Enter]", Style::default().fg(Color::Cyan)),
            Span::raw(" close"),
        ])),
        rows[1],
    );
}

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
