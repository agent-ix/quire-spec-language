// SPDX-License-Identifier: AGPL-3.0-only
//! TC-060/063: generated native shapes at each supplied value/capture site.

use super::invocation_cases::{authored_model, qualified, recorded};
use super::*;
use quire_spec_language::runtime::RuntimePathSegment;
use serde_json::json;

#[derive(Clone, Copy, Debug)]
enum Shape {
    Boolean,
    Integer,
    Text,
    Enum,
    Record,
    Option,
    Sequence,
    Reference,
    Object,
    Nested,
}

const SHAPES: [Shape; 10] = [
    Shape::Boolean,
    Shape::Integer,
    Shape::Text,
    Shape::Enum,
    Shape::Record,
    Shape::Option,
    Shape::Sequence,
    Shape::Reference,
    Shape::Object,
    Shape::Nested,
];

fn push(arena: &mut Vec<ValueNode>, node: ValueNode) -> ValueId {
    let value = ValueId::new(u32::try_from(arena.len()).unwrap());
    arena.push(node);
    value
}

impl Shape {
    fn ty(self) -> serde_json::Value {
        match self {
            Self::Boolean => json!({"kind": "boolean"}),
            Self::Integer => json!({"kind": "scalar", "name": "Signed"}),
            Self::Text => json!({"kind": "scalar", "name": "ObjectId"}),
            Self::Enum => json!({"kind": "enum", "name": "Mode"}),
            Self::Record => json!({"kind": "record", "name": "Payload"}),
            Self::Option => json!({"kind": "option", "value": Self::Integer.ty()}),
            Self::Sequence => {
                json!({"kind": "sequence", "maximum": 3, "value": Self::Integer.ty()})
            }
            Self::Reference => json!({"kind": "record", "name": "NodeRef"}),
            Self::Object => json!({"kind": "record", "name": "Leaf"}),
            Self::Nested => {
                json!({"kind": "option", "value": {"kind": "sequence", "maximum": 3, "value": Self::Record.ty()}})
            }
        }
    }

    fn model(self) -> NativeModel {
        authored_model(|data| {
            data["enums"] = json!([{"name": "Mode", "variants": ["Cold", "Hot"]}]);
            data["records"]
                .as_array_mut()
                .unwrap()
                .push(json!({"name": "Payload", "fields": [
                    {"name": "number", "type": Self::Integer.ty()},
                    {"name": "flag", "type": Self::Boolean.ty()}
                ]}));
            data["records"].as_array_mut().unwrap().extend([
                json!({"name": "Leaf", "fields": [{"name": "flag", "type": Self::Boolean.ty()}]}),
                json!({"name": "LeafRef", "fields": [{"name": "id", "type": Self::Text.ty()}]}),
            ]);
            data["objects"].as_array_mut().unwrap().push(json!({"record": "Leaf", "reference": "LeafRef", "identity_field": "id", "universe": "leaves"}));
            data["records"][0]["fields"]
                .as_array_mut()
                .unwrap()
                .push(json!({"name": "payload", "type": self.ty()}));
            data["values"].as_array_mut().unwrap().extend([
                json!({"name": "payload_state", "kind": "state", "type": self.ty()}),
                json!({"name": "argument", "kind": "input", "type": self.ty()}),
            ]);
            data["values"][2]["type"] = self.ty();
            data["operations"][0]["parameters"] = json!(["argument"]);
        })
    }

