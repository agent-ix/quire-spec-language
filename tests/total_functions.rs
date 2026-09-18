// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-191 total pure function checking over the real `value` checker and
//! evaluator boundary (FR-146).

use ix_trace_rs::trace;
use quire_spec_language::value::{
    Accumulation, BinaryOperator, CardinalityBound, CatalogRole, ChargePoint, CheckCause,
    CheckMode, CheckRefusal, CheckedPackage, CheckingLimitKind, CheckingLimits, CheckingStage,
    CollectionKind, CollectionType, CompositeDeclaration, CompositeShape, DefinitionLock,
    Expression, FieldDeclaration, FieldValue, FunctionDeclaration, IeeeValue, IeeeWidth,
    IllTypedCause, Incomplete, Integer, IntegerInterval, LimitKind, MeasureObligation, Meter,
    NodeKey, ObjectEnvironment, ObjectIdentity, ObjectReference, ObjectTypeDeclaration, Obligation,
    OptionValue, Origin, Outcome, PackageDeclarations, Presence, ProvedInterval, Rational,
    RationalDomain, Refusal, ScalarLimits, TypeEnvironment, Undefined, UniverseIdentity, Value,
    ValueType,
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

fn integer(value: i64) -> Integer {
    Integer::from(value)
}

fn int(value: i64) -> Value {
    Value::Integer(integer(value))
}

fn interval(lower: i64, upper: i64) -> IntegerInterval {
    IntegerInterval::new(integer(lower), integer(upper)).unwrap()
}

fn int_type(lower: i64, upper: i64) -> ValueType {
    ValueType::Int(interval(lower, upper))
}

fn quotient_type() -> ValueType {
    ValueType::Rational(RationalDomain::new(interval(-9, 9), interval(1, 9)).unwrap())
}

fn sequence(minimum: u64, maximum: u64) -> ValueType {
    ValueType::collection(CollectionType::new(
        CollectionKind::Sequence,
        ValueType::Integer,
        CardinalityBound::new(minimum, maximum).unwrap(),
    ))
}

fn name(spelling: &str) -> Expression {
    Expression::Name(spelling.to_owned())
}

fn literal(value: i64) -> Expression {
    Expression::Integer(integer(value))
}

fn binary(operator: BinaryOperator, left: Expression, right: Expression) -> Expression {
    Expression::Binary {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn call(target: &str, arguments: Vec<Expression>) -> Expression {
    Expression::Call {
        name: target.to_owned(),
        arguments,
    }
}

fn if_then(condition: Expression, then: Expression, otherwise: Expression) -> Expression {
    Expression::If {
        condition: Box::new(condition),
        then: Box::new(then),
        otherwise: Box::new(otherwise),
    }
}

fn field(operand: Expression, spelling: &str) -> Expression {
    Expression::Field {
        operand: Box::new(operand),
        field: spelling.to_owned(),
    }
}

fn present(operand: Expression) -> Expression {
    Expression::Present(Box::new(operand))
}

fn value(operand: Expression) -> Expression {
    Expression::Value(Box::new(operand))
}

fn function(
    spelling: &str,
    parameters: &[(&str, ValueType)],
    result: ValueType,
    measure: Option<Expression>,
    body: Expression,
) -> FunctionDeclaration {
    FunctionDeclaration::new(
        spelling.to_owned(),
        parameters
            .iter()
            .map(|(name, value_type)| ((*name).to_owned(), value_type.clone()))
            .collect(),
        result,
        measure,
        body,
    )
}

fn declarations(
    types: TypeEnvironment,
    functions: Vec<FunctionDeclaration>,
) -> PackageDeclarations {
    PackageDeclarations {
        types,
        aliases: vec![("Total".to_owned(), ValueType::Integer)],
        functions,
        ..PackageDeclarations::default()
    }
}

fn check(functions: Vec<FunctionDeclaration>) -> Result<CheckedPackage, Vec<CheckRefusal>> {
    declarations(TypeEnvironment::default(), functions).check(CheckingLimits::default())
}

/// The single refusal of a package check.
fn refusal(result: Result<CheckedPackage, Vec<CheckRefusal>>) -> CheckRefusal {
    match result {
        Err(mut refusals) if refusals.len() == 1 => refusals.remove(0),
        Err(refusals) => panic!("one refusal, not {refusals:?}"),
        Ok(_) => panic!("a refusal, not an admitted package"),
    }
}

fn unproved_decrease(
    result: Result<CheckedPackage, Vec<CheckRefusal>>,
) -> (Vec<String>, MeasureObligation) {
    let refusal = refusal(result);
    assert_eq!(refusal.cause.code().as_str(), "undefined_expression");
    assert_eq!(refusal.cause.cause(), Some("unproved-decrease"));
    match refusal.cause {
        CheckCause::UnprovedDecrease { cycle, obligation } => (cycle, obligation),
        other => panic!("unproved-decrease, not {other:?}"),
    }
}

fn names(spellings: &[&str]) -> Vec<String> {
    spellings
        .iter()
        .map(|spelling| (*spelling).to_owned())
        .collect()
}

fn node_environment() -> TypeEnvironment {
    TypeEnvironment::new(
        [CompositeDeclaration::new(
            key("Node"),
            "Node",
            CompositeShape::Record(vec![
                FieldDeclaration::new("head", ValueType::Integer, Presence::Required),
                FieldDeclaration::new(
                    "next",
                    ValueType::option(ValueType::Composite(key("Node"))),
                    Presence::Required,
                ),
            ]),
        )],
        [],
    )
    .unwrap()
}

fn last() -> FunctionDeclaration {
    let next = || field(name("n"), "next");
    function(
        "last",
        &[("n", ValueType::Composite(key("Node")))],
        ValueType::Integer,
        Some(name("n")),
        if_then(
            present(next()),
            call("last", vec![value(next())]),
            field(name("n"), "head"),
        ),
    )
}

#[trace("TC-191", "FR-146-AC-1")]
#[test]
fn p01_projection_under_a_presence_guard_decreases_structurally() {
    declarations(node_environment(), vec![last()])
        .check(CheckingLimits::default())
        .unwrap();
}

fn down_body(decrement: i64) -> Expression {
    if_then(
        binary(BinaryOperator::Greater, name("n"), literal(decrement - 1)),
        call(
            "down",
            vec![binary(
                BinaryOperator::Subtract,
                name("n"),
                literal(decrement),
            )],
        ),
        literal(0),
    )
}

fn down() -> FunctionDeclaration {
    function(
        "down",
        &[("n", int_type(0, 9))],
        ValueType::Integer,
        Some(name("n")),
        down_body(1),
    )
}

#[trace("TC-191", "FR-146-AC-2")]
#[trace("TC-191", "FR-146-AC-7")]
#[test]
fn p02_self_call_without_a_decrease_or_a_measure_is_refused() {
    let looping = |measure| {
        function(
            "loop",
            &[("n", int_type(0, 9))],
            ValueType::Integer,
            measure,
            call("loop", vec![name("n")]),
        )
    };
    let refused = refusal(check(vec![looping(Some(name("n")))]));
    assert_eq!(
        refused.location.origin,
        Origin::Body {
            function: "loop".to_owned(),
            index: 0
        }
    );
    assert!(refused.location.path.is_empty());
    assert_eq!(
        unproved_decrease(check(vec![looping(Some(name("n")))])),
        (names(&["loop", "loop"]), MeasureObligation::Decrease)
    );
    assert_eq!(
        unproved_decrease(check(vec![looping(None)])),
        (names(&["loop", "loop"]), MeasureObligation::MissingMeasure)
    );
}

fn parity(own: &str, other: &str, measure: Expression) -> FunctionDeclaration {
    function(
        own,
        &[("n", int_type(0, 9))],
        ValueType::Boolean,
        Some(measure),
        if_then(
            binary(BinaryOperator::Equal, name("n"), literal(0)),
            Expression::Boolean(true),
            call(
                other,
                vec![binary(BinaryOperator::Subtract, name("n"), literal(1))],
            ),
        ),
    )
}

#[trace("TC-191", "FR-146-AC-4")]
#[trace("TC-191", "FR-146-AC-7")]
#[test]
fn p03_mutual_recursion_needs_measures_of_one_arity() {
    check(vec![
        parity("even", "odd", name("n")),
        parity("odd", "even", name("n")),
    ])
    .unwrap();

    let types = TypeEnvironment::new(
        [CompositeDeclaration::new(
            key("M2"),
            "M2",
            CompositeShape::Tuple(vec![ValueType::Integer, ValueType::Integer]),
        )],
        [],
    )
    .unwrap();
    let result = declarations(
        types,
        vec![
            parity("even", "odd", name("n")),
            parity("odd", "even", call("M2", vec![name("n"), literal(0)])),
        ],
    )
    .check(CheckingLimits::default());
    assert_eq!(
        unproved_decrease(result),
        (
            names(&["even", "odd", "even"]),
            MeasureObligation::MeasureArity
        )
    );
}

fn quotient(body: Expression) -> FunctionDeclaration {
    function(
        "q",
        &[("a", int_type(-9, 9)), ("b", int_type(-9, 9))],
        quotient_type(),
        None,
        body,
    )
}

fn divide() -> Expression {
    binary(BinaryOperator::Divide, name("a"), name("b"))
}

fn zero_quotient() -> Expression {
    Expression::Rational(integer(0), integer(1))
}

fn unproved(result: Result<CheckedPackage, Vec<CheckRefusal>>) -> (Obligation, &'static str) {
    let refusal = refusal(result);
    assert_eq!(refusal.cause.code().as_str(), "undefined_expression");
    let cause = refusal.cause.cause().unwrap();
    match refusal.cause {
        CheckCause::Unproved(obligation) => (obligation, cause),
        other => panic!("an unproved obligation, not {other:?}"),
    }
}

fn reduce_total(source: &str) -> Expression {
    Expression::Accumulate {
        form: Accumulation::Reduce,
        accumulator_type: "Total".to_owned(),
        accumulator: "acc".to_owned(),
        binder: "x".to_owned(),
        source: Box::new(name(source)),
        step: Box::new(binary(BinaryOperator::Add, name("acc"), name("x"))),
        identity: None,
    }
}

#[trace("TC-191", "FR-146-AC-6")]
#[trace("TC-191", "FR-146-AC-8")]
#[test]
fn p04_division_presence_and_reduction_need_static_proofs() {
    assert_eq!(
        unproved(check(vec![quotient(divide())])),
        (Obligation::Nonzero, "unproved-nonzero")
    );
    check(vec![quotient(if_then(
        binary(BinaryOperator::NotEqual, name("b"), literal(0)),
        divide(),
        zero_quotient(),
    ))])
    .unwrap();

    let unguarded = function(
        "v",
        &[("o", ValueType::option(ValueType::Integer))],
        ValueType::Integer,
        None,
        value(name("o")),
    );
    assert_eq!(
        unproved(check(vec![unguarded])),
        (Obligation::Presence, "unproved-presence")
    );

    let reducing = |minimum, body| {
        function(
            "total",
            &[("s", sequence(minimum, 3))],
            ValueType::Integer,
            None,
            body,
        )
    };
    assert_eq!(
        unproved(check(vec![reducing(0, reduce_total("s"))])),
        (
            Obligation::NonemptyReduction {
                size: Box::new(ProvedInterval {
                    lower: Some(integer(0)),
                    upper: Some(integer(3)),
                }),
            },
            "unproved-range"
        )
    );
    check(vec![reducing(1, reduce_total("s"))]).unwrap();
    check(vec![reducing(
        0,
        if_then(
            binary(
                BinaryOperator::GreaterOrEqual,
                Expression::Size(Box::new(name("s"))),
                literal(1),
            ),
            reduce_total("s"),
            literal(0),
        ),
    )])
    .unwrap();
}

#[trace("TC-191", "FR-146-AC-3")]
#[trace("TC-191", "FR-146-AC-7")]
#[test]
fn p05_unreachable_calls_still_resolve_and_model_operations_are_ineligible() {
    let calling = |target: &str| {
        function(
            "r",
            &[("x", ValueType::Integer)],
            ValueType::Integer,
            None,
            if_then(
                Expression::Boolean(true),
                literal(1),
                call(target, vec![name("x")]),
            ),
        )
    };
    let missing = refusal(check(vec![calling("undeclaredHost")]));
    assert_eq!(missing.cause.code().as_str(), "missing_declaration");
    assert_eq!(missing.cause.cause(), Some("missing-name"));
    assert_eq!(missing.location.path, vec![2]);

    let model = PackageDeclarations {
        model_operations: vec!["M::pay".to_owned()],
        functions: vec![calling("M::pay")],
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::default());
    let ineligible = refusal(model);
    assert_eq!(
        ineligible.cause,
        CheckCause::IllTyped(IllTypedCause::OperatorIneligible)
    );
    assert_eq!(ineligible.location.path, vec![2]);
}

fn chain(types: &TypeEnvironment, heads: &[i64]) -> Value {
    let node_type = ValueType::Composite(key("Node"));
    let mut next = OptionValue::none(node_type.clone());
    for head in heads.iter().rev() {
        let node = types
            .record(
                key("Node"),
                vec![
                    ("head", FieldValue::Present(int(*head))),
                    ("next", FieldValue::Present(next)),
                ],
            )
            .unwrap();
        next = OptionValue::present(node_type.clone(), node).unwrap();
    }
    match next {
        Value::Option(option) => option.payload().cloned().unwrap(),
        other => panic!("an option, not {other:?}"),
    }
}

#[trace("TC-191", "FR-146-AC-5")]
#[trace("TC-191", "FR-146-AC-6")]
#[test]
fn p06_each_call_charges_function_call() {
    let types = node_environment();
    let package = declarations(types.clone(), vec![last()])
        .check(CheckingLimits::default())
        .unwrap();
    let objects = ObjectEnvironment::default();
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = package
        .call(
            "last",
            vec![chain(&types, &[1, 2, 3])],
            &objects,
            &mut meter,
        )
        .unwrap();
    assert_eq!(
        format!("{:?}", evaluation.outcome),
        format!("{:?}", Outcome::Completed(int(3)))
    );
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 3);

    let mut meter = Meter::new(work_limit(2));
    let evaluation = package
        .call(
            "last",
            vec![chain(&types, &[1, 2, 3])],
            &objects,
            &mut meter,
        )
        .unwrap();
    assert_eq!(
        format!("{:?}", evaluation.outcome),
        format!(
            "{:?}",
            Outcome::<Value>::Incomplete(Incomplete {
                limit_kind: LimitKind::WorkUnits,
                limit: 2,
                consumed: 2,
                next_charge: integer(1),
                charge_point: ChargePoint::FunctionCall,
            })
        )
    );
}

#[trace("TC-191", "FR-146-AC-7")]
#[test]
fn p07_nonrecursive_functions_need_no_measure() {
    let double = |measure| {
        function(
            "double",
            &[("n", ValueType::Integer)],
            ValueType::Integer,
            measure,
            binary(BinaryOperator::Add, name("n"), name("n")),
        )
    };
    let quad = function(
        "quad",
        &[("n", int_type(0, 9))],
        ValueType::Integer,
        None,
        call("double", vec![call("double", vec![name("n")])]),
    );
    check(vec![double(None), quad.clone()]).unwrap();
    check(vec![double(Some(name("n"))), quad]).unwrap();
}

#[trace("TC-191", "FR-146-AC-7")]
#[test]
fn p07_a_call_interval_is_its_declared_result_so_a_ranged_nesting_is_unproved() {
    let double = function(
        "double",
        &[("n", int_type(0, 9))],
        ValueType::Integer,
        None,
        binary(BinaryOperator::Add, name("n"), name("n")),
    );
    let quad = function(
        "quad",
        &[("n", int_type(0, 9))],
        ValueType::Integer,
        None,
        call("double", vec![call("double", vec![name("n")])]),
    );
    let refused = refusal(check(vec![double, quad]));
    assert_eq!(refused.cause.cause(), Some("unproved-range"));
    assert_eq!(
        refused.location.origin,
        Origin::Body {
            function: "quad".to_owned(),
            index: 1
        }
    );
}

#[trace("TC-191", "FR-146-AC-2")]
#[trace("TC-191", "FR-146-AC-7")]
#[trace("TC-191", "FR-146-AC-8")]
#[test]
fn p08_measures_decrease_lexicographically_by_accepted_forms() {
    check(vec![down()]).unwrap();
    check(vec![function(
        "down",
        &[("n", int_type(0, 9))],
        ValueType::Integer,
        Some(name("n")),
        down_body(2),
    )])
    .unwrap();

    let shifted = function(
        "down",
        &[("n", int_type(0, 9))],
        ValueType::Integer,
        Some(binary(BinaryOperator::Add, name("n"), literal(0))),
        down_body(1),
    );
    assert_eq!(
        unproved_decrease(check(vec![shifted])).1,
        MeasureObligation::MeasureKind
    );
    let unbounded = function(
        "down",
        &[("n", ValueType::Integer)],
        ValueType::Integer,
        Some(name("n")),
        down_body(1),
    );
    assert_eq!(
        unproved_decrease(check(vec![unbounded])).1,
        MeasureObligation::Nonnegative
    );

    let two = function(
        "two",
        &[("n", int_type(0, 9)), ("m", int_type(0, 9))],
        ValueType::Integer,
        Some(name("n")),
        if_then(
            binary(BinaryOperator::Greater, name("m"), literal(0)),
            call(
                "two",
                vec![
                    binary(BinaryOperator::Subtract, name("m"), literal(1)),
                    binary(BinaryOperator::Subtract, name("m"), literal(1)),
                ],
            ),
            literal(0),
        ),
    );
    assert_eq!(
        unproved_decrease(check(vec![two])).1,
        MeasureObligation::Decrease
    );

    let size = || Expression::Size(Box::new(name("s")));
    let cardinality = function(
        "g",
        &[("s", sequence(0, 3))],
        ValueType::Integer,
        Some(size()),
        if_then(
            binary(BinaryOperator::Greater, size(), literal(0)),
            call("g", vec![name("s")]),
            literal(0),
        ),
    );
    assert_eq!(
        unproved_decrease(check(vec![cardinality])).1,
        MeasureObligation::Decrease
    );

    let types = TypeEnvironment::new(
        [CompositeDeclaration::new(
            key("P2"),
            "P2",
            CompositeShape::Tuple(vec![ValueType::Integer, ValueType::Integer]),
        )],
        [],
    )
    .unwrap();
    let lex = function(
        "lex",
        &[("a", int_type(0, 9)), ("b", int_type(0, 9))],
        ValueType::Integer,
        Some(call("P2", vec![name("a"), name("b")])),
        if_then(
            binary(BinaryOperator::Greater, name("b"), literal(0)),
            call(
                "lex",
                vec![
                    name("a"),
                    binary(BinaryOperator::Subtract, name("b"), literal(1)),
                ],
            ),
            if_then(
                binary(BinaryOperator::Greater, name("a"), literal(0)),
                call(
                    "lex",
                    vec![
                        binary(BinaryOperator::Subtract, name("a"), literal(1)),
                        literal(9),
                    ],
                ),
                literal(0),
            ),
        ),
    );
    declarations(types, vec![lex])
        .check(CheckingLimits::default())
        .unwrap();
}

#[trace("TC-191", "FR-146-AC-8")]
#[test]
fn p09_intervals_come_only_from_declared_types_and_literal_guards() {
    let body = |body| {
        function(
            "f",
            &[("a", int_type(0, 3)), ("b", int_type(0, 3))],
            int_type(0, 9),
            None,
            body,
        )
    };
    let product = || binary(BinaryOperator::Multiply, name("a"), name("b"));
    let successor = || binary(BinaryOperator::Add, product(), literal(1));
    check(vec![body(product())]).unwrap();
    let range = |lower, upper| {
        (
            Obligation::Range {
                required: Box::new(ProvedInterval {
                    lower: Some(integer(0)),
                    upper: Some(integer(9)),
                }),
                proved: Box::new(ProvedInterval {
                    lower: Some(integer(lower)),
                    upper: Some(integer(upper)),
                }),
            },
            "unproved-range",
        )
    };
    assert_eq!(unproved(check(vec![body(successor())])), range(1, 10));
    check(vec![body(if_then(
        binary(BinaryOperator::Less, name("a"), literal(3)),
        successor(),
        literal(0),
    ))])
    .unwrap();
    assert_eq!(
        unproved(check(vec![body(if_then(
            binary(BinaryOperator::Less, name("a"), name("b")),
            binary(BinaryOperator::Subtract, name("b"), name("a")),
            literal(0),
        ))])),
        range(-3, 3)
    );
    check(vec![body(binary(
        BinaryOperator::Add,
        Expression::Negate(Box::new(name("a"))),
        literal(3),
    ))])
    .unwrap();

    check(vec![quotient(if_then(
        binary(BinaryOperator::Greater, name("b"), literal(0)),
        divide(),
        zero_quotient(),
    ))])
    .unwrap();
    assert_eq!(
        unproved(check(vec![quotient(if_then(
            binary(BinaryOperator::GreaterOrEqual, name("b"), name("a")),
            divide(),
            zero_quotient(),
        ))])),
        (Obligation::Nonzero, "unproved-nonzero")
    );
}

fn ieee_profile() -> quire_spec_language::value::AdmittedIeeeProfile {
    let lock = DefinitionLock::pinned().unwrap();
    let reference = lock
        .entry(CatalogRole::IeeeProfile)
        .unwrap()
        .definition
        .clone();
    lock.admit_ieee_profile(&[reference], &[]).unwrap()
}

fn object() -> ObjectReference {
    ObjectReference::new(
        UniverseIdentity::new(b"u1").unwrap(),
        key("M::Obj"),
        ObjectIdentity::new(b"o1").unwrap(),
    )
}

fn box_environment() -> TypeEnvironment {
    TypeEnvironment::new(
        [CompositeDeclaration::new(
            key("Box"),
            "Box",
            CompositeShape::Record(vec![FieldDeclaration::new(
                "o",
                ValueType::option(ValueType::Integer),
                Presence::Required,
            )]),
        )],
        [ObjectTypeDeclaration::new(
            key("M::Obj"),
            "Obj",
            vec![FieldDeclaration::new(
                "n",
                ValueType::Integer,
                Presence::Required,
            )],
        )],
    )
    .unwrap()
}

#[trace("TC-191", "FR-146-AC-6")]
#[trace("TC-191", "FR-146-AC-8")]
#[test]
fn p10_stable_paths_ieee_conversion_references_duplicates_and_node_limits() {
    let types = box_environment();
    let pick = || {
        function(
            "pick",
            &[("x", ValueType::Composite(key("Box")))],
            ValueType::option(ValueType::Integer),
            None,
            field(name("x"), "o"),
        )
    };
    let package = |body: Expression| {
        PackageDeclarations {
            types: types.clone(),
            functions: vec![
                pick(),
                function(
                    "f",
                    &[
                        ("x", ValueType::Composite(key("Box"))),
                        ("f", ValueType::Float(IeeeWidth::Binary64)),
                        ("ro", ValueType::option(ValueType::Reference(key("M::Obj")))),
                    ],
                    ValueType::Integer,
                    None,
                    body,
                ),
            ],
            ieee_profile: Some(ieee_profile()),
            ..PackageDeclarations::default()
        }
        .check(CheckingLimits::default())
    };
    let guarded = |operand: &dyn Fn() -> Expression| {
        if_then(present(operand()), value(operand()), literal(0))
    };
    package(guarded(&|| field(name("x"), "o"))).unwrap();
    assert_eq!(
        unproved(package(guarded(&|| call("pick", vec![name("x")])))),
        (Obligation::Presence, "unproved-presence")
    );
    package(Expression::Let {
        name: "y".to_owned(),
        value: Box::new(call("pick", vec![name("x")])),
        body: Box::new(guarded(&|| name("y"))),
    })
    .unwrap();

    let admitted = package(literal(0)).unwrap();
    let converted = admitted
        .check_expression(
            vec![("f".to_owned(), ValueType::Float(IeeeWidth::Binary64))],
            &Expression::Convert {
                target: quotient_type(),
                operand: Box::new(name("f")),
            },
            None,
            CheckMode::Linked,
            CheckingLimits::default(),
        )
        .unwrap();
    let objects = ObjectEnvironment::new(
        &types,
        [(object(), vec![("n", FieldValue::Present(int(7)))])],
    )
    .unwrap();
    let convert = |bits: u64| {
        let mut meter = Meter::new(UNLIMITED);
        let evaluation = admitted
            .evaluate(
                &converted,
                vec![Value::Float(IeeeValue::binary64(bits))],
                &objects,
                &mut meter,
            )
            .unwrap();
        (evaluation.outcome, meter.admitted_charges().to_vec())
    };
    let (nan, charges) = convert(0x7FF8_0000_0000_0000);
    assert!(matches!(nan, Outcome::Undefined(Undefined::IeeeNotFinite)));
    assert_eq!(charges, vec![ChargePoint::IeeeOperands]);
    assert!(matches!(
        convert(0x4034_0000_0000_0000).0,
        Outcome::Refused(Refusal::IeeeRationalOutOfDomain)
    ));
    assert_eq!(
        format!("{:?}", convert(0x3FE0_0000_0000_0000).0),
        format!(
            "{:?}",
            Outcome::Completed(Value::Rational(
                Rational::new(integer(1), integer(2)).unwrap()
            ))
        )
    );

    let dereference = package(if_then(
        present(name("ro")),
        field(Expression::Deref(Box::new(value(name("ro")))), "n"),
        literal(0),
    ))
    .unwrap();
    assert_eq!(dereference.dereferences("f").unwrap().len(), 1);
    assert_eq!(
        refusal(
            package(field(Expression::Deref(Box::new(name("ro"))), "n"))
                .map(|_| { unreachable!() })
        )
        .cause,
        CheckCause::IllTyped(IllTypedCause::TypeMismatch)
    );

    let dup = || function("dup", &[], ValueType::Integer, None, literal(1));
    let duplicates = check(vec![dup(), dup()]).unwrap_err();
    assert_eq!(duplicates.len(), 2);
    for refusal in &duplicates {
        assert_eq!(refusal.cause.code().as_str(), "ambiguous_declaration");
        assert_eq!(refusal.cause.cause(), Some("ambiguous-name"));
        let CheckCause::AmbiguousName { loci, .. } = &refusal.cause else {
            panic!("ambiguous-name");
        };
        assert_eq!(loci.len(), 2);
    }

    let limited = PackageDeclarations {
        functions: vec![down()],
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::new(4, 128).unwrap());
    let exhausted = refusal(limited);
    assert_eq!(exhausted.cause.code().as_str(), "resource_exhausted");
    assert_eq!(exhausted.cause.cause(), Some("insufficient-next-charge"));
    assert_eq!(
        exhausted.cause,
        CheckCause::ResourceExhausted {
            stage: CheckingStage::Typing,
            kind: CheckingLimitKind::Nodes,
            limit: 4,
        }
    );
}

#[trace("TC-191", "FR-146-AC-5")]
#[trace("TC-191", "FR-146-AC-6")]
#[test]
fn p11_evaluation_charges_calls_orderings_arithmetic_and_skipped_operands() {
    let package = check(vec![
        down(),
        quotient(if_then(
            binary(BinaryOperator::NotEqual, name("b"), literal(0)),
            divide(),
            zero_quotient(),
        )),
    ])
    .unwrap();
    let objects = ObjectEnvironment::default();
    let invoke = |function: &str, arguments: Vec<Value>, limits| {
        let mut meter = Meter::new(limits);
        let evaluation = package
            .call(function, arguments, &objects, &mut meter)
            .unwrap();
        (
            format!("{:?}", evaluation.outcome),
            meter.consumed(LimitKind::WorkUnits),
            meter.consumed(LimitKind::ResultUnits),
        )
    };
    let incomplete = |limit: u64, charge_point| {
        format!(
            "{:?}",
            Outcome::<Value>::Incomplete(Incomplete {
                limit_kind: LimitKind::WorkUnits,
                limit,
                consumed: limit,
                next_charge: integer(1),
                charge_point,
            })
        )
    };

    assert_eq!(
        invoke("down", vec![int(2)], UNLIMITED),
        (format!("{:?}", Outcome::Completed(int(0))), 18, 5)
    );
    assert_eq!(
        invoke("down", vec![int(2)], work_limit(17)).0,
        incomplete(17, ChargePoint::OrderingResultRetain)
    );
    let half = Value::Rational(Rational::new(integer(3), integer(2)).unwrap());
    assert_eq!(
        invoke("q", vec![int(3), int(2)], UNLIMITED),
        (format!("{:?}", Outcome::Completed(half)), 10, 2)
    );
    // `rational-arithmetic.arithmetic` for `3/1` and `2/1`: `N = bits(3) +
    // bits(1) = 3`, `D = bits(1) + bits(2) = 3`, so `integer_bits` 3; the
    // operands (2) and normalize (2) stay below it.
    let mut meter = Meter::new(UNLIMITED);
    package
        .call("q", vec![int(3), int(2)], &objects, &mut meter)
        .unwrap();
    assert_eq!(meter.consumed(LimitKind::IntegerBits), 3);
    let mut narrow = Meter::new(ScalarLimits {
        integer_bits: 2,
        ..UNLIMITED
    });
    assert_eq!(
        format!(
            "{:?}",
            package
                .call("q", vec![int(3), int(2)], &objects, &mut narrow)
                .unwrap()
                .outcome
        ),
        format!(
            "{:?}",
            Outcome::<Value>::Incomplete(Incomplete {
                limit_kind: LimitKind::IntegerBits,
                limit: 2,
                consumed: 2,
                next_charge: integer(3),
                charge_point: ChargePoint::RationalArithmeticArithmetic,
            })
        )
    );
    assert_eq!(
        invoke("q", vec![int(3), int(2)], work_limit(9)).0,
        incomplete(9, ChargePoint::RationalArithmeticResultRetain)
    );

    let implication = binary(
        BinaryOperator::Implies,
        binary(BinaryOperator::Greater, name("a"), literal(0)),
        binary(BinaryOperator::Greater, name("b"), literal(0)),
    );
    let checked = package
        .check_expression(
            vec![
                ("a".to_owned(), ValueType::Integer),
                ("b".to_owned(), ValueType::Integer),
            ],
            &implication,
            None,
            CheckMode::Linked,
            CheckingLimits::default(),
        )
        .unwrap();
    for (a, work, results) in [(3, 7, 3), (-1, 4, 2)] {
        let mut meter = Meter::new(UNLIMITED);
        let evaluation = package
            .evaluate(&checked, vec![int(a), int(2)], &objects, &mut meter)
            .unwrap();
        assert_eq!(
            format!("{:?}", evaluation.outcome),
            format!("{:?}", Outcome::Completed(Value::Boolean(true)))
        );
        assert_eq!(
            (
                meter.consumed(LimitKind::WorkUnits),
                meter.consumed(LimitKind::ResultUnits)
            ),
            (work, results)
        );
    }
}
