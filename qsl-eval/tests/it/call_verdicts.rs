// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-164 / FR-065-AC-5: a function application receives the same verdict
//! wherever the S3 typer reaches it -- a declaration body checked through
//! `PackageDeclarations::check`, a precondition clause checked through
//! `CheckedGraph::check_clause_expression`, and a `decreases` measure.

use ix_trace_rs::trace;

use qsl_forms::{ClauseKind, Expression, FunctionDeclaration};
use qsl_semantics::check::{CheckCause, CheckMode, CheckingLimits, NodeKind, PackageDeclarations};
use quire_exact::{IllTypedCause, Integer, ValueType};

use crate::support::type_form::type_form;

fn boolean() -> qsl_forms::TypeForm {
    type_form(&ValueType::Boolean)
}

/// `f(x: Boolean): Boolean` with body `x`.
fn f() -> FunctionDeclaration {
    FunctionDeclaration::new(
        "f",
        vec![("x".to_owned(), boolean())],
        boolean(),
        None,
        Expression::Name("x".to_owned()),
    )
}

fn call(name: &str, arguments: Vec<Expression>) -> Expression {
    Expression::Call {
        name: name.to_owned(),
        arguments,
    }
}

fn package(functions: Vec<FunctionDeclaration>) -> PackageDeclarations {
    PackageDeclarations {
        functions,
        ..PackageDeclarations::new(qsl_semantics::check::fixture_source())
    }
}

/// The four calls of TC-164 and the cause each must be refused with
/// (`None` = admitted).
fn calls() -> Vec<(&'static str, Expression, Option<CheckCause>)> {
    let t = || Expression::Boolean(true);
    let mismatch = || Some(CheckCause::IllTyped(IllTypedCause::TypeMismatch));
    vec![
        ("f(true)", call("f", vec![t()]), None),
        (
            "f(true, false)",
            call("f", vec![t(), Expression::Boolean(false)]),
            mismatch(),
        ),
        (
            "nowhere(true)",
            call("nowhere", vec![t()]),
            Some(CheckCause::MissingName("nowhere".to_owned())),
        ),
        (
            "f(1)",
            call("f", vec![Expression::Integer(Integer::from(1_i64))]),
            mismatch(),
        ),
    ]
}

#[trace("TC-164", "FR-065-AC-5")]
#[test]
fn a_call_receives_the_same_verdict_from_a_declaration_body_a_clause_and_a_measure() {
    for (label, expression, expected) in calls() {
        // Step 2: as the body of a parameterless Boolean declaration `g`.
        let body = package(vec![
            f(),
            FunctionDeclaration::new("g", vec![], boolean(), None, expression.clone()),
        ])
        .check(CheckingLimits::default());

        // Step 3: as a precondition clause over a package declaring only `f`.
        let graph = package(vec![f()])
            .check(CheckingLimits::default())
            .expect("the package declaring only f checks cleanly");
        let clause = graph.check_clause_expression(
            Vec::new(),
            &expression,
            Some(&ValueType::Boolean),
            ClauseKind::Precondition,
            CheckMode::Linked,
            CheckingLimits::default(),
        );

        match &expected {
            None => {
                let graph = body.unwrap_or_else(|r| panic!("{label}: body refused {r:?}"));
                let g = graph.function_state(1).expect("g is function 1");
                assert_eq!(g.name, "g", "{label}");
                assert_eq!(g.body.value_type(), &ValueType::Boolean, "{label}: body");
                assert!(
                    matches!(g.body.kind(), NodeKind::Call { function: 0, .. }),
                    "{label}: body: expected a call to f (function 0), got {:?}",
                    g.body.kind()
                );
                let checked = clause.unwrap_or_else(|r| panic!("{label}: clause refused {r:?}"));
                assert_eq!(checked.value_type(), &ValueType::Boolean, "{label}");
                assert!(
                    matches!(checked.root().kind(), NodeKind::Call { function: 0, .. }),
                    "{label}: expected a call to f (function 0), got {:?}",
                    checked.root().kind()
                );
            }
            Some(cause) => {
                let refusals = body.expect_err(&format!("{label}: the body must be refused"));
                assert_eq!(refusals.len(), 1, "{label}: {refusals:?}");
                assert_eq!(&refusals[0].cause, cause, "{label}: body");
                let refusal = clause.expect_err(&format!("{label}: the clause must be refused"));
                assert_eq!(&refusal.cause, cause, "{label}: clause");

                // Step 4: as the `decreases` measure of a body-`true` declaration.
                let measure = package(vec![
                    f(),
                    FunctionDeclaration::new(
                        "h",
                        vec![],
                        boolean(),
                        Some(expression.clone()),
                        Expression::Boolean(true),
                    ),
                ])
                .check(CheckingLimits::default())
                .expect_err(&format!("{label}: the measure must be refused"));
                assert!(
                    measure.iter().any(|r| &r.cause == cause),
                    "{label}: measure refusals {measure:?} lack {cause:?}"
                );
            }
        }
    }
}
