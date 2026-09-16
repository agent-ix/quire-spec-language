// SPDX-License-Identifier: AGPL-3.0-or-later
//! Mathematical integers and explicit inclusive integer domains (AD-005, FR-147).
//!
//! `Integer` is unbounded. A finite consumer never narrows it: membership in an
//! [`IntegerInterval`] is an explicit admission that either returns a
//! [`BoundedInteger`] or refuses.

use std::fmt;
use std::num::NonZeroU32;
use std::str::FromStr;

use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::{One, Signed, Zero};

/// An exact, arbitrary-precision mathematical integer.
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Integer(BigInt);

impl Integer {
    /// The integer zero.
    pub fn zero() -> Self {
        Self(BigInt::zero())
    }

    /// The integer one.
    pub fn one() -> Self {
        Self(BigInt::one())
    }

    /// Whether this integer is zero.
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// Whether this integer is strictly negative.
    pub fn is_negative(&self) -> bool {
        self.0.is_negative()
    }

    /// `bits(x)` from `quire.value.accounting/v1`: the magnitude bit length,
    /// where zero has length one.
    pub fn magnitude_bits(&self) -> u64 {
        self.0.bits().max(1)
    }

    /// `digits(x)` from `quire.value.accounting/v1`: the base-ten magnitude
    /// digit count, where zero has one digit.
    pub fn decimal_digits(&self) -> u64 {
        let length = self.0.magnitude().to_str_radix(10).len();
        // A `usize` length always fits `u64` on every supported target.
        u64::try_from(length).unwrap_or(u64::MAX)
    }

    /// Whether this integer is even.
    pub fn is_even(&self) -> bool {
        self.0.is_even()
    }

    pub(crate) fn add(&self, other: &Self) -> Self {
        Self(&self.0 + &other.0)
    }

    pub(crate) fn sub(&self, other: &Self) -> Self {
        Self(&self.0 - &other.0)
    }

    pub(crate) fn mul(&self, other: &Self) -> Self {
        Self(&self.0 * &other.0)
    }

    pub(crate) fn neg(&self) -> Self {
        Self(-&self.0)
    }

    pub(crate) fn gcd(&self, other: &Self) -> Self {
        Self(self.0.gcd(&other.0))
    }

    /// Exact quotient of a division known to be exact; `divisor` is nonzero.
    pub(crate) fn exact_div(&self, divisor: &Self) -> Self {
        Self(&self.0 / &divisor.0)
    }

    /// Truncating quotient/remainder; `divisor` is nonzero.
    pub(crate) fn div_rem_truncating(&self, divisor: &Self) -> (Self, Self) {
        let (quotient, remainder) = self.0.div_rem(&divisor.0);
        (Self(quotient), Self(remainder))
    }

    /// Floor quotient/remainder; `divisor` is nonzero.
    pub(crate) fn div_mod_floor(&self, divisor: &Self) -> (Self, Self) {
        let (quotient, remainder) = self.0.div_mod_floor(&divisor.0);
        (Self(quotient), Self(remainder))
    }

    /// Exact `10^exponent`.
    pub(crate) fn power_of_ten(exponent: u64) -> Self {
        let mut result = BigInt::one();
        let mut base = BigInt::from(10_u8);
        let mut remaining = exponent;
        while remaining > 0 {
            if remaining & 1 == 1 {
                result *= &base;
            }
            remaining >>= 1;
            if remaining > 0 {
                base = &base * &base;
            }
        }
        Self(result)
    }

    /// `2^exponent`.
    fn power_of_two(exponent: u32) -> Self {
        Self(BigInt::one() << exponent)
    }
}

impl From<i64> for Integer {
    fn from(value: i64) -> Self {
        Self(BigInt::from(value))
    }
}

impl From<i128> for Integer {
    fn from(value: i128) -> Self {
        Self(BigInt::from(value))
    }
}

impl From<u64> for Integer {
    fn from(value: u64) -> Self {
        Self(BigInt::from(value))
    }
}

impl fmt::Display for Integer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, formatter)
    }
}

/// The canonical integer spelling `^(0|-?[1-9][0-9]*)$` was not supplied.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("non-canonical integer spelling")]
pub struct NonCanonicalInteger;

impl FromStr for Integer {
    type Err = NonCanonicalInteger;

    /// Parse the canonical wire spelling used by complete-V1 schemas.
    fn from_str(spelling: &str) -> Result<Self, Self::Err> {
        let digits = spelling.strip_prefix('-').unwrap_or(spelling);
        let canonical = match digits.as_bytes() {
            [] => false,
            [b'0'] => digits.len() == spelling.len(),
            [first, rest @ ..] => {
                (b'1'..=b'9').contains(first) && rest.iter().all(u8::is_ascii_digit)
            }
        };
        if !canonical {
            return Err(NonCanonicalInteger);
        }
        BigInt::from_str(spelling)
            .map(Self)
            .map_err(|_| NonCanonicalInteger)
    }
}

/// A nonempty inclusive integer domain `[lower, upper]`.
///
/// This is the finite domain descriptor consumed by bounded backends; it is
/// never inferred from a host integer width.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct IntegerInterval {
    lower: Integer,
    upper: Integer,
}

/// An interval's lower bound exceeds its upper bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("empty integer interval")]
pub struct EmptyInterval;

impl IntegerInterval {
    /// Construct `[lower, upper]`, refusing an empty interval.
    pub fn new(lower: Integer, upper: Integer) -> Result<Self, EmptyInterval> {
        if lower > upper {
            return Err(EmptyInterval);
        }
        Ok(Self { lower, upper })
    }

    /// The two's-complement signed domain of `width` bits,
    /// `[-(2^(width-1)), 2^(width-1) - 1]`.
    pub fn signed_twos_complement(width: NonZeroU32) -> Self {
        let magnitude = Integer::power_of_two(width.get() - 1);
        Self {
            lower: magnitude.neg(),
            upper: magnitude.sub(&Integer::one()),
        }
    }

    /// Inclusive lower bound.
    pub fn lower(&self) -> &Integer {
        &self.lower
    }

    /// Inclusive upper bound.
    pub fn upper(&self) -> &Integer {
        &self.upper
    }

    /// Whether `value` is a member.
    pub fn contains(&self, value: &Integer) -> bool {
        &self.lower <= value && value <= &self.upper
    }

    /// Admit `value` into this domain without narrowing or saturation.
    pub fn admit(&self, value: Integer) -> Result<BoundedInteger, OutOfDomain> {
        if self.contains(&value) {
            Ok(BoundedInteger {
                value,
                domain: self.clone(),
            })
        } else {
            Err(OutOfDomain)
        }
    }
}

/// A value is outside its declared domain. The value is not retained.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("integer outside declared domain")]
pub struct OutOfDomain;

/// An integer admitted into an explicit inclusive domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedInteger {
    value: Integer,
    domain: IntegerInterval,
}

impl BoundedInteger {
    /// The admitted mathematical value.
    pub fn value(&self) -> &Integer {
        &self.value
    }

    /// The domain that admitted it.
    pub fn domain(&self) -> &IntegerInterval {
        &self.domain
    }
}

/// The domain of an integer consumer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntegerDomain {
    /// Unbounded mathematical integers; evaluation has no overflow.
    Mathematical,
    /// A finite consumer that must prove membership of every exposed result.
    Bounded(IntegerInterval),
}

impl IntegerDomain {
    /// Whether `value` belongs to this domain.
    pub fn contains(&self, value: &Integer) -> bool {
        match self {
            Self::Mathematical => true,
            Self::Bounded(interval) => interval.contains(value),
        }
    }
}
