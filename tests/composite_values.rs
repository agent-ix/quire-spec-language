// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-188 records, tuples and finite recursive values over the real `value`
//! boundary (FR-143).
//!
//! Declaration node keys are opaque fixture keys: the pinned specification
//! defines no record, tuple or variant declaration preimage (SPEC-GAP(119-5)).
//! Declarations that intentionally share names and shapes get distinct keys.

use ix_trace_rs::trace;
use quire_spec_language::value::{
    evaluate_equality, Component, CompositeDeclaration, CompositeShape, ConstructionCause,
    ConstructionRefusal, ConstructorDeclaration, DeclarationCause, FieldDeclaration, FieldValue,
    GraphCause, GraphNode, GraphNodeId, GraphRefusal, GraphSlot, IllTyped, IllTypedCause, Integer,
    InvalidDeclaration, Meter, NodeKey, ObjectEnvironment, ObjectEnvironmentCause,
    ObjectEnvironmentRefusal, ObjectIdentity, ObjectReference, OptionValue, Outcome, Presence,
    ScalarLimits, TypeEnvironment, Value, ValueGraph, ValueType,
};
use sha2::{Digest, Sha256};

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

fn key(label: &str) -> NodeKey {
    let digest: String = Sha256::digest(label.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    NodeKey::from_hex(&digest).unwrap()
}

fn int(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

fn field(label: &str, value_type: ValueType, presence: Presence) -> FieldDeclaration {
    FieldDeclaration::new(key(label), value_type, presence)
}

fn equal(left: &Value, right: &Value) -> Result<Outcome<bool>, IllTyped> {
    evaluate_equality(left, right, &mut Meter::new(UNLIMITED))
}

fn refusal(component: Component, cause: ConstructionCause) -> ConstructionRefusal {
    ConstructionRefusal { component, cause }
}

/// `record-A` and `record-B { x: Integer, y: Integer }` sharing field
/// identities; `tuple-A` and `tuple-B (Integer, Integer)`; `Slots { req,
/// opt?, nul!, both?! }`; `List = Nil | Cons { head, tail }`.
fn environment() -> TypeEnvironment {
    let point = || {
        CompositeShape::Record(vec![
            field("x", ValueType::Integer, Presence::Required),
            field("y", ValueType::Integer, Presence::Required),
        ])
    };
    let pair = || CompositeShape::Tuple(vec![ValueType::Integer, ValueType::Integer]);
    TypeEnvironment::new([
        CompositeDeclaration::new(key("record-A"), point()),
        CompositeDeclaration::new(key("record-B"), point()),
        CompositeDeclaration::new(key("tuple-A"), pair()),
        CompositeDeclaration::new(key("tuple-B"), pair()),
        CompositeDeclaration::new(
            key("Slots"),
            CompositeShape::Record(vec![
                field("req", ValueType::Integer, Presence::Required),
                field("opt", ValueType::Integer, Presence::Optional),
                field("nul", ValueType::Integer, Presence::Nullable),
                field("both", ValueType::Integer, Presence::OptionalNullable),
            ]),
        ),
        list_declaration(),
    ])
    .unwrap()
}

fn list_declaration() -> CompositeDeclaration {
    CompositeDeclaration::new(
        key("List"),
        CompositeShape::Variant(vec![
            ConstructorDeclaration::new(key("Nil"), vec![]),
            ConstructorDeclaration::new(
                key("Cons"),
                vec![
                    field("head", ValueType::Integer, Presence::Required),
                    field(
                        "tail",
                        ValueType::Composite(key("List")),
                        Presence::Required,
                    ),
                ],
            ),
        ]),
    )
}

fn point(env: &TypeEnvironment, declaration: &str, x: i64, y: i64) -> Value {
    env.record(
        key(declaration),
        vec![
            (key("x"), FieldValue::Present(int(x))),
            (key("y"), FieldValue::Present(int(y))),
        ],
    )
    .unwrap()
}

fn pair(env: &TypeEnvironment, declaration: &str, first: i64, second: i64) -> Value {
    env.tuple(key(declaration), vec![int(first), int(second)])
        .unwrap()
}

#[trace("TC-188", "FR-143-AC-1")]
#[test]
fn equal_records_and_tuples_of_one_declaration_compare_structurally() {
    let env = environment();
    assert_eq!(
        equal(
            &point(&env, "record-A", 1, 2),
            &point(&env, "record-A", 1, 2)
        ),
        Ok(Outcome::Completed(true))
    );
    assert_eq!(
        equal(
            &point(&env, "record-A", 1, 2),
            &point(&env, "record-A", 1, 3)
        ),
        Ok(Outcome::Completed(false))
    );
    assert_eq!(
        equal(&pair(&env, "tuple-A", 1, 2), &pair(&env, "tuple-A", 1, 2)),
        Ok(Outcome::Completed(true))
    );
    assert_eq!(
        equal(&pair(&env, "tuple-A", 1, 2), &pair(&env, "tuple-A", 2, 1)),
        Ok(Outcome::Completed(false))
    );
}

#[trace("TC-188", "FR-143-AC-2")]
#[test]
fn equal_shapes_of_different_declarations_are_not_interchangeable() {
    let env = environment();
    let distinct = Err(IllTyped {
        cause: IllTypedCause::DistinctDeclarations,
    });
    assert_eq!(
        equal(
            &point(&env, "record-A", 1, 2),
            &point(&env, "record-B", 1, 2)
        ),
        distinct
    );
    assert_eq!(
        equal(&pair(&env, "tuple-A", 1, 2), &pair(&env, "tuple-B", 1, 2)),
        distinct
    );
    // A record-B value is not a member of a record-A field type.
    let holder = TypeEnvironment::new([
        CompositeDeclaration::new(
            key("record-A"),
            CompositeShape::Record(vec![field("x", ValueType::Integer, Presence::Required)]),
        ),
        CompositeDeclaration::new(
            key("record-B"),
            CompositeShape::Record(vec![field("x", ValueType::Integer, Presence::Required)]),
        ),
        CompositeDeclaration::new(
            key("Holder"),
            CompositeShape::Tuple(vec![ValueType::Composite(key("record-A"))]),
        ),
    ])
    .unwrap();
    let b = holder
        .record(
            key("record-B"),
            vec![(key("x"), FieldValue::Present(int(1)))],
        )
        .unwrap();
    assert_eq!(
        holder.tuple(key("Holder"), vec![b]).unwrap_err(),
        refusal(Component::Position(0), ConstructionCause::TypeMismatch)
    );
}

#[trace("TC-188", "FR-143-AC-5")]
#[test]
fn canonical_field_order_does_not_equate_different_declarations() {
    let env = environment();
    let reordered = env
        .record(
            key("record-A"),
            vec![
                (key("y"), FieldValue::Present(int(2))),
                (key("x"), FieldValue::Present(int(1))),
            ],
        )
        .unwrap();
    // Supplied order is not identity: the same declaration still compares equal.
    assert_eq!(
        equal(&reordered, &point(&env, "record-A", 1, 2)),
        Ok(Outcome::Completed(true))
    );
    // Identical canonical field order and field identities still differ by
    // declaration.
    assert_eq!(
        equal(&reordered, &point(&env, "record-B", 1, 2)),
        Err(IllTyped {
            cause: IllTypedCause::DistinctDeclarations
        })
    );
}

fn node(id: u64) -> GraphNodeId {
    GraphNodeId(id)
}

fn cons_node(head: i64, tail: GraphNodeId) -> GraphNode {
    GraphNode::Variant {
        declaration: key("List"),
        constructor: key("Cons"),
        fields: vec![
            (key("head"), GraphSlot::Value(int(head))),
            (key("tail"), GraphSlot::Node(tail)),
        ],
    }
}

fn graph(nodes: Vec<(GraphNodeId, GraphNode)>) -> ValueGraph {
    ValueGraph::new(nodes).unwrap()
}

fn nil_node() -> GraphNode {
    GraphNode::Variant {
        declaration: key("List"),
        constructor: key("Nil"),
        fields: vec![],
    }
}

#[trace("TC-188", "FR-143-AC-3")]
#[test]
fn containment_cycles_refuse_while_finite_recursion_and_sharing_construct() {
    let env = environment();
    // Two lists [1, 2] sharing one immutable tail node form a DAG.
    let shared = graph(vec![
        (node(0), nil_node()),
        (node(1), cons_node(2, node(0))),
        (node(2), cons_node(1, node(1))),
        (
            node(3),
            GraphNode::Tuple {
                declaration: key("Lists"),
                positions: vec![GraphSlot::Node(node(2)), GraphSlot::Node(node(1))],
            },
        ),
    ]);
    let lists = TypeEnvironment::new([
        list_declaration(),
        CompositeDeclaration::new(
            key("Lists"),
            CompositeShape::Tuple(vec![
                ValueType::Composite(key("List")),
                ValueType::Composite(key("List")),
            ]),
        ),
    ])
    .unwrap();
    assert!(lists.build(&shared, node(3)).is_ok());
    let first = env.build(&shared, node(2)).unwrap();
    let again = env.build(&shared, node(2)).unwrap();
    assert_eq!(equal(&first, &again), Ok(Outcome::Completed(true)));

    // A containment back-edge refuses at the node that closes the cycle.
    let cyclic = graph(vec![
        (node(0), cons_node(1, node(1))),
        (node(1), cons_node(2, node(0))),
    ]);
    assert_eq!(
        env.build(&cyclic, node(0)).unwrap_err(),
        GraphRefusal {
            node: node(0),
            cause: GraphCause::ContainmentCycle
        }
    );
    let self_loop = graph(vec![(node(7), cons_node(1, node(7)))]);
    assert_eq!(
        env.build(&self_loop, node(7)).unwrap_err(),
        GraphRefusal {
            node: node(7),
            cause: GraphCause::ContainmentCycle
        }
    );
    assert_eq!(
        env.build(&cyclic, node(9)).unwrap_err(),
        GraphRefusal {
            node: node(9),
            cause: GraphCause::UnknownNode
        }
    );

    // A recursion edge that crosses no named field refuses the declaration.
    let unnamed = TypeEnvironment::new([CompositeDeclaration::new(
        key("Loop"),
        CompositeShape::Tuple(vec![ValueType::Option(Box::new(ValueType::Composite(
            key("Loop"),
        )))]),
    )]);
    assert_eq!(
        unnamed.unwrap_err(),
        InvalidDeclaration {
            declaration: key("Loop"),
            cause: DeclarationCause::UnnamedRecursion
        }
    );
    // Recursion through a named record field is legal and finite.
    let chain = TypeEnvironment::new([CompositeDeclaration::new(
        key("Chain"),
        CompositeShape::Record(vec![field(
            "next",
            ValueType::Option(Box::new(ValueType::Composite(key("Chain")))),
            Presence::Required,
        )]),
    )])
    .unwrap();
    let none = OptionValue::none(ValueType::Composite(key("Chain")));
    let end = chain
        .record(key("Chain"), vec![(key("next"), FieldValue::Present(none))])
        .unwrap();
    let link = OptionValue::present(ValueType::Composite(key("Chain")), end).unwrap();
    assert!(chain
        .record(key("Chain"), vec![(key("next"), FieldValue::Present(link))])
        .is_ok());

    // Object-reference cycles are admitted in a closed object environment and
    // stay distinct from value containment.
    let objects = TypeEnvironment::new([CompositeDeclaration::new(
        key("Account"),
        CompositeShape::Record(vec![field(
            "peer",
            ValueType::Reference(key("Account")),
            Presence::Required,
        )]),
    )])
    .unwrap();
    let reference = |name: &str| {
        ObjectReference::new(
            key("Account"),
            ObjectIdentity::new(vec!["accounts".into(), name.into()]).unwrap(),
        )
    };
    let state = |peer: &str| {
        objects
            .record(
                key("Account"),
                vec![(
                    key("peer"),
                    FieldValue::Present(Value::Reference(reference(peer))),
                )],
            )
            .unwrap()
    };
    let closed =
        ObjectEnvironment::new([(reference("a"), state("b")), (reference("b"), state("a"))])
            .unwrap();
    assert!(closed.resolve(&reference("a")).is_some());
    assert_eq!(
        ObjectEnvironment::new([(reference("a"), state("missing"))]).unwrap_err(),
        ObjectEnvironmentRefusal {
            identity: reference("a").identity().clone(),
            cause: ObjectEnvironmentCause::DanglingReference(
                reference("missing").identity().clone()
            ),
        }
    );
}

#[trace("TC-188", "FR-143-AC-4")]
#[test]
fn malformed_components_refuse_at_their_origin() {
    let env = environment();
    let slots = |fields: Vec<(&str, FieldValue)>| {
        env.record(
            key("Slots"),
            fields
                .into_iter()
                .map(|(label, value)| (key(label), value))
                .collect(),
        )
    };
    let complete = || {
        vec![
            ("req", FieldValue::Present(int(1))),
            ("opt", FieldValue::Absent),
            ("nul", FieldValue::Null),
            ("both", FieldValue::Null),
        ]
    };
    assert!(slots(complete()).is_ok());
    let with = |label: &'static str, value: FieldValue| {
        let mut fields = complete();
        fields.retain(|(name, _)| *name != label);
        fields.push((label, value));
        fields
    };
    let cases = [
        (
            complete().into_iter().skip(1).collect::<Vec<_>>(),
            refusal(
                Component::Field(key("req")),
                ConstructionCause::MissingField,
            ),
        ),
        (
            with("extra", FieldValue::Present(int(1))),
            refusal(
                Component::Field(key("extra")),
                ConstructionCause::ExtraField,
            ),
        ),
        (
            {
                let mut fields = complete();
                fields.push(("req", FieldValue::Present(int(2))));
                fields
            },
            refusal(
                Component::Field(key("req")),
                ConstructionCause::DuplicateField,
            ),
        ),
        (
            with("req", FieldValue::Absent),
            refusal(
                Component::Field(key("req")),
                ConstructionCause::AbsenceNotAdmitted,
            ),
        ),
        // Absence is not substituted for a nullable field's null, nor null
        // for an optional field's absence.
        (
            with("nul", FieldValue::Absent),
            refusal(
                Component::Field(key("nul")),
                ConstructionCause::AbsenceNotAdmitted,
            ),
        ),
        (
            with("opt", FieldValue::Null),
            refusal(
                Component::Field(key("opt")),
                ConstructionCause::NullNotAdmitted,
            ),
        ),
        (
            with("both", FieldValue::Present(Value::Boolean(true))),
            refusal(
                Component::Field(key("both")),
                ConstructionCause::TypeMismatch,
            ),
        ),
    ];
    for (fields, expected) in cases {
        assert_eq!(slots(fields).unwrap_err(), expected);
    }

    assert_eq!(
        env.tuple(key("tuple-A"), vec![int(1)]).unwrap_err(),
        refusal(
            Component::Value,
            ConstructionCause::WrongArity {
                declared: 2,
                supplied: 1
            }
        )
    );
    assert_eq!(
        env.tuple(key("tuple-A"), vec![int(1), int(2), int(3)])
            .unwrap_err(),
        refusal(
            Component::Value,
            ConstructionCause::WrongArity {
                declared: 2,
                supplied: 3
            }
        )
    );
    assert_eq!(
        env.tuple(key("tuple-A"), vec![int(1), Value::Boolean(false)])
            .unwrap_err(),
        refusal(Component::Position(1), ConstructionCause::TypeMismatch)
    );
    assert_eq!(
        env.tuple(key("record-A"), vec![int(1), int(2)])
            .unwrap_err(),
        refusal(Component::Value, ConstructionCause::UnknownDeclaration)
    );
    assert_eq!(
        env.variant(key("List"), key("Snoc"), vec![]).unwrap_err(),
        refusal(Component::Value, ConstructionCause::UnknownConstructor)
    );

    // A tuple position has no absent or null state.
    let tuple_graph = graph(vec![(
        node(0),
        GraphNode::Tuple {
            declaration: key("tuple-A"),
            positions: vec![GraphSlot::Value(int(1)), GraphSlot::Null],
        },
    )]);
    assert_eq!(
        env.build(&tuple_graph, node(0)).unwrap_err(),
        GraphRefusal {
            node: node(0),
            cause: GraphCause::Construction(refusal(
                Component::Position(1),
                ConstructionCause::NullNotAdmitted
            )),
        }
    );
}

#[trace("TC-188", "FR-143-AC-4")]
#[test]
fn malformed_declarations_refuse_at_admission() {
    let record =
        |label: &str, fields| CompositeDeclaration::new(key(label), CompositeShape::Record(fields));
    let x = || field("x", ValueType::Integer, Presence::Required);
    let cases = [
        (
            vec![record("R", vec![x()]), record("R", vec![x()])],
            InvalidDeclaration {
                declaration: key("R"),
                cause: DeclarationCause::DuplicateDeclaration,
            },
        ),
        (
            vec![record("R", vec![x(), x()])],
            InvalidDeclaration {
                declaration: key("R"),
                cause: DeclarationCause::DuplicateMember(key("x")),
            },
        ),
        (
            vec![record(
                "R",
                vec![field(
                    "x",
                    ValueType::Composite(key("S")),
                    Presence::Required,
                )],
            )],
            InvalidDeclaration {
                declaration: key("R"),
                cause: DeclarationCause::UnknownDeclaration(key("S")),
            },
        ),
        (
            vec![CompositeDeclaration::new(
                key("V"),
                CompositeShape::Variant(vec![]),
            )],
            InvalidDeclaration {
                declaration: key("V"),
                cause: DeclarationCause::EmptyVariant,
            },
        ),
    ];
    for (declarations, expected) in cases {
        assert_eq!(TypeEnvironment::new(declarations).unwrap_err(), expected);
    }
}
