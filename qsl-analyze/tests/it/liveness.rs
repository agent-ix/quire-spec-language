// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126's liveness phase (TP-4) and its weak-fairness filter over the
//! `Counter` subject (`crate::counter`) and its `setTo` variant: the
//! product's SCC phase, the fairness filter, the canonical lasso, the
//! terminal stutter loop, the limits and the refusals of what is not built.
//!
//! Every expected loop below is derived by hand from the model's arithmetic
//! and FR-101's canonical order (see each test), and every lasso is also
//! checked by an oracle written here that reads only the model's successor
//! relation: the loop closes, the claim is false on it, and each weak
//! constraint is taken in it or disabled somewhere in it.
//!
//! ADR-018 §6's ConfigVersion units and FR-126-AC-4's `Health` unit are
//! `example_units`. Not covered here: the `holds` atoms are read through the
//! test's own [`Letters`]; FR-339's component certificate for a proof is not
//! built (`proof_waits_for_the_component_certificate`), and the lasso's
//! replay is FR-128's.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{
    check_model, LivenessRefusal, ModelCheckRefusal, ModelCheckRequest,
};
use qsl_eval::model_subject::{ModelInstance, ModelLetters};
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::temporal::UndefinedCause;
use qsl_package::CheckedPackage;
use qsl_replay::{
    CounterexampleKind, IncompleteCause, Lasso, ModelCheckItem, ModelCheckLimit, ModelCheckLimits,
    ModelCheckOutcome, ReachedLimit, TemporalCounterexample, UndefinedLocus,
};
use qsl_semantics::check::CheckedTemporalClause;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{Cancel, CancelCause, ScalarLimits};

use crate::counter::{
    clause_text, document, document_with_ranged_parameter, key, package_with_clauses, subject,
    unit_text, value, Documents, Letters, CONTRACT, UNLIMITED, ZERO,
};

/// A Counter world with one declared clause.
pub(crate) struct Fixture {
    pub(crate) package: CheckedPackage,
    pub(crate) clauses: Vec<CheckedTemporalClause>,
    pub(crate) documents: Documents,
    pub(crate) initial: Vec<DocumentRef>,
    pub(crate) keys: Vec<&'static str>,
    /// The native unit the package was checked from.
    pub(crate) unit: Vec<u8>,
}

/// The world over `document` with `body`, one start with the counters
/// `starts`, and the clause `fairness` + `formula` over `c`.
pub(crate) fn fixture(
    document: Vec<u8>,
    body: &str,
    fairness: &str,
    formula: &str,
    starts: &[(&'static str, i64)],
) -> Fixture {
    let (package, clauses) = package_with_clauses(&document, body, &[("C", fairness, formula)]);
    let unit = unit_text(
        &document,
        &(body.to_owned() + &clause_text("C", fairness, formula)),
    )
    .into_bytes();
    let mut documents = Documents::new(document);
    let initial = vec![documents.start("s0", starts)];
    Fixture {
        package,
        clauses,
        documents,
        initial,
        keys: starts.iter().map(|(key, _)| *key).collect(),
        unit,
    }
}

/// The one-counter fixture over `document`, started at `c = 0`.
pub(crate) fn counter(document: Vec<u8>, fairness: &str, formula: &str) -> Fixture {
    fixture(document, CONTRACT, fairness, formula, &[("c", 0)])
}

impl Fixture {
    fn letters(&self, atom: fn(i64) -> Option<bool>) -> Letters {
        Letters { atom }
    }

    pub(crate) fn run(
        &self,
        atom: fn(i64) -> Option<bool>,
        max_depth: Option<usize>,
        limits: ModelCheckLimits,
        meter: ScalarLimits,
        cancel: &Cancel,
    ) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
        let letters = self.letters(atom);
        check_model(
            ModelCheckRequest {
                subject: subject(
                    &self.package,
                    &self.documents,
                    self.initial.clone(),
                    &self.keys,
                    meter,
                ),
                item: ModelCheckItem::Clause {
                    clause: &self.clauses[0],
                    letters: &letters,
                },
                max_depth,
                limits,
                reductions: Default::default(),
            },
            cancel,
        )
    }

    pub(crate) fn decide(
        &self,
        atom: fn(i64) -> Option<bool>,
    ) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
        self.run(
            atom,
            None,
            ModelCheckLimits::default(),
            UNLIMITED,
            &Cancel::new(),
        )
    }

    fn system(&self) -> ModelSystem<'_> {
        ModelSystem::new(
            subject(
                &self.package,
                &self.documents,
                self.initial.clone(),
                &self.keys,
                UNLIMITED,
            )
            .simulation,
        )
        .expect("the subject admits")
    }

    /// The model states a counterexample's steps reach, position by
    /// position: each step's post-state is the successor of its transition
    /// identity whose state-key digest it records.
    fn positions(&self, counterexample: &TemporalCounterexample) -> Vec<ModelSimState> {
        let system = self.system();
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

    pub(crate) fn values(&self, counterexample: &TemporalCounterexample, object: &str) -> Vec<i64> {
        self.positions(counterexample)
            .iter()
            .map(|state| value(state, object))
            .collect()
    }
}

