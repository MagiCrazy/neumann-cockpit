//! Headless status views (`--status <view>`, issue #229).
//!
//! The third non-TUI surface, after the script runner and the API diagnostic:
//! a read-only report of live game state, printed and gone. It exists so the
//! cockpit can answer questions without being *opened* — a statusline, a
//! monitoring check, a shell script deciding whether it is worth logging in.
//!
//! Two constraints shape the whole module:
//!
//! - **Each view fetches only what it needs.** The cockpit's `fetch_all` fires
//!   seven requests; a statusline polling `--status probe` every ten seconds
//!   through that would eat the per-token window on its own (the server meters
//!   ~120 requests a minute per bearer token, API v104). So `probe` costs one
//!   request, `mannies` one, `sector` one — and the report says how many it
//!   spent, because a caller putting this in a loop deserves to know.
//! - **`--json` is a contract.** Once someone builds a statusline on it,
//!   renaming a field breaks their bar. The shapes below are meant to be
//!   stable, and the tests pin the key names rather than just the values.
//!
//! Plain text, never ANSI: the human output is piped as often as it is read.

use anyhow::{bail, Result};
use serde_json::{json, Value};

use crate::api::client::ApiClient;
use crate::api::types::{
    Manny, MannyTaskVisibility, MovementPhase, Probe, ProbeStatus, ScutCoverageStatus, SectorObservation,
};
use crate::config::{Config, ConfigStatus};

/// Which slice of the game state to print.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Probe,
    Sector,
    Mannies,
    Scut,
    /// Probe, sector and mannies together — the "what is going on" view, at
    /// the cost of three requests instead of one.
    All,
}

impl View {
    pub fn from_label(label: &str) -> Option<Self> {
        match label.trim().to_ascii_lowercase().as_str() {
            "probe" => Some(View::Probe),
            "sector" => Some(View::Sector),
            "mannies" | "manny" => Some(View::Mannies),
            "scut" => Some(View::Scut),
            "all" => Some(View::All),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            View::Probe => "probe",
            View::Sector => "sector",
            View::Mannies => "mannies",
            View::Scut => "scut",
            View::All => "all",
        }
    }

    /// Every view a caller may ask for, for the usage line.
    pub const ALL: [View; 5] = [View::Probe, View::Sector, View::Mannies, View::Scut, View::All];
}

/// A parsed `--status` launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusRequest {
    pub view: View,
    /// `--json`: machine output instead of the human report.
    pub json: bool,
}

/// The `--status <view>` / `--status=<view>` argument, if present.
///
/// Returns `None` for `latency`, which is the diagnostic's own alias and is
/// resolved before this (`headless::diagnostic_arg`). Saying so here rather
/// than relying on the call order keeps the two from quietly diverging.
pub fn status_arg(args: &[String]) -> Option<StatusRequest> {
    let json = args.iter().any(|a| a == "--json");
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        let value = if let Some(v) = a.strip_prefix("--status=") {
            Some(v.to_string())
        } else if a == "--status" {
            it.next().cloned()
        } else {
            None
        };
        if let Some(value) = value {
            if value.eq_ignore_ascii_case("latency") {
                return None; // the diagnostic owns this one
            }
            return View::from_label(&value).map(|view| StatusRequest { view, json });
        }
    }
    None
}

/// Whether the launch asked for a status view the parser did not recognise, so
/// `main` can say what the choices are instead of opening the cockpit on a
/// typo — a bare `--status` in a monitoring script is a mistake, not a request
/// for the TUI.
pub fn unknown_status_view(args: &[String]) -> Option<String> {
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        let value = if let Some(v) = a.strip_prefix("--status=") {
            Some(v.to_string())
        } else if a == "--status" {
            Some(it.next().cloned().unwrap_or_default())
        } else {
            None
        };
        if let Some(value) = value {
            if value.eq_ignore_ascii_case("latency") || View::from_label(&value).is_some() {
                return None;
            }
            return Some(value);
        }
    }
    None
}

/// The usage line for an unrecognised view.
pub fn usage() -> String {
    let views: Vec<&str> = View::ALL.iter().map(|v| v.label()).collect();
    format!("usage: --status <{}|latency> [--json]", views.join("|"))
}

