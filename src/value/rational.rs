// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical exact rationals (AD-005, FR-140 loss records).

use std::cmp::Ordering;
use std::fmt;

use super::numeric::ArithmeticOperator;
use quire_exact::{Integer, IntegerInterval};

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
    pub(crate) fn contains_domain(&self, other: &Self) -> bool {
        let within = |outer: &IntegerInterval, inner: &IntegerInterval| {
            outer.lower() <= inner.lower() && inner.upper() <= outer.upper()
        };
        within(&self.numerator, &other.numerator) && within(&self.denominator, &other.denominator)
    }

    /// Whether no member is zero.
    pub(crate) fn excludes_zero(&self) -> bool {
        let zero = Integer::zero();
        self.numerator.lower() > &zero || self.numerator.upper() < &zero
    }

    /// The domain of every reduced `-x` for a member `x`: negation keeps the
    /// denominator and negates the numerator.
    pub(crate) fn negated(&self) -> Self {
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
    pub(crate) fn result_of(&self, operator: ArithmeticOperator, other: &Self) -> Self {
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
