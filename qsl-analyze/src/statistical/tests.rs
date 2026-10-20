// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-4 over behaviour samplers that draw ADR-024 §7's distributions with
//! FR-188's weighted choice: `Service`'s latency (§7.2), `Avail`'s up and
//! down chain (§7.3), and the `Coin` and `Wide` draws of FR-189-AC-6 and
//! AC-7. Each sampler draws exactly the values its model's sampler draws
//! at each (seed, trace, step, choice), so these tests exercise the
//! engine's statistics, budgets, trace indices and provenance on the
//! spec's own distributions.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use ix_trace_rs::trace;
use qsl_eval::simulation::{weighted_choice, ChoicePoint, Weights, WeightsRefusal};
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::digest::DigestRecord;
use qsl_foundation::selection::DefinitionRef;
use qsl_replay::statistical::{
    ClaimForm, Confidence, Coverage, Decision, StatisticalLimit, StatisticalLimits,
    StatisticalMethod, StatisticalOutcome, StatisticalRefusal, StatisticalStop,
    StatisticalUnsupported, StatisticalVerdict, UndecidedCause,
};
use qsl_replay::{UndefinedEvaluation, UndefinedLocus, UndefinedSite};
use quire_exact::{Cancel, CancelCause, Integer, Rational};

use super::bounds::{int, normal_quantile_upper, quotient, sqrt};
use super::*;

fn ratio(numerator: i64, denominator: i64) -> Rational {
    quotient(&int(numerator), &int(denominator)).expect("nonzero denominator")
}

fn weights(values: &[i64]) -> Weights {
    Weights::new(&values.iter().map(|value| int(*value)).collect::<Vec<_>>()).expect("weights")
}

fn draw(seed: u64, trace: u64, step: u64, choice: u32, weights: &Weights) -> usize {
    weighted_choice(
        ChoicePoint {
            seed,
            trace,
            step,
            choice,
        },
        weights,
    )
}

fn sampler_ref() -> DefinitionRef {
    DefinitionRef::new("agent-ix", "quire.simulation.sampler/v1").expect("sampler reference")
}

fn confidence(iota: Rational) -> Confidence {
    Confidence::new(ratio(1, 100), ratio(1, 100), iota).expect("confidence")
}

fn claim(
    form: ClaimForm,
    comparison: Comparison,
    threshold: Rational,
    iota: Rational,
) -> StatisticalClaim {
    StatisticalClaim::new(
        form,
        Bound {
            comparison,
            threshold,
        },
        confidence(iota),
    )
    .expect("claim")
}

/// ADR-024 §7.2's `P95` at `c` milliseconds: `quantile 0.95 of M <= c`,
/// reduced to `Pr(M <= c) >= 0.95` at `ι = 1/100`.
fn p95() -> StatisticalClaim {
    let (_, bound) = reduce_quantile(ratio(95, 100), QuantileComparison::AtMost, int(5));
    StatisticalClaim::new(ClaimForm::Quantile, bound, confidence(ratio(1, 100))).expect("P95")
}

fn one_test() -> Vec<TestSubject> {
    vec![TestSubject {
        initial: 0,
        binding: None,
    }]
}

fn request<'a>(
    claim: &'a StatisticalClaim,
    tests: &'a [TestSubject],
    method: StatisticalMethod,
    sampler: &'a DefinitionRef,
) -> StatisticalRequest<'a> {
    StatisticalRequest {
        claim,
        tests,
        method,
        seed: 7,
        sampler,
        limits: StatisticalLimits::default(),
    }
}

fn run<S: BehaviourSampler>(
    request: StatisticalRequest<'_>,
    sampler: &mut S,
) -> StatisticalOutcome {
    check_statistical(request, sampler, &Cancel::new()).expect("a statistical outcome")
}

fn completed(outcome: StatisticalOutcome) -> StatisticalVerdict {
    match outcome {
        StatisticalOutcome::Completed(verdict) => *verdict,
        other => panic!("expected a completed run, got {other:?}"),
    }
}

/// No walk: the Bernoulli samplers never run the regenerative method.
struct NoWalk;

impl RegenerationWalk for NoWalk {
    fn step(&mut self) -> Result<WalkStep, SampleHalt> {
        Err(SampleHalt::NotMarkov)
    }
}

/// ADR-024 §7.2's `Service` latency: per attempt `d ~ {1: 90, 3: 7, 12: 3}`
/// ms at choice 0 and `outcome ~ {Ok: 98, Fail: 2}` at choice 1; a failed
/// first attempt is followed by a second, and `M` is the summed `d`. With
/// `activation`, `request` first sets `phase = Busy` with probability 1/2
/// (FR-189-AC-4's variant), otherwise the sample is not activated.
struct Service {
    threshold: i64,
    activation: bool,
    durations: Weights,
    outcomes: Weights,
    coin: Weights,
    /// Each drawn sample's (initial state, trace index) in draw order.
    drawn: Vec<(u32, u64)>,
    /// Each activated sample's `M`, by trace index.
    latencies: BTreeMap<u64, i64>,
}

impl Service {
    fn new(threshold: i64) -> Self {
        Self {
            threshold,
            activation: false,
            durations: weights(&[90, 7, 3]),
            outcomes: weights(&[98, 2]),
            coin: weights(&[1, 1]),
            drawn: Vec::new(),
            latencies: BTreeMap::new(),
        }
    }

