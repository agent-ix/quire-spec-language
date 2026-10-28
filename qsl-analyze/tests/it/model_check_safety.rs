// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126 (ADR-018 EN-1, TP-2 and TP-3): EN-1 decides bounded MLTL and
//! infinite-trace safety clauses on the product of a model subject with the
//! clause's lazy bad-prefix monitor, over the subjects of `crate::model_world`.
//!
//! Every expected path is computed from the subject's own arithmetic and
//! read back by re-executing the counterexample's steps. A clause's `holds`
//! atoms are read through the test's own letters, each atom by a predicate
//! that computes the condition its source text spells. A guarded `on each`
//! clause's `when` condition is read the same way: the source writes the
//! placeholder `when (true)` and the guard's reading is the test's own
//! (FR-126-AC-16), given to the engine directly.
//!
//! Not covered here: FR-126-AC-1, AC-3 to AC-5, AC-7 and AC-8 rest on TP-4
//! claims (QSL-530); AC-9's TP-4 row has no fixture in this module. AC-6's
//! refused-contract row has none in this crate: every kernel refusal a
//! contract can raise is proved away by S3, a family refusal needs a
//! dispatch-family fixture whose `family_steps` or `ancestor_steps` ceiling refuses a dispatched
//! call, and the dispatch fixtures live in `qsl-eval`'s tests (Remaining
//! work: QSL-898). Replay's reading of a guarded condition
//! (`ReplayedLetters::activation`) is untested here: replay recompiles the
//! unit and reads the guard through the production letters.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_model, Atoms, ModelCheckRefusal, ModelCheckRequest, Por};
use qsl_eval::automaton::translate;
use qsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters};
use qsl_eval::simulation::model::ModelSimState;
use qsl_eval::temporal::AtomReading;
use qsl_eval::temporal::UndefinedCause;
use qsl_replay::{
    CounterexampleKind, IncompleteCause, ModelCheckItem, ModelCheckLimit, ModelCheckLimits,
    ModelCheckOutcome, ReachedLimit, TemporalCounterexample, UndefinedLocus,
};
use qsl_semantics::check::FormulaNodeId;
use quire_exact::{Cancel, Charge, ChargePoint, Incomplete, LimitKind, Meter};
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

/// FR-125-AC-18 for the `Counter` unit through its production letters: the
/// guard `q.value >= 1` of `on each (q: Counter)` is read from the state at
/// each position, its footprint is the `Counter.value` read through the
/// receiver (the trigger aliases the clause's `over`), and a run under it
/// violates only where the guard holds while `when (false)` holds vacuously.
#[trace("TC-520", "FR-125-AC-18")]
#[test]
fn the_production_letters_read_the_counter_guard_and_its_footprint() {
    use qsl_eval::simulation::model::ModelSystem;
    use qsl_semantics::check::footprint::{FootprintObject, SymbolicLocation};

    let run = |guard: &str| {
        let activation = format!("{} when ({guard})", each("Counter"));
        let world = counter_world(
            INFINITE,
            &[Clause {
                activation: &activation,
                formula: "always holds(c.value <= 2)",
            }],
        );
        let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation)
            .expect("the subject admits");
        let node = &world.package.graph().temporal_clauses()[0];
        let letters = system.clause_letters(node).expect("the clause has letters");
        let instance = ModelInstance {
            population: world.domain.key(world.domain.population),
            key: "c".to_owned(),
        };
        let start = system.initial_states()[0].clone();
        let reading = letters
            .activation(&instance, &start, &mut Meter::new(UNLIMITED))
            .expect("the guard reads");
        let reads = letters
            .activation_reads()
            .expect("the guard has a footprint");
        let outcome = check_model(
            ModelCheckRequest {
                subject: world.subject(&["c"], UNLIMITED),
                item: ModelCheckItem::Clause {
                    clause: node.clause(),
                    letters: &letters,
                },
                max_depth: None,
                limits: ModelCheckLimits::default(),
                reductions: Default::default(),
            },
            &Cancel::new(),
        )
        .expect("checked");
        (reading, reads, outcome, world.domain.key("Counter/value"))
    };
    let (reading, reads, outcome, value_key) = run("q.value >= 1");
    // The initial value is 0, so the guard is false there.
    assert_eq!(reading, AtomReading::False);
    assert_eq!(
        reads.into_iter().collect::<Vec<_>>(),
        [SymbolicLocation::Field {
            object: FootprintObject::Receiver,
            field: value_key,
        }]
    );
    assert_eq!(violated(&outcome).kind, CounterexampleKind::Formula);

    let (reading, reads, outcome, _) = run("false");
    assert_eq!(reading, AtomReading::False);
    assert!(reads.is_empty());
    holds(&outcome);
}

