// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-188 records, tuples and finite recursive values over the real `value`
//! boundary (FR-143).
//!
//! Declaration node keys are opaque producer-assigned fixture keys; the
//! declarations that intentionally share names and shapes get distinct keys.
//! R04 runs through the complete-source parser and R03's library-import row
//! through FR-307 library resolution.

use std::cell::Cell;

use ix_trace_rs::trace;
use qsl_cst::{CompleteCause, CompleteCode, Limits};
use qsl_foundation::SourceIdentity;
use quire_exact::{
    CardinalityBound, ChargePoint, CollectionKind, Incomplete, Integer, LimitKind, Meter,
    ScalarLimits,
};
use quire_spec_language::library::{
    resolve_libraries, ImportDeclaration, LibraryName, LibraryPackage, PackageId,
};
use quire_spec_language::value::{
    CollectionType, Component, CompositeDeclaration, CompositeShape, ConstructionCause,
    ConstructionRefusal, DeclarationCause, EqualityOperand, EqualityOperator, FieldDeclaration,
    FieldExpression, FieldValue, GraphCause, GraphNode, GraphNodeId, GraphRefusal, GraphSlot,
    IllTyped, IllTypedCause, InvalidDeclaration, NodeKey, ObjectEnvironment,
    ObjectEnvironmentCause, ObjectEnvironmentRefusal, ObjectIdentity, ObjectReference,
    ObjectTypeDeclaration, OptionValue, Outcome, Presence, QualifiedName, RecursionEdges,
    TypeEnvironment, UniverseIdentity, Value, ValueGraph, ValueType,
};
use serde_json::json;
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

fn field(name: &str, value_type: ValueType, presence: Presence) -> FieldDeclaration {
    FieldDeclaration::new(name, value_type, presence)
}

fn record(label: &str, fields: Vec<FieldDeclaration>) -> CompositeDeclaration {
    CompositeDeclaration::new(key(label), label, CompositeShape::Record(fields))
}

fn composite(label: &str) -> ValueType {
    ValueType::Composite(key(label))
}

fn sequence_of(element: ValueType, minimum: u64, maximum: u64) -> ValueType {
    ValueType::collection(CollectionType::new(
        CollectionKind::Sequence,
        element,
        CardinalityBound::new(minimum, maximum).unwrap(),
    ))
}

fn refusal(component: Component, cause: ConstructionCause) -> ConstructionRefusal {
    ConstructionRefusal { component, cause }
}

/// `left = right` for two parameters of `value_type`.
fn equal(
    env: &TypeEnvironment,
    value_type: &ValueType,
    left: &Value,
    right: &Value,
) -> Result<Outcome<bool>, IllTyped> {
    env.check_equality(
        EqualityOperator::Equal,
        EqualityOperand::typed(value_type.clone()),
        EqualityOperand::typed(value_type.clone()),
    )
    .map(|checked| checked.evaluate(left, right, &mut Meter::new(UNLIMITED)))
}

/// `record P { a: Integer; b: Integer?; }` and `tuple T(Integer, Integer);`.
fn p_environment() -> TypeEnvironment {
    TypeEnvironment::new(
        [
            record(
                "P",
                vec![
                    field("a", ValueType::Integer, Presence::Required),
                    field("b", ValueType::Integer, Presence::Optional),
                ],
            ),
            CompositeDeclaration::new(
                key("T"),
                "T",
                CompositeShape::Tuple(vec![ValueType::Integer, ValueType::Integer]),
            ),
        ],
        [],
    )
    .unwrap()
}

#[trace("TC-188", "FR-143-AC-1")]
#[trace("TC-188", "FR-143-AC-5")]
#[test]
fn r01_equal_records_compare_structurally_whatever_the_source_field_order() {
    let env = p_environment();
    let p = composite("P");
    let a1 = || env.record(key("P"), vec![("a", FieldValue::Present(int(1)))]);
    assert_eq!(
        equal(&env, &p, &a1().unwrap(), &a1().unwrap()),
        Ok(Outcome::Completed(true))
    );
    let ab = env
        .record(
            key("P"),
            vec![
                ("a", FieldValue::Present(int(1))),
                ("b", FieldValue::Present(int(2))),
            ],
        )
        .unwrap();
    let ba = env
        .record(
            key("P"),
            vec![
                ("b", FieldValue::Present(int(2))),
                ("a", FieldValue::Present(int(1))),
            ],
        )
        .unwrap();
    assert_eq!(equal(&env, &p, &ab, &ba), Ok(Outcome::Completed(true)));
    let t = composite("T");
    let tuple = |second| env.tuple(key("T"), vec![int(1), int(second)]).unwrap();
    assert_eq!(
        equal(&env, &t, &tuple(2), &tuple(2)),
        Ok(Outcome::Completed(true))
    );
    assert_eq!(
        equal(&env, &t, &tuple(2), &tuple(3)),
        Ok(Outcome::Completed(false))
    );
}

