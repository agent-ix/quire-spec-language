// SPDX-License-Identifier: AGPL-3.0-only
//! TC-061–063: actual operation captures, population deltas and storage frames.

use super::*;

#[test]
#[trace("TC-061", "TC-062", "FR-007-AC-4", "FR-007-AC-10")]
fn recorded_input_checks_immutable_field_permissions_before_any_truth() {
    let models = [native_rule_model::parts().model()];
    for kind in [ClauseKind::Precondition, ClauseKind::Postcondition] {
        let checked = checked_kind(&models, "false", kind);
        for (field, allowed) in [("n", true), ("count", false)] {
            let mut after = draft(&models[0]);
            change_field(&mut after, field, ValueNode::Integer { value: 2 });
            let (input, selected) = recorded(&models[0], draft(&models[0]), after, |_| {});
            let result = validate(
                &checked,
                input,
                selected,
                ValidationLimits::default(),
                || false,
            );
            if allowed {
                let context = result.expect("the model explicitly permits changing only n");
                assert!(context.invocation().is_some());
                assert_eq!(
                    context
                        .snapshot(ir::StateObservation::Pre)
                        .unwrap()
                        .draft()
                        .observation,
                    ir::StateObservation::Pre
                );
                assert_eq!(
                    context
                        .snapshot(ir::StateObservation::Post)
                        .unwrap()
                        .draft()
                        .observation,
                    ir::StateObservation::Post
                );
                assert!(context.snapshot(ir::StateObservation::Current).is_none());
            } else {
                let report = result.unwrap_err();
                assert_eq!(report.status, ValidationStatus::Refused);
                assert!(report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == Code::FrameViolation));
            }
        }
    }
}

#[test]
#[trace("TC-061", "FR-007-AC-9", "FR-007-AC-10")]
fn precondition_can_record_permitted_self_deletion_and_pre_captured_parameter() {
    let models = [authored_model(|data| {
        data["operations"][0]["frame"]["deleted"] = serde_json::json!(["Node"]);
        data["values"].as_array_mut().unwrap().push(serde_json::json!({"name":"captured","kind":"input","type":{"kind":"record","name":"NodeRef"}}));
        data["operations"][0]["parameters"] = serde_json::json!(["captured"]);
    })];
    for kind in [ClauseKind::Precondition, ClauseKind::Postcondition] {
        let checked = checked_kind(&models, "true", kind);
        let mut after = draft(&models[0]);
        after.populations[0].objects.clear();
        let (input, selected) = recorded(&models[0], draft(&models[0]), after, |invocation| {
            invocation.deleted.push(object(&models[0], "self"));
            invocation.arena.push(ValueNode::Reference {
                identity: object(&models[0], "self"),
            });
            invocation.parameters.push(ValueBinding {
                declaration: qualified(&models[0], "captured"),
                value: ValueId::new(1),
            });
        });
        let result = validate(
            &checked,
            input,
            selected,
            ValidationLimits::default(),
            || false,
        );
        if kind == ClauseKind::Precondition {
            let context =
                result.expect("pre self and parameter resolve before the permitted deletion");
            assert!(context
                .object(ir::StateObservation::Pre, &object(&models[0], "self"))
                .is_some());
            assert!(context
                .object(ir::StateObservation::Post, &object(&models[0], "self"))
                .is_none());
            assert_eq!(context.invocation().unwrap().draft().parameters.len(), 1);
        } else {
            let report = result.unwrap_err();
            assert_eq!(report.status, ValidationStatus::Refused);
            assert!(report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == Code::DanglingReference
                    && diagnostic.runtime.as_ref().unwrap().observation
                        == Some(ir::StateObservation::Post)));
        }
    }
}

