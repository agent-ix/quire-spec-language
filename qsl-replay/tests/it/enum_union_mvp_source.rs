// SPDX-License-Identifier: AGPL-3.0-or-later
//! Enum and union MVP source admission; downstream proof qualification is separate.

use ix_trace_rs::trace;
use qsl_replay::{
    compile_admitted, AdmittedCompositeForm, DependencyInput, ReplayLimits, SourceIdentity,
    StageLimits,
};
use quire_contract_model::{
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
};
use quire_semantic_value::declaration::{
    CompositeDeclaration, CompositeShape, TypeEnvironment, UnionMemberDeclaration,
};
use serde_json::Value;

const VERIFY: &str = include_str!("../fixtures/enum_union_mvp/symmetric.native");
const MUTANT: &str = include_str!("../fixtures/enum_union_mvp/wrong_comparison.native");

#[trace("TC-415", "FR-093-AC-3")]
#[test]
fn enum_union_symmetry_and_wrong_comparison_mutant_admit_strictly() {
    for (name, source) in [("symmetric", VERIFY), ("wrong_comparison", MUTANT)] {
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
        let union = compiled
            .context()
            .composites()
            .find(|declaration| declaration.form() == AdmittedCompositeForm::Union)
            .expect("source Choice union is admitted");
        let members = union
            .union_members()
            .expect("union member bindings are exposed");
        assert_eq!(members.len(), 2);
        assert_ne!(members[0].variant(), members[1].variant());
        let mut types = TypeEnvironment::new(
            [CompositeDeclaration::new(
                union.key(),
                "Choice",
                CompositeShape::Union(
                    members
                        .iter()
                        .map(|member| {
                            UnionMemberDeclaration::resolved(
                                member.identifier().clone(),
                                member.payload().to_vec(),
                            )
                        })
                        .collect(),
                ),
            )],
            [],
        )
        .expect("declare Choice using only admitted payloads and kernel key");
        types
            .seal_union_verified(
                union.key(),
                union.node(),
                members
                    .iter()
                    .map(|member| member.member().clone())
                    .collect(),
            )
            .expect("QSL-945 admitted bindings seal with the producer's node");
        assert_eq!(types.union_key(union.key()), Some(union.node()));
        let enumeration = compiled
            .context()
            .enums()
            .next()
            .expect("Status enum is admitted");
        assert_eq!(enumeration.cases().len(), 2);
        assert_ne!(enumeration.cases()[0].key(), enumeration.cases()[1].key());
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
        for identity in [
            "quire.op.enum.eq",
            "quire.op.structural.eq",
            "quire.op.boolean.eq",
        ] {
            assert!(
                operations.contains(&identity),
                "{name}: missing {identity}: {operations:?}"
            );
        }
    }
    assert_eq!(
        VERIFY.replace("((b = a) and (y = x))", "((a = a) and (x = x))"),
        MUTANT
    );
}
