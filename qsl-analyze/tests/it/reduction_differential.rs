// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-021 RV-10 (FR-160-AC-6, TC-886): the reduced-versus-unreduced
//! differential.
//!
//! Every case of the corpus below is checked unreduced and under each
//! selection of symmetry, partial-order reduction, or both, that the case's
//! preservation-table rows and subject admit. Every pair must settle the same
//! verdict, and every counterexample from either run must replay.
//!
//! The corpus is ADR-021 §7.1, §7.2, §7.5 and §7.6 and FR-156-AC-3's ring.
//! ADR-021 §7.4 is over `ProtocolSystem`, which `qsl-analyze` does not build
//! yet, and ADR-018's and ADR-019's worked examples are checked unreduced by
//! their own tests; neither is in this corpus. A liveness `Holds` that the
//! unreduced engine reaches is refused only for want of FR-339's component
//! certificate, after it has explored the whole product, so that refusal is
//! the unreduced run's `Holds` here. A pair in which either run ends at its
//! horizon or at a limit is not compared; none of these runs sets either.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{
    LivenessRefusal, ModelCheckRefusal, Reductions, SymmetryDeclaration,
};
use qsl_replay::{
    CounterexampleKind, ModelCheckLimits, ModelCheckOutcome, ModelTraceSettlement,
    TemporalCounterexample,
};

use crate::partial_order::{OPENS, REACHES_TWO};
use crate::por_world::{self, gate, ring, Guard};
use crate::reduction::CYCLE;
use crate::reduction_world::{atom, counters, start, Clause, Shape, World, INFINITE};

type Outcome = Result<ModelCheckOutcome, ModelCheckRefusal>;

/// What a pair of runs is compared by.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Verdict {
    Holds,
    Violated,
}

/// The ADR-018 verdict of an outcome, `None` for one the differential does
/// not compare.
fn verdict(outcome: &Outcome) -> Option<Verdict> {
    match outcome {
        Ok(ModelCheckOutcome::Holds { .. } | ModelCheckOutcome::HoldsReduced { .. })
        | Err(ModelCheckRefusal::Liveness(LivenessRefusal::ComponentCertificateNotBuilt {
            ..
        })) => Some(Verdict::Holds),
        Ok(ModelCheckOutcome::Violated { .. }) => Some(Verdict::Violated),
        _ => None,
    }
}

/// One model-check case: how to run it under a selection and how to replay a
/// counterexample of it.
struct Case {
    name: &'static str,
    /// Whether the subject declares a `symmetric` population.
    symmetric: bool,
    run: Box<dyn Fn(Reductions) -> Outcome>,
    replay: Box<dyn Fn(TemporalCounterexample) -> ModelTraceSettlement>,
}

fn symmetry() -> Vec<SymmetryDeclaration> {
    vec![SymmetryDeclaration {
        population: counters(),
        classes: vec![vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]],
    }]
}

/// The selections that admit a case: partial-order reduction, and symmetry
/// and both where the subject declares a `symmetric` population.
fn selections(case: &Case) -> Vec<(&'static str, Reductions)> {
    let mut selections = vec![(
        "partial-order",
        Reductions {
            partial_order: true,
            ..Reductions::default()
        },
    )];
    if case.symmetric {
        selections.push((
            "symmetry",
            Reductions {
                symmetry: symmetry(),
                ..Reductions::default()
            },
        ));
        selections.push((
            "symmetry and partial-order",
            Reductions {
                symmetry: symmetry(),
                partial_order: true,
                ..Reductions::default()
            },
        ));
    }
    selections
}

/// A case over a `reduction_world` subject: clause `0`, read by `atoms`, or
/// the deadlock-freedom item when `atoms` is `None`.
fn counter_case(
    name: &'static str,
    symmetric: bool,
    contracts: &'static str,
    formula: Option<&'static str>,
    atoms: fn() -> Vec<crate::reduction_world::Atom>,
) -> Case {
    let build = move || {
        let clauses: Vec<Clause<'_>> = formula
            .map(|formula| {
                vec![Clause {
                    profile: INFINITE,
                    formula,
                }]
            })
            .unwrap_or_default();
        World::new(
            Shape {
                symmetric,
                max: 2,
                link: false,
            },
            contracts,
            &clauses,
            &[start([0, 0, 0])],
            &["a", "b", "c"],
        )
    };
    let (world, replay_world) = (build(), build());
    let clause = formula.map(|_| 0);
    Case {
        name,
        symmetric,
        run: Box::new(move |selection| {
            world.run(
                clause,
                atoms(),
                selection,
                None,
                ModelCheckLimits::default(),
            )
        }),
        replay: Box::new(move |payload| match clause {
            Some(clause) => replay_world.replay_clause(clause, atoms(), payload),
            None => replay_world.replay(payload),
        }),
    }
}

/// A case over a `por_world` subject, clause `0` or the deadlock item.
fn fixture_case(
    name: &'static str,
    build: impl Fn() -> por_world::Fixture + 'static,
    clause: Option<usize>,
    atoms: fn() -> Vec<por_world::Atom>,
) -> Case {
    let (fixture, replay_fixture) = (build(), build());
    Case {
        name,
        symmetric: false,
        run: Box::new(move |selection| fixture.run(clause, atoms(), selection)),
        replay: Box::new(move |payload| replay_fixture.replay(clause, atoms(), payload)),
    }
}

fn reaches_two() -> Vec<crate::reduction_world::Atom> {
    vec![atom(|v| Some(v == 2))]
}

fn below_two() -> Vec<crate::reduction_world::Atom> {
    vec![atom(|v| Some(v < 2))]
}

fn not_two() -> Vec<crate::reduction_world::Atom> {
    vec![atom(|v| Some(v != 2))]
}