#[trace("TC-188", "FR-143-AC-2")]
#[trace("TC-188", "FR-143-AC-5")]
#[trace("TC-188", "FR-143-AC-6")]
#[test]
fn r02_equal_shapes_of_distinct_declarations_do_not_compare() {
    let x = || vec![field("x", ValueType::Integer, Presence::Required)];
    let env = TypeEnvironment::new([record("A", x()), record("B", x())], []).unwrap();
    assert_eq!(
        env.check_equality(
            EqualityOperator::Equal,
            EqualityOperand::typed(composite("A")),
            EqualityOperand::typed(composite("B")),
        ),
        Err(IllTyped {
            cause: IllTypedCause::TypeMismatch
        })
    );
    // A value of one declaration is not a member of the other.
    let a = env
        .record(key("A"), vec![("x", FieldValue::Present(int(1)))])
        .unwrap();
    assert!(composite("A").admits(&a));
    assert!(!composite("B").admits(&a));
}

#[trace("TC-188", "FR-143-AC-3")]
#[trace("TC-188", "FR-143-AC-7")]
#[trace("TC-188", "FR-143-AC-9")]
#[test]
fn r05_recursion_rule_admits_escaping_and_named_recursion() {
    let admitted = TypeEnvironment::new(
        [
            record(
                "List",
                vec![
                    field("head", ValueType::Integer, Presence::Required),
                    field("tail", composite("List"), Presence::Optional),
                ],
            ),
            record(
                "N",
                vec![field(
                    "kids",
                    sequence_of(composite("N"), 0, 2),
                    Presence::Required,
                )],
            ),
            record("A", vec![field("b", composite("B"), Presence::Required)]),
            record("B", vec![field("a", composite("A"), Presence::Optional)]),
            record(
                "O",
                vec![field(
                    "r",
                    ValueType::Reference(key("M::Obj")),
                    Presence::Required,
                )],
            ),
        ],
        [ObjectTypeDeclaration::new(key("M::Obj"), "Obj", vec![])],
    );
    assert!(admitted.is_ok(), "{admitted:?}");

    let bad = TypeEnvironment::new(
        [
            record(
                "P",
                vec![field("x", ValueType::Integer, Presence::Required)],
            ),
            record(
                "Bad",
                vec![field(
                    "r",
                    ValueType::Reference(key("P")),
                    Presence::Required,
                )],
            ),
        ],
        [],
    )
    .unwrap_err();
    assert_eq!(
        bad,
        InvalidDeclaration {
            declaration: "Bad".into(),
            cause: DeclarationCause::Type(IllTypedCause::TypeMismatch),
        }
    );
    assert_eq!(bad.code(), "ill_typed");
}

#[trace("TC-188", "FR-143-AC-3")]
#[trace("TC-188", "FR-143-AC-7")]
#[test]
fn r06_recursion_rule_names_each_refused_cycle() {
    let cycle = |edges, name: &str| InvalidDeclaration {
        declaration: name.into(),
        cause: DeclarationCause::Recursion {
            edges,
            cycle: vec![name.into(), name.into()],
        },
    };
    let cases = [
        (
            record(
                "Loop",
                vec![field("next", composite("Loop"), Presence::Required)],
            ),
            cycle(RecursionEdges::NonEscaping, "Loop"),
        ),
        (
            record(
                "M",
                vec![field(
                    "kids",
                    sequence_of(composite("M"), 1, 2),
                    Presence::Required,
                )],
            ),
            cycle(RecursionEdges::NonEscaping, "M"),
        ),
        (
            CompositeDeclaration::new(
                key("Pair"),
                "Pair",
                CompositeShape::Tuple(vec![
                    ValueType::Integer,
                    ValueType::option(composite("Pair")),
                ]),
            ),
            cycle(RecursionEdges::Unnamed, "Pair"),
        ),
    ];
    for (declaration, expected) in cases {
        let refused = TypeEnvironment::new([declaration], []).unwrap_err();
        assert_eq!(refused, expected);
        assert_eq!(refused.code(), "ill_typed");
    }
}

