//! Input routing for the unified Cockpit v2 interface (blocs U2–U5).
//!
//! Normal-mode navigation: `ertdfgcvb` activates a pane, `jk`/arrows move the
//! cursor within it, `l`/`h` drill in/out, `z` zooms, `Tab` cycles panes,
//! `F1` toggles the hints line, `F5` refreshes. `Enter` opens the contextual
//! action menu, which launches the existing wizards. Command mode (`:`) lands
//! in a later bloc; unhandled keys are ignored.

use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use super::geometry::{is_list_nav_key, list_move};
use crate::api::client::ApiClient;
use crate::api::tasks::{
    fetch_ack_alert, fetch_ack_damage_warning, fetch_alerts, fetch_all, fetch_damage_warnings, fetch_inspect,
    fetch_logbook_page, fetch_logbook_pages, fetch_messages, fetch_reassign_reservations, fetch_recover,
    fetch_scut_network, fetch_sector, fetch_sent_messages, fetch_set_default_probe, fetch_storage_container_detail,
};
use crate::api::types::{MannyTask, MannyTaskVisibility};
use crate::app::{
    ActiveWizard, ApiMessage, AppState, AssembleProbeInput, CommandLine, CommsCategory, DeployInput, DetachInput,
    DrillLevel, DropCargoInput, DropStorageContainerInput, FabricationInput, GotoVisitedInput, ImproveInput, InputMode,
    InspectInput, LogEvent, LogbookInput, MenuAction, MessagesInput, MindSnapshotInput, MineInput, MissionsCategory,
    MissionsInput, ObjectActionInput, Pane, ProbeSwitchInput, RecallInput, RecoverInput, RefuelInput, RemoteMineInput,
    RenameContainerInput, RenameMannyInput, RenameProbeInput, RepairInput, SalvageInput, ScanMode, ScannerFocus,
    ScutCorridorInput, ScutNetworkInput, ShareBlueprintInput, StorageMoveInput, TransferDeuteriumInput,
    TransferProbeInput, TravelInput, WaypointsInput, LIST_PAGE,
};

pub fn handle_cockpit_event(code: KeyCode, state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    // Cockpit keys (ertdfgcvb, jkhl, z, q, …) are all lowercase, but CapsLock —
    // or Shift — sends uppercase letters, and no cockpit binding uses those.
    // Normalize so the grid stays navigable regardless of CapsLock. Text entry
    // is handled by the wizard/command layers before this, so it is unaffected.
    let code = match code {
        KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
        other => other,
    };

    // A context menu, when open, captures all input.
    if matches!(state.mode, InputMode::Menu(_)) {
        handle_menu_key(code, state, client, tx);
        return;
    }

    match code {
        KeyCode::Char('q') => state.set_quit(),
        KeyCode::Enter => open_actions(state, client, tx),
        KeyCode::Char(c) if Pane::from_key(c).is_some() => {
            state.active_pane = Pane::from_key(c).unwrap();
        }
        // A detail view has no cursor to move: the same keys scroll it
        // (issue #337), so this must come before the list routing.
        _ if state.detail_view_active() && scroll_detail(code, state) => {}
        // The Scanner is two scrollable columns, so the focus decides which
        // one the shared nav keys drive (issue #347).
        _ if state.active_pane == Pane::Scanner
            && state.scanner_focus == ScannerFocus::Detail
            && scroll_scan_detail(code, state) => {}
        KeyCode::Down | KeyCode::Char('j') => state.pane_cursor_down(),
        KeyCode::Up | KeyCode::Char('k') => state.pane_cursor_up(),
        // Paging + jump to ends, for lists that grow over a session (scan
        // history, messages). `g`/`G` are pane keys, so use PageUp/Down/Home/End.
        KeyCode::PageDown => state.pane_cursor_page_down(),
        KeyCode::PageUp => state.pane_cursor_page_up(),
        KeyCode::Home => state.pane_cursor_top(),
        KeyCode::End => state.pane_cursor_bottom(),
        // On the Scanner these move between the two columns rather than
        // drilling: the detail is the left column, the history the right one,
        // so the keys point where the eye does (issue #347).
        KeyCode::Right | KeyCode::Char('l') if state.active_pane == Pane::Scanner => {
            state.scanner_focus = ScannerFocus::History;
        }
        KeyCode::Left | KeyCode::Char('h') if state.active_pane == Pane::Scanner => {
            state.scanner_focus = ScannerFocus::Detail;
        }
        KeyCode::Tab if state.active_pane == Pane::Scanner => {
            state.scanner_focus = match state.scanner_focus {
                ScannerFocus::History => ScannerFocus::Detail,
                ScannerFocus::Detail => ScannerFocus::History,
            };
        }
        KeyCode::Right | KeyCode::Char('l') => drill_in(state, client, tx),
        KeyCode::Left | KeyCode::Char('h') => {
            drill_out(state);
        }
        // The Map pane has no in-pane zoom view — `z` opens the full isometric
        // map overlay (its own pan/travel controls take over).
        KeyCode::Char('z') if state.active_pane == Pane::Map => state.open_map(),
        KeyCode::Char('z') => state.toggle_zoom(),
        KeyCode::Char(':') => state.mode = InputMode::Command(CommandLine::default()),
        KeyCode::Char('?') => state.help_open = true,
        // Jump to the next idle Manny (focuses the Mannies pane).
        KeyCode::Char('i') => state.cycle_to_next_idle_manny(),
        // Logbook writing keys (issue #254). Scoped to the Logbook half of the
        // Log pane, so `c` and `x` keep their cockpit meanings everywhere else.
        KeyCode::Char('c') | KeyCode::Char('e') | KeyCode::Char('x')
            if state.active_pane == Pane::Missions && state.missions_category() == Some(MissionsCategory::Logbook) =>
        {
            logbook_key(code, state);
        }
        // Storage: toggle server order ↔ alphabetical (issue #333).
        KeyCode::Char('s') if state.active_pane == Pane::Storage => {
            state.storage_toggle_sort();
            let how = if state.storage_sort_alpha { "a-z" } else { "probe order" };
            state.set_toast(format!("containers sorted: {how}"));
        }
        KeyCode::F(1) => {
            state.hints_visible = !state.hints_visible;
            state.stage_settings_save();
        }
        // Esc backs out one step: leave zoom first, then drill up.
        KeyCode::Esc => {
            if state.zoomed {
                state.zoomed = false;
            } else {
                drill_out(state);
            }
        }
        KeyCode::Tab => state.cycle_pane(true),
        KeyCode::BackTab => state.cycle_pane(false),
        KeyCode::F(5) if !state.loading => {
            state.clear_error();
            state.loading = true;
            fetch_all(client.clone(), tx.clone());
        }
        _ => {}
    }
}

