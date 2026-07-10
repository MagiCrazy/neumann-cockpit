use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::{
    fetch_deploy, fetch_detach, fetch_drop_manny_cargo, fetch_drop_storage_container,
    fetch_inspect, fetch_reassign_mind_snapshot, fetch_recall, fetch_refill_deuterium,
    fetch_recover, fetch_rename_manny, fetch_salvage,
};
use crate::app::{
    ActiveWizard, ApiMessage, AppState, DeployInput, DetachInput, DropCargoInput,
    DropStorageContainerInput, InspectInput, LogEvent, RecallInput, RefuelInput,
    RecoverInput, RenameMannyInput, SalvageInput, DETACH_MODES,
};
use super::geometry::list_move;
pub(super) fn handle_salvage_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    if let ActiveWizard::Salvage(SalvageInput::PickTarget { selection, candidates, .. }) = &mut state.active_wizard {
        if list_move(code, selection, candidates.len()) {
            return;
        }
    }
    match &state.active_wizard {
        ActiveWizard::Salvage(SalvageInput::PickTarget { .. }) => match code {
            KeyCode::Esc => state.close_wizard(),
            KeyCode::Enter => {
                let (manny_id, manny_name, object_id, object_name) = {
                    let ActiveWizard::Salvage(SalvageInput::PickTarget { manny_id, manny_name, candidates, selection }) = &state.active_wizard else { return };
                    let (id, name) = candidates[*selection].clone();
                    (manny_id.clone(), manny_name.clone(), id, name)
                };
                state.active_wizard = ActiveWizard::Salvage(SalvageInput::Confirm {
                    manny_id, manny_name, object_id, object_name, error: None,
                });
            }
            _ => {}
        },
        ActiveWizard::Salvage(SalvageInput::Confirm { .. }) => match code {
            KeyCode::Esc => state.close_wizard(),
            KeyCode::Enter => {
                let (manny_id, object_id, object_name) = {
                    let ActiveWizard::Salvage(SalvageInput::Confirm { manny_id, object_id, object_name, .. }) = &state.active_wizard else { return };
                    (manny_id.clone(), object_id.clone(), object_name.clone())
                };
                fetch_salvage(manny_id, object_id, client.clone(), tx.clone());
                state.log_event(LogEvent::salvage(&object_name, state.active_probe_id));
            }
            _ => {}
        },
        _ => {}
    }
}

pub(super) fn handle_recall_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match code {
        KeyCode::Esc => state.close_wizard(),
        KeyCode::Enter => {
            let (manny_id, manny_name, remote) = {
                let ActiveWizard::Recall(RecallInput::Confirm { manny_id, manny_name, remote, .. }) = &state.active_wizard else { return };
                (manny_id.clone(), manny_name.clone(), *remote)
            };
            fetch_recall(manny_id, client.clone(), tx.clone());
            state.log_event(LogEvent::recall(&manny_name, remote, state.active_probe_id));
        }
        _ => {}
    }
}

pub(super) fn handle_refuel_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match code {
        KeyCode::Esc | KeyCode::Char('n') => state.close_wizard(),
        KeyCode::Enter | KeyCode::Char('y') => {
            let manny_id = {
                let ActiveWizard::Refuel(RefuelInput::Confirm { manny_id, .. }) = &state.active_wizard else { return };
                manny_id.clone()
            };
            fetch_refill_deuterium(manny_id, client.clone(), tx.clone());
            state.log_event(LogEvent::refuel(state.active_probe_id));
        }
        _ => {}
    }
}

pub(super) fn handle_mind_snapshot_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match code {
        KeyCode::Esc | KeyCode::Char('n') => state.close_wizard(),
        KeyCode::Enter | KeyCode::Char('y') => {
            fetch_reassign_mind_snapshot(client.clone(), tx.clone());
            state.log_event(LogEvent::mind_snapshot(state.active_probe_id));
        }
        _ => {}
    }
}