#[test]
#[trace("TC-062", "FR-007-AC-4", "FR-007-AC-11")]
fn actual_created_sets_need_both_exact_recording_and_model_permission() {
    for allowed in [false, true] {
        let models = [authored_model(|data| {
            if allowed {
                data["operations"][0]["frame"]["created"] = serde_json::json!(["Node"]);
            }
        })];
        let checked = checked_kind(&models, "true", ClauseKind::Postcondition);
        for recorded_correctly in [false, true] {
            let mut after = draft(&models[0]);
            let mut other = after.populations[0].objects[0].clone();
            other.key = "other".into();
            after.populations[0].objects.push(other);
            let (input, selected) = recorded(&models[0], draft(&models[0]), after, |invocation| {
                if recorded_correctly {
                    invocation.created.push(object(&models[0], "other"));
                }
            });
            let result = validate(
                &checked,
                input,
                selected,
                ValidationLimits::default(),
                || false,
            );
            if allowed && recorded_correctly {
                let context = result.unwrap();
                assert!(context
                    .object(ir::StateObservation::Post, &object(&models[0], "other"))
                    .is_some());
            } else {
                let report = result.unwrap_err();
                assert_eq!(report.status, ValidationStatus::Refused);
                assert_eq!(
                    report
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.code == Code::FrameViolation),
                    !allowed
                );
                assert_eq!(
                    report
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.code == Code::PopulationDeltaMismatch),
                    !recorded_correctly
                );
            }
        }
    }
}

#[test]
#[trace("TC-061", "TC-062", "FR-007-AC-9", "FR-007-AC-11")]
fn unavailable_pre_does_not_become_empty_or_suppress_known_post_and_delta_defects() {
    let models = [native_rule_model::parts().model()];
    let checked = checked_kind(&models, "true", ClauseKind::Postcondition);
    let mut after = draft(&models[0]);
    change_field(&mut after, "n", ValueNode::Integer { value: 1001 });
    let (mut input, selected) = recorded(&models[0], draft(&models[0]), after, |invocation| {
        invocation.created = vec![object(&models[0], "self"), object(&models[0], "self")];
        invocation.deleted = vec![object(&models[0], "self")];
    });
    input.snapshots.remove(0);
    let report = validate(
        &checked,
        input,
        selected,
        ValidationLimits::default(),
        || false,
    )
    .unwrap_err();
    assert_eq!(report.status, ValidationStatus::Refused);
    for code in [
        Code::UnavailableObservation,
        Code::InvalidRuntimeInput,
        Code::PopulationDeltaMismatch,
    ] {
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == code),
            "{code:?}"
        );
    }
    assert!(report
        .diagnostics
        .iter()
        .all(|diagnostic| diagnostic.code != Code::FrameViolation));
}

#[test]
#[trace("TC-063", "FR-007-AC-14")]
fn state_roots_need_preserved_storage_values_and_explicit_counterparts() {
    let models = [native_rule_model::parts().model()];
    let checked = checked_kind(&models, "true", ClauseKind::Postcondition);
    for variant in 0..3 {
        let mut before = draft(&models[0]);
        let mut other = before.populations[0].objects[0].clone();
        other.key = "other".into();
        before.populations[0].objects.push(other);
        before.arena.push(ValueNode::Object {
            identity: object(&models[0], "self"),
        });
        before.values.push(ValueBinding {
            declaration: qualified(&models[0], "other"),
            value: ValueId::new(5),
        });
        let mut after = before.clone();
        match variant {
            0 => change_field(&mut after, "n", ValueNode::Integer { value: 2 }),
            1 => {
                after.arena[5] = ValueNode::Object {
                    identity: object(&models[0], "other"),
                }
            }
            2 => after.values.clear(),
            _ => unreachable!(),
        }
        let (input, selected) = recorded(&models[0], before, after, |_| {});
        let result = validate(
            &checked,
            input,
            selected,
            ValidationLimits::default(),
            || false,
        );
        if variant == 0 {
            let context = result.expect(
                "preserved root identity permits the model-authorized population field change",
            );
            assert_eq!(
                context.state(ir::StateObservation::Post, &qualified(&models[0], "other")),
                Some(ValueId::new(5))
            );
        } else {
            let report = result.unwrap_err();
            let (status, code) = if variant == 1 {
                (ValidationStatus::Refused, Code::FrameViolation)
            } else {
                (ValidationStatus::Incomplete, Code::UnavailableObservation)
            };
            assert_eq!(report.status, status);
            assert!(report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == code));
        }
    }
}

