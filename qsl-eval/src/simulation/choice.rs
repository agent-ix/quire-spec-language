// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-188: one weighted choice of the `quire.simulation.sampler/v1`
//! generator (QSpec `proposals/quire-v1/definitions/simulation-sampler.md`,
//! "Uniform and weighted selection").
//!
//! A choice scales its positive exact rational weights to integers
//! `w_0, …, w_{k−1}` with total `n` by the least common multiple of their
//! denominators ([`Weights`]), then reads draws `v` at its [`ChoicePoint`]:
//! `v` is accepted when `v < n · ⌊2^256 / n⌋`, and the choice selects the
//! least index `i` with `v mod n < w_0 + … + w_i`. A uniform choice over `k`
//! items is the choice over `k` unit weights, so FR-101's step draw is this
//! function at choice index 0.

use quire_canonical::FixedShape;
use quire_exact::{Integer, NonZeroInteger, Rational};
use serde::Serialize;

use crate::simulation::key::plain_digest;

/// Where a choice draws from: the run's seed, the trace's index within the
/// run, the step's index within the trace and the choice's index within the
/// step (FR-188; QSpec FR-181 "Digests").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ChoicePoint {
    /// The run's seed.
    pub seed: u64,
    /// The trace's 0-based index within the run.
    pub trace: u64,
    /// The step's 0-based index within the trace.
    pub step: u64,
    /// The choice's 0-based index within the step.
    pub choice: u32,
}

/// One draw's JCS preimage: `{"choice":"<decimal>","draw":"<decimal>",
/// "seed":"<decimal>","step":"<decimal>","trace":"<decimal>"}` (QSpec
/// FR-181).
#[derive(Serialize, FixedShape)]
pub(crate) struct DrawPreimage {
    pub(crate) choice: String,
    pub(crate) draw: String,
    pub(crate) seed: String,
    pub(crate) step: String,
    pub(crate) trace: String,
}

impl DrawPreimage {
    fn at(point: ChoicePoint, draw: u64) -> Self {
        Self {
            choice: point.choice.to_string(),
            draw: draw.to_string(),
            seed: point.seed.to_string(),
            step: point.step.to_string(),
            trace: point.trace.to_string(),
        }
    }
}

/// Why a list of weights admits no choice (FR-188).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum WeightsRefusal {
    /// The list is empty: an empty item set ends a trace without a draw, so
    /// it is never a choice.
    #[error("a choice needs at least one weight")]
    Empty,
    /// The weight at `index` is zero or negative.
    #[error("weight {index} is not positive")]
    NotPositive {
        /// The weight's 0-based position in the list.
        index: usize,
    },
    /// The scaled total exceeds `2^256`, so no 256-bit draw is ever accepted
    /// (`n · ⌊2^256 / n⌋ = 0`).
    #[error("the scaled weight total exceeds 2^256")]
    TotalAboveDrawRange,
}

/// The integer weights of one choice: positive exact rationals scaled by the
/// least common multiple of their denominators, with their total `n` and the
/// acceptance bound `n · ⌊2^256 / n⌋` (FR-188).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Weights {
    /// `w_0, …, w_{k−1}`, in the caller's order.
    scaled: Vec<Integer>,
    /// `n`, the sum of `scaled`; at least 1.
    total: Integer,
    /// `n · ⌊2^256 / n⌋`: a draw below it is accepted.
    accept_below: Integer,
}

impl Weights {
    /// Scale `weights` to integers by the least common multiple of their
    /// denominators.
    ///
    /// # Errors
    ///
    /// [`WeightsRefusal`] for an empty list, a weight that is not positive,
    /// or a scaled total above `2^256`.
    pub fn new(weights: &[Rational]) -> Result<Self, WeightsRefusal> {
        if let Some(index) = weights
            .iter()
            .position(|weight| weight.is_zero() || weight.numerator().is_negative())
        {
            return Err(WeightsRefusal::NotPositive { index });
        }
        // A reduced `Rational`'s denominator is positive, so the running
        // multiple and the gcd of two positive integers stay positive.
        let multiple = weights.iter().fold(Integer::one(), |multiple, weight| {
            let gcd = multiple.gcd(weight.denominator());
            multiple
                .exact_div(proven_positive(&gcd))
                .mul(weight.denominator())
        });
        let scaled = weights
            .iter()
            .map(|weight| {
                let factor = multiple.exact_div(proven_positive(weight.denominator()));
                weight.numerator().mul(&factor)
            })
            .collect();
        Self::from_scaled(scaled)
    }

    /// `count` unit weights: the uniform choice over `count` items.
    ///
    /// # Errors
    ///
    /// [`WeightsRefusal::Empty`] for `count = 0`;
    /// [`WeightsRefusal::TotalAboveDrawRange`] cannot arise, since `count`
    /// fits a `usize`.
    pub fn uniform(count: usize) -> Result<Self, WeightsRefusal> {
        Self::from_scaled(vec![Integer::one(); count])
    }

    fn from_scaled(scaled: Vec<Integer>) -> Result<Self, WeightsRefusal> {
        let total = scaled
            .iter()
            .fold(Integer::zero(), |total, weight| total.add(weight));
        if scaled.is_empty() {
            return Err(WeightsRefusal::Empty);
        }
        // Every scaled weight is positive, so a non-empty total is too.
        let divisor = proven_positive(&total);
        let range = Integer::one().shifted_left(256);
        let (quotient, _) = range.div_mod_floor(divisor);
        if quotient.is_zero() {
            return Err(WeightsRefusal::TotalAboveDrawRange);
        }
        let accept_below = quotient.mul(&total);
        Ok(Self {
            scaled,
            total,
            accept_below,
        })
    }

