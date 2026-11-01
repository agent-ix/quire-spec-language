// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-253/FR-254: exact conditioning of timed delay laws over enabled windows.

use qsl_semantics::check::DelayDistribution;
use quire_exact::{Integer, Rational};

use super::super::choice::{uniform_grid_value, weighted_choice_counted, ChoicePoint};
use super::super::model::DelayInterval;

/// Why a delay law has no supported draw in an enabled window.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TimedDelayRefusal {
    /// No support point or positive probability mass intersects the window.
    #[error("the delay distribution has no mass in its window")]
    NoMass,
    /// A uniform distribution over an unbounded window has no finite law.
    #[error("a uniform delay has an unbounded window")]
    UnboundedUniform,
    /// The requested uniform grid exponent is above the sampler's 256-bit
    /// draw domain.
    #[error("delay grid exponent {q} exceeds 256")]
    GridExponent {
        /// The refused exponent.
        q: u32,
    },
}

/// Draw one exact delay from `distribution` conditioned on the union of
/// `windows`, using `at` as its one sampler choice (FR-253/FR-254).
pub fn draw_timed_delay(
    distribution: &DelayDistribution,
    windows: &[DelayInterval],
    at: ChoicePoint,
    q: u32,
) -> Result<Rational, TimedDelayRefusal> {
    match distribution {
        DelayDistribution::Discrete(support) => {
            let (delays, weights): (Vec<_>, Vec<_>) = support
                .iter()
                .filter(|(delay, _)| windows.iter().any(|window| window.contains(delay)))
                .map(|(delay, weight)| (delay.clone(), weight.clone()))
                .unzip();
            if delays.is_empty() {
                return Err(TimedDelayRefusal::NoMass);
            }
            if delays.len() == 1 {
                return Ok(delays[0].clone());
            }
            let (selected, _) = weighted_choice_counted(at, &weights);
            let index = selected.map_err(|_| TimedDelayRefusal::NoMass)?;
            delays.get(index).cloned().ok_or(TimedDelayRefusal::NoMass)
        }
        DelayDistribution::Uniform => {
            let ranges = ranges(windows, None)?;
            draw_uniform(&ranges, at, q)
        }
        DelayDistribution::UniformRange { a, b } => {
            let ranges = ranges(windows, Some((a, b)))?;
            draw_uniform(&ranges, at, q)
        }
        DelayDistribution::Exponential { rate } => {
            let ranges = ranges(windows, None)?;
            if ranges.len() != 1 || ranges[0].1.is_some() {
                return Err(TimedDelayRefusal::NoMass);
            }
            let lower = ranges[0].0.clone();
            let u = grid(at, q)?;
            let residual = Rational::from_integer(Integer::one()).sub(&u);
            if residual.is_zero() {
                return Err(TimedDelayRefusal::NoMass);
            }
            let inverse_survival = quotient(&Rational::from_integer(Integer::one()), &residual);
            let delay = lower.add(&quotient(&ln_positive(&inverse_survival), rate));
            Ok(delay)
        }
    }
}

/// A set of clipped intervals represented by their finite ends.
fn ranges(
    windows: &[DelayInterval],
    clip: Option<(&Rational, &Rational)>,
) -> Result<Vec<(Rational, Option<Rational>)>, TimedDelayRefusal> {
    let mut ranges = Vec::new();
    for window in windows {
        let (mut lower, _) = window.lower();
        let mut upper = window.upper().map(|(upper, _)| upper);
        if let Some((a, b)) = clip {
            if lower < a {
                lower = a;
            }
            if upper.is_none_or(|upper| upper > b) {
                upper = Some(b);
            }
        }
        if upper.is_some_and(|upper| upper < lower) {
            continue;
        }
        ranges.push((lower.clone(), upper.cloned()));
    }
    if ranges.is_empty() {
        Err(TimedDelayRefusal::NoMass)
    } else {
        Ok(ranges)
    }
}

fn draw_uniform(
    ranges: &[(Rational, Option<Rational>)],
    at: ChoicePoint,
    q: u32,
) -> Result<Rational, TimedDelayRefusal> {
    if ranges.iter().any(|(_, upper)| upper.is_none()) {
        return Err(TimedDelayRefusal::UnboundedUniform);
    }
    let total = ranges.iter().fold(
        Rational::from_integer(Integer::zero()),
        |sum, (lower, upper)| sum.add(&upper.as_ref().expect("finite range").sub(lower)),
    );
    if total.is_zero() {
        if ranges.len() == 1 {
            return Ok(ranges[0].0.clone());
        }
        let weights = vec![Rational::from_integer(Integer::one()); ranges.len()];
        let (selected, _) = weighted_choice_counted(at, &weights);
        return selected
            .map(|index| ranges[index].0.clone())
            .map_err(|_| TimedDelayRefusal::NoMass);
    }
    let u = grid(at, q)?;
    let mut remaining = total.mul(&u);
    for (lower, upper) in ranges {
        let length = upper.as_ref().expect("finite range").sub(lower);
        if remaining <= length {
            return Ok(lower.add(&remaining));
        }
        remaining = remaining.sub(&length);
    }
    Err(TimedDelayRefusal::NoMass)
}

fn grid(at: ChoicePoint, q: u32) -> Result<Rational, TimedDelayRefusal> {
    uniform_grid_value(at, q).ok_or(TimedDelayRefusal::GridExponent { q })
}

/// Natural logarithm by the exact atanh series after power-of-two reduction.
/// The bounded tail is omitted; adding one `2^-256` unit gives an upward
/// approximation for the positive logarithm used by the exponential draw.
fn ln_positive(value: &Rational) -> Rational {
    let one = Rational::from_integer(Integer::one());
    if value == &one {
        return Rational::from_integer(Integer::zero());
    }
    let mut reduced = value.clone();
    let mut powers = 0_u32;
    while reduced < one {
        reduced = reduced.mul(&Rational::from_integer(Integer::from(2_u64)));
        powers += 1;
    }
    let z = quotient(&reduced.sub(&one), &reduced.add(&one));
    let z2 = z.mul(&z);
    let mut term = z.clone();
    let mut sum = Rational::from_integer(Integer::zero());
    for index in 0_u64..256 {
        let denominator = Rational::from_integer(Integer::from(2 * index + 1));
        sum = sum.add(&quotient(&term, &denominator));
        term = term.mul(&z2);
    }
    let ln_reduced = sum.mul(&Rational::from_integer(Integer::from(2_u64)));
    let ln_two = ln_two();
    let ln_value =
        ln_reduced.sub(&ln_two.mul(&Rational::from_integer(Integer::from(u64::from(powers)))));
    if ln_value.numerator().is_negative() {
        ln_value
    } else {
        ln_value.add(&Rational::from_integer(Integer::one()).divided_by_power_of_two(256))
    }
}

fn ln_two() -> Rational {
    let z = quotient(
        &Rational::from_integer(Integer::one()),
        &Rational::from_integer(Integer::from(3_u64)),
    );
    let z2 = z.mul(&z);
    let mut term = z.clone();
    let mut sum = Rational::from_integer(Integer::zero());
    for index in 0_u64..256 {
        sum = sum.add(&quotient(
            &term,
            &Rational::from_integer(Integer::from(2 * index + 1)),
        ));
        term = term.mul(&z2);
    }
    sum.mul(&Rational::from_integer(Integer::from(2_u64)))
}

fn quotient(numerator: &Rational, denominator: &Rational) -> Rational {
    numerator
        .div(denominator)
        .expect("every division has a positive nonzero denominator")
}
