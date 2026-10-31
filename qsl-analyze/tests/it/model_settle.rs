// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-127 and FR-128 over the Counter subject (`crate::counter`): an EN-1
//! outcome settles as one terminal record, and a deadlock-freedom
//! counterexample settles `refuted` only after `replay_model_trace`
//! recompiles the unit, re-admits the subject from the byte provision and
//! reproduces it.
//!
//! Not covered: a counterexample of an authored clause (no recompiled
//! package holds a temporal clause node until FR-337, so its replay
//! refuses), the `ReplaySource::ModelTrace` arm (the envelope carries a witness source
//! the model-trace replay does not read). A `Holds` outcome's settlement
//! through the closure checker is `crate::closure_certificate`'s.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_model, ModelCheckRequest};
use qsl_eval::simulation::model::ExpansionLimits;
use qsl_eval::simulation::SubjectTransition;
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_package::CheckedPackage;
use qsl_replay::{
    replay_model_trace, settle_model_check, CertificateRequest, CounterexampleKind,
    DisagreementCause, EnvelopeSource, Execution, IncompleteCause, InconclusiveCause,
    ModelCheckItem, ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome, ModelTraceRefusal,
    ModelTraceReplayResult, ModelTraceSettlement, ModelTraceSubject, ModelTraceValue,
    QualifiedName, ReachedLimit, ReplayLimits, ReplayRefusal, ReplayRequestWire, ReplaySource,
    ResultLabel, RunStats, SettlementBasis, StateEnvironment, TemporalCounterexample,
    TerminalValue, Truth, UndecidedCause, UndefinedLocus, Witness, WitnessEnvelope, WitnessPacket,
    EXPLICIT_STATE,
};
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{Cancel, CancelCause, Identifier, NodeKey, Origin, Role};
use quire_semantic_value::catalog::Category;
use quire_semantic_value::catalog::InternalFault;

use crate::counter::{
    document, package, subject, unit_text, universe, Documents, Letters, CONTRACT, UNLIMITED,
};

/// The letters of a replay that reads no atom: the deadlock-freedom item has
/// none.
const NO_ATOMS: Letters = Letters { atom: |_| None };

const AUTHORITY: &str = "agent-ix";
const IDENTITY: &str = "counter";

/// The fixture package manifest's top-level name.
pub(crate) const PACKAGE_NAME: &str = "package";

/// A compiled Counter unit with its documents, initial snapshots and the
/// engine's deadlock-freedom outcome over universe `{c}`.
pub(crate) struct Case {
    unit: Vec<u8>,
    documents: Documents,
    initial: Vec<DocumentRef>,
    checked: CheckedPackage,
    outcome: ModelCheckOutcome,
}

impl Case {
    /// The deadlock-freedom item `analyze` runs (FR-281): the run, its
    /// closure checker's request, and the
    /// counterexample replay of [`replay`] over this case's source.
    pub(crate) fn model_item(&self) -> qsl_analyze::analyze::ModelItem<'_> {
        qsl_analyze::analyze::ModelItem {
            request: ModelCheckRequest {
                subject: subject(
                    &self.checked,
                    &self.documents,
                    self.initial.clone(),
                    &["c"],
                    UNLIMITED,
                ),
                item: ModelCheckItem::DeadlockFreedom,
                max_depth: None,
                limits: ModelCheckLimits::default(),
                reductions: Default::default(),
            },
            certificate: self.request(),
            replay: Box::new(|payload| replay(self, payload.clone())),
        }
    }

    /// The deadlock-freedom item's certificate request over the case's
    /// subject.
    pub(crate) fn request(&self) -> CertificateRequest<'_> {
        CertificateRequest {
            subject: subject(
                &self.checked,
                &self.documents,
                self.initial.clone(),
                &["c"],
                UNLIMITED,
            ),
            item: ModelCheckItem::DeadlockFreedom,
            smt_encoding: qsl_replay::SmtEncodingLimits::default(),
        }
    }
}

