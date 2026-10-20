// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-128 for an authored clause's counterexample: a lasso or a finite
//! prefix that EN-1 found over the `Counter` subject (`crate::counter`)
//! replays through `replay_model_trace`, which recompiles the unit, resolves
//! the clause by its `clause_node` and `occurrence_key`, re-executes the
//! steps, closes the loop, checks it against the fairness set and evaluates
//! the formula over it (ADR-018 CX-3, SM-8).
//!
//! Not covered: FR-128-AC-1's `ConfigVersion` and `Branch` subjects and
//! FR-128-AC-4's `upd(a)` lasso, which need subjects this crate's fixtures
//! do not have; the `interval` and `trace: "observed"` refusals of AC-3,
//! since the payload carries neither member yet; and the undefined-letter
//! counterexamples of AC-5 (TC-539), which this replay does not build.

use ix_trace_rs::trace;
use qsl_eval::simulation::model::ExpansionLimits;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::SourceIdentity;
use qsl_replay::{
    compile_package, replay_model_trace, settle_model_check, CertificateRequest, DependencyInput,
    DigestDomain, DigestRecord, DisagreementCause, InconclusiveCause, ModelCheckItem,
    ModelCheckLimits, ModelCheckOutcome, ModelTraceRefusal, ModelTraceReplayResult,
    ModelTraceSettlement, ModelTraceSubject, ModelTraceValue, ReplayLimits, ReplayRefusal,
    ResultLabel, TemporalCounterexample, TerminalValue, WireNodeId,
};
use quire_exact::Cancel;

use crate::counter::{
    document, document_with_ranged_parameter, package_from_text, subject, unit_text, universe,
    Documents, Letters, CONTRACT, UNLIMITED,
};
use crate::liveness::{counter, is_zero, Fixture};
use crate::model_settle::{envelope_at, request};

const FORMULA: &str = "always eventually holds(c.value = 3)";

fn is_three(value: i64) -> Option<bool> {
    Some(value == 3)
}

fn below_two(value: i64) -> Option<bool> {
    Some(value < 2)
}

/// A fixture's compiled `package_id`, as the replay request recompiles it.
fn package_id(fixture: &Fixture) -> DigestRecord {
    compile_package(
        SourceIdentity::new("agent-ix", "counter", "git", "1"),
        "counter.native",
        &fixture.unit,
        [fixture.documents.document.as_slice()],
        &DependencyInput::default(),
        qsl_replay::StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect("the unit compiles")
    .package_id()
}

/// The identities of the fixture's one clause, as an envelope carries them.
fn clause_identity(fixture: &Fixture) -> (WireNodeId, OccurrenceKey) {
    let node = &fixture.package.graph().temporal_clauses()[0];
    let id = WireNodeId::from_digest(*node.identity().as_bytes());
    (id, OccurrenceKey::new(id, node.claim().clone()))
}

fn subject_of(fixture: &Fixture) -> ModelTraceSubject {
    ModelTraceSubject {
        initial: fixture.initial.clone(),
        universes: universe(&fixture.keys),
        expansion: ExpansionLimits {
            max_candidates: 1_000_000,
        },
    }
}

/// Replay `payload` as the fixture's clause's counterexample, the clause's
/// atoms read by `atom`.
fn replay_with(
    fixture: &Fixture,
    payload: TemporalCounterexample,
    atom: fn(i64) -> Option<bool>,
) -> Result<ModelTraceReplayResult, ReplayRefusal> {
    let id = package_id(fixture);
    let (node, occurrence) = clause_identity(fixture);
    replay_model_trace(
        request(
            &fixture.documents.document,
            fixture.documents.snapshot_bytes(),
            &fixture.unit,
            id,
        ),
        &subject_of(fixture),
        &envelope_at(&fixture.unit, id, payload, node, occurrence),
        &Letters { atom },
        ReplayLimits::default(),
    )
}

/// The counterexample EN-1 finds for the fixture's clause.
fn found(fixture: &Fixture, atom: fn(i64) -> Option<bool>) -> TemporalCounterexample {
    match fixture.decide(atom).expect("checked") {
        ModelCheckOutcome::Violated { counterexample, .. } => counterexample.as_ref().clone(),
        other => panic!("expected a violation, got {other:?}"),
    }
}

fn reproduced(result: Result<ModelTraceReplayResult, ReplayRefusal>) -> Option<u64> {
    match result.expect("replays").settlement {
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            trace_position,
        } => trace_position,
        other => panic!("expected a reproduction, got {other:?}"),
    }
}