/// The instance of the clause that binds the counter `object`.
fn instance(object: &str) -> ModelInstance {
    ModelInstance {
        population: key("counters"),
        key: object.to_owned(),
    }
}

fn violated(outcome: &ModelCheckOutcome) -> &TemporalCounterexample {
    match outcome {
        ModelCheckOutcome::Violated { counterexample, .. } => counterexample,
        other => panic!("expected a violation, got {other:?}"),
    }
}

fn lasso_of(counterexample: &TemporalCounterexample) -> Lasso {
    counterexample
        .lasso
        .expect("a liveness refutation is a lasso")
}

/// Whether the transition is an application of the `Counter` operation
/// `operation`.
fn applies(transition: &qsl_replay::ModelStep, operation: &str) -> bool {
    transition
        .transition
        .operation()
        .contains(&format!("Counter/{operation}"))
}

/// The oracle for a lasso over `fixture`, read from the model's own
/// successor relation: the last post-state of the prefix is the state at
/// the loop's entry, and a weak constraint on the `Counter` operation
/// `operation` (all of its transitions together, or each transition alone
/// when `each`) is taken in the loop or disabled at some state of it.
fn assert_weakly_fair(
    fixture: &Fixture,
    counterexample: &TemporalCounterexample,
    operation: &str,
    each: bool,
) {
    let lasso = lasso_of(counterexample);
    assert!(!lasso.terminal_stutter);
    let states = fixture.positions(counterexample);
    let system = fixture.system();
    let digest = |state: &ModelSimState| system.state_key(state).expect("encodes").1;
    assert_eq!(
        digest(states.last().expect("a position")),
        digest(&states[lasso.entry]),
        "the loop re-enters its entry state"
    );
    let loop_steps = &counterexample.prefix[lasso.entry..];
    assert!(!loop_steps.is_empty(), "a model loop has a step");
    let loop_states = &states[lasso.entry..states.len() - 1];
    let enabled: Vec<Vec<qsl_replay::ModelStep>> = loop_states
        .iter()
        .map(|state| {
            system
                .successors(state)
                .expect("the state expands")
                .successors
                .into_iter()
                .map(|(transition, next)| qsl_replay::ModelStep {
                    transition,
                    post_state: digest(&next),
                })
                .collect()
        })
        .collect();
    let groups: Vec<Option<&qsl_replay::ModelStep>> = if each {
        let mut identities: Vec<&qsl_replay::ModelStep> = enabled
            .iter()
            .flatten()
            .filter(|step| applies(step, operation))
            .collect();
        identities.sort_by(|left, right| left.transition.cmp(&right.transition));
        identities.dedup_by(|left, right| left.transition == right.transition);
        identities.into_iter().map(Some).collect()
    } else {
        vec![None]
    };
    for group in groups {
        let covers = |step: &qsl_replay::ModelStep| {
            applies(step, operation) && group.is_none_or(|one| one.transition == step.transition)
        };
        let taken = loop_steps.iter().any(covers);
        let disabled_somewhere = enabled.iter().any(|steps| !steps.iter().any(covers));
        assert!(
            taken || disabled_somewhere,
            "the weak constraint on {operation} ({group:?}) is starved in the loop"
        );
    }
}

