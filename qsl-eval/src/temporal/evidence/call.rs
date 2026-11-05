// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-516: the temporal evidence call, `evaluate_temporal_evidence`.
//!
//! The call refuses before it charges anything (a refused binding, control,
//! progress, relation, delivery contradiction or trigger selection is
//! `Err(CallFailure::Admission)` with an empty charge log), then admits the
//! clause, resolves the instances (FR-524), assesses the one the request's
//! `trigger` names under the finite-window evaluator (FR-501) and chains the
//! result to its predecessor (FR-526).

use std::collections::{BTreeMap, BTreeSet};

use qsl_semantics::check::{Activation as ClauseActivation, TemporalProfile};
use quire_eval::{CallAdmissionFailure, CallAdmissionLocus, CallAdmissionRecord, CallFailure};
use quire_exact::{ChargePoint, Incomplete, LimitKind, Meter};
use quire_semantic_value::admission::AdmissionRecord;
use quire_semantic_value::catalog::{CatalogCode, InternalFault};

use super::activation::{
    activate, retained_instance, scope_disposition, ActivationInput, ActivationStop, ClauseShape,
    Disposition, FaultKind, FaultSubject, Instance, InstanceState, UnknownKind,
};
use super::binding::{AuthenticatedBinding, ClauseSelection};
use super::counters::{CallMeter, ClauseStop, EvidenceStop, LimitReached, Visit};
use super::identity::{
    IdentityBuilder, IdentityMember, InputIdentity, MemberEncoder, ResultIdentity,
};
use super::ledger::{EvidenceLedger, ObligationKey, ResultEntry};
use super::limits::{EvidenceCounter, EvidenceUsage};
use super::request::{
    ActivationState, EvidenceIncomplete, EvidencePosition, EvidenceRef, EvidenceRequest,
    EvidenceResult, EvidenceSubject, Indeterminate, NonValue, NonValueKind, Relation, Settlement,
    SupportRef, Truth,
};
use super::trigger::{
    CaptureFact, Evicted, SemanticTriggerIdentity, TriggerEvidence, TriggerRecord, TriggerScope,
};
use super::work::ReachFault;
use crate::temporal::finite_window::{
    evaluate_finite_window_at, Basis, ClockCover, IndeterminateReason, LowerBoundary,
    ProgressedRefusal, SampleProgress, TimestampedObservation, TimestampedTrace, Valuation,
    Watermark, WindowCharge, WindowClock, WindowProgress, WindowUnit, WindowVerdict,
};
use crate::temporal::ledger::{
    ObservedPosition, ProgressAssertion, ProgressClosure, ProgressCompleteness,
};
use crate::temporal::letters::{
    AtomReading, CaptureReading, LetterStop, TemporalLetters, UndefinedCause,
};
use crate::temporal::outcome::TemporalRefusal;
use crate::temporal::position::TemporalPosition;
use crate::temporal::window::{
    AggregateKind, AggregateSample, InsufficientData, WindowRead, WindowSource,
};
use qsl_semantics::check::FormulaNodeId;

pub(super) fn fault(invariant: &'static str) -> CallFailure {
    CallFailure::Fault(InternalFault::new("temporal.evidence", invariant))
}

/// `Err(CallFailure::Admission)` for the member `member`, with `cause` and
/// `fields` in the diagnostic.
pub(super) fn refusal(
    cause: &'static str,
    member: &str,
    fields: &[(&'static str, String)],
) -> CallFailure {
    let mut named = BTreeMap::from([("cause", cause.to_owned())]);
    named.extend(fields.iter().map(|(name, value)| (*name, value.clone())));
    CallFailure::Admission(Box::new(CallAdmissionFailure::Refused(
        CallAdmissionRecord {
            record: AdmissionRecord {
                catalog: CatalogCode::new("invalid_runtime_input", "invalid-value"),
                fields: named,
                path: Vec::new(),
                kernel: None,
            },
            locus: CallAdmissionLocus::Member {
                name: member.to_owned(),
                index: None,
            },
        },
    )))
}

/// The subject with the occurrence the request selected and its clock row.
struct ResolvedSubject<'p> {
    selection: ClauseSelection<'p>,
    facts: &'p crate::temporal::facts::ClauseFacts,
    instance_bound: &'p BTreeSet<u64>,
    activation_map: &'p [super::request::ActivationRow],
    progress: crate::temporal::ledger::LedgerBinding,
    progress_capacity: u64,
    max_window_values: u64,
    occurrence: u64,
    occurrence_key: Vec<u8>,
}

impl ResolvedSubject<'_> {
    fn shape(&self) -> ClauseShape {
        ClauseShape::of(self.facts)
    }
}

/// The occurrence of `occurrences` that `requested` names by its key
/// `{ role, ordinal }`, never by position; the only occurrence when none is
/// requested and the node has one.
///
/// # Errors
///
/// `occurrence-required` when the node has several occurrences and none is
/// requested, `occurrence-unknown` when `requested` names none of them.
pub(super) fn select_occurrence<'a, 'p>(
    occurrences: &'a [super::request::SubjectOccurrence<'p>],
    requested: Option<&qsl_package::CheckedOccurrence>,
) -> Result<&'a super::request::SubjectOccurrence<'p>, CallFailure> {
    match (requested, occurrences) {
        (Some(key), rows) => rows
            .iter()
            .find(|row| row.occurrence == *key)
            .ok_or_else(|| refusal("occurrence-unknown", "occurrence", &[])),
        (None, [only]) => Ok(only),
        (None, _) => Err(refusal("occurrence-required", "occurrence", &[])),
    }
}

/// Select the occurrence the request names by its key `{ role, ordinal }`,
/// never by position (FR-516 `occurrence`).
fn resolve<'p>(
    subject: &'p EvidenceSubject<'p>,
    request: &EvidenceRequest,
) -> Result<ResolvedSubject<'p>, CallFailure> {
    let chosen = select_occurrence(&subject.occurrences, request.occurrence.as_ref())?;
    let occurrence_key = serde_json::to_vec(&(subject.declaration, &chosen.occurrence))
        .map_err(|_| fault("clause-identity-encodes"))?;
    Ok(ResolvedSubject {
        selection: ClauseSelection {
            declaration: subject.declaration,
            profile: subject.profile,
            clock: chosen.clock,
        },
        facts: &subject.facts,
        instance_bound: &subject.instance_bound,
        activation_map: subject.activation_map,
        progress: crate::temporal::ledger::LedgerBinding {
            occurrence: chosen.occurrence.ordinal,
            ..subject.progress.clone()
        },
        progress_capacity: subject.progress_capacity,
        max_window_values: subject.max_window_values,
        occurrence: chosen.occurrence.ordinal,
        occurrence_key,
    })
}

/// Everything of the request that is not an instance, derived once.
struct Facts<'a> {
    subject: &'a ResolvedSubject<'a>,
    request: &'a EvidenceRequest,
    authenticated: AuthenticatedBinding,
    obligation: ObligationKey,
    input_identity: InputIdentity,
    result_identity: ResultIdentity,
    lineage: Option<u64>,
    progress_ledger: Option<crate::temporal::ledger::ProgressLedger>,
    /// The covered-position limit the progress assertion reached, if any.
    progress_stop: Option<crate::temporal::ledger::CoveredPositionsLimitReached>,
}

/// What a call concluded, before the common members are added.
struct Outcome {
    activation: ActivationState,
    usage: EvidenceUsage,
    truth: Option<Truth>,
    non_value: Option<NonValue>,
    settlement: Option<Settlement>,
    indeterminate: Option<Indeterminate>,
    incomplete: Option<EvidenceIncomplete>,
    support: Vec<SupportRef>,
    instances: Vec<Instance>,
    ledger: Option<EvidenceLedger>,
}

impl Outcome {
    fn of(activation: ActivationState) -> Self {
        Self {
            activation,
            usage: EvidenceUsage::default(),
            truth: None,
            non_value: None,
            settlement: None,
            indeterminate: None,
            incomplete: None,
            support: Vec::new(),
            instances: Vec::new(),
            ledger: None,
        }
    }

    fn with_non_value(
        mut self,
        kind: NonValueKind,
        code: &'static str,
        dimension: Option<String>,
    ) -> Self {
        self.non_value = Some(NonValue {
            kind,
            code,
            dimension,
            detail: Vec::new(),
        });
        self
    }

    fn stopped(stop: EvidenceIncomplete) -> Self {
        Self {
            incomplete: Some(stop),
            ..Self::of(ActivationState::Unavailable)
        }
    }
}

fn incomplete_of(stop: EvidenceStop) -> EvidenceIncomplete {
    match stop {
        EvidenceStop::Limit(reached) => EvidenceIncomplete::Limit(reached),
        EvidenceStop::Meter {
            incomplete,
            instance,
        } => EvidenceIncomplete::Meter {
            incomplete,
            instance,
        },
    }
}

