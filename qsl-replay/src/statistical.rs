// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-4's result types (FR-189 to FR-191; ADR-024 ST-1, SV-2, RP-1): the
//! method and budgets of a statistical request, each test's result, the
//! verdict, its basis and provenance, and the outcome and refusals of a
//! run.
//!
//! The engine that computes them is `qsl-analyze`'s `statistical` module
//! (layer A). They live here, in layer 6, because ADR-024 ST-1 places every
//! verdict type the settlement map and CG read in `qsl-replay`.

use qsl_eval::model_subject::ModelInstance;
use qsl_eval::simulation::model::{ClauseOutcome, NotMarkov};
use qsl_eval::simulation::{ExpansionStop, RequiresBound, SampledStep};
use qsl_foundation::selection::DefinitionRef;
use qsl_foundation::InternalFault;
use qsl_semantics::model::observation::AdmissionFailure;
use quire_exact::{CancelCause, Integer, Rational};

use crate::proof_result::SymmetryRefusal;
use crate::UndefinedEvaluation;

mod settle;
mod witness;

pub use settle::{
    settle_statistical, settle_statistical_refusal, NotStatisticalRequest, StatisticalRecord,
    SubjectLimits, UnsettledRefusal,
};
pub use witness::{
    replay_sampled_witness, SampledParity, SampledWitnessRefusal, SampledWitnessReplay,
    WitnessReadStop, WitnessReader, WitnessReading, WitnessReplay,
};

/// How a request decides a claim from its samples (FR-189, FR-190;
/// ADR-024 ST-4, ST-5, ST-7). A request setting, outside the obligation
/// identity (ST-8).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StatisticalMethod {
    /// Okamoto's fixed-sample bound (ST-4).
    Okamoto,
    /// Wald's sequential probability ratio test (ST-5).
    Sprt,
    /// The regenerative ratio estimator for a long-run fraction (ST-7),
    /// closing at least `min_cycles` cycles per test before it decides.
    Regenerative {
        /// The fewest regeneration cycles a test closes before it decides.
        min_cycles: u64,
    },
}

impl StatisticalMethod {
    /// The coverage of the method's interval (ADR-024 SV-2): `Asymptotic`
    /// for the regenerative method, `FiniteSample` for the others.
    pub const fn coverage(self) -> Coverage {
        match self {
            Self::Okamoto | Self::Sprt => Coverage::FiniteSample,
            Self::Regenerative { .. } => Coverage::Asymptotic,
        }
    }
}

/// Whether an interval's coverage is a finite-sample bound or a limit
/// (ADR-024 SV-2, RU-1).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Coverage {
    /// Okamoto and SPRT: the error bounds hold at every sample size.
    FiniteSample,
    /// The regenerative method: coverage approaches the level as the cycle
    /// count grows.
    Asymptotic,
}

/// The form of a probabilistic claim, as EN-4 decides it (ADR-024 PF-1 to
/// PF-6).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ClaimForm {
    /// A probability bound over an event.
    Probability,
    /// A quantile of a measure, decided as a probability bound conditioned
    /// on activation (PF-4).
    Quantile,
    /// A mean of a fraction (PF-5); never decided by SPRT.
    MeanOfFraction,
    /// A long-run fraction (PF-6); decided only by the regenerative method.
    LongRunFraction,
}

impl ClaimForm {
    /// Whether `method` decides this form (FR-189 "Methods", FR-190):
    /// the regenerative method decides exactly a long-run fraction, SPRT a
    /// probability bound or a quantile, Okamoto every form but a long-run
    /// fraction.
    pub const fn admits(self, method: StatisticalMethod) -> bool {
        match (self, method) {
            (Self::LongRunFraction, StatisticalMethod::Regenerative { .. })
            | (Self::Probability | Self::Quantile, StatisticalMethod::Sprt)
            | (
                Self::Probability | Self::Quantile | Self::MeanOfFraction,
                StatisticalMethod::Okamoto,
            ) => true,
            (Self::LongRunFraction, StatisticalMethod::Okamoto | StatisticalMethod::Sprt)
            | (
                Self::Probability | Self::Quantile | Self::MeanOfFraction,
                StatisticalMethod::Regenerative { .. },
            )
            | (Self::MeanOfFraction, StatisticalMethod::Sprt) => false,
        }
    }
}

