// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-130 and FR-131 (ADR-019 FS-2 to FS-8, SF-4, SV-4): strong fairness on
//! the explicit-state product, and replay's check of it, over ADR-019 section
//! 6's mutex written over `Counter` (`crate::strong_hint::mutex`: `acquire(p)`
//! is the ranged `setTo`, `release` is `inc`) and its `Handoff` variant.
//!
//! Not covered: FR-130-AC-4 and FR-130-AC-5's certificates and the
//! `Holds{Exhaustive}` they ride on. A TP-4 proof still settles only with
//! FR-339's component certificate, which EN-1 does not build for weak
//! fairness either, so a proof under strong fairness is the run's
//! `ComponentCertificateNotBuilt` refusal carrying its statistics. The
//! tests below assert the proof's content through those statistics: that
//! the refinement found no fair accepting component, over the product
//! states the claim names.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

use ix_trace_rs::trace;
use qsl_analyze::model_check::{LivenessRefusal, ModelCheckRefusal};
use qsl_foundation::diagnostic::Code;
use qsl_replay::{
    settle_model_check, CertificateRequest, DisagreementCause, IncompleteCause, InconclusiveCause,
    ModelCheckItem, ModelCheckLimits, ModelCheckOutcome, ModelCheckRecord, ModelTraceRefusal,
    ModelTraceSettlement, ReplayRefusal, ResultLabel, RunStats, TemporalCounterexample,
    TerminalValue,
};
use quire_exact::{Cancel, CancelCause};
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};

use crate::counter::{document_with_ranged_parameter, subject, Letters, UNLIMITED};
use crate::lasso_replay::{found, refusal, replay_with};
use crate::liveness::{counter, fixture, is_zero, Fixture};
use crate::strong_hint::{is_one, mutex};

const WEAK_EACH: &str = "fair weak each M::Counter::setTo;";
const STRONG_EACH: &str = "fair strong each M::Counter::setTo;";
const STRONG_WHOLE: &str = "fair strong M::Counter::setTo;";

/// The outcome of `fixture`'s claim.
fn decided(fixture: &Fixture, atom: fn(i64) -> Option<bool>) -> ModelCheckOutcome {
    fixture.decide(atom).expect("checked")
}

/// The states the proof of `fixture`'s claim examined: the run's statistics
/// in the certificate refusal that stands for a proof.
fn proof_stats(fixture: &Fixture, atom: fn(i64) -> Option<bool>) -> RunStats {
    match fixture.decide(atom) {
        Err(ModelCheckRefusal::Liveness(LivenessRefusal::ComponentCertificateNotBuilt {
            stats,
        })) => stats,
        other => panic!("expected a proof awaiting its certificate, got {other:?}"),
    }
}

/// Process 1 holds the lock, or process 2 does.
fn is_one_or_two(value: i64) -> Option<bool> {
    Some(value == 1 || value == 2)
}

/// The `Handoff` mutex: owner `Int[0, 3]`, `acquire(p)` the ranged `setTo`
/// with `1 <= p <= 3` and precondition `owner = 0`, `release` (`inc`) with
/// precondition `owner != 0`, and `pass` with precondition `owner = 2 or
/// owner = 3`, setting the owner to `5 - pre(owner)` (written as the two cases, which
/// the range check can prove).
fn handoff(fairness: &str) -> Fixture {
    let mut domain: serde_json::Value =
        serde_json::from_slice(&document_with_ranged_parameter()).expect("JSON");
    let types = domain["types"].as_array_mut().expect("types");
    let counter = types
        .iter_mut()
        .find(|node| node["identity"] == "ix://test/counter/Counter")
        .expect("Counter");
    let operations = counter["operations"].as_array_mut().expect("operations");
    let mut pass = operations[0].clone();
    pass["identity"] = "ix://test/counter/Counter/pass".into();
    pass["name"] = "pass".into();
    operations.push(pass);
    let body = "pre Free using v on M::Counter::setTo { self.value = 0 and n >= 1 and n <= 3 }\n\
        post Takes using v on M::Counter::setTo { self.value = n }\n\
        pre Held using v on M::Counter::inc { self.value != 0 }\n\
        post Frees using v on M::Counter::inc { self.value = 0 }\n\
        pre Handed using v on M::Counter::pass { self.value = 2 or self.value = 3 }\n\
        post Passes using v on M::Counter::pass { (pre(self.value) = 2 and self.value = 3) or (pre(self.value) = 3 and self.value = 2) }\n";
    fixture(
        domain.to_string().into_bytes(),
        body,
        fairness,
        "always eventually holds(c.value = 1)",
        &[("c", 0)],
    )
}

