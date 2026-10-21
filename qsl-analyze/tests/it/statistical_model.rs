// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-189 to FR-193 (TC-624, TC-625, TC-627, TC-628, TC-640): EN-4's
//! `check_statistical` over ADR-024 §7's models (see [`crate::prob_world`]):
//! FR-188's behaviours drawn from the subject, each read by the layer-5
//! evaluator.

use ix_trace_rs::trace;
use qsl_analyze::statistical::{check_statistical, StatisticalRequest, DEFAULT_MAX_WITNESSES};
use qsl_eval::simulation::model::TransitionId;
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::diagnostic::UndefinedReason;
use qsl_foundation::selection::DefinitionRef;
use qsl_replay::statistical::{
    Decision, MeasureValue, StatisticalLimit, StatisticalLimits, StatisticalMethod,
    StatisticalOutcome, StatisticalRefusal, StatisticalStop, StatisticalUnsupported,
    StatisticalVerdict, WitnessValue,
};
use qsl_replay::UndefinedLocus;
use quire_exact::{Cancel, Integer, Rational, Value};

use crate::prob_world::{self, World};

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

fn sampler() -> DefinitionRef {
    DefinitionRef::new("agent-ix", "quire.simulation.sampler/v1").expect("a valid DefinitionRef")
}

fn run(
    world: &World,
    claim: &str,
    method: StatisticalMethod,
    limits: StatisticalLimits,
    max_witnesses: u32,
) -> Result<StatisticalOutcome, StatisticalRefusal> {
    check_statistical(
        StatisticalRequest {
            subject: world.subject(),
            claim: world.claim(claim),
            method,
            seed: 7,
            sampler: sampler(),
            limits,
            max_witnesses,
        },
        &Cancel::new(),
    )
}

fn completed(outcome: Result<StatisticalOutcome, StatisticalRefusal>) -> StatisticalVerdict {
    match outcome {
        Ok(StatisticalOutcome::Completed(verdict)) => *verdict,
        other => panic!("expected a completed run, got {other:?}"),
    }
}

fn decided(world: &World, claim: &str, method: StatisticalMethod) -> StatisticalVerdict {
    completed(run(
        world,
        claim,
        method,
        StatisticalLimits::default(),
        DEFAULT_MAX_WITNESSES,
    ))
}

fn integer_of(value: &Value) -> i64 {
    match value {
        Value::Integer(integer) => integer.to_string().parse().expect("small"),
        other => panic!("not an integer: {other:?}"),
    }
}

/// ADR-024 §7.2's `P95` at `threshold`: the `duration` from `Busy` to
/// `Done` within 10 steps.
fn p95(name: &str, threshold: u32) -> String {
    format!(
        "probabilistic {name} using ev on M over (s: M::Server) under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.01 \
         {{ quantile 0.95 of (accumulate duration from holds(s.phase = 1) \
         until holds(s.phase = 2) within 10) <= {threshold} }}\n"
    )
}

fn service() -> World {
    World::new(&prob_world::service(&format!(
        "{}{}",
        p95("P95", 5),
        p95("Two", 2)
    )))
}

/// Checks FR-189-AC-1 (TC-624 step 1): `P95` with Okamoto runs one test of
/// `N = 23,026` samples, each of at most 11 positions, and is Accepted with
/// an interval of half-width `1/100` whose lower end is at least `19/20`; at
/// `2 ms` it is Rejected with an interval whose upper end is below `19/20`.
/// SPRT decides both the same way with fewer samples.
#[trace("TC-624", "FR-189-AC-1")]
#[test]
fn p95_over_service_is_decided_from_the_models_own_behaviours() {
    let world = service();
    let accepted = decided(&world, "P95", StatisticalMethod::Okamoto);
    assert_eq!(accepted.decision, Decision::Accepted);
    assert_eq!(accepted.tests.len(), 1);
    assert_eq!(accepted.tests[0].samples, 23_026);
    let (low, high) = accepted.interval.clone().expect("an Okamoto interval");
    assert_eq!(high.sub(&low), ratio(2, 100));
    assert!(low >= ratio(19, 20), "lower end {low:?}");
    for witness in &accepted.witnesses {
        assert!(witness.steps.len() <= 10, "at most 11 positions");
    }
    let rejected = decided(&world, "Two", StatisticalMethod::Okamoto);
    assert_eq!(rejected.decision, Decision::Rejected);
    let (_, high) = rejected.interval.clone().expect("an Okamoto interval");
    assert!(high < ratio(19, 20), "upper end {high:?}");

    for (claim, decision) in [("P95", Decision::Accepted), ("Two", Decision::Rejected)] {
        let sequential = decided(&world, claim, StatisticalMethod::Sprt);
        assert_eq!(sequential.decision, decision, "{claim}");
        assert!(sequential.tests[0].samples < 23_026, "{claim}");
        assert_eq!(sequential.interval, None);
    }
}

