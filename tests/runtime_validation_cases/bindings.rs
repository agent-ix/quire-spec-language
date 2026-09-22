// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-058/059/061/064: exact binding, closure and diagnostic permutation controls.

use super::setup::{authored_model, qualified, recorded};
use super::*;
use quire_spec_language::runtime::{RuntimePathSegment, RuntimeReference};
use serde_json::json;

#[test]
#[trace("TC-058", "FR-007-AC-3", "FR-007-AC-6")]
fn each_model_operation_and_observation_binding_is_exact() {
    let models = [native_rule_model::parts().model()];
    let checked = checked_kind(&models, "true", ClauseKind::Postcondition);
    for variant in 0..9 {
        let (mut offered, selected) = recorded(
            &models[0],
            draft(&models[0]),
            draft(&models[0]),
            |invocation| match variant {
                0 => {
                    invocation.context.model =
                        ir::RequirementRef::parse("example/foreign", "Foreign", 1).unwrap()
                }
                1 => invocation.context.name = symbol("NodeRef"),
                2 => invocation.operation = symbol("different"),
                3 => invocation.anchor = ir::AnchorName::new("different").unwrap(),
                4 => invocation.models[0].digest = ByteDigest::of(b"stale model bytes"),
                5 => {
                    invocation.models[0].model =
                        ir::RequirementRef::parse("example/foreign", "Foreign", 1).unwrap()
                }
                6 => invocation.models.push(invocation.models[0].clone()),
                7 => invocation.pre = invocation.post.clone(),
                8 => {}
                _ => unreachable!(),
            },
        );
        if variant == 8 {
            // Snapshot and invocation labels share one namespace, even with different bytes/kinds.
            offered.snapshots.push(
                Snapshot::new(
                    offered.invocations[0].identity().clone(),
                    draft(&models[0]),
                    ArtifactLimits::default(),
                )
                .unwrap(),
            );
        }
        let expected = match variant {
            0..=3 | 7 => Code::WrongSnapshot,
            4 => Code::StaleDependency,
            5 => Code::InvalidModelBinding,
            6 | 8 => Code::InvalidRuntimeInput,
            _ => unreachable!(),
        };
        let report = validate(
            &checked,
            offered,
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap_err();
        assert_eq!(
            report.status,
            ValidationStatus::Refused,
            "binding mutation {variant}"
        );
        assert!(report.terminal.is_none());
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.diagnostic.code == expected),
            "binding mutation {variant}: {report:?}"
        );
    }
}

#[test]
#[trace("TC-059", "FR-007-AC-1", "FR-007-AC-8", "FR-007-AC-9")]
fn object_identity_uses_exact_unicode_keys_and_complete_membership() {
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, "false");
    for key in [
        String::new(),
        "é".repeat(256),
        "e\u{301}".repeat(128),
        "界".repeat(257),
    ] {
        let mut data = draft(&models[0]);
        data.populations[0].objects[0].key.clone_from(&key);
        data.arena[3] = ValueNode::Reference {
            identity: object(&models[0], &key),
        };
        let artifact = snapshot(data);
        let mut selected = selection(&models[0], artifact.reference());
        if let ObservationSelection::Current { self_object, .. } = &mut selected.observation {
            self_object.key.clone_from(&key);
        }
        let result = validate(
            &checked,
            input(artifact),
            selected,
            ValidationLimits::default(),
            || false,
        );
        if key.chars().count() > 256 {
            let report = result.unwrap_err();
            assert_eq!(report.status, ValidationStatus::Refused);
            assert!(report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.diagnostic.code == Code::InvalidRuntimeInput));
        } else {
            let context = result.unwrap();
            assert_eq!(
                context
                    .object(ir::StateObservation::Current, &object(&models[0], &key))
                    .unwrap()
                    .key,
                key
            );
        }
    }
    for variant in 0..5 {
        let mut data = draft(&models[0]);
        let mut reference = object(&models[0], "self");
        match variant {
            0 => {
                reference.model =
                    ir::RequirementRef::parse("example/foreign", "Foreign", 1).unwrap()
            }
            1 => reference.record = symbol("NodeRef"),
            2 => reference.universe = symbol("foreign"),
            3 => {
                data.populations[0].objects[0].key = "é".into();
                reference.key = "e\u{301}".into();
            }
            4 => data.populations[0].objects.clear(),
            _ => unreachable!(),
        }
        data.arena[3] = ValueNode::Reference {
            identity: reference,
        };
        let artifact = snapshot(data);
        let selected = selection(&models[0], artifact.reference());
        let report = validate(
            &checked,
            input(artifact),
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap_err();
        assert_eq!(report.status, ValidationStatus::Refused);
        let expected = if variant < 3 {
            Code::InvalidRuntimeInput
        } else {
            Code::DanglingReference
        };
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.diagnostic.code == expected),
            "reference mutation {variant}"
        );
    }
}