/// Evaluate the temporal clause `subject` over `request` (FR-516).
///
/// One call assesses one instance, which activates once. The request's
/// `trigger` names the assessed instance among those its `deliveries` or
/// `origins` resolve; the others are reported in `instances` and not assessed.
///
/// # Errors
///
/// `Err(CallFailure::Admission)` with an empty charge log for a refused
/// binding (FR-522), a control that does not map to the clause, a progress
/// contradiction (FR-502), a relation refusal (FR-526), a delivery
/// contradiction (FR-524) or a `trigger-required` / `trigger-unknown`
/// selection; `CallFailure::Fault` for a broken invariant.
pub fn evaluate_temporal_evidence(
    subject: &EvidenceSubject<'_>,
    request: &EvidenceRequest,
    meter: &mut Meter,
) -> Result<EvidenceResult, CallFailure> {
    let resolved = resolve(subject, request)?;
    let facts = admit(&resolved, request, meter)?;
    let shape = resolved.shape();
    let outcome = assess(&facts, &shape, meter)?;
    Ok(conclude(&facts, outcome, meter))
}

fn admit<'a>(
    subject: &'a ResolvedSubject<'a>,
    request: &'a EvidenceRequest,
    meter: &Meter,
) -> Result<Facts<'a>, CallFailure> {
    admit_aggregate_samples(subject.facts.formula(), request)?;
    let declaration = subject.selection.declaration;
    let authenticated = request
        .binding
        .authenticate(&subject.selection)
        .map_err(|mismatch| {
            refusal(
                "binding-mismatch",
                "binding",
                &[
                    ("mismatched_dimension", mismatch.dimension.name().to_owned()),
                    ("asserted", mismatch.asserted.clone()),
                    ("expected", mismatch.expected.clone().unwrap_or_default()),
                ],
            )
        })?;
    if let Some(control) = &request.control {
        let row = subject
            .activation_map
            .iter()
            .find(|row| row.control == *control);
        match row {
            None => return Err(refusal("activation-mapping-missing", "control", &[])),
            Some(row) if row.clause != *declaration => {
                return Err(refusal("activation-mapping-mismatch", "control", &[]));
            }
            Some(_) => {}
        }
    }
    let obligation = ObligationKey::new(
        declaration.package.clone(),
        subject.occurrence_key.clone(),
        subject.occurrence,
    );
    let (progress_ledger, progress_stop) = match admit_progress(subject, request)? {
        ProgressAdmission::Ledger(ledger) => (ledger, None),
        ProgressAdmission::Limit(limit) => (request.progress_ledger.clone(), Some(limit)),
    };
    let identity = identity_of(subject, request, &authenticated, &obligation, meter)
        .map_err(|_| fault("identity-members-encode"))?;
    let (result_identity, input_identity) = (identity.result_identity(), identity.input_identity());
    let lineage = admit_relation(request, &obligation, &input_identity)?;
    match subject.facts.activation() {
        ClauseActivation::Each { .. } if !request.origins.is_empty() => {
            return Err(refusal("activation-list-mismatch", "origins", &[]));
        }
        ClauseActivation::Origin if !request.deliveries.is_empty() => {
            return Err(refusal("activation-list-mismatch", "deliveries", &[]));
        }
        ClauseActivation::Each { .. } | ClauseActivation::Origin => {}
    }
    Ok(Facts {
        subject,
        request,
        authenticated,
        obligation,
        input_identity,
        result_identity,
        lineage,
        progress_ledger,
        progress_stop,
    })
}

/// Refuse a sample of a kind its package aggregate does not consume before
/// the call charges any work (FR-516-AC-50).
fn admit_aggregate_samples(
    formula: &crate::temporal::facts::ClauseFormula,
    request: &EvidenceRequest,
) -> Result<(), CallFailure> {
    let refuse = |atom: u64, position: u64| {
        refusal(
            "aggregate-sample-kind",
            "aggregate_samples",
            &[
                ("atom", atom.to_string()),
                ("position", position.to_string()),
            ],
        )
    };
    for supplied in &request.positions {
        for (atom, sample) in &supplied.aggregate_samples {
            let valid = formula
                .aggregate_atom(*atom)
                .is_some_and(|aggregate| sample.is_taken_by(aggregate.kind()));
            if !valid {
                return Err(refuse(*atom, supplied.position));
            }
        }
        for (instance, samples) in &supplied.instance_aggregate_samples {
            let _ = instance;
            for (atom, sample) in samples {
                let valid = formula
                    .aggregate_atom(*atom)
                    .is_some_and(|aggregate| sample.is_taken_by(aggregate.kind()));
                if !valid {
                    return Err(refuse(*atom, supplied.position));
                }
            }
        }
    }
    Ok(())
}

/// FR-502 as the call applies it: the contradiction is a refusal and the
/// progress ledger the caller holds is then unchanged.
/// What the call's progress assertion became (FR-502).
enum ProgressAdmission {
    /// The updated progress ledger, or none when the request holds none.
    Ledger(Option<crate::temporal::ledger::ProgressLedger>),
    /// The covered positions outgrew the system setting: an `Incomplete`.
    Limit(crate::temporal::ledger::CoveredPositionsLimitReached),
}

fn admit_progress(
    subject: &ResolvedSubject<'_>,
    request: &EvidenceRequest,
) -> Result<ProgressAdmission, CallFailure> {
    let Some(ledger) = &request.progress_ledger else {
        return Ok(ProgressAdmission::Ledger(None));
    };
    let (Some(watermark), Some(scope), Some(execution), Some(record)) = (
        request.decision_progress,
        request.decision_closure,
        request.surrounding_closure,
        request.completeness.as_ref(),
    ) else {
        // A fact is withdrawn: there is no assertion to retain.
        return Ok(ProgressAdmission::Ledger(Some(ledger.clone())));
    };
    let positions: Vec<ObservedPosition> = request
        .positions
        .iter()
        .map(|position| ObservedPosition {
            identity: position.observation.identity.clone(),
            stamp: position.position,
        })
        .collect();
    let assertion = ProgressAssertion {
        watermark,
        completeness: record.state,
        scope,
        execution,
    };
    match ledger.admit(
        &subject.progress,
        assertion,
        &positions,
        subject.progress_capacity,
    ) {
        Ok(updated) => Ok(ProgressAdmission::Ledger(Some(updated))),
        Err(crate::temporal::ledger::LedgerRefusal::Contradiction(contradiction)) => Err(refusal(
            "progress-contradiction",
            "progress_ledger",
            &[(
                "contradicted_dimension",
                format!("{:?}", contradiction.dimension),
            )],
        )),
        Err(crate::temporal::ledger::LedgerRefusal::LimitReached(limit)) => {
            Ok(ProgressAdmission::Limit(limit))
        }
    }
}

/// FR-526: the lineage the request's relation gives, or its refusal. A sum
/// that does not fit is `None`, which the call reports as `Incomplete`
/// naming `lineage`.
fn admit_relation(
    request: &EvidenceRequest,
    obligation: &ObligationKey,
    input: &InputIdentity,
) -> Result<Option<u64>, CallFailure> {
    let prev = match &request.relation {
        Relation::Original => return Ok(Some(0)),
        Relation::Superseding(prev) | Relation::Invalidating(prev) => prev,
    };
    let ledger = request
        .ledger
        .as_ref()
        .ok_or_else(|| refusal("no-ledger", "ledger", &[]))?;
    let entry = ledger
        .result(obligation, request.trigger.as_ref())
        .ok_or_else(|| refusal("predecessor-unknown", "relation", &[]))?;
    if entry.result_identity() != prev {
        return Err(refusal("predecessor-not-latest", "relation", &[]));
    }
    if entry.input_identity() == input {
        return Err(refusal("unchanged-input", "relation", &[]));
    }
    if entry.correspondence() != encode_ref(MemberEncoder::new(), &request.correspondence).finish()
    {
        return Err(refusal("correspondence-changed", "relation", &[]));
    }
    Ok(entry.lineage().checked_add(1))
}

fn encode_ref(encoder: MemberEncoder, evidence: &EvidenceRef) -> MemberEncoder {
    encoder
        .text(&evidence.identity)
        .text(&evidence.authority_identity)
        .text(&evidence.authority_revision)
        .text(&evidence.scope)
        .text(&evidence.population)
        .text(&evidence.contract)
        .bytes(&evidence.schema_digest)
        .bytes(&evidence.digest)
        .bytes(&evidence.authority_digest)
}

const fn closure_code(closure: Option<ProgressClosure>) -> u64 {
    match closure {
        None => 0,
        Some(ProgressClosure::Closed) => 1,
        Some(ProgressClosure::Open) => 2,
    }
}

fn encode_completeness(encoder: MemberEncoder, state: ProgressCompleteness) -> MemberEncoder {
    match state {
        ProgressCompleteness::Complete { through } => encoder.flag(true).u64(through),
        ProgressCompleteness::Incomplete => encoder.flag(false),
    }
}

