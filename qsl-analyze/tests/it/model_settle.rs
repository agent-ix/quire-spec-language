// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-127 and FR-128 over the Counter subject (`crate::counter`): an EN-1
//! outcome settles as one terminal record, and a deadlock-freedom
//! counterexample settles `refuted` only after `replay_model_trace`
//! recompiles the unit, re-admits the subject from the byte provision and
//! reproduces it.
//!
//! Not covered: a counterexample of an authored clause (no recompiled
//! package holds a temporal clause node until FR-337, so its replay
//! refuses), one over a unit with a `terminal` declaration (S4 refuses the
//! declaration as not emitted, so the recompile refuses), the
//! `ReplaySource::ModelTrace` arm (the envelope carries a witness source
//! the model-trace replay does not read), and a `Proved{Exhaustive,
//! Certified}` settlement, which waits on the closure certificate's core
//! checker.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_model, ModelCheckItem, ModelCheckRequest};
use qsl_eval::simulation::model::ExpansionLimits;
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::diagnostic::{Category, Code};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::SourceIdentity;
use qsl_replay::{
    compile_package, replay_model_trace, settle_model_check, Certification, CounterexampleKind,
    DependencyInput, DigestDomain, DigestRecord, DisagreementCause, Execution, IncompleteCause,
    InconclusiveCause, InternalFault, ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome,
    ModelTraceRefusal, ModelTraceReplayResult, ModelTraceSettlement, ModelTraceSubject,
    ModelTraceValue, ProofBasis, QualifiedName, ReachedLimit, ReplayLimits, ReplayRefusal,
    ReplayRequestWire, ReplaySource, ResultLabel, RunStats, SettlementBasis, StateEnvironment,
    TemporalCounterexample, TerminalValue, Truth, UndecidedCause, UndefinedLocus, WireNodeId,
    Witness, WitnessEnvelope, WitnessPacket, EXPLICIT_STATE,
};
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{Cancel, CancelCause, Identifier, Origin, Role};

use crate::counter::{document, document_digest, package, subject, universe, unit_text, Documents, CONTRACT, UNLIMITED};

const AUTHORITY: &str = "agent-ix";
const IDENTITY: &str = "counter";

/// A compiled Counter unit with its documents, initial snapshots and the
/// engine's deadlock-freedom outcome over universe `{c}`.
struct Case {
    unit: Vec<u8>,
    documents: Documents,
    initial: Vec<DocumentRef>,
    package_id: DigestRecord,
    outcome: ModelCheckOutcome,
}