    fn activated(threshold: i64) -> Self {
        Self {
            activation: true,
            ..Self::new(threshold)
        }
    }
}

const MILLISECONDS: [i64; 3] = [1, 3, 12];

impl BehaviourSampler for Service {
    type Walk = NoWalk;

    fn sample(&mut self, test: &TestSubject, seed: u64, trace: u64) -> Result<Sample, SampleHalt> {
        self.drawn.push((test.initial, trace));
        let mut step = 0;
        if self.activation {
            if draw(seed, trace, step, 0, &self.coin) == 1 {
                return Ok(Sample::NotActivated);
            }
            step += 1;
        }
        let mut latency = 0;
        for _attempt in 0..2 {
            latency += MILLISECONDS[draw(seed, trace, step, 0, &self.durations)];
            let failed = draw(seed, trace, step, 1, &self.outcomes) == 1;
            step += 1;
            if !failed {
                break;
            }
        }
        self.latencies.insert(trace, latency);
        let (event, _) = reduce_quantile(
            ratio(95, 100),
            QuantileComparison::AtMost,
            int(self.threshold),
        );
        Ok(Sample::Event(
            event.holds(&MeasureValue::Value(int(latency))),
        ))
    }

    fn walk(&mut self, _test: &TestSubject, _seed: u64, _trace: u64) -> NoWalk {
        NoWalk
    }
}

/// FR-189-AC-1: `P95` with Okamoto accepts at 5 ms over `N = 23,026`
/// samples with an interval of half-width 1/100 whose lower end is at least
/// 19/20, and rejects at 2 ms with an upper end below 19/20; SPRT accepts
/// at 5 ms and rejects at 2 ms, each below 23,026 samples.
#[trace("TC-624", "FR-189-AC-1")]
#[test]
fn p95_settles_by_okamoto_and_by_sprt() {
    let tests = one_test();
    let sampler = sampler_ref();
    let at = |milliseconds: i64| {
        let (_, bound) = reduce_quantile(
            ratio(95, 100),
            QuantileComparison::AtMost,
            int(milliseconds),
        );
        StatisticalClaim::new(ClaimForm::Quantile, bound, confidence(ratio(1, 100))).expect("P95")
    };
    let five = at(5);
    let two = at(2);
    let nineteen_twentieths = ratio(19, 20);

    let accepted = completed(run(
        request(&five, &tests, StatisticalMethod::Okamoto, &sampler),
        &mut Service::new(5),
    ));
    assert_eq!(accepted.decision, Decision::Accepted);
    assert_eq!(accepted.basis.samples, 23_026);
    assert_eq!(accepted.basis.coverage, Coverage::FiniteSample);
    let (lower, upper) = accepted.interval.clone().expect("an Okamoto interval");
    assert_eq!(upper.sub(&lower), ratio(2, 100));
    assert!(lower >= nineteen_twentieths);

    let rejected = completed(run(
        request(&two, &tests, StatisticalMethod::Okamoto, &sampler),
        &mut Service::new(2),
    ));
    assert_eq!(rejected.decision, Decision::Rejected);
    let (_, upper) = rejected.interval.expect("an Okamoto interval");
    assert!(upper < nineteen_twentieths);

    for (claim, threshold, decision) in [
        (&five, 5, Decision::Accepted),
        (&two, 2, Decision::Rejected),
    ] {
        let verdict = completed(run(
            request(claim, &tests, StatisticalMethod::Sprt, &sampler),
            &mut Service::new(threshold),
        ));
        assert_eq!(verdict.decision, decision);
        assert!(verdict.basis.samples < 23_026);
        assert_eq!(verdict.interval, None);
    }
}

/// FR-189-AC-2: two runs of one request give equal outcomes.
#[trace("TC-624", "FR-189-AC-2")]
#[test]
fn one_request_twice_gives_equal_outcomes() {
    let claim = p95();
    let tests = one_test();
    let sampler = sampler_ref();
    let first = run(
        request(&claim, &tests, StatisticalMethod::Okamoto, &sampler),
        &mut Service::new(5),
    );
    let second = run(
        request(&claim, &tests, StatisticalMethod::Okamoto, &sampler),
        &mut Service::new(5),
    );
    assert_eq!(first, second);
}

/// FR-189-AC-3: two initial states run `m = 2` tests at `α' = β' = 1/200`,
/// sample `i` of test `j` at trace index `2i + j`.
#[trace("TC-624", "FR-189-AC-3")]
#[test]
fn two_initial_states_run_two_bonferroni_tests_at_interleaved_traces() {
    let claim = p95();
    let tests = vec![
        TestSubject {
            initial: 0,
            binding: None,
        },
        TestSubject {
            initial: 1,
            binding: None,
        },
    ];
    let sampler = sampler_ref();
    let mut service = Service::new(5);
    let verdict = completed(run(
        request(&claim, &tests, StatisticalMethod::Sprt, &sampler),
        &mut service,
    ));
    assert_eq!(verdict.basis.tests, 2);
    assert_eq!(verdict.basis.alpha_per_test, ratio(1, 200));
    assert_eq!(verdict.basis.beta_per_test, ratio(1, 200));
    for (initial, j) in [(0_u32, 0_u64), (1, 1)] {
        let traces: Vec<u64> = service
            .drawn
            .iter()
            .filter(|(state, _)| *state == initial)
            .map(|(_, trace)| *trace)
            .collect();
        assert!(!traces.is_empty());
        for (i, trace) in (0_u64..).zip(traces) {
            assert_eq!(trace, 2 * i + j);
        }
    }
}