/// Scroll the active pane's detail viewport (issue #337). Returns `true` when
/// the key was consumed.
///
/// It answers the shared navigation keys (#325) but deliberately does **not**
/// wrap: that contract is about a cursor walking a list, and a viewport that
/// snapped back to the top on the keypress after the last line would read as a
/// glitch rather than as a convenience.
fn scroll_detail(code: KeyCode, state: &mut AppState) -> bool {
    let Some(DrillLevel::Manny(id)) = state.pane_nav[state.active_pane.index()].drill.last().cloned() else {
        return false;
    };
    let (width, height) = crate::ui::cockpit_v2::active_pane_inner_size(state);
    let total = crate::ui::cockpit_v2::manny_detail_height(state, &id, width, state.palette());
    // The estimate is a floor (word wrapping can add a row), so allow a little
    // slack — the renderer clamps the offset for real, which makes overscroll
    // impossible and keeps the last row reachable when the estimate is short.
    let max = total.saturating_sub(height as usize) + DETAIL_SCROLL_SLACK;
    let cur = state.detail_scroll();
    let next = match code {
        KeyCode::Down | KeyCode::Char('j') => (cur + 1).min(max),
        KeyCode::Up | KeyCode::Char('k') => cur.saturating_sub(1),
        KeyCode::PageDown => (cur + LIST_PAGE).min(max),
        KeyCode::PageUp => cur.saturating_sub(LIST_PAGE),
        KeyCode::Home => 0,
        KeyCode::End => max,
        _ => return false,
    };
    state.set_detail_scroll(next);
    true
}

/// Rows of slack allowed over the estimated detail height — see [`scroll_detail`].
const DETAIL_SCROLL_SLACK: usize = 2;

/// Scroll the Scanner's observation detail (issue #347). Returns `true` when
/// the key was consumed.
///
/// Unlike the Manny detail (#337) the bound here is **exact**: the detail
/// paragraph is drawn without wrapping, so its line count is its rendered
/// height, and `scanner_detail_lines` is the very list the renderer draws.
fn scroll_scan_detail(code: KeyCode, state: &mut AppState) -> bool {
    let (_, height) = crate::ui::cockpit_v2::active_pane_inner_size(state);
    let total = crate::ui::panels::scanner::scanner_detail_lines(state, state.palette()).len();
    let max = total.saturating_sub(height as usize);
    let cur = state.scan_detail_scroll;
    let next = match code {
        KeyCode::Down | KeyCode::Char('j') => (cur + 1).min(max),
        KeyCode::Up | KeyCode::Char('k') => cur.saturating_sub(1),
        KeyCode::PageDown => (cur + LIST_PAGE).min(max),
        KeyCode::PageUp => cur.saturating_sub(LIST_PAGE),
        KeyCode::Home => 0,
        KeyCode::End => max,
        _ => return false,
    };
    state.scan_detail_scroll = next;
    true
}

