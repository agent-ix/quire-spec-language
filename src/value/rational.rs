// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical exact rationals (AD-005, FR-140 loss records).
//!
//! [`Rational`], [`ZeroDenominator`], [`RationalDomain`] and
//! [`NonPositiveDenominatorBound`] are `quire_exact`'s own canonical items,
//! re-exported below rather than duplicated: diffed method-for-method
//! against `quire-exact/src/rational.rs` before the cut, `new`,
//! `from_integer`, `numerator`, `denominator`, `is_integer`, `is_zero`,
//! `max_part_bits`, `Ord`/`PartialOrd`/`Display`, and
//! `RationalDomain::new`/`numerator`/`denominator`/`contains`/
//! `contains_domain`/`excludes_zero`/`negated` are identical and reachable
//! straight off the re-exported type.
//!
//! Two kinds of item are not reachable that way, so a small set of free
//! functions stand in for them below, each computed over the re-exported
//! type's own public API rather than a struct literal, which private fields
//! now forbid:
//!
//! - `quire_exact::Rational` deliberately drops the unmetered inherent
//!   `add`/`sub`/`neg`/`mul`/`div`/`pow` arithmetic (its own module doc:
//!   no caller anywhere in that crate, and a `pub` convenience would let a
//!   caller compute rational arithmetic while bypassing every
//!   `quire.value.accounting/v1` charge). [`super::quantity`]'s unit-graph
//!   arithmetic still needs them, so [`rational_add`], [`rational_sub`],
//!   [`rational_neg`], [`rational_mul`], [`rational_div`] and
//!   [`rational_pow`] recompute the identical reduction through the public
//!   [`Rational::new`] constructor.
//! - `Rational::divided_by_power_of_ten` and
//!   `Rational::divided_by_power_of_two` exist in `quire_exact` too, but
//!   `pub(crate)` there (its own H-5 doc comment: an unmetered
//!   caller-supplied exponent is a real out-of-memory `BigInt` risk) --
//!   invisible outside that crate regardless of a matching body.
//!   [`super::decimal`] and [`super::ieee`] still call them, so
//!   [`rational_divided_by_power_of_ten`] and
//!   [`rational_divided_by_power_of_two`] port the identical computation.
//! - `RationalDomain::result_of` takes an `ArithmeticOperator`, and
//!   `quire_exact`'s own copy of that enum is `pub` there (its own module
//!   doc says why: `RationalDomain::result_of` names it in a public
//!   signature) but a distinct type from this crate's own
//!   `value::numeric::ArithmeticOperator` (`pub(crate)`, out of this
//!   slice's scope). [`rational_domain_result_of`] translates the one enum
//!   value and calls the re-exported type's own, now-reachable, public
//!   `result_of`.
//!
//! Remaining work (Linear QSL-131): [`super::decimal`]'s own evaluation
//! engine (`evaluate_decimal` and everything beneath it) still returns this
//! crate's own `Outcome`/`Refusal` (`value::outcome`), a strict superset of
//! `quire_exact`'s kernel `Outcome`/`Refusal`, and constructs
//! `DecimalResult`/`DecimalLoss` only through their own private struct
//! literals -- neither type has a public constructor in `quire_exact` -- so
//! that engine cannot be cut until `value::outcome` unifies with
//! `quire_exact::outcome` (QSL-166, QSL-174).

use super::numeric::ArithmeticOperator;
pub use quire_exact::{NonPositiveDenominatorBound, Rational, RationalDomain, ZeroDenominator};

/// Exact `left + right`, ported from `quire_exact::Rational`'s dropped
/// private `add` (see the module doc): recomputed through the public
/// [`Rational::new`], which performs the identical reduction.
pub(crate) fn rational_add(left: &Rational, right: &Rational) -> Rational {
    Rational::new(
        left.numerator()
            .mul(right.denominator())
            .add(&right.numerator().mul(left.denominator())),
        left.denominator().mul(right.denominator()),
    )
    .expect("a product of two positive denominators is never zero")
}

/// Exact `left - right`.
pub(crate) fn rational_sub(left: &Rational, right: &Rational) -> Rational {
    rational_add(left, &rational_neg(right))
}

/// Exact `-value`.
pub(crate) fn rational_neg(value: &Rational) -> Rational {
    Rational::new(value.numerator().neg(), value.denominator().clone())
        .expect("negating the numerator leaves the always-positive denominator unchanged")
}

/// Exact `left * right`.
pub(crate) fn rational_mul(left: &Rational, right: &Rational) -> Rational {
    Rational::new(
        left.numerator().mul(right.numerator()),
        left.denominator().mul(right.denominator()),
    )
    .expect("a product of two positive denominators is never zero")
}