fn encode_valuation(encoder: MemberEncoder, valuation: &Valuation) -> MemberEncoder {
    match valuation {
        Valuation::True => encoder.u64(1),
        Valuation::False => encoder.u64(2),
        Valuation::Missing => encoder.u64(3),
        Valuation::Undefined(_) => encoder.u64(4),
    }
}

fn encode_valuations(
    mut encoder: MemberEncoder,
    valuations: &BTreeMap<u64, Valuation>,
) -> MemberEncoder {
    encoder = encoder.u64(valuations.len() as u64);
    for (atom, valuation) in valuations {
        encoder = encode_valuation(encoder.u64(*atom), valuation);
    }
    encoder
}

fn encode_aggregate_samples(
    mut encoder: MemberEncoder,
    samples: &BTreeMap<u64, AggregateSample>,
) -> MemberEncoder {
    encoder = encoder.u64(samples.len() as u64);
    for (atom, sample) in samples {
        encoder = encoder.u64(*atom);
        encoder = match sample {
            AggregateSample::Excluded => encoder.u64(1),
            AggregateSample::Value(value) => encoder
                .u64(2)
                .text(&value.numerator().to_string())
                .text(&value.denominator().to_string()),
            AggregateSample::Holds(value) => encoder.u64(3).flag(*value),
            AggregateSample::Counted => encoder.u64(4),
            AggregateSample::Undefined(cause) => encoder.u64(5).text(&format!("{cause:?}")),
        };
    }
    encoder
}

/// The positions in the canonical order of FR-501, so the listing decides
/// nothing.
fn canonical(positions: &[EvidencePosition]) -> Vec<&EvidencePosition> {
    let mut ordered: Vec<&EvidencePosition> = positions.iter().collect();
    ordered.sort_by(|left, right| {
        left.position
            .cmp(&right.position)
            .then_with(|| match (&left.order_key, &right.order_key) {
                (Some(left), Some(right)) => left.cmp(right),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            })
            .then_with(|| left.observation.identity.cmp(&right.observation.identity))
    });
    ordered
}

fn encode_captures(mut encoder: MemberEncoder, captures: &[CaptureFact]) -> MemberEncoder {
    encoder = encoder.u64(captures.len() as u64);
    for fact in captures {
        encoder = match fact {
            CaptureFact::Value { anchor, value } => {
                encoder.u64(1).text(anchor).bytes(value.as_bytes())
            }
            CaptureFact::Missing => encoder.u64(2),
            CaptureFact::Null => encoder.u64(3),
            CaptureFact::WrongType => encoder.u64(4),
            CaptureFact::Stale => encoder.u64(5),
        };
    }
    encoder
}

fn encode_trace(subject: &ResolvedSubject<'_>, request: &EvidenceRequest) -> Vec<u8> {
    let mut encoder = MemberEncoder::new().u64(request.positions.len() as u64);
    // The listing is the order of an event-position or fixed-sample trace;
    // a timestamped trace has the canonical order of FR-501.
    let ordered = if subject.facts.profile() == TemporalProfile::TimestampedEventFiniteWindow {
        canonical(&request.positions)
    } else {
        request.positions.iter().collect()
    };
    for position in ordered {
        encoder = encode_ref(encoder, &position.observation).u64(position.position);
        encoder = encoder.option(position.order_key.as_ref(), |encoder, key| {
            encoder.text(&key.authority).u64(key.key as u64)
        });
        encoder = encode_valuations(encoder, &position.valuations);
        encoder = encode_aggregate_samples(encoder, &position.aggregate_samples);
        encoder = encoder.u64(position.instance_valuations.len() as u64);
        for (instance, valuations) in &position.instance_valuations {
            encoder = encode_valuations(encoder.bytes(instance.as_bytes()), valuations);
        }
        encoder = encoder.u64(position.instance_aggregate_samples.len() as u64);
        for (instance, samples) in &position.instance_aggregate_samples {
            encoder = encode_aggregate_samples(encoder.bytes(instance.as_bytes()), samples);
        }
    }
    encoder.finish()
}

fn encode_deliveries(request: &EvidenceRequest) -> Vec<u8> {
    let mut encoder = MemberEncoder::new().u64(request.deliveries.len() as u64);
    for delivery in &request.deliveries {
        encoder = encoder
            .bytes(delivery.identity.as_bytes())
            .bytes(delivery.receipt.as_bytes())
            .text(&delivery.anchor)
            .bytes(delivery.payload.as_bytes())
            .option(delivery.guard.as_ref(), |encoder, guard| {
                encoder.flag(*guard)
            });
        encoder = encode_captures(encoder, &delivery.captures);
    }
    encoder.finish()
}

fn encode_origins(request: &EvidenceRequest) -> Vec<u8> {
    let mut encoder = MemberEncoder::new().u64(request.origins.len() as u64);
    for origin in &request.origins {
        encoder = encoder
            .bytes(origin.identity.as_bytes())
            .bytes(origin.payload.as_bytes());
        encoder = encode_captures(encoder, &origin.captures);
    }
    encoder.finish()
}

fn encode_evicted(request: &EvidenceRequest) -> Vec<u8> {
    let mut encoder = MemberEncoder::new().u64(request.evicted.len() as u64);
    for record in &request.evicted {
        encoder = match record {
            Evicted::Capture { instance, capture } => {
                encoder.u64(1).bytes(instance.as_bytes()).text(capture)
            }
            Evicted::Valuation {
                instance,
                position,
                atom,
            } => encoder
                .u64(2)
                .option(instance.as_ref(), |encoder, instance| {
                    encoder.bytes(instance.as_bytes())
                })
                .u64(*position)
                .u64(*atom),
        };
    }
    encoder.finish()
}

fn encode_progress(request: &EvidenceRequest) -> Vec<u8> {
    let mut encoder = MemberEncoder::new()
        .option(request.decision_progress.as_ref(), |encoder, value| {
            encoder.u64(*value)
        })
        .u64(closure_code(request.decision_closure))
        .option(request.surrounding_progress.as_ref(), |encoder, value| {
            encoder.u64(*value)
        })
        .u64(closure_code(request.surrounding_closure))
        .u64(closure_code(Some(request.trigger_scope)))
        .u64(match request.trigger_evidence {
            TriggerEvidence::Admitted => 1,
            TriggerEvidence::Missing => 2,
            TriggerEvidence::Refused => 3,
        })
        .flag(request.execution == super::activation::ExecutionDisposition::Completed)
        .option(request.authoritative_origin.as_ref(), |encoder, value| {
            encoder.flag(*value)
        });
    encoder = encoder.option(request.completeness.as_ref(), |encoder, record| {
        let encoder = encode_ref(encoder, &record.reference);
        let encoder = encode_completeness(encoder, record.state);
        record
            .facts
            .iter()
            .fold(encoder.u64(record.facts.len() as u64), |encoder, fact| {
                encoder.text(fact)
            })
    });
    encoder.finish()
}

fn encode_retained(request: &EvidenceRequest, obligation: &ObligationKey) -> Vec<u8> {
    let mut encoder = MemberEncoder::new();
    if let Some(ledger) = &request.ledger {
        for (identity, entry) in ledger.instances_of(obligation) {
            encoder = encoder
                .bytes(identity.as_bytes())
                .bytes(entry.payload().as_bytes())
                .u64(entry.evaluations())
                .u64(entry.receipts().len() as u64);
            for receipt in entry.receipts() {
                encoder = encoder.bytes(receipt.as_bytes());
            }
            encoder = encoder.option(entry.fault(), |encoder, fault| {
                let encoder = match &fault.subject {
                    super::activation::FaultSubject::Guard => encoder.u64(1),
                    super::activation::FaultSubject::Anchor => encoder.u64(2),
                    super::activation::FaultSubject::Capture(name) => encoder.u64(3).text(name),
                };
                encoder.flag(fault.kind == super::activation::FaultKind::Missing)
            });
            for capture in entry.environment().entries() {
                encoder = encoder
                    .text(capture.name())
                    .bytes(capture.value().as_bytes())
                    .u64(capture.source().index as u64)
                    .text(capture.anchor());
            }
        }
    }
    encoder.finish()
}

fn encode_progress_entry(subject: &ResolvedSubject<'_>, request: &EvidenceRequest) -> Vec<u8> {
    let entry = request
        .progress_ledger
        .as_ref()
        .and_then(|ledger| ledger.entry(&subject.progress));
    MemberEncoder::new()
        .option(entry, |encoder, entry| {
            let encoder = encode_completeness(
                encoder
                    .u64(entry.watermark())
                    .u64(closure_code(Some(entry.scope())))
                    .u64(closure_code(Some(entry.execution()))),
                entry.completeness(),
            );
            entry.covered().fold(encoder, |encoder, position| {
                encoder.text(&position.identity).u64(position.stamp)
            })
        })
        .finish()
}

