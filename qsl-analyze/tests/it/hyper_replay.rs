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
    settle_hyper_check, DisagreementCause, HyperCounterexample, HyperCounterexampleKind,
    HyperLocus, HyperOutcome, HyperStep, InconclusiveCause, ModelCheckLimit, ModelCheckLimits,
    ModelStep, ModelTraceRefusal, ModelTraceSettlement, ModelTraceValue, ReachedLimit,
    ReplayRefusal, RunStats, TerminalValue,
};
use quire_exact::{Cancel, Incomplete, Meter, Undefined};

use crate::hyper_witness::{members, ExhaustedExistential, GUARDED, GUARDED_CLAUSE};
use crate::hyper_world::{high, low, non_interference, opaque, Vault, World, EQUAL_INPUTS, STARTS};

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
        Ok(ModelTraceSettlement::Inconclusive(
            DisagreementCause::Verdicts { .. }
        ))
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
/// `X_1` empty, and reproduces at position 1. The replay recomputes the sets
/// with the engine's witness product and does not return them, so the test
/// reads `X_0` of the counterexample's initial state off the same product and
/// asserts the replay reproduces at the position after it; the leaky
/// `Lockstep` test above covers the other counterexample the criterion
/// names.
#[trace("TC-608", "FR-183-AC-1")]
#[test]
fn the_leaky_witness_exhausted_counterexample_replays_to_its_empty_set() {
    let world = opaque_over(Vault::Leaky);
    let counterexample = exhausted();
    let explored = world.explore_witness();
    let start = explored
        .graph
        .initial()
        .get(counterexample.traces[0].initial as usize)
        .and_then(|id| explored.graph.state(*id))
        .expect("the trace starts at an initial product state");
    assert_eq!(
        (
            high(&start.state.components[0]),
            low(&start.state.components[0])
        ),
        (0, 0)
    );
    assert_eq!(members(&start.state), [(1, 0)]);
    assert_eq!(
        world.replay(&counterexample).expect("the replay settles"),
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
        matches!(
            nonempty,
            ModelTraceSettlement::Inconclusive(DisagreementCause::Verdicts { .. })
        ),
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
        ModelTraceSettlement::Stopped {
            reached: ReachedLimit {
                limit: ModelCheckLimit::MaxWitnessSet,
                value: 0
            },
            count: Some(1),
        }
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
/// other than its position, a loop entry outside its steps, and a clause that is not forall-exists. The universal traces are
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

    // A clause with two universal variables is not forall-exists. The
    // envelope carries the clause's own variables `a` and `b`, so only its
    // form is wrong.
    let lockstep = leaky();
    let mut universal = refuted(&lockstep);
    universal.kind = HyperCounterexampleKind::WitnessExhausted {
        loop_entry: 1,
        position: 1,
    };
    shape_of(&lockstep, &universal);
}

/// The refusal of `world` replaying `changed` is a hyper shape one.
fn shape_of(world: &World, changed: &HyperCounterexample) {
    assert!(matches!(
        refusal(world.replay(changed)),
        ModelTraceRefusal::HyperShape { .. }
    ));
}

/// ADR-023 section 8.2's `Opaque` over a vault whose `l` is the last input, run
/// from the starts `(0, 0)` and `(1, 1)` with a body that reads the past: the
/// engine rotates the universal lasso to its canonical form, so its
/// `WitnessExhausted` counterexample can have fewer steps than its joint
/// position.
const ROTATED: &str = "hyper Rotated using inf over behaviours (v: V::Vault) \
     forall trace a of V exists trace b of V \
     match { a.step.Vault::step.i = b.step.Vault::step.i } \
     { holds(v.h @ a != v.h @ b) and always (once[2,2] holds(v.l @ b = 1) \
     implies holds(v.l @ b = 1)) }";

fn rotated() -> World {
    World::new(
        Vault::Custom("post Step using v on V::Vault::step { self.l = i }\n"),
        false,
        &[(0, 0), (1, 1)],
        ROTATED,
    )
}

/// FR-183 "WitnessExhausted": the replay reads joint positions past the last
/// step around the loop, so the engine's own counterexample, whose position
/// the canonical rotation leaves past its steps, replays to its position.
#[trace("TC-608", "FR-183-AC-1")]
#[test]
fn a_witness_exhausted_position_past_a_rotated_lasso_reads_around_the_loop() {
    let world = rotated();
    let counterexample = refuted(&world);
    let HyperCounterexampleKind::WitnessExhausted {
        loop_entry,
        position,
    } = counterexample.kind
    else {
        panic!("expected a witness-exhausted counterexample");
    };
    assert_eq!((loop_entry, position), (0, 2));
    assert_eq!(counterexample.steps(), 1);
    assert_eq!(
        world.replay(&counterexample).expect("the replay settles"),
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            trace_position: Some(2),
        }
    );
}

