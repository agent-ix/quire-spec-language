// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-124, FR-126, FR-127 and FR-128 over FR-124-AC-1's `Counter` subject
//! (`crate::counter`): the deadlock classifier, EN-1's first phase for a
//! reachable invariant (TP-1) and the deadlock-freedom item, the settlement
//! map, and the replay of a deadlock-freedom counterexample.
//!
//! Every expected path is computed independently of the engine, from the
//! Counter model's arithmetic or from a breadth-first enumeration of
//! integer pairs written here, and every reported post-state is checked by
//! the replay.
//!
//! Not covered: a TP-1 clause's `holds` atom is read through the test's own
//! [`crate::counter::Letters`], since S3 does not check a `holds` condition
//! yet, so FR-107's evaluation of an authored atom is not exercised; and an
//! authored clause's counterexample does not replay, since S4 emits no
//! temporal clause node (FR-337).

use std::collections::{BTreeMap, VecDeque};

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_model, ModelCheckRefusal, ModelCheckRequest};
use qsl_eval::simulation::model::{ModelSimState, ModelSystem, PopulationUniverse, TerminalClass};
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::bound::{DomainKey, DomainKind, FiniteBound, ProofBound};
use qsl_package::CheckedPackage;
use qsl_replay::{
    CounterexampleKind, IncompleteCause, ModelCheckItem, ModelCheckLimit, ModelCheckLimits,
    ModelCheckOutcome, ReachedLimit, TemporalCounterexample, UndecidedCause, UndefinedLocus,
    DEFAULT_MAX_AUTOMATON_STATES,
};
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{Cancel, CancelCause, ScalarLimits};
use quire_exact::{Integer, ValueType};

use crate::counter::{
    document, document_with_ranged_parameter, package, package_with_clauses, reference, subject,
    universe, value, Documents, Letters, CONTRACT, UNLIMITED, ZERO,
};

const WHEN: &str = "terminal Done using v on M when { self.value = 3 };\n";
const ANY: &str = "terminal Halt using v on M any;\n";

/// The Counter world: its package with `terminal`, the documents and the
/// initial snapshots of `starts`, each a list of `(key, value)`.
struct World {
    body: String,
    package: CheckedPackage,
    documents: Documents,
    initial: Vec<DocumentRef>,
}

fn world(terminal: &str, starts: &[&[(&str, i64)]]) -> World {
    world_with(document(false), &format!("{CONTRACT}{terminal}"), starts)
}

fn world_with(document: Vec<u8>, body: &str, starts: &[&[(&str, i64)]]) -> World {
    let package = package(&document, body);
    let mut documents = Documents::new(document);
    let initial = starts
        .iter()
        .enumerate()
        .map(|(index, objects)| documents.start(&format!("s{index}"), objects))
        .collect();
    World {
        body: body.to_owned(),
        package,
        documents,
        initial,
    }
}

fn run(
    world: &World,
    keys: &[&str],
    item: ModelCheckItem<'_>,
    max_depth: Option<usize>,
    limits: ModelCheckLimits,
    meter: ScalarLimits,
) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
    run_in(world, &world.package, keys, item, max_depth, limits, meter)
}

/// [`run`] over `package`, the world's package with a clause declared.
fn run_in(
    world: &World,
    package: &CheckedPackage,
    keys: &[&str],
    item: ModelCheckItem<'_>,
    max_depth: Option<usize>,
    limits: ModelCheckLimits,
    meter: ScalarLimits,
) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
    check_model(
        ModelCheckRequest {
            subject: subject(
                package,
                &world.documents,
                world.initial.clone(),
                keys,
                meter,
            ),
            item,
            max_depth,
            limits,
        },
        &Cancel::new(),
    )
}

fn deadlock(world: &World) -> ModelCheckOutcome {
    run(
        world,
        &["c"],
        ModelCheckItem::DeadlockFreedom,
        None,
        ModelCheckLimits::default(),
        UNLIMITED,
    )
    .expect("the subject is checked")
}

fn system<'w>(world: &'w World, keys: &[&str]) -> ModelSystem<'w> {
    ModelSystem::new(
        subject(
            &world.package,
            &world.documents,
            world.initial.clone(),
            keys,
            UNLIMITED,
        )
        .simulation,
    )
    .expect("the subject admits")
}

