use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::fetch_sector;
use crate::app::{
    ActiveWizard, ApiMessage, AppState, ScanMode,
    GotoVisitedInput, InputMode, ProbeSwitchInput,
};
mod alerts;
mod assemble;
mod cockpit;
mod command;
mod containers;
mod craft;
mod fleet;
mod geometry;
mod improve;
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
use assemble::handle_assemble_probe_event;
use cockpit::handle_cockpit_event;
use containers::{
    handle_container_rules_event, handle_rename_container_event,
};
use craft::handle_fabrication_event;
use fleet::{
    handle_probe_switch_event, handle_rename_probe_event, handle_transfer_deuterium_event,
};
use geometry::face_d2;
use improve::handle_improve_event;
use jettison::handle_jettison_event;
use command::handle_command_event;
use map::{handle_goto_visited_event, handle_map_event};
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

type WizardGuard = fn(&AppState) -> bool;
type WizardHandler = fn(KeyCode, &mut AppState, &ApiClient, &mpsc::Sender<ApiMessage>);

/// The wizards, in input-precedence order: the first whose guard matches
/// consumes the key. This is the single source of truth for key routing —
/// adding a wizard means adding one line here, instead of a hoisted `in_*`
/// bool plus a block in the old hand-ordered if-cascade. Handlers all take the
/// uniform `(KeyCode, &mut AppState, &ApiClient, &Sender)` shape; `waypoints`
/// (which ignores client/tx) is wrapped to match.
#[allow(clippy::type_complexity)]
const WIZARD_INPUTS: &[(WizardGuard, WizardHandler)] = &[
    (|s| matches!(s.active_wizard, ActiveWizard::AssembleProbe(_)), handle_assemble_probe_event),
    (|s| matches!(s.active_wizard, ActiveWizard::RenameProbe(_)), handle_rename_probe_event),
    (|s| matches!(s.active_wizard, ActiveWizard::Jettison(_)), handle_jettison_event),
    (|s| matches!(s.active_wizard, ActiveWizard::Fabrication(_)), handle_fabrication_event),
    (|s| matches!(s.active_wizard, ActiveWizard::Improve(_)), handle_improve_event),
    (|s| matches!(s.active_wizard, ActiveWizard::Salvage(_)), handle_salvage_event),
    (|s| matches!(s.active_wizard, ActiveWizard::Recall(_)), handle_recall_event),
    (|s| matches!(s.active_wizard, ActiveWizard::Refuel(_)), handle_refuel_event),
    (|s| matches!(s.active_wizard, ActiveWizard::TransferDeuterium(_)), handle_transfer_deuterium_event),
    (|s| matches!(s.active_wizard, ActiveWizard::MindSnapshot(_)), handle_mind_snapshot_event),
    (|s| matches!(s.active_wizard, ActiveWizard::ScutRelay(_)), handle_scut_relay_event),
    (|s| matches!(s.active_wizard, ActiveWizard::ScutNetwork(_)), handle_scut_network_event),
    (|s| matches!(s.active_wizard, ActiveWizard::Missions(_)), handle_missions_event),
    (|s| matches!(s.active_wizard, ActiveWizard::Messages(_)), handle_messages_event),
    (|s| matches!(s.active_wizard, ActiveWizard::RenameManny(_)), handle_rename_manny_event),
    (|s| matches!(s.active_wizard, ActiveWizard::Deploy(_)), handle_deploy_event),
    (|s| matches!(s.active_wizard, ActiveWizard::Inspect(_)), handle_inspect_event),
    (|s| matches!(s.active_wizard, ActiveWizard::Recover(_)), handle_recover_event),
    (|s| matches!(s.active_wizard, ActiveWizard::Detach(_)), handle_detach_event),
    (|s| matches!(s.active_wizard, ActiveWizard::Alerts(_)), handle_alerts_event),
    (|s| matches!(s.active_wizard, ActiveWizard::RenameContainer(_)), handle_rename_container_event),
    (|s| matches!(s.active_wizard, ActiveWizard::ContainerRules(_)), handle_container_rules_event),
    (|s| matches!(s.active_wizard, ActiveWizard::StorageMove(_)), handle_storage_move_event),
    (|s| matches!(s.active_wizard, ActiveWizard::DropCargo(_)), handle_drop_cargo_event),
    (|s| matches!(s.active_wizard, ActiveWizard::DropContainer(_)), handle_drop_container_event),
    (|s| matches!(s.active_wizard, ActiveWizard::ObjectAction(_)), handle_object_action_event),
    (|s| matches!(s.active_wizard, ActiveWizard::Waypoints(_)), |c, s, _, _| handle_waypoints_event(c, s)),
    (|s| matches!(s.active_wizard, ActiveWizard::Travel(_)), handle_travel_event),
    (|s| matches!(s.active_wizard, ActiveWizard::Repair(_)), handle_repair_event),
    (|s| matches!(s.active_wizard, ActiveWizard::Mine(_)), handle_mine_event),
    (|s| matches!(s.active_wizard, ActiveWizard::RemoteMine(_)), handle_remote_mine_event),
];

