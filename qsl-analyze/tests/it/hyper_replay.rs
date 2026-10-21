// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-183 (TC-608): the replay of a hyper counterexample through each trace
//! variable's `ModelSystem`, on the vault of `crate::hyper_world`.
//!
//! The counterexamples are the engine's own (FR-176, FR-177) or built from
//! their traces by changing one thing, and each refusal is the one thing
//! changed. The facade entry `replay_model_trace_tuple`, the `Projected` and
//! `StepTuple` arms and the recompile that resolves the clause are not built
//! (S4 does not yet emit a hyper clause), so FR-183-AC-2, AC-3 and AC-5 are
//! not claimed.

use ix_trace_rs::trace;
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::temporal::UndefinedCause;
use qsl_replay::{
    HyperCounterexample, HyperCounterexampleKind, HyperLocus, HyperOutcome, HyperStep,
    ModelCheckLimit, ModelCheckLimits, ModelStep, ModelTraceRefusal, ModelTraceSettlement,
    ModelTraceValue, ReachedLimit, ReplayRefusal,
};
use quire_exact::{Cancel, Undefined};

use crate::hyper_world::{non_interference, opaque, Vault, World, EQUAL_INPUTS, STARTS};

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

/// ADR-023 section 8.1's leaky `Lockstep` counterexample replays to
/// `reproduced-with-evaluated-witness` with `trace_position` 1, and replaying
/// one counterexample twice gives equal results.
#[trace("TC-608", "FR-183-AC-1")]
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

/// FR-183 criterion 3, the refusals that settle no result (not claimed whole): a counterexample with one
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

/// FR-183 criterion 3, last sentence (not claimed whole): a counterexample under `fair weak
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

/// A hand-built `Lockstep` counterexample over the leaky vault with `a` and
/// `b` both from `(0, 0)` taking `step(0)`, loop entry 1, on which `l` stays
/// equal, settles `inconclusive`.
#[trace("TC-608", "FR-183-AC-4")]
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

/// ADR-023 section 8.2's `Opaque` over `vault`.
fn opaque_over(vault: Vault) -> World {
    World::new(vault, false, &STARTS, &opaque(""))
}

/// The leaky `Opaque` counterexample: `a`'s lasso `(0, 0) -step(0)-> (0, 0)`
/// with loop entry 0 and position 1.
fn exhausted() -> HyperCounterexample {
    refuted(&opaque_over(Vault::Leaky))
}

/// ADR-023 section 8.2's leaky counterexample recomputes the witness sets
/// along `a`'s lasso, `X_0` holding the partner with the other secret and
/// `X_1` empty, and reproduces at position 1. The sets are the engine's own
/// product, which the FR-177 tests read; the leaky `Lockstep` test above
/// covers the other counterexample the criterion names.
#[trace("TC-608", "FR-183-AC-1")]
#[test]
fn the_leaky_witness_exhausted_counterexample_replays_to_its_empty_set() {
    assert_eq!(
        opaque_over(Vault::Leaky)
            .replay(&exhausted())
            .expect("the replay settles"),
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            trace_position: Some(1),
        }
    );
}

