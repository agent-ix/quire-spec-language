// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-408's fixed-sample (Okamoto) and sequential (SPRT) methods over
//! one test's `α'`, `β'` and the claim's `ι` (ADR-024 ST-4, ST-5, ST-9), in
//! exact rationals with every rounding outward, toward undecided.

use qsl_replay::statistical::{Decision, UndecidedCause};
use quire_exact::{Integer, Rational};
use quire_semantic_value::catalog::InternalFault;

use super::bounds::{ceil, int, ln, quotient, Bounds};
use super::claim::{Bound, Comparison};

/// Decide `bound` over the interval `[estimate − half_width, estimate +
/// half_width]` (ADR-024 ST-4): a `>= θ` bound accepts when the interval
/// lies at or above `θ` and rejects when it lies below; a `<= θ` bound is
/// symmetric; the strict `< θ` bound accepts only when the interval lies
/// strictly below `θ` and rejects as `<= θ` does. Anything else is
/// undecided.
pub(crate) fn interval_decision(
    bound: &Bound,
    estimate: &Rational,
    half_width: &Rational,
) -> (Decision, (Rational, Rational)) {
    let lower = estimate.sub(half_width);
    let upper = estimate.add(half_width);
    let theta = &bound.threshold;
    let (accepted, rejected) = match bound.comparison {
        Comparison::AtLeast => (lower >= *theta, upper < *theta),
        Comparison::AtMost => (upper <= *theta, lower > *theta),
        Comparison::Below => (upper < *theta, lower > *theta),
    };
    let decision = if accepted {
        Decision::Accepted
    } else if rejected {
        Decision::Rejected
    } else {
        Decision::Undecided(UndecidedCause::IndifferenceRegion)
    };
    (decision, (lower, upper))
}

/// The lesser of `α'` and `β'`.
pub(crate) fn least<'r>(alpha: &'r Rational, beta: &'r Rational) -> &'r Rational {
    if alpha <= beta {
        alpha
    } else {
        beta
    }
}

/// Okamoto's sample count `N = ⌈ln(1 / min(α', β')) / (2 ι²)⌉`, with the
/// upper bound of the logarithm, rounded up.
pub(crate) fn okamoto_sample_count(
    alpha: &Rational,
    beta: &Rational,
    iota: &Rational,
) -> Result<Integer, InternalFault> {
    let log = ln(&quotient(&int(1), least(alpha, beta))?)?.upper;
    Ok(ceil(&quotient(&log, &int(2).mul(&iota.mul(iota)))?))
}

/// SPRT's thresholds, each rounded away from zero: the rejection threshold
/// the upper bound of `ln(1/α')`, the acceptance threshold the lower bound
/// of `ln(β')`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SprtThresholds {
    pub(crate) reject: Rational,
    pub(crate) accept: Rational,
}

pub(crate) fn sprt_thresholds(
    alpha: &Rational,
    beta: &Rational,
) -> Result<SprtThresholds, InternalFault> {
    Ok(SprtThresholds {
        reject: ln(&quotient(&int(1), alpha)?)?.upper,
        accept: ln(beta)?.lower,
    })
}

/// One test's sequential probability ratio test (ADR-024 ST-5): the bounds
/// on each sample's increment and the counts of holding and failing
/// samples, from which the lower and upper sums follow exactly.
#[derive(Clone, Debug)]
pub(crate) struct Sprt {
    holds: Bounds,
    fails: Bounds,
    thresholds: SprtThresholds,
    held: u64,
    failed: u64,
}

impl Sprt {
    /// The test of `bound` at `ι` with per-test `α'` and `β'`. For `>= θ`
    /// it tests `H0: p >= θ + ι` against `H1: p <= θ − ι`; a `<= θ` or
    /// `< θ` bound swaps the hypotheses. A holding sample adds
    /// `ln(p1 / p0)`, a failing one `ln((1 − p1) / (1 − p0))`, `p1` the
    /// hypothesis whose acceptance rejects the bound.
    pub(crate) fn new(
        bound: &Bound,
        iota: &Rational,
        alpha: &Rational,
        beta: &Rational,
    ) -> Result<Self, InternalFault> {
        let above = bound.threshold.add(iota);
        let below = bound.threshold.sub(iota);
        let (p0, p1) = match bound.comparison {
            Comparison::AtLeast => (above, below),
            Comparison::AtMost | Comparison::Below => (below, above),
        };
        let holds = ln(&quotient(&p1, &p0)?)?;
        let fails = ln(&quotient(&int(1).sub(&p1), &int(1).sub(&p0))?)?;
        Ok(Self {
            holds,
            fails,
            thresholds: sprt_thresholds(alpha, beta)?,
            held: 0,
            failed: 0,
        })
    }

    /// Count one sample, by whether its event held.
    pub(crate) fn count(&mut self, held: bool) {
        if held {
            self.held += 1;
        } else {
            self.failed += 1;
        }
    }

