pub(crate) mod alerts;
pub(crate) mod assemble;
pub(crate) mod containers;
pub(crate) mod craft;
pub(crate) mod drop_container;
pub(crate) mod fleet;
pub(crate) mod help;
pub(crate) mod improve;
pub(crate) mod inventory_detail;
pub(crate) mod jettison;
pub(crate) mod map;
pub(crate) mod messages;
pub(crate) mod mine;
pub(crate) mod missions;
pub(crate) mod object_actions;
pub(crate) mod pickers;
pub(crate) mod remote_mine;
pub(crate) mod repair;
pub(crate) mod scanner;
pub(crate) mod scut_network;
pub(crate) mod storage_move;
pub(crate) mod travel;
pub(crate) mod waypoints;

pub(crate) use alerts::render_alerts_overlay;
pub(crate) use assemble::render_assemble_probe_overlay;
pub(crate) use containers::{render_container_rules_overlay, render_rename_container_overlay};
pub(crate) use craft::render_fabrication_overlay;
pub(crate) use drop_container::render_drop_container_overlay;
pub(crate) use fleet::{render_probe_switch_overlay, render_rename_probe_overlay, render_transfer_deuterium_overlay};
pub(crate) use help::{help_row_count, render_help_overlay};
pub(crate) use improve::render_improve_overlay;
pub(crate) use inventory_detail::render_inventory_detail_overlay;
pub(crate) use jettison::render_jettison_overlay;
pub(crate) use map::{render_goto_visited_overlay, render_map_overlay};
pub(crate) use messages::render_messages_overlay;
pub(crate) use mine::render_mine_overlay;
pub(crate) use missions::render_missions_overlay;
pub(crate) use object_actions::render_object_action_overlay;
pub(crate) use pickers::{
    render_deploy_overlay, render_detach_overlay, render_drop_cargo_overlay, render_inspect_overlay,
    render_mind_snapshot_overlay, render_recall_overlay, render_recover_overlay, render_refuel_overlay,
    render_rename_manny_overlay, render_salvage_overlay, render_scut_relay_overlay,
};
pub(crate) use remote_mine::render_remote_mine_overlay;
pub(crate) use repair::render_repair_overlay;
pub(crate) use scanner::render_scan_input_overlay;
pub(crate) use scut_network::render_scut_network_overlay;
pub(crate) use storage_move::render_storage_move_overlay;
pub(crate) use travel::render_travel_overlay;
pub(crate) use waypoints::render_waypoints_overlay;

use crate::api::types::DangerLevel;
use crate::app::{ActiveWizard, AppState, GotoVisitedInput, ProbeSwitchInput, ScanMode, RESOURCE_LABELS};
use crate::ui::theme::{palette, Palette};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

type OverlayGuard = fn(&AppState) -> bool;
type OverlayRender = fn(&mut Frame, Rect, &AppState);

/// The wizard overlays, in z-order (back to front): a guard (is this wizard
/// active?) paired with its render fn. This is the single source of truth for
/// the render side — adding a wizard overlay is one line here, replacing the
/// hand-synchronized `if !matches!` ladder. The guard is required: not every
/// render fn early-returns when inactive (several draw their frame before
/// matching the state), so the caller must gate them.
#[allow(clippy::type_complexity)]
#[rustfmt::skip]
const WIZARD_OVERLAYS: &[(OverlayGuard, OverlayRender)] = &[
    (|s| matches!(s.active_wizard, ActiveWizard::AssembleProbe(_)), render_assemble_probe_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::RenameProbe(_)), render_rename_probe_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::Alerts(_)), render_alerts_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::Travel(_)), render_travel_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::Repair(_)), render_repair_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::Mine(_)), render_mine_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::RemoteMine(_)), render_remote_mine_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::Jettison(_)), render_jettison_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::Fabrication(_)), render_fabrication_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::Improve(_)), render_improve_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::Salvage(_)), render_salvage_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::Recall(_)), render_recall_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::Refuel(_)), render_refuel_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::TransferDeuterium(_)), render_transfer_deuterium_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::MindSnapshot(_)), render_mind_snapshot_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::Missions(_)), render_missions_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::Messages(_)), render_messages_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::ScutRelay(_)), render_scut_relay_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::ScutNetwork(_)), render_scut_network_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::DropCargo(_)), render_drop_cargo_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::Deploy(_)), render_deploy_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::RenameManny(_)), render_rename_manny_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::Inspect(_)), render_inspect_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::Recover(_)), render_recover_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::Detach(_)), render_detach_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::DropContainer(_)), render_drop_container_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::ObjectAction(_)), render_object_action_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::Waypoints(_)), render_waypoints_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::RenameContainer(_)), render_rename_container_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::ContainerRules(_)), render_container_rules_overlay),
    (|s| matches!(s.active_wizard, ActiveWizard::StorageMove(_)), render_storage_move_overlay),
];

