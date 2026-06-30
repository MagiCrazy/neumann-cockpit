pub(crate) mod alerts;
pub(crate) mod containers;
pub(crate) mod craft;
pub(crate) mod drop_container;
pub(crate) mod help;
pub(crate) mod inventory_detail;
pub(crate) mod jettison;
pub(crate) mod map;
pub(crate) mod mine;
pub(crate) mod missions;
pub(crate) mod remote_mine;
pub(crate) mod object_actions;
pub(crate) mod scut_network;
pub(crate) mod pickers;
pub(crate) mod repair;
pub(crate) mod storage_move;
pub(crate) mod travel;
pub(crate) mod waypoints;

pub(crate) use alerts::render_alerts_overlay;
pub(crate) use containers::{
    render_container_detail_overlay, render_container_rules_overlay, render_containers_overlay,
    render_rename_container_overlay,
};
pub(crate) use craft::{render_atomic_printer_craft_overlay, render_craft_overlay};
pub(crate) use drop_container::render_drop_container_overlay;
pub(crate) use help::render_help_overlay;
pub(crate) use inventory_detail::render_inventory_detail_overlay;
pub(crate) use jettison::render_jettison_overlay;
pub(crate) use map::render_map_overlay;
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
pub(crate) use storage_move::render_storage_move_overlay;
pub(crate) use travel::render_travel_overlay;
pub(crate) use waypoints::render_waypoints_overlay;

use crate::app::{
    AlertsInput, AppState, AtomicPrinterCraftInput, ContainerRulesInput, ContainersInput,
    CraftInput, DeployInput, DetachInput, DropCargoInput, DropStorageContainerInput, InspectInput,
    JettisonInput, MineInput,
    MindSnapshotInput, MissionsInput, ObjectActionInput, RecallInput, RecoverInput, RefuelInput,
    RemoteMineInput, RenameContainerInput, RenameMannyInput, RepairInput, SalvageInput,
    ScutNetworkInput, ScutRelayInput, StorageMoveInput, TravelInput, WaypointsInput,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Render whichever wizard overlays are active, on top of the current
/// layout. Shared by the classic and retro themes.
pub(crate) fn render_active_overlays(frame: &mut Frame, area: Rect, state: &AppState) {
    // Informational overlays first (lowest in the stack); action wizards on top.
    if !matches!(state.alerts_input, AlertsInput::Inactive) {
        render_alerts_overlay(frame, area, state);
    }
    if !matches!(state.containers_input, ContainersInput::Inactive) {
        render_containers_overlay(frame, area, state);
    }
    if !matches!(state.travel, TravelInput::Inactive) {
        render_travel_overlay(frame, area, state);
    }
    if !matches!(state.repair, RepairInput::Inactive) {
        render_repair_overlay(frame, area, state);
    }
    if !matches!(state.mine, MineInput::Inactive) {
        render_mine_overlay(frame, area, state);
    }
    if !matches!(state.remote_mine, RemoteMineInput::Inactive) {
        render_remote_mine_overlay(frame, area, state);
    }
    if state.map.open {
        render_map_overlay(frame, area, state);
    }
    if !matches!(state.jettison, JettisonInput::Inactive) {
        render_jettison_overlay(frame, area, state);
    }
    if !matches!(state.craft, CraftInput::Inactive) {
        render_craft_overlay(frame, area, state);
    }
    if !matches!(state.atomic_printer_craft, AtomicPrinterCraftInput::Inactive) {
        render_atomic_printer_craft_overlay(frame, area, state);
    }
    if !matches!(state.salvage, SalvageInput::Inactive) {
        render_salvage_overlay(frame, area, state);
    }
    if !matches!(state.recall, RecallInput::Inactive) {
        render_recall_overlay(frame, area, state);
    }
    if !matches!(state.refuel, RefuelInput::Inactive) {
        render_refuel_overlay(frame, area, state);
    }
    if !matches!(state.mind_snapshot, MindSnapshotInput::Inactive) {
        render_mind_snapshot_overlay(frame, area, state);
    }
    if !matches!(state.missions_input, MissionsInput::Inactive) {
        render_missions_overlay(frame, area, state);
    }
    if !matches!(state.scut_relay, ScutRelayInput::Inactive) {
        render_scut_relay_overlay(frame, area, state);
    }
    if !matches!(state.scut_network, ScutNetworkInput::Inactive) {
        render_scut_network_overlay(frame, area, state);
    }
    if !matches!(state.drop_cargo, DropCargoInput::Inactive) {
        render_drop_cargo_overlay(frame, area, state);
    }
    if !matches!(state.deploy, DeployInput::Inactive) {
        render_deploy_overlay(frame, area, state);
    }
    if !matches!(state.rename_manny, RenameMannyInput::Inactive) {
        render_rename_manny_overlay(frame, area, state);
    }
    if !matches!(state.inspect, InspectInput::Inactive) {
        render_inspect_overlay(frame, area, state);
    }
    if !matches!(state.recover, RecoverInput::Inactive) {
        render_recover_overlay(frame, area, state);
    }
    if !matches!(state.detach, DetachInput::Inactive) {
        render_detach_overlay(frame, area, state);
    }
    if !matches!(state.drop_container, DropStorageContainerInput::Inactive) {
        render_drop_container_overlay(frame, area, state);
    }
    if !matches!(state.object_action, ObjectActionInput::Inactive) {
        render_object_action_overlay(frame, area, state);
    }
    if !matches!(state.waypoints, WaypointsInput::Inactive) {
        render_waypoints_overlay(frame, area, state);
    }
    if state.inventory_detail_open {
        render_inventory_detail_overlay(frame, area, state);
    }
    // Container action wizards + read-only detail sit above the list.
    if !matches!(state.rename_container, RenameContainerInput::Inactive) {
        render_rename_container_overlay(frame, area, state);
    }
    if !matches!(state.container_rules, ContainerRulesInput::Inactive) {
        render_container_rules_overlay(frame, area, state);
    }
    if state.storage_container_detail.is_some() {
        render_container_detail_overlay(frame, area, state);
    }
    if !matches!(state.storage_move, StorageMoveInput::Inactive) {
        render_storage_move_overlay(frame, area, state);
    }
    if state.help_open {
        render_help_overlay(frame, area);
    }
}

pub(crate) fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(r.width), height.min(r.height))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_pick_list(
    frame: &mut Frame,
    area: Rect,
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
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let mut lines: Vec<Line> = Vec::new();
    if let Some(p) = prompt {
        lines.push(Line::from(Span::styled(p.to_owned(), Style::default().fg(Color::Cyan))));
        lines.push(Line::default());
    }
    for (i, name) in items.iter().enumerate() {
        if i == selection {
            lines.push(Line::from(vec![
                Span::styled("▶ ", Style::default().fg(Color::Yellow)),
                Span::styled(name.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(name.to_string(), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }
    if let Some(err) = error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("✗ {err}"),
            Style::default().fg(Color::Red),
        )));
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[↑/↓]", Style::default().fg(Color::Cyan)),
            Span::raw(" select  "),
            Span::styled("[Enter]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(format!(" {action}  ")),
            Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ])),
        rows[1],
    );
}