/// `Enter` action for the active pane: panes with a discrete action set open
/// the contextual menu; panes backed by a rich wizard reuse its overlay.
fn open_actions(state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    match state.active_pane {
        Pane::Mannies | Pane::Inventory | Pane::Probe | Pane::Storage | Pane::Scanner | Pane::Map => {
            match state.build_context_menu() {
                Some(menu) if !menu.items.is_empty() => state.mode = InputMode::Menu(menu),
                _ => state.set_toast("no actions here"),
            }
        }
        Pane::Missions => match state.missions_category() {
            // Root: Enter enters the selected category, like `l`.
            None => missions_activate(state, client, tx),
            // Both journals are records, not consoles. The logbook is written
            // with its own keys (c/e/x), not through Enter (issue #254).
            Some(MissionsCategory::ShipsLog) => state.set_toast("ship's log — read only"),
            Some(MissionsCategory::Logbook) => missions_activate(state, client, tx),
            Some(MissionsCategory::Missions) => {
                let in_detail = matches!(
                    state.pane_nav[Pane::Missions.index()].drill.last(),
                    Some(DrillLevel::Mission(_))
                );
                if in_detail {
                    // Viewing a mission's steps — no extra action.
                } else if state.missions.is_empty() {
                    state.set_toast("no missions");
                } else {
                    let selection = state.pane_nav[Pane::Missions.index()].cursor;
                    state.active_wizard = ActiveWizard::Missions(MissionsInput::Browsing { selection });
                }
            }
        },
        Pane::Comms => comms_activate(state, client, tx),
        // Enter is drill-in here: the halves and the pages are what there is
        // to act on, and writing has its own keys (c/e/x).
        Pane::Sector => open_sector_object_actions(state),
    }
}

/// Comms activation, shared by `Enter` and `l`: at the root, pick a category
/// (Messages opens its overlay; Alerts/Warnings drill into an in-pane list);
/// inside Alerts/Warnings, acknowledge the selected entry.
/// `c` writes a new page, `e` edits the selected one, `x` asks before deleting
/// it (issue #254). Editing loads the body already fetched for the reader; a
/// page whose body has not arrived yet cannot be edited, and says so.
fn logbook_key(code: KeyCode, state: &mut AppState) {
    let cursor = state.pane_nav[Pane::Missions.index()].cursor;
    let selected = state
        .logbook_pages
        .as_ref()
        .and_then(|pages| pages.get(cursor))
        .map(|p| (p.id, p.title.clone()));
    match code {
        KeyCode::Char('c') => {
            state.active_wizard = ActiveWizard::Logbook(LogbookInput::Title {
                page_id: None,
                title: String::new(),
                content: String::new(),
                error: None,
            });
        }
        KeyCode::Char('e') => match selected {
            Some((id, title)) => match state.logbook_page.as_ref().filter(|p| p.id == id) {
                Some(page) => {
                    state.active_wizard = ActiveWizard::Logbook(LogbookInput::Title {
                        page_id: Some(id),
                        title,
                        content: page.content.clone(),
                        error: None,
                    });
                }
                // Editing a body we have not read would silently truncate it.
                None => state.set_toast("open the page first (l), then edit"),
            },
            None => state.set_toast("no page selected"),
        },
        KeyCode::Char('x') => match selected {
            Some((page_id, title)) => {
                state.active_wizard = ActiveWizard::Logbook(LogbookInput::ConfirmDelete { page_id, title });
            }
            None => state.set_toast("no page selected"),
        },
        _ => {}
    }
}

/// `l`/`Enter` on the Log pane: enter a half, then read a logbook page
/// (issues #345, #254). The pages are fetched lazily, when the half is opened
fn comms_activate(state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    let cursor = state.pane_nav[Pane::Comms.index()].cursor;
    match state.comms_drill() {
        None => match CommsCategory::ALL.get(cursor) {
            Some(CommsCategory::Messages) => {
                state.active_wizard = ActiveWizard::Messages(MessagesInput::Browsing {
                    sent_tab: false,
                    selection: 0,
                });
                fetch_messages(client.clone(), tx.clone());
                fetch_sent_messages(client.clone(), tx.clone());
            }
            Some(CommsCategory::Alerts) => {
                state.comms_enter_category(CommsCategory::Alerts);
                fetch_alerts(client.clone(), tx.clone());
            }
            Some(CommsCategory::Warnings) => {
                state.comms_enter_category(CommsCategory::Warnings);
                fetch_damage_warnings(client.clone(), tx.clone());
            }
            None => {}
        },
        Some(CommsCategory::Alerts) => {
            if let Some(id) = state.alerts.get(cursor).map(|a| a.id) {
                fetch_ack_alert(id, client.clone(), tx.clone());
            }
        }
        Some(CommsCategory::Warnings) => {
            if let Some(id) = state.damage_warnings.get(cursor).map(|w| w.id) {
                fetch_ack_damage_warning(id, client.clone(), tx.clone());
            }
        }
        Some(CommsCategory::Messages) => {}
    }
}

