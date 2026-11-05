// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-524 (TC-1319) and the evidence entry of FR-516 (TC-1311): one temporal
//! instance per trigger or origin with immutable captures, every activation
//! fault named, and the four dispositions of a scope with no instance.
//!
//! Clauses are real complete-V1 source taken through S1, S2 and the
//! `TemporalTrace` family's check; the clause rows are the rows a recipe
//! package carries. The `holds` conditions are not checked at S3, so the
//! evaluator reads each atom from the valuations the test authors per
//! position and per instance.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::SubjectOccurrence;
use qsl_eval::temporal::evidence::{
    activate, evaluate_temporal_evidence, ActivationInput, ActivationRow, ActivationState,
    ActivationStop, CaptureFact, CapturedValue, CompletenessRecord, Disposition, Evicted,
    EvidenceCounter, EvidenceIncomplete, EvidencePosition, EvidenceRef, EvidenceRequest,
    EvidenceResult, EvidenceSubject, ExecutionDisposition, ExecutionOrigin, FaultKind,
    FaultSubject, Indeterminate, InstanceState, NonValueKind, ObligationKey, Payload, Receipt,
    Relation, SemanticTriggerIdentity, Settlement, TemporalEvidenceLimits, TriggerDelivery,
    TriggerEvidence, TriggerRecord, TriggerScope, Truth, UnknownKind,
};
use qsl_eval::temporal::evidence::{CallMeter, ClauseFacts, ClauseShape, TraceBinding};
use qsl_eval::temporal::{
    ClockCover, LedgerBinding, ProgressClosure, ProgressCompleteness, Valuation,
};
use qsl_package::{CheckedNodeId, CheckedSelection, ClockConfiguration, TemporalBinding};
use qsl_semantics::check::{FormulaNode, SCALAR_LIMITS_UNLIMITED};
use quire_eval::{CallAdmissionFailure, CallFailure};
use quire_exact::Meter;
use serde_json::json;

use crate::temporal_trace::{fixture, Fixture};

const FINITE_WINDOW: &str = "quire.temporal.timestamped-event.finite-window/v1";

/// The occurrence `{ claim, ordinal }` of a clause declaration.
pub(crate) fn claim(ordinal: u64) -> qsl_package::CheckedOccurrence {
    serde_json::from_value(json!({"role": "claim", "ordinal": ordinal})).expect("an occurrence key")
}

pub(crate) fn id(text: &str) -> SemanticTriggerIdentity {
    SemanticTriggerIdentity::new(text.as_bytes()).expect("a non-empty identity")
}

/// A recipe: one clause with its rows, built once per test.
pub(crate) struct Env {
    pub(crate) fixture: Fixture,
    declaration: CheckedNodeId,
    profile: CheckedSelection,
    row: TemporalBinding,
    rows: Vec<ActivationRow>,
    pub(crate) binding: TraceBinding,
    pub(crate) occurrence: u64,
}

pub(crate) fn env(activation: &str, body: &str) -> Env {
    build(
        FINITE_WINDOW,
        json!({"kind": "timestamped_event", "timestamp_unit": "ms"}),
        activation,
        body,
    )
}

const EVENT_POSITION: &str = "quire.temporal.event-position.false-extension/v1";
const FIXED_SAMPLE: &str = "quire.temporal.fixed-sample.false-extension/v1";

/// A clause under the fixed-sample profile (FR-504).
pub(crate) fn fixed_env(activation: &str, body: &str) -> Env {
    build(
        FIXED_SAMPLE,
        json!({
            "kind": "fixed_sample",
            "epoch": {"kind": "integer", "decimal": "0"},
            "period": {"kind": "integer", "decimal": "1"},
            "unit": "sample",
        }),
        activation,
        body,
    )
}

/// A clause under the event-position profile (FR-504).
pub(crate) fn event_env(activation: &str, body: &str) -> Env {
    build(
        EVENT_POSITION,
        json!({"kind": "event_position", "sequence_authority": "order-events"}),
        activation,
        body,
    )
}

fn build(profile_identity: &str, clock: serde_json::Value, activation: &str, body: &str) -> Env {
    let fixture = fixture(profile_identity, "c: Counter", activation, body);
    rows_for(fixture, profile_identity, clock)
}

/// The recipe rows around an already checked clause.
pub(crate) fn rows_for(fixture: Fixture, profile_identity: &str, clock: serde_json::Value) -> Env {
    let declaration: CheckedNodeId =
        serde_json::from_value(json!({"package": "recipe", "path": ["Due"], "ordinal": 0}))
            .expect("a node identity");
    let profile: CheckedSelection = serde_json::from_value(json!({
        "role": "temporal_profile",
        "definition": {"authority": "agent-ix", "identity": profile_identity},
    }))
    .expect("a profile selection");
    let row: TemporalBinding = serde_json::from_value(json!({
        "declaration": {"package": "recipe", "path": ["Due"], "ordinal": 0},
        "occurrence": 0,
        "clock_name": "stamp-clock",
        "clock": clock.clone(),
    }))
    .expect("a temporal binding row");
    let binding = TraceBinding {
        profile: profile_identity.into(),
        clock_name: "stamp-clock".into(),
        parameters: serde_json::from_value::<ClockConfiguration>(clock).expect("a clock"),
    };
    Env {
        fixture,
        declaration,
        profile,
        row,
        rows: Vec::new(),
        binding,
        occurrence: 0,
    }
}