/// A claim's confidence parameters (ADR-024 PF-10): the error bounds `α`
/// and `β` and the indifference half-width `ι`, each in `(0, 1)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Confidence {
    alpha: Rational,
    beta: Rational,
    indifference: Rational,
}

/// Why three rationals are not a claim's confidence parameters: the one
/// that lies outside `(0, 1)`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ConfidenceRefusal {
    /// `α` is not in `(0, 1)`.
    #[error("alpha is not in (0, 1)")]
    Alpha,
    /// `β` is not in `(0, 1)`.
    #[error("beta is not in (0, 1)")]
    Beta,
    /// `ι` is not in `(0, 1)`.
    #[error("indifference is not in (0, 1)")]
    Indifference,
}

fn in_unit_open(value: &Rational) -> bool {
    let zero = Rational::from_integer(Integer::zero());
    let one = Rational::from_integer(Integer::one());
    *value > zero && *value < one
}

impl Confidence {
    /// The parameters `α`, `β` and `ι`.
    ///
    /// # Errors
    ///
    /// The first parameter, in that order, that is not in `(0, 1)`.
    pub fn new(
        alpha: Rational,
        beta: Rational,
        indifference: Rational,
    ) -> Result<Self, ConfidenceRefusal> {
        if !in_unit_open(&alpha) {
            return Err(ConfidenceRefusal::Alpha);
        }
        if !in_unit_open(&beta) {
            return Err(ConfidenceRefusal::Beta);
        }
        if !in_unit_open(&indifference) {
            return Err(ConfidenceRefusal::Indifference);
        }
        Ok(Self {
            alpha,
            beta,
            indifference,
        })
    }

    /// `α`, the bound on wrongly rejecting.
    pub fn alpha(&self) -> &Rational {
        &self.alpha
    }

    /// `β`, the bound on wrongly accepting.
    pub fn beta(&self) -> &Rational {
        &self.beta
    }

    /// `ι`, the indifference half-width.
    pub fn indifference(&self) -> &Rational {
        &self.indifference
    }
}

/// EN-4's caller-set budgets (FR-191; ADR-024 ST-8), each an ADR-014 B-5
/// budget with a published default. None enters the obligation identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StatisticalLimits {
    /// Counted samples, or closed cycles, per test.
    pub max_samples: u64,
    /// Samples drawn per test, counted or not; steps taken for the
    /// regenerative method.
    pub max_draws: u64,
    /// Steps in one regeneration cycle.
    pub max_cycle_steps: u64,
}

impl Default for StatisticalLimits {
    /// FR-191's published defaults: `max_samples` 2^24, `max_draws` 2^26,
    /// `max_cycle_steps` 2^24.
    fn default() -> Self {
        Self {
            max_samples: 1 << 24,
            max_draws: 1 << 26,
            max_cycle_steps: 1 << 24,
        }
    }
}

impl StatisticalLimits {
    /// The value this request sets for `limit`.
    pub const fn value(&self, limit: StatisticalLimit) -> u64 {
        match limit {
            StatisticalLimit::MaxSamples => self.max_samples,
            StatisticalLimit::MaxDraws => self.max_draws,
            StatisticalLimit::MaxCycleSteps => self.max_cycle_steps,
        }
    }
}

/// One member of [`StatisticalLimits`] (FR-191).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StatisticalLimit {
    /// [`StatisticalLimits::max_samples`].
    MaxSamples,
    /// [`StatisticalLimits::max_draws`].
    MaxDraws,
    /// [`StatisticalLimits::max_cycle_steps`].
    MaxCycleSteps,
}

