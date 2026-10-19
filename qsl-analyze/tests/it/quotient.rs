// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-021 AQ-1 to AQ-7 (FR-153, FR-161's `each` loop): liveness under
//! symmetry, decided on the annotated quotient, end to end through
//! `check_model`, over ADR-021 §7.1's subject (`crate::reduction_world`,
//! three `Counter` objects whose `bump` advances `v` modulo 3) and
//! FR-153-AC-3's `Toggle` subject (`crate::toggle_world`).
//!
//! The cycle groups, witness sets and the strong refinement are asserted
//! on the quotients themselves by `model_check::quotient`'s unit tests;
//! here the verdicts, product counts, concretised loops and limits are.
//! Every counterexample is read back by re-executing its steps through
//! `ModelSystem`.
//!
//! The unreduced comparison FR-153-AC-4 asks for is made where the
//! unreduced engine decides the item: a refutation is `Violated` both
//! ways, and the unreduced proof is refused only for want of FR-339's
//! component certificate after exploring the whole product; a strong
//! constraint is refused unreduced (`LivenessRefusal::StrongFairness`), so
//! AC-3's strong run has no unreduced verdict to compare, and AC-4 stays
//! untagged. FR-161-AC-3's replay waits on FR-128's replay of an authored
//! clause, so the concretised `each` loop is checked by re-execution only.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{
    LivenessRefusal, ModelCheckRefusal, Reductions, SymmetryDeclaration,
};
use qsl_eval::simulation::model::ModelSimState;
use qsl_replay::{
    AppliedReduction, IncompleteCause, ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome,
    ReachedLimit, TemporalCounterexample,
};
use quire_exact::ScalarLimits;

use crate::counter::UNLIMITED;
use crate::reduction::CYCLE;
use crate::reduction_world::{
    atom, counters, start, value, values, Clause, Shape, World, INFINITE,
};
use crate::toggle_world::{flag, toggles, World as Toggles, TOGGLES};

const KEYS: [&str; 3] = ["a", "b", "c"];

/// §7.1's annotated subject, all versions 0, with one clause `formula`.
fn configs(formula: &str) -> World {
    World::new(
        Shape {
            symmetric: true,
            max: 2,
            link: false,
        },
        CYCLE,
        &[Clause {
            profile: INFINITE,
            formula,
        }],
        &[start([0, 0, 0])],
        &KEYS,
    )
}

fn keys(keys: &[&str]) -> Vec<String> {
    keys.iter().map(|key| (*key).to_owned()).collect()
}

/// The full class `[a, b, c]` of `counters`.
fn symmetric() -> Reductions {
    Reductions {
        symmetry: vec![SymmetryDeclaration {
            population: counters(),
            classes: vec![keys(&KEYS)],
        }],
        ..Reductions::default()
    }
}

type Outcome = Result<ModelCheckOutcome, ModelCheckRefusal>;

/// The run of `world`'s clause, its one atom read by `test`.
fn run(world: &World, test: fn(i64) -> Option<bool>, reductions: Reductions) -> Outcome {
    world.run(
        Some(0),
        vec![atom(test)],
        reductions,
        None,
        ModelCheckLimits::default(),
    )
}

fn violation(outcome: &Outcome) -> &TemporalCounterexample {
    match outcome {
        Ok(ModelCheckOutcome::Violated { counterexample, .. }) => counterexample,
        other => panic!("expected a violation, got {other:?}"),
    }
}

/// The states of a lasso's loop, the entry first and last.
fn looped<'s>(
    counterexample: &TemporalCounterexample,
    states: &'s [ModelSimState],
) -> &'s [ModelSimState] {
    let lasso = counterexample.lasso.expect("a lasso");
    &states[lasso.entry..]
}

/// The key each step of a counter loop moves.
fn moved(states: &[ModelSimState]) -> Vec<&'static str> {
    states
        .windows(2)
        .map(|pair| {
            KEYS.into_iter()
                .find(|key| value(&pair[0], key) != value(&pair[1], key))
                .expect("a step changes its receiver")
        })
        .collect()
}

const PROOF: &str = "fair weak each M::Counter::bump; always eventually holds(x.v = 2)";
const REFUTATION: &str = "fair weak each M::Counter::bump; eventually always holds(x.v != 2)";

