// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-408's regenerative method (FR-190; ADR-024 ST-7): the ratio
//! estimator over closed regeneration cycles and its Chow–Robbins
//! half-width `z · √(S² + 1/n) / (L̄ · √n)`, rounded up.
//!
//! The sums `Σ Y`, `Σ L`, `Σ Y²`, `Σ Y·L` and `Σ L²` are kept exactly, so
//! `S² = Σ (Y_k − Â·L_k)² / (n − 1)` follows from them after each cycle
//! without revisiting the cycles.

use quire_exact::{Integer, Rational};
use quire_semantic_value::catalog::InternalFault;

use super::bounds::{int, quotient, sqrt};

/// The closed cycles of one test, as exact sums.
#[derive(Clone, Debug)]
pub(crate) struct Cycles {
    count: u64,
    y: Rational,
    l: Rational,
    yy: Rational,
    yl: Rational,
    ll: Rational,
}

/// The estimate and the upper bound on its half-width after a cycle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RatioInterval {
    pub(crate) estimate: Rational,
    pub(crate) half_width: Rational,
}

impl Cycles {
    pub(crate) fn new() -> Self {
        Self {
            count: 0,
            y: int(0),
            l: int(0),
            yy: int(0),
            yl: int(0),
            ll: int(0),
        }
    }

    /// Close one cycle with weighted count `y` and total weight `l`. A
    /// cycle of weight 0 is kept.
    pub(crate) fn close(&mut self, y: &Rational, l: &Rational) {
        self.count += 1;
        self.y = self.y.add(y);
        self.l = self.l.add(l);
        self.yy = self.yy.add(&y.mul(y));
        self.yl = self.yl.add(&y.mul(l));
        self.ll = self.ll.add(&l.mul(l));
    }

    /// The cycles closed.
    pub(crate) fn count(&self) -> u64 {
        self.count
    }

    /// `Â = Σ Y / Σ L`; `None` while every closed cycle has weight 0.
    pub(crate) fn estimate(&self) -> Option<Rational> {
        self.y.div(&self.l)
    }

    /// The estimate and the half-width's upper bound, using `z_upper` for
    /// `z`, the upper square-root bound of `S² + 1/n`, the exact `L̄` and
    /// the lower bound of `√n`. `None` while there is no estimate or fewer
    /// than two cycles, which have no sample variance.
    pub(crate) fn interval(
        &self,
        z_upper: &Rational,
    ) -> Result<Option<RatioInterval>, InternalFault> {
        let Some(estimate) = self.estimate() else {
            return Ok(None);
        };
        if self.count < 2 {
            return Ok(None);
        }
        let n = Rational::from_integer(Integer::from(self.count));
        let squares = self
            .yy
            .sub(&int(2).mul(&estimate).mul(&self.yl))
            .add(&estimate.mul(&estimate).mul(&self.ll));
        let variance = quotient(&squares, &n.sub(&int(1)))?;
        let root = sqrt(&variance.add(&quotient(&int(1), &n)?))?.upper;
        let mean_weight = quotient(&self.l, &n)?;
        let root_n = sqrt(&n)?.lower;
        let half_width = quotient(&z_upper.mul(&root), &mean_weight.mul(&root_n))?;
        Ok(Some(RatioInterval {
            estimate,
            half_width,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ratio(numerator: i64, denominator: i64) -> Rational {
        quotient(&int(numerator), &int(denominator)).expect("nonzero denominator")
    }

    #[test]
    fn the_incremental_variance_equals_the_two_pass_variance() {
        let cycles = [(1, 1), (1, 3), (0, 2), (2, 2), (1, 1)];
        let mut tally = Cycles::new();
        for (y, l) in cycles {
            tally.close(&int(y), &int(l));
        }
        let z = int(2);
        let got = tally.interval(&z).expect("interval").expect("an estimate");
        let estimate = ratio(5, 9);
        assert_eq!(got.estimate, estimate);
        let squares = cycles.iter().fold(int(0), |sum, (y, l)| {
            let residual = int(*y).sub(&estimate.mul(&int(*l)));
            sum.add(&residual.mul(&residual))
        });
        let n = int(5);
        let inner = quotient(&squares, &int(4))
            .expect("variance")
            .add(&ratio(1, 5));
        let expected = quotient(
            &z.mul(&sqrt(&inner).expect("root").upper),
            &ratio(9, 5).mul(&sqrt(&n).expect("√n").lower),
        )
        .expect("half-width");
        assert_eq!(got.half_width, expected);
    }

    #[test]
    fn weightless_cycles_have_no_estimate_and_one_cycle_no_interval() {
        let mut tally = Cycles::new();
        tally.close(&int(0), &int(0));
        tally.close(&int(0), &int(0));
        assert_eq!(tally.count(), 2);
        assert_eq!(tally.estimate(), None);
        assert_eq!(tally.interval(&int(2)).expect("interval"), None);
        let mut single = Cycles::new();
        single.close(&int(1), &int(1));
        assert_eq!(single.estimate(), Some(int(1)));
        assert_eq!(single.interval(&int(2)).expect("interval"), None);
    }
}
