// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-173-AC-6 (TC-2243): an event-position hyper clause reaches a verdict
//! through EN-1 over closed finite executions, never `FormNotDecided`, and
//! its counterexample replays as closed traces with no loop.
//!
//! The fixtures are FR-173-AC-6's: two Secure vaults `V` and `N` with the one
//! initial state `h = 1`, `l = 0`, whose `step(i)` sets `l = i` for `i` in
//! `{0, 1}` and which have no terminal state.

use filament_exact::{Cancel, CancelCause};
use fsl_replay::{
    settle_hyper_check, ExistentialOutcome, HyperCounterexampleKind, HyperOutcome, HyperStep,
    IncompleteCause, ModelCheckLimits, ModelTraceSettlement, ModelTraceValue, ResultLabel, RunEnd,
    TerminalValue,
};
use fsl_route::reductions::Reductions;
use fsl_semantics::check::{HyperForm, TemporalProfile};
use ix_trace_rs::trace;

use crate::hyper::refuted;
use crate::hyper_world::{Vault, World, KEY};

/// The Secure vault twice, `V` and `N`, started from `h = 1`, `l = 0`.
fn two_models(quantifiers: &str, body: &str) -> World {
    let declaration = format!(
        "hyper Saves using ep over behaviours (v: V::Vault, w: N::Vault) \
         {quantifiers} {{ {body} }}"
    );
    let vault = [(KEY, (1, 0))];
    World::with_second_vaults(Vault::Secure, &vault, &vault, &declaration)
}

fn run(world: &World) -> HyperOutcome {
    world
        .run_selecting(Reductions::default())
        .expect("the run is not refused")
}

fn settled(world: &World, outcome: &HyperOutcome) -> (ResultLabel, TerminalValue) {
    let record = settle_hyper_check(
        outcome,
        &ModelCheckLimits::default(),
        &[],
        None,
        &Cancel::new(),
        |counterexample| world.replay(counterexample),
    )
    .expect("the outcome settles");
    (record.label, record.value)
}

const FORALL_FORALL: &str = "forall trace a of V forall trace b of N";
const FORALL_EXISTS: &str = "forall trace a of V exists trace b of N";

/// HP-2: the two vaults' `l` differ, so the clause is refuted by one closed
/// trace of four positions per variable, neither with a loop, and it replays.
#[trace("TC-2243", "FR-173-AC-6")]
#[test]
fn an_event_position_universal_clause_is_refuted_by_closed_traces() {
    let world = two_models(FORALL_FORALL, "always [0, 3] holds(v.l @ a = w.l @ b)");
    assert_eq!(
        world.clause.profile(),
        TemporalProfile::EventPositionFalseExtension
    );
    assert_eq!(world.clause.form(), Some(&HyperForm::Universal));
    let outcome = run(&world);
    let counterexample = refuted(outcome.clone());
    assert_eq!(counterexample.kind, HyperCounterexampleKind::Closed);
    assert_eq!(counterexample.traces.len(), 2);
    for trace in &counterexample.traces {
        assert_eq!(trace.steps.len(), 3, "four positions, 0 to 3");
        assert!(trace
            .steps
            .iter()
            .all(|step| matches!(step, HyperStep::Model(_))));
    }
    let replayed = world.replay(&counterexample).expect("the replay reads it");
    assert!(
        matches!(
            replayed,
            ModelTraceSettlement::Reproduced {
                value: ModelTraceValue::Truth(false),
                ..
            }
        ),
        "{replayed:?}"
    );
    assert_eq!(settled(&world, &outcome).0, ResultLabel::Refuted);
}