fn case(body: &str, values: &[i64]) -> Case {
    let document = document(false);
    let unit = unit_text(&document, body).into_bytes();
    let package_id = compile_package(
        SourceIdentity::new(AUTHORITY, IDENTITY, "git", "1"),
        "counter.native",
        &unit,
        [document.as_slice()],
        &DependencyInput::default(),
        qsl_replay::StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect("the unit compiles")
    .package_id();
    let checked = package(&document, body);
    let mut documents = Documents::new(document);
    let initial: Vec<DocumentRef> = values
        .iter()
        .enumerate()
        .map(|(index, value)| documents.start(&format!("s{index}"), &[("c", *value)]))
        .collect();
    let outcome = check_model(
        ModelCheckRequest {
            subject: subject(&checked, &documents, initial.clone(), &["c"], UNLIMITED),
            item: ModelCheckItem::DeadlockFreedom,
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("checked");
    Case {
        unit,
        documents,
        initial,
        package_id,
        outcome,
    }
}

fn source_digest(bytes: &[u8]) -> DigestRecord {
    DigestRecord::mint(
        DigestDomain::SourceBytesV1,
        qsl_foundation::ByteDigest::of(bytes).as_bytes(),
    )
}

/// A source reference as the request and the envelope spell it.
type SourceWire = (String, String, String, String, Option<String>, String);

fn source_wire(bytes: &[u8]) -> SourceWire {
    (
        AUTHORITY.to_owned(),
        IDENTITY.to_owned(),
        "git".to_owned(),
        "1".to_owned(),
        Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
        source_digest(bytes).hex(),
    )
}

fn witness_source() -> ReplaySource {
    ReplaySource::Witness(Witness::parse("<<<assertion|model_check|deadlock|>>>").expect("a transcript"))
}

fn selected() -> QualifiedName {
    QualifiedName::new(vec![Identifier::new("Noop").expect("an identifier")]).expect("a name")
}

/// FR-098's request for `unit` at `package_id`, with the domain package and
/// the snapshots in the byte provision.
fn request(case: &Case, unit: &[u8], package_id: DigestRecord) -> ReplayRequestWire {
    let sha256_jcs = |digest: [u8; 32]| {
        (
            Some(DigestDomain::Sha256Jcs.as_str().to_owned()),
            DigestRecord::mint(DigestDomain::Sha256Jcs, digest).hex(),
        )
    };
    let source = source_digest(unit);
    let mut byte_provision = vec![
        (
            Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
            source.hex(),
            unit.to_vec(),
        ),
        {
            let (domain, hex) = sha256_jcs(document_digest(&case.documents.document));
            (domain, hex, case.documents.document.clone())
        },
    ];
    for (digest, bytes) in case.documents.snapshot_bytes() {
        let (domain, hex) = sha256_jcs(*digest);
        byte_provision.push((domain, hex, bytes.clone()));
    }
    ReplayRequestWire {
        profile_selections: Vec::new(),
        package_id: (
            Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
            package_id.hex(),
        ),
        source_digests: vec![source_wire(unit)],
        dependencies: Vec::new(),
        selected_function: selected(),
        source: witness_source(),
        obligation_identity: [3; 32],
        backend: "qsl-en-1".to_owned(),
        state_environment: StateEnvironment::new(Vec::new()),
        accounting_limits: UNLIMITED,
        stage_limits: std::collections::BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision,
    }
}

/// The envelope carrying `payload` at `package_id`, from `unit`.
fn envelope(
    unit: &[u8],
    package_id: DigestRecord,
    payload: TemporalCounterexample,
) -> WitnessEnvelope<TemporalCounterexample> {
    let node = WireNodeId::from_digest([7; 32]);
    WitnessEnvelope::reconstruct(
        WitnessPacket {
            obligation_identity: Some([3; 32]),
            occurrence_key: Some(OccurrenceKey::new(
                node,
                Origin::new(Role::new("deadlock-freedom"), 0),
            )),
            clause_node: Some(node),
            selected_function: Some(selected()),
            package_id: Some((
                Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
                package_id.hex(),
            )),
            source_digests: Some(vec![source_wire(unit)]),
            profile_selections: Some(Vec::new()),
            run_limits: Some(UNLIMITED),
            declared_domains: Some(Vec::new()),
            backend: Some("qsl-en-1".to_owned()),
            trace_position: Some(None),
            source: Some(witness_source()),
            family_payload: Some(payload),
        },
        ReplayLimits::default(),
    )
    .expect("a complete packet reconstructs")
}

fn trace_subject(case: &Case) -> ModelTraceSubject {
    ModelTraceSubject {
        initial: case.initial.clone(),
        universes: universe(&["c"]),
        expansion: ExpansionLimits {
            max_candidates: 1_000_000,
        },
    }
}

fn replay(case: &Case, payload: TemporalCounterexample) -> Result<ModelTraceReplayResult, ReplayRefusal> {
    replay_model_trace(
        request(case, &case.unit, case.package_id),
        &trace_subject(case),
        &envelope(&case.unit, case.package_id, payload),
        ReplayLimits::default(),
    )
}

fn counterexample(case: &Case) -> TemporalCounterexample {
    match &case.outcome {
        ModelCheckOutcome::Violated { counterexample, .. } => counterexample.as_ref().clone(),
        other => panic!("expected a violation, got {other:?}"),
    }
}

/// FR-128-AC-2: FR-126-AC-3's deadlock counterexample reproduces, the
/// item's value `false`; truncated to end at value 2, where `inc` is
/// enabled, it settles `inconclusive` by `Verdicts`. Replaying one envelope
/// twice gives equal results (FR-128-AC-4).
#[trace("TC-523")]
#[test]
fn a_deadlock_counterexample_reproduces_and_a_truncated_one_disagrees() {
    let case = case(CONTRACT, &[0]);
    let payload = counterexample(&case);
    let replayed = replay(&case, payload.clone()).expect("replays");
    assert_eq!(
        replayed.settlement,
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            trace_position: None,
        }
    );
    assert_eq!(replay(&case, payload.clone()).expect("replays"), replayed);

    let mut truncated = payload;
    truncated.prefix.pop();
    let ModelTraceSettlement::Inconclusive(cause) =
        replay(&case, truncated).expect("replays").settlement
    else {
        panic!("a truncated deadlock disagrees");
    };
    assert!(matches!(cause, DisagreementCause::Verdicts { .. }), "{cause:?}");
    assert_eq!(cause.replayed().category(), Category::Success);
}

fn refusal(result: Result<ModelTraceReplayResult, ReplayRefusal>) -> (ModelTraceRefusal, Code, &'static str) {
    match result {
        Err(ReplayRefusal::ModelTrace(refusal)) => {
            let error = ReplayRefusal::ModelTrace(refusal.clone());
            (*refusal, error.code(), error.cause().expect("a cause"))
        }
        other => panic!("expected a model-trace refusal, got {other:?}"),
    }
}

/// FR-128-AC-3 (prefix half): a post-state digest altered refuses
/// `stale_dependency`/`content-mismatch` naming the step and the digest; a
/// step appended past the terminal state is not enabled and refuses
/// `invalid_runtime_input`/`invalid-value` naming the step; an
/// `initial_state` of 1 over a one-snapshot subject refuses
/// `invalid-value`; a `Formula` counterexample names an authored clause no
/// recompiled package holds and refuses `content-mismatch`; and a source
/// edit refuses by FR-098's stale-package rule (FR-128-AC-4).
#[trace("TC-523")]
#[test]
fn a_counterexample_the_model_does_not_take_refuses_with_no_result() {
    let case = case(CONTRACT, &[0]);
    let payload = counterexample(&case);

    let mut altered = payload.clone();
    let recorded = DigestRecord::mint(DigestDomain::SimulationStateKeyV1, [9; 32]);
    altered.prefix[1].post_state = recorded;
    let (cause, code, tag) = refusal(replay(&case, altered));
    assert_eq!(cause, ModelTraceRefusal::PostState { step: 1, recorded });
    assert_eq!((code, tag), (Code::StaleDependency, "content-mismatch"));

    let mut extended = payload.clone();
    let last = extended.prefix[2].clone();
    extended.prefix.push(last.clone());
    let (cause, code, tag) = refusal(replay(&case, extended));
    assert_eq!(
        cause,
        ModelTraceRefusal::NotEnabled {
            step: 3,
            transition: Box::new(last.transition),
        }
    );
    assert_eq!((code, tag), (Code::InvalidRuntimeInput, "invalid-value"));

    let mut outside = payload.clone();
    outside.initial_state = 1;
    let (cause, code, _) = refusal(replay(&case, outside));
    assert_eq!(cause, ModelTraceRefusal::InitialState { index: 1, count: 1 });
    assert_eq!(code, Code::InvalidRuntimeInput);

    let mut formula = payload.clone();
    formula.kind = CounterexampleKind::Formula;
    let (cause, code, tag) = refusal(replay(&case, formula));
    assert!(matches!(cause, ModelTraceRefusal::UnknownTemporalClause { .. }));
    assert_eq!((code, tag), (Code::StaleDependency, "content-mismatch"));

    let edited = unit_text(
        &case.documents.document,
        &format!("{CONTRACT}function Other using v(): Boolean pure {{ false }}\n"),
    )
    .into_bytes();
    let stale = replay_model_trace(
        request(&case, &edited, case.package_id),
        &trace_subject(&case),
        &envelope(&edited, case.package_id, payload),
        ReplayLimits::default(),
    );
    assert!(
        matches!(stale, Err(ReplayRefusal::PackageIdMismatch { .. })),
        "{stale:?}"
    );
}

const GUARDED: &str = "pre CanInc using v on M::Counter::inc { 6 mod (2 - self.value) >= 0 }\n\
                       post Inc using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n";

/// FR-128-AC-5 for the deadlock-freedom item (ADR-018 UE-6): the
/// counterexample of an undefined precondition guard replays to
/// `reproduced-with-evaluated-witness` with the undefined value at position
/// 2; with `where` moved to position 1, with its locus naming another
/// clause, or truncated to end at value 1, it settles `inconclusive`.
#[trace("TC-539")]
#[test]
fn an_undefined_guard_counterexample_reproduces_only_where_it_is_undefined() {
    let case = case(GUARDED, &[0]);
    let payload = counterexample(&case);
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &payload.kind else {
        panic!("an undefined guard: {payload:?}");
    };
    assert_eq!(evaluation.site.position, 2);
    assert_eq!(
        evaluation.cause,
        UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero)
    );
    assert_eq!(
        replay(&case, payload.clone()).expect("replays").settlement,
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Undefined(evaluation.clone()),
            trace_position: Some(2),
        }
    );
    let disagrees = |payload: TemporalCounterexample| {
        matches!(
            replay(&case, payload).expect("replays").settlement,
            ModelTraceSettlement::Inconclusive(DisagreementCause::Verdicts { .. })
        )
    };
    let mut moved = payload.clone();
    let mut other = payload.clone();
    if let CounterexampleKind::UndefinedEvaluation(evaluation) = &mut moved.kind {
        evaluation.site.position = 1;
    }
    if let CounterexampleKind::UndefinedEvaluation(evaluation) = &mut other.kind {
        evaluation.site.locus = UndefinedLocus::Contract("Inc".to_owned());
    }
    let mut truncated = payload.clone();
    truncated.prefix.pop();
    assert!(disagrees(moved), "where at position 1");
    assert!(disagrees(other), "another locus");
    assert!(disagrees(truncated), "truncated to value 1");
}

