// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-174 (FR-068-AC-5), QSL-183 edge cut: the three
//! `CheckedPackage::call`/refusal fixtures formerly in
//! `value::expression::mod`'s own `#[cfg(test)] mod tests` reach only the
//! crate's public API (`PackageDeclarations::check`, `CheckedPackage::call`,
//! and the public refusal types), so they moved here in #323.
//!
//! `function_slots_are_stable_across_declaration_order` (formerly kept in
//! `value::expression::mod` because it read that module's same-crate
//! `check::CheckedGraph::function_states()`, `pub(crate)`) also moves here
//! now, rewritten rather than relocated verbatim: `function_states()` itself
//! has no public accessor and nothing outside this crate needs one, but the
//! property it existed to guard -- a checked function's own evaluation-slot
//! count never leaking onto a *different* function's frame, regardless of
//! declaration order -- is independently observable through
//! `CheckedGraph::check_expression`/`CheckedPackage::evaluate` (both
//! public): `evaluate`'s `Machine` resizes each callee's frame from exactly
//! this per-function slot count (`evaluate.rs`'s `frame.resize(callable.
//! slots.max(frame.len()), None)`) when a checked expression's own `Call`
//! node runs, so a scrambled count surfaces as a wrong or undefined result,
//! not only as an internal accessor mismatch. See
//! `evaluated_call_slots_are_stable_across_declaration_order` below --
//! renamed off the original because "slots" is no longer read directly, and
//! kept on `TC-174`/`FR-068-AC-5` because it is still exactly that test
//! case's own property, exercised through the public surface instead.

use ix_trace_rs::trace;
use quire_exact::{Integer, Meter, ScalarLimits};
use quire_spec_language::value::{
    BinaryOperator, CheckCause, CheckMode, CheckRefusal, CheckedGraph, CheckedPackage,
    CheckingLimits, Expression, FunctionDeclaration, InputRefusal, ObjectEnvironment, Outcome,
    PackageDeclarations, QualifiedName, Value, ValueType,
};

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

/// One parameter, no `let`: the minimal shape.
fn function_one() -> FunctionDeclaration {
    FunctionDeclaration::new(
        "one",
        vec![("a".to_owned(), ValueType::Integer)],
        ValueType::Integer,
        None,
        Expression::Name("a".to_owned()),
    )
}

/// Two parameters and one `let`-bound local: deliberately a different
/// parameter count and slot shape from [`function_one`], matching the
/// fixture `value::expression::mod`'s own retained test still uses.
fn function_two() -> FunctionDeclaration {
    FunctionDeclaration::new(
        "two",
        vec![
            ("a".to_owned(), ValueType::Integer),
            ("b".to_owned(), ValueType::Integer),
        ],
        ValueType::Integer,
        None,
        Expression::Let {
            name: "x".to_owned(),
            value: Box::new(Expression::Binary {
                operator: BinaryOperator::Add,
                left: Box::new(Expression::Name("a".to_owned())),
                right: Box::new(Expression::Name("b".to_owned())),
            }),
            body: Box::new(Expression::Name("x".to_owned())),
        },
    )
}

fn declarations(functions: Vec<FunctionDeclaration>) -> PackageDeclarations {
    PackageDeclarations {
        functions,
        ..PackageDeclarations::default()
    }
}

/// ADR-013 T-1 (FR-087, QSL-158 S-3a): the S4 link step, over an empty
/// dependency closure -- every fixture here declares no import, so E4
/// never has a real dependency to populate (see [`CheckedPackage`]'s own
/// `dependencies` field doc).
fn link(graph: CheckedGraph) -> CheckedPackage {
    CheckedPackage::link(graph)
}

/// TC-174 steps 1-4: `CheckedPackage::call` resolves each function by
/// name, not position -- the same fixture, called through the public
/// runtime entry point in both declaration orders, must produce the
/// same result each time.
#[trace("TC-174", "FR-068-AC-5")]
#[test]
fn call_resolves_by_name_regardless_of_declaration_order() {
    let objects = ObjectEnvironment::default();
    for functions in [
        vec![function_one(), function_two()],
        vec![function_two(), function_one()],
    ] {
        let package = link(
            declarations(functions)
                .check(CheckingLimits::default())
                .unwrap(),
        );
        let mut meter = Meter::new(UNLIMITED);
        let one = package
            .call(
                &QualifiedName::unqualified("one").unwrap(),
                vec![Value::Integer(Integer::from(5_i64))],
                &objects,
                &mut meter,
            )
            .unwrap();
        assert_eq!(
            format!("{:?}", one.outcome),
            format!(
                "{:?}",
                Outcome::Completed(Value::Integer(Integer::from(5_i64)))
            )
        );
        let mut meter = Meter::new(UNLIMITED);
        let two = package
            .call(
                &QualifiedName::unqualified("two").unwrap(),
                vec![
                    Value::Integer(Integer::from(3_i64)),
                    Value::Integer(Integer::from(4_i64)),
                ],
                &objects,
                &mut meter,
            )
            .unwrap();
        assert_eq!(
            format!("{:?}", two.outcome),
            format!(
                "{:?}",
                Outcome::Completed(Value::Integer(Integer::from(7_i64)))
            )
        );
    }
}

