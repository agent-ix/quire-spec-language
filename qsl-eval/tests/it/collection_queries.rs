// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-190 collection query and conversion algebra over the real `value`
//! checker and evaluator boundary (FR-145).

use ix_trace_rs::trace;
use qsl_eval::value::CheckedPackageEvaluation;
use qsl_forms::{Accumulation, BinaryOperator, BinderQuery, Expression, FunctionDeclaration};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{
    CheckCause, CheckRefusal, CollectionLoss, CollectionProperty, Obligation, PackageDeclarations,
    ProvedInterval,
};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::FamilyOutcome;
use quire_exact::EffectiveId;
use quire_exact::NodeKey;
use quire_exact::{
    BoundViolation, CardinalityBound, ChargePoint, CollectionKind, Incomplete, Integer,
    IntegerInterval, LimitKind, Meter, ObjectId, ObjectReference, Outcome, Refusal, ScalarLimits,
    Undefined, UnitId, UniverseId,
};
use quire_exact::{
    CarriedQuantityType, DecimalType, Quantity, QuantityCarrier, Rational, RationalDomain,
    RoundingMode,
};
use quire_exact::{CollectionType, FieldValue, Value, ValueType};
use quire_exact::{IllTypedCause, Presence};
use quire_semantic_value::checking::{CheckMode, CheckingLimits};
use quire_semantic_value::declaration::{
    CompositeDeclaration, CompositeShape, FieldDeclaration, ObjectTypeDeclaration, TypeEnvironment,
};
use quire_semantic_value::location::ExpressionLocation as Location;
use quire_semantic_value::object_closure::ObjectClosure;

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

fn work_limit(work_units: u64) -> ScalarLimits {
    ScalarLimits {
        work_units,
        ..UNLIMITED
    }
}

fn key(label: &str) -> NodeKey {
    crate::support::ids::node_key(label.as_bytes())
}

/// A model object type's effective-declaration identity (ADR-013 O-05): the
/// identity `Reference<T>` and `ObjectTypeDeclaration` carry.
fn object_type(label: &str) -> EffectiveId {
    crate::support::ids::effective_id(label.as_bytes())
}

