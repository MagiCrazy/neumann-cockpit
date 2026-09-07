//! Aiming a full motorized asteroid (API v116, issue #308).
//!
//! Two modes that share only their first screen. The wizard branches rather
//! than collecting a superset of fields because each request arm declares
//! `additionalProperties: false`: a leftover key from the other mode is a 422,
//! not a null the server ignores.
//!
//! Both constraints the server enforces are enforced here first — the speed
//! bound and the direct-neighbour rule — so the pilot meets them as a form
//! that will not accept a bad value, rather than as
//! `invalid_asteroid_target_speed` after the fact.

use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use super::geometry::{is_list_nav_key, list_nav};
use crate::api::client::ApiClient;
use crate::api::tasks::fetch_launch_trajectory;
use crate::app::{
    ActiveWizard, AimAsteroidInput, ApiMessage, AppState, Heading, ImpactTarget, LogEvent, MAX_TARGET_SPEED_C,
};

/// The two modes, in the order the first screen lists them.
const MODES: [&str; 2] = ["system impact", "sector transfer"];

pub(super) fn handle_aim_asteroid_event(
    code: KeyCode,
    state: &mut AppState,
    client: &ApiClient,
    tx: &mpsc::Sender<ApiMessage>,
) {
    if code == KeyCode::Esc {
        step_back(state);
        return;
    }
    match &state.active_wizard {
        ActiveWizard::AimAsteroid(AimAsteroidInput::PickMode { .. }) => pick_mode(code, state),
        ActiveWizard::AimAsteroid(AimAsteroidInput::PickImpactTarget { .. }) => pick_target(code, state),
        ActiveWizard::AimAsteroid(AimAsteroidInput::EnterSpeed { .. }) => enter_speed(code, state),
        ActiveWizard::AimAsteroid(AimAsteroidInput::PickHeading { .. }) => pick_heading(code, state),
        ActiveWizard::AimAsteroid(AimAsteroidInput::Confirm { .. }) => confirm(code, state, client, tx),
        _ => {}
    }
}

/// `Esc` walks back one screen rather than dropping the whole wizard, so a
/// mistyped speed does not cost the target pick — the same courtesy the
/// assembly wizard's model step gained in #275 phase 4.
fn step_back(state: &mut AppState) {
    let back = match &state.active_wizard {
        ActiveWizard::AimAsteroid(AimAsteroidInput::PickImpactTarget {
            asteroid_id,
            asteroid_name,
            ..
        })
        | ActiveWizard::AimAsteroid(AimAsteroidInput::PickHeading {
            asteroid_id,
            asteroid_name,
            ..
        }) => Some(AimAsteroidInput::PickMode {
            asteroid_id: asteroid_id.clone(),
            asteroid_name: asteroid_name.clone(),
            selection: 0,
        }),
        ActiveWizard::AimAsteroid(AimAsteroidInput::EnterSpeed {
            asteroid_id,
            asteroid_name,
            ..
        }) => Some(impact_targets_step(state, asteroid_id.clone(), asteroid_name.clone())),
        _ => None,
    };
    match back {
        Some(step) => state.active_wizard = ActiveWizard::AimAsteroid(step),
        None => state.close_wizard(),
    }
}

fn impact_targets_step(state: &AppState, asteroid_id: String, asteroid_name: String) -> AimAsteroidInput {
    AimAsteroidInput::PickImpactTarget {
        targets: state.impact_targets(&asteroid_id),
        asteroid_id,
        asteroid_name,
        selection: 0,
    }
}

