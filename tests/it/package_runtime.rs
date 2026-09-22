// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-089 / IT-007: runtime observations through newly reconstructed packages.

use crate::support::runtime_setup as setup;
use setup::*;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::checking::CheckedPackage;
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::package::{
    NativePackage, PackageLimits, PackageReadLimits, PackageSupport,
};
use quire_spec_language::runtime::{
    evaluate, validate, EvaluationLimits, EvaluationOutcome, ImplicationEventKind, SnapshotDraft,
    SnapshotRef, ValidationLimits, ValidationStatus, ValueBinding, ValueId, ValueNode,
};
use quire_spec_language::syntax::ClauseKind;
use quire_spec_language::{ByteDigest, Code, Phase};
use serde_json::{json, Value};

const PARENT: &str = "present(self.parent) implies deref(value(self.parent)).n < self.n";

fn reconstruct<'a>(checked: CheckedPackage<'a>, models: &'a [NativeModel]) -> NativePackage<'a> {
    let original = NativePackage::new(checked, PackageLimits::default()).unwrap();
    let bytes = original.bytes().to_vec();
    let reference = original.reference();
    let identity = original.canonical_identity();
    let bindings = original.checked().bindings().clone();
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        manifest["source"]["digest"],
        bindings.source.source().digest().to_string()
    );
    assert_eq!(manifest["clauses"].as_array().unwrap().len(), 2);
    assert_eq!(
        manifest["models"][0]["artifact"],
        std::str::from_utf8(models[0].artifact_bytes()).unwrap()
    );
    for (index, clause) in original
        .checked()
        .linked()
        .unit()
        .clauses()
        .iter()
        .enumerate()
    {
        assert_eq!(manifest["clauses"][index]["name"], clause.name.value);
        assert_eq!(
            manifest["clauses"][index]["projections"][1]["status"],
            "unlowered"
        );
        assert_eq!(
            manifest["clauses"][index]["projections"][1]["code"],
            "unsupported_construct"
        );
        let root = original
            .checked()
            .linked()
            .unit()
            .expression(clause.expression)
            .unwrap();
        assert_eq!(
            manifest["clauses"][index]["projections"][1]["span"],
            json!({"start":root.span.start,"end":root.span.end})
        );
    }
    drop(original);
    let package = NativePackage::read_verified(
        &bytes,
        reference,
        bindings.clone(),
        models,
        &PackageSupport::default(),
        PackageReadLimits::default(),
    )
    .unwrap();
    assert_eq!(package.bytes(), bytes);
    assert_eq!(package.digest(), ByteDigest::of(&bytes));
    assert_eq!(package.canonical_identity(), identity);
    assert_eq!(
        package.checked().bindings().source.identity(),
        bindings.source.identity()
    );
    assert_eq!(package.checked().bindings().clauses, bindings.clauses);
    assert!(package.usage().checking.is_some());
    assert!(package.usage().compare.is_some());
    package
}

fn append(data: &mut SnapshotDraft, value: ValueNode) -> ValueId {
    let id = ValueId::new(u32::try_from(data.arena.len()).unwrap());
    data.arena.push(value);
    id
}

fn parent_data(model: &NativeModel, version: i64, cycle: bool) -> SnapshotDraft {
    let mut data = draft(model);
    let mut parent = data.populations[0].objects[0].clone();
    parent.key = "parent".into();
    let number = append(&mut data, ValueNode::Integer { value: version });
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
    data
}