impl StatisticalLimit {
    /// The budget's FR-331 `limit` spelling: kebab-case, as every
    /// [`ModelCheckLimit`](crate::ModelCheckLimit) spells its limit; the
    /// snake-case member name is its setting's suffix ([`Self::setting`]).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MaxSamples => "max-samples",
            Self::MaxDraws => "max-draws",
            Self::MaxCycleSteps => "max-cycle-steps",
        }
    }

    /// The request setting that raises this limit, by FR-255's convention
    /// (`statistical.` and the member name; QSpec FR-408 owns the name).
    pub const fn setting(self) -> &'static str {
        match self {
            Self::MaxSamples => "statistical.max_samples",
            Self::MaxDraws => "statistical.max_draws",
            Self::MaxCycleSteps => "statistical.max_cycle_steps",
        }
    }
}

/// A test's or a claim's decision (FR-189).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Decision {
    /// The bound holds, wrong with probability at most `β'`.
    Accepted,
    /// The bound fails, wrong with probability at most `α'`, or a sample
    /// evaluated undefined.
    Rejected,
    /// The method completed without deciding.
    Undecided(UndecidedCause),
}

/// Why a completed test is undecided (FR-189).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UndecidedCause {
    /// The interval straddles the threshold: the true value lies within the
    /// indifference region.
    IndifferenceRegion,
    /// A contract conjunction evaluated undecided during a sampled step
    /// (ADR-024 ST-10).
    UndecidedSuccessor,
}

/// The sample on which a claim evaluated undefined (ADR-024 SV-11): the
/// test index, the trace index and, in `evaluation`, the position, the
/// state key there, the expression and the cause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SampledUndefined {
    /// The test's index.
    pub test: u32,
    /// The sample's trace index.
    pub trace: u64,
    /// Where in the sample, and why.
    pub evaluation: UndefinedEvaluation,
}

/// One statistical test's result (FR-189 `TestResult`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestResult {
    /// The test's index `j`, in subject order.
    pub index: u32,
    /// The test's initial state, as its index in the subject's ordered
    /// initial states.
    pub initial: u32,
    /// The `over` binding, when the claim has one.
    pub binding: Option<ModelInstance>,
    /// The test's decision.
    pub decision: Decision,
    /// The exact point estimate; `None` while the test has counted no
    /// sample, or for the regenerative method no cycle of positive weight.
    pub estimate: Option<Rational>,
    /// The interval, for Okamoto and the regenerative method; `None` for
    /// SPRT and for a test that ended before its method decided.
    pub interval: Option<(Rational, Rational)>,
    /// A quantile claim's empirical nearest-rank `q`-quantile of the
    /// measures of the activated samples counted (FR-192 "estimate"), the
    /// `⌈q · n⌉`-th smallest of the `n` counted; censored when that rank
    /// lies among the censored samples, which exceed every value. `None`
    /// for any other form and for a test that counted no sample.
    pub quantile: Option<MeasureValue>,
    /// Samples counted, or cycles closed.
    pub samples: u64,
    /// Samples drawn, or steps taken.
    pub draws: u64,
    /// The undefined evaluation that rejected the test, if one did.
    pub undefined: Option<SampledUndefined>,
}

/// What a statistical result rests on (FR-192 `StatisticalBasis`; ADR-024
/// SV-2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatisticalBasis {
    /// The request's method.
    pub method: StatisticalMethod,
    /// The claim's `α`, `β` and `ι`.
    pub confidence: Confidence,
    /// Samples counted, or cycles closed, over every test.
    pub samples: u64,
    /// Samples drawn, or steps taken, over every test.
    pub draws: u64,
    /// The test count `m`.
    pub tests: u32,
    /// `α' = α / m`.
    pub alpha_per_test: Rational,
    /// `β' = β / m`.
    pub beta_per_test: Rational,
    /// The method's coverage.
    pub coverage: Coverage,
}