#[test]
#[trace("TC-064", "FR-007-AC-5", "FR-007-AC-13")]
fn incomplete_population_retains_known_unauthorized_field_change() {
    let models = [native_rule_model::parts().model()];
    let checked = checked_kind(&models, "true", ClauseKind::Postcondition);
    let mut before = draft(&models[0]);
    before.populations[0].complete = false;
    let mut after = draft(&models[0]);
    change_field(&mut after, "count", ValueNode::Integer { value: 2 });
    let (input, selected) = recorded(&models[0], before, after, |_| {});
    let report = validate(
        &checked,
        input,
        selected,
        ValidationLimits::default(),
        || false,
    )
    .unwrap_err();
    assert_eq!(report.status, ValidationStatus::Refused);
    for code in [Code::IncompletePopulation, Code::FrameViolation] {
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code));
    }
    assert!(report
        .diagnostics
        .iter()
        .all(|diagnostic| diagnostic.code != Code::PopulationDeltaMismatch));
}

#[test]
#[trace("TC-064", "FR-007-AC-5", "FR-007-AC-13")]
fn unrelated_duplicate_objects_or_fields_do_not_hide_known_frame_changes() {
    let models = [native_rule_model::parts().model()];
    let checked = checked_kind(&models, "true", ClauseKind::Postcondition);
    for duplicate_object in [true, false] {
        let mut before = draft(&models[0]);
        if duplicate_object {
            let mut duplicate = before.populations[0].objects[0].clone();
            duplicate.key = "duplicate".into();
            before.populations[0]
                .objects
                .extend([duplicate.clone(), duplicate]);
        } else {
            before.populations[0].objects[0].fields.push(field("n", 0));
        }
        let mut after = before.clone();
        change_field(&mut after, "count", ValueNode::Integer { value: 2 });
        let (input, selected) = recorded(&models[0], before, after, |_| {});
        let report = validate(
            &checked,
            input,
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap_err();
        assert_eq!(report.status, ValidationStatus::Refused);
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == Code::InvalidRuntimeInput));
        let frame: Vec<_> = report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == Code::FrameViolation)
            .collect();
        assert_eq!(
            frame.len(),
            1,
            "unrelated duplicate object: {duplicate_object}"
        );
        assert_eq!(
            frame[0].runtime.as_ref().unwrap().path.last(),
            Some(&quire_spec_language::runtime::RuntimePathSegment::Field(
                symbol("count")
            ))
        );
    }
}

#[test]
#[trace("TC-062", "FR-007-AC-4", "FR-007-AC-11")]
fn field_and_population_permissions_do_not_extend_to_other_supplied_models() {
    let mut data: serde_json::Value = serde_json::from_str(native_rule_model::FIXTURE).unwrap();
    data["package"] = serde_json::json!("example/other-model");
    let text = serde_json::to_string_pretty(&data).unwrap();
    let source = quire_spec_language::Source::read(
        SourceIdentity {
            identity: "test:other-model".into(),
            revision: "1".into(),
        },
        "other-model.json",
        text.as_bytes(),
        1_048_576,
    )
    .unwrap();
    let source = FormalSource::new(
        source,
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new("OtherRuleModel").unwrap(),
            ir::SourceRevision::new(1).unwrap(),
        ),
    );
    let models = [
        native_rule_model::parts().model(),
        native_rule_model::from_source(source).unwrap().model(),
    ];
    let checked = checked_kind(&models, "false", ClauseKind::Postcondition);
    let mut before = draft(&models[0]);
    before.models.push(ModelBinding {
        model: models[1].environment().owner().clone(),
        digest: models[1].digest(),
    });
    let mut foreign = draft(&models[1]).populations.remove(0);
    before.arena.push(ValueNode::Reference {
        identity: object(&models[1], "self"),
    });
    foreign.objects[0]
        .fields
        .iter_mut()
        .find(|field| field.name.as_str() == "peer")
        .unwrap()
        .value = ValueId::new(5);
    before.populations.push(foreign);
    for variant in 0..4 {
        let mut after = before.clone();
        match variant {
            0 => {}
            1 => {
                after.arena.push(ValueNode::Integer { value: 2 });
                after.populations[1].objects[0].fields[0].value = ValueId::new(6);
            }
            2 => {
                let mut created = after.populations[1].objects[0].clone();
                created.key = "other".into();
                after.populations[1].objects.push(created);
            }
            3 => after.populations[1].objects.clear(),
            _ => unreachable!(),
        }
        let (input, selected) = recorded(&models[0], before.clone(), after, |invocation| {
            if variant == 2 {
                invocation.created.push(object(&models[1], "other"));
            }
            if variant == 3 {
                invocation.deleted.push(object(&models[1], "self"));
            }
        });
        let result = validate(
            &checked,
            input,
            selected,
            ValidationLimits::default(),
            || false,
        );
        if variant == 0 {
            let context = result.unwrap();
            assert_eq!(
                context
                    .object(ir::StateObservation::Post, &object(&models[1], "self"))
                    .unwrap()
                    .fields[0],
                field("n", 0)
            );
        } else {
            let report = result.unwrap_err();
            assert_eq!(report.status, ValidationStatus::Refused);
            assert_eq!(
                report.diagnostics.len(),
                1,
                "effect variant {variant}: {report:?}"
            );
            assert_eq!(report.diagnostics[0].code, Code::FrameViolation);
        }
    }
}