impl Env {
    pub(crate) fn subject(&self, bound: &[u64]) -> EvidenceSubject<'_> {
        EvidenceSubject {
            declaration: &self.declaration,
            profile: &self.profile,
            occurrences: vec![SubjectOccurrence {
                occurrence: claim(self.occurrence),
                clock: Some(&self.row),
            }],
            facts: ClauseFacts::of_checked(&self.fixture.clause),
            instance_bound: bound.iter().copied().collect(),
            activation_map: &self.rows,
            progress: LedgerBinding {
                package: "a".to_owned(),
                clause: crate::support::ids::node_key(b"Due"),
                occurrence: 0,
                clock: "stamp-clock".to_owned(),
                profile: FINITE_WINDOW.to_owned(),
            },
            progress_capacity: qsl_eval::temporal::DEFAULT_COVERED_POSITIONS,
        }
    }

    pub(crate) fn atom(&self, name: &str) -> u64 {
        let want = format!("holds({name})");
        self.fixture
            .clause
            .formula()
            .nodes()
            .iter()
            .position(|node| {
                matches!(node, FormulaNode::Holds { span }
                    if self.fixture.text[span.start..span.end] == want)
            })
            .map(|index| index as u64)
            .expect("the clause reads the atom")
    }

    pub(crate) fn obligation(&self) -> ObligationKey {
        ObligationKey::new(
            self.declaration.package.clone(),
            serde_json::to_vec(&(&self.declaration, &claim(self.occurrence))).expect("encodes"),
            self.occurrence,
        )
    }

    /// The same clause body as another occurrence of the declaration.
    pub(crate) fn occurrence(mut self, occurrence: u64) -> Self {
        self.occurrence = occurrence;
        self
    }

    /// The same clause under another package and declaration name.
    pub(crate) fn moved(mut self, package: &str, name: &str) -> Self {
        self.declaration =
            serde_json::from_value(json!({"package": package, "path": [name], "ordinal": 0}))
                .expect("a node identity");
        self
    }

    /// The same clause under another clock binding name, in the row and in
    /// the binding the request asserts.
    pub(crate) fn with_clock_name(mut self, name: &str) -> Self {
        self.row.clock_name = name.into();
        self.binding.clock_name = name.into();
        self
    }

    /// The same clause under another profile identity, in the lock and in
    /// the binding the request asserts.
    pub(crate) fn with_profile(mut self, identity: &str) -> Self {
        self.profile.definition.identity = identity.into();
        self.binding.profile = identity.into();
        self
    }

    pub(crate) fn with_row(mut self, control: &str) -> Self {
        let control: CheckedNodeId =
            serde_json::from_value(json!({"package": "recipe", "path": [control], "ordinal": 0}))
                .expect("a node identity");
        self.rows.push(ActivationRow {
            control,
            clause: self.declaration.clone(),
        });
        self
    }
}

pub(crate) fn node(name: &str) -> CheckedNodeId {
    serde_json::from_value(json!({"package": "recipe", "path": [name], "ordinal": 0}))
        .expect("a node identity")
}

/// A position `obs<n>` at stamp `n` whose shared atoms read `values`.
pub(crate) fn position(index: u64, values: &[(u64, Valuation)]) -> EvidencePosition {
    EvidencePosition {
        observation: EvidenceRef::named(format!("obs{index}")),
        position: index,
        valuations: values.iter().cloned().collect(),
        ..EvidencePosition::default()
    }
}

pub(crate) fn request(env: &Env, positions: Vec<EvidencePosition>) -> EvidenceRequest {
    EvidenceRequest {
        trigger: None,
        correspondence: EvidenceRef::named("corr"),
        positions,
        anchor: Some("obs0".to_owned()),
        deliveries: Vec::new(),
        origins: Vec::new(),
        trigger_evidence: TriggerEvidence::Admitted,
        trigger_scope: ProgressClosure::Closed,
        decision_progress: Some(100),
        decision_closure: Some(ProgressClosure::Closed),
        surrounding_progress: Some(100),
        surrounding_closure: Some(ProgressClosure::Closed),
        execution: ExecutionDisposition::Completed,
        completeness: Some(CompletenessRecord {
            reference: EvidenceRef::named("complete"),
            state: ProgressCompleteness::Complete { through: 100 },
            facts: Vec::new(),
        }),
        authoritative_origin: Some(true),
        evicted: Vec::new(),
        cover: ClockCover::Exact,
        progress_ledger: None,
        ledger: None,
        relation: Relation::Original,
        control: None,
        binding: env.binding.clone(),
        occurrence: None,
        limits: TemporalEvidenceLimits::default(),
    }
}

pub(crate) fn delivery(
    identity: &str,
    receipt: &str,
    payload: &str,
    amounts: &[&str],
) -> TriggerDelivery {
    TriggerDelivery {
        identity: id(identity),
        receipt: Receipt::new(receipt.as_bytes()),
        anchor: "obs0".to_owned(),
        payload: Payload::new(payload.as_bytes()),
        guard: Some(true),
        captures: amounts
            .iter()
            .map(|amount| CaptureFact::Value {
                anchor: "obs0".to_owned(),
                value: CapturedValue::new(amount.as_bytes()),
            })
            .collect(),
    }
}

pub(crate) fn unlimited() -> Meter {
    Meter::new(SCALAR_LIMITS_UNLIMITED)
}

pub(crate) fn scope() -> TriggerScope {
    TriggerScope {
        closure: ProgressClosure::Closed,
        evidence: TriggerEvidence::Admitted,
        record: TriggerRecord::Complete,
    }
}

pub(crate) fn input(deliveries: Vec<TriggerDelivery>) -> ActivationInput {
    ActivationInput {
        scope: scope(),
        deliveries,
        origins: Vec::new(),
        anchor: "obs0".to_owned(),
        evicted: Vec::new(),
        ledger: None,
        execution: ExecutionDisposition::Completed,
    }
}