/// The trace indices one test used (FR-191; ADR-024 RP-1): `count` indices
/// `first`, `first + step`, … — `i · m + j` for sample `i` of test `j`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TraceRange {
    /// The first trace index, `j`.
    pub first: u64,
    /// The stride, `m`.
    pub step: u64,
    /// How many indices the test used.
    pub count: u64,
}

/// One test's provenance (FR-191).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestProvenance {
    /// The test's index.
    pub index: u32,
    /// The test's initial state index.
    pub initial: u32,
    /// The `over` binding.
    pub binding: Option<ModelInstance>,
    /// The trace indices the test used.
    pub traces: TraceRange,
}

/// What a statistical run reproduces from (FR-191; ADR-024 RP-1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatisticalProvenance {
    /// The run's seed.
    pub seed: u64,
    /// The sampler's identity.
    pub sampler: DefinitionRef,
    /// Each test run, in order.
    pub tests: Vec<TestProvenance>,
}

/// What a sampled witness's claim evaluated to (FR-193): the sampled value
/// on the violating side of the claim's bound, or the undefined evaluation
/// that rejected its test.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WitnessValue {
    /// The event's value (a quantile claim's reduced event among them):
    /// `false` against a `>= θ` bound, `true` against a `<= θ` bound.
    Event(bool),
    /// A mean-of-fraction sample's fraction, below `θ` against a `>= θ`
    /// bound or above it against a `<= θ` bound.
    Fraction(Rational),
    /// A quantile claim's activated sample whose measure is on the violating
    /// side of its PF-4 event: the measure's value, or censored.
    Measure(MeasureValue),
    /// The claim evaluated undefined on the sample (ADR-024 SV-11).
    Undefined(UndefinedEvaluation),
}

/// One activated sample's bounded measure (FR-189 "Per sample"; ADR-024
/// PF-2): its value, or censored, with no end within the window.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MeasureValue {
    /// The measure's value, in the reward's own unit for an `accumulate`.
    Value(Rational),
    /// Activated with no end position: exceeds every finite threshold.
    Censored,
}

/// One sampled witness (FR-193; ADR-024 SV-6): a sample on the violating
/// side of the claim's bound, or one on which the claim evaluated
/// undefined, with the steps it took and their exact path probability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SampledWitness {
    /// The test's index.
    pub test: u32,
    /// The sample's trace index.
    pub trace: u64,
    /// The sample's steps (FR-188), one per position after the first.
    pub steps: Vec<SampledStep>,
    /// The product of the steps' probabilities.
    pub path_probability: Rational,
    /// What the claim evaluated to on the sample.
    pub value: WitnessValue,
}

/// A completed run's verdict (FR-192 `StatisticalVerdict`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatisticalVerdict {
    /// Accepted when every test accepts, Rejected when any rejects,
    /// Undecided otherwise.
    pub decision: Decision,
    /// The least favourable test's estimate.
    pub estimate: Option<Rational>,
    /// The least favourable test's interval.
    pub interval: Option<(Rational, Rational)>,
    /// The least favourable test's empirical quantile (FR-192).
    pub quantile: Option<MeasureValue>,
    /// What the verdict rests on.
    pub basis: StatisticalBasis,
    /// What it reproduces from.
    pub provenance: StatisticalProvenance,
    /// Each test's result, in order.
    pub tests: Vec<TestResult>,
    /// The sampled witnesses kept (FR-193): every undefined one, and up to
    /// `max_witnesses` others, the first in trace-index order across tests.
    pub witnesses: Vec<SampledWitness>,
}

