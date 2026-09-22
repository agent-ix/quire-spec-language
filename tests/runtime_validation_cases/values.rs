// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-060/063: generated native shapes at each supplied value/capture site.

use super::setup::{authored_model, qualified, recorded};
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
    ReferenceRecord,
    Object,
    Nested,
}

const SHAPES: [Shape; 11] = [
    Shape::Boolean,
    Shape::Integer,
    Shape::Text,
    Shape::Enum,
    Shape::Record,
    Shape::Option,
    Shape::Sequence,
    Shape::Reference,
    Shape::ReferenceRecord,
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
            Self::ReferenceRecord => json!({"kind": "record", "name": "ReferencePayload"}),
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
                json!({"name": "ReferencePayload", "fields": [{"name": "target", "type": Self::Reference.ty()}]}),
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
            Self::ReferenceRecord => ValueNode::Record {
                declaration: qualified(model, "ReferencePayload"),
                fields: vec![FieldBinding {
                    name: symbol("target"),
                    value: Self::Reference.value(model, arena, changed),
                }],
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
            Self::ReferenceRecord => ValueNode::Record {
                declaration: qualified(model, "ReferencePayload"),
                fields: vec![FieldBinding {
                    name: symbol("target"),
                    value: Self::Reference.invalid(model, arena),
                }],
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
                    report
                        .diagnostics
                        .iter()
                        .any(
                            |diagnostic| diagnostic.diagnostic.code == Code::InvalidRuntimeInput
                                && diagnostic.runtime.path.contains(&expected)
                        ),
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
        for variant in 0..3 {
            if variant == 2 && !matches!(shape, Shape::Sequence) {
                continue;
            }
            let changed = variant != 0;
            let before = snapshot_values(model, shape);
            let mut after = before.clone();
            let root = shape.value(model, &mut after.arena, changed);
            if variant == 2 {
                let ValueNode::Sequence { values } =
                    &mut after.arena[usize::try_from(root.index()).unwrap()]
                else {
                    unreachable!()
                };
                values.pop();
            }
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
                    .all(|diagnostic| diagnostic.diagnostic.code == Code::FrameViolation));
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

#[test]
#[trace("TC-060", "FR-007-AC-7")]
fn nominal_owners_record_shapes_and_signed_lower_bounds_are_not_inferred_from_values() {
    for shape in [Shape::Enum, Shape::Record, Shape::Integer] {
        let models = [shape.model()];
        let model = &models[0];
        let checked = checked(&models, "false");
        for variant in 0..4 {
            let mut data = snapshot_values(model, shape);
            let mut root = data.values[0].value;
            let node = &mut data.arena[usize::try_from(root.index()).unwrap()];
            match node {
                ValueNode::Enum {
                    declaration,
                    variant: name,
                } => match variant {
                    0 => declaration.model = authored_owner(),
                    1 => declaration.name = symbol("DifferentMode"),
                    2 => *name = symbol("Unknown"),
                    3 => {
                        *node = ValueNode::Text {
                            value: "Cold".into(),
                        }
                    }
                    _ => unreachable!(),
                },
                ValueNode::Record {
                    declaration,
                    fields,
                } => match variant {
                    0 => declaration.model = authored_owner(),
                    1 => declaration.name = symbol("DifferentPayload"),
                    2 => fields.push(fields[0].clone()),
                    3 => fields.push(field("extra", 0)),
                    _ => unreachable!(),
                },
                ValueNode::Integer { .. } => {
                    // Shared values at unrelated scalar sites remain typed per use.
                    root = push(
                        &mut data.arena,
                        match variant {
                            0 => ValueNode::Integer { value: -11 },
                            1 => ValueNode::Integer { value: i64::MIN },
                            2 => ValueNode::Integer { value: i64::MAX },
                            3 => ValueNode::Text {
                                value: "-10".into(),
                            },
                            _ => unreachable!(),
                        },
                    );
                    data.values[0].value = root;
                }
                _ => unreachable!(),
            }
            let artifact = snapshot(data);
            let selected = selection(model, artifact.reference());
            let report = validate(
                &checked,
                input(artifact),
                selected,
                ValidationLimits::default(),
                || false,
            )
            .unwrap_err();
            assert_eq!(
                report.status,
                ValidationStatus::Refused,
                "{shape:?} mutation {variant}, value {root:?}"
            );
            assert!(report.terminal.is_none());
            assert!(report
                .diagnostics
                .iter()
                .any(
                    |diagnostic| diagnostic.diagnostic.code == Code::InvalidRuntimeInput
                        && diagnostic
                            .runtime
                            .path
                            .contains(&RuntimePathSegment::State(qualified(
                                model,
                                "payload_state"
                            )))
                ));
        }
    }
}

#[test]
#[trace("TC-064", "TC-066", "FR-007-AC-13", "FR-007-AC-15")]
fn mixed_defects_preserve_diagnostics_under_inventory_population_object_and_field_permutations() {
    use quire_spec_language::runtime::RuntimeReference;
    let shape = Shape::Record;
    let models = [shape.model()];
    let model = &models[0];
    let checked = checked_kind(&models, "false", ClauseKind::Postcondition);
    let mut before = snapshot_values(model, shape);
    before.populations[1].complete = false;
    before.models.push(ModelBinding {
        model: authored_owner(),
        digest: model.digest(),
    });
    let mut after = snapshot_values(model, shape);
    let invalid = push(&mut after.arena, ValueNode::Integer { value: 1001 });
    after.populations[0].objects[0]
        .fields
        .iter_mut()
        .find(|field| field.name.as_str() == "n")
        .unwrap()
        .value = invalid;
    let unauthorized = push(&mut after.arena, ValueNode::Integer { value: 2 });
    after.populations[0].objects[0]
        .fields
        .iter_mut()
        .find(|field| field.name.as_str() == "count")
        .unwrap()
        .value = unauthorized;
    let mut baseline_references: Vec<RuntimeReference> = Vec::new();
    let mut baseline = None;
    for mask in 0..8 {
        let mut before = before.clone();
        let mut after = after.clone();
        for data in [&mut before, &mut after] {
            if mask & 1 != 0 {
                data.populations.reverse();
            }
            if mask & 2 != 0 {
                for population in &mut data.populations {
                    population.objects.reverse();
                    for object in &mut population.objects {
                        object.fields.reverse();
                    }
                }
            }
        }
        let (mut offered, selected) = recorded(model, before, after, |invocation| {
            let value = shape.value(model, &mut invocation.arena, false);
            invocation.parameters.push(ValueBinding {
                declaration: qualified(model, "argument"),
                value,
            });
            invocation.result = Some(value);
        });
        if mask & 4 != 0 {
            offered.snapshots.reverse();
        }
        let actual: Vec<_> = offered
            .snapshots
            .iter()
            .map(|snapshot| RuntimeReference::Snapshot(snapshot.reference()))
            .chain(
                offered
                    .invocations
                    .iter()
                    .map(|invocation| RuntimeReference::Invocation(invocation.reference())),
            )
            .collect();
        let mut report = validate(
            &checked,
            offered,
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap_err();
        assert_eq!(report.status, ValidationStatus::Refused);
        assert!(report.terminal.is_none());
        for code in [
            Code::InvalidModelBinding,
            Code::InvalidRuntimeInput,
            Code::IncompletePopulation,
            Code::FrameViolation,
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.diagnostic.code == code),
                "mask {mask}, code {code:?}"
            );
        }
        if mask == 0 {
            baseline_references = actual.clone();
        }
        for diagnostic in &mut report.diagnostics {
            let runtime = &mut diagnostic.runtime;
            assert!(
                actual.contains(&runtime.artifact),
                "diagnostics bind this permutation's actual bytes"
            );
            runtime.artifact = baseline_references
                .iter()
                .find(|reference| reference.identity() == runtime.artifact.identity())
                .unwrap()
                .clone();
        }
        if let Some((diagnostics, usage)) = &baseline {
            assert_eq!(
                &report.diagnostics, diagnostics,
                "logical diagnostic permutation {mask}"
            );
            assert_eq!(&report.usage, usage);
        } else {
            baseline = Some((report.diagnostics, report.usage));
        }
    }
}

#[test]
#[trace("TC-063", "TC-064", "FR-007-AC-5", "FR-007-AC-14")]
fn ambiguous_sequence_element_does_not_hide_changed_available_elements() {
    let shape = Shape::Nested;
    let models = [shape.model()];
    let model = &models[0];
    let checked = checked_kind(&models, "false", ClauseKind::Postcondition);
    let mut before = snapshot_values(model, shape);
    before.populations[0].objects.truncate(1);
    let root = before.values[0].value;
    let ValueNode::Present { value: sequence } =
        before.arena[usize::try_from(root.index()).unwrap()]
    else {
        unreachable!()
    };
    let ValueNode::Sequence { ref values } =
        before.arena[usize::try_from(sequence.index()).unwrap()]
    else {
        unreachable!()
    };
    let first = values[0];
    let second = Shape::Record.value(model, &mut before.arena, false);
    let ValueNode::Record { ref mut fields, .. } =
        before.arena[usize::try_from(first.index()).unwrap()]
    else {
        unreachable!()
    };
    fields.push(fields[1].clone());
    let sequence = push(
        &mut before.arena,
        ValueNode::Sequence {
            values: vec![first, second],
        },
    );
    let root = push(&mut before.arena, ValueNode::Present { value: sequence });
    before.values[0].value = root;
    before.populations[0].objects[0]
        .fields
        .last_mut()
        .unwrap()
        .value = root;
    let mut after = before.clone();
    let changed = Shape::Record.value(model, &mut after.arena, true);
    let sequence = push(
        &mut after.arena,
        ValueNode::Sequence {
            values: vec![first, changed],
        },
    );
    let root = push(&mut after.arena, ValueNode::Present { value: sequence });
    after.values[0].value = root;
    after.populations[0].objects[0]
        .fields
        .last_mut()
        .unwrap()
        .value = root;
    let (input, selected) = recorded(model, before, after, |invocation| {
        let value = shape.value(model, &mut invocation.arena, false);
        invocation.parameters.push(ValueBinding {
            declaration: qualified(model, "argument"),
            value,
        });
        invocation.result = Some(value);
    });
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
        .any(|diagnostic| diagnostic.diagnostic.code == Code::InvalidRuntimeInput));
    assert_eq!(
        report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.diagnostic.code == Code::FrameViolation)
            .count(),
        2,
        "the known second-element change affects both the object field and State root"
    );
}
