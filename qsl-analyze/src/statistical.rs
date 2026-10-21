// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-4, statistical model checking (FR-189 to FR-191; ADR-024 ST-1 to
//! ST-10, RP-1 to RP-3): decide a probabilistic claim from sampled
//! behaviours by Okamoto's fixed-sample bound, Wald's SPRT, or the
//! regenerative estimator for a long-run fraction, in exact rationals, with
//! Bonferroni over the tests, caller-set budgets and a provenance the run
//! reproduces from.
//!
//! The engine reads behaviours through [`BehaviourSampler`], the narrowest
//! seam FR-189's per-sample rules imply: for sample `i` of test `j` it asks
//! for the sample at trace index `i · m + j` and receives the evaluated
//! event, the fraction, or that the sample was not activated; for the
//! regenerative method it walks one behaviour from the test's initial state
//! and receives, per position, whether `P` holds, the position's weight and
//! whether the next position regenerates. [`check_sampled`] decides at that
//! seam; [`check_statistical`] is FR-189's entry point over a model subject
//! and a checked claim, whose sampler draws FR-188's behaviours and reads the
//! claim on each with the layer-5 evaluator (`model`). This module owns the
//! statistics, the budgets, the trace-index assignment and the outcome.
//!
//! [`require_statistical`] is the negotiation gate before any of it: it
//! refuses a checked claim only exact evidence decides, so no sample is drawn
//! for it.

mod bounds;
mod claim;
mod method;
mod model;
mod regenerative;
#[cfg(test)]
mod tests;

use qsl_eval::model_subject::ModelInstance;
use qsl_eval::simulation::model::{ClauseOutcome, NotMarkov};
use qsl_eval::simulation::{ExpansionStop, SampledStep};
use qsl_foundation::selection::DefinitionRef;
use qsl_foundation::InternalFault;
use qsl_replay::statistical::{
    ClaimForm, Decision, MeasureValue, SampledUndefined, SampledWitness, StatisticalBasis,
    StatisticalLimit, StatisticalLimits, StatisticalMethod, StatisticalOutcome,
    StatisticalProvenance, StatisticalRefusal, StatisticalStop, StatisticalUnsupported,
    StatisticalVerdict, TestProvenance, TestResult, TraceRange, UndecidedCause, WitnessValue,
};
use qsl_replay::UndefinedEvaluation;
use quire_exact::{Cancel, CancelCause, Integer, Rational};

pub use claim::{
    reduce_quantile, require_statistical, Bound, ClaimRefusal, Comparison, MeasureEvent,
    QuantileComparison, StatisticalClaim,
};
pub use model::{check_statistical, StatisticalRequest};

use bounds::{int, normal_quantile_upper, quotient, QuantileFailure};
use claim::Comparison as Cmp;
use method::{interval_decision, least, okamoto_sample_count, Sprt};
use regenerative::Cycles;

/// One statistical test's subject: an (initial state, `over` binding) pair
/// of the subject, in subject order, after any symmetry reduction (FR-189
/// "Tests").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestSubject {
    /// The initial state's index in the subject's ordered initial states.
    pub initial: u32,
    /// The `over` binding, when the claim has one.
    pub binding: Option<ModelInstance>,
}

/// A statistical request at the [`BehaviourSampler`] seam: FR-189's
/// [`StatisticalRequest`] with its subject read into tests and its checked
/// claim reduced to the [`StatisticalClaim`] the statistics decide.
#[derive(Clone, Copy, Debug)]
pub struct SampledRequest<'a> {
    /// The claim's form, bound and confidence.
    pub claim: &'a StatisticalClaim,
    /// The tests to run, `m` in all, in subject order.
    pub tests: &'a [TestSubject],
    /// The method.
    pub method: StatisticalMethod,
    /// The seed.
    pub seed: u64,
    /// The sampler identity, recorded in the provenance.
    pub sampler: &'a DefinitionRef,
    /// The budgets.
    pub limits: StatisticalLimits,
    /// The most sampled witnesses kept beside an undefined one (FR-193);
    /// [`DEFAULT_MAX_WITNESSES`] by default.
    pub max_witnesses: u32,
}

