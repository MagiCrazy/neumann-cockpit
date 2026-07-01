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

use crate::ui::theme::{format_duration, palette, pane_block};
use chrono::Utc;
// ── Mannies panel ─────────────────────────────────────────────────────────────

pub(crate) fn render_mannies_panel(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let block = pane_block(" MANNIES ", focused, palette(state.color_mode));
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

/// Short label for a Manny task (shared by the list and the detail view).
pub(crate) fn manny_task_label(task: Option<&MannyTask>) -> &'static str {
    match task {
        None => "idle",
        Some(MannyTask::Repair) => "repair",
        Some(MannyTask::Mining) => "mining",
        Some(MannyTask::Crafting) => "crafting",
        Some(MannyTask::AssistingAtomicPrinter) => "assisting printer",
        Some(MannyTask::Salvage) => "salvage",
        Some(MannyTask::InstallingWaypointBookmark) => "installing waypoint",
        Some(MannyTask::DetachingStorageContainer) => "detaching container",
        Some(MannyTask::InspectingAsteroid) => "inspecting",
        Some(MannyTask::Returning) => "returning",
        Some(MannyTask::WaitingForSpace) => "waiting for space",
        Some(MannyTask::MovingStockage) => "moving cargo",
        Some(MannyTask::DroppingStorageContainer) => "dropping container",
        Some(MannyTask::RefillingDeuteriumTank) => "refueling",
        Some(MannyTask::TurningOnScutRelay) => "activating relay",
        Some(MannyTask::UnknownTooFar) => "too far",
        Some(MannyTask::Unknown) => "?",
    }
}

fn manny_task_color(task: Option<&MannyTask>) -> Color {
    match task {
        None => Color::DarkGray,
        Some(MannyTask::Repair | MannyTask::Crafting | MannyTask::AssistingAtomicPrinter) => Color::Cyan,
        Some(
            MannyTask::Mining
            | MannyTask::Salvage
            | MannyTask::InspectingAsteroid
            | MannyTask::DetachingStorageContainer
            | MannyTask::DroppingStorageContainer,
        ) => Color::Yellow,
        Some(MannyTask::InstallingWaypointBookmark | MannyTask::RefillingDeuteriumTank) => Color::Green,
        Some(MannyTask::Returning | MannyTask::MovingStockage) => Color::Blue,
        Some(MannyTask::WaitingForSpace) => Color::Magenta,
        Some(MannyTask::TurningOnScutRelay) => Color::LightBlue,
        Some(MannyTask::UnknownTooFar | MannyTask::Unknown) => Color::DarkGray,
    }
}

/// Time remaining on the current task, as a compact duration (if known).
pub(crate) fn manny_task_eta(m: &Manny) -> Option<String> {
    m.task_estimated_end_time
        .map(|end| format_duration((end - Utc::now()).num_seconds().max(0)))
}

pub(crate) fn manny_list_item(m: &Manny) -> ListItem<'_> {
    let loc_icon = match m.location.location_type {
        MannyLocationType::Probe => Span::styled("●", Style::default().fg(Color::Green)),
        MannyLocationType::Sector => Span::styled("◌", Style::default().fg(Color::Yellow)),
        MannyLocationType::Unknown => Span::styled("?", Style::default().fg(Color::DarkGray)),
    };
    let task = m.current_task.as_ref();
    let task_text = Span::styled(manny_task_label(task), Style::default().fg(manny_task_color(task)));

    let progress = if m.current_task.is_some() {
        format!(" {:3.0}%", m.task_progress_percent)
    } else {
        String::new()
    };
    // Time remaining next to the progress, when the task has an ETA.
    let eta = manny_task_eta(m)
        .filter(|_| m.current_task.is_some())
        .map(|d| format!(" · {d}"))
        .unwrap_or_default();

    let name = format!("{:<12}", m.name);

    let via_scut = if matches!(m.task_visibility, Some(MannyTaskVisibility::ScutNetwork)) {
        Span::styled(" ≣", Style::default().fg(Color::LightBlue))
    } else {
        Span::raw("")
    };

    ListItem::new(Line::from(vec![
        loc_icon,
        Span::raw(" "),
        Span::raw(name),
        task_text,
        Span::styled(progress, Style::default().fg(Color::DarkGray)),
        Span::styled(eta, Style::default().fg(Color::DarkGray)),
        via_scut,
    ]))
}

