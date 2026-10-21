// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-193 "Replay" and "The qualitative claim" (TC-628, TC-640): a sampled
//! witness of `check_statistical` replays by re-running the sampler through
//! `replay_sampled_witness`, over the admitted subject and the claim's
//! reader, and converts to the ADR-018 counterexample it is.

use ix_trace_rs::trace;
use qsl_analyze::statistical::{
    check_statistical, witness_reader, StatisticalRequest, DEFAULT_MAX_WITNESSES,
};
use qsl_eval::simulation::model::{ModelSystem, TransitionId};
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::simulation::{sample_probabilistic, SampledDisagreement};
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::diagnostic::{CatalogCode, UndefinedReason};
use qsl_foundation::selection::DefinitionRef;
use qsl_replay::statistical::{
    replay_sampled_witness, SampledParity, SampledWitness, SampledWitnessRefusal,
    SampledWitnessReplay, StatisticalLimits, StatisticalMethod, StatisticalOutcome,
    StatisticalProvenance, StatisticalVerdict, WitnessReplay, WitnessValue,
};
use qsl_replay::{CounterexampleKind, UndefinedLocus};
use qsl_semantics::check::ProbScheduler;
use quire_exact::{Cancel, Integer, Value};

use crate::prob_world::{self, World};

fn sampler() -> DefinitionRef {
    DefinitionRef::new("agent-ix", "quire.simulation.sampler/v1").expect("a valid DefinitionRef")
}

/// `claim` of `world` decided with `method` at seed 7.
fn decide(
    world: &World,
    claim: &str,
    method: StatisticalMethod,
    max_witnesses: u32,
) -> StatisticalVerdict {
    let outcome = check_statistical(
        StatisticalRequest {
            subject: world.subject(),
            claim: world.claim(claim),
            method,
            seed: 7,
            sampler: sampler(),
            limits: StatisticalLimits::default(),
            max_witnesses,
            symmetry: Vec::new(),
        },
        &Cancel::new(),
    )
    .expect("the claim is decided");
    match outcome {
        StatisticalOutcome::Completed(verdict) => *verdict,
        other => panic!("expected a completed run, got {other:?}"),
    }
}

/// `witness` replayed over `world` under `provenance`, reading `claim`.
fn replay(
    world: &World,
    claim: &str,
    provenance: &StatisticalProvenance,
    witness: &SampledWitness,
) -> Result<SampledWitnessReplay, SampledWitnessRefusal> {
    let system = ModelSystem::new(world.subject().simulation).expect("the subject admits");
    let cancel = Cancel::new();
    let checked = world.claim(claim);
    let ProbScheduler::Workload(name) = &checked.scheduler else {
        panic!("a workload claim");
    };
    let workload = world
        .package
        .graph()
        .probabilistic_model()
        .workload(name)
        .expect("the workload is declared");
    let mut reader = witness_reader(&system, &world.package, checked, &sampler(), &cancel)
        .expect("the claim is read");
    replay_sampled_witness(
        &WitnessReplay {
            system: &system,
            workload,
            provenance,
            cancel: &cancel,
        },
        witness,
        &mut reader,
    )
}

fn p95(name: &str, threshold: u32) -> String {
    format!(
        "probabilistic {name} using ev on M over (s: M::Server) under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.01 \
         {{ quantile 0.95 of (accumulate duration from holds(s.phase = 1) \
         until holds(s.phase = 2) within 10) <= {threshold} }}\n"
    )
}

fn invalid_value() -> CatalogCode {
    CatalogCode::new("invalid_runtime_input", "invalid-value")
}

