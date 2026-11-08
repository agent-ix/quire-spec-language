// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-329 "Bounded-profile clause over a zero-counted loop" (AC-8, AC-9,
//! AC-10; TC-839) over protocol-subject lassos built through FR-217's replay
//! (`replay_protocol_trace_with_temporal_selections`).
//!
//! The zero-counted subject is a `repeat` around a `parallel` whose branches
//! are empty sequences, so the only loop steps are `fork` and `join`
//! (ADR-027 PB-3: neither counts). A protocol system's atoms read the model
//! state, which a fork/join loop does not change, so a replayed lasso cannot
//! make one atom true at one loop position and false at another; the AC-8
//! row for that shape evaluates the lasso's real shape (its steps and their
//! `counts` flags, from the replayed counterexample) with a position-keyed
//! atom.

use std::cell::Cell;

use ix_trace_rs::trace;
use qsl_eval::model_subject::ModelLetters;
use qsl_eval::simulation::model::ExpansionLimits;
use qsl_eval::simulation::protocol::{ProtocolTransition, StepKind};
use qsl_eval::temporal::{
    evaluate_at, AtomReading, LetterStop, TemporalLetters, TemporalOutcome, TemporalPosition,
    TemporalRefusal, TemporalTrace,
};
use qsl_foundation::diagnostic::Code;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_replay::{
    replay_protocol_trace_with_temporal_selections, DisagreementCause, ModelCheckOutcome,
    ModelTraceRefusal, ModelTraceReplayResult, ModelTraceSettlement, ModelTraceSubject,
    ModelTraceValue, ProtocolTraceSubject, ReplayLimits, ReplayRefusal, TemporalCounterexample,
};
use qsl_semantics::check::FormulaNodeId;
use quire_exact::Meter;

use crate::counter::UNLIMITED;
use crate::fill::{attempt, contracts, document, instance, protocol, universe};
use crate::model_settle::{envelope_at, request};
use crate::protocol_counted_steps::{claim, lossy_channel, Case, CellV};

/// A `repeat` around a `parallel` of two empty branches: the loop is `fork`
/// then `join`.
const FORK_JOIN: &str = "repeat Loop by w visible () while { true } \
     parallel Pa { branch a sequence Ea { } branch b sequence Eb { } } join all [a, b];";

fn fork_join(formula: &str) -> Case {
    let unit = contracts("self.v <= 1") + &protocol("P", "", FORK_JOIN, "d");
    Case::new(&(unit + &claim(formula)))
}

fn is_one(value: i64) -> bool {
    value == 1
}

fn is_zero(value: i64) -> bool {
    value == 0
}

/// `payload` replayed as the first clause of `case`, a protocol `P` subject.
fn replay(
    case: &Case,
    letters: &dyn ModelLetters,
    payload: &TemporalCounterexample<ProtocolTransition>,
) -> Result<ModelTraceReplayResult, ReplayRefusal> {
    let unit = crate::fill::unit_text(&case.body).into_bytes();
    let node = &case.checked.graph().temporal_clauses()[0];
    let clause_node = node.identity().clone();
    let occurrence = OccurrenceKey::new(clause_node.clone(), node.claim().clone());
    let subject = case.subject("P");
    replay_protocol_trace_with_temporal_selections(
        request(
            &case.checked,
            &case.temporal_clock_selections,
            &document(),
            case.documents.snapshots.iter(),
            &unit,
        ),
        &ProtocolTraceSubject {
            model: ModelTraceSubject {
                timed: None,
                initial: case.initial.clone(),
                universes: universe(&["c"]),
                expansion: ExpansionLimits {
                    max_candidates: 1_000_000,
                },
            },
            protocol: "P".to_owned(),
            over: instance("c"),
            roles: subject.roles,
            memory: subject.memory,
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
        },
        &envelope_at(payload.clone(), clause_node, occurrence),
        letters,
        &case.temporal_clock_selections,
        ReplayLimits::default(),
    )
}

/// The counterexample EN-1 finds for `case`'s claim under `letters`.
fn refutation(
    case: &Case,
    letters: &dyn ModelLetters,
) -> TemporalCounterexample<ProtocolTransition> {
    match case.check("P", letters) {
        ModelCheckOutcome::Violated { counterexample, .. } => counterexample.as_ref().clone(),
        other => panic!("expected a violation, got {:?}", other.stats()),
    }
}