/// Missions activation for `l` (and root `Enter`): at the root, enter the
/// selected category (missions list / ship's log); in the missions list, drill
/// into the selected mission's steps. The ship's log is read-only.
/// `l`/`Enter` on the Missions pane: enter a category, then drill one level —
/// a mission into its steps, a logbook page into its body (issue #254).
///
/// The logbook's pages are fetched **lazily**, when the category is opened: a
/// pilot who never writes never pays the call.
fn missions_activate(state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    let cursor = state.pane_nav[Pane::Missions.index()].cursor;
    match state.missions_category() {
        None => {
            let Some(&cat) = MissionsCategory::ALL.get(cursor) else {
                return;
            };
            state.missions_enter_category(cat);
            if cat == MissionsCategory::Logbook {
                match state.probe_id() {
                    Some(id) => fetch_logbook_pages(id, client.clone(), tx.clone()),
                    // Mirror-only endpoints: without a probe sync there is no
                    // path to call, so say so rather than fail at request time.
                    None => state.logbook_error = Some("waiting for a probe sync".into()),
                }
            }
        }
        Some(MissionsCategory::Missions) => {
            let drilled = matches!(
                state.pane_nav[Pane::Missions.index()].drill.last(),
                Some(DrillLevel::Mission(_))
            );
            if !drilled {
                if let Some(id) = state.missions.get(cursor).map(|m| m.id.clone()) {
                    state.missions_drill_into(id);
                }
            }
        }
        // A flat read: nothing to drill into.
        Some(MissionsCategory::ShipsLog) => {}
        Some(MissionsCategory::Logbook) => {
            if state.logbook_open_page().is_some() {
                return;
            }
            let page = state.logbook_pages.as_ref().and_then(|p| p.get(cursor)).map(|p| p.id);
            if let (Some(page_id), Some(probe_id)) = (page, state.probe_id()) {
                state.logbook_page = None;
                state.pane_nav[Pane::Missions.index()]
                    .drill
                    .push(DrillLevel::LogbookPage(page_id));
                fetch_logbook_page(probe_id, page_id, client.clone(), tx.clone());
            }
        }
    }
}

/// Drill into the selected element. For Storage, this fetches the container's
/// contents so they can be rendered inline (no legacy content modal).
fn drill_in(state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    // Comms drives its own drill (categories → in-pane alert/warning lists).
    if state.active_pane == Pane::Comms {
        comms_activate(state, client, tx);
        return;
    }
    // Missions likewise (categories → missions list → steps, or ship's log).
    if state.active_pane == Pane::Missions {
        missions_activate(state, client, tx);
        return;
    }
    state.pane_drill_in();
    if state.active_pane == Pane::Storage {
        if let Some(DrillLevel::Container(id)) = state.pane_nav[Pane::Storage.index()].drill.last().cloned() {
            state.storage_container_detail = None;
            state.storage_container_detail_error = None;
            fetch_storage_container_detail(id, client.clone(), tx.clone());
        }
    }
}

/// Drill out one level, clearing any transient detail loaded for the level.
fn drill_out(state: &mut AppState) {
    if state.active_pane == Pane::Storage {
        state.storage_container_detail = None;
        state.storage_container_detail_error = None;
    }
    state.pane_drill_out();
}

fn open_sector_object_actions(state: &mut AppState) {
    let entries = state.scanner_objects();
    let cur = state.pane_nav[Pane::Sector.index()].cursor;
    let Some(entry) = entries.get(cur) else {
        state.set_toast("no object selected");
        return;
    };
    let actions = state.actions_for_object(entry);
    if actions.is_empty() {
        state.set_toast(format!("no actions for {}", entry.name));
        return;
    }
    state.active_wizard = ActiveWizard::ObjectAction(ObjectActionInput::PickAction {
        object_id: entry.id.clone(),
        object_name: entry.name.clone(),
        actions,
        selection: 0,
    });
}