pub(crate) const THREE_CAPTURES: &str =
    "capture a: Int[0, 9] = q.level; capture b: Int[0, 9] = q.level; capture c2: Int[0, 9] = q.level; eventually[0, 2] holds(p)";
pub(crate) const ONE_CAPTURE: &str =
    "capture amount: Int[0, 9] = q.level; eventually[0, 2] holds(p)";
pub(crate) const EACH: &str = "on each (q: Counter)";
pub(crate) const GUARDED: &str = "on each (q: Counter) when (q.level > 0)";

/// Activate over `clause` with an unlimited meter.
pub(crate) fn run(
    env: &Env,
    input: ActivationInput,
) -> Result<qsl_eval::temporal::evidence::Activation, ActivationStop> {
    let mut meter = unlimited();
    let mut call = CallMeter::new(TemporalEvidenceLimits::default(), &mut meter);
    activate(
        &ClauseShape::of(&ClauseFacts::of_checked(&env.fixture.clause)),
        &env.obligation(),
        input,
        &mut call,
    )
}

pub(crate) fn activated(
    env: &Env,
    input: ActivationInput,
) -> qsl_eval::temporal::evidence::Activation {
    match run(env, input) {
        Ok(activation) => activation,
        Err(stop) => panic!("an activation, not {stop:?}"),
    }
}

pub(crate) fn fault_of(
    activation: &qsl_eval::temporal::evidence::Activation,
    identity: &str,
) -> (FaultSubject, FaultKind) {
    let instance = activation
        .instances
        .iter()
        .find(|instance| instance.identity == id(identity))
        .expect("the instance is reported");
    match &instance.state {
        InstanceState::Unavailable(fault) => (fault.subject.clone(), fault.kind),
        InstanceState::Active => panic!("{identity} is faulted"),
    }
}

#[trace("FR-524-AC-1", "FR-524-AC-7", "TC-1319")]
#[test]
fn distinct_identities_make_distinct_instances_even_with_equal_content() {
    let env = env(EACH, ONE_CAPTURE);
    let activation = activated(
        &env,
        input(vec![
            delivery("t1", "r1", "same", &["5"]),
            delivery("t2", "r1", "same", &["5"]),
        ]),
    );
    assert_eq!(activation.disposition, Disposition::Active);
    assert_eq!(activation.instances.len(), 2);
    assert_ne!(
        activation.instances[0].identity,
        activation.instances[1].identity
    );
    for instance in &activation.instances {
        let amount = instance.environment.get("amount").expect("the capture");
        assert_eq!(amount.value().as_bytes(), b"5");
        assert_eq!(amount.anchor(), "obs0");
        assert_eq!(amount.source().index, 0);
    }
    // One identity byte apart: two instances, equal in everything else.
    let close = activated(
        &env,
        input(vec![
            delivery("t1", "r1", "same", &["5"]),
            delivery("t\u{1}", "r1", "same", &["5"]),
        ]),
    );
    assert_eq!(close.instances.len(), 2);
}

#[trace(
    "FR-524-AC-3",
    "FR-524-AC-4",
    "FR-524-AC-19",
    "FR-516-AC-11",
    "TC-1319",
    "TC-1311"
)]
#[test]
fn a_redelivery_with_an_equal_payload_adds_a_receipt_and_keeps_the_environment() {
    let env = env(EACH, ONE_CAPTURE);
    let first = activated(&env, input(vec![delivery("t1", "relay-a", "p", &["5"])]));
    let before = first.instances[0].environment.clone();
    // Within one call.
    let both = activated(
        &env,
        input(vec![
            delivery("t1", "relay-a", "p", &["5"]),
            delivery("t1", "relay-b", "p", &["9"]),
        ]),
    );
    assert_eq!(both.instances.len(), 1);
    let receipts: Vec<&[u8]> = both.instances[0]
        .receipts
        .iter()
        .map(Receipt::as_bytes)
        .collect();
    assert_eq!(receipts, [b"relay-a".as_slice(), b"relay-b".as_slice()]);
    assert_eq!(both.instances[0].environment, before);
    // Against a ledger of an earlier call: the count stays one.
    let mut later = input(vec![delivery("t1", "relay-c", "p", &["7"])]);
    later.ledger = Some(first.ledger.clone());
    let again = activated(&env, later);
    assert_eq!(again.instances.len(), 1);
    assert_eq!(again.instances[0].evaluations, 1);
    assert_eq!(again.instances[0].environment, before);
    assert_eq!(again.instances[0].receipts.len(), 2);
    assert_eq!(
        again
            .ledger
            .instance(&env.obligation(), &id("t1"))
            .expect("retained")
            .receipts()
            .len(),
        2
    );
    // The ledger the first call returned is as it was.
    assert_eq!(first.ledger.instance_count(), 1);
    assert_eq!(
        first
            .ledger
            .instance(&env.obligation(), &id("t1"))
            .expect("retained")
            .receipts()
            .len(),
        1
    );
}