/// FR-189's published default for `max_witnesses`: 8.
pub const DEFAULT_MAX_WITNESSES: u32 = 8;

/// A fraction in `[0, 1]`: one sample's value for a mean of a fraction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitFraction(Rational);

/// A rational outside `[0, 1]` offered as a [`UnitFraction`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("a fraction lies in [0, 1]")]
pub struct NotAFraction;

impl TryFrom<Rational> for UnitFraction {
    type Error = NotAFraction;

    fn try_from(value: Rational) -> Result<Self, Self::Error> {
        if value >= int(0) && value <= int(1) {
            Ok(Self(value))
        } else {
            Err(NotAFraction)
        }
    }
}

/// A non-negative position weight: 1 unweighted, the reward of the step
/// that leaves the position under `weighted by R` (FR-190).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Weight(Rational);

/// A negative rational offered as a [`Weight`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("a position weight is not negative")]
pub struct NegativeWeight;

impl Weight {
    /// The unweighted position's weight, 1.
    pub fn unit() -> Self {
        Self(int(1))
    }
}

impl TryFrom<Rational> for Weight {
    type Error = NegativeWeight;

    fn try_from(value: Rational) -> Result<Self, Self::Error> {
        if value >= int(0) {
            Ok(Self(value))
        } else {
            Err(NegativeWeight)
        }
    }
}

/// One drawn sample, as the claim reads it (FR-189 "Per sample").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Sample {
    /// The event, on a sample.
    Event(bool),
    /// A quantile claim's activated sample: whether its reduced event
    /// ([`MeasureEvent::holds`]) holds, and the measure it holds of, which a
    /// witness keeps (FR-193).
    Measured {
        /// Whether the reduced event holds.
        holds: bool,
        /// The sample's measure.
        value: MeasureValue,
    },
    /// A mean-of-fraction claim's fraction.
    Fraction(UnitFraction),
    /// A quantile sample with no activation: drawn, not counted.
    NotActivated,
    /// The event, measure or comparison evaluated undefined.
    Undefined(UndefinedEvaluation),
    /// A contract conjunction evaluated undecided during a sampled step.
    UndecidedSuccessor,
}

/// One drawn sample and the steps it took (FR-188), which EN-4 keeps when
/// the sample is a witness (FR-193).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Drawn {
    /// The sample, as the claim reads it.
    pub sample: Sample,
    /// The sample's steps, one per position after the first.
    pub steps: Vec<SampledStep>,
}

/// One step of a regeneration walk (FR-190).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WalkStep {
    /// The step leaving the current position.
    Position {
        /// Whether `P` holds at the position.
        holds: bool,
        /// The position's weight.
        weight: Weight,
        /// Whether the next position's state key equals the initial
        /// state's, closing the cycle.
        returned: bool,
    },
    /// `P` evaluated undefined at the position.
    Undefined {
        /// Where, and why.
        evaluation: UndefinedEvaluation,
        /// The walk's steps from its initial state to the position.
        steps: Vec<SampledStep>,
    },
    /// A contract conjunction evaluated undecided during the step.
    UndecidedSuccessor,
}

/// Why a sample or a walk could not continue (FR-188 `SampleStop`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SampleHalt {
    /// A drawn step broke the Markov condition.
    NotMarkov(Box<NotMarkov>),
    /// A choice's scaled weight total exceeds `2^256`.
    TotalAboveDrawRange {
        /// The scaled total.
        total: Integer,
    },
    /// The expansion stopped.
    Expansion(ExpansionStop),
    /// The sampler is not `quire.simulation.sampler/v1`.
    GeneratorMismatch {
        /// The supplied sampler.
        supplied: DefinitionRef,
    },
    /// The sampler observed the run's `Cancel` handle cancelled.
    Cancelled(CancelCause),
    /// The bounded event's automaton would exceed its state limit, at the
    /// value it was set to (FR-126 `ResourceExhausted`).
    AutomatonStates {
        /// The limit's value.
        value: u64,
    },
    /// A claim predicate's read was refused with no value undefined.
    PredicateRefused(ClauseOutcome),
}