/// Checks FR-193-AC-2 (TC-628 step 2): each witness of the `2 ms` claim
/// replays `Reproduced`; with `d` changed to `2 ms` it refuses
/// `invalid_runtime_input`/`invalid-value`; under a seed whose draws differ
/// it settles `Inconclusive` on a draw; with its path probability changed it
/// settles `Inconclusive` on the path probability; and the verdict is the
/// same after each replay, so the item stays `measured`, rejected.
#[trace("TC-628", "FR-193-AC-2")]
#[test]
fn a_kept_witness_replays_and_each_alteration_is_caught() {
    let world = World::new(&prob_world::service(&p95("Two", 2)));
    let verdict = decide(&world, "Two", StatisticalMethod::Okamoto, 3);
    let before = verdict.clone();
    assert_eq!(verdict.witnesses.len(), 3);
    for witness in &verdict.witnesses {
        assert_eq!(
            replay(&world, "Two", &verdict.provenance, witness),
            Ok(SampledWitnessReplay::Reproduced)
        );
    }

    let witness = &verdict.witnesses[0];
    let attempt = witness
        .steps
        .iter()
        .position(|step| step.random.len() == 2)
        .expect("the witness takes an attempt step");
    let mut outside = witness.clone();
    outside.steps[attempt].random[0] = Value::Integer(Integer::from(2_i64));
    let refusal = replay(&world, "Two", &verdict.provenance, &outside)
        .expect_err("2 ms is outside the support");
    assert_eq!(refusal.catalog_code(), invalid_value());

    // A seed under which the recomputed attempt step draws `d = 1 ms`, where
    // the record has another `d`.
    let system = ModelSystem::new(world.subject().simulation).expect("the subject admits");
    let ProbScheduler::Workload(name) = &world.claim("Two").scheduler else {
        panic!("a workload claim");
    };
    let workload = world
        .package
        .graph()
        .probabilistic_model()
        .workload(name)
        .expect("the workload is declared");
    let start = system.initial().remove(0);
    let is_one = |value: Option<&Value>| matches!(value, Some(Value::Integer(integer)) if *integer == Integer::from(1_i64));
    let found = (8_u64..).take(512).find_map(|seed| {
        let mut provenance = verdict.provenance.clone();
        provenance.seed = seed;
        let Ok(SampledWitnessReplay::Inconclusive(SampledParity::Draw(
            SampledDisagreement::Step { step },
        ))) = replay(&world, "Two", &provenance, witness)
        else {
            return None;
        };
        let at = usize::try_from(step).ok()?;
        let recorded = witness.steps.get(at)?;
        let other = sample_probabilistic(
            &system,
            workload,
            &start,
            &sampler(),
            seed,
            witness.trace,
            u64::try_from(witness.steps.len()).ok()?.saturating_add(1),
            &Cancel::new(),
        )
        .ok()?;
        let drawn = other.steps.get(at)?;
        (recorded.random.len() == 2
            && is_one(drawn.random.first())
            && !is_one(recorded.random.first()))
        .then_some(seed)
    });
    assert!(
        found.is_some(),
        "some seed recomputes d = 1 ms at an attempt"
    );

    let mut doubled = witness.clone();
    doubled.path_probability = witness.path_probability.add(&witness.path_probability);
    assert_eq!(
        replay(&world, "Two", &verdict.provenance, &doubled),
        Ok(SampledWitnessReplay::Inconclusive(SampledParity::Draw(
            SampledDisagreement::PathProbability {
                recomputed: witness.path_probability.clone()
            }
        )))
    );
    assert_eq!(verdict, before, "a replay leaves the verdict unchanged");
}

/// Checks FR-193-AC-2's value clause: a witness whose recorded value is not
/// the one the claim reads on the recomputed sample, and one cut short of
/// its steps, settle `Inconclusive` though every recorded draw is right.
#[trace("TC-628", "FR-193-AC-2")]
#[test]
fn a_witness_with_another_value_or_length_is_inconclusive() {
    let world = World::new(&prob_world::service(&p95("Two", 2)));
    let verdict = decide(&world, "Two", StatisticalMethod::Okamoto, 1);
    let witness = &verdict.witnesses[0];

    let mut other = witness.clone();
    other.value = WitnessValue::Event(true);
    let Ok(SampledWitnessReplay::Inconclusive(SampledParity::Value { recorded, replayed })) =
        replay(&world, "Two", &verdict.provenance, &other)
    else {
        panic!("another value disagrees");
    };
    assert_eq!(*recorded, WitnessValue::Event(true));
    assert_eq!(replayed.as_deref(), Some(&witness.value));

    let mut short = witness.clone();
    short.steps.pop();
    short.path_probability = short.steps.iter().fold(
        witness
            .path_probability
            .div(&witness.path_probability)
            .expect("one"),
        |product, step| product.mul(&step.probability),
    );
    assert!(matches!(
        replay(&world, "Two", &verdict.provenance, &short),
        Ok(SampledWitnessReplay::Inconclusive(
            SampledParity::Length { .. }
        ))
    ));
}