fn int(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

fn ints(values: &[i64]) -> Vec<Value> {
    values.iter().copied().map(int).collect()
}

fn int_type(lower: i64, upper: i64) -> ValueType {
    ValueType::Int(IntegerInterval::new(Integer::from(lower), Integer::from(upper)).unwrap())
}

fn collection_type(
    kind: CollectionKind,
    element: ValueType,
    minimum: u64,
    maximum: u64,
) -> CollectionType {
    CollectionType::new(
        kind,
        element,
        Some(CardinalityBound::new(minimum, maximum).unwrap()),
    )
}

fn of(kind: CollectionKind, element: ValueType, minimum: u64, maximum: u64) -> ValueType {
    ValueType::collection(collection_type(kind, element, minimum, maximum))
}

fn integers(kind: CollectionKind, minimum: u64, maximum: u64) -> ValueType {
    of(kind, ValueType::Integer, minimum, maximum)
}

fn name(spelling: &str) -> Expression {
    Expression::name(spelling.to_owned())
}

fn literal(value: i64) -> Expression {
    Expression::integer(Integer::from(value))
}

fn binary(operator: BinaryOperator, left: Expression, right: Expression) -> Expression {
    Expression::binary(operator, left, right)
}

fn query(query: BinderQuery, source: &str, body: Expression) -> Expression {
    Expression::query(query, "x".to_owned(), name(source), body)
}

fn accumulate(
    form: Accumulation,
    accumulator_type: &str,
    source: &str,
    step: Expression,
    identity: Option<Expression>,
) -> Expression {
    Expression::accumulate(
        form,
        qsl_forms::DeclaredName {
            name: accumulator_type.to_owned(),
            span: qsl_foundation::Span { start: 0, end: 0 },
        },
        "acc".to_owned(),
        "x".to_owned(),
        name(source),
        step,
        identity,
    )
}

fn convert(target: ValueType, operand: &str) -> Expression {
    Expression::convert(crate::support::type_form::type_form(&target), name(operand))
}

fn aliases() -> Vec<(String, ValueType)> {
    vec![
        ("Total".to_owned(), ValueType::Integer),
        ("Small".to_owned(), int_type(0, 30)),
        ("Flag".to_owned(), ValueType::Boolean),
        ("Tiny".to_owned(), int_type(0, 5)),
        ("Three".to_owned(), int_type(0, 3)),
        ("Positive".to_owned(), int_type(1, 3)),
        ("Hundred".to_owned(), int_type(0, 100)),
        ("Li".to_owned(), carried(integer_carrier(-5, 5))),
        ("Lp".to_owned(), carried(integer_carrier(1, 5))),
        (
            "Ld".to_owned(),
            carried(decimal_carrier(RoundingMode::NearestEven)),
        ),
        (
            "Lx".to_owned(),
            carried(decimal_carrier(RoundingMode::Exact)),
        ),
        ("Lr".to_owned(), carried(rational_carrier())),
    ]
}

fn package(types: TypeEnvironment, functions: Vec<FunctionDeclaration>) -> CheckedPackage {
    let graph = {
        let mut declarations = PackageDeclarations::new(
            qsl_semantics::check::fixture_source(),
            "fixture".to_owned(),
            qsl_foundation::IdentityLimits::default(),
        );
        declarations.types = types;
        declarations.aliases = aliases();
        declarations.functions = functions;
        // FR-094: `holder_environment`'s `Reference<M::Obj>` field keys
        // over this model's node.
        declarations.models = vec![crate::support::model::object_model(
            "collection-queries",
            "M::Obj",
            object_type("M::Obj"),
        )];
        declarations
    }
    .check(
        CheckingLimits::default(),
        quire_semantic_value::checking::CheckMode::Linked,
    )
    .unwrap();
    // ADR-013 T-1 (FR-087): the S4 link step, over an empty
    // dependency closure -- this fixture declares no import.
    CheckedPackage::link(graph)
}

fn plain() -> CheckedPackage {
    package(TypeEnvironment::default(), Vec::new())
}

fn check(
    package: &CheckedPackage,
    parameters: &[(&str, ValueType)],
    expression: &Expression,
) -> Result<qsl_semantics::check::CheckedExpression, CheckRefusal> {
    check_as(package, parameters, expression, None)
}

/// [`check`] against an expected type, which a collection literal needs to
/// fix its bound.
fn check_as(
    package: &CheckedPackage,
    parameters: &[(&str, ValueType)],
    expression: &Expression,
    expected: Option<&ValueType>,
) -> Result<qsl_semantics::check::CheckedExpression, CheckRefusal> {
    let parameters = parameters
        .iter()
        .map(|(name, value_type)| ((*name).to_owned(), value_type.clone()))
        .collect();
    package.graph().check_expression(
        parameters,
        expression,
        expected,
        CheckMode::Kernel,
        CheckingLimits::default(),
    )
}

fn ill_typed(
    package: &CheckedPackage,
    parameters: &[(&str, ValueType)],
    expression: &Expression,
) -> IllTypedCause {
    match check(package, parameters, expression).map(|_| ()) {
        Err(CheckRefusal {
            cause: CheckCause::IllTyped(cause),
            ..
        }) => cause,
        other => panic!("an ill_typed refusal, not {other:?}"),
    }
}

/// A collection argument formed by the real kernel from `values`.
fn collection(value_type: &ValueType, values: Vec<Value>) -> Value {
    let ValueType::Collection(collection_type) = value_type else {
        panic!("a collection type");
    };
    quire_exact::form_collection(collection_type, values, &mut Meter::new(UNLIMITED))
        .unwrap()
        .completed()
        .unwrap()
}

struct Run {
    outcome: Outcome<Value>,
    work: u64,
    results: u64,
}

fn run(
    package: &CheckedPackage,
    parameters: &[(&str, ValueType)],
    expression: &Expression,
    arguments: Vec<Value>,
    limits: ScalarLimits,
) -> Run {
    run_in(
        package,
        parameters,
        expression,
        arguments,
        limits,
        &ObjectEnvironment::default(),
    )
}

fn run_in(
    package: &CheckedPackage,
    parameters: &[(&str, ValueType)],
    expression: &Expression,
    arguments: Vec<Value>,
    limits: ScalarLimits,
    objects: &ObjectEnvironment,
) -> Run {
    let checked = check(package, parameters, expression).unwrap();
    evaluate_checked(package, &checked, arguments, limits, objects)
}

fn evaluate_checked(
    package: &CheckedPackage,
    checked: &qsl_semantics::check::CheckedExpression,
    arguments: Vec<Value>,
    limits: ScalarLimits,
    objects: &ObjectEnvironment,
) -> Run {
    let mut meter = Meter::new(limits);
    let evaluation = package
        .evaluate(checked, arguments, objects, &mut meter)
        .unwrap();
    let outcome = match evaluation.outcome {
        FamilyOutcome::Evaluated(outcome) => outcome,
        other => panic!("expected FamilyOutcome::Evaluated(_), got {other:?}"),
    };
    Run {
        outcome,
        work: meter.consumed(LimitKind::WorkUnits),
        results: meter.consumed(LimitKind::ResultUnits),
    }
}

fn completed(run: &Run) -> &Value {
    match &run.outcome {
        Outcome::Completed(value) => value,
        other => panic!("a completed value, not {other:?}"),
    }
}

fn elements(value: &Value) -> Vec<Value> {
    match value {
        Value::Collection(collection) => collection.elements().to_vec(),
        other => panic!("a collection, not {other:?}"),
    }
}

/// The integer a value is, or a failed test.
fn integer_of(value: &Value) -> Integer {
    match value {
        Value::Integer(integer) => integer.clone(),
        other => panic!("an integer, not {other:?}"),
    }
}

/// The boolean a value is, or a failed test.
fn boolean_of(value: &Value) -> bool {
    match value {
        Value::Boolean(boolean) => *boolean,
        other => panic!("a boolean, not {other:?}"),
    }
}

fn assert_elements(run: &Run, expected: &[i64]) {
    let held: Vec<Integer> = elements(completed(run)).iter().map(integer_of).collect();
    let expected: Vec<Integer> = expected.iter().copied().map(Integer::from).collect();
    assert_eq!(held, expected);
}

fn incomplete(consumed_and_limit: u64, next_charge: i64, charge_point: ChargePoint) -> String {
    format!(
        "{:?}",
        Outcome::<Value>::Incomplete(Incomplete {
            limit_kind: LimitKind::WorkUnits,
            limit: consumed_and_limit,
            consumed: consumed_and_limit,
            next_charge: Integer::from(next_charge),
            charge_point,
        })
    )
}

#[trace("QSpec-TC-190", "QSpec-FR-145-AC-1")]
#[test]
fn q01_map_keeps_the_source_kind_and_derives_its_bound() {
    let package = plain();
    let body = query(BinderQuery::Map, "c", literal(0));
    let cases = [
        (CollectionKind::Sequence, 0, vec![1, 2], vec![0, 0], 0),
        (CollectionKind::Set, 1, vec![1, 2], vec![0], 1),
        (CollectionKind::Bag, 0, vec![1, 2], vec![0, 0], 0),
        (CollectionKind::OrderedSet, 0, vec![2, 1], vec![0], 0),
    ];
    for (kind, minimum, source, expected, result_minimum) in cases {
        let source_type = integers(kind, minimum, 2);
        let checked = check(&package, &[("c", source_type.clone())], &body).unwrap();
        assert_eq!(checked.value_type(), &integers(kind, result_minimum, 2));
        let result = run(
            &package,
            &[("c", source_type.clone())],
            &body,
            vec![collection(&source_type, ints(&source))],
            UNLIMITED,
        );
        assert_elements(&result, &expected);
    }
}

#[trace("QSpec-TC-190", "QSpec-FR-145-AC-1")]
#[test]
fn q02_filter_keeps_multiplicity_and_first_occurrence_order() {
    let package = plain();
    let bag = integers(CollectionKind::Bag, 1, 3);
    let kept = query(
        BinderQuery::Filter,
        "b",
        binary(BinaryOperator::Equal, name("x"), literal(1)),
    );
    let checked = check(&package, &[("b", bag.clone())], &kept).unwrap();
    assert_eq!(checked.value_type(), &integers(CollectionKind::Bag, 0, 3));
    let result = run(
        &package,
        &[("b", bag.clone())],
        &kept,
        vec![collection(&bag, ints(&[1, 1, 2]))],
        UNLIMITED,
    );
    assert_elements(&result, &[1, 1]);

    let ordered = integers(CollectionKind::OrderedSet, 1, 3);
    let dropped = query(
        BinderQuery::Filter,
        "o",
        binary(BinaryOperator::NotEqual, name("x"), literal(1)),
    );
    let checked = check(&package, &[("o", ordered.clone())], &dropped).unwrap();
    assert_eq!(
        checked.value_type(),
        &integers(CollectionKind::OrderedSet, 0, 3)
    );
    let result = run(
        &package,
        &[("o", ordered.clone())],
        &dropped,
        vec![collection(&ordered, ints(&[2, 1, 3]))],
        UNLIMITED,
    );
    assert_elements(&result, &[2, 3]);
}

#[trace("QSpec-TC-190", "QSpec-FR-145-AC-4")]
#[trace("QSpec-TC-190", "QSpec-FR-145-AC-6")]
#[test]
fn q03_flatten_admits_kinds_and_derives_bounds() {
    let package = plain();
    let flatten = Expression::flatten(name("c"));
    let cases = [
        (
            CollectionKind::Sequence,
            vec![vec![1, 2], vec![2]],
            vec![1, 2, 2],
        ),
        (CollectionKind::Set, vec![vec![1], vec![1, 2]], vec![1, 2]),
        (
            CollectionKind::Bag,
            vec![vec![1], vec![1, 2]],
            vec![1, 1, 2],
        ),
        (
            CollectionKind::OrderedSet,
            vec![vec![2, 1], vec![1, 3]],
            vec![2, 1, 3],
        ),
    ];
    for (kind, inner, expected) in cases {
        let inner_type = integers(kind, 0, 2);
        let outer_type = of(kind, inner_type.clone(), 0, 2);
        let inner = inner
            .iter()
            .map(|values| collection(&inner_type, ints(values)))
            .collect();
        let result = run(
            &package,
            &[("c", outer_type.clone())],
            &flatten,
            vec![collection(&outer_type, inner)],
            UNLIMITED,
        );
        assert_elements(&result, &expected);
    }

    let bounded = of(
        CollectionKind::Set,
        integers(CollectionKind::Set, 1, 2),
        1,
        3,
    );
    let checked = check(&package, &[("c", bounded)], &flatten).unwrap();
    assert_eq!(checked.value_type(), &integers(CollectionKind::Set, 1, 6));

    for (outer, inner) in [
        (CollectionKind::Sequence, CollectionKind::Set),
        (CollectionKind::OrderedSet, CollectionKind::Bag),
    ] {
        let refused = of(outer, integers(inner, 0, 2), 0, 2);
        assert_eq!(
            ill_typed(&package, &[("c", refused)], &flatten),
            IllTypedCause::TypeMismatch
        );
    }
    let admitted = of(
        CollectionKind::Set,
        integers(CollectionKind::Sequence, 0, 2),
        0,
        2,
    );
    let checked = check(&package, &[("c", admitted)], &flatten).unwrap();
    assert_eq!(checked.value_type(), &integers(CollectionKind::Set, 0, 4));
}

#[trace("QSpec-TC-190", "QSpec-FR-145-AC-2")]
#[test]
fn q04_empty_fold_uses_identity_and_empty_reduce_is_undefined_or_refused() {
    let package = plain();
    let step = binary(BinaryOperator::Add, name("acc"), name("x"));
    let fold = accumulate(
        Accumulation::Fold,
        "Total",
        "e",
        step.clone(),
        Some(literal(0)),
    );
    let reduce = accumulate(Accumulation::Reduce, "Total", "e", step.clone(), None);
    for kind in [
        CollectionKind::Sequence,
        CollectionKind::Set,
        CollectionKind::Bag,
        CollectionKind::OrderedSet,
    ] {
        let source = integers(kind, 0, 2);
        let empty = || vec![collection(&source, Vec::new())];
        let folded = run(
            &package,
            &[("e", source.clone())],
            &fold,
            empty(),
            UNLIMITED,
        );
        assert_eq!(format!("{:?}", completed(&folded)), format!("{:?}", int(0)));
        assert_eq!((folded.work, folded.results), (1, 1));

        let checked = check(&package, &[("e", source.clone())], &reduce).unwrap();
        let mut meter = Meter::new(UNLIMITED);
        let evaluation = package
            .evaluate(&checked, empty(), &ObjectEnvironment::default(), &mut meter)
            .unwrap();
        assert!(matches!(
            evaluation.outcome,
            FamilyOutcome::Evaluated(Outcome::Undefined(Undefined::EmptyReduction))
        ));
        // FR-090-OQ-3 (ruled option A): `Evaluation.location` carries the
        // locus alongside `FamilyOutcome`.
        assert!(evaluation.location.is_some());
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 0);
    }

    let linked = {
        let mut declarations = PackageDeclarations::new(
            qsl_semantics::check::fixture_source(),
            "fixture".to_owned(),
            qsl_foundation::IdentityLimits::default(),
        );
        declarations.aliases = aliases();
        declarations.functions = vec![FunctionDeclaration::new(
            "r".to_owned(),
            vec![(
                "e".to_owned(),
                crate::support::type_form::type_form(&integers(CollectionKind::Set, 0, 2)),
            )],
            crate::support::type_form::type_form(&ValueType::Integer),
            None,
            reduce,
        )];
        declarations
    }
    .check(
        CheckingLimits::default(),
        quire_semantic_value::checking::CheckMode::Linked,
    )
    .unwrap_err();
    assert_eq!(linked.len(), 1);
    let refusal = &linked[0];
    assert_eq!(refusal.cause.code().as_str(), "undefined_expression");
    assert_eq!(refusal.cause.cause(), Some("unproved-range"));
    assert_eq!(
        refusal.cause,
        CheckCause::Unproved(Obligation::NonemptyReduction {
            size: Box::new(ProvedInterval {
                lower: Some(Integer::from(0_i64)),
                upper: Some(Integer::from(2_i64)),
            }),
        })
    );
    assert!(refusal.location.depth() == 0);

    let source = [("e", integers(CollectionKind::Sequence, 0, 2))];
    let without_identity = accumulate(Accumulation::Fold, "Total", "e", step.clone(), None);
    let with_identity = accumulate(Accumulation::Reduce, "Total", "e", step, Some(literal(0)));
    assert_eq!(
        ill_typed(&package, &source, &without_identity),
        IllTypedCause::TypeMismatch
    );
    assert_eq!(
        ill_typed(&package, &source, &with_identity),
        IllTypedCause::TypeMismatch
    );
}

