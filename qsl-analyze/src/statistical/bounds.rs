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
/// tail is below `2^-70` (QSpec FR-408).
const TAIL_BITS: u64 = 70;
/// The fixed-point scale of a square-root bound: `⌊√⌊x · 2^140⌋⌋ / 2^70`.
const SQRT_BITS: u64 = 70;
/// The width the normal-quantile bisection narrows its bracket below:
/// `2^-65`, half QSpec's `2^-64`, leaving the other half for the `Φ`
/// bounds' own width.
const QUANTILE_BITS: u64 = 65;
/// The largest upper end the normal-quantile search tries: `1 − Φ(16) <
/// 10^-57`, so a smaller `δ` is refused rather than searched.
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
/// geometric tail bound `u^(2n+3) / ((2n + 3)(1 − u²))` is below
/// `2^-bits`: the partial sum is the lower bound and the partial sum plus
/// the tail bound the upper.
fn atanh(u: &Rational, bits: u64) -> Result<Bounds, InternalFault> {
    let square = u.mul(u);
    let complement = int(1).sub(&square);
    let limit = epsilon(bits);
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

/// `ln 2 = 2 · atanh(1/3)`, its tail below `2^-bits`.
fn ln_two(bits: u64) -> Result<Bounds, InternalFault> {
    let third = quotient(&int(1), &int(3))?;
    let half = atanh(&third, bits)?;
    Ok(Bounds {
        lower: half.lower.mul(&int(2)),
        upper: half.upper.mul(&int(2)),
    })
}

/// Bounds on `ln x` for a rational `x > 0`: `x = 2^k · y` with
/// `1 <= y < 2`, and `ln x = k · ln 2 + 2 · atanh((y − 1) / (y + 1))`.
/// `ln 2`'s tail is taken below `2^-(71 + bits(|k|))`, so `k · ln 2` adds at
/// most `2^-70` to the width whatever `k` is, and the bound stays within
/// `2^-64`.
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
    let series = atanh(&u, TAIL_BITS)?;
    let exponent_bits = u64::from(128 - exponent.unsigned_abs().leading_zeros());
    let two = ln_two(TAIL_BITS + 1 + exponent_bits)?;
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
    sqrt_at(x, SQRT_BITS)
}

/// `√x` bounded as [`sqrt`] at the fixed-point scale `2^-bits`.
fn sqrt_at(x: &Rational, bits: u64) -> Result<Bounds, InternalFault> {
    if *x < int(0) {
        return Err(ROOT_ARGUMENT);
    }
    let scaled = x.numerator().as_big() << (2 * bits);
    let floor = scaled.div_floor(x.denominator().as_big());
    let root = floor.sqrt();
    Ok(Bounds {
        lower: big(root.clone()).divided_by_power_of_two(bits),
        upper: big(root + 1).divided_by_power_of_two(bits),
    })
}

