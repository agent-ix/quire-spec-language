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
use qsl_semantics::check::{LockEvidence, PackageDeclarations};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::{CatalogRole, DefinitionLock};
use quire_exact::{admit_text, TextPayload, TextProfile, TextType};
use quire_exact::{
    EffectiveId, FieldValue, Integer, Meter, ObjectId, ObjectReference, Outcome, Presence,
    ScalarLimits, UniverseId, Value, ValueType,
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

fn type_form(value_type: &ValueType, name: &str) -> TypeForm {
    match value_type {
        ValueType::Composite(_) => crate::support::type_form::named_type_form(name),
        _ => crate::support::type_form::type_form(value_type),
    }
}

fn value_package(functions: Vec<FunctionDeclaration>) -> CheckedPackage {
    let money = money_type();
    let types = TypeEnvironment::new([money], []).unwrap();
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

fn name(value: &str) -> Expression {
    Expression::name(value.to_owned())
}

fn integer(value: i64) -> Expression {
    Expression::integer(Integer::from(value))
}

fn money_function(name_: &str, body: Expression) -> FunctionDeclaration {
    let money = ValueType::Composite(key("Money"));
    FunctionDeclaration::new(
        name_,
        vec![
            ("left".to_owned(), type_form(&money, "Money")),
            ("right".to_owned(), type_form(&money, "Money")),
        ],
        crate::support::type_form::type_form(&ValueType::Boolean),
        None,
        body,
    )
}

#[trace("QSpec-TC-235", "QSpec-FR-208-AC-4")]
#[test]
fn tc235_m02_record_values_compare_every_field_at_runtime() {
    let fixture = crate::tc235_k2::k2();
    assert_eq!(fixture["types"].as_array().unwrap().len(), 8);
    let package = value_package(vec![money_function(
        "quoted_is_total",
        Expression::binary(BinaryOperator::Equal, name("left"), name("right")),
    )]);
    let call = |left: Value, right: Value| {
        package
            .call(
                &QualifiedName::unqualified("quoted_is_total").unwrap(),
                vec![left, right],
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
        call(money(100, "EUR"), money(100, "USD")),
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    ));
}

#[trace("QSpec-TC-235", "QSpec-FR-208-AC-10")]
#[test]
fn tc235_m13_record_invariant_runs_for_all_reachable_values() {
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
        money(5, "EUR"),
        money(1, "EUR"),
        money(-2, "EUR"),
        money(-1, "EUR"),
        money(-5, "EUR"),
        money(-7, "EUR"),
    ];
    let results: Vec<_> = values
        .into_iter()
        .map(|value| {
            package
                .call(
                    &QualifiedName::unqualified("non_negative").unwrap(),
                    vec![value],
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
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    assert!(results[2..].iter().all(|result| matches!(
        result,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    )));
}

#[trace("QSpec-TC-235", "QSpec-FR-208-AC-10")]
#[test]
fn tc235_m14_event_invariant_does_not_invent_an_event_value() {
    let package = value_package(vec![FunctionDeclaration::new(
        "occurred",
        Vec::new(),
        crate::support::type_form::type_form(&ValueType::Boolean),
        None,
        Expression::boolean(true),
    )]);
    let evaluation = package
        .call(
            &QualifiedName::unqualified("occurred").unwrap(),
            Vec::new(),
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .unwrap();
    assert!(matches!(
        evaluation.outcome,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
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
    let objects = ObjectEnvironment::new(
        ObjectClosure::new(
            package.graph().scope().types(),
            [(
                reference.clone(),
                vec![
                    ("placed", FieldValue::Present(Value::Boolean(true))),
                    ("cancelled", FieldValue::Present(Value::Boolean(false))),
                ],
            )],
            &[],
        )
        .unwrap(),
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
    let evaluation = package
        .evaluate(
            &checked,
            vec![Value::Reference(reference)],
            &objects,
            &mut Meter::new(UNLIMITED),
        )
        .unwrap();
    assert!(matches!(
        evaluation.outcome,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
}

#[trace("QSpec-TC-235", "QSpec-FR-207-AC-8", "QSpec-FR-047-AC-9")]
#[test]
fn tc235_m22_receiver_and_parameter_paths_are_distinct() {
    let observation = qsl_eval::value::ObservationIdentity {
        authority: "test".into(),
        identity: "q1".into(),
        revision_namespace: "fixture".into(),
        revision: "1".into(),
    };
    let receiver = ObjectReference::new(
        UniverseId::from_digest([2; 32]),
        object_type("OrderLifecycle"),
        ObjectId::new("i1").unwrap(),
    );
    let parameter = ObjectReference::new(
        UniverseId::from_digest([2; 32]),
        object_type("Flags"),
        ObjectId::new("flags").unwrap(),
    );
    let receiver_path = qsl_eval::value::RuntimeValuePath {
        observation: observation.clone(),
        subject: qsl_eval::value::ValuePathSubject::Object(receiver),
        steps: vec![qsl_eval::value::ValuePathStep::Field("placed".into())],
    };
    let parameter_path = qsl_eval::value::RuntimeValuePath {
        observation,
        subject: qsl_eval::value::ValuePathSubject::Object(parameter),
        steps: vec![qsl_eval::value::ValuePathStep::Field("placed".into())],
    };
    assert_ne!(receiver_path, parameter_path);
}