/// Run one status view. Returns the process exit code: `0` when the report was
/// produced, `1` when the data could not be fetched.
pub async fn run(request: StatusRequest) -> Result<i32> {
    let config = match Config::load_status() {
        ConfigStatus::Ready(c) => c,
        _ => bail!("no valid config — launch the cockpit once to set your API key"),
    };
    let client = ApiClient::new(config.base_url.clone(), config.api_key.clone())?;

    match collect(&client, request.view).await {
        Ok(report) => {
            if request.json {
                println!("{}", serde_json::to_string_pretty(&report.json)?);
            } else {
                print!("{}", report.text);
            }
            Ok(0)
        }
        Err(e) => {
            // The failure goes to stderr so a `--json` caller's parser is not
            // handed prose, and the exit code carries the verdict.
            eprintln!("status: {e}");
            if request.json {
                println!("{}", json!({ "error": e.to_string() }));
            }
            Ok(1)
        }
    }
}

/// A rendered report in both shapes, so the caller picks without re-fetching.
struct Report {
    text: String,
    json: Value,
}

async fn collect(client: &ApiClient, view: View) -> Result<Report> {
    match view {
        View::Probe => {
            let probe = client.get_probe().await?;
            Ok(Report {
                text: probe_text(&probe),
                json: json!({ "requests": 1, "probe": probe_json(&probe) }),
            })
        }
        View::Sector => {
            let sector = client.get_probe_sector().await?;
            Ok(Report {
                text: sector_text(&sector),
                json: json!({ "requests": 1, "sector": sector_json(&sector) }),
            })
        }
        View::Mannies => {
            let roster = client.get_mannies().await?;
            Ok(Report {
                text: mannies_text(&roster.mannies),
                json: json!({ "requests": 1, "mannies": roster.mannies.iter().map(manny_json).collect::<Vec<_>>() }),
            })
        }
        View::Scut => {
            let sector = client.get_probe_sector().await?;
            // One extra request only when exactly one network covers this
            // sector — the same rule the cockpit's inspector uses, and it
            // keeps the view at two requests rather than N+1.
            let detail = match sector.scut_networks.as_slice() {
                [only] => client.get_scut_network(only.id).await.ok(),
                _ => None,
            };
            let requests = if detail.is_some() { 2 } else { 1 };
            let relays: Vec<Value> = detail
                .iter()
                .flat_map(|n| n.relays.iter())
                .map(|r| {
                    json!({
                        "id": r.id,
                        "name": r.name,
                        "active": matches!(r.status, crate::api::types::ScutRelayStatus::On),
                        "transit_beacon": r.is_transit_beacon,
                    })
                })
                .collect();
            Ok(Report {
                text: scut_text(&sector, detail.as_ref()),
                json: json!({
                    "requests": requests,
                    "scut": {
                        "coverage": coverage_label(&sector),
                        "networks": sector.scut_networks.iter()
                            .map(|n| json!({ "id": n.id, "name": n.name }))
                            .collect::<Vec<_>>(),
                        "relays": relays,
                    }
                }),
            })
        }
        View::All => {
            let probe = client.get_probe().await?;
            let sector = client.get_probe_sector().await?;
            let roster = client.get_mannies().await?;
            let text = format!(
                "{}\n{}\n{}",
                probe_text(&probe),
                sector_text(&sector),
                mannies_text(&roster.mannies)
            );
            Ok(Report {
                text,
                json: json!({
                    "requests": 3,
                    "probe": probe_json(&probe),
                    "sector": sector_json(&sector),
                    "mannies": roster.mannies.iter().map(manny_json).collect::<Vec<_>>(),
                }),
            })
        }
    }
}

// ── formatting ────────────────────────────────────────────────────────────
//
// `label  value`, one fact per line, aligned on a fixed column. Not a table:
// the output is piped into `grep`, `awk` and statusline scripts at least as
// often as it is read, and a fixed column is what makes that easy.

const LABEL_WIDTH: usize = 12;

fn row(label: &str, value: impl std::fmt::Display) -> String {
    format!("{label:<LABEL_WIDTH$}{value}\n")
}