pub(crate) fn case(body: &str, values: &[i64]) -> Case {
    let document = document(false);
    let unit = unit_text(body).into_bytes();
    let checked = package(&document, body);
    let mut documents = Documents::new(document);
    let initial: Vec<DocumentRef> = values
        .iter()
        .enumerate()
        .map(|(index, value)| documents.start(&format!("s{index}-{value}"), &[("c", *value)]))
        .collect();
    let outcome = check_model(
        ModelCheckRequest {
            subject: subject(&checked, &documents, initial.clone(), &["c"], UNLIMITED),
            item: ModelCheckItem::DeadlockFreedom,
            max_depth: None,
            limits: ModelCheckLimits::default(),
            reductions: Default::default(),
        },
        &Cancel::new(),
    )
    .expect("checked");
    Case {
        unit,
        documents,
        initial,
        checked,
        outcome,
    }
}

/// A source reference as the request spells it.
type SourceWire = (String, String, String, String);

pub(crate) fn source_wire() -> SourceWire {
    (
        AUTHORITY.to_owned(),
        IDENTITY.to_owned(),
        "git".to_owned(),
        "1".to_owned(),
    )
}

fn witness_source() -> ReplaySource {
    ReplaySource::Witness(
        Witness::parse("<<<assertion|model_check|deadlock|>>>").expect("a transcript"),
    )
}

fn envelope_witness_source() -> EnvelopeSource {
    EnvelopeSource::Witness(
        Witness::parse("<<<assertion|model_check|deadlock|>>>").expect("a transcript"),
    )
}

fn selected() -> QualifiedName {
    QualifiedName::new(vec![Identifier::new("Noop").expect("an identifier")]).expect("a name")
}

/// FR-098's request for `unit`, with the domain package and
/// the snapshots in the byte provision.
pub(crate) fn request<'a>(
    document: &[u8],
    snapshots: impl Iterator<
        Item = (
            &'a qsl_semantics::model::observation::DocumentRef,
            &'a Vec<u8>,
        ),
    >,
    unit: &[u8],
) -> ReplayRequestWire {
    let byte_provision = vec![(AUTHORITY.to_owned(), IDENTITY.to_owned(), unit.to_vec())];
    let mut documents = vec![document.to_vec()];
    for (_, bytes) in snapshots {
        documents.push(bytes.clone());
    }
    ReplayRequestWire {
        package_name: PACKAGE_NAME.to_owned(),
        profile_selections: Vec::new(),
        sources: vec![source_wire()],
        dependencies: Vec::new(),
        replay_target: qsl_replay::RequestTargetWire::function(
            selected(),
            qsl_replay::OccurrenceKey::new(
                crate::fixture_ids::node_key([7]),
                qsl_replay::Origin::new(qsl_replay::Role::new("declaration"), 0),
            ),
        ),
        source: witness_source(),
        obligation_identity: crate::fixture_ids::obligation_identity(),
        backend: "qsl-en-1".to_owned(),
        state_environment: StateEnvironment::new(Vec::new()),
        accounting_limits: UNLIMITED,
        stage_limits: std::collections::BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision,
        documents,
    }
}

/// The identity of a package-level freedom item over a model subject: the
/// selected model's `model_root` node of the domain package `identity`, with
/// the `generated` occurrence 0.
pub(crate) fn model_anchor(identity: &str) -> (NodeKey, OccurrenceKey) {
    let node = NodeKey::new(
        identity,
        vec![
            quire_exact::PathSegment::Text("model".to_owned()),
            quire_exact::PathSegment::Text("model_root".to_owned()),
        ],
        0,
    )
    .expect("a model root is admissible");
    let occurrence = OccurrenceKey::new(node.clone(), Origin::new(Role::new("generated"), 0));
    (node, occurrence)
}

/// The identity of a package-level freedom item over the protocol `protocol`
/// of `package`: its `protocol_clause` node with the `declaration`
/// occurrence 0.
pub(crate) fn protocol_anchor(
    package: &qsl_package::CheckedPackage,
    protocol: &str,
) -> (NodeKey, OccurrenceKey) {
    // A package that selects no protocol profile has no clause node: the
    // fixture node is then unknown to the replay, which refuses it.
    let node = package
        .graph()
        .protocol_node_keys(protocol)
        .and_then(|keys| keys.clause.clone())
        .unwrap_or_else(|| crate::fixture_ids::node_key("protocol-clause"));
    let occurrence = OccurrenceKey::new(node.clone(), Origin::new(Role::new("declaration"), 0));
    (node, occurrence)
}