/// FR-125-AC-18: each guarded row is compiled from source and decided through
/// the production ClauseLetters, and POR visibility from the guard-only
/// footprint.
#[trace("TC-520", "FR-125-AC-18")]
#[test]
fn production_letters_decide_each_guard_row_and_include_guard_only_por_visibility() {
    use qsl_eval::simulation::model::ModelSystem;
    use qsl_eval::simulation::TransitionSystem;
    use qsl_semantics::check::footprint::{FootprintObject, SymbolicLocation};

    let run = |profile: &str, guard: &str, formula: &str| {
        let activation = format!("{} when ({guard})", each("Counter"));
        let world = counter_world(
            profile,
            &[Clause {
                activation: &activation,
                formula,
            }],
        );
        let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation)
            .expect("the subject admits");
        let node = &world.package.graph().temporal_clauses()[0];
        let letters = system
            .clause_letters(node)
            .expect("the production letters build");
        let outcome = check_model(
            ModelCheckRequest {
                subject: world.subject(&["c"], UNLIMITED),
                item: ModelCheckItem::Clause {
                    clause: node.clause(),
                    letters: &letters,
                },
                max_depth: None,
                limits: ModelCheckLimits::default(),
                reductions: Default::default(),
            },
            &Cancel::new(),
        )
        .expect("checked");
        (world, outcome)
    };

    let (event_world, event) = run(BOUNDED, "c.value = 1", "eventually[0,1] holds(c.value = 3)");
    let event = violated(&event);
    assert_eq!(values(&event_world, &["c"], event, "c", "value"), [0, 1, 2]);

    let (_, exhaustive) = run(
        BOUNDED,
        "c.value >= 2",
        "eventually[0,1] holds(c.value = 3)",
    );
    assert!(matches!(exhaustive, ModelCheckOutcome::Holds { .. }));

    let (live_world, live) = run(INFINITE, "c.value >= 3", "always holds(c.value <= 1)");
    let live = violated(&live);
    assert_eq!(
        values(&live_world, &["c"], live, "c", "value"),
        [0, 1, 2, 3]
    );

    let (undefined_world, undefined_outcome) =
        run(INFINITE, "6 mod (2 - c.value) = 0", "always holds(true)");
    assert_eq!(
        undefined(&undefined_world, &undefined_outcome),
        (2, vec![0, 1, 2])
    );

    // The atom has no reads; the activation guard alone reads Counter.value.
    let activation = format!("{} when (c.value >= 3)", each("Counter"));
    let por_world = counter_world(
        INFINITE,
        &[Clause {
            activation: &activation,
            formula: "always holds(true)",
        }],
    );
    let system = ModelSystem::new(por_world.subject(&["c"], UNLIMITED).simulation)
        .expect("the subject admits");
    let node = &por_world.package.graph().temporal_clauses()[0];
    let letters = system
        .clause_letters(node)
        .expect("the production letters build");
    let automaton = translate(node.clause(), 1 << 20).expect("the clause translates");
    let instance = ModelInstance {
        population: por_world.domain.key(por_world.domain.population),
        key: "c".to_owned(),
    };
    let por = Por::new(
        &system,
        Some(Atoms {
            letters: &letters,
            nodes: automaton.atoms(),
            guarded: true,
        }),
        std::slice::from_ref(&instance),
        node.clause().fairness(),
        None,
    )
    .expect("the production footprints derive");
    let start = system.initial_states()[0].clone();
    let inc = system
        .successors(&start)
        .expect("the initial state expands")
        .successors
        .into_iter()
        .find(|(_, state)| por_world.domain.integer(state, "c", "value") == 1)
        .expect("inc is enabled")
        .0;
    assert!(por
        .visible(&start, &inc)
        .expect("the visibility is derived"));
    assert!(letters
        .activation_reads()
        .expect("the guard has a footprint")
        .contains(&SymbolicLocation::Field {
            object: FootprintObject::Receiver,
            field: por_world.domain.key("Counter/value"),
        }));
}

