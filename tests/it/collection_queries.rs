// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-190 collection query and conversion algebra over the real `value`
//! checker and evaluator boundary (FR-145).

use ix_trace_rs::trace;
use quire_exact::{
    CardinalityBound, ChargePoint, CollectionKind, Incomplete, Integer, IntegerInterval, LimitKind,
    Meter, ScalarLimits,
};
use quire_spec_language::value::{
    Accumulation, BinaryOperator, BinderQuery, BoundViolation, CheckCause, CheckMode, CheckRefusal,
    CheckedPackage, CheckingLimits, CollectionLoss, CollectionProperty, CollectionType,
    CompositeDeclaration, CompositeShape, Expression, FieldDeclaration, FieldValue,
    FunctionDeclaration, IllTypedCause, NodeKey, ObjectEnvironment, ObjectIdentity,
    ObjectReference, ObjectTypeDeclaration, Obligation, Outcome, PackageDeclarations, Presence,
    ProvedInterval, Refusal, TypeEnvironment, Undefined, UniverseIdentity, Value, ValueType,
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

fn work_limit(work_units: u64) -> ScalarLimits {
    ScalarLimits {
        work_units,
        ..UNLIMITED
    }
}

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
        CardinalityBound::new(minimum, maximum).unwrap(),
    )
}

fn of(kind: CollectionKind, element: ValueType, minimum: u64, maximum: u64) -> ValueType {
    ValueType::collection(collection_type(kind, element, minimum, maximum))
}

fn integers(kind: CollectionKind, minimum: u64, maximum: u64) -> ValueType {
    of(kind, ValueType::Integer, minimum, maximum)
}

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

fn query(query: BinderQuery, source: &str, body: Expression) -> Expression {
    Expression::Query {
        query,
        binder: "x".to_owned(),
        source: Box::new(name(source)),
        body: Box::new(body),
    }
}

fn accumulate(
    form: Accumulation,
    accumulator_type: &str,
    source: &str,
    step: Expression,
    identity: Option<Expression>,
) -> Expression {
    Expression::Accumulate {
        form,
        accumulator_type: accumulator_type.to_owned(),
        accumulator: "acc".to_owned(),
        binder: "x".to_owned(),
        source: Box::new(name(source)),
        step: Box::new(step),
        identity: identity.map(Box::new),
    }
}

fn convert(target: ValueType, operand: &str) -> Expression {
    Expression::Convert {
        target,
        operand: Box::new(name(operand)),
    }
}

fn aliases() -> Vec<(String, ValueType)> {
    vec![
        ("Total".to_owned(), ValueType::Integer),
        ("Small".to_owned(), int_type(0, 30)),
        ("Flag".to_owned(), ValueType::Boolean),
        ("Tiny".to_owned(), int_type(0, 5)),
    ]
}

