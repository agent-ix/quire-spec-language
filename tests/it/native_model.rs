// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-015: real source-derived native model admission, not a mock IR environment.

use crate::support::native_rule_model;

use ix_trace_rs::trace;
use native_rule_model::{parts, symbol};
use qsl_foundation::{ByteDigest, Code, Phase};
use quire_contract_ir as ir;
use quire_spec_language::native_model::{ModelLimits, NativeModel, ScalarKind, ScalarSite, Unit};

#[test]
#[trace("TC-040", "FR-015-AC-1")]
fn tc_040_source_derived_rule_model() {
    let model = parts().model();
    // The producer's optional enum inventory defaults to empty in this fixture.
    assert_eq!(model.environment().types().len(), 2);
    for declaration in model.environment().types() {
        let span = model.source().to_native(declaration.source()).unwrap();
        let original = &model.source().source().text()[span.start..span.end];
        let authored: serde_json::Value = serde_json::from_str(original).unwrap();
        assert_eq!(authored["name"], declaration.name().as_str());
        assert!(authored["fields"].is_array());
    }
    let node = model
        .environment()
        .types()
        .iter()
        .find(|d| d.name().as_str() == "Node")
        .unwrap();
    let ir::TypeDeclaration::Record { declaration } = node else {
        panic!("Node record")
    };
    for (name, min, max) in [
        ("n", 0, 1000),
        ("signed", -10, 10),
        ("count", 0, 3),
        ("wide", i64::MIN, i64::MAX),
    ] {
        let field = declaration
            .fields()
            .iter()
            .find(|f| f.name().as_str() == name)
            .unwrap();
        let ir::ValueType::Integer { value } = field.value_type() else {
            panic!("integer field")
        };
        assert_eq!(
            (
                value.minimum(),
                value.maximum(),
                value.domain(),
                value.overflow()
            ),
            (
                min,
                max,
                ir::IntegerDomain::Signed,
                ir::OverflowPolicy::Reject
            )
        );
        model.source().to_native(field.source()).unwrap();
    }
    for (name, unit) in [("Distance", "metre"), ("Duration", "second")] {
        let role = model
            .roles()
            .scalars
            .iter()
            .find(|r| r.name.as_str() == name)
            .unwrap();
        assert_eq!(
            role.kind,
            ScalarKind::Integer {
                unit: Unit::Named(symbol(unit))
            }
        );
    }
    assert_eq!(model.roles().objects[0].reference.as_str(), "NodeRef");
    assert_eq!(model.roles().objects[0].universe.as_str(), "nodes");
    assert_eq!(
        model.roles().operations[0]
            .result
            .as_ref()
            .unwrap()
            .as_str(),
        "step_result"
    );
    assert_eq!(
        model
            .environment()
            .values()
            .iter()
            .find(|v| v.name().as_str() == "step_result")
            .unwrap()
            .value_type(),
        &ir::ValueType::Boolean
    );
    let data: serde_json::Value = serde_json::from_slice(model.artifact_bytes()).unwrap();
    assert_eq!(data["profile"], "native-state-model/1");
    assert_eq!(model.digest(), ByteDigest::of(model.artifact_bytes()));
    assert_eq!(model.source().source().text(), native_rule_model::FIXTURE);
}

#[test]
#[trace("TC-041", "FR-015-AC-2")]
fn tc_041_invalid_roles_refuse_atomically() {
    for mutation in 0..8 {
        let mut input = parts();
        match mutation {
            0 => {
                input.roles.scalars.pop();
            }
            1 => input.roles.scalars.push(input.roles.scalars[0].clone()),
            2 => {
                let duplicate = input.roles.scalars[0].sites[0].clone();
                input.roles.scalars[0].sites.push(duplicate);
            }
            3 => input.roles.scalars[0].sites.push(ScalarSite::Value {
                name: symbol("self"),
            }),
            4 => input.roles.objects[0].reference = symbol("Node"),
            5 => input.roles.objects[0].identity_field = symbol("missing"),
            6 => input.roles.operations[0].parameters.push(symbol("self")),
            7 => input.roles.operations[0]
                .frame
                .fields
                .push((symbol("Node"), symbol("missing"))),
            _ => unreachable!(),
        }
        let error = NativeModel::new(
            input.source,
            input.environment,
            input.roles,
            ModelLimits::default(),
        )
        .unwrap_err();
        assert_eq!(
            error.diagnostic.code,
            Code::InvalidModelBinding,
            "mutation {mutation}"
        );
        assert_eq!(error.diagnostic.phase, Phase::Link);
        assert!(!error.diagnostic.is_incomplete());
        assert_eq!(parts().model().roles().objects.len(), 1);
    }
}