pub(super) fn handle_drop_container_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &mut state.active_wizard {
        ActiveWizard::DropContainer(DropStorageContainerInput::PickContainer { selection, containers, .. }) => {
            if list_move(code, selection, containers.len()) {
                return;
            }
        }
        ActiveWizard::DropContainer(DropStorageContainerInput::PickPlanet { selection, planets, .. }) => {
            if list_move(code, selection, planets.len()) {
                return;
            }
        }
        _ => {}
    }
    match &state.active_wizard {
        ActiveWizard::DropContainer(DropStorageContainerInput::PickContainer { .. }) => match code {
            KeyCode::Esc => state.close_wizard(),
            KeyCode::Enter => {
                let (manny_id, manny_name, container_id, container_name) = {
                    let ActiveWizard::DropContainer(DropStorageContainerInput::PickContainer {
                        manny_id, manny_name, containers, selection,
                    }) = &state.active_wizard else { return };
                    let (id, name) = containers[*selection].clone();
                    (manny_id.clone(), manny_name.clone(), id, name)
                };
                let planets = state.collect_planet_candidates();
                if planets.is_empty() {
                    state.close_wizard();
                    state.error = Some("no planet in current sector — scan first".into());
                    return;
                }
                state.active_wizard = ActiveWizard::DropContainer(DropStorageContainerInput::PickPlanet {
                    manny_id, manny_name, container_id, container_name,
                    planets, selection: 0, error: None,
                });
            }
            _ => {}
        },
        ActiveWizard::DropContainer(DropStorageContainerInput::PickPlanet { .. }) => match code {
            KeyCode::Esc => state.close_wizard(),
            KeyCode::Enter => {
                let (manny_id, container_id, planet_id, container_name, planet_name) = {
                    let ActiveWizard::DropContainer(DropStorageContainerInput::PickPlanet {
                        manny_id, container_id, container_name, planets, selection, ..
                    }) = &state.active_wizard else { return };
                    (
                        manny_id.clone(),
                        container_id.clone(),
                        planets[*selection].0.clone(),
                        container_name.clone(),
                        planets[*selection].1.clone(),
                    )
                };
                fetch_drop_storage_container(manny_id, container_id, planet_id, client.clone(), tx.clone());
                state.log_event(LogEvent::drop_container(&container_name, &planet_name, state.active_probe_id));
            }
            _ => {}
        },
        _ => {}
    }
}

pub(super) fn handle_drop_cargo_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match code {
        KeyCode::Esc | KeyCode::Char('n') => state.close_wizard(),
        KeyCode::Enter | KeyCode::Char('y') => {
            let manny_id = {
                let ActiveWizard::DropCargo(DropCargoInput::Confirm { manny_id, .. }) = &state.active_wizard else { return };
                manny_id.clone()
            };
            fetch_drop_manny_cargo(manny_id, client.clone(), tx.clone());
            state.log_event(LogEvent::drop_cargo(state.active_probe_id));
        }
        _ => {}
    }
}

