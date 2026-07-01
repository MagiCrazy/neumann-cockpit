use crate::api::types::{
    Manny, MannyLocationType, MannyTask, MannyTaskVisibility,
};
use crate::app::AppState;
use ratatui::{
    layout::Rect,
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

    // Action hints come from the cockpit's shared hints line (F1), so the
    // pane gives its whole area to the list.
    let Some(mannies) = &state.mannies else {
        frame.render_widget(
            Paragraph::new("No data").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    };

    if mannies.is_empty() {
        frame.render_widget(
            Paragraph::new("No mannies aboard").style(Style::default().fg(Color::DarkGray)),
            inner,
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

    frame.render_stateful_widget(list, inner, &mut list_state);
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
        Some(MannyTask::RefillingDeuteriumTank) => Span::styled("refueling", Style::default().fg(Color::Green)),
        Some(MannyTask::TurningOnScutRelay) => Span::styled("activating relay", Style::default().fg(Color::LightBlue)),
        Some(MannyTask::UnknownTooFar) => Span::styled("too far", Style::default().fg(Color::DarkGray)),
        Some(MannyTask::Unknown) => Span::styled("?", Style::default().fg(Color::DarkGray)),
    };

    let progress = if m.current_task.is_some() {
        format!(" {:3.0}%", m.task_progress_percent)
    } else {
        String::new()
    };

    let name = format!("{:<14}", m.name);

    let via_scut = if matches!(m.task_visibility, Some(MannyTaskVisibility::ScutNetwork)) {
        Span::styled(" ≣ via SCUT", Style::default().fg(Color::LightBlue))
    } else {
        Span::raw("")
    };

    ListItem::new(Line::from(vec![
        loc_icon,
        Span::raw(" "),
        Span::raw(name),
        task_text,
        Span::styled(progress, Style::default().fg(Color::DarkGray)),
        via_scut,
    ]))
}