/// The hand-built `WitnessExhausted` envelopes: the leaky counterexample
/// replayed over the secure vault, whose `X_1` is not empty, settles
/// `inconclusive`; replayed with `max_witness_set` 0 it returns the replay
/// stopped with that limit and its value, and with 1 it reproduces; one
/// envelope replayed twice gives equal results. A claim that `X_0` is empty,
/// and one that `X_2` is the first empty set when `X_1` is, each settle
/// `inconclusive`.
#[trace("TC-608", "FR-183-AC-4")]
#[test]
fn a_witness_envelope_whose_sets_disagree_is_inconclusive_and_a_small_budget_stops_it() {
    let counterexample = exhausted();
    let secure = opaque_over(Vault::Secure);
    let nonempty = secure.replay(&counterexample).expect("the replay settles");
    assert!(
        matches!(nonempty, ModelTraceSettlement::Inconclusive(_)),
        "{nonempty:?}"
    );
    assert_eq!(
        secure.replay(&counterexample).expect("the replay settles"),
        nonempty
    );

    let leaky = opaque_over(Vault::Leaky);
    let stopped = leaky
        .replay_under(&counterexample, 0)
        .expect("the replay settles");
    assert_eq!(
        stopped,
        ModelTraceSettlement::Stopped(ReachedLimit {
            limit: ModelCheckLimit::MaxWitnessSet,
            value: 0
        })
    );
    assert_eq!(
        leaky
            .replay_under(&counterexample, 0)
            .expect("the replay settles"),
        stopped
    );
    assert!(matches!(
        leaky.replay_under(&counterexample, 1),
        Ok(ModelTraceSettlement::Reproduced { .. })
    ));

    let at = |position: u64, extra_step: bool| {
        let mut changed = counterexample.clone();
        if extra_step {
            let again = changed.traces[0].steps[0].clone();
            changed.traces[0].steps.push(again);
        }
        changed.kind = HyperCounterexampleKind::WitnessExhausted {
            loop_entry: 0,
            position,
        };
        changed.trace_position = position;
        leaky.replay(&changed)
    };
    assert!(matches!(
        at(0, false),
        Ok(ModelTraceSettlement::Inconclusive(_))
    ));
    assert!(matches!(
        at(2, true),
        Ok(ModelTraceSettlement::Inconclusive(_))
    ));
}

/// A `WitnessExhausted` counterexample that breaks a reader rule is refused
/// with no result: a trace for the existential variable, a trace position
/// other than its position, a position past its lasso, a loop entry outside
/// its steps, and a clause that is not forall-exists. The universal traces are
/// checked as a lasso is, so a loop that does not close, a joint step `μ_U`
/// rejects and an unfair loop are refused as they are for `Lockstep`.
#[trace("TC-608")]
#[test]
fn a_witness_exhausted_counterexample_that_breaks_a_replay_rule_is_refused() {
    let world = opaque_over(Vault::Leaky);
    let counterexample = exhausted();
    let shape = |changed: &HyperCounterexample| {
        assert!(
            matches!(
                refusal(world.replay(changed)),
                ModelTraceRefusal::HyperShape { .. }
            ),
            "{changed:?}"
        );
    };

    let mut with_partner = counterexample.clone();
    let mut partner = with_partner.traces[0].clone();
    partner.variable = "b".to_owned();
    with_partner.traces.push(partner);
    shape(&with_partner);

    let mut moved = counterexample.clone();
    moved.trace_position = 0;
    shape(&moved);

    let mut past = counterexample.clone();
    past.kind = HyperCounterexampleKind::WitnessExhausted {
        loop_entry: 0,
        position: 2,
    };
    past.trace_position = 2;
    shape(&past);

    let mut outside = counterexample.clone();
    outside.kind = HyperCounterexampleKind::WitnessExhausted {
        loop_entry: 1,
        position: 1,
    };
    shape(&outside);

    // `step(0)` then `step(1)` from (0, 0) ends at (0, 1), not where it began.
    let system = ModelSystem::new(world.subject().simulation).expect("the subject admits");
    let initial = system.initial()[0].clone();
    let (first, after) = take(&world, &initial, 0);
    let (second, _) = take(&world, &after, 1);
    let mut open = counterexample.clone();
    open.traces[0].steps = vec![HyperStep::Model(first), HyperStep::Model(second)];
    open.kind = HyperCounterexampleKind::WitnessExhausted {
        loop_entry: 0,
        position: 1,
    };
    assert!(matches!(
        refusal(world.replay(&open)),
        ModelTraceRefusal::LoopOpen { entry: 0, steps: 2 }
    ));

    let lockstep = leaky();
    let mut universal = counterexample;
    universal.traces.push(universal.traces[0].clone());
    shape_of(&lockstep, &universal);
}

/// The refusal of `world` replaying `changed` is a hyper shape one.
fn shape_of(world: &World, changed: &HyperCounterexample) {
    assert!(matches!(
        refusal(world.replay(changed)),
        ModelTraceRefusal::HyperShape { .. }
    ));
}
