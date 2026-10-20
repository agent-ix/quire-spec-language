// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-183 (TC-608): the replay of a hyper counterexample through each trace
//! variable's `ModelSystem`, on the vault of `crate::hyper_world`.
//!
//! The counterexamples are the engine's own (FR-176) or built from its
//! traces by changing one thing, and each refusal is the one thing changed.
//! The facade entry `replay_model_trace_tuple`, the `WitnessExhausted`,
//! `Projected` and `StepTuple` arms and the recompile that resolves the clause
//! are not built (S4 does not yet emit a hyper clause), so the ACs of FR-183
//! are not claimed whole.

use ix_trace_rs::trace;
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::temporal::UndefinedCause;
use qsl_replay::{
    HyperCounterexample, HyperCounterexampleKind, HyperLocus, HyperOutcome, HyperStep,
    ModelCheckLimits, ModelStep, ModelTraceRefusal, ModelTraceSettlement, ModelTraceValue,
    ReplayRefusal,
};
use quire_exact::{Cancel, Undefined};

use crate::hyper_world::{non_interference, Vault, World, EQUAL_INPUTS, STARTS};

/// The counterexample of `world`'s run.
fn refuted(world: &World) -> HyperCounterexample {
    match world
        .run(ModelCheckLimits::default(), &Cancel::new())
        .expect("the run is not refused")
    {
        HyperOutcome::Violated { counterexample, .. } => *counterexample,
        other => panic!("expected a violation, got {other:?}"),
    }
}

/// The model refusal of a refused replay.
fn refusal(result: Result<ModelTraceSettlement, ReplayRefusal>) -> ModelTraceRefusal {
    match result {
        Err(ReplayRefusal::ModelTrace(refusal)) => *refusal,
        other => panic!("expected a model-trace refusal, got {other:?}"),
    }
}

/// The step `step(input)` out of `state` and the state it reaches.
fn take(world: &World, state: &ModelSimState, input: i64) -> (ModelStep, ModelSimState) {
    let system = ModelSystem::new(world.subject().simulation).expect("the subject admits");
    let (transition, next) = system
        .successors(state)
        .expect("the vault expands")
        .successors
        .into_iter()
        .find(|(transition, _)| {
            transition.operation().contains("/Vault/step\"")
                && transition.arguments()[0].contains(&format!("\"value\":\"{input}\""))
        })
        .expect("step(i) is enabled");
    let post_state = system.state_key(&next).expect("a key").1;
    (
        ModelStep {
            transition,
            post_state,
        },
        next,
    )
}

fn leaky() -> World {
    World::new(
        Vault::Leaky,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    )
}

/// FR-183-AC-1, first sentence: ADR-023 section 8.1's leaky `Lockstep`
/// counterexample replays to `reproduced-with-evaluated-witness` with
/// `trace_position` 1, and replaying one counterexample twice gives equal
/// results.
#[trace("TC-608")]
#[test]
fn the_leaky_lockstep_counterexample_replays_to_its_evaluated_witness() {
    let world = leaky();
    let counterexample = refuted(&world);
    let replayed = world.replay(&counterexample).expect("the replay settles");
    assert_eq!(
        replayed,
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            trace_position: Some(1),
        }
    );
    assert_eq!(world.replay(&counterexample).expect("settles"), replayed);
}

/// FR-183-AC-3, the refusals that settle no result: a counterexample with one
/// trace one step shorter; with `b`'s inputs changed to 1, which makes `μ_U`
/// false; with one post-state digest altered; with a loop that does not
/// close; and with a stutter marker on a state that is not terminal.
#[trace("TC-608")]
#[test]
fn a_counterexample_that_breaks_a_replay_rule_is_refused() {
    let world = leaky();
    let counterexample = refuted(&world);
    let system = ModelSystem::new(world.subject().simulation).expect("the subject admits");

    let mut shorter = counterexample.clone();
    shorter.traces[1].steps.pop();
    assert!(matches!(
        refusal(world.replay(&shorter)),
        ModelTraceRefusal::HyperShape { .. }
    ));

    let mut unmatched = counterexample.clone();
    // `b` takes `step(1)` at both steps, so its loop still closes (its secret
    // is 1, and `l` stays 0); only `μ_U` is false.
    let initial = system.initial()[unmatched.traces[1].initial as usize].clone();
    let (step, _) = take(&world, &initial, 1);
    unmatched.traces[1].steps = vec![HyperStep::Model(step.clone()), HyperStep::Model(step)];
    assert_eq!(
        refusal(world.replay(&unmatched)),
        ModelTraceRefusal::HyperUnmatched { joint: 1 }
    );

    let mut altered = counterexample.clone();
    let HyperStep::Model(step) = &mut altered.traces[0].steps[1] else {
        panic!("a model step");
    };
    step.post_state = system.state_key(&system.initial()[1]).expect("a key").1;
    assert!(matches!(
        refusal(world.replay(&altered)),
        ModelTraceRefusal::PostState { step: 1, .. }
    ));

    // `a` returns to its initial state, `b` does not: the loop re-entered at 0
    // closes for `a` and not for `b`.
    let mut open = counterexample.clone();
    open.kind = HyperCounterexampleKind::Lockstep { loop_entry: 0 };
    assert!(matches!(
        refusal(world.replay(&open)),
        ModelTraceRefusal::LoopOpen { entry: 0, steps: 2 }
    ));

    let mut stutter = counterexample;
    stutter.traces[0].steps[1] = HyperStep::TerminalStutter;
    assert_eq!(
        refusal(world.replay(&stutter)),
        ModelTraceRefusal::StutterNotTerminal { position: 1 }
    );
}

