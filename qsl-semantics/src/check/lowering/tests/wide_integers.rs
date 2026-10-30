// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-910 step 1 (FR-092-AC-14) and TC-909 step 4 (FR-091-AC-38): `Int`
//! ranges up to the i128 extremes key to FR-092's T13 to T16, and the folded
//! `i128::MIN` literal keys to L7. The vectors are the ones FR-092 publishes.

use super::*;
use crate::check::{CheckCause, I128Limit, IntegerOutsideI128, IntegerSite};

/// The `Int[lower, upper]` type form with the bounds as written.
fn wide_form(lower: &str, upper: &str) -> TypeForm {
    TypeForm::builtin(BuiltinType::Int, SPAN).with_bounds(vec![lower.to_owned(), upper.to_owned()])
}

/// `g(x: Int[i128::MIN, 0]): Boolean { x >= <literal> }`.
fn compared_with(literal: Integer) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    check(vec![function(
        "g",
        &[(
            "x",
            wide_form("-170141183460469231731687303715884105728", "0"),
        )],
        boolean(),
        None,
        binary(
            BinaryOperator::GreaterOrEqual,
            name_expr("x"),
            Expression::integer(literal),
        ),
    )])
}

/// FR-091-AC-38: a literal one past either i128 limit refuses
/// `IntegerOutsideI128` at the literal, naming the limit it crosses.
#[trace("FR-091-AC-38", "TC-909")]
#[test]
fn a_literal_outside_i128_refuses_naming_the_limit_it_crosses() {
    let above = Integer::from(i128::MAX).add(&Integer::one());
    let below = Integer::from(i128::MIN).sub(&Integer::one());
    for (literal, limit) in [(above, I128Limit::Max), (below, I128Limit::Min)] {
        let refusals = compared_with(literal.clone()).expect_err("the literal is outside i128");
        assert_eq!(refusals.len(), 1, "{literal}");
        assert_eq!(
            refusals[0].cause,
            CheckCause::IntegerOutsideI128(IntegerOutsideI128 {
                site: IntegerSite::Literal,
                value: literal.to_string(),
                limit,
            })
        );
        assert_eq!(refusals[0].cause.code().as_str(), "ill_typed");
        assert_eq!(refusals[0].cause.cause(), Some("type-mismatch"));
    }
}