#[trace("FR-524-AC-5", "FR-524-AC-20", "TC-1319")]
#[test]
fn a_different_payload_under_one_identity_is_the_contradiction_refusal() {
    let env = env(EACH, ONE_CAPTURE);
    let refused = run(
        &env,
        input(vec![
            delivery("t1", "r1", "one", &["5"]),
            delivery("t1", "r2", "two", &["5"]),
        ]),
    );
    match refused {
        Err(ActivationStop::Contradiction(contradiction)) => {
            assert_eq!(contradiction.contradicted_identity, id("t1"));
        }
        other => panic!("a contradiction, not {other:?}"),
    }
    // Against the retained instance: refused, the ledger unchanged.
    let first = activated(&env, input(vec![delivery("t1", "r1", "one", &["5"])]));
    let ledger = first.ledger.clone();
    let mut later = input(vec![delivery("t1", "r2", "two", &["6"])]);
    later.ledger = Some(ledger.clone());
    match run(&env, later) {
        Err(ActivationStop::Contradiction(contradiction)) => {
            assert_eq!(contradiction.contradicted_identity, id("t1"));
        }
        other => panic!("a contradiction, not {other:?}"),
    }
    // The retained entry is the first delivery's: payload, one receipt, the
    // captured amount and one instance.
    let kept = ledger
        .instance(&env.obligation(), &id("t1"))
        .expect("retained");
    assert_eq!(kept.payload().as_bytes(), b"one");
    assert_eq!(kept.receipts().len(), 1);
    assert_eq!(kept.receipts()[0].as_bytes(), b"r1");
    assert_eq!(
        kept.environment()
            .get("amount")
            .expect("kept")
            .value()
            .as_bytes(),
        b"5"
    );
    assert_eq!(ledger.instance_count(), 1);
}

pub(crate) fn origin(identity: &str, payload: &str) -> ExecutionOrigin {
    ExecutionOrigin {
        identity: id(identity),
        payload: Payload::new(payload.as_bytes()),
        captures: Vec::new(),
    }
}

#[trace("FR-524-AC-8", "FR-524-AC-9", "TC-1319")]
#[test]
fn an_origin_clause_has_one_instance_per_origin_identity() {
    let env = env("on origin", "eventually[0, 2] holds(p)");
    let mut two = input(Vec::new());
    two.origins = vec![origin("o1", "a"), origin("o2", "a"), origin("o1", "a")];
    let activation = activated(&env, two);
    let identities: Vec<_> = activation
        .instances
        .iter()
        .map(|instance| instance.identity.clone())
        .collect();
    assert_eq!(identities, [id("o1"), id("o2")]);
    let mut clash = input(Vec::new());
    clash.origins = vec![origin("o1", "a"), origin("o1", "b")];
    match run(&env, clash) {
        Err(ActivationStop::Contradiction(contradiction)) => {
            assert_eq!(contradiction.contradicted_identity, id("o1"));
        }
        other => panic!("a contradiction, not {other:?}"),
    }
}

#[trace("FR-524-AC-10", "FR-524-AC-11", "TC-1319")]
#[test]
fn a_false_guard_makes_no_instance_and_an_unknown_guard_is_a_fault() {
    let env = env(GUARDED, ONE_CAPTURE);
    let mut falsified = delivery("t1", "r1", "p", &["5"]);
    falsified.guard = Some(false);
    let all_false = activated(&env, input(vec![falsified.clone()]));
    assert!(all_false.instances.is_empty());
    assert_eq!(all_false.guard_false, [id("t1")]);
    assert_eq!(all_false.disposition, Disposition::Inactive);
    assert_eq!(all_false.ledger.instance_count(), 0);
    let mut unknown = delivery("t2", "r1", "p", &["5"]);
    unknown.guard = None;
    let faulted = activated(&env, input(vec![falsified, unknown]));
    assert_eq!(
        fault_of(&faulted, "t2"),
        (FaultSubject::Guard, FaultKind::Missing)
    );
    // The faulted delivery is retained as an unavailable entry (FR-524).
    let kept = faulted
        .ledger
        .instance(&env.obligation(), &id("t2"))
        .expect("retained");
    assert!(kept.fault().is_some());
    assert_eq!(kept.environment().entries().len(), 0);
    assert_eq!(faulted.ledger.instance_count(), 1);
}

pub(crate) fn capture_named(name: &str) -> FaultSubject {
    FaultSubject::Capture(name.to_owned())
}

#[trace(
    "FR-524-AC-12",
    "FR-524-AC-13",
    "FR-524-AC-14",
    "FR-524-AC-15",
    "TC-1319"
)]
#[test]
fn every_capture_fault_names_its_input_and_kind() {
    let env = env(EACH, THREE_CAPTURES);
    let faults = |edit: &dyn Fn(&mut TriggerDelivery), evicted: Vec<Evicted>| {
        let mut broken = delivery("t1", "r1", "p", &["1", "2", "3"]);
        edit(&mut broken);
        let mut request = input(vec![broken]);
        request.evicted = evicted;
        fault_of(&activated(&env, request), "t1")
    };
    assert_eq!(
        faults(&|d| d.captures[1] = CaptureFact::Missing, vec![]),
        (capture_named("b"), FaultKind::Missing)
    );
    assert_eq!(
        faults(&|d| d.captures[2] = CaptureFact::Null, vec![]),
        (capture_named("c2"), FaultKind::Missing)
    );
    assert_eq!(
        faults(
            &|_| {},
            vec![Evicted::Capture {
                instance: id("t1"),
                capture: "a".to_owned()
            }]
        ),
        (capture_named("a"), FaultKind::Missing)
    );
    assert_eq!(
        faults(&|d| d.captures.truncate(2), vec![]),
        (capture_named("c2"), FaultKind::Missing)
    );
    assert_eq!(
        faults(&|d| d.captures.clear(), vec![]),
        (capture_named("a"), FaultKind::Missing)
    );
    assert_eq!(
        faults(&|d| d.captures[0] = CaptureFact::WrongType, vec![]),
        (capture_named("a"), FaultKind::Refused)
    );
    assert_eq!(
        faults(&|d| d.captures[0] = CaptureFact::Stale, vec![]),
        (capture_named("a"), FaultKind::Refused)
    );
    assert_eq!(
        faults(&|d| d.anchor = "other".to_owned(), vec![]),
        (FaultSubject::Anchor, FaultKind::Refused)
    );
    assert_eq!(
        faults(
            &|d| {
                d.captures[1] = CaptureFact::Value {
                    anchor: "other".to_owned(),
                    value: CapturedValue::new(b"2".as_slice()),
                }
            },
            vec![]
        ),
        (capture_named("b"), FaultKind::Refused)
    );
}

