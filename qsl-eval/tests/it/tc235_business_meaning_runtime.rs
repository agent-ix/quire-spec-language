// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-235 M02/M13/M14/M15/M22 runtime vectors.
//!
//! The semantic binder owns K2 admission and charge assertions. These rows
//! exercise the evaluator boundary with the same record, event and lifecycle
//! meanings, keeping the runtime oracle independent of the binder's JSON walk.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_forms::{BinaryOperator, ClauseKind, Expression, FunctionDeclaration, TypeForm};
use qsl_foundation::selection::ProfileSelection;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckCause, LockEvidence, PackageDeclarations, WrongSnapshotCause};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::{CatalogRole, DefinitionLock};
use quire_exact::{admit_text, TextPayload, TextProfile, TextType};
use quire_exact::{
    CardinalityBound, CollectionKind, CollectionType, EffectiveId, FieldValue, Integer, Meter,
    ObjectId, ObjectReference, OptionValue, Outcome, Presence, ScalarLimits, UniverseId, Value,
    ValueType,
};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::declaration::{
    CompositeDeclaration, CompositeShape, FieldDeclaration, ObjectTypeDeclaration, TypeEnvironment,
};
use quire_semantic_value::object_closure::ObjectClosure;
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

fn key(label: &str) -> quire_exact::NodeKey {
    quire_exact::NodeKey::from_digest(Sha256::digest(label.as_bytes()).into())
}

fn object_type(label: &str) -> EffectiveId {
    EffectiveId::from_digest(Sha256::digest(label.as_bytes()).into())
}

