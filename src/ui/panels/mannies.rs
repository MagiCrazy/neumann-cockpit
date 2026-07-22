use crate::api::types::{Manny, MannyLocationType, MannyTask, MannyTaskVisibility};
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
        .map(|(i, m)| manny_list_item(m, focused && i == sel, p, inner.width))
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
        Some(MannyTask::InspectingSectorObject) => "inspecting",
        Some(MannyTask::ImprovingProbe) => "improving probe",
        Some(MannyTask::TransferringDeuteriumToProbe) => "transferring deuterium",
        Some(MannyTask::TransferringToProbe) => "transferring to probe",
        Some(MannyTask::InstallingScutTransitBeacon) => "installing beacon",
        Some(MannyTask::AssemblingProbe) => "assembling probe",
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

/// Mining task detail, extracted from the Manny's `task` payload: which
/// asteroid, the resource types, and where the output goes (a named container
/// or the probe). `None` unless the Manny is mining with a visible payload.
pub(crate) struct MiningDetail {
    pub target: String,
    pub resources: Option<String>,
    pub destination: String,
}

pub(crate) fn manny_mining_detail(m: &Manny) -> Option<MiningDetail> {
    if m.current_task != Some(MannyTask::Mining) {
        return None;
    }
    let task = m.task.as_ref()?;
    let target = task.get("target");
    let name = target
        .and_then(|t| t.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("asteroid")
        .to_string();
    let resources = target
        .and_then(|t| t.get("resourceTypes"))
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join("/"))
        .filter(|s| !s.is_empty());
    // A targetContainer object means the output is dropped into that detached
    // container; otherwise it comes back to the probe.
    let destination = match task.get("targetContainer") {
        Some(tc) if tc.is_object() => tc
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("container")
            .to_string(),
        _ => "probe".to_string(),
    };
    Some(MiningDetail {
        target: name,
        resources,
        destination,
    })
}