/// The strong-fairness hints of the replay of `payload` (FR-133).
pub(crate) fn hints_of(
    fixture: &Fixture,
    payload: &TemporalCounterexample,
    atom: fn(i64) -> Option<bool>,
) -> Vec<qsl_replay::StrongFairnessHint> {
    replay_with(fixture, payload.clone(), atom)
        .expect("replays")
        .hints
}

fn refusal(
    result: Result<ModelTraceReplayResult, ReplayRefusal>,
) -> (ModelTraceRefusal, Code, &'static str) {
    match result {
        Err(ReplayRefusal::ModelTrace(refusal)) => {
            let error = ReplayRefusal::ModelTrace(refusal.clone());
            (*refusal, error.code(), error.cause().expect("a cause"))
        }
        other => panic!("expected a model-trace refusal, got {other:?}"),
    }
}

/// Verifies part of FR-128-AC-1: the lasso whose loop is the terminal
/// stutter step at value 3 (FR-126-AC-3's last sentence) reproduces with
/// `trace_position` 0, and so does the lasso a weak `inc` constraint forces
/// through `inc` and back by `setTo(0)`. Replaying one envelope twice gives
/// equal results (FR-128-AC-4's last sentence).
#[trace("TC-523")]
#[test]
fn a_lasso_reproduces_with_its_formula_false_on_the_unrolled_trace() {
    let stutter = counter(document(false), "", "always eventually holds(c.value = 0)");
    let payload = found(&stutter, is_zero);
    assert_eq!(
        reproduced(replay_with(&stutter, payload.clone(), is_zero)),
        Some(0)
    );
    assert_eq!(
        replay_with(&stutter, payload.clone(), is_zero).expect("replays"),
        replay_with(&stutter, payload, is_zero).expect("replays")
    );

    let fair = counter(
        document_with_ranged_parameter(),
        "fair weak M::Counter::inc;",
        FORMULA,
    );
    let payload = found(&fair, is_three);
    assert_eq!(payload.fairness.len(), 1);
    assert_eq!(reproduced(replay_with(&fair, payload, is_three)), Some(0));
}

/// Verifies part of FR-128-AC-1: a finite prefix of a safety clause
/// (`always holds(c.value < 2)`, violated by `0 -inc-> 1 -inc-> 2`)
/// reproduces with `trace_position` 2, the prefix's last position.
#[trace("TC-523")]
#[test]
fn a_bad_prefix_reproduces_at_its_last_position() {
    let safety = counter(document(false), "", "always holds(c.value < 2)");
    let payload = found(&safety, below_two);
    assert_eq!(payload.lasso, None);
    assert_eq!(payload.prefix.len(), 2);
    assert_eq!(
        reproduced(replay_with(&safety, payload, below_two)),
        Some(2)
    );
}

/// Verifies part of FR-128-AC-4: a lasso on which the claim is true when
/// the replay reads the atoms (here every counter value reads as 3) settles
/// `inconclusive`, `ReplayParity` by `Verdicts`, not a reproduction.
#[trace("TC-523")]
#[test]
fn a_lasso_on_which_the_claim_holds_disagrees() {
    let fixture = counter(document_with_ranged_parameter(), "", FORMULA);
    let payload = found(&fixture, is_three);
    let ModelTraceSettlement::Inconclusive(cause) = replay_with(&fixture, payload, |_| Some(true))
        .expect("replays")
        .settlement
    else {
        panic!("a true claim disagrees with the refutation");
    };
    assert!(
        matches!(cause, DisagreementCause::Verdicts { .. }),
        "{cause:?}"
    );
}