fn settle(outcome: &ModelCheckOutcome) -> qsl_replay::ModelCheckRecord {
    settle_model_check(outcome, &ModelCheckLimits::default(), |_| {
        panic!("only a violation is replayed")
    })
}

/// FR-127's map for the outcomes that need no replay: `Holds` settles
/// `proved`, `closed-scope`, `Proved{Exhaustive, Uncertified}` (EN-1 has no
/// core certificate checker yet, ADR-018 PC-1), success; `BoundReached{2}`
/// settles `inconclusive`, `unsettled`, execution `completed`, truth
/// `pending`, depth 2; `UndecidedSuccessor` and `NoInitialState` settle
/// `inconclusive` with their causes; `Stopped` at `max_states` 2 settles
/// `failed`, `resource-incomplete`, `unavailable`,
/// `Incomplete(LimitReached{MaxStates, 2})` naming `max_states`; a
/// cancelled run keeps its source. Every record states method
/// `explicit-state` and the three limits used.
#[trace("TC-522")]
#[test]
fn each_outcome_settles_its_row_of_the_map() {
    let stats = RunStats::default();
    let holds = settle(&ModelCheckOutcome::Holds { stats });
    assert_eq!(
        holds.value,
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Uncertified,
        }
    );
    assert_eq!(
        (holds.label, holds.basis, holds.execution, holds.truth),
        (
            ResultLabel::Proved,
            SettlementBasis::ClosedScope,
            Execution::Completed,
            Truth::Satisfied
        )
    );
    assert_eq!(holds.value.category(), Category::Success);

    let bound = settle(&ModelCheckOutcome::BoundReached { depth: 2, stats });
    assert_eq!(
        bound.value,
        TerminalValue::Inconclusive(InconclusiveCause::BoundReached { depth: 2 })
    );
    assert_eq!(
        (bound.label, bound.basis, bound.execution, bound.truth, bound.depth),
        (
            ResultLabel::Inconclusive,
            SettlementBasis::Unsettled,
            Execution::Completed,
            Truth::Pending,
            Some(2)
        )
    );
    assert_eq!(bound.value.category(), Category::Inconclusive);

    for (cause, expected) in [
        (UndecidedCause::UndecidedSuccessor, InconclusiveCause::UndecidedSuccessor),
        (UndecidedCause::NoInitialState, InconclusiveCause::NoInitialState),
    ] {
        let record = settle(&ModelCheckOutcome::Undecided { cause, stats });
        assert_eq!(record.value, TerminalValue::Inconclusive(expected));
        assert_eq!(record.label, ResultLabel::Inconclusive);
        assert_eq!(record.basis, SettlementBasis::Unsettled);
    }

    let reached = ReachedLimit {
        limit: ModelCheckLimit::MaxStates,
        value: 2,
    };
    let stopped = settle(&ModelCheckOutcome::Stopped {
        cause: IncompleteCause::ResourceExhausted,
        limit: Some(reached),
        stats,
    });
    let limit_reached = IncompleteCause::LimitReached {
        limit: ModelCheckLimit::MaxStates,
        value: 2,
    };
    assert_eq!(stopped.value, TerminalValue::Incomplete(limit_reached));
    assert_eq!(limit_reached.as_str(), "limit-reached");
    assert_eq!(ModelCheckLimit::MaxStates.setting(), "max_states");
    assert_eq!(
        (stopped.label, stopped.basis, stopped.execution, stopped.truth),
        (
            ResultLabel::Failed,
            SettlementBasis::Unavailable,
            Execution::ResourceIncomplete,
            Truth::Unavailable
        )
    );
    assert_eq!(stopped.limit_reached, Some(reached));
    assert_eq!(stopped.value.category(), Category::Incomplete);

    for source in [CancelCause::Requested, CancelCause::Deadline] {
        let cancelled = settle(&ModelCheckOutcome::Stopped {
            cause: IncompleteCause::Cancelled { source },
            limit: None,
            stats,
        });
        assert_eq!(
            cancelled.value,
            TerminalValue::Incomplete(IncompleteCause::Cancelled { source })
        );
        assert_eq!(cancelled.execution, Execution::ResourceIncomplete);
        assert_eq!(cancelled.limit_reached, None);
    }

    let used = [
        (ModelCheckLimit::MaxStates, 10_000_000),
        (ModelCheckLimit::MaxTransitions, 100_000_000),
        (ModelCheckLimit::MaxAutomatonStates, 1_048_576),
    ];
    for record in [holds, bound, stopped] {
        assert_eq!(record.method, EXPLICIT_STATE);
        assert_eq!(record.limits, used);
    }
}