/// The behaviours EN-4 decides from: FR-188's sampler and the layer-5
/// evaluator over one subject, workload and claim.
pub trait BehaviourSampler {
    /// A regeneration walk.
    type Walk: RegenerationWalk;

    /// The sample at `trace` under `seed`, from `test`'s initial state,
    /// evaluated against the claim's event, fraction or activation, with
    /// the steps it took.
    ///
    /// # Errors
    ///
    /// A [`SampleHalt`] when the sample cannot be drawn to its end.
    fn sample(&mut self, test: &TestSubject, seed: u64, trace: u64) -> Result<Drawn, SampleHalt>;

    /// A walk of one behaviour at `trace` under `seed` from `test`'s
    /// initial state, for the regenerative method.
    fn walk(&mut self, test: &TestSubject, seed: u64, trace: u64) -> Self::Walk;
}

/// One behaviour sampled step by step (FR-190).
pub trait RegenerationWalk {
    /// Take the next step.
    ///
    /// # Errors
    ///
    /// A [`SampleHalt`] when the step cannot be drawn.
    fn step(&mut self) -> Result<WalkStep, SampleHalt>;
}

/// What a test decided, before its counts are attached.
struct Decided {
    decision: Decision,
    estimate: Option<Rational>,
    interval: Option<(Rational, Rational)>,
    undefined: Option<SampledUndefined>,
}

/// How one test ended.
enum TestEnd {
    /// The method completed, or a sample decided the test at once.
    Decided(Decided),
    /// A sample evaluated undefined: the test is rejected and the run
    /// stops (FR-189 "Per sample").
    Undefined(Decided),
    /// The run stops with no decision.
    Stopped(StatisticalStop),
    /// The run is unsupported.
    Unsupported(StatisticalUnsupported),
}

/// One test's running counts.
struct Tally {
    counted: u64,
    draws: u64,
    sum: Rational,
}

impl Tally {
    fn new() -> Self {
        Self {
            counted: 0,
            draws: 0,
            sum: int(0),
        }
    }

    fn estimate(&self) -> Option<Rational> {
        self.sum
            .div(&Rational::from_integer(Integer::from(self.counted)))
    }
}

/// The sampled witnesses of a run (FR-193): every undefined one, and the
/// `max` others first in trace-index order across tests.
struct Witnesses {
    max: usize,
    kept: Vec<SampledWitness>,
    undefined: Option<SampledWitness>,
}

fn witness(test: u32, trace: u64, steps: Vec<SampledStep>, value: WitnessValue) -> SampledWitness {
    let path_probability = steps
        .iter()
        .fold(int(1), |product, step| product.mul(&step.probability));
    SampledWitness {
        test,
        trace,
        steps,
        path_probability,
        value,
    }
}

impl Witnesses {
    fn new(max: u32) -> Self {
        Self {
            max: usize::try_from(max).unwrap_or(usize::MAX),
            kept: Vec::new(),
            undefined: None,
        }
    }

    /// Keep `witness` when it is among the first `max` in trace-index order.
    fn offer(&mut self, witness: SampledWitness) {
        let at = self.kept.partition_point(|kept| kept.trace < witness.trace);
        if at < self.max {
            self.kept.insert(at, witness);
            self.kept.truncate(self.max);
        }
    }

    /// The undefined witness first (QSpec FR-408-AC-11), then the others in
    /// trace-index order.
    fn into_vec(self) -> Vec<SampledWitness> {
        self.undefined.into_iter().chain(self.kept).collect()
    }
}

