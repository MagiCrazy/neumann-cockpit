use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::fetch_sector;
use crate::app::{
    AlertsInput, ApiMessage, AppState, AtomicPrinterCraftInput, ContainerRulesInput,
    CraftInput, DeployInput, DetachInput, DropCargoInput,
    DropStorageContainerInput, InspectInput, JettisonInput, MindSnapshotInput, MineInput,
    MessagesInput, MissionsInput, ObjectActionInput, RecallInput, RecoverInput, RefuelInput,
    RemoteMineInput, RenameContainerInput, RenameMannyInput, RepairInput, SalvageInput, ScanMode,
    ScutNetworkInput, ScutRelayInput, StorageMoveInput, TravelInput, WaypointsInput,
};
mod alerts;
mod cockpit;
mod containers;
mod craft;
mod geometry;
mod jettison;
mod map;
mod messages;
mod mine;
mod missions;
mod pickers;
mod repair;
mod scanner;
mod storage_move;
mod travel;

use alerts::handle_alerts_event;
use cockpit::handle_cockpit_event;
use containers::{
    handle_container_rules_event, handle_rename_container_event,
};
use craft::{handle_atomic_printer_craft_event, handle_craft_event};
use geometry::face_d2;
use jettison::handle_jettison_event;
use map::handle_map_event;
use messages::handle_messages_event;
use mine::{handle_mine_event, handle_remote_mine_event};
use missions::handle_missions_event;
use pickers::{
    handle_deploy_event, handle_detach_event, handle_drop_cargo_event,
    handle_drop_container_event, handle_inspect_event, handle_mind_snapshot_event,
    handle_recall_event, handle_recover_event, handle_refuel_event, handle_rename_manny_event,
    handle_salvage_event,
};
use repair::handle_repair_event;
use scanner::{
    handle_object_action_event, handle_scut_network_event, handle_scut_relay_event,
    handle_waypoints_event,
};
use storage_move::handle_storage_move_event;
use travel::handle_travel_event;
pub fn handle_event(
    event: Event,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    let Event::Key(k) = event else { return };
    if k.kind != KeyEventKind::Press { return };
    // Toasts and inline errors are transient: any keypress dismisses them.
    state.toast = None;
    state.error = None;
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let in_scan_input = matches!(state.scan_mode, ScanMode::Input(_));
    let in_direction_pick = matches!(state.scan_mode, ScanMode::DirectionPick);
    let in_travel = !matches!(state.travel, TravelInput::Inactive);
    let in_repair = !matches!(state.repair, RepairInput::Inactive);
    let in_jettison = !matches!(state.jettison, JettisonInput::Inactive);
    let in_craft = !matches!(state.craft, CraftInput::Inactive);
    let in_atomic_craft = !matches!(state.atomic_printer_craft, AtomicPrinterCraftInput::Inactive);
    let in_salvage = !matches!(state.salvage, SalvageInput::Inactive);
    let in_recall = !matches!(state.recall, RecallInput::Inactive);
    let in_refuel = !matches!(state.refuel, RefuelInput::Inactive);
    let in_mind_snapshot = !matches!(state.mind_snapshot, MindSnapshotInput::Inactive);
    let in_remote_mine = !matches!(state.remote_mine, RemoteMineInput::Inactive);
    let in_scut_relay = !matches!(state.scut_relay, ScutRelayInput::Inactive);
    let in_scut_network = !matches!(state.scut_network, ScutNetworkInput::Inactive);
    let in_missions = !matches!(state.missions_input, MissionsInput::Inactive);
    let in_messages = !matches!(state.messages_input, MessagesInput::Inactive);
    let in_rename_manny = !matches!(state.rename_manny, RenameMannyInput::Inactive);
    let in_deploy = !matches!(state.deploy, DeployInput::Inactive);
    let in_inspect = !matches!(state.inspect, InspectInput::Inactive);
    let in_recover = !matches!(state.recover, RecoverInput::Inactive);
    let in_detach = !matches!(state.detach, DetachInput::Inactive);
    let in_alerts = !matches!(state.alerts_input, AlertsInput::Inactive);
    let in_rename_container = !matches!(state.rename_container, RenameContainerInput::Inactive);
    let in_container_rules = !matches!(state.container_rules, ContainerRulesInput::Inactive);
    let in_storage_move = !matches!(state.storage_move, StorageMoveInput::Inactive);
    let in_drop_cargo = !matches!(state.drop_cargo, DropCargoInput::Inactive);
    let in_drop_container = !matches!(state.drop_container, DropStorageContainerInput::Inactive);

    if ctrl && k.code == KeyCode::Char('c') {
        state.set_quit();
        return;
    }

    // Any key skips the boot assembly and drops into the live cockpit.
    if state.booting {
        state.skip_boot();
        return;
    }

    if k.code == KeyCode::F(2) {
        // F2 cycles the cockpit color mode.
        state.color_mode = state.color_mode.cycle();
        state.set_toast(format!("color mode: {}", state.color_mode.label()));
        return;
    }

    if state.help_open {
        if matches!(k.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')) {
            state.help_open = false;
        }
        return;
    }

    if state.inventory_detail_open {
        if matches!(k.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')) {
            state.inventory_detail_open = false;
        }
        return;
    }

    if state.map.open {
        handle_map_event(k.code, state);
        return;
    }

    if in_jettison {
        handle_jettison_event(k.code, state, client, tx);
        return;
    }

    if in_craft {
        handle_craft_event(k.code, state, client, tx);
        return;
    }

    if in_atomic_craft {
        handle_atomic_printer_craft_event(k.code, state, client, tx);
        return;
    }

    if in_salvage {
        handle_salvage_event(k.code, state, client, tx);
        return;
    }

    if in_recall {
        handle_recall_event(k.code, state, client, tx);
        return;
    }

    if in_refuel {
        handle_refuel_event(k.code, state, client, tx);
        return;
    }

    if in_mind_snapshot {
        handle_mind_snapshot_event(k.code, state, client, tx);
        return;
    }

    if in_scut_relay {
        handle_scut_relay_event(k.code, state, client, tx);
        return;
    }

    if in_scut_network {
        handle_scut_network_event(k.code, state, client, tx);
        return;
    }

    if in_missions {
        handle_missions_event(k.code, state, client, tx);
        return;
    }

    if in_messages {
        handle_messages_event(k.code, state, client, tx);
        return;
    }

    if in_rename_manny {
        handle_rename_manny_event(k.code, state, client, tx);
        return;
    }

    if in_deploy {
        handle_deploy_event(k.code, state, client, tx);
        return;
    }

    if in_inspect {
        handle_inspect_event(k.code, state, client, tx);
        return;
    }

    if in_recover {
        handle_recover_event(k.code, state, client, tx);
        return;
    }

    if in_detach {
        handle_detach_event(k.code, state, client, tx);
        return;
    }

    if in_alerts {
        handle_alerts_event(k.code, state, client, tx);
        return;
    }

    if in_rename_container {
        handle_rename_container_event(k.code, state, client, tx);
        return;
    }

    if in_container_rules {
        handle_container_rules_event(k.code, state, client, tx);
        return;
    }

    if in_storage_move {
        handle_storage_move_event(k.code, state, client, tx);
        return;
    }

    if in_drop_cargo {
        handle_drop_cargo_event(k.code, state, client, tx);
        return;
    }

    if in_drop_container {
        handle_drop_container_event(k.code, state, client, tx);
        return;
    }

    if !matches!(state.object_action, ObjectActionInput::Inactive) {
        handle_object_action_event(k.code, state, client, tx);
        return;
    }

    if !matches!(state.waypoints, WaypointsInput::Inactive) {
        handle_waypoints_event(k.code, state);
        return;
    }

    if in_travel {
        handle_travel_event(k.code, state, client, tx);
        return;
    }

    if in_repair {
        handle_repair_event(k.code, state, client, tx);
        return;
    }

    let in_mine = !matches!(state.mine, MineInput::Inactive);
    if in_mine {
        handle_mine_event(k.code, state, client, tx);
        return;
    }

    if in_remote_mine {
        handle_remote_mine_event(k.code, state, client, tx);
        return;
    }

    if in_direction_pick {
        match k.code {
            KeyCode::Esc => state.scan_mode = ScanMode::Current,
            KeyCode::Char(axis @ ('x' | 'y' | 'z')) => {
                if let Some(pos) = state.probe_sector_coords() {
                    let offsets = face_d2(axis as u8);
                    let n = offsets.len();
                    state.start_batch(n);
                    state.scan_mode = ScanMode::Current;
                    for (dx, dy, dz) in offsets {
                        fetch_sector(Some((pos.0 + dx, pos.1 + dy, pos.2 + dz)), client.clone(), tx.clone());
                    }
                }
            }
            _ => {}
        }
        return;
    }

    if in_scan_input {
        match k.code {
            KeyCode::Esc => state.scan_mode = ScanMode::Current,
            KeyCode::Backspace => state.scan_backspace(),
            KeyCode::Enter => {
                if let Some(coords) = state.parse_scan_coords() {
                    state.scan_loading = true;
                    state.scan_error = None;
                    fetch_sector(Some(coords), client.clone(), tx.clone());
                }
            }
            KeyCode::Char(c) => state.scan_type_char(c),
            _ => {}
        }
        return;
    }

    // Cockpit navigation + contextual-menu dispatch. Runs after the shared
    // wizard/overlay handlers above, so an open wizard keeps receiving keys.
    handle_cockpit_event(k.code, state, client, tx);
}
