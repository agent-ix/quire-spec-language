// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical exact rationals (AD-005, FR-140 loss records).
//!
//! Ported from QSL `value::rational` as part of QSL#213 S-1 (ADR-011 X-1);
//! no edge needed cutting. One adaptation: the unmetered inherent
//! `add`/`sub`/`neg`/`mul`/`div`/`pow` arithmetic methods are dropped. They
//! had no caller anywhere in the kernel crate -- [`crate::numeric`]'s
//! metered `evaluate_rational_arithmetic` computes over the numerator and
//! denominator directly rather than through them -- and exposing them as a
//! `pub` convenience would let a caller compute rational arithmetic while
//! silently bypassing every `quire.value.accounting/v1` charge, which is the
//! one property this kernel's accounting types exist to make impossible to
//! skip by construction. `RationalDomain::contains_domain`/`excludes_zero`/
//! `negated`/`result_of` are, by contrast, widened from `pub(crate)` to
//! `pub` below: none of them perform arithmetic outside a charge already
//! taken by their caller.

use std::cmp::Ordering;
use std::fmt;

use crate::integer::{Integer, IntegerInterval};
use crate::numeric::ArithmeticOperator;

/// A reduced rational: positive denominator, `gcd(numerator, denominator) = 1`,
/// and zero is exactly `0/1`. Construction is the only way to obtain one.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Rational {
    numerator: Integer,
    denominator: Integer,
}

/// A rational with a zero denominator was requested.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("zero rational denominator")]
pub struct ZeroDenominator;

impl Rational {
    /// Normalize `numerator/denominator`, refusing a zero denominator.
    pub fn new(numerator: Integer, denominator: Integer) -> Result<Self, ZeroDenominator> {
        if denominator.is_zero() {
            return Err(ZeroDenominator);
        }
        Ok(Self::reduce(numerator, denominator))
    }

    /// Reduce a fraction whose denominator is known to be nonzero.
    fn reduce(numerator: Integer, denominator: Integer) -> Self {
        if numerator.is_zero() {
            return Self::from_integer(Integer::zero());
        }
        let divisor = numerator.gcd(&denominator);
        let (mut numerator, mut denominator) = (
            numerator.exact_div(&divisor),
            denominator.exact_div(&divisor),
        );
        if denominator.is_negative() {
            numerator = numerator.neg();
            denominator = denominator.neg();
        }
        Self {
            numerator,
            denominator,
        }
    }

    /// The exact integer `value/1`.
    pub fn from_integer(value: Integer) -> Self {
        Self {
            numerator: value,
            denominator: Integer::one(),
        }
    }

    /// Reduced signed numerator.
    pub fn numerator(&self) -> &Integer {
        &self.numerator
    }

    /// Reduced positive denominator.
    pub fn denominator(&self) -> &Integer {
        &self.denominator
    }

    /// Whether the value is an integer.
    pub fn is_integer(&self) -> bool {
        self.denominator == Integer::one()
    }

    /// The exact value `self / 10^exponent`.
    pub fn divided_by_power_of_ten(&self, exponent: u64) -> Self {
        let denominator = self.denominator.mul(&Integer::power_of_ten(exponent));
        let divisor = self.numerator.gcd(&denominator);
        Self {
            numerator: self.numerator.exact_div(&divisor),
            denominator: denominator.exact_div(&divisor),
        }
    }

    /// Whether the value is zero.
    pub fn is_zero(&self) -> bool {
        self.numerator.is_zero()
    }

    /// The exact value `self / 2^exponent` (IEEE exact conversions). Total: the
    /// power-of-two denominator is never zero.
    pub(crate) fn divided_by_power_of_two(&self, exponent: u64) -> Self {
        let power = Integer::from_big(num_bigint::BigInt::from(1_u8) << exponent);
        Self::reduce(self.numerator.clone(), self.denominator.mul(&power))
    }

    /// `maxparts(r)` from `quire.value.accounting/v1`.
    pub fn max_part_bits(&self) -> u64 {
        self.numerator
            .magnitude_bits()
            .max(self.denominator.magnitude_bits())
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Self) -> Ordering {
        // Denominators are positive, so cross multiplication preserves order.
        self.numerator
            .mul(&other.denominator)
            .cmp(&other.numerator.mul(&self.denominator))
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}", self.numerator, self.denominator)
    }
}

/// A grammar-named `Rational[lo, hi; dmin, dmax]` domain: the reduced
/// numerator lies in `[lo, hi]` and the positive denominator in
/// `[dmin, dmax]`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RationalDomain {
    numerator: IntegerInterval,
    denominator: IntegerInterval,
}

/// A rational domain's denominator interval admits a denominator below one.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("rational denominator bound below one")]
pub struct NonPositiveDenominatorBound;