/// The identity of the derived deadlock-freedom item over the counter unit's
/// model.
pub(crate) fn deadlock_freedom_clause() -> (NodeKey, OccurrenceKey) {
    model_anchor(crate::counter::PACKAGE)
}

/// The envelope carrying `payload` under the derived deadlock-freedom item's
/// identity.
fn envelope(payload: TemporalCounterexample) -> WitnessEnvelope<TemporalCounterexample> {
    let (node, occurrence) = deadlock_freedom_clause();
    envelope_at(payload, node, occurrence)
}

/// The envelope carrying `payload`, naming the
/// item by `node` and `occurrence`.
pub(crate) fn envelope_at<T: SubjectTransition>(
    payload: TemporalCounterexample<T>,
    node: NodeKey,
    occurrence: OccurrenceKey,
) -> WitnessEnvelope<TemporalCounterexample<T>> {
    WitnessEnvelope::reconstruct(
        WitnessPacket {
            obligation_identity: Some(crate::fixture_ids::obligation_identity()),
            occurrence_key: Some(occurrence),
            clause_node: Some(node),
            replay_target: qsl_replay::ReplayTargetMembers::function(selected()),
            profile_selections: Some(Vec::new()),
            run_limits: Some(UNLIMITED),
            declared_domains: Some(Vec::new()),
            backend: Some("qsl-en-1".to_owned()),
            trace_position: Some(None),
            source: Some(envelope_witness_source()),
            family_payload: Some(payload),
        },
        ReplayLimits::default(),
    )
    .expect("a complete packet reconstructs")
}

fn trace_subject(case: &Case) -> ModelTraceSubject {
    ModelTraceSubject {
        timed: None,
        initial: case.initial.clone(),
        universes: universe(&["c"]),
        expansion: ExpansionLimits {
            max_candidates: 1_000_000,
        },
    }
}

pub(crate) fn replay(
    case: &Case,
    payload: TemporalCounterexample,
) -> Result<ModelTraceReplayResult, ReplayRefusal> {
    replay_model_trace(
        request(
            &case.documents.document,
            case.documents.snapshot_bytes(),
            &case.unit,
        ),
        &trace_subject(case),
        &envelope(payload),
        &NO_ATOMS,
        ReplayLimits::default(),
    )
}

pub(crate) fn counterexample(case: &Case) -> TemporalCounterexample {
    match &case.outcome {
        ModelCheckOutcome::Violated { counterexample, .. } => counterexample.as_ref().clone(),
        other => panic!("expected a violation, got {other:?}"),
    }
}

/// Verifies part of FR-128-AC-2: FR-126-AC-3's deadlock counterexample, in
/// the derived item's envelope, reproduces, the item's value `false`;
/// truncated to end at value 2, where `inc` is enabled, it settles
/// `inconclusive` by `Verdicts`. Replaying one envelope twice gives equal
/// results (FR-128-AC-4's last clause).
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
    assert!(
        matches!(cause, DisagreementCause::Verdicts { .. }),
        "{cause:?}"
    );
    assert_eq!(
        cause.replayed().map(|verdict| verdict.category()),
        Some(Category::Success)
    );
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

/// Verifies the prefix refusals of FR-128-AC-3, not its lasso,
/// fairness-set, interval, stutter or `observed` refusals: a post-state
/// state key altered refuses
/// `stale_dependency`/`content-mismatch` naming the step and the state key; a
/// step appended past the terminal state is not enabled and refuses
/// `invalid_runtime_input`/`invalid-value` naming the step; an
/// `initial_state` of 1 over a one-snapshot subject refuses
/// `invalid-value`; and a source edit refuses by FR-098's stale-package rule
/// (FR-128-AC-4's second clause).
#[trace("TC-523")]
#[test]
fn a_counterexample_the_model_does_not_take_refuses_with_no_result() {
    let case = case(CONTRACT, &[0]);
    let payload = counterexample(&case);

    let mut altered = payload.clone();
    let request = case.request();
    let system = qsl_eval::simulation::model::ModelSystem::new(request.subject.simulation)
        .expect("the model admits");
    let (bytes, _) = system
        .state_key(&system.initial_states()[0])
        .expect("encodes");
    let recorded = qsl_eval::simulation::StateKey::from_canonical_bytes(bytes);
    altered.prefix[1].post_state = recorded.clone();
    let (cause, code, tag) = refusal(replay(&case, altered));
    assert_eq!(
        cause,
        ModelTraceRefusal::PostState {
            step: 1,
            recorded: recorded.clone(),
            reached: vec![payload.prefix[1].post_state.clone()]
        }
    );
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
    assert_eq!(
        cause,
        ModelTraceRefusal::InitialState { index: 1, count: 1 }
    );
    assert_eq!(code, Code::InvalidRuntimeInput);
}