/// The replay of `payload` reproduced a violation at `position`.
fn assert_reproduced_at(result: Result<ModelTraceReplayResult, ReplayRefusal>, position: u64) {
    match result.expect("the lasso replays").settlement {
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            trace_position,
        } => assert_eq!(trace_position, Some(position)),
        other => panic!("expected a reproduction at {position}, got {other:?}"),
    }
}

/// The replay evaluated the clause `true` on the lasso, so it disagrees with
/// the refutation it was handed.
fn assert_clause_holds(result: Result<ModelTraceReplayResult, ReplayRefusal>) {
    use quire_semantic_value::catalog::Category;
    match result.expect("the lasso replays").settlement {
        ModelTraceSettlement::Inconclusive(DisagreementCause::Verdicts { replayed, .. }) => {
            assert_eq!(
                replayed,
                qsl_replay::Verdict::from_category(Category::Success)
            );
        }
        other => panic!("expected a true evaluation, got {other:?}"),
    }
}

/// The zero-counted `fork`/`join` lasso EN-1 finds for `eventually[1,1]
/// holds(k.v = 1)` (`p` false at every position), asserted to be one.
fn zero_counted_lasso() -> (Case, TemporalCounterexample<ProtocolTransition>) {
    let case = fork_join("eventually[1,1] holds(k.v = 1)");
    let payload = refutation(&case, &CellV(is_one));
    let lasso = payload.lasso.expect("a lasso");
    assert_eq!((lasso.entry, lasso.terminal_stutter), (0, false));
    let kinds: Vec<StepKind> = payload
        .prefix
        .iter()
        .map(|step| step.transition.kind())
        .collect();
    assert!(!kinds.is_empty());
    assert!(
        kinds
            .iter()
            .all(|kind| matches!(kind, StepKind::Fork | StepKind::Join)),
        "{kinds:?}"
    );
    assert!(payload.prefix.iter().all(|step| !step.transition.counts()));
    (case, payload)
}

/// Letters for the lasso of `payload` read by position: `p` holds at the
/// positions `at` names, each step counts as the replayed transition says.
struct LassoLetters {
    counted: Vec<bool>,
    p: fn(u64) -> bool,
    reads: Cell<u64>,
}

impl LassoLetters {
    fn of(payload: &TemporalCounterexample<ProtocolTransition>, p: fn(u64) -> bool) -> Self {
        Self {
            counted: payload
                .prefix
                .iter()
                .map(|step| step.transition.counts())
                .collect(),
            p,
            reads: Cell::new(0),
        }
    }

    /// The lasso's shape: no stem, the whole prefix is the loop.
    fn trace(&self) -> TemporalTrace {
        TemporalTrace::Lasso {
            prefix: 0,
            loop_len: u64::try_from(self.counted.len()).expect("a small loop"),
        }
    }
}

