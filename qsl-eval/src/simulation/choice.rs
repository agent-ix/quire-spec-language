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
    /// The preimage of draw `draw` at `point`.
    pub(crate) fn at(point: ChoicePoint, draw: u64) -> Self {
        Self {
            choice: point.choice.to_string(),
            draw: draw.to_string(),
            seed: point.seed.to_string(),
            step: point.step.to_string(),
            trace: point.trace.to_string(),
        }
    }
}

/// The scaled total `N` of a choice's weights exceeds `2^256`, so the
/// generator's acceptance test accepts no draw and a draw would never end
/// (FR-188; QSpec FR-181-AC-14, `total-above-draw-range{total}`).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("the scaled weight total {total} exceeds 2^256")]
pub struct TotalAboveDrawRange {
    /// The scaled total `N`, above `2^256`.
    pub total: Integer,
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
    #[error(transparent)]
    TotalAboveDrawRange(TotalAboveDrawRange),
}

/// The integer weights of one choice: positive exact rationals scaled by the
/// least common multiple of their denominators, with their total `n` and the
/// acceptance bound `n · ⌊2^256 / n⌋` (FR-188).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Weights {
    /// `w_0, …, w_{k−1}`, in the caller's order.
    items: Items,
    /// `n`, the sum of the scaled weights; at least 1.
    total: Integer,
    /// `n · ⌊2^256 / n⌋`: a draw below it is accepted.
    accept_below: Integer,
}

/// The scaled weights of a choice. Unit weights are held as their count, so
/// a uniform choice selects `v mod n` directly: the least index whose
/// running sum of ones exceeds `v mod n` is `v mod n` itself.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Items {
    /// `count` weights of 1.
    Unit(usize),
    /// Any other positive integer weights.
    Scaled(Vec<Integer>),
}

impl Items {
    fn len(&self) -> usize {
        match self {
            Self::Unit(count) => *count,
            Self::Scaled(scaled) => scaled.len(),
        }
    }
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

    fn from_scaled(scaled: Vec<Integer>) -> Result<Self, WeightsRefusal> {
        let one = Integer::one();
        if scaled.iter().all(|weight| *weight == one) {
            return Self::uniform(scaled.len());
        }
        let total = scaled
            .iter()
            .fold(Integer::zero(), |total, weight| total.add(weight));
        Self::with_total(Items::Scaled(scaled), total)
    }

    /// `count` unit weights: the uniform choice over `count` items.
    ///
    /// # Errors
    ///
    /// [`WeightsRefusal::Empty`] for `count = 0`;
    /// [`WeightsRefusal::TotalAboveDrawRange`] cannot arise, since `count`
    /// fits a `usize`.
    pub fn uniform(count: usize) -> Result<Self, WeightsRefusal> {
        Self::with_total(Items::Unit(count), Integer::from(count))
    }

    fn with_total(items: Items, total: Integer) -> Result<Self, WeightsRefusal> {
        if items.len() == 0 {
            return Err(WeightsRefusal::Empty);
        }
        // Every scaled weight is positive, so a non-empty total is too.
        let divisor = proven_positive(&total);
        let range = Integer::one().shifted_left(256);
        let (quotient, _) = range.div_mod_floor(divisor);
        if quotient.is_zero() {
            return Err(WeightsRefusal::TotalAboveDrawRange(TotalAboveDrawRange {
                total,
            }));
        }
        let accept_below = quotient.mul(&total);
        Ok(Self {
            items,
            total,
            accept_below,
        })
    }

    /// The scaled integer weights, in the caller's order.
    pub fn scaled(&self) -> Vec<Integer> {
        match &self.items {
            Items::Unit(count) => vec![Integer::one(); *count],
            Items::Scaled(scaled) => scaled.clone(),
        }
    }

    /// How many items the choice is over.
    pub(crate) fn len(&self) -> usize {
        self.items.len()
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
        let scaled = match &self.items {
            Items::Unit(_) => {
                return remainder
                    .to_u64()
                    .and_then(|index| usize::try_from(index).ok())
                    .expect("a remainder below a usize count fits a usize");
            }
            Items::Scaled(scaled) => scaled,
        };
        let mut cumulative = Integer::zero();
        for (index, weight) in scaled.iter().enumerate() {
            cumulative = cumulative.add(weight);
            if *remainder < cumulative {
                return index;
            }
        }
        // `remainder < total`, the last cumulative sum, so the loop returns.
        scaled.len() - 1
    }
}