#[trace("FR-524-AC-16", "TC-1319")]
#[test]
fn faults_are_examined_in_a_fixed_order() {
    let env = env(EACH, THREE_CAPTURES);
    let mut anchored = delivery("t1", "r1", "p", &["1", "2", "3"]);
    anchored.anchor = "other".to_owned();
    anchored.captures[0] = CaptureFact::Missing;
    assert_eq!(
        fault_of(&activated(&env, input(vec![anchored])), "t1"),
        (FaultSubject::Anchor, FaultKind::Refused)
    );
    let mut layered = delivery("t1", "r1", "p", &["1", "2", "3"]);
    layered.captures[1] = CaptureFact::WrongType;
    let mut request = input(vec![layered]);
    request.evicted = vec![Evicted::Capture {
        instance: id("t1"),
        capture: "a".to_owned(),
    }];
    assert_eq!(
        fault_of(&activated(&env, request), "t1"),
        (capture_named("a"), FaultKind::Missing)
    );
    // An evicted record is read before the same capture's fact.
    let mut same = delivery("t1", "r1", "p", &["1", "2", "3"]);
    same.captures[0] = CaptureFact::WrongType;
    let mut request = input(vec![same]);
    request.evicted = vec![Evicted::Capture {
        instance: id("t1"),
        capture: "a".to_owned(),
    }];
    assert_eq!(
        fault_of(&activated(&env, request), "t1"),
        (capture_named("a"), FaultKind::Missing)
    );
}

#[trace("FR-524-AC-17", "TC-1319")]
#[test]
fn a_faulted_instance_leaves_its_sibling_active() {
    let env = env(EACH, ONE_CAPTURE);
    let mut broken = delivery("t2", "r1", "p", &["5"]);
    broken.captures[0] = CaptureFact::Missing;
    let activation = activated(&env, input(vec![delivery("t1", "r1", "p", &["5"]), broken]));
    assert_eq!(activation.disposition, Disposition::Active);
    assert!(matches!(
        activation.instances[0].state,
        InstanceState::Active
    ));
    assert_eq!(
        activation.instances[0]
            .environment
            .get("amount")
            .expect("kept")
            .value()
            .as_bytes(),
        b"5"
    );
    assert!(matches!(
        activation.instances[1].state,
        InstanceState::Unavailable(_)
    ));
    assert_eq!(activation.ledger.instance_count(), 2);
}

#[trace("FR-524-AC-18", "FR-524-AC-22", "TC-1319")]
#[test]
fn captures_keep_authored_order_and_no_later_delivery_changes_them() {
    let env = env(EACH, THREE_CAPTURES);
    let first = activated(
        &env,
        input(vec![delivery("t1", "r1", "p", &["7", "8", "9"])]),
    );
    let names: Vec<&str> = first.instances[0]
        .environment
        .entries()
        .iter()
        .map(|entry| entry.name())
        .collect();
    assert_eq!(names, ["a", "b", "c2"]);
    let indices: Vec<usize> = first.instances[0]
        .environment
        .entries()
        .iter()
        .map(|entry| entry.source().index)
        .collect();
    assert_eq!(indices, [0, 1, 2]);
    let mut changed = input(vec![delivery("t1", "r2", "p", &["1", "1", "1"])]);
    changed.ledger = Some(first.ledger.clone());
    let second = activated(&env, changed);
    assert_eq!(
        second.instances[0].environment,
        first.instances[0].environment
    );
    assert_eq!(second.instances[0].evaluations, 1);
}

#[trace("FR-524-AC-21", "TC-1319")]
#[test]
fn an_evicted_capture_of_a_retained_instance_is_a_missing_fault_and_recomputes_nothing() {
    let env = env(EACH, ONE_CAPTURE);
    let first = activated(&env, input(vec![delivery("t1", "r1", "p", &["5"])]));
    let mut later = input(vec![delivery("t1", "r2", "p", &["9"])]);
    later.ledger = Some(first.ledger.clone());
    later.evicted = vec![Evicted::Capture {
        instance: id("t1"),
        capture: "amount".to_owned(),
    }];
    let again = activated(&env, later);
    assert_eq!(
        fault_of(&again, "t1"),
        (capture_named("amount"), FaultKind::Missing)
    );
    assert_eq!(again.ledger, first.ledger);
}

pub(crate) fn charges(env: &Env, input: ActivationInput) -> [u64; 4] {
    let mut meter = unlimited();
    let mut call = CallMeter::new(TemporalEvidenceLimits::default(), &mut meter);
    activate(
        &ClauseShape::of(&ClauseFacts::of_checked(&env.fixture.clause)),
        &env.obligation(),
        input,
        &mut call,
    )
    .expect("an activation");
    let usage = call.usage();
    [
        usage.get(EvidenceCounter::EvaluationSteps),
        usage.get(EvidenceCounter::Instances),
        usage.get(EvidenceCounter::Captures),
        usage.get(EvidenceCounter::Visited),
    ]
}

#[trace("FR-524-AC-26", "TC-1319")]
#[test]
fn deliveries_captures_and_instances_are_charged_on_their_counters() {
    let env = env(EACH, ONE_CAPTURE);
    let usage = charges(
        &env,
        input(vec![
            delivery("t1", "r1", "p", &["5"]),
            delivery("t2", "r1", "p", &["5"]),
        ]),
    );
    // Two deliveries and two captures are four visits; two instances; two
    // captures retained; the captures are two of the visited units.
    assert_eq!(usage, [4, 2, 2, 2]);
}