/// A TP-4 clause is decided by the liveness engine (here its proof is
/// FR-339's component certificate), and a guarded `on each` activation is
/// no reason to refuse it: the engine reads the `when` condition through the
/// letters (FR-126-AC-16).
#[trace("TC-521", "FR-126-AC-16")]
#[test]
fn a_liveness_clause_is_decided_with_or_without_a_guarded_activation() {
    let guarded = format!("{} when (true)", each("Counter"));
    let proved = |profile, activation: &str, formula, guarded: bool| {
        let world = counter_world(
            profile,
            &[Clause {
                activation,
                formula,
            }],
        );
        let letters = Letters::new(&world.clauses[0], vec![value(|v| Some(v == 3))]);
        let letters = if guarded {
            letters.guarded(value(|_| Some(true)), crate::units::unread())
        } else {
            letters
        };
        run_letters(&world, 0, &["c"], &letters, ModelCheckLimits::default()).expect("checked")
    };
    let refusal = proved(
        INFINITE,
        ORIGIN,
        "always eventually holds(c.value = 3)",
        false,
    );
    assert!(
        matches!(
            refusal,
            ModelCheckOutcome::Holds {
                certificate: qsl_replay::ProofCertificate::Component(_),
                ..
            }
        ),
        "{refusal:?}"
    );
    let refusal = proved(
        INFINITE,
        &guarded,
        "always eventually holds(c.value = 3)",
        true,
    );
    assert!(
        matches!(
            refusal,
            ModelCheckOutcome::Holds {
                certificate: qsl_replay::ProofCertificate::Component(_),
                ..
            }
        ),
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
    let letters =
        Letters::new(&world.clauses[0], atoms).guarded(value(guard), crate::units::unread());
    run_letters(world, 0, &["c"], &letters, ModelCheckLimits::default()).expect("checked")
}

/// A letter seam that denies the selected read through the real evaluation
/// meter, preserving the counter named by the denied charge.
struct MeteredLetters<'a> {
    inner: &'a Letters,
    charge_atom: bool,
    charge_activation: bool,
    counter: LimitKind,
}

impl MeteredLetters<'_> {
    fn charge(&self, meter: &mut Meter) -> Result<(), Incomplete> {
        let charge = Charge::new(ChargePoint::GraphExpand);
        let charge = match self.counter {
            LimitKind::WorkUnits => charge,
            counter => charge.size(counter, 1),
        };
        meter.charge(charge)
    }
}

impl ModelLetters for MeteredLetters<'_> {
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        if self.charge_atom {
            self.charge(meter)?;
        }
        self.inner.atom(atom, instance, state, meter)
    }

    fn activation(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        if self.charge_activation {
            self.charge(meter).map_err(AtomStop::Incomplete)?;
        }
        self.inner.activation(instance, state, meter)
    }

    fn reads(&self, atom: FormulaNodeId) -> crate::units::Reads {
        self.inner.reads(atom)
    }

    fn activation_reads(&self) -> crate::units::Reads {
        self.inner.activation_reads()
    }
}