const BUMPS: &str = "pre Below using v on M::Counter::bump { self.v < 2 }\n\
     post Up using v on M::Counter::bump { self.v - pre(self.v) = 1 }\n";

fn done() -> Vec<por_world::Atom> {
    vec![Box::new(|state, key| {
        por_world::truth(state, "gates", key, "Gate/done")
    })]
}

fn below_two_q() -> Vec<por_world::Atom> {
    vec![Box::new(|state, key| {
        por_world::number(state, "counters", key, "Counter/v").map(|v| v < 2)
    })]
}

fn corpus() -> Vec<Case> {
    vec![
        // ADR-021 §7.1: three interchangeable configs.
        counter_case(
            "7.1 fair weak each: proof",
            true,
            CYCLE,
            Some("fair weak each M::Counter::bump; always eventually holds(x.v = 2)"),
            reaches_two,
        ),
        counter_case(
            "7.1 fair weak each: refutation",
            true,
            CYCLE,
            Some("fair weak each M::Counter::bump; eventually always holds(x.v != 2)"),
            not_two,
        ),
        counter_case(
            "7.1 fair weak whole: refutation",
            true,
            CYCLE,
            Some("fair weak M::Counter::bump; always eventually holds(x.v = 2)"),
            reaches_two,
        ),
        // ADR-021 §7.2: independent bumps.
        counter_case(
            "7.2 ReachesTwo",
            true,
            BUMPS,
            Some(REACHES_TWO),
            reaches_two,
        ),
        counter_case("7.2 deadlock-freedom", true, BUMPS, None, Vec::new),
        counter_case(
            "7.2 always below two",
            true,
            BUMPS,
            Some("always holds(x.v < 2)"),
            below_two,
        ),
        counter_case(
            "7.2 always at most two",
            true,
            BUMPS,
            Some("always holds(x.v <= 2)"),
            || vec![atom(|v| Some(v <= 2))],
        ),
        // FR-156-AC-3: a ring that would ignore a visible transition.
        fixture_case(
            "ring",
            || ring(&["always holds(x.v < 2)"]),
            Some(0),
            below_two_q,
        ),
        // ADR-021 §7.5: the Gate.
        fixture_case(
            "7.5 Opens fair weak go",
            || gate(Guard::Pre, false, &OPENS),
            Some(0),
            done,
        ),
        fixture_case(
            "7.5 Opens no fairness",
            || gate(Guard::Pre, false, &OPENS),
            Some(1),
            done,
        ),
        fixture_case(
            "7.5 Opens fair strong go",
            || gate(Guard::Pre, false, &OPENS),
            Some(3),
            done,
        ),
        fixture_case(
            "7.5 Opens fair weak each go",
            || gate(Guard::Pre, false, &OPENS),
            Some(2),
            done,
        ),
        // ADR-021 §7.6: complete enabling footprints.
        fixture_case(
            "7.6 Gate with the guard in the postcondition",
            || gate(Guard::Post, false, &OPENS),
            Some(0),
            done,
        ),
        fixture_case(
            "7.6 a guard in a postcondition",
            por_world::guard_in_postcondition,
            Some(0),
            por_world::no_alarm,
        ),
        fixture_case(
            "7.6 deleting the receiver",
            por_world::deleting_the_receiver,
            Some(0),
            por_world::not_bad,
        ),
        fixture_case(
            "7.6 a delete blocked by a reference",
            por_world::blocked_delete,
            Some(0),
            por_world::zero_x,
        ),
        fixture_case(
            "7.6 a navigation through a rewritten reference",
            por_world::navigation,
            Some(0),
            por_world::no_alarm,
        ),
        fixture_case(
            "7.6 a postcondition reading through result",
            por_world::result_read,
            Some(0),
            por_world::no_alarm,
        ),
    ]
}

/// FR-160-AC-6: every case of the corpus settles the same verdict unreduced
/// and under each selection that admits it, and every counterexample from
/// either run replays (an authored undefined evaluation, which FR-128
/// refuses, is held to its prefix by the case's own test).
#[trace("TC-886", "FR-160-AC-6")]
#[test]
fn reduced_and_unreduced_runs_agree_over_the_corpus() {
    let mut compared = 0;
    let mut verdicts = std::collections::BTreeSet::new();
    for case in corpus() {
        let unreduced = (case.run)(Reductions::default());
        let expected = verdict(&unreduced).unwrap_or_else(|| {
            panic!(
                "{}: the unreduced run is compared: {unreduced:?}",
                case.name
            )
        });
        verdicts.insert(format!("{expected:?}"));
        let mut runs = vec![("unreduced", unreduced)];
        for (label, selection) in selections(&case) {
            runs.push((label, (case.run)(selection)));
        }
        for (label, outcome) in &runs {
            assert_eq!(
                verdict(outcome),
                Some(expected),
                "{} under {label}: {outcome:?}",
                case.name
            );
            compared += 1;
            if let Ok(ModelCheckOutcome::Violated { counterexample, .. }) = outcome {
                if matches!(
                    counterexample.kind,
                    CounterexampleKind::UndefinedEvaluation(_)
                ) {
                    continue;
                }
                assert!(
                    matches!(
                        (case.replay)(counterexample.as_ref().clone()),
                        ModelTraceSettlement::Reproduced { .. }
                    ),
                    "{} under {label}: the counterexample replays",
                    case.name
                );
            }
        }
    }
    let expected: usize = corpus().iter().map(|case| 1 + selections(case).len()).sum();
    assert_eq!(compared, expected, "every run of every case is compared");
    assert_eq!(verdicts.len(), 2, "the corpus holds proofs and refutations");
}