/// What a sample on the violating side of `bound` evaluated to (FR-193): an
/// event false against `>= θ` or true against `<= θ` (the reduced strict
/// bound of a quantile among them), or a fraction below or above `θ`.
fn violating(bound: &Bound, sample: &Sample) -> Option<WitnessValue> {
    match sample {
        Sample::Event(held) => {
            let violates = match bound.comparison {
                Cmp::AtLeast => !held,
                Cmp::AtMost | Cmp::Below => *held,
            };
            violates.then_some(WitnessValue::Event(*held))
        }
        Sample::Measured { holds, value } => {
            let violates = match bound.comparison {
                Cmp::AtLeast => !holds,
                Cmp::AtMost | Cmp::Below => *holds,
            };
            violates.then(|| WitnessValue::Measure(value.clone()))
        }
        Sample::Fraction(UnitFraction(value)) => {
            let violates = match bound.comparison {
                Cmp::AtLeast => *value < bound.threshold,
                Cmp::AtMost | Cmp::Below => *value > bound.threshold,
            };
            violates.then(|| WitnessValue::Fraction(value.clone()))
        }
        Sample::NotActivated | Sample::Undefined(_) | Sample::UndecidedSuccessor => None,
    }
}

/// The regenerative method's per-run constants.
struct Regeneration {
    /// The upper bound on `z_{1 − min(α', β')}`.
    z_upper: Rational,
    /// The fewest cycles a test closes before it decides.
    min_cycles: u64,
}

/// The per-test constants of one run (FR-189 "Tests").
struct Plan<'r> {
    request: SampledRequest<'r>,
    /// The run's cancellation handle, polled before each draw.
    cancel: &'r Cancel,
    tests: u32,
    alpha: Rational,
    beta: Rational,
}

/// Whether `cancel` has been cancelled, and why.
fn cancelled(cancel: &Cancel) -> Option<CancelCause> {
    if cancel.poll() {
        cancel.cause()
    } else {
        None
    }
}

/// The test's end for a sampler stop (FR-189 "Decision and stops"), or
/// the request's refusal for a generator mismatch.
fn halted(halt: SampleHalt) -> Result<TestEnd, StatisticalRefusal> {
    Ok(match halt {
        SampleHalt::NotMarkov(cause) => {
            TestEnd::Unsupported(StatisticalUnsupported::NotMarkov(cause))
        }
        SampleHalt::TotalAboveDrawRange { total } => {
            TestEnd::Unsupported(StatisticalUnsupported::TotalAboveDrawRange { total })
        }
        SampleHalt::Expansion(stop) => TestEnd::Stopped(StatisticalStop::Expansion(stop)),
        SampleHalt::Cancelled(cause) => TestEnd::Stopped(StatisticalStop::Cancelled(cause)),
        SampleHalt::AutomatonStates { value } => {
            TestEnd::Stopped(StatisticalStop::AutomatonStates { value })
        }
        SampleHalt::PredicateRefused(outcome) => {
            return Err(StatisticalRefusal::PredicateRefused(outcome))
        }
        SampleHalt::GeneratorMismatch { supplied } => {
            return Err(StatisticalRefusal::GeneratorMismatch { supplied })
        }
    })
}

fn fault(fault: InternalFault) -> StatisticalRefusal {
    StatisticalRefusal::Internal(fault)
}

/// The fault of a fixed-sample test that completed with no counted sample:
/// `N >= 1` always.
const EMPTY_FIXED_SAMPLE: InternalFault =
    InternalFault::new("statistical", "fixed-sample-count-positive");

/// A test decided at once: undecided by a successor, with no interval.
fn undecided_successor(estimate: Option<Rational>) -> TestEnd {
    TestEnd::Decided(Decided {
        decision: Decision::Undecided(UndecidedCause::UndecidedSuccessor),
        estimate,
        interval: None,
        undefined: None,
    })
}

/// A test rejected by an undefined evaluation at `trace` (ADR-024 SV-11).
fn undefined(
    index: u32,
    trace: u64,
    estimate: Option<Rational>,
    evaluation: UndefinedEvaluation,
) -> TestEnd {
    TestEnd::Undefined(Decided {
        decision: Decision::Rejected,
        estimate,
        interval: None,
        undefined: Some(SampledUndefined {
            test: index,
            trace,
            evaluation,
        }),
    })
}