#[trace(
    "FR-524-AC-27",
    "FR-524-AC-28",
    "FR-524-AC-29",
    "FR-524-AC-30",
    "TC-1319"
)]
#[test]
fn a_scope_with_no_instance_reports_one_of_four_dispositions() {
    let env = env(EACH, ONE_CAPTURE);
    let with = |closure, evidence, record| {
        let mut request = input(Vec::new());
        request.scope = TriggerScope {
            closure,
            evidence,
            record,
        };
        let activation = activated(&env, request);
        assert!(activation.instances.is_empty());
        activation.disposition
    };
    let unknown = |kind, completeness| Disposition::Unknown {
        kind,
        completeness,
        execution: ExecutionDisposition::Completed,
    };
    use qsl_eval::temporal::evidence::Completeness::{Complete, Incomplete};
    assert_eq!(
        with(
            ProgressClosure::Closed,
            TriggerEvidence::Admitted,
            TriggerRecord::Complete
        ),
        Disposition::Inactive
    );
    assert_eq!(
        with(
            ProgressClosure::Open,
            TriggerEvidence::Admitted,
            TriggerRecord::Complete
        ),
        unknown(UnknownKind::Open, Complete)
    );
    for closure in [ProgressClosure::Closed, ProgressClosure::Open] {
        assert_eq!(
            with(closure, TriggerEvidence::Missing, TriggerRecord::Complete),
            unknown(UnknownKind::Missing, Complete)
        );
        assert_eq!(
            with(closure, TriggerEvidence::Refused, TriggerRecord::Complete),
            unknown(UnknownKind::Refused, Complete)
        );
    }
    assert_eq!(
        with(
            ProgressClosure::Closed,
            TriggerEvidence::Admitted,
            TriggerRecord::Incomplete
        ),
        unknown(UnknownKind::Open, Incomplete)
    );
}

// The evidence call.

pub(crate) fn at_t0(values: &[(u64, Valuation)]) -> Vec<EvidencePosition> {
    vec![
        position(0, values),
        position(1, values),
        position(2, values),
    ]
}

pub(crate) fn call(
    env: &Env,
    subject: &EvidenceSubject<'_>,
    request: &EvidenceRequest,
) -> Result<EvidenceResult, CallFailure> {
    let _ = env;
    evaluate_temporal_evidence(subject, request, &mut unlimited())
}

pub(crate) fn expect_refusal(result: Result<EvidenceResult, CallFailure>, cause: &str) {
    match result {
        Err(CallFailure::Admission(failure)) => match *failure {
            CallAdmissionFailure::Refused(record) => {
                assert_eq!(
                    record.record.fields.get("cause").map(String::as_str),
                    Some(cause)
                );
            }
            other => panic!("a refusal, not {other:?}"),
        },
        Err(other) => panic!("an admission refusal, not {other:?}"),
        Ok(result) => panic!("a refusal, not {:?}", result.activation),
    }
}

pub(crate) fn each_env() -> (Env, u64) {
    let env = env(EACH, ONE_CAPTURE);
    let p = env.atom("p");
    (env, p)
}

#[trace("FR-516-AC-2", "FR-524-AC-31", "TC-1311", "TC-1319")]
#[test]
fn the_activation_states_follow_the_scope_dispositions_and_faults() {
    let (env, p) = each_env();
    let subject = env.subject(&[]);
    let positions = at_t0(&[(p, Valuation::True)]);
    let mut closed = request(&env, positions.clone());
    let result = call(&env, &subject, &closed).expect("a result");
    assert_eq!(result.activation, ActivationState::Inactive);
    assert_eq!(result.truth, None);
    assert_eq!(
        result.non_value.as_ref().map(|n| n.kind),
        Some(NonValueKind::Inactive)
    );
    closed.trigger_scope = ProgressClosure::Open;
    let result = call(&env, &subject, &closed).expect("a result");
    assert_eq!(result.activation, ActivationState::Unknown);
    assert_eq!(
        result.non_value.as_ref().map(|n| n.kind),
        Some(NonValueKind::ActivationUnknown)
    );
    let mut active = request(&env, positions.clone());
    active.trigger = Some(id("t1"));
    active.deliveries = vec![delivery("t1", "r1", "p", &["5"])];
    let result = call(&env, &subject, &active).expect("a result");
    assert_eq!(result.activation, ActivationState::Active);
    assert_eq!(result.truth, Some(Truth::True));
    assert_eq!(result.trigger, Some(id("t1")));
    let mut missing = active.clone();
    missing.deliveries[0].captures[0] = CaptureFact::Missing;
    let result = call(&env, &subject, &missing).expect("a result");
    assert_eq!(result.activation, ActivationState::Unavailable);
    assert_eq!(result.truth, None);
    let named = result.non_value.expect("a non-value");
    assert_eq!(
        (named.kind, named.dimension.as_deref()),
        (NonValueKind::Missing, Some("amount"))
    );
    let mut withdrawn = active.clone();
    withdrawn.anchor = None;
    let result = call(&env, &subject, &withdrawn).expect("a result");
    assert_eq!(result.activation, ActivationState::Unavailable);
    assert_eq!(
        result.non_value.expect("a non-value").dimension.as_deref(),
        Some("anchor")
    );
}