impl TemporalLetters for LassoLetters {
    fn atom(
        &mut self,
        _atom: FormulaNodeId,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        self.reads.set(self.reads.get() + 1);
        Ok(if (self.p)(position.index()) {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }

    fn activation(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        panic!("these claims have no `when` condition");
    }

    fn counts_step(&self, position: TemporalPosition) -> bool {
        let step = usize::try_from(position.index()).expect("a small position") - 1;
        self.counted[step]
    }

    fn counts_loop_edge(&self) -> bool {
        *self.counted.last().expect("a loop")
    }
}

fn at_origin(case: &Case, letters: &mut LassoLetters) -> Result<TemporalOutcome, TemporalRefusal> {
    evaluate_at(
        case.clause(),
        letters.trace(),
        TemporalPosition::new(0),
        letters,
        &mut Meter::new(UNLIMITED),
    )
}

/// FR-329-AC-8: over a fair protocol lasso whose loop is only `fork` and
/// `join`, entered at interval distance 0 and replayed under a bounded
/// profile: with `p` false at every position `eventually[1,1] holds(p)` is
/// `Completed(false)` at position 0; with `p` true at position 0
/// `eventually[0,1] holds(p)` is `Completed(true)`; and `eventually[1,1]`
/// stays false even where `p` holds at every position, because no position
/// lies one counted step away. `p` true only at a loop position, at
/// distance 0, `eventually[0,1] holds(p)` is `Completed(true)`.
#[trace("TC-839", "FR-329-AC-8")]
#[test]
fn a_zero_counted_protocol_lasso_decides_intervals_in_counted_steps() {
    let (false_case, payload) = zero_counted_lasso();
    assert!(
        !payload.fairness.is_empty(),
        "the default scheduler constraints bind the lasso: it is a fair one"
    );

    // `p` false at every position: no position at distance 1.
    assert_reproduced_at(replay(&false_case, &CellV(is_one), &payload), 0);

    // `p` true at position 0 (and every position): `[0,1]` is met at distance
    // 0, `[1,1]` is not, since the loop never reaches distance 1.
    let met = fork_join("eventually[0,1] holds(k.v = 0)");
    assert_clause_holds(replay(&met, &CellV(is_zero), &payload));
    let unreachable = fork_join("eventually[1,1] holds(k.v = 0)");
    assert_reproduced_at(replay(&unreachable, &CellV(is_zero), &payload), 0);

    // `p` true only at a loop position: positions of the replayed lasso, the
    // steps' real `counts` flags.
    let loop_len = payload.prefix.len();
    assert!(loop_len >= 2, "a fork and a join");
    let later = fork_join("eventually[0,1] holds(k.v = 1)");
    let mut letters = LassoLetters::of(&payload, |position| position == 1);
    assert_eq!(
        at_origin(&later, &mut letters),
        Ok(TemporalOutcome::Holds),
        "position 1 is at distance 0 from position 0"
    );
    let beyond = fork_join("eventually[1,1] holds(k.v = 1)");
    let mut letters = LassoLetters::of(&payload, |position| position == 1);
    assert_eq!(
        at_origin(&beyond, &mut letters),
        Ok(TemporalOutcome::Violated {
            at: TemporalPosition::new(0)
        })
    );
}

/// The body of a protocol whose loop takes one counted step per pass.
fn counted_loop(infinite: bool) -> Case {
    let run = format!(
        "repeat Loop by w visible () while {{ true }} {}",
        attempt("T", "touch", "TouchPre", "t", "true")
    );
    let unit = contracts("self.v <= 1") + &protocol("P", "", &run, "d");
    let claim = if infinite {
        "temporal Claim using t0 over (k: M::Cell) clock \"model-steps\" on origin \
         { always eventually holds(k.v = 1) }\n"
            .to_owned()
    } else {
        claim("eventually[0,1] holds(k.v = 1)")
    };
    Case::new(&(unit + &claim))
}

/// FR-329-AC-9: under a bounded profile the evaluator refuses a lasso that
/// is not zero-counted, `unsupported_construct`/`expression-form`, reading no
/// atom and giving no truth value: a lasso whose steps all count (an
/// observed lasso, and a model subject's, whose every step counts, ADR-027
/// PB-3), and a protocol lasso whose loop has one counted step, whether the
/// step is in the loop or closes it. The zero-counted loop of AC-8 is read.
#[trace("TC-839", "FR-329-AC-9")]
#[test]
fn a_bounded_clause_refuses_a_lasso_that_is_not_zero_counted() {
    let (case, zero) = zero_counted_lasso();
    let mut letters = LassoLetters::of(&zero, |_| true);
    assert!(at_origin(&case, &mut letters).is_ok());

    let refused = |counted: Vec<bool>| {
        let mut letters = LassoLetters {
            counted,
            p: |_| true,
            reads: Cell::new(0),
        };
        let result = at_origin(&case, &mut letters);
        let Err(refusal @ TemporalRefusal::LassoUnderBoundedProfile { .. }) = result else {
            panic!("expected a refusal, got {result:?}");
        };
        let code = refusal.catalog_code();
        assert_eq!(
            (code.code(), code.cause()),
            ("unsupported_construct", "expression-form")
        );
        assert_eq!(letters.reads.get(), 0, "no atom is read: no truth value");
    };
    // Observed and model-subject lassos: every step counts.
    refused(vec![true, true, true]);
    // A protocol lasso with one counted step in its loop, and with the
    // counted step closing it.
    refused(vec![false, true, false]);
    refused(vec![false, false, true]);
}

/// FR-329-AC-9, the replay of the protocol lasso whose loop has one counted
/// step (FR-217-AC-7): the `touch` loop's lasso, found for the same package
/// under the infinite-trace profile, replayed against the bounded clause
/// gives no value.
#[trace("TC-839", "FR-329-AC-9")]
#[test]
fn replaying_a_counted_protocol_lasso_under_a_bounded_clause_gives_no_value() {
    let infinite = counted_loop(true);
    let payload = refutation(&infinite, &CellV(is_one));
    assert!(payload.lasso.is_some());
    assert!(payload.prefix.iter().any(|step| step.transition.counts()));

    let bounded = counted_loop(false);
    // The bounded clause carries the package's scheduler set as the lasso does.
    let result = replay(&bounded, &CellV(is_one), &payload).expect("the lasso replays");
    match result.settlement {
        ModelTraceSettlement::Inconclusive(DisagreementCause::NoValue { .. }) => {}
        other => panic!("expected no value, got {other:?}"),
    }
}

/// FR-329-AC-10: under a bounded profile, a zero-counted protocol lasso that
/// is unfair for a scheduler constraint refuses `invalid_runtime_input`/
/// `invalid-value` naming the constraint; the same lasso replayed against the
/// package declaring `scheduling adversarial` is fair, and with `p` false at
/// every position `eventually[1,1] holds(p)` is `Completed(false)` at
/// position 0.
///
/// The lasso is `lossy_channel`'s: the stem is the counted `send`, the loop
/// is the uncounted `duplicate` and `lose`, and the thread resting before `A`
/// is enabled throughout.
#[trace("TC-839", "FR-329-AC-10")]
#[test]
fn an_unfair_zero_counted_lasso_refuses_naming_the_scheduler_constraint() {
    let adversarial = lossy_channel("scheduling adversarial;");
    let payload = refutation(&adversarial, &CellV(is_one));
    let lasso = payload.lasso.expect("a lasso");
    assert!(payload.fairness.is_empty(), "no scheduler constraint");
    assert!(payload.prefix[lasso.entry..]
        .iter()
        .all(|step| !step.transition.counts()));

    // The same lasso against the default scheduler: the first replay names
    // the set the package recompiles, the second carries it.
    let default = lossy_channel("");
    let stale = replay(&default, &CellV(is_one), &payload);
    let Err(ReplayRefusal::ModelTrace(stale)) = stale else {
        panic!("expected a fairness set refusal, got {stale:?}");
    };
    let ModelTraceRefusal::ProtocolFairnessSet { recompiled, .. } = *stale else {
        panic!("expected a fairness set refusal, got {stale:?}");
    };
    assert!(!recompiled.is_empty());
    let mut carried = payload.clone();
    carried.fairness.clone_from(&recompiled);
    let unfair = replay(&default, &CellV(is_one), &carried);
    let Err(error) = unfair else {
        panic!("expected an unfair lasso, got {unfair:?}");
    };
    let ReplayRefusal::ModelTrace(refusal) = &error else {
        panic!("expected a model-trace refusal, got {error:?}");
    };
    let ModelTraceRefusal::UnfairProtocolLasso { constraint } = refusal.as_ref() else {
        panic!("expected an unfair protocol lasso, got {refusal:?}");
    };
    assert!(
        recompiled.contains(constraint),
        "the named constraint is one of the package's: {constraint:?}"
    );
    assert_eq!(error.code(), Code::InvalidRuntimeInput);
    assert_eq!(error.cause(), Some("invalid-value"));

    // Against the adversarial package it is fair; `p` false throughout, the
    // distance-1 window holds no `p`.
    let window = adversarial_window();
    assert_reproduced_at(replay(&window, &CellV(is_one), &payload), 0);
}

/// `lossy_channel`'s package with the claim `eventually[1,1] holds(k.v = 1)`.
fn adversarial_window() -> Case {
    let members = "channel ch from w to w carries Boolean ordering fifo by (m: Boolean) \
                   { true } delivery unknown capacity 2 overflow block; scheduling adversarial;";
    let run = format!(
        "sequence Main {{ send S1 via ch as (s1: Boolean) {{ s1 }}; {} }}",
        attempt("A", "inc", "IncPre, IncPost", "a", "true")
    );
    let unit = contracts("self.v <= 1") + &protocol("P", members, &run, "d");
    Case::new(&(unit + &claim("eventually[1,1] holds(k.v = 1)")))
}