#[test]
#[trace("TC-089", "FR-020-AC-1", "FR-019-AC-1", "FR-019-AC-7")]
fn reconstructed_current_workflows_preserve_truth_events_and_exact_costs() {
    let models = [native_rule_model::parts().model()];
    for (expression, version, cycle, truth, steps, graph) in [
        (PARENT, 1, false, true, 12, 0),
        (PARENT, 2, false, false, 12, 0),
        ("reaches(self, self, parent)", 1, false, false, 3, 2),
        ("reaches(self, self, parent)", 1, true, true, 3, 2),
    ] {
        let package = reconstruct(checked(&models, expression), &models);
        let artifact = snapshot(parent_data(&models[0], version, cycle));
        let selected = selection(&models[0], artifact.reference());
        let context = validate(
            package.checked(),
            input(artifact),
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap();
        let result = evaluate(
            &context,
            EvaluationLimits {
                expression_steps: steps,
                graph_steps: graph,
                ..EvaluationLimits::default()
            },
            || false,
        );
        assert_eq!(result.outcome(), &EvaluationOutcome::Completed(truth));
        assert_eq!(result.usage().expression_steps, steps);
        assert_eq!(result.usage().graph_steps, graph);
        assert_eq!(context.clause().binding().requirement, authored_owner());
        if expression == PARENT {
            assert_eq!(
                result
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
    }
    let package = reconstruct(
        checked(&models, "forall(item in self.items: item < self.n)"),
        &models,
    );
    for (values, truth, steps, comparisons) in [
        (vec![1, 1, 1], true, 15, 3),
        (vec![1, 2, 1], false, 11, 2),
        (vec![], true, 3, 0),
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
            package.checked(),
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
        assert_eq!(report.outcome(), &EvaluationOutcome::Completed(truth));
        assert_eq!(report.usage().expression_steps, steps);
        assert_eq!(report.usage().comparisons, comparisons);
    }
}

#[test]
#[trace("TC-089", "FR-020-AC-1", "FR-019-AC-7")]
fn reconstructed_validation_and_exhaustion_preserve_refusal_and_fresh_retries() {
    let models = [native_rule_model::parts().model()];
    let package = reconstruct(checked(&models, PARENT), &models);
    for (complete, stale, code, status) in [
        (
            true,
            false,
            Code::DanglingReference,
            ValidationStatus::Refused,
        ),
        (
            false,
            false,
            Code::IncompletePopulation,
            ValidationStatus::Incomplete,
        ),
        (true, true, Code::StaleDependency, ValidationStatus::Refused),
    ] {
        let mut data = parent_data(&models[0], 1, false);
        if !stale {
            data.populations[0].objects.pop();
        }
        data.populations[0].complete = complete;
        let artifact = snapshot(data);
        let reference = if stale {
            SnapshotRef::new(artifact.identity().clone(), ByteDigest::of(b"stale bytes")).unwrap()
        } else {
            artifact.reference()
        };
        let selected = selection(&models[0], reference);
        let error = validate(
            package.checked(),
            input(artifact),
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap_err();
        assert_eq!(error.status, status);
        let diagnostic = error.diagnostics.iter().find(|d| d.code == code).unwrap();
        assert_eq!(diagnostic.phase, Phase::Validate);
        assert_eq!(
            diagnostic.source,
            *package.checked().bindings().source.source().identity()
        );
        assert!(diagnostic
            .message
            .contains(&format!("{:?}", authored_owner())));
    }
    let artifact = snapshot(parent_data(&models[0], 1, false));
    let selected = selection(&models[0], artifact.reference());
    let context = validate(
        package.checked(),
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
    let retry = evaluate(&context, EvaluationLimits::default(), || false);
    assert_eq!(retry.outcome(), &EvaluationOutcome::Completed(true));
    assert_eq!(retry.usage().expression_steps, 12);
    let again = evaluate(&context, limits, || false);
    assert_eq!(again.outcome(), stopped.outcome());
    assert_eq!(again.usage(), stopped.usage());
    assert_eq!(again.events(), stopped.events());
}

#[test]
#[trace("TC-089", "FR-020-AC-1", "FR-019-AC-1")]
fn reconstructed_operation_retains_deleted_pre_capture_post_result_and_frame_refusal() {
    let models = [authored_model(|model| {
        model["operations"][0]["frame"]["deleted"] = json!(["Node"]);
        model["values"].as_array_mut().unwrap().push(
            json!({"name":"captured","kind":"input","type":{"kind":"record","name":"NodeRef"}}),
        );
        model["operations"][0]["parameters"] = json!(["captured"]);
    })];
    let post = reconstruct(
        checked_kind(
            &models,
            "deref(captured).n = 1 and pre(self.n) = 1 and self.n = 2 and result",
            ClauseKind::Postcondition,
        ),
        &models,
    );
    let pre = reconstruct(
        checked_kind(&models, "self.n = 1", ClauseKind::Precondition),
        &models,
    );
    for (result, forbidden, missing_deletion) in [
        (true, false, false),
        (false, false, false),
        (true, true, false),
        (true, false, true),
    ] {
        let mut before = draft(&models[0]);
        let mut deleted = before.populations[0].objects[0].clone();
        deleted.key = "deleted".into();
        before.populations[0].objects.push(deleted);
        let mut after = draft(&models[0]);
        change_field(&mut after, "n", ValueNode::Integer { value: 2 });
        if forbidden {
            change_field(&mut after, "signed", ValueNode::Integer { value: 1 });
        }
        let (offered, selected) = recorded(&models[0], before, after, |invocation| {
            if !missing_deletion {
                invocation.deleted.push(object(&models[0], "deleted"));
            }
            invocation.arena[0] = ValueNode::Boolean { value: result };
            invocation.arena.push(ValueNode::Reference {
                identity: object(&models[0], "deleted"),
            });
            invocation.parameters.push(ValueBinding {
                declaration: qualified(&models[0], "captured"),
                value: ValueId::new(1),
            });
        });
        let validated = validate(
            post.checked(),
            offered.clone(),
            selected.clone(),
            ValidationLimits::default(),
            || false,
        );
        if forbidden || missing_deletion {
            let report = validated.unwrap_err();
            assert_eq!(report.status, ValidationStatus::Refused);
            let code = if forbidden {
                Code::FrameViolation
            } else {
                Code::PopulationDeltaMismatch
            };
            assert!(report.diagnostics.iter().any(|d| d.code == code));
            continue;
        }
        let context = validated.unwrap();
        assert!(context
            .object(ir::StateObservation::Pre, &object(&models[0], "deleted"))
            .is_some());
        assert!(context
            .object(ir::StateObservation::Post, &object(&models[0], "deleted"))
            .is_none());
        assert_eq!(
            evaluate(&context, EvaluationLimits::default(), || false).outcome(),
            &EvaluationOutcome::Completed(result)
        );
        let context = validate(
            pre.checked(),
            offered,
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap();
        assert_eq!(
            evaluate(&context, EvaluationLimits::default(), || false).outcome(),
            &EvaluationOutcome::Completed(true)
        );
    }
}