/// Bounds on an alternating series whose terms decrease in magnitude past
/// `partial`: the partial sum and the partial sum plus the next, signed,
/// term.
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
/// the next term is below `2^-bits`.
fn atan(x: &Rational, bits: u64) -> Result<Bounds, InternalFault> {
    let square = x.mul(x);
    let limit = epsilon(bits);
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

/// `π = 16 · atan(1/5) − 4 · atan(1/239)` (Machin), each tail below
/// `2^-bits`.
fn pi(bits: u64) -> Result<Bounds, InternalFault> {
    let fifth = atan(&quotient(&int(1), &int(5))?, bits)?;
    let small = atan(&quotient(&int(1), &int(239))?, bits)?;
    Ok(Bounds {
        lower: fifth.lower.mul(&int(16)).sub(&small.upper.mul(&int(4))),
        upper: fifth.upper.mul(&int(16)).sub(&small.lower.mul(&int(4))),
    })
}

/// Bounds on `1/√(2π)` at precision `2^-bits`.
fn inverse_root_two_pi(bits: u64) -> Result<Bounds, InternalFault> {
    let pi = pi(bits)?;
    let lower_root = sqrt_at(&pi.lower.mul(&int(2)), bits)?.lower;
    let upper_root = sqrt_at(&pi.upper.mul(&int(2)), bits)?.upper;
    Ok(Bounds {
        lower: quotient(&int(1), &upper_root)?,
        upper: quotient(&int(1), &lower_root)?,
    })
}

/// Bounds on `Φ(z) = 1/2 + c · Σ (−1)^n z^(2n+1) / (2^n n! (2n + 1))` for
/// `z >= 0`, `c` bounded by [`inverse_root_two_pi`]. The series stops at the
/// first term past `z²` whose successor is below `2^-bits`; from there its
/// terms decrease in magnitude, so the partial sum and the next term bound
/// it.
fn phi(z: &Rational, c: &Bounds, bits: u64) -> Result<Bounds, InternalFault> {
    let series = phi_series(z, bits)?;
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

/// `Σ (−1)^n z^(2n+1) / (2^n n! (2n + 1))` bounded as [`phi`] states, summed
/// in integers over one running common denominator, so no partial sum is
/// reduced: with `z = p/q`, term `n` is `p^(2n+1) / (q^(2n+1) 2^n n! (2n+1))`,
/// and the denominator `q^(2n+1) 2^n n! · lcm(1, 3, …, 2n + 1)` grows by an
/// integer factor at each step.
fn phi_series(z: &Rational, bits: u64) -> Result<Bounds, InternalFault> {
    use num_traits::One;
    let p = z.numerator().as_big().clone();
    let q = z.denominator().as_big().clone();
    let (p_square, q_square) = (&p * &p, &q * &q);
    // `z²` rounded up, for the monotone-tail condition `n + 1 >= z²`.
    let square_ceiling = p_square.div_ceil(&q_square);
    // `base = q^(2n+1) 2^n n!`, `odd = lcm(1, 3, …, 2n + 1)`, the running
    // sum is `sum / (base · odd)`, and `power = p^(2n+1)`.
    let mut base = q;
    let mut odd = BigInt::one();
    let mut power = p;
    let mut sum = power.clone();
    let mut index: u64 = 0;
    loop {
        let step = BigInt::from(2 * (index + 1));
        let next_odd = BigInt::from(2 * index + 3);
        let next_base = &base * &q_square * &step;
        let next_power = &power * &p_square;
        // The next term `next_power / (next_base (2n + 3))` against `2^-bits`.
        let small = (&next_power << bits) < &next_base * &next_odd;
        if small && BigInt::from(index + 1) >= square_ceiling {
            let denominator = &base * &odd;
            let partial = Rational::new(Integer::from_big(sum), Integer::from_big(denominator))
                .map_err(|_| DIVISOR)?;
            let next = Rational::new(
                Integer::from_big(next_power),
                Integer::from_big(next_base * next_odd),
            )
            .map_err(|_| DIVISOR)?;
            let signed = if index.is_multiple_of(2) { next.neg() } else { next };
            return Ok(alternating(partial, &signed));
        }
        let gcd = odd.gcd(&next_odd);
        let widened = &next_odd / &gcd;
        let next_lcm = &odd * &widened;
        // Rescale the sum to the new denominator `next_base · next_lcm`.
        sum = sum * &q_square * &step * &widened;
        let term = &next_power * (&next_lcm / &next_odd);
        if index.is_multiple_of(2) {
            sum -= term;
        } else {
            sum += term;
        }
        base = next_base;
        odd = next_lcm;
        power = next_power;
        index += 1;
    }
}

/// The `Φ` precision that keeps the quantile within `2^-65` of `z_{1−δ}`
/// for any `z_{1−δ} <= upper`.
///
/// A `Φ` bound of width `W` places the bisection's lower end within
/// `W / φ(z)` of the quantile, and `φ` falls as `z` grows:
/// `log2(1/φ(z)) < 3z²/4 + 2`. QSpec FR-408's `2^-70` tails give `W` near
/// `2^-68`, which meets `2^-64` only while `φ(z)` is large (`δ` near 1/100);
/// at `δ = 10^-10` it misses by a factor of about `4·10^6`. So the tails
/// deepen by `3·upper²/4` bits, plus 2 for the factor `W` carries from `c`
/// and the series, making `W / φ(z) <= 2^-65`.
fn quantile_bits(upper: &Rational) -> u64 {
    let ceiling = ceil(&upper.mul(upper).mul(&int(3)).divided_by_power_of_two(2));
    TAIL_BITS + ceiling.to_u64().map_or(256, |extra| extra.min(256)) + 2
}

/// An upper bound on the standard normal quantile `z_{1−δ}` for
/// `0 < δ < 1`, within `2^-64` of it: bisection over rationals on `Φ`,
/// keeping as the upper end a point whose lower `Φ` bound reaches `1 − δ`,
/// until the bracket is narrower than `2^-65`, with `Φ` bounded at
/// [`quantile_bits`]. The upper end is never below 0, so it bounds a
/// negative quantile too.
pub(crate) fn normal_quantile_upper(delta: &Rational) -> Result<Rational, QuantileFailure> {
    let target = int(1).sub(delta);
    let mut lower = int(0);
    let mut upper = int(1);
    loop {
        let bits = quantile_bits(&upper);
        let c = inverse_root_two_pi(bits)?;
        if phi(&upper, &c, bits)?.lower >= target {
            break;
        }
        if upper >= int(QUANTILE_CEILING) {
            return Err(QuantileFailure::BeyondPrecision);
        }
        lower = upper.clone();
        upper = upper.mul(&int(2));
    }
    // `φ` is least at the bracket's upper end, so its precision covers the
    // whole bracket.
    let bits = quantile_bits(&upper);
    let c = inverse_root_two_pi(bits)?;
    let width = epsilon(QUANTILE_BITS);
    while upper.sub(&lower) >= width {
        let middle = lower.add(&upper).divided_by_power_of_two(1);
        if phi(&middle, &c, bits)?.lower >= target {
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
        within(&pi(TAIL_BITS).expect("π"), &lower, &upper);
    }

    /// The quantile's upper end lies at or above `z_{1−δ}` and within
    /// `2^-64` of it, at `δ = 1/100` (QSpec FR-408-AC-12's
    /// `z_0.99 ≈ 2.3263478740`) and at `δ = 10^-10`, where `φ(z)` is small.
    #[test]
    fn the_upper_normal_quantile_is_within_two_to_the_minus_64() {
        for (delta, digits) in [
            (
                quotient(&int(1), &int(100)).expect("1/100"),
                "2326347874040841100885606163346911723351817",
            ),
            (
                int(1).divided_by_power_of_ten(10),
                "6361340902404056204695375828265221679203937",
            ),
        ] {
            let z = normal_quantile_upper(&delta).expect("a quantile");
            let lower = decimal(digits, 42);
            let upper = lower.add(&int(1).divided_by_power_of_ten(42));
            assert!(z >= upper, "the upper end is below the quantile");
            assert!(
                z.sub(&lower) < epsilon(64),
                "the upper end is not within 2^-64"
            );
        }
    }

    /// `ln(2^-40) = −40 ln 2`, bracketed within `2^-64` although the binary
    /// exponent is large.
    #[test]
    fn ln_of_a_large_binary_exponent_stays_within_two_to_the_minus_64() {
        // 40 ln 2 = 27.7258872223978123766892848583270627230200053744…
        let lower = decimal("-277258872223978123766892848583270627231", 37);
        let upper = decimal("-277258872223978123766892848583270627230", 37);
        let x = int(1).divided_by_power_of_two(40);
        within(&ln(&x).expect("ln 2^-40"), &lower, &upper);
        let big = Rational::from_integer(Integer::one().shifted_left(200));
        let width = ln(&big).expect("ln 2^200");
        assert!(width.upper.sub(&width.lower) < epsilon(64));
    }

    /// `1 − Φ(16) ≈ 2^-190`: a `δ` below it is refused, not searched.
    #[test]
    fn a_delta_beyond_the_search_ceiling_is_refused() {
        let delta = int(1).divided_by_power_of_two(200);
        assert_eq!(
            normal_quantile_upper(&delta),
            Err(QuantileFailure::BeyondPrecision)
        );
    }
}