/// FR-153-AC-1: `always eventually holds(c.versionNumber = 2)` under `fair
/// weak each attemptUpdate`, reduced by `[[a, b, c]]`, runs one instance
/// (`c = a`, orbit `[a, b, c]`, stabiliser `[b, c]`) and finds no fair
/// accepting component on the annotated quotient: `HoldsReduced` over 30
/// product states. Repeating the run gives the equal outcome. Unreduced, the
/// engine explores all 135 product states of the three instances and finds
/// no fair accepting cycle either.
#[trace("TC-570", "FR-153-AC-1")]
#[test]
fn an_each_proof_on_the_annotated_quotient_holds_over_thirty_product_states() {
    let world = configs(PROOF);
    let outcome = run(&world, |v| Some(v == 2), symmetric());
    let Ok(ModelCheckOutcome::HoldsReduced { reductions, stats }) = &outcome else {
        panic!("expected a reduced proof, got {outcome:?}");
    };
    assert_eq!(stats.product_states, 30);
    let [AppliedReduction::Symmetry {
        orbit, stabiliser, ..
    }] = reductions.as_slice()
    else {
        panic!("one symmetry record: {reductions:?}");
    };
    assert_eq!(*orbit, keys(&KEYS));
    assert_eq!(stabiliser.len(), 1);
    assert_eq!(stabiliser[0].keys, keys(&["b", "c"]));
    assert_eq!(
        outcome.as_ref().ok(),
        run(&world, |v| Some(v == 2), symmetric()).as_ref().ok()
    );
    match run(&world, |v| Some(v == 2), Reductions::default()) {
        Err(ModelCheckRefusal::Liveness(LivenessRefusal::ComponentCertificateNotBuilt {
            stats,
        })) => assert_eq!(stats.product_states, 135),
        other => panic!("expected the unreduced run to find no fair cycle: {other:?}"),
    }
}

/// FR-153-AC-2, with FR-161's `each` loop (ADR-021 AQ-7): `eventually
/// always holds(c.versionNumber != 2)` under `fair weak each attemptUpdate`
/// is refuted on the annotated quotient, bound to `c = a`. The concretised
/// lasso has an empty stem and a loop from `(0,0,0)` back to `(0,0,0)` that
/// updates `a`, `b` and `c` and passes `a = 2`; it is a path of the
/// subject, and repeating the run gives the equal counterexample. The
/// unreduced run refutes the item too.
#[trace("TC-570", "FR-153-AC-2")]
#[test]
fn an_each_refutation_concretises_to_a_loop_through_every_receiver() {
    let world = configs(REFUTATION);
    let outcome = run(&world, |v| Some(v != 2), symmetric());
    let counterexample = violation(&outcome);
    assert_eq!(
        counterexample
            .over_binding
            .as_ref()
            .map(|binding| binding.key.as_str()),
        Some("a")
    );
    assert_eq!(counterexample.lasso.map(|lasso| lasso.entry), Some(0));
    let states = world.positions(counterexample);
    let cycle = looped(counterexample, &states);
    assert_eq!(values(&cycle[0]), (0, 0, 0));
    assert_eq!(values(&cycle[cycle.len() - 1]), (0, 0, 0));
    let receivers = moved(cycle);
    for key in KEYS {
        assert!(
            receivers.contains(&key),
            "{key} is updated in {receivers:?}"
        );
    }
    assert!(cycle.iter().any(|state| value(state, "a") == 2));
    assert_eq!(
        outcome.as_ref().ok(),
        run(&world, |v| Some(v != 2), symmetric()).as_ref().ok()
    );
    violation(&run(&world, |v| Some(v != 2), Reductions::default()));
}

/// The full class `[a, b]` of `toggles`.
fn toggled() -> Reductions {
    Reductions {
        symmetry: vec![SymmetryDeclaration {
            population: toggles(),
            classes: vec![keys(&TOGGLES)],
        }],
        ..Reductions::default()
    }
}

/// AC-3's claim, `eventually` every toggle fired. A temporal atom cannot
/// name a population (S3 refuses `M::toggles` there with `missing-name`),
/// so the atom is spelled over the bound watcher and read by the test's
/// letters as "every toggle has fired".
const EVENTUALLY_FIRED: &str = "eventually holds(w = w)";