/// HP-2: the two vaults' `h` never change, so the clause holds on every tuple
/// of closed executions and settles `proved`.
#[trace("TC-2243", "FR-173-AC-6")]
#[test]
fn an_event_position_universal_clause_that_holds_settles_proved() {
    let world = two_models(FORALL_FORALL, "always [0, 3] holds(v.h @ a = w.h @ b)");
    let outcome = run(&world);
    assert!(
        matches!(outcome, HyperOutcome::HoldsClosed { .. }),
        "{outcome:?}"
    );
    let (label, value) = settled(&world, &outcome);
    assert_eq!(label, ResultLabel::Proved);
    assert!(matches!(value, TerminalValue::Proved { .. }), "{value:?}");
}

/// HP-3: every universal trace has an existential partner of the same `l`, so
/// the clause settles `proved`; with `!=` no partner exists at position 0 and
/// the counterexample is a universal closed trace alone, which replays.
#[trace("TC-2243", "FR-173-AC-6")]
#[test]
fn an_event_position_forall_exists_clause_settles_by_its_witness_search() {
    let held = two_models(FORALL_EXISTS, "always [0, 3] holds(v.l @ a = w.l @ b)");
    assert_eq!(held.clause.form(), Some(&HyperForm::ForallExistsSafety));
    let outcome = run(&held);
    assert!(
        matches!(outcome, HyperOutcome::HoldsClosed { .. }),
        "{outcome:?}"
    );
    assert_eq!(settled(&held, &outcome).0, ResultLabel::Proved);

    let broken = two_models(FORALL_EXISTS, "always [0, 3] holds(v.l @ a != w.l @ b)");
    let outcome = run(&broken);
    let counterexample = refuted(outcome.clone());
    assert_eq!(counterexample.kind, HyperCounterexampleKind::Closed);
    assert_eq!(counterexample.trace_position, 0);
    assert_eq!(counterexample.traces.len(), 1, "the universal trace alone");
    assert_eq!(counterexample.traces[0].steps.len(), 3);
    assert_eq!(settled(&broken, &outcome).0, ResultLabel::Refuted);
}

/// A counterexample whose trace is not closed (it stops before the horizon at
/// a state that enables steps) is refused by replay.
#[trace("TC-2243", "FR-173-AC-6")]
#[test]
fn replay_refuses_a_trace_that_is_not_closed() {
    let world = two_models(FORALL_FORALL, "always [0, 3] holds(v.l @ a = w.l @ b)");
    let mut counterexample = refuted(run(&world));
    for trace in &mut counterexample.traces {
        trace.steps.pop();
    }
    let refusal = world
        .replay(&counterexample)
        .expect_err("a trace that stops short of its horizon is refused");
    assert!(
        format!("{refusal:?}")
            .contains("neither ends at a terminal state nor covers the body's horizon"),
        "the refusal names the closed-trace rule: {refusal:?}"
    );
}

/// FR-178-AC-1's `align skip { V::Vault::mix }` clause under the event-position
/// profile over `body`.
fn aligned(body: &str) -> String {
    format!(
        "hyper NonInterference using ep over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V align skip {{ V::Vault::mix }} \
         match {{ (if present(a.step.Vault::step.i) and present(b.step.Vault::step.i) \
         then value(a.step.Vault::step.i) = value(b.step.Vault::step.i) else true) }} \
         {{ {body} }}"
    )
}