#[test]
#[trace("TC-042", "FR-015-AC-3")]
fn tc_042_artifact_binds_semantics_and_provenance() {
    let original = parts().model();
    let mut reordered = parts();
    reordered.roles.scalars.reverse();
    for role in &mut reordered.roles.scalars {
        role.sites.reverse();
    }
    reordered.environment = ir::DeclarationEnvironment::new(
        reordered.environment.owner().clone(),
        reordered
            .environment
            .types()
            .iter()
            .rev()
            .cloned()
            .collect(),
        reordered
            .environment
            .values()
            .iter()
            .rev()
            .cloned()
            .collect(),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(
        original.artifact_bytes(),
        reordered.model().artifact_bytes()
    );
    for mutation in 0..4 {
        let mut changed = parts();
        match mutation {
            0 => changed.roles.scalars[0].name = symbol("Renamed"),
            1 => changed.roles.objects[0].universe = symbol("anotherUniverse"),
            2 => changed.roles.operations[0].frame.fields.clear(),
            3 => changed.roles.operations[0].anchor = ir::AnchorName::new("anotherStep").unwrap(),
            _ => unreachable!(),
        }
        assert_ne!(
            original.digest(),
            changed.model().digest(),
            "mutation {mutation}"
        );
    }
    let relocated =
        native_rule_model::from_text(native_rule_model::FIXTURE, "another/path", "draft:1")
            .unwrap()
            .model();
    assert_eq!(original.artifact_bytes(), relocated.artifact_bytes());
    assert_ne!(
        original.digest(),
        native_rule_model::from_text(
            native_rule_model::FIXTURE,
            "native-rule-model.json",
            "draft:2"
        )
        .unwrap()
        .model()
        .digest()
    );
    let with_whitespace = format!("{}\n", native_rule_model::FIXTURE);
    assert_ne!(
        original.digest(),
        native_rule_model::from_text(&with_whitespace, "native-rule-model.json", "draft:1")
            .unwrap()
            .model()
            .digest()
    );
}

#[test]
#[trace("TC-043", "FR-015-AC-4")]
fn tc_043_constructor_valid_false_role_coordinates_refuse() {
    let mut input = parts();
    let old = &input.roles.scalars[0].source;
    input.roles.scalars[0].source = ir::SourceSpan::new(
        ir::SourceLocation::new(
            old.start().source().clone(),
            old.start().line() + 1,
            old.start().column(),
            old.start().byte_offset(),
        )
        .unwrap(),
        ir::SourceLocation::new(
            old.end().source().clone(),
            old.end().line() + 1,
            old.end().column(),
            old.end().byte_offset(),
        )
        .unwrap(),
    )
    .unwrap();
    let error = NativeModel::new(
        input.source,
        input.environment,
        input.roles,
        ModelLimits::default(),
    )
    .unwrap_err();
    assert_eq!(error.diagnostic.code, Code::InvalidModelBinding);
    assert_eq!(error.diagnostic.phase, Phase::Link);
}

#[test]
#[trace("TC-045", "FR-015-AC-6")]
fn tc_045_artifact_and_role_limits_are_inclusive() {
    let length = parts().model().artifact_bytes().len();
    for maximum in [0, length - 1, length, usize::MAX] {
        let input = parts();
        let result = NativeModel::new(
            input.source,
            input.environment,
            input.roles,
            ModelLimits {
                artifact_bytes: maximum,
                ..ModelLimits::default()
            },
        );
        if maximum < length {
            assert_eq!(result.unwrap_err().diagnostic.code, Code::ResourceExhausted);
        } else {
            assert_eq!(result.unwrap().artifact_bytes().len(), length);
        }
    }
    for maximum in [0, 8, 9] {
        let input = parts();
        let result = NativeModel::new(
            input.source,
            input.environment,
            input.roles,
            ModelLimits {
                roles: maximum,
                ..ModelLimits::default()
            },
        );
        if maximum < 9 {
            assert_eq!(result.unwrap_err().diagnostic.code, Code::ResourceExhausted);
        } else {
            assert_eq!(result.unwrap().roles().scalars.len(), 7);
        }
    }
}