/// Checks FR-193-AC-4 (TC-640 step 3): FR-189-AC-6's undefined sample is
/// kept as a witness with value `Undefined(UndefinedEvaluation)` and replays
/// `Reproduced`; with its cause changed to `precondition-false` it settles
/// `Inconclusive`.
#[trace("TC-640", "FR-193-AC-4")]
#[test]
fn an_undefined_witness_replays_and_a_changed_cause_is_inconclusive() {
    let claim = "probabilistic Kept using ev on M over (c: M::Coin) under Even on origin \
         with alpha 0.01, beta 0.01, indifference 0.1 \
         { probability >= 0.5 [ always[0,3] holds(c.v = 0) ] }";
    let world = World::new(&prob_world::coin(claim));
    let verdict = decide(
        &world,
        "Kept",
        StatisticalMethod::Okamoto,
        DEFAULT_MAX_WITNESSES,
    );
    let witness = verdict.witnesses.first().expect("the undefined witness");
    let WitnessValue::Undefined(evaluation) = &witness.value else {
        panic!("an undefined witness: {:?}", witness.value);
    };
    assert_eq!(
        replay(&world, "Kept", &verdict.provenance, witness),
        Ok(SampledWitnessReplay::Reproduced)
    );

    let mut changed = witness.clone();
    let mut evaluation = evaluation.clone();
    let UndefinedCause::Family(record) = &mut evaluation.cause else {
        panic!("a family cause: {:?}", evaluation.cause);
    };
    record.reason = UndefinedReason::PreconditionFalse;
    changed.value = WitnessValue::Undefined(evaluation);
    assert!(matches!(
        replay(&world, "Kept", &verdict.provenance, &changed),
        Ok(SampledWitnessReplay::Inconclusive(
            SampledParity::Value { .. }
        ))
    ));
}

/// Checks FR-193 "The qualitative claim" (FR-193-AC-3's conversion): a
/// witness of ADR-024 §7.1's fault claim, one with a fault, converts to the
/// counterexample of its subject: the initial state and binding its test
/// records and its steps and post-state digests; an undefined witness
/// converts to the undefined-evaluation counterexample of its evaluation.
#[trace("TC-628", "FR-193-AC-3")]
#[test]
fn a_witness_is_the_counterexample_of_the_qualitative_claim() {
    let claim = "probabilistic Fault using ev on M over (n: M::Node) under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.005 \
         { probability <= 0.01 [ eventually[0,1000] holds(not n.healthy) ] }";
    let world = World::new(&prob_world::health(
        "self.healthy = (pre(self.healthy) and not fault)",
        "true: 1, false: 99",
        false,
        claim,
    ));
    let verdict = decide(&world, "Fault", StatisticalMethod::Sprt, 1);
    assert_eq!(verdict.witnesses.len(), 1);
    let witness = &verdict.witnesses[0];
    assert_eq!(witness.value, WitnessValue::Event(true));
    let counterexample = witness
        .counterexample(&verdict.provenance)
        .expect("the provenance places the witness");
    let test = &verdict.provenance.tests[0];
    assert_eq!(counterexample.initial_state, 0);
    assert_eq!(counterexample.over_binding, test.binding);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(counterexample.lasso, None);
    assert_eq!(counterexample.prefix.len(), witness.steps.len());
    for (step, taken) in counterexample.prefix.iter().zip(&witness.steps) {
        let TransitionId::Operation(transition) = &taken.transition else {
            panic!("an operation step");
        };
        assert_eq!(&step.transition, transition);
        assert_eq!(step.post_state, taken.post_digest);
    }

    let unknown = SampledWitness {
        test: 9,
        ..witness.clone()
    };
    assert_eq!(
        unknown.counterexample(&verdict.provenance),
        Err(SampledWitnessRefusal::UnknownTest { test: 9 })
    );

    let coin = "probabilistic Kept using ev on M over (c: M::Coin) under Even on origin \
         with alpha 0.01, beta 0.01, indifference 0.1 \
         { probability >= 0.5 [ always[0,3] holds(c.v = 0) ] }";
    let world = World::new(&prob_world::coin(coin));
    let coin_verdict = decide(&world, "Kept", StatisticalMethod::Okamoto, 0);
    let undefined = coin_verdict
        .witnesses
        .first()
        .expect("the undefined witness");
    let counterexample = undefined
        .counterexample(&coin_verdict.provenance)
        .expect("the provenance places the witness");
    let WitnessValue::Undefined(evaluation) = &undefined.value else {
        panic!("undefined");
    };
    assert_eq!(
        counterexample.kind,
        CounterexampleKind::UndefinedEvaluation(evaluation.clone())
    );
    assert!(matches!(evaluation.site.locus, UndefinedLocus::Atom(_)));
}