#[trace("TC-188", "FR-143-AC-4")]
#[trace("TC-188", "FR-143-AC-6")]
#[test]
fn r07_absence_null_and_malformed_constructions_refuse_at_their_origin() {
    let env = p_environment();
    let p = composite("P");
    let absent = env
        .record(key("P"), vec![("a", FieldValue::Present(int(1)))])
        .unwrap();
    let null = env
        .record(
            key("P"),
            vec![("a", FieldValue::Present(int(1))), ("b", FieldValue::Null)],
        )
        .unwrap();
    let slot = |value: &Value| match value {
        Value::Composite(record) => record.slots()[1].clone(),
        other => panic!("a record, not {other:?}"),
    };
    assert!(matches!(slot(&absent), FieldValue::Absent));
    assert!(matches!(slot(&null), FieldValue::Null));
    assert_eq!(
        equal(&env, &p, &absent, &null),
        Ok(Outcome::Completed(false))
    );

    let field_refusal = |name: &str, cause| refusal(Component::Field(name.into()), cause);
    let cases = [
        (
            vec![("b", FieldValue::Present(int(2)))],
            field_refusal("a", ConstructionCause::MissingField),
        ),
        (
            vec![
                ("a", FieldValue::Present(int(1))),
                ("c", FieldValue::Present(int(2))),
            ],
            field_refusal("c", ConstructionCause::UndeclaredField),
        ),
        (
            vec![
                ("a", FieldValue::Present(int(1))),
                ("a", FieldValue::Present(int(2))),
            ],
            field_refusal("a", ConstructionCause::DuplicateField),
        ),
        (
            vec![("a", FieldValue::Null)],
            field_refusal("a", ConstructionCause::NullForRequiredField),
        ),
    ];
    for (fields, expected) in cases {
        let refused = env.record(key("P"), fields).unwrap_err();
        assert_eq!(refused, expected);
        assert_eq!(ConstructionRefusal::CODE, "ill_typed");
    }
    assert_eq!(
        env.tuple(key("T"), vec![int(1)]).unwrap_err(),
        refusal(
            Component::Value,
            ConstructionCause::WrongArity {
                declared: 2,
                supplied: 1
            }
        )
    );

    // `null` is not an option value: a required `Option<Integer>` slot
    // refuses it.
    let options = TypeEnvironment::new(
        [record(
            "Holder",
            vec![field(
                "o",
                ValueType::option(ValueType::Integer),
                Presence::Required,
            )],
        )],
        [],
    )
    .unwrap();
    assert_eq!(
        options
            .record(key("Holder"), vec![("o", FieldValue::Null)])
            .unwrap_err(),
        field_refusal("o", ConstructionCause::NullForRequiredField)
    );
    let none = OptionValue::none(ValueType::Integer);
    assert!(options
        .record(key("Holder"), vec![("o", FieldValue::Present(none))])
        .is_ok());
}

fn node_environment() -> TypeEnvironment {
    TypeEnvironment::new(
        [],
        [ObjectTypeDeclaration::new(
            key("M::Node"),
            "Node",
            vec![field(
                "peer",
                ValueType::Reference(key("M::Node")),
                Presence::Required,
            )],
        )],
    )
    .unwrap()
}

fn node_reference(name: &str) -> ObjectReference {
    ObjectReference::new(
        UniverseIdentity::new(b"snapshot-1").unwrap(),
        key("M::Node"),
        ObjectIdentity::new(name.as_bytes()).unwrap(),
    )
}

#[trace("TC-188", "FR-143-AC-3")]
#[trace("TC-188", "FR-143-AC-9")]
#[test]
fn r09_object_reference_cycles_are_admitted_and_compare_by_identity() {
    let env = node_environment();
    let peer = |name: &str| {
        vec![(
            "peer",
            FieldValue::Present(Value::Reference(node_reference(name))),
        )]
    };
    let objects = ObjectEnvironment::new(
        &env,
        [
            (node_reference("o1"), peer("o2")),
            (node_reference("o2"), peer("o1")),
        ],
    )
    .unwrap();
    let project = |object: &str| match objects.attribute(&env, &node_reference(object), "peer") {
        Some(FieldValue::Present(value)) => value.clone(),
        other => panic!("peer is present, not {other:?}"),
    };
    let reference_type = ValueType::Reference(key("M::Node"));
    assert_eq!(
        equal(&env, &reference_type, &project("o1"), &project("o1")),
        Ok(Outcome::Completed(true))
    );
    assert_eq!(
        equal(&env, &reference_type, &project("o1"), &project("o2")),
        Ok(Outcome::Completed(false))
    );
    assert_eq!(
        ObjectEnvironment::new(&env, [(node_reference("o1"), peer("missing"))]).unwrap_err(),
        ObjectEnvironmentRefusal {
            object: node_reference("o1"),
            cause: ObjectEnvironmentCause::DanglingReference(Box::new(node_reference("missing"))),
        }
    );
    assert_eq!(
        ObjectIdentity::new(b"").map(|_| ()),
        Err(quire_spec_language::value::InvalidObjectIdentity)
    );
}

