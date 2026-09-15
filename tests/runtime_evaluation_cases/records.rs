// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-072/075: eligible nominal/nested records and bounded recursive comparisons.

use super::*;
use quire_spec_language::runtime::FieldBinding;
use serde_json::json;

fn model() -> NativeModel {
    authored_model(|data| {
        data["enums"] = json!([{"name":"Mode", "variants":["On", "Off"]}]);
        data["records"].as_array_mut().unwrap().extend([
            json!({"name":"Payload", "fields":[
                {"name":"text", "type":{"kind":"scalar", "name":"ObjectId"}},
                {"name":"reference", "type":{"kind":"record", "name":"NodeRef"}},
                {"name":"flag", "type":{"kind":"boolean"}},
                {"name":"mode", "type":{"kind":"enum", "name":"Mode"}}
            ]}),
            json!({"name":"Pair", "fields":[
                {"name":"z", "type":{"kind":"record", "name":"Payload"}},
                {"name":"a", "type":{"kind":"scalar", "name":"Version"}}
            ]}),
        ]);
        for name in ["pair", "right_pair"] {
            data["values"].as_array_mut().unwrap().push(
                json!({"name":name, "kind":"state", "type":{"kind":"record", "name":"Pair"}}),
            );
        }
    })
}

fn data(model: &NativeModel, a: i64, mode: &str) -> SnapshotDraft {
    let mut data = draft(model);
    for (name, number, variant) in [("pair", 1, "On"), ("right_pair", a, mode)] {
        let number = append(&mut data, ValueNode::Integer { value: number });
        let text = append(
            &mut data,
            ValueNode::Text {
                value: "é界".into(),
            },
        );
        let flag = append(&mut data, ValueNode::Boolean { value: true });
        let mode = append(
            &mut data,
            ValueNode::Enum {
                declaration: qualified(model, "Mode"),
                variant: symbol(variant),
            },
        );
        let mut fields = vec![
            FieldBinding {
                name: symbol("text"),
                value: text,
            },
            FieldBinding {
                name: symbol("flag"),
                value: flag,
            },
            FieldBinding {
                name: symbol("mode"),
                value: mode,
            },
            FieldBinding {
                name: symbol("reference"),
                value: ValueId::new(3),
            },
        ];
        if name == "right_pair" {
            fields.reverse();
        }
        let payload = append(
            &mut data,
            ValueNode::Record {
                declaration: qualified(model, "Payload"),
                fields,
            },
        );
        let mut fields = vec![
            FieldBinding {
                name: symbol("z"),
                value: payload,
            },
            FieldBinding {
                name: symbol("a"),
                value: number,
            },
        ];
        if name == "right_pair" {
            fields.reverse();
        }
        let value = append(
            &mut data,
            ValueNode::Record {
                declaration: qualified(model, "Pair"),
                fields,
            },
        );
        data.values.push(ValueBinding {
            declaration: qualified(model, name),
            value,
        });
    }
    data
}

#[test]
#[trace("TC-072", "FR-008-AC-15", "FR-008-AC-18")]
fn eligible_nested_record_equality_uses_declared_names_and_types() {
    let models = [model()];
    let checked = checked(&models, "pair = right_pair");
    for (number, mode, truth, pairs, text_steps) in [
        (1, "On", true, 7, 6),
        (2, "On", false, 2, 0),
        (1, "Off", false, 5, 0),
    ] {
        let context = current(&checked, &models[0], data(&models[0], number, mode));
        let limits = EvaluationLimits {
            comparisons: pairs,
            text_steps,
            ..EvaluationLimits::default()
        };
        let report = evaluate(&context, limits, || false);
        assert_eq!(report.outcome(), &EvaluationOutcome::Completed(truth));
        assert_eq!(report.usage().comparisons, pairs);
        assert_eq!(report.usage().text_steps, text_steps);
        let stopped = evaluate(
            &context,
            EvaluationLimits {
                comparisons: pairs - 1,
                ..limits
            },
            || false,
        );
        assert!(
            matches!(stopped.outcome(), EvaluationOutcome::Incomplete(d) if d.code == Code::ResourceExhausted)
        );
    }
}