impl Plan<'_> {
    fn limit_stop(&self, limit: StatisticalLimit) -> TestEnd {
        TestEnd::Stopped(StatisticalStop::Limit {
            limit,
            value: self.request.limits.value(limit),
        })
    }

    /// One Okamoto or SPRT test (FR-189 "Methods"): sample `i` at trace
    /// index `i · m + j`, until the method decides or a budget is reached.
    fn sampled_test<S: BehaviourSampler>(
        &self,
        index: u32,
        subject: &TestSubject,
        sampler: &mut S,
        tally: &mut Tally,
        witnesses: &mut Witnesses,
    ) -> Result<TestEnd, StatisticalRefusal> {
        let claim = self.request.claim;
        let iota = claim.confidence().indifference();
        let limits = self.request.limits;
        let mut sequential = match self.request.method {
            StatisticalMethod::Sprt => {
                Some(Sprt::new(claim.bound(), iota, &self.alpha, &self.beta).map_err(fault)?)
            }
            StatisticalMethod::Okamoto | StatisticalMethod::Regenerative { .. } => None,
        };
        // `None` when `N` exceeds every count a test can reach, so only a
        // budget ends the test.
        let fixed = match sequential {
            Some(_) => None,
            None => okamoto_sample_count(&self.alpha, &self.beta, iota)
                .map_err(fault)?
                .to_u64(),
        };
        let stride = u64::from(self.tests);
        loop {
            if fixed == Some(tally.counted) {
                let estimate = tally.estimate().ok_or(fault(EMPTY_FIXED_SAMPLE))?;
                let (decision, interval) = interval_decision(claim.bound(), &estimate, iota);
                return Ok(TestEnd::Decided(Decided {
                    decision,
                    estimate: Some(estimate),
                    interval: Some(interval),
                    undefined: None,
                }));
            }
            if tally.counted == limits.max_samples {
                return Ok(self.limit_stop(StatisticalLimit::MaxSamples));
            }
            if tally.draws == limits.max_draws {
                return Ok(self.limit_stop(StatisticalLimit::MaxDraws));
            }
            if let Some(cause) = cancelled(self.cancel) {
                return Ok(TestEnd::Stopped(StatisticalStop::Cancelled(cause)));
            }
            let trace = tally
                .draws
                .checked_mul(stride)
                .and_then(|base| base.checked_add(u64::from(index)))
                .ok_or(StatisticalRefusal::TraceIndexRange {
                    max_draws: limits.max_draws,
                    tests: self.tests,
                })?;
            let drawn = sampler.sample(subject, self.request.seed, trace);
            tally.draws += 1;
            let Drawn { sample, steps } = match drawn {
                Err(halt) => return halted(halt),
                Ok(drawn) => drawn,
            };
            if let Some(value) = violating(claim.bound(), &sample) {
                witnesses.offer(witness(index, trace, steps, value));
            } else if let Sample::Undefined(evaluation) = &sample {
                let value = WitnessValue::Undefined(evaluation.clone());
                witnesses.undefined = Some(witness(index, trace, steps, value));
            }
            match sample {
                Sample::NotActivated => {}
                Sample::Event(held) | Sample::Measured { holds: held, .. } => {
                    tally.counted += 1;
                    if held {
                        tally.sum = tally.sum.add(&int(1));
                    }
                    if let Some(test) = sequential.as_mut() {
                        test.count(held);
                        if let Some(decision) = test.decision() {
                            return Ok(TestEnd::Decided(Decided {
                                decision,
                                estimate: tally.estimate(),
                                interval: None,
                                undefined: None,
                            }));
                        }
                    }
                }
                Sample::Fraction(UnitFraction(value)) => {
                    // Only a mean of a fraction takes fractions; every other
                    // form's test is Bernoulli, under Okamoto as under SPRT.
                    if sequential.is_some() || claim.form() != ClaimForm::MeanOfFraction {
                        return Err(StatisticalRefusal::FractionForEvent);
                    }
                    tally.counted += 1;
                    tally.sum = tally.sum.add(&value);
                }
                Sample::Undefined(evaluation) => {
                    return Ok(undefined(index, trace, tally.estimate(), evaluation));
                }
                Sample::UndecidedSuccessor => return Ok(undecided_successor(tally.estimate())),
            }
        }
    }

    /// One regenerative test (FR-190): one behaviour at trace index `j`,
    /// cut at each return to the initial state.
    fn regenerative_test<S: BehaviourSampler>(
        &self,
        index: u32,
        subject: &TestSubject,
        sampler: &mut S,
        regeneration: &Regeneration,
        tally: &mut Tally,
        witnesses: &mut Witnesses,
    ) -> Result<TestEnd, StatisticalRefusal> {
        let claim = self.request.claim;
        let iota = claim.confidence().indifference();
        let limits = self.request.limits;
        let trace = u64::from(index);
        let mut walk = sampler.walk(subject, self.request.seed, trace);
        let mut cycles = Cycles::new();
        let mut cycle_steps: u64 = 0;
        let mut held = int(0);
        let mut weight = int(0);
        loop {
            tally.counted = cycles.count();
            if cycles.count() == limits.max_samples {
                return Ok(self.limit_stop(StatisticalLimit::MaxSamples));
            }
            if tally.draws == limits.max_draws {
                return Ok(self.limit_stop(StatisticalLimit::MaxDraws));
            }
            if cycle_steps == limits.max_cycle_steps {
                return Ok(self.limit_stop(StatisticalLimit::MaxCycleSteps));
            }
            if let Some(cause) = cancelled(self.cancel) {
                return Ok(TestEnd::Stopped(StatisticalStop::Cancelled(cause)));
            }
            let step = walk.step();
            tally.draws += 1;
            let (holds, Weight(step_weight), returned) = match step {
                Err(halt) => return halted(halt),
                // FR-193: a long-run fraction keeps no witness but an
                // undefined one.
                Ok(WalkStep::Undefined { evaluation, steps }) => {
                    let value = WitnessValue::Undefined(evaluation.clone());
                    witnesses.undefined = Some(witness(index, trace, steps, value));
                    return Ok(undefined(index, trace, cycles.estimate(), evaluation));
                }
                Ok(WalkStep::UndecidedSuccessor) => {
                    return Ok(undecided_successor(cycles.estimate()));
                }
                Ok(WalkStep::Position {
                    holds,
                    weight,
                    returned,
                }) => (holds, weight, returned),
            };
            cycle_steps += 1;
            if holds {
                held = held.add(&step_weight);
            }
            weight = weight.add(&step_weight);
            if !returned {
                continue;
            }
            cycles.close(&held, &weight);
            tally.counted = cycles.count();
            cycle_steps = 0;
            held = int(0);
            weight = int(0);
            if cycles.count() < regeneration.min_cycles {
                continue;
            }
            let Some(interval) = cycles.interval(&regeneration.z_upper).map_err(fault)? else {
                continue;
            };
            if interval.half_width <= *iota {
                let (decision, bounds) =
                    interval_decision(claim.bound(), &interval.estimate, &interval.half_width);
                return Ok(TestEnd::Decided(Decided {
                    decision,
                    estimate: Some(interval.estimate),
                    interval: Some(bounds),
                    undefined: None,
                }));
            }
        }
    }

    fn basis(&self, samples: u64, draws: u64) -> StatisticalBasis {
        StatisticalBasis {
            method: self.request.method,
            confidence: self.request.claim.confidence().clone(),
            samples,
            draws,
            tests: self.tests,
            alpha_per_test: self.alpha.clone(),
            beta_per_test: self.beta.clone(),
            coverage: self.request.method.coverage(),
        }
    }

    fn provenance(&self, tests: Vec<TestProvenance>) -> StatisticalProvenance {
        StatisticalProvenance {
            seed: self.request.seed,
            sampler: self.request.sampler.clone(),
            tests,
        }
    }
}