/// FR-128 "resolve the clause by its `clause_node` and `occurrence_key`",
/// for the derived deadlock-freedom item: its payload reproduces only under
/// the item's own identity ([`deadlock_freedom_clause`]). The same deadlock payload under an authored clause's
/// identity, or under the item's node with another occurrence, refuses
/// `stale_dependency`/`content-mismatch` naming both identities; a `Formula`
/// payload under the item's identity refuses the same way; and a `Formula`
/// payload under an authored identity names a clause no recompiled package
/// holds (FR-337). Verifies FR-128-AC-4's `clause_node` clause for the
/// derived item only, not its lasso over ADR-018 §6's subject.
#[trace("TC-523")]
#[test]
fn a_counterexample_resolves_its_item_by_clause_node_and_occurrence_key() {
    let case = case(CONTRACT, &[0]);
    let payload = counterexample(&case);
    let (node, occurrence) = deadlock_freedom_clause();
    let replay_at = |payload: TemporalCounterexample, node, occurrence| {
        replay_model_trace(
            request(
                &case.documents.document,
                case.documents.snapshot_bytes(),
                &case.unit,
            ),
            &trace_subject(&case),
            &envelope_at(payload, node, occurrence),
            &NO_ATOMS,
            ReplayLimits::default(),
        )
    };
    assert!(matches!(
        replay_at(payload.clone(), node.clone(), occurrence.clone())
            .expect("replays")
            .settlement,
        ModelTraceSettlement::Reproduced { .. }
    ));

    let authored = crate::fixture_ids::node_key([7]);
    let authored_key = OccurrenceKey::new(authored.clone(), Origin::new(Role::new("temporal"), 0));
    let (cause, code, tag) = refusal(replay_at(
        payload.clone(),
        authored.clone(),
        authored_key.clone(),
    ));
    assert_eq!(
        cause,
        ModelTraceRefusal::UnknownTemporalClause {
            clause_node: authored.clone()
        }
    );
    assert_eq!((code, tag), (Code::StaleDependency, "content-mismatch"));

    let other_key = OccurrenceKey::new(node.clone(), Origin::new(Role::new("deadlock-freedom"), 1));
    let (cause, _, _) = refusal(replay_at(payload.clone(), node.clone(), other_key));
    assert!(
        matches!(cause, ModelTraceRefusal::ItemIdentity { .. }),
        "{cause:?}"
    );

    let mut formula = payload;
    formula.kind = CounterexampleKind::Formula;
    let (cause, code, tag) = refusal(replay_at(formula.clone(), node.clone(), occurrence));
    assert!(
        matches!(cause, ModelTraceRefusal::UnknownTemporalClause { .. }),
        "{cause:?}"
    );
    assert_eq!((code, tag), (Code::StaleDependency, "content-mismatch"));
    let (cause, code, tag) = refusal(replay_at(formula, authored.clone(), authored_key));
    assert_eq!(
        cause,
        ModelTraceRefusal::UnknownTemporalClause {
            clause_node: authored
        }
    );
    assert_eq!((code, tag), (Code::StaleDependency, "content-mismatch"));
}

const GUARDED: &str = "pre CanInc using v on M::Counter::inc { 6 mod (2 - self.value) >= 0 }\n\
                       post Inc using v on M::Counter::inc { self.value - pre(self.value) = 1 }\n";