/// HP-6: the busy vault from its idle initial states holds `always [0, 2]
/// l @ a = l @ b` on its projection; the leaky vault started mid-operation is
/// refuted by a projected counterexample that replays.
#[trace("TC-2243", "FR-173-AC-6")]
#[test]
fn an_event_position_projection_aligned_clause_reaches_a_verdict() {
    let body = "always [0, 2] holds(v.l @ a = v.l @ b)";
    let held = World::busy(Vault::Busy, &crate::hyper_projected::IDLE, &aligned(body));
    assert_eq!(held.clause.form(), Some(&HyperForm::ProjectionAligned));
    let outcome = held
        .run(ModelCheckLimits::default(), &Cancel::new())
        .expect("the run is not refused");
    assert!(
        matches!(outcome, HyperOutcome::HoldsClosed { .. }),
        "{outcome:?}"
    );
    assert_eq!(settled(&held, &outcome).0, ResultLabel::Proved);
    let leaky = World::busy(
        Vault::BusyLeaky,
        &crate::hyper_projected::MID_OPERATION,
        &aligned(body),
    );
    let outcome = leaky
        .run(ModelCheckLimits::default(), &Cancel::new())
        .expect("the run is not refused");
    let counterexample = refuted(outcome.clone());
    assert_eq!(counterexample.kind, HyperCounterexampleKind::Closed);
    assert_eq!(counterexample.traces.len(), 2);
    for trace in &counterexample.traces {
        assert!(
            trace
                .steps
                .iter()
                .all(|step| matches!(step, HyperStep::Model(_))),
            "a closed trace holds no stutter step and no loop"
        );
    }
    let replayed = leaky.replay(&counterexample).expect("the replay reads it");
    assert!(
        matches!(replayed, ModelTraceSettlement::Reproduced { .. }),
        "{replayed:?}"
    );
    assert_eq!(settled(&leaky, &outcome).0, ResultLabel::Refuted);
}

/// HP-5: `exists trace b of V { eventually [0, 2] holds(v.l @ b = 1) }` is
/// witnessed by one closed trace of three positions with no loop, and the
/// same clause over `v.h @ b = 0` (`h` stays 1) is trapped at the initial
/// state.
#[trace("TC-2243", "FR-173-AC-6")]
#[test]
fn an_event_position_single_existential_clause_is_witnessed_or_trapped() {
    let single = |body: &str| {
        let declaration = format!(
            "hyper Possible using ep over behaviours (v: V::Vault) exists trace b of V {{ {body} }}"
        );
        World::new(Vault::Secure, false, &[(1, 0)], &declaration)
    };
    let witnessed = single("eventually [0, 2] holds(v.l @ b = 1)");
    assert_eq!(witnessed.clause.form(), Some(&HyperForm::SingleExistential));
    let run = witnessed
        .run_single(ModelCheckLimits::default(), None, &Cancel::new())
        .expect("the run is not refused");
    let [instance] = run.instances.as_slice() else {
        panic!("one instance: {run:?}");
    };
    let ExistentialOutcome::ClosedWitnessed { paths } = &instance.outcome else {
        panic!("a closed witness: {instance:?}");
    };
    let [path] = paths.as_slice() else {
        panic!("one witness per initial state: {paths:?}");
    };
    assert_eq!((path.initial, path.steps.len()), (0, 2), "three positions");

    let trapped = single("eventually [0, 2] holds(v.h @ b = 0)");
    let run = trapped
        .run_single(ModelCheckLimits::default(), None, &Cancel::new())
        .expect("the run is not refused");
    let [instance] = run.instances.as_slice() else {
        panic!("one instance: {run:?}");
    };
    assert!(
        matches!(&instance.outcome, ExistentialOutcome::Trapped { stem } if stem.initial == 0 && stem.steps.is_empty()),
        "{instance:?}"
    );
}

/// A vault that steps only while `h = 1`: started from `h = 0` it is terminal
/// at once.
const STEPS_WHILE_HIGH: Vault = Vault::Custom(
    "pre Go using v on V::Vault::step { self.h = 1 }\n\
     post Step using v on V::Vault::step { self.l = i }\n",
);