#[trace("QSpec-TC-190", "QSpec-FR-145-AC-5")]
#[trace("QSpec-TC-190", "QSpec-FR-145-AC-9")]
#[test]
fn q05_set_and_bag_steps_must_be_in_the_syntactic_catalog() {
    let add = FunctionDeclaration::new(
        "add".to_owned(),
        vec![
            (
                "a".to_owned(),
                crate::support::type_form::type_form(&ValueType::Integer),
            ),
            (
                "b".to_owned(),
                crate::support::type_form::type_form(&ValueType::Integer),
            ),
        ],
        crate::support::type_form::type_form(&ValueType::Integer),
        None,
        binary(BinaryOperator::Add, name("a"), name("b")),
    );
    let package = package(TypeEnvironment::default(), vec![add]);
    let set = integers(CollectionKind::Set, 0, 3);
    let parameters = [("s", set.clone())];
    let fold = |step: Expression, identity: Expression| {
        accumulate(Accumulation::Fold, "Total", "s", step, Some(identity))
    };
    let sum = fold(
        binary(BinaryOperator::Add, name("acc"), name("x")),
        literal(0),
    );
    let result = run(
        &package,
        &parameters,
        &sum,
        vec![collection(&set, ints(&[1, 2, 3]))],
        UNLIMITED,
    );
    assert_eq!(format!("{:?}", completed(&result)), format!("{:?}", int(6)));
    check(
        &package,
        &parameters,
        &fold(
            binary(BinaryOperator::Multiply, name("x"), name("acc")),
            literal(0),
        ),
    )
    .unwrap();

    let ineligible = [
        binary(BinaryOperator::Subtract, name("acc"), name("x")),
        binary(BinaryOperator::Add, name("acc"), name("acc")),
        Expression::call("add".to_owned(), vec![name("acc"), name("x")]),
    ];
    for step in ineligible {
        assert_eq!(
            ill_typed(&package, &parameters, &fold(step, literal(0))),
            IllTypedCause::OperatorIneligible
        );
    }
    assert_eq!(
        ill_typed(
            &package,
            &parameters,
            &fold(
                binary(BinaryOperator::Equal, name("x"), literal(1)),
                literal(0)
            )
        ),
        IllTypedCause::TypeMismatch
    );
    assert_eq!(
        ill_typed(
            &package,
            &parameters,
            &fold(
                binary(BinaryOperator::Add, name("acc"), name("x")),
                Expression::boolean(true)
            )
        ),
        IllTypedCause::TypeMismatch
    );

    let bounded_step = binary(BinaryOperator::Add, name("acc"), name("x"));
    for element in [int_type(0, 30), int_type(0, 10)] {
        let parameters = [("s", of(CollectionKind::Set, element, 0, 3))];
        let small = accumulate(
            Accumulation::Fold,
            "Small",
            "s",
            bounded_step.clone(),
            Some(literal(0)),
        );
        assert_eq!(
            ill_typed(&package, &parameters, &small),
            IllTypedCause::OperatorIneligible
        );
    }

    let flags = [("s", of(CollectionKind::Set, ValueType::Boolean, 0, 2))];
    let all = accumulate(
        Accumulation::Fold,
        "Flag",
        "s",
        binary(BinaryOperator::And, name("acc"), name("x")),
        Some(Expression::boolean(true)),
    );
    check(&package, &flags, &all).unwrap();
    let sequence = [("s", integers(CollectionKind::Sequence, 0, 3))];
    let difference = accumulate(
        Accumulation::Fold,
        "Total",
        "s",
        binary(BinaryOperator::Subtract, name("acc"), name("x")),
        Some(literal(0)),
    );
    check(&package, &sequence, &difference).unwrap();
}

#[trace("QSpec-TC-190", "QSpec-FR-145-AC-3")]
#[trace("QSpec-TC-190", "QSpec-FR-145-AC-7")]
#[test]
fn q06_conversions_record_static_loss_and_check_the_target_bound() {
    use CollectionProperty::{Multiplicity, Order, Uniqueness};
    let package = plain();
    let cases = [
        (
            integers(CollectionKind::Sequence, 0, 3),
            integers(CollectionKind::Set, 0, 3),
            vec![1, 2, 3],
            vec![1, 2, 3],
            vec![Order, Multiplicity],
        ),
        (
            integers(CollectionKind::Bag, 0, 3),
            integers(CollectionKind::Sequence, 0, 3),
            vec![2, 1, 1],
            vec![1, 1, 2],
            vec![],
        ),
        (
            integers(CollectionKind::OrderedSet, 0, 2),
            integers(CollectionKind::Bag, 0, 2),
            vec![2, 1],
            vec![1, 2],
            vec![Order, Uniqueness],
        ),
        (
            integers(CollectionKind::Sequence, 0, 2),
            integers(CollectionKind::Set, 0, 1),
            vec![1, 1],
            vec![1],
            vec![Order, Multiplicity],
        ),
    ];
    for (source, target, values, expected, discarded) in cases {
        let expression = convert(target, "a");
        let parameters = [("a", source.clone())];
        let checked = check(&package, &parameters, &expression).unwrap();
        let losses: Vec<CollectionLoss> = checked.losses();
        assert_eq!(losses.len(), 1);
        assert_eq!(losses[0].discarded, discarded);
        let result = run(
            &package,
            &parameters,
            &expression,
            vec![collection(&source, ints(&values))],
            UNLIMITED,
        );
        assert_elements(&result, &expected);
    }

    let source = integers(CollectionKind::Sequence, 0, 2);
    let target = collection_type(CollectionKind::Set, ValueType::Integer, 0, 1);
    let result = run(
        &package,
        &[("e", source.clone())],
        &convert(ValueType::collection(target.clone()), "e"),
        vec![collection(&source, ints(&[1, 2]))],
        UNLIMITED,
    );
    assert!(matches!(
        result.outcome,
        Outcome::Refused(Refusal::CardinalityOutOfBound {
            violation: BoundViolation::AboveMaximum,
            count: 2,
            ..
        })
    ));

    assert_eq!(
        ill_typed(
            &package,
            &[("x", integers(CollectionKind::Set, 0, 2))],
            &convert(of(CollectionKind::Set, int_type(0, 5), 0, 2), "x")
        ),
        IllTypedCause::TypeMismatch
    );
}

#[trace("QSpec-TC-190", "QSpec-FR-145-AC-8")]
#[test]
fn q07_map_charges_visits_then_membership_formation() {
    let package = plain();
    let set = integers(CollectionKind::Set, 0, 3);
    let body = query(BinderQuery::Map, "s", literal(0));
    let arguments = || vec![collection(&set, ints(&[1, 2, 3]))];
    let parameters = [("s", set.clone())];
    let result = run(&package, &parameters, &body, arguments(), UNLIMITED);
    assert_elements(&result, &[0]);
    assert_eq!((result.work, result.results), (11, 2));

    let limited = run(&package, &parameters, &body, arguments(), work_limit(10));
    assert_eq!(
        format!("{:?}", limited.outcome),
        incomplete(10, 1, ChargePoint::CollectionResultRetain)
    );
}

#[trace("QSpec-TC-190", "QSpec-FR-145-AC-8")]
#[trace("QSpec-TC-041", "QSpec-FR-041-AC-5")]
#[test]
fn q08_exists_stops_at_the_first_witness_and_filter_exposes_no_partial_result() {
    let package = plain();
    let sequence = integers(CollectionKind::Sequence, 0, 3);
    let parameters = [("q", sequence.clone())];
    let arguments = || vec![collection(&sequence, ints(&[1, 2, 3]))];
    let exists = query(
        BinderQuery::Exists,
        "q",
        binary(BinaryOperator::Equal, name("x"), literal(2)),
    );
    let result = run(&package, &parameters, &exists, arguments(), UNLIMITED);
    assert_eq!(
        format!("{:?}", completed(&result)),
        format!("{:?}", Value::Boolean(true))
    );
    assert_eq!((result.work, result.results), (13, 3));

    let filter = query(
        BinderQuery::Filter,
        "q",
        binary(BinaryOperator::Equal, name("x"), literal(1)),
    );
    let limited = run(&package, &parameters, &filter, arguments(), work_limit(7));
    assert_eq!(
        format!("{:?}", limited.outcome),
        incomplete(7, 2, ChargePoint::EqualityPlanForm)
    );
}

