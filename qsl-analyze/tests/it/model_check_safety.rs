// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126 (ADR-018 EN-1, TP-2 and TP-3): EN-1 decides bounded MLTL and
//! infinite-trace safety clauses on the product of a model subject with the
//! clause's lazy bad-prefix monitor, over the subjects of `crate::model_world`.
//!
//! Every expected path is computed from the subject's own arithmetic and
//! read back by re-executing the counterexample's steps. A clause's `holds`
//! atoms are read through the test's own letters (S3 checks no `holds`
//! condition yet, as `crate::model_check` notes), each atom by a predicate
//! that computes the condition its source text spells.
//!
//! Not covered here: FR-126-AC-1, AC-3 to AC-5, AC-7 and AC-8 rest on TP-4
//! claims (QSL-530); AC-6's refused-contract and `test/tallies` rows and
//! AC-9's TP-4 row have no fixture in this module.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_model, ModelCheckRefusal, ModelCheckRequest};
use qsl_replay::{
    CounterexampleKind, IncompleteCause, ModelCheckItem, ModelCheckLimit, ModelCheckLimits,
    ModelCheckOutcome, ReachedLimit, TemporalCounterexample,
};
use quire_exact::Cancel;
use serde_json::json;

use crate::counter::{CONTRACT, UNLIMITED};
use crate::model_world::{self, Atom, Clause, Letters, World, BOUNDED, INFINITE};

const ORIGIN: &str = "on origin";

/// FR-126-AC-4's `Restless` unit: `fail` has no precondition.
const RESTLESS: &str = "post Failed using v on M::Health::fail { \
                        not self.healthy and self.failures = 1 }\n\
                        pre CanRecover using v on M::Health::recover { not self.healthy }\n\
                        post Recovered using v on M::Health::recover { self.healthy }\n";

fn each(world: &str) -> String {
    format!("on each (q: M::{world})")
}

/// The run of the world's clause `index` over the universe `keys`.
fn run(
    world: &World,
    index: usize,
    keys: &[&str],
    atoms: Vec<Atom>,
    limits: ModelCheckLimits,
) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
    let clause = &world.clauses[index];
    let letters = Letters::new(
        clause,
        keys.iter().map(|key| world.reference(key)).collect(),
        atoms,
    );
    check_model(
        ModelCheckRequest {
            subject: world.subject(keys, UNLIMITED),
            item: ModelCheckItem::Clause {
                clause,
                letters: &letters,
            },
            max_depth: None,
            limits,
        },
        &Cancel::new(),
    )
}

fn checked(
    world: &World,
    index: usize,
    keys: &[&str],
    atoms: Vec<Atom>,
) -> ModelCheckOutcome {
    run(world, index, keys, atoms, ModelCheckLimits::default()).expect("checked")
}

fn violated(outcome: &ModelCheckOutcome) -> &TemporalCounterexample {
    match outcome {
        ModelCheckOutcome::Violated { counterexample, .. } => counterexample,
        other => panic!("expected a violation, got {other:?}"),
    }
}

fn holds(outcome: &ModelCheckOutcome) -> u64 {
    match outcome {
        ModelCheckOutcome::Holds { stats, .. } => stats.product_states,
        other => panic!("expected Holds, got {other:?}"),
    }
}

/// The integer field `field` of `object` at each position of
/// `counterexample`.
fn values(
    world: &World,
    keys: &[&str],
    counterexample: &TemporalCounterexample,
    object: &str,
    field: &str,
) -> Vec<i64> {
    world
        .positions(keys, counterexample)
        .iter()
        .map(|state| world.domain.integer(state, object, field))
        .collect()
}