#[test]
#[trace("TC-061", "FR-007-AC-9", "FR-007-AC-10")]
fn missing_required_state_and_operation_capture_correspondence_are_diagnosed() {
    let models = [authored_model(|data| {
        data["values"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name": "argument", "kind": "input", "type": {"kind": "boolean"}}));
        data["operations"][0]["parameters"] = json!(["argument"]);
    })];
    let checked = checked_kind(&models, "other.n >= 0", ClauseKind::Postcondition);
    for variant in 0..6 {
        let mut before = draft(&models[0]);
        before.arena.push(ValueNode::Object {
            identity: object(&models[0], "self"),
        });
        before.values.push(ValueBinding {
            declaration: qualified(&models[0], "other"),
            value: ValueId::new(5),
        });
        let mut after = before.clone();
        if variant == 0 {
            after.values.clear();
        }
        let (offered, selected) = recorded(&models[0], before, after, |invocation| {
            invocation.parameters.push(ValueBinding {
                declaration: qualified(&models[0], "argument"),
                value: ValueId::new(0),
            });
            match variant {
                0 => {}
                1 => invocation.parameters.clear(),
                2 => invocation.parameters.push(invocation.parameters[0].clone()),
                3 => invocation.parameters[0].declaration.name = symbol("foreign"),
                4 => invocation.result = None,
                5 => {
                    invocation.arena[0] = ValueNode::Integer { value: 0 };
                }
                _ => unreachable!(),
            }
        });
        let report = validate(
            &checked,
            offered,
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap_err();
        if variant == 0 {
            assert_eq!(report.status, ValidationStatus::Incomplete);
            assert!(report
                .diagnostics
                .iter()
                .any(
                    |diagnostic| diagnostic.diagnostic.code == Code::UnavailableObservation
                        && diagnostic.runtime.path
                            == [RuntimePathSegment::State(qualified(&models[0], "other"))]
                ));
        } else {
            assert_eq!(
                report.status,
                ValidationStatus::Refused,
                "capture mutation {variant}"
            );
            assert!(report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.diagnostic.code == Code::InvalidRuntimeInput));
        }
    }
}

#[test]
#[trace("TC-064", "FR-007-AC-13")]
fn conflicting_field_permutations_retain_the_same_sorted_diagnostics() {
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, "false");
    let mut data = draft(&models[0]);
    data.arena.push(ValueNode::Integer { value: 1001 });
    data.arena.push(ValueNode::Boolean { value: false });
    data.populations[0].objects[0]
        .fields
        .extend([field("n", 5), field("n", 6)]);
    let baseline = snapshot(data.clone()).reference();
    let mut reports = Vec::new();
    for reverse in [false, true] {
        let mut current = data.clone();
        if reverse {
            current.populations[0].objects[0].fields.reverse();
        }
        let artifact = snapshot(current);
        let actual = artifact.reference();
        let selected = selection(&models[0], actual.clone());
        let mut report = validate(
            &checked,
            input(artifact),
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap_err();
        assert_eq!(report.status, ValidationStatus::Refused);
        assert!(report.terminal.is_none());
        for diagnostic in &mut report.diagnostics {
            let runtime = &mut diagnostic.runtime;
            assert_eq!(runtime.artifact, RuntimeReference::Snapshot(actual.clone()));
            runtime.artifact = RuntimeReference::Snapshot(baseline.clone());
        }
        reports.push(report.diagnostics);
    }
    assert_eq!(reports[0], reports[1]);
}