#[trace("QSpec-TC-190", "QSpec-FR-145-AC-8")]
#[test]
fn q09_sequence_to_set_conversion_coalesces_with_membership_charges() {
    let package = plain();
    let sequence = integers(CollectionKind::Sequence, 0, 3);
    let expression = convert(integers(CollectionKind::Set, 0, 3), "q");
    let parameters = [("q", sequence.clone())];
    let checked = check(&package, &parameters, &expression).unwrap();
    assert_eq!(
        checked.losses()[0].discarded,
        vec![CollectionProperty::Order, CollectionProperty::Multiplicity]
    );
    let result = run(
        &package,
        &parameters,
        &expression,
        vec![collection(&sequence, ints(&[1, 2, 1]))],
        UNLIMITED,
    );
    assert_elements(&result, &[1, 2]);
    assert_eq!((result.work, result.results), (11, 3));
}

fn holder_environment() -> TypeEnvironment {
    TypeEnvironment::new(
        [CompositeDeclaration::new(
            key("Holder"),
            "Holder",
            CompositeShape::Record(vec![FieldDeclaration::new(
                "r",
                ValueType::Reference(object_type("M::Obj")),
                Presence::Required,
            )]),
        )],
        [ObjectTypeDeclaration::new(
            object_type("M::Obj"),
            "Obj",
            vec![],
        )],
    )
    .unwrap()
}

fn universe_id(tag: &str) -> UniverseId {
    crate::support::ids::universe_id(tag.as_bytes())
}

fn object(identity: &str) -> ObjectReference {
    ObjectReference::new(
        universe_id("u1"),
        object_type("M::Obj"),
        ObjectId::new(identity).unwrap(),
    )
}

fn holder(types: &TypeEnvironment, identity: &str) -> Value {
    types
        .record(
            key("Holder"),
            vec![("r", FieldValue::Present(Value::Reference(object(identity))))],
        )
        .unwrap()
}

#[trace("QSpec-TC-190", "QSpec-FR-145-AC-8")]
#[test]
fn q10_contains_stops_at_the_first_equal_member_and_size_only_retains() {
    let types = holder_environment();
    let objects = ObjectEnvironment::new(
        ObjectClosure::new(
            &types,
            [(object("h1"), Vec::new()), (object("h2"), Vec::new())],
            &[],
        )
        .unwrap(),
    );
    let package = package(types.clone(), Vec::new());
    let holder_type = ValueType::Composite(key("Holder"));
    let set = of(CollectionKind::Set, holder_type.clone(), 0, 2);
    let (h1, h2) = (holder(&types, "h1"), holder(&types, "h2"));
    let parameters = [
        ("hs", set.clone()),
        ("h1", holder_type.clone()),
        ("h2", holder_type),
    ];
    let arguments = || {
        vec![
            collection(&set, vec![h2.clone(), h1.clone()]),
            h1.clone(),
            h2.clone(),
        ]
    };
    let contains = |item: &str| Expression::contains(name("hs"), name(item));
    let true_value = format!("{:?}", Value::Boolean(true));

    let second = run_in(
        &package,
        &parameters,
        &contains("h2"),
        arguments(),
        UNLIMITED,
        &objects,
    );
    assert_eq!(format!("{:?}", completed(&second)), true_value);
    assert_eq!((second.work, second.results), (13, 1));

    let first = run_in(
        &package,
        &parameters,
        &contains("h1"),
        arguments(),
        UNLIMITED,
        &objects,
    );
    assert_eq!(format!("{:?}", completed(&first)), true_value);
    assert_eq!(first.work, 7);

    let limited = run_in(
        &package,
        &parameters,
        &contains("h2"),
        arguments(),
        work_limit(3),
        &objects,
    );
    assert_eq!(
        format!("{:?}", limited.outcome),
        format!(
            "{:?}",
            Outcome::<Value>::Incomplete(Incomplete {
                limit_kind: LimitKind::WorkUnits,
                limit: 3,
                consumed: 0,
                next_charge: Integer::from(4_i64),
                charge_point: ChargePoint::CollectionMemberWalk,
            })
        )
    );

    let size = run_in(
        &package,
        &parameters,
        &Expression::size(name("hs")),
        arguments(),
        UNLIMITED,
        &objects,
    );
    assert_eq!(format!("{:?}", completed(&size)), format!("{:?}", int(2)));
    assert_eq!((size.work, size.results), (1, 1));
}

#[trace("QSpec-TC-190", "QSpec-FR-145-AC-8")]
#[test]
fn q11_fold_charges_visit_step_and_one_accumulator_retain() {
    let package = plain();
    let sequence = integers(CollectionKind::Sequence, 0, 2);
    let parameters = [("q", sequence.clone())];
    let fold = accumulate(
        Accumulation::Fold,
        "Total",
        "q",
        binary(BinaryOperator::Add, name("acc"), name("x")),
        Some(literal(0)),
    );
    let arguments = || vec![collection(&sequence, ints(&[1, 2]))];
    let result = run(&package, &parameters, &fold, arguments(), UNLIMITED);
    assert_eq!(format!("{:?}", completed(&result)), format!("{:?}", int(3)));
    assert_eq!((result.work, result.results), (9, 3));

    let limited = run(&package, &parameters, &fold, arguments(), work_limit(8));
    assert_eq!(
        format!("{:?}", limited.outcome),
        incomplete(8, 1, ChargePoint::CollectionResultRetain)
    );
}

fn sum(result_type: &str, source: &str, summand: Expression) -> Expression {
    Expression::sum(
        qsl_forms::DeclaredName {
            name: result_type.to_owned(),
            span: qsl_foundation::Span { start: 0, end: 0 },
        },
        "x".to_owned(),
        name(source),
        summand,
    )
}

fn check_linked(
    package: &CheckedPackage,
    parameters: &[(&str, ValueType)],
    expression: &Expression,
) -> Result<qsl_semantics::check::CheckedExpression, CheckRefusal> {
    let parameters = parameters
        .iter()
        .map(|(name, value_type)| ((*name).to_owned(), value_type.clone()))
        .collect();
    package.graph().check_expression(
        parameters,
        expression,
        None,
        CheckMode::Linked,
        CheckingLimits::default(),
    )
}

#[trace("QSpec-TC-190", "QSpec-FR-145-AC-8")]
#[test]
fn q12_sum_charges_visits_additions_and_one_scalar_retain() {
    let package = plain();
    let sequence = of(CollectionKind::Sequence, int_type(0, 2), 0, 3);
    let parameters = [("q", sequence.clone())];
    let total = sum("Total", "q", name("x"));
    let result = run(
        &package,
        &parameters,
        &total,
        vec![collection(&sequence, ints(&[1, 2]))],
        UNLIMITED,
    );
    assert_eq!(format!("{:?}", completed(&result)), format!("{:?}", int(3)));
    // Two visits, one integer addition, then the scalar retain.
    assert_eq!((result.work, result.results), (6, 2));

    let empty = run(
        &package,
        &parameters,
        &total,
        vec![collection(&sequence, Vec::new())],
        UNLIMITED,
    );
    assert_eq!(format!("{:?}", completed(&empty)), format!("{:?}", int(0)));
    assert_eq!((empty.work, empty.results), (1, 1));

    let limited = run(
        &package,
        &parameters,
        &total,
        vec![collection(&sequence, ints(&[1, 2]))],
        work_limit(5),
    );
    assert_eq!(
        format!("{:?}", limited.outcome),
        incomplete(5, 1, ChargePoint::CollectionResultRetain)
    );
}

#[trace("QSpec-TC-190", "QSpec-FR-145-AC-6")]
#[trace("QSpec-TC-041", "QSpec-FR-041-AC-6")]
#[test]
fn q13_sum_proves_every_prefix_inside_its_domain_for_every_order() {
    let package = plain();
    for kind in [CollectionKind::Sequence, CollectionKind::Bag] {
        let natural = [("q", of(kind, int_type(0, 2), 0, 3))];
        check_linked(&package, &natural, &sum("Small", "q", name("x"))).unwrap();
        check_linked(&package, &natural, &sum("Tiny", "q", name("x"))).unwrap_err();

        // A negative summand makes a prefix of `-3` reachable in some order.
        let signed = [("q", of(kind, int_type(-1, 2), 0, 3))];
        assert_eq!(
            check_linked(&package, &signed, &sum("Small", "q", name("x")))
                .unwrap_err()
                .cause,
            CheckCause::Unproved(Obligation::Range {
                required: Box::new(ProvedInterval {
                    lower: Some(Integer::from(0_i64)),
                    upper: Some(Integer::from(30_i64)),
                }),
                proved: Box::new(ProvedInterval {
                    lower: Some(Integer::from(-3_i64)),
                    upper: Some(Integer::from(6_i64)),
                }),
            })
        );
    }

    let sequence = of(CollectionKind::Sequence, int_type(0, 2), 0, 3);
    let over = run(
        &package,
        &[("q", sequence.clone())],
        &sum("Tiny", "q", name("x")),
        vec![collection(&sequence, ints(&[2, 2, 2]))],
        UNLIMITED,
    );
    // FR-096: a running total outside `N` is undefined, never refused.
    assert!(matches!(
        over.outcome,
        Outcome::Undefined(Undefined::SumOutOfDomain)
    ));
    assert_eq!(
        ill_typed(&package, &[("q", sequence)], &sum("Flag", "q", name("x"))),
        IllTypedCause::TypeMismatch
    );
}