pub(super) fn handle_deploy_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &mut state.active_wizard {
        ActiveWizard::Deploy(DeployInput::PickManny { selection, mannies }) => {
            if list_move(code, selection, mannies.len()) {
                return;
            }
        }
        ActiveWizard::Deploy(DeployInput::PickObject { selection, candidates, .. }) => {
            if list_move(code, selection, candidates.len()) {
                return;
            }
        }
        _ => {}
    }
    match &state.active_wizard {
        ActiveWizard::Deploy(DeployInput::PickManny { .. }) => match code {
            KeyCode::Esc => state.close_wizard(),
            KeyCode::Enter => {
                let manny_id = {
                    let ActiveWizard::Deploy(DeployInput::PickManny { mannies, selection }) = &state.active_wizard else { return };
                    mannies[*selection].0.clone()
                };
                let candidates = state.collect_deploy_candidates();
                if candidates.is_empty() {
                    state.close_wizard();
                    state.error = Some("no targets in current sector".into());
                } else if candidates.len() == 1 {
                    let (object_id, object_name) = candidates.into_iter().next().unwrap();
                    state.active_wizard = ActiveWizard::Deploy(DeployInput::EnterName { manny_id, object_id, object_name, name_buf: String::new(), error: None });
                } else {
                    state.active_wizard = ActiveWizard::Deploy(DeployInput::PickObject { manny_id, candidates, selection: 0 });
                }
            }
            _ => {}
        },
        ActiveWizard::Deploy(DeployInput::PickObject { .. }) => match code {
            KeyCode::Esc => state.close_wizard(),
            KeyCode::Enter => {
                let (manny_id, object_id, object_name) = {
                    let ActiveWizard::Deploy(DeployInput::PickObject { manny_id, candidates, selection }) = &state.active_wizard else { return };
                    let (id, name) = candidates[*selection].clone();
                    (manny_id.clone(), id, name)
                };
                state.active_wizard = ActiveWizard::Deploy(DeployInput::EnterName {
                    manny_id, object_id, object_name, name_buf: String::new(), error: None,
                });
            }
            _ => {}
        },
        ActiveWizard::Deploy(DeployInput::EnterName { .. }) => match code {
            KeyCode::Esc => state.close_wizard(),
            KeyCode::Backspace => state.deploy_backspace(),
            KeyCode::Char(c) => state.deploy_type_char(c),
            KeyCode::Enter => {
                let (manny_id, object_id, name) = {
                    let ActiveWizard::Deploy(DeployInput::EnterName { manny_id, object_id, name_buf, .. }) = &state.active_wizard else { return };
                    if name_buf.is_empty() { return }
                    (manny_id.clone(), object_id.clone(), name_buf.clone())
                };
                let waypoint_name = name.clone();
                fetch_deploy(manny_id, object_id, name, client.clone(), tx.clone());
                state.log_event(LogEvent::deploy_waypoint(&waypoint_name, state.active_probe_id));
            }
            _ => {}
        },
        _ => {}
    }
}

pub(super) fn handle_rename_manny_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match code {
        KeyCode::Esc => state.close_wizard(),
        KeyCode::Tab => {
            let s = state.next_name_suggestion();
            if let ActiveWizard::RenameManny(RenameMannyInput::Typing { buf, .. }) = &mut state.active_wizard {
                *buf = s;
            }
        }
        KeyCode::Backspace => state.rename_manny_backspace(),
        KeyCode::Char(c) => state.rename_manny_type_char(c),
        KeyCode::Enter => {
            let (manny_id, name, old_name) = {
                let ActiveWizard::RenameManny(RenameMannyInput::Typing { manny_id, buf, manny_name, .. }) = &state.active_wizard else { return };
                if buf.is_empty() { return }
                (manny_id.clone(), buf.clone(), manny_name.clone())
            };
            let new_name = name.clone();
            fetch_rename_manny(manny_id, name, client.clone(), tx.clone());
            state.log_event(LogEvent::rename_manny(&old_name, &new_name, state.active_probe_id));
        }
        _ => {}
    }
}

pub(super) fn handle_inspect_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    if let ActiveWizard::Inspect(InspectInput::PickTarget { selection, candidates, .. }) = &mut state.active_wizard {
        if list_move(code, selection, candidates.len()) {
            return;
        }
    }
    match code {
        KeyCode::Esc => state.close_wizard(),
        KeyCode::Enter => {
            let (manny_id, object_id, object_name) = {
                let ActiveWizard::Inspect(InspectInput::PickTarget { manny_id, candidates, selection, .. }) = &state.active_wizard else { return };
                (manny_id.clone(), candidates[*selection].0.clone(), candidates[*selection].1.clone())
            };
            fetch_inspect(manny_id, object_id, client.clone(), tx.clone());
            state.log_event(LogEvent::inspect(&object_name, state.active_probe_id));
        }
        _ => {}
    }
}

