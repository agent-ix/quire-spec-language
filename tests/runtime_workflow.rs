// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-077 / IT-006: the complete source/model/input/reference-result API workflow.

#[path = "support/runtime_setup.rs"]
mod setup;
use setup::*;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::checking::CheckedPackage;
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::runtime::{
    evaluate, validate, EvaluationLimits, EvaluationOutcome, EvaluationReport,
    ImplicationEventKind, ObservationSelection, SnapshotDraft, SnapshotRef, ValidationLimits,
    ValidationReport, ValidationStatus, ValueBinding, ValueId, ValueNode,
};
use quire_spec_language::syntax::ClauseKind;
use quire_spec_language::{ByteDigest, Code, Phase};
use serde_json::json;

const PARENT: &str = "present(self.parent) implies deref(value(self.parent)).n < self.n";

fn append(data: &mut SnapshotDraft, value: ValueNode) -> ValueId {
    let id = ValueId::new(u32::try_from(data.arena.len()).unwrap());
    data.arena.push(value);
    id
}

fn parent_data(model: &NativeModel, parent_version: i64, cycle: bool) -> SnapshotDraft {
    let mut data = draft(model);
    let mut parent = data.populations[0].objects[0].clone();
    parent.key = "parent".into();
    let number = append(
        &mut data,
        ValueNode::Integer {
            value: parent_version,
        },
    );
    parent
        .fields
        .iter_mut()
        .find(|field| field.name.as_str() == "n")
        .unwrap()
        .value = number;
    if cycle {
        let reference = append(
            &mut data,
            ValueNode::Present {
                value: ValueId::new(3),
            },
        );
        parent
            .fields
            .iter_mut()
            .find(|field| field.name.as_str() == "parent")
            .unwrap()
            .value = reference;
    }
    data.populations[0].objects.push(parent);
    change_field(&mut data, "n", ValueNode::Integer { value: 2 });
    let reference = append(
        &mut data,
        ValueNode::Reference {
            identity: object(model, "parent"),
        },
    );
    change_field(&mut data, "parent", ValueNode::Present { value: reference });
    change_field(
        &mut data,
        "items",
        ValueNode::Sequence {
            values: vec![ValueId::new(0); 3],
        },
    );
    data
}

fn record_result(name: &str, report: &EvaluationReport<'_, '_, '_>) {
    let context = report.context();
    let source = context.checked().linked().unit().source();
    println!(
        "case={name} phase=evaluate outcome={:?} usage={:?} cost_model={} events={:?}",
        report.outcome(),
        report.usage(),
        report.cost_model(),
        report.events()
    );
    println!(
        "case={name} source={:?} source_digest={} authored={:?} clause={:?} selected={:?}",
        source.identity(),
        source.digest(),
        context.clause().binding().requirement,
        context.clause().binding().clause,
        context.selection().observation
    );
    for model in context.checked().linked().models() {
        println!(
            "case={name} model={:?} model_digest={}",
            model.environment().owner(),
            model.native_model().unwrap().digest()
        );
    }
    for snapshot in &context.input().snapshots {
        println!(
            "case={name} observation={:?} snapshot={:?}",
            snapshot.draft().observation,
            snapshot.reference()
        );
    }
}

fn assert_validation_failure(
    name: &str,
    checked: &CheckedPackage<'_>,
    report: &ValidationReport,
    status: ValidationStatus,
    code: Code,
) {
    assert_eq!(report.status, status, "{name}");
    let diagnostic = report
        .diagnostics
        .iter()
        .find(|d| d.code == code)
        .expect("expected validation code after successful setup");
    assert_eq!(diagnostic.phase, Phase::Validate);
    assert_eq!(
        diagnostic.source,
        *checked.linked().unit().source().identity()
    );
    let runtime = diagnostic
        .runtime
        .as_ref()
        .expect("runtime provenance accompanies validation refusal");
    assert_eq!(runtime.requirement, authored_owner());
    assert_eq!(runtime.clause.as_str(), "population_rule");
    assert!(report.usage.work > 0);
    println!("case={name} phase=validate status={:?} code={} native={:?} runtime={runtime:?} related={:?} usage={:?}",
        report.status, diagnostic.code, diagnostic.span, diagnostic.related, report.usage);
}