impl RationalDomain {
    /// Construct the domain, refusing a denominator interval that reaches
    /// below one, since a reduced denominator is always positive.
    pub fn new(
        numerator: IntegerInterval,
        denominator: IntegerInterval,
    ) -> Result<Self, NonPositiveDenominatorBound> {
        if denominator.lower() < &Integer::one() {
            return Err(NonPositiveDenominatorBound);
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    /// The reduced-numerator interval.
    pub fn numerator(&self) -> &IntegerInterval {
        &self.numerator
    }

    /// The positive-denominator interval.
    pub fn denominator(&self) -> &IntegerInterval {
        &self.denominator
    }

    /// Whether the reduced `value` is a member.
    pub fn contains(&self, value: &Rational) -> bool {
        self.numerator.contains(value.numerator()) && self.denominator.contains(value.denominator())
    }

    /// Whether every member of `other` is a member of `self`.
    pub fn contains_domain(&self, other: &Self) -> bool {
        let within = |outer: &IntegerInterval, inner: &IntegerInterval| {
            outer.lower() <= inner.lower() && inner.upper() <= outer.upper()
        };
        within(&self.numerator, &other.numerator) && within(&self.denominator, &other.denominator)
    }

    /// Whether no member is zero.
    pub fn excludes_zero(&self) -> bool {
        let zero = Integer::zero();
        self.numerator.lower() > &zero || self.numerator.upper() < &zero
    }

    /// The domain of every reduced `-x` for a member `x`: negation keeps the
    /// denominator and negates the numerator.
    pub fn negated(&self) -> Self {
        Self {
            numerator: ordered(self.numerator.upper().neg(), self.numerator.lower().neg()),
            denominator: self.denominator.clone(),
        }
    }

    /// A domain containing every reduced `x op y` for members `x` of `self`
    /// and nonzero-divisor members `y` of `other`, derived from interval
    /// arithmetic on the unreduced parts. Reduction keeps the numerator's
    /// sign and never grows either part, so the numerator lies between zero
    /// and the unreduced extreme, and the denominator in `[1, max]`.
    pub fn result_of(&self, operator: ArithmeticOperator, other: &Self) -> Self {
        let (numerator, denominator) = match operator {
            ArithmeticOperator::Multiply => (
                product(&self.numerator, &other.numerator),
                self.denominator.upper().mul(other.denominator.upper()),
            ),
            ArithmeticOperator::Add | ArithmeticOperator::Subtract => {
                let left = product(&self.numerator, &other.denominator);
                let right = product(&other.numerator, &self.denominator);
                let right = if operator == ArithmeticOperator::Add {
                    right
                } else {
                    ordered(right.upper().neg(), right.lower().neg())
                };
                (
                    ordered(
                        left.lower().add(right.lower()),
                        left.upper().add(right.upper()),
                    ),
                    self.denominator.upper().mul(other.denominator.upper()),
                )
            }
            ArithmeticOperator::Divide => {
                let scaled = product(&self.numerator, &other.denominator);
                let zero = Integer::zero();
                let numerator = if other.numerator.lower() > &zero {
                    scaled
                } else if other.numerator.upper() < &zero {
                    ordered(scaled.upper().neg(), scaled.lower().neg())
                } else {
                    let magnitude = scaled.lower().abs().max(scaled.upper().abs());
                    ordered(magnitude.neg(), magnitude)
                };
                let divisor = other
                    .numerator
                    .lower()
                    .abs()
                    .max(other.numerator.upper().abs());
                (numerator, self.denominator.upper().mul(&divisor))
            }
        };
        let zero = Integer::zero();
        Self {
            numerator: ordered(
                numerator.lower().clone().min(zero.clone()),
                numerator.upper().clone().max(zero),
            ),
            denominator: ordered(Integer::one(), denominator.max(Integer::one())),
        }
    }
}

/// The interval spanning two ends.
fn ordered(lower: Integer, upper: Integer) -> IntegerInterval {
    IntegerInterval::spanning(lower, upper)
}

/// The hull of every product of a member of `left` and a member of `right`.
fn product(left: &IntegerInterval, right: &IntegerInterval) -> IntegerInterval {
    let [a, b, c, d] = [
        left.lower().mul(right.lower()),
        left.lower().mul(right.upper()),
        left.upper().mul(right.lower()),
        left.upper().mul(right.upper()),
    ];
    let (lower, upper) = (
        a.clone().min(b.clone()).min(c.clone()).min(d.clone()),
        a.max(b).max(c).max(d),
    );
    ordered(lower, upper)
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

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
            IntegerInterval::new(Integer::zero(), Integer::from(10_u64)).unwrap(),
            IntegerInterval::new(Integer::one(), Integer::one()).unwrap(),
        )
        .unwrap();
        assert!(domain.contains(&Rational::from_integer(Integer::from(5_u64))));
        assert!(!domain.contains(&Rational::from_integer(Integer::from(11_u64))));
    }
}