    /// The decision the sums reach, if any: rejected when the lower sum
    /// reaches the rejection threshold, accepted when the upper sum reaches
    /// the acceptance threshold.
    pub(crate) fn decision(&self) -> Option<Decision> {
        let held = Rational::from_integer(Integer::from(self.held));
        let failed = Rational::from_integer(Integer::from(self.failed));
        let lower = held
            .mul(&self.holds.lower)
            .add(&failed.mul(&self.fails.lower));
        if lower >= self.thresholds.reject {
            return Some(Decision::Rejected);
        }
        let upper = held
            .mul(&self.holds.upper)
            .add(&failed.mul(&self.fails.upper));
        (upper <= self.thresholds.accept).then_some(Decision::Accepted)
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    fn ratio(numerator: i64, denominator: i64) -> Rational {
        quotient(&int(numerator), &int(denominator)).expect("nonzero denominator")
    }

    /// `N` is 23,026 at `α' = β' = ι = 1/100`, 26,492 at `α' = 1/200` (two
    /// tests) and 9,210,341 at `ι = 1/2000` (ADR-024 §7.1's `NoFault`).
    #[trace("TC-624", "FR-189-AC-2")]
    #[test]
    fn okamoto_sample_counts_match_the_spec_vectors() {
        let hundredth = ratio(1, 100);
        let n = |alpha: &Rational, iota: &Rational| {
            okamoto_sample_count(alpha, alpha, iota).expect("sample count")
        };
        assert_eq!(n(&hundredth, &hundredth), Integer::from(23_026_u64));
        assert_eq!(n(&ratio(1, 200), &hundredth), Integer::from(26_492_u64));
        assert_eq!(n(&hundredth, &ratio(1, 2000)), Integer::from(9_210_341_u64));
    }

    /// The thresholds bound `±ln 100` outward, each within `2^-64` of it.
    #[trace("TC-624", "FR-189-AC-2")]
    #[test]
    fn sprt_thresholds_bound_ln_100_outward_within_two_to_the_minus_64() {
        let thresholds = sprt_thresholds(&ratio(1, 100), &ratio(1, 100)).expect("thresholds");
        // ln 100 = 4.605170185988091368035982909368728415202202977257…
        let digits: Integer = "4605170185988091368035982909368728415202"
            .parse()
            .expect("digits");
        let ln_lower = Rational::from_integer(digits).divided_by_power_of_ten(39);
        let ln_upper = ln_lower.add(&int(1).divided_by_power_of_ten(39));
        let precision = int(1).divided_by_power_of_two(64);
        assert!(thresholds.reject >= ln_upper);
        assert!(thresholds.reject.sub(&ln_lower) < precision);
        assert!(thresholds.accept <= ln_lower.neg());
        assert!(ln_upper.neg().sub(&thresholds.accept) < precision);
    }

    #[test]
    fn the_interval_rule_follows_each_comparison() {
        let half = ratio(1, 2);
        let tenth = ratio(1, 10);
        let bound = |comparison| Bound {
            comparison,
            threshold: half.clone(),
        };
        let decide = |comparison, estimate: Rational| {
            interval_decision(&bound(comparison), &estimate, &tenth).0
        };
        let undecided = Decision::Undecided(UndecidedCause::IndifferenceRegion);
        assert_eq!(
            decide(Comparison::AtLeast, ratio(6, 10)),
            Decision::Accepted
        );
        assert_eq!(
            decide(Comparison::AtLeast, ratio(39, 100)),
            Decision::Rejected
        );
        assert_eq!(decide(Comparison::AtLeast, ratio(55, 100)), undecided);
        assert_eq!(decide(Comparison::AtMost, ratio(4, 10)), Decision::Accepted);
        assert_eq!(
            decide(Comparison::AtMost, ratio(61, 100)),
            Decision::Rejected
        );
        // The strict bound never accepts an interval touching θ.
        assert_eq!(decide(Comparison::Below, ratio(4, 10)), undecided);
        assert_eq!(
            decide(Comparison::Below, ratio(39, 100)),
            Decision::Accepted
        );
        assert_eq!(
            decide(Comparison::Below, ratio(61, 100)),
            Decision::Rejected
        );
    }

    #[test]
    fn sprt_decides_by_the_two_sums() {
        let bound = Bound {
            comparison: Comparison::AtLeast,
            threshold: ratio(95, 100),
        };
        let hundredth = ratio(1, 100);
        let mut test = Sprt::new(&bound, &hundredth, &hundredth, &hundredth).expect("sprt");
        // ADR-024 §7.2: with no failing sample SPRT accepts after 219.
        for _ in 0..218 {
            test.count(true);
            assert_eq!(test.decision(), None);
        }
        test.count(true);
        assert_eq!(test.decision(), Some(Decision::Accepted));
        let mut failing = Sprt::new(&bound, &hundredth, &hundredth, &hundredth).expect("sprt");
        // A failing sample adds ln 1.5 ≈ 0.4055; ln 100 / ln 1.5 ≈ 11.36.
        for _ in 0..11 {
            failing.count(false);
            assert_eq!(failing.decision(), None);
        }
        failing.count(false);
        assert_eq!(failing.decision(), Some(Decision::Rejected));
    }

    /// QSpec FR-408-AC-7: a sum within the rounding error of a threshold
    /// gives no decision. With `p0 = 2/5`, `p1 = 1/5` a holding sample adds
    /// exactly `ln(1/2)`, so four of them reach `ln(β') = ln(1/16)` exactly;
    /// the outward sums do not, and the test decides only at the fifth.
    #[test]
    fn a_sum_on_the_threshold_within_rounding_is_undecided() {
        let bound = Bound {
            comparison: Comparison::AtLeast,
            threshold: ratio(3, 10),
        };
        let sixteenth = ratio(1, 16);
        let mut test = Sprt::new(&bound, &ratio(1, 10), &sixteenth, &sixteenth).expect("sprt");
        for _ in 0..4 {
            test.count(true);
        }
        assert_eq!(test.decision(), None);
        test.count(true);
        assert_eq!(test.decision(), Some(Decision::Accepted));
    }
}