/// The three-process mutex: `owner` in `0..=3`, `acquire(p)` for `p` in
/// `1..=3`, under the claim that process 1 or 2 holds it infinitely often.
fn three_processes(fairness: &str) -> Fixture {
    let body = "pre Free using v on M::Counter::setTo { self.value = 0 and n >= 1 and n <= 3 }\n\
        post Takes using v on M::Counter::setTo { self.value = n }\n\
        pre Held using v on M::Counter::inc { self.value != 0 }\n\
        post Frees using v on M::Counter::inc { self.value = 0 }\n";
    fixture(
        document_with_ranged_parameter(),
        body,
        fairness,
        "always eventually holds(c.value = 1 or c.value = 2)",
        &[("c", 0)],
    )
}

/// The owner values along a counterexample's positions.
fn owners(fixture: &Fixture, counterexample: &TemporalCounterexample) -> Vec<i64> {
    fixture.values(counterexample, "c")
}

/// FR-130-AC-1: ADR-019 section 6's mutex, `always eventually
/// holds(owner = 1)`. Under `weak each` on `acquire` it is refuted with an
/// empty stem and the loop `0 -acq(2)-> 2 -rel-> 0`; under `strong each` the
/// refinement leaves no fair accepting component, so the claim holds over the
/// 5 product states, although the product has no `acq(1)` edge out of
/// `(0, q1)`; under `strong` with no granularity it is refuted with the same
/// loop as under `weak each`.
#[trace("TC-531", "FR-130-AC-1")]
#[test]
fn strong_each_holds_where_weak_each_and_strong_whole_are_refuted() {
    let weak = mutex(WEAK_EACH);
    let outcome = decided(&weak, is_one);
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("expected a violation, got {outcome:?}");
    };
    assert_eq!(owners(&weak, counterexample), [0, 2, 0]);
    assert_eq!(counterexample.lasso.map(|lasso| lasso.entry), Some(0));

    let whole = mutex(STRONG_WHOLE);
    let outcome = decided(&whole, is_one);
    let ModelCheckOutcome::Violated {
        counterexample: strong,
        ..
    } = &outcome
    else {
        panic!("expected a violation, got {outcome:?}");
    };
    assert_eq!(owners(&whole, strong), [0, 2, 0]);
    assert_eq!(strong.lasso, counterexample.lasso);
    assert_eq!(
        strong.prefix, counterexample.prefix,
        "the same steps as under `weak each`"
    );

    let each = mutex(STRONG_EACH);
    assert_eq!(proof_stats(&each, is_one).product_states, 5);
}

/// FR-130-AC-2: the `Handoff` mutex under `strong each` on `acquire`. The
/// accepting SCC `{(0, q1), (2, q1), (3, q1)}` fails (c) for `acq(1)`, its
/// refinement removes `(0, q1)`, and the remainder `{(2, q1), (3, q1)}`
/// passes, so the engine returns the stem `0 -acq(2)-> 2` and the loop
/// `2 -pass-> 3 -pass-> 2`.
#[trace("TC-531", "FR-130-AC-2")]
#[test]
fn a_refined_remainder_passes_and_gives_the_lasso() {
    let fixture = handoff(STRONG_EACH);
    let outcome = decided(&fixture, is_one);
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("expected a violation, got {outcome:?}");
    };
    assert_eq!(owners(&fixture, counterexample), [0, 2, 3, 2]);
    assert_eq!(
        counterexample.lasso.map(|lasso| lasso.entry),
        Some(1),
        "the stem is the one acquisition"
    );
}

/// A run of `fixture`'s claim whose handle is cancelled at its `at`-th
/// charge.
fn cancelled_at(fixture: &Fixture, at: u32) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
    let seen = Arc::new(AtomicU32::new(0));
    let handle: Arc<OnceLock<Cancel>> = Arc::default();
    let (count, inner) = (Arc::clone(&seen), Arc::clone(&handle));
    let cancel = Cancel::observing(move || {
        if count.fetch_add(1, Ordering::Relaxed) + 1 == at {
            if let Some(cancel) = inner.get() {
                cancel.cancel(CancelCause::Requested);
            }
        }
    });
    assert!(handle.set(cancel.clone()).is_ok(), "set once");
    fixture.run(
        is_one,
        None,
        ModelCheckLimits::default(),
        UNLIMITED,
        &cancel,
    )
}

