// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-560: the constructs this backend translates, published as one value.

use qsl_eval::execution::{BackendCoverage, CoverageOperation, CoverageType};
use std::sync::OnceLock;

/// The ADR-012 `Value`-family forms this backend translates.
pub(crate) const FORMS: [&str; 7] = [
    "literal",
    "operator",
    "let",
    "if",
    "call",
    "record",
    "collection",
];

const INTEGER_OPERATORS: [&str; 16] = [
    "+", "-", "*", "negate", "div", "rem", "mod", "/", "<", "<=", ">", ">=", "coerce", "convert",
    "=", "!=",
];

const BOOLEAN_OPERATORS: [&str; 6] = ["and", "or", "implies", "not", "=", "!="];

/// `Rational` and `Decimal`: the four arithmetic operators, negation,
/// ordering and equality; a `Decimal` or `Rational` also converts to a
/// `Decimal`.
const NUMERIC_OPERATORS: [&str; 12] = [
    "+", "-", "*", "/", "negate", "<", "<=", ">", ">=", "=", "!=", "convert",
];

/// A record: equality.
const EQUALITY_OPERATORS: [&str; 2] = ["=", "!="];

/// A collection: equality, its size, membership, flattening, conversion
/// between kinds, and removal of the first or a matching occurrence.
const COLLECTION_OPERATORS: [&str; 8] = [
    "=",
    "!=",
    "size",
    "contains",
    "flatten",
    "convert",
    "drop-first",
    "remove-one",
];

/// `Text`: ordering and equality.
const TEXT_OPERATORS: [&str; 6] = ["<", "<=", ">", ">=", "=", "!="];

/// The constructs this backend translates (FR-560).
///
/// This is the FR-560 coverage floor: `Boolean`, bounded and unbounded
/// `Integer`, `Rational`, `Decimal`, `Text`, record and collection constructs,
/// with `call` to a translated function. The `collection` form is the
/// collection literal, `size`, `contains`, `flatten`, kind conversion,
/// `drop_first`, `remove_one`, the `map`, `filter`, `count`, `sum` and `only`
/// queries, `fold` and `reduce`. The binder queries `forall` and `exists` are
/// named by the floor as outside it, and `seq` (admitted by the checker only
/// in a refinement row) is its own form, not listed.
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
            over(CoverageType::Record, &EQUALITY_OPERATORS),
            over(CoverageType::Collection, &COLLECTION_OPERATORS),
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
                CoverageType::Record,
                CoverageType::Collection,
            ],
            operations,
        )
    })
}