/// `value` as a divisor. Every caller passes an integer it has proven
/// positive, as `quire_exact`'s own reduction does; a zero here is a defect
/// of this module, never a property of the input.
fn proven_positive(value: &Integer) -> NonZeroInteger<'_> {
    NonZeroInteger::try_from(value).expect("a weight denominator, gcd or total is positive")
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
    if weights.items.len() == 1 {
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

/// [`weighted_choice`], with how many draw digests it computed: none when
/// the weights are refused or there is one of them.
pub(crate) fn weighted_choice_counted(
    at: ChoicePoint,
    weights: &[Rational],
) -> (Result<usize, WeightsRefusal>, u64) {
    match Weights::new(weights) {
        Ok(weights) => {
            let (index, digests) = choose(at, &weights);
            (Ok(index), digests)
        }
        Err(refusal) => (Err(refusal), 0),
    }
}

/// The index of `weights` the generator selects at `at` (FR-188): the
/// weights scaled to integers by the least common multiple of their
/// denominators, each index selected with probability exactly its scaled
/// weight over their total. Exact, reproducible from the seed and
/// independent of every other choice point. A one-item choice selects index
/// 0 and computes no digest.
///
/// # Errors
///
/// [`WeightsRefusal::TotalAboveDrawRange`] when the scaled total exceeds
/// `2^256`, before any digest is computed; [`WeightsRefusal::Empty`] or
/// [`WeightsRefusal::NotPositive`] for a list that is not one of positive
/// weights.
pub fn weighted_choice(at: ChoicePoint, weights: &[Rational]) -> Result<usize, WeightsRefusal> {
    weighted_choice_counted(at, weights).0
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
            Weights::from_scaled(vec![range.clone(), Integer::one()]),
            Err(WeightsRefusal::TotalAboveDrawRange(TotalAboveDrawRange {
                total: range.add(&Integer::one())
            }))
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

    fn at(trace: u64) -> ChoicePoint {
        ChoicePoint {
            seed: 424_242,
            trace,
            step: 0,
            choice: 0,
        }
    }

    /// FR-188-AC-5: weights `(2^256 − 1, 1)` scale to the total `2^256`,
    /// which accepts every draw: each choice computes exactly one digest and
    /// selects index 1 only when `v` is the last value below `2^256`.
    #[trace("TC-623", "FR-188-AC-5")]
    #[test]
    fn a_total_of_two_to_the_256_accepts_every_draw() {
        let range = Integer::one().shifted_left(256);
        let first = range.sub(&Integer::one());
        let weights = [
            Rational::from_integer(first.clone()),
            Rational::from_integer(Integer::one()),
        ];
        for trace in 0..50 {
            let (selected, digests) = weighted_choice_counted(at(trace), &weights);
            assert_eq!(digests, 1, "trace {trace}");
            let v = big_endian(plain_digest(&DrawPreimage::at(at(trace), 0)));
            assert_eq!(selected, Ok(usize::from(v >= first)), "trace {trace}");
        }
    }

    /// FR-188-AC-5: weights `(2^256, 1)` in canonical value order (QSpec
    /// TC-354 SM-09) are refused `TotalAboveDrawRange{total: 2^256 + 1}`
    /// with no digest computed.
    #[trace("TC-623", "FR-188-AC-5")]
    #[test]
    fn weights_above_the_draw_range_refuse_before_any_digest() {
        let range = Integer::one().shifted_left(256);
        let weights = [
            Rational::from_integer(range.clone()),
            Rational::from_integer(Integer::one()),
        ];
        assert_eq!(
            weighted_choice_counted(at(0), &weights),
            (
                Err(WeightsRefusal::TotalAboveDrawRange(TotalAboveDrawRange {
                    total: range.add(&Integer::one())
                })),
                0
            )
        );
    }

    /// FR-188-AC-4: `weighted_choice` over one weight returns 0 with no
    /// digest computed.
    #[trace("TC-623", "FR-188-AC-4")]
    #[test]
    fn one_weight_selects_index_zero_without_a_digest() {
        let weights = [Rational::from_integer(Integer::from(7u64))];
        assert_eq!(weighted_choice_counted(at(0), &weights), (Ok(0), 0));
    }

    /// The unit-weight fast path selects exactly what the running-sum
    /// selection over the same ones selects.
    #[trace("TC-623", "FR-188-AC-2")]
    #[test]
    fn unit_weights_select_as_the_running_sum_over_ones() {
        for count in [2usize, 3, 5, 7] {
            let fast = Weights::uniform(count).expect("items");
            let slow = Weights::with_total(
                Items::Scaled(vec![Integer::one(); count]),
                Integer::from(count),
            )
            .expect("items");
            for trace in 0..200 {
                let at = ChoicePoint {
                    seed: 424_242,
                    trace,
                    step: 0,
                    choice: 0,
                };
                assert_eq!(choose(at, &fast), choose(at, &slow), "n = {count}");
            }
        }
    }
}
