// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-191 total pure function checking over the real `value` checker and
//! evaluator boundary (FR-146).

use ix_trace_rs::trace;
use qsl_forms::{Accumulation, BinaryOperator, Expression, FunctionDeclaration, TypeForm};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{
    CheckCause, CheckMode, CheckRefusal, CheckedGraph, CheckingLimitKind, CheckingLimits,
    CheckingStage, Location, MeasureObligation, Obligation, Origin, PackageDeclarations,
    ProvedInterval,
};
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::declaration::{
    CompositeDeclaration, CompositeShape, FieldDeclaration, ObjectTypeDeclaration, TypeEnvironment,
};
use qsl_semantics::value::{CatalogRole, DefinitionLock, DefinitionReference, DefinitionRevision};
use quire_exact::EffectiveId;
use quire_exact::NodeKey;
use quire_exact::{
    CardinalityBound, ChargePoint, CollectionKind, Incomplete, Integer, IntegerInterval, LimitKind,
    Meter, Outcome, Refusal, ScalarLimits, Undefined,
};
use quire_exact::{CollectionType, FieldValue, OptionValue, RationalDomain, Value, ValueType};
use quire_exact::{
    Decimal, DecimalType, IeeeValue, IeeeWidth, IllTypedCause, ObjectId, ObjectReference, Presence,
    Rational, RoundingMode, UniverseId,
};
use quire_spec_language::value::{
    CheckedPackageEvaluation, Evaluation, LocatedLoss, QualifiedName, ValueLoss,
};

use sha2::{Digest, Sha256};

/// FR-090: `CheckedPackage::call`/`evaluate` return `Evaluation { outcome:
/// FamilyOutcome, .. }` (FR-090-OQ-3, ruled option A); every fixture in this
/// file evaluates through the kernel path, so extracting `Evaluated`'s
/// outcome is all these tests need.
fn evaluated(evaluation: Evaluation) -> Outcome<Value> {
    match evaluation.outcome {
        FamilyOutcome::Evaluated(outcome) => outcome,
        other => panic!("expected FamilyOutcome::Evaluated(_), got {other:?}"),
    }
}

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
    NodeKey::from_digest(Sha256::digest(label.as_bytes()).into())
}