/// A collection literal's `form` call keeps an `Incomplete` outcome intact:
/// a meter denied at `collection.result-retain` stops the literal
/// `Incomplete`, not refused.
#[trace("QSpec-TC-190", "QSpec-FR-145-AC-8")]
#[trace("QSpec-TC-041", "QSpec-FR-041-AC-5")]
#[test]
fn q14_collection_literal_formation_stops_incomplete_at_result_retain() {
    let package = plain();
    let literal = Expression::collection(CollectionKind::Set, vec![name("a"), name("b")]);
    let parameters = [("a", ValueType::Integer), ("b", ValueType::Integer)];
    let expected = integers(CollectionKind::Set, 0, 2);
    let checked = check_as(&package, &parameters, &literal, Some(&expected)).unwrap();
    let evaluate = |limits| {
        evaluate_checked(
            &package,
            &checked,
            vec![int(2), int(1)],
            limits,
            &ObjectEnvironment::default(),
        )
    };
    let result = evaluate(UNLIMITED);
    assert_elements(&result, &[1, 2]);
    assert_eq!((result.work, result.results), (7, 3));

    let limited = evaluate(work_limit(6));
    assert_eq!(
        format!("{:?}", limited.outcome),
        incomplete(6, 1, ChargePoint::CollectionResultRetain)
    );
}

/// A sequence-to-set conversion's `form` call keeps an `Incomplete` outcome
/// intact at `collection.result-retain`.
#[trace("QSpec-TC-190", "QSpec-FR-145-AC-8")]
#[trace("QSpec-TC-041", "QSpec-FR-041-AC-5")]
#[test]
fn q15_sequence_to_set_conversion_stops_incomplete_at_result_retain() {
    let package = plain();
    let sequence = integers(CollectionKind::Sequence, 0, 3);
    let expression = convert(integers(CollectionKind::Set, 0, 3), "q");
    let parameters = [("q", sequence.clone())];
    let limited = run(
        &package,
        &parameters,
        &expression,
        vec![collection(&sequence, ints(&[1, 2, 1]))],
        work_limit(10),
    );
    assert_eq!(
        format!("{:?}", limited.outcome),
        incomplete(10, 1, ChargePoint::CollectionResultRetain)
    );
}

/// `flatten`'s `form` call keeps an `Incomplete` outcome intact at
/// `collection.result-retain`.
#[trace("QSpec-TC-190", "QSpec-FR-145-AC-8")]
#[trace("QSpec-TC-041", "QSpec-FR-041-AC-5")]
#[test]
fn q16_flatten_stops_incomplete_at_result_retain() {
    let package = plain();
    let inner_type = integers(CollectionKind::Sequence, 0, 2);
    let outer_type = of(CollectionKind::Sequence, inner_type.clone(), 0, 2);
    let flatten = Expression::flatten(name("c"));
    let parameters = [("c", outer_type.clone())];
    let arguments = || {
        let inner = vec![
            collection(&inner_type, ints(&[1, 2])),
            collection(&inner_type, ints(&[2])),
        ];
        vec![collection(&outer_type, inner)]
    };
    let result = run(&package, &parameters, &flatten, arguments(), UNLIMITED);
    assert_elements(&result, &[1, 2, 2]);
    assert_eq!((result.work, result.results), (7, 4));

    let limited = run(&package, &parameters, &flatten, arguments(), work_limit(6));
    assert_eq!(
        format!("{:?}", limited.outcome),
        incomplete(6, 1, ChargePoint::CollectionResultRetain)
    );
}

/// The unbounded `K<element>` (ADR-014 N-3).
fn unbounded(kind: CollectionKind, element: ValueType) -> ValueType {
    ValueType::collection(CollectionType::new(kind, element, None))
}

/// TC-441 (ADR-014 §2, N-3): `map` and `flatMap` over an unbounded source
/// give an unbounded result, and evaluate over a concrete value of it.
#[trace("TC-441", "FR-097-AC-7")]
#[test]
fn tc_441_map_and_flat_map_over_an_unbounded_source_are_unbounded() {
    let package = plain();
    let source_type = unbounded(CollectionKind::Sequence, int_type(0, 9));
    let parameters = [("c", source_type.clone())];
    let mapped = query(BinderQuery::Map, "c", literal(0));
    let checked = check(&package, &parameters, &mapped).unwrap();
    assert_eq!(
        checked.value_type(),
        &unbounded(CollectionKind::Sequence, ValueType::Integer)
    );
    let result = run(
        &package,
        &parameters,
        &mapped,
        vec![collection(&source_type, ints(&[1, 2, 3, 4, 5]))],
        UNLIMITED,
    );
    assert_elements(&result, &[0, 0, 0, 0, 0]);

    // `flatMap` over an unbounded source of bounded inner sequences.
    let outer = unbounded(
        CollectionKind::Sequence,
        integers(CollectionKind::Sequence, 0, 2),
    );
    let flat_mapped = query(BinderQuery::FlatMap, "c", name("x"));
    let checked = check(&package, &[("c", outer)], &flat_mapped).unwrap();
    assert_eq!(
        checked.value_type(),
        &unbounded(CollectionKind::Sequence, ValueType::Integer)
    );
}

/// TC-441: `filter` over an unbounded source gives an unbounded result.
#[trace("TC-441", "FR-097-AC-7")]
#[test]
fn tc_441_filter_over_an_unbounded_source_is_unbounded() {
    let package = plain();
    let source_type = unbounded(CollectionKind::Bag, ValueType::Integer);
    let parameters = [("b", source_type.clone())];
    let kept = query(
        BinderQuery::Filter,
        "b",
        binary(BinaryOperator::Equal, name("x"), literal(1)),
    );
    let checked = check(&package, &parameters, &kept).unwrap();
    assert_eq!(checked.value_type(), &source_type);
    let result = run(
        &package,
        &parameters,
        &kept,
        vec![collection(&source_type, ints(&[1, 2, 1, 3]))],
        UNLIMITED,
    );
    assert_elements(&result, &[1, 1]);
}

/// TC-441: `flatten` is unbounded when the outer, the inner or both are,
/// and keeps its derived bound when both are bounded.
#[trace("TC-441", "FR-097-AC-7")]
#[test]
fn tc_441_flatten_is_unbounded_when_either_level_is() {
    let package = plain();
    let flatten = Expression::flatten(name("c"));
    let bounded_inner = integers(CollectionKind::Sequence, 0, 2);
    let unbounded_inner = unbounded(CollectionKind::Sequence, ValueType::Integer);
    let cases = [
        (
            unbounded(CollectionKind::Sequence, bounded_inner.clone()),
            unbounded(CollectionKind::Sequence, ValueType::Integer),
        ),
        (
            of(CollectionKind::Sequence, unbounded_inner.clone(), 0, 2),
            unbounded(CollectionKind::Sequence, ValueType::Integer),
        ),
        (
            unbounded(CollectionKind::Sequence, unbounded_inner),
            unbounded(CollectionKind::Sequence, ValueType::Integer),
        ),
        (
            of(CollectionKind::Sequence, bounded_inner, 0, 2),
            integers(CollectionKind::Sequence, 0, 4),
        ),
    ];
    for (outer, expected) in cases {
        let checked = check(&package, &[("c", outer.clone())], &flatten).unwrap();
        assert_eq!(checked.value_type(), &expected, "flatten of {outer:?}");
    }
}

/// TC-441 (checked facts): an unbounded source's size is `[0, +inf)`, so a
/// `sum` into a bounded result is unproved, with no proved upper bound,
/// where the same source bounded at `[0, 3]` proves it.
#[trace("TC-441", "FR-097-AC-7")]
#[test]
fn tc_441_an_unbounded_source_has_no_proved_size_maximum() {
    let package = plain();
    let bounded = [("q", of(CollectionKind::Sequence, int_type(0, 2), 0, 3))];
    check_linked(&package, &bounded, &sum("Small", "q", name("x"))).unwrap();
    let open = [("q", unbounded(CollectionKind::Sequence, int_type(0, 2)))];
    assert_eq!(
        check_linked(&package, &open, &sum("Small", "q", name("x")))
            .unwrap_err()
            .cause,
        CheckCause::Unproved(Obligation::Range {
            required: Box::new(ProvedInterval {
                lower: Some(Integer::from(0_i64)),
                upper: Some(Integer::from(30_i64)),
            }),
            proved: Box::new(ProvedInterval {
                lower: Some(Integer::from(0_i64)),
                upper: None,
            }),
        })
    );
}

