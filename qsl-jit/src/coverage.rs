// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-560: the constructs this backend translates, published as one value.

use qsl_eval::execution::{BackendCoverage, CoverageOperation, CoverageType};
use std::sync::OnceLock;

/// The ADR-012 `Value`-family forms this backend translates.
pub(crate) const FORMS: [&str; 5] = ["literal", "operator", "let", "if", "call"];

const INTEGER_OPERATORS: [&str; 15] = [
    "+", "-", "*", "negate", "div", "rem", "mod", "/", "<", "<=", ">", ">=", "coerce", "=", "!=",
];

const BOOLEAN_OPERATORS: [&str; 6] = ["and", "or", "implies", "not", "=", "!="];

/// `Rational` and `Decimal`: the four arithmetic operators, negation,
/// ordering and equality; a `Decimal` or `Rational` also converts to a
/// `Decimal`.
const NUMERIC_OPERATORS: [&str; 12] = [
    "+", "-", "*", "/", "negate", "<", "<=", ">", ">=", "=", "!=", "convert",
];

/// `Text`: ordering and equality.
const TEXT_OPERATORS: [&str; 6] = ["<", "<=", ">", ">=", "=", "!="];

/// The constructs this backend translates (FR-560).
///
/// This is the implemented subset of the FR-560 coverage floor: `Boolean`,
/// bounded and unbounded `Integer`, `Rational`, `Decimal` and `Text`
/// constructs, with `call` to a translated function. The floor's `record` and
/// `collection` forms and types are not yet translated, so they are not listed
/// and an entry holding one is listed unsupported.
#[must_use]
pub fn coverage() -> &'static BackendCoverage {
    static COVERAGE: OnceLock<BackendCoverage> = OnceLock::new();
    COVERAGE.get_or_init(|| {
        let over = |operand: CoverageType, operators: &'static [&'static str]| {
            operators
                .iter()
                .map(move |operator| CoverageOperation { operator, operand })
                .collect::<Vec<_>>()
        };
        let operations = [
            over(CoverageType::BoundedInteger, &INTEGER_OPERATORS),
            over(CoverageType::UnboundedInteger, &INTEGER_OPERATORS),
            over(CoverageType::Boolean, &BOOLEAN_OPERATORS),
            over(CoverageType::Rational, &NUMERIC_OPERATORS),
            over(CoverageType::Decimal, &NUMERIC_OPERATORS),
            over(CoverageType::Text, &TEXT_OPERATORS),
        ]
        .concat();
        BackendCoverage::new(
            FORMS,
            [
                CoverageType::Boolean,
                CoverageType::BoundedInteger,
                CoverageType::UnboundedInteger,
                CoverageType::Rational,
                CoverageType::Decimal,
                CoverageType::Text,
            ],
            operations,
        )
    })
}