/// Checks FR-193-AC-4's replay clause for a long-run fraction, whose only
/// witness is an undefined one: over `Coin`, the regenerative walk reads the
/// coin after a `drop` with `b = 0` deleted it (`absent-key`), keeps that
/// walk as an undefined witness, and the witness replays `Reproduced` by
/// walking again to its position.
#[trace("TC-640", "FR-193-AC-4")]
#[test]
fn a_long_run_undefined_witness_replays() {
    let claim = "probabilistic Stays using ev on M over (c: M::Coin) under Even on origin \
         with alpha 0.1, beta 0.1, indifference 0.1 \
         { long_run_fraction holds(c.v = 0) >= 0.3 }";
    let world = World::new(&prob_world::coin(claim));
    let verdict = decide(
        &world,
        "Stays",
        StatisticalMethod::Regenerative { min_cycles: 10 },
        DEFAULT_MAX_WITNESSES,
    );
    let [witness] = &verdict.witnesses[..] else {
        panic!("one undefined witness: {:?}", verdict.witnesses);
    };
    assert!(matches!(witness.value, WitnessValue::Undefined(_)));
    assert_eq!(
        replay(&world, "Stays", &verdict.provenance, witness),
        Ok(SampledWitnessReplay::Reproduced)
    );
}

/// FR-193 "Replay": a witness whose test the provenance does not record, or
/// whose test starts at an initial state the subject lacks, is refused
/// `invalid_runtime_input`/`invalid-value`, not settled.
#[trace("TC-628", "FR-193-AC-2")]
#[test]
fn a_witness_the_subject_does_not_place_is_refused_with_a_catalog_code() {
    let world = World::new(&prob_world::service(&p95("Two", 2)));
    let verdict = decide(&world, "Two", StatisticalMethod::Okamoto, 1);
    let witness = &verdict.witnesses[0];

    let stray = SampledWitness {
        test: 9,
        ..witness.clone()
    };
    let refusal = replay(&world, "Two", &verdict.provenance, &stray).expect_err("no test 9");
    assert_eq!(refusal, SampledWitnessRefusal::UnknownTest { test: 9 });
    assert_eq!(refusal.catalog_code(), invalid_value());

    let mut provenance = verdict.provenance.clone();
    provenance.tests[0].initial = 5;
    let refusal = replay(&world, "Two", &provenance, witness).expect_err("no initial state 5");
    assert!(matches!(
        refusal,
        SampledWitnessRefusal::UnknownInitial { initial: 5, .. }
    ));
    assert_eq!(refusal.catalog_code(), invalid_value());
}

/// Checks FR-193-AC-3's first clause: a `Measure` witness of the `2 ms`
/// claim converts to the counterexample of the qualitative claim, with its
/// steps and post digests, as a formula refutation. (Its FR-128 settlement
/// is STD-250's.)
#[trace("TC-628", "FR-193-AC-3")]
#[test]
fn a_measure_witness_converts_to_a_formula_counterexample() {
    let world = World::new(&prob_world::service(&p95("Two", 2)));
    let verdict = decide(&world, "Two", StatisticalMethod::Okamoto, 1);
    let witness = &verdict.witnesses[0];
    assert!(matches!(witness.value, WitnessValue::Measure(_)));
    let counterexample = witness
        .counterexample(&verdict.provenance)
        .expect("the provenance places the witness");
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(counterexample.prefix.len(), witness.steps.len());
    for (step, taken) in counterexample.prefix.iter().zip(&witness.steps) {
        let TransitionId::Operation(transition) = &taken.transition else {
            panic!("an operation step");
        };
        assert_eq!(&step.transition, transition);
        assert_eq!(step.post_state, taken.post_digest);
    }
}

/// FR-193 "Replay": a replay the caller cancels is refused
/// `cancelled`/`caller-cancelled`, not reported as a broken invariant.
#[trace("TC-628", "FR-193-AC-2")]
#[test]
fn a_cancelled_replay_is_refused_as_cancelled() {
    let world = World::new(&prob_world::service(&p95("Two", 2)));
    let verdict = decide(&world, "Two", StatisticalMethod::Okamoto, 1);
    let system = ModelSystem::new(world.subject().simulation).expect("the subject admits");
    let cancel = Cancel::new();
    cancel.cancel(quire_exact::CancelCause::Requested);
    let checked = world.claim("Two");
    let ProbScheduler::Workload(name) = &checked.scheduler else {
        panic!("a workload claim");
    };
    let workload = world
        .package
        .graph()
        .probabilistic_model()
        .workload(name)
        .expect("the workload is declared");
    let mut reader = witness_reader(&system, &world.package, checked, &sampler(), &cancel)
        .expect("the claim is read");
    let refusal = replay_sampled_witness(
        &WitnessReplay {
            system: &system,
            workload,
            provenance: &verdict.provenance,
            cancel: &cancel,
        },
        &verdict.witnesses[0],
        &mut reader,
    )
    .expect_err("a cancelled replay is refused");
    assert_eq!(
        refusal.catalog_code(),
        CatalogCode::new("cancelled", "caller-cancelled")
    );
}