/// FR-183-AC-3, last sentence: a counterexample under `fair weak
/// V::Vault::reset` on `a` whose loop never takes or disables `reset` is
/// refused as unfair. The loop is the engine's counterexample of the same
/// clause without the fairness set.
#[trace("TC-608")]
#[test]
fn a_loop_that_is_unfair_under_its_own_fairness_set_is_refused() {
    let clause = |fairness: &str| {
        format!(
            "hyper Returns using inf over behaviours (v: V::Vault) \
             forall trace a of V {fairness} {{ always eventually holds(v.l @ a = 0) }}"
        )
    };
    let unfair = World::new(Vault::Secure, true, &STARTS[..1], &clause(""));
    let counterexample = refuted(&unfair);
    let fair = World::new(
        Vault::Secure,
        true,
        &STARTS[..1],
        &clause("fair weak V::Vault::reset;"),
    );
    assert_eq!(
        refusal(fair.replay(&counterexample)),
        ModelTraceRefusal::HyperUnfair {
            variable: "a".to_owned()
        }
    );
    // Without the constraint the same loop replays.
    assert!(matches!(
        unfair.replay(&counterexample),
        Ok(ModelTraceSettlement::Reproduced { .. })
    ));
}

/// FR-183-AC-4, first sentence: a hand-built `Lockstep` counterexample over
/// the leaky vault with `a` and `b` both from `(0, 0)` taking `step(0)`, loop
/// entry 1, on which `l` stays equal, settles `inconclusive`.
#[trace("TC-608")]
#[test]
fn a_lockstep_counterexample_on_which_the_body_holds_is_inconclusive() {
    let world = leaky();
    let mut counterexample = refuted(&world);
    counterexample.traces[1] = counterexample.traces[0].clone();
    counterexample.traces[1].variable = "b".to_owned();
    assert!(matches!(
        world.replay(&counterexample),
        Ok(ModelTraceSettlement::Inconclusive(_))
    ));
}

/// FR-183 "Undefined evaluation": an `Undefined` counterexample replays to
/// the `UndefinedEvaluation` it carries, with one path per variable, and the
/// same payload with its locus changed settles `inconclusive`.
#[trace("TC-608")]
#[test]
fn an_undefined_match_counterexample_replays_to_its_value() {
    let divided = "match { a.step.op = b.step.op and \
         a.step.Vault::step.i = b.step.Vault::step.i and \
         (if present(a.step.Vault::step.i) then 1 / value(a.step.Vault::step.i) else 1) = 1 }";
    let world = World::new(Vault::Secure, false, &STARTS, &non_interference(divided));
    let counterexample = refuted(&world);
    let HyperCounterexampleKind::Undefined(claimed) = counterexample.kind.clone() else {
        panic!("expected an undefined-evaluation counterexample");
    };
    assert_eq!(
        claimed.cause,
        UndefinedCause::Kernel(Undefined::DivisionByZero)
    );
    assert_eq!(
        world.replay(&counterexample).expect("the replay settles"),
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::HyperUndefined(claimed.clone()),
            trace_position: Some(1),
        }
    );

    let mut elsewhere = counterexample;
    let HyperCounterexampleKind::Undefined(changed) = &mut elsewhere.kind else {
        panic!("expected an undefined-evaluation counterexample");
    };
    changed.locus = HyperLocus::Match { conjunct: 1 };
    assert!(matches!(
        world.replay(&elsewhere),
        Ok(ModelTraceSettlement::Inconclusive(_))
    ));
}