/// One kernel-mode `sum<Three>(x in q: x)` evaluation: its outcome, its
/// location, the summand's and the `sum` node's own locations, and every
/// admitted charge point.
struct SumRun {
    outcome: FamilyOutcome<Value>,
    location: Option<Location>,
    sum: Location,
    summand: Location,
    charges: Vec<ChargePoint>,
}

fn run_three_sum(source: ValueType, values: &[i64]) -> SumRun {
    let package = plain();
    let parameters = [("q", source.clone())];
    let checked = check(&package, &parameters, &sum("Three", "q", name("x"))).unwrap();
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = package
        .evaluate(
            &checked,
            vec![collection(&source, ints(values))],
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .unwrap();
    SumRun {
        outcome: evaluation.outcome,
        location: evaluation.location,
        sum: checked.root().location().clone(),
        summand: checked.root().children()[1].location().clone(),
        charges: crate::support::admitted_charge_points(&meter),
    }
}

/// TC-500 step 1 (FR-096-AC-14): `sum<Three>` over `2, 2` leaves `Int[0, 3]`
/// at its one addition: undefined at the `sum` node, not refused, with the
/// addition's `arithmetic` charge the last and no `result-retain`.
#[trace("TC-500", "FR-096-AC-14")]
#[trace("QSpec-TC-041", "QSpec-FR-041-AC-6")]
#[test]
fn tc_500_a_running_total_outside_the_domain_is_undefined_at_the_sum_node() {
    let run = run_three_sum(of(CollectionKind::Sequence, int_type(0, 3), 0, 2), &[2, 2]);
    assert!(
        matches!(
            run.outcome,
            FamilyOutcome::Evaluated(Outcome::Undefined(Undefined::SumOutOfDomain))
        ),
        "{:?}",
        run.outcome
    );
    assert_eq!(run.location.as_ref(), Some(&run.sum));
    assert_ne!(run.sum, run.summand);
    assert_eq!(
        run.charges.last(),
        Some(&ChargePoint::IntegerArithmeticArithmetic)
    );
    assert!(!run
        .charges
        .contains(&ChargePoint::IntegerArithmeticResultRetain));
    assert!(!run.charges.contains(&ChargePoint::CollectionResultRetain));
}

/// TC-500 step 2 (FR-096-AC-14): the same `sum` over `1, 2` completes with
/// `3`.
#[trace("TC-500", "FR-096-AC-14")]
#[test]
fn tc_500_a_running_total_inside_the_domain_completes() {
    let run = run_three_sum(of(CollectionKind::Sequence, int_type(0, 3), 0, 2), &[1, 2]);
    match run.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => {
            assert_eq!(format!("{value:?}"), format!("{:?}", int(3)));
        }
        other => panic!("a completed 3, not {other:?}"),
    }
}

/// TC-500 step 3 (FR-096-AC-14): a seed outside `Int[0, 3]` (`5`, from a
/// `Sequence<Int[0, 9]>`) is undefined at the summand node, before any
/// addition is charged, even though the next summand would bring the total
/// back.
#[trace("TC-500", "FR-096-AC-14")]
#[test]
fn tc_500_a_seed_outside_the_domain_is_undefined_at_the_summand_node() {
    let run = run_three_sum(of(CollectionKind::Sequence, int_type(0, 9), 0, 2), &[5, 0]);
    assert!(
        matches!(
            run.outcome,
            FamilyOutcome::Evaluated(Outcome::Undefined(Undefined::SumOutOfDomain))
        ),
        "{:?}",
        run.outcome
    );
    assert_eq!(run.location.as_ref(), Some(&run.summand));
    assert!(!run.charges.iter().any(|point| matches!(
        point,
        ChargePoint::IntegerArithmeticOperands
            | ChargePoint::IntegerArithmeticArithmetic
            | ChargePoint::IntegerArithmeticResultRetain
    )));
}

fn count(result_type: &str, source: &str, predicate: Expression) -> Expression {
    Expression::count(
        qsl_forms::DeclaredName {
            name: result_type.to_owned(),
            span: qsl_foundation::Span { start: 0, end: 0 },
        },
        "x".to_owned(),
        name(source),
        predicate,
    )
}

/// The `Int[..]` domain a real kernel evaluation's `IntegerOutOfDomain`
/// refusal names, or a failed test.
fn refused_domain(outcome: &Outcome<Value>) -> IntegerInterval {
    match outcome {
        Outcome::Refused(Refusal::IntegerOutOfDomain { target }) => (**target).clone(),
        other => panic!("an IntegerOutOfDomain refusal, not {other:?}"),
    }
}