pub(super) fn handle_recover_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    if let ActiveWizard::Recover(RecoverInput::PickContainer { selection, candidates, .. }) = &mut state.active_wizard {
        if list_move(code, selection, candidates.len()) {
            return;
        }
    }
    match code {
        KeyCode::Esc => state.close_wizard(),
        KeyCode::Enter => {
            let (manny_id, object_id, container_name) = {
                let ActiveWizard::Recover(RecoverInput::PickContainer { manny_id, candidates, selection, .. }) = &state.active_wizard else { return };
                (manny_id.clone(), candidates[*selection].0.clone(), candidates[*selection].1.clone())
            };
            fetch_recover(manny_id, object_id, client.clone(), tx.clone());
            state.log_event(LogEvent::recover(&container_name, state.active_probe_id));
        }
        _ => {}
    }
}

pub(super) fn handle_detach_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &mut state.active_wizard {
        ActiveWizard::Detach(DetachInput::PickContainer { selection, containers, .. }) => {
            if list_move(code, selection, containers.len()) {
                return;
            }
        }
        ActiveWizard::Detach(DetachInput::PickMode { selection, .. }) => {
            if list_move(code, selection, DETACH_MODES.len()) {
                return;
            }
        }
        ActiveWizard::Detach(DetachInput::PickAsteroid { selection, asteroids, .. }) => {
            if list_move(code, selection, asteroids.len()) {
                return;
            }
        }
        _ => {}
    }
    match &state.active_wizard {
        ActiveWizard::Detach(DetachInput::PickContainer { .. }) => match code {
            KeyCode::Esc => state.close_wizard(),
            KeyCode::Enter => {
                let (manny_id, manny_name, container_id, container_name) = {
                    let ActiveWizard::Detach(DetachInput::PickContainer { manny_id, manny_name, containers, selection }) = &state.active_wizard else { return };
                    let (id, name) = containers[*selection].clone();
                    (manny_id.clone(), manny_name.clone(), id, name)
                };
                state.active_wizard = ActiveWizard::Detach(DetachInput::PickMode { manny_id, manny_name, container_id, container_name, selection: 0, error: None });
            }
            _ => {}
        },
        ActiveWizard::Detach(DetachInput::PickMode { .. }) => match code {
            KeyCode::Esc => state.close_wizard(),
            KeyCode::Enter => {
                let (manny_id, manny_name, container_id, container_name, sel) = {
                    let ActiveWizard::Detach(DetachInput::PickMode { manny_id, manny_name, container_id, container_name, selection, .. }) = &state.active_wizard else { return };
                    (manny_id.clone(), manny_name.clone(), container_id.clone(), container_name.clone(), *selection)
                };
                let mode = DETACH_MODES[sel].0;
                if mode == "hidden_on_asteroid" {
                    let asteroids = state.collect_asteroid_candidates();
                    if asteroids.is_empty() {
                        state.set_detach_error("no asteroids in current sector — scan first".into());
                    } else {
                        state.active_wizard = ActiveWizard::Detach(DetachInput::PickAsteroid { manny_id, manny_name, container_id, container_name, asteroids, selection: 0, error: None });
                    }
                } else {
                    fetch_detach(manny_id, container_id, "drifting".into(), None, client.clone(), tx.clone());
                    state.log_event(LogEvent::detach_container(&container_name, false, state.active_probe_id));
                }
            }
            _ => {}
        },
        ActiveWizard::Detach(DetachInput::PickAsteroid { .. }) => match code {
            KeyCode::Esc => state.close_wizard(),
            KeyCode::Enter => {
                let (manny_id, container_id, object_id, container_name) = {
                    let ActiveWizard::Detach(DetachInput::PickAsteroid { manny_id, container_id, container_name, asteroids, selection, .. }) = &state.active_wizard else { return };
                    (manny_id.clone(), container_id.clone(), asteroids[*selection].0.clone(), container_name.clone())
                };
                fetch_detach(manny_id, container_id, "hidden_on_asteroid".into(), Some(object_id), client.clone(), tx.clone());
                state.log_event(LogEvent::detach_container(&container_name, true, state.active_probe_id));
            }
            _ => {}
        },
        _ => {}
    }
}
