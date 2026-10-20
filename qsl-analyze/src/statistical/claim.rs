// SPDX-License-Identifier: AGPL-3.0-or-later
//! The part of a checked probabilistic claim EN-4 decides from (FR-189):
//! its form, the bound its counts are compared with, and its confidence
//! parameters; and the reduction of a quantile claim to a probability bound
//! over an event (ADR-024 PF-4).

use qsl_replay::statistical::{ClaimForm, Confidence, StatisticalRefusal};
use qsl_semantics::check::{CheckedProbabilisticClaim, ProbScheduler};
use quire_exact::Rational;

use super::bounds::int;

/// How a bound compares the true value with its threshold.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Comparison {
    /// `>= θ`.
    AtLeast,
    /// `<= θ`.
    AtMost,
    /// `< θ`: the strict bound `quantile q of M >= c` reduces to
    /// (FR-189 "Per sample"). Decided as `<= θ`, except that an interval
    /// touching `θ` is never accepted.
    Below,
}

/// A bound on a probability, a mean fraction or a long-run fraction: the
/// comparison and a threshold in `(0, 1)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bound {
    /// The comparison.
    pub comparison: Comparison,
    /// The threshold `θ`.
    pub threshold: Rational,
}

impl Bound {
    /// Whether the true value `value` meets the bound.
    pub fn holds(&self, value: &Rational) -> bool {
        match self.comparison {
            Comparison::AtLeast => *value >= self.threshold,
            Comparison::AtMost => *value <= self.threshold,
            Comparison::Below => *value < self.threshold,
        }
    }
}

/// What EN-4 needs of a checked probabilistic claim (FR-186's
/// `CheckedProbabilisticClaim`): its form, its bound and its confidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatisticalClaim {
    form: ClaimForm,
    bound: Bound,
    confidence: Confidence,
}

/// Why a form, a bound and confidence parameters are not a claim EN-4
/// decides.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ClaimRefusal {
    /// `θ − ι <= 0` or `θ + ι >= 1`: the indifference region leaves `(0, 1)`
    /// (FR-186 `invalid-confidence`), so SPRT's hypotheses are not
    /// probabilities.
    #[error("the indifference region about the threshold leaves (0, 1)")]
    NoIndifferenceRegion,
    /// A strict bound on a form other than a quantile.
    #[error("only a quantile claim reduces to a strict bound")]
    StrictBound,
}

impl StatisticalClaim {
    /// A claim of `form` with `bound` and `confidence`.
    ///
    /// # Errors
    ///
    /// [`ClaimRefusal::NoIndifferenceRegion`] when `[θ − ι, θ + ι]` is not
    /// inside `(0, 1)`; [`ClaimRefusal::StrictBound`] for a [`Comparison::Below`]
    /// bound on a form other than [`ClaimForm::Quantile`].
    pub fn new(
        form: ClaimForm,
        bound: Bound,
        confidence: Confidence,
    ) -> Result<Self, ClaimRefusal> {
        let iota = confidence.indifference();
        if bound.threshold.sub(iota) <= int(0) || bound.threshold.add(iota) >= int(1) {
            return Err(ClaimRefusal::NoIndifferenceRegion);
        }
        if bound.comparison == Comparison::Below && form != ClaimForm::Quantile {
            return Err(ClaimRefusal::StrictBound);
        }
        Ok(Self {
            form,
            bound,
            confidence,
        })
    }

    /// The claim's form.
    pub fn form(&self) -> ClaimForm {
        self.form
    }

    /// The bound the counts decide.
    pub fn bound(&self) -> &Bound {
        &self.bound
    }

    /// The claim's `α`, `β` and `ι`.
    pub fn confidence(&self) -> &Confidence {
        &self.confidence
    }
}

/// The direction of a quantile claim: `quantile q of M <= c` or `>= c`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum QuantileComparison {
    /// `quantile q of M <= c`.
    AtMost,
    /// `quantile q of M >= c`.
    AtLeast,
}

/// One activated sample's measure: a value, or censored, with no end
/// within the window (FR-189 "Per sample").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MeasureValue {
    /// The measure's value.
    Value(Rational),
    /// Activated with no end position: exceeds every finite threshold.
    Censored,
}

/// The event a quantile claim reduces to: `M <= c`, or `M < c` when
/// `strict` (ADR-024 PF-4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeasureEvent {
    /// Whether the event is `M < c` rather than `M <= c`.
    pub strict: bool,
    /// The threshold `c`.
    pub threshold: Rational,
}

impl MeasureEvent {
    /// Whether the event holds on a sample with measure `value`; a
    /// censored sample exceeds every finite threshold, so it never holds.
    pub fn holds(&self, value: &MeasureValue) -> bool {
        match value {
            MeasureValue::Value(value) if self.strict => *value < self.threshold,
            MeasureValue::Value(value) => *value <= self.threshold,
            MeasureValue::Censored => false,
        }
    }
}

/// Reduce `quantile q of M <= c` to `Pr(M <= c | activated) >= q`, and
/// `quantile q of M >= c` to the strict `Pr(M < c | activated) < q`
/// (FR-189 "Per sample"; ADR-024 PF-4).
pub fn reduce_quantile(
    level: Rational,
    comparison: QuantileComparison,
    threshold: Rational,
) -> (MeasureEvent, Bound) {
    match comparison {
        QuantileComparison::AtMost => (
            MeasureEvent {
                strict: false,
                threshold,
            },
            Bound {
                comparison: Comparison::AtLeast,
                threshold: level,
            },
        ),
        QuantileComparison::AtLeast => (
            MeasureEvent {
                strict: true,
                threshold,
            },
            Bound {
                comparison: Comparison::Below,
                threshold: level,
            },
        ),
    }
}

/// Whether statistical evidence can decide `claim` at all (ADR-024 SV-5,
/// ADR-028 XF-7): the refusal EN-4 gives at negotiation, before it draws a
/// sample, for a claim only exact evidence decides. A caller runs it on the
/// checked claim before it builds a [`StatisticalClaim`] for
/// [`check_statistical`](super::check_statistical); that function takes the
/// reduced claim and cannot see the scheduler or the form.
///
/// It refuses exactly the claims [`CheckedProbabilisticClaim::exact_only`]
/// marks. The cause is the first that applies: a claim under every scheduler,
/// then an exact-only form, then a claim that states no confidence
/// parameters.
///
/// # Errors
///
/// [`StatisticalRefusal::EveryScheduler`], [`StatisticalRefusal::ExactOnlyForm`]
/// or [`StatisticalRefusal::MissingConfidence`].
pub fn require_statistical(claim: &CheckedProbabilisticClaim) -> Result<(), StatisticalRefusal> {
    if !claim.exact_only() {
        return Ok(());
    }
    Err(if claim.scheduler == ProbScheduler::Every {
        StatisticalRefusal::EveryScheduler
    } else if claim.exact_only_form() {
        StatisticalRefusal::ExactOnlyForm
    } else {
        StatisticalRefusal::MissingConfidence
    })
}