/// TC-428 (FR-096-AC-8): a `Coerce` into `Int[0, 3]` refuses with that
/// declared domain, and `count<Three>` past `3` refuses with its own.
#[trace("TC-428", "FR-096-AC-8")]
#[test]
fn coerce_and_count_refusals_carry_the_declared_domain() {
    let package = plain();
    let three = IntegerInterval::spanning(Integer::from(0_i64), Integer::from(3_i64));

    let parameters = [("a", ValueType::Integer)];
    let checked = check_as(&package, &parameters, &name("a"), Some(&int_type(0, 3))).unwrap();
    let coerced = evaluate_checked(
        &package,
        &checked,
        vec![int(7)],
        UNLIMITED,
        &ObjectEnvironment::default(),
    );
    assert_eq!(refused_domain(&coerced.outcome), three);

    // AC-8: the record built from that S6a `Evaluation` renders the domain.
    let evaluation = package
        .evaluate(
            &checked,
            vec![int(7)],
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .unwrap();
    let record =
        qsl_eval::value::refusal_record(&evaluation, |location| package.graph().region(location))
            .expect("integer_out_of_domain has a record");
    assert_eq!(
        (record.code().code(), record.code().cause()),
        ("integer_out_of_domain", "outside-domain")
    );
    assert_eq!(
        record.fields().get("expected").map(String::as_str),
        Some("Int[0, 3]")
    );

    let source = of(CollectionKind::Sequence, ValueType::Boolean, 0, 5);
    let counted = run(
        &package,
        &[("q", source.clone())],
        &count("Three", "q", name("x")),
        vec![collection(&source, vec![Value::Boolean(true); 5])],
        UNLIMITED,
    );
    assert_eq!(refused_domain(&counted.outcome), three);
}

/// TC-500 / FR-100-AC-10: `sum<Int[1, 3]>` over an empty sequence checks
/// under `CheckMode::Kernel` and the seed `0` is outside `N`: undefined at
/// the `sum` node, with no addition and no retain charged.
#[trace("TC-500", "FR-096-AC-14")]
#[test]
fn tc_500_an_empty_sum_whose_domain_excludes_zero_is_undefined_at_the_sum_node() {
    let package = plain();
    let source = of(CollectionKind::Sequence, int_type(1, 3), 0, 2);
    let parameters = [("q", source.clone())];
    let checked = check(&package, &parameters, &sum("Positive", "q", name("x"))).unwrap();
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = package
        .evaluate(
            &checked,
            vec![collection(&source, ints(&[]))],
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .unwrap();
    assert!(matches!(
        evaluation.outcome,
        FamilyOutcome::Evaluated(Outcome::Undefined(Undefined::SumOutOfDomain))
    ));
    assert_eq!(
        evaluation.location.as_ref(),
        Some(checked.root().location())
    );
    assert!(!crate::support::admitted_charge_points(&meter)
        .contains(&ChargePoint::CollectionResultRetain));
    assert!(!crate::support::admitted_charge_points(&meter)
        .contains(&ChargePoint::IntegerArithmeticArithmetic));
}

/// QSpec FR-041's example domain: `Amount` 1..20, `N = 5`, `Total` 0..100 and
/// `Count` 0..5, over the amounts `[2, 2, 3]`.
fn amounts() -> ValueType {
    of(CollectionKind::Sequence, int_type(1, 20), 0, 5)
}

fn run_amounts(expression: &Expression, values: &[i64]) -> Run {
    let source = amounts();
    run(
        &plain(),
        &[("q", source.clone())],
        expression,
        vec![collection(&source, ints(values))],
        UNLIMITED,
    )
}

fn is_two() -> Expression {
    binary(BinaryOperator::Equal, name("x"), literal(2))
}

#[trace("QSpec-TC-041", "QSpec-FR-041-AC-1")]
#[test]
fn fr041_the_amounts_example_keeps_duplicate_occurrences_and_its_exact_aggregates() {
    let map = run_amounts(&query(BinderQuery::Map, "q", name("x")), &[2, 2, 3]);
    assert_elements(&map, &[2, 2, 3]);
    let filter = run_amounts(
        &query(
            BinderQuery::Filter,
            "q",
            binary(BinaryOperator::GreaterOrEqual, name("x"), literal(2)),
        ),
        &[2, 2, 3],
    );
    assert_elements(&filter, &[2, 2, 3]);
    let counted = run_amounts(&count("Tiny", "q", is_two()), &[2, 2, 3]);
    assert_eq!(integer_of(completed(&counted)), Integer::from(2_i64));
    let summed = run_amounts(&sum("Hundred", "q", name("x")), &[2, 2, 3]);
    assert_eq!(integer_of(completed(&summed)), Integer::from(7_i64));
    let size = run_amounts(&Expression::size(name("q")), &[2, 2, 3]);
    assert_eq!(integer_of(completed(&size)), Integer::from(3_i64));
}

#[trace("QSpec-TC-041", "QSpec-FR-041-AC-2")]
#[test]
fn fr041_an_empty_domain_yields_the_declared_query_identities() {
    let empty = |expression: &Expression| completed(&run_amounts(expression, &[])).clone();
    let always = binary(BinaryOperator::Equal, literal(1), literal(1));
    assert!(boolean_of(&empty(&query(
        BinderQuery::Forall,
        "q",
        always.clone()
    ))));
    assert!(!boolean_of(&empty(&query(
        BinderQuery::Exists,
        "q",
        always
    ))));
    assert!(!boolean_of(&empty(&Expression::contains(
        name("q"),
        literal(2)
    ))));
    assert_elements(
        &run_amounts(&query(BinderQuery::Map, "q", name("x")), &[]),
        &[],
    );
    assert_elements(
        &run_amounts(&query(BinderQuery::Filter, "q", is_two()), &[]),
        &[],
    );
    for zero in [
        count("Tiny", "q", is_two()),
        sum("Hundred", "q", name("x")),
        Expression::size(name("q")),
    ] {
        assert_eq!(integer_of(&empty(&zero)), Integer::from(0_i64));
    }
}

#[trace("QSpec-TC-041", "QSpec-FR-041-AC-3")]
#[test]
fn fr041_a_wrong_domain_refuses_and_six_items_are_not_admitted_to_the_total() {
    let package = plain();
    // N = 5 over Amount 1..20 proves a total in 0..100; N = 6 does not, even
    // though the values observed in a run are small.
    let five = [("q", amounts())];
    check_linked(&package, &five, &sum("Hundred", "q", name("x"))).unwrap();
    let six = [("q", of(CollectionKind::Sequence, int_type(1, 20), 0, 6))];
    assert!(matches!(
        check_linked(&package, &six, &sum("Hundred", "q", name("x")))
            .unwrap_err()
            .cause,
        CheckCause::Unproved(Obligation::Range { .. })
    ));
    // A non-integer total domain is ill typed.
    assert_eq!(
        ill_typed(&package, &five, &sum("Flag", "q", name("x"))),
        IllTypedCause::TypeMismatch
    );
    // A summand of a unit-carrying type does not fit a unitless total.
    let metres = [(
        "q",
        of(
            CollectionKind::Sequence,
            ValueType::Quantity(UnitId::declared(key("metre"))),
            0,
            5,
        ),
    )];
    assert_eq!(
        ill_typed(&package, &metres, &sum("Hundred", "q", name("x"))),
        IllTypedCause::TypeMismatch
    );
    // Six supplied items do not form a collection of cardinality at most 5.
    let source = amounts();
    let ValueType::Collection(five_type) = &source else {
        panic!("a collection type")
    };
    let formed = quire_exact::form_collection(
        five_type,
        ints(&[1, 1, 1, 1, 1, 1]),
        &mut Meter::new(UNLIMITED),
    )
    .unwrap();
    let Outcome::Refused(Refusal::CardinalityOutOfBound {
        violation,
        kind,
        bound,
        count,
    }) = formed
    else {
        panic!("six items refuse the cardinality bound, not {formed:?}");
    };
    assert_eq!(violation, BoundViolation::AboveMaximum);
    assert_eq!(kind, CollectionKind::Sequence);
    assert_eq!(Some(bound), five_type.bound());
    assert_eq!(count, 6);
}

#[trace("QSpec-TC-041", "QSpec-FR-041-AC-4")]
#[test]
fn fr041_decisive_forms_stop_and_the_rest_reach_every_occurrence() {
    let work = |expression: &Expression, values: &[i64]| run_amounts(expression, values).work;
    let exists = query(BinderQuery::Exists, "q", is_two());
    let forall = query(
        BinderQuery::Forall,
        "q",
        binary(BinaryOperator::Greater, name("x"), literal(2)),
    );
    let contains = Expression::contains(name("q"), literal(2));
    for stopping in [&exists, &contains] {
        // The decisive occurrence is first: later occurrences cost nothing.
        assert_eq!(work(stopping, &[2, 3, 3]), work(stopping, &[2]));
        assert!(work(stopping, &[3, 3, 2]) > work(stopping, &[2, 3, 3]));
    }
    // `forall` stops at its first false occurrence.
    assert_eq!(work(&forall, &[2, 3, 3]), work(&forall, &[2]));
    assert!(work(&forall, &[3, 3, 2]) > work(&forall, &[2, 3, 3]));
    // filter, map, count and sum retain every reached occurrence, duplicates
    // included, each read through its own binding of `x`.
    let values = [2, 2, 3];
    assert_elements(
        &run_amounts(&query(BinderQuery::Filter, "q", is_two()), &values),
        &[2, 2],
    );
    assert_elements(
        &run_amounts(
            &query(
                BinderQuery::Map,
                "q",
                binary(BinaryOperator::Multiply, name("x"), literal(10)),
            ),
            &values,
        ),
        &[20, 20, 30],
    );
    assert_eq!(
        integer_of(completed(&run_amounts(
            &count("Tiny", "q", is_two()),
            &values
        ))),
        Integer::from(2_i64)
    );
    assert_eq!(
        integer_of(completed(&run_amounts(
            &sum("Hundred", "q", name("x")),
            &values
        ))),
        Integer::from(7_i64)
    );
    // filter, map, count and sum visit every occurrence they reach.
    let reaching = [
        query(BinderQuery::Filter, "q", is_two()),
        query(BinderQuery::Map, "q", name("x")),
        count("Tiny", "q", is_two()),
        sum("Hundred", "q", name("x")),
    ];
    for expression in &reaching {
        assert!(work(expression, &[2, 2, 3]) > work(expression, &[2, 2]));
        assert!(work(expression, &[2, 2]) > work(expression, &[2]));
    }
}

/// An `Incomplete` outcome of a query's domain reaches the enclosing query,
/// aggregate and membership test: none reads it as an empty domain, and none
/// completes.
#[trace("QSpec-TC-041", "QSpec-FR-041-AC-5")]
#[test]
fn fr041_an_incomplete_domain_is_never_read_as_empty_or_aggregated() {
    let package = plain();
    let source = amounts();
    let parameters = [("q", source.clone())];
    let domain = || query(BinderQuery::Filter, "q", is_two());
    let declared = |name: &str| qsl_forms::DeclaredName {
        name: name.to_owned(),
        span: qsl_foundation::Span { start: 0, end: 0 },
    };
    let over_domain = [
        // An empty domain would make `forall` true and `exists` false.
        Expression::query(
            BinderQuery::Forall,
            "x".to_owned(),
            domain(),
            binary(BinaryOperator::Equal, literal(1), literal(1)),
        ),
        Expression::query(
            BinderQuery::Exists,
            "x".to_owned(),
            domain(),
            binary(BinaryOperator::Equal, literal(1), literal(1)),
        ),
        // Unknown membership is not `false`.
        Expression::contains(domain(), literal(2)),
        // Zero is the aggregate of an empty domain.
        Expression::size(domain()),
        Expression::count(
            declared("Tiny"),
            "x".to_owned(),
            domain(),
            binary(BinaryOperator::Equal, literal(1), literal(1)),
        ),
        Expression::sum(declared("Hundred"), "x".to_owned(), domain(), name("x")),
    ];
    let arguments = || vec![collection(&source, ints(&[2, 2, 3]))];
    // The least work that forms the whole domain.
    let domain_work = run(&package, &parameters, &domain(), arguments(), UNLIMITED).work;
    assert!(domain_work > 0);
    for expression in &over_domain {
        // With enough work the expression completes.
        let whole = run(&package, &parameters, expression, arguments(), UNLIMITED);
        assert!(matches!(whole.outcome, Outcome::Completed(_)));
        // With too little work to form its domain it stays incomplete.
        for limit in 0..domain_work {
            let stopped = run(
                &package,
                &parameters,
                expression,
                arguments(),
                work_limit(limit),
            );
            assert!(
                matches!(stopped.outcome, Outcome::Incomplete(_)),
                "limit {limit}: {:?}",
                stopped.outcome
            );
        }
    }
}

// QSpec FR-142 "Carried quantity sum" (TC-187 U46 to U50), run directly
// through the kernel: a linked sum of this length is not provable, so the
// fold, its accounting and the empty sum are driven unlinked.

fn integer_carrier(lower: i64, upper: i64) -> QuantityCarrier {
    QuantityCarrier::Integer {
        domain: IntegerInterval::new(Integer::from(lower), Integer::from(upper)).unwrap(),
        rounding: RoundingMode::Exact,
    }
}

fn decimal_carrier(rounding: RoundingMode) -> QuantityCarrier {
    QuantityCarrier::Decimal(
        DecimalType::new(
            Integer::from(-500_i64),
            Integer::from(500_i64),
            2,
            2,
            rounding,
        )
        .unwrap(),
    )
}

fn rational_carrier() -> QuantityCarrier {
    let interval = |lower: i64, upper: i64| {
        IntegerInterval::new(Integer::from(lower), Integer::from(upper)).unwrap()
    };
    QuantityCarrier::Rational(RationalDomain::new(interval(-3, 7), interval(1, 12)).unwrap())
}

fn carried(carrier: QuantityCarrier) -> ValueType {
    ValueType::carried_quantity(CarriedQuantityType::new(
        UnitId::declared(key("metre")),
        carrier,
    ))
}

fn metres(numerator: i64, denominator: i64) -> Value {
    Value::Quantity(Quantity::new(
        Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap(),
        UnitId::declared(key("metre")),
    ))
}

struct CarriedRun {
    outcome: FamilyOutcome<Value>,
    location: Option<Location>,
    sum: Location,
    meter: Meter,
}

/// `sum<alias>(x in q: x)` over `values` (numerator, denominator pairs), under
/// `limits`.
fn run_carried(alias: &str, values: &[(i64, i64)], limits: ScalarLimits) -> CarriedRun {
    let package = plain();
    let (_, element) = aliases()
        .into_iter()
        .find(|(name, _)| name == alias)
        .expect("the carried alias");
    let source = of(CollectionKind::Sequence, element, 0, 4);
    let checked = check(
        &package,
        &[("q", source.clone())],
        &sum(alias, "q", name("x")),
    )
    .unwrap();
    let values_debug = values.to_vec();
    let values = values.iter().map(|(n, d)| metres(*n, *d)).collect();
    let ValueType::Collection(source_type) = &source else {
        panic!("a collection type");
    };
    // The kernel's constructor does not admit a carried element; `form` is the
    // one the evaluator builds its sequences with.
    let argument = quire_exact::form(source_type, values, &mut Meter::new(UNLIMITED))
        .completed()
        .expect("the carried sequence forms");
    let mut meter = Meter::new(limits);
    let evaluation = package
        .evaluate(
            &checked,
            vec![argument],
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .unwrap_or_else(|failure| panic!("{alias} over {values_debug:?}: {failure:?}"));
    CarriedRun {
        outcome: evaluation.outcome,
        location: evaluation.location,
        sum: checked.root().location().clone(),
        meter,
    }
}

fn completed_quantity(run: &CarriedRun) -> &Rational {
    match &run.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Quantity(quantity))) => {
            quantity.magnitude()
        }
        other => panic!("a completed quantity, not {other:?}"),
    }
}

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
}