/// FR-130-AC-3: a `Cancel` handle cancelled when the second phase first
/// checks it stops the run with `Stopped{Cancelled, None}`; the fairness set
/// `weak each` and `strong each` on `acquire` returns the `strong each`
/// verdict; and the requests of AC-1 and AC-2, run twice, give equal outcomes
/// and byte-equal counterexamples.
///
/// The second phase's first check is the first charge after the whole
/// product is built: a run cancelled earlier stops with fewer product states
/// than the proof examines.
#[trace("TC-531", "FR-130-AC-3")]
#[test]
fn cancellation_mixed_kinds_and_determinism() {
    let mutex = mutex(STRONG_EACH);
    let full = proof_stats(&mutex, is_one);

    let charges = Arc::new(AtomicU32::new(0));
    let counter_of = Arc::clone(&charges);
    let counting = Cancel::observing(move || {
        counter_of.fetch_add(1, Ordering::Relaxed);
    });
    let uncancelled = mutex.run(
        is_one,
        None,
        ModelCheckLimits::default(),
        UNLIMITED,
        &counting,
    );
    assert!(uncancelled.is_err());
    let total = charges.load(Ordering::Relaxed);

    let stopped = |at| match cancelled_at(&mutex, at) {
        Ok(ModelCheckOutcome::Stopped {
            cause,
            limit,
            stats,
        }) => {
            assert_eq!(
                cause,
                IncompleteCause::Cancelled {
                    source: CancelCause::Requested
                }
            );
            assert_eq!(limit, None);
            stats
        }
        other => panic!("expected a cancelled run, got {other:?}"),
    };
    let first = (1..=total)
        .find(|at| stopped(*at) == full)
        .expect("a charge after the product is built");
    assert!(
        first > 1 && stopped(first - 1) != full,
        "the charge before it belongs to the first phase"
    );

    let mixed =
        self::mutex("fair weak each M::Counter::setTo; fair strong each M::Counter::setTo;");
    assert_eq!(proof_stats(&mixed, is_one), full);

    for fixture in [
        self::mutex(WEAK_EACH),
        self::mutex(STRONG_WHOLE),
        handoff(STRONG_EACH),
    ] {
        let once = decided(&fixture, is_one);
        assert_eq!(once, decided(&fixture, is_one));
        let ModelCheckOutcome::Violated { counterexample, .. } = &once else {
            panic!("expected a violation");
        };
        let again = found(&fixture, is_one);
        assert_eq!(format!("{counterexample:?}"), format!("{again:?}"));
    }
    assert_eq!(proof_stats(&mutex, is_one), full);
}

/// FR-130 "The filter" (the engine half of FR-130-AC-5) over its three-process mutex under `strong
/// each` on `acquire`: two strong constraints, `acq(1)` and `acq(2)`, fail on
/// the one SCC `{(0, q1), (3, q1)}` and one refinement removes the states
/// enabling either, so the claim holds. The certificate that proof carries
/// (AC-4, AC-5) is not built.
#[trace("TC-531")]
#[test]
fn two_strong_constraints_failing_on_one_component_refine_together() {
    let fixture = three_processes(STRONG_EACH);
    let stats = proof_stats(&fixture, is_one_or_two);
    assert!(stats.product_states > 0);
    // Under weak fairness the same claim is refuted, since `acq(3)` and `rel`
    // alone starve nothing weakly.
    let weak = three_processes(WEAK_EACH);
    assert!(matches!(
        decided(&weak, is_one_or_two),
        ModelCheckOutcome::Violated { .. }
    ));
}

/// The lasso EN-1 finds for the mutex under `weak each`, carried into the
/// clause `fairness` of the same claim: the steps are the model's, and the
/// envelope's fairness set is the clause's.
fn mutex_lasso_for(fairness: &str) -> (Fixture, TemporalCounterexample) {
    let weak = mutex(WEAK_EACH);
    let mut payload = found(&weak, is_one);
    let target = mutex(fairness);
    payload.fairness = target.clauses[0].fairness().to_vec();
    (target, payload)
}

/// FR-131-AC-1: the lasso `0 -acq(2)-> 2 -rel-> 0` reproduces in an envelope
/// for the `weak each` clause and for the `strong` (whole) clause; for the
/// `strong each` clause it refuses `UnfairLasso{Strong, acquire, Each}`,
/// `invalid_runtime_input`/`invalid-value`; the lasso with its last step
/// removed refuses with the loop-closure refusal, which is not `UnfairLasso`.
#[trace("TC-532", "FR-131-AC-1")]
#[test]
fn a_weakly_fair_lasso_is_refused_under_strong_each() {
    for fairness in [WEAK_EACH, STRONG_WHOLE] {
        let (fixture, payload) = mutex_lasso_for(fairness);
        replay_with(&fixture, payload, is_one).expect("reproduces");
    }

    let (fixture, payload) = mutex_lasso_for(STRONG_EACH);
    let error = replay_with(&fixture, payload, is_one).expect_err("unfair");
    let ReplayRefusal::UnfairLasso { constraint } = &error else {
        panic!("expected an unfair lasso, got {error:?}");
    };
    assert_eq!(constraint.kind(), FairnessKind::Strong);
    assert_eq!(constraint.operation().name(), "setTo");
    assert_eq!(constraint.granularity(), FairnessGranularity::Each);
    assert_eq!(error.code(), Code::InvalidRuntimeInput);
    assert_eq!(error.cause(), Some("invalid-value"));

    let (fixture, mut payload) = mutex_lasso_for(WEAK_EACH);
    payload.prefix.pop();
    let (cause, code, tag) = refusal(replay_with(&fixture, payload, is_one));
    assert!(
        matches!(cause, ModelTraceRefusal::LoopOpen { .. }),
        "{cause:?}"
    );
    assert_eq!((code, tag), (Code::InvalidRuntimeInput, "invalid-value"));
}