fn no_value_settlement(result: Result<ModelTraceSettlement, ReplayRefusal>) {
    assert!(
        matches!(
            result,
            Ok(ModelTraceSettlement::Inconclusive(
                DisagreementCause::NoValue { .. }
            ))
        ),
        "{result:?}"
    );
}

/// FR-183 "WitnessExhausted": a set emptied by a member the replay could not
/// keep is not an empty set. §8.2's leaky counterexample replayed over the
/// secure vault, with a `match` whose existential conjunct exhausts its
/// meter, has every member refused, so `X_1` is empty; the engine over those
/// inputs is `Undecided(MatchUndetermined)`, and the replay completes no value
/// and the item settles `inconclusive`, never `refuted`.
#[trace("TC-608", "FR-183-AC-4")]
#[test]
fn a_witness_set_emptied_by_a_refused_match_reproduces_nothing() {
    let secure = opaque_over(Vault::Secure);
    let counterexample = exhausted();
    let replayed = secure.replay_through(
        &counterexample,
        &secure.seams(),
        &ExhaustedExistential(secure.seams()),
        65_536,
    );
    no_value_settlement(replayed);

    let record = settle_hyper_check(
        &HyperOutcome::Violated {
            counterexample: Box::new(counterexample),
            stats: RunStats::default(),
        },
        &ModelCheckLimits::default(),
        &[],
        |counterexample| {
            secure.replay_through(
                counterexample,
                &secure.seams(),
                &ExhaustedExistential(secure.seams()),
                65_536,
            )
        },
    )
    .expect("settles");
    assert!(matches!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(
            DisagreementCause::NoValue { .. }
        ))
    ));
    assert_eq!(record.counterexample, None);
}

/// A set emptied by an existential `μ_E` with no value is the same: §8.2's
/// leaky counterexample replayed over the leaky vault with `b`'s input divided
/// into, whose engine run is `Undefined`, not an exhausted set.
#[trace("TC-608", "FR-183-AC-4")]
#[test]
fn a_witness_set_emptied_by_an_undefined_match_reproduces_nothing() {
    let divided = " and (if present(b.step.Vault::step.i) then 1 / value(b.step.Vault::step.i) \
                   else 1) = 1";
    let world = World::new(Vault::Leaky, false, &STARTS, &opaque(divided));
    no_value_settlement(world.replay(&exhausted()));
}

/// The same for a member whose contract has no value: the partner from
/// `(0, 0)` is dropped where `step(1)` leaves it at `(0, 1)`, a state in
/// which the guard divides by zero. `a` runs `(1, 0) -step(0)-> (1, 1)` and
/// loops there; the replay reads the guard's finding when it expands the
/// product state at joint position 1.
#[trace("TC-608", "FR-183-AC-4")]
#[test]
fn a_witness_set_that_lost_a_member_to_an_undefined_contract_reproduces_nothing() {
    let world = World::new(GUARDED, false, &STARTS, GUARDED_CLAUSE);
    let system = ModelSystem::new(world.subject().simulation).expect("the subject admits");
    let initial = system.initial()[1].clone();
    let (first, after) = take(&world, &initial, 0);
    let (second, _) = take(&world, &after, 0);
    let mut counterexample = exhausted();
    counterexample.traces[0].initial = 1;
    counterexample.traces[0].steps = vec![HyperStep::Model(first), HyperStep::Model(second)];
    counterexample.kind = HyperCounterexampleKind::WitnessExhausted {
        loop_entry: 1,
        position: 2,
    };
    counterexample.trace_position = 2;
    no_value_settlement(world.replay(&counterexample));
}