/// Verifies part of FR-128-AC-3: the lasso with its last step removed does
/// not close (the loop is empty), a `terminal-stutter` loop on a state that
/// enables `inc` refuses, and a post-state digest altered refuses
/// `stale_dependency`/`content-mismatch` naming the step and the digest;
/// each refuses with no result.
#[trace("TC-523")]
#[test]
fn a_lasso_that_does_not_close_or_does_not_stutter_at_a_terminal_state_refuses() {
    let fixture = counter(document_with_ranged_parameter(), "", FORMULA);
    let payload = found(&fixture, is_three);
    assert_eq!(payload.prefix.len(), 1, "the free loop is `setTo(0)` alone");

    let mut open = payload.clone();
    open.prefix.pop();
    let (cause, code, tag) = refusal(replay_with(&fixture, open, is_three));
    assert_eq!(cause, ModelTraceRefusal::LoopOpen { entry: 0, steps: 0 });
    assert_eq!((code, tag), (Code::InvalidRuntimeInput, "invalid-value"));

    // A loop of model steps that ends where it did not begin.
    let stutter_fixture = counter(document(false), "", "always eventually holds(c.value = 0)");
    let mut ends = found(&stutter_fixture, is_zero);
    ends.lasso = Some(qsl_replay::Lasso {
        entry: 0,
        terminal_stutter: false,
    });
    let (cause, ..) = refusal(replay_with(&stutter_fixture, ends, is_zero));
    assert_eq!(cause, ModelTraceRefusal::LoopOpen { entry: 0, steps: 3 });

    let mut stutter = payload.clone();
    stutter.prefix.clear();
    stutter.lasso = Some(qsl_replay::Lasso {
        entry: 0,
        terminal_stutter: true,
    });
    let (cause, code, _) = refusal(replay_with(&fixture, stutter, is_three));
    assert_eq!(cause, ModelTraceRefusal::StutterNotTerminal { position: 0 });
    assert_eq!(code, Code::InvalidRuntimeInput);

    let mut altered = payload;
    let recorded = DigestRecord::mint(DigestDomain::SimulationStateKeyV1, [9; 32]);
    altered.prefix[0].post_state = recorded;
    let (cause, code, tag) = refusal(replay_with(&fixture, altered, is_three));
    assert_eq!(cause, ModelTraceRefusal::PostState { step: 0, recorded });
    assert_eq!((code, tag), (Code::StaleDependency, "content-mismatch"));
}

/// Verifies part of FR-128-AC-3: the lasso EN-1 finds with no constraint
/// (loop `setTo(0)` alone), in an envelope for the clause with the weak
/// `inc` constraint, carrying that clause's identities and fairness set, is
/// unfair, and refuses `UnfairLasso` naming that constraint, whose code is
/// `invalid_runtime_input`/`invalid-value`. The same payload with its
/// fairness set left empty names another set than the clause's and refuses
/// `stale_dependency`/`content-mismatch`.
#[trace("TC-523")]
#[test]
fn an_unfair_lasso_refuses_naming_the_constraint() {
    let free = counter(document_with_ranged_parameter(), "", FORMULA);
    let fair = counter(
        document_with_ranged_parameter(),
        "fair weak M::Counter::inc;",
        FORMULA,
    );
    let mut payload = found(&free, is_three);
    assert!(payload.fairness.is_empty());

    let (cause, code, tag) = refusal(replay_with(&fair, payload.clone(), is_three));
    assert!(
        matches!(&cause, ModelTraceRefusal::FairnessSet { payload, recompiled }
            if payload.is_empty() && recompiled.len() == 1),
        "{cause:?}"
    );
    assert_eq!((code, tag), (Code::StaleDependency, "content-mismatch"));

    let clause = fair.clauses[0].fairness().to_vec();
    payload.fairness.clone_from(&clause);
    let refused = replay_with(&fair, payload, is_three);
    let Err(error @ ReplayRefusal::UnfairLasso { .. }) = refused else {
        panic!("expected an unfair lasso, got {refused:?}");
    };
    let ReplayRefusal::UnfairLasso { constraint } = &error else {
        unreachable!("matched above");
    };
    assert_eq!(constraint, &clause[0]);
    assert_eq!(error.code(), Code::InvalidRuntimeInput);
    assert_eq!(error.cause(), Some("invalid-value"));
}

/// Verifies part of FR-128-AC-4: a `clause_node` that names no clause of the
/// recompiled package refuses `stale_dependency`/`content-mismatch`, and so
/// does the clause's own node under another occurrence.
#[trace("TC-523")]
#[test]
fn a_counterexample_names_its_clause_by_node_and_occurrence() {
    let fixture = counter(document_with_ranged_parameter(), "", FORMULA);
    let payload = found(&fixture, is_three);
    let id = package_id(&fixture);
    let (node, occurrence) = clause_identity(&fixture);
    let replay_at = |node, occurrence| {
        replay_model_trace(
            request(
                &fixture.documents.document,
                fixture.documents.snapshot_bytes(),
                &fixture.unit,
                id,
            ),
            &subject_of(&fixture),
            &envelope_at(&fixture.unit, id, payload.clone(), node, occurrence),
            &Letters { atom: is_three },
            ReplayLimits::default(),
        )
    };
    assert!(replay_at(node, occurrence.clone()).is_ok());

    let other = WireNodeId::from_digest([7; 32]);
    let other_key = OccurrenceKey::new(other, occurrence.origin().clone());
    let (cause, code, tag) = refusal(replay_at(other, other_key));
    assert_eq!(
        cause,
        ModelTraceRefusal::UnknownTemporalClause { clause_node: other }
    );
    assert_eq!((code, tag), (Code::StaleDependency, "content-mismatch"));

    let moved = OccurrenceKey::new(
        node,
        quire_exact::Origin::new(quire_exact::Role::new("claim"), 9),
    );
    let (cause, ..) = refusal(replay_at(node, moved));
    assert!(
        matches!(cause, ModelTraceRefusal::ItemIdentity { .. }),
        "{cause:?}"
    );
}

