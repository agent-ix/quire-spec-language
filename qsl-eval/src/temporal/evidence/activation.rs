// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-524: one temporal instance per semantic trigger or execution origin,
//! with captures that are evaluated once and kept, a fault that names the
//! input it found, and one of four dispositions for a scope that creates no
//! instance. No output holds a truth value.
//!
//! [`activate`] resolves the ordered `deliveries` (or `origins`) against the
//! caller's [`EvidenceLedger`]. The resolution that can refuse
//! ([`ActivationInput::contradiction`]) charges nothing, so a refused call
//! has an empty charge log; the establishment that follows charges each
//! delivery, capture, evicted record and instance on the call's counters.

use std::collections::BTreeMap;

use qsl_semantics::check::Activation as ClauseActivation;

use crate::temporal::facts::ClauseFacts;
use quire_exact::ValueType;

use super::counters::{CallMeter, EvidenceStop, Visit};
use super::ledger::{
    CaptureEntry, CaptureEnvironment, EvidenceLedger, ObligationKey, RetainedInstance,
    SourceBinding,
};
use super::limits::EvidenceCounter;
use super::trigger::{
    CaptureFact, Evicted, ExecutionOrigin, Receipt, SemanticTriggerIdentity, TriggerDelivery,
    TriggerEvidence, TriggerRecord, TriggerScope,
};
use crate::temporal::ledger::ProgressClosure;

/// The facts of a checked clause that activation reads (FR-515 `Activation`
/// and `captures`): how it activates and its captures in authored order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseShape {
    name: Box<str>,
    activation: ClauseActivation,
    captures: Vec<(Box<str>, ValueType)>,
}

impl ClauseShape {
    /// The shape of `clause`.
    pub fn of(clause: &ClauseFacts) -> Self {
        Self {
            name: clause.name().into(),
            activation: clause.activation(),
            captures: clause
                .captures()
                .map(|(name, declared)| (name.into(), declared.clone()))
                .collect(),
        }
    }

    /// The clause's declared name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// How the clause activates.
    pub const fn activation(&self) -> ClauseActivation {
        self.activation
    }

    /// The captures' names, in authored order.
    pub fn capture_names(&self) -> impl Iterator<Item = &str> {
        self.captures.iter().map(|(name, _)| &**name)
    }

    fn guarded(&self) -> bool {
        matches!(self.activation, ClauseActivation::Each { guarded: true })
    }
}

/// The assessment-execution disposition of the surrounding execution
/// (FR-516 `execution`).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ExecutionDisposition {
    /// The execution completed.
    Completed,
    /// The execution failed.
    Failed,
}

/// What [`activate`] reads (FR-524 "Inputs").
#[derive(Clone, Debug)]
pub struct ActivationInput {
    /// The trigger scope.
    pub scope: TriggerScope,
    /// The ordered deliveries of a clause that activates `on each`.
    pub deliveries: Vec<TriggerDelivery>,
    /// The ordered origins of a clause that activates `on origin`.
    pub origins: Vec<ExecutionOrigin>,
    /// The identity of the evaluation anchor.
    pub anchor: String,
    /// The retained records the caller reports as gone.
    pub evicted: Vec<Evicted>,
    /// The caller's ledger of retained instances.
    pub ledger: Option<EvidenceLedger>,
    /// The surrounding execution's disposition, which an `Unknown`
    /// disposition carries.
    pub execution: ExecutionDisposition,
}

/// Whether the trigger record that an `Unknown` rests on is complete.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Completeness {
    /// The record is complete.
    Complete,
    /// The record is incomplete.
    Incomplete,
}

/// Which of the three `Unknown` dispositions a scope reports.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UnknownKind {
    /// The scope is open, or closed over an incomplete record.
    Open,
    /// The trigger evidence is missing.
    Missing,
    /// The trigger evidence was refused.
    Refused,
}

/// The activation disposition (FR-524 "Outputs"). None of the three is a
/// truth value, and none evaluates a position.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Disposition {
    /// A closed, complete scope with admitted evidence created no instance.
    Inactive,
    /// The scope created no instance and cannot say it never will.
    Unknown {
        /// Which `Unknown`.
        kind: UnknownKind,
        /// The completeness of the trigger record.
        completeness: Completeness,
        /// The assessment-execution disposition.
        execution: ExecutionDisposition,
    },
    /// At least one instance exists.
    Active,
}