fn handle_menu_key(code: KeyCode, state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    match code {
        KeyCode::Esc => state.mode = InputMode::Normal,
        // The menu is a list like any other: same keys, same wrap (issue #325).
        _ if is_list_nav_key(code) => {
            if let InputMode::Menu(m) = &mut state.mode {
                let count = m.items.len();
                list_move(code, &mut m.cursor, count);
            }
        }
        KeyCode::Enter => {
            let idx = if let InputMode::Menu(m) = &state.mode {
                m.cursor
            } else {
                return;
            };
            pick_menu_item(idx, state, client, tx);
        }
        // 1-9 accelerators: fire the nth item directly (one keystroke instead
        // of walking there with j/k). Out-of-range digits noop; a disabled one
        // explains itself, exactly as Enter on it does.
        KeyCode::Char(c @ '1'..='9') => {
            pick_menu_item(c as usize - '1' as usize, state, client, tx);
        }
        _ => {}
    }
}

/// Fire the menu item at `idx`. A **disabled** item is not a dead keypress: it
/// already carries the reason it cannot run, so surface that as a toast rather
/// than silently dropping the action (issue #325).
fn pick_menu_item(idx: usize, state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    let InputMode::Menu(m) = &state.mode else {
        return;
    };
    let Some(item) = m.items.get(idx) else {
        return;
    };
    if !item.enabled {
        let label = item.label.clone();
        let reason = item.disabled_reason.clone();
        state.set_toast(match reason {
            Some(reason) => format!("{label} — {reason}"),
            None => format!("{label} — unavailable"),
        });
        return;
    }
    let action = item.action;
    state.mode = InputMode::Normal;
    fire_menu_action(action, state, client, tx);
}

