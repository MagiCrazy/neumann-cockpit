use crate::api::types::{
    Manny, MannyLocationType, MannyTask, MannyTaskVisibility,
};
use crate::app::AppState;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::ui::theme::{format_duration, palette, pane_block, Palette};
use chrono::Utc;
// ── Mannies panel ─────────────────────────────────────────────────────────────

pub(crate) fn render_mannies_panel(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let p = palette(state.color_mode);
    let block = pane_block(" MANNIES ", focused, p);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Action hints come from the cockpit's shared hints line (F1), so the
    // pane gives its whole area to the list.
    let Some(mannies) = &state.mannies else {
        frame.render_widget(Paragraph::new("No data").style(Style::default().fg(p.dim)), inner);
        return;
    };

    if mannies.is_empty() {
        frame.render_widget(
            Paragraph::new("No mannies aboard").style(Style::default().fg(p.dim)),
            inner,
        );
        return;
    }

    // Selection is styled per-row (accent) rather than via a background fill,
    // so the progress/ETA stay legible on the selected line.
    let sel = state.mannies_selection;
    let items: Vec<ListItem> = mannies
        .iter()
        .enumerate()
        .map(|(i, m)| manny_list_item(m, focused && i == sel, p))
        .collect();

    let list = List::new(items)
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().fg(p.accent).add_modifier(Modifier::BOLD));
    let mut list_state = ListState::default();
    if focused {
        list_state.select(Some(sel));
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

/// Time remaining on the current task, as a compact duration (if known).
pub(crate) fn manny_task_eta(m: &Manny) -> Option<String> {
    m.task_estimated_end_time
        .map(|end| format_duration((end - Utc::now()).num_seconds().max(0)))
}

pub(crate) fn manny_list_item(m: &Manny, selected: bool, p: Palette) -> ListItem<'_> {
    // On the selected row everything is accent so the ETA/% stay legible;
    // otherwise the palette's text for the name/task and dim for the rest.
    let primary = if selected {
        Style::default().fg(p.accent).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(p.text)
    };
    let secondary = if selected {
        Style::default().fg(p.accent)
    } else {
        Style::default().fg(p.dim)
    };

    let loc = match m.location.location_type {
        MannyLocationType::Probe => "●",
        MannyLocationType::Sector => "◌",
        MannyLocationType::Unknown => "?",
    };
    let task = m.current_task.as_ref();
    let task_style = if task.is_none() { secondary } else { primary };

    let progress = if m.current_task.is_some() {
        format!(" {:3.0}%", m.task_progress_percent)
    } else {
        String::new()
    };
    let eta = manny_task_eta(m)
        .filter(|_| m.current_task.is_some())
        .map(|d| format!(" · {d}"))
        .unwrap_or_default();

    let via_scut = if matches!(m.task_visibility, Some(MannyTaskVisibility::ScutNetwork)) {
        " ≣"
    } else {
        ""
    };

    ListItem::new(Line::from(vec![
        Span::styled(format!("{loc} "), secondary),
        Span::styled(format!("{:<12}", m.name), primary),
        Span::styled(manny_task_label(task), task_style),
        Span::styled(progress, secondary),
        Span::styled(eta, secondary),
        Span::styled(via_scut, secondary),
    ]))
}

