// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-191 total pure function checking over the real `value` boundary
//! (FR-146), following the P01–P06 vectors of the FR-146 amendment in QSpec
//! PR #74.
//!
//! Function and composite declaration keys are opaque fixture keys: no
//! function source form or declaration preimage is available to the value
//! layer (SPEC-GAP(119-19), SPEC-GAP(119-5)).

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use quire_spec_language::diagnostic::Code;
use quire_spec_language::value::{
    check_functions, evaluate_equality, map, CardinalityBound, ChargePoint, CheckedFunctions,
    CollectionKind, CollectionValue, CompositeDeclaration, CompositeShape, DefinitionLock,
    DivisionProfile, Effect, ElementRelation, Expression, FieldDeclaration, FieldValue,
    FunctionCause, FunctionDeclaration, FunctionLimits, FunctionRefusal, Incomplete, Integer,
    IntegerComparison, IntegerInterval, LimitKind, Location, MeasureElement, Meter, NodeKey,
    OptionValue, Outcome, ParameterDeclaration, PathStep, Precondition, PreconditionEvidence,
    Presence, ScalarLimits, Termination, TypeEnvironment, Value, ValueType,
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

const LIMITS: FunctionLimits = FunctionLimits {
    expression_depth: 64,
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

fn parameter(index: usize) -> Expression {
    Expression::Parameter(index)
}

fn literal(value: i64) -> Expression {
    Expression::Literal {
        value: int(value),
        value_type: ValueType::Integer,
    }
}

fn boolean(value: bool) -> Expression {
    Expression::Literal {
        value: Value::Boolean(value),
        value_type: ValueType::Boolean,
    }
}

fn call(callee: &str, arguments: Vec<Expression>) -> Expression {
    Expression::Call {
        callee: key(callee),
        arguments,
    }
}

fn conditional(condition: Expression, then: Expression, otherwise: Expression) -> Expression {
    Expression::If {
        condition: Box::new(condition),
        then: Box::new(then),
        otherwise: Box::new(otherwise),
    }
}

fn compare(operator: IntegerComparison, left: Expression, right: Expression) -> Expression {
    Expression::Compare {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn minus(left: Expression, right: Expression) -> Expression {
    Expression::Arithmetic {
        operator: quire_spec_language::value::ArithmeticOperator::Subtract,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn field(record: Expression, name: &str) -> Expression {
    Expression::Field {
        record: Box::new(record),
        field: key(name),
    }
}

fn bounded(lower: i64, upper: i64) -> ParameterDeclaration {
    ParameterDeclaration::bounded(
        IntegerInterval::new(Integer::from(lower), Integer::from(upper)).unwrap(),
    )
}

fn function(
    name: &str,
    parameters: Vec<ParameterDeclaration>,
    result: ValueType,
    measure: Option<Vec<MeasureElement>>,
    body: Expression,
) -> FunctionDeclaration {
    FunctionDeclaration {
        key: key(name),
        parameters,
        result,
        effects: BTreeSet::new(),
        measure,
        body,
    }
}

fn located(name: &str, path: Vec<PathStep>, cause: FunctionCause) -> FunctionRefusal {
    FunctionRefusal {
        location: Location {
            function: key(name),
            path,
        },
        cause,
    }
}

fn meter() -> Meter {
    Meter::new(UNLIMITED)
}

fn node_type() -> ValueType {
    ValueType::Composite(key("Node"))
}

/// `record Node { head: Integer; next: Option<Node>; }`.
fn environment() -> TypeEnvironment {
    TypeEnvironment::new([CompositeDeclaration::new(
        key("Node"),
        CompositeShape::Record(vec![
            FieldDeclaration::new(key("head"), ValueType::Integer, Presence::Required),
            FieldDeclaration::new(
                key("next"),
                ValueType::Option(Box::new(node_type())),
                Presence::Required,
            ),
        ]),
    )])
    .unwrap()
}

/// A chain whose heads are `heads`, first node first.
fn chain(environment: &TypeEnvironment, heads: &[i64]) -> Value {
    let mut next = OptionValue::none(node_type());
    for head in heads.iter().rev() {
        let node = environment
            .record(
                key("Node"),
                vec![
                    (key("head"), FieldValue::Present(int(*head))),
                    (key("next"), FieldValue::Present(next)),
                ],
            )
            .unwrap();
        next = OptionValue::present(node_type(), node).unwrap();
    }
    match next {
        Value::Option(option) => option.payload().unwrap().clone(),
        other => panic!("not an option: {other:?}"),
    }
}

/// P01: `function last(n: Node): Integer pure decreases(n) { if
/// present(n.next) then last(value(n.next)) else n.head }`.
fn last() -> FunctionDeclaration {
    function(
        "last",
        vec![ParameterDeclaration::new(node_type())],
        ValueType::Integer,
        Some(vec![MeasureElement::Containment(0)]),
        conditional(
            Expression::Present(Box::new(field(parameter(0), "next"))),
            call(
                "last",
                vec![Expression::Value(Box::new(field(parameter(0), "next")))],
            ),
            field(parameter(0), "head"),
        ),
    )
}

/// P03: `even`/`odd` over `Int[0,9]` with the given `odd` measure.
fn parity(odd_measure: Vec<MeasureElement>) -> Vec<FunctionDeclaration> {
    let step = |name: &str, base: bool, other: &str, measure| {
        function(
            name,
            vec![bounded(0, 9)],
            ValueType::Boolean,
            Some(measure),
            conditional(
                compare(IntegerComparison::Equal, parameter(0), literal(0)),
                boolean(base),
                call(other, vec![minus(parameter(0), literal(1))]),
            ),
        )
    };
    vec![
        step("even", true, "odd", vec![MeasureElement::Integer(0)]),
        step("odd", false, "even", odd_measure),
    ]
}

fn completed(outcome: Outcome<Value>) -> Value {
    match outcome {
        Outcome::Completed(value) => value,
        other => panic!("not completed: {other:?}"),
    }
}

/// Complete-value equality of two values.
fn assert_same(actual: &Value, expected: &Value) {
    match evaluate_equality(actual, expected, &mut meter()) {
        Ok(Outcome::Completed(true)) => {}
        other => panic!("{actual:?} is not {expected:?}: {other:?}"),
    }
}

fn admitted(declarations: Vec<FunctionDeclaration>) -> CheckedFunctions {
    check_functions(&environment(), declarations, LIMITS).unwrap()
}

#[trace("TC-191", "FR-146-AC-1")]
#[test]
fn proved_structural_recursion_is_admitted_with_published_facts_and_evaluates() {
    let environment = environment();
    let checked = check_functions(&environment, vec![last()], LIMITS).unwrap();
    let last = checked.function(key("last")).unwrap();

    let Termination::Recursive {
        component,
        measure,
        edges,
    } = last.termination()
    else {
        panic!("not recursive: {:?}", last.termination());
    };
    assert_eq!(component, &vec![key("last")]);
    assert_eq!(measure, &vec![MeasureElement::Containment(0)]);
    assert_eq!(edges.len(), 1);
    assert_eq!(
        edges[0].call,
        Location {
            function: key("last"),
            path: vec![PathStep::Then],
        }
    );
    assert_eq!(edges[0].relations, vec![ElementRelation::Less]);
    assert_eq!(edges[0].decreasing_position(), Some(0));
    assert_eq!(last.definedness().len(), 1);
    assert_eq!(last.definedness()[0].precondition, Precondition::Present);
    assert_eq!(
        last.definedness()[0].evidence,
        PreconditionEvidence::PresenceGuard
    );

    // P06, first half: three calls, one `function.call` charge each.
    let mut metered = meter();
    let outcome = checked
        .evaluate(
            key("last"),
            vec![chain(&environment, &[4, 5, 6])],
            &mut metered,
        )
        .unwrap();
    assert_same(&completed(outcome), &int(6));
    assert_eq!(metered.admitted_charges(), &[ChargePoint::FunctionCall; 3]);
    assert_eq!(metered.consumed(LimitKind::WorkUnits), 3);

    // A checked unary function is an FR-145 `ValueFunction`.
    let increment = admitted(vec![function(
        "increment",
        vec![ParameterDeclaration::new(ValueType::Integer)],
        ValueType::Integer,
        None,
        Expression::Arithmetic {
            operator: quire_spec_language::value::ArithmeticOperator::Add,
            left: Box::new(parameter(0)),
            right: Box::new(literal(1)),
        },
    )]);
    assert_eq!(
        increment.function(key("increment")).unwrap().termination(),
        &Termination::Acyclic
    );
    let bound = Some(CardinalityBound::new(0, 4).unwrap());
    let source = |values: &[i64]| {
        CollectionValue::construct(
            CollectionKind::Sequence,
            ValueType::Integer,
            bound,
            values.iter().copied().map(int).collect(),
        )
        .unwrap()
    };
    let Value::Collection(items) = source(&[1, 2]) else {
        panic!("not a collection");
    };
    let mapped = map(
        &items,
        &increment.value_function(key("increment")).unwrap(),
        bound,
        &mut meter(),
    )
    .unwrap();
    assert_same(&completed(mapped), &source(&[2, 3]));
}

#[trace("TC-191", "FR-146-AC-2")]
#[test]
fn nondecreasing_or_unmeasured_cycles_refuse_with_cycle_and_obligation() {
    // P02: `loop(n: Int[0,9]) decreases(n) { loop(n) }`.
    let looping = |measure| {
        function(
            "loop",
            vec![bounded(0, 9)],
            ValueType::Integer,
            measure,
            call("loop", vec![parameter(0)]),
        )
    };
    let refusal = check_functions(
        &environment(),
        vec![looping(Some(vec![MeasureElement::Integer(0)]))],
        LIMITS,
    )
    .unwrap_err();
    let FunctionCause::NonDecreasing { cycle, obligation } = &refusal.cause else {
        panic!("not a decrease refusal: {refusal:?}");
    };
    assert_eq!(cycle, &vec![key("loop"), key("loop")]);
    assert_eq!(obligation.callee, key("loop"));
    assert_eq!(obligation.call.path, Vec::<PathStep>::new());
    assert_eq!(obligation.relations, vec![ElementRelation::Equal]);
    assert_eq!(obligation.decreasing_position(), None);
    assert_eq!(refusal.cause.code(), Code::UndefinedExpression);

    let refusal = check_functions(&environment(), vec![looping(None)], LIMITS).unwrap_err();
    assert_eq!(
        refusal,
        located(
            "loop",
            Vec::new(),
            FunctionCause::MissingMeasure {
                cycle: vec![key("loop"), key("loop")],
            },
        )
    );
    assert_eq!(refusal.cause.code(), Code::UndefinedExpression);

    // An unbounded `Integer` measure is not proved nonnegative on entry.
    let mut unbounded = looping(Some(vec![MeasureElement::Integer(0)]));
    unbounded.parameters = vec![ParameterDeclaration::new(ValueType::Integer)];
    assert_eq!(
        check_functions(&environment(), vec![unbounded], LIMITS).unwrap_err(),
        located("loop", Vec::new(), FunctionCause::MeasureNotNonnegative(0))
    );
}

#[trace("TC-191", "FR-146-AC-3")]
#[test]
fn effects_undeclared_calls_and_wrong_typing_refuse_even_when_unreachable() {
    let unreachable = |otherwise: Expression| {
        function(
            "r",
            vec![ParameterDeclaration::new(ValueType::Integer)],
            ValueType::Integer,
            None,
            conditional(boolean(true), literal(1), otherwise),
        )
    };
    let refuse = |declarations| check_functions(&environment(), declarations, LIMITS).unwrap_err();

    // P05: an undeclared host call, with no call edge.
    let refusal = refuse(vec![unreachable(call(
        "undeclaredHost",
        vec![parameter(0)],
    ))]);
    assert_eq!(
        refusal,
        located(
            "r",
            vec![PathStep::Otherwise],
            FunctionCause::UnknownCallee(key("undeclaredHost")),
        )
    );
    assert_eq!(refusal.cause.code(), Code::MissingDeclaration);

    for effect in [
        Effect::Io,
        Effect::Mutation,
        Effect::Reflection,
        Effect::AmbientLookup,
    ] {
        let refusal = refuse(vec![unreachable(Expression::Effect(effect))]);
        assert_eq!(
            refusal,
            located(
                "r",
                vec![PathStep::Otherwise],
                FunctionCause::ProhibitedEffect(effect),
            )
        );
        assert_eq!(refusal.cause.code(), Code::IllTyped);

        let mut declared = unreachable(literal(2));
        declared.effects.insert(effect);
        assert_eq!(
            refuse(vec![declared]),
            located("r", Vec::new(), FunctionCause::DeclaredEffect(effect))
        );
    }

    let wrong_result = refuse(vec![unreachable(boolean(false))]);
    assert_eq!(
        wrong_result,
        located("r", vec![PathStep::Otherwise], FunctionCause::ResultType)
    );
    assert_eq!(wrong_result.cause.code(), Code::IllTyped);

    let callee = function(
        "id",
        vec![ParameterDeclaration::new(ValueType::Integer)],
        ValueType::Integer,
        None,
        parameter(0),
    );
    let wrong_argument = refuse(vec![
        callee.clone(),
        unreachable(call("id", vec![boolean(true)])),
    ]);
    assert_eq!(
        wrong_argument,
        located(
            "r",
            vec![PathStep::Otherwise, PathStep::Argument(0)],
            FunctionCause::ArgumentType(0),
        )
    );
    assert_eq!(wrong_argument.cause.code(), Code::IllTyped);
    assert_eq!(
        refuse(vec![callee, unreachable(call("id", Vec::new()))]).cause,
        FunctionCause::Arity {
            declared: 1,
            supplied: 0,
        }
    );
}

#[trace("TC-191", "FR-146-AC-4")]
#[test]
fn mutual_recursion_is_admitted_only_when_every_edge_decreases_the_shared_measure() {
    let checked = admitted(parity(vec![MeasureElement::Integer(0)]));
    for (name, other) in [("even", "odd"), ("odd", "even")] {
        let Termination::Recursive {
            component, edges, ..
        } = checked.function(key(name)).unwrap().termination()
        else {
            panic!("{name} is not recursive");
        };
        let mut members = vec![key("even"), key("odd")];
        members.sort();
        assert_eq!(component, &members);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].callee, key(other));
        assert_eq!(edges[0].relations, vec![ElementRelation::Less]);
    }
    for (n, even) in [(0, true), (1, false), (4, true), (9, false)] {
        let outcome = checked
            .evaluate(key("even"), vec![int(n)], &mut meter())
            .unwrap();
        assert_same(&completed(outcome), &Value::Boolean(even));
    }

    // The measure arity differs within the component.
    let refusal = check_functions(
        &environment(),
        parity(vec![MeasureElement::Integer(0), MeasureElement::Integer(0)]),
        LIMITS,
    )
    .unwrap_err();
    let (first, second) = if key("even") < key("odd") {
        ("even", "odd")
    } else {
        ("odd", "even")
    };
    assert_eq!(
        refusal,
        located(
            first,
            Vec::new(),
            FunctionCause::MeasureShape {
                cycle: vec![key(first), key(second), key(first)],
            },
        )
    );
    assert_eq!(refusal.cause.code(), Code::UndefinedExpression);

    // One edge that does not decrease refuses the whole component.
    let mut declarations = parity(vec![MeasureElement::Integer(0)]);
    declarations[1].body = conditional(
        compare(IntegerComparison::Equal, parameter(0), literal(0)),
        boolean(false),
        call("even", vec![parameter(0)]),
    );
    let refusal = check_functions(&environment(), declarations, LIMITS).unwrap_err();
    let FunctionCause::NonDecreasing { cycle, obligation } = &refusal.cause else {
        panic!("not a decrease refusal: {refusal:?}");
    };
    assert_eq!(cycle, &vec![key("odd"), key("even"), key("odd")]);
    assert_eq!(obligation.call.function, key("odd"));
    assert_eq!(obligation.call.path, vec![PathStep::Otherwise]);
    assert_eq!(obligation.relations, vec![ElementRelation::Equal]);
}

#[trace("TC-191", "FR-146-AC-5")]
#[test]
fn exhausted_fuel_is_incomplete_and_leaves_the_totality_verdict_unchanged() {
    let environment = environment();
    let checked = check_functions(&environment, vec![last()], LIMITS).unwrap();
    let verdict = checked.function(key("last")).unwrap().termination().clone();

    // P06, second half: `work_units: 2` denies the third call.
    let mut limited = Meter::new(ScalarLimits {
        work_units: 2,
        ..UNLIMITED
    });
    let outcome = checked
        .evaluate(
            key("last"),
            vec![chain(&environment, &[4, 5, 6])],
            &mut limited,
        )
        .unwrap();
    let Outcome::Incomplete(record) = outcome else {
        panic!("not incomplete: {outcome:?}");
    };
    assert_eq!(
        record,
        Incomplete {
            limit_kind: LimitKind::WorkUnits,
            limit: 2,
            consumed: 2,
            next_charge: Integer::from(1_i64),
            charge_point: ChargePoint::FunctionCall,
        }
    );
    assert_eq!(
        checked.function(key("last")).unwrap().termination(),
        &verdict
    );
    let again = checked
        .evaluate(
            key("last"),
            vec![chain(&environment, &[4, 5, 6])],
            &mut meter(),
        )
        .unwrap();
    assert_same(&completed(again), &int(6));
}

#[trace("TC-191", "FR-146-AC-1")]
#[test]
fn partial_operations_need_proved_preconditions() {
    let law = DefinitionLock::pinned()
        .unwrap()
        .admit_integer_division(
            &[DefinitionLock::pinned()
                .unwrap()
                .entry(quire_spec_language::value::CatalogRole::IntegerDivisionTruncating)
                .unwrap()
                .definition
                .clone()],
            None,
        )
        .unwrap();
    assert_eq!(law.profile(), DivisionProfile::Truncating);
    let quotient = |body| {
        function(
            "q",
            vec![bounded(-9, 9), bounded(-9, 9)],
            ValueType::Integer,
            None,
            body,
        )
    };
    let divide = || Expression::Quotient {
        law,
        dividend: Box::new(parameter(0)),
        divisor: Box::new(parameter(1)),
    };

    // P04: an unguarded divisor, then the guarded body.
    let refusal = check_functions(&environment(), vec![quotient(divide())], LIMITS).unwrap_err();
    assert_eq!(
        refusal,
        located(
            "q",
            vec![PathStep::Right],
            FunctionCause::UnprovedPrecondition(Precondition::NonzeroDivisor),
        )
    );
    assert_eq!(refusal.cause.code(), Code::UndefinedExpression);

    let checked = admitted(vec![quotient(conditional(
        compare(IntegerComparison::NotEqual, parameter(1), literal(0)),
        divide(),
        literal(0),
    ))]);
    let facts = checked.function(key("q")).unwrap().definedness();
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].location.path, vec![PathStep::Then]);
    assert_eq!(facts[0].precondition, Precondition::NonzeroDivisor);
    assert_eq!(facts[0].evidence, PreconditionEvidence::ExcludedZero);
    for (a, b, expected) in [(7, 2, 3), (-7, 2, -3), (7, 0, 0)] {
        let outcome = checked
            .evaluate(key("q"), vec![int(a), int(b)], &mut meter())
            .unwrap();
        assert_same(&completed(outcome), &int(expected));
    }

    // P04: `value(o)` without `present(o)`.
    let refusal = check_functions(
        &environment(),
        vec![function(
            "v",
            vec![ParameterDeclaration::new(ValueType::Option(Box::new(
                ValueType::Integer,
            )))],
            ValueType::Integer,
            None,
            Expression::Value(Box::new(parameter(0))),
        )],
        LIMITS,
    )
    .unwrap_err();
    assert_eq!(
        refusal,
        located(
            "v",
            Vec::new(),
            FunctionCause::UnprovedPrecondition(Precondition::Present),
        )
    );
    assert_eq!(refusal.cause.code(), Code::UndefinedExpression);

    // An `Int[0,9]` argument must be proved inside the interval.
    let digit = function(
        "digit",
        vec![bounded(0, 9)],
        ValueType::Integer,
        None,
        parameter(0),
    );
    let caller = |argument| {
        function(
            "caller",
            vec![ParameterDeclaration::new(ValueType::Integer)],
            ValueType::Integer,
            None,
            call("digit", vec![argument]),
        )
    };
    assert_eq!(
        check_functions(
            &environment(),
            vec![digit.clone(), caller(parameter(0))],
            LIMITS
        )
        .unwrap_err(),
        located(
            "caller",
            vec![PathStep::Argument(0)],
            FunctionCause::UnprovedPrecondition(Precondition::ArgumentInterval(0)),
        )
    );
    let checked = admitted(vec![digit, caller(literal(7))]);
    assert_eq!(
        checked.function(key("caller")).unwrap().definedness()[0].evidence,
        PreconditionEvidence::Bounds {
            lower: Some(Integer::from(7_i64)),
            upper: Some(Integer::from(7_i64)),
        }
    );
}