fn test_result(index: u32, subject: &TestSubject, decided: Decided, tally: &Tally) -> TestResult {
    TestResult {
        index,
        initial: subject.initial,
        binding: subject.binding.clone(),
        decision: decided.decision,
        estimate: decided.estimate,
        interval: decided.interval,
        samples: tally.counted,
        draws: tally.draws,
        undefined: decided.undefined,
    }
}

/// How unfavourable a decision is to the claim: a rejection most.
fn severity(decision: Decision) -> u8 {
    match decision {
        Decision::Rejected => 0,
        Decision::Undecided(_) => 1,
        Decision::Accepted => 2,
    }
}

/// The least favourable test (FR-189): the worst decision (Rejected, then
/// Undecided, then Accepted), then the smallest margin (`estimate − θ` for
/// `>= θ`, `θ − estimate` for `<= θ` and the strict `< θ`; no estimate is
/// the worst margin), then the lowest index.
fn least_favourable(tests: &[TestResult], comparison: Cmp) -> Option<&TestResult> {
    let worse = |candidate: &TestResult, current: &TestResult| -> bool {
        let (a, b) = (severity(candidate.decision), severity(current.decision));
        if a != b {
            return a < b;
        }
        match (&candidate.estimate, &current.estimate) {
            (None, Some(_)) => true,
            (Some(_) | None, None) => false,
            (Some(candidate), Some(current)) => match comparison {
                Cmp::AtLeast => candidate < current,
                Cmp::AtMost | Cmp::Below => candidate > current,
            },
        }
    };
    tests.iter().fold(None, |chosen, test| match chosen {
        Some(current) if !worse(test, current) => Some(current),
        _ => Some(test),
    })
}