#[test]
#[trace("TC-077", "FR-008-AC-1", "FR-008-AC-2", "FR-018-AC-3")]
fn native_pipeline_distinguishes_parent_order_cycle_and_duplicate_aggregate_truth() {
    // Workflow steps 1/2/3/7: actual setup and independent truth/cost expectations.
    let models = [native_rule_model::parts().model()];
    for (name, expression, parent_version, cycle, truth, steps, graph_steps) in [
        ("healthy-parent", PARENT, 1, false, true, 12, 0),
        ("violating-parent", PARENT, 2, false, false, 12, 0),
        ("cycle-still-local-order", PARENT, 1, true, true, 12, 0),
        (
            "acyclic",
            "reaches(self, self, parent)",
            1,
            false,
            false,
            3,
            2,
        ),
        ("cycle", "reaches(self, self, parent)", 1, true, true, 3, 2),
        (
            "repeated-aggregate",
            "forall(item in self.items: item < self.n)",
            1,
            false,
            true,
            15,
            0,
        ),
    ] {
        let checked = checked(&models, expression);
        assert!(std::ptr::eq(
            checked.linked().models()[0].native_model().unwrap(),
            &models[0]
        ));
        assert_eq!(checked.clauses()[0].binding().requirement, authored_owner());
        assert_eq!(
            checked.bindings().source.source().digest(),
            checked.linked().unit().source().digest()
        );
        let artifact = snapshot(parent_data(&models[0], parent_version, cycle));
        assert_eq!(artifact.digest(), ByteDigest::of(artifact.bytes()));
        let reference = artifact.reference();
        let selected = selection(&models[0], reference.clone());
        let context = validate(
            &checked,
            input(artifact),
            selected.clone(),
            ValidationLimits::default(),
            || false,
        )
        .expect("all real model/source/input stages succeed before truth is asserted");
        assert_eq!(context.selection(), &selected);
        assert_eq!(context.input().snapshots[0].reference(), reference);
        let report = evaluate(
            &context,
            EvaluationLimits {
                expression_steps: steps,
                graph_steps,
                ..EvaluationLimits::default()
            },
            || false,
        );
        assert_eq!(
            report.outcome(),
            &EvaluationOutcome::Completed(truth),
            "{name}"
        );
        assert_eq!(report.usage().expression_steps, steps, "{name}");
        assert_eq!(report.usage().graph_steps, graph_steps, "{name}");
        assert!(std::ptr::eq(report.context(), &context));
        if expression == PARENT {
            assert_eq!(
                report
                    .events()
                    .iter()
                    .map(|event| event.kind)
                    .collect::<Vec<_>>(),
                [
                    ImplicationEventKind::AntecedentEntered,
                    ImplicationEventKind::AntecedentCompleted(true),
                    ImplicationEventKind::ConsequentEntered
                ]
            );
        }
        record_result(name, &report);
    }
}

#[test]
#[trace("TC-077", "FR-007-AC-5", "FR-018-AC-3")]
fn native_pipeline_retains_dangling_stale_and_incomplete_input_stages() {
    // Workflow steps 4/5/7: construction succeeds; validation supplies each observed result.
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, PARENT);
    for (name, complete, stale, status, code) in [
        (
            "dangling-parent",
            true,
            false,
            ValidationStatus::Refused,
            Code::DanglingReference,
        ),
        (
            "incomplete-population",
            false,
            false,
            ValidationStatus::Incomplete,
            Code::IncompletePopulation,
        ),
        (
            "stale-selected-bytes",
            true,
            true,
            ValidationStatus::Refused,
            Code::StaleDependency,
        ),
    ] {
        let mut data = parent_data(&models[0], 1, false);
        if !stale {
            data.populations[0].objects.pop();
        }
        data.populations[0].complete = complete;
        let artifact = snapshot(data);
        let reference = if stale {
            SnapshotRef::new(
                artifact.identity().clone(),
                ByteDigest::of(b"stale selected bytes"),
            )
            .unwrap()
        } else {
            artifact.reference()
        };
        let selected = selection(&models[0], reference);
        let report = validate(
            &checked,
            input(artifact),
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap_err();
        assert_validation_failure(name, &checked, &report, status, code);
        if code == Code::DanglingReference {
            assert!(report
                .diagnostics
                .iter()
                .any(|d| d.code == code && !d.related.is_empty()));
        }
    }
}

