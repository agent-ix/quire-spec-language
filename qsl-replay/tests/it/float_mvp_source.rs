// SPDX-License-Identifier: AGPL-3.0-or-later
//! IEEE MVP source admission; downstream proof and counterexample are separate.

use ix_trace_rs::trace;
use qsl_replay::{compile_admitted, DependencyInput, ReplayLimits, SourceIdentity, StageLimits};
use quire_contract_model::{
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
};
use serde_json::{json, Value};

const VERIFY: &str = include_str!("../fixtures/float_mvp/commutative.native");
const MUTANT: &str = include_str!("../fixtures/float_mvp/wrong_operation.native");

#[trace("TC-1091", "FR-397-AC-3", "FR-397-AC-4")]
#[test]
fn float_commutativity_and_arithmetic_mutant_admit_strictly() {
    for (name, source, subtraction) in [
        ("commutative", VERIFY, false),
        ("wrong_operation", MUTANT, true),
    ] {
        let compiled = compile_admitted(
            SourceIdentity::new("agent-ix", "ir260-float-source", "git", "1"),
            name,
            source.as_bytes(),
            [],
            &DependencyInput::default(),
            StageLimits::default(),
            ReplayLimits::default(),
        )
        .unwrap_or_else(|error| panic!("{name}: QSL admission failed: {error:?}"));
        let wire: Value = serde_json::from_slice(compiled.package().bytes()).unwrap();
        let mut evidence = CheckedPackageEvidence::new();
        for feature in wire["lock"]["required_features"].as_array().unwrap() {
            evidence.support_feature(feature.as_str().unwrap());
        }
        let read = CheckedPackageV2::read(
            compiled.package().bytes(),
            CheckedPackageReadLimits::bounded(),
            &evidence,
        );
        let CheckedPackageV2ReadResult::Admitted(package) = read else {
            panic!("{name}: strict IR admission failed: {read:?}");
        };
        let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
        assert_eq!(package.graph().nodes.len(), nodes.len());
        assert_eq!(
            package.package_id().digest.as_ref(),
            compiled.package().package_id().hex()
        );

        let selected: Vec<_> = wire["lock"]["definition_selections"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["identity"] == "quire.value.ieee754-2019-default/v1")
            .collect();
        assert_eq!(selected.len(), 1, "{name}: one admitted IEEE profile");
        let operations: Vec<_> = nodes
            .iter()
            .filter_map(|node| node["body"].get("operation"))
            .collect();
        for identity in ["quire.op.ieee.float64.add", "quire.op.ieee.to_rational"] {
            assert!(
                operations
                    .iter()
                    .any(|operation| operation["identity"] == identity),
                "{name}: missing {identity}"
            );
        }
        assert_eq!(
            operations
                .iter()
                .any(|operation| operation["identity"] == "quire.op.ieee.float64.sub"),
            subtraction,
            "{name}: preserve the actual arithmetic mutation"
        );
        for operation in operations {
            let identity = operation["identity"].as_str().unwrap();
            if identity.starts_with("quire.op.ieee.") {
                assert_eq!(
                    operation["laws"],
                    json!([{"role": "ieee_profile", "definition": selected[0]}]),
                    "{name}: {identity} carries the selected law"
                );
            }
        }
        println!("IR-260 {name}: compile_admitted=admitted; strict V2=admitted");
    }
    assert_eq!(VERIFY.replace("(b + a)", "(b - a)"), MUTANT);
}
