// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-008: independent truth and accounting through checked, validated native inputs.

use crate::support::runtime_setup as setup;
use setup::*;

#[path = "../runtime_evaluation_cases/events_limits.rs"]
mod events_limits;
#[path = "../runtime_evaluation_cases/graphs_sequences.rs"]
mod graphs_sequences;
#[path = "../runtime_evaluation_cases/hard_limits.rs"]
mod hard_limits;
#[path = "../runtime_evaluation_cases/observations.rs"]
mod observations;
#[path = "../runtime_evaluation_cases/records.rs"]
mod records;
#[path = "../runtime_evaluation_cases/scalars.rs"]
mod scalars;

use ix_trace_rs::trace;
use qsl_foundation::Code;
use quire_spec_language::checking::CheckedPackage;
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::runtime::{
    evaluate, validate, EvaluationLimits, EvaluationOutcome, ImplicationEventKind, SnapshotDraft,
    ValidatedContext, ValidationLimits, ValueBinding, ValueId, ValueNode,
};

fn current<'c, 'm>(
    checked: &'c CheckedPackage<'m>,
    model: &NativeModel,
    data: SnapshotDraft,
) -> ValidatedContext<'c, 'm> {
    let artifact = snapshot(data);
    let selected = selection(model, artifact.reference());
    validate(
        checked,
        input(artifact),
        selected,
        ValidationLimits::default(),
        || false,
    )
    .expect("checked source and finite runtime data must validate before truth assertions")
}

fn append(data: &mut SnapshotDraft, value: ValueNode) -> ValueId {
    let id = ValueId::new(u32::try_from(data.arena.len()).unwrap());
    data.arena.push(value);
    id
}

#[test]
#[trace("TC-074", "FR-008-AC-4", "FR-008-AC-16")]
fn independent_expression_cost_vectors_and_one_below() {
    let models = [native_rule_model::parts().model()];
    for (expression, steps, truth) in [
        ("true", 1, true),
        ("(true)", 1, true),
        ("false implies true", 2, true),
        ("true implies false", 3, false),
        ("let x = true in x and x", 5, true),
        ("let x = false in x and x", 4, false),
        ("if true then true else false", 3, true),
    ] {
        let checked = checked(&models, expression);
        let artifact = snapshot(draft(&models[0]));
        let selected = selection(&models[0], artifact.reference());
        let context = validate(
            &checked,
            input(artifact),
            selected,
            ValidationLimits::default(),
            || false,
        )
        .expect("cost-vector setup must validate separately");
        let limits = EvaluationLimits {
            expression_steps: steps,
            ..EvaluationLimits::default()
        };
        let report = evaluate(&context, limits, || false);
        assert_eq!(
            report.outcome(),
            &EvaluationOutcome::Completed(truth),
            "{expression}"
        );
        assert!(std::ptr::eq(report.context(), &context));
        assert_eq!(report.cost_model(), "native-ref-cost/1-draft");
        assert_eq!(report.usage().expression_steps, steps, "{expression}");
        assert_eq!(report.usage().graph_steps, 0);
        let stopped = evaluate(
            &context,
            EvaluationLimits {
                expression_steps: steps - 1,
                ..limits
            },
            || false,
        );
        assert!(
            matches!(stopped.outcome(), EvaluationOutcome::Incomplete(d) if d.code == Code::ResourceExhausted),
            "{expression}: {stopped:?}"
        );
        assert_eq!(stopped.usage().expression_steps, steps - 1);
        if expression == "true implies false" {
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
        }
    }
}

#[test]
#[trace("TC-068", "TC-074", "FR-008-AC-3", "FR-008-AC-16")]
fn positive_length_graph_cost_vectors_and_one_below() {
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, "reaches(self, self, parent)");
    for (edges, graph_steps, truth) in [
        (vec![None], 1, false),
        (vec![Some(0)], 1, true),
        (vec![Some(1), Some(0)], 2, true),
    ] {
        let mut before = draft(&models[0]);
        let template = before.populations[0].objects[0].clone();
        before.populations[0].objects.clear();
        for (index, edge) in edges.iter().enumerate() {
            let mut node = template.clone();
            node.key = if index == 0 {
                "self".into()
            } else {
                format!("object-{index}")
            };
            if let Some(target) = edge {
                let key = if *target == 0 {
                    "self".into()
                } else {
                    format!("object-{target}")
                };
                let reference = ValueId::new(u32::try_from(before.arena.len()).unwrap());
                before.arena.push(ValueNode::Reference {
                    identity: object(&models[0], &key),
                });
                let parent = ValueId::new(u32::try_from(before.arena.len()).unwrap());
                before.arena.push(ValueNode::Present { value: reference });
                node.fields
                    .iter_mut()
                    .find(|field| field.name.as_str() == "parent")
                    .unwrap()
                    .value = parent;
            }
            before.populations[0].objects.push(node);
        }
        let artifact = snapshot(before);
        let selected = selection(&models[0], artifact.reference());
        let context = validate(
            &checked,
            input(artifact),
            selected,
            ValidationLimits::default(),
            || false,
        )
        .expect("graph setup must establish finite closure before traversal");
        let limits = EvaluationLimits {
            expression_steps: 3,
            graph_steps,
            ..EvaluationLimits::default()
        };
        let report = evaluate(&context, limits, || false);
        assert_eq!(report.outcome(), &EvaluationOutcome::Completed(truth));
        assert_eq!(report.usage().expression_steps, 3);
        assert_eq!(report.usage().graph_steps, graph_steps);
        for lowered in [
            EvaluationLimits {
                expression_steps: 2,
                ..limits
            },
            EvaluationLimits {
                graph_steps: graph_steps - 1,
                ..limits
            },
        ] {
            let stopped = evaluate(&context, lowered, || false);
            assert!(
                matches!(stopped.outcome(), EvaluationOutcome::Incomplete(d) if d.code == Code::ResourceExhausted)
            );
        }
    }
}
