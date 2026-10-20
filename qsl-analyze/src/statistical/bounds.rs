// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-408 "Exact rational bounds" (ADR-024 ST-9): a lower and an
//! upper rational bound on each logarithm, square root and standard normal
//! quantile EN-4 uses, computed in integer arithmetic by the method QSpec
//! states, so every rounded quantity can be rounded outward.

use num_bigint::BigInt;
use num_integer::Integer as _;
use qsl_foundation::InternalFault;
use quire_exact::{Integer, Rational};

/// The bit precision of every series tail bound: a series stops once its
/// tail is below `2^-70`.
const TAIL_BITS: u64 = 70;
/// The fixed-point scale of a square-root bound: `⌊√⌊x · 2^140⌋⌋ / 2^70`.
const SQRT_BITS: u64 = 70;
/// The width the normal-quantile bisection narrows its bracket below.
const QUANTILE_BITS: u64 = 64;
/// The largest upper end the normal-quantile search tries. Within the
/// `Φ` bounds' precision no `δ` that admits a quantile at all lies beyond
/// it: `1 − Φ(16) < 10^-57`.
const QUANTILE_CEILING: i64 = 16;

const DIVISOR: InternalFault = InternalFault::new("statistical-bounds", "nonzero-divisor");
const LOG_ARGUMENT: InternalFault =
    InternalFault::new("statistical-bounds", "positive-log-argument");
const ROOT_ARGUMENT: InternalFault =
    InternalFault::new("statistical-bounds", "non-negative-root-argument");

/// A lower and an upper bound on one real number.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Bounds {
    /// At most the true value.
    pub(crate) lower: Rational,
    /// At least the true value.
    pub(crate) upper: Rational,
}

/// Why the normal quantile has no bound EN-4 can use.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum QuantileFailure {
    /// `δ` is too small for the `Φ` bounds' precision to separate
    /// `1 − δ` from 1.
    BeyondPrecision,
    /// An arithmetic invariant broke.
    Fault(InternalFault),
}

impl From<InternalFault> for QuantileFailure {
    fn from(fault: InternalFault) -> Self {
        Self::Fault(fault)
    }
}