    /// The scaled integer weights, in the caller's order.
    pub fn scaled(&self) -> &[Integer] {
        &self.scaled
    }

    /// Their total `n`.
    pub fn total(&self) -> &Integer {
        &self.total
    }

    /// Whether draw `v` is accepted: `v < n · ⌊2^256 / n⌋`.
    fn accepts(&self, v: &Integer) -> bool {
        *v < self.accept_below
    }

    /// The least index `i` with `remainder < w_0 + … + w_i`.
    fn select(&self, remainder: &Integer) -> usize {
        let mut cumulative = Integer::zero();
        for (index, weight) in self.scaled.iter().enumerate() {
            cumulative = cumulative.add(weight);
            if *remainder < cumulative {
                return index;
            }
        }
        // `remainder < total`, the last cumulative sum, so the loop returns.
        self.scaled.len() - 1
    }
}

/// `value` as a divisor. Every caller passes an integer it has proven
/// positive, as `quire_exact`'s own reduction does; a zero here is a defect
/// of this module, never a property of the input.
fn proven_positive(value: &Integer) -> NonZeroInteger<'_> {
    NonZeroInteger::try_from(value)
        .unwrap_or_else(|_| panic!("a weight denominator, gcd or total is positive"))
}

/// A draw digest read as a big-endian unsigned 256-bit integer.
fn big_endian(digest: [u8; 32]) -> Integer {
    digest.iter().fold(Integer::zero(), |value, byte| {
        value.shifted_left(8).add(&Integer::from(u64::from(*byte)))
    })
}

/// The index `weights` selects at `at`, and how many draw digests the
/// selection computed: none for a one-item choice.
pub(crate) fn choose(at: ChoicePoint, weights: &Weights) -> (usize, u64) {
    if weights.scaled.len() == 1 {
        return (0, 0);
    }
    let modulus = proven_positive(&weights.total);
    let mut draw: u64 = 0;
    loop {
        let v = big_endian(plain_digest(&DrawPreimage::at(at, draw)));
        // A digest computed is a draw used, accepted or rejected.
        draw = draw.wrapping_add(1);
        if weights.accepts(&v) {
            let (_, remainder) = v.div_mod_floor(modulus);
            return (weights.select(&remainder), draw);
        }
    }
}

/// The index of `weights` the generator selects at `at` (FR-188): exact,
/// reproducible from the seed and independent of every other choice point.
/// A one-item choice selects index 0 and computes no digest.
pub fn weighted_choice(at: ChoicePoint, weights: &Weights) -> usize {
    choose(at, weights).0
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    fn integers(values: &[u64]) -> Vec<Integer> {
        values.iter().copied().map(Integer::from).collect()
    }

    /// FR-188-AC-2: the acceptance bound at `n = 7` is the real
    /// `n · ⌊2^256 / 7⌋`: one below it is accepted and it is not.
    #[trace("TC-623", "FR-188-AC-2")]
    #[test]
    fn the_acceptance_bound_is_the_largest_multiple_of_n_in_range() {
        let weights = Weights::uniform(7).expect("seven items");
        let range = Integer::one().shifted_left(256);
        let seven = Integer::from(7u64);
        let (_, excess) = range.div_mod_floor(NonZeroInteger::try_from(&seven).expect("7"));
        let bound = range.sub(&excess);
        assert_eq!(weights.accept_below, bound);
        assert!(weights.accepts(&bound.sub(&Integer::one())));
        assert!(!weights.accepts(&bound));
    }

    /// A total that divides `2^256` accepts every draw.
    #[trace("TC-623", "FR-188-AC-2")]
    #[test]
    fn a_power_of_two_total_accepts_every_draw() {
        let weights = Weights::from_scaled(integers(&[1, 3])).expect("n = 4");
        assert_eq!(weights.accept_below, Integer::one().shifted_left(256));
    }

    /// The selection is the least index whose running sum exceeds the
    /// remainder: weights `(2, 1, 3)` map `0, 1 → 0`, `2 → 1`, `3, 4, 5 → 2`.
    #[trace("TC-623", "FR-188-AC-2")]
    #[test]
    fn a_remainder_selects_the_least_covering_index() {
        let weights = Weights::from_scaled(integers(&[2, 1, 3])).expect("n = 6");
        let selected: Vec<usize> = (0..6u64)
            .map(|remainder| weights.select(&Integer::from(remainder)))
            .collect();
        assert_eq!(selected, [0, 0, 1, 2, 2, 2]);
    }

    /// A total above `2^256` is refused rather than looping forever; `2^256`
    /// itself is admitted.
    #[trace("TC-623")]
    #[test]
    fn a_total_above_the_draw_range_is_refused() {
        let range = Integer::one().shifted_left(256);
        assert!(Weights::from_scaled(vec![range.sub(&Integer::one()), Integer::one()]).is_ok());
        assert_eq!(
            Weights::from_scaled(vec![range, Integer::one()]),
            Err(WeightsRefusal::TotalAboveDrawRange)
        );
    }

    /// A one-item choice computes no digest; a two-item one computes one or
    /// more.
    #[trace("TC-623")]
    #[test]
    fn a_one_item_choice_draws_nothing() {
        let at = ChoicePoint {
            seed: 424_242,
            trace: 0,
            step: 0,
            choice: 0,
        };
        assert_eq!(choose(at, &Weights::uniform(1).expect("one")), (0, 0));
        assert!(choose(at, &Weights::uniform(2).expect("two")).1 >= 1);
    }
}
