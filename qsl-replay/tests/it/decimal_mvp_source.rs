// SPDX-License-Identifier: AGPL-3.0-or-later
//! Decimal MVP source admission; downstream proof qualification is separate.

use ix_trace_rs::trace;
use qsl_replay::{compile_admitted, DependencyInput, ReplayLimits, SourceIdentity, StageLimits};
use quire_contract_model::{
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
};
use serde_json::Value;

const VERIFY: &str = include_str!("../fixtures/decimal_mvp/commutative.native");
const MUTANT: &str = include_str!("../fixtures/decimal_mvp/wrong_addend.native");

#[trace("TC-415", "FR-093-AC-3")]
#[test]
fn decimal_commutativity_and_wrong_addend_mutant_admit_strictly() {
    for (name, source) in [("commutative", VERIFY), ("wrong_addend", MUTANT)] {
        let compiled = compile_admitted(
            SourceIdentity::new("a", "u", "git", "1"),
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
        assert_eq!(
            package.graph().nodes.len(),
            wire["semantic_graph"]["nodes"].as_array().unwrap().len()
        );
        let operations: Vec<_> = wire["semantic_graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|node| node["body"]["operation"]["identity"].as_str())
            .collect();
        for identity in ["quire.op.decimal.add", "quire.op.decimal.eq"] {
            assert!(
                operations.contains(&identity),
                "{name}: missing {identity}: {operations:?}"
            );
        }
    }
    assert_eq!(VERIFY.replace("= add(b, a)", "= add(a, a)"), MUTANT);
}