/// FR-043 with FR-173 "Event-position form": a trace that reaches a terminal
/// state inside the body's horizon is closed there, and an atom past its end
/// reads false, so `always [0, 3]` is refuted by it; the counterexample's
/// trace is shorter than the horizon and replays.
#[trace("TC-2243", "FR-173-AC-6")]
#[test]
fn an_event_position_trace_that_ends_inside_the_horizon_is_closed_there() {
    let world = two_models(FORALL_FORALL, "always [0, 3] holds(v.h @ a = w.h @ b)");
    let ended = World::with_second_vaults(
        Vault::Terminal,
        &[(KEY, (1, 0))],
        &[(KEY, (1, 0))],
        "hyper Saves using ep over behaviours (v: V::Vault, w: N::Vault) \
         forall trace a of V forall trace b of N { always [0, 3] holds(v.h @ a = w.h @ b) }",
    );
    // Without a terminal state the same body holds over every full trace.
    assert!(matches!(run(&world), HyperOutcome::HoldsClosed { .. }));
    let outcome = run(&ended);
    let counterexample = refuted(outcome.clone());
    assert_eq!(counterexample.kind, HyperCounterexampleKind::Closed);
    assert!(
        counterexample
            .traces
            .iter()
            .any(|trace| trace.steps.len() < 3),
        "a trace that ends at its terminal state inside the horizon"
    );
    assert!(counterexample
        .traces
        .iter()
        .all(|trace| trace.steps.iter().all(|s| matches!(s, HyperStep::Model(_)))));
    let replayed = ended.replay(&counterexample).expect("the replay reads it");
    assert!(
        matches!(
            replayed,
            ModelTraceSettlement::Reproduced {
                value: ModelTraceValue::Truth(false),
                ..
            }
        ),
        "{replayed:?}"
    );
}

/// FR-173 "Event-position form": `match` constrains a joint step only through
/// the traces its conjunct reads. With `c` terminal at its start the conjunct
/// over `a` and `b` still holds every joint step of `a` and `b`, so the
/// clause, whose body reads `a` and `b` alone, is proved; skipping the whole
/// `match` once `c` has ended would admit steps with different inputs.
#[trace("TC-2243", "FR-173-AC-6")]
#[test]
fn a_match_conjunct_over_two_traces_still_holds_after_a_third_has_ended() {
    let declaration = "hyper Three using ep over behaviours (v: V::Vault, w: N::Vault) \
        forall trace a of V forall trace b of V forall trace c of N \
        match { a.step.Vault::step.i = b.step.Vault::step.i } \
        { always [0, 2] holds(v.l @ a = v.l @ b) }";
    let world = World::with_second_vaults(
        STEPS_WHILE_HIGH,
        &[(KEY, (1, 0))],
        &[(KEY, (0, 0))],
        declaration,
    );
    let outcome = run(&world);
    assert!(
        matches!(outcome, HyperOutcome::HoldsClosed { .. }),
        "{outcome:?}"
    );
}

/// FR-176 "Limits": a state budget the closed enumeration passes stops the
/// run `ResourceExhausted`, never a verdict; a cancelled run stops
/// `Cancelled` for both the HP-2 and the HP-3 search.
#[trace("TC-2243", "FR-173-AC-6")]
#[test]
fn an_event_position_run_that_a_limit_or_cancellation_stops_decides_nothing() {
    let universal = "hyper U using ep over behaviours (v: V::Vault) \
        forall trace a of V forall trace b of V { always [0, 3] holds(v.h @ a = v.h @ b) }";
    let partnered = "hyper P using ep over behaviours (v: V::Vault) \
        forall trace a of V exists trace b of V { always [0, 3] holds(v.l @ a = v.l @ b) }";
    for declaration in [universal, partnered] {
        let world = World::new(Vault::Secure, false, &[(1, 0)], declaration);
        let mut limits = ModelCheckLimits::default();
        limits.limits.max_states = 1;
        let outcome = world
            .run(limits, &Cancel::new())
            .expect("the run is not refused");
        let HyperOutcome::Stopped { cause, limit, .. } = outcome else {
            panic!("{declaration}: expected a stop, got {outcome:?}");
        };
        assert_eq!(cause, IncompleteCause::ResourceExhausted, "{declaration}");
        assert!(limit.is_some(), "{declaration}");

        let cancel = Cancel::new();
        cancel.cancel(CancelCause::Requested);
        let outcome = world
            .run(ModelCheckLimits::default(), &cancel)
            .expect("the run is not refused");
        let HyperOutcome::Stopped { cause, limit, .. } = outcome else {
            panic!("{declaration}: expected a stop, got {outcome:?}");
        };
        assert_eq!(
            cause,
            IncompleteCause::Cancelled {
                source: CancelCause::Requested
            },
            "{declaration}"
        );
        assert_eq!(limit, None, "{declaration}");
    }
}