    fn value(self, model: &NativeModel, arena: &mut Vec<ValueNode>, changed: bool) -> ValueId {
        let node = match self {
            Self::Boolean => ValueNode::Boolean { value: changed },
            Self::Integer => ValueNode::Integer {
                value: if changed { 10 } else { -10 },
            },
            Self::Text => ValueNode::Text {
                value: if changed {
                    "界".repeat(256)
                } else {
                    "é".repeat(256)
                },
            },
            Self::Enum => ValueNode::Enum {
                declaration: qualified(model, "Mode"),
                variant: symbol(if changed { "Hot" } else { "Cold" }),
            },
            Self::Record => {
                let number = Self::Integer.value(model, arena, changed);
                let flag = Self::Boolean.value(model, arena, false);
                ValueNode::Record {
                    declaration: qualified(model, "Payload"),
                    fields: vec![
                        FieldBinding {
                            name: symbol("number"),
                            value: number,
                        },
                        FieldBinding {
                            name: symbol("flag"),
                            value: flag,
                        },
                    ],
                }
            }
            Self::Option => {
                if changed {
                    ValueNode::Present {
                        value: Self::Integer.value(model, arena, false),
                    }
                } else {
                    ValueNode::Absent
                }
            }
            Self::Sequence => {
                let first = Self::Integer.value(model, arena, false);
                let second = Self::Integer.value(model, arena, true);
                ValueNode::Sequence {
                    values: if changed {
                        vec![second, first, first]
                    } else {
                        vec![first, second, first]
                    },
                }
            }
            Self::Reference => ValueNode::Reference {
                identity: object(model, if changed { "other" } else { "self" }),
            },
            Self::Object => ValueNode::Object {
                identity: ObjectIdentity {
                    record: symbol("Leaf"),
                    universe: symbol("leaves"),
                    ..object(model, if changed { "other" } else { "self" })
                },
            },
            Self::Nested => {
                let record = Self::Record.value(model, arena, changed);
                let sequence = push(
                    arena,
                    ValueNode::Sequence {
                        values: vec![record, record],
                    },
                );
                ValueNode::Present { value: sequence }
            }
        };
        push(arena, node)
    }

    fn invalid(self, model: &NativeModel, arena: &mut Vec<ValueNode>) -> ValueId {
        let node = match self {
            Self::Boolean => ValueNode::Integer { value: 0 },
            Self::Integer => ValueNode::Integer { value: 11 },
            Self::Text => ValueNode::Text {
                value: "界".repeat(257),
            },
            Self::Enum => ValueNode::Enum {
                declaration: qualified(model, "Mode"),
                variant: symbol("Foreign"),
            },
            Self::Record => ValueNode::Record {
                declaration: qualified(model, "Payload"),
                fields: Vec::new(),
            },
            Self::Option => ValueNode::Present {
                value: Self::Integer.invalid(model, arena),
            },
            Self::Sequence => {
                let child = Self::Integer.value(model, arena, false);
                ValueNode::Sequence {
                    values: vec![child; 4],
                }
            }
            Self::Reference => ValueNode::Object {
                identity: object(model, "self"),
            },
            Self::Object => ValueNode::Reference {
                identity: ObjectIdentity {
                    record: symbol("Leaf"),
                    universe: symbol("leaves"),
                    ..object(model, "self")
                },
            },
            Self::Nested => {
                let child = Self::Record.invalid(model, arena);
                let sequence = push(
                    arena,
                    ValueNode::Sequence {
                        values: vec![child],
                    },
                );
                ValueNode::Present { value: sequence }
            }
        };
        push(arena, node)
    }
}

fn snapshot_values(model: &NativeModel, shape: Shape) -> SnapshotDraft {
    let mut data = draft(model);
    let root = shape.value(model, &mut data.arena, false);
    data.populations[0].objects[0].fields.push(FieldBinding {
        name: symbol("payload"),
        value: root,
    });
    let mut other = data.populations[0].objects[0].clone();
    other.key = "other".into();
    data.populations[0].objects.push(other);
    let flag = push(&mut data.arena, ValueNode::Boolean { value: false });
    data.populations.push(Population {
        model: model.environment().owner().clone(),
        record: symbol("Leaf"),
        universe: symbol("leaves"),
        complete: true,
        objects: ["self", "other"]
            .into_iter()
            .map(|key| ObjectEntry {
                key: key.into(),
                fields: vec![FieldBinding {
                    name: symbol("flag"),
                    value: flag,
                }],
            })
            .collect(),
    });
    data.values.push(ValueBinding {
        declaration: qualified(model, "payload_state"),
        value: root,
    });
    data
}