/// FR-153-AC-3: over the `Toggle` subject under its full group, `fair weak
/// each flip` and `fair weak each fire` refute the claim with a loop that
/// flips `a` and `b` and never fires; each `fire` identity is disabled
/// somewhere in it, so weak `each` is met. Under `fair weak each flip` and
/// `fair strong each fire` the claim holds on the annotated quotient: the
/// component of that loop enables both `fire` identities and takes
/// neither, and its refinement leaves no fair accepting cycle (asserted on
/// the quotient by `quotient`'s unit test). The weak run is refuted
/// unreduced too.
#[trace("TC-571", "FR-153-AC-3")]
#[test]
fn strong_each_holds_where_weak_each_admits_a_loop_that_never_fires() {
    let weak = Toggles::new(
        "fair weak each M::Toggle::flip; fair weak each M::Toggle::fire;",
        EVENTUALLY_FIRED,
    );
    let outcome = weak.run(toggled());
    let counterexample = violation(&outcome);
    let states = weak.positions(counterexample);
    let cycle = looped(counterexample, &states);
    assert!(cycle.len() > 1, "a loop with steps");
    for toggle in TOGGLES {
        assert!(
            cycle.iter().all(|state| !flag(state, toggle, "fired")),
            "{toggle} never fires"
        );
        assert!(
            cycle
                .windows(2)
                .any(|pair| flag(&pair[0], toggle, "on") != flag(&pair[1], toggle, "on")),
            "{toggle} is flipped"
        );
        assert!(
            cycle.iter().any(|state| !flag(state, toggle, "on")),
            "fire({toggle}) is disabled in the loop"
        );
    }
    violation(&weak.run(Reductions::default()));
    let strong = Toggles::new(
        "fair weak each M::Toggle::flip; fair strong each M::Toggle::fire;",
        EVENTUALLY_FIRED,
    );
    let outcome = strong.run(toggled());
    assert!(
        matches!(outcome, Ok(ModelCheckOutcome::HoldsReduced { .. })),
        "{outcome:?}"
    );
    assert_eq!(outcome.as_ref().ok(), strong.run(toggled()).as_ref().ok());
}

/// FR-153 "Limits": with the run's meter large enough for every expansion
/// but below the cost of AC-2's cycle groups and orbit closures, the run
/// explores the whole quotient product (18 states) and then stops
/// `Stopped(ResourceExhausted, …)` naming the evaluation meter.
#[test]
fn the_annotated_quotient_charges_the_run_meter() {
    let mut world = configs(REFUTATION);
    world.meter = ScalarLimits {
        work_units: 100,
        ..UNLIMITED
    };
    match run(&world, |v| Some(v != 2), symmetric()) {
        Ok(ModelCheckOutcome::Stopped {
            cause: IncompleteCause::ResourceExhausted,
            limit:
                Some(ReachedLimit {
                    limit: ModelCheckLimit::EvaluationMeter,
                    value: 100,
                }),
            stats,
        }) => assert_eq!(stats.product_states, 18),
        other => panic!("expected the meter to stop the run: {other:?}"),
    }
}

/// FR-161 "Length and limits": a `max_transitions` that admits the reduced
/// product's edges but not the concretised loop stops the run naming it.
#[test]
fn concretising_the_loop_counts_against_max_transitions() {
    let world = configs(REFUTATION);
    let explored = match run(&world, |v| Some(v != 2), symmetric()) {
        Ok(outcome) => outcome.stats().product_edges,
        Err(refusal) => panic!("the run refutes: {refusal}"),
    };
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_transitions = usize::try_from(explored).expect("small") + 1;
    match world.run(
        Some(0),
        vec![atom(|v| Some(v != 2))],
        symmetric(),
        None,
        limits,
    ) {
        Ok(ModelCheckOutcome::Stopped {
            limit:
                Some(ReachedLimit {
                    limit: ModelCheckLimit::MaxTransitions,
                    ..
                }),
            ..
        }) => {}
        other => panic!("expected max_transitions to stop the run: {other:?}"),
    }
}
