// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-092-AC-28 (TC-483): the emitter writes each source dimension's
//! explicit `base` flag on the checked-package wire, so IR reads a base
//! dimension as base and a derived dimension whose terms cancel to empty as
//! derived.

use quire_contract_model::{
    read_checked_package, CheckedNodeId, CheckedPackageDispatchResult, CheckedPackageEvidence,
    CheckedPackageReadLimits, CheckedTerm,
};

use super::*;

const SOURCE: &str = "dimension L; dimension N = L / L; dimension M = L * L;\n\
    unit m : L = rational(1, 1); unit one : N = rational(1, 1);\n\
    unit m2 : M = rational(1, 1);\n\
    function f using v(a: Quantity<m>, b: Quantity<one>, c: Quantity<m2>): Boolean pure { true }";

/// FR-092-AC-29: `dimension L` is written `base: true`, the empty derived
/// `dimension N = L / L` is written `base: false` with no terms, and
/// `dimension M = L * L` is written `base: false` with one term; IR's
/// strict V2 reader admits the package and reports the same flags.
#[trace("FR-092-AC-29", "TC-483")]
#[test]
fn an_emitted_dimension_carries_its_base_flag_to_ir() {
    let (_, emission) = emit_from_text(SOURCE);
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let CheckedPackageDispatchResult::AdmittedV2(package) = read_checked_package(
        emission.package.bytes(),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    ) else {
        panic!("IR admits the emitted package");
    };
    let dimension = |name: &str| {
        let node = nodes(&wire)
            .iter()
            .find(|node| node["declaration"]["qualified_name"] == json!([name]))
            .unwrap_or_else(|| panic!("{name} is written"));
        assert_eq!(node["semantic_form"], json!("dimension"));
        let id: CheckedNodeId = serde_json::from_value(node["node_id"].clone()).unwrap();
        let Some(term @ CheckedTerm::NominalBody(body)) = package.term(&id) else {
            panic!("IR reads {name} as a nominal body");
        };
        let written = serde_json::to_value(term).unwrap();
        assert_eq!(
            body.dimension_base().map(Value::from),
            Some(written["base"].clone())
        );
        (
            body.dimension_base(),
            written["terms"].as_array().unwrap().len(),
        )
    };
    assert_eq!(dimension("L"), (Some(true), 0));
    assert_eq!(dimension("N"), (Some(false), 0));
    assert_eq!(dimension("M"), (Some(false), 1));
}
