// SPDX-License-Identifier: AGPL-3.0-only
//! TC-071/075: finite hard ceilings and borrowed sequence reads under zero comparison fuel.

use super::*;
use serde_json::json;

fn model() -> NativeModel {
    authored_model(|data| {
        data["records"][0]["fields"][9]["type"]["maximum"] = json!(10000);
        data["scalars"][2]["maximum"] = json!(10000);
        data["values"].as_array_mut().unwrap().extend([
            json!({"name":"label", "kind":"state", "type":{"kind":"scalar", "name":"ObjectId"}}),
            json!({"name":"right_label", "kind":"state", "type":{"kind":"scalar", "name":"ObjectId"}}),
        ]);
        data["scalars"][6]["max_scalars"] = json!(1024);
    })
}

fn data(model: &NativeModel, count: usize) -> SnapshotDraft {
    let mut data = draft(model);
    change_field(
        &mut data,
        "items",
        ValueNode::Sequence {
            values: vec![ValueId::new(0); count],
        },
    );
    change_field(
        &mut data,
        "parent",
        ValueNode::Present {
            value: ValueId::new(3),
        },
    );
    change_field(
        &mut data,
        "count",
        ValueNode::Integer {
            value: i64::try_from(count).unwrap(),
        },
    );
    for name in ["label", "right_label"] {
        let value = append(
            &mut data,
            ValueNode::Text {
                value: "界".repeat(511),
            },
        );
        data.values.push(ValueBinding {
            declaration: qualified(model, name),
            value,
        });
    }
    data
}

fn raised() -> EvaluationLimits {
    EvaluationLimits {
        expression_steps: usize::MAX,
        graph_steps: usize::MAX,
        comparisons: usize::MAX,
        text_steps: usize::MAX,
        events: usize::MAX,
        depth: usize::MAX,
    }
}

#[test]
#[trace("TC-075", "FR-008-AC-18", "FR-008-AC-19")]
fn raised_options_cannot_disable_expression_graph_comparison_text_or_event_ceiling() {
    let models = [model()];
    for (expression, count, dimension, maximum) in [
        (
            "forall(x in self.items: forall(y in self.items: true))",
            1000,
            "expression",
            1_000_000,
        ),
        (
            "forall(x in self.items: reaches(self, self, parent) and reaches(self, self, parent))",
            5001,
            "graph",
            10_000,
        ),
        (
            "forall(x in self.items: forall(y in self.items: x = y))",
            400,
            "comparison",
            100_000,
        ),
        (
            "forall(x in self.items: label = right_label)",
            1025,
            "text",
            1_048_576,
        ),
        (
            "forall(x in self.items: false implies true)",
            5001,
            "event",
            10_000,
        ),
    ] {
        let checked = checked(&models, expression);
        let context = current(&checked, &models[0], data(&models[0], count));
        let normal = evaluate(&context, EvaluationLimits::default(), || false);
        let raised = evaluate(&context, raised(), || false);
        assert!(
            matches!(normal.outcome(), EvaluationOutcome::Incomplete(d) if d.code == Code::ResourceExhausted),
            "{dimension}"
        );
        assert_eq!(normal.outcome(), raised.outcome());
        assert_eq!(normal.events(), raised.events());
        assert_eq!(normal.usage(), raised.usage());
        let usage = normal.usage();
        let used = match dimension {
            "expression" => usage.expression_steps,
            "graph" => usage.graph_steps,
            "comparison" => usage.comparisons,
            "text" => usage.text_steps,
            "event" => usage.events,
            _ => unreachable!(),
        };
        assert_eq!(used, maximum, "{dimension}");
    }
}

#[test]
#[trace("TC-075", "FR-008-AC-18")]
fn exact_hard_graph_comparison_text_and_event_work_can_complete() {
    let models = [model()];
    let ten_comparisons = std::iter::repeat_n("(x = x)", 10)
        .collect::<Vec<_>>()
        .join(" and ");
    let comparisons = format!("forall(x in self.items: {ten_comparisons})");
    for (expression, count, dimension, maximum) in [
        (
            "forall(x in self.items: reaches(self, self, parent) and reaches(self, self, parent))",
            5000,
            "graph",
            10_000,
        ),
        (comparisons.as_str(), 10000, "comparison", 100_000),
        (
            "forall(x in self.items: label = right_label)",
            1024,
            "text",
            1_048_576,
        ),
        (
            "forall(x in self.items: false implies true)",
            5000,
            "event",
            10_000,
        ),
    ] {
        let checked = checked(&models, expression);
        let context = current(&checked, &models[0], data(&models[0], count));
        let report = evaluate(&context, EvaluationLimits::default(), || false);
        assert_eq!(
            report.outcome(),
            &EvaluationOutcome::Completed(true),
            "{dimension}"
        );
        let usage = report.usage();
        let used = match dimension {
            "graph" => usage.graph_steps,
            "comparison" => usage.comparisons,
            "text" => usage.text_steps,
            "event" => usage.events,
            _ => unreachable!(),
        };
        assert_eq!(used, maximum, "{dimension}");
    }
}

#[test]
#[trace("TC-071", "FR-008-AC-14", "FR-008-AC-18")]
fn large_sequence_binding_and_reads_do_not_spend_deep_comparison_work() {
    let models = [model()];
    let checked = checked(
        &models,
        "let xs = self.items in forall(x in xs: true) and forall(y in xs: true)",
    );
    let context = current(&checked, &models[0], data(&models[0], 10000));
    let report = evaluate(
        &context,
        EvaluationLimits {
            comparisons: 0,
            text_steps: 0,
            events: 0,
            graph_steps: 0,
            ..EvaluationLimits::default()
        },
        || false,
    );
    assert_eq!(report.outcome(), &EvaluationOutcome::Completed(true));
    // let + field/self initializer + and + two quantifier/name entries + 20,000 predicates.
    assert_eq!(report.usage().expression_steps, 20008);
    assert_eq!(report.usage().comparisons, 0);
    assert_eq!(report.usage().text_steps, 0);
}

#[test]
#[trace("TC-075", "FR-008-AC-18")]
fn maximum_expression_depth_and_frontend_coupling_are_explicit() {
    let models = [native_rule_model::parts().model()];
    let expression = format!("{}true", "not ".repeat(63));
    let checked = checked(&models, &expression);
    let context = current(&checked, &models[0], draft(&models[0]));
    let report = evaluate(&context, raised(), || false);
    assert_eq!(report.outcome(), &EvaluationOutcome::Completed(false));
    assert_eq!(report.usage().expression_depth, 64);
    let stopped = evaluate(
        &context,
        EvaluationLimits {
            depth: 63,
            ..EvaluationLimits::default()
        },
        || false,
    );
    assert!(
        matches!(stopped.outcome(), EvaluationOutcome::Incomplete(d) if d.code == Code::ResourceExhausted)
    );
    assert_eq!(stopped.usage().expression_steps, 63);
    // A 65-frame source cannot create a checked context under the existing frontend ceilings.
    let too_deep = request(
        &models,
        &format!("not {expression}"),
        quire_spec_language::syntax::ClauseKind::Invariant,
    )
    .unwrap_err();
    assert_eq!(too_deep.code, Code::ResourceExhausted);
    assert_ne!(too_deep.phase, quire_spec_language::Phase::Evaluate);
}