/// The claim's decision from its tests' (FR-189 "Decision and stops"):
/// Rejected when any test rejects, Accepted when every test accepts,
/// otherwise the first undecided test's cause.
fn claim_decision(tests: &[TestResult]) -> Decision {
    if tests.iter().any(|test| test.decision == Decision::Rejected) {
        return Decision::Rejected;
    }
    tests
        .iter()
        .map(|test| test.decision)
        .find(|decision| matches!(decision, Decision::Undecided(_)))
        .unwrap_or(Decision::Accepted)
}

/// Decide `request`'s claim from `sampler`'s behaviours (FR-189 to
/// FR-191), polling `cancel` before each draw.
///
/// The outcome is a function of the claim, the tests, the method, the
/// seed, the sampler's behaviours and the limits (FR-191
/// "Reproduction").
///
/// # Errors
///
/// [`StatisticalRefusal::NotStatistical`] for a method the claim's form does
/// not admit, [`StatisticalRefusal::NoTest`] and
/// [`StatisticalRefusal::TooManyTests`] for a test list of no or too many
/// tests, [`StatisticalRefusal::TraceIndexRange`] when `max_draws · m`
/// overflows the trace index for Okamoto or SPRT, and
/// [`StatisticalRefusal::ConfidenceBeyondPrecision`] for a regenerative
/// request whose `min(α', β')` has no normal-quantile bound, each before any
/// sample is drawn; [`StatisticalRefusal::GeneratorMismatch`] when the
/// sampler refuses its generator; [`StatisticalRefusal::FractionForEvent`]
/// when the sampler returns a fraction for a Bernoulli test;
/// [`StatisticalRefusal::Internal`] when an exact-arithmetic invariant
/// breaks.
pub fn check_sampled<S: BehaviourSampler>(
    request: SampledRequest<'_>,
    sampler: &mut S,
    cancel: &Cancel,
) -> Result<StatisticalOutcome, StatisticalRefusal> {
    let form = request.claim.form();
    if !form.admits(request.method) {
        return Err(StatisticalRefusal::NotStatistical {
            method: request.method,
            form,
        });
    }
    if request.tests.is_empty() {
        return Err(StatisticalRefusal::NoTest);
    }
    let tests =
        u32::try_from(request.tests.len()).map_err(|_| StatisticalRefusal::TooManyTests {
            count: request.tests.len(),
        })?;
    // Only Okamoto and SPRT step through `i · m + j`; regenerative test `j`
    // walks one behaviour at trace index `j < m`.
    let interleaved = !matches!(request.method, StatisticalMethod::Regenerative { .. });
    if interleaved
        && request
            .limits
            .max_draws
            .checked_mul(u64::from(tests))
            .is_none()
    {
        return Err(StatisticalRefusal::TraceIndexRange {
            max_draws: request.limits.max_draws,
            tests,
        });
    }
    let m = int(i64::from(tests));
    let confidence = request.claim.confidence();
    let plan = Plan {
        request,
        cancel,
        tests,
        alpha: quotient(confidence.alpha(), &m).map_err(fault)?,
        beta: quotient(confidence.beta(), &m).map_err(fault)?,
    };
    let regenerative = match request.method {
        StatisticalMethod::Regenerative { min_cycles } => {
            let z = normal_quantile_upper(least(&plan.alpha, &plan.beta)).map_err(|failure| {
                match failure {
                    QuantileFailure::BeyondPrecision => {
                        StatisticalRefusal::ConfidenceBeyondPrecision
                    }
                    QuantileFailure::Fault(inner) => fault(inner),
                }
            })?;
            Some(Regeneration {
                z_upper: z,
                min_cycles,
            })
        }
        StatisticalMethod::Okamoto | StatisticalMethod::Sprt => None,
    };

    let mut results = Vec::with_capacity(request.tests.len());
    let mut provenance = Vec::with_capacity(request.tests.len());
    let (mut samples, mut draws) = (0_u64, 0_u64);
    let mut witnesses = Witnesses::new(request.max_witnesses);
    for (index, subject) in (0_u32..).zip(request.tests) {
        let mut tally = Tally::new();
        let end = match &regenerative {
            Some(regeneration) => plan.regenerative_test(
                index,
                subject,
                sampler,
                regeneration,
                &mut tally,
                &mut witnesses,
            )?,
            None => plan.sampled_test(index, subject, sampler, &mut tally, &mut witnesses)?,
        };
        let used = match regenerative {
            Some(_) => u64::from(tally.draws > 0),
            None => tally.draws,
        };
        provenance.push(TestProvenance {
            index,
            initial: subject.initial,
            binding: subject.binding.clone(),
            traces: TraceRange {
                first: u64::from(index),
                step: u64::from(tests),
                count: used,
            },
        });
        // Each test's counts are bounded by `u64` budgets; their sum over
        // `m` tests saturates rather than wraps.
        samples = samples.saturating_add(tally.counted);
        draws = draws.saturating_add(tally.draws);
        match end {
            TestEnd::Decided(decided) => results.push(test_result(index, subject, decided, &tally)),
            TestEnd::Undefined(decided) => {
                results.push(test_result(index, subject, decided, &tally));
                break;
            }
            TestEnd::Stopped(stop) => {
                return Ok(StatisticalOutcome::Stopped {
                    stop,
                    partial: Box::new(plan.basis(samples, draws)),
                    provenance: plan.provenance(provenance),
                })
            }
            TestEnd::Unsupported(cause) => {
                return Ok(StatisticalOutcome::Unsupported {
                    cause,
                    provenance: plan.provenance(provenance),
                })
            }
        }
    }
    let decision = claim_decision(&results);
    let chosen = least_favourable(&results, request.claim.bound().comparison);
    Ok(StatisticalOutcome::Completed(Box::new(
        StatisticalVerdict {
            decision,
            estimate: chosen.and_then(|test| test.estimate.clone()),
            interval: chosen.and_then(|test| test.interval.clone()),
            basis: plan.basis(samples, draws),
            provenance: plan.provenance(provenance),
            tests: results,
            witnesses: witnesses.into_vec(),
        },
    )))
}