/// The disposition of a trigger scope that creates no instance (FR-524
/// "Dispositions of a scope with no instance").
pub fn scope_disposition(scope: TriggerScope, execution: ExecutionDisposition) -> Disposition {
    let completeness = match scope.record {
        TriggerRecord::Complete => Completeness::Complete,
        TriggerRecord::Incomplete => Completeness::Incomplete,
    };
    let unknown = |kind| Disposition::Unknown {
        kind,
        completeness,
        execution,
    };
    match (scope.evidence, scope.closure, scope.record) {
        (TriggerEvidence::Refused, _, _) => unknown(UnknownKind::Refused),
        (TriggerEvidence::Missing, _, _) => unknown(UnknownKind::Missing),
        (TriggerEvidence::Admitted, ProgressClosure::Closed, TriggerRecord::Complete) => {
            Disposition::Inactive
        }
        (TriggerEvidence::Admitted, ProgressClosure::Open, _)
        | (TriggerEvidence::Admitted, ProgressClosure::Closed, TriggerRecord::Incomplete) => {
            unknown(UnknownKind::Open)
        }
    }
}

/// How an input of an activation faulted (FR-516 `non_value.kind`).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FaultKind {
    /// The input is incomplete.
    Missing,
    /// The input is refused.
    Refused,
}

/// The input an activation fault names.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum FaultSubject {
    /// The clause's `when` condition.
    Guard,
    /// The anchor of the delivery.
    Anchor,
    /// The capture declared under this name.
    Capture(String),
}

/// An activation that cannot be established (FR-524 "Outputs").
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ActivationFault {
    /// The input the fault names.
    pub subject: FaultSubject,
    /// Missing or refused.
    pub kind: FaultKind,
}

/// The state of one resolved instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InstanceState {
    /// The activation is established.
    Active,
    /// The activation has a fault: `Unavailable` (FR-516).
    Unavailable(ActivationFault),
}

/// One instance a call resolved (FR-524 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Instance {
    /// The identity.
    pub identity: SemanticTriggerIdentity,
    /// Every receipt, in arrival order: the provenance.
    pub receipts: Vec<Receipt>,
    /// The captures, in authored order; empty for a faulted new delivery.
    pub environment: CaptureEnvironment,
    /// Active, or unavailable with its fault.
    pub state: InstanceState,
    /// How many times the initializers were evaluated.
    pub evaluations: u64,
}

/// The result of [`activate`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Activation {
    /// The disposition.
    pub disposition: Disposition,
    /// The instances the lists resolved, in order of first appearance.
    pub instances: Vec<Instance>,
    /// The identities whose deliveries had a false guard: no instance.
    pub guard_false: Vec<SemanticTriggerIdentity>,
    /// The ledger with the created instances and added receipts.
    pub ledger: EvidenceLedger,
}

/// Two payloads under one identity (FR-524 "Instance identity"):
/// `invalid_runtime_input`/`invalid-value` carrying `contradicted_identity`.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("one identity carries two payloads")]
pub struct Contradiction {
    /// The trigger or origin identity.
    pub contradicted_identity: SemanticTriggerIdentity,
}

/// A delivery or origin that supplies more capture facts than the clause
/// declares: `invalid_runtime_input`/`invalid-value` carrying
/// `surplus_capture_identity` and `surplus_capture_index` (FR-524 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("a delivery carries more capture facts than the clause declares")]
pub struct SurplusCapture {
    /// The delivery's or origin's identity.
    pub surplus_capture_identity: SemanticTriggerIdentity,
    /// The index of the first fact beyond the declared captures.
    pub surplus_capture_index: usize,
}

/// Why [`activate`] returned no activation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActivationStop {
    /// A contradiction refusal; nothing was charged.
    Contradiction(Contradiction),
    /// A surplus-capture refusal; nothing was charged.
    Surplus(SurplusCapture),
    /// A counter or the meter stopped the work.
    Stop(EvidenceStop),
}

impl From<EvidenceStop> for ActivationStop {
    fn from(stop: EvidenceStop) -> Self {
        Self::Stop(stop)
    }
}

impl ActivationInput {
    /// The first identity, in list order, that carries a payload different
    /// from an earlier delivery's, an earlier origin's or the retained
    /// instance's (FR-524 "Instance identity"). Nothing is charged.
    pub fn contradiction(&self, obligation: &ObligationKey) -> Option<Contradiction> {
        let mut seen: BTreeMap<&SemanticTriggerIdentity, &[u8]> = BTreeMap::new();
        if let Some(ledger) = &self.ledger {
            for (identity, entry) in ledger.instances_of(obligation) {
                seen.insert(identity, entry.payload().as_bytes());
            }
        }
        let listed = self
            .deliveries
            .iter()
            .map(|delivery| (&delivery.identity, delivery.payload.as_bytes()))
            .chain(
                self.origins
                    .iter()
                    .map(|origin| (&origin.identity, origin.payload.as_bytes())),
            );
        for (identity, payload) in listed {
            match seen.get(identity) {
                Some(earlier) if *earlier != payload => {
                    return Some(Contradiction {
                        contradicted_identity: identity.clone(),
                    });
                }
                Some(_) => {}
                None => {
                    seen.insert(identity, payload);
                }
            }
        }
        None
    }