/// The value of `object` at each position of `counterexample`, read by
/// re-executing its steps: each step's post-state is the successor of its
/// transition identity whose state-key digest it records.
fn positions(
    world: &World,
    keys: &[&str],
    counterexample: &TemporalCounterexample,
) -> Vec<ModelSimState> {
    let system = system(world, keys);
    let mut current = system.initial()[counterexample.initial_state].clone();
    let mut states = vec![current.clone()];
    for step in &counterexample.prefix {
        let expansion = system.successors(&current).expect("the state expands");
        current = expansion
            .successors
            .into_iter()
            .find(|(transition, state)| {
                *transition == step.transition
                    && system.state_key(state).expect("encodes").1 == step.post_state
            })
            .map(|(_, state)| state)
            .expect("the recorded step is a successor");
        states.push(current.clone());
    }
    states
}

fn values(world: &World, counterexample: &TemporalCounterexample, object: &str) -> Vec<i64> {
    positions(world, &["c"], counterexample)
        .iter()
        .map(|state| value(state, object))
        .collect()
}

fn violated(outcome: &ModelCheckOutcome) -> &TemporalCounterexample {
    match outcome {
        ModelCheckOutcome::Violated { counterexample, .. } => counterexample,
        other => panic!("expected a violation, got {other:?}"),
    }
}

/// FR-124-AC-3: over the Counter subject, `value = 3` is the one terminal
/// state, deadlocked under `None` and intended under `When` and `Any`;
/// `value = 0` to `2` are not terminal under any declaration. The
/// classifier reads the state and its expansion alone and takes no fairness
/// set as input, so no fairness constraint of an item can change a
/// classification.
#[trace("TC-519", "FR-124-AC-3")]
#[test]
fn the_terminal_state_is_deadlocked_only_without_a_declaration_covering_it() {
    for (terminal, at_three) in [
        ("", TerminalClass::Deadlocked),
        (WHEN, TerminalClass::Intended),
        (ANY, TerminalClass::Intended),
    ] {
        let starts: Vec<Vec<(&str, i64)>> = (0..=3).map(|value| vec![("c", value)]).collect();
        let starts: Vec<&[(&str, i64)]> = starts.iter().map(Vec::as_slice).collect();
        let world = world(terminal, &starts);
        let system = system(&world, &["c"]);
        for (index, state) in system.initial().iter().enumerate() {
            let expansion = system.successors(state).expect("the state expands");
            let class = system
                .terminal_class(state, !expansion.successors.is_empty())
                .expect("the state classifies");
            let expected = if index == 3 {
                at_three.clone()
            } else {
                TerminalClass::NotTerminal
            };
            assert_eq!(class, expected, "{terminal:?} at value {index}");
        }
    }
}

/// Verifies the deadlock half of FR-126-AC-3, not its terminal-stutter
/// lasso, which needs EN-1's second phase: with no `terminal` declaration the
/// deadlock-freedom item is violated with `kind: Deadlock` and the prefix
/// `0 -inc-> 1 -inc-> 2 -inc-> 3`; with a `When` declaration covering 3, or
/// `Any`, it holds after exploring the four states.
#[trace("TC-521")]
#[test]
fn deadlock_freedom_is_violated_at_the_unintended_terminal_state() {
    let world = self::world("", &[&[("c", 0)]]);
    let outcome = deadlock(&world);
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Deadlock);
    assert_eq!(values(&world, counterexample, "c"), [0, 1, 2, 3]);
    assert_eq!(counterexample.initial_state, 0);
    assert_eq!(counterexample.over_binding, None);
    assert!(counterexample.fairness.is_empty());
    assert_eq!(counterexample.len(), 3);
    for step in &counterexample.prefix {
        assert!(
            format!("{:?}", step.transition).contains("Counter/inc"),
            "every step is inc: {:?}",
            step.transition
        );
    }
    assert_eq!(counterexample.trace_position(), None);

    for terminal in [WHEN, ANY] {
        let world = self::world(terminal, &[&[("c", 0)]]);
        match deadlock(&world) {
            ModelCheckOutcome::Holds { stats, .. } => {
                assert_eq!(stats.product_states, 4, "{terminal}");
                assert_eq!(stats.product_edges, 3, "{terminal}");
                assert_eq!(stats.depth, 3, "{terminal}");
            }
            other => panic!("{terminal}: expected Holds, got {other:?}"),
        }
    }
}