fn is_three(value: i64) -> Option<bool> {
    Some(value == 3)
}

pub(crate) fn is_zero(value: i64) -> Option<bool> {
    Some(value == 0)
}

/// FR-126-AC-3's last sentence, with the deadlock half in
/// `model_check::deadlock_freedom_is_violated_at_the_unintended_terminal_state`:
/// over the `Counter` subject (no `terminal` declaration), `always
/// eventually holds(c.value = 0)` under infinite-trace is violated by the
/// lasso whose loop is the terminal stutter step at value 3, with the
/// stutter marker set. The one behaviour is `0, 1, 2, 3, 3, ...`, so the
/// stem is `0 -inc-> 1 -inc-> 2 -inc-> 3` and the loop has no model step.
#[trace("TC-521", "FR-126-AC-3")]
#[test]
fn a_behaviour_that_ends_is_the_lasso_whose_loop_is_the_terminal_stutter_step() {
    let fixture = counter(document(false), "", "always eventually holds(c.value = 0)");
    let outcome = fixture.decide(is_zero).expect("checked");
    let counterexample = violated(&outcome);
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(fixture.values(counterexample, "c"), [0, 1, 2, 3]);
    assert_eq!(
        counterexample.lasso,
        Some(Lasso {
            entry: 3,
            terminal_stutter: true
        })
    );
    // The stem's three `inc` steps and the stutter step of the loop.
    assert_eq!(counterexample.len(), 4);
    assert_eq!(counterexample.prefix.len(), 3);
    assert_eq!(counterexample.initial_state, 0);
    assert_eq!(counterexample.over_binding, Some(instance("c")));
    assert_eq!(counterexample.trace_position(), Some(0));
    assert!(counterexample
        .prefix
        .iter()
        .all(|step| applies(step, "inc")));
}

/// FR-126 "Second phase": the first phase's states are the product's: a
/// terminal state's stutter edge is a product edge, so the single behaviour
/// `0, 1, 2, 3, 3, ...` is a path of the product. `always eventually
/// holds(c.value = 3)` holds on it, so no component passes. A proof needs
/// FR-339's component certificate, which is not built: the run is refused
/// naming it, and says it explored the product. The negation `eventually
/// always not p` is a two-state automaton whose second state reads `not p`,
/// so the product holds 3 + 3 states at values 0 to 2 and, at value 3 where
/// `p` holds, the first automaton state alone: 7. The second state has no
/// move at value 3, but that model state is also a product state, so the
/// model-only pass finds nothing left to read.
///
/// This asserts an interim behaviour that FR-126 and TC-521 contradict
/// (they expect `Holds{Exhaustive}`), so it carries no AC tag; PR 2 replaces
/// it with the `Holds` test.
#[test]
fn proof_waits_for_the_component_certificate() {
    let fixture = counter(document(false), "", "always eventually holds(c.value = 3)");
    let refusal = fixture.decide(is_three).expect_err("no certificate yet");
    let ModelCheckRefusal::Liveness(LivenessRefusal::ComponentCertificateNotBuilt { stats }) =
        refusal
    else {
        panic!("expected the missing certificate: {refusal:?}");
    };
    assert_eq!(stats.product_states, 7);
    assert_eq!(stats.automaton_states, 2);
    assert_eq!(stats.depth, 3);
}