fn counter_world(profile: &str, clauses: &[Clause<'_>]) -> World {
    World::new(
        model_world::counter(),
        CONTRACT,
        profile,
        clauses,
        &[("c", json!({"value": {"integer": "0"}}))],
    )
}

/// `c.value` read by `test`; `None` is undefined.
fn value(test: fn(i64) -> Option<bool>) -> Atom {
    let domain = model_world::counter();
    Box::new(move |state, object| test(domain.integer(state, object, "value")))
}

/// The `Counter` rows of FR-126-AC-2: over the Counter subject with no
/// `terminal` declaration, under event-position false-extension, `on each`,
/// `eventually[0,3] holds(c.value = 3)` holds and `eventually[0,1]
/// holds(c.value = 3)` is violated with the prefix `0 -inc-> 1`. Its rows
/// over ADR-018 §6's subject are not verified: FR-120's frame grants
/// `attemptUpdate` the `versionNumber` of every object, so one expansion of
/// that subject enumerates 1001 values for each of `a` and `b` and stops at
/// the subject's `max_candidates`, rather than the §6 example's two
/// successors per state.
#[trace("TC-521")]
#[test]
fn bounded_each_clauses_are_decided_on_the_product() {
    let counter = each("Counter");
    let world = counter_world(BOUNDED, &[
        Clause {
            activation: &counter,
            formula: "eventually[0,3] holds(c.value = 3)",
        },
        Clause {
            activation: &counter,
            formula: "eventually[0,1] holds(c.value = 3)",
        },
    ]);
    holds(&checked(&world, 0, &["c"], vec![value(|v| Some(v == 3))]));
    let outcome = checked(&world, 1, &["c"], vec![value(|v| Some(v == 3))]);
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(values(&world, &["c"], counterexample, "c", "value"), [0, 1]);
}

/// FR-126 "Under a bounded profile": at a terminal model state a TP-2
/// monitor reads the rest of its horizon with every atom false. Over the
/// Counter subject, whose chain ends at value 3 after three steps,
/// `eventually[0,5] holds(c.value > 3)` `on origin` is violated by that
/// closure at the terminal state, with the prefix `0 -inc-> 1 -inc-> 2
/// -inc-> 3`, while `eventually[0,5] holds(c.value = 3)` holds.
#[trace("TC-521")]
#[test]
fn a_terminal_state_closes_a_bounded_monitor_with_false_letters() {
    let world = counter_world(BOUNDED, &[
        Clause {
            activation: ORIGIN,
            formula: "eventually[0,5] holds(c.value > 3)",
        },
        Clause {
            activation: ORIGIN,
            formula: "eventually[0,5] holds(c.value = 3)",
        },
    ]);
    let outcome = checked(&world, 0, &["c"], vec![value(|v| Some(v > 3))]);
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(
        values(&world, &["c"], counterexample, "c", "value"),
        [0, 1, 2, 3]
    );
    holds(&checked(&world, 1, &["c"], vec![value(|v| Some(v == 3))]));
}

/// FR-126 "Under infinite-trace, a terminal model state SHALL contribute one
/// terminal stutter edge": over the Counter subject, `always (holds(c.value =
/// 3) implies eventually[0,1] holds(c.value = 0))` (TP-3) is violated on the
/// stutter after value 3, whose behaviour is the prefix `0 -inc-> 1 -inc-> 2
/// -inc-> 3` stuttered forever; `always (holds(c.value = 2) implies
/// eventually[0,1] holds(c.value = 3))` holds.
#[trace("TC-521")]
#[test]
fn a_terminal_state_stutters_under_infinite_trace() {
    let world = counter_world(INFINITE, &[
        Clause {
            activation: ORIGIN,
            formula: "always (holds(c.value = 3) implies eventually[0,1] holds(c.value = 0))",
        },
        Clause {
            activation: ORIGIN,
            formula: "always (holds(c.value = 2) implies eventually[0,1] holds(c.value = 3))",
        },
    ]);
    let outcome = checked(
        &world,
        0,
        &["c"],
        vec![value(|v| Some(v == 3)), value(|v| Some(v == 0))],
    );
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(
        values(&world, &["c"], counterexample, "c", "value"),
        [0, 1, 2, 3]
    );
    let outcome = checked(
        &world,
        1,
        &["c"],
        vec![value(|v| Some(v == 2)), value(|v| Some(v == 3))],
    );
    assert_eq!(holds(&outcome), 4);
}

/// ADR-018 SM-6 on the product: the monitor keeps only states with an
/// infinite continuation, so it rejects at the first bad prefix. Over the
/// Counter subject, `always (holds(c.value <= 1) or eventually[0,2] false)`
/// is `always holds(c.value <= 1)`; it is violated at value 2, with the
/// prefix `0 -inc-> 1 -inc-> 2`, although the dead `eventually[0,2] false`
/// branch would only expire two positions later.
#[test]
fn the_monitor_rejects_at_the_first_bad_prefix() {
    let world = counter_world(INFINITE, &[Clause {
        activation: ORIGIN,
        formula: "always (holds(c.value <= 1) or eventually[0,2] false)",
    }]);
    let outcome = checked(&world, 0, &["c"], vec![value(|v| Some(v <= 1))]);
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(values(&world, &["c"], counterexample, "c", "value"), [0, 1, 2]);
}

/// The FR-126-AC-6 rows over the `Restless` subject (FR-126-AC-4: `fail`
/// can repeat forever): `always (holds(not c.healthy) implies
/// eventually[0,100] holds(c.healthy))` with `max_automaton_states` 50
/// stops `Stopped{ResourceExhausted, {MaxAutomatonStates, 50}}` with 50
/// automaton states and no counterexample; with the default limit it is
/// violated by a finite prefix with at least 101 consecutive unhealthy
/// positions.
#[trace("TC-521")]
#[test]
fn a_long_interval_materializes_automaton_states_as_the_product_reaches_them() {
    let world = World::new(
        model_world::health(),
        RESTLESS,
        INFINITE,
        &[Clause {
            activation: ORIGIN,
            formula: "always (holds(not c.healthy) implies eventually[0,100] holds(c.healthy))",
        }],
        &[(
            "s",
            json!({"healthy": {"boolean": true}, "failures": {"integer": "0"}}),
        )],
    );
    let healthy = |negated: bool| -> Atom {
        let domain = model_world::health();
        Box::new(move |state, object| Some(domain.boolean(state, object, "healthy") != negated))
    };
    let tight = ModelCheckLimits {
        max_automaton_states: 50,
        ..ModelCheckLimits::default()
    };
    let outcome = run(&world, 0, &["s"], vec![healthy(true), healthy(false)], tight)
        .expect("checked");
    let ModelCheckOutcome::Stopped {
        cause,
        limit,
        stats,
    } = outcome
    else {
        panic!("expected a stopped run, got {outcome:?}");
    };
    assert_eq!(cause, IncompleteCause::ResourceExhausted);
    assert_eq!(
        limit,
        Some(ReachedLimit {
            limit: ModelCheckLimit::MaxAutomatonStates,
            value: 50
        })
    );
    assert_eq!(stats.automaton_states, 50);

    let outcome = checked(&world, 0, &["s"], vec![healthy(true), healthy(false)]);
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    let mut run = 0;
    let mut longest = 0;
    for state in world.positions(&["s"], counterexample) {
        run = if world.domain.boolean(&state, "s", "healthy") {
            0
        } else {
            run + 1
        };
        longest = longest.max(run);
    }
    assert!(longest >= 101, "{longest} consecutive unhealthy positions");
}

/// The TP-3 and TP-2 rows of FR-126-AC-9 over the Counter subject, the
/// division written `mod` as in `crate::model_check` (S3 types `mod` over
/// integers): `always (holds(c.value <= 0) and holds(6 mod (2 - c.value) >=
/// 0))` is violated with `kind: Formula` and the prefix `0 -inc-> 1`, before
/// the atom is undefined at position 2; `eventually[0,1] holds(6 mod (2 -
/// c.value) = 0 and c.value = 1)` under event-position false-extension, `on
/// origin`, false at position 0 and true at 1, holds, reading positions 0
/// and 1 only, so the undefined atom at position 2 is never evaluated.
#[trace("TC-537")]
#[test]
fn a_formula_violation_before_an_undefined_position_and_an_unread_position() {
    let world = counter_world(
        INFINITE,
        &[Clause {
            activation: ORIGIN,
            formula: "always (holds(c.value <= 0) and holds(6 mod (2 - c.value) >= 0))",
        }],
    );
    let outcome = checked(
        &world,
        0,
        &["c"],
        vec![
            value(|v| Some(v <= 0)),
            value(|v| (v != 2).then(|| 6 % (2 - v) >= 0)),
        ],
    );
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(values(&world, &["c"], counterexample, "c", "value"), [0, 1]);

    let world = counter_world(
        BOUNDED,
        &[Clause {
            activation: ORIGIN,
            formula: "eventually[0,1] holds(6 mod (2 - c.value) = 0 and c.value = 1)",
        }],
    );
    let outcome = checked(
        &world,
        0,
        &["c"],
        vec![value(|v| (v != 2).then(|| 6 % (2 - v) == 0 && v == 1))],
    );
    holds(&outcome);
}

/// A TP-4 clause is not decided by the first phase and an `on each`
/// activation with a `when` condition has no letter seam; both are refused
/// before any state is explored.
#[test]
fn a_liveness_clause_and_a_guarded_activation_are_refused() {
    let guarded = format!("{} when (true)", each("Counter"));
    let refused = |profile, activation: &str, formula| {
        let world = counter_world(
            profile,
            &[Clause {
                activation,
                formula,
            }],
        );
        run(
            &world,
            0,
            &["c"],
            vec![value(|v| Some(v == 3))],
            ModelCheckLimits::default(),
        )
        .expect_err("refused before exploring")
    };
    let refusal = refused(INFINITE, ORIGIN, "always eventually holds(c.value = 3)");
    assert!(
        matches!(refusal, ModelCheckRefusal::FormNotDecided { .. }),
        "{refusal:?}"
    );
    let refusal = refused(BOUNDED, &guarded, "eventually[0,1] holds(c.value = 3)");
    assert!(
        matches!(refusal, ModelCheckRefusal::GuardedActivation),
        "{refusal:?}"
    );
}