/// FR-127-AC-5 (settlement half): FR-126-AC-3's deadlock-freedom violation
/// settles `refuted`, `decisive-counterexample`, `Refuted`, violation, only
/// through its replay, and the record carries the counterexample with `kind:
/// Deadlock`.
#[trace("TC-522")]
#[test]
fn a_deadlock_violation_settles_refuted_through_its_replay() {
    let case = case(CONTRACT, &[0]);
    let mut replayed = false;
    let record = settle_model_check(&case.outcome, &ModelCheckLimits::default(), |payload| {
        replayed = true;
        replay(&case, payload.clone())
    });
    assert!(replayed, "the map replays the counterexample");
    assert_eq!(record.value, TerminalValue::Refuted);
    assert_eq!(
        (record.label, record.basis, record.truth),
        (
            ResultLabel::Refuted,
            SettlementBasis::DecisiveCounterexample,
            Truth::Violated
        )
    );
    assert_eq!(record.value.category(), Category::Violation);
    assert_eq!(
        record.counterexample.map(|counterexample| counterexample.kind),
        Some(CounterexampleKind::Deadlock)
    );
}

/// FR-127-AC-10, and FR-127-AC-3's replay rows over a deadlock: a replay
/// that disagrees by `Verdicts` settles `inconclusive`, cause
/// `ReplayParity`, spelled `replay-parity`; a replay refused for a
/// post-state digest settles `inconclusive`, `ReplayRefused` with the
/// refusal's code; a replay that faults settles `failed`, category failed.
#[trace("TC-522", "FR-127-AC-10")]
#[test]
fn a_disagreeing_or_refused_replay_never_settles_refuted() {
    let case = case(CONTRACT, &[0]);
    let violated = |payload: TemporalCounterexample| ModelCheckOutcome::Violated {
        counterexample: Box::new(payload),
        stats: RunStats::default(),
    };
    let mut truncated = counterexample(&case);
    truncated.prefix.pop();
    let parity = settle_model_check(
        &violated(truncated),
        &ModelCheckLimits::default(),
        |payload| replay(&case, payload.clone()),
    );
    let TerminalValue::Inconclusive(cause) = &parity.value else {
        panic!("expected inconclusive: {parity:?}");
    };
    assert!(
        matches!(
            cause,
            InconclusiveCause::ReplayParity(DisagreementCause::Verdicts { .. })
        ),
        "{cause:?}"
    );
    assert_eq!(cause.as_str(), "replay-parity");
    assert_eq!(parity.label, ResultLabel::Inconclusive);

    let mut altered = counterexample(&case);
    altered.prefix[0].post_state = DigestRecord::mint(DigestDomain::SimulationStateKeyV1, [9; 32]);
    let refused = settle_model_check(
        &violated(altered),
        &ModelCheckLimits::default(),
        |payload| replay(&case, payload.clone()),
    );
    assert_eq!(
        refused.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(Code::StaleDependency))
    );

    let failed = settle_model_check(
        &violated(counterexample(&case)),
        &ModelCheckLimits::default(),
        |_| Err(ReplayRefusal::Fault(InternalFault::new("test", "replay-faults"))),
    );
    assert_eq!(failed.value, TerminalValue::Failed);
    assert_eq!(failed.label, ResultLabel::Failed);
    assert_eq!(failed.value.category(), Category::InternalFailure);
}