/// The deadlock-freedom item's analogue of FR-128-AC-5 (ADR-018 UE-6),
/// which is not that criterion (FR-128-AC-5 is tested in lasso_replay).
/// The
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

/// A replay that a limit stopped settles `failed`, execution
/// `resource-incomplete`, `Incomplete(LimitReached{limit, value})` naming the
/// limit it reached, and carries no counterexample (FR-127). No liveness replay
/// stops on a limit today; the map reads the stop of one that will, so the
/// result is injected.
#[trace("TC-522", "FR-127-AC-1")]
#[test]
fn a_replay_a_limit_stopped_settles_incomplete_naming_the_limit() {
    let case = case(CONTRACT, &[0]);
    let record = settle_model_check(
        &ModelCheckOutcome::Violated {
            counterexample: Box::new(counterexample(&case)),
            stats: RunStats::default(),
        },
        &ModelCheckLimits::default(),
        &case.request(),
        &Cancel::new(),
        |payload| {
            let mut result = replay(&case, payload.clone())?;
            result.settlement = ModelTraceSettlement::Stopped {
                reached: ReachedLimit {
                    limit: ModelCheckLimit::MaxStates,
                    value: 2,
                },
                count: None,
            };
            Ok(result)
        },
    );
    assert_eq!(
        record.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: 2
        })
    );
    assert_eq!(record.execution, Execution::ResourceIncomplete);
    assert_eq!(record.counterexample, None);
}

fn settle(case: &Case, outcome: &ModelCheckOutcome) -> qsl_replay::ModelCheckRecord {
    settle_model_check(
        outcome,
        &ModelCheckLimits::default(),
        &case.request(),
        &Cancel::new(),
        |_| panic!("only a violation is replayed"),
    )
}

