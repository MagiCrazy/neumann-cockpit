use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::api::client::ApiClient;
use crate::api::tasks::{
    fetch_deploy, fetch_detach, fetch_drop_manny_cargo, fetch_drop_storage_container,
    fetch_inspect, fetch_recall,
    fetch_recover, fetch_rename_manny, fetch_salvage,
};
use crate::app::{
    ApiMessage, AppState, DeployInput, DetachInput, DropCargoInput, DropStorageContainerInput,
    InspectInput, RecallInput,
    RecoverInput, RenameMannyInput, SalvageInput, DETACH_MODES,
};
use super::geometry::list_nav;
pub(super) fn handle_salvage_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match &state.salvage {
        SalvageInput::PickTarget { selection, candidates, .. } => {
            let sel = *selection;
            let count = candidates.len();
            match code {
                KeyCode::Esc => state.salvage = SalvageInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let SalvageInput::PickTarget { ref mut selection, .. } = state.salvage {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, manny_name, object_id, object_name) = {
                        let SalvageInput::PickTarget { ref manny_id, ref manny_name, ref candidates, selection } = state.salvage else { return };
                        let (id, name) = candidates[selection].clone();
                        (manny_id.clone(), manny_name.clone(), id, name)
                    };
                    state.salvage = SalvageInput::Confirm {
                        manny_id, manny_name, object_id, object_name, error: None,
                    };
                }
                _ => {}
            }
        }
        SalvageInput::Confirm { .. } => {
            match code {
                KeyCode::Esc => state.salvage = SalvageInput::Inactive,
                KeyCode::Enter => {
                    let (manny_id, object_id) = {
                        let SalvageInput::Confirm { ref manny_id, ref object_id, .. } = state.salvage else { return };
                        (manny_id.clone(), object_id.clone())
                    };
                    fetch_salvage(manny_id, object_id, client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        SalvageInput::Inactive => {}
    }
}

pub(super) fn handle_recall_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match code {
        KeyCode::Esc => state.recall = RecallInput::Inactive,
        KeyCode::Enter => {
            let manny_id = {
                let RecallInput::Confirm { ref manny_id, .. } = state.recall else { return };
                manny_id.clone()
            };
            fetch_recall(manny_id, client.clone(), tx.clone());
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
    match &state.drop_container {
        DropStorageContainerInput::PickContainer { selection, containers, .. } => {
            let (sel, count) = (*selection, containers.len());
            match code {
                KeyCode::Esc => state.drop_container = DropStorageContainerInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(ns) = list_nav(code, sel, count) {
                        if let DropStorageContainerInput::PickContainer { selection, .. } =
                            &mut state.drop_container
                        {
                            *selection = ns;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, manny_name, container_id, container_name) = {
                        let DropStorageContainerInput::PickContainer {
                            manny_id, manny_name, containers, selection,
                        } = &state.drop_container else { return };
                        let (id, name) = containers[*selection].clone();
                        (manny_id.clone(), manny_name.clone(), id, name)
                    };
                    let planets = state.collect_planet_candidates();
                    if planets.is_empty() {
                        state.drop_container = DropStorageContainerInput::Inactive;
                        state.error = Some("no planet in current sector — scan first".into());
                        return;
                    }
                    state.drop_container = DropStorageContainerInput::PickPlanet {
                        manny_id, manny_name, container_id, container_name,
                        planets, selection: 0, error: None,
                    };
                }
                _ => {}
            }
        }
        DropStorageContainerInput::PickPlanet { selection, planets, .. } => {
            let (sel, count) = (*selection, planets.len());
            match code {
                KeyCode::Esc => state.drop_container = DropStorageContainerInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(ns) = list_nav(code, sel, count) {
                        if let DropStorageContainerInput::PickPlanet { selection, .. } =
                            &mut state.drop_container
                        {
                            *selection = ns;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, container_id, planet_id) = {
                        let DropStorageContainerInput::PickPlanet {
                            manny_id, container_id, planets, selection, ..
                        } = &state.drop_container else { return };
                        (manny_id.clone(), container_id.clone(), planets[*selection].0.clone())
                    };
                    fetch_drop_storage_container(manny_id, container_id, planet_id, client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        DropStorageContainerInput::Inactive => {}
    }
}

pub(super) fn handle_drop_cargo_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match code {
        KeyCode::Esc | KeyCode::Char('n') => state.drop_cargo = DropCargoInput::Inactive,
        KeyCode::Enter | KeyCode::Char('y') => {
            let manny_id = {
                let DropCargoInput::Confirm { ref manny_id, .. } = state.drop_cargo else { return };
                manny_id.clone()
            };
            fetch_drop_manny_cargo(manny_id, client.clone(), tx.clone());
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
    match &state.deploy {
        DeployInput::PickManny { selection, mannies } => {
            let sel = *selection;
            let count = mannies.len();
            match code {
                KeyCode::Esc => state.deploy = DeployInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let DeployInput::PickManny { ref mut selection, .. } = state.deploy {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let manny_id = {
                        let DeployInput::PickManny { ref mannies, selection } = state.deploy else { return };
                        mannies[selection].0.clone()
                    };
                    let candidates = state.collect_deploy_candidates();
                    if candidates.is_empty() {
                        state.deploy = DeployInput::Inactive;
                        state.error = Some("no targets in current sector".into());
                    } else if candidates.len() == 1 {
                        let (object_id, object_name) = candidates.into_iter().next().unwrap();
                        state.deploy = DeployInput::EnterName { manny_id, object_id, object_name, name_buf: String::new(), error: None };
                    } else {
                        state.deploy = DeployInput::PickObject { manny_id, candidates, selection: 0 };
                    }
                }
                _ => {}
            }
        }
        DeployInput::PickObject { selection, candidates, .. } => {
            let sel = *selection;
            let count = candidates.len();
            match code {
                KeyCode::Esc => state.deploy = DeployInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let DeployInput::PickObject { ref mut selection, .. } = state.deploy {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, object_id, object_name) = {
                        let DeployInput::PickObject { ref manny_id, ref candidates, selection } = state.deploy else { return };
                        let (id, name) = candidates[selection].clone();
                        (manny_id.clone(), id, name)
                    };
                    state.deploy = DeployInput::EnterName {
                        manny_id,
                        object_id,
                        object_name,
                        name_buf: String::new(),
                        error: None,
                    };
                }
                _ => {}
            }
        }
        DeployInput::EnterName { .. } => {
            match code {
                KeyCode::Esc => state.deploy = DeployInput::Inactive,
                KeyCode::Backspace => state.deploy_backspace(),
                KeyCode::Char(c) => state.deploy_type_char(c),
                KeyCode::Enter => {
                    let (manny_id, object_id, name) = {
                        let DeployInput::EnterName { ref manny_id, ref object_id, ref name_buf, .. } = state.deploy else { return };
                        if name_buf.is_empty() { return }
                        (manny_id.clone(), object_id.clone(), name_buf.clone())
                    };
                    fetch_deploy(manny_id, object_id, name, client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        DeployInput::Inactive => {}
    }
}

pub(super) fn handle_rename_manny_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    match code {
        KeyCode::Esc => state.rename_manny = RenameMannyInput::Inactive,
        KeyCode::Backspace => state.rename_manny_backspace(),
        KeyCode::Char(c) => state.rename_manny_type_char(c),
        KeyCode::Enter => {
            let (manny_id, name) = {
                let RenameMannyInput::Typing { ref manny_id, ref buf, .. } = state.rename_manny else { return };
                if buf.is_empty() { return }
                (manny_id.clone(), buf.clone())
            };
            fetch_rename_manny(manny_id, name, client.clone(), tx.clone());
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
    let &InspectInput::PickAsteroid { selection, ref candidates, .. } = &state.inspect else { return };
    let sel = selection;
    let count = candidates.len();
    match code {
        KeyCode::Esc => state.inspect = InspectInput::Inactive,
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(new_sel) = list_nav(code, sel, count) {
                if let InspectInput::PickAsteroid { ref mut selection, .. } = state.inspect {
                    *selection = new_sel;
                }
            }
        }
        KeyCode::Enter => {
            let (manny_id, object_id) = {
                let InspectInput::PickAsteroid { ref manny_id, ref candidates, selection, .. } = state.inspect else { return };
                (manny_id.clone(), candidates[selection].0.clone())
            };
            fetch_inspect(manny_id, object_id, client.clone(), tx.clone());
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
    let &RecoverInput::PickContainer { selection, ref candidates, .. } = &state.recover else { return };
    let sel = selection;
    let count = candidates.len();
    match code {
        KeyCode::Esc => state.recover = RecoverInput::Inactive,
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
            if let Some(new_sel) = list_nav(code, sel, count) {
                if let RecoverInput::PickContainer { ref mut selection, .. } = state.recover {
                    *selection = new_sel;
                }
            }
        }
        KeyCode::Enter => {
            let (manny_id, object_id) = {
                let RecoverInput::PickContainer { ref manny_id, ref candidates, selection, .. } = state.recover else { return };
                (manny_id.clone(), candidates[selection].0.clone())
            };
            fetch_recover(manny_id, object_id, client.clone(), tx.clone());
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
    match &state.detach {
        DetachInput::PickContainer { selection, containers, .. } => {
            let sel = *selection;
            let count = containers.len();
            match code {
                KeyCode::Esc => state.detach = DetachInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let DetachInput::PickContainer { ref mut selection, .. } = state.detach {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, manny_name, container_id, container_name) = {
                        let DetachInput::PickContainer { ref manny_id, ref manny_name, ref containers, selection } = state.detach else { return };
                        let (id, name) = containers[selection].clone();
                        (manny_id.clone(), manny_name.clone(), id, name)
                    };
                    state.detach = DetachInput::PickMode { manny_id, manny_name, container_id, container_name, selection: 0, error: None };
                }
                _ => {}
            }
        }
        DetachInput::PickMode { selection, .. } => {
            let sel = *selection;
            let count = DETACH_MODES.len();
            match code {
                KeyCode::Esc => state.detach = DetachInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let DetachInput::PickMode { ref mut selection, .. } = state.detach {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, manny_name, container_id, container_name, sel) = {
                        let DetachInput::PickMode { ref manny_id, ref manny_name, ref container_id, ref container_name, selection, .. } = state.detach else { return };
                        (manny_id.clone(), manny_name.clone(), container_id.clone(), container_name.clone(), selection)
                    };
                    let mode = DETACH_MODES[sel].0;
                    if mode == "hidden_on_asteroid" {
                        let asteroids = state.collect_asteroid_candidates();
                        if asteroids.is_empty() {
                            state.set_detach_error("no asteroids in current sector — scan first".into());
                        } else {
                            state.detach = DetachInput::PickAsteroid { manny_id, manny_name, container_id, container_name, asteroids, selection: 0, error: None };
                        }
                    } else {
                        fetch_detach(manny_id, container_id, "drifting".into(), None, client.clone(), tx.clone());
                    }
                }
                _ => {}
            }
        }
        DetachInput::PickAsteroid { selection, asteroids, .. } => {
            let sel = *selection;
            let count = asteroids.len();
            match code {
                KeyCode::Esc => state.detach = DetachInput::Inactive,
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(new_sel) = list_nav(code, sel, count) {
                        if let DetachInput::PickAsteroid { ref mut selection, .. } = state.detach {
                            *selection = new_sel;
                        }
                    }
                }
                KeyCode::Enter => {
                    let (manny_id, container_id, object_id) = {
                        let DetachInput::PickAsteroid { ref manny_id, ref container_id, ref asteroids, selection, .. } = state.detach else { return };
                        (manny_id.clone(), container_id.clone(), asteroids[selection].0.clone())
                    };
                    fetch_detach(manny_id, container_id, "hidden_on_asteroid".into(), Some(object_id), client.clone(), tx.clone());
                }
                _ => {}
            }
        }
        DetachInput::Inactive => {}
    }
}