fn package(types: TypeEnvironment, functions: Vec<FunctionDeclaration>) -> CheckedPackage {
    let graph = PackageDeclarations {
        types,
        aliases: aliases(),
        functions,
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::default())
    .unwrap();
    // ADR-013 T-1 (FR-087, QSL-158 S-3a): the S4 link step, over an empty
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
) -> Result<quire_spec_language::value::CheckedExpression, CheckRefusal> {
    let parameters = parameters
        .iter()
        .map(|(name, value_type)| ((*name).to_owned(), value_type.clone()))
        .collect();
    package.graph().check_expression(
        parameters,
        expression,
        None,
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
    quire_spec_language::value::form_collection(collection_type, values, &mut Meter::new(UNLIMITED))
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
    let mut meter = Meter::new(limits);
    let evaluation = package
        .evaluate(&checked, arguments, objects, &mut meter)
        .unwrap();
    Run {
        outcome: evaluation.outcome,
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

fn assert_elements(run: &Run, expected: &[i64]) {
    assert_eq!(
        format!("{:?}", elements(completed(run))),
        format!("{:?}", ints(expected))
    );
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

#[trace("TC-190", "FR-145-AC-1")]
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

#[trace("TC-190", "FR-145-AC-1")]
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

#[trace("TC-190", "FR-145-AC-4")]
#[trace("TC-190", "FR-145-AC-6")]
#[test]
fn q03_flatten_admits_kinds_and_derives_bounds() {
    let package = plain();
    let flatten = Expression::Flatten(Box::new(name("c")));
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

#[trace("TC-190", "FR-145-AC-2")]
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
            Outcome::Undefined(Undefined::EmptyReduction)
        ));
        assert!(evaluation.location.is_some());
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 0);
    }

    let linked = PackageDeclarations {
        aliases: aliases(),
        functions: vec![FunctionDeclaration::new(
            "r".to_owned(),
            vec![("e".to_owned(), integers(CollectionKind::Set, 0, 2))],
            ValueType::Integer,
            None,
            reduce,
        )],
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::default())
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
    assert!(refusal.location.path.is_empty());

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

#[trace("TC-190", "FR-145-AC-5")]
#[trace("TC-190", "FR-145-AC-9")]
#[test]
fn q05_set_and_bag_steps_must_be_in_the_syntactic_catalog() {
    let add = FunctionDeclaration::new(
        "add".to_owned(),
        vec![
            ("a".to_owned(), ValueType::Integer),
            ("b".to_owned(), ValueType::Integer),
        ],
        ValueType::Integer,
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
        Expression::Call {
            name: "add".to_owned(),
            arguments: vec![name("acc"), name("x")],
        },
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
                Expression::Boolean(true)
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
        Some(Expression::Boolean(true)),
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

#[trace("TC-190", "FR-145-AC-3")]
#[trace("TC-190", "FR-145-AC-7")]
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

#[trace("TC-190", "FR-145-AC-8")]
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

#[trace("TC-190", "FR-145-AC-8")]
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

#[trace("TC-190", "FR-145-AC-8")]
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
                ValueType::Reference(key("M::Obj")),
                Presence::Required,
            )]),
        )],
        [ObjectTypeDeclaration::new(key("M::Obj"), "Obj", vec![])],
    )
    .unwrap()
}

fn object(identity: &str) -> ObjectReference {
    ObjectReference::new(
        UniverseIdentity::new(b"u1").unwrap(),
        key("M::Obj"),
        ObjectIdentity::new(identity.as_bytes()).unwrap(),
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

#[trace("TC-190", "FR-145-AC-8")]
#[test]
fn q10_contains_stops_at_the_first_equal_member_and_size_only_retains() {
    let types = holder_environment();
    let objects = ObjectEnvironment::new(
        &types,
        [(object("h1"), Vec::new()), (object("h2"), Vec::new())],
    )
    .unwrap();
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
    let contains = |item: &str| Expression::Contains {
        collection: Box::new(name("hs")),
        item: Box::new(name(item)),
    };
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
        &Expression::Size(Box::new(name("hs"))),
        arguments(),
        UNLIMITED,
        &objects,
    );
    assert_eq!(format!("{:?}", completed(&size)), format!("{:?}", int(2)));
    assert_eq!((size.work, size.results), (1, 1));
}

#[trace("TC-190", "FR-145-AC-8")]
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
    Expression::Sum {
        result_type: result_type.to_owned(),
        binder: "x".to_owned(),
        source: Box::new(name(source)),
        summand: Box::new(summand),
    }
}

fn check_linked(
    package: &CheckedPackage,
    parameters: &[(&str, ValueType)],
    expression: &Expression,
) -> Result<quire_spec_language::value::CheckedExpression, CheckRefusal> {
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

#[trace("TC-190", "FR-145-AC-8")]
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

#[trace("TC-190", "FR-145-AC-6")]
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
    assert!(matches!(
        over.outcome,
        Outcome::Refused(Refusal::IntegerOutOfDomain)
    ));
    assert_eq!(
        ill_typed(&package, &[("q", sequence)], &sum("Flag", "q", name("x"))),
        IllTypedCause::TypeMismatch
    );
}