#[test]
#[trace("TC-062", "FR-007-AC-4", "FR-007-AC-11")]
fn deletion_requires_both_exact_recording_and_model_permission() {
    for allowed in [false, true] {
        let models = [authored_model(|data| {
            if allowed {
                data["operations"][0]["frame"]["deleted"] = serde_json::json!(["Node"]);
            }
        })];
        let checked = checked_kind(&models, "true", ClauseKind::Postcondition);
        let mut before = draft(&models[0]);
        let mut deleted = before.populations[0].objects[0].clone();
        deleted.key = "other".into();
        before.populations[0].objects.push(deleted);
        for recording in 0..4 {
            let (input, selected) = recorded(
                &models[0],
                before.clone(),
                draft(&models[0]),
                |invocation| match recording {
                    0 => invocation.deleted.push(object(&models[0], "other")),
                    1 => {}
                    2 => invocation.deleted.push(object(&models[0], "self")),
                    3 => invocation
                        .deleted
                        .extend([object(&models[0], "other"), object(&models[0], "other")]),
                    _ => unreachable!(),
                },
            );
            let result = validate(
                &checked,
                input,
                selected,
                ValidationLimits::default(),
                || false,
            );
            if allowed && recording == 0 {
                let context = result.unwrap();
                assert!(context
                    .object(ir::StateObservation::Pre, &object(&models[0], "other"))
                    .is_some());
                assert!(context
                    .object(ir::StateObservation::Post, &object(&models[0], "other"))
                    .is_none());
            } else {
                let report = result.unwrap_err();
                assert_eq!(report.status, ValidationStatus::Refused);
                assert_eq!(
                    report
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.code == Code::FrameViolation),
                    !allowed
                );
                assert_eq!(
                    report
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.code == Code::PopulationDeltaMismatch),
                    recording != 0
                );
            }
        }
    }
}

#[test]
#[trace("TC-061", "FR-007-AC-10")]
fn absent_and_present_results_must_correspond_to_the_declared_operation() {
    for declared in [false, true] {
        let models = [authored_model(|data| {
            if !declared {
                data["operations"][0]["result"] = serde_json::Value::Null;
                data["values"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|value| value["name"] != "step_result");
            }
        })];
        let checked = checked_kind(&models, "false", ClauseKind::Postcondition);
        for supplied in [false, true] {
            let (input, selected) = recorded(
                &models[0],
                draft(&models[0]),
                draft(&models[0]),
                |invocation| {
                    if !supplied {
                        invocation.result = None;
                    }
                },
            );
            let result = validate(
                &checked,
                input,
                selected,
                ValidationLimits::default(),
                || false,
            );
            if supplied == declared {
                let context = result.unwrap();
                assert_eq!(
                    context.invocation().unwrap().draft().result,
                    supplied.then_some(ValueId::new(0))
                );
            } else {
                let report = result.unwrap_err();
                assert_eq!(report.status, ValidationStatus::Refused);
                assert_eq!(report.diagnostics.len(), 1);
                assert_eq!(report.diagnostics[0].code, Code::InvalidRuntimeInput);
            }
        }
    }
}
