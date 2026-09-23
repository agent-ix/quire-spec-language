// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-174 (FR-068-AC-5): `CheckedPackage::call` and `CheckedPackage::evaluate`,
//! this crate's public runtime entry points, exercised together with the
//! checking pipeline that produces their `CheckedPackage`/`CheckedExpression`
//! inputs (`PackageDeclarations::check`, `CheckedGraph::check_expression`,
//! `CheckedPackage::link`) -- an integration test, allowed to reach both the
//! `forms`-owned fixture types (`FunctionDeclaration`, `Expression`,
//! `BinaryOperator`) and this crate's checking/evaluation pipeline in the
//! same file.
//!
//! `evaluated_call_slots_are_stable_across_declaration_order` checks that a
//! standalone expression calling into a declared function resolves to the
//! correct result regardless of the package's declaration order:
//! `evaluate`'s `Machine` resizes each callee's frame from that function's
//! own evaluation-slot count (`evaluate.rs`'s `frame.resize(callable.
//! slots.max(frame.len()), None)`), so a count that leaked from a different
//! function surfaces here as a wrong or undefined result. `two`'s body needs
//! a `let`-bound local beyond its two parameters, discriminating it from
//! `one`'s smaller slot count.

use ix_trace_rs::trace;
use qsl_forms::{BinaryOperator, Expression, FunctionDeclaration};
use quire_exact::{Integer, Meter, Outcome, ScalarLimits};
use quire_spec_language::value::{
    CallFailure, CheckCause, CheckMode, CheckRefusal, CheckedGraph, CheckedPackage,
    CheckedPackageEvaluation, CheckingLimits, Evaluation, FamilyOutcome, InputRefusal,
    ObjectEnvironment, PackageDeclarations, QualifiedName, Value, ValueType,
};

/// FR-090: `CheckedPackage::call`/`evaluate` return `Evaluation { outcome:
/// FamilyOutcome, .. }` (FR-090-OQ-3, ruled option A); every fixture in this
/// file completes, so extracting `Evaluated`'s kernel outcome is all these
/// tests need.
fn evaluated(evaluation: Evaluation) -> quire_exact::Outcome<Value> {
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

/// One parameter, no `let`: the minimal shape.
fn function_one() -> FunctionDeclaration {
    FunctionDeclaration::new(
        "one",
        vec![(
            "a".to_owned(),
            crate::support::type_form::type_form(&ValueType::Integer),
        )],
        crate::support::type_form::type_form(&ValueType::Integer),
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
            format!("{:?}", evaluated(one)),
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
            format!("{:?}", evaluated(two)),
            format!(
                "{:?}",
                Outcome::Completed(Value::Integer(Integer::from(7_i64)))
            )
        );
    }
}

/// TC-174 steps 1-4: a standalone expression that calls into a declared
/// function, checked and evaluated through the public
/// `CheckedGraph::check_expression`/`CheckedPackage::evaluate` pair,
/// resolves to the correct result regardless of the package's declaration
/// order. `two`'s body needs a `let`-bound local beyond its two parameters,
/// so a declaration-order bug that hands it `one`'s (smaller) slot count
/// produces a wrong or undefined result here, not a passing one.
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
            format!("{:?}", evaluated(one)),
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
            format!("{:?}", evaluated(two)),
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
    assert!(matches!(
        result,
        Err(CallFailure::Input(InputRefusal::UnknownFunction(name))) if name == "missing"
    ));
}