/// ADR-018 UE-1: a model state reached only where the negation automaton
/// has no move still has its letter read. `eventually holds(c.value = 1)`
/// negates to `always not p`, which has no move at value 1, so values 2 and
/// 3 are reached by no product state: the product holds value 0 alone and
/// counts one state and no edge. The model-only pass reads the rest: with
/// the atom undefined at value 2 the claim is refuted at position 2 by the
/// prefix `0 -inc-> 1 -inc-> 2`, and with it defined everywhere the run is
/// the interim certificate refusal over the same one product state.
#[test]
fn positions_past_a_dead_automaton_are_still_read() {
    let fixture = counter(document(false), "", "eventually holds(c.value = 1)");
    let outcome = fixture
        .decide(|v| match v {
            1 => Some(true),
            2 => None,
            _ => Some(false),
        })
        .expect("checked");
    let counterexample = violated(&outcome);
    assert_eq!(fixture.values(counterexample, "c"), [0, 1, 2]);
    assert_eq!(counterexample.lasso, None);
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("expected an undefined evaluation: {counterexample:?}");
    };
    assert_eq!(evaluation.site.position, 2);
    assert_eq!(outcome.stats().product_states, 1);

    // The model-only pass is charged to the run's budgets: with none left it
    // stops the run naming the limit instead of walking the model.
    let stops = |limits: ModelCheckLimits| match fixture
        .run(|v| Some(v == 1), None, limits, UNLIMITED, &Cancel::new())
        .expect("checked")
    {
        ModelCheckOutcome::Stopped { cause, limit, .. } => (cause, limit),
        other => panic!("expected a stopped run, got {other:?}"),
    };
    let mut states = ModelCheckLimits::default();
    states.limits.max_states = 1;
    assert_eq!(
        stops(states),
        (
            IncompleteCause::ResourceExhausted,
            Some(ReachedLimit {
                limit: ModelCheckLimit::MaxStates,
                value: 1
            })
        )
    );
    let mut edges = ModelCheckLimits::default();
    edges.limits.max_transitions = 0;
    assert_eq!(
        stops(edges),
        (
            IncompleteCause::ResourceExhausted,
            Some(ReachedLimit {
                limit: ModelCheckLimit::MaxTransitions,
                value: 0
            })
        )
    );

    let refusal = fixture
        .decide(|v| Some(v == 1))
        .expect_err("no certificate yet");
    let ModelCheckRefusal::Liveness(LivenessRefusal::ComponentCertificateNotBuilt { stats }) =
        refusal
    else {
        panic!("expected the missing certificate: {refusal:?}");
    };
    // The one edge leads to a model state with no product state.
    assert_eq!((stats.product_states, stats.product_edges), (1, 0));
}

/// FR-126 "Explicit-state limits": `max_states` and `max_transitions` are the
/// run's, not each instance's. Over two counters a clause binds two
/// instances, each its own product; with `max_states` 20 the run stops
/// naming 20 and the products together hold at most 20 states, and a run
/// with no such limit reports more. The negation automaton of both
/// instances is one automaton: two states, not two per instance.
#[test]
fn the_state_budget_is_shared_by_every_instance() {
    // `p` holds everywhere, so the negation has no run and nothing is
    // violated: each instance's product is the whole model.
    let formula = "always eventually holds(c.value = 3)";
    let always = |_: i64| Some(true);
    let one = fixture(document(false), CONTRACT, "", formula, &[("a", 0)]);
    let alone = unwrap_refusal(one.decide(always).expect_err("no certificate yet"));
    let both = fixture(
        document(false),
        CONTRACT,
        "",
        formula,
        &[("a", 0), ("b", 0)],
    );
    let together = unwrap_refusal(both.decide(always).expect_err("no certificate yet"));
    assert!(together.product_states > 20, "{together:?}");
    assert_eq!(together.automaton_states, alone.automaton_states);

    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 20;
    let outcome = both
        .run(always, None, limits, UNLIMITED, &Cancel::new())
        .expect("checked");
    let ModelCheckOutcome::Stopped { limit, stats, .. } = outcome else {
        panic!("expected a stopped run, got {outcome:?}");
    };
    assert_eq!(
        limit,
        Some(ReachedLimit {
            limit: ModelCheckLimit::MaxStates,
            value: 20
        })
    );
    assert!(stats.product_states <= 20, "{stats:?}");
}

/// The counts of a run refused for want of the component certificate.
fn unwrap_refusal(refusal: ModelCheckRefusal) -> qsl_replay::RunStats {
    let ModelCheckRefusal::Liveness(LivenessRefusal::ComponentCertificateNotBuilt { stats }) =
        refusal
    else {
        panic!("expected the missing certificate: {refusal:?}");
    };
    stats
}

