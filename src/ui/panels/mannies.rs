use crate::api::types::{
    Manny, MannyLocationType, MannyTask,
};
use crate::app::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::ui::theme::panel_block;
// ── Mannies panel ─────────────────────────────────────────────────────────────

pub(crate) fn render_mannies_panel(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let block = panel_block(" MANNIES ", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);
    let list_area = rows[0];
    let hint_area = rows[1];

    // Hint bar
    if focused {
        let selected_manny = state.mannies.as_ref()
            .and_then(|m| m.get(state.mannies_selection));
        let can_order = selected_manny.map(|m| m.can_receive_orders).unwrap_or(false);
        let is_busy = selected_manny.map(|m| !m.can_receive_orders && m.current_task.is_some()).unwrap_or(false);
        if can_order {
            let has_detachable = !state.collect_detachable_containers().is_empty();
            let has_detached = !state.collect_detached_containers().is_empty();
            let mut spans = vec![
                Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
                Span::raw(" repair  "),
                Span::styled("[e]", Style::default().fg(Color::Cyan)),
                Span::raw(" mine  "),
                Span::styled("[c]", Style::default().fg(Color::Cyan)),
                Span::raw(" craft  "),
                Span::styled("[s]", Style::default().fg(Color::Cyan)),
                Span::raw(" salvage  "),
                Span::styled("[x]", Style::default().fg(Color::Cyan)),
                Span::raw(" inspect"),
            ];
            if has_detachable {
                spans.push(Span::raw("  "));
                spans.push(Span::styled("[D]", Style::default().fg(Color::Cyan)));
                spans.push(Span::raw(" detach"));
            }
            if has_detached {
                spans.push(Span::raw("  "));
                spans.push(Span::styled("[v]", Style::default().fg(Color::Cyan)));
                spans.push(Span::raw(" recover"));
            }
            spans.push(Span::raw("  "));
            spans.push(Span::styled("[n]", Style::default().fg(Color::Cyan)));
            spans.push(Span::raw(" rename"));
            if is_busy {
                spans.push(Span::raw("  "));
                spans.push(Span::styled("[R]", Style::default().fg(Color::Yellow)));
                spans.push(Span::raw(" recall"));
            }
            frame.render_widget(Paragraph::new(Line::from(spans)), hint_area);
        } else if is_busy {
            let is_waiting = selected_manny
                .map(|m| m.current_task == Some(MannyTask::WaitingForSpace))
                .unwrap_or(false);
            let mut spans = vec![
                Span::styled("busy  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[R]", Style::default().fg(Color::Yellow)),
                Span::raw(" recall  "),
                Span::styled("[n]", Style::default().fg(Color::Cyan)),
                Span::raw(" rename"),
            ];
            if is_waiting {
                spans.push(Span::raw("  "));
                spans.push(Span::styled("[X]", Style::default().fg(Color::Red)));
                spans.push(Span::raw(" drop cargo"));
            }
            frame.render_widget(Paragraph::new(Line::from(spans)), hint_area);
        } else {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("busy — cannot receive orders  ", Style::default().fg(Color::DarkGray)),
                    Span::styled("[n]", Style::default().fg(Color::Cyan)),
                    Span::raw(" rename"),
                ])),
                hint_area,
            );
        }
    }

    let Some(mannies) = &state.mannies else {
        frame.render_widget(
            Paragraph::new("No data").style(Style::default().fg(Color::DarkGray)),
            list_area,
        );
        return;
    };

    if mannies.is_empty() {
        frame.render_widget(
            Paragraph::new("No mannies aboard").style(Style::default().fg(Color::DarkGray)),
            list_area,
        );
        return;
    }

    let items: Vec<ListItem> = mannies.iter().map(|m| manny_list_item(m)).collect();

    let highlight_style = if focused {
        Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let list = List::new(items)
        .highlight_style(highlight_style)
        .highlight_symbol("▶ ");

    let mut list_state = ListState::default();
    if focused {
        list_state.select(Some(state.mannies_selection));
    }

    frame.render_stateful_widget(list, list_area, &mut list_state);
}

pub(crate) fn manny_list_item(m: &Manny) -> ListItem<'_> {
    let loc_icon = match m.location.location_type {
        MannyLocationType::Probe => Span::styled("●", Style::default().fg(Color::Green)),
        MannyLocationType::Sector => Span::styled("◌", Style::default().fg(Color::Yellow)),
        MannyLocationType::Unknown => Span::styled("?", Style::default().fg(Color::DarkGray)),
    };

    let task_text = match &m.current_task {
        None => Span::styled("idle", Style::default().fg(Color::DarkGray)),
        Some(MannyTask::Repair) => Span::styled("repair", Style::default().fg(Color::Cyan)),
        Some(MannyTask::Mining) => Span::styled("mining", Style::default().fg(Color::Yellow)),
        Some(MannyTask::Crafting) => Span::styled("crafting", Style::default().fg(Color::Cyan)),
        Some(MannyTask::AssistingAtomicPrinter) => Span::styled("assisting printer", Style::default().fg(Color::Cyan)),
        Some(MannyTask::Salvage) => Span::styled("salvage", Style::default().fg(Color::Yellow)),
        Some(MannyTask::InstallingWaypointBookmark) => Span::styled("installing waypoint", Style::default().fg(Color::Green)),
        Some(MannyTask::DetachingStorageContainer) => Span::styled("detaching container", Style::default().fg(Color::Yellow)),
        Some(MannyTask::InspectingAsteroid) => Span::styled("inspecting", Style::default().fg(Color::Yellow)),
        Some(MannyTask::Returning) => Span::styled("returning", Style::default().fg(Color::Blue)),
        Some(MannyTask::WaitingForSpace) => Span::styled("waiting", Style::default().fg(Color::Magenta)),
        Some(MannyTask::MovingStockage) => Span::styled("moving cargo", Style::default().fg(Color::Blue)),
        Some(MannyTask::DroppingStorageContainer) => Span::styled("dropping container", Style::default().fg(Color::Yellow)),
        Some(MannyTask::Unknown) => Span::styled("?", Style::default().fg(Color::DarkGray)),
    };

    let progress = if m.current_task.is_some() {
        format!(" {:3.0}%", m.task_progress_percent)
    } else {
        String::new()
    };

    let name = format!("{:<14}", m.name);

    ListItem::new(Line::from(vec![
        loc_icon,
        Span::raw(" "),
        Span::raw(name),
        task_text,
        Span::styled(progress, Style::default().fg(Color::DarkGray)),
    ]))
}