/// Render whichever wizard overlays are active, on top of the cockpit grid.
pub(crate) fn render_active_overlays(frame: &mut Frame, area: Rect, state: &AppState) {
    for (active, render) in WIZARD_OVERLAYS {
        if active(state) {
            render(frame, area, state);
        }
    }
    // Modals that don't follow the `*Input::Inactive` pattern (bool flags or a
    // distinct enum) stay explicit. `help` is topmost, so it renders last.
    if state.map.open {
        render_map_overlay(frame, area, state);
    }
    if matches!(state.probe_switch, ProbeSwitchInput::Picking { .. }) {
        render_probe_switch_overlay(frame, area, state);
    }
    if matches!(state.goto_visited, GotoVisitedInput::Picking { .. }) {
        render_goto_visited_overlay(frame, area, state);
    }
    if state.inventory_detail_open {
        render_inventory_detail_overlay(frame, area, state);
    }
    if !matches!(state.scan_mode, ScanMode::Current) {
        render_scan_input_overlay(frame, area, state);
    }
    if state.help_open {
        render_help_overlay(frame, area, palette(state.color_mode), state.help_scroll);
    }
}

pub(crate) fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(r.width), height.min(r.height))
}

/// Semantic weight of a footer key, driving its colour and emphasis.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyTone {
    /// Navigation / intermediate step (no game effect). Accent, not bold.
    Nav,
    /// Terminal commit with a game effect (POST/PATCH). Good + bold.
    Commit,
    /// Terminal commit that is destructive or irreversible. Crit + bold.
    Danger,
    /// A commit key that is currently unavailable (form invalid). Dim.
    Disabled,
}

/// One `[key] label` entry in a modal footer.
pub(crate) struct FooterKey<'a> {
    pub key: &'a str,
    pub label: &'a str,
    pub tone: KeyTone,
}

impl<'a> FooterKey<'a> {
    pub fn nav(key: &'a str, label: &'a str) -> Self {
        Self {
            key,
            label,
            tone: KeyTone::Nav,
        }
    }
    pub fn commit(key: &'a str, label: &'a str) -> Self {
        Self {
            key,
            label,
            tone: KeyTone::Commit,
        }
    }
    pub fn danger(key: &'a str, label: &'a str) -> Self {
        Self {
            key,
            label,
            tone: KeyTone::Danger,
        }
    }
}