/// A `μ_U` that asserts it is read the universal variables' steps alone.
struct UniversalOnly<'w>(crate::hyper_world::Seams<'w>);

impl qsl_eval::hyper::StepMatch for UniversalOnly<'_> {
    fn universal(
        &self,
        instance: &[qsl_eval::hyper::ParameterBinding],
        steps: &[qsl_eval::hyper::StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<qsl_eval::hyper::MatchReading, Incomplete> {
        assert_eq!(steps.len(), 1, "μ_U reads the universal variable `a` alone");
        self.0.universal(instance, steps, meter)
    }

    fn existential(
        &self,
        instance: &[qsl_eval::hyper::ParameterBinding],
        steps: &[qsl_eval::hyper::StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<qsl_eval::hyper::MatchReading, Incomplete> {
        assert_eq!(steps.len(), 2, "μ_E reads both variables");
        self.0.existential(instance, steps, meter)
    }
}

/// The `Opaque` whose `match` has a universal conjunct, `a` takes only
/// `step(0)`, and an existential one that divides by `b`'s input.
fn divided_over_a_universal_match() -> World {
    let divided = " and (if present(a.step.Vault::step.i) then value(a.step.Vault::step.i) else 0) = 0 \
                   and (if present(b.step.Vault::step.i) then 1 / value(b.step.Vault::step.i) else 1) = 1";
    World::new(Vault::Secure, false, &STARTS, &opaque(divided))
}

/// FR-183 "Undefined evaluation" for an HP-3 clause: `μ_U` is read over the
/// universal variables' steps and `μ_E` over every variable's, and a step `μ_E`
/// rejects is not one the existential path took, so it settles `inconclusive`
/// with the replay's `Verdicts`. The payload is FR-177-AC-5's with `b`'s
/// first step changed to `step(1)`, which breaks the first conjunct.
#[trace("TC-608", "FR-183-AC-6")]
#[test]
fn an_undefined_witness_counterexample_reads_each_match_over_its_own_variables() {
    let world = divided_over_a_universal_match();
    let counterexample = refuted(&world);
    assert!(matches!(
        counterexample.kind,
        HyperCounterexampleKind::Undefined(_)
    ));
    let seams = UniversalOnly(world.seams());
    let replayed = world
        .replay_through(&counterexample, &world.seams(), &seams, 65_536)
        .expect("the replay settles");
    assert!(
        matches!(replayed, ModelTraceSettlement::Reproduced { .. }),
        "{replayed:?}"
    );

    let system = ModelSystem::new(world.subject().simulation).expect("the subject admits");
    let initial = system.initial()[counterexample.traces[1].initial as usize].clone();
    let (other, _) = take(&world, &initial, 1);
    let mut rejected = counterexample;
    rejected.traces[1].steps = vec![HyperStep::Model(other)];
    let replayed = world
        .replay_through(&rejected, &world.seams(), &seams, 65_536)
        .expect("the replay settles");
    assert!(
        matches!(
            replayed,
            ModelTraceSettlement::Inconclusive(DisagreementCause::Verdicts { .. })
        ),
        "{replayed:?}"
    );
}

/// A vault whose `step` leaves `l` either 0 or 1.
const CHOOSING: Vault =
    Vault::Custom("post Step using v on V::Vault::step { self.l = 0 or self.l = 1 }\n");

/// FR-183 "WitnessExhausted": a step is replayed to the post-state its trace
/// records, not to any successor of its transition. `step` leaves `l` either
/// 0 or 1 and the body needs `l = 0` on `a`, so the set empties only along a
/// path that takes the post-state with `l = 1`; the replay follows the
/// engine's counterexample to it and reproduces.
#[trace("TC-608", "FR-183-AC-1")]
#[test]
fn a_witness_replay_follows_the_post_state_a_nondeterministic_step_took() {
    let clause = "hyper Chosen using inf over behaviours (v: V::Vault) \
         forall trace a of V exists trace b of V \
         match { a.step.Vault::step.i = b.step.Vault::step.i } \
         { holds(v.h @ a != v.h @ b) and always holds(v.l @ a = 0) }";
    let world = World::new(CHOOSING, false, &STARTS, clause);
    let counterexample = refuted(&world);
    let HyperCounterexampleKind::WitnessExhausted { .. } = counterexample.kind else {
        panic!("expected a witness-exhausted counterexample");
    };
    assert_eq!(
        world.replay(&counterexample).expect("the replay settles"),
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            trace_position: Some(counterexample.trace_position),
        }
    );
    // The first post-state `a`'s first step has is not the one it took.
    let system = ModelSystem::new(world.subject().simulation).expect("the subject admits");
    let initial = system.initial()[counterexample.traces[0].initial as usize].clone();
    let HyperStep::Model(first) = &counterexample.traces[0].steps[0] else {
        panic!("a model step");
    };
    let posts: Vec<_> = system
        .successors(&initial)
        .expect("the vault expands")
        .successors
        .into_iter()
        .filter(|(transition, _)| *transition == first.transition)
        .map(|(_, state)| system.state_key(&state).expect("a key").1)
        .collect();
    assert_eq!(posts.len(), 2, "a step with two post-states");
    assert_ne!(posts[0], first.post_state, "the second one is taken");
}

/// Further counterexamples the engine produces with a position past their
/// steps, one that goes around a two-step loop from its first step and one
/// that goes around a one-step loop re-entered at step 1: the replay reads
/// the position through the loop, whatever its entry and length.
#[trace("TC-608", "FR-183-AC-1")]
#[test]
fn a_position_past_a_longer_or_late_loop_reads_the_state_it_goes_around_to() {
    let toggling = Vault::Custom(
        "post Step using v on V::Vault::step { (pre(self.l) = 0 and self.l = 1) or (pre(self.l) = 1 and self.l = 0) }\n",
    );
    let fresh = Vault::Custom(
        "pre Fresh using v on V::Vault::step { self.l = 0 }\npost Step using v on V::Vault::step { self.l = i }\n",
    );
    let cases = [
        (
            toggling,
            &[(0, 1), (1, 0)][..],
            "hyper P using inf over behaviours (v: V::Vault) \
             forall trace a of V exists trace b of V \
             { holds(v.h @ a != v.h @ b) and always (once[3,3] holds(v.l @ b = 1) \
             implies holds(v.l @ b = 1)) }",
            (0, 2, 3),
        ),
        (
            fresh,
            &[(0, 0)][..],
            "hyper P using inf over behaviours (v: V::Vault) \
             forall trace a of V exists trace b of V match { a.step.op = b.step.op } \
             { always (once[3,3] holds(v.l @ b = 1) implies holds(v.l @ a = 0)) }",
            (1, 2, 4),
        ),
    ];
    for (vault, starts, clause, (entry, steps, position)) in cases {
        let world = World::new(vault, false, starts, clause);
        let counterexample = refuted(&world);
        assert_eq!(
            (
                counterexample.kind.clone(),
                counterexample.steps(),
                counterexample.trace_position
            ),
            (
                HyperCounterexampleKind::WitnessExhausted {
                    loop_entry: entry,
                    position
                },
                steps,
                position
            ),
            "{clause}"
        );
        assert_eq!(
            world.replay(&counterexample).expect("the replay settles"),
            ModelTraceSettlement::Reproduced {
                value: ModelTraceValue::Truth(false),
                trace_position: Some(position),
            },
            "{clause}"
        );
        // One position earlier or later is a different claim: the set is not
        // first empty there.
        for claimed in [position - 1, position + 1] {
            let mut moved = counterexample.clone();
            moved.kind = HyperCounterexampleKind::WitnessExhausted {
                loop_entry: entry,
                position: claimed,
            };
            moved.trace_position = claimed;
            assert!(
                matches!(
                    world.replay(&moved),
                    Ok(ModelTraceSettlement::Inconclusive(
                        DisagreementCause::Verdicts { .. }
                    ))
                ),
                "{clause} at {claimed}"
            );
        }
    }
}

/// A letter with no value where `b`'s `l` is 1, read as the vault world reads
/// every other.
struct UndefinedWhereBIsHigh<'w>(crate::hyper_world::Seams<'w>);

impl qsl_eval::hyper::HyperLetters for UndefinedWhereBIsHigh<'_> {
    fn atom(
        &self,
        atom: qsl_semantics::check::FormulaNodeId,
        instance: &[qsl_eval::hyper::ParameterBinding],
        states: &[&ModelSimState],
        meter: &mut Meter,
    ) -> Result<qsl_eval::temporal::AtomReading, Incomplete> {
        if low(states[1]) == 1 {
            return Ok(qsl_eval::temporal::AtomReading::Undefined(
                UndefinedCause::Kernel(Undefined::DivisionByZero),
            ));
        }
        self.0.atom(atom, instance, states, meter)
    }
}