/// FR-127 V-4 for a TP-4 outcome: the lasso EN-1 returns settles `refuted`
/// only through `replay_model_trace` (ADR-018 SM-7), as it settled `failed`
/// while a formula counterexample could not be replayed.
#[trace("TC-523")]
#[test]
fn a_liveness_violation_settles_refuted_through_replay() {
    let fixture = counter(document_with_ranged_parameter(), "", FORMULA);
    let outcome = fixture.decide(is_three).expect("checked");
    let id = package_id(&fixture);
    let (node, occurrence) = clause_identity(&fixture);
    let letters = Letters { atom: is_three };
    let record = settle_model_check(
        &outcome,
        &ModelCheckLimits::default(),
        &id,
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
        |counterexample| {
            replay_model_trace(
                request(
                    &fixture.documents.document,
                    fixture.documents.snapshot_bytes(),
                    &fixture.unit,
                    id,
                ),
                &subject_of(&fixture),
                &envelope_at(
                    &fixture.unit,
                    id,
                    counterexample.clone(),
                    node,
                    occurrence.clone(),
                ),
                &letters,
                ReplayLimits::default(),
            )
        },
    );
    assert_eq!(record.value, TerminalValue::Refuted);
    assert_eq!(record.label, ResultLabel::Refuted);
    assert!(record.counterexample.is_some());

    let stale = settle_model_check(
        &outcome,
        &ModelCheckLimits::default(),
        &id,
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
        |counterexample| {
            let mut open = counterexample.clone();
            open.prefix.pop();
            replay_model_trace(
                request(
                    &fixture.documents.document,
                    fixture.documents.snapshot_bytes(),
                    &fixture.unit,
                    id,
                ),
                &subject_of(&fixture),
                &envelope_at(&fixture.unit, id, open, node, occurrence.clone()),
                &letters,
                ReplayLimits::default(),
            )
        },
    );
    assert!(
        matches!(
            stale.value,
            TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(
                Code::InvalidRuntimeInput
            ))
        ),
        "{:?}",
        stale.value
    );
}

/// A `Counter` fixture whose one clause `formula` is under the event-position
/// false-extension profile, `on origin`.
pub(crate) fn bounded_fixture(formula: &str) -> Fixture {
    let doc = document(false);
    let body = CONTRACT.to_owned()
        + &format!(
            "temporal C using tb over (c: M::Counter) clock \"model-steps\" on origin {{ {formula} }}\n"
        );
    let text = unit_text(&doc, &body).replace(
        "profile t0 = \"quire.temporal.infinite-trace/v1\";\n",
        "profile tb = \"quire.temporal.event-position.false-extension/v1\";\n",
    );
    let package = package_from_text(&doc, &text);
    let clauses = package
        .graph()
        .temporal_clauses()
        .iter()
        .map(|node| node.clause().clone())
        .collect();
    let mut documents = Documents::new(doc);
    let initial = vec![documents.start("s0", &[("c", 0)])];
    Fixture {
        package,
        clauses,
        documents,
        initial,
        keys: vec!["c"],
        unit: text.into_bytes(),
    }
}

/// A bad prefix refutes a safety clause whatever its fairness set says of
/// the continuation: `fair weak inc; always holds(c.value < 2)` is refuted by
/// `0 -inc-> 1 -inc-> 2`, which reproduces at position 2.
#[trace("TC-523")]
#[test]
fn a_bad_prefix_of_a_fair_safety_clause_reproduces() {
    let safety = counter(
        document(false),
        "fair weak M::Counter::inc;",
        "always holds(c.value < 2)",
    );
    let payload = found(&safety, below_two);
    assert_eq!(payload.lasso, None);
    assert_eq!(payload.fairness.len(), 1);
    assert_eq!(
        reproduced(replay_with(&safety, payload, below_two)),
        Some(2)
    );
}