/// Render the unified modal footer into `area` (expected to be a single row).
///
/// Each key is drawn as a bracketed, coloured `[key]` followed by its raw
/// label, entries separated by two spaces. This is the single source of truth
/// for every modal's bottom hint line; `[Esc]` should always come last as a
/// `Nav` key.
pub(crate) fn render_footer(frame: &mut Frame, area: Rect, p: Palette, keys: &[FooterKey]) {
    let mut spans: Vec<Span> = Vec::with_capacity(keys.len() * 2);
    for (i, fk) in keys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let key_style = match fk.tone {
            KeyTone::Nav => Style::default().fg(p.accent),
            KeyTone::Commit => Style::default().fg(p.good).add_modifier(Modifier::BOLD),
            KeyTone::Danger => Style::default().fg(p.crit).add_modifier(Modifier::BOLD),
            KeyTone::Disabled => Style::default().fg(p.dim),
        };
        spans.push(Span::styled(fk.key.to_owned(), key_style));
        spans.push(Span::raw(format!(" {}", fk.label)));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Format a sector-object pick row shared by every asteroid/object picker:
/// `#n [name][ ⚠/☠]  metals 1.20  ice 0.55 …`. The name is dropped when it is a
/// bare "unnamed" placeholder; danger and per-resource reserves are appended
/// when known. Mirrors the Sector-pane object line so pickers read the same.
pub(crate) fn object_pick_label(
    index: usize,
    name: &str,
    reserves: Option<([bool; 4], [f64; 4])>,
    danger: Option<&DangerLevel>,
) -> String {
    let mut label = format!("#{}", index + 1);
    let name = name.trim();
    if !name.is_empty() && name != "unnamed" {
        label.push_str(&format!(" {name}"));
    }
    match danger {
        Some(DangerLevel::Moderate) => label.push_str(" ⚠"),
        Some(DangerLevel::Extreme) => label.push_str(" ☠"),
        _ => {}
    }
    if let Some((flags, res)) = reserves {
        for (k, &res_label) in RESOURCE_LABELS.iter().enumerate() {
            if !flags[k] {
                continue;
            }
            if res[k] > 0.0 {
                label.push_str(&format!("  {res_label} {:.2}", res[k]));
            } else {
                label.push_str(&format!("  {res_label}"));
            }
        }
    }
    label
}

/// `object_pick_label` resolving reserves + danger from the probe's current
/// sector — for pickers targeting objects here (mine, detach, inspect, deploy).
pub(crate) fn probe_object_label(state: &AppState, index: usize, id: &str, name: &str) -> String {
    let (reserves, danger) = state.probe_object_pick_info(id);
    object_pick_label(index, name, reserves, danger.as_ref())
}

/// `object_pick_label` resolving from a specific sector by coordinates — for the
/// remote-mine picker, whose asteroid lives in a SCUT-reachable sector.
pub(crate) fn sector_object_label(
    state: &AppState,
    x: i32,
    y: i32,
    z: i32,
    index: usize,
    id: &str,
    name: &str,
) -> String {
    let (reserves, danger) = state.sector_object_pick_info(x, y, z, id);
    object_pick_label(index, name, reserves, danger.as_ref())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_pick_list(
    frame: &mut Frame,
    area: Rect,
    p: Palette,
    title: &str,
    width: u16,
    height: u16,
    prompt: Option<&str>,
    items: &[&str],
    selection: usize,
    error: Option<&str>,
    action: &str,
) {
    let popup = centered_rect(width, height, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(title.to_owned())
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.accent));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::new();
    if let Some(prompt) = prompt {
        lines.push(Line::from(Span::styled(
            prompt.to_owned(),
            Style::default().fg(p.accent),
        )));
        lines.push(Line::default());
    }
    for (i, name) in items.iter().enumerate() {
        if i == selection {
            lines.push(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(p.accent)),
                Span::styled(
                    name.to_string(),
                    Style::default().fg(p.text).add_modifier(Modifier::BOLD),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(name.to_string(), Style::default().fg(p.dim)),
            ]));
        }
    }
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(p.crit),
        )));
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);
    render_footer(
        frame,
        rows[1],
        p,
        &[
            FooterKey::nav("[↑/↓]", "select"),
            FooterKey::commit("[Enter]", action),
            FooterKey::nav("[Esc]", "cancel"),
        ],
    );
}