fn identity_of(
    subject: &ResolvedSubject<'_>,
    request: &EvidenceRequest,
    authenticated: &AuthenticatedBinding,
    obligation: &ObligationKey,
    meter: &Meter,
) -> Result<IdentityBuilder, super::identity::MemberFault> {
    let cover = match request.cover {
        ClockCover::Exact => MemberEncoder::new().u64(0).finish(),
        ClockCover::Counter { uncertainty } => {
            MemberEncoder::new().u64(1).u64(uncertainty).finish()
        }
    };
    let relation = match &request.relation {
        Relation::Original => MemberEncoder::new().u64(0),
        Relation::Superseding(prev) => MemberEncoder::new().u64(1).bytes(prev.as_bytes()),
        Relation::Invalidating(prev) => MemberEncoder::new().u64(2).bytes(prev.as_bytes()),
    }
    .finish();
    let trigger = MemberEncoder::new()
        .option(request.trigger.as_ref(), |encoder, trigger| {
            encoder.bytes(trigger.as_bytes())
        })
        .finish();
    Ok(IdentityBuilder::new()
        .clause(subject.selection.declaration)?
        .authenticated(authenticated)?
        .ceilings(&request.limits)
        .meter_ceiling(meter.limits().work_units)
        .member(IdentityMember::Cover, cover)
        .member(IdentityMember::Trace, encode_trace(subject, request))
        .member(IdentityMember::Progress, encode_progress(request))
        .member(IdentityMember::Trigger, trigger)
        .member(
            IdentityMember::RetainedInstances,
            encode_retained(request, obligation),
        )
        .member(
            IdentityMember::ProgressLedger,
            encode_progress_entry(subject, request),
        )
        .member(IdentityMember::Relation, relation)
        .member(IdentityMember::Deliveries, encode_deliveries(request))
        .member(IdentityMember::Origins, encode_origins(request))
        .member(
            IdentityMember::Anchor,
            MemberEncoder::new()
                .option(request.anchor.as_ref(), |encoder, anchor| {
                    encoder.text(anchor)
                })
                .finish(),
        )
        .member(IdentityMember::Evicted, encode_evicted(request))
        .member(
            IdentityMember::Correspondence,
            encode_ref(MemberEncoder::new(), &request.correspondence).finish(),
        )
        .member(
            IdentityMember::Occurrence,
            MemberEncoder::new().bytes(&subject.occurrence_key).finish(),
        ))
}

/// The byte lengths of the request's opaque byte members: their total and
/// the longest one.
fn request_bytes(request: &EvidenceRequest) -> (u64, u64) {
    let mut lengths: Vec<usize> = Vec::new();
    lengths.extend(
        request
            .trigger
            .iter()
            .map(|trigger| trigger.as_bytes().len()),
    );
    lengths.extend(request.correspondence.member_lengths());
    for position in &request.positions {
        lengths.extend(position.observation.member_lengths());
        lengths.extend(
            position
                .instance_valuations
                .keys()
                .map(|identity| identity.as_bytes().len()),
        );
    }
    for delivery in &request.deliveries {
        lengths.extend([
            delivery.identity.as_bytes().len(),
            delivery.receipt.as_bytes().len(),
            delivery.payload.as_bytes().len(),
        ]);
        lengths.extend(delivery.captures.iter().filter_map(|fact| match fact {
            CaptureFact::Value { value, .. } => Some(value.as_bytes().len()),
            CaptureFact::Missing
            | CaptureFact::Null
            | CaptureFact::WrongType
            | CaptureFact::Stale => None,
        }));
    }
    for origin in &request.origins {
        lengths.extend([
            origin.identity.as_bytes().len(),
            origin.payload.as_bytes().len(),
        ]);
    }
    totals(&lengths)
}

fn totals(lengths: &[usize]) -> (u64, u64) {
    let total = lengths.iter().map(|length| *length as u64).sum();
    let longest = lengths
        .iter()
        .copied()
        .max()
        .map_or(0, |length| length as u64);
    (total, longest)
}

fn result_bytes(outcome: &Outcome, facts: &Facts<'_>) -> (u64, u64) {
    let mut lengths: Vec<usize> = Vec::new();
    lengths.extend(
        facts
            .request
            .trigger
            .iter()
            .map(|trigger| trigger.as_bytes().len()),
    );
    lengths.extend(facts.request.correspondence.member_lengths());
    for support in &outcome.support {
        lengths.extend(support.observation.member_lengths());
    }
    for instance in &outcome.instances {
        lengths.push(instance.identity.as_bytes().len());
        lengths.extend(
            instance
                .receipts
                .iter()
                .map(|receipt| receipt.as_bytes().len()),
        );
        lengths.extend(
            instance
                .environment
                .entries()
                .iter()
                .map(|entry| entry.value().as_bytes().len()),
        );
    }
    totals(&lengths)
}

/// The `Incomplete` a counter stop hands the evaluator, which stops the run;
/// the stop itself is kept by the charger.
fn halted(limit: &LimitReached) -> Incomplete {
    Incomplete {
        limit_kind: LimitKind::WorkUnits,
        limit: limit.value,
        consumed: limit.value,
        next_charge: quire_exact::Integer::from(1_u64),
        charge_point: ChargePoint::TemporalVisit,
    }
}

/// Charges the evaluator's units on the call's counters and evaluator meter.
struct UnitCharger<'c, 'm> {
    call: &'c mut CallMeter<'m>,
    stop: Option<EvidenceStop>,
}

impl WindowCharge for UnitCharger<'_, '_> {
    fn charge(&mut self, unit: WindowUnit) -> Result<(), Incomplete> {
        let visit = match unit {
            WindowUnit::Step => Visit::Step,
            WindowUnit::Scan => Visit::Scan,
            WindowUnit::Lookup => Visit::Lookup,
            WindowUnit::SilentLookup => Visit::SilentLookup,
            WindowUnit::SilentStep => Visit::SilentStep,
            WindowUnit::SilentScan => Visit::SilentScan,
        };
        match self.call.visit(visit) {
            Ok(()) => Ok(()),
            Err(stop) => {
                let incomplete = match &stop {
                    EvidenceStop::Limit(limit) => halted(limit),
                    EvidenceStop::Meter { incomplete, .. } => incomplete.clone(),
                };
                self.stop = Some(stop);
                Err(incomplete)
            }
        }
    }

    fn aggregate_window_visit(&mut self, meter: &mut Meter) -> Result<(), Incomplete> {
        match self.call.visit_with_meter(Visit::Lookup, meter) {
            Ok(()) => Ok(()),
            Err(stop) => {
                let incomplete = match &stop {
                    EvidenceStop::Limit(limit) => halted(limit),
                    EvidenceStop::Meter { incomplete, .. } => incomplete.clone(),
                };
                self.stop = Some(stop);
                Err(incomplete)
            }
        }
    }
}

/// Reads each atom of the clause from the request's valuations: shared ones
/// by position, instance-bound ones from the assessed instance's own entry
/// (FR-524 "Instance-bound atoms").
struct EvidenceLetters<'a> {
    request: &'a EvidenceRequest,
    trace: &'a TimestampedTrace,
    bound: &'a BTreeSet<u64>,
    formula: &'a crate::temporal::facts::ClauseFormula,
    max_window_values: u64,
    assessed: Option<&'a SemanticTriggerIdentity>,
    evicted: Option<EvidenceIncomplete>,
    holders: BTreeMap<(u64, u64), Vec<SemanticTriggerIdentity>>,
    /// The `(position, atom)` valuations the request does not hold.
    absent: Vec<(u64, u64)>,
    /// Aggregate atoms whose windows fell short of `min_count`.
    insufficient: Vec<(u64, u64, InsufficientData)>,
}

/// The sparse aggregate samples the observation owner supplied for one
/// aggregate atom and one instance (FR-516).
struct EvidenceAggregateSamples<'a> {
    request: &'a EvidenceRequest,
    trace: &'a TimestampedTrace,
    atom_index: u64,
    instance_bound: bool,
    assessed: Option<&'a SemanticTriggerIdentity>,
    kind: AggregateKind,
}

impl<'a> EvidenceAggregateSamples<'a> {
    fn at(&self, position: TemporalPosition) -> Result<Option<&'a AggregateSample>, LetterStop> {
        let Some(supplied) = self
            .trace
            .supplied_index(position)
            .and_then(|index| self.request.positions.get(index))
        else {
            return Ok(None);
        };
        let samples = if self.instance_bound {
            self.assessed
                .and_then(|identity| supplied.instance_aggregate_samples.get(identity))
        } else {
            Some(&supplied.aggregate_samples)
        };
        let Some(sample) = samples.and_then(|samples| samples.get(&self.atom_index)) else {
            return Ok(None);
        };
        if !sample.is_taken_by(&self.kind) {
            return Err(LetterStop::Fault(InternalFault::new(
                "temporal.evidence",
                "aggregate-sample-kind-validated-on-admission",
            )));
        }
        Ok(Some(sample))
    }
}