#[trace("FR-516-AC-23", "FR-516-AC-24", "TC-1311")]
#[test]
fn an_anchor_naming_no_position_is_unavailable_not_inactive() {
    let (env, p) = each_env();
    let subject = env.subject(&[]);
    let mut stray = request(&env, at_t0(&[(p, Valuation::True)]));
    stray.anchor = Some("nowhere".to_owned());
    let result = call(&env, &subject, &stray).expect("a result");
    assert_eq!(result.activation, ActivationState::Unavailable);
    assert_eq!(result.truth, None);
    assert_eq!(
        result.non_value.expect("a non-value").kind,
        NonValueKind::Missing
    );
}

#[trace("FR-516-AC-10", "FR-516-AC-12", "FR-524-AC-6", "TC-1311", "TC-1319")]
#[test]
fn deliveries_resolve_instances_and_a_contradiction_is_refused_uncharged() {
    let (env, p) = each_env();
    let subject = env.subject(&[]);
    let mut two = request(&env, at_t0(&[(p, Valuation::True)]));
    two.trigger = Some(id("t1"));
    two.deliveries = vec![
        delivery("t1", "r", "p", &["5"]),
        delivery("t2", "r", "p", &["5"]),
        delivery("t1", "r2", "p", &["5"]),
    ];
    let result = call(&env, &subject, &two).expect("a result");
    let identities: Vec<_> = result
        .instances
        .iter()
        .map(|i| i.identity.clone())
        .collect();
    assert_eq!(identities, [id("t1"), id("t2")]);
    assert_eq!(result.instances[0].receipts.len(), 2);
    let mut clash = two.clone();
    clash.deliveries[2].payload = Payload::new(b"other".as_slice());
    let mut meter = unlimited();
    let refused = evaluate_temporal_evidence(&subject, &clash, &mut meter);
    expect_refusal(refused, "delivery-contradiction");
    assert!(meter.admitted_charges().is_empty());
}

#[trace("FR-516-AC-19", "FR-516-AC-20", "FR-516-AC-21", "TC-1311")]
#[test]
fn the_trigger_must_select_a_resolved_instance() {
    let (env, p) = each_env();
    let subject = env.subject(&[]);
    let mut lists = request(&env, at_t0(&[(p, Valuation::True)]));
    lists.deliveries = vec![delivery("t1", "r", "p", &["5"])];
    expect_refusal(call(&env, &subject, &lists), "trigger-required");
    lists.trigger = Some(id("nobody"));
    expect_refusal(call(&env, &subject, &lists), "trigger-unknown");
    let guarded = self::env(GUARDED, ONE_CAPTURE);
    let gp = guarded.atom("p");
    let gsubject = guarded.subject(&[]);
    let mut off = request(&guarded, at_t0(&[(gp, Valuation::True)]));
    let mut falsified = delivery("t1", "r", "p", &["5"]);
    falsified.guard = Some(false);
    off.deliveries = vec![falsified];
    off.trigger = Some(id("t1"));
    let result = call(&guarded, &gsubject, &off).expect("a result");
    assert_eq!(result.truth, None);
    assert_eq!(result.activation, ActivationState::Inactive);
    assert!(result.instances.is_empty());
}

#[trace("FR-524-AC-1", "FR-524-AC-2", "TC-1319")]
#[test]
fn an_instance_bound_atom_reads_only_its_own_instance() {
    let (env, p) = each_env();
    let subject = env.subject(&[p]);
    let mut positions = at_t0(&[]);
    let mut only_t2 = BTreeMap::new();
    only_t2.insert(p, Valuation::True);
    for position in &mut positions {
        position
            .instance_valuations
            .insert(id("t2"), only_t2.clone());
    }
    let mut asked = request(&env, positions);
    asked.deliveries = vec![
        delivery("t1", "r", "p", &["5"]),
        delivery("t2", "r", "p", &["5"]),
    ];
    asked.trigger = Some(id("t1"));
    let t1 = call(&env, &subject, &asked).expect("a result");
    assert_eq!(t1.truth, None);
    match t1.indeterminate.expect("an indeterminate reason") {
        Indeterminate::MissingValuation {
            atom,
            instance,
            also_held_by,
            ..
        } => {
            assert_eq!(atom, p);
            assert_eq!(instance, Some(id("t1")));
            assert_eq!(also_held_by, [id("t2")]);
        }
        other => panic!("a missing valuation, not {other:?}"),
    }
    asked.trigger = Some(id("t2"));
    let t2 = call(&env, &subject, &asked).expect("a result");
    assert_eq!(t2.truth, Some(Truth::True));
    assert_eq!(t2.settlement, Some(Settlement::ClosedScope));
}

#[trace("FR-516-AC-15", "FR-516-AC-16", "TC-1311")]
#[test]
fn control_must_map_to_the_clause() {
    let (base, p) = each_env();
    let env = base.with_row("Ctl");
    let subject = env.subject(&[]);
    let mut asked = request(&env, at_t0(&[(p, Valuation::True)]));
    asked.control = Some(node("Ctl"));
    assert!(call(&env, &subject, &asked).is_ok());
    asked.control = Some(node("Nope"));
    expect_refusal(call(&env, &subject, &asked), "activation-mapping-missing");
    let mut other = env.subject(&[]);
    let elsewhere = vec![ActivationRow {
        control: node("Ctl"),
        clause: node("Else"),
    }];
    other.activation_map = &elsewhere;
    asked.control = Some(node("Ctl"));
    expect_refusal(call(&env, &other, &asked), "activation-mapping-mismatch");
}