    /// The first delivery or origin, in list order, that supplies more capture
    /// facts than `clause` declares. Nothing is charged.
    pub fn surplus(&self, clause: &ClauseShape) -> Option<SurplusCapture> {
        let declared = clause.captures.len();
        let listed = self
            .deliveries
            .iter()
            .map(|delivery| (&delivery.identity, delivery.captures.len()))
            .chain(
                self.origins
                    .iter()
                    .map(|origin| (&origin.identity, origin.captures.len())),
            );
        listed
            .into_iter()
            .find(|(_, supplied)| *supplied > declared)
            .map(|(identity, _)| SurplusCapture {
                surplus_capture_identity: identity.clone(),
                surplus_capture_index: declared,
            })
    }

    fn is_evicted_capture(&self, identity: &SemanticTriggerIdentity, name: &str) -> bool {
        self.evicted.iter().any(|record| {
            matches!(record, Evicted::Capture { instance, capture }
                if instance == identity && capture == name)
        })
    }
}

/// What one delivery or origin asks of the establishment.
struct Arrival<'a> {
    identity: &'a SemanticTriggerIdentity,
    payload: &'a super::trigger::Payload,
    receipt: Option<&'a Receipt>,
    guard: Option<bool>,
    anchor: Option<&'a str>,
    captures: &'a [CaptureFact],
}

/// Create one instance per distinct identity of `input`'s deliveries (or
/// origins) over `clause`, evaluating each new instance's captures once at
/// the anchor, and report the faults and the disposition.
///
/// # Errors
///
/// [`ActivationStop::Contradiction`] when one identity carries two payloads
/// (nothing is charged and the ledger is as it was), and
/// [`ActivationStop::Stop`] when a counter or the meter refuses a charge.
pub fn activate(
    clause: &ClauseShape,
    obligation: &ObligationKey,
    input: ActivationInput,
    call: &mut CallMeter<'_>,
) -> Result<Activation, ActivationStop> {
    if let Some(contradiction) = input.contradiction(obligation) {
        return Err(ActivationStop::Contradiction(contradiction));
    }
    if let Some(surplus) = input.surplus(clause) {
        return Err(ActivationStop::Surplus(surplus));
    }
    let arrivals: Vec<Arrival<'_>> = match clause.activation() {
        ClauseActivation::Each { .. } => input
            .deliveries
            .iter()
            .map(|delivery| Arrival {
                identity: &delivery.identity,
                payload: &delivery.payload,
                receipt: Some(&delivery.receipt),
                guard: delivery.guard,
                anchor: Some(&delivery.anchor),
                captures: &delivery.captures,
            })
            .collect(),
        ClauseActivation::Origin => input
            .origins
            .iter()
            .map(|origin| Arrival {
                identity: &origin.identity,
                payload: &origin.payload,
                receipt: None,
                guard: Some(true),
                anchor: None,
                captures: &origin.captures,
            })
            .collect(),
    };
    let mut ledger = input.ledger.clone().unwrap_or_default();
    let mut instances: Vec<Instance> = Vec::new();
    let mut guard_false = Vec::new();
    let mut created = 0_u64;
    for arrival in &arrivals {
        call.visit(Visit::Step)?;
        let listed = instances
            .iter()
            .position(|instance| &instance.identity == arrival.identity);
        let retained = ledger.instance(obligation, arrival.identity).cloned();
        if let Some(entry) = retained {
            // A redelivery of an instance created earlier in this call or
            // retained: no initializer runs (FR-524 "Captures").
            let evicted = retained_fault(&input, arrival.identity, &entry, call)?;
            let next = match (&evicted, arrival.receipt) {
                (None, Some(receipt)) => entry.with_receipt(receipt),
                (_, _) => entry.clone(),
            };
            let instance = instance_of(arrival.identity, &next, evicted.clone());
            if evicted.is_none() {
                ledger = ledger.with_instance(obligation, arrival.identity, next);
            }
            match listed {
                Some(index) => {
                    if let Some(slot) = instances.get_mut(index) {
                        *slot = instance;
                    }
                }
                None => instances.push(instance),
            }
            continue;
        }
        if arrival.guard == Some(false) {
            guard_false.push(arrival.identity.clone());
            continue;
        }
        let established = establish(clause, &input, arrival, call, created)?;
        match established {
            Established::Created(entry) => {
                created += 1;
                let instance = instance_of(arrival.identity, &entry, None);
                ledger = ledger.with_instance(obligation, arrival.identity, entry);
                instances.push(instance);
            }
            Established::Faulted(fault) => {
                let entry = RetainedInstance::faulted(
                    arrival.payload.clone(),
                    arrival.receipt.cloned(),
                    fault,
                );
                let instance = instance_of(arrival.identity, &entry, None);
                ledger = ledger.with_instance(obligation, arrival.identity, entry);
                match listed {
                    Some(index) => {
                        if let Some(slot) = instances.get_mut(index) {
                            *slot = instance;
                        }
                    }
                    None => instances.push(instance),
                }
            }
        }
    }
    let disposition = if instances.is_empty() {
        scope_disposition(input.scope, input.execution)
    } else {
        Disposition::Active
    };
    Ok(Activation {
        disposition,
        instances,
        guard_false,
        ledger,
    })
}