/// A two-counter state: `(a, b)`.
type Pair = (i64, i64);

/// A breadth-first enumeration of the two-counter subject, written from
/// FR-120's semantics of the fixture: `inc` with receiver `x` needs `x < 3`
/// and sets `x` to `x + 1`, and since its frame grants the `value` field of
/// every counter while its postcondition constrains the receiver alone, the
/// other counter takes each value in `0..=3`. Successors are in canonical
/// order: `inc(a)` before `inc(b)` (receiver key `a` before `b`), then by
/// post-state, `(a, b)` ascending. Returns the reachable states in
/// discovery order, the number of edges, and each state's first-found
/// predecessor.
fn pairs() -> (Vec<Pair>, usize, BTreeMap<Pair, Pair>) {
    let mut order = vec![(0, 0)];
    let mut parent = BTreeMap::new();
    let mut queue = VecDeque::from([(0, 0)]);
    let mut edges = 0;
    while let Some((a, b)) = queue.pop_front() {
        let mut successors = Vec::new();
        if a < 3 {
            successors.extend((0..=3).map(|other| (a + 1, other)));
        }
        if b < 3 {
            successors.extend((0..=3).map(|other| (other, b + 1)));
        }
        for next in successors {
            edges += 1;
            if !order.contains(&next) {
                order.push(next);
                parent.insert(next, (a, b));
                queue.push_back(next);
            }
        }
    }
    (order, edges, parent)
}

/// FR-126 "First phase" against an explicit enumeration: over two counters,
/// the deadlock-freedom item is violated at `(3, 3)`, the only terminal
/// state, by the breadth-first tree path the enumeration finds, and with
/// `terminal any` it holds after exploring every one of the 16 states and
/// 96 edges.
#[trace("TC-521")]
#[test]
fn the_deadlock_path_and_counts_match_an_explicit_enumeration() {
    let (order, edges, parent) = pairs();
    assert_eq!((order.len(), edges), (16, 96));
    let mut expected = vec![(3, 3)];
    while let Some(previous) = parent.get(expected.last().expect("a state")) {
        expected.push(*previous);
    }
    expected.reverse();

    let world = self::world("", &[&[("a", 0), ("b", 0)]]);
    let outcome = run(
        &world,
        &["a", "b"],
        ModelCheckItem::DeadlockFreedom,
        None,
        ModelCheckLimits::default(),
        UNLIMITED,
    )
    .expect("checked");
    let counterexample = violated(&outcome);
    let reached: Vec<(i64, i64)> = positions(&world, &["a", "b"], counterexample)
        .iter()
        .map(|state| (value(state, "a"), value(state, "b")))
        .collect();
    assert_eq!(reached, expected);
    assert_eq!(counterexample.kind, CounterexampleKind::Deadlock);
    let discovered = order
        .iter()
        .position(|state| *state == (3, 3))
        .expect("reachable");
    assert_eq!(
        outcome.stats().product_states,
        u64::try_from(order.len()).expect("small"),
        "every state is discovered before (3, 3), number {discovered}, is expanded"
    );

    let world = self::world(ANY, &[&[("a", 0), ("b", 0)]]);
    let outcome = run(
        &world,
        &["a", "b"],
        ModelCheckItem::DeadlockFreedom,
        None,
        ModelCheckLimits::default(),
        UNLIMITED,
    )
    .expect("checked");
    let ModelCheckOutcome::Holds { stats, .. } = outcome else {
        panic!("expected Holds, got {outcome:?}");
    };
    assert_eq!((stats.product_states, stats.product_edges), (16, 96));
}

fn letters(world: &World, keys: &[&str], atom: fn(i64) -> Option<bool>) -> Letters {
    Letters {
        bindings: keys
            .iter()
            .map(|key| reference(&world.package, &world.documents, key))
            .collect(),
        atom,
    }
}

fn invariant(
    world: &World,
    keys: &[&str],
    formula: &str,
    atom: fn(i64) -> Option<bool>,
) -> ModelCheckOutcome {
    let (package, clauses) = package_with_clauses(
        &world.documents.document,
        &world.body,
        &[("C", "", formula)],
    );
    let [clause] = clauses.as_slice() else {
        panic!("one declared clause");
    };
    let letters = letters(world, keys, atom);
    run_in(
        world,
        &package,
        keys,
        ModelCheckItem::Clause {
            clause,
            letters: &letters,
        },
        None,
        ModelCheckLimits::default(),
        UNLIMITED,
    )
    .expect("checked")
}

