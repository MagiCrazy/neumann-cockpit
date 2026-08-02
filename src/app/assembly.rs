//! Probe assembly bills (API v81, per-model since v104).
//!
//! A drone is assembled by a Manny from two empty additional containers plus a
//! fixed component bill. Since v104 the `assemble-probe` task takes a
//! [`ProbeModel`]: the `deuterium_tanker` costs the generic bill **plus** its
//! own extras, and carries a 400-point deuterium tank instead of 100.
//!
//! The bill is server-enforced; it lives here so the assembly wizard can show
//! the cost before committing the ~3 h task and `:tree` can price it down to
//! base resources. Component ids match crafting-recipe ids, so they resolve
//! through the ordinary roll-up.

use crate::api::types::ProbeModel;

/// One line of a bill: a component id (a crafting-recipe id) and how many.
pub type BillLine = (&'static str, f64);

/// Components every model needs, on top of the two empty containers.
const BILL_GENERIC: &[BillLine] = &[
    ("deuterium_engine", 1.0),
    ("scut_relay", 1.0),
    ("electric_motor", 5.0),
    ("atomic_printer_part", 2.0),
    ("solar_panel", 4.0),
];

/// What a `deuterium_tanker` costs on top of the generic bill.
const BILL_TANKER_EXTRA: &[BillLine] = &[
    ("steel_plate", 10.0),
    ("linear_actuator", 2.0),
    ("integrated_circuit", 1.0),
];

/// How long the assemble-probe task runs (~3 h), for the tech-tree estimate.
pub const ASSEMBLY_SECONDS: i64 = 3 * 3600;

/// The models the pilot can assemble, in wizard order.
pub const ASSEMBLABLE_MODELS: [ProbeModel; 2] = [ProbeModel::Generic, ProbeModel::DeuteriumTanker];

/// The full component bill for `model`, generic lines first.
pub fn assembly_bill(model: ProbeModel) -> Vec<BillLine> {
    let mut bill = BILL_GENERIC.to_vec();
    if model == ProbeModel::DeuteriumTanker {
        bill.extend_from_slice(BILL_TANKER_EXTRA);
    }
    bill
}

/// Pilot-facing model name.
pub fn model_label(model: ProbeModel) -> &'static str {
    match model {
        ProbeModel::Generic => "generic",
        ProbeModel::DeuteriumTanker => "deuterium tanker",
        ProbeModel::Unknown => "unknown model",
    }
}

/// One-line description of what a model is for, shown next to it in the picker.
pub fn model_blurb(model: ProbeModel) -> &'static str {
    match model {
        ProbeModel::Generic => "standard hull · 100-point deuterium tank",
        ProbeModel::DeuteriumTanker => "400-point deuterium tank · costlier bill",
        ProbeModel::Unknown => "",
    }
}

/// The wire value for the `model` field of the assemble-probe request.
pub fn model_wire(model: ProbeModel) -> &'static str {
    match model {
        ProbeModel::Generic => "generic",
        ProbeModel::DeuteriumTanker => "deuterium_tanker",
        // Never sent: the picker only offers the two known models. Falling back
        // to the server default beats inventing a value it would 422 on.
        ProbeModel::Unknown => "generic",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tanker_bill_extends_the_generic_one() {
        let generic = assembly_bill(ProbeModel::Generic);
        let tanker = assembly_bill(ProbeModel::DeuteriumTanker);
        assert_eq!(generic.len(), 5);
        assert_eq!(tanker.len(), 8);
        assert!(
            tanker.starts_with(&generic),
            "the tanker pays the generic bill plus its extras"
        );
        assert_eq!(
            tanker[5..].to_vec(),
            vec![
                ("steel_plate", 10.0),
                ("linear_actuator", 2.0),
                ("integrated_circuit", 1.0)
            ]
        );
    }
}
