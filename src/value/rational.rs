// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical exact rationals (AD-005, FR-140 loss records).

use std::cmp::Ordering;
use std::fmt;

use super::integer::Integer;

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

    /// Exact `self + other`.
    pub(crate) fn add(&self, other: &Self) -> Self {
        Self::reduce(
            self.numerator
                .mul(&other.denominator)
                .add(&other.numerator.mul(&self.denominator)),
            self.denominator.mul(&other.denominator),
        )
    }

    /// Exact `self - other`.
    pub(crate) fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    /// Exact `-self`.
    pub(crate) fn neg(&self) -> Self {
        Self {
            numerator: self.numerator.neg(),
            denominator: self.denominator.clone(),
        }
    }

    /// Exact `self × other`.
    pub(crate) fn mul(&self, other: &Self) -> Self {
        Self::reduce(
            self.numerator.mul(&other.numerator),
            self.denominator.mul(&other.denominator),
        )
    }

    /// Exact `self / other`, or `None` for a zero divisor.
    pub(crate) fn div(&self, other: &Self) -> Option<Self> {
        (!other.is_zero()).then(|| {
            Self::reduce(
                self.numerator.mul(&other.denominator),
                self.denominator.mul(&other.numerator),
            )
        })
    }

    /// Exact `self^exponent`, or `None` for zero raised to a negative power.
    /// Callers bound the result size before calling.
    pub(crate) fn pow(&self, exponent: &Integer) -> Option<Self> {
        let (numerator, denominator) =
            (self.numerator.pow(exponent), self.denominator.pow(exponent));
        if exponent.is_negative() {
            (!numerator.is_zero()).then(|| Self::reduce(denominator, numerator))
        } else {
            Some(Self::reduce(numerator, denominator))
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
