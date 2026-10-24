// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact rational helpers of the schedulability engine.

use num_bigint::BigInt;
use num_integer::Integer as NumInteger;
use quire_exact::{Integer, Rational};

pub(super) fn int(value: u64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

fn from_big(value: BigInt) -> Rational {
    Rational::from_integer(Integer::from_big(value))
}

/// `numerator / denominator`. A period is positive by the task set's
/// admission, so the quotient always exists.
pub(super) fn quotient(numerator: &Rational, denominator: &Rational) -> Rational {
    numerator.div(denominator).unwrap_or_else(|| int(0))
}

/// The least integer at or above `value`.
pub(super) fn ceil(value: &Rational) -> Rational {
    from_big(
        value
            .numerator()
            .as_big()
            .div_ceil(value.denominator().as_big()),
    )
}

/// The greatest integer at or below `value`.
pub(super) fn floor(value: &Rational) -> Rational {
    from_big(
        value
            .numerator()
            .as_big()
            .div_floor(value.denominator().as_big()),
    )
}