#[test]
#[trace("TC-072", "FR-008-AC-15")]
fn distinct_equal_valued_objects_remain_unequal_and_enum_literals_are_nominal() {
    let models = [model()];
    for (expression, truth) in [
        ("self = other", false),
        ("self != other", true),
        ("M::Mode::On = M::Mode::On", true),
        ("M::Mode::On = M::Mode::Off", false),
    ] {
        let checked = checked(&models, expression);
        let mut data = data(&models[0], 1, "On");
        let mut other = data.populations[0].objects[0].clone();
        other.key = "other".into();
        data.populations[0].objects.push(other);
        let value = append(
            &mut data,
            ValueNode::Object {
                identity: object(&models[0], "other"),
            },
        );
        data.values.push(ValueBinding {
            declaration: qualified(&models[0], "other"),
            value,
        });
        let context = current(&checked, &models[0], data);
        assert_eq!(
            evaluate(&context, EvaluationLimits::default(), || false).outcome(),
            &EvaluationOutcome::Completed(truth),
            "{expression}"
        );
    }
}

#[test]
#[trace("TC-075", "FR-008-AC-18")]
fn deeply_shared_records_use_separate_comparison_depth_and_exact_pairs() {
    for levels in [1, 8, 63] {
        let models = [authored_model(|data| {
            for level in 0..levels {
                let ty = if level == 0 {
                    json!({"kind":"boolean"})
                } else {
                    json!({"kind":"record", "name":format!("Level{}", level - 1)})
                };
                data["records"].as_array_mut().unwrap().push(
                    json!({"name":format!("Level{level}"), "fields":[{"name":"child", "type":ty}]}),
                );
            }
            data["values"].as_array_mut().unwrap().push(json!({"name":"deep", "kind":"state", "type":{"kind":"record", "name":format!("Level{}", levels - 1)}}));
        })];
        let checked = checked(&models, "deep = deep");
        let mut data = draft(&models[0]);
        let mut value = append(&mut data, ValueNode::Boolean { value: true });
        for level in 0..levels {
            value = append(
                &mut data,
                ValueNode::Record {
                    declaration: qualified(&models[0], &format!("Level{level}")),
                    fields: vec![FieldBinding {
                        name: symbol("child"),
                        value,
                    }],
                },
            );
        }
        data.values.push(ValueBinding {
            declaration: qualified(&models[0], "deep"),
            value,
        });
        let context = current(&checked, &models[0], data);
        let depth = levels + 1;
        let limits = EvaluationLimits {
            depth,
            comparisons: depth,
            ..EvaluationLimits::default()
        };
        let report = evaluate(&context, limits, || false);
        assert_eq!(report.outcome(), &EvaluationOutcome::Completed(true));
        assert_eq!(report.usage().comparison_depth, depth);
        assert_eq!(report.usage().expression_depth, 2);
        assert_eq!(report.usage().comparisons, depth);
        let stopped = evaluate(
            &context,
            EvaluationLimits {
                depth: depth - 1,
                ..limits
            },
            || false,
        );
        assert!(
            matches!(stopped.outcome(), EvaluationOutcome::Incomplete(d) if d.code == Code::ResourceExhausted)
        );
        // At depth one, expression evaluation stops before either record is read.
        assert_eq!(
            stopped.usage().comparisons,
            if depth == 2 { 0 } else { depth - 1 }
        );
    }
}

#[test]
#[trace("TC-072", "FR-008-AC-15")]
fn distinct_declared_universes_cannot_enter_equality_through_matching_keys() {
    let models = [authored_model(|data| {
        data["records"].as_array_mut().unwrap().extend([
            json!({"name":"Alien", "fields":[{"name":"flag", "type":{"kind":"boolean"}}]}),
            json!({"name":"AlienRef", "fields":[{"name":"id", "type":{"kind":"scalar", "name":"ObjectId"}}]}),
        ]);
        data["objects"].as_array_mut().unwrap().push(json!({"record":"Alien", "reference":"AlienRef", "identity_field":"id", "universe":"aliens"}));
        data["values"].as_array_mut().unwrap().push(
            json!({"name":"alien", "kind":"state", "type":{"kind":"record", "name":"Alien"}}),
        );
    })];
    let refused = request(
        &models,
        "self = alien",
        quire_spec_language::syntax::ClauseKind::Invariant,
    )
    .unwrap_err();
    assert_eq!(refused.phase, quire_spec_language::Phase::Check);
    assert_eq!(refused.code, Code::IllTyped);
}