/// Checks FR-193-AC-1 (TC-628 step 1): the `2 ms` claim with seed 7 and
/// `max_witnesses` 3 keeps three witnesses in increasing trace-index order,
/// each with latency above `2 ms` and the product of its steps' FR-187
/// probabilities; in one whose attempt drew `d = 3`, `ok`, the `request`
/// step has probability 1 and the `attempt` step `343/5000`.
#[trace("TC-628", "FR-193-AC-1")]
#[test]
fn the_two_ms_claim_keeps_its_first_violating_behaviours() {
    let world = service();
    let verdict = completed(run(
        &world,
        "Two",
        StatisticalMethod::Okamoto,
        StatisticalLimits::default(),
        3,
    ));
    assert_eq!(verdict.witnesses.len(), 3);
    let traces: Vec<u64> = verdict
        .witnesses
        .iter()
        .map(|witness| witness.trace)
        .collect();
    assert!(
        traces.windows(2).all(|pair| pair[0] < pair[1]),
        "{traces:?}"
    );
    let mut three_ok = 0;
    for witness in &verdict.witnesses {
        match &witness.value {
            WitnessValue::Measure(MeasureValue::Value(latency)) => {
                assert!(*latency > ratio(2, 1), "{latency:?}");
            }
            WitnessValue::Measure(MeasureValue::Censored) => {}
            other => panic!("a measured witness: {other:?}"),
        }
        let product = witness
            .steps
            .iter()
            .fold(ratio(1, 1), |product, step| product.mul(&step.probability));
        assert_eq!(witness.path_probability, product);
        let [request, attempt, ..] = &witness.steps[..] else {
            panic!("a request and an attempt: {:?}", witness.steps);
        };
        assert_eq!(request.probability, ratio(1, 1));
        if integer_of(&attempt.random[0]) == 3 && matches!(attempt.random[1], Value::Boolean(true))
        {
            assert_eq!(attempt.probability, ratio(343, 5000));
            three_ok += 1;
        }
    }
    assert!(three_ok > 0, "a witness whose attempt drew 3 ms and ok");
}

/// Checks FR-189-AC-4 (TC-624 step 4), the activation clause: over a
/// `Service` whose `request` starts the server with probability `1/2`, an
/// Okamoto `P95` counts only activated samples, `23,026` of them, and draws
/// more.
#[trace("TC-624")]
#[test]
fn a_quantile_counts_only_activated_samples() {
    let world = World::new(&prob_world::activated_service(&p95("P95", 5)));
    let verdict = decided(&world, "P95", StatisticalMethod::Okamoto);
    assert_eq!(verdict.tests[0].samples, 23_026);
    assert!(verdict.tests[0].draws > verdict.tests[0].samples);
}

/// Checks FR-189-AC-12 (TC-624 step 9): the activation variant whose
/// activation never holds, with `max_draws` 1,000, stops `MaxDraws` with no
/// sample counted.
#[trace("TC-624", "FR-189-AC-12")]
#[test]
fn a_quantile_with_no_activation_stops_on_max_draws() {
    let never = "probabilistic Never using ev on M over (s: M::Server) under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.01 \
         { quantile 0.95 of (accumulate duration from holds(s.phase = 1 and s.phase = 2) \
         until holds(s.phase = 2) within 10) <= 5 }";
    let world = World::new(&prob_world::activated_service(never));
    let outcome = run(
        &world,
        "Never",
        StatisticalMethod::Okamoto,
        StatisticalLimits {
            max_draws: 1_000,
            ..StatisticalLimits::default()
        },
        DEFAULT_MAX_WITNESSES,
    );
    let Ok(StatisticalOutcome::Stopped { stop, partial, .. }) = outcome else {
        panic!("a stopped run: {outcome:?}");
    };
    assert_eq!(
        stop,
        StatisticalStop::Limit {
            limit: StatisticalLimit::MaxDraws,
            value: 1_000
        }
    );
    assert_eq!(partial.samples, 0);
    assert_eq!(partial.draws, 1_000);
}

