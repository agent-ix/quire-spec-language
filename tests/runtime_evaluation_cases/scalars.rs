// SPDX-License-Identifier: AGPL-3.0-only
//! TC-069/072/075: exact scalar operations and separate comparison/text budgets.

use super::*;
use quire_spec_language::syntax::ClauseKind;
use quire_spec_language::Phase;
use serde_json::json;

#[test]
#[trace("TC-069", "FR-008-AC-12")]
fn independent_signed_arithmetic_vectors_and_actual_static_refusals() {
    let models = [native_rule_model::parts().model()];
    for (expression, expected) in [
        ("7 div 3", 2),
        ("-7 div 3", -2),
        ("7 div -3", -2),
        ("-7 div -3", 2),
        ("7 rem 3", 1),
        ("-7 rem 3", -1),
        ("7 rem -3", 1),
        ("-7 rem -3", -1),
        ("0 div -3", 0),
        ("0 rem -3", 0),
        ("4 + 6", 10),
        ("-4 - 6", -10),
        ("-2 * 5", -10),
        ("-(-10)", 10),
    ] {
        let checked = checked(&models, &format!("self.signed = {expression}"));
        let mut data = draft(&models[0]);
        change_field(&mut data, "signed", ValueNode::Integer { value: expected });
        let context = current(&checked, &models[0], data);
        assert_eq!(
            evaluate(&context, EvaluationLimits::default(), || false).outcome(),
            &EvaluationOutcome::Completed(true),
            "{expression}"
        );
    }
    let checked = checked(
        &models,
        "self.wide + 0 = self.wide and self.wide * 1 = self.wide",
    );
    for endpoint in [i64::MIN, i64::MAX] {
        let mut data = draft(&models[0]);
        change_field(&mut data, "wide", ValueNode::Integer { value: endpoint });
        let context = current(&checked, &models[0], data);
        assert_eq!(
            evaluate(&context, EvaluationLimits::default(), || false).outcome(),
            &EvaluationOutcome::Completed(true)
        );
    }
    for expression in [
        "self.signed div 0 = self.signed",
        "self.wide div -1 = self.wide",
        "self.signed + 1 = self.signed",
    ] {
        let refused = request(&models, expression, ClauseKind::Invariant).unwrap_err();
        assert_eq!(refused.phase, Phase::Check);
        assert_eq!(refused.code, Code::UndefinedExpression);
    }
}

pub(super) fn text_model() -> NativeModel {
    authored_model(|data| {
        data["scalars"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name":"Label", "kind":"text", "max_scalars":100000}));
        data["values"].as_array_mut().unwrap().extend([
            json!({"name":"label", "kind":"state", "type":{"kind":"scalar", "name":"Label"}}),
            json!({"name":"right_label", "kind":"state", "type":{"kind":"scalar", "name":"Label"}}),
        ]);
    })
}

pub(super) fn text_draft(model: &NativeModel, a: &str, b: &str) -> SnapshotDraft {
    let mut data = draft(model);
    for (name, text) in [("label", a), ("right_label", b)] {
        let value = append(&mut data, ValueNode::Text { value: text.into() });
        data.values.push(ValueBinding {
            declaration: qualified(model, name),
            value,
        });
    }
    data
}

#[test]
#[trace("TC-072", "FR-008-AC-15")]
fn exact_unicode_scalar_order_and_equality_use_no_normalization() {
    let models = [text_model()];
    let less = checked(&models, "label < right_label");
    let equal = checked(&models, "label = right_label");
    for (a, b, order, same) in [
        ("", "", false, true),
        ("", "a", true, false),
        ("a", "ab", true, false),
        ("é", "e\u{301}", false, false),
        ("\u{e000}", "\u{10000}", true, false),
        ("界", "界", false, true),
        ("z", "a", false, false),
    ] {
        for (checked, truth) in [(&less, order), (&equal, same)] {
            let context = current(checked, &models[0], text_draft(&models[0], a, b));
            assert_eq!(
                evaluate(&context, EvaluationLimits::default(), || false).outcome(),
                &EvaluationOutcome::Completed(truth),
                "{a:?}, {b:?}"
            );
        }
    }
}

#[test]
#[trace("TC-075", "FR-008-AC-18", "FR-008-AC-19")]
fn comparison_and_text_limits_stop_at_exact_per_side_advances() {
    let models = [text_model()];
    let checked = checked(&models, "label = right_label");
    for text in ["".to_owned(), "é界".to_owned(), "界".repeat(4096)] {
        let context = current(&checked, &models[0], text_draft(&models[0], &text, &text));
        let steps = 2 * (text.chars().count() + 1);
        let limits = EvaluationLimits {
            comparisons: 1,
            text_steps: steps,
            expression_steps: 3,
            ..EvaluationLimits::default()
        };
        let report = evaluate(&context, limits, || false);
        assert_eq!(report.outcome(), &EvaluationOutcome::Completed(true));
        assert_eq!(report.usage().comparisons, 1);
        assert_eq!(report.usage().text_steps, steps);
        assert_eq!(report.usage().expression_steps, 3);
        for lowered in [
            EvaluationLimits {
                comparisons: 0,
                ..limits
            },
            EvaluationLimits {
                text_steps: steps - 1,
                ..limits
            },
            EvaluationLimits {
                text_steps: 0,
                ..limits
            },
        ] {
            let stopped = evaluate(&context, lowered, || false);
            assert!(
                matches!(stopped.outcome(), EvaluationOutcome::Incomplete(d) if d.code == Code::ResourceExhausted)
            );
            assert_eq!(stopped.usage().expression_steps, 3);
            assert_eq!(
                stopped.usage().text_steps,
                if lowered.comparisons == 0 {
                    0
                } else {
                    lowered.text_steps
                }
            );
        }
    }
}

#[test]
#[trace("TC-069", "FR-008-AC-12", "FR-008-AC-15")]
fn scalar_comparison_truth_tables_and_invalid_runtime_integer_stage() {
    let models = [native_rule_model::parts().model()];
    for (operator, expected) in [
        ("<", [true, false, false]),
        ("<=", [true, true, false]),
        (">", [false, false, true]),
        (">=", [false, true, true]),
        ("=", [false, true, false]),
        ("!=", [true, false, true]),
    ] {
        let checked = checked(&models, &format!("self.signed {operator} self.den"));
        for (value, truth) in [-1, 0, 1].into_iter().zip(expected) {
            let mut data = draft(&models[0]);
            change_field(&mut data, "signed", ValueNode::Integer { value });
            change_field(&mut data, "den", ValueNode::Integer { value: 0 });
            let context = current(&checked, &models[0], data);
            assert_eq!(
                evaluate(&context, EvaluationLimits::default(), || false).outcome(),
                &EvaluationOutcome::Completed(truth)
            );
        }
    }
    let checked = checked(&models, "self.signed + 0 = self.signed");
    let mut data = draft(&models[0]);
    change_field(&mut data, "signed", ValueNode::Integer { value: 11 });
    let artifact = snapshot(data);
    let selected = selection(&models[0], artifact.reference());
    let refused = validate(
        &checked,
        input(artifact),
        selected,
        ValidationLimits::default(),
        || false,
    )
    .unwrap_err();
    assert_eq!(
        refused.status,
        quire_spec_language::runtime::ValidationStatus::Refused
    );
    assert!(refused
        .diagnostics
        .iter()
        .any(|d| d.phase == Phase::Validate && d.code == Code::InvalidRuntimeInput));
}