/// Crafting task detail, extracted from the Manny's `task` payload: the
/// human-readable recipe name (`recipeName`, falling back to the `recipe` id).
/// Covers both a Manny crafting on its own and one assisting the atomic
/// printer — both carry the recipe. `None` unless it is (assisting a) craft
/// with a visible payload.
pub(crate) fn manny_crafting_detail(m: &Manny) -> Option<String> {
    if !matches!(
        m.current_task,
        Some(MannyTask::Crafting) | Some(MannyTask::AssistingAtomicPrinter)
    ) {
        return None;
    }
    let task = m.task.as_ref()?;
    task.get("recipeName")
        .or_else(|| task.get("recipe"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// A hidden artificial object (detached container) a Manny turned up while
/// mining, extracted from its `task` payload. `None` unless one was detected.
pub(crate) fn manny_artificial_detection(m: &Manny) -> Option<crate::api::types::ArtificialObjectDetection> {
    let v = m.task.as_ref()?.get("artificialObjectDetected")?;
    serde_json::from_value(v.clone()).ok()
}

/// Time remaining on the current task, as a compact duration (if known).
pub(crate) fn manny_task_eta(m: &Manny) -> Option<String> {
    m.task_estimated_end_time
        .map(|end| format_duration((end - Utc::now()).num_seconds().max(0)))
}

/// Task progress in 0..=1, interpolated client-side so it ticks between
/// fetches. The server sends a snapshot `task_progress_percent` at
/// `observed_at` plus an estimated end time; assuming a linear task we rebuild
/// the timeline and advance progress with the wall clock. Falls back to the
/// raw snapshot when timestamps are missing, and never runs backward from it.
pub(crate) fn manny_task_progress(m: &Manny) -> f64 {
    let p0 = (m.task_progress_percent / 100.0).clamp(0.0, 1.0);
    let (Some(obs), Some(end)) = (m.observed_at, m.task_estimated_end_time) else {
        return p0;
    };
    let remaining_at_obs = (end - obs).num_seconds() as f64;
    if remaining_at_obs <= 0.0 || p0 >= 1.0 {
        // Overdue or already complete at the snapshot.
        return if end <= Utc::now() { 1.0 } else { p0 };
    }
    let total = remaining_at_obs / (1.0 - p0);
    let remaining_now = (end - Utc::now()).num_seconds() as f64;
    (1.0 - remaining_now / total).clamp(p0, 1.0)
}

/// Truncate `s` to at most `max` display columns, appending an ellipsis when
/// it does not fit. Returns an empty string if `max` is 0.
fn truncate_ellipsis(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    match max {
        0 => String::new(),
        1 => "…".to_string(),
        _ => format!("{}…", s.chars().take(max - 1).collect::<String>()),
    }
}

pub(crate) fn manny_list_item(m: &Manny, selected: bool, p: Palette, width: u16) -> ListItem<'_> {
    // On the selected row everything is accent so the ETA stays legible;
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

    // Time remaining only — the raw % lives in the wider overview/detail views;
    // the compact row keeps the ETA, which is what the pilot watches.
    let eta = manny_task_eta(m)
        .filter(|_| m.current_task.is_some())
        .map(|d| format!(" · {d}"))
        .unwrap_or_default();

    let via_scut = if matches!(m.task_visibility, Some(MannyTaskVisibility::ScutNetwork)) {
        " ≣"
    } else {
        ""
    };

    // The flexible detail span, mirroring the wider views in brief: the recipe
    // a Manny is crafting, or what/where it is mining (`{resources} → {dest}`).
    // It truncates to the width left after the fixed columns and the ETA, so a
    // long name never pushes the ETA off-row.
    let label = manny_task_label(task);
    let detail_text = manny_crafting_detail(m).or_else(|| {
        manny_mining_detail(m).map(|d| {
            let what = d.resources.unwrap_or(d.target);
            format!("{what} → {}", d.destination)
        })
    });
    let detail = detail_text
        .map(|t| {
            // Reserved: highlight symbol (2) · "{loc} " (2) · name (12) ·
            // label · eta · scut · the detail's own leading space (1).
            let fixed = 2 + 2 + 12 + label.chars().count() + eta.chars().count() + via_scut.chars().count() + 1;
            let budget = (width as usize).saturating_sub(fixed);
            truncate_ellipsis(&t, budget)
        })
        .filter(|t| !t.is_empty())
        .map(|t| format!(" {t}"))
        .unwrap_or_default();

    ListItem::new(Line::from(vec![
        Span::styled(format!("{loc} "), secondary),
        Span::styled(format!("{:<12}", m.name), primary),
        Span::styled(label, task_style),
        Span::styled(detail, secondary),
        Span::styled(eta, secondary),
        Span::styled(via_scut, secondary),
    ]))
}

#[cfg(test)]
mod tests {
    use super::{manny_artificial_detection, manny_crafting_detail};
    use crate::api::types::Manny;

    fn mining_manny(task: &str) -> Manny {
        serde_json::from_str(&format!(
            r#"{{
            "id":"m1","name":"Manny-1","location":{{"type":"sector","sector":null}},
            "currentTask":"mining","taskProgressPercent":50.0,
            "cargo":{{"capacity":0.3,"deuterium":0.0,"metals":0.0,"ice":0.0,"organicCompounds":0.0}},
            "canReceiveOrders":false,"taskEstimatedEndTime":null,"task":{task}
        }}"#
        ))
        .unwrap()
    }

    fn crafting_manny(current_task: &str, task: &str) -> Manny {
        serde_json::from_str(&format!(
            r#"{{
            "id":"m1","name":"Manny-1","location":{{"type":"probe","sector":null}},
            "currentTask":"{current_task}","taskProgressPercent":50.0,
            "cargo":{{"capacity":0.3,"deuterium":0.0,"metals":0.0,"ice":0.0,"organicCompounds":0.0}},
            "canReceiveOrders":false,"taskEstimatedEndTime":null,"task":{task}
        }}"#
        ))
        .unwrap()
    }

    #[test]
    fn detects_hidden_container_in_mining_task() {
        let m = mining_manny(
            r#"{"objectId":"ast-1","artificialObjectDetected":
            {"type":"detached_storage_container","detection":"hidden_on_asteroid","objectId":"c-9"}}"#,
        );
        let d = manny_artificial_detection(&m).expect("detection present");
        assert_eq!(d.object_id.as_deref(), Some("c-9"));
    }

    #[test]
    fn no_detection_without_payload() {
        let m = mining_manny(r#"{"objectId":"ast-1"}"#);
        assert!(manny_artificial_detection(&m).is_none());
    }

    #[test]
    fn crafting_detail_prefers_recipe_name() {
        let m = crafting_manny("crafting", r#"{"recipe":"battery_pack","recipeName":"Battery pack"}"#);
        assert_eq!(manny_crafting_detail(&m).as_deref(), Some("Battery pack"));
    }

    #[test]
    fn crafting_detail_falls_back_to_recipe_id() {
        let m = crafting_manny("crafting", r#"{"recipe":"battery_pack"}"#);
        assert_eq!(manny_crafting_detail(&m).as_deref(), Some("battery_pack"));
    }

    #[test]
    fn crafting_detail_covers_atomic_printer_assist() {
        let m = crafting_manny(
            "assisting_atomic_printer",
            r#"{"recipe":"micro_conductor","recipeName":"Micro-etched conductor"}"#,
        );
        assert_eq!(manny_crafting_detail(&m).as_deref(), Some("Micro-etched conductor"));
    }

    #[test]
    fn truncate_ellipsis_fits_and_shrinks() {
        use super::truncate_ellipsis;
        assert_eq!(truncate_ellipsis("Battery pack", 20), "Battery pack");
        assert_eq!(truncate_ellipsis("Battery pack", 12), "Battery pack");
        assert_eq!(truncate_ellipsis("Battery pack", 5), "Batt…");
        assert_eq!(truncate_ellipsis("Battery pack", 1), "…");
        assert_eq!(truncate_ellipsis("Battery pack", 0), "");
    }

    #[test]
    fn new_v96_task_states_have_labels_not_placeholder() {
        use super::manny_task_label;
        use crate::api::types::MannyTask;
        for (raw, expected) in [
            ("transferring_deuterium_to_probe", "transferring deuterium"),
            ("transferring_to_probe", "transferring to probe"),
            ("installing_scut_transit_beacon", "installing beacon"),
            ("assembling_probe", "assembling probe"),
        ] {
            let task: MannyTask = serde_json::from_str(&format!("\"{raw}\"")).unwrap();
            assert_ne!(task, MannyTask::Unknown, "{raw} must not fall back to Unknown");
            assert_eq!(manny_task_label(Some(&task)), expected);
        }
    }

    #[test]
    fn crafting_detail_none_when_not_crafting() {
        let m = mining_manny(r#"{"recipeName":"Battery pack"}"#);
        assert!(manny_crafting_detail(&m).is_none());
    }
}