#[test]
#[trace("TC-077", "FR-008-AC-1", "FR-008-AC-2")]
fn native_aggregate_pipeline_observes_duplicates_empty_input_and_first_violation() {
    // Workflow steps 2/3/7: each duplicate contributes work; false stops at its occurrence.
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, "forall(item in self.items: item < self.n)");
    for (name, values, truth, steps, comparisons) in [
        ("aggregate-duplicates", vec![1, 1, 1], true, 15, 3),
        ("aggregate-middle-violation", vec![1, 2, 1], false, 11, 2),
        ("aggregate-empty", vec![], true, 3, 0),
    ] {
        let mut data = parent_data(&models[0], 1, false);
        let values = values
            .into_iter()
            .map(|value| append(&mut data, ValueNode::Integer { value }))
            .collect();
        change_field(&mut data, "items", ValueNode::Sequence { values });
        let artifact = snapshot(data);
        let selected = selection(&models[0], artifact.reference());
        let context = validate(
            &checked,
            input(artifact),
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap();
        let report = evaluate(
            &context,
            EvaluationLimits {
                expression_steps: steps,
                comparisons,
                ..EvaluationLimits::default()
            },
            || false,
        );
        assert_eq!(
            report.outcome(),
            &EvaluationOutcome::Completed(truth),
            "{name}"
        );
        assert_eq!(report.usage().expression_steps, steps, "{name}");
        assert_eq!(report.usage().comparisons, comparisons, "{name}");
        assert!(report.events().is_empty());
        record_result(name, &report);
    }
}

#[test]
#[trace("TC-077", "FR-008-AC-9", "FR-008-AC-19")]
fn native_pipeline_keeps_actual_event_prefixes_and_fresh_retry_budgets() {
    // Workflow steps 5/7: a completed antecedent does not manufacture a clause Boolean.
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, PARENT);
    let artifact = snapshot(parent_data(&models[0], 1, false));
    let bytes = artifact.bytes().to_vec();
    let selected = selection(&models[0], artifact.reference());
    let context = validate(
        &checked,
        input(artifact),
        selected,
        ValidationLimits::default(),
        || false,
    )
    .unwrap();
    let limits = EvaluationLimits {
        expression_steps: 4,
        ..EvaluationLimits::default()
    };
    let stopped = evaluate(&context, limits, || false);
    assert!(
        matches!(stopped.outcome(), EvaluationOutcome::Incomplete(d) if d.code == Code::ResourceExhausted)
    );
    assert_eq!(stopped.usage().expression_steps, 4);
    assert_eq!(
        stopped
            .events()
            .iter()
            .map(|event| event.kind)
            .collect::<Vec<_>>(),
        [
            ImplicationEventKind::AntecedentEntered,
            ImplicationEventKind::AntecedentCompleted(true)
        ]
    );
    record_result("exhausted-before-consequent", &stopped);
    let cancelled = evaluate(&context, EvaluationLimits::default(), || true);
    assert!(
        matches!(cancelled.outcome(), EvaluationOutcome::Incomplete(d) if d.code == Code::Cancelled)
    );
    assert_eq!(cancelled.usage().expression_steps, 0);
    assert!(cancelled.events().is_empty());
    record_result("cancelled-before-entry", &cancelled);
    let retry = evaluate(
        &context,
        EvaluationLimits {
            expression_steps: 12,
            ..EvaluationLimits::default()
        },
        || false,
    );
    assert_eq!(retry.outcome(), &EvaluationOutcome::Completed(true));
    assert_eq!(retry.usage().expression_steps, 12);
    record_result("successful-fresh-retry", &retry);
    let smaller = evaluate(&context, limits, || false);
    assert_eq!(smaller.outcome(), stopped.outcome());
    assert_eq!(smaller.usage(), stopped.usage());
    assert_eq!(smaller.events(), stopped.events());
    assert_eq!(context.input().snapshots[0].bytes(), bytes);
}