#[trace("TC-188", "FR-143-AC-8")]
#[test]
fn r10_record_construction_charges_one_result_retain() {
    let env = p_environment();
    let run = |limits| {
        let mut meter = Meter::new(limits);
        let outcome = env
            .evaluate_record(
                key("P"),
                vec![(
                    "a",
                    FieldExpression::Evaluate(Box::new(|_: &mut Meter| Outcome::Completed(int(1)))),
                )],
                &mut meter,
            )
            .unwrap();
        (outcome, meter)
    };
    let (outcome, meter) = run(ScalarLimits {
        result_units: 1,
        ..UNLIMITED
    });
    assert_eq!(
        format!("{outcome:?}"),
        format!(
            "{:?}",
            Outcome::<Value>::Incomplete(Incomplete {
                limit_kind: LimitKind::ResultUnits,
                limit: 1,
                consumed: 0,
                next_charge: Integer::from(2_i64),
                charge_point: ChargePoint::CompositeResultRetain,
            })
        )
    );
    assert!(meter.admitted_charges().is_empty());

    let (outcome, meter) = run(UNLIMITED);
    let Outcome::Completed(value) = outcome else {
        panic!("the record completes");
    };
    assert_eq!(value.occ(), Integer::from(2_i64));
    assert_eq!(
        meter.admitted_charges(),
        [ChargePoint::CompositeResultRetain]
    );
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 1);
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 2);
}

#[trace("TC-188", "FR-143-AC-8")]
#[test]
fn record_fields_run_in_declaration_order_and_the_first_stop_propagates() {
    let env = p_environment();
    let b_ran = Cell::new(false);
    let mut meter = Meter::new(UNLIMITED);
    let outcome = env
        .evaluate_record(
            key("P"),
            vec![
                (
                    "b",
                    FieldExpression::Evaluate(Box::new(|_: &mut Meter| {
                        b_ran.set(true);
                        Outcome::Completed(int(2))
                    })),
                ),
                (
                    "a",
                    FieldExpression::Evaluate(Box::new(|_: &mut Meter| {
                        Outcome::Refused(quire_spec_language::value::Refusal::CheckedInvariant)
                    })),
                ),
            ],
            &mut meter,
        )
        .unwrap();
    assert!(matches!(
        outcome,
        Outcome::Refused(quire_spec_language::value::Refusal::CheckedInvariant)
    ));
    assert!(!b_ran.get());
    assert!(meter.admitted_charges().is_empty());
}

fn list_environment() -> TypeEnvironment {
    TypeEnvironment::new(
        [
            record(
                "List",
                vec![
                    field("head", ValueType::Integer, Presence::Required),
                    field("tail", composite("List"), Presence::Optional),
                ],
            ),
            CompositeDeclaration::new(
                key("Lists"),
                "Lists",
                CompositeShape::Tuple(vec![composite("List"), composite("List")]),
            ),
        ],
        [],
    )
    .unwrap()
}

fn cons(head: i64, tail: GraphSlot) -> GraphNode {
    GraphNode::Record {
        declaration: key("List"),
        fields: vec![
            ("head".into(), GraphSlot::Value(int(head))),
            ("tail".into(), tail),
        ],
    }
}