fn k2_order_with(fields: serde_json::Value, clauses: serde_json::Value) -> serde_json::Value {
    let mut document = crate::tc235_k2::k2();
    let order = document["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|ty| ty["identity"] == crate::tc235_k2::id("Order"))
        .expect("K2 Order declaration");
    order["fields"] = fields;
    order["clauses"] = clauses;
    document
}

fn field(name: &str, value_type: ValueType) -> FieldDeclaration {
    FieldDeclaration::new(name, value_type, Presence::Required)
}

fn money_type() -> CompositeDeclaration {
    CompositeDeclaration::new(
        key("Money"),
        "Money",
        CompositeShape::Record(vec![
            field("amount_minor", ValueType::Integer),
            field(
                "currency",
                ValueType::Text(TextType::new(0, 16, TextProfile::BinaryUtf8).unwrap()),
            ),
        ]),
    )
}

fn money(amount: i64, currency: &str) -> Value {
    TypeEnvironment::new([money_type()], [])
        .unwrap()
        .record(
            key("Money"),
            vec![
                (
                    "amount_minor",
                    FieldValue::Present(Value::Integer(Integer::from(amount))),
                ),
                (
                    "currency",
                    FieldValue::Present(Value::Text(
                        admit_text(
                            &TextPayload::from_utf8(currency.as_bytes()).unwrap(),
                            &TextType::new(0, 16, TextProfile::BinaryUtf8).unwrap(),
                            &mut Meter::new(UNLIMITED),
                        )
                        .completed()
                        .unwrap(),
                    )),
                ),
            ],
        )
        .unwrap()
}

fn order_type(fields: Vec<FieldDeclaration>) -> CompositeDeclaration {
    CompositeDeclaration::new(key("Order"), "Order", CompositeShape::Record(fields))
}

fn order_value(fields: Vec<(&str, FieldValue)>, types: &[CompositeDeclaration]) -> Value {
    record_value(key("Order"), fields, types)
}

fn record_value(
    record_key: quire_exact::NodeKey,
    fields: Vec<(&str, FieldValue)>,
    types: &[CompositeDeclaration],
) -> Value {
    TypeEnvironment::new(types.iter().cloned(), [])
        .unwrap()
        .record(record_key, fields)
        .unwrap()
}

fn sequence(element_type: &CollectionType, values: Vec<Value>) -> Value {
    quire_exact::form_collection(element_type, values, &mut Meter::new(UNLIMITED))
        .expect("reachable sequence values admit")
        .completed()
        .unwrap()
}

fn type_form(value_type: &ValueType, name: &str) -> TypeForm {
    match value_type {
        ValueType::Composite(_) => crate::support::type_form::named_type_form(name),
        _ => crate::support::type_form::type_form(value_type),
    }
}

fn package_with_types(
    types: TypeEnvironment,
    functions: Vec<FunctionDeclaration>,
) -> CheckedPackage {
    let function_selections = functions
        .iter()
        .enumerate()
        .map(|(index, _)| {
            (
                index,
                ProfileSelection {
                    alias: "text".to_owned(),
                    identity: DefinitionLock::pinned()
                        .entry(CatalogRole::TextProfile)
                        .identity
                        .to_owned(),
                    span: qsl_foundation::Span { start: 0, end: 0 },
                    identity_span: qsl_foundation::Span { start: 0, end: 0 },
                },
            )
        })
        .collect();
    CheckedPackage::link(
        PackageDeclarations {
            types,
            functions,
            function_selections,
            lock_evidence: LockEvidence::default().with_text_profile(
                DefinitionLock::pinned()
                    .entry(CatalogRole::TextProfile)
                    .reference(),
            ),
            ..PackageDeclarations::new(
                qsl_semantics::check::fixture_source(),
                qsl_foundation::IdentityLimits::default(),
            )
        }
        .check(CheckingLimits::default())
        .unwrap(),
    )
}

fn value_package(functions: Vec<FunctionDeclaration>) -> CheckedPackage {
    package_with_types(TypeEnvironment::new([money_type()], []).unwrap(), functions)
}

fn name(value: &str) -> Expression {
    Expression::name(value.to_owned())
}

fn integer(value: i64) -> Expression {
    Expression::integer(Integer::from(value))
}

fn order_function(name_: &str, body: Expression) -> FunctionDeclaration {
    let order = ValueType::Composite(key("Order"));
    FunctionDeclaration::new(
        name_,
        vec![("self".to_owned(), type_form(&order, "Order"))],
        crate::support::type_form::type_form(&ValueType::Boolean),
        None,
        body,
    )
}

#[trace("QSpec-TC-235", "QSpec-FR-208-AC-4")]
#[test]
fn tc235_m02_record_values_compare_every_field_at_runtime() {
    let fixture = k2_order_with(
        serde_json::json!([
            crate::tc235_k2::field("Order", "total", &crate::tc235_k2::id("Money")),
            crate::tc235_k2::field("Order", "quoted", &crate::tc235_k2::id("Money")),
        ]),
        serde_json::json!([crate::tc235_k2::clause(
            "Order",
            "QuotedIsTotal",
            "self.quoted = self.total"
        )]),
    );
    assert!(fixture["types"][0]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field["name"] == "quoted"));
    assert_eq!(
        fixture["types"][0]["clauses"][0]["clauseId"],
        "QuotedIsTotal"
    );
    let order = order_type(vec![
        field("total", ValueType::Composite(key("Money"))),
        field("quoted", ValueType::Composite(key("Money"))),
    ]);
    let package = package_with_types(
        TypeEnvironment::new([money_type(), order], []).unwrap(),
        vec![order_function(
            "quoted_is_total",
            Expression::binary(
                BinaryOperator::Equal,
                Expression::field(name("self"), "quoted"),
                Expression::field(name("self"), "total"),
            ),
        )],
    );
    let call = |quoted: Value, total: Value| {
        let values = [
            ("total", FieldValue::Present(total)),
            ("quoted", FieldValue::Present(quoted)),
        ];
        let value = order_value(
            values.to_vec(),
            &[
                money_type(),
                order_type(vec![
                    field("total", ValueType::Composite(key("Money"))),
                    field("quoted", ValueType::Composite(key("Money"))),
                ]),
            ],
        );
        package
            .call(
                &QualifiedName::unqualified("quoted_is_total").unwrap(),
                vec![value],
                &ObjectEnvironment::default(),
                &mut Meter::new(UNLIMITED),
            )
            .unwrap()
            .outcome
    };
    assert!(matches!(
        call(money(100, "EUR"), money(100, "EUR")),
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    assert!(matches!(
        call(money(100, "USD"), money(100, "EUR")),
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    ));
}

#[trace("QSpec-TC-235", "QSpec-FR-208-AC-10")]
#[test]
fn tc235_m13_record_invariant_runs_for_all_reachable_values() {
    let mut history_field = crate::tc235_k2::field(
        "Order",
        "history",
        &crate::tc235_k2::id("Money"),
    );
    history_field["presence"] = serde_json::json!("optional");
    history_field["multiplicity"] = serde_json::json!({
        "lower": 0,
        "upper": 3,
        "ordered": true,
        "unique": false
    });
    let mut discount_field = crate::tc235_k2::field(
        "Order",
        "discount",
        &crate::tc235_k2::id("Money"),
    );
    discount_field["presence"] = serde_json::json!("optional");
    let mut fixture = k2_order_with(
        serde_json::json!([
            crate::tc235_k2::field("Order", "total", &crate::tc235_k2::id("Money")),
            crate::tc235_k2::field("Order", "quoted", &crate::tc235_k2::id("Money")),
            history_field,
            discount_field,
        ]),
        serde_json::json!([]),
    );
    let money_decl = fixture["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|ty| ty["identity"] == crate::tc235_k2::id("Money"))
        .expect("K2 Money declaration");
    money_decl["clauses"] = serde_json::json!([crate::tc235_k2::clause(
        "Money",
        "NonNegative",
        "self.amount_minor >= 0"
    )]);
    assert_eq!(fixture["types"][0]["fields"].as_array().unwrap().len(), 4);
    assert_eq!(fixture["types"][0]["fields"][2]["multiplicity"]["upper"], 3);
    assert_eq!(fixture["types"][0]["fields"][3]["presence"], "optional");
    assert_eq!(fixture["types"][2]["clauses"][0]["clauseId"], "NonNegative");
    let history_type = CollectionType::new(
        CollectionKind::Sequence,
        ValueType::Composite(key("Money")),
        Some(CardinalityBound::new(0, 3).unwrap()),
    );
    let observed_order = order_value(
        vec![
            ("total", FieldValue::Present(money(5, "EUR"))),
            ("quoted", FieldValue::Present(money(-5, "EUR"))),
            (
                "history",
                FieldValue::Present(sequence(
                    &history_type,
                    vec![money(1, "EUR"), money(-2, "EUR")],
                )),
            ),
            (
                "discount",
                FieldValue::Present(
                    OptionValue::present(ValueType::Composite(key("Money")), money(-1, "EUR"))
                        .unwrap(),
                ),
            ),
        ],
        &[
            money_type(),
            order_type(vec![
                field("total", ValueType::Composite(key("Money"))),
                field("quoted", ValueType::Composite(key("Money"))),
                field("history", ValueType::collection(history_type.clone())),
                field(
                    "discount",
                    ValueType::option(ValueType::Composite(key("Money"))),
                ),
            ]),
        ],
    );
    assert!(matches!(observed_order, Value::Composite(_)));
    let money_value_type = ValueType::Composite(key("Money"));
    let function = FunctionDeclaration::new(
        "non_negative",
        vec![("self".to_owned(), type_form(&money_value_type, "Money"))],
        crate::support::type_form::type_form(&ValueType::Boolean),
        None,
        Expression::binary(
            BinaryOperator::GreaterOrEqual,
            Expression::field(name("self"), "amount_minor".to_owned()),
            integer(0),
        ),
    );
    let package = value_package(vec![function]);
    let values = [
        ("(s1, object o1).field total", money(5, "EUR")),
        ("(s1, object o1).field quoted", money(-5, "EUR")),
        ("(s1, object o1).field history.index 0", money(1, "EUR")),
        ("(s1, object o1).field history.index 1", money(-2, "EUR")),
        (
            "(s1, object o1).field discount.option_value",
            money(-1, "EUR"),
        ),
        (
            "(pre observation of i1, invocation i1 parameter amount)",
            money(-7, "EUR"),
        ),
    ];
    let results: Vec<_> = values
        .iter()
        .map(|(_, value)| {
            package
                .call(
                    &QualifiedName::unqualified("non_negative").unwrap(),
                    vec![value.clone()],
                    &ObjectEnvironment::default(),
                    &mut Meter::new(UNLIMITED),
                )
                .unwrap()
                .outcome
        })
        .collect();
    assert!(matches!(
        results[0],
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    assert!(matches!(
        results[1],
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    ));
    assert!(matches!(
        results[2],
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    assert!(results[3..].iter().all(|result| matches!(
        result,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    )));
    let false_paths: Vec<_> = values
        .iter()
        .zip(&results)
        .filter_map(|((path, _), result)| {
            matches!(
                result,
                qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(
                    Value::Boolean(false)
                ))
            )
            .then_some(*path)
        })
        .collect();
    let mut false_paths = false_paths;
    false_paths.sort_by_key(|path| match *path {
        "(s1, object o1).field discount.option_value" => 0,
        "(s1, object o1).field history.index 1" => 1,
        "(s1, object o1).field quoted" => 2,
        _ => 3,
    });
    assert_eq!(
        false_paths,
        vec![
            "(s1, object o1).field discount.option_value",
            "(s1, object o1).field history.index 1",
            "(s1, object o1).field quoted",
            "(pre observation of i1, invocation i1 parameter amount)",
        ]
    );
}

#[trace("QSpec-TC-235", "QSpec-FR-208-AC-10")]
#[test]
fn tc235_m14_event_invariant_does_not_invent_an_event_value() {
    let event = object_type("OrderPlaced");
    let types = TypeEnvironment::new(
        [],
        [ObjectTypeDeclaration::new(
            event,
            "OrderPlaced",
            vec![field("occurred_at", ValueType::Boolean)],
        )],
    )
    .unwrap();
    let package = CheckedPackage::link(
        PackageDeclarations {
            types,
            ..PackageDeclarations::new(
                qsl_semantics::check::fixture_source(),
                qsl_foundation::IdentityLimits::default(),
            )
        }
        .check(CheckingLimits::default())
        .unwrap(),
    );
    let reference = ObjectReference::new(
        UniverseId::from_digest([4; 32]),
        event,
        ObjectId::new("event-absent").unwrap(),
    );
    let checked = package
        .graph()
        .check_clause_expression(
            vec![("self".to_owned(), ValueType::Reference(event))],
            &Expression::binary(
                BinaryOperator::Equal,
                Expression::field(Expression::deref(name("self")), "occurred_at"),
                Expression::field(Expression::deref(name("self")), "occurred_at"),
            ),
            None,
            ClauseKind::Invariant,
            quire_semantic_value::checking::CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .unwrap();
    let evaluation = package
        .evaluate(
            &checked,
            vec![Value::Reference(reference)],
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .expect_err("an event invariant has no admitted event value");
    assert!(matches!(evaluation, qsl_eval::value::CallFailure::Input(_)));
}

#[trace("QSpec-TC-235", "QSpec-FR-208-AC-10", "QSpec-FR-208-AC-11")]
#[test]
fn tc235_m15_lifecycle_invariant_keeps_receiver_context() {
    let lifecycle = object_type("OrderLifecycle");
    let types = TypeEnvironment::new(
        [],
        [ObjectTypeDeclaration::new(
            lifecycle,
            "OrderLifecycle",
            vec![
                field("placed", ValueType::Boolean),
                field("cancelled", ValueType::Boolean),
            ],
        )],
    )
    .unwrap();
    let package = CheckedPackage::link(
        PackageDeclarations {
            types,
            functions: Vec::new(),
            ..PackageDeclarations::new(
                qsl_semantics::check::fixture_source(),
                qsl_foundation::IdentityLimits::default(),
            )
        }
        .check(CheckingLimits::default())
        .unwrap(),
    );
    let reference = ObjectReference::new(
        UniverseId::from_digest([1; 32]),
        lifecycle,
        ObjectId::new("i1").unwrap(),
    );
    let checked = package
        .graph()
        .check_clause_expression(
            vec![("self".to_owned(), ValueType::Reference(lifecycle))],
            &Expression::logical_not(Expression::binary(
                BinaryOperator::And,
                Expression::field(Expression::deref(name("self")), "placed".to_owned()),
                Expression::field(Expression::deref(name("self")), "cancelled".to_owned()),
            )),
            None,
            ClauseKind::Invariant,
            quire_semantic_value::checking::CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .unwrap();
    let evaluate = |placed: bool, cancelled: bool, reference: ObjectReference| {
        let objects = ObjectEnvironment::new(
            ObjectClosure::new(
                package.graph().scope().types(),
                [(
                    reference.clone(),
                    vec![
                        ("placed", FieldValue::Present(Value::Boolean(placed))),
                        ("cancelled", FieldValue::Present(Value::Boolean(cancelled))),
                    ],
                )],
                &[],
            )
            .unwrap(),
        );
        let evaluation = package
            .evaluate(
                &checked,
                vec![Value::Reference(reference)],
                &objects,
                &mut Meter::new(UNLIMITED),
            )
            .unwrap();
        evaluation
    };
    assert!(matches!(
        evaluate(false, false, reference.clone()).outcome,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    assert!(matches!(
        evaluate(true, false, reference.clone()).outcome,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    assert!(matches!(
        evaluate(true, true, reference.clone()).outcome,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    ));
    assert!(package
        .evaluate(
            &checked,
            vec![Value::Reference(ObjectReference::new(
                UniverseId::from_digest([4; 32]),
                lifecycle,
                ObjectId::new("wrong-context").unwrap(),
            ))],
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .is_err());
    let pre_read = package.graph().check_clause_expression(
        vec![("self".to_owned(), ValueType::Reference(lifecycle))],
        &Expression::pre(Expression::field(Expression::deref(name("self")), "placed")),
        None,
        ClauseKind::Precondition,
        quire_semantic_value::checking::CheckMode::Kernel,
        CheckingLimits::default(),
    );
    assert!(matches!(
        pre_read,
        Err(refusal)
            if matches!(
                refusal.cause,
                CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
            )
    ));
}

#[trace("QSpec-TC-235", "QSpec-FR-207-AC-8", "QSpec-FR-047-AC-9")]
#[test]
fn tc235_m22_receiver_and_parameter_paths_are_distinct() {
    let lifecycle = object_type("OrderLifecycle");
    let flags = CompositeDeclaration::new(
        key("Flags"),
        "Flags",
        CompositeShape::Record(vec![field("placed", ValueType::Boolean)]),
    );
    let types = TypeEnvironment::new(
        [flags],
        [ObjectTypeDeclaration::new(
            lifecycle,
            "OrderLifecycle",
            vec![
                field("placed", ValueType::Boolean),
                field("cancelled", ValueType::Boolean),
            ],
        )],
    )
    .unwrap();
    let package = CheckedPackage::link(
        PackageDeclarations {
            types,
            ..PackageDeclarations::new(
                qsl_semantics::check::fixture_source(),
                qsl_foundation::IdentityLimits::default(),
            )
        }
        .check(CheckingLimits::default())
        .unwrap(),
    );
    let receiver = ObjectReference::new(
        UniverseId::from_digest([2; 32]),
        lifecycle,
        ObjectId::new("i1").unwrap(),
    );
    let objects = ObjectEnvironment::new(
        ObjectClosure::new(
            package.graph().scope().types(),
            [(
                receiver.clone(),
                vec![
                    ("placed", FieldValue::Present(Value::Boolean(true))),
                    ("cancelled", FieldValue::Present(Value::Boolean(true))),
                ],
            )],
            &[],
        )
        .unwrap(),
    );
    let checked = package
        .graph()
        .check_clause_expression(
            vec![
                ("self".to_owned(), ValueType::Reference(lifecycle)),
                ("flags".to_owned(), ValueType::Composite(key("Flags"))),
            ],
            &Expression::logical_not(Expression::binary(
                BinaryOperator::And,
                Expression::binary(
                    BinaryOperator::Equal,
                    Expression::field(Expression::deref(name("self")), "placed"),
                    Expression::field(name("flags"), "placed"),
                ),
                Expression::field(Expression::deref(name("self")), "cancelled"),
            )),
            None,
            ClauseKind::Invariant,
            quire_semantic_value::checking::CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .unwrap();
    let flags_true = record_value(
        key("Flags"),
        vec![("placed", FieldValue::Present(Value::Boolean(true)))],
        &[CompositeDeclaration::new(
            key("Flags"),
            "Flags",
            CompositeShape::Record(vec![field("placed", ValueType::Boolean)]),
        )],
    );
    let flags_false = record_value(
        key("Flags"),
        vec![("placed", FieldValue::Present(Value::Boolean(false)))],
        &[CompositeDeclaration::new(
            key("Flags"),
            "Flags",
            CompositeShape::Record(vec![field("placed", ValueType::Boolean)]),
        )],
    );
    let evaluate = |flags: Value| {
        package
            .evaluate(
                &checked,
                vec![Value::Reference(receiver.clone()), flags],
                &objects,
                &mut Meter::new(UNLIMITED),
            )
            .unwrap()
            .outcome
    };
    assert!(matches!(
        evaluate(flags_true),
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    ));
    assert!(matches!(
        evaluate(flags_false),
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    let receiver_path = serde_json::json!({
        "kind": "runtime_value",
        "root": {"observation": "q1", "subject": {"tag": "invocation", "invocation": "i1", "binding": "receiver"}},
        "steps": [{"tag": "field", "name": "placed"}],
    });
    let parameter_path = serde_json::json!({
        "kind": "runtime_value",
        "root": {"observation": "q1", "subject": {"tag": "invocation", "invocation": "i1", "binding": "parameter", "parameter": "flags"}},
        "steps": [{"tag": "field", "name": "placed"}],
    });
    assert_ne!(receiver_path, parameter_path);
    assert_eq!(parameter_path["root"]["subject"]["binding"], "parameter");
    assert_eq!(receiver_path["root"]["subject"]["binding"], "receiver");
    assert!(
        parameter_path["root"]["subject"]["binding"]
            .as_str()
            .unwrap()
            < receiver_path["root"]["subject"]["binding"]
                .as_str()
                .unwrap()
    );
}