/// FR-183 "Undefined evaluation": a step `μ_E` rejects is not one the
/// existential path took, so the path is not the engine's even when a later
/// expression on it has no value. `b` takes `step(1)` against `a`'s `step(0)`,
/// which the `match` rejects, into a state where an atom has no value and the
/// payload names that atom: without the rejection the replay would
/// reproduce it, with it the replay settles `inconclusive`, `Verdicts`.
#[trace("TC-608", "FR-183-AC-6")]
#[test]
fn an_undefined_witness_path_through_a_rejected_existential_step_is_not_reproduced() {
    let world = World::new(Vault::Secure, false, &STARTS, &opaque(""));
    let system = ModelSystem::new(world.subject().simulation).expect("the subject admits");
    let (a_step, _) = take(&world, &system.initial()[0], 0);
    let (b_step, _) = take(&world, &system.initial()[1], 1);
    let atom = qsl_eval::automaton::translate_negation(
        world.clause.body(),
        world.clause.profile(),
        u64::MAX,
        None,
    )
    .expect("the body translates")
    .atoms()[0];
    let mut counterexample = exhausted();
    counterexample.traces.push(counterexample.traces[0].clone());
    counterexample.traces[1].variable = "b".to_owned();
    counterexample.traces[1].initial = 1;
    counterexample.traces[0].steps = vec![HyperStep::Model(a_step.clone())];
    counterexample.traces[1].steps = vec![HyperStep::Model(b_step.clone())];
    counterexample.trace_position = 1;
    counterexample.kind = HyperCounterexampleKind::Undefined(qsl_replay::HyperUndefined {
        position: 1,
        state_keys: vec![a_step.post_state, b_step.post_state],
        variables: vec![
            ("a".to_owned(), "V".to_owned()),
            ("b".to_owned(), "V".to_owned()),
        ],
        locus: HyperLocus::Atom(atom),
        transition: None,
        cause: UndefinedCause::Kernel(Undefined::DivisionByZero),
    });
    let letters = UndefinedWhereBIsHigh(world.seams());
    let replayed = world
        .replay_through(&counterexample, &letters, &world.seams(), 65_536)
        .expect("the replay settles");
    assert!(
        matches!(
            replayed,
            ModelTraceSettlement::Inconclusive(DisagreementCause::Verdicts { .. })
        ),
        "{replayed:?}"
    );
}

/// A contract the replay's own universal variable cannot evaluate: `reset`'s
/// guard divides by `h`, which is 0 where `a` starts and 1 for its partner, so
/// the expansion of the product state at joint position 0 has a finding for
/// `a`'s component and none for a member. The
/// counterexample is the leaky one with `reset` in the vault, which the
/// engine would not return from such a run.
#[trace("TC-608", "FR-183-AC-4")]
#[test]
fn a_witness_set_computed_past_a_universal_contract_with_no_value_reproduces_nothing() {
    let world = World::new(
        Vault::Custom(
            "post Step using v on V::Vault::step { (i = self.h and self.l = 0) or (i != self.h and self.l = 1) }\n\
             pre Guard using v on V::Vault::reset { 1 div self.h >= 0 }\n\
             post Reset using v on V::Vault::reset { self.l = 0 }\n",
        ),
        true,
        &STARTS,
        &opaque(""),
    );
    let system = ModelSystem::new(world.subject().simulation).expect("the subject admits");
    let (step, _) = take(&world, &system.initial()[0], 0);
    let mut counterexample = exhausted();
    counterexample.traces[0].steps = vec![HyperStep::Model(step)];
    no_value_settlement(world.replay(&counterexample));
}