/// FR-126 "Fairness filter" and "Counterexamples" over the Counter with
/// `setTo(n)`, whose post-state is any value in `0..=3` (it has no `post`),
/// so every value reaches every value and `inc` also steps `v -> v + 1` for
/// `v < 3`. `always eventually holds(c.value = 3)` is violated by a run
/// that stays in `0..=2`; the product's accepting component is
/// `{(0, q1), (1, q1), (2, q1)}` whose first state `(0, q1)` is an initial
/// state, so the stem is empty.
///
/// With no constraint the loop is the shortest cycle through the entry: the
/// first edge in canonical order that returns to it, `setTo(0)` back to
/// value 0. A weak `inc` forces the loop to take `inc`, nearest first
/// (`0 -inc-> 1`), then back by the shortest path to the entry
/// (`1 -setTo(0)-> 0`).
#[trace("TC-521")]
#[test]
fn a_weak_constraint_forces_the_loop_through_the_operation() {
    let formula = "always eventually holds(c.value = 3)";
    let free = counter(document_with_ranged_parameter(), "", formula);
    let outcome = free.decide(is_three).expect("checked");
    let counterexample = violated(&outcome);
    assert_eq!(free.values(counterexample, "c"), [0, 0]);
    assert_eq!(
        lasso_of(counterexample),
        Lasso {
            entry: 0,
            terminal_stutter: false
        }
    );
    assert!(applies(&counterexample.prefix[0], "setTo"));
    assert!(counterexample.fairness.is_empty());

    for (fairness, each) in [
        ("fair weak M::Counter::inc;", false),
        ("fair weak whole M::Counter::inc;", false),
        ("fair weak each M::Counter::inc;", true),
    ] {
        let fixture = counter(document_with_ranged_parameter(), fairness, formula);
        let outcome = fixture.decide(is_three).expect("checked");
        let counterexample = violated(&outcome);
        assert_eq!(fixture.values(counterexample, "c"), [0, 1, 0], "{fairness}");
        assert_eq!(
            lasso_of(counterexample),
            Lasso {
                entry: 0,
                terminal_stutter: false
            }
        );
        assert!(applies(&counterexample.prefix[0], "inc"));
        assert!(applies(&counterexample.prefix[1], "setTo"));
        assert_eq!(counterexample.fairness.len(), 1);
        assert_weakly_fair(&fixture, counterexample, "inc", each);
        assert_eq!(counterexample.len(), 2);
    }
}

/// ADR-018 FA-1 and FA-6: over two counters a weak `whole` constraint on
/// `inc` is met by `inc` on either counter, a weak `each` constraint needs
/// both `inc(a)` and `inc(b)` in the loop (or disabled in it). The claim
/// over `c = a` (the first binding) is `always eventually holds(c.value =
/// 3)`: the lasso keeps `a` below 3.
#[trace("TC-521")]
#[test]
fn each_needs_every_transition_of_the_operation_and_whole_needs_one() {
    let formula = "always eventually holds(c.value = 3)";
    let starts = [("a", 0), ("b", 0)];
    for (fairness, each) in [
        ("fair weak whole M::Counter::inc;", false),
        ("fair weak each M::Counter::inc;", true),
    ] {
        let fixture = fixture(
            document_with_ranged_parameter(),
            CONTRACT,
            fairness,
            formula,
            &starts,
        );
        let outcome = fixture.decide(is_three).expect("checked");
        let counterexample = violated(&outcome);
        assert_eq!(counterexample.over_binding, Some(instance("a")));
        assert_weakly_fair(&fixture, counterexample, "inc", each);
        let a = fixture.values(counterexample, "a");
        let lasso = lasso_of(counterexample);
        assert!(
            a[lasso.entry..].iter().all(|value| *value != 3),
            "a never reaches 3 in the loop: {a:?}"
        );
        let loop_steps = &counterexample.prefix[lasso.entry..];
        let incs = loop_steps
            .iter()
            .filter(|step| applies(step, "inc"))
            .count();
        assert!(incs >= 1, "{fairness}: the loop takes inc");
    }
}