#[test]
#[trace("TC-060", "FR-007-AC-7", "FR-007-AC-8")]
fn every_native_shape_is_checked_at_fields_state_parameters_and_results() {
    for shape in SHAPES {
        let models = [shape.model()];
        let model = &models[0];
        let checked = checked_kind(&models, "false", ClauseKind::Postcondition);
        for site in 0..5 {
            let mut before = snapshot_values(model, shape);
            let after = before.clone();
            if site == 1 || site == 2 {
                let invalid = shape.invalid(model, &mut before.arena);
                if site == 1 {
                    before.populations[0].objects[0]
                        .fields
                        .last_mut()
                        .unwrap()
                        .value = invalid;
                } else {
                    before.values[0].value = invalid;
                }
            }
            let (input, selected) = recorded(model, before, after, |invocation| {
                let valid = shape.value(model, &mut invocation.arena, false);
                let invalid = shape.invalid(model, &mut invocation.arena);
                invocation.parameters = vec![ValueBinding {
                    declaration: qualified(model, "argument"),
                    value: if site == 3 { invalid } else { valid },
                }];
                invocation.result = Some(if site == 4 { invalid } else { valid });
            });
            let result = validate(
                &checked,
                input,
                selected,
                ValidationLimits::default(),
                || false,
            );
            if site == 0 {
                let context = result.unwrap_or_else(|report| panic!("valid {shape:?}: {report:?}"));
                assert!(context
                    .state(
                        ir::StateObservation::Pre,
                        &qualified(model, "payload_state")
                    )
                    .is_some());
                assert_eq!(context.invocation().unwrap().draft().parameters.len(), 1);
            } else {
                let report = result.unwrap_err();
                assert_eq!(
                    report.status,
                    ValidationStatus::Refused,
                    "{shape:?} at {site}"
                );
                assert!(report.terminal.is_none());
                let expected = match site {
                    1 => RuntimePathSegment::Field(symbol("payload")),
                    2 => RuntimePathSegment::State(qualified(model, "payload_state")),
                    3 => RuntimePathSegment::Parameter(qualified(model, "argument")),
                    4 => RuntimePathSegment::Result,
                    _ => unreachable!(),
                };
                assert!(
                    report.diagnostics.iter().any(|diagnostic| diagnostic.code
                        == Code::InvalidRuntimeInput
                        && diagnostic
                            .runtime
                            .as_ref()
                            .unwrap()
                            .path
                            .contains(&expected)),
                    "{shape:?} at {site}"
                );
            }
        }
    }
}

#[test]
#[trace("TC-063", "FR-007-AC-14")]
fn storage_frames_detect_each_value_kind_and_ignore_record_field_order() {
    for shape in SHAPES {
        let models = [shape.model()];
        let model = &models[0];
        let checked = checked_kind(&models, "false", ClauseKind::Postcondition);
        for changed in [false, true] {
            let before = snapshot_values(model, shape);
            let mut after = before.clone();
            let root = shape.value(model, &mut after.arena, changed);
            if let ValueNode::Record { fields, .. } =
                &mut after.arena[usize::try_from(root.index()).unwrap()]
            {
                fields.reverse();
            }
            after.populations[0].objects[0]
                .fields
                .last_mut()
                .unwrap()
                .value = root;
            after.values[0].value = root;
            let (input, selected) = recorded(model, before, after, |invocation| {
                let value = shape.value(model, &mut invocation.arena, false);
                invocation.parameters = vec![ValueBinding {
                    declaration: qualified(model, "argument"),
                    value,
                }];
                invocation.result = Some(value);
            });
            let result = validate(
                &checked,
                input,
                selected,
                ValidationLimits::default(),
                || false,
            );
            if changed {
                let report = result.unwrap_err();
                assert_eq!(report.status, ValidationStatus::Refused);
                assert!(report.terminal.is_none());
                assert_eq!(
                    report.diagnostics.len(),
                    2,
                    "one field and one State change for {shape:?}: {report:?}"
                );
                assert!(report
                    .diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.code == Code::FrameViolation));
            } else {
                let context = result.unwrap_or_else(|report| panic!("equal {shape:?}: {report:?}"));
                assert!(context
                    .state(
                        ir::StateObservation::Post,
                        &qualified(model, "payload_state")
                    )
                    .is_some());
            }
        }
    }
}