/// A model object type's effective-declaration identity (ADR-013 O-05): the
/// identity `Reference<T>` and `ObjectTypeDeclaration` carry.
fn object_type(label: &str) -> EffectiveId {
    EffectiveId::from_digest(Sha256::digest(label.as_bytes()).into())
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

/// `crate::support::type_form::type_form` can't recover a
/// declared composite/object type's own name from a bare `ValueType::
/// Composite`/`Reference` digest, so this file's own fixtures (whose only
/// three such parameter/result types are "Node", "Box" and the object type
/// registered at `object_type("M::Obj")` but declared as "Obj") map those back
/// explicitly instead.
fn param_type_form(value_type: &ValueType) -> TypeForm {
    match value_type {
        ValueType::Composite(k) if *k == key("Node") => {
            crate::support::type_form::named_type_form("Node")
        }
        ValueType::Composite(k) if *k == key("Box") => {
            crate::support::type_form::named_type_form("Box")
        }
        ValueType::Reference(k) if *k == object_type("M::Obj") => {
            crate::support::type_form::named_type_form("Obj")
        }
        ValueType::Option(payload) => TypeForm::builtin(
            qsl_forms::BuiltinType::Option,
            crate::support::type_form::SPAN,
        )
        .with_arguments(vec![param_type_form(payload)]),
        _ => crate::support::type_form::type_form(value_type),
    }
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
            .map(|(name, value_type)| ((*name).to_owned(), param_type_form(value_type)))
            .collect(),
        param_type_form(&result),
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

fn check(functions: Vec<FunctionDeclaration>) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    declarations(TypeEnvironment::default(), functions).check(CheckingLimits::default())
}

/// The single refusal of a package check.
fn refusal(result: Result<CheckedGraph, Vec<CheckRefusal>>) -> CheckRefusal {
    match result {
        Err(mut refusals) if refusals.len() == 1 => refusals.remove(0),
        Err(refusals) => panic!("one refusal, not {refusals:?}"),
        Ok(_) => panic!("a refusal, not an admitted package"),
    }
}

fn unproved_decrease(
    result: Result<CheckedGraph, Vec<CheckRefusal>>,
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

fn unproved(result: Result<CheckedGraph, Vec<CheckRefusal>>) -> (Obligation, &'static str) {
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
    let graph = declarations(types.clone(), vec![last()])
        .check(CheckingLimits::default())
        .unwrap();
    // ADR-013 T-1 (FR-087, QSL-158 S-3a): the S4 link step, over an empty
    // dependency closure -- this fixture declares no import.
    let package = CheckedPackage::link(graph);
    let objects = ObjectEnvironment::default();
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = package
        .call(
            &QualifiedName::unqualified("last").unwrap(),
            vec![chain(&types, &[1, 2, 3])],
            &objects,
            &mut meter,
        )
        .unwrap();
    assert_eq!(
        format!("{:?}", evaluated(evaluation)),
        format!("{:?}", Outcome::Completed(int(3)))
    );
    // PR #302 review finding 2: 2, not 3 -- one `function.call` charge per
    // *nested* self-call inside `last`'s own body (a 3-node chain recurses
    // twice before reaching the end of the chain), never a third charge for
    // the top-level call itself. `CheckedPackage::call` charges the
    // top-level call's own admission against a separate contract-level
    // meter now (`ValueFunctionFamily::evaluate`'s own doc), not against
    // this `meter` (`env.local_meter`) -- an earlier version charged
    // `function.call` against `local_meter` for the top-level call too,
    // restating the same named charge a second time for one logical call.
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 2);

    let mut meter = Meter::new(work_limit(1));
    let evaluation = package
        .call(
            &QualifiedName::unqualified("last").unwrap(),
            vec![chain(&types, &[1, 2, 3])],
            &objects,
            &mut meter,
        )
        .unwrap();
    assert_eq!(
        format!("{:?}", evaluated(evaluation)),
        format!(
            "{:?}",
            Outcome::<Value>::Incomplete(Incomplete {
                limit_kind: LimitKind::WorkUnits,
                limit: 1,
                consumed: 1,
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

fn ieee_profile() -> qsl_semantics::value::AdmittedIeeeProfile {
    let lock = DefinitionLock::pinned();
    let entry = lock.entry(CatalogRole::IeeeProfile).unwrap();
    let reference = DefinitionReference {
        authority: entry.authority.to_owned(),
        identity: entry.identity.to_owned(),
        revision: DefinitionRevision {
            namespace: entry.revision_namespace.to_owned(),
            value: entry.revision_value.to_owned(),
        },
        digest_domain: "quire.definition.bytes/v1".to_owned(),
        digest: "0".repeat(64),
    };
    lock.admit_ieee_profile(&[reference], &[]).unwrap()
}

fn object() -> ObjectReference {
    ObjectReference::new(
        UniverseId::from_digest(Sha256::digest(b"u1").into()),
        object_type("M::Obj"),
        ObjectId::new("o1").unwrap(),
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
            object_type("M::Obj"),
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
                        (
                            "ro",
                            ValueType::option(ValueType::Reference(object_type("M::Obj"))),
                        ),
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
                target: crate::support::type_form::type_form(&quotient_type()),
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
    // ADR-013 T-1 (FR-087, QSL-158 S-3a): the S4 link step, over an empty
    // dependency closure -- this fixture declares no import. `converted`
    // above is checked against `admitted` (S3, `CheckedGraph`) directly;
    // evaluation needs the S4 `CheckedPackage` `admitted` links into.
    let admitted = CheckedPackage::link(admitted);
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
        (evaluated(evaluation), meter.admitted_charges().to_vec())
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

/// PR #303 review round 3, finding F1: `CheckingLimits::new`'s own doc
/// ("at most `nodes` expression nodes per checked package") is a bound on
/// the whole package, not on any one declaration in it -- QSL-148's move of
/// typing into `ValueFunctionFamily::check` briefly reset the `Typer` node
/// counter to zero for every declaration (`check::family::
/// check_declaration_body`), so two declarations that each individually fit
/// comfortably under a small `nodes` budget were both admitted even though
/// their combined node count exceeded it; a package of many small
/// declarations could exceed a caller's configured `nodes` budget without
/// limit. Fixed by threading the package's running node total through
/// `ValueDeclarations::nodes_used`/`CheckedDeclarationBody::nodes_used`
/// (ordinary `Ok` payloads, not a side channel) -- see
/// `check_declaration_body`'s own doc for the mechanism.
#[trace("TC-381", "FR-062-AC-11")]
#[test]
fn nodes_limit_is_enforced_across_the_whole_package_not_per_declaration() {
    fn small(name: &str) -> FunctionDeclaration {
        function(
            name,
            &[],
            ValueType::Integer,
            None,
            binary(BinaryOperator::Add, literal(1), literal(1)),
        )
    }

    // One such declaration, alone, fits comfortably under a budget of 4.
    PackageDeclarations {
        functions: vec![small("a")],
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::new(4, 128).unwrap())
    .expect("one small declaration admits under a budget of 4");

    // Two declarations under a budget wide enough for both still admit --
    // the fix does not just refuse every multi-declaration package.
    PackageDeclarations {
        functions: vec![small("a"), small("b")],
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::new(100, 128).unwrap())
    .expect("two small declarations admit under a generous package-wide budget");

    // The same two declarations, under the reviewer's own measured budget
    // (main refuses the second at exactly this limit; the PR before this
    // fix admitted both): combined, they exceed 4 nodes even though neither
    // does alone.
    let exhausted = refusal(
        PackageDeclarations {
            functions: vec![small("a"), small("b")],
            ..PackageDeclarations::default()
        }
        .check(CheckingLimits::new(4, 128).unwrap()),
    );
    assert_eq!(
        exhausted.cause,
        CheckCause::ResourceExhausted {
            stage: CheckingStage::Typing,
            kind: CheckingLimitKind::Nodes,
            // The caller's own original configured limit (4), not a
            // remaining/partial figure -- `Typer`'s cap is never modified,
            // only its counter's starting value.
            limit: 4,
        }
    );
}

/// PR #302 review finding 1: `CheckingLimits::with_input_bytes` is a real,
/// caller-configurable knob on the public entry point, not just the
/// `ValueFunctionFamily::check`-level mechanism `family_contract_tests`
/// exercises directly -- one byte of budget cannot afford even the
/// declaration's own package-identity/name/parameter preimage, so `check`
/// refuses through `PackageDeclarations::check` naming
/// `CheckingLimitKind::InputBytes`, and the unconfigured (unlimited)
/// default admits the identical package.
#[test]
fn p_input_bytes_limit_refuses_through_package_declarations_check() {
    let limited = PackageDeclarations {
        functions: vec![down()],
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::default().with_input_bytes(1));
    let exhausted = refusal(limited);
    assert_eq!(
        exhausted.cause,
        CheckCause::ResourceExhausted {
            stage: CheckingStage::Typing,
            kind: CheckingLimitKind::InputBytes,
            limit: 1,
        }
    );

    PackageDeclarations {
        functions: vec![down()],
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::default())
    .expect("the unconfigured default is unlimited, so the same package admits");
}

/// PR #302 review finding 1/3: `CheckingLimits::with_work_budget` is a
/// real, caller-configurable knob reaching the shared kernel meter
/// `ValueFunctionFamily::check` charges `ChargePoint::DeclarationCheck`
/// against (finding 3's own fix) -- zero work units cannot afford even one
/// checked declaration, so `check` refuses through
/// `PackageDeclarations::check` naming `CheckingLimitKind::WorkBudget`, and
/// the unconfigured (unlimited) default admits the identical package.
#[test]
fn p_work_budget_limit_refuses_through_package_declarations_check() {
    let limited = PackageDeclarations {
        functions: vec![down()],
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::default().with_work_budget(0));
    let exhausted = refusal(limited);
    assert_eq!(
        exhausted.cause,
        CheckCause::ResourceExhausted {
            stage: CheckingStage::Typing,
            kind: CheckingLimitKind::WorkBudget,
            limit: 0,
        }
    );

    PackageDeclarations {
        functions: vec![down()],
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::default())
    .expect("the unconfigured default is unlimited, so the same package admits");
}

#[trace("TC-191", "FR-146-AC-5")]
#[trace("TC-191", "FR-146-AC-6")]
#[test]
fn p11_evaluation_charges_calls_orderings_arithmetic_and_skipped_operands() {
    let graph = check(vec![
        down(),
        quotient(if_then(
            binary(BinaryOperator::NotEqual, name("b"), literal(0)),
            divide(),
            zero_quotient(),
        )),
    ])
    .unwrap();
    // ADR-013 T-1 (FR-087, QSL-158 S-3a): the S4 link step, over an empty
    // dependency closure -- this fixture declares no import.
    let package = CheckedPackage::link(graph);
    let objects = ObjectEnvironment::default();
    let invoke = |function: &str, arguments: Vec<Value>, limits| {
        let mut meter = Meter::new(limits);
        let evaluation = package
            .call(
                &QualifiedName::unqualified(function).unwrap(),
                arguments,
                &objects,
                &mut meter,
            )
            .unwrap();
        (
            format!("{:?}", evaluated(evaluation)),
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

    // PR #302 review finding 2: every total/threshold below is one less
    // than before -- `package.call`'s own top-level admission charge is
    // against a separate contract-level meter now (`ValueFunctionFamily::
    // evaluate`'s own doc), never against this `meter` (`env.local_meter`,
    // what `invoke` reports); an earlier version charged `function.call`
    // against `local_meter` for the top-level call too, ahead of every
    // charge below, so every later charge's own position -- and so the
    // `work_limit` that exhausts exactly at it -- shifts down by one.
    assert_eq!(
        invoke("down", vec![int(2)], UNLIMITED),
        (format!("{:?}", Outcome::Completed(int(0))), 17, 5)
    );
    assert_eq!(
        invoke("down", vec![int(2)], work_limit(16)).0,
        incomplete(16, ChargePoint::OrderingResultRetain)
    );
    let half = Value::Rational(Rational::new(integer(3), integer(2)).unwrap());
    assert_eq!(
        invoke("q", vec![int(3), int(2)], UNLIMITED),
        (format!("{:?}", Outcome::Completed(half)), 9, 2)
    );
    // `rational-arithmetic.arithmetic` for `3/1` and `2/1`: `N = bits(3) +
    // bits(1) = 3`, `D = bits(1) + bits(2) = 3`, so `integer_bits` 3; the
    // operands (2) and normalize (2) stay below it.
    let mut meter = Meter::new(UNLIMITED);
    package
        .call(
            &QualifiedName::unqualified("q").unwrap(),
            vec![int(3), int(2)],
            &objects,
            &mut meter,
        )
        .unwrap();
    assert_eq!(meter.consumed(LimitKind::IntegerBits), 3);
    let mut narrow = Meter::new(ScalarLimits {
        integer_bits: 2,
        ..UNLIMITED
    });
    assert_eq!(
        format!(
            "{:?}",
            evaluated(
                package
                    .call(
                        &QualifiedName::unqualified("q").unwrap(),
                        vec![int(3), int(2)],
                        &objects,
                        &mut narrow,
                    )
                    .unwrap()
            )
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
        invoke("q", vec![int(3), int(2)], work_limit(8)).0,
        incomplete(8, ChargePoint::RationalArithmeticResultRetain)
    );

    let implication = binary(
        BinaryOperator::Implies,
        binary(BinaryOperator::Greater, name("a"), literal(0)),
        binary(BinaryOperator::Greater, name("b"), literal(0)),
    );
    let checked = package
        .graph()
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
            format!("{:?}", evaluated(evaluation)),
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

/// One S6a result through `CheckedPackage::call`: the kernel outcome inside
/// `FamilyOutcome::Evaluated`, and the call's recorded location and losses.
/// Any other arm or an `Err` fails the test.
fn call_evaluated(
    package: &CheckedPackage,
    function: &str,
    arguments: Vec<Value>,
    limits: ScalarLimits,
) -> (Outcome<Value>, Option<Location>, Vec<LocatedLoss>) {
    let mut meter = Meter::new(limits);
    let evaluation = package
        .call(
            &QualifiedName::unqualified(function.to_owned()).unwrap(),
            arguments,
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .unwrap_or_else(|failure| panic!("{function}: expected Ok, got Err({failure:?})"));
    match evaluation.outcome {
        FamilyOutcome::Evaluated(outcome) => (outcome, evaluation.location, evaluation.losses),
        FamilyOutcome::FamilyEvaluated(result) => {
            panic!("{function}: expected Evaluated, got FamilyEvaluated({result:?})")
        }
    }
}

/// TC-382 (FR-090-AC-1): S6a returns each kernel outcome of
/// `f(x: Float[binary64]): Rational[-9..9 / 1..9] = convert(x)` unchanged in
/// `FamilyOutcome::Evaluated`, through `CheckedPackage::call`, with the
/// location and losses the hook records. Each outcome is compared with a
/// fixed literal. A second function, a rounding decimal division, records a
/// loss, so `call` is shown to carry the hook's losses too.
#[trace("FR-090-AC-1", "TC-382")]
#[test]
fn s6a_returns_kernel_outcomes_unchanged_in_evaluated() {
    let package = PackageDeclarations {
        functions: vec![
            function(
                "f",
                &[("x", ValueType::Float(IeeeWidth::Binary64))],
                quotient_type(),
                None,
                Expression::Convert {
                    target: crate::support::type_form::type_form(&quotient_type()),
                    operand: Box::new(name("x")),
                },
            ),
            function(
                "divide",
                &[("p", decimal_type(0, 100)), ("q", decimal_type(1, 9))],
                ValueType::Decimal(
                    DecimalType::new(integer(0), integer(10_000), 2, 2, RoundingMode::NearestEven)
                        .unwrap(),
                ),
                None,
                binary(BinaryOperator::Divide, name("p"), name("q")),
            ),
        ],
        ieee_profile: Some(ieee_profile()),
        ..PackageDeclarations::default()
    }
    .check(CheckingLimits::default())
    .expect("convert(x) carries no definedness obligation, and q is nonzero");
    let package = CheckedPackage::link(package);
    let float = |bits: u64| vec![Value::Float(IeeeValue::binary64(bits))];

    let (half, location, losses) =
        call_evaluated(&package, "f", float(0x3FE0_0000_0000_0000), UNLIMITED);
    assert_eq!(
        format!("{half:?}"),
        format!(
            "{:?}",
            Outcome::<Value>::Completed(Value::Rational(
                Rational::new(integer(1), integer(2)).unwrap()
            ))
        )
    );
    assert_eq!(location, None);
    assert!(losses.is_empty());

    let (nan, location, losses) =
        call_evaluated(&package, "f", float(0x7FF8_0000_0000_0000), UNLIMITED);
    assert!(
        matches!(nan, Outcome::Undefined(Undefined::IeeeNotFinite)),
        "{nan:?}"
    );
    assert!(location.is_some());
    assert!(losses.is_empty());

    let (twenty, location, losses) =
        call_evaluated(&package, "f", float(0x4034_0000_0000_0000), UNLIMITED);
    assert!(
        matches!(twenty, Outcome::Refused(Refusal::IeeeRationalOutOfDomain)),
        "{twenty:?}"
    );
    assert!(location.is_some());
    assert!(losses.is_empty());

    let (denied, location, losses) =
        call_evaluated(&package, "f", float(0x3FE0_0000_0000_0000), work_limit(0));
    let Outcome::Incomplete(incomplete) = denied else {
        panic!("expected Incomplete for a zero work limit, got {denied:?}");
    };
    assert_eq!(incomplete.charge_point, ChargePoint::FunctionCall);
    assert_eq!(location, None);
    assert!(losses.is_empty());

    let decimal = |coefficient: i64| Value::Decimal(Decimal::new(integer(coefficient), 0));
    let (third, location, losses) =
        call_evaluated(&package, "divide", vec![decimal(1), decimal(3)], UNLIMITED);
    assert_eq!(
        format!("{third:?}"),
        format!(
            "{:?}",
            Outcome::<Value>::Completed(Value::Decimal(Decimal::new(integer(33), 2)))
        )
    );
    assert_eq!(location, None);
    assert_eq!(losses.len(), 1, "{losses:?}");
    assert!(
        matches!(losses[0].loss, ValueLoss::Decimal(_)),
        "{losses:?}"
    );
}

/// `Decimal[lower..upper]` at scale 0, exact rounding.
fn decimal_type(lower: i64, upper: i64) -> ValueType {
    ValueType::Decimal(
        DecimalType::new(integer(lower), integer(upper), 0, 0, RoundingMode::Exact).unwrap(),
    )
}