/// TC-174 steps 1-4 (formerly `value::expression::mod::tests::
/// function_slots_are_stable_across_declaration_order`, QSL-183 edge cut):
/// a standalone expression that calls into a declared function, checked and
/// evaluated through the public `CheckedGraph::check_expression`/
/// `CheckedPackage::evaluate` pair, resolves to the correct result
/// regardless of the package's declaration order -- the same property the
/// original fixture read off `function_states()` directly, now observed
/// through `evaluate`'s own per-function frame allocation instead. `two`'s
/// body needs a `let`-bound local beyond its two parameters, so a
/// declaration-order bug that hands it `one`'s (smaller) slot count would
/// produce a wrong or undefined result here, not a passing one.
#[trace("TC-174", "FR-068-AC-5")]
#[test]
fn evaluated_call_slots_are_stable_across_declaration_order() {
    let objects = ObjectEnvironment::default();
    for functions in [
        vec![function_one(), function_two()],
        vec![function_two(), function_one()],
    ] {
        let graph = declarations(functions)
            .check(CheckingLimits::default())
            .unwrap();
        let call_one = graph
            .check_expression(
                Vec::new(),
                &Expression::Call {
                    name: "one".to_owned(),
                    arguments: vec![Expression::Integer(Integer::from(5_i64))],
                },
                Some(&ValueType::Integer),
                CheckMode::Linked,
                CheckingLimits::default(),
            )
            .unwrap();
        let call_two = graph
            .check_expression(
                Vec::new(),
                &Expression::Call {
                    name: "two".to_owned(),
                    arguments: vec![
                        Expression::Integer(Integer::from(3_i64)),
                        Expression::Integer(Integer::from(4_i64)),
                    ],
                },
                Some(&ValueType::Integer),
                CheckMode::Linked,
                CheckingLimits::default(),
            )
            .unwrap();
        let package = link(graph);

        let mut meter = Meter::new(UNLIMITED);
        let one = package
            .evaluate(&call_one, Vec::new(), &objects, &mut meter)
            .unwrap();
        assert_eq!(
            format!("{:?}", one.outcome),
            format!(
                "{:?}",
                Outcome::Completed(Value::Integer(Integer::from(5_i64)))
            )
        );

        let mut meter = Meter::new(UNLIMITED);
        let two = package
            .evaluate(&call_two, Vec::new(), &objects, &mut meter)
            .unwrap();
        assert_eq!(
            format!("{:?}", two.outcome),
            format!(
                "{:?}",
                Outcome::Completed(Value::Integer(Integer::from(7_i64)))
            )
        );
    }
}

/// TC-174 step 1's refusal fixture: a duplicate function name is
/// refused `CheckCause::AmbiguousName`, unaffected by the split.
#[trace("TC-174", "FR-068-AC-5")]
#[test]
fn duplicate_function_name_is_refused_ambiguous_name() {
    let result =
        declarations(vec![function_one(), function_one()]).check(CheckingLimits::default());
    let refusals = result.expect_err("a duplicate name must be refused, not admitted");
    assert!(refusals.iter().any(|refusal: &CheckRefusal| matches!(
        &refusal.cause,
        CheckCause::AmbiguousName { name, .. } if name == "one"
    )));
}

/// TC-174 step 1's `InputRefusal` fixture: calling an undeclared name
/// refuses `UnknownFunction`, unaffected by the split.
#[trace("TC-174", "FR-068-AC-5")]
#[test]
fn call_to_an_unknown_function_is_refused() {
    let package = link(
        declarations(vec![function_one()])
            .check(CheckingLimits::default())
            .unwrap(),
    );
    let objects = ObjectEnvironment::default();
    let mut meter = Meter::new(UNLIMITED);
    let result = package.call(
        &QualifiedName::unqualified("missing").unwrap(),
        vec![Value::Integer(Integer::from(1_i64))],
        &objects,
        &mut meter,
    );
    assert!(matches!(result, Err(InputRefusal::UnknownFunction(name)) if name == "missing"));
}
