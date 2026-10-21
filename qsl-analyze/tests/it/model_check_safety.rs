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
//! claims (QSL-530); AC-9's TP-4 row has no fixture in this module; AC-6's
//! refused-contract row has none either: every kernel refusal a contract can
//! raise is proved away by S3, and a family refusal needs a population
//! reference (`PopulationReferenceNotEmitted`) or a dispatch family.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_model, ModelCheckRefusal, ModelCheckRequest};
use qsl_eval::temporal::UndefinedCause;
use qsl_replay::{
    CounterexampleKind, IncompleteCause, ModelCheckItem, ModelCheckLimit, ModelCheckLimits,
    ModelCheckOutcome, ReachedLimit, TemporalCounterexample, UndefinedLocus,
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
    let letters = Letters::new(&world.clauses[index], atoms);
    run_letters(world, index, keys, &letters, limits)
}

/// The run of the world's clause `index` over the universe `keys`, read
/// through `letters`.
fn run_letters(
    world: &World,
    index: usize,
    keys: &[&str],
    letters: &Letters,
    limits: ModelCheckLimits,
) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
    let clause = &world.clauses[index];
    check_model(
        ModelCheckRequest {
            subject: world.subject(keys, UNLIMITED),
            item: ModelCheckItem::Clause { clause, letters },
            max_depth: None,
            limits,
            reductions: Default::default(),
        },
        &Cancel::new(),
    )
}

fn checked(world: &World, index: usize, keys: &[&str], atoms: Vec<Atom>) -> ModelCheckOutcome {
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
    let world = counter_world(
        BOUNDED,
        &[
            Clause {
                activation: &counter,
                formula: "eventually[0,3] holds(c.value = 3)",
            },
            Clause {
                activation: &counter,
                formula: "eventually[0,1] holds(c.value = 3)",
            },
        ],
    );
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
#[test]
fn a_terminal_state_closes_a_bounded_monitor_with_false_letters() {
    let world = counter_world(
        BOUNDED,
        &[
            Clause {
                activation: ORIGIN,
                formula: "eventually[0,5] holds(c.value > 3)",
            },
            Clause {
                activation: ORIGIN,
                formula: "eventually[0,5] holds(c.value = 3)",
            },
        ],
    );
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
#[test]
fn a_terminal_state_stutters_under_infinite_trace() {
    let world = counter_world(
        INFINITE,
        &[
            Clause {
                activation: ORIGIN,
                formula: "always (holds(c.value = 3) implies eventually[0,1] holds(c.value = 0))",
            },
            Clause {
                activation: ORIGIN,
                formula: "always (holds(c.value = 2) implies eventually[0,1] holds(c.value = 3))",
            },
        ],
    );
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
    let world = counter_world(
        INFINITE,
        &[Clause {
            activation: ORIGIN,
            formula: "always (holds(c.value <= 1) or eventually[0,2] false)",
        }],
    );
    let outcome = checked(&world, 0, &["c"], vec![value(|v| Some(v <= 1))]);
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(
        values(&world, &["c"], counterexample, "c", "value"),
        [0, 1, 2]
    );
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
    let outcome = run(
        &world,
        0,
        &["s"],
        vec![healthy(true), healthy(false)],
        tight,
    )
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

/// The counterexample's undefined evaluation and the counter values on its
/// prefix.
fn undefined(world: &World, outcome: &ModelCheckOutcome) -> (u64, Vec<i64>) {
    let counterexample = violated(outcome);
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("expected an undefined evaluation: {counterexample:?}");
    };
    assert_eq!(
        evaluation.cause,
        UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero)
    );
    (
        evaluation.site.position,
        values(world, &["c"], counterexample, "c", "value"),
    )
}

/// ADR-018 UE-1: a claim reads every atom of the claim at every position it
/// reads, also once its monitor no longer depends on the atom. Over the
/// Counter subject under infinite-trace, `holds(c.value >= 0) release
/// holds(6 mod (2 - c.value) >= 0)` is discharged at position 0, yet its
/// second atom is still read at position 2, where it is undefined: the run is
/// violated with `UndefinedEvaluation` at position 2, prefix `0 -inc-> 1
/// -inc-> 2`.
#[test]
fn a_discharged_infinite_trace_claim_still_reads_every_atom() {
    let world = counter_world(
        INFINITE,
        &[Clause {
            activation: ORIGIN,
            formula: "holds(c.value >= 0) release holds(6 mod (2 - c.value) >= 0)",
        }],
    );
    let outcome = checked(
        &world,
        0,
        &["c"],
        vec![
            value(|v| Some(v >= 0)),
            value(|v| (v != 2).then(|| 6 % (2 - v) >= 0)),
        ],
    );
    assert_eq!(undefined(&world, &outcome), (2, vec![0, 1, 2]));
}

/// ADR-018 UE-1 for TP-2 `on origin`: the claim reads positions 0 to its
/// horizon `h`. Over the Counter subject under event-position
/// false-extension, `holds(c.value = 1) implies eventually[0,3] holds(6 mod
/// (2 - c.value) = 0)` is decided true at position 0, whose antecedent is
/// false, yet position 2 is within the horizon 3 and reads the consequent's
/// atom undefined there: violated with `UndefinedEvaluation` at position 2.
#[test]
fn a_decided_bounded_claim_still_reads_every_atom_up_to_its_horizon() {
    let world = counter_world(
        BOUNDED,
        &[Clause {
            activation: ORIGIN,
            formula: "holds(c.value = 1) implies eventually[0,3] holds(6 mod (2 - c.value) = 0)",
        }],
    );
    let outcome = checked(
        &world,
        0,
        &["c"],
        vec![
            value(|v| Some(v == 1)),
            value(|v| (v != 2).then(|| 6 % (2 - v) == 0)),
        ],
    );
    assert_eq!(undefined(&world, &outcome), (2, vec![0, 1, 2]));
}

/// A TP-4 clause is decided by the liveness engine (here its proof waits for
/// FR-339's component certificate), but the liveness engine does not read an
/// `on each` activation's `when` condition, so a guarded TP-4 clause is
/// refused before any state is explored (FR-126 "Unsupported liveness
/// clauses").
#[test]
fn a_liveness_clause_is_decided_and_a_guarded_activation_is_refused() {
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
        matches!(
            refusal,
            ModelCheckRefusal::Liveness(
                qsl_analyze::model_check::LivenessRefusal::ComponentCertificateNotBuilt { .. }
            )
        ),
        "{refusal:?}"
    );
    let refusal = refused(INFINITE, &guarded, "always eventually holds(c.value = 3)");
    assert!(
        matches!(refusal, ModelCheckRefusal::GuardedActivation),
        "{refusal:?}"
    );
}

