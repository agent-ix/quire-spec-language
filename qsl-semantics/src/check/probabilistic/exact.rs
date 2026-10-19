// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-185 "Exact values": an `exact-value` measured as written, bounded by
//! the rational-bits ceiling, then reduced to an exact rational in lowest
//! terms.
//!
//! The measure is the largest bit length among the written integers, where a
//! `decimal(m, s)` and a decimal literal also count the bit length of their
//! denominator `10^s`, `⌊s · log₂ 10⌋ + 1`. That length is computed from `s`
//! in integer arithmetic, so a scale of any size is measured without
//! expanding `10^s`. Only a value within the ceiling is reduced.

use qsl_forms::{ExactValueForm, ExactValueKind};
use quire_exact::{Integer, Rational};

/// FR-185's default rational-bits ceiling, inclusive. The `CheckingLimits`
/// setting that lets a caller change it, and the limit refusal past it, wait
/// on QSL-883.
pub(crate) const RATIONAL_BITS_CEILING: u64 = 65_536;

/// A lower bound of `log₂ 10`, as `LOG2_10_LOWER / 10^28`, truncated from
/// `3.32192809488736234787031942948…`.
const LOG2_10_LOWER: u128 = 33_219_280_948_873_623_478_703_194_294;

/// `10^28`, the denominator of [`LOG2_10_LOWER`].
const LOG2_10_SCALE: u128 = 10_000_000_000_000_000_000_000_000_000;

/// Why an exact value has no rational S3 admits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExactFault {
    /// A `rational(n, 0)`.
    ZeroDenominator,
    /// The value as written is past [`RATIONAL_BITS_CEILING`]; `bits` is its
    /// measure (a lower bound once a decimal scale reaches `2^32`, where
    /// every measure is past the ceiling).
    AboveCeiling {
        /// The measure.
        bits: u64,
    },
}

/// The bit length of `10^scale`: `⌊scale · log₂ 10⌋ + 1`.
///
/// Below `2^32` the truncated bound is exact: its error is under
/// `scale · 10^-28 < 5 · 10^-19`, and no `scale` below `2^32` puts
/// `scale · log₂ 10` within `4 · 10^-11` of an integer (the nearest is the
/// continued-fraction convergent denominator 579,001,193), so the floor of
/// the bound is the floor of the product. From `2^32` up, `3 · scale` is a
/// lower bound past the ceiling.
fn power_of_ten_bits(scale: &Integer) -> u64 {
    let Some(scale) = scale.to_u64() else {
        return u64::MAX;
    };
    match u32::try_from(scale) {
        Ok(scale) => {
            let product = u128::from(scale) * LOG2_10_LOWER / LOG2_10_SCALE;
            // `product` is below `2^32 · 4`, so it fits.
            u64::try_from(product).unwrap_or(u64::MAX).saturating_add(1)
        }
        Err(_) => scale.saturating_mul(3),
    }
}

/// FR-185's measure of `value` as written.
pub(crate) fn written_bits(value: &ExactValueForm) -> u64 {
    let first = value.first.magnitude_bits();
    let second = match value.kind {
        ExactValueKind::Natural => 0,
        ExactValueKind::Rational => value.second.magnitude_bits(),
        ExactValueKind::Decimal | ExactValueKind::DecimalLiteral => {
            power_of_ten_bits(&value.second)
        }
    };
    first.max(second)
}

/// `value` reduced to lowest terms, once its measure is within the ceiling.
pub(crate) fn value(value: &ExactValueForm) -> Result<Rational, ExactFault> {
    let bits = written_bits(value);
    if bits > RATIONAL_BITS_CEILING {
        return Err(ExactFault::AboveCeiling { bits });
    }
    match value.kind {
        ExactValueKind::Natural => Ok(Rational::from_integer(value.first.clone())),
        ExactValueKind::Rational => Rational::new(value.first.clone(), value.second.clone())
            .map_err(|_| ExactFault::ZeroDenominator),
        ExactValueKind::Decimal | ExactValueKind::DecimalLiteral => {
            // Within the ceiling, the scale is below 19,729.
            let scale = value
                .second
                .to_u64()
                .ok_or(ExactFault::AboveCeiling { bits })?;
            Ok(Rational::from_integer(value.first.clone()).divided_by_power_of_ten(scale))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-185 "Exact values": the computed bit length of `10^s` equals the
    /// bit length of the expanded power for every scale up to past the
    /// ceiling.
    #[test]
    fn the_power_of_ten_bit_length_matches_the_expanded_power() {
        for scale in (0..=20_000_u64).chain([65_536, 100_000]) {
            assert_eq!(
                power_of_ten_bits(&Integer::from(scale)),
                Integer::power_of_ten(scale).magnitude_bits(),
                "10^{scale}"
            );
        }
        assert_eq!(power_of_ten_bits(&Integer::from(0_u64)), 1);
        assert_eq!(power_of_ten_bits(&Integer::from(19_728_u64)), 65_535);
        assert_eq!(power_of_ten_bits(&Integer::from(19_729_u64)), 65_539);
        assert!(power_of_ten_bits(&Integer::from(4_294_967_295_u64)) > RATIONAL_BITS_CEILING);
        assert!(power_of_ten_bits(&Integer::from(u64::MAX)) > RATIONAL_BITS_CEILING);
    }
}