/// FR-131-AC-2: the `Handoff` lasso reproduces under its `strong each`
/// clause, since `acquire` is enabled at no loop state; and the terminal
/// stutter lasso over `Counter` reproduces in an envelope for the same claim
/// with `strong` on `inc` added, since nothing is enabled at a terminal
/// state.
#[trace("TC-532", "FR-131-AC-2")]
#[test]
fn a_loop_that_enables_nothing_starved_and_a_stutter_loop_are_fair() {
    let fixture = handoff(STRONG_EACH);
    let payload = found(&fixture, is_one);
    replay_with(&fixture, payload, is_one).expect("reproduces");

    let stutter = counter(
        crate::counter::document(false),
        "fair strong M::Counter::inc;",
        "always eventually holds(c.value = 0)",
    );
    let payload = found(&stutter, is_zero);
    assert!(payload.lasso.is_some_and(|lasso| lasso.terminal_stutter));
    let result = replay_with(&stutter, payload, is_zero).expect("reproduces");
    assert!(matches!(
        result.settlement,
        ModelTraceSettlement::Reproduced { .. }
    ));
}

/// The record the settlement map gives `outcome` as EN-1's for `fixture`'s
/// claim, its counterexample replayed through the fixture's own clause.
pub(crate) fn settle_en1(
    fixture: &Fixture,
    outcome: &ModelCheckOutcome,
    atom: fn(i64) -> Option<bool>,
) -> ModelCheckRecord {
    let letters = Letters { atom };
    settle_model_check(
        outcome,
        &ModelCheckLimits::default(),
        &crate::lasso_replay::package_id(fixture),
        &CertificateRequest {
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
        },
        &Cancel::new(),
        |payload| replay_with(fixture, payload.clone(), atom),
    )
}

/// FR-131-AC-3: a `Violated` outcome given to the settlement map as EN-1's,
/// carrying AC-1's lasso for the `strong each` clause, whose replay refuses it
/// with `UnfairLasso`, settles `inconclusive`, `ReplayParity`; the loop-closure
/// refusal of an EN-1 outcome settles `inconclusive`, `ReplayRefused`.
#[trace("TC-532", "FR-131-AC-3")]
#[test]
fn an_unfair_en1_lasso_settles_replay_parity() {
    let (fixture, payload) = mutex_lasso_for(STRONG_EACH);
    let settle = |payload: &TemporalCounterexample| {
        settle_en1(
            &fixture,
            &ModelCheckOutcome::Violated {
                counterexample: Box::new(payload.clone()),
                stats: RunStats::default(),
            },
            is_one,
        )
    };
    let record = settle(&payload);
    assert_eq!(record.label, ResultLabel::Inconclusive);
    assert!(
        matches!(
            record.value,
            TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(
                DisagreementCause::NoValue { .. }
            ))
        ),
        "{:?}",
        record.value
    );
    assert!(record.counterexample.is_none());

    let mut open = payload;
    open.prefix.pop();
    let record = settle(&open);
    assert_eq!(record.label, ResultLabel::Inconclusive);
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(Code::InvalidRuntimeInput))
    );
}

/// The weak case of FR-127-AC-3: the lasso EN-1 finds with no constraint
/// (loop `setTo(0)` alone), replayed in an envelope for the clause with the
/// weak `inc` constraint, which it fails, settles `inconclusive`,
/// `ReplayParity`, since EN-1 builds only fair lassos.
#[trace("TC-522")]
#[test]
fn a_weakly_unfair_en1_lasso_settles_replay_parity() {
    let free = counter(
        document_with_ranged_parameter(),
        "",
        "always eventually holds(c.value = 3)",
    );
    let mut payload = found(&free, |value| Some(value == 3));
    let weak = counter(
        document_with_ranged_parameter(),
        "fair weak M::Counter::inc;",
        "always eventually holds(c.value = 3)",
    );
    payload.fairness = weak.clauses[0].fairness().to_vec();
    let record = settle_en1(
        &weak,
        &ModelCheckOutcome::Violated {
            counterexample: Box::new(payload),
            stats: RunStats::default(),
        },
        |value| Some(value == 3),
    );
    assert_eq!(record.label, ResultLabel::Inconclusive);
    assert!(matches!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(_))
    ));
}