fn coords(v: Option<&crate::api::types::Vector>) -> String {
    v.map(|c| format!("{} {} {}", c.x as i64, c.y as i64, c.z as i64))
        .unwrap_or_else(|| "unknown".into())
}

fn pct(part: Option<f64>, whole: Option<f64>) -> String {
    match (part, whole) {
        (Some(p), Some(w)) if w > 0.0 => format!("{p:.1}/{w:.0}  {:.0}%", p / w * 100.0),
        (Some(p), _) => format!("{p:.1}"),
        _ => "unknown".into(),
    }
}

/// Whether a movement is actually under way.
///
/// The server keeps reporting the **last** movement after it completes, so a
/// probe sitting still comes back carrying a `movement` whose arrival is in
/// the past — printing "travelling" from its mere presence says the probe is
/// moving when it is not. The cockpit's own probe pane reads the phase for
/// exactly this reason (`panels/probe.rs`), and so does this.
fn is_travelling(m: &crate::api::types::ProbeMovement) -> bool {
    !matches!(
        m.phase.as_ref().unwrap_or(&m.status),
        MovementPhase::Arrived | MovementPhase::Failed | MovementPhase::Destroyed | MovementPhase::Idle
    )
}

/// The API's own status word, lower-cased. Deriving it beats a hand-written
/// match, which would report a newly added variant as "unknown".
fn status_label(status: &ProbeStatus) -> String {
    let raw = format!("{status:?}");
    // `TrappedByBlackHole` → `trapped_by_black_hole`
    let mut out = String::new();
    for (i, c) in raw.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.extend(c.to_lowercase());
    }
    out
}

fn probe_text(probe: &Probe) -> String {
    let mut out = String::from("PROBE\n");
    out.push_str(&row("name", &probe.name));
    out.push_str(&row(
        "status",
        // Rendered from the enum rather than mapped by hand: a variant added
        // to the API would otherwise silently print as "unknown".
        status_label(&probe.status),
    ));
    out.push_str(&row(
        "sector",
        coords(probe.sector.as_ref().and_then(|s| s.relative.as_ref())),
    ));
    out.push_str(&row("fuel", pct(probe.fuel.deuterium, probe.fuel.max_deuterium)));
    out.push_str(&row(
        "integrity",
        probe
            .systems
            .as_ref()
            .and_then(|s| s.integrity_percent)
            .map(|i| format!("{i:.0}%"))
            .unwrap_or_else(|| "unknown".into()),
    ));
    out.push_str(&row(
        "cargo",
        pct(Some(probe.inventory.used_capacity), Some(probe.inventory.capacity)),
    ));
    if let Some(m) = probe.movement.as_ref().filter(|m| is_travelling(m)) {
        let left = (m.arrival_at - chrono::Utc::now()).num_seconds().max(0);
        out.push_str(&row("travelling", coords(Some(&m.target))));
        out.push_str(&row("arrival", crate::ui::theme::format_duration(left)));
    }
    if probe.alert.is_some() {
        out.push_str(&row("alert", "recovery needed — see the cockpit"));
    }
    out
}

fn probe_json(probe: &Probe) -> Value {
    json!({
        "id": probe.id,
        "name": probe.name,
        "status": status_label(&probe.status),
        "sector": probe.sector.as_ref().and_then(|s| s.relative.as_ref())
            .map(|c| json!([c.x as i64, c.y as i64, c.z as i64])),
        "fuel": { "deuterium": probe.fuel.deuterium, "max": probe.fuel.max_deuterium },
        "integrity_percent": probe.systems.as_ref().and_then(|s| s.integrity_percent),
        "cargo": { "used": probe.inventory.used_capacity, "capacity": probe.inventory.capacity },
        "travelling": probe.movement.as_ref().filter(|m| is_travelling(m)).map(|m| json!({
            "target": [m.target.x as i64, m.target.y as i64, m.target.z as i64],
            "arrival_at": m.arrival_at.to_rfc3339(),
            "seconds_remaining": (m.arrival_at - chrono::Utc::now()).num_seconds().max(0),
        })),
        "needs_recovery": probe.alert.is_some(),
    })
}