#[trace("FR-516-AC-22", "TC-1311")]
#[test]
fn an_each_clause_activates_at_its_anchor_and_an_origin_clause_at_zero() {
    // `holds(p)` is true at stamp 2 only: the activation at obs2 sees it, the
    // activation at obs0 (eventually within [0,1]) does not.
    let env = env(
        EACH,
        "capture amount: Int[0, 9] = q.level; eventually[0, 1] holds(p)",
    );
    let p = env.atom("p");
    let subject = env.subject(&[]);
    let mut positions = vec![
        position(0, &[(p, Valuation::False)]),
        position(1, &[(p, Valuation::False)]),
        position(2, &[(p, Valuation::True)]),
    ];
    positions[2].observation = EvidenceRef::named("obs2");
    let mut asked = request(&env, positions.clone());
    asked.trigger = Some(id("t1"));
    asked.deliveries = vec![delivery("t1", "r", "p", &["5"])];
    let at_zero = call(&env, &subject, &asked).expect("a result");
    assert_eq!(at_zero.truth, Some(Truth::False));
    asked.anchor = Some("obs2".to_owned());
    asked.deliveries[0].anchor = "obs2".to_owned();
    asked.deliveries[0].captures = vec![CaptureFact::Value {
        anchor: "obs2".to_owned(),
        value: CapturedValue::new(b"5".as_slice()),
    }];
    let at_two = call(&env, &subject, &asked).expect("a result");
    assert_eq!(at_two.truth, Some(Truth::True));
    // `on origin`: position 0 whatever the anchor names.
    let origin_env = self::env("on origin", "eventually[0, 1] holds(p)");
    let op = origin_env.atom("p");
    let origin_subject = origin_env.subject(&[]);
    let mut origin_request = request(
        &origin_env,
        vec![
            position(0, &[(op, Valuation::False)]),
            position(1, &[(op, Valuation::False)]),
            position(2, &[(op, Valuation::True)]),
        ],
    );
    origin_request.anchor = Some("obs2".to_owned());
    origin_request.origins = vec![origin("o1", "a")];
    origin_request.trigger = Some(id("o1"));
    let result = call(&origin_env, &origin_subject, &origin_request).expect("a result");
    assert_eq!(result.truth, Some(Truth::False));
}

#[trace("FR-516-AC-7", "TC-1311")]
#[test]
fn each_counter_completes_at_its_usage_and_one_below_is_incomplete() {
    let (env, p) = each_env();
    let subject = env.subject(&[]);
    let mut asked = request(&env, at_t0(&[(p, Valuation::True)]));
    asked.deliveries = vec![delivery("t1", "r", "p", &["5"])];
    asked.trigger = Some(id("t1"));
    let full = call(&env, &subject, &asked).expect("a result");
    assert_eq!(full.truth, Some(Truth::True));
    let measured = full.usage;
    assert!(measured.get(EvidenceCounter::Positions) > 0);
    for counter in EvidenceCounter::ALL {
        let used = measured.get(counter);
        if used == 0 {
            continue;
        }
        let mut at = asked.clone();
        at.limits = TemporalEvidenceLimits::default().with_ceiling(counter, used);
        let done = call(&env, &subject, &at).expect("a result");
        assert_eq!(
            done.truth,
            Some(Truth::True),
            "{counter:?} at its usage completes"
        );
        let mut below = asked.clone();
        below.limits = TemporalEvidenceLimits::default().with_ceiling(counter, used - 1);
        let stopped = call(&env, &subject, &below).expect("a result");
        assert_eq!(stopped.truth, None, "{counter:?} one below stops");
        match stopped.incomplete {
            Some(EvidenceIncomplete::Limit(reached)) => {
                assert_eq!(reached.counter, counter);
            }
            other => panic!("{counter:?}: a counter stop, not {other:?}"),
        }
    }
}

#[trace("FR-516-AC-17", "FR-502-AC-1", "TC-1311")]
#[test]
fn a_regressing_watermark_is_refused_and_the_progress_ledger_is_unchanged() {
    let (env, p) = each_env();
    let subject = env.subject(&[]);
    let mut asked = request(&env, at_t0(&[(p, Valuation::True)]));
    asked.progress_ledger = Some(qsl_eval::temporal::ProgressLedger::new());
    let first = call(&env, &subject, &asked).expect("a result");
    let kept = first.progress_ledger.clone().expect("a progress ledger");
    asked.progress_ledger = Some(kept.clone());
    asked.decision_progress = Some(50);
    let mut meter = unlimited();
    let refused = evaluate_temporal_evidence(&subject, &asked, &mut meter);
    expect_refusal(refused, "progress-contradiction");
    assert!(meter.admitted_charges().is_empty());
    let binding = {
        let mut b = subject.progress.clone();
        b.occurrence = 0;
        b
    };
    assert_eq!(kept.entry(&binding).expect("an entry").watermark(), 100);
}

#[trace("FR-516-AC-2", "TC-1311")]
#[test]
fn an_event_position_clause_is_evaluated_through_the_progress_carrier() {
    let env = event_env("on origin", "eventually[0, 1] holds(p)");
    let p = env.atom("p");
    let subject = env.subject(&[]);
    let at = |values: [Valuation; 3]| {
        let mut asked = request(
            &env,
            values
                .into_iter()
                .enumerate()
                .map(|(index, value)| position(index as u64, &[(p, value)]))
                .collect(),
        );
        asked.origins = vec![origin("o1", "a")];
        asked.trigger = Some(id("o1"));
        asked
    };
    let holds = call(
        &env,
        &subject,
        &at([Valuation::False, Valuation::True, Valuation::False]),
    )
    .expect("a result");
    assert_eq!(holds.activation, ActivationState::Active);
    assert_eq!(holds.truth, Some(Truth::True));
    let fails = call(
        &env,
        &subject,
        &at([Valuation::False, Valuation::False, Valuation::False]),
    )
    .expect("a result");
    assert_eq!(fails.truth, Some(Truth::False));
}