/// FR-142-AC-34 (TC-187 U46): `2 m, -1 m, 3 m` over `Li` is `4 m` after
/// fourteen work units and three result units; thirteen work units stop it
/// at `collection.result-retain` and three integer bits at the first
/// addition's `unit.rational-arithmetic`.
#[trace("QSpec-TC-187", "QSpec-FR-142-AC-34")]
#[trace("FR-517-AC-20", "TC-1312")]
#[test]
fn a_carried_fold_adds_and_charges_the_unit_addition_schedule() {
    let values = [(2, 1), (-1, 1), (3, 1)];
    let run = run_carried("Li", &values, UNLIMITED);
    assert_eq!(completed_quantity(&run), &ratio(4, 1));
    assert_eq!(run.meter.consumed(LimitKind::WorkUnits), 14);
    assert_eq!(run.meter.consumed(LimitKind::ResultUnits), 3);

    let limited = |limits: ScalarLimits| match run_carried("Li", &values, limits).outcome {
        FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) => incomplete,
        other => panic!("an incomplete outcome, not {other:?}"),
    };
    let work = limited(ScalarLimits {
        work_units: 13,
        ..UNLIMITED
    });
    assert_eq!(
        (work.limit_kind, work.consumed, work.charge_point),
        (
            LimitKind::WorkUnits,
            13,
            ChargePoint::CollectionResultRetain
        )
    );
    let bits = limited(ScalarLimits {
        integer_bits: 3,
        ..UNLIMITED
    });
    assert_eq!(
        (bits.limit_kind, bits.consumed, bits.charge_point),
        (
            LimitKind::IntegerBits,
            2,
            ChargePoint::UnitRationalArithmetic
        )
    );
}

/// FR-142-AC-35 (TC-187 U47): `3 m, 3 m, -2 m` is undefined at the second
/// addition after six work units, with no result unit, although the final
/// `4 m` is inside; `5 m, 0 m` and `-3 m, -2 m` are `5 m` and `-5 m`.
#[trace("QSpec-TC-187", "QSpec-FR-142-AC-35")]
#[trace("FR-517-AC-20", "TC-1312")]
#[test]
fn a_carried_total_outside_the_carrier_is_undefined_at_its_addition() {
    let run = run_carried("Li", &[(3, 1), (3, 1), (-2, 1)], UNLIMITED);
    assert!(
        matches!(
            run.outcome,
            FamilyOutcome::Evaluated(Outcome::Undefined(Undefined::SumOutOfDomain))
        ),
        "{:?}",
        run.outcome
    );
    assert_eq!(run.location.as_ref(), Some(&run.sum));
    assert_eq!(run.meter.consumed(LimitKind::WorkUnits), 6);
    assert_eq!(run.meter.consumed(LimitKind::ResultUnits), 0);
    assert_eq!(
        completed_quantity(&run_carried("Li", &[(5, 1), (0, 1)], UNLIMITED)),
        &ratio(5, 1)
    );
    assert_eq!(
        completed_quantity(&run_carried("Li", &[(-3, 1), (-2, 1)], UNLIMITED)),
        &ratio(-5, 1)
    );
}

/// FR-142-AC-36 (TC-187 U48): an empty `Li` sum is `0 m` after one scalar
/// retain; an empty sum over `Int[1, 5]` is undefined with no charge.
#[trace("QSpec-TC-187", "QSpec-FR-142-AC-36")]
#[trace("FR-517-AC-20", "TC-1312")]
#[test]
fn an_empty_carried_sum_is_the_carriers_zero_or_undefined() {
    let run = run_carried("Li", &[], UNLIMITED);
    assert_eq!(completed_quantity(&run), &ratio(0, 1));
    assert_eq!(run.meter.consumed(LimitKind::WorkUnits), 1);
    assert_eq!(run.meter.consumed(LimitKind::ResultUnits), 1);
    let run = run_carried("Lp", &[], UNLIMITED);
    assert!(
        matches!(
            run.outcome,
            FamilyOutcome::Evaluated(Outcome::Undefined(Undefined::SumOutOfDomain))
        ),
        "{:?}",
        run.outcome
    );
    assert_eq!(run.meter.consumed(LimitKind::WorkUnits), 0);
}

/// FR-142-AC-37 (TC-187 U49): a decimal carrier's fold never rounds, under
/// either mode; `4.00 m + 2.00 m` is undefined; the empty sum is zero.
#[trace("QSpec-TC-187", "QSpec-FR-142-AC-37")]
#[trace("FR-517-AC-20", "TC-1312")]
#[test]
fn a_carried_decimal_fold_never_rounds() {
    for alias in ["Ld", "Lx"] {
        let run = run_carried(alias, &[(5, 4), (5, 2)], UNLIMITED);
        assert_eq!(completed_quantity(&run), &ratio(15, 4), "{alias}");
        assert_eq!(run.meter.consumed(LimitKind::WorkUnits), 8, "{alias}");
        assert_eq!(run.meter.consumed(LimitKind::ResultUnits), 2, "{alias}");
        let run = run_carried(alias, &[(4, 1), (2, 1)], UNLIMITED);
        assert!(
            matches!(
                run.outcome,
                FamilyOutcome::Evaluated(Outcome::Undefined(Undefined::SumOutOfDomain))
            ),
            "{alias}: {:?}",
            run.outcome
        );
        assert_eq!(
            completed_quantity(&run_carried(alias, &[], UNLIMITED)),
            &ratio(0, 1),
            "{alias}"
        );
    }
}

/// FR-142-AC-38 (TC-187 U50): the empty sum over a rational carrier is `0 m`.
/// The fold itself (`1/4 m + 1/3 m`, the totals outside the bounds) is run
/// over the carrier by `carried_sum::tests`: the kernel's argument admission
/// refuses every non-empty `Sequence` of a rational-carried quantity as
/// `WrongValueKind`, so no such sequence reaches the evaluator from here.
#[trace("QSpec-TC-187", "QSpec-FR-142-AC-38")]
#[trace("FR-517-AC-20", "TC-1312")]
#[test]
fn an_empty_carried_rational_sum_is_zero() {
    let run = run_carried("Lr", &[], UNLIMITED);
    assert_eq!(completed_quantity(&run), &ratio(0, 1));
    assert_eq!(run.meter.consumed(LimitKind::WorkUnits), 1);
}