fn sector_text(sector: &SectorObservation) -> String {
    let mut out = String::from("SECTOR\n");
    let c = &sector.relative_coordinates;
    out.push_str(&row("at", format!("{} {} {}", c.x as i64, c.y as i64, c.z as i64)));
    let objects = sector.objects.as_deref().unwrap_or_default();
    out.push_str(&row("objects", objects.len()));
    for o in objects {
        let name = o
            .name
            .clone()
            .unwrap_or_else(|| format!("{:?}", o.object_type).to_lowercase());
        let kind = format!("{:?}", o.object_type).to_lowercase();
        out.push_str(&format!("  {kind:<18}{name}\n"));
    }
    out
}

fn sector_json(sector: &SectorObservation) -> Value {
    let c = &sector.relative_coordinates;
    json!({
        "at": [c.x as i64, c.y as i64, c.z as i64],
        "objects": sector.objects.as_deref().unwrap_or_default().iter().map(|o| json!({
            "id": o.id,
            "type": format!("{:?}", o.object_type).to_lowercase(),
            "name": o.name,
        })).collect::<Vec<_>>(),
    })
}

fn mannies_text(mannies: &[Manny]) -> String {
    let mut out = String::from("MANNIES\n");
    out.push_str(&row("count", mannies.len()));
    for m in mannies {
        let task = m
            .current_task
            .as_ref()
            .map(|t| format!("{t:?}").to_lowercase())
            .unwrap_or_else(|| "idle".into());
        let progress = if m.current_task.is_some() {
            format!("  {:.0}%", m.task_progress_percent)
        } else {
            String::new()
        };
        let remote = match &m.task_visibility {
            Some(MannyTaskVisibility::ScutNetwork) => "  via scut",
            Some(MannyTaskVisibility::TooFar) => "  too far",
            _ => "",
        };
        out.push_str(&format!("  {:<16}{task}{progress}{remote}\n", m.name));
    }
    out
}

fn manny_json(m: &Manny) -> Value {
    json!({
        "id": m.id,
        "name": m.name,
        "task": m.current_task.as_ref().map(|t| format!("{t:?}").to_lowercase()),
        "progress_percent": m.current_task.as_ref().map(|_| m.task_progress_percent),
        "can_receive_orders": m.can_receive_orders,
        "visibility": m.task_visibility.as_ref().map(|v| format!("{v:?}").to_lowercase()),
    })
}

fn coverage_label(sector: &SectorObservation) -> &'static str {
    match sector.scut_coverage_status {
        Some(ScutCoverageStatus::Covered) => "covered",
        Some(ScutCoverageStatus::Uncovered) => "uncovered",
        // The server omits the network list entirely while coverage is
        // unknown, so "no networks" is not the same answer as "uncovered".
        _ => "unknown",
    }
}