impl StatisticalVerdict {
    /// Bytes the verdict adds to an encoded record beyond its fixed size:
    /// its tests, its provenance's tests and each witness with its steps,
    /// selections, drawn values and rewards (FR-069-AC-4).
    pub(crate) fn measured_bytes(&self) -> usize {
        let step = |step: &SampledStep| {
            size_of::<SampledStep>()
                .saturating_add(step.choices.len().saturating_mul(size_of::<u32>()))
                .saturating_add(
                    step.random
                        .len()
                        .saturating_add(step.rewards.len())
                        .saturating_mul(size_of::<quire_exact::Value>()),
                )
        };
        let witnesses = self.witnesses.iter().fold(0_usize, |bytes, witness| {
            let steps = witness
                .steps
                .iter()
                .fold(0_usize, |sum, each| sum.saturating_add(step(each)));
            bytes
                .saturating_add(size_of::<SampledWitness>())
                .saturating_add(steps)
        });
        self.tests
            .len()
            .saturating_mul(size_of::<TestResult>())
            .saturating_add(
                self.provenance
                    .tests
                    .len()
                    .saturating_mul(size_of::<TestProvenance>()),
            )
            .saturating_add(witnesses)
    }
}

/// Why a run stopped before its method completed (FR-191; ADR-024 SV-4).
/// A stopped run carries no decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatisticalStop {
    /// A test would exceed a budget: the budget and the value it was set
    /// to; [`StatisticalLimit::setting`] names the member that raises it.
    Limit {
        /// The budget.
        limit: StatisticalLimit,
        /// Its value in the run.
        value: u64,
    },
    /// A sample's expansion stopped (ADR-024 ST-10), for example on the
    /// clause meter.
    Expansion(ExpansionStop),
    /// The caller's `Cancel` handle was cancelled (FR-276).
    Cancelled(CancelCause),
    /// The bounded event's automaton would exceed FR-126's
    /// `max_automaton_states`, at the value it was set to: FR-126's
    /// `ResourceExhausted`. EN-4 has no member of its own for the budget, so
    /// the run reads FR-126's published default.
    AutomatonStates {
        /// The limit's value in the run.
        value: u64,
    },
}

/// Why a run is unsupported once sampling began (FR-189).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatisticalUnsupported {
    /// A drawn step broke the Markov condition (ADR-024 PM-3): the state,
    /// the scheduled identity, the drawn vector and its post-state count.
    NotMarkov(Box<NotMarkov>),
    /// A choice's scaled weight total exceeds `2^256`, so no draw is
    /// accepted (QSpec FR-181-AC-14).
    TotalAboveDrawRange {
        /// The scaled total.
        total: Integer,
    },
}

/// The outcome of a statistical run (FR-189 `StatisticalOutcome`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StatisticalOutcome {
    /// Every test completed its method, or a sample evaluated undefined.
    Completed(Box<StatisticalVerdict>),
    /// A budget, an expansion stop or a cancellation ended the run before
    /// its method completed.
    Stopped {
        /// Why.
        stop: StatisticalStop,
        /// The counts reached.
        partial: Box<StatisticalBasis>,
        /// The tests begun and the traces they used.
        provenance: StatisticalProvenance,
    },
    /// The run reached a step the method cannot sample.
    Unsupported {
        /// Why.
        cause: StatisticalUnsupported,
        /// The tests begun and the traces they used.
        provenance: StatisticalProvenance,
    },
}