/// An undefined-evaluation counterexample of an authored clause is refused
/// not yet implemented (FR-128-AC-5 is not built) rather than checked
/// against the deadlock item's rules and settled as an engine disagreement.
#[trace("TC-523")]
#[test]
fn an_authored_clauses_undefined_evaluation_is_refused_until_it_is_built() {
    fn undefined_at_two(value: i64) -> Option<bool> {
        (value != 2).then_some(true)
    }
    let fixture = counter(document(false), "", "always holds(c.value >= 0)");
    let payload = found(&fixture, undefined_at_two);
    assert!(matches!(
        payload.kind,
        qsl_replay::CounterexampleKind::UndefinedEvaluation(_)
    ));
    let (cause, code, tag) = refusal(replay_with(&fixture, payload, undefined_at_two));
    assert_eq!(cause, ModelTraceRefusal::AuthoredUndefined);
    assert_eq!(
        (code, tag),
        (Code::UnsupportedConstruct, "not-yet-implemented")
    );
}

/// Under a bounded profile a finite prefix reads as a closed trace, so a
/// prefix that neither ends at a terminal state nor covers the horizon from
/// the activation it refutes shows nothing (ADR-018 CX-1). `always[0,2]
/// holds(c.value >= 0)` holds over `Counter`, yet an empty prefix at the
/// non-terminal initial state reads false past its end: it refuses, and
/// `always[0,2] holds(c.value < 2)`'s own two-step prefix, which covers the
/// horizon, reproduces while its one-step truncation refuses.
#[trace("TC-523")]
#[test]
fn a_bounded_prefix_that_is_cut_short_refuses_and_a_covering_one_reproduces() {
    let holds = bounded_fixture("always[0,2] holds(c.value >= 0)");
    assert!(matches!(
        holds.decide(|value| Some(value >= 0)).expect("checked"),
        ModelCheckOutcome::Holds { .. }
    ));
    let binding = found(
        &counter(document(false), "", "always holds(c.value < 2)"),
        below_two,
    )
    .over_binding;
    let empty = TemporalCounterexample {
        initial_state: 0,
        prefix: Vec::new(),
        over_binding: binding,
        fairness: Vec::new(),
        kind: qsl_replay::CounterexampleKind::Formula,
        lasso: None,
        blocked: Vec::new(),
    };
    let (cause, code, tag) = refusal(replay_with(&holds, empty, |value| Some(value >= 0)));
    assert!(
        matches!(
            cause,
            ModelTraceRefusal::PrefixShort {
                horizon: 2,
                positions: 1
            }
        ),
        "{cause:?}"
    );
    assert_eq!((code, tag), (Code::InvalidRuntimeInput, "invalid-value"));

    let violated = bounded_fixture("always[0,2] holds(c.value < 2)");
    let payload = found(&violated, below_two);
    assert_eq!(payload.prefix.len(), 2);
    assert_eq!(
        reproduced(replay_with(&violated, payload.clone(), below_two)),
        Some(2)
    );
    let mut cut = payload;
    cut.prefix.pop();
    let (cause, ..) = refusal(replay_with(&violated, cut, below_two));
    assert!(
        matches!(cause, ModelTraceRefusal::PrefixShort { .. }),
        "{cause:?}"
    );

    // EN-1 stops at the first position where the formula is already decided
    // false, often before its horizon: such a prefix is a refutation for
    // every continuation and reproduces. `always[0,1] eventually[0,2]
    // holds(c.value = 3)` (horizon 3) is decided by `0 -inc-> 1 -inc-> 2`,
    // and `holds(c.value = 1) until[0,3] holds(c.value = 2)` by the empty
    // prefix.
    let is_three: fn(i64) -> Option<bool> = |value| Some(value == 3);
    let one_or_two: fn(i64) -> Option<bool> = |value| Some(value == 1 || value == 2);
    for (formula, atom, steps) in [
        (
            "always[0,1] eventually[0,2] holds(c.value = 3)",
            is_three,
            2,
        ),
        (
            "holds(c.value = 1) until[0,3] holds(c.value = 2)",
            one_or_two,
            0,
        ),
    ] {
        let decided = bounded_fixture(formula);
        let payload = found(&decided, atom);
        assert_eq!(payload.prefix.len(), steps, "{formula}");
        assert_eq!(payload.lasso, None);
        assert!(
            reproduced(replay_with(&decided, payload, atom)).is_some(),
            "{formula}"
        );
    }
}