/// FR-189-AC-4: activation counts only activated samples; a censored
/// sample exceeds the threshold; `quantile 0.5 of M >= 3` reduces to the
/// strict `Pr(M < 3) < 1/2`, which `Pr(M < 3) = 1/2` does not meet.
#[trace("TC-624", "FR-189-AC-4")]
#[test]
fn a_quantile_claim_counts_only_activated_samples() {
    let claim = p95();
    let tests = one_test();
    let sampler = sampler_ref();
    let verdict = completed(run(
        request(&claim, &tests, StatisticalMethod::Okamoto, &sampler),
        &mut Service::activated(5),
    ));
    assert_eq!(verdict.tests[0].samples, 23_026);
    assert!(verdict.tests[0].draws > verdict.tests[0].samples);

    let (event, _) = reduce_quantile(ratio(95, 100), QuantileComparison::AtMost, int(5));
    assert!(!event.holds(&MeasureValue::Censored));

    let (event, bound) = reduce_quantile(ratio(1, 2), QuantileComparison::AtLeast, int(3));
    assert_eq!(
        event,
        MeasureEvent {
            strict: true,
            threshold: int(3)
        }
    );
    assert!(event.holds(&MeasureValue::Value(int(1))));
    assert!(!event.holds(&MeasureValue::Value(int(3))));
    assert_eq!(bound.comparison, Comparison::Below);
    assert!(
        !bound.holds(&ratio(1, 2)),
        "Pr(M < 3) = 1/2 meets the strict bound"
    );
}

/// A sampler that reports one fixed stop or sample for every draw.
struct Fixed {
    sample: Result<Sample, SampleHalt>,
    calls: u64,
}

impl BehaviourSampler for Fixed {
    type Walk = NoWalk;

    fn sample(
        &mut self,
        _test: &TestSubject,
        _seed: u64,
        _trace: u64,
    ) -> Result<Sample, SampleHalt> {
        self.calls += 1;
        self.sample.clone()
    }

    fn walk(&mut self, _test: &TestSubject, _seed: u64, _trace: u64) -> NoWalk {
        NoWalk
    }
}

fn fixed(sample: Result<Sample, SampleHalt>) -> Fixed {
    Fixed { sample, calls: 0 }
}

/// ADR-024 §7.1's `NoFault`: `probability >= 0.999` at `ι = 1/2000`.
fn no_fault() -> StatisticalClaim {
    claim(
        ClaimForm::Probability,
        Comparison::AtLeast,
        ratio(999, 1000),
        ratio(1, 2000),
    )
}

/// FR-189-AC-5: a non-unique post-state is `Unsupported(NotMarkov)`; an
/// undecided contract conjunction ends its test Undecided,
/// `UndecidedSuccessor`; a `mean of` claim with SPRT is refused
/// `NotStatistical`.
#[trace("TC-624", "FR-189-AC-5")]
#[test]
fn not_markov_undecided_successor_and_sequential_mean_settle_as_stated() {
    let claim = no_fault();
    let tests = one_test();
    let sampler = sampler_ref();
    let outcome = run(
        request(&claim, &tests, StatisticalMethod::Okamoto, &sampler),
        &mut fixed(Err(SampleHalt::NotMarkov)),
    );
    let StatisticalOutcome::Unsupported { cause, provenance } = outcome else {
        panic!("expected Unsupported, got {outcome:?}");
    };
    assert_eq!(cause, StatisticalUnsupported::NotMarkov);
    assert_eq!(provenance.tests[0].traces.count, 1);

    let verdict = completed(run(
        request(&claim, &tests, StatisticalMethod::Okamoto, &sampler),
        &mut fixed(Ok(Sample::UndecidedSuccessor)),
    ));
    let undecided = Decision::Undecided(UndecidedCause::UndecidedSuccessor);
    assert_eq!(verdict.tests[0].decision, undecided);
    assert_eq!(verdict.decision, undecided);

    let mean = claim_of_mean();
    assert_eq!(
        check_statistical(
            request(&mean, &tests, StatisticalMethod::Sprt, &sampler),
            &mut fixed(Ok(Sample::Event(true))),
            &Cancel::new(),
        ),
        Err(StatisticalRefusal::NotStatistical {
            method: StatisticalMethod::Sprt,
            form: ClaimForm::MeanOfFraction,
        })
    );
}

fn claim_of_mean() -> StatisticalClaim {
    claim(
        ClaimForm::MeanOfFraction,
        Comparison::AtLeast,
        ratio(1, 2),
        ratio(1, 10),
    )
}

/// FR-189 "Methods": a mean of a fraction is decided by the fixed-sample
/// method over the sample mean.
#[trace("TC-624", "FR-189-AC-5")]
#[test]
fn okamoto_decides_a_mean_of_a_fraction_over_the_sample_mean() {
    let claim = claim_of_mean();
    let tests = one_test();
    let sampler = sampler_ref();
    let three_quarters = UnitFraction::try_from(ratio(3, 4)).expect("a fraction");
    let verdict = completed(run(
        request(&claim, &tests, StatisticalMethod::Okamoto, &sampler),
        &mut fixed(Ok(Sample::Fraction(three_quarters))),
    ));
    assert_eq!(verdict.decision, Decision::Accepted);
    assert_eq!(verdict.estimate, Some(ratio(3, 4)));
    assert_eq!(UnitFraction::try_from(ratio(5, 4)), Err(NotAFraction));
}

