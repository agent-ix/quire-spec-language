// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-560: the constructs this backend translates, published as one value.

use qsl_eval::execution::{BackendCoverage, CoverageOperation, CoverageType};
use std::sync::OnceLock;

/// The ADR-012 `Value`-family forms this backend translates.
pub(crate) const FORMS: [&str; 5] = ["literal", "operator", "let", "if", "call"];

const INTEGER_OPERATORS: [&str; 14] = [
    "+", "-", "*", "negate", "div", "rem", "mod", "<", "<=", ">", ">=", "coerce", "=", "!=",
];

const BOOLEAN_OPERATORS: [&str; 6] = ["and", "or", "implies", "not", "=", "!="];

/// The constructs this backend translates (FR-560).
///
/// This is the implemented subset of the FR-560 coverage floor: the scalar
/// `Boolean` and `Integer` constructs, bounded and unbounded (the latter
/// through the arena handles of QSL-1825), with `call` to a translated
/// function. The floor's `record` and `collection` forms, its `Rational`,
/// `Decimal`, `Text`, record and collection types, and the operations over
/// those types are not yet translated, so they are not listed and an entry
/// holding one is listed unsupported.
#[must_use]
pub fn coverage() -> &'static BackendCoverage {
    static COVERAGE: OnceLock<BackendCoverage> = OnceLock::new();
    COVERAGE.get_or_init(|| {
        let operations = [CoverageType::BoundedInteger, CoverageType::UnboundedInteger]
            .into_iter()
            .flat_map(|operand| {
                INTEGER_OPERATORS
                    .into_iter()
                    .map(move |operator| CoverageOperation { operator, operand })
            });
        let booleans = BOOLEAN_OPERATORS
            .into_iter()
            .map(|operator| CoverageOperation {
                operator,
                operand: CoverageType::Boolean,
            });
        BackendCoverage::new(
            FORMS,
            [
                CoverageType::Boolean,
                CoverageType::BoundedInteger,
                CoverageType::UnboundedInteger,
            ],
            operations.chain(booleans),
        )
    })
}