fn instance_of(
    identity: &SemanticTriggerIdentity,
    entry: &RetainedInstance,
    fault: Option<ActivationFault>,
) -> Instance {
    Instance {
        identity: identity.clone(),
        receipts: entry.receipts().to_vec(),
        environment: entry.environment().clone(),
        state: fault
            .or_else(|| entry.fault().cloned())
            .map_or(InstanceState::Active, InstanceState::Unavailable),
        evaluations: entry.evaluations(),
    }
}

/// The instance a retained entry stands for in a call, faulted when the
/// caller evicted one of its capture records (FR-524 "Captures"): the first
/// such capture in authored order, `Missing`, with nothing recomputed.
pub(super) fn retained_instance(
    identity: &SemanticTriggerIdentity,
    entry: &RetainedInstance,
    input: &ActivationInput,
    call: &mut CallMeter<'_>,
) -> Result<Instance, EvidenceStop> {
    let fault = retained_fault(input, identity, entry, call)?;
    Ok(instance_of(identity, entry, fault))
}

fn retained_fault(
    input: &ActivationInput,
    identity: &SemanticTriggerIdentity,
    entry: &RetainedInstance,
    call: &mut CallMeter<'_>,
) -> Result<Option<ActivationFault>, EvidenceStop> {
    for capture in entry.environment().entries() {
        if input.is_evicted_capture(identity, capture.name()) {
            call.visit(Visit::Evicted)?;
            return Ok(Some(ActivationFault {
                subject: FaultSubject::Capture(capture.name().to_owned()),
                kind: FaultKind::Missing,
            }));
        }
    }
    Ok(None)
}

enum Established {
    Created(RetainedInstance),
    Faulted(ActivationFault),
}

/// Establish a new instance: examine the faults in FR-524's order (guard,
/// number of captures, delivery anchor, then each capture in authored order,
/// an evicted record before the capture's fact), and keep the captures.
fn establish(
    clause: &ClauseShape,
    input: &ActivationInput,
    arrival: &Arrival<'_>,
    call: &mut CallMeter<'_>,
    created: u64,
) -> Result<Established, EvidenceStop> {
    let fault = |subject, kind| Ok(Established::Faulted(ActivationFault { subject, kind }));
    if clause.guarded() && arrival.guard.is_none() {
        return fault(FaultSubject::Guard, FaultKind::Missing);
    }
    if let Some((name, _)) = clause.captures.get(arrival.captures.len()) {
        return fault(FaultSubject::Capture(name.to_string()), FaultKind::Missing);
    }
    if arrival.anchor.is_some_and(|anchor| anchor != input.anchor) {
        return fault(FaultSubject::Anchor, FaultKind::Refused);
    }
    let mut entries = Vec::new();
    for (index, ((name, declared), fact)) in
        clause.captures.iter().zip(arrival.captures).enumerate()
    {
        let capture = FaultSubject::Capture(name.to_string());
        if input.is_evicted_capture(arrival.identity, name) {
            call.visit(Visit::Evicted)?;
            return fault(capture, FaultKind::Missing);
        }
        match fact {
            CaptureFact::Missing | CaptureFact::Null => {
                return fault(capture, FaultKind::Missing);
            }
            CaptureFact::WrongType | CaptureFact::Stale => {
                return fault(capture, FaultKind::Refused);
            }
            CaptureFact::Value { anchor, value } => {
                if *anchor != input.anchor {
                    return fault(capture, FaultKind::Refused);
                }
                call.visit(Visit::Capture)?;
                entries.push(CaptureEntry::new(
                    name,
                    declared.clone(),
                    value.clone(),
                    SourceBinding {
                        clause: clause.name.clone(),
                        index,
                    },
                    anchor,
                ));
            }
        }
    }
    call.charge(&[(EvidenceCounter::Instances, created + 1)])
        .map_err(EvidenceStop::Limit)?;
    Ok(Established::Created(RetainedInstance::created(
        arrival.payload.clone(),
        arrival.receipt.cloned(),
        CaptureEnvironment::new(entries),
    )))
}