/// FR-127's map for the outcomes that need no replay or certificate check:
/// `BoundReached{2}`
/// settles `inconclusive`, `unsettled`, execution `completed`, truth
/// `pending`, depth 2; `UndecidedSuccessor`, `NoInitialState` and `NoInstance` settle
/// `inconclusive` with their causes; `Stopped` at `max_states` 2 settles
/// `failed`, `resource-incomplete`, `unavailable`,
/// `Incomplete(LimitReached{MaxStates, 2})` naming `max_states`; a
/// cancelled run keeps its source. Every record states method
/// `explicit-state` and the three limits used.
#[trace("TC-522", "FR-127-AC-1")]
#[test]
fn each_outcome_settles_its_row_of_the_map() {
    let case = case(CONTRACT, &[0]);
    let settle = |outcome: &ModelCheckOutcome| settle(&case, outcome);
    let stats = RunStats::default();
    let bound = settle(&ModelCheckOutcome::BoundReached { depth: 2, stats });
    assert_eq!(
        bound.value,
        TerminalValue::Inconclusive(InconclusiveCause::BoundReached { depth: 2 })
    );
    assert_eq!(
        (
            bound.label,
            bound.basis,
            bound.execution,
            bound.truth,
            bound.depth
        ),
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
        (
            UndecidedCause::UndecidedSuccessor,
            InconclusiveCause::UndecidedSuccessor,
        ),
        (
            UndecidedCause::NoInitialState,
            InconclusiveCause::NoInitialState,
        ),
        (UndecidedCause::NoInstance, InconclusiveCause::NoInstance),
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
    assert_eq!(
        ModelCheckLimit::MaxStates.setting(),
        "model_check.max_states"
    );
    assert_eq!(
        (
            stopped.label,
            stopped.basis,
            stopped.execution,
            stopped.truth
        ),
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
    for record in [bound, stopped] {
        assert_eq!(record.method, EXPLICIT_STATE);
        assert_eq!(record.limits, used);
    }
}

/// FR-127-AC-16: a stop by an exhausted meter counter or by the candidate
/// bound settles `failed`, `resource-incomplete`, `unavailable`,
/// `Incomplete(LimitReached{limit, value, setting})`, category incomplete,
/// for `work_units` 0, `value_occurrences` 7, `integer_bits` `u64::MAX` and
/// `max_candidates` 1. Each setting is the FR-255 accounting or expansion
/// name, never `meter`, a bare counter or a bare `max_candidates`, and the
/// settings operation accepts each accounting name to raise that bound (the
/// settings table registers no `expansion.*` row yet).
#[trace("TC-522", "FR-127-AC-16")]
#[test]
fn a_meter_or_candidate_stop_settles_naming_its_setting() {
    use qsl_replay::CallerLimits;
    use quire_exact::LimitKind;

    let case = case(CONTRACT, &[0]);
    for (limit, value, name, setting, operand) in [
        (
            ModelCheckLimit::EvaluationMeter {
                counter: LimitKind::WorkUnits,
            },
            0,
            "work_units",
            "accounting.work_units",
            "accounting.work_units=1",
        ),
        (
            ModelCheckLimit::EvaluationMeter {
                counter: LimitKind::ValueOccurrences,
            },
            7,
            "value_occurrences",
            "accounting.value_occurrences",
            "accounting.value_occurrences=8",
        ),
        (
            ModelCheckLimit::EvaluationMeter {
                counter: LimitKind::IntegerBits,
            },
            u64::MAX,
            "integer_bits",
            "accounting.integer_bits",
            "accounting.integer_bits=1",
        ),
        (
            ModelCheckLimit::MaxCandidates,
            1,
            "max_candidates",
            "expansion.max_candidates",
            "expansion.max_candidates=2",
        ),
    ] {
        let reached = ReachedLimit { limit, value };
        let record = settle(
            &case,
            &ModelCheckOutcome::Stopped {
                cause: IncompleteCause::ResourceExhausted,
                limit: Some(reached),
                stats: RunStats::default(),
            },
        );
        assert_eq!(
            record.value,
            TerminalValue::Incomplete(IncompleteCause::LimitReached { limit, value })
        );
        assert_eq!(
            (record.label, record.basis, record.execution, record.truth),
            (
                ResultLabel::Failed,
                SettlementBasis::Unavailable,
                Execution::ResourceIncomplete,
                Truth::Unavailable
            )
        );
        assert_eq!(record.value.category(), Category::Incomplete);
        assert_eq!(record.limit_reached, Some(reached));
        assert_eq!(limit.as_str(), name);
        assert_eq!(limit.setting(), setting);
        assert!(!matches!(setting, "meter" | "max_candidates"));
        assert!(!setting.eq(name), "no bare counter name: {setting}");
        CallerLimits::from_operands([operand]).expect("the settings operation accepts the name");
    }
}

/// Verifies the settlement half of FR-127-AC-5, not its obligation-identity
/// clause (the record carries no obligation identity): FR-126-AC-3's
/// deadlock-freedom violation
/// settles `refuted`, `decisive-counterexample`, `Refuted`, violation, only
/// through its replay, and the record carries the counterexample with `kind:
/// Deadlock`.
#[trace("TC-522", "FR-127-AC-1")]
#[test]
fn a_deadlock_violation_settles_refuted_through_its_replay() {
    let case = case(CONTRACT, &[0]);
    let mut replayed = false;
    let record = settle_model_check(
        &case.outcome,
        &ModelCheckLimits::default(),
        &case.request(),
        &Cancel::new(),
        |payload| {
            replayed = true;
            replay(&case, payload.clone())
        },
    );
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
        record
            .counterexample
            .map(|counterexample| counterexample.kind),
        Some(CounterexampleKind::Deadlock)
    );
}

/// FR-127-AC-10, and FR-127-AC-3's replay rows over a deadlock: a replay
/// that disagrees by `Verdicts` settles `inconclusive`, cause
/// `ReplayParity`, spelled `replay-parity`; a replay refused for a
/// post-state key settles `inconclusive`, `ReplayRefused` with the
/// refusal's code; a replay reproduced against another package than the
/// run's settles `ReplayRefused(stale_dependency)`; a replay that faults
/// settles `failed`, category failed.
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
        &case.request(),
        &Cancel::new(),
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
    let request = case.request();
    let system = qsl_eval::simulation::model::ModelSystem::new(request.subject.simulation)
        .expect("the model admits");
    let (bytes, _) = system
        .state_key(&system.initial_states()[0])
        .expect("encodes");
    altered.prefix[0].post_state = qsl_eval::simulation::StateKey::from_canonical_bytes(bytes);
    let refused = settle_model_check(
        &violated(altered),
        &ModelCheckLimits::default(),
        &case.request(),
        &Cancel::new(),
        |payload| replay(&case, payload.clone()),
    );
    assert_eq!(
        refused.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(Code::StaleDependency))
    );

    let failed = settle_model_check(
        &violated(counterexample(&case)),
        &ModelCheckLimits::default(),
        &case.request(),
        &Cancel::new(),
        |_| {
            Err(ReplayRefusal::Fault(InternalFault::new(
                "test",
                "replay-faults",
            )))
        },
    );
    assert_eq!(failed.value, TerminalValue::Failed);
    assert_eq!(failed.label, ResultLabel::Failed);
    assert_eq!(failed.value.category(), Category::InternalFailure);
}

