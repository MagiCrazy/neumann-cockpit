//! Sharing an improvement blueprint through SCUT (API v116, #301 phase 3).
//!
//! Three steps once the network is known: recipient, then blueprint, then fire.
//! The recipient list comes from the network's own probes minus our fleet — the
//! server rejects sharing with oneself, and a candidate the cockpit knows is
//! invalid has no business being offered.

use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::{fetch_scut_network, fetch_share_blueprint};
use crate::app::{ActiveWizard, ApiMessage, AppState, ShareBlueprintInput};

use super::geometry::{is_list_nav_key, list_nav};

pub(super) fn handle_share_blueprint_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.active_wizard {
        ActiveWizard::ShareBlueprint(ShareBlueprintInput::PickNetwork { networks, selection }) => {
            let (count, selection) = (networks.len(), *selection);
            match code {
                KeyCode::Esc => state.close_wizard(),
                _ if is_list_nav_key(code) => {
                    if let Some(ns) = list_nav(code, selection, count) {
                        if let ActiveWizard::ShareBlueprint(ShareBlueprintInput::PickNetwork { selection, .. }) =
                            &mut state.active_wizard
                        {
                            *selection = ns;
                        }
                    }
                }
                KeyCode::Enter => {
                    let ActiveWizard::ShareBlueprint(ShareBlueprintInput::PickNetwork { networks, .. }) =
                        &state.active_wizard
                    else {
                        return;
                    };
                    let (id, name) = networks[selection].clone();
                    state.scut_network_view = None;
                    state.active_wizard =
                        ActiveWizard::ShareBlueprint(ShareBlueprintInput::Loading { network_name: name });
                    fetch_scut_network(id, client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        ActiveWizard::ShareBlueprint(ShareBlueprintInput::Loading { .. }) if code == KeyCode::Esc => {
            state.close_wizard();
            state.scut_network_view = None;
        }
        ActiveWizard::ShareBlueprint(ShareBlueprintInput::PickRecipient {
            recipients, selection, ..
        }) => {
            let (count, selection) = (recipients.len(), *selection);
            match code {
                KeyCode::Esc => {
                    state.close_wizard();
                    state.scut_network_view = None;
                }
                _ if is_list_nav_key(code) => {
                    if let Some(ns) = list_nav(code, selection, count) {
                        if let ActiveWizard::ShareBlueprint(ShareBlueprintInput::PickRecipient { selection, .. }) =
                            &mut state.active_wizard
                        {
                            *selection = ns;
                        }
                    }
                }
                KeyCode::Enter => {
                    let ActiveWizard::ShareBlueprint(ShareBlueprintInput::PickRecipient { recipients, .. }) =
                        &state.active_wizard
                    else {
                        return;
                    };
                    let Some((recipient_id, recipient_name)) = recipients.get(selection).cloned() else {
                        return;
                    };
                    let blueprints = state.shareable_blueprints();
                    state.active_wizard = ActiveWizard::ShareBlueprint(ShareBlueprintInput::PickBlueprint {
                        recipient_id,
                        recipient_name,
                        blueprints,
                        selection: 0,
                        error: None,
                    });
                }
                _ => {}
            }
        }
        ActiveWizard::ShareBlueprint(ShareBlueprintInput::PickBlueprint {
            recipient_id,
            blueprints,
            selection,
            ..
        }) => {
            let (count, selection) = (blueprints.len(), *selection);
            let recipient_id = *recipient_id;
            match code {
                KeyCode::Esc => {
                    state.close_wizard();
                    state.scut_network_view = None;
                }
                _ if is_list_nav_key(code) => {
                    if let Some(ns) = list_nav(code, selection, count) {
                        if let ActiveWizard::ShareBlueprint(ShareBlueprintInput::PickBlueprint { selection, .. }) =
                            &mut state.active_wizard
                        {
                            *selection = ns;
                        }
                    }
                }
                KeyCode::Enter => {
                    let ActiveWizard::ShareBlueprint(ShareBlueprintInput::PickBlueprint { blueprints, .. }) =
                        &state.active_wizard
                    else {
                        return;
                    };
                    let Some((blueprint_id, _)) = blueprints.get(selection).cloned() else {
                        return;
                    };
                    // Mirror-only path: without a probe sync there is no id to
                    // build it with, and the menu entry is gated on that.
                    let Some(probe_id) = state.probe_id() else {
                        state.set_wizard_error("no probe id yet — wait for a sync".into());
                        return;
                    };
                    fetch_share_blueprint(probe_id, blueprint_id, recipient_id, client.clone(), tx.clone());
                    state.loading = true;
                }
                _ => {}
            }
        }
        _ => {}
    }
}