fn scut_text(sector: &SectorObservation, detail: Option<&crate::api::types::ScutNetwork>) -> String {
    let mut out = String::from("SCUT\n");
    out.push_str(&row("coverage", coverage_label(sector)));
    out.push_str(&row("networks", sector.scut_networks.len()));
    for n in &sector.scut_networks {
        out.push_str(&format!("  {:<18}#{}\n", n.name, n.id));
    }
    if let Some(network) = detail {
        out.push_str(&row("relays", network.relays.len()));
        for r in &network.relays {
            let active = matches!(r.status, crate::api::types::ScutRelayStatus::On);
            let beacon = if r.is_transit_beacon { "  beacon" } else { "" };
            out.push_str(&format!(
                "  {:<18}{}{beacon}\n",
                r.name,
                if active { "active" } else { "inactive" }
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        std::iter::once("neumann-cockpit".to_string())
            .chain(list.iter().map(|s| s.to_string()))
            .collect()
    }

    #[test]
    fn both_argument_shapes_are_accepted() {
        assert_eq!(
            status_arg(&args(&["--status", "probe"])),
            Some(StatusRequest {
                view: View::Probe,
                json: false
            })
        );
        assert_eq!(
            status_arg(&args(&["--status=mannies"])),
            Some(StatusRequest {
                view: View::Mannies,
                json: false
            })
        );
        assert_eq!(
            status_arg(&args(&["--status", "SECTOR"])),
            Some(StatusRequest {
                view: View::Sector,
                json: false
            }),
            "case-insensitive, like every other label in the cockpit"
        );
    }

    #[test]
    fn json_is_recognised_on_either_side() {
        for form in [
            vec!["--status", "probe", "--json"],
            vec!["--json", "--status", "probe"],
            vec!["--status=probe", "--json"],
        ] {
            assert_eq!(
                status_arg(&args(&form)),
                Some(StatusRequest {
                    view: View::Probe,
                    json: true
                }),
                "{form:?}"
            );
        }
    }

    #[test]
    fn latency_is_left_to_the_diagnostic() {
        // It is the diagnostic's own alias, resolved before this. Saying so
        // here keeps the two parsers from quietly diverging.
        assert_eq!(status_arg(&args(&["--status", "latency"])), None);
        assert_eq!(status_arg(&args(&["--status=latency"])), None);
        assert_eq!(unknown_status_view(&args(&["--status", "latency"])), None);
    }

    #[test]
    fn a_typo_is_reported_rather_than_opening_the_cockpit() {
        // A bare or misspelt `--status` in a monitoring script is a mistake;
        // launching the TUI over it would hang a cron job.
        assert_eq!(status_arg(&args(&["--status", "prob"])), None);
        assert_eq!(
            unknown_status_view(&args(&["--status", "prob"])).as_deref(),
            Some("prob")
        );
        assert_eq!(unknown_status_view(&args(&["--status"])).as_deref(), Some(""));
        assert!(usage().contains("probe"));
        assert!(
            usage().contains("latency"),
            "the diagnostic alias is a valid answer too"
        );
    }

    #[test]
    fn an_ordinary_launch_asks_for_no_view() {
        assert_eq!(status_arg(&args(&[])), None);
        assert_eq!(status_arg(&args(&["--json"])), None, "--json alone is not a request");
        assert_eq!(unknown_status_view(&args(&[])), None);
    }

    #[test]
    fn every_view_round_trips_through_its_label() {
        for view in View::ALL {
            assert_eq!(View::from_label(view.label()), Some(view));
            assert!(usage().contains(view.label()), "{} missing from usage", view.label());
        }
        assert_eq!(View::from_label("manny"), Some(View::Mannies), "the singular works too");
    }

    #[test]
    fn the_human_report_is_plain_and_aligned() {
        // Piped as often as read: no ANSI, one fact per line, fixed column.
        let probe: Probe = serde_json::from_str(
            r#"{"id": 1, "name": "Grey Area", "status": "idle",
                "fuel": {"deuterium": 42.5, "maxDeuterium": 100.0}, "sensorMode": "normal",
                "sector": {"relative": {"x": 4.0, "y": -2.0, "z": 0.0}},
                "movement": null, "systems": {"integrityPercent": 80.0},
                "inventory": {"capacity": 10.0, "usedCapacity": 2.5, "freeCapacity": 7.5,
                    "items": [], "resourceStocks": [], "externalTanks": [], "containers": []}}"#,
        )
        .unwrap();
        let text = probe_text(&probe);
        assert!(!text.contains('\u{1b}'), "no escape sequences: {text:?}");
        assert!(text.contains("Grey Area"));
        assert!(text.contains("4 -2 0"), "coordinates in the frame `move` takes: {text}");
        assert!(text.contains("80%"));
        for line in text.lines().skip(1).filter(|l| !l.starts_with("  ")) {
            assert!(line.len() > LABEL_WIDTH, "every row carries a value: {line:?}");
        }
    }

    #[test]
    fn the_json_shape_is_the_contract() {
        // Someone's statusline will depend on these key names; renaming one
        // silently breaks their bar, so the names are pinned here.
        let probe: Probe = serde_json::from_str(
            r#"{"id": 7, "name": "Grey Area", "status": "idle",
                "fuel": {"deuterium": 42.5, "maxDeuterium": 100.0}, "sensorMode": "normal",
                "sector": {"relative": {"x": 4.0, "y": -2.0, "z": 0.0}},
                "movement": null, "systems": {"integrityPercent": 80.0},
                "inventory": {"capacity": 10.0, "usedCapacity": 2.5, "freeCapacity": 7.5,
                    "items": [], "resourceStocks": [], "externalTanks": [], "containers": []}}"#,
        )
        .unwrap();
        let v = probe_json(&probe);
        assert_eq!(v["id"], 7);
        assert_eq!(v["name"], "Grey Area");
        assert_eq!(v["status"], "idle");
        assert_eq!(v["sector"], json!([4, -2, 0]));
        assert_eq!(v["fuel"]["deuterium"], 42.5);
        assert_eq!(v["integrity_percent"], 80.0);
        assert_eq!(v["cargo"]["used"], 2.5);
        assert_eq!(v["needs_recovery"], false);
        assert!(v["travelling"].is_null(), "a stationary probe travels nowhere");
    }

    /// A probe carrying a movement in the given phase.
    fn probe_with_movement(phase: &str) -> Probe {
        serde_json::from_str(&format!(
            r#"{{"id": 1, "name": "Grey Area", "status": "idle",
                "fuel": {{"deuterium": 42.5, "maxDeuterium": 100.0}}, "sensorMode": "normal",
                "sector": {{"relative": {{"x": 4.0, "y": -2.0, "z": 0.0}}}},
                "movement": {{"status": "{phase}",
                    "origin": {{"x": 0.0, "y": 0.0, "z": 0.0}},
                    "target": {{"x": 4.0, "y": -2.0, "z": 0.0}},
                    "distance": 6, "fuelCostDeuterium": 3.0,
                    "startedAt": "2026-09-06T10:00:00Z", "arrivalAt": "2026-09-06T10:30:00Z"}},
                "systems": {{"integrityPercent": 80.0}},
                "inventory": {{"capacity": 10.0, "usedCapacity": 2.5, "freeCapacity": 7.5,
                    "items": [], "resourceStocks": [], "externalTanks": [], "containers": []}}}}"#
        ))
        .unwrap()
    }

    #[test]
    fn a_finished_movement_is_not_reported_as_travelling() {
        // Found by running the view against the live server: the API keeps
        // reporting the **last** movement after it completes, so a probe
        // sitting still comes back carrying one whose arrival is in the past.
        // Printing "travelling" from its mere presence says the probe is
        // moving when it is not.
        for done in ["arrived", "idle", "failed", "destroyed"] {
            let probe = probe_with_movement(done);
            let text = probe_text(&probe);
            assert!(!text.contains("travelling"), "phase {done} is not a journey: {text}");
            assert!(probe_json(&probe)["travelling"].is_null(), "nor in JSON, for {done}");
        }

        for under_way in ["accelerating", "cruising", "decelerating", "preparing"] {
            let probe = probe_with_movement(under_way);
            assert!(
                probe_text(&probe).contains("travelling"),
                "phase {under_way} is a journey"
            );
            assert!(!probe_json(&probe)["travelling"].is_null());
        }
    }

    #[test]
    fn a_status_word_the_api_adds_later_still_reads() {
        // Derived from the enum, so a variant we have not hand-mapped prints
        // as itself rather than as "unknown".
        assert_eq!(status_label(&ProbeStatus::Idle), "idle");
        assert_eq!(status_label(&ProbeStatus::TrappedByBlackHole), "trapped_by_black_hole");
        assert_eq!(status_label(&ProbeStatus::Decelerating), "decelerating");
    }

    #[test]
    fn unknown_scut_coverage_is_not_reported_as_uncovered() {
        // The server omits the network list entirely while coverage is
        // unknown, so an empty list alone means nothing (API v104).
        let sector: SectorObservation = serde_json::from_str(
            r#"{"relativeCoordinates": {"x": 0.0, "y": 0.0, "z": 0.0}, "distance": 0,
                 "knowledgeLevel": "detailed", "confidence": 1.0,
                 "scan": {"currentSectorResidenceSeconds": 60,
                          "requiredResidenceSeconds": 60, "scanQuality": 1.0}}"#,
        )
        .unwrap();
        assert_eq!(coverage_label(&sector), "unknown");
        assert!(scut_text(&sector, None).contains("unknown"));
    }
}