fn pick_mode(code: KeyCode, state: &mut AppState) {
    let ActiveWizard::AimAsteroid(AimAsteroidInput::PickMode {
        asteroid_id,
        asteroid_name,
        selection,
    }) = &state.active_wizard
    else {
        return;
    };
    let (asteroid_id, asteroid_name, sel) = (asteroid_id.clone(), asteroid_name.clone(), *selection);
    if is_list_nav_key(code) {
        let new_sel = list_nav(code, sel, MODES.len());
        if let ActiveWizard::AimAsteroid(AimAsteroidInput::PickMode { selection, .. }) = &mut state.active_wizard {
            if let Some(v) = new_sel {
                *selection = v;
            }
        }
        return;
    }
    if code != KeyCode::Enter {
        return;
    }
    let next = if sel == 0 {
        let step = impact_targets_step(state, asteroid_id, asteroid_name);
        if let AimAsteroidInput::PickImpactTarget { targets, .. } = &step {
            if targets.is_empty() {
                // Nothing local to hit is a fact about the sector, not an
                // error the server should have to tell us.
                state.set_error("no local body to aim at".to_string());
                state.close_wizard();
                return;
            }
        }
        step
    } else {
        let headings = state.transfer_headings();
        if headings.is_empty() {
            state.set_error("waiting for a probe sync".to_string());
            state.close_wizard();
            return;
        }
        AimAsteroidInput::PickHeading {
            asteroid_id,
            asteroid_name,
            headings,
            selection: 0,
        }
    };
    state.active_wizard = ActiveWizard::AimAsteroid(next);
}

fn pick_target(code: KeyCode, state: &mut AppState) {
    let ActiveWizard::AimAsteroid(AimAsteroidInput::PickImpactTarget {
        asteroid_id,
        asteroid_name,
        targets,
        selection,
    }) = &state.active_wizard
    else {
        return;
    };
    if is_list_nav_key(code) {
        let new_sel = list_nav(code, *selection, targets.len());
        if let ActiveWizard::AimAsteroid(AimAsteroidInput::PickImpactTarget { selection, .. }) =
            &mut state.active_wizard
        {
            if let Some(v) = new_sel {
                *selection = v;
            }
        }
        return;
    }
    if code != KeyCode::Enter {
        return;
    }
    let Some(target) = targets.get(*selection).cloned() else {
        return;
    };
    state.active_wizard = ActiveWizard::AimAsteroid(AimAsteroidInput::EnterSpeed {
        asteroid_id: asteroid_id.clone(),
        asteroid_name: asteroid_name.clone(),
        target,
        // The server's own ceiling as the default: it is the only speed the
        // pilot can name without knowing the scale.
        buf: "0.10".into(),
        error: None,
    });
}

fn enter_speed(code: KeyCode, state: &mut AppState) {
    let ActiveWizard::AimAsteroid(AimAsteroidInput::EnterSpeed { .. }) = &state.active_wizard else {
        return;
    };
    match code {
        KeyCode::Backspace => {
            if let ActiveWizard::AimAsteroid(AimAsteroidInput::EnterSpeed { buf, error, .. }) = &mut state.active_wizard
            {
                buf.pop();
                *error = None;
            }
        }
        KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
            if let ActiveWizard::AimAsteroid(AimAsteroidInput::EnterSpeed { buf, error, .. }) = &mut state.active_wizard
            {
                if buf.len() < 6 {
                    buf.push(c);
                }
                *error = None;
            }
        }
        KeyCode::Enter => {
            let ActiveWizard::AimAsteroid(AimAsteroidInput::EnterSpeed {
                asteroid_id,
                asteroid_name,
                target,
                buf,
                ..
            }) = &state.active_wizard
            else {
                return;
            };
            match parse_speed(buf) {
                Ok(speed) => {
                    let step = impact_confirm(asteroid_id.clone(), asteroid_name.clone(), target, speed);
                    state.active_wizard = ActiveWizard::AimAsteroid(step);
                }
                Err(msg) => {
                    if let ActiveWizard::AimAsteroid(AimAsteroidInput::EnterSpeed { error, .. }) =
                        &mut state.active_wizard
                    {
                        *error = Some(msg);
                    }
                }
            }
        }
        _ => {}
    }
}

/// The server's bound, checked here: `(0, 0.5]` fractions of c.
pub fn parse_speed(buf: &str) -> Result<f64, String> {
    let v: f64 = buf.trim().parse().map_err(|_| "not a number".to_string())?;
    if !v.is_finite() || v <= 0.0 {
        return Err("a speed has to be above zero".into());
    }
    if v > MAX_TARGET_SPEED_C {
        return Err(format!("the engine tops out at {MAX_TARGET_SPEED_C}c"));
    }
    Ok(v)
}

