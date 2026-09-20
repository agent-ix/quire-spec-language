// SPDX-License-Identifier: AGPL-3.0-or-later
//! Mathematical integers and explicit inclusive integer domains (AD-005, FR-147).
//!
//! `Integer` is unbounded. A finite consumer never narrows it: membership in an
//! [`IntegerInterval`] is an explicit admission that either returns a
//! [`BoundedInteger`] or refuses.
//!
//! Ported verbatim from QSL `value::integer` as part of QSL#213 S-1
//! (ADR-011 X-1); no edge needed cutting.

use std::fmt;
use std::num::NonZeroU32;
use std::str::FromStr;

use num_bigint::{BigInt, BigUint};
use num_integer::Integer as _;
use num_traits::{One, Signed, Zero};

use crate::accounting::length_amount;

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
        length_amount(self.0.magnitude().to_str_radix(10).len())
    }

    /// The value as a `u64`, if it is one.
    pub fn to_u64(&self) -> Option<u64> {
        u64::try_from(&self.0).ok()
    }

    /// The magnitude `|self|`.
    pub fn abs(&self) -> Self {
        Self(self.0.abs())
    }

    /// `self^|exponent|`. Callers bound the result size before calling.
    pub fn pow(&self, exponent: &Self) -> Self {
        Self(num_traits::Pow::pow(&self.0, exponent.0.magnitude()))
    }

    /// `(self / 2^k, k)` for the greatest `k <= limit` with `2^k | self`.
    /// Zero has no greatest such `k` and is returned with `k = 0`.
    pub fn split_factor_two(&self, limit: u64) -> (Self, u64) {
        match self.0.trailing_zeros() {
            None => (self.clone(), 0),
            Some(zeros) => {
                let shift = zeros.min(limit);
                (Self(&self.0 >> shift), shift)
            }
        }
    }

    /// `self × 2^shift`. Callers bound the result size before calling.
    pub fn shifted_left(&self, shift: u64) -> Self {
        Self(&self.0 << shift)
    }

    /// Whether this integer is even.
    pub fn is_even(&self) -> bool {
        self.0.is_even()
    }

    pub fn add(&self, other: &Self) -> Self {
        Self(&self.0 + &other.0)
    }

    pub fn sub(&self, other: &Self) -> Self {
        Self(&self.0 - &other.0)
    }

    pub fn mul(&self, other: &Self) -> Self {
        Self(&self.0 * &other.0)
    }

    pub fn neg(&self) -> Self {
        Self(-&self.0)
    }

    pub fn gcd(&self, other: &Self) -> Self {
        Self(self.0.gcd(&other.0))
    }

    /// Exact quotient of a division known to be exact; `divisor` is nonzero.
    pub fn exact_div(&self, divisor: &Self) -> Self {
        Self(&self.0 / &divisor.0)
    }

    /// Truncating quotient/remainder; `divisor` is nonzero.
    pub fn div_rem_truncating(&self, divisor: &Self) -> (Self, Self) {
        let (quotient, remainder) = self.0.div_rem(&divisor.0);
        (Self(quotient), Self(remainder))
    }

    /// Floor quotient/remainder; `divisor` is nonzero.
    pub fn div_mod_floor(&self, divisor: &Self) -> (Self, Self) {
        let (quotient, remainder) = self.0.div_mod_floor(&divisor.0);
        (Self(quotient), Self(remainder))
    }

    /// Exact `10^exponent`.
    pub fn power_of_ten(exponent: u64) -> Self {
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

    /// `bits(|factor| × |base|^exponent)` for a nonnegative `exponent`, derived
    /// without materializing the power.
    ///
    /// A power-of-two base is exact by shifting. Otherwise the power is bracketed
    /// by truncated lower and upper `P`-bit mantissas under a shared binary
    /// exponent, and `P` doubles until both brackets have one bit length. The
    /// product is then not a power of two, so a finite precision separates it
    /// from the nearest power of two and the loop terminates.
    pub fn power_product_bits(factor: &Self, base: &Self, exponent: &Self) -> Self {
        let factor = factor.0.magnitude();
        let base = base.0.magnitude();
        let exponent = exponent.0.magnitude();
        let factor_bits = BigUint::from(factor.bits().max(1));
        if factor.is_zero() || base.is_zero() && !exponent.is_zero() {
            return Self::one();
        }
        if exponent.is_zero() || base.is_one() {
            return Self(BigInt::from(factor_bits));
        }
        if base.count_ones() == 1 {
            let shift = BigUint::from(base.bits() - 1);
            return Self(BigInt::from(factor_bits + shift * exponent));
        }
        let mut precision = 64_u64;
        loop {
            let (low, high) = bracket_bits(factor, base, exponent, precision);
            if low == high {
                return Self(BigInt::from(low));
            }
            precision = precision.saturating_mul(2);
        }
    }

    /// `2^exponent`.
    fn power_of_two(exponent: u32) -> Self {
        Self(BigInt::one() << exponent)
    }
}

