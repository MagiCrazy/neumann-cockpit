use crate::app::{AppState, StorageMoveInput, MOVE_RESOURCE_TYPES};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::{centered_rect, render_pick_list};

fn framed(title: &str, area: Rect, width: u16, height: u16, frame: &mut Frame) -> [Rect; 2] {
    let popup = centered_rect(width, height, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(title.to_owned())
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    [rows[0], rows[1]]
}

/// One `label: < value >` form row, highlighted when active.
fn field_row(label: &str, value: String, active: bool, editing: bool) -> Line<'static> {
    let lab_style = if active {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let val = if editing {
        format!("{value}█")
    } else if active {
        format!("‹ {value} ›")
    } else {
        format!("  {value}  ")
    };
    let val_style = if active {
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    Line::from(vec![
        Span::styled(format!("{label:<10}"), lab_style),
        Span::styled(val, val_style),
    ])
}

pub(crate) fn render_storage_move_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    match &state.storage_move {
        StorageMoveInput::Inactive => {}
        StorageMoveInput::PickManny { mannies, selection } => {
            let names: Vec<&str> = mannies.iter().map(|(_, n)| n.as_str()).collect();
            let height = (mannies.len() as u16 + 6).min(18);
            render_pick_list(frame, area, " STORAGE MOVE — SELECT MANNY ", 52, height,
                None, &names, *selection, None, "select");
        }
        StorageMoveInput::PickKind { selection, .. } => {
            render_pick_list(frame, area, " STORAGE MOVE — KIND ", 46, 8,
                None, &["resource", "item"], *selection, None, "select");
        }
        StorageMoveInput::ConfigureResource {
            containers, resource_idx, from_sel, to_sel, amount_buf, field, error, ..
        } => {
            let [body, footer] = framed(" STORAGE MOVE — RESOURCE ", area, 56, 9, frame);
            let cname = |i: usize| containers.get(i).map(|(_, l)| l.clone()).unwrap_or_default();
            let lines = vec![
                field_row("Resource:", MOVE_RESOURCE_TYPES[*resource_idx].to_string(), *field == 0, false),
                field_row("From:", cname(*from_sel), *field == 1, false),
                field_row("To:", cname(*to_sel), *field == 2, false),
                field_row("Amount:", format!("{amount_buf} ECE"), *field == 3, *field == 3),
                error_line(error),
            ];
            frame.render_widget(Paragraph::new(lines), body);
            frame.render_widget(form_footer(), footer);
        }
        StorageMoveInput::ConfigureItem {
            containers, items, to_sel, item_cursor, field, error, ..
        } => {
            let height = (items.len() as u16 + 7).clamp(10, 22);
            let [body, footer] = framed(" STORAGE MOVE — ITEMS ", area, 60, height, frame);
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(2), Constraint::Min(1)])
                .split(body);

            let dest = containers.get(*to_sel).map(|(_, l)| l.clone()).unwrap_or_default();
            let mut head = vec![field_row("To:", dest, *field == 1, false)];
            if let Some(err) = error {
                head.push(Line::from(Span::styled(format!("✗ {err}"), Style::default().fg(Color::Red))));
            }
            frame.render_widget(Paragraph::new(head), rows[0]);

            let item_lines: Vec<ListItem> = items
                .iter()
                .map(|(_, label, sel)| {
                    let mark = if *sel { "[x] " } else { "[ ] " };
                    let color = if *sel { Color::Green } else { Color::DarkGray };
                    ListItem::new(Line::from(vec![
                        Span::styled(mark, Style::default().fg(color)),
                        Span::styled(label.clone(), Style::default().fg(Color::White)),
                    ]))
                })
                .collect();
            let mut list = List::new(item_lines);
            if *field == 0 {
                list = list
                    .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                    .highlight_symbol("▶ ");
            }
            let mut ls = ListState::default();
            if *field == 0 && !items.is_empty() {
                ls.select(Some((*item_cursor).min(items.len() - 1)));
            }
            frame.render_stateful_widget(list, rows[1], &mut ls);
            frame.render_widget(item_footer(), footer);
        }
    }
}

fn error_line(error: &Option<String>) -> Line<'static> {
    match error {
        Some(err) => Line::from(Span::styled(format!("✗ {err}"), Style::default().fg(Color::Red))),
        None => Line::default(),
    }
}

fn form_footer() -> Paragraph<'static> {
    Paragraph::new(Line::from(vec![
        Span::styled("[↑/↓]", Style::default().fg(Color::Cyan)),
        Span::raw(" field  "),
        Span::styled("[←/→]", Style::default().fg(Color::Cyan)),
        Span::raw(" change  "),
        Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw(" move  "),
        Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
        Span::raw(" cancel"),
    ]))
}

fn item_footer() -> Paragraph<'static> {
    Paragraph::new(Line::from(vec![
        Span::styled("[Tab]", Style::default().fg(Color::Cyan)),
        Span::raw(" pane  "),
        Span::styled("[Space]", Style::default().fg(Color::Cyan)),
        Span::raw(" toggle  "),
        Span::styled("[←/→]", Style::default().fg(Color::Cyan)),
        Span::raw(" dest  "),
        Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw(" move  "),
        Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
        Span::raw(" cancel"),
    ]))
}