fn impact_confirm(asteroid_id: String, asteroid_name: String, target: &ImpactTarget, speed: f64) -> AimAsteroidInput {
    AimAsteroidInput::Confirm {
        summary: format!("system impact on «{}» at {speed}c", target.name),
        // A body in this system belongs to whoever is standing on it. The
        // cockpit does not refuse the shot — the API allows it — but it does
        // not let it happen without saying what it is.
        warning: Some("the tank is spent on launch and cannot be recalled".into()),
        body: serde_json::json!({
            "mode": "system_impact",
            "targetObjectId": target.id,
            "targetSpeedC": speed,
        }),
        asteroid_id,
        asteroid_name,
    }
}

fn pick_heading(code: KeyCode, state: &mut AppState) {
    let ActiveWizard::AimAsteroid(AimAsteroidInput::PickHeading {
        asteroid_id,
        asteroid_name,
        headings,
        selection,
    }) = &state.active_wizard
    else {
        return;
    };
    if is_list_nav_key(code) {
        let new_sel = list_nav(code, *selection, headings.len());
        if let ActiveWizard::AimAsteroid(AimAsteroidInput::PickHeading { selection, .. }) = &mut state.active_wizard {
            if let Some(v) = new_sel {
                *selection = v;
            }
        }
        return;
    }
    if code != KeyCode::Enter {
        return;
    }
    let Some(h) = headings.get(*selection).cloned() else {
        return;
    };
    let step = heading_confirm(asteroid_id.clone(), asteroid_name.clone(), &h);
    state.active_wizard = ActiveWizard::AimAsteroid(step);
}

fn heading_confirm(asteroid_id: String, asteroid_name: String, h: &Heading) -> AimAsteroidInput {
    AimAsteroidInput::Confirm {
        summary: format!("heading through ({}, {}, {})", h.x, h.y, h.z),
        // The single most misread thing about this mode, stated where it
        // cannot be missed: the neighbour is not where the asteroid stops.
        warning: Some(
            "a heading, not a destination — the asteroid crosses one sector a day until something captures it".into(),
        ),
        body: serde_json::json!({
            "mode": "sector_transfer",
            "target": { "x": h.x, "y": h.y, "z": h.z },
        }),
        asteroid_id,
        asteroid_name,
    }
}

fn confirm(code: KeyCode, state: &mut AppState, client: &ApiClient, tx: &mpsc::Sender<ApiMessage>) {
    if !matches!(code, KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y')) {
        return;
    }
    let ActiveWizard::AimAsteroid(AimAsteroidInput::Confirm {
        asteroid_id,
        asteroid_name,
        summary,
        body,
        ..
    }) = &state.active_wizard
    else {
        return;
    };
    let (asteroid_id, asteroid_name, summary, body) = (
        asteroid_id.clone(),
        asteroid_name.clone(),
        summary.clone(),
        body.clone(),
    );
    let Some(probe_id) = state.probe_id() else {
        state.set_error("waiting for a probe sync".to_string());
        state.close_wizard();
        return;
    };
    fetch_launch_trajectory(probe_id, asteroid_id, body, client.clone(), tx.clone());
    state.log_event(LogEvent::launch_asteroid(
        &asteroid_name,
        &summary,
        state.active_probe_id,
    ));
    state.close_wizard();
}

#[cfg(test)]
mod tests {
    use super::parse_speed;

    #[test]
    fn the_speed_bound_is_enforced_before_the_request() {
        // `(0, 0.5]` fractions of c. The pilot meets the bound as a form that
        // will not take a bad value, rather than as
        // `invalid_asteroid_target_speed` after the tank is already committed.
        assert_eq!(parse_speed("0.25"), Ok(0.25));
        assert_eq!(parse_speed(" 0.5 "), Ok(0.5), "the bound is inclusive");
        assert!(parse_speed("0").is_err(), "zero is not a launch");
        assert!(parse_speed("0.51").is_err());
        assert!(parse_speed("-0.2").is_err());
        assert!(parse_speed("fast").is_err());
        assert!(parse_speed("").is_err());
    }
}