/// FR-189-AC-6's `Coin`: `v` starts 0 and each `flip` draws `b ~ {0: 1,
/// 1: 1}` at choice 0; `1 / (1 − c.v)` is undefined at the first position
/// with `v = 1`.
struct Coin {
    bit: Weights,
    calls: u64,
}

fn coin_position(seed: u64, trace: u64, bit: &Weights) -> Option<u64> {
    (0_u64..3)
        .find(|step| draw(seed, trace, *step, 0, bit) == 1)
        .map(|step| step + 1)
}

impl BehaviourSampler for Coin {
    type Walk = NoWalk;

    fn sample(&mut self, _test: &TestSubject, seed: u64, trace: u64) -> Result<Sample, SampleHalt> {
        self.calls += 1;
        Ok(match coin_position(seed, trace, &self.bit) {
            Some(position) => Sample::Undefined(UndefinedEvaluation {
                site: UndefinedSite {
                    position,
                    state_key: DigestRecord::raw_artifact(b"v = 1"),
                    locus: UndefinedLocus::Contract("holds(1 / (1 - c.v) = 1)".to_owned()),
                    transition: None,
                },
                cause: UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero),
            }),
            None => Sample::Event(true),
        })
    }

    fn walk(&mut self, _test: &TestSubject, _seed: u64, _trace: u64) -> NoWalk {
        NoWalk
    }
}

/// FR-189-AC-6: the test is rejected at the first sample in trace-index
/// order that reaches `v = 1`, no further sample is drawn, and the result
/// names test 0, that trace index and the first position with `v = 1`,
/// cause division by zero; the claim settles `Completed` Rejected.
#[trace("TC-640", "FR-189-AC-6")]
#[test]
fn a_sampled_undefined_evaluation_rejects_at_once() {
    let claim = claim(
        ClaimForm::Probability,
        Comparison::AtLeast,
        ratio(1, 2),
        ratio(1, 10),
    );
    let tests = one_test();
    let sampler = sampler_ref();
    let bit = weights(&[1, 1]);
    let (first, position) = (0_u64..)
        .find_map(|trace| coin_position(7, trace, &bit).map(|position| (trace, position)))
        .expect("some trace reaches v = 1");
    let mut coin = Coin { bit, calls: 0 };
    let verdict = completed(run(
        request(&claim, &tests, StatisticalMethod::Okamoto, &sampler),
        &mut coin,
    ));
    assert_eq!(verdict.decision, Decision::Rejected);
    assert_eq!(
        coin.calls,
        first + 1,
        "a sample was drawn after the undefined one"
    );
    let undefined = verdict.tests[0]
        .undefined
        .clone()
        .expect("the undefined evaluation");
    assert_eq!(undefined.test, 0);
    assert_eq!(undefined.trace, first);
    assert_eq!(undefined.evaluation.site.position, position);
    assert_eq!(
        undefined.evaluation.cause,
        UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero)
    );
}

/// FR-189-AC-7's `Wide`: `spin(x) ~ {true: 1, false: 2^256}`.
struct Wide {
    calls: u64,
}

impl BehaviourSampler for Wide {
    type Walk = NoWalk;

    fn sample(
        &mut self,
        _test: &TestSubject,
        _seed: u64,
        _trace: u64,
    ) -> Result<Sample, SampleHalt> {
        self.calls += 1;
        let two_256 = Rational::from_integer(Integer::one().shifted_left(256));
        let spin = [int(1), two_256];
        match Weights::new(&spin) {
            Err(WeightsRefusal::TotalAboveDrawRange) => Err(SampleHalt::TotalAboveDrawRange {
                total: spin
                    .iter()
                    .fold(int(0), |sum, weight| sum.add(weight))
                    .numerator()
                    .clone(),
            }),
            other => panic!("expected the draw range refusal, got {other:?}"),
        }
    }

    fn walk(&mut self, _test: &TestSubject, _seed: u64, _trace: u64) -> NoWalk {
        NoWalk
    }
}

/// FR-189-AC-7: the run stops at the first sample and returns
/// `Unsupported(TotalAboveDrawRange{total})` with `total = 2^256 + 1`.
#[trace("TC-624", "FR-189-AC-7")]
#[test]
fn a_weight_total_above_the_draw_range_is_unsupported_at_the_first_sample() {
    let claim = claim(
        ClaimForm::Probability,
        Comparison::AtLeast,
        ratio(1, 2),
        ratio(1, 10),
    );
    let tests = one_test();
    let sampler = sampler_ref();
    let mut wide = Wide { calls: 0 };
    let outcome = run(
        request(&claim, &tests, StatisticalMethod::Okamoto, &sampler),
        &mut wide,
    );
    let StatisticalOutcome::Unsupported {
        cause: StatisticalUnsupported::TotalAboveDrawRange { total },
        ..
    } = outcome
    else {
        panic!("expected TotalAboveDrawRange, got {outcome:?}");
    };
    assert_eq!(wide.calls, 1);
    assert_eq!(
        total.to_string(),
        "115792089237316195423570985008687907853269984665640564039457584007913129639937"
    );
}

/// Each closed cycle's `(Y, L)`, as a walk recorded it.
type CycleLog = Rc<RefCell<Vec<(Rational, Rational)>>>;