/// Verifies the TP-1 half of FR-126-AC-2, over the Counter subject rather
/// than ADR-018 §6's; its TP-2 `on each` rows are not verified, since this
/// phase decides TP-1 alone. Over Counter: `always holds(c.value
/// <= 3)` holds after every reachable state is examined; `always
/// holds(c.value != 2)` is violated with `kind: Formula`, the prefix `0
/// -inc-> 1 -inc-> 2` and `over_binding` `c`, its trace position 2.
#[trace("TC-521")]
#[test]
fn a_reachable_invariant_holds_or_is_violated_at_its_first_bad_state() {
    let world = self::world("", &[&[("c", 0)]]);
    match invariant(&world, &["c"], "always holds(c.value <= 3)", |v| {
        Some(v <= 3)
    }) {
        ModelCheckOutcome::Holds { stats, .. } => assert_eq!(stats.product_states, 4),
        other => panic!("expected Holds, got {other:?}"),
    }
    let outcome = invariant(&world, &["c"], "always holds(c.value != 2)", |v| {
        Some(v != 2)
    });
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(values(&world, counterexample, "c"), [0, 1, 2]);
    assert_eq!(
        counterexample.over_binding,
        Some(reference(&world.package, &world.documents, "c"))
    );
    assert_eq!(counterexample.trace_position(), Some(2));
}

/// The bound instance a formula violation names is the first instance, in
/// binding order, whose atom is false at the violating state: over two
/// counters `a` (initial 2) and `b` (initial 0), `always holds(c.value !=
/// 3)` is first false for `a`, at `(3, 0)`.
#[trace("TC-521")]
#[test]
fn a_violation_names_the_first_bound_instance_that_is_false() {
    let world = self::world("", &[&[("a", 2), ("b", 0)]]);
    let outcome = invariant(&world, &["a", "b"], "always holds(c.value != 3)", |v| {
        Some(v != 3)
    });
    let counterexample = violated(&outcome);
    let last = positions(&world, &["a", "b"], counterexample)
        .last()
        .map(|state| (value(state, "a"), value(state, "b")))
        .expect("a state");
    assert_eq!(last, (3, 0));
    assert_eq!(
        counterexample.over_binding,
        Some(reference(&world.package, &world.documents, "a"))
    );
}

/// Verifies the TP-1 half of FR-126-AC-9; its TP-4 claim, the `kind:
/// Formula` conjunction and the TP-2 `Holds` are not verified. Over the
/// Counter subject: `always holds(6 mod (2 -
/// c.value) >= 0)` is violated with the prefix `0 -inc-> 1 -inc-> 2` and
/// `kind: UndefinedEvaluation{where: position 2, cause: division-by-zero}`,
/// at the clause's atom.
#[trace("TC-537")]
#[test]
fn an_undefined_atom_refutes_at_the_first_position_it_is_undefined() {
    let world = self::world("", &[&[("c", 0)]]);
    let formula = "always holds(6 mod (2 - c.value) >= 0)";
    let outcome = invariant(&world, &["c"], formula, |v| {
        (v != 2).then(|| 6 % (2 - v) >= 0)
    });
    let counterexample = violated(&outcome);
    assert_eq!(values(&world, counterexample, "c"), [0, 1, 2]);
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("expected an undefined evaluation: {counterexample:?}");
    };
    assert_eq!(evaluation.site.position, 2);
    assert!(matches!(evaluation.site.locus, UndefinedLocus::Atom(_)));
    assert_eq!(
        evaluation.cause,
        UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero)
    );
    let last = positions(&world, &["c"], counterexample);
    let digest = system(&world, &["c"])
        .state_key(last.last().expect("a state"))
        .expect("encodes")
        .1;
    assert_eq!(evaluation.site.state_key, digest);
    assert_eq!(counterexample.trace_position(), Some(2));
}