/// Launch the wizard behind a menu action for the selected Manny. Mirrors the
/// classic single-key launches; the shared wizard handlers take over next.
fn fire_menu_action(action: MenuAction, state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    // Inventory-pane actions operate on the selected inventory row, not a Manny.
    match action {
        MenuAction::Jettison => {
            match state.jettison_for_selected() {
                Ok(input) => state.active_wizard = ActiveWizard::Jettison(input),
                Err(msg) => state.error = Some(msg),
            }
            return;
        }
        // The Inventory pane opens the catalog with no builder pre-chosen; the
        // Mannies-pane variant (with a builder) is handled further down.
        MenuAction::Fabricate if state.active_pane == Pane::Inventory => {
            if state.fabrication_recipes().is_empty() {
                state.error = Some("recipes not loaded yet — F5 to refresh".into());
            } else {
                state.active_wizard = ActiveWizard::Fabrication(FabricationInput::pick_recipe(None));
            }
            return;
        }
        MenuAction::MoveStock => {
            let mannies = state.collect_idle_onboard_mannies();
            match mannies.len() {
                0 => state.error = Some("no idle Manny on board".into()),
                1 => {
                    let (id, name) = mannies.into_iter().next().unwrap();
                    state.active_wizard = ActiveWizard::StorageMove(StorageMoveInput::PickKind {
                        actor_manny_id: id,
                        actor_manny_name: name,
                        selection: 0,
                    });
                }
                _ => {
                    state.active_wizard =
                        ActiveWizard::StorageMove(StorageMoveInput::PickManny { mannies, selection: 0 })
                }
            }
            return;
        }
        MenuAction::Deploy => {
            // Deploy a held waypoint bookmark: pick the installing Manny, then a
            // target object in the current sector, then a name.
            let mannies = state.collect_idle_onboard_mannies();
            if mannies.is_empty() {
                state.error = Some("no idle Manny on board".into());
            } else if state.collect_deploy_candidates().is_empty() {
                state.error = Some("no target object in current sector — scan first".into());
            } else {
                state.active_wizard = ActiveWizard::Deploy(DeployInput::PickManny { mannies, selection: 0 });
            }
            return;
        }
        MenuAction::ShareBlueprint => {
            let nets = state.scut_coverage();
            match nets.len() {
                0 => state.error = Some("no SCUT network covers this sector".into()),
                1 => {
                    state.scut_network_view = None;
                    state.active_wizard = ActiveWizard::ShareBlueprint(ShareBlueprintInput::Loading {
                        network_name: nets[0].1.clone(),
                    });
                    fetch_scut_network(nets[0].0, client.clone(), tx.clone());
                }
                _ => {
                    state.active_wizard = ActiveWizard::ShareBlueprint(ShareBlueprintInput::PickNetwork {
                        networks: nets,
                        selection: 0,
                    })
                }
            }
            return;
        }
        MenuAction::ScutCorridors => {
            // The local half is known from the sector scan; the remote half
            // needs the network detail, so open on Loading and let the fetch
            // land (API v96, issue #257).
            let nets = state.local_beacon_networks();
            match nets.len() {
                0 => state.error = Some("no beacon relay in this sector".into()),
                1 => {
                    state.scut_network_view = None;
                    state.active_wizard = ActiveWizard::ScutCorridor(ScutCorridorInput::Loading {
                        network_name: nets[0].1.clone(),
                    });
                    fetch_scut_network(nets[0].0, client.clone(), tx.clone());
                }
                _ => {
                    state.active_wizard = ActiveWizard::ScutCorridor(ScutCorridorInput::PickNetwork {
                        networks: nets,
                        selection: 0,
                    })
                }
            }
            return;
        }
        MenuAction::ScutInspect => {
            let nets = state.scut_coverage();
            match nets.len() {
                0 => state.error = Some("no SCUT network covers this sector".into()),
                1 => {
                    state.active_wizard = ActiveWizard::ScutNetwork(ScutNetworkInput::Viewing { error: None });
                    state.scut_network_view = None;
                    fetch_scut_network(nets[0].0, client.clone(), tx.clone());
                }
                _ => {
                    state.active_wizard = ActiveWizard::ScutNetwork(ScutNetworkInput::Picking {
                        networks: nets,
                        selection: 0,
                    })
                }
            }
            return;
        }
        MenuAction::Improve => {
            if state.has_orderable_improvement() {
                state.active_wizard = ActiveWizard::Improve(ImproveInput::PickImprovement {
                    selection: 0,
                    error: None,
                });
            } else {
                state.error = Some("no probe improvement available".into());
            }
            return;
        }
        MenuAction::MindSnapshot => {
            if state.probe_terminal_alert().is_some() {
                state.active_wizard = ActiveWizard::MindSnapshot(MindSnapshotInput::Confirm { error: None });
            }
            return;
        }
        MenuAction::RenameContainer => {
            if let Some(id) = state.storage_selected_container_id() {
                if let Some((container_id, label)) =
                    state.storage_container(&id).map(|c| (c.id.clone(), c.label.clone()))
                {
                    // Prefilled with the current name: renaming something
                    // already named is the common case, and the ceremony is
                    // better offered than imposed — `Tab` still suggests (#330).
                    let buf = label.clone();
                    state.active_wizard = ActiveWizard::RenameContainer(RenameContainerInput::Typing {
                        container_id,
                        current_label: label,
                        buf,
                        error: None,
                    });
                }
            }
            return;
        }
        MenuAction::EditContainerRules => {
            if let Some(id) = state.storage_selected_container_id() {
                if let Some(editor) = state.rules_editor_for(&id) {
                    state.active_wizard = ActiveWizard::ContainerRules(editor);
                }
            }
            return;
        }
        MenuAction::ReassignReservations => {
            // Atomic server-side: every reservation moves, or none does and the
            // container is left exactly as it was (API v116).
            if let (Some(id), Some(probe_id)) = (state.storage_selected_container_id(), state.probe_id()) {
                fetch_reassign_reservations(probe_id, id, client.clone(), tx.clone());
                state.loading = true;
            }
            return;
        }
        MenuAction::ScanAround => {
            if let Some((x, y, z)) = state.probe_sector_coords() {
                let offsets = super::geometry::neighbors_d1();
                state.start_batch(offsets.len());
                for (dx, dy, dz) in offsets {
                    fetch_sector(Some((x + dx, y + dy, z + dz)), client.clone(), tx.clone());
                }
            }
            return;
        }
        MenuAction::ScanDirection => {
            if state.probe_sector_coords().is_some() {
                state.scan_mode = ScanMode::DirectionPick;
            }
            return;
        }
        MenuAction::ScanObserve => {
            state.scan_mode = ScanMode::Input(String::new());
            return;
        }
        MenuAction::ScanFilter => {
            state.cycle_scan_filter();
            state.set_toast(format!("filter: {}", state.scan_filter.label()));
            return;
        }
        MenuAction::ScanTravel => {
            if let Some(s) = state.current_sector() {
                let c = &s.relative_coordinates;
                let (x, y, z) = (c.x.round() as i32, c.y.round() as i32, c.z.round() as i32);
                state.travel_go_sector(x, y, z);
            }
            return;
        }
        MenuAction::OpenMap => {
            state.open_map();
            return;
        }
        MenuAction::Travel => {
            state.active_wizard = ActiveWizard::Travel(TravelInput::Typing(String::new()));
            return;
        }
        MenuAction::GotoVisited => {
            if !state.visited_sectors.is_empty() {
                state.goto_visited = GotoVisitedInput::Picking { selection: 0 };
            }
            return;
        }
        MenuAction::Waypoints => {
            let entries = state.collect_waypoints();
            if !entries.is_empty() {
                state.active_wizard = ActiveWizard::Waypoints(WaypointsInput::Browsing { entries, selection: 0 });
            }
            return;
        }
        MenuAction::SwitchProbe => {
            if state.fleet.len() > 1 {
                // Open the picker on the currently active probe.
                let active = state.active_probe_id.or(state.default_probe_id);
                let selection = state.fleet.iter().position(|p| Some(p.id) == active).unwrap_or(0);
                state.probe_switch = ProbeSwitchInput::Picking { selection };
            }
            return;
        }
        MenuAction::SetDefaultProbe => {
            if let Some(active) = state.active_probe_summary() {
                if !active.is_default && active.is_reachable {
                    let (id, name) = (active.id, active.name.clone());
                    fetch_set_default_probe(id, name.clone(), client.clone(), tx.clone());
                    state.log_event(LogEvent::set_default_probe(&name, Some(id)));
                }
            }
            return;
        }
        MenuAction::RenameProbe => {
            if let Some((id, name)) = state.active_probe_identity() {
                let buf = name.clone();
                state.active_wizard = ActiveWizard::RenameProbe(RenameProbeInput::Typing {
                    probe_id: id,
                    current_name: name,
                    buf,
                    error: None,
                });
            }
            return;
        }
        _ => {}
    }

    let Some(m) = state.mannies.as_ref().and_then(|v| v.get(state.mannies_selection)) else {
        return;
    };
    let id = m.id.clone();
    let name = m.name.clone();
    let can = m.can_receive_orders;
    let has_task = m.current_task.is_some();
    let waiting_space = m.current_task == Some(MannyTask::WaitingForSpace);
    let remote_recall = matches!(m.task_visibility, Some(MannyTaskVisibility::ScutNetwork));
    let remote_minable = state.manny_remote_minable(m);
    let coords = state.manny_sector_coords(m).unwrap_or((0, 0, 0));

    match action {
        MenuAction::Repair if can => {
            state.active_wizard = ActiveWizard::Repair(RepairInput::Typing {
                manny_id: id,
                manny_name: name,
                buf: String::new(),
                error: None,
            });
        }
        MenuAction::Fabricate if can => {
            if state.fabrication_recipes().is_empty() {
                state.error = Some("recipes not loaded yet — F5 to refresh".into());
            } else {
                state.active_wizard = ActiveWizard::Fabrication(FabricationInput::pick_recipe(Some((id, name))));
            }
        }
        MenuAction::AssembleProbe if can => {
            let containers = state.collect_empty_containers();
            if containers.len() < 2 {
                state.error = Some("need two empty additional containers".into());
            } else {
                // The hull model comes first (API v104): it sets the bill the
                // container step shows.
                state.active_wizard = ActiveWizard::AssembleProbe(AssembleProbeInput::PickModel {
                    manny_id: id,
                    manny_name: name,
                    containers,
                    cursor: 0,
                });
            }
        }
        MenuAction::Mine => {
            if remote_minable {
                state.active_wizard = ActiveWizard::RemoteMine(RemoteMineInput::Loading {
                    manny_id: id,
                    manny_name: name,
                    x: coords.0,
                    y: coords.1,
                    z: coords.2,
                });
                state.set_toast("fetching remote sector…");
                fetch_sector(Some(coords), client.clone(), tx.clone());
            } else if can {
                let candidates = state.collect_mineable_candidates();
                match candidates.len() {
                    0 => state.error = Some("no mineable objects in current sector — scan first".into()),
                    1 => {
                        let (object_id, object_name) = candidates.into_iter().next().unwrap();
                        state.active_wizard = ActiveWizard::Mine(MineInput::Configure {
                            manny_id: id,
                            manny_name: name,
                            object_id,
                            object_name,
                            resources: [false, true, false, false],
                            amount_buf: "0.30".into(),
                            amount_mode: false,
                            target_container: None,
                            error: None,
                        });
                    }
                    _ => {
                        state.active_wizard = ActiveWizard::Mine(MineInput::PickAsteroid {
                            manny_id: id,
                            manny_name: name,
                            candidates,
                            selection: 0,
                        })
                    }
                }
            }
        }
        MenuAction::Salvage if can => {
            let candidates = state.collect_salvage_candidates();
            match candidates.len() {
                0 => state.error = Some("no salvageable objects in current sector — scan first".into()),
                1 => {
                    let (object_id, object_name) = candidates.into_iter().next().unwrap();
                    state.active_wizard = ActiveWizard::Salvage(SalvageInput::Confirm {
                        manny_id: id,
                        manny_name: name,
                        object_id,
                        object_name,
                        error: None,
                    });
                }
                _ => {
                    state.active_wizard = ActiveWizard::Salvage(SalvageInput::PickTarget {
                        manny_id: id,
                        manny_name: name,
                        candidates,
                        selection: 0,
                    })
                }
            }
        }
        MenuAction::Inspect if can => {
            let candidates = state.collect_inspectable_candidates();
            match candidates.len() {
                0 => state.error = Some("no inspectable objects in current sector — scan first".into()),
                1 => {
                    let (object_id, object_name) = candidates.into_iter().next().unwrap();
                    fetch_inspect(id, object_id, client.clone(), tx.clone());
                    state.log_event(LogEvent::inspect(&object_name, state.active_probe_id));
                }
                _ => {
                    state.active_wizard = ActiveWizard::Inspect(InspectInput::PickTarget {
                        manny_id: id,
                        manny_name: name,
                        candidates,
                        selection: 0,
                        error: None,
                    })
                }
            }
        }
        MenuAction::Recover if can => {
            let candidates = state.collect_detached_containers();
            match candidates.len() {
                0 => state.error = Some("no detached containers in current sector — scan first".into()),
                1 => {
                    let (object_id, container_name) = candidates.into_iter().next().unwrap();
                    fetch_recover(id, object_id, client.clone(), tx.clone());
                    state.log_event(LogEvent::recover(&container_name, state.active_probe_id));
                }
                _ => {
                    state.active_wizard = ActiveWizard::Recover(RecoverInput::PickContainer {
                        manny_id: id,
                        manny_name: name,
                        candidates,
                        selection: 0,
                        error: None,
                    })
                }
            }
        }
        MenuAction::Detach if can => {
            let containers = state.collect_detachable_containers();
            match containers.len() {
                0 => state.error = Some("no detachable containers in inventory".into()),
                1 => {
                    let (container_id, container_name) = containers.into_iter().next().unwrap();
                    state.active_wizard = ActiveWizard::Detach(DetachInput::PickMode {
                        manny_id: id,
                        manny_name: name,
                        container_id,
                        container_name,
                        selection: 0,
                        error: None,
                    });
                }
                _ => {
                    state.active_wizard = ActiveWizard::Detach(DetachInput::PickContainer {
                        manny_id: id,
                        manny_name: name,
                        containers,
                        selection: 0,
                    })
                }
            }
        }
        MenuAction::DropStorageContainer if can => {
            let containers = state.collect_detachable_containers();
            if containers.is_empty() {
                state.error = Some("no detachable containers in inventory".into());
            } else if !state.has_atmospheric_drop_kit() {
                state.error = Some("no atmospheric_drop_kit in inventory".into());
            } else {
                let planets = state.collect_planet_candidates();
                if planets.is_empty() {
                    state.error = Some("no planet in current sector — scan first".into());
                } else if containers.len() == 1 {
                    let (container_id, container_name) = containers.into_iter().next().unwrap();
                    state.active_wizard = ActiveWizard::DropContainer(DropStorageContainerInput::PickPlanet {
                        manny_id: id,
                        manny_name: name,
                        container_id,
                        container_name,
                        planets,
                        selection: 0,
                        error: None,
                    });
                } else {
                    state.active_wizard = ActiveWizard::DropContainer(DropStorageContainerInput::PickContainer {
                        manny_id: id,
                        manny_name: name,
                        containers,
                        selection: 0,
                    });
                }
            }
        }
        MenuAction::Refuel if can => {
            if state.deuterium_station_in_current_sector() {
                state.active_wizard = ActiveWizard::Refuel(RefuelInput::Confirm {
                    manny_id: id,
                    manny_name: name,
                    error: None,
                });
            } else {
                state.error = Some("no deuterium refuel station in this sector".into());
            }
        }
        MenuAction::TransferDeuterium if can => {
            let targets = state.other_fleet_probes();
            if targets.is_empty() {
                state.error = Some("no other probe in the fleet".into());
            } else {
                state.active_wizard = ActiveWizard::TransferDeuterium(TransferDeuteriumInput::PickTarget {
                    manny_id: id,
                    manny_name: name,
                    targets,
                    selection: 0,
                });
            }
        }
        MenuAction::TransferProbe if can => {
            let targets = state.other_fleet_probes();
            if targets.is_empty() {
                state.error = Some("no other probe in the fleet".into());
            } else {
                state.active_wizard = ActiveWizard::TransferProbe(TransferProbeInput::PickTarget {
                    manny_id: id,
                    manny_name: name,
                    targets,
                    selection: 0,
                    error: None,
                });
            }
        }
        MenuAction::DropCargo if waiting_space => {
            state.active_wizard = ActiveWizard::DropCargo(DropCargoInput::Confirm {
                manny_id: id,
                manny_name: name,
                error: None,
            });
        }
        MenuAction::Recall if !can && has_task => {
            state.active_wizard = ActiveWizard::Recall(RecallInput::Confirm {
                manny_id: id,
                manny_name: name,
                remote: remote_recall,
                error: None,
            });
        }
        MenuAction::Rename => {
            let buf = name.clone();
            state.active_wizard = ActiveWizard::RenameManny(RenameMannyInput::Typing {
                manny_id: id,
                manny_name: name,
                buf,
                error: None,
            });
        }
        // Guard mismatch (state changed since the menu was built): no-op.
        _ => {}
    }
}