/// FR-125-AC-6's TP-4 half and FR-126-AC-9's TP-4 claim: when the letter is
/// undefined at position 2 (`6 mod (2 - c.value)` at value 2), `always
/// eventually holds(...)` returns the undefined-evaluation counterexample,
/// the prefix `0 -inc-> 1 -inc-> 2` with no lasso, not the terminal stutter
/// lasso. The same holds for an undefined precondition guard (ADR-018 UE-6):
/// the item is refuted at the expanded state, naming the guarded transition.
#[trace("TC-537")]
#[test]
fn an_undefined_letter_refutes_a_liveness_claim_before_any_lasso() {
    let formula = "always eventually holds(6 mod (2 - c.value) = 0)";
    let fixture = counter(document(false), "", formula);
    let outcome = fixture
        .decide(|v| (v != 2).then(|| 6 % (2 - v) == 0))
        .expect("checked");
    let counterexample = violated(&outcome);
    assert_eq!(fixture.values(counterexample, "c"), [0, 1, 2]);
    assert_eq!(counterexample.lasso, None);
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("expected an undefined evaluation: {counterexample:?}");
    };
    assert_eq!(evaluation.site.position, 2);
    assert!(matches!(evaluation.site.locus, UndefinedLocus::Atom(_)));
    assert_eq!(
        evaluation.cause,
        UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero)
    );
    assert_eq!(counterexample.trace_position(), Some(2));

    let guarded = "pre CanInc using v on M::Counter::inc { 6 mod (2 - self.value) >= 0 }\n\
                   post Inc using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n";
    let fixture = fixture_with_body(guarded, "always eventually holds(c.value = 0)");
    let outcome = fixture.decide(is_zero).expect("checked");
    let counterexample = violated(&outcome);
    assert_eq!(fixture.values(counterexample, "c"), [0, 1, 2]);
    assert_eq!(counterexample.lasso, None);
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
        "the guard names inc"
    );
}

fn fixture_with_body(body: &str, formula: &str) -> Fixture {
    fixture(document(false), body, "", formula, &[("c", 0)])
}

/// FR-126 "Explicit-state limits" for a TP-4 item over the Counter subject
/// and `always eventually holds(c.value = 0)` (the stutter lasso at value 3):
/// the lasso needs the depth-3 product state expanded, so a horizon of 3
/// settles `BoundReached{3}` and a horizon of 4 finds it, found by the
/// second phase over the retained graph although the frontier is not empty;
/// `max_states` 2 and `max_transitions` 3 stop the run naming the limit; an
/// already-cancelled handle stops it with no limit; a zero evaluation meter
/// stops it at `inc`'s precondition; `max_automaton_states` 1 stops it at
/// the negation's second state.
#[trace("TC-521")]
#[test]
fn limits_and_cancellation_stop_a_liveness_run_naming_what_stopped_it() {
    let fixture = counter(document(false), "", "always eventually holds(c.value = 0)");
    let defaults = ModelCheckLimits::default();
    let with = |max_depth, limits, meter, cancel: &Cancel| {
        fixture
            .run(is_zero, max_depth, limits, meter, cancel)
            .expect("checked")
    };
    let live = Cancel::new();
    assert!(matches!(
        with(Some(3), defaults, UNLIMITED, &live),
        ModelCheckOutcome::BoundReached { depth: 3, .. }
    ));
    assert_eq!(
        lasso_of(violated(&with(Some(4), defaults, UNLIMITED, &live))),
        Lasso {
            entry: 3,
            terminal_stutter: true
        }
    );
    let stopped = |outcome: ModelCheckOutcome| match outcome {
        ModelCheckOutcome::Stopped { cause, limit, .. } => (cause, limit),
        other => panic!("expected a stopped run, got {other:?}"),
    };
    let reached = |limit, value| {
        (
            IncompleteCause::ResourceExhausted,
            Some(ReachedLimit { limit, value }),
        )
    };
    let mut states = defaults;
    states.limits.max_states = 2;
    assert_eq!(
        stopped(with(None, states, UNLIMITED, &live)),
        reached(ModelCheckLimit::MaxStates, 2)
    );
    let mut transitions = defaults;
    transitions.limits.max_transitions = 3;
    assert_eq!(
        stopped(with(None, transitions, UNLIMITED, &live)),
        reached(ModelCheckLimit::MaxTransitions, 3)
    );
    let mut automaton = defaults;
    automaton.max_automaton_states = 1;
    assert_eq!(
        stopped(with(None, automaton, UNLIMITED, &live)),
        reached(ModelCheckLimit::MaxAutomatonStates, 1)
    );
    assert_eq!(
        stopped(with(None, defaults, ZERO, &live)),
        reached(ModelCheckLimit::EvaluationMeter, 0)
    );
    let cancelled = Cancel::new();
    cancelled.cancel(CancelCause::Requested);
    assert_eq!(
        stopped(with(None, defaults, UNLIMITED, &cancelled)),
        (
            IncompleteCause::Cancelled {
                source: CancelCause::Requested
            },
            None
        )
    );
}