pub(crate) fn int(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

fn big(value: BigInt) -> Rational {
    Rational::from_integer(Integer::from_big(value))
}

/// `numerator / denominator`.
pub(crate) fn quotient(
    numerator: &Rational,
    denominator: &Rational,
) -> Result<Rational, InternalFault> {
    numerator.div(denominator).ok_or(DIVISOR)
}

/// `2^-bits`.
fn epsilon(bits: u64) -> Rational {
    int(1).divided_by_power_of_two(bits)
}

/// `value · 2^shift`.
fn times_power_of_two(value: &Rational, shift: u64) -> Rational {
    value.mul(&Rational::from_integer(Integer::one().shifted_left(shift)))
}

/// The least integer at or above `value`.
pub(crate) fn ceil(value: &Rational) -> Integer {
    Integer::from_big(
        value
            .numerator()
            .as_big()
            .div_ceil(value.denominator().as_big()),
    )
}

/// `atanh(u) = Σ u^(2n+1) / (2n + 1)` for `0 <= u < 1`, summed until the
/// geometric tail bound `u^(2n+3) / ((2n + 3)(1 − u²))` is below `2^-70`:
/// the partial sum is the lower bound and the partial sum plus the tail
/// bound the upper.
fn atanh(u: &Rational) -> Result<Bounds, InternalFault> {
    let square = u.mul(u);
    let complement = int(1).sub(&square);
    let limit = epsilon(TAIL_BITS);
    let mut power = u.clone();
    let mut sum = int(0);
    let mut index: i64 = 0;
    loop {
        sum = sum.add(&quotient(&power, &int(2 * index + 1))?);
        let next = power.mul(&square);
        let tail = quotient(&next, &int(2 * index + 3).mul(&complement))?;
        if tail < limit {
            return Ok(Bounds {
                upper: sum.add(&tail),
                lower: sum,
            });
        }
        power = next;
        index += 1;
    }
}

/// `ln 2 = 2 · atanh(1/3)`.
fn ln_two() -> Result<Bounds, InternalFault> {
    let third = quotient(&int(1), &int(3))?;
    let half = atanh(&third)?;
    Ok(Bounds {
        lower: half.lower.mul(&int(2)),
        upper: half.upper.mul(&int(2)),
    })
}

/// Bounds on `ln x` for a rational `x > 0`: `x = 2^k · y` with
/// `1 <= y < 2`, and `ln x = k · ln 2 + 2 · atanh((y − 1) / (y + 1))`.
pub(crate) fn ln(x: &Rational) -> Result<Bounds, InternalFault> {
    if *x <= int(0) {
        return Err(LOG_ARGUMENT);
    }
    // `x / 2^k` for the signed `k`: a bit-length difference, so its
    // magnitude fits in `u64`.
    let scale = |exponent: i128| -> Result<Rational, InternalFault> {
        let shift = u64::try_from(exponent.unsigned_abs()).map_err(|_| LOG_ARGUMENT)?;
        Ok(if exponent >= 0 {
            x.divided_by_power_of_two(shift)
        } else {
            times_power_of_two(x, shift)
        })
    };
    let numerator_bits = i128::from(x.numerator().as_big().bits());
    let denominator_bits = i128::from(x.denominator().as_big().bits());
    let mut exponent = numerator_bits - denominator_bits;
    let mut mantissa = scale(exponent)?;
    if mantissa < int(1) {
        exponent -= 1;
        mantissa = scale(exponent)?;
    }
    let u = quotient(&mantissa.sub(&int(1)), &mantissa.add(&int(1)))?;
    let series = atanh(&u)?;
    let two = ln_two()?;
    let k = big(BigInt::from(exponent));
    let (k_lower, k_upper) = if exponent >= 0 {
        (k.mul(&two.lower), k.mul(&two.upper))
    } else {
        (k.mul(&two.upper), k.mul(&two.lower))
    };
    Ok(Bounds {
        lower: k_lower.add(&series.lower.mul(&int(2))),
        upper: k_upper.add(&series.upper.mul(&int(2))),
    })
}

/// Bounds on `√x` for a rational `x >= 0`: the integer square root of
/// `⌊x · 2^140⌋` and its successor, each scaled by `2^-70`.
pub(crate) fn sqrt(x: &Rational) -> Result<Bounds, InternalFault> {
    if *x < int(0) {
        return Err(ROOT_ARGUMENT);
    }
    let scaled = x.numerator().as_big() << (2 * SQRT_BITS);
    let floor = scaled.div_floor(x.denominator().as_big());
    let root = floor.sqrt();
    Ok(Bounds {
        lower: big(root.clone()).divided_by_power_of_two(SQRT_BITS),
        upper: big(root + 1).divided_by_power_of_two(SQRT_BITS),
    })
}

/// Bounds on an alternating series `Σ (−1)^n a_n` whose terms decrease in
/// magnitude from `first` on: the partial sum through `first` terms and
/// the partial sum plus the next term.
fn alternating(partial: Rational, next_term: &Rational) -> Bounds {
    let other = partial.add(next_term);
    if other < partial {
        Bounds {
            lower: other,
            upper: partial,
        }
    } else {
        Bounds {
            lower: partial,
            upper: other,
        }
    }
}

/// `atan(x) = Σ (−1)^n x^(2n+1) / (2n + 1)` for `0 < x < 1`, summed until
/// the next term is below `2^-70`.
fn atan(x: &Rational) -> Result<Bounds, InternalFault> {
    let square = x.mul(x);
    let limit = epsilon(TAIL_BITS);
    let mut power = x.clone();
    let mut sum = int(0);
    let mut index: i64 = 0;
    loop {
        let term = quotient(&power, &int(2 * index + 1))?;
        sum = if index % 2 == 0 {
            sum.add(&term)
        } else {
            sum.sub(&term)
        };
        power = power.mul(&square);
        let next = quotient(&power, &int(2 * index + 3))?;
        if next < limit {
            let signed = if index % 2 == 0 { next.neg() } else { next };
            return Ok(alternating(sum, &signed));
        }
        index += 1;
    }
}

/// `π = 16 · atan(1/5) − 4 · atan(1/239)` (Machin).
fn pi() -> Result<Bounds, InternalFault> {
    let fifth = atan(&quotient(&int(1), &int(5))?)?;
    let small = atan(&quotient(&int(1), &int(239))?)?;
    Ok(Bounds {
        lower: fifth.lower.mul(&int(16)).sub(&small.upper.mul(&int(4))),
        upper: fifth.upper.mul(&int(16)).sub(&small.lower.mul(&int(4))),
    })
}

/// Bounds on `1/√(2π)`.
fn inverse_root_two_pi() -> Result<Bounds, InternalFault> {
    let pi = pi()?;
    let lower_root = sqrt(&pi.lower.mul(&int(2)))?.lower;
    let upper_root = sqrt(&pi.upper.mul(&int(2)))?.upper;
    Ok(Bounds {
        lower: quotient(&int(1), &upper_root)?,
        upper: quotient(&int(1), &lower_root)?,
    })
}

/// Bounds on `Φ(z) = 1/2 + c · Σ (−1)^n z^(2n+1) / (2^n n! (2n + 1))` for
/// `z >= 0`, `c` bounded by [`inverse_root_two_pi`]. The series stops at the
/// first term past `z²` whose successor is below `2^-70`; from there its
/// terms decrease in magnitude, so the partial sum and the next term bound
/// it.
fn phi(z: &Rational, c: &Bounds) -> Result<Bounds, InternalFault> {
    let square = z.mul(z);
    let limit = epsilon(TAIL_BITS);
    // `term` is `z^(2n+1) / (2^n n!)`, without its sign and its `2n + 1`.
    let mut term = z.clone();
    let mut sum = int(0);
    let mut index: i64 = 0;
    let series = loop {
        let value = quotient(&term, &int(2 * index + 1))?;
        sum = if index % 2 == 0 {
            sum.add(&value)
        } else {
            sum.sub(&value)
        };
        term = quotient(&term.mul(&square), &int(2 * (index + 1)))?;
        let next = quotient(&term, &int(2 * index + 3))?;
        if next < limit && int(index + 1) >= square {
            let signed = if index % 2 == 0 { next.neg() } else { next };
            break alternating(sum, &signed);
        }
        index += 1;
    };
    let product_lower = if series.lower >= int(0) {
        c.lower.mul(&series.lower)
    } else {
        c.upper.mul(&series.lower)
    };
    let product_upper = if series.upper >= int(0) {
        c.upper.mul(&series.upper)
    } else {
        c.lower.mul(&series.upper)
    };
    let half = quotient(&int(1), &int(2))?;
    Ok(Bounds {
        lower: half.add(&product_lower),
        upper: half.add(&product_upper),
    })
}

/// An upper bound on the standard normal quantile `z_{1−δ}` for
/// `0 < δ < 1`: bisection over rationals on `Φ`, keeping as the upper end
/// a point whose lower `Φ` bound reaches `1 − δ`, until the bracket is
/// narrower than `2^-64`. The upper end is never below 0, so it bounds a
/// negative quantile too.
pub(crate) fn normal_quantile_upper(delta: &Rational) -> Result<Rational, QuantileFailure> {
    let target = int(1).sub(delta);
    let c = inverse_root_two_pi()?;
    let mut lower = int(0);
    let mut upper = int(1);
    while phi(&upper, &c)?.lower < target {
        if upper >= int(QUANTILE_CEILING) {
            return Err(QuantileFailure::BeyondPrecision);
        }
        lower = upper.clone();
        upper = upper.mul(&int(2));
    }
    let width = epsilon(QUANTILE_BITS);
    while upper.sub(&lower) >= width {
        let middle = lower.add(&upper).divided_by_power_of_two(1);
        if phi(&middle, &c)?.lower >= target {
            upper = middle;
        } else {
            lower = middle;
        }
    }
    Ok(upper)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `numerator / 10^digits` from a decimal digit string.
    fn decimal(digits: &str, scale: u64) -> Rational {
        let value: Integer = digits.parse().expect("decimal digits");
        Rational::from_integer(value).divided_by_power_of_ten(scale)
    }

    fn within(bounds: &Bounds, truth_lower: &Rational, truth_upper: &Rational) {
        assert!(bounds.lower <= *truth_lower, "lower bound above the value");
        assert!(bounds.upper >= *truth_upper, "upper bound below the value");
        assert!(
            bounds.upper.sub(&bounds.lower) < epsilon(64),
            "bounds wider than 2^-64"
        );
    }

    #[test]
    fn ln_100_brackets_its_true_value_within_two_to_the_minus_64() {
        // ln 100 = 4.60517018598809136803598290936872841520220297725754…
        let lower = decimal("460517018598809136803598290936872841520", 38);
        let upper = decimal("460517018598809136803598290936872841521", 38);
        within(&ln(&int(100)).expect("ln 100"), &lower, &upper);
    }

    #[test]
    fn ln_of_a_fraction_below_one_is_negative_and_bracketed() {
        // ln(3/4) = -0.28768207245178092743921900599382743150350971089776…
        let x = quotient(&int(3), &int(4)).expect("3/4");
        let lower = decimal("-287682072451780927439219005993827431504", 39);
        let upper = decimal("-287682072451780927439219005993827431503", 39);
        within(&ln(&x).expect("ln 3/4"), &lower, &upper);
    }

    #[test]
    fn ln_of_zero_is_an_internal_fault() {
        assert_eq!(ln(&int(0)), Err(LOG_ARGUMENT));
    }

    #[test]
    fn sqrt_bounds_bracket_an_irrational_root() {
        // √2 = 1.41421356237309504880168872420969807856967187537694…
        let lower = decimal("141421356237309504880168872420969807856", 38);
        let upper = decimal("141421356237309504880168872420969807857", 38);
        within(&sqrt(&int(2)).expect("√2"), &lower, &upper);
        let exact = sqrt(&int(9)).expect("√9");
        assert_eq!(exact.lower, int(3));
        assert!(exact.upper > int(3));
    }

    #[test]
    fn pi_is_bracketed() {
        // π = 3.14159265358979323846264338327950288419716939937510…
        let lower = decimal("314159265358979323846264338327950288419", 38);
        let upper = decimal("314159265358979323846264338327950288420", 38);
        within(&pi().expect("π"), &lower, &upper);
    }

    #[test]
    fn the_upper_normal_quantile_at_one_percent_is_just_above_its_value() {
        // z_0.99 = 2.32634787404084110089… (QSpec FR-408-AC-12: ≈ 2.3263478740).
        let delta = quotient(&int(1), &int(100)).expect("1/100");
        let z = normal_quantile_upper(&delta).expect("z_0.99");
        let truth = decimal("232634787404084110089", 20);
        assert!(z >= truth, "the upper end is below z_0.99");
        assert!(
            z.sub(&truth) < decimal("1", 12),
            "the upper end is far above z_0.99"
        );
    }

    #[test]
    fn a_vanishing_delta_is_beyond_the_quantile_precision() {
        let delta = int(1).divided_by_power_of_two(80);
        assert_eq!(
            normal_quantile_upper(&delta),
            Err(QuantileFailure::BeyondPrecision)
        );
    }
}
