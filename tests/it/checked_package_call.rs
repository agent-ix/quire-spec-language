// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-174 (FR-068-AC-5), QSL-183 edge cut: the three
//! `CheckedPackage::call`/refusal fixtures formerly in
//! `value::expression::mod`'s own `#[cfg(test)] mod tests` reach only the
//! crate's public API (`PackageDeclarations::check`, `CheckedPackage::call`,
//! and the public refusal types) -- unlike
//! `function_slots_are_stable_across_declaration_order` (kept in
//! `value::expression::mod`, which still needs that module's same-crate
//! read of `check::CheckedGraph`'s `pub(crate)` `function_states()`
//! accessor), so these three belong in this crate's own `tests/it`
//! integration binary, which is allowed to reach both the `forms`-owned
//! fixture types (`FunctionDeclaration`, `Expression`, `BinaryOperator`)
//! and the `check`/`value::expression` checking-and-evaluation pipeline in
//! the same test (QSL-183's own audit: "an integration test under
//! tests/it/ that is allowed to use both layers").

use ix_trace_rs::trace;
use quire_exact::{Integer, Meter, ScalarLimits};
use quire_spec_language::value::{
    BinaryOperator, CheckCause, CheckRefusal, CheckedGraph, CheckedPackage, CheckingLimits,
    Expression, FunctionDeclaration, InputRefusal, ObjectEnvironment, Outcome, PackageDeclarations,
    QualifiedName, Value, ValueType,
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