#[cfg(test)]
mod tests {
    use super::{object_pick_label, render_active_overlays, DangerLevel};
    use crate::app::{ActiveWizard, AppState, TravelInput};
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn object_pick_label_drops_placeholder_and_appends_danger_then_reserves() {
        // Unnamed asteroid, extreme danger, deuterium + metals reserves.
        let reserves = Some(([true, true, false, false], [0.42, 1.20, 0.0, 0.0]));
        assert_eq!(
            object_pick_label(0, "unnamed", reserves, Some(&DangerLevel::Extreme)),
            "#1 ☠  deuterium 0.42  metals 1.20",
        );
        // Named target, moderate danger, present resource with unknown amount.
        let reserves = Some(([false, true, false, false], [0.0; 4]));
        assert_eq!(
            object_pick_label(1, "Big Rock", reserves, Some(&DangerLevel::Moderate)),
            "#2 Big Rock ⚠  metals",
        );
        // Non-asteroid (no reserves), no danger → bare numbered name.
        assert_eq!(
            object_pick_label(2, "dormant construct", None, None),
            "#3 dormant construct"
        );
        // Low/Unknown danger adds no glyph.
        assert_eq!(object_pick_label(0, "", None, Some(&DangerLevel::Low)), "#1");
    }

    /// Render the overlays over a blank terminal and return all cell text.
    fn rendered_text(state: &AppState) -> String {
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                render_active_overlays(f, area, state);
            })
            .unwrap();
        terminal.backend().buffer().content.iter().map(|c| c.symbol()).collect()
    }

    #[test]
    fn no_overlay_frame_when_all_wizards_inactive() {
        // Regression: several overlay render fns draw their frame (Clear + border)
        // before matching Inactive, so rendering them unconditionally painted an
        // empty modal — e.g. a stray " TRAVEL " frame after boot. The registry
        // guard prevents it.
        let text = rendered_text(&AppState::default());
        assert!(
            !text.contains("TRAVEL"),
            "no empty TRAVEL frame with an inactive wizard"
        );
        assert!(!text.contains("REMOTE MINE"), "no empty overlay frames at all");
    }

    #[test]
    fn active_wizard_frame_renders() {
        let mut state = AppState::default();
        state.active_wizard = ActiveWizard::Travel(TravelInput::Typing(String::new()));
        assert!(
            rendered_text(&state).contains("TRAVEL"),
            "active travel wizard renders its frame"
        );
    }

    #[test]
    fn fabrication_catalog_renders_both_sections() {
        use crate::app::FabricationInput;
        let mut state = AppState::default();
        state.recipes = vec![
            serde_json::from_str(r#"{"id":"integrated_circuit","name":"Integrated circuit","craftableBy":["atomic_3d_printer"],
                "ingredients":[{"type":"micro_conductor","quantity":2,"unit":"item"}],"durationSeconds":1200,
                "output":{"type":"integrated_circuit","name":"Integrated circuit","containerSpace":0.001,"containerSpaceUnit":"ECE","capacityBonus":null}}"#).unwrap(),
            serde_json::from_str(r#"{"id":"steel_plate","name":"Steel plate","craftableBy":["manny"],
                "ingredients":[],"durationSeconds":300,
                "output":{"type":"steel_plate","name":"Steel plate","containerSpace":0.01,"containerSpaceUnit":"ECE","capacityBonus":null}}"#).unwrap(),
        ];
        state.active_wizard = ActiveWizard::Fabrication(FabricationInput::PickRecipe {
            prefilled_manny: None,
            selection: 0,
            error: None,
        });
        let text = rendered_text(&state);
        assert!(text.contains("FABRICATION"), "unified title renders");
        assert!(text.contains("ATOMIC PRINTER"), "atomic section header renders");
        assert!(text.contains("MANNY FABRICATION"), "manny section header renders");
        assert!(
            text.contains("Integrated circuit") && text.contains("Steel plate"),
            "both recipes listed"
        );
        // Regression: the detail panel must always show the selected recipe's
        // ingredient breakdown (have/need), no matter how long the catalog gets.
        assert!(
            text.contains("INGREDIENTS"),
            "detail panel shows the ingredients header"
        );
        assert!(
            text.contains("micro_conductor"),
            "selected recipe's ingredient is listed"
        );
    }
}