impl WindowSource for EvidenceAggregateSamples<'_> {
    fn admits(
        &mut self,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<WindowRead<bool>, LetterStop> {
        Ok(match self.at(position)? {
            None => WindowRead::Missing,
            Some(AggregateSample::Excluded) => WindowRead::Defined(false),
            Some(AggregateSample::Undefined(cause)) => WindowRead::Undefined(cause.clone()),
            Some(
                AggregateSample::Value(_) | AggregateSample::Holds(_) | AggregateSample::Counted,
            ) => WindowRead::Defined(true),
        })
    }

    fn value(
        &mut self,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<WindowRead<quire_exact::Rational>, LetterStop> {
        Ok(match self.at(position)? {
            None => WindowRead::Missing,
            Some(AggregateSample::Value(value)) => WindowRead::Defined(value.clone()),
            Some(AggregateSample::Undefined(cause)) => WindowRead::Undefined(cause.clone()),
            Some(
                AggregateSample::Excluded | AggregateSample::Holds(_) | AggregateSample::Counted,
            ) => {
                return Err(LetterStop::Fault(InternalFault::new(
                    "temporal.evidence",
                    "aggregate-sample-kind-validated-on-admission",
                )))
            }
        })
    }

    fn predicate(
        &mut self,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<WindowRead<bool>, LetterStop> {
        Ok(match self.at(position)? {
            None => WindowRead::Missing,
            Some(AggregateSample::Holds(value)) => WindowRead::Defined(*value),
            Some(AggregateSample::Undefined(cause)) => WindowRead::Undefined(cause.clone()),
            Some(
                AggregateSample::Excluded | AggregateSample::Value(_) | AggregateSample::Counted,
            ) => {
                return Err(LetterStop::Fault(InternalFault::new(
                    "temporal.evidence",
                    "aggregate-sample-kind-validated-on-admission",
                )))
            }
        })
    }
}

impl EvidenceLetters<'_> {
    fn invariant(code: &'static str) -> LetterStop {
        LetterStop::Fault(InternalFault::new("temporal.evidence", code))
    }
}

impl TemporalLetters for EvidenceLetters<'_> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        match self.valuation(atom, position, meter)? {
            Valuation::True => Ok(AtomReading::True),
            Valuation::False => Ok(AtomReading::False),
            Valuation::Missing | Valuation::Undefined(_) => {
                Err(Self::invariant("atom-reads-a-definite-valuation"))
            }
        }
    }

    fn valuation(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<Valuation, LetterStop> {
        self.valuation_with_window_visits(atom, position, meter, &mut || Ok(()))
    }

    fn valuation_with_window_visits(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
        visit: &mut dyn FnMut() -> Result<(), LetterStop>,
    ) -> Result<Valuation, LetterStop> {
        let atom_index = self
            .formula
            .atom_index(atom.index())
            .ok_or_else(|| Self::invariant("letter-read-is-an-atom"))?;
        if let Some(aggregate) = self.formula.aggregate_atom(atom_index) {
            let mut source = EvidenceAggregateSamples {
                request: self.request,
                trace: self.trace,
                atom_index,
                instance_bound: self.bound.contains(&atom_index),
                assessed: self.assessed,
                kind: aggregate.kind().clone(),
            };
            let trace = self
                .trace
                .aggregate_window_trace(aggregate.window())
                .map_err(|_| Self::invariant("aggregate-window-matches-profile"))?;
            let value = aggregate.evaluate_valuation_with_visits(
                &trace,
                &mut source,
                self.max_window_values,
                position,
                meter,
                visit,
            )?;
            if value == Valuation::Missing {
                self.absent.push((position.index(), atom_index));
            }
            if let Valuation::Undefined(UndefinedCause::InsufficientData(data)) = &value {
                self.insufficient
                    .push((position.index(), atom_index, *data));
            }
            return Ok(value);
        }
        let Some(supplied) = self
            .trace
            .supplied_index(position)
            .and_then(|index| self.request.positions.get(index))
        else {
            // A silent sample is no observation: every atom reads false
            // there (FR-504).
            return if position.index() >= self.trace.observed() as u64 {
                Ok(Valuation::False)
            } else {
                Err(Self::invariant("position-is-supplied"))
            };
        };
        let hit = self.request.evicted.iter().find(|record| {
            matches!(record, Evicted::Valuation { instance, position, atom }
                if *atom == atom_index
                    && *position == supplied.position
                    && instance.as_ref().is_none_or(|bound| Some(bound) == self.assessed))
        });
        if let Some(Evicted::Valuation {
            instance,
            position,
            atom,
        }) = hit
        {
            self.evicted = Some(EvidenceIncomplete::EvictedValuation {
                instance: instance.clone(),
                position: *position,
                atom: *atom,
            });
            return Err(Self::invariant("valuation-is-retained"));
        }
        if !self.bound.contains(&atom_index) {
            return Ok(match supplied.valuations.get(&atom_index) {
                Some(valuation) => valuation.clone(),
                None => {
                    self.absent.push((position.index(), atom_index));
                    Valuation::Missing
                }
            });
        }
        let own = self
            .assessed
            .and_then(|assessed| supplied.instance_valuations.get(assessed))
            .and_then(|valuations| valuations.get(&atom_index));
        if let Some(valuation) = own {
            return Ok(valuation.clone());
        }
        let others = supplied
            .instance_valuations
            .iter()
            .filter(|(identity, valuations)| {
                Some(*identity) != self.assessed && valuations.contains_key(&atom_index)
            })
            .map(|(identity, _)| identity.clone())
            .collect();
        self.absent.push((position.index(), atom_index));
        self.holders.insert((position.index(), atom_index), others);
        Ok(Valuation::Missing)
    }

    fn activation(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        // The anchored run never asks: the instance's guard was established
        // when it was created.
        Ok(AtomReading::True)
    }

    fn captures(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<CaptureReading, LetterStop> {
        // The instance's captures were established once, by `activate`.
        Ok(CaptureReading::Fixed)
    }
}

fn watermark_of(request: &EvidenceRequest) -> Watermark {
    request
        .decision_progress
        .map_or(Watermark::HalfOpen { end: 0 }, Watermark::Inclusive)
}

/// Admit the trace. A withdrawn fact is read as nothing asserted: an open
/// scope, an incomplete record, a history cutoff and no watermark, which
/// never makes a verdict more definite than the evidence supports.
fn admit_trace(
    facts: &Facts<'_>,
    meter: &mut Meter,
) -> Result<Result<TimestampedTrace, Box<Outcome>>, CallFailure> {
    let request = facts.request;
    let clause_facts = facts.subject.facts;
    let completeness = request
        .completeness
        .as_ref()
        .map_or(ProgressCompleteness::Incomplete, |record| record.state);
    let scope = request.decision_closure.unwrap_or(ProgressClosure::Open);
    let execution = request.surrounding_closure.unwrap_or(ProgressClosure::Open);
    let lower_boundary = if request.authoritative_origin == Some(true) {
        LowerBoundary::AuthoritativeOrigin
    } else {
        LowerBoundary::HistoryCutoff
    };
    if matches!(
        clause_facts.profile(),
        TemporalProfile::EventPositionFalseExtension | TemporalProfile::FixedSampleFalseExtension
    ) {
        // Position `i` of the listing has coordinate `i` (FR-504). A withdrawn
        // watermark asserts nothing beyond the last observation.
        let last = u64::try_from(request.positions.len().saturating_sub(1))
            .map_err(|_| fault("positions-in-u64"))?;
        let progress = SampleProgress {
            watermark: request.decision_progress.unwrap_or(last),
            completeness,
            scope,
            execution,
            lower_boundary,
        };
        let observations = request
            .positions
            .iter()
            .map(|position| position.observation.identity.clone())
            .collect();
        return match TimestampedTrace::admit_progressed(clause_facts, observations, progress, meter)
        {
            Err(_) => Err(fault("trace-admits")),
            Ok(Ok(trace)) => Ok(Ok(trace)),
            Ok(Err(ProgressedRefusal::Incomplete(incomplete))) => {
                Ok(Err(Box::new(Outcome::stopped(EvidenceIncomplete::Meter {
                    incomplete,
                    instance: None,
                }))))
            }
            Ok(Err(refused)) => Err(refusal(
                "progress-inconsistent",
                "positions",
                &[("refusal", format!("{refused:?}"))],
            )),
        };
    }
    let unit = match &facts.authenticated.clock {
        qsl_package::ClockConfiguration::TimestampedEvent { timestamp_unit } => {
            timestamp_unit.to_string()
        }
        qsl_package::ClockConfiguration::EventPosition { .. }
        | qsl_package::ClockConfiguration::FixedSample { .. } => String::new(),
    };
    TimestampedTrace::admit(
        clause_facts.name(),
        WindowClock {
            unit,
            cover: request.cover,
        },
        request
            .positions
            .iter()
            .map(|position| TimestampedObservation {
                stamp: position.position,
                order_key: position.order_key.clone(),
                observation: position.observation.identity.clone(),
            })
            .collect(),
        WindowProgress {
            watermark: watermark_of(request),
            completeness,
            scope,
            execution,
            lower_boundary,
        },
    )
    .map(Ok)
    .map_err(|_| fault("trace-admits"))
}

fn no_instance(scope: TriggerScope, request: &EvidenceRequest) -> Outcome {
    disposition_outcome(scope_disposition(scope, request.execution))
}

fn disposition_outcome(disposition: Disposition) -> Outcome {
    match disposition {
        Disposition::Inactive => Outcome::of(ActivationState::Inactive).with_non_value(
            NonValueKind::Inactive,
            "no-trigger-admitted",
            None,
        ),
        Disposition::Unknown { kind, .. } => Outcome::of(ActivationState::Unknown).with_non_value(
            NonValueKind::ActivationUnknown,
            match kind {
                UnknownKind::Open => "trigger-scope-open",
                UnknownKind::Missing => "trigger-evidence-missing",
                UnknownKind::Refused => "trigger-evidence-refused",
            },
            None,
        ),
        Disposition::Active => Outcome::of(ActivationState::Active),
    }
}

fn assess(
    facts: &Facts<'_>,
    shape: &ClauseShape,
    meter: &mut Meter,
) -> Result<Outcome, CallFailure> {
    let request = facts.request;
    let scope = TriggerScope {
        closure: request.trigger_scope,
        evidence: request.trigger_evidence,
        record: match request.completeness {
            Some(ref record) if matches!(record.state, ProgressCompleteness::Complete { .. }) => {
                TriggerRecord::Complete
            }
            Some(_) | None => TriggerRecord::Incomplete,
        },
    };
    let anchor = request.anchor.as_ref().and_then(|anchor| {
        request
            .positions
            .iter()
            .any(|position| position.observation.identity == *anchor)
            .then_some(anchor)
    });
    let Some(anchor) = anchor else {
        return Ok(Outcome::of(ActivationState::Unavailable).with_non_value(
            NonValueKind::Missing,
            "anchor-withdrawn",
            Some("anchor".to_owned()),
        ));
    };
    let input = ActivationInput {
        scope,
        deliveries: request.deliveries.clone(),
        origins: request.origins.clone(),
        anchor: anchor.clone(),
        evicted: request.evicted.clone(),
        ledger: request.ledger.clone(),
        execution: request.execution,
    };
    if let Some(contradiction) = input.contradiction(&facts.obligation) {
        return Err(refusal(
            "delivery-contradiction",
            "deliveries",
            &[(
                "contradicted_identity",
                format!("{:?}", contradiction.contradicted_identity.as_bytes()),
            )],
        ));
    }
    if let Some(surplus) = input.surplus(shape) {
        return Err(refusal(
            "surplus-capture",
            "captures",
            &[
                (
                    "surplus_capture_identity",
                    format!("{:?}", surplus.surplus_capture_identity.as_bytes()),
                ),
                (
                    "surplus_capture_index",
                    surplus.surplus_capture_index.to_string(),
                ),
            ],
        ));
    }
    let listed: BTreeSet<&SemanticTriggerIdentity> = request
        .deliveries
        .iter()
        .map(|delivery| &delivery.identity)
        .chain(request.origins.iter().map(|origin| &origin.identity))
        .collect();
    let retained = |identity: &SemanticTriggerIdentity| {
        request
            .ledger
            .as_ref()
            .and_then(|ledger| ledger.instance(&facts.obligation, identity))
    };
    match &request.trigger {
        None if !listed.is_empty() => return Err(refusal("trigger-required", "trigger", &[])),
        Some(trigger) if !listed.contains(trigger) && retained(trigger).is_none() => {
            return Err(refusal("trigger-unknown", "trigger", &[]));
        }
        Some(_) | None => {}
    }
    if let Some(limit) = facts.progress_stop {
        return Ok(Outcome::stopped(EvidenceIncomplete::CoveredPositions(
            limit,
        )));
    }
    let trace = match admit_trace(facts, &mut *meter)? {
        Ok(trace) => trace,
        Err(stopped) => return Ok(*stopped),
    };
    let canonical_anchor = (0..trace.len())
        .find(|index| {
            trace
                .supplied_index(TemporalPosition::new(*index as u64))
                .and_then(|supplied| request.positions.get(supplied))
                .is_some_and(|position| position.observation.identity == *anchor)
        })
        .ok_or_else(|| fault("anchor-names-a-position"))?;
    let activation_position = match shape.activation() {
        ClauseActivation::Origin => 0,
        ClauseActivation::Each { .. } => canonical_anchor,
    };
    let anchor_tick = trace
        .positions()
        .get(activation_position)
        .map(|position| position.stamp)
        .ok_or_else(|| fault("activation-position-is-admitted"))?;
    match facts.subject.facts.formula().work() {
        Ok(_) | Err(ReachFault::Overflow) => {}
        Err(unreadable) => {
            return Err(refusal(
                "clause-unreadable",
                "clause",
                &[("reason", unreadable.to_string())],
            ));
        }
    }
    let mut call = CallMeter::new(request.limits, meter);
    let prepared = Prepared {
        scope,
        input,
        trace,
        activation_position,
        anchor_tick,
    };
    let mut outcome = run_instance(facts, shape, &prepared, &mut call)?;
    outcome.usage = call.usage();
    Ok(outcome)
}

/// What the admission established before the first charge.
struct Prepared {
    scope: TriggerScope,
    input: ActivationInput,
    trace: TimestampedTrace,
    activation_position: usize,
    anchor_tick: u64,
}

fn run_instance(
    facts: &Facts<'_>,
    shape: &ClauseShape,
    prepared: &Prepared,
    call: &mut CallMeter<'_>,
) -> Result<Outcome, CallFailure> {
    let request = facts.request;
    let subject = facts.subject;
    let Prepared {
        scope,
        input,
        trace,
        activation_position,
        anchor_tick,
    } = prepared;
    if let Some(trigger) = &request.trigger {
        call.assess(trigger.as_bytes());
    }
    let (bytes, longest) = request_bytes(request);
    if let Err(limit) = call.charge(&[
        (EvidenceCounter::RequestBytes, bytes),
        (EvidenceCounter::MemberBytes, longest),
    ]) {
        return Ok(Outcome::stopped(EvidenceIncomplete::Limit(limit)));
    }
    let uncertainty = match request.cover {
        ClockCover::Exact => None,
        ClockCover::Counter { uncertainty } => Some(uncertainty),
    };
    match call.admit_work(subject.facts.formula().work(), uncertainty, [*anchor_tick]) {
        Ok(_) => {}
        Err(ClauseStop::Limit(limit)) => {
            return Ok(Outcome::stopped(EvidenceIncomplete::Limit(limit)));
        }
        Err(ClauseStop::Unreadable(_)) => return Err(fault("clause-reach-is-finite")),
    }
    let Some(lineage) = facts.lineage else {
        return Ok(Outcome::stopped(EvidenceIncomplete::Limit(LimitReached {
            counter: EvidenceCounter::Lineage,
            limit: EvidenceCounter::Lineage.limit_kind(),
            value: request.limits.lineage,
            setting: EvidenceCounter::Lineage.setting(),
            instance: request
                .trigger
                .as_ref()
                .map(|trigger| trigger.as_bytes().into()),
        })));
    };
    if let Err(limit) = call.charge(&[(EvidenceCounter::Lineage, lineage)]) {
        return Ok(Outcome::stopped(EvidenceIncomplete::Limit(limit)));
    }
    let activation = match activate(shape, &facts.obligation, input.clone(), call) {
        Ok(activation) => activation,
        Err(ActivationStop::Contradiction(_) | ActivationStop::Surplus(_)) => {
            return Err(fault("refusals-precede-charges"));
        }
        Err(ActivationStop::Stop(stop)) => return Ok(Outcome::stopped(incomplete_of(stop))),
    };
    let mut instances = activation.instances.clone();
    let mut outcome = match &request.trigger {
        None => no_instance(*scope, request),
        Some(trigger) => {
            let mut resolved = instances
                .iter()
                .find(|instance| &instance.identity == trigger)
                .cloned();
            if resolved.is_none() && !activation.guard_false.contains(trigger) {
                if let Some(entry) = activation.ledger.instance(&facts.obligation, trigger) {
                    match retained_instance(trigger, entry, input, call) {
                        Ok(instance) => {
                            instances.push(instance.clone());
                            resolved = Some(instance);
                        }
                        Err(stop) => return Ok(Outcome::stopped(incomplete_of(stop))),
                    }
                }
            }
            match resolved {
                None => no_instance(*scope, request),
                Some(instance) => match &instance.state {
                    InstanceState::Unavailable(instance_fault) => {
                        Outcome::of(ActivationState::Unavailable).with_non_value(
                            match instance_fault.kind {
                                FaultKind::Missing => NonValueKind::Missing,
                                FaultKind::Refused => NonValueKind::Refused,
                            },
                            "activation-fault",
                            Some(match &instance_fault.subject {
                                FaultSubject::Guard => "guard".to_owned(),
                                FaultSubject::Anchor => "anchor".to_owned(),
                                FaultSubject::Capture(name) => name.clone(),
                            }),
                        )
                    }
                    InstanceState::Active => {
                        evaluate(facts, trace, *activation_position, trigger, call)?
                    }
                },
            }
        }
    };
    outcome.instances = instances;
    if outcome.incomplete.is_none() {
        outcome.ledger = Some(activation.ledger);
        let (bytes, longest) = result_bytes(&outcome, facts);
        let charged = call.charge(&[
            (EvidenceCounter::Support, outcome.support.len() as u64),
            (EvidenceCounter::ResultBytes, bytes),
            (EvidenceCounter::MemberBytes, longest),
        ]);
        if let Err(limit) = charged {
            outcome = Outcome::stopped(EvidenceIncomplete::Limit(limit));
        }
    }
    Ok(outcome)
}

/// Assess the instance `trigger`, whose activation is established, under the
/// finite-window evaluator (FR-501) at its one activation position.
fn evaluate(
    facts: &Facts<'_>,
    trace: &TimestampedTrace,
    activation_position: usize,
    trigger: &SemanticTriggerIdentity,
    call: &mut CallMeter<'_>,
) -> Result<Outcome, CallFailure> {
    let request = facts.request;
    let clause_facts = facts.subject.facts;
    let mut letters = EvidenceLetters {
        request,
        trace,
        bound: facts.subject.instance_bound,
        formula: facts.subject.facts.formula(),
        max_window_values: facts.subject.max_window_values,
        assessed: Some(trigger),
        evicted: None,
        holders: BTreeMap::new(),
        absent: Vec::new(),
        insufficient: Vec::new(),
    };
    let mut scratch = Meter::new(*call.meter().limits());
    let mut charger = UnitCharger { call, stop: None };
    let evaluation = evaluate_finite_window_at(
        clause_facts,
        trace,
        activation_position,
        &mut letters,
        &mut charger,
        &mut scratch,
    );
    let stop = charger.stop.take();
    let evaluation = match evaluation {
        Ok(evaluation) => evaluation,
        Err(refused) => {
            if let Some(evicted) = letters.evicted.take() {
                // One `evicted` visit for the hit (FR-524); the result is an
                // `Incomplete`, whichever charge stops it.
                return Ok(Outcome::stopped(match call.visit(Visit::Evicted) {
                    Ok(()) => evicted,
                    Err(stop) => incomplete_of(stop),
                }));
            }
            return match refused {
                TemporalRefusal::Fault(internal) => Err(CallFailure::Fault(internal)),
                // The evidence letters read no S6a condition, so none can
                // be refused.
                TemporalRefusal::ConditionRefused { .. } => Err(CallFailure::Fault(
                    InternalFault::new("temporal.evidence", "evidence-letters-refuse-no-condition"),
                )),
                // A refusal of the evaluation itself (for example two
                // positions of one stamp with no usable order) is a result
                // of a call that ran, not a refusal before any charge.
                other => {
                    let (kind, detail) = refusal_kind(&other);
                    let mut outcome = Outcome::of(ActivationState::Active).with_non_value(
                        NonValueKind::Refused,
                        "temporal-refusal",
                        Some(kind.to_owned()),
                    );
                    if let Some(non_value) = outcome.non_value.as_mut() {
                        non_value.detail = detail;
                    }
                    Ok(outcome)
                }
            };
        }
    };
    let mut outcome = Outcome::of(ActivationState::Active);
    outcome.settlement = Some(match evaluation.basis {
        Basis::ClosedScope => Settlement::ClosedScope,
        Basis::DecisiveWitness => Settlement::DecisiveWitness,
        Basis::DecisiveCounterexample => Settlement::DecisiveCounterexample,
        Basis::Unsettled => Settlement::Unsettled,
        Basis::Unavailable => Settlement::Unavailable,
    });
    match evaluation.verdict {
        WindowVerdict::Holds => outcome.truth = Some(Truth::True),
        WindowVerdict::Violated { .. } => outcome.truth = Some(Truth::False),
        WindowVerdict::Pending { .. } | WindowVerdict::PendingNextEvent { .. } => {
            outcome.truth = Some(Truth::Pending);
        }
        WindowVerdict::Indeterminate { reason, .. } => {
            let (indeterminate, kind, code) = match reason {
                IndeterminateReason::MissingValuation { position, atom } => {
                    let atom = facts
                        .subject
                        .facts
                        .formula()
                        .atom_index(atom.index())
                        .ok_or_else(|| fault("indeterminate-atom-is-an-atom"))?;
                    let bound = facts.subject.instance_bound.contains(&atom);
                    (
                        Indeterminate::MissingValuation {
                            position: position.index(),
                            atom,
                            instance: bound.then(|| trigger.clone()),
                            also_held_by: letters
                                .holders
                                .get(&(position.index(), atom))
                                .cloned()
                                .unwrap_or_default(),
                        },
                        NonValueKind::Missing,
                        "missing-valuation",
                    )
                }
                IndeterminateReason::ConflictingValuation { position, atom } => (
                    Indeterminate::ConflictingValuation {
                        position: position.index(),
                        atom: facts
                            .subject
                            .facts
                            .formula()
                            .atom_index(atom.index())
                            .ok_or_else(|| fault("indeterminate-atom-is-an-atom"))?,
                    },
                    NonValueKind::Refused,
                    "conflicting-valuation",
                ),
                IndeterminateReason::IncompleteInput => (
                    Indeterminate::IncompleteInput,
                    NonValueKind::Missing,
                    "incomplete-input",
                ),
                IndeterminateReason::MissingHistory => (
                    Indeterminate::MissingHistory,
                    NonValueKind::Missing,
                    "missing-history",
                ),
                IndeterminateReason::StraddlesBound => (
                    Indeterminate::StraddlesBound,
                    NonValueKind::Missing,
                    "straddles-bound",
                ),
            };
            outcome.indeterminate = Some(indeterminate);
            outcome = outcome.with_non_value(kind, code, None);
        }
        WindowVerdict::Undefined(undefined) => match undefined.cause {
            UndefinedCause::InsufficientData(data) => {
                let atom = letters
                    .insufficient
                    .iter()
                    .find_map(|(position, atom, recorded)| {
                        (*position == undefined.at.index() && *recorded == data).then_some(*atom)
                    })
                    .ok_or_else(|| fault("insufficient-data-has-aggregate-atom"))?;
                let dimension = format!("valuation:{atom}@{}", undefined.at.index());
                outcome = outcome.with_non_value(
                    NonValueKind::Missing,
                    "insufficient-data",
                    Some(dimension),
                );
                if let Some(non_value) = outcome.non_value.as_mut() {
                    non_value.detail = vec![
                        ("count".to_owned(), data.count.to_string()),
                        ("min_count".to_owned(), data.min_count.to_string()),
                    ];
                }
            }
            cause => {
                outcome = outcome.with_non_value(NonValueKind::Refused, "undefined-atom", None);
                if let Some(non_value) = outcome.non_value.as_mut() {
                    non_value.detail = vec![("cause".to_owned(), format!("{cause:?}"))];
                }
            }
        },
        WindowVerdict::WindowLimitReached(limit) => {
            outcome.activation = ActivationState::Unavailable;
            outcome.incomplete = Some(EvidenceIncomplete::WindowLimit(limit));
            outcome.settlement = None;
        }
        WindowVerdict::Incomplete(incomplete) => {
            outcome.activation = ActivationState::Unavailable;
            outcome.incomplete = Some(match stop {
                Some(stop) => incomplete_of(stop),
                None => EvidenceIncomplete::Meter {
                    incomplete,
                    instance: Some(trigger.as_bytes().into()),
                },
            });
            outcome.settlement = None;
        }
        WindowVerdict::ActivationIncomplete { .. } => {
            return Err(fault("anchored-activation-has-no-guard"));
        }
    }
    for pair in &evaluation.support {
        // A pair at a silent sample names no observed fact.
        let Some(observation) = trace
            .supplied_index(pair.position)
            .and_then(|index| request.positions.get(index))
            .map(|position| position.observation.clone())
        else {
            continue;
        };
        outcome.support.push(SupportRef {
            observation,
            atom: facts
                .subject
                .facts
                .formula()
                .atom_index(pair.atom.index())
                .ok_or_else(|| fault("support-atom-is-an-atom"))?,
        });
    }
    if outcome.truth.is_some() {
        let absent = letters
            .absent
            .first()
            .map(|(position, atom)| format!("valuation:{atom}@{position}"));
        let withdrawn = if request.decision_closure.is_none() {
            Some("decision_closure".to_owned())
        } else if request.completeness.is_none() {
            Some("completeness".to_owned())
        } else {
            absent
        };
        if let Some(fact) = withdrawn {
            outcome = outcome.with_non_value(NonValueKind::Missing, "fact-withdrawn", Some(fact));
        }
    }
    Ok(outcome)
}

/// Add the members every result carries and record the result as the latest
/// of its obligation instance (FR-526).
fn conclude(facts: &Facts<'_>, outcome: Outcome, _meter: &Meter) -> EvidenceResult {
    let request = facts.request;
    let stopped = outcome.incomplete.is_some();
    let base = request.ledger.clone().unwrap_or_default();
    let ledger = if stopped {
        base
    } else {
        outcome.ledger.unwrap_or(base).with_result(
            &facts.obligation,
            request.trigger.as_ref(),
            ResultEntry::new(
                facts.result_identity.clone(),
                facts.input_identity.clone(),
                encode_ref(MemberEncoder::new(), &request.correspondence)
                    .finish()
                    .into_boxed_slice(),
                facts.lineage.unwrap_or_default(),
            ),
        )
    };
    EvidenceResult {
        activation: outcome.activation,
        execution: request.execution,
        truth: outcome.truth,
        non_value: outcome.non_value,
        settlement: outcome.settlement,
        indeterminate: outcome.indeterminate,
        incomplete: outcome.incomplete,
        support: outcome.support,
        trigger_evidence: request.trigger_evidence,
        trigger_scope: request.trigger_scope,
        decision_progress: request.decision_progress,
        decision_closure: request.decision_closure,
        surrounding_progress: request.surrounding_progress,
        surrounding_closure: request.surrounding_closure,
        completeness: request.completeness.clone(),
        correspondence: request.correspondence.clone(),
        trigger: request.trigger.clone(),
        usage: outcome.usage,
        relation: request.relation.clone(),
        lineage: facts.lineage.unwrap_or_default(),
        input_identity: facts.input_identity.clone(),
        result_identity: facts.result_identity.clone(),
        ledger,
        progress_ledger: if stopped {
            request.progress_ledger.clone()
        } else {
            facts.progress_ledger.clone()
        },
        authenticated: facts.authenticated.clone(),
        instances: outcome.instances,
    }
}

/// The stable kebab-case kind of an evaluation refusal and its fields as
/// ordered name/value pairs (FR-516). `Fault` is not a refusal result.
fn refusal_kind(refused: &TemporalRefusal) -> (&'static str, Vec<(String, String)>) {
    use crate::temporal::position::PositionRefusal;
    let pair = |name: &str, value: String| vec![(name.to_owned(), value)];
    let position = |at: &TemporalPosition| pair("position", at.index().to_string());
    let profile = |profile: &qsl_semantics::check::TemporalProfile| {
        pair("profile", profile.identity().to_string())
    };
    match refused {
        TemporalRefusal::EmptyTrace => ("empty-trace", Vec::new()),
        TemporalRefusal::TupleArity { expected, found } => (
            "tuple-arity",
            vec![
                ("expected".to_owned(), expected.to_string()),
                ("found".to_owned(), found.to_string()),
            ],
        ),
        TemporalRefusal::AlignSkipNotLockstep => ("align-skip-not-lockstep", Vec::new()),
        TemporalRefusal::NotProjectionAligned => ("not-projection-aligned", Vec::new()),
        TemporalRefusal::EmptyLoop => ("empty-loop", Vec::new()),
        TemporalRefusal::TraceTooLong => ("trace-too-long", Vec::new()),
        TemporalRefusal::LassoUnderBoundedProfile { profile: of } => {
            ("lasso-under-bounded-profile", profile(of))
        }
        TemporalRefusal::ProfileNotEvaluated { profile: of } => {
            ("profile-not-evaluated", profile(of))
        }
        TemporalRefusal::TraceCarriesNoStamps { profile: of } => {
            ("trace-carries-no-stamps", profile(of))
        }
        TemporalRefusal::NotFiniteWindowProfile { profile: of } => {
            ("not-finite-window-profile", profile(of))
        }
        TemporalRefusal::UnorderedTie { position: at } => ("unordered-tie", position(at)),
        TemporalRefusal::NotTimedProfile { profile: of } => ("not-timed-profile", profile(of)),
        TemporalRefusal::NonPositiveLoopDelay => ("non-positive-loop-delay", Vec::new()),
        TemporalRefusal::LoopReentersBeforeLastStamp => {
            ("loop-reenters-before-last-stamp", Vec::new())
        }
        TemporalRefusal::NegativeStamp { position: at } => ("negative-stamp", position(at)),
        TemporalRefusal::DecreasingStamp { position: at } => ("decreasing-stamp", position(at)),
        TemporalRefusal::Position(inner) => (
            "position-refusal",
            pair(
                "kind",
                match inner {
                    PositionRefusal::NotDecimal => "not-decimal",
                    PositionRefusal::LeadingZero => "leading-zero",
                    PositionRefusal::Overflow => "overflow",
                    PositionRefusal::OutsideTrace { .. } => "outside-trace",
                }
                .to_owned(),
            ),
        ),
        TemporalRefusal::Window(inner) => (
            "window-refusal",
            pair(
                "kind",
                match inner {
                    super::super::WindowRefusal::OutsideTrace { .. } => "outside-trace",
                    super::super::WindowRefusal::WindowKindMismatch => "window-kind-mismatch",
                    super::super::WindowRefusal::TimestampsDecrease => "timestamps-decrease",
                }
                .to_owned(),
            ),
        ),
        TemporalRefusal::Fault(_) | TemporalRefusal::ConditionRefused { .. } => {
            ("fault", Vec::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::refusal_kind;
    use crate::temporal::outcome::TemporalRefusal;
    use crate::temporal::position::{PositionRefusal, TemporalPosition};
    use crate::temporal::window::WindowRefusal;
    use qsl_semantics::check::TemporalProfile;

    /// FR-516: every evaluation refusal has one stable kebab-case kind.
    #[ix_trace_rs::trace("FR-516-AC-42", "TC-1311")]
    #[test]
    fn every_temporal_refusal_has_its_exact_kebab_case_kind() {
        let at = TemporalPosition::new(1);
        let profile = TemporalProfile::TimestampedEventFiniteWindow;
        let cases = [
            (TemporalRefusal::EmptyTrace, "empty-trace"),
            (
                TemporalRefusal::TupleArity {
                    expected: 2,
                    found: 1,
                },
                "tuple-arity",
            ),
            (
                TemporalRefusal::AlignSkipNotLockstep,
                "align-skip-not-lockstep",
            ),
            (
                TemporalRefusal::NotProjectionAligned,
                "not-projection-aligned",
            ),
            (TemporalRefusal::EmptyLoop, "empty-loop"),
            (TemporalRefusal::TraceTooLong, "trace-too-long"),
            (
                TemporalRefusal::LassoUnderBoundedProfile { profile },
                "lasso-under-bounded-profile",
            ),
            (
                TemporalRefusal::ProfileNotEvaluated { profile },
                "profile-not-evaluated",
            ),
            (
                TemporalRefusal::TraceCarriesNoStamps { profile },
                "trace-carries-no-stamps",
            ),
            (
                TemporalRefusal::NotFiniteWindowProfile { profile },
                "not-finite-window-profile",
            ),
            (
                TemporalRefusal::UnorderedTie { position: at },
                "unordered-tie",
            ),
            (
                TemporalRefusal::NotTimedProfile { profile },
                "not-timed-profile",
            ),
            (
                TemporalRefusal::NonPositiveLoopDelay,
                "non-positive-loop-delay",
            ),
            (
                TemporalRefusal::LoopReentersBeforeLastStamp,
                "loop-reenters-before-last-stamp",
            ),
            (
                TemporalRefusal::NegativeStamp { position: at },
                "negative-stamp",
            ),
            (
                TemporalRefusal::DecreasingStamp { position: at },
                "decreasing-stamp",
            ),
            (
                TemporalRefusal::Position(PositionRefusal::Overflow),
                "position-refusal",
            ),
            (
                TemporalRefusal::Window(WindowRefusal::WindowKindMismatch),
                "window-refusal",
            ),
        ];
        assert_eq!(cases.len(), 18);
        for (refused, expected) in &cases {
            assert_eq!(refusal_kind(refused).0, *expected);
        }
        // The inner kinds of the two wrapping refusals.
        assert_eq!(
            refusal_kind(&TemporalRefusal::Position(PositionRefusal::Overflow)).1,
            [("kind".to_owned(), "overflow".to_owned())]
        );
        assert_eq!(
            refusal_kind(&TemporalRefusal::Window(WindowRefusal::TimestampsDecrease)).1,
            [("kind".to_owned(), "timestamps-decrease".to_owned())]
        );
    }
}