/// ADR-024 §7.3's `Avail`: at `up`, `tick` draws `fail ~ {true: 1, false:
/// 1999}`; at `down`, `repair` draws `ok ~ {true: 9, false: 1}`; each at
/// choice 0, the one random parameter of the one enabled operation. With
/// `transient`, the walk starts at a start state no step re-enters; with
/// `duration`, an up position weighs 1 (`tick`) and a down position 3
/// (`repair`).
#[derive(Default)]
struct Avail {
    transient: bool,
    duration: bool,
    cycles: CycleLog,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Node {
    Start,
    Up,
    Down,
}

struct AvailWalk {
    seed: u64,
    trace: u64,
    step: u64,
    node: Node,
    initial: Node,
    duration: bool,
    fail: Weights,
    repair: Weights,
    open: (Rational, Rational),
    cycles: CycleLog,
}

impl RegenerationWalk for AvailWalk {
    fn step(&mut self) -> Result<WalkStep, SampleHalt> {
        let (holds, weight, next) = match self.node {
            Node::Start => (false, 1, Node::Up),
            Node::Up => {
                let failed = draw(self.seed, self.trace, self.step, 0, &self.fail) == 0;
                (true, 1, if failed { Node::Down } else { Node::Up })
            }
            Node::Down => {
                let repaired = draw(self.seed, self.trace, self.step, 0, &self.repair) == 0;
                let weight = if self.duration { 3 } else { 1 };
                (false, weight, if repaired { Node::Up } else { Node::Down })
            }
        };
        self.step += 1;
        self.node = next;
        let weight = int(weight);
        if holds {
            self.open.0 = self.open.0.add(&weight);
        }
        self.open.1 = self.open.1.add(&weight);
        let returned = next == self.initial;
        if returned {
            let cycle = std::mem::replace(&mut self.open, (int(0), int(0)));
            self.cycles.borrow_mut().push(cycle);
        }
        Ok(WalkStep::Position {
            holds,
            weight: Weight::try_from(weight).expect("a non-negative weight"),
            returned,
        })
    }
}

impl BehaviourSampler for Avail {
    type Walk = AvailWalk;

    fn sample(
        &mut self,
        _test: &TestSubject,
        _seed: u64,
        _trace: u64,
    ) -> Result<Sample, SampleHalt> {
        Err(SampleHalt::NotMarkov)
    }

    fn walk(&mut self, _test: &TestSubject, seed: u64, trace: u64) -> AvailWalk {
        let initial = if self.transient {
            Node::Start
        } else {
            Node::Up
        };
        AvailWalk {
            seed,
            trace,
            step: 0,
            node: initial,
            initial,
            duration: self.duration,
            fail: weights(&[1, 1999]),
            repair: weights(&[9, 1]),
            open: (int(0), int(0)),
            cycles: Rc::clone(&self.cycles),
        }
    }
}

/// ADR-024 §7.3's `LongRun`: `long-run fraction holds(v.up) >= 0.999` at
/// `ι = 1/5000`, or `<= 0.9995` with the same confidence.
fn long_run(comparison: Comparison, threshold: Rational) -> StatisticalClaim {
    claim(
        ClaimForm::LongRunFraction,
        comparison,
        threshold,
        ratio(1, 5000),
    )
}

/// The half-width after each recorded cycle, recomputed from the cycles
/// by `z · √(S² + 1/n) / (L̄ · √n)` with the same outward bounds, `S²` from
/// running sums of `Y`, `L`, `Y²`, `Y·L` and `L²`; `None` before an estimate
/// exists or below two cycles.
fn recomputed_half_widths(cycles: &[(Rational, Rational)], z: &Rational) -> Vec<Option<Rational>> {
    let zero = || int(0);
    let (mut y, mut l, mut yy, mut yl, mut ll) = (zero(), zero(), zero(), zero(), zero());
    let mut widths = Vec::with_capacity(cycles.len());
    for (count, (yk, lk)) in (1_i64..).zip(cycles) {
        y = y.add(yk);
        l = l.add(lk);
        yy = yy.add(&yk.mul(yk));
        yl = yl.add(&yk.mul(lk));
        ll = ll.add(&lk.mul(lk));
        let n = int(count);
        let width = (|| {
            let estimate = y.div(&l)?;
            let squares = yy
                .sub(&int(2).mul(&estimate).mul(&yl))
                .add(&estimate.mul(&estimate).mul(&ll));
            let variance = squares.div(&n.sub(&int(1)))?;
            let root = sqrt(&variance.add(&int(1).div(&n)?)).ok()?.upper;
            let mean = l.div(&n)?;
            z.mul(&root).div(&mean.mul(&sqrt(&n).ok()?.lower))
        })();
        widths.push(width);
    }
    widths
}

/// FR-190-AC-1: `LongRun` with `Regenerative{min_cycles: 100}` is accepted
/// with an estimate within 1/5000 of 1800/1801, a half-width at most 1/5000
/// and a lower end at least 999/1000, after the first `n >= 100` cycles at
/// which the half-width recomputed from the recorded cycles is at most
/// 1/5000, with asymptotic coverage; `<= 0.9995` is Undecided; two runs are
/// equal.
#[trace("TC-625", "FR-190-AC-1")]
#[test]
fn long_run_is_accepted_by_regeneration_with_asymptotic_coverage() {
    let claim = long_run(Comparison::AtLeast, ratio(999, 1000));
    let tests = one_test();
    let sampler = sampler_ref();
    let method = StatisticalMethod::Regenerative { min_cycles: 100 };
    let mut avail = Avail::default();
    let outcome = run(request(&claim, &tests, method, &sampler), &mut avail);
    let verdict = completed(outcome.clone());
    assert_eq!(verdict.decision, Decision::Accepted);
    assert_eq!(verdict.basis.coverage, Coverage::Asymptotic);
    assert_eq!(verdict.basis.method, method);
    let estimate = verdict.estimate.clone().expect("an estimate");
    let exact = ratio(1800, 1801);
    let iota = ratio(1, 5000);
    let distance = if estimate > exact {
        estimate.sub(&exact)
    } else {
        exact.sub(&estimate)
    };
    assert!(distance <= iota);
    let (lower, upper) = verdict.interval.clone().expect("an interval");
    assert!(upper.sub(&lower) <= ratio(2, 5000));
    assert!(lower >= ratio(999, 1000));

    let cycles = avail.cycles.borrow().clone();
    let n = usize::try_from(verdict.basis.samples).expect("a cycle count");
    assert_eq!(cycles.len(), n);
    assert!(n > 100, "the run stopped at min_cycles");
    let z = normal_quantile_upper(&ratio(1, 100)).expect("z");
    let widths = recomputed_half_widths(&cycles, &z);
    let first = (100..=n)
        .find(|count| {
            widths[count - 1]
                .as_ref()
                .is_some_and(|width| *width <= iota)
        })
        .expect("some cycle count meets the half-width");
    assert_eq!(first, n);

    let again = run(
        request(&claim, &tests, method, &sampler),
        &mut Avail::default(),
    );
    assert_eq!(outcome, again);

    let tight = long_run(Comparison::AtMost, ratio(9995, 10000));
    let undecided = completed(run(
        request(&tight, &tests, method, &sampler),
        &mut Avail::default(),
    ));
    assert_eq!(
        undecided.decision,
        Decision::Undecided(UndecidedCause::IndifferenceRegion)
    );
}

/// A walk whose every cycle is one position where `P` holds (QSpec
/// FR-408-AC-12's degenerate fixture).
struct Identical;

struct IdenticalWalk;

impl RegenerationWalk for IdenticalWalk {
    fn step(&mut self) -> Result<WalkStep, SampleHalt> {
        Ok(WalkStep::Position {
            holds: true,
            weight: Weight::unit(),
            returned: true,
        })
    }
}

impl BehaviourSampler for Identical {
    type Walk = IdenticalWalk;