/// FR-126 "Determinism" for a TP-4 item: two runs of one request give equal
/// outcomes and byte-equal counterexamples.
#[trace("TC-521")]
#[test]
fn two_liveness_runs_give_equal_outcomes() {
    let fixture = counter(
        document_with_ranged_parameter(),
        "fair weak each M::Counter::inc;",
        "always eventually holds(c.value = 3)",
    );
    let once = || fixture.decide(is_three).expect("checked");
    assert_eq!(once(), once());
}

/// Letters that read the counter an instance binds by its key.
struct ByKey(fn(&str, i64) -> bool);

impl ModelLetters for ByKey {
    fn reads(&self, _atom: qsl_semantics::check::FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        _atom: qsl_semantics::check::FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut quire_exact::Meter,
    ) -> Result<qsl_eval::temporal::AtomReading, quire_exact::Incomplete> {
        Ok(if (self.0)(&instance.key, value(state, &instance.key)) {
            qsl_eval::temporal::AtomReading::True
        } else {
            qsl_eval::temporal::AtomReading::False
        })
    }
}

/// FR-126 "Explicit-state limits": the model-only pass's states are charged
/// to the run's `max_states`, across instances. Over two counters, `eventually
/// holds(c.value = 1)` is read as `value = 1` for `a` and as always true for
/// `b`. For `b` the negation has no move at the first position, so `b` has no
/// product state and its model-only pass reads all 16 model states. The least
/// `max_states` that lets a run finish is then more than `a`'s product plus
/// those 16: `a`'s own model-only pass has been charged too, so the budget
/// `b` gets is what `a`'s product and pass left.
#[test]
fn the_model_only_pass_spends_the_budget_the_next_instance_gets() {
    let fixture = fixture(
        document(false),
        CONTRACT,
        "",
        "eventually holds(c.value = 1)",
        &[("a", 0), ("b", 0)],
    );
    let letters = ByKey(|object, value| object == "b" || value == 1);
    let run = |budget: usize| {
        let mut limits = ModelCheckLimits::default();
        limits.limits.max_states = budget;
        check_model(
            ModelCheckRequest {
                subject: subject(
                    &fixture.package,
                    &fixture.documents,
                    fixture.initial.clone(),
                    &fixture.keys,
                    UNLIMITED,
                ),
                item: ModelCheckItem::Clause {
                    clause: &fixture.clauses[0],
                    letters: &letters,
                },
                max_depth: None,
                limits,
                reductions: Default::default(),
            },
            &Cancel::new(),
        )
        .expect("checked")
    };
    let least = (1..=200)
        .find(|budget| !matches!(run(*budget), ModelCheckOutcome::Stopped { .. }))
        .expect("a budget finishes the run");
    let counted = run(least).stats().product_states;
    let counted = usize::try_from(counted).expect("small");
    // `b` has no product state, so the product states are `a`'s alone.
    assert!(
        least > counted + 16,
        "a's model-only pass is charged before b's: {least} against {counted} + 16"
    );
}