/// Verifies the deadlock half of FR-126-AC-9 (its other halves are listed
/// above), its division written `mod`, which S3
/// types over integers with no division law: with `terminal when 6 mod (3 -
/// self.value) = 0`, the deadlock-freedom item is violated with the prefix to value 3 and
/// `kind: UndefinedEvaluation{where: position 3, cause: division-by-zero}`
/// at the declaration of `M`. ADR-018 UE-6: an undefined precondition
/// guard refutes the item at the expanded state, naming the guarded
/// transition, never reading the step as disabled.
#[trace("TC-537")]
#[test]
fn an_undefined_model_side_expression_refutes_the_deadlock_item() {
    let world = self::world(
        "terminal Done using v on M when { 6 mod (3 - self.value) = 0 };\n",
        &[&[("c", 0)]],
    );
    let outcome = deadlock(&world);
    let counterexample = violated(&outcome);
    assert_eq!(values(&world, counterexample, "c"), [0, 1, 2, 3]);
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("expected an undefined evaluation: {counterexample:?}");
    };
    assert_eq!(evaluation.site.position, 3);
    assert_eq!(
        evaluation.site.locus,
        UndefinedLocus::Terminal("M".to_owned())
    );
    assert_eq!(
        evaluation.cause,
        UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero)
    );

    let guarded = "pre CanInc using v on M::Counter::inc { 6 mod (2 - self.value) >= 0 }\n\
                   post Inc using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n";
    let world = world_with(document(false), guarded, &[&[("c", 0)]]);
    let outcome = deadlock(&world);
    let counterexample = violated(&outcome);
    assert_eq!(values(&world, counterexample, "c"), [0, 1, 2]);
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("expected an undefined evaluation: {counterexample:?}");
    };
    assert_eq!(evaluation.site.position, 2);
    assert_eq!(
        evaluation.site.locus,
        UndefinedLocus::Contract("CanInc".to_owned())
    );
    assert!(
        format!("{:?}", evaluation.site.transition).contains("Counter/inc"),
        "the guard names inc: {:?}",
        evaluation.site.transition
    );
}

/// FR-126 limits over the Counter subject's deadlock-freedom item: a
/// horizon of 2 completes without the depth-3 deadlock and settles
/// `BoundReached{2}`, a horizon of 3 finds it; `max_states` 2 and
/// `max_transitions` 1 stop the run naming the limit and its value; an
/// already-cancelled handle stops it with no limit; a zero evaluation meter
/// stops it at `inc`'s precondition naming the meter, 0.
#[trace("TC-521")]
#[test]
fn limits_and_cancellation_stop_the_run_naming_what_stopped_it() {
    let world = self::world("", &[&[("c", 0)]]);
    let with = |max_depth, limits, meter| {
        run(
            &world,
            &["c"],
            ModelCheckItem::DeadlockFreedom,
            max_depth,
            limits,
            meter,
        )
        .expect("checked")
    };
    let defaults = ModelCheckLimits::default();
    assert!(matches!(
        with(Some(2), defaults, UNLIMITED),
        ModelCheckOutcome::BoundReached { depth: 2, .. }
    ));
    assert_eq!(
        violated(&with(Some(3), defaults, UNLIMITED)).kind,
        CounterexampleKind::Deadlock
    );
    let stopped = |outcome: ModelCheckOutcome| match outcome {
        ModelCheckOutcome::Stopped { cause, limit, .. } => (cause, limit),
        other => panic!("expected a stopped run, got {other:?}"),
    };
    let mut states = defaults;
    states.limits.max_states = 2;
    assert_eq!(
        stopped(with(None, states, UNLIMITED)),
        (
            IncompleteCause::ResourceExhausted,
            Some(ReachedLimit {
                limit: ModelCheckLimit::MaxStates,
                value: 2
            })
        )
    );
    let mut transitions = defaults;
    transitions.limits.max_transitions = 1;
    assert_eq!(
        stopped(with(None, transitions, UNLIMITED)),
        (
            IncompleteCause::ResourceExhausted,
            Some(ReachedLimit {
                limit: ModelCheckLimit::MaxTransitions,
                value: 1
            })
        )
    );
    assert_eq!(
        stopped(with(None, defaults, ZERO)),
        (
            IncompleteCause::ResourceExhausted,
            Some(ReachedLimit {
                limit: ModelCheckLimit::EvaluationMeter,
                value: 0
            })
        )
    );
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let cancelled = check_model(
        ModelCheckRequest {
            subject: subject(
                &world.package,
                &world.documents,
                world.initial.clone(),
                &["c"],
                UNLIMITED,
            ),
            item: ModelCheckItem::DeadlockFreedom,
            max_depth: None,
            limits: defaults,
        },
        &cancel,
    )
    .expect("checked");
    assert_eq!(
        stopped(cancelled),
        (
            IncompleteCause::Cancelled {
                source: CancelCause::Requested
            },
            None
        )
    );
}

