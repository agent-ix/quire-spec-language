// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical exact rationals (AD-005, FR-140 loss records).

use std::cmp::Ordering;
use std::fmt;

use super::integer::{Integer, IntegerInterval};

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
        if numerator.is_zero() {
            return Ok(Self::from_integer(Integer::zero()));
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
        Ok(Self {
            numerator,
            denominator,
        })
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

    /// The exact value `self / 2^exponent` (IEEE exact conversions). Total: the
    /// power-of-two denominator is never zero.
    pub(crate) fn divided_by_power_of_two(&self, exponent: u64) -> Self {
        let power = Integer::from_big(num_bigint::BigInt::from(1_u8) << exponent);
        let denominator = self.denominator.mul(&power);
        let divisor = self.numerator.gcd(&denominator);
        Self {
            numerator: self.numerator.exact_div(&divisor),
            denominator: denominator.exact_div(&divisor),
        }
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
}