/// HP-5 over closed executions: a body evaluation whose meter is exhausted
/// settles no witness, so the instance has no decision and is never `Trapped`.
#[trace("TC-2243", "FR-173-AC-6")]
#[test]
fn an_event_position_single_existential_run_that_a_limit_stops_is_not_trapped() {
    let declaration = "hyper Possible using ep over behaviours (v: V::Vault) \
        exists trace b of V { eventually [0, 2] holds(v.h @ b = 0) }";
    let world = World::new(Vault::Secure, false, &[(1, 0)], declaration);
    let run = world
        .run_single_metered(
            ModelCheckLimits::default(),
            None,
            crate::counter::ZERO,
            &Cancel::new(),
        )
        .expect("the run is not refused");
    let [instance] = run.instances.as_slice() else {
        panic!("one instance: {run:?}");
    };
    let ExistentialOutcome::NoDecision { end, .. } = &instance.outcome else {
        panic!("no decision, not a trap: {instance:?}");
    };
    assert!(
        matches!(
            end,
            RunEnd::Stopped {
                cause: IncompleteCause::ResourceExhausted,
                limit: Some(_)
            }
        ),
        "{end:?}"
    );
}

/// A universe naming no selected model's population is refused
/// `wrong-role-mapping` by `over_model` and by the alias path alike; a universe
/// of another selected model's population is refused by `over_model` and
/// skipped by the alias path, where one clause ranges over both models.
#[trace("TC-2243", "FR-173-AC-6")]
#[test]
fn a_universe_of_no_selected_model_is_refused_and_one_of_another_model_is_skipped() {
    use fsl_analyze::model_check::{alias_provisions, alias_system};
    use fsl_eval::simulation::model::{ModelSystem, PopulationUniverse};

    let world = two_models(FORALL_FORALL, "always [0, 3] holds(v.l @ a = w.l @ b)");
    let subject = world.subject();
    let own = world
        .package
        .graph()
        .model_package_identity("V")
        .expect("V names a model")
        .to_owned();
    // An alias system reads its own model's part of the snapshots.
    let alias_of = |simulation: &fsl_eval::simulation::model::ModelSimulationRequest<'_>,
                    alias: &str| {
        let provisions = alias_provisions(simulation, &world.clause);
        let slot = world
            .clause
            .aliases()
            .iter()
            .position(|candidate| candidate.as_str() == alias)
            .expect("the alias is the clause's");
        alias_system(simulation, provisions[slot].as_ref(), alias).map(|_| ())
    };
    let refused = |simulation: fsl_eval::simulation::model::ModelSimulationRequest<'_>,
                   alias: Option<&str>| {
        let failure = match alias {
            Some(alias) => alias_of(&simulation, alias),
            None => ModelSystem::over_model(simulation, &own).map(|_| ()),
        }
        .expect_err("the universe is refused");
        format!("{failure:?}")
    };

    // The alias path skips the other model's universe: both aliases admit.
    assert!(alias_of(&subject.simulation, "V").is_ok());
    assert!(alias_of(&subject.simulation, "N").is_ok());
    // `over_model` keeps the refusal for it.
    let text = refused(subject.simulation.clone(), None);
    assert!(text.contains("wrong-role-mapping"), "{text}");

    // A mis-keyed universe is refused on both paths.
    let mut mis_keyed = subject.simulation.clone();
    mis_keyed.universes.push(PopulationUniverse {
        population: crate::hyper_world::key("nowhere"),
        keys: vec![KEY.to_owned()],
    });
    for alias in [None, Some("V"), Some("N")] {
        let text = refused(mis_keyed.clone(), alias);
        assert!(text.contains("wrong-role-mapping"), "{alias:?}: {text}");
    }
}