const TERMINAL_WHEN: &str = "terminal Done using v on M when { 6 mod (3 - self.value) = 0 };\n";

fn with_terminal(terminal: &str) -> String {
    format!("{CONTRACT}{terminal}")
}

/// FR-128-AC-6 (deadlock-freedom half) and FR-411-AC-3: under `terminal when
/// 6 mod (3 - c.value) = 0`, the prefix to value 3 with `kind:
/// UndefinedEvaluation{where: position 3, cause: division-by-zero}` at the
/// terminal reproduces with `trace_position` 3, and the same prefix with
/// `kind: Deadlock` settles the same witness value and the item `refuted`.
#[trace("TC-539", "FR-128-AC-6", "FR-411-AC-3")]
#[test]
fn an_undefined_terminal_predicate_reproduces_and_a_deadlock_kind_settles_the_same_value() {
    let case = case(&with_terminal(TERMINAL_WHEN), &[0]);
    let payload = counterexample(&case);
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &payload.kind else {
        panic!("an undefined terminal predicate: {payload:?}");
    };
    assert_eq!(evaluation.site.position, 3);
    assert_eq!(
        evaluation.site.locus,
        UndefinedLocus::Terminal("M".to_owned())
    );
    let witness = ModelTraceValue::Undefined(evaluation.clone());
    assert_eq!(
        replay(&case, payload.clone()).expect("replays").settlement,
        ModelTraceSettlement::Reproduced {
            value: witness.clone(),
            trace_position: Some(3),
        }
    );

    let mut deadlock = payload;
    deadlock.kind = CounterexampleKind::Deadlock;
    let ModelTraceSettlement::Reproduced { value, .. } =
        replay(&case, deadlock.clone()).expect("replays").settlement
    else {
        panic!("the deadlock kind reproduces");
    };
    assert_eq!(value, witness);
    let record = settle_model_check(
        &ModelCheckOutcome::Violated {
            counterexample: Box::new(deadlock),
            stats: RunStats::default(),
        },
        &ModelCheckLimits::default(),
        &case.request(),
        &Cancel::new(),
        |payload| replay(&case, payload.clone()),
    );
    assert_eq!(record.value, TerminalValue::Refuted);
}

/// FR-227: a race counterexample names a protocol subject's memory model; the
/// replay over a model subject refuses it, `RaceOverModel`.
/// It asserts no acceptance criterion; it backs a refusal no AC names.
#[test]
fn a_race_counterexample_over_a_model_subject_refuses_the_replay() {
    let case = case(CONTRACT, &[0]);
    let mut payload = counterexample(&case);
    payload.kind = CounterexampleKind::Race(qsl_replay::Race {
        location: qsl_eval::simulation::Location::AnyField {
            field: crate::litmus::key("v"),
        },
        earlier: 1,
        later: 1,
    });
    let (node, occurrence) = crate::model_settle::deadlock_freedom_clause();
    let result = replay_model_trace(
        request(
            &case.documents.document,
            case.documents.snapshot_bytes(),
            &case.unit,
        ),
        &trace_subject(&case),
        &envelope_at(payload, node, occurrence),
        &NO_ATOMS,
        ReplayLimits::default(),
    );
    let (refused, _, _) = refusal(result);
    assert_eq!(refused, ModelTraceRefusal::RaceOverModel);
}