/// Exact `left / right`, or `None` for a zero divisor.
pub(crate) fn rational_div(left: &Rational, right: &Rational) -> Option<Rational> {
    if right.is_zero() {
        return None;
    }
    Some(
        Rational::new(
            left.numerator().mul(right.denominator()),
            left.denominator().mul(right.numerator()),
        )
        .expect("right is confirmed nonzero, and left's denominator is always positive"),
    )
}

/// Exact `value^exponent`, or `None` for zero raised to a negative power.
/// Callers bound the result size before calling.
pub(crate) fn rational_pow(value: &Rational, exponent: &quire_exact::Integer) -> Option<Rational> {
    let numerator = value.numerator().pow(exponent);
    let denominator = value.denominator().pow(exponent);
    if exponent.is_negative() {
        if numerator.is_zero() {
            return None;
        }
        Some(Rational::new(denominator, numerator).expect("numerator is confirmed nonzero above"))
    } else {
        Some(
            Rational::new(numerator, denominator)
                .expect("a positive base's denominator stays positive at any nonnegative power"),
        )
    }
}

/// The exact value `value / 10^exponent`.
pub(crate) fn rational_divided_by_power_of_ten(value: &Rational, exponent: u64) -> Rational {
    let denominator = value
        .denominator()
        .mul(&quire_exact::Integer::power_of_ten(exponent));
    Rational::new(value.numerator().clone(), denominator)
        .expect("a positive denominator times a positive power of ten is never zero")
}

/// The exact value `value / 2^exponent` (IEEE exact conversions). Total: the
/// power-of-two denominator is never zero.
pub(crate) fn rational_divided_by_power_of_two(value: &Rational, exponent: u64) -> Rational {
    let power = quire_exact::Integer::from_big(num_bigint::BigInt::from(1_u8) << exponent);
    Rational::new(value.numerator().clone(), value.denominator().mul(&power))
        .expect("a positive denominator times a power of two is never zero")
}

/// `domain.result_of(operator, other)`, translating this crate's own
/// `value::numeric::ArithmeticOperator` onto `quire_exact`'s same-named but
/// distinct enum (see the module doc) so the re-exported
/// [`RationalDomain::result_of`] can be called with it.
pub(crate) fn rational_domain_result_of(
    domain: &RationalDomain,
    operator: ArithmeticOperator,
    other: &RationalDomain,
) -> RationalDomain {
    let operator = match operator {
        ArithmeticOperator::Add => quire_exact::ArithmeticOperator::Add,
        ArithmeticOperator::Subtract => quire_exact::ArithmeticOperator::Subtract,
        ArithmeticOperator::Multiply => quire_exact::ArithmeticOperator::Multiply,
        ArithmeticOperator::Divide => quire_exact::ArithmeticOperator::Divide,
    };
    domain.result_of(operator, other)
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_exact::Integer;

    use super::*;

    /// TC-332: `Rational::new` reduces to lowest terms and normalizes the
    /// sign onto the numerator, so `2/4` and `-1/-2` both construct the same
    /// canonical `1/2`.
    #[trace("TC-332")]
    #[test]
    fn tc_332_new_reduces_and_normalizes_sign() {
        let two_fourths = Rational::new(Integer::from(2_u64), Integer::from(4_u64)).unwrap();
        let neg_over_neg = Rational::new(
            Integer::zero().sub(&Integer::one()),
            Integer::zero().sub(&Integer::from(2_u64)),
        )
        .unwrap();
        let half = Rational::new(Integer::one(), Integer::from(2_u64)).unwrap();
        assert_eq!(two_fourths, half);
        assert_eq!(neg_over_neg, half);
    }

    /// TC-333: a zero denominator is refused, never silently treated as an
    /// undefined or infinite value.
    #[trace("TC-333")]
    #[test]
    fn tc_333_zero_denominator_is_refused() {
        assert_eq!(
            Rational::new(Integer::one(), Integer::zero()),
            Err(ZeroDenominator)
        );
    }

    /// TC-334: `RationalDomain::contains` is exactly the closed
    /// numerator/denominator interval product membership test.
    #[trace("TC-334")]
    #[test]
    fn tc_334_domain_contains_checks_both_intervals() {
        let domain = RationalDomain::new(
            quire_exact::IntegerInterval::new(Integer::zero(), Integer::from(10_u64)).unwrap(),
            quire_exact::IntegerInterval::new(Integer::one(), Integer::one()).unwrap(),
        )
        .unwrap();
        assert!(domain.contains(&Rational::from_integer(Integer::from(5_u64))));
        assert!(!domain.contains(&Rational::from_integer(Integer::from(11_u64))));
    }
}
