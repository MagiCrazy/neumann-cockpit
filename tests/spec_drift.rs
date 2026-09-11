//! Drift guards between the vendored OpenAPI spec and the constants that
//! mirror it by hand.
//!
//! A hand-copied mirror of a server-side enum has no way to notice when the
//! server's copy grows: `SHAREABLE_BLUEPRINTS` stayed at the three values of
//! v116 while the share path grew to five, and the only symptom was two
//! blueprints silently missing from the wizard (#359). These tests read the
//! spec back out of `api-specs/` at test time, so vendoring a newer version
//! whose enum has grown fails the build instead of going unnoticed.
//!
//! The **newest** vendored spec is the witness, resolved at runtime rather
//! than named in a constant: a guard pinned to `v130.yaml` would keep passing
//! the day `v140.yaml` lands, which is exactly the failure it exists to catch.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

/// Path and version of the highest-numbered `api-specs/vN.yaml`.
fn newest_spec() -> (u32, PathBuf) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("api-specs");
    let mut best: Option<(u32, PathBuf)> = None;
    for entry in fs::read_dir(&dir).expect("api-specs/ is missing") {
        let path = entry.expect("unreadable directory entry").path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(version) = name
            .strip_prefix('v')
            .and_then(|rest| rest.strip_suffix(".yaml"))
            .and_then(|digits| digits.parse::<u32>().ok())
        else {
            continue;
        };
        if best.as_ref().is_none_or(|(seen, _)| version > *seen) {
            best = Some((version, path));
        }
    }
    best.expect("no vN.yaml found under api-specs/")
}

/// The inline `enum: [a, b, c]` of a named path parameter, as the spec spells
/// it. Deliberately a targeted scan rather than a YAML parse: one line of one
/// known block, and a shape change in the spec should fail loudly here too.
fn path_parameter_enum(spec: &str, path: &str, parameter: &str) -> Vec<String> {
    let after_path = spec
        .split_once(&format!("\n  {path}:\n"))
        .unwrap_or_else(|| panic!("path {path} not found in the spec"))
        .1;
    let after_param = after_path
        .split_once(&format!("- name: {parameter}\n"))
        .unwrap_or_else(|| panic!("parameter {parameter} not found under {path}"))
        .1;
    let line = after_param
        .lines()
        .take_while(|l| !l.trim_start().starts_with("- name:"))
        .find_map(|l| l.trim().strip_prefix("enum: ["))
        .unwrap_or_else(|| panic!("no inline enum for {parameter} under {path}"));
    let list = line
        .strip_suffix(']')
        .unwrap_or_else(|| panic!("unterminated enum for {parameter} under {path}"));
    list.split(',').map(|v| v.trim().to_string()).collect()
}

#[test]
fn shareable_blueprints_match_the_vendored_spec() {
    let (version, path) = newest_spec();
    let spec = fs::read_to_string(&path).expect("the newest spec is unreadable");

    let from_spec: BTreeSet<String> = path_parameter_enum(
        &spec,
        "/api/probe/{probeId}/probe-improvement-blueprints/{improvementId}/share",
        "improvementId",
    )
    .into_iter()
    .collect();
    let from_cockpit: BTreeSet<String> = neumann_cockpit::app::SHAREABLE_BLUEPRINTS
        .iter()
        .map(|id| (*id).to_string())
        .collect();

    assert_eq!(
        from_cockpit, from_spec,
        "SHAREABLE_BLUEPRINTS disagrees with the improvementId enum of api-specs/v{version}.yaml — \
         the server's list has changed, so the share wizard is offering the wrong set"
    );
}