/// Why EN-4 refuses a request with no outcome (FR-189 `StatisticalRefusal`).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum StatisticalRefusal {
    /// A claim predicate's read was refused with no value undefined: its
    /// truth at the state is not known (FR-196's `PredicateRefused`, which
    /// EN-5 returns for the same read).
    #[error("a claim predicate's read was refused: {0:?}")]
    PredicateRefused(ClauseOutcome),
    /// FR-120's admission of the subject's `ModelSystem` failed.
    #[error("the subject's model system did not admit: {0:?}")]
    Admission(Box<AdmissionFailure>),
    /// A symmetry declaration of the request is malformed (FR-151 "Form").
    #[error(transparent)]
    Symmetry(SymmetryRefusal),
    /// The claim's `over` type has an admitting population with no universe
    /// (FR-101), so the subject's tests are not finitely many.
    #[error("the subject has an unbounded population and requires a bound")]
    RequiresBound(RequiresBound),
    /// A claim `under every scheduler` has no sampling route (ADR-024 PM-8,
    /// ADR-028 XF-7); only exact evidence decides it.
    #[error("a claim under every scheduler is decided by exact evidence only")]
    EveryScheduler,
    /// A claim form only EN-5 decides: an unbounded reachability or an
    /// expected-reward form (ADR-028 XF-4, XF-5, XF-7).
    #[error("this claim form is decided by exact evidence only")]
    ExactOnlyForm,
    /// A claim that states no confidence parameters is decided by exact
    /// evidence only (ADR-024 PF-10).
    #[error("a claim with no confidence parameters is decided by exact evidence only")]
    MissingConfidence,
    /// The method does not decide the claim's form (FR-189, FR-190).
    #[error("{method:?} does not decide a {form:?} claim")]
    NotStatistical {
        /// The requested method.
        method: StatisticalMethod,
        /// The claim's form.
        form: ClaimForm,
    },
    /// The sampler is not `quire.simulation.sampler/v1` (FR-188).
    #[error("sampler {supplied:?} is not the quire.simulation.sampler/v1 generator")]
    GeneratorMismatch {
        /// The supplied sampler.
        supplied: DefinitionRef,
    },
    /// The request has no test to run: the subject has no (initial state,
    /// binding) pair.
    #[error("the request has no test to run")]
    NoTest,
    /// More tests than a test index (`u32`) counts.
    #[error("{count} tests exceed the test-index range")]
    TooManyTests {
        /// The number of tests requested.
        count: usize,
    },
    /// `min(α', β')` is so small that QSpec's `Φ` bounds cannot separate
    /// `1 − min(α', β')` from 1, so the regenerative method has no normal
    /// quantile bound.
    #[error("min(alpha', beta') is beyond the normal-quantile precision")]
    ConfidenceBeyondPrecision,
    /// An exact-arithmetic invariant broke.
    #[error("internal fault: {0:?}")]
    Internal(InternalFault),
    /// `max_draws · m` exceeds the trace-index range, so sample `i` of test
    /// `j` has no trace index `i · m + j`.
    #[error("max_draws {max_draws} times {tests} tests exceeds the trace-index range")]
    TraceIndexRange {
        /// The request's `max_draws`.
        max_draws: u64,
        /// The test count `m`.
        tests: u32,
    },
    /// The sampler reported a value the claim's form cannot take: a
    /// fraction for a Bernoulli test.
    #[error("the sampler returned a fraction for a Bernoulli test")]
    FractionForEvent,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ratio(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("ratio")
    }

    #[test]
    fn confidence_parameters_lie_in_the_open_unit_interval() {
        let hundredth = ratio(1, 100);
        assert!(Confidence::new(hundredth.clone(), hundredth.clone(), hundredth.clone()).is_ok());
        assert_eq!(
            Confidence::new(ratio(0, 1), hundredth.clone(), hundredth.clone()),
            Err(ConfidenceRefusal::Alpha)
        );
        assert_eq!(
            Confidence::new(hundredth.clone(), ratio(1, 1), hundredth.clone()),
            Err(ConfidenceRefusal::Beta)
        );
        assert_eq!(
            Confidence::new(hundredth.clone(), hundredth, ratio(3, 2)),
            Err(ConfidenceRefusal::Indifference)
        );
    }

    #[test]
    fn each_form_admits_exactly_its_methods() {
        let regenerative = StatisticalMethod::Regenerative { min_cycles: 1 };
        for (form, okamoto, sprt, regenerates) in [
            (ClaimForm::Probability, true, true, false),
            (ClaimForm::Quantile, true, true, false),
            (ClaimForm::MeanOfFraction, true, false, false),
            (ClaimForm::LongRunFraction, false, false, true),
        ] {
            assert_eq!(form.admits(StatisticalMethod::Okamoto), okamoto);
            assert_eq!(form.admits(StatisticalMethod::Sprt), sprt);
            assert_eq!(form.admits(regenerative), regenerates);
        }
        assert_eq!(regenerative.coverage(), Coverage::Asymptotic);
        assert_eq!(StatisticalMethod::Sprt.coverage(), Coverage::FiniteSample);
    }
}