/// FR-126-AC-6's empty-prefix row: from an initial state whose `pre`
/// evaluates undefined, `always holds(true)` is violated by the empty prefix
/// at that state, with `kind: UndefinedEvaluation` at position 0. Over the
/// Counter subject with `pre CanInc { 6 mod (2 - self.value) >= 0 }` from
/// value 2, the modulus is undefined (`DivisionByZero`) in the initial
/// state's own expansion. FR-120-AC-9's `test/tallies` subject, whose
/// `sum<Low>` would be undefined (`SumOutOfDomain`), is not constructible:
/// QSpec FR-145's range proof refuses at S3 a `sum<N>` it cannot prove within
/// `N`, and `SumOutOfDomain` is raised only by a `sum` checked under
/// `CheckMode::Kernel`, so no linked package reaches it.
#[trace("TC-521", "FR-126-AC-6")]
#[test]
fn an_undefined_precondition_at_the_initial_state_is_an_empty_prefix_violation() {
    let world = World::new(
        model_world::counter(),
        "pre CanInc using v on M::Counter::inc { 6 mod (2 - self.value) >= 0 }\n\
         post Inc using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n",
        INFINITE,
        &[Clause {
            activation: ORIGIN,
            formula: "always holds(true)",
        }],
        &[("c", json!({"value": {"integer": "2"}}))],
    );
    let outcome = checked(&world, 0, &["c"], vec![Box::new(|_, _| Some(true))]);
    let counterexample = violated(&outcome);
    assert!(counterexample.prefix.is_empty(), "{counterexample:?}");
    assert_eq!(counterexample.lasso, None);
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("expected an undefined evaluation: {counterexample:?}");
    };
    assert_eq!(evaluation.site.position, 0);
    assert_eq!(
        evaluation.cause,
        UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero)
    );
    assert_eq!(values(&world, &["c"], counterexample, "c", "value"), [2]);
}

