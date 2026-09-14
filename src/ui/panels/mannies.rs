use crate::api::types::{Manny, MannyLocationType, MannyTask, MannyTaskVisibility};
use crate::app::AppState;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::ui::theme::{format_duration, pane_block, scroll_markers, Palette};
use chrono::{DateTime, Utc};
// ── Mannies panel ─────────────────────────────────────────────────────────────

pub(crate) fn render_mannies_panel(frame: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let p = state.palette();
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
    let total = mannies.len();
    frame.render_stateful_widget(list, inner, &mut list_state);
    // The widget resolves its own scroll during the render, so its offset is
    // only known afterwards — read it back for the overflow markers (#326).
    scroll_markers(frame, area, list_state.offset() as u16, total, focused, p);
}

/// Short label for a Manny task (shared by the list and the detail view).
pub(crate) fn manny_task_label(task: Option<&MannyTask>) -> &'static str {
    match task {
        None => "idle",
        Some(MannyTask::Repair) => "repair",
        Some(MannyTask::Mining) => "mining",
        Some(MannyTask::MotorizingAsteroid) => "motorizing asteroid",
        Some(MannyTask::RefuelingMotorizedAsteroid) => "refueling asteroid",
        Some(MannyTask::SculptingDuckAsteroid) => "sculpting a duck",
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

/// Seven continuous days without capacity and the Manny abandons its cargo,
/// retries docking, and — if its own 0.05 ECE slot is still unavailable — is
/// detached from the probe and becomes an `abandoned` sector object to be
/// recovered (API v123, issue #364).
pub(crate) const STORAGE_WAIT_ABANDON_SECS: i64 = 7 * 24 * 3600;

/// How loudly the remaining wait is stated: the last day before the deadline
/// is the one where the pilot can still act on it.
pub(crate) const STORAGE_WAIT_CRITICAL_SECS: i64 = 24 * 3600;

/// A Manny's storage-docking wait, when it is in one.
pub(crate) struct StorageWait {
    pub elapsed_secs: i64,
    pub remaining_secs: i64,
}

impl StorageWait {
    /// True in the last day, when the loss is close enough to act on.
    pub fn critical(&self) -> bool {
        self.remaining_secs <= STORAGE_WAIT_CRITICAL_SECS
    }

    /// `waiting 3d 4h · abandons cargo in 3d 20h`, the whole story on one row.
    pub fn summary(&self) -> String {
        format!(
            "waiting {} · abandons cargo in {}",
            format_duration(self.elapsed_secs),
            format_duration(self.remaining_secs)
        )
    }
}

/// The Manny's storage-docking wait, read from the task payload the way the
/// hidden-container detection is (`artificialObjectDetected`): `task` is kept
/// as a raw value, so no typing change is needed to consume one more field.
///
/// A label alone ("waiting for space") says nothing about a seven-day clock
/// that ends in losing both the cargo and the Manny.
pub(crate) fn manny_storage_wait(m: &Manny) -> Option<StorageWait> {
    if m.current_task != Some(MannyTask::WaitingForSpace) {
        return None;
    }
    let since = m.task.as_ref()?.get("waitingForSpaceSince")?.as_str()?;
    let since: DateTime<Utc> = since.parse().ok()?;
    // A clock that moved yields a negative span; clamp rather than render a
    // countdown that has run backwards.
    let elapsed_secs = (Utc::now() - since).num_seconds().max(0);
    Some(StorageWait {
        elapsed_secs,
        remaining_secs: (STORAGE_WAIT_ABANDON_SECS - elapsed_secs).max(0),
    })
}

/// Time remaining on the current task, as a compact duration (if known).
pub(crate) fn manny_task_eta(m: &Manny) -> Option<String> {
    m.task_estimated_end_time
        .map(|end| format_duration((end - Utc::now()).num_seconds().max(0)))
}

/// Task progress in 0..=1, interpolated client-side so it ticks between
/// fetches.
///
/// With `task_start_time` (API v116) the span is known exactly, so progress is
/// simply where the wall clock sits between start and end. Before v116 the
/// server gave only a snapshot `task_progress_percent` stamped at `observed_at`
/// plus an estimated end, from which the total had to be *inferred* — that
/// fallback is kept for older servers, and it is an approximation: it trusts a
/// percentage the server rounded, and it drifts if the task is not linear.
pub(crate) fn manny_task_progress(m: &Manny) -> f64 {
    let p0 = (m.task_progress_percent / 100.0).clamp(0.0, 1.0);
    if let (Some(start), Some(end)) = (m.task_start_time, m.task_estimated_end_time) {
        let total = (end - start).num_seconds() as f64;
        if total > 0.0 {
            let elapsed = (Utc::now() - start).num_seconds() as f64;
            return (elapsed / total).clamp(0.0, 1.0);
        }
        // start == end (or worse): nothing to interpolate over.
        return if end <= Utc::now() { 1.0 } else { p0 };
    }
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
    // A storage wait outranks the other details: it is the only task on the
    // roster with a deadline that destroys something (API v123, #364).
    let detail_text = manny_storage_wait(m)
        .map(|w| w.summary())
        .or_else(|| manny_crafting_detail(m))
        .or_else(|| {
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

    // The wait is the one detail that is not quiet: it is a countdown to
    // losing the cargo and then the Manny, and it gets louder in the last day.
    let detail_style = match manny_storage_wait(m) {
        Some(w) if w.critical() => Style::default().fg(p.crit).add_modifier(Modifier::BOLD),
        Some(_) => Style::default().fg(p.warn),
        None => secondary,
    };

    ListItem::new(Line::from(vec![
        Span::styled(format!("{loc} "), secondary),
        Span::styled(format!("{:<12}", m.name), primary),
        Span::styled(label, task_style),
        Span::styled(detail, detail_style),
        Span::styled(eta, secondary),
        Span::styled(via_scut, secondary),
    ]))
}

#[cfg(test)]
mod tests {
    use super::{manny_artificial_detection, manny_crafting_detail, manny_storage_wait, STORAGE_WAIT_ABANDON_SECS};
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

    fn waiting_manny(task: &str) -> Manny {
        serde_json::from_str(&format!(
            r#"{{
            "id":"m1","name":"Manny-1","location":{{"type":"probe","sector":null}},
            "currentTask":"waiting_for_space","taskProgressPercent":0.0,
            "cargo":{{"capacity":0.3,"deuterium":0.0,"metals":0.1,"ice":0.0,"organicCompounds":0.0}},
            "canReceiveOrders":false,"taskEstimatedEndTime":null,"task":{task}
        }}"#
        ))
        .unwrap()
    }

    #[test]
    fn a_storage_wait_carries_its_seven_day_deadline() {
        // The label alone said nothing about a clock that ends in losing both
        // the cargo and the Manny (API v123, issue #364).
        let since = chrono::Utc::now() - chrono::Duration::days(3);
        let m = waiting_manny(&format!(
            r#"{{"waitingFor":"storage_space","waitingForSpaceSince":"{}"}}"#,
            since.to_rfc3339()
        ));
        let w = manny_storage_wait(&m).expect("a wait is reported");
        assert!((w.elapsed_secs - 3 * 24 * 3600).abs() < 5, "three days in");
        assert!(
            (w.remaining_secs - 4 * 24 * 3600).abs() < 5,
            "four days left of the seven"
        );
        assert!(!w.critical(), "not yet the last day");
        assert!(w.summary().contains("abandons cargo in"), "{}", w.summary());
    }

    #[test]
    fn the_last_day_of_a_storage_wait_is_critical() {
        let since = chrono::Utc::now() - chrono::Duration::seconds(STORAGE_WAIT_ABANDON_SECS - 3600);
        let m = waiting_manny(&format!(
            r#"{{"waitingFor":"storage_space","waitingForSpaceSince":"{}"}}"#,
            since.to_rfc3339()
        ));
        let w = manny_storage_wait(&m).unwrap();
        assert!(w.critical(), "an hour from losing the cargo");
        assert!(w.remaining_secs > 0);
    }

    #[test]
    fn a_storage_wait_claims_nothing_without_a_start() {
        // Pre-v123 servers omit the field, and a Manny on another task is not
        // waiting at all — neither invents a countdown.
        assert!(manny_storage_wait(&waiting_manny(r#"{"waitingFor":"storage_space"}"#)).is_none());
        assert!(manny_storage_wait(&waiting_manny("null")).is_none());
        assert!(
            manny_storage_wait(&crafting_manny(
                "crafting",
                r#"{"waitingForSpaceSince":"2020-01-01T00:00:00Z"}"#
            ))
            .is_none(),
            "the field only means something on a waiting Manny"
        );
    }

    #[test]
    fn a_storage_wait_past_its_deadline_does_not_count_backwards() {
        let since = chrono::Utc::now() - chrono::Duration::seconds(STORAGE_WAIT_ABANDON_SECS + 9_000);
        let m = waiting_manny(&format!(
            r#"{{"waitingFor":"storage_space","waitingForSpaceSince":"{}"}}"#,
            since.to_rfc3339()
        ));
        let w = manny_storage_wait(&m).unwrap();
        assert_eq!(w.remaining_secs, 0, "clamped, not negative");
        assert!(w.critical());
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