#[trace("TC-521", "FR-126-AC-5", "FR-126-AC-19", "FR-127-AC-4")]
#[test]
fn a_denied_letter_or_guard_charge_stops_with_the_accounting_setting() {
    for (guarded, counter, setting) in [
        (false, LimitKind::WorkUnits, "accounting.work_units"),
        (true, LimitKind::WorkUnits, "accounting.work_units"),
        (
            false,
            LimitKind::ValueOccurrences,
            "accounting.value_occurrences",
        ),
        (
            true,
            LimitKind::ValueOccurrences,
            "accounting.value_occurrences",
        ),
    ] {
        let world = if guarded {
            guarded_world(BOUNDED, "always[0,1] holds(c.value <= 3)")
        } else {
            counter_world(
                BOUNDED,
                &[Clause {
                    activation: ORIGIN,
                    formula: "always[0,1] holds(c.value <= 3)",
                }],
            )
        };
        let letters = Letters::new(&world.clauses[0], vec![value(|v| Some(v <= 3))]);
        let letters = if guarded {
            letters.guarded(value(|_| Some(true)), crate::units::unread())
        } else {
            letters
        };
        let metered = MeteredLetters {
            inner: &letters,
            charge_atom: !guarded,
            charge_activation: guarded,
            counter,
        };
        let mut meter = UNLIMITED;
        match counter {
            LimitKind::WorkUnits => meter.work_units = 0,
            LimitKind::ValueOccurrences => meter.value_occurrences = 0,
            _ => unreachable!("test cases choose a configured counter"),
        }
        let outcome = check_model(
            ModelCheckRequest {
                subject: world.subject(&["c"], meter),
                item: ModelCheckItem::Clause {
                    clause: &world.clauses[0],
                    letters: &metered,
                },
                max_depth: None,
                limits: ModelCheckLimits::default(),
                reductions: Default::default(),
            },
            &Cancel::new(),
        )
        .expect("checked");
        let ModelCheckOutcome::Stopped { cause, limit, .. } = outcome else {
            panic!("letter or guard exhaustion must stop the run: {outcome:?}");
        };
        assert_eq!(cause, IncompleteCause::ResourceExhausted);
        let reached = limit.expect("the denied meter counter is named");
        assert_eq!(reached.limit, ModelCheckLimit::EvaluationMeter { counter });
        assert_eq!(reached.value, 0);
        assert_eq!(reached.limit.setting(), setting);
    }
}

/// FR-126-AC-16 for TP-2: a guarded `on each` clause activates at the
/// positions where its `when` condition is true. Over the Counter subject,
/// `eventually[0,1] holds(c.value = 3)` is violated unguarded by the prefix
/// `0 -inc-> 1`; guarded by `c.value = 1` it activates at position 1 alone
/// and is violated by the prefix `0 -inc-> 1 -inc-> 2`; guarded by
/// `c.value >= 2` it activates at values 2 and 3, each of which reaches value 3 within
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
    let qsl_replay::ProofCertificate::Closure(certificate) = certificate else {
        panic!("a safety clause holds with a closure: {certificate:?}");
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

/// FR-126-AC-16 under partial-order reduction (FR-156): a partial-order run
/// of a guarded clause asks the letter seam for the guard's footprint, and a
/// guard whose footprint has no rule refuses the run `Footprint` before any
/// state is explored; the unreduced run reads no footprint. A guard that
/// reads nothing leaves the reduced and the unreduced run both violating the
/// clause. Which transitions the footprint makes visible is
/// `crate::partial_order`'s test of the same name.
#[trace("TC-521", "FR-126-AC-16")]
#[test]
fn a_partial_order_run_reads_the_guards_footprint() {
    let activation = format!("{} when (true)", each("Counter"));
    let world = counter_world(
        INFINITE,
        &[Clause {
            activation: &activation,
            formula: "always holds(c.value <= 2)",
        }],
    );
    let run = |partial_order: bool, reads: crate::units::Reads| {
        let letters = Letters::new(&world.clauses[0], vec![Box::new(|_, _| Some(false))])
            .guarded(value(|v| Some(v >= 1)), reads);
        check_model(
            ModelCheckRequest {
                subject: world.subject(&["c"], UNLIMITED),
                item: ModelCheckItem::Clause {
                    clause: &world.clauses[0],
                    letters: &letters,
                },
                max_depth: None,
                limits: ModelCheckLimits::default(),
                reductions: qsl_route::reductions::Reductions {
                    partial_order,
                    ..Default::default()
                },
            },
            &Cancel::new(),
        )
    };
    let no_rule = || {
        Err(
            qsl_semantics::check::footprint::FootprintRefusal::UnsupportedForm {
                clause: "C0".to_owned(),
                form: "when",
            },
        )
    };
    for (partial_order, reads) in [
        (false, crate::units::unread()),
        (true, crate::units::unread()),
        (false, no_rule()),
    ] {
        let outcome = run(partial_order, reads).expect("checked");
        assert_eq!(violated(&outcome).kind, CounterexampleKind::Formula);
    }
    let refusal = run(true, no_rule()).expect_err("the footprint has no rule");
    assert!(
        matches!(refusal, ModelCheckRefusal::Footprint(_)),
        "{refusal:?}"
    );
}