/// Checks FR-189-AC-5 (TC-624 step 5), the `NotMarkov` clause: a `Health`
/// whose `tick` leaves `spare` unconstrained has two post-states per draw,
/// and the run returns `Unsupported` naming the state, the scheduled
/// identity, the draw and the two post-states.
#[trace("TC-624")]
#[test]
fn a_non_unique_post_state_is_unsupported_with_its_step() {
    let claim = "probabilistic Steadily using ev on M over (n: M::Node) under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.1 \
         { probability >= 0.5 [ always[0,3] holds(n.healthy) ] }";
    let world = World::new(&prob_world::health(
        "self.healthy = (pre(self.healthy) and not fault)",
        "true: 1, false: 9",
        true,
        claim,
    ));
    let outcome = run(
        &world,
        "Steadily",
        StatisticalMethod::Okamoto,
        StatisticalLimits::default(),
        DEFAULT_MAX_WITNESSES,
    );
    let Ok(StatisticalOutcome::Unsupported {
        cause: StatisticalUnsupported::NotMarkov(cause),
        provenance,
    }) = outcome
    else {
        panic!("an unsupported run: {outcome:?}");
    };
    assert_eq!(cause.post_states, 2);
    assert!(
        cause.transition.operation.contains("Node/tick"),
        "{cause:?}"
    );
    assert_eq!(cause.random.len(), 1);
    assert_eq!(provenance.tests.len(), 1);
}

/// Checks FR-189-AC-6 (TC-640 step 1) and FR-193-AC-4's keeping clause:
/// over `Coin`, `always[0,3] holds(c.v = 0)` is Rejected at the first sample
/// whose path drops the coin with `b = 0`, no later sample is drawn, and the
/// test's `undefined` names test 0, that sample's trace index and the first
/// position after the drop, `absent-key`; the sample is kept as an undefined
/// witness.
#[trace("TC-640", "FR-189-AC-6")]
#[test]
fn a_dropped_coin_rejects_its_test_at_the_undefined_position() {
    let claim = "probabilistic Kept using ev on M over (c: M::Coin) under Even on origin \
         with alpha 0.01, beta 0.01, indifference 0.1 \
         { probability >= 0.5 [ always[0,3] holds(c.v = 0) ] }";
    let world = World::new(&prob_world::coin(claim));
    let verdict = decided(&world, "Kept", StatisticalMethod::Okamoto);
    assert_eq!(verdict.decision, Decision::Rejected);
    let [test] = &verdict.tests[..] else {
        panic!("one test: {:?}", verdict.tests);
    };
    assert_eq!(test.decision, Decision::Rejected);
    let undefined = test.undefined.as_ref().expect("an undefined sample");
    assert_eq!(undefined.test, 0);
    // The run stops at this sample: it is the test's last draw.
    assert_eq!(undefined.trace, test.draws - 1);
    let witness = verdict.witnesses.first().expect("the undefined witness");
    assert_eq!(witness.trace, undefined.trace);
    assert_eq!(
        witness.value,
        WitnessValue::Undefined(undefined.evaluation.clone())
    );
    let drop = witness.steps.last().expect("the drop");
    assert!(matches!(drop.transition, TransitionId::Operation(_)));
    assert_eq!(integer_of(&drop.random[0]), 0);
    for kept in &witness.steps[..witness.steps.len() - 1] {
        assert_eq!(
            integer_of(&kept.random[0]),
            1,
            "an earlier drop keeps the coin"
        );
    }
    let site = &undefined.evaluation.site;
    assert_eq!(
        site.position,
        u64::try_from(witness.steps.len()).expect("small")
    );
    assert!(matches!(site.locus, UndefinedLocus::Atom(_)));
    let UndefinedCause::Family(record) = &undefined.evaluation.cause else {
        panic!("absent-key: {:?}", undefined.evaluation.cause);
    };
    assert_eq!(record.reason, UndefinedReason::AbsentKey);
}

/// Checks FR-189-AC-7 (TC-624 step 7): over `Wide`, whose `spin` draws from
/// weights totalling `2^256 + 1`, the first sample stops the run
/// `Unsupported` with `TotalAboveDrawRange{total}`.
#[trace("TC-624", "FR-189-AC-7")]
#[test]
fn a_draw_above_the_range_is_unsupported() {
    let claim = "probabilistic Spun using ev on M over (w: M::Wide) under Unit on origin \
         with alpha 0.01, beta 0.01, indifference 0.1 \
         { probability >= 0.5 [ always[0,3] holds(w.v or not w.v) ] }";
    let world = World::new(&prob_world::wide(claim));
    let outcome = run(
        &world,
        "Spun",
        StatisticalMethod::Okamoto,
        StatisticalLimits::default(),
        DEFAULT_MAX_WITNESSES,
    );
    let Ok(StatisticalOutcome::Unsupported {
        cause: StatisticalUnsupported::TotalAboveDrawRange { total },
        provenance,
    }) = outcome
    else {
        panic!("an unsupported run: {outcome:?}");
    };
    assert_eq!(
        total.to_string(),
        "115792089237316195423570985008687907853269984665640564039457584007913129639937"
    );
    assert_eq!(provenance.tests[0].traces.count, 1);
}

