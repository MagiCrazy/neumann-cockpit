pub(crate) mod alerts;
pub(crate) mod containers;
pub(crate) mod craft;
pub(crate) mod drop_container;
pub(crate) mod help;
pub(crate) mod inventory_detail;
pub(crate) mod jettison;
pub(crate) mod map;
pub(crate) mod messages;
pub(crate) mod mine;
pub(crate) mod missions;
pub(crate) mod remote_mine;
pub(crate) mod object_actions;
pub(crate) mod scut_network;
pub(crate) mod pickers;
pub(crate) mod repair;
pub(crate) mod scanner;
pub(crate) mod storage_move;
pub(crate) mod travel;
pub(crate) mod waypoints;

pub(crate) use alerts::render_alerts_overlay;
pub(crate) use containers::{render_container_rules_overlay, render_rename_container_overlay};
pub(crate) use craft::{render_atomic_printer_craft_overlay, render_craft_overlay};
pub(crate) use drop_container::render_drop_container_overlay;
pub(crate) use help::render_help_overlay;
pub(crate) use inventory_detail::render_inventory_detail_overlay;
pub(crate) use jettison::render_jettison_overlay;
pub(crate) use map::{render_goto_visited_overlay, render_map_overlay};
pub(crate) use messages::render_messages_overlay;
pub(crate) use mine::render_mine_overlay;
pub(crate) use remote_mine::render_remote_mine_overlay;
pub(crate) use missions::render_missions_overlay;
pub(crate) use scut_network::render_scut_network_overlay;
pub(crate) use object_actions::render_object_action_overlay;
pub(crate) use pickers::{
    render_deploy_overlay, render_detach_overlay, render_drop_cargo_overlay, render_inspect_overlay,
    render_mind_snapshot_overlay, render_recall_overlay, render_recover_overlay,
    render_refuel_overlay, render_rename_manny_overlay, render_salvage_overlay,
    render_scut_relay_overlay,
};
pub(crate) use repair::render_repair_overlay;
pub(crate) use scanner::render_scan_input_overlay;
pub(crate) use storage_move::render_storage_move_overlay;
pub(crate) use travel::render_travel_overlay;
pub(crate) use waypoints::render_waypoints_overlay;

use crate::app::{
    AlertsInput, AppState, AtomicPrinterCraftInput, ContainerRulesInput, CraftInput, DeployInput,
    DetachInput, DropCargoInput, DropStorageContainerInput, GotoVisitedInput, InspectInput,
    JettisonInput, MessagesInput, MindSnapshotInput, MineInput, MissionsInput, ObjectActionInput,
    RecallInput, RecoverInput, RefuelInput, RemoteMineInput, RenameContainerInput, RenameMannyInput,
    RepairInput, SalvageInput, ScanMode, ScutNetworkInput, ScutRelayInput, StorageMoveInput,
    TravelInput, WaypointsInput,
};
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
const WIZARD_OVERLAYS: &[(OverlayGuard, OverlayRender)] = &[
    (|s| !matches!(s.alerts_input, AlertsInput::Inactive), render_alerts_overlay),
    (|s| !matches!(s.travel, TravelInput::Inactive), render_travel_overlay),
    (|s| !matches!(s.repair, RepairInput::Inactive), render_repair_overlay),
    (|s| !matches!(s.mine, MineInput::Inactive), render_mine_overlay),
    (|s| !matches!(s.remote_mine, RemoteMineInput::Inactive), render_remote_mine_overlay),
    (|s| !matches!(s.jettison, JettisonInput::Inactive), render_jettison_overlay),
    (|s| !matches!(s.craft, CraftInput::Inactive), render_craft_overlay),
    (|s| !matches!(s.atomic_printer_craft, AtomicPrinterCraftInput::Inactive), render_atomic_printer_craft_overlay),
    (|s| !matches!(s.salvage, SalvageInput::Inactive), render_salvage_overlay),
    (|s| !matches!(s.recall, RecallInput::Inactive), render_recall_overlay),
    (|s| !matches!(s.refuel, RefuelInput::Inactive), render_refuel_overlay),
    (|s| !matches!(s.mind_snapshot, MindSnapshotInput::Inactive), render_mind_snapshot_overlay),
    (|s| !matches!(s.missions_input, MissionsInput::Inactive), render_missions_overlay),
    (|s| !matches!(s.messages_input, MessagesInput::Inactive), render_messages_overlay),
    (|s| !matches!(s.scut_relay, ScutRelayInput::Inactive), render_scut_relay_overlay),
    (|s| !matches!(s.scut_network, ScutNetworkInput::Inactive), render_scut_network_overlay),
    (|s| !matches!(s.drop_cargo, DropCargoInput::Inactive), render_drop_cargo_overlay),
    (|s| !matches!(s.deploy, DeployInput::Inactive), render_deploy_overlay),
    (|s| !matches!(s.rename_manny, RenameMannyInput::Inactive), render_rename_manny_overlay),
    (|s| !matches!(s.inspect, InspectInput::Inactive), render_inspect_overlay),
    (|s| !matches!(s.recover, RecoverInput::Inactive), render_recover_overlay),
    (|s| !matches!(s.detach, DetachInput::Inactive), render_detach_overlay),
    (|s| !matches!(s.drop_container, DropStorageContainerInput::Inactive), render_drop_container_overlay),
    (|s| !matches!(s.object_action, ObjectActionInput::Inactive), render_object_action_overlay),
    (|s| !matches!(s.waypoints, WaypointsInput::Inactive), render_waypoints_overlay),
    (|s| !matches!(s.rename_container, RenameContainerInput::Inactive), render_rename_container_overlay),
    (|s| !matches!(s.container_rules, ContainerRulesInput::Inactive), render_container_rules_overlay),
    (|s| !matches!(s.storage_move, StorageMoveInput::Inactive), render_storage_move_overlay),
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
        render_help_overlay(frame, area, palette(state.color_mode));
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
        Self { key, label, tone: KeyTone::Nav }
    }
    pub fn commit(key: &'a str, label: &'a str) -> Self {
        Self { key, label, tone: KeyTone::Commit }
    }
    pub fn danger(key: &'a str, label: &'a str) -> Self {
        Self { key, label, tone: KeyTone::Danger }
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
        lines.push(Line::from(Span::styled(prompt.to_owned(), Style::default().fg(p.accent))));
        lines.push(Line::default());
    }
    for (i, name) in items.iter().enumerate() {
        if i == selection {
            lines.push(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(p.accent)),
                Span::styled(name.to_string(), Style::default().fg(p.text).add_modifier(Modifier::BOLD)),
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
        lines.push(Line::from(Span::styled(format!("✗ {err}"), Style::default().fg(p.crit))));
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
    use super::render_active_overlays;
    use crate::app::{AppState, TravelInput};
    use ratatui::{backend::TestBackend, Terminal};

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
        assert!(!text.contains("TRAVEL"), "no empty TRAVEL frame with an inactive wizard");
        assert!(!text.contains("REMOTE MINE"), "no empty overlay frames at all");
    }

    #[test]
    fn active_wizard_frame_renders() {
        let mut state = AppState::default();
        state.travel = TravelInput::Typing(String::new());
        assert!(rendered_text(&state).contains("TRAVEL"), "active travel wizard renders its frame");
    }
}