#[trace("TC-188", "FR-143-AC-3")]
#[test]
fn containment_cycles_refuse_while_shared_finite_values_construct() {
    let env = list_environment();
    let node = GraphNodeId;
    // Two lists sharing one immutable tail node form a DAG.
    let shared = ValueGraph::new([
        (node(0), cons(2, GraphSlot::Absent)),
        (node(1), cons(1, GraphSlot::Node(node(0)))),
        (
            node(2),
            GraphNode::Tuple {
                declaration: key("Lists"),
                positions: vec![GraphSlot::Node(node(1)), GraphSlot::Node(node(0))],
            },
        ),
    ])
    .unwrap();
    assert!(env.build(&shared, node(2)).is_ok());
    let list = composite("List");
    assert_eq!(
        equal(
            &env,
            &list,
            &env.build(&shared, node(1)).unwrap(),
            &env.build(&shared, node(1)).unwrap()
        ),
        Ok(Outcome::Completed(true))
    );

    let cyclic = ValueGraph::new([
        (node(0), cons(1, GraphSlot::Node(node(1)))),
        (node(1), cons(2, GraphSlot::Node(node(0)))),
    ])
    .unwrap();
    assert_eq!(
        env.build(&cyclic, node(0)).unwrap_err(),
        GraphRefusal {
            node: node(0),
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
    // A tuple position has no absent or null state.
    let tuple = ValueGraph::new([
        (node(0), cons(1, GraphSlot::Absent)),
        (
            node(1),
            GraphNode::Tuple {
                declaration: key("Lists"),
                positions: vec![GraphSlot::Node(node(0)), GraphSlot::Null],
            },
        ),
    ])
    .unwrap();
    assert_eq!(
        env.build(&tuple, node(1)).unwrap_err(),
        GraphRefusal {
            node: node(1),
            cause: GraphCause::Construction(refusal(
                Component::Position(1),
                ConstructionCause::TypeMismatch
            )),
        }
    );
}

#[trace("TC-188", "FR-143-AC-4")]
#[test]
fn malformed_declarations_refuse_at_admission() {
    let x = || field("x", ValueType::Integer, Presence::Required);
    let cases = [
        (
            vec![record("R", vec![x()]), record("R", vec![x()])],
            InvalidDeclaration {
                declaration: "R".into(),
                cause: DeclarationCause::DuplicateKey,
            },
        ),
        (
            vec![record("R", vec![x(), x()])],
            InvalidDeclaration {
                declaration: "R".into(),
                cause: DeclarationCause::DuplicateMember("x".into()),
            },
        ),
        (
            vec![record(
                "R",
                vec![field("x", composite("S"), Presence::Required)],
            )],
            InvalidDeclaration {
                declaration: "R".into(),
                cause: DeclarationCause::UnknownDeclaration(key("S")),
            },
        ),
    ];
    for (declarations, expected) in cases {
        let refused = TypeEnvironment::new(declarations, []).unwrap_err();
        assert_eq!(refused.code(), "invalid_semantic_graph");
        assert_eq!(refused, expected);
    }
}

/// Rows that need the FR-146 checker and evaluator boundary.
mod checked {
    use super::*;
    use quire_spec_language::value::{
        BinaryOperator, BoundViolation, CheckCause, CheckMode, CheckRefusal, CheckedExpression,
        CheckedPackage, CheckingLimits, Expression, FieldInitializer, FunctionDeclaration,
        InputRefusal, Obligation, PackageDeclarations, Refusal,
    };

    fn name(spelling: &str) -> Expression {
        Expression::Name(spelling.to_owned())
    }

    fn literal(value: i64) -> Expression {
        Expression::Integer(Integer::from(value))
    }

    fn binary(operator: BinaryOperator, left: Expression, right: Expression) -> Expression {
        Expression::Binary {
            operator,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    fn project(operand: Expression, spelling: &str) -> Expression {
        Expression::Field {
            operand: Box::new(operand),
            field: spelling.to_owned(),
        }
    }

    fn call(target: &str) -> Expression {
        Expression::Call {
            name: target.to_owned(),
            arguments: Vec::new(),
        }
    }

    fn function(
        spelling: &str,
        parameters: &[(&str, ValueType)],
        body: Expression,
    ) -> FunctionDeclaration {
        FunctionDeclaration::new(
            spelling.to_owned(),
            owned(parameters),
            ValueType::Integer,
            None,
            body,
        )
    }

    fn owned(parameters: &[(&str, ValueType)]) -> Vec<(String, ValueType)> {
        parameters
            .iter()
            .map(|(name, value_type)| ((*name).to_owned(), value_type.clone()))
            .collect()
    }

    fn package(
        types: TypeEnvironment,
        aliases: Vec<(String, ValueType)>,
        functions: Vec<FunctionDeclaration>,
    ) -> Result<CheckedPackage, Vec<CheckRefusal>> {
        PackageDeclarations {
            types,
            aliases,
            functions,
            ..PackageDeclarations::default()
        }
        .check(CheckingLimits::default())
        // ADR-013 T-1 (FR-087, QSL-158 S-3a): the S4 link step, over an
        // empty dependency closure -- this fixture declares no import.
        .map(CheckedPackage::link)
    }

    fn check(
        package: &CheckedPackage,
        parameters: &[(&str, ValueType)],
        expression: &Expression,
        mode: CheckMode,
    ) -> Result<CheckedExpression, CheckRefusal> {
        package.graph().check_expression(
            owned(parameters),
            expression,
            None,
            mode,
            CheckingLimits::default(),
        )
    }

    fn evaluate(
        package: &CheckedPackage,
        checked: &CheckedExpression,
        arguments: Vec<Value>,
        limits: ScalarLimits,
    ) -> (Outcome<Value>, Meter) {
        let mut meter = Meter::new(limits);
        let evaluation = package
            .evaluate(
                checked,
                arguments,
                &ObjectEnvironment::default(),
                &mut meter,
            )
            .unwrap();
        (evaluation.outcome, meter)
    }

    fn cause(result: Result<CheckedExpression, CheckRefusal>) -> CheckCause {
        match result {
            Err(refusal) => refusal.cause,
            Ok(_) => panic!("a refusal, not a checked expression"),
        }
    }

    fn mismatch() -> CheckCause {
        CheckCause::IllTyped(IllTypedCause::TypeMismatch)
    }

    fn assert_completed(outcome: &Outcome<Value>, expected: Value) {
        assert_eq!(
            format!("{outcome:?}"),
            format!("{:?}", Outcome::Completed(expected))
        );
    }

    #[trace("TC-188", "FR-143-AC-2")]
    #[trace("TC-188", "FR-143-AC-6")]
    #[test]
    fn r02_an_alias_creates_no_declaration_identity() {
        let x = || vec![field("x", ValueType::Integer, Presence::Required)];
        let types = TypeEnvironment::new([record("A", x()), record("B", x())], []).unwrap();
        let package = package(
            types.clone(),
            vec![("C".to_owned(), composite("A"))],
            Vec::new(),
        )
        .unwrap();
        let parameters = [("a", composite("A")), ("b", composite("B"))];
        let equal = |left, right| binary(BinaryOperator::Equal, left, right);
        assert_eq!(
            cause(check(
                &package,
                &parameters,
                &equal(name("a"), name("b")),
                CheckMode::Kernel
            )),
            mismatch()
        );

        let aliased = Expression::Record {
            name: "C".to_owned(),
            fields: vec![("x".to_owned(), FieldInitializer::Value(literal(1)))],
        };
        let checked = check(
            &package,
            &parameters,
            &equal(aliased, name("a")),
            CheckMode::Kernel,
        )
        .unwrap();
        let value = |declaration| {
            types
                .record(key(declaration), vec![("x", FieldValue::Present(int(1)))])
                .unwrap()
        };
        let (outcome, _) = evaluate(&package, &checked, vec![value("A"), value("B")], UNLIMITED);
        assert_completed(&outcome, Value::Boolean(true));
    }

    #[trace("TC-188", "FR-143-AC-6")]
    #[trace("TC-188", "FR-143-AC-9")]
    #[test]
    fn r03_declaration_keys_of_another_package_refuse_at_checking_and_input() {
        let r = |label| {
            TypeEnvironment::new(
                [record(
                    label,
                    vec![field("x", ValueType::Integer, Presence::Required)],
                )],
                [],
            )
            .unwrap()
        };
        let (p_types, q_types) = (r("P::R"), r("Q::R"));
        let p = package(
            p_types,
            Vec::new(),
            vec![function("id", &[("r", composite("P::R"))], literal(1))],
        )
        .unwrap();
        assert_eq!(
            cause(check(
                &p,
                &[("a", composite("P::R")), ("b", composite("Q::R"))],
                &binary(BinaryOperator::Equal, name("a"), name("b")),
                CheckMode::Kernel,
            )),
            mismatch()
        );

        let foreign = q_types
            .record(key("Q::R"), vec![("x", FieldValue::Present(int(1)))])
            .unwrap();
        let mut meter = Meter::new(UNLIMITED);
        let refused = p
            .call(
                &QualifiedName::unqualified("id").unwrap(),
                vec![foreign],
                &ObjectEnvironment::default(),
                &mut meter,
            )
            .unwrap_err();
        assert_eq!(refused, InputRefusal::WrongValueKind { parameter: 0 });
        assert_eq!(refused.code().as_str(), "invalid_runtime_input");
        assert_eq!(refused.cause(), "wrong-value-kind");
        assert!(meter.admitted_charges().is_empty());
    }

    #[trace("TC-188", "FR-143-AC-8")]
    #[test]
    fn r08_field_a_stops_construction_before_field_b_is_evaluated() {
        let set = ValueType::collection(CollectionType::new(
            CollectionKind::Set,
            ValueType::Integer,
            CardinalityBound::new(0, 1).unwrap(),
        ));
        let types = TypeEnvironment::new(
            [record(
                "Two",
                vec![
                    field("a", set.clone(), Presence::Required),
                    field("b", ValueType::Integer, Presence::Required),
                ],
            )],
            [],
        )
        .unwrap();
        let package = package(
            types,
            Vec::new(),
            vec![
                function("f", &[], literal(1)),
                function("g", &[], call("f")),
            ],
        )
        .unwrap();
        let q_type = sequence_of(ValueType::Integer, 0, 2);
        let construction = Expression::Record {
            name: "Two".to_owned(),
            fields: vec![
                ("b".to_owned(), FieldInitializer::Value(call("g"))),
                (
                    "a".to_owned(),
                    FieldInitializer::Value(Expression::Convert {
                        target: set.clone(),
                        operand: Box::new(name("q")),
                    }),
                ),
            ],
        };
        let checked = check(
            &package,
            &[("q", q_type.clone())],
            &construction,
            CheckMode::Linked,
        )
        .unwrap();
        let ValueType::Collection(q_collection) = &q_type else {
            panic!("a collection type");
        };
        let q = quire_spec_language::value::form_collection(
            q_collection,
            vec![int(1), int(2)],
            &mut Meter::new(UNLIMITED),
        )
        .unwrap()
        .completed()
        .unwrap();
        let (outcome, meter) = evaluate(
            &package,
            &checked,
            vec![q],
            ScalarLimits {
                work_units: 6,
                ..UNLIMITED
            },
        );
        let ValueType::Collection(set_type) = &set else {
            panic!("a collection type");
        };
        assert_eq!(
            format!("{outcome:?}"),
            format!(
                "{:?}",
                Outcome::<Value>::Refused(Refusal::CardinalityOutOfBound {
                    violation: BoundViolation::AboveMaximum,
                    kind: CollectionKind::Set,
                    bound: set_type.bound(),
                    count: 2,
                })
            )
        );
        assert_eq!(
            meter.admitted_charges(),
            [
                ChargePoint::CollectionVisit,
                ChargePoint::CollectionVisit,
                ChargePoint::CollectionMemberWalk,
                ChargePoint::CollectionMemberTest,
                ChargePoint::CollectionBound,
            ]
        );
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 6);
        assert_eq!(meter.consumed(LimitKind::ResultUnits), 0);
    }

    #[trace("TC-188", "FR-143-AC-9")]
    #[test]
    fn r09_no_source_form_converts_into_a_reference() {
        let package = package(node_environment(), Vec::new(), Vec::new()).unwrap();
        let conversion = Expression::Convert {
            target: ValueType::Reference(key("M::Node")),
            operand: Box::new(literal(1)),
        };
        assert_eq!(
            cause(check(&package, &[], &conversion, CheckMode::Kernel)),
            mismatch()
        );
    }

    #[trace("TC-188", "FR-143-AC-10")]
    #[test]
    fn r11_optional_field_projections_need_present_or_value() {
        let x = [("x", composite("P"))];
        let b = || project(name("x"), "b");
        let body = |body| package(p_environment(), Vec::new(), vec![function("f", &x, body)]);
        body(Expression::If {
            condition: Box::new(Expression::Present(Box::new(b()))),
            then: Box::new(Expression::Value(Box::new(b()))),
            otherwise: Box::new(literal(0)),
        })
        .unwrap();
        let only = |result: Result<CheckedPackage, Vec<CheckRefusal>>| match result {
            Err(refusals) => match refusals.as_slice() {
                [refusal] => refusal.cause.clone(),
                other => panic!("one refusal, not {other:?}"),
            },
            Ok(_) => panic!("a refusal, not an admitted package"),
        };
        let unguarded = only(body(Expression::Value(Box::new(b()))));
        assert_eq!(unguarded, CheckCause::Unproved(Obligation::Presence));
        assert_eq!(unguarded.code().as_str(), "undefined_expression");
        assert_eq!(unguarded.cause(), Some("unproved-presence"));
        assert_eq!(
            only(body(binary(BinaryOperator::Add, b(), literal(1)))),
            mismatch()
        );
        body(binary(
            BinaryOperator::Add,
            project(name("x"), "a"),
            literal(1),
        ))
        .unwrap();

        let package = package(p_environment(), Vec::new(), Vec::new()).unwrap();
        let checked = check(
            &package,
            &x,
            &Expression::Present(Box::new(b())),
            CheckMode::Linked,
        )
        .unwrap();
        let env = p_environment();
        for (slot, expected) in [
            (None, false),
            (Some(FieldValue::Null), false),
            (Some(FieldValue::Present(int(2))), true),
        ] {
            let mut fields = vec![("a", FieldValue::Present(int(1)))];
            fields.extend(slot.map(|slot| ("b", slot)));
            let value = env.record(key("P"), fields).unwrap();
            let (outcome, _) = evaluate(&package, &checked, vec![value], UNLIMITED);
            assert_completed(&outcome, Value::Boolean(expected));
        }
    }
}

#[trace("TC-188", "FR-143-AC-10")]
#[test]
fn r04_a_sum_declaration_is_invalid_syntax_at_variant() {
    let text = "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\nvariant V { A, B }";
    let parsed = qsl_cst::parse(
        SourceIdentity {
            identity: "test:tc-188".into(),
            revision: "1".into(),
        },
        "tc-188.native",
        text.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    assert!(!parsed.is_admissible());
    let diagnostic = &parsed.diagnostics()[0];
    assert_eq!(
        (diagnostic.code, diagnostic.cause),
        (CompleteCode::InvalidSyntax, CompleteCause::UnexpectedToken)
    );
    assert_eq!(diagnostic.span.start.byte, text.find("variant").unwrap());
}

mod library_import {
    use super::*;

    fn hex(label: &str) -> String {
        Sha256::digest(label.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    /// The JCS `quire.checked-package-id/v2` preimage of `label`, whose one
    /// projection node declares `R` when `exports` is set.
    fn preimage(label: &str, exports: bool) -> Box<[u8]> {
        let reference = json!({"digest": hex(label), "domain": "quire.checked-semantic-node/v1"});
        let mut node = json!({
            "body": {"members": [], "term": "aggregate"},
            "dependencies": [],
            "node_id": reference,
            "node_tag": "composite_type",
            "schema_version": "quire.checked-semantic-graph/v2",
            "semantic_form": "record",
            "semantic_type": reference,
        });
        if exports {
            node["nominal_identity_preimage"] = json!({"qualified_declaration": ["R"]});
            node["declaration"] = json!({"qualified_name": ["R"]});
        }
        let value = json!({
            "definition_selections": [],
            "dependency_selections": [],
            "edition": {"role": "edition"},
            "identity_projection": [node],
            "model_selections": [],
            "profile_selections": [],
            "required_features": ["quire.value.complete/v1"],
            "version": "quire.checked-package-id/v2",
        });
        serde_json::to_vec(&value).unwrap().into_boxed_slice()
    }

    fn name(library: &str) -> LibraryName {
        LibraryName::new(vec![library.to_owned()]).unwrap()
    }

    fn package(library: &str, imports: &[&str], exports: bool) -> LibraryPackage {
        let bytes = preimage(library, exports);
        LibraryPackage {
            library: name(library),
            version: "1".to_owned(),
            package_id: PackageId::of_preimage(&bytes),
            identity_preimage: bytes,
            imports: imports
                .iter()
                .map(|imported| ImportDeclaration {
                    library: name(imported),
                    version: "1".to_owned(),
                    package_id: PackageId::of_preimage(&preimage(imported, *imported == "L")),
                    qualifier: Some(imported.to_lowercase()),
                })
                .collect(),
            exports: if exports {
                vec!["R".to_owned()]
            } else {
                Vec::new()
            },
        }
    }

    // FR-143-AC-6's own claim -- one library export reached by two import
    // paths (A and B, both over L) evaluates as one declaration at the
    // composite-value level -- is not backed here. FR-087 (#213 S-3a)
    // relocated `resolve_name`/`ExportIdentity` out of `library` (name
    // resolution is E3's own, over an `ImportView`, per ADR-013 R-06), and
    // no E3 resolution exists yet to derive each path's own export node id
    // for a real, independent comparison: the previous version of this test
    // built both "paths'" `NodeKey`s from the same `declaration_key`
    // variable, so its composite-value equality check compared a key with
    // itself and could not have failed for any resolution defect. Left
    // untraced (FR-143-AC-6 unbacked by this vector) until E3 exists to
    // derive two real, independently-resolved node ids to compare.
    #[trace("TC-227", "FR-307-AC-2")]
    #[test]
    fn r03_one_library_export_reached_by_two_paths_is_one_declaration() {
        let root = package("P", &["A", "B"], false);
        let supplied = [
            package("A", &["L"], false),
            package("B", &["L"], false),
            package("L", &[], true),
        ];
        // `resolve_libraries` unifies the diamond into a single selection
        // for `L` (TC-227 l04's own claim, re-demonstrated here over the
        // real `value::library`... boundary this test module exercises).
        let lock = resolve_libraries(&root, &supplied).unwrap();
        let l_selections: Vec<_> = lock
            .selections()
            .into_iter()
            .filter(|(library, _)| *library == name("L"))
            .collect();
        assert_eq!(
            l_selections.len(),
            1,
            "L is selected once despite two import paths"
        );
    }
}