#[test]
#[trace("TC-077", "FR-007-AC-5", "FR-008-AC-6")]
fn native_operation_pipeline_validates_frames_before_pre_parameter_and_post_result_reads() {
    // Workflow steps 4/6/7: the operation records a permitted deletion and changed n.
    #[derive(Clone, Copy)]
    enum Case {
        Healthy,
        ResultFalse,
        WrongObservation,
        ForbiddenField,
        MissingDeletion,
    }
    let models = [authored_model(|data| {
        data["operations"][0]["frame"]["deleted"] = json!(["Node"]);
        data["values"].as_array_mut().unwrap().push(
            json!({"name":"captured", "kind":"input", "type":{"kind":"record", "name":"NodeRef"}}),
        );
        data["operations"][0]["parameters"] = json!(["captured"]);
    })];
    let post = checked_kind(
        &models,
        "deref(captured).n = 1 and pre(self.n) = 1 and self.n = 2 and result",
        ClauseKind::Postcondition,
    );
    let pre = checked_kind(&models, "self.n = 1", ClauseKind::Precondition);
    for case in [
        Case::Healthy,
        Case::ResultFalse,
        Case::WrongObservation,
        Case::ForbiddenField,
        Case::MissingDeletion,
    ] {
        let mut before = draft(&models[0]);
        let mut deleted = before.populations[0].objects[0].clone();
        deleted.key = "deleted".into();
        before.populations[0].objects.push(deleted);
        let mut after = draft(&models[0]);
        change_field(&mut after, "n", ValueNode::Integer { value: 2 });
        if matches!(case, Case::ForbiddenField) {
            change_field(&mut after, "signed", ValueNode::Integer { value: 1 });
        }
        let (offered, selected) = recorded(&models[0], before, after, |invocation| {
            invocation.deleted.push(object(&models[0], "deleted"));
            invocation.arena.push(ValueNode::Reference {
                identity: object(&models[0], "deleted"),
            });
            invocation.parameters.push(ValueBinding {
                declaration: qualified(&models[0], "captured"),
                value: ValueId::new(1),
            });
            match case {
                Case::Healthy | Case::ForbiddenField => {}
                Case::ResultFalse => invocation.arena[0] = ValueNode::Boolean { value: false },
                Case::WrongObservation => invocation.pre = invocation.post.clone(),
                Case::MissingDeletion => invocation.deleted.clear(),
            }
        });
        let result = validate(
            &post,
            offered.clone(),
            selected.clone(),
            ValidationLimits::default(),
            || false,
        );
        match case {
            Case::Healthy | Case::ResultFalse => {
                let context =
                    result.expect("permitted frame and exact deltas precede reference execution");
                assert!(context
                    .object(ir::StateObservation::Pre, &object(&models[0], "deleted"))
                    .is_some());
                assert!(context
                    .object(ir::StateObservation::Post, &object(&models[0], "deleted"))
                    .is_none());
                let report = evaluate(&context, EvaluationLimits::default(), || false);
                assert_eq!(
                    report.outcome(),
                    &EvaluationOutcome::Completed(matches!(case, Case::Healthy))
                );
                assert!(matches!(
                    context.selection().observation,
                    ObservationSelection::Invocation { .. }
                ));
                record_result(
                    if matches!(case, Case::Healthy) {
                        "post-captured-deleted-target"
                    } else {
                        "post-result-false"
                    },
                    &report,
                );
                let pre_context =
                    validate(&pre, offered, selected, ValidationLimits::default(), || {
                        false
                    })
                    .unwrap();
                let pre_report = evaluate(&pre_context, EvaluationLimits::default(), || false);
                assert_eq!(pre_report.outcome(), &EvaluationOutcome::Completed(true));
                record_result("precondition-uses-pre", &pre_report);
            }
            Case::WrongObservation => assert_validation_failure(
                "wrong-invocation-observation",
                &post,
                &result.unwrap_err(),
                ValidationStatus::Refused,
                Code::WrongSnapshot,
            ),
            Case::ForbiddenField => assert_validation_failure(
                "forbidden-frame-change",
                &post,
                &result.unwrap_err(),
                ValidationStatus::Refused,
                Code::FrameViolation,
            ),
            Case::MissingDeletion => assert_validation_failure(
                "missing-deletion-inventory",
                &post,
                &result.unwrap_err(),
                ValidationStatus::Refused,
                Code::PopulationDeltaMismatch,
            ),
        }
    }
}