/// FR-192-AC-4's gate clauses (TC-627 step 4): through `check_statistical`,
/// a claim under every scheduler, one with no confidence parameters and an
/// expected-reward form are each refused with its cause before the subject
/// is read.
#[trace("TC-627")]
#[test]
fn exact_only_claims_are_refused_before_any_sample() {
    let claims = "probabilistic Every using ev on M over (s: M::Server) under every scheduler \
         on origin with alpha 0.01, beta 0.01, indifference 0.01 \
         { probability >= 0.5 [ eventually[0,3] holds(s.phase = 2) ] }\n\
         probabilistic Bare using ev on M over (s: M::Server) under Steady on origin \
         { probability >= 0.5 [ eventually[0,3] holds(s.phase = 2) ] }\n\
         probabilistic Expected using inf on M over (s: M::Server) under Steady on origin \
         { expected accumulate duration until holds(s.phase = 2) <= 5 }";
    let world = World::new(&prob_world::service(claims));
    for (claim, refusal) in [
        ("Every", StatisticalRefusal::EveryScheduler),
        ("Bare", StatisticalRefusal::MissingConfidence),
        ("Expected", StatisticalRefusal::ExactOnlyForm),
    ] {
        assert_eq!(
            run(
                &world,
                claim,
                StatisticalMethod::Okamoto,
                StatisticalLimits::default(),
                DEFAULT_MAX_WITNESSES
            ),
            Err(refusal),
            "{claim}"
        );
    }
}

/// FR-189 "Methods" and FR-190: over `Service` a mean of a fraction is
/// decided by Okamoto over the windowed fraction of `Idle` positions; over a
/// `Health` whose `tick` sets `healthy` to a fair coin, a long-run fraction
/// is decided by the regenerative method, which keeps no witness
/// (FR-193-AC-3's `LongRun` clause).
#[trace("TC-624", "TC-625")]
#[test]
fn fractions_are_decided_over_the_models_behaviours() {
    let mean = "probabilistic Mean using ev on M over (s: M::Server) under Steady on origin \
         with alpha 0.1, beta 0.1, indifference 0.1 \
         { mean of (fraction holds(s.phase = 0) over [0, 10]) >= 0.2 }";
    let world = World::new(&prob_world::service(mean));
    let mean = decided(&world, "Mean", StatisticalMethod::Okamoto);
    assert_eq!(mean.decision, Decision::Accepted);
    // `N = ⌈ln 10 / (2 · 1/100)⌉`.
    assert_eq!(mean.tests[0].samples, 116);
    let healthy = "probabilistic Up using ev on M over (n: M::Node) under Steady on origin \
         with alpha 0.1, beta 0.1, indifference 0.1 \
         { long_run_fraction holds(n.healthy) >= 0.3 }";
    let world = World::new(&prob_world::health(
        "self.healthy = (not fault)",
        "true: 1, false: 1",
        false,
        healthy,
    ));
    let long_run = decided(
        &world,
        "Up",
        StatisticalMethod::Regenerative { min_cycles: 10 },
    );
    assert_eq!(long_run.decision, Decision::Accepted);
    let estimate = long_run.estimate.clone().expect("a ratio estimate");
    let (low, high) = long_run.interval.clone().expect("a regenerative interval");
    assert!(
        low >= ratio(3, 10) && low < estimate && estimate < high,
        "{low:?} {estimate:?} {high:?}"
    );
    assert!(long_run.tests[0].samples >= 10, "min_cycles cycles closed");
    assert!(long_run.witnesses.is_empty());
}

/// FR-189 "Inputs": the sampler is checked by identity before any draw.
#[trace("TC-624")]
#[test]
fn another_sampler_is_refused() {
    let world = service();
    let other = DefinitionRef::new("agent-ix", "quire.simulation.other/v1").expect("valid");
    let refusal = check_statistical(
        StatisticalRequest {
            subject: world.subject(),
            claim: world.claim("P95"),
            method: StatisticalMethod::Okamoto,
            seed: 7,
            sampler: other.clone(),
            limits: StatisticalLimits::default(),
            max_witnesses: DEFAULT_MAX_WITNESSES,
        },
        &Cancel::new(),
    );
    assert_eq!(
        refusal,
        Err(StatisticalRefusal::GeneratorMismatch { supplied: other })
    );
}
