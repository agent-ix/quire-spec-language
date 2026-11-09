// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-560: the constructs this backend translates, published as one value.

use qsl_eval::execution::{BackendCoverage, CoverageOperation, CoverageType};
use std::sync::OnceLock;

/// The ADR-012 `Value`-family forms this backend translates.
pub(crate) const FORMS: [&str; 4] = ["literal", "operator", "let", "if"];

const INTEGER_OPERATORS: [&str; 12] = [
    "+", "-", "*", "negate", "div", "rem", "mod", "<", "<=", ">", ">=", "coerce",
];

/// The constructs this backend translates (FR-560).
///
/// This is the implemented subset of the FR-560 coverage floor: the scalar
/// `Boolean` and `Integer` constructs. The floor's `call`, `record` and
/// `collection` forms, its `Rational`, `Decimal`, `Text`, record and
/// collection types, and the equality and boolean-connective operators are
/// not yet translated, so they are not listed and an entry holding one is
/// listed unsupported.
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
        BackendCoverage::new(
            FORMS,
            [
                CoverageType::Boolean,
                CoverageType::BoundedInteger,
                CoverageType::UnboundedInteger,
            ],
            operations,
        )
    })
}