/// Verifies the limits half of FR-126-AC-8, not all of it:
/// `ModelCheckLimits::default()` is FR-101's
/// `Limits::default()` (10,000,000 states, 100,000,000 transitions) and
/// 1,048,576 automaton states; `max_automaton_states` 1 stops a run at the
/// monitor's second state, naming the limit.
#[trace("TC-536")]
#[test]
fn the_limits_publish_their_defaults() {
    let defaults = ModelCheckLimits::default();
    assert_eq!(defaults.limits.max_states, 10_000_000);
    assert_eq!(defaults.limits.max_transitions, 100_000_000);
    assert_eq!(defaults.max_automaton_states, 1_048_576);
    assert_eq!(DEFAULT_MAX_AUTOMATON_STATES, 1 << 20);

    let world = self::world("", &[&[("c", 0)]]);
    let tight = ModelCheckLimits {
        max_automaton_states: 1,
        ..defaults
    };
    let outcome = run(
        &world,
        &["c"],
        ModelCheckItem::DeadlockFreedom,
        None,
        tight,
        UNLIMITED,
    )
    .expect("checked");
    assert!(
        matches!(
            outcome,
            ModelCheckOutcome::Stopped {
                limit: Some(ReachedLimit {
                    limit: ModelCheckLimit::MaxAutomatonStates,
                    value: 1
                }),
                ..
            }
        ),
        "{outcome:?}"
    );
}

/// FR-125 and ADR-016 EX-10: a subject with no initial state is
/// `Undecided(NoInitialState)` before any state is explored; FR-126's
/// pre-check: a subject with an operation parameter no finite domain bounds
/// is refused `RequiresBound` before exploring; a TP-3 clause is not
/// decided by this phase and is refused.
#[trace("TC-521")]
#[test]
fn an_empty_or_unbounded_subject_and_an_undecided_form_explore_nothing() {
    let world = self::world("", &[]);
    assert!(matches!(
        deadlock(&world),
        ModelCheckOutcome::Undecided {
            cause: UndecidedCause::NoInitialState,
            stats,
        } if stats.product_states == 0
    ));

    let world = world_with(document(true), CONTRACT, &[&[("c", 0)]]);
    let refusal = run(
        &world,
        &["c"],
        ModelCheckItem::DeadlockFreedom,
        None,
        ModelCheckLimits::default(),
        UNLIMITED,
    )
    .expect_err("an unbounded parameter");
    assert!(
        matches!(&refusal, ModelCheckRefusal::RequiresBound(_)),
        "{refusal:?}"
    );

    let world = self::world("", &[&[("c", 0)]]);
    let (package, clauses) = package_with_clauses(
        &world.documents.document,
        &world.body,
        &[(
            "C",
            "",
            "always (holds(c.value <= 0) and holds(c.value >= 0))",
        )],
    );
    let [safety] = clauses.as_slice() else {
        panic!("one declared clause");
    };
    let letters = letters(&world, &["c"], |_| Some(true));
    let refusal = run_in(
        &world,
        &package,
        &["c"],
        ModelCheckItem::Clause {
            clause: safety,
            letters: &letters,
        },
        None,
        ModelCheckLimits::default(),
        UNLIMITED,
    )
    .expect_err("TP-3 is not decided by this phase");
    assert!(
        matches!(refusal, ModelCheckRefusal::FormNotDecided { .. }),
        "{refusal:?}"
    );
}

/// FR-126 "Determinism": two runs of one request give equal outcomes and
/// equal counterexamples.
#[trace("TC-521")]
#[test]
fn two_runs_give_equal_outcomes() {
    let world = self::world("", &[&[("a", 0), ("b", 1)]]);
    let once = || {
        run(
            &world,
            &["a", "b"],
            ModelCheckItem::DeadlockFreedom,
            None,
            ModelCheckLimits::default(),
            UNLIMITED,
        )
        .expect("checked")
    };
    assert_eq!(once(), once());
}