/// Bit lengths of a lower and upper bound of `factor × base^exponent`, each kept
/// to at most `precision` mantissa bits.
fn bracket_bits(
    factor: &BigUint,
    base: &BigUint,
    exponent: &BigUint,
    precision: u64,
) -> (BigUint, BigUint) {
    let (low, high, shift) = bracket(factor, base, exponent, precision);
    (
        BigUint::from(low.bits()) + &shift,
        BigUint::from(high.bits()) + shift,
    )
}

/// `(low, high, shift)` with `low × 2^shift <= factor × base^exponent <=
/// high × 2^shift`, the power's mantissas kept to at most `precision` bits.
/// With no truncation `low == high` is the exact value and `shift` is zero.
fn bracket(
    factor: &BigUint,
    base: &BigUint,
    exponent: &BigUint,
    precision: u64,
) -> (BigUint, BigUint, BigUint) {
    let mut low = BigUint::one();
    let mut high = BigUint::one();
    let mut shift = BigUint::zero();
    let truncate = |low: &mut BigUint, high: &mut BigUint, shift: &mut BigUint| {
        let excess = high.bits().saturating_sub(precision);
        if excess > 0 {
            *low >>= excess;
            *high = (&*high + ((BigUint::one() << excess) - 1_u8)) >> excess;
            *shift += excess;
        }
    };
    for bit in (0..exponent.bits()).rev() {
        low = &low * &low;
        high = &high * &high;
        shift = &shift << 1_u8;
        truncate(&mut low, &mut high, &mut shift);
        if exponent.bit(bit) {
            low *= base;
            high *= base;
            truncate(&mut low, &mut high, &mut shift);
        }
    }
    low *= factor;
    high *= factor;
    (low, high, shift)
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

impl From<usize> for Integer {
    fn from(value: usize) -> Self {
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

    /// The smallest interval containing both `a` and `b`, in either order.
    pub fn spanning(a: Integer, b: Integer) -> Self {
        if a <= b {
            Self { lower: a, upper: b }
        } else {
            Self { lower: b, upper: a }
        }
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

impl Integer {
    /// Wrap an arbitrary-precision integer (IEEE exact conversions).
    pub fn from_big(value: BigInt) -> Self {
        Self(value)
    }

    /// The arbitrary-precision integer (IEEE exact conversions).
    pub fn as_big(&self) -> &BigInt {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use ix_trace_rs::trace;

    use super::*;

    /// TC-329: `IntegerInterval::admit` accepts a value inside the declared
    /// domain, and `BoundedInteger` reports back the exact value and domain
    /// that admitted it (ADR-013 O-21 kernel bound value type).
    #[trace("TC-329")]
    #[test]
    fn tc_329_bounded_integer_admits_within_domain() {
        let domain = IntegerInterval::new(Integer::zero(), Integer::from(10_u64)).unwrap();
        let admitted = domain.admit(Integer::from(7_u64)).unwrap();
        assert_eq!(admitted.value(), &Integer::from(7_u64));
        assert_eq!(admitted.domain(), &domain);
    }

    /// TC-330: a value outside the declared domain is refused with
    /// `OutOfDomain`, never silently narrowed or saturated.
    #[trace("TC-330")]
    #[test]
    fn tc_330_bounded_integer_refuses_outside_domain() {
        let domain = IntegerInterval::new(Integer::zero(), Integer::from(10_u64)).unwrap();
        assert_eq!(domain.admit(Integer::from(11_u64)), Err(OutOfDomain));
    }

    /// TC-331: parsing round-trips through `Display` for a canonical
    /// decimal integer literal, negative sign included.
    #[trace("TC-331")]
    #[test]
    fn tc_331_integer_parse_display_round_trips() {
        let parsed = Integer::from_str("-42").unwrap();
        assert_eq!(parsed.to_string(), "-42");
    }
}