/// Route a key to the first active wizard. Returns `true` if one consumed it.
fn dispatch_wizard_key(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) -> bool {
    for (active, handle) in WIZARD_INPUTS {
        if active(state) {
            handle(code, state, client, tx);
            return true;
        }
    }
    false
}

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
        // Clamp against the same body height the renderer uses (near-fullscreen
        // minus a 2-row margin, 2 borders and the footer).
        let (_, term_h) = crossterm::terminal::size().unwrap_or((80, 24));
        let max = (crate::ui::overlays::help_row_count() as u16)
            .saturating_sub(term_h.saturating_sub(5));
        match k.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                state.help_open = false;
                state.help_scroll = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                state.help_scroll = state.help_scroll.saturating_add(1).min(max);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                state.help_scroll = state.help_scroll.saturating_sub(1);
            }
            KeyCode::PageDown | KeyCode::Char(' ') => {
                state.help_scroll = state.help_scroll.saturating_add(10).min(max);
            }
            KeyCode::PageUp => state.help_scroll = state.help_scroll.saturating_sub(10),
            KeyCode::Char('g') | KeyCode::Home => state.help_scroll = 0,
            KeyCode::Char('G') | KeyCode::End => state.help_scroll = max,
            _ => {}
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

    if matches!(state.goto_visited, GotoVisitedInput::Picking { .. }) {
        handle_goto_visited_event(k.code, state);
        return;
    }

    if matches!(state.probe_switch, ProbeSwitchInput::Picking { .. }) {
        handle_probe_switch_event(k.code, state);
        return;
    }

    if matches!(state.mode, InputMode::Command(_)) {
        handle_command_event(k.code, state, client, tx);
        return;
    }

    // A single wizard consumes the key if one is active (registry below).
    if dispatch_wizard_key(k.code, state, client, tx) {
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

#[cfg(test)]
mod tests {
    //! Characterization tests locking the current key-dispatch precedence:
    //! an open wizard captures keys before the cockpit grid, and Esc closes it.
    //! These pin behavior ahead of the wizard-registry refactor.
    use super::*;
    use crate::app::{AlertsInput, Pane, TravelInput};
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    fn dummy_client() -> ApiClient {
        ApiClient::new("http://localhost:0".into(), "test-key".into()).unwrap()
    }

    /// Feed one key press through the real `handle_event`.
    fn press(state: &mut AppState, code: KeyCode) {
        let client = dummy_client();
        let (tx, _rx) = mpsc::channel(32);
        handle_event(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)), state, &client, &tx);
    }

    #[tokio::test]
    async fn cockpit_pane_key_switches_pane_without_a_wizard() {
        let mut state = AppState::default();
        assert_eq!(state.active_pane, Pane::Probe, "default centre pane");
        press(&mut state, KeyCode::Char('e'));
        assert_eq!(state.active_pane, Pane::Scanner, "no wizard → cockpit handles the pane key");
    }

    #[tokio::test]
    async fn cockpit_pane_key_works_with_capslock() {
        // CapsLock (or Shift) sends an uppercase letter; cockpit keys must still work.
        let mut state = AppState::default();
        press(&mut state, KeyCode::Char('E'));
        assert_eq!(state.active_pane, Pane::Scanner, "uppercase 'E' routes like 'e'");
    }

    #[tokio::test]
    async fn menu_digit_accelerator_fires_nth_item() {
        use crate::app::{ContextMenu, InputMode, MenuAction, MenuItem};
        let mut state = AppState::default();
        state.mode = InputMode::Menu(ContextMenu {
            title: "TEST".into(),
            items: vec![MenuItem {
                action: MenuAction::Travel,
                label: "Travel…".into(),
                enabled: true,
                disabled_reason: None,
            }],
            cursor: 0,
        });
        press(&mut state, KeyCode::Char('1'));
        assert!(matches!(state.mode, InputMode::Normal), "digit fired and closed the menu");
        assert!(matches!(state.active_wizard, ActiveWizard::Travel(_)), "the item's wizard launched");
    }

    #[tokio::test]
    async fn open_wizard_captures_keys_before_cockpit() {
        let mut state = AppState::default();
        state.active_wizard = ActiveWizard::Travel(TravelInput::Typing(String::new()));
        press(&mut state, KeyCode::Char('e'));
        assert_eq!(state.active_pane, Pane::Probe, "wizard swallows the key; cockpit must not switch pane");
        assert!(matches!(state.active_wizard, ActiveWizard::Travel(_)), "the wizard stays open");
    }

    #[tokio::test]
    async fn esc_closes_the_active_wizard() {
        let mut state = AppState::default();
        state.active_wizard = ActiveWizard::Travel(TravelInput::Typing("12".into()));
        press(&mut state, KeyCode::Esc);
        assert!(matches!(state.active_wizard, ActiveWizard::None), "Esc closes the travel wizard");
    }

    #[tokio::test]
    async fn opening_a_second_wizard_replaces_the_first() {
        // The structural guarantee: `active_wizard` is a single field, so a new
        // wizard cannot coexist with a previous one — it replaces it. No two
        // wizards can be open at once (that state is now unrepresentable).
        let mut state = AppState::default();
        state.active_wizard = ActiveWizard::Travel(TravelInput::Typing(String::new()));
        state.active_wizard = ActiveWizard::Alerts(AlertsInput::Browsing { selection: 0, show_warnings: false });
        assert!(matches!(state.active_wizard, ActiveWizard::Alerts(_)), "the second wizard is now active");
        assert!(!matches!(state.active_wizard, ActiveWizard::Travel(_)), "the first was replaced, not kept alongside");
    }

    #[tokio::test]
    async fn tabbed_wizard_also_captures_keys_and_closes_on_esc() {
        let mut state = AppState::default();
        state.active_wizard = ActiveWizard::Alerts(AlertsInput::Browsing { selection: 0, show_warnings: false });
        // A pane key must not reach the cockpit while the alerts overlay is open.
        press(&mut state, KeyCode::Char('b'));
        assert_eq!(state.active_pane, Pane::Probe, "alerts overlay swallows the pane key");
        assert!(matches!(state.active_wizard, ActiveWizard::Alerts(_)));
        press(&mut state, KeyCode::Esc);
        assert!(matches!(state.active_wizard, ActiveWizard::None), "Esc closes the alerts overlay");
    }

    #[tokio::test]
    async fn pick_list_j_moves_selection_and_esc_closes() {
        use crate::app::SalvageInput;
        let mut state = AppState::default();
        state.active_wizard = ActiveWizard::Salvage(SalvageInput::PickTarget {
            manny_id: "m1".into(),
            manny_name: "M".into(),
            candidates: vec![("a".into(), "A".into()), ("b".into(), "B".into())],
            selection: 0,
        });
        press(&mut state, KeyCode::Char('j'));
        match &state.active_wizard {
            ActiveWizard::Salvage(SalvageInput::PickTarget { selection, .. }) => assert_eq!(*selection, 1, "j moves the cursor"),
            _ => panic!("should still be picking"),
        }
        press(&mut state, KeyCode::Esc);
        assert!(matches!(state.active_wizard, ActiveWizard::None), "Esc closes the picker");
    }

    fn idle_onboard_manny(id: &str) -> crate::api::types::Manny {
        serde_json::from_str(&format!(r#"{{
            "id": "{id}", "name": "{id}",
            "location": {{"type": "probe", "sector": null}},
            "currentTask": null, "taskProgressPercent": 0.0,
            "cargo": {{"capacity": 0.3, "deuterium": 0.0, "metals": 0.0, "ice": 0.0, "organicCompounds": 0.0}},
            "canReceiveOrders": true, "taskEstimatedEndTime": null
        }}"#)).unwrap()
    }

    fn manny_recipe(id: &str) -> crate::api::types::CraftingRecipe {
        serde_json::from_str(&format!(r#"{{"id":"{id}","name":"{id}","craftableBy":["manny"],
            "ingredients":[],"durationSeconds":60,
            "output":{{"type":"x","name":"X","containerSpace":1.0,"containerSpaceUnit":"ECE","capacityBonus":null}}}}"#)).unwrap()
    }

    #[tokio::test]
    async fn fabricate_manny_recipe_advances_to_builder_pick_when_several_idle() {
        use crate::app::FabricationInput;
        let mut state = AppState::default();
        state.recipes = vec![manny_recipe("solar_panel")];
        state.mannies = Some(vec![idle_onboard_manny("m1"), idle_onboard_manny("m2")]);
        state.active_wizard = ActiveWizard::Fabrication(FabricationInput::PickRecipe { prefilled_manny: None, selection: 0, error: None });
        press(&mut state, KeyCode::Enter);
        match &state.active_wizard {
            ActiveWizard::Fabrication(FabricationInput::PickBuilder { recipe_id, mannies, .. }) => {
                assert_eq!(recipe_id, "solar_panel");
                assert_eq!(mannies.len(), 2, "both idle mannies offered as builders");
            }
            _ => panic!("manny recipe with 2 idle mannies must open the builder picker"),
        }
    }

    #[tokio::test]
    async fn fabricate_manny_recipe_errors_when_no_idle_manny() {
        use crate::app::FabricationInput;
        let mut state = AppState::default();
        state.recipes = vec![manny_recipe("solar_panel")];
        state.mannies = Some(vec![]);
        state.active_wizard = ActiveWizard::Fabrication(FabricationInput::PickRecipe { prefilled_manny: None, selection: 0, error: None });
        press(&mut state, KeyCode::Enter);
        match &state.active_wizard {
            ActiveWizard::Fabrication(FabricationInput::PickRecipe { error, .. }) => {
                assert!(error.as_deref().unwrap_or("").contains("no idle Manny"), "surfaces the no-manny error");
            }
            _ => panic!("stays on the recipe step with an error"),
        }
    }

    #[tokio::test]
    async fn scut_inspect_from_probe_menu_opens_picker() {
        use crate::app::{Pane, ScutNetworkInput};
        let mut state = AppState::default();
        state.active_pane = Pane::Probe;
        // Two SCUT networks cover the current sector (first scan, no probe sector).
        state.scan_history = vec![serde_json::from_str(r#"{
            "relativeCoordinates":{"x":0,"y":0,"z":0},"distance":0,
            "knowledgeLevel":"detailed","confidence":1.0,
            "scutNetworks":[{"id":1,"name":"Alpha"},{"id":2,"name":"Beta"}],
            "scan":{"currentSectorResidenceSeconds":60,"requiredResidenceSeconds":60,"scanQuality":1.0}
        }"#).unwrap()];
        press(&mut state, KeyCode::Enter); // open the Probe context menu
        press(&mut state, KeyCode::Enter); // fire the first enabled item (Inspect SCUT network…)
        match &state.active_wizard {
            ActiveWizard::ScutNetwork(ScutNetworkInput::Picking { networks, .. }) => assert_eq!(networks.len(), 2, "both networks offered"),
            _ => panic!("two networks should open the picker"),
        }
    }

    #[tokio::test]
    async fn scut_inspect_single_network_goes_straight_to_view() {
        use crate::app::{Pane, ScutNetworkInput};
        let mut state = AppState::default();
        state.active_pane = Pane::Probe;
        state.scan_history = vec![serde_json::from_str(r#"{
            "relativeCoordinates":{"x":0,"y":0,"z":0},"distance":0,
            "knowledgeLevel":"detailed","confidence":1.0,
            "scutNetworks":[{"id":7,"name":"Solo"}],
            "scan":{"currentSectorResidenceSeconds":60,"requiredResidenceSeconds":60,"scanQuality":1.0}
        }"#).unwrap()];
        press(&mut state, KeyCode::Enter);
        press(&mut state, KeyCode::Enter);
        assert!(matches!(state.active_wizard, ActiveWizard::ScutNetwork(ScutNetworkInput::Viewing { .. })), "a single network views directly");
    }

    #[tokio::test]
    async fn comms_root_enters_alerts_category_in_pane() {
        use crate::app::{CommsCategory, Pane};
        let mut state = AppState::default();
        state.active_pane = Pane::Comms;
        state.pane_nav[Pane::Comms.index()].cursor = 1; // Alerts row
        press(&mut state, KeyCode::Enter);
        assert_eq!(state.comms_drill(), Some(CommsCategory::Alerts), "drills into the Alerts list in-pane");
        // `h` backs out to the category root.
        press(&mut state, KeyCode::Char('h'));
        assert_eq!(state.comms_drill(), None, "back to the Comms root");
    }

    #[tokio::test]
    async fn comms_root_messages_opens_the_overlay() {
        use crate::app::{MessagesInput, Pane};
        let mut state = AppState::default();
        state.active_pane = Pane::Comms;
        state.pane_nav[Pane::Comms.index()].cursor = 0; // Messages row
        press(&mut state, KeyCode::Enter);
        assert!(matches!(state.active_wizard, ActiveWizard::Messages(MessagesInput::Browsing { .. })), "Messages opens its overlay");
    }

    #[tokio::test]
    async fn improve_from_probe_menu_opens_the_picker() {
        use crate::app::{ImproveInput, Pane};
        let mut state = AppState::default();
        state.active_pane = Pane::Probe;
        state.probe_improvements = vec![serde_json::from_str(
            r#"{"id":"deuterium_compression","name":"Deuterium compression","description":"d",
                "available":true,"done":false,"durationSeconds":300,"ingredients":[],"effects":null}"#,
        ).unwrap()];
        press(&mut state, KeyCode::Enter); // open the Probe menu (Improve is the first enabled item)
        press(&mut state, KeyCode::Enter); // fire it
        assert!(
            matches!(state.active_wizard, ActiveWizard::Improve(ImproveInput::PickImprovement { .. })),
            "an orderable improvement opens the picker"
        );
    }
}