/// Run the deadlock item over `world` with `universes` in place of the
/// subject's universe and `proof_bounds` beside it, under `meter`.
fn run_with(
    world: &World,
    universes: Vec<PopulationUniverse>,
    proof_bounds: Vec<ProofBound>,
    meter: ScalarLimits,
) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
    let mut subject = subject(
        &world.package,
        &world.documents,
        world.initial.clone(),
        &["c"],
        meter,
    );
    subject.simulation.universes = universes;
    subject.proof_bounds = proof_bounds;
    check_model(
        ModelCheckRequest {
            subject,
            item: ModelCheckItem::DeadlockFreedom,
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
}

fn system_without_universe(world: &World) -> ModelSystem<'_> {
    let mut simulation = subject(
        &world.package,
        &world.documents,
        world.initial.clone(),
        &["c"],
        UNLIMITED,
    )
    .simulation;
    simulation.universes = Vec::new();
    ModelSystem::new(simulation).expect("the subject admits")
}

fn unbounded_roots(refusal: &ModelCheckRefusal) -> Vec<(DomainKey, DomainKind)> {
    match refusal {
        ModelCheckRefusal::RequiresBound(bound) => bound
            .domains
            .iter()
            .map(|(key, kind)| (key.clone(), kind))
            .collect(),
        other => panic!("not RequiresBound: {other:?}"),
    }
}

/// FR-336-AC-2: a population root with no universe is refused
/// `RequiresBound` naming it, before any state is sampled -- a zero meter,
/// which stops any exploration, changes nothing. This covers the refusal
/// half of FR-336-AC-2 only: its "explores and returns FR-126-AC-1's
/// outcome" half needs `Holds` over 15 product states for `always
/// eventually`, which EN-1 refuses as `FormNotDecided` yet, so the AC is
/// not tagged.
#[trace("TC-846")]
#[test]
fn a_population_with_no_universe_is_refused_before_exploring() {
    let world = self::world("", &[&[("c", 0)]]);
    let refusal = run_with(&world, Vec::new(), Vec::new(), ZERO).expect_err("no universe");
    let roots = unbounded_roots(&refusal);
    assert_eq!(roots.len(), 1, "{roots:?}");
    assert_eq!(roots[0].1, DomainKind::Population);
    let held: Vec<_> = system_without_universe(&world)
        .domains()
        .into_iter()
        .filter(|(_, value_type)| matches!(value_type, ValueType::Population(_)))
        .collect();
    let [(population, _)] = held.as_slice() else {
        panic!("one population root: {held:?}");
    };
    assert_eq!(
        roots[0].0,
        DomainKey::Node {
            node: *population,
            path: Vec::new()
        }
    );

    let outcome = run_with(&world, universe(&["c"]), Vec::new(), UNLIMITED)
        .expect("a universe makes the root finite");
    assert!(
        matches!(
            &outcome,
            ModelCheckOutcome::Violated { .. } | ModelCheckOutcome::Holds { .. }
        ),
        "{outcome:?}"
    );
}

/// FR-336-AC-3: an unranged `Integer` parameter is refused naming that
/// root with or without a `ProofBound` on it; declared `Int[0, 3]` it
/// explores.
#[trace("TC-846", "FR-336-AC-3")]
#[test]
fn a_proof_bound_does_not_make_an_unranged_parameter_finite() {
    let world = world_with(document(true), CONTRACT, &[&[("c", 0)]]);
    let bare = run_with(&world, universe(&["c"]), Vec::new(), UNLIMITED).expect_err("unranged");
    let roots = unbounded_roots(&bare);
    assert_eq!(roots.len(), 1, "{roots:?}");
    assert_eq!(roots[0].1, DomainKind::Integer);

    let range =
        FiniteBound::integer_range(Integer::from(0_i64), Integer::from(3_i64)).expect("a range");
    let proof = ProofBound::new(roots[0].0.clone(), Some(DomainKind::Integer), range)
        .expect("an integer range pairs with an integer domain");
    let bounded = run_with(&world, universe(&["c"]), vec![proof], UNLIMITED)
        .expect_err("a proof bound is not a universe");
    assert_eq!(unbounded_roots(&bounded), roots);

    let world = world_with(document_with_ranged_parameter(), CONTRACT, &[&[("c", 0)]]);
    let outcome =
        run_with(&world, universe(&["c"]), Vec::new(), UNLIMITED).expect("Int[0, 3] is finite");
    assert!(
        matches!(
            &outcome,
            ModelCheckOutcome::Violated { .. } | ModelCheckOutcome::Holds { .. }
        ),
        "{outcome:?}"
    );
}