/// A guarded world: the Counter subject with the one `on each` clause
/// `activation` over `profile` and `formula`.
fn guarded_world(profile: &str, formula: &str) -> World {
    let activation = format!("{} when (true)", each("Counter"));
    counter_world(
        profile,
        &[Clause {
            activation: &activation,
            formula,
        }],
    )
}

/// The run of the guarded world's one clause, its `holds` atoms read by
/// `atoms` and its `when` condition by `guard`.
fn guarded_run(
    world: &World,
    atoms: Vec<Atom>,
    guard: fn(i64) -> Option<bool>,
) -> ModelCheckOutcome {
    let letters = Letters::new(&world.clauses[0], atoms).guarded(value(guard));
    run_letters(world, 0, &["c"], &letters, ModelCheckLimits::default()).expect("checked")
}

/// FR-126-AC-16 for TP-2: a guarded `on each` clause activates at the
/// positions where its `when` condition is true. Over the Counter subject,
/// `eventually[0,1] holds(c.value = 3)` is violated unguarded by the prefix
/// `0 -inc-> 1`; guarded by `c.value = 1` it activates at position 1 alone
/// and is violated by the prefix `0 -inc-> 1 -inc-> 2`; guarded by `c.value
/// >= 2` it activates at values 2 and 3, each of which reaches value 3 within
/// its window, and holds.
#[trace("TC-521", "FR-126-AC-16")]
#[test]
fn a_guarded_bounded_clause_activates_where_its_condition_holds() {
    let world = guarded_world(BOUNDED, "eventually[0,1] holds(c.value = 3)");
    let outcome = guarded_run(&world, vec![value(|v| Some(v == 3))], |v| Some(v == 1));
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(
        values(&world, &["c"], counterexample, "c", "value"),
        [0, 1, 2]
    );
    let outcome = guarded_run(&world, vec![value(|v| Some(v == 3))], |v| Some(v >= 2));
    holds(&outcome);
}

/// FR-126-AC-16 for TP-1: `always holds(c.value <= 1)` `on each`, guarded by
/// `c.value >= 3`, is violated at the guarded position 3 by the prefix `0
/// -inc-> 1 -inc-> 2 -inc-> 3`, where the unguarded clause is violated at
/// value 2; guarded by `c.value = 7`, a condition no position meets, it
/// holds. A guarded proof carries no closure certificate.
#[trace("TC-521", "FR-126-AC-16")]
#[test]
fn a_guarded_invariant_clause_is_read_from_its_activating_positions() {
    let activation = format!("{} when (true)", each("Counter"));
    let world = counter_world(
        INFINITE,
        &[Clause {
            activation: &activation,
            formula: "always holds(c.value <= 1)",
        }],
    );
    let outcome = guarded_run(&world, vec![value(|v| Some(v <= 1))], |v| Some(v >= 3));
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(
        values(&world, &["c"], counterexample, "c", "value"),
        [0, 1, 2, 3]
    );
    let outcome = guarded_run(&world, vec![value(|v| Some(v <= 1))], |v| Some(v == 7));
    let ModelCheckOutcome::Holds { certificate, .. } = &outcome else {
        panic!("expected Holds, got {outcome:?}");
    };
    assert!(certificate.states.is_empty(), "{certificate:?}");
}

/// FR-126-AC-16: a `when` condition with no value at a position the monitor
/// reads ends the run with `UndefinedEvaluation` located at the activation,
/// not read as false. `eventually[0,3] holds(c.value = 3)` `on each`, its
/// condition undefined at value 2, is violated at position 2 with the prefix
/// `0 -inc-> 1 -inc-> 2`.
#[trace("TC-521", "FR-126-AC-16")]
#[test]
fn an_undefined_activation_condition_is_an_undefined_evaluation() {
    let world = guarded_world(BOUNDED, "eventually[0,3] holds(c.value = 3)");
    let outcome = guarded_run(&world, vec![value(|v| Some(v == 3))], |v| {
        (v != 2).then_some(true)
    });
    let counterexample = violated(&outcome);
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("expected an undefined evaluation: {counterexample:?}");
    };
    assert_eq!(evaluation.site.locus, UndefinedLocus::Activation);
    assert_eq!(evaluation.site.position, 2);
    assert_eq!(
        evaluation.cause,
        UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero)
    );
    assert_eq!(
        values(&world, &["c"], counterexample, "c", "value"),
        [0, 1, 2]
    );
}