    fn sample(
        &mut self,
        _test: &TestSubject,
        _seed: u64,
        _trace: u64,
    ) -> Result<Sample, SampleHalt> {
        Err(SampleHalt::NotMarkov)
    }

    fn walk(&mut self, _test: &TestSubject, _seed: u64, _trace: u64) -> IdenticalWalk {
        IdenticalWalk
    }
}

/// FR-190-AC-2: a start state no step re-enters stops `ResourceExhausted`
/// on `max_cycle_steps` with no decision; with `min_cycles` above what the
/// half-width rule needs, the run closes exactly `min_cycles` cycles. A
/// degenerate sample's half-width is `z / n`, so with `min_cycles` 1000 it
/// stops at exactly 11,632 cycles (QSpec FR-408-AC-12).
#[trace("TC-625", "FR-190-AC-2")]
#[test]
fn a_transient_start_stops_on_max_cycle_steps_and_min_cycles_is_a_floor() {
    let claim = long_run(Comparison::AtLeast, ratio(999, 1000));
    let tests = one_test();
    let sampler = sampler_ref();
    let mut transient = request(
        &claim,
        &tests,
        StatisticalMethod::Regenerative { min_cycles: 100 },
        &sampler,
    );
    transient.limits.max_cycle_steps = 10_000;
    let mut avail = Avail {
        transient: true,
        ..Avail::default()
    };
    let outcome = run(transient, &mut avail);
    let StatisticalOutcome::Stopped { stop, partial, .. } = outcome else {
        panic!("expected Stopped, got {outcome:?}");
    };
    assert_eq!(
        stop,
        StatisticalStop::Limit {
            limit: StatisticalLimit::MaxCycleSteps,
            value: 10_000
        }
    );
    assert_eq!(
        StatisticalLimit::MaxCycleSteps.setting(),
        "statistical.max_cycle_steps"
    );
    assert_eq!(partial.samples, 0);
    assert_eq!(partial.draws, 10_000);

    let floor = |min_cycles| {
        completed(run(
            request(
                &claim,
                &tests,
                StatisticalMethod::Regenerative { min_cycles },
                &sampler,
            ),
            &mut Identical,
        ))
    };
    let degenerate = floor(1000);
    assert_eq!(degenerate.basis.samples, 11_632);
    assert_eq!(degenerate.decision, Decision::Accepted);
    assert_eq!(floor(20_000).basis.samples, 20_000);
}

/// FR-190-AC-3: under `weighted by duration` a down position weighs 3 in
/// `L_k`, the weight of the `repair` step that leaves it; `Regenerative`
/// for `P95` and `Okamoto` for `LongRun` are refused `NotStatistical`.
#[trace("TC-625", "FR-190-AC-3")]
#[test]
fn a_weighted_long_run_reads_each_position_weight_from_its_leaving_step() {
    let claim = claim(
        ClaimForm::LongRunFraction,
        Comparison::AtLeast,
        ratio(99, 100),
        ratio(1, 1000),
    );
    let tests = one_test();
    let sampler = sampler_ref();
    let method = StatisticalMethod::Regenerative { min_cycles: 100 };
    let mut avail = Avail {
        duration: true,
        ..Avail::default()
    };
    let verdict = completed(run(request(&claim, &tests, method, &sampler), &mut avail));
    let cycles = avail.cycles.borrow().clone();
    let failed = cycles
        .iter()
        .find(|(y, l)| l != y)
        .expect("some cycle visits down");
    // One up position (weight 1) and `k` down positions (weight 3 each).
    let down = failed.1.sub(&failed.0);
    assert_eq!(failed.0, int(1));
    assert!(quotient(&down, &int(3)).expect("a third").is_integer());
    let (y, l) = cycles
        .iter()
        .fold((int(0), int(0)), |(y, l), (yk, lk)| (y.add(yk), l.add(lk)));
    assert_eq!(verdict.estimate, y.div(&l));

    let p95 = p95();
    assert_eq!(
        check_statistical(
            request(&p95, &tests, method, &sampler),
            &mut Service::new(5),
            &Cancel::new()
        ),
        Err(StatisticalRefusal::NotStatistical {
            method,
            form: ClaimForm::Quantile,
        })
    );
    assert_eq!(
        check_statistical(
            request(&claim, &tests, StatisticalMethod::Okamoto, &sampler),
            &mut Avail::default(),
            &Cancel::new(),
        ),
        Err(StatisticalRefusal::NotStatistical {
            method: StatisticalMethod::Okamoto,
            form: ClaimForm::LongRunFraction,
        })
    );
}

/// FR-191-AC-1: `NoFault` with Okamoto and `max_samples` 1,000,000 stops
/// before deciding, naming `statistical.max_samples`, with 1,000,000
/// samples counted; with 9,210,341 it completes and decides; the default
/// is 16,777,216.
#[trace("TC-626", "FR-191-AC-1")]
#[test]
fn max_samples_stops_a_run_before_its_method_completes() {
    let claim = no_fault();
    let tests = one_test();
    let sampler = sampler_ref();
    let mut limited = request(&claim, &tests, StatisticalMethod::Okamoto, &sampler);
    limited.limits.max_samples = 1_000_000;
    let outcome = run(limited, &mut fixed(Ok(Sample::Event(true))));
    let StatisticalOutcome::Stopped { stop, partial, .. } = outcome else {
        panic!("expected Stopped, got {outcome:?}");
    };
    assert_eq!(
        stop,
        StatisticalStop::Limit {
            limit: StatisticalLimit::MaxSamples,
            value: 1_000_000
        }
    );
    assert_eq!(
        StatisticalLimit::MaxSamples.setting(),
        "statistical.max_samples"
    );
    assert_eq!(partial.samples, 1_000_000);

    limited.limits.max_samples = 9_210_341;
    let verdict = completed(run(limited, &mut fixed(Ok(Sample::Event(true)))));
    assert_eq!(verdict.decision, Decision::Accepted);
    assert_eq!(verdict.basis.samples, 9_210_341);
    assert_eq!(StatisticalLimits::default().max_samples, 16_777_216);
}

/// FR-191-AC-2: the activation variant at `max_draws` 1,000 stops naming
/// `statistical.max_draws`; a cancelled handle stops `Cancelled`; neither
/// carries a decision.
#[trace("TC-626", "FR-191-AC-2")]
#[test]
fn max_draws_and_cancellation_stop_with_no_decision() {
    let claim = p95();
    let tests = one_test();
    let sampler = sampler_ref();
    let mut limited = request(&claim, &tests, StatisticalMethod::Okamoto, &sampler);
    limited.limits.max_draws = 1_000;
    let outcome = run(limited, &mut Service::activated(5));
    let StatisticalOutcome::Stopped { stop, partial, .. } = outcome else {
        panic!("expected Stopped, got {outcome:?}");
    };
    assert_eq!(
        stop,
        StatisticalStop::Limit {
            limit: StatisticalLimit::MaxDraws,
            value: 1_000
        }
    );
    assert_eq!(
        StatisticalLimit::MaxDraws.setting(),
        "statistical.max_draws"
    );
    assert_eq!(partial.draws, 1_000);
    assert!(partial.samples < 1_000);

    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let outcome = check_statistical(
        request(&claim, &tests, StatisticalMethod::Okamoto, &sampler),
        &mut Service::new(5),
        &cancel,
    )
    .expect("an outcome");
    let StatisticalOutcome::Stopped { stop, partial, .. } = outcome else {
        panic!("expected Stopped, got {outcome:?}");
    };
    assert_eq!(stop, StatisticalStop::Cancelled(CancelCause::Requested));
    assert_eq!(partial.draws, 0);
}

/// FR-191-AC-3: `P95` with SPRT and seed 7 twice gives equal results; the
/// run stopped by `max_samples` 100 and the rerun with the default limit
/// draw equal samples at trace indices 0 to 99; seed 8 is recorded and
/// draws a different sample at some trace index.
#[trace("TC-626", "FR-191-AC-3")]
#[test]
fn a_rerun_reproduces_every_sample_from_its_seed() {
    let claim = p95();
    let tests = one_test();
    let sampler = sampler_ref();
    let sprt = request(&claim, &tests, StatisticalMethod::Sprt, &sampler);
    let first = run(sprt, &mut Service::new(5));
    let second = run(sprt, &mut Service::new(5));
    assert_eq!(first, second);
    assert_eq!(format!("{first:?}"), format!("{second:?}"));

    let mut stopped = sprt;
    stopped.limits.max_samples = 100;
    let mut short = Service::new(5);
    assert!(matches!(
        run(stopped, &mut short),
        StatisticalOutcome::Stopped {
            stop: StatisticalStop::Limit {
                limit: StatisticalLimit::MaxSamples,
                value: 100
            },
            ..
        }
    ));
    let mut full = Service::new(5);
    let _ = run(sprt, &mut full);
    let prefix = |service: &Service| -> Vec<(u64, i64)> {
        service
            .latencies
            .range(0..100)
            .map(|(trace, latency)| (*trace, *latency))
            .collect()
    };
    assert_eq!(prefix(&short).len(), 100);
    assert_eq!(prefix(&short), prefix(&full));

    let mut reseeded = sprt;
    reseeded.seed = 8;
    let mut other = Service::new(5);
    let outcome = run(reseeded, &mut other);
    let seed = match outcome {
        StatisticalOutcome::Completed(verdict) => verdict.provenance.seed,
        StatisticalOutcome::Stopped { provenance, .. }
        | StatisticalOutcome::Unsupported { provenance, .. } => provenance.seed,
    };
    assert_eq!(seed, 8);
    assert!(other.latencies.iter().any(|(trace, latency)| full
        .latencies
        .get(trace)
        .is_some_and(|seven| seven != latency)));
}

/// FR-191-AC-4: a two-test run records per test its initial state index,
/// binding and trace-index range, test 1 starting at trace 1 in steps of 2;
/// changing the method from Okamoto to SPRT changes the result's method.
#[trace("TC-626", "FR-191-AC-4")]
#[test]
fn provenance_records_each_test_and_its_trace_range() {
    let claim = p95();
    let tests = vec![
        TestSubject {
            initial: 0,
            binding: None,
        },
        TestSubject {
            initial: 1,
            binding: None,
        },
    ];
    let sampler = sampler_ref();
    let okamoto = completed(run(
        request(&claim, &tests, StatisticalMethod::Okamoto, &sampler),
        &mut Service::new(5),
    ));
    let provenance = &okamoto.provenance;
    assert_eq!(provenance.seed, 7);
    assert_eq!(provenance.sampler, sampler);
    assert_eq!(provenance.tests.len(), 2);
    let second = &provenance.tests[1];
    assert_eq!(
        (second.index, second.initial, &second.binding),
        (1, 1, &None)
    );
    assert_eq!((second.traces.first, second.traces.step), (1, 2));
    assert_eq!(second.traces.count, okamoto.tests[1].draws);
    let sprt = completed(run(
        request(&claim, &tests, StatisticalMethod::Sprt, &sampler),
        &mut Service::new(5),
    ));
    assert_eq!(okamoto.basis.method, StatisticalMethod::Okamoto);
    assert_eq!(sprt.basis.method, StatisticalMethod::Sprt);
}

/// A request is refused before any draw for no test or a trace-index
/// overflow, and on the sampler's generator mismatch; a claim whose
/// indifference region leaves `(0, 1)`, or a strict bound off a quantile,
/// is not a claim EN-4 decides.
#[test]
fn requests_and_claims_outside_the_method_are_refused() {
    let claim = p95();
    let sampler = sampler_ref();
    let none: Vec<TestSubject> = Vec::new();
    let mut probe = fixed(Ok(Sample::Event(true)));
    assert_eq!(
        check_statistical(
            request(&claim, &none, StatisticalMethod::Okamoto, &sampler),
            &mut probe,
            &Cancel::new(),
        ),
        Err(StatisticalRefusal::NoTest)
    );
    let tests = vec![
        TestSubject {
            initial: 0,
            binding: None,
        };
        2
    ];
    let mut overflowing = request(&claim, &tests, StatisticalMethod::Okamoto, &sampler);
    overflowing.limits.max_draws = u64::MAX;
    assert_eq!(
        check_statistical(overflowing, &mut probe, &Cancel::new()),
        Err(StatisticalRefusal::TraceIndexRange {
            max_draws: u64::MAX,
            tests: 2
        })
    );
    assert_eq!(probe.calls, 0);

    let other = DefinitionRef::new("agent-ix", "quire.simulation.sampler/v2").expect("reference");
    let mut mismatch = fixed(Err(SampleHalt::GeneratorMismatch {
        supplied: other.clone(),
    }));
    assert_eq!(
        check_statistical(
            request(&claim, &one_test(), StatisticalMethod::Okamoto, &sampler),
            &mut mismatch,
            &Cancel::new(),
        ),
        Err(StatisticalRefusal::GeneratorMismatch { supplied: other })
    );

    let wide = Bound {
        comparison: Comparison::AtLeast,
        threshold: ratio(1, 20),
    };
    assert_eq!(
        StatisticalClaim::new(ClaimForm::Probability, wide, confidence(ratio(1, 10))),
        Err(ClaimRefusal::NoIndifferenceRegion)
    );
    let strict = Bound {
        comparison: Comparison::Below,
        threshold: ratio(1, 2),
    };
    assert_eq!(
        StatisticalClaim::new(ClaimForm::Probability, strict, confidence(ratio(1, 10))),
        Err(ClaimRefusal::StrictBound)
    );
}
