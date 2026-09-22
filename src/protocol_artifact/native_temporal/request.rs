// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical `quire.native-temporal-request/v1` production and strict reading.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

use super::common::{
    decode, encode, error, exhausted, identity, invalid, raw_digest, report, validate_digest,
    validate_string, Error, ErrorCode, EvidenceRef, Limits, Report, Usage, WireLimits,
    REQUEST_CONTRACT,
};
use crate::{
    protocol_artifact::{temporal_subject::ValidatedTemporalSubject, v2, wire as w},
    temporal,
};
use qsl_foundation::ByteDigest;

/// Canonical immutable JSON Schema bytes for [`CONTRACT`].
pub const SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../schemas/native-temporal-request-v1.schema.json");
/// SHA-256 digest of [`SCHEMA_BYTES`].
pub const SCHEMA_SHA256: &str = "2539140ff1f6fb5e481e5ae658b81c325a284bfcd85cc5c048ea1d49c4dfdebe";
/// Exact request contract selection.
pub const CONTRACT: &str = REQUEST_CONTRACT;

#[derive(Clone, Debug)]
/// One decision or surrounding-execution progress assertion.
pub struct ProgressInput {
    /// Evidence reference that grounds this progress assertion.
    pub reference: EvidenceRef,
    /// Monotonic progress watermark carried by this assertion.
    pub watermark: i64,
}

#[derive(Clone, Debug)]
/// One independently referenced closure assertion.
pub struct ClosureInput {
    /// Evidence reference that grounds this closure assertion.
    pub reference: EvidenceRef,
    /// Open/closed state being asserted for the covered scope.
    pub state: temporal::Closure,
}

#[derive(Clone, Debug)]
/// Completeness authority and the exact fact population it covers.
pub struct CompletenessInput {
    /// Evidence reference for the completeness authority.
    pub reference: EvidenceRef,
    /// Complete/incomplete state being asserted.
    pub state: temporal::Completeness,
    /// Exact population of fact references the completeness state covers.
    pub facts: Vec<EvidenceRef>,
}

#[derive(Clone, Debug)]
/// One observation-bound native trace position.
pub struct ObservedPosition {
    /// Evidence reference for the observation backing this position.
    pub observation: EvidenceRef,
    /// Native trace position observed at that evidence.
    pub position: temporal::Position,
}

#[derive(Clone, Debug)]
/// Complete downstream-supplied native evaluation input; it contains no result fields.
pub struct Input {
    /// Identity of the temporal instance this input evaluates.
    pub instance: String,
    /// Evidence reference correlating this input to its originating execution.
    pub correspondence: EvidenceRef,
    /// Observed native trace positions supplied for evaluation.
    pub positions: Vec<ObservedPosition>,
    /// Anchor identity the input's trigger and captures are bound to.
    pub anchor: String,
    /// Trigger occurrences supplied alongside this input.
    pub triggers: Vec<temporal::Trigger>,
    /// Whether trigger evidence was admitted, missing, or refused.
    pub trigger_evidence: temporal::Evidence,
    /// Open/closed scope covering the supplied triggers.
    pub trigger_scope: temporal::Closure,
    /// Progress assertion for the decision axis.
    pub decision_progress: ProgressInput,
    /// Closure assertion for the decision axis.
    pub decision_closure: ClosureInput,
    /// Progress assertion for the surrounding-execution axis.
    pub surrounding_progress: ProgressInput,
    /// Closure assertion for the surrounding-execution axis.
    pub surrounding_closure: ClosureInput,
    /// Completed/failed outcome of the surrounding execution.
    pub execution: temporal::Execution,
    /// Completeness authority and the covered fact population.
    pub completeness: CompletenessInput,
    /// Whether this input originates at the authoritative execution origin.
    pub authoritative_origin: bool,
    /// Evictions applied to the trace prior to this input.
    pub evicted: Vec<temporal::Eviction>,
}

#[derive(Clone, Debug)]
/// Canonical request bytes and their distinct raw and semantic identities.
pub struct Document {
    bytes: Vec<u8>,
    digest: ByteDigest,
    wire: Wire,
    trace: temporal::Trace,
    limits: Limits,
}

impl Document {
    /// Returns the canonical request bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Returns the digest of the canonical request bytes.
    pub const fn digest(&self) -> ByteDigest {
        self.digest
    }
    /// Returns the request's content-derived identity string.
    pub fn identity(&self) -> &str {
        &self.wire.identity
    }
}

#[derive(Clone, Debug)]
/// Constructor-private request admitted by production or strict reading.
///
/// ```compile_fail
/// use quire_spec_language::protocol_artifact::native_temporal::request::ValidatedRequest;
/// let _ = ValidatedRequest;
/// ```
pub struct ValidatedRequest {
    document: Document,
    package: Arc<v2::AdmittedPackage>,
    declaration: u32,
}

impl ValidatedRequest {
    /// Returns the underlying validated document.
    pub fn document(&self) -> &Document {
        &self.document
    }
    /// Returns the identity of the checked temporal subject this request was validated against.
    pub fn subject_identity(&self) -> &str {
        &self.document.wire.subject.identity
    }
    /// Returns the digest of the checked temporal subject this request was validated against.
    pub fn subject_digest(&self) -> &str {
        &self.document.wire.subject.digest
    }
    /// Returns the digest of the package that owns the subject's declaration.
    pub fn package_digest(&self) -> &str {
        &self.document.wire.subject.package_digest
    }
    /// Returns the index of the subject's declaration within its owning package.
    pub const fn declaration(&self) -> u32 {
        self.document.wire.subject.declaration
    }
    /// Returns the handle to the subject's root formula node.
    pub fn root(&self) -> &w::Handle {
        &self.document.wire.subject.root
    }
    /// Returns the identity of the temporal instance this request evaluates.
    pub fn instance(&self) -> &str {
        &self.document.wire.instance
    }
    /// Returns the evidence reference correlating this request to its originating execution.
    pub fn correspondence(&self) -> &EvidenceRef {
        &self.document.wire.correspondence
    }
    /// Returns the anchor identity this request's triggers and captures are bound to.
    pub fn anchor(&self) -> &str {
        &self.document.wire.anchor
    }
    /// Returns the identity of the temporal profile this request was produced against.
    pub fn profile_identity(&self) -> &str {
        &self.document.wire.definition.profile_identity
    }
    /// Returns the revision of the temporal profile this request was produced against.
    pub fn profile_revision(&self) -> &str {
        &self.document.wire.definition.profile_revision
    }
    /// Returns the name of the clock binding used to evaluate this request.
    pub fn clock_name(&self) -> &str {
        &self.document.wire.definition.clock_name
    }
    /// Returns the clock configuration used to evaluate this request.
    pub fn clock_configuration(&self) -> &v2::wire::ClockConfiguration {
        &self.document.wire.definition.clock_configuration
    }
    /// Returns an iterator over the observed trace positions carried by this request.
    pub fn positions(&self) -> impl ExactSizeIterator<Item = PositionView<'_>> {
        self.document.wire.positions.iter().map(PositionView)
    }
    /// Returns a view onto the decision-axis progress assertion.
    pub fn decision_progress(&self) -> ProgressView<'_> {
        ProgressView(&self.document.wire.axes.decision_progress)
    }
    /// Returns a view onto the decision-axis closure assertion.
    pub fn decision_closure(&self) -> ClosureView<'_> {
        ClosureView(&self.document.wire.axes.decision_closure)
    }
    /// Returns a view onto the surrounding-execution progress assertion.
    pub fn surrounding_progress(&self) -> ProgressView<'_> {
        ProgressView(&self.document.wire.axes.surrounding_progress)
    }
    /// Returns a view onto the surrounding-execution closure assertion.
    pub fn surrounding_closure(&self) -> ClosureView<'_> {
        ClosureView(&self.document.wire.axes.surrounding_closure)
    }
    /// Returns a view onto the completeness authority and the facts it covers.
    pub fn completeness(&self) -> CompletenessView<'_> {
        CompletenessView(&self.document.wire.completeness)
    }
    /// Returns an iterator over the trigger occurrences carried by this request.
    pub fn triggers(&self) -> impl ExactSizeIterator<Item = TriggerView<'_>> {
        self.document.wire.triggers.iter().map(TriggerView)
    }
    /// Returns whether trigger evidence was admitted, missing, or refused.
    pub fn trigger_evidence(&self) -> temporal::Evidence {
        self.document.wire.trigger_evidence.into()
    }
    /// Returns the open/closed scope covering this request's triggers.
    pub fn trigger_scope(&self) -> temporal::Closure {
        self.document.wire.trigger_scope.into()
    }
    /// Returns the completed/failed outcome of the surrounding execution.
    pub fn execution(&self) -> temporal::Execution {
        self.document.trace.execution
    }
    /// Returns whether this request originates at the authoritative execution origin.
    pub const fn authoritative_origin(&self) -> bool {
        self.document.trace.authoritative_origin
    }
    /// Returns the resource limits this request was validated under.
    pub fn limits(&self) -> Limits {
        self.document.limits
    }

    pub(crate) fn trace(&self) -> &temporal::Trace {
        &self.document.trace
    }
    pub(crate) fn subject_package(&self) -> &v2::AdmittedPackage {
        &self.package
    }
    pub(crate) const fn subject_declaration(&self) -> u32 {
        self.declaration
    }
}

#[derive(Clone, Copy)]
/// Read-only view onto one trigger occurrence carried by a validated request.
pub struct TriggerView<'a>(&'a TriggerWire);

impl<'a> TriggerView<'a> {
    /// Returns the trigger's identity string.
    pub fn identity(self) -> &'a str {
        &self.0.identity
    }
    /// Returns the trigger's receipt string.
    pub fn receipt(self) -> &'a str {
        &self.0.receipt
    }
    /// Returns the anchor identity the trigger is bound to.
    pub fn anchor(self) -> &'a str {
        &self.0.anchor
    }
    /// Returns the trigger's payload as a string.
    pub fn payload(self) -> &'a str {
        &self.0.payload
    }
    /// Returns the trigger's optional guard evaluation result.
    pub const fn guard(self) -> Option<bool> {
        self.0.guard
    }
    /// Returns an iterator over the outcomes of the trigger's captures.
    pub fn captures(self) -> impl ExactSizeIterator<Item = CaptureView<'a>> {
        self.0.captures.iter().map(|capture| match capture {
            CaptureWire::Value { anchor, value } => CaptureView::Value { anchor, value },
            CaptureWire::Missing => CaptureView::Missing,
            CaptureWire::Null => CaptureView::Null,
            CaptureWire::WrongType => CaptureView::WrongType,
            CaptureWire::Stale => CaptureView::Stale,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Outcome of one capture attempt made against a trigger.
pub enum CaptureView<'a> {
    /// The capture succeeded and produced an anchor/value pair.
    Value {
        /// Anchor identity the captured value is attached to.
        anchor: &'a str,
        /// Captured value, as a validated string.
        value: &'a str,
    },
    /// The capture target was absent.
    Missing,
    /// The capture target was present but null.
    Null,
    /// The capture target was present but of the wrong type.
    WrongType,
    /// The capture target was present but stale relative to the trigger.
    Stale,
}

#[derive(Clone, Copy)]
/// Read-only view onto one observed trace position carried by a validated request.
pub struct PositionView<'a>(&'a PositionWire);

impl<'a> PositionView<'a> {
    /// Returns the position's content-derived identity string.
    pub fn identity(self) -> &'a str {
        &self.0.identity
    }
    /// Returns the evidence reference for the observation backing this position.
    pub fn observation(self) -> &'a EvidenceRef {
        &self.0.observation
    }
    /// Returns the position's clock coordinate.
    pub const fn coordinate(self) -> i64 {
        self.0.coordinate
    }
    /// Returns the position's tie-breaking order authority and key, if present.
    pub fn order(self) -> Option<(&'a str, i64)> {
        self.0
            .order
            .as_ref()
            .map(|order| (order.authority.as_str(), order.key))
    }
    /// Returns an iterator over each formula leaf's node index, handle, and Boolean valuation at this position.
    pub fn valuations(self) -> impl ExactSizeIterator<Item = (u32, &'a w::Handle, bool)> {
        self.0
            .valuations
            .iter()
            .map(|value| (value.node, &value.leaf, value.value))
    }
}

#[derive(Clone, Copy)]
/// Read-only view onto a progress assertion carried by a validated request.
pub struct ProgressView<'a>(&'a ProgressWire);
impl<'a> ProgressView<'a> {
    /// Returns the evidence reference grounding this progress assertion.
    pub fn reference(self) -> &'a EvidenceRef {
        &self.0.reference
    }
    /// Returns the progress watermark value asserted.
    pub const fn watermark(self) -> i64 {
        self.0.watermark
    }
}

#[derive(Clone, Copy)]
/// Read-only view onto a closure assertion carried by a validated request.
pub struct ClosureView<'a>(&'a ClosureWire);
impl<'a> ClosureView<'a> {
    /// Returns the evidence reference grounding this closure assertion.
    pub fn reference(self) -> &'a EvidenceRef {
        &self.0.reference
    }
    /// Returns the asserted open/closed state.
    pub fn state(self) -> temporal::Closure {
        self.0.state.into()
    }
}

#[derive(Clone, Copy)]
/// Read-only view onto the completeness authority carried by a validated request.
pub struct CompletenessView<'a>(&'a CompletenessWire);
impl<'a> CompletenessView<'a> {
    /// Returns the evidence reference for the completeness authority.
    pub fn reference(self) -> &'a EvidenceRef {
        &self.0.reference
    }
    /// Returns the asserted complete/incomplete state.
    pub fn state(self) -> temporal::Completeness {
        self.0.state.into()
    }
    /// Returns an iterator over the fact references the completeness state covers.
    pub fn facts(self) -> impl ExactSizeIterator<Item = &'a EvidenceRef> {
        self.0.facts.iter()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ClosureStateWire {
    Open,
    Closed,
}
impl From<temporal::Closure> for ClosureStateWire {
    fn from(value: temporal::Closure) -> Self {
        match value {
            temporal::Closure::Open => Self::Open,
            temporal::Closure::Closed => Self::Closed,
        }
    }
}
impl From<ClosureStateWire> for temporal::Closure {
    fn from(value: ClosureStateWire) -> Self {
        match value {
            ClosureStateWire::Open => Self::Open,
            ClosureStateWire::Closed => Self::Closed,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CompletenessStateWire {
    Complete,
    Incomplete,
}
impl From<temporal::Completeness> for CompletenessStateWire {
    fn from(value: temporal::Completeness) -> Self {
        match value {
            temporal::Completeness::Complete => Self::Complete,
            temporal::Completeness::Incomplete => Self::Incomplete,
        }
    }
}
impl From<CompletenessStateWire> for temporal::Completeness {
    fn from(value: CompletenessStateWire) -> Self {
        match value {
            CompletenessStateWire::Complete => Self::Complete,
            CompletenessStateWire::Incomplete => Self::Incomplete,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExecutionWire {
    Completed,
    Failed,
}
impl From<temporal::Execution> for ExecutionWire {
    fn from(value: temporal::Execution) -> Self {
        match value {
            temporal::Execution::Completed => Self::Completed,
            temporal::Execution::Failed => Self::Failed,
        }
    }
}
impl From<ExecutionWire> for temporal::Execution {
    fn from(value: ExecutionWire) -> Self {
        match value {
            ExecutionWire::Completed => Self::Completed,
            ExecutionWire::Failed => Self::Failed,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EvidenceWire {
    Admitted,
    Missing,
    Refused,
}
impl From<temporal::Evidence> for EvidenceWire {
    fn from(value: temporal::Evidence) -> Self {
        match value {
            temporal::Evidence::Admitted => Self::Admitted,
            temporal::Evidence::Missing => Self::Missing,
            temporal::Evidence::Refused => Self::Refused,
        }
    }
}
impl From<EvidenceWire> for temporal::Evidence {
    fn from(value: EvidenceWire) -> Self {
        match value {
            EvidenceWire::Admitted => Self::Admitted,
            EvidenceWire::Missing => Self::Missing,
            EvidenceWire::Refused => Self::Refused,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SubjectWire {
    contract: String,
    identity: String,
    digest: String,
    package_digest: String,
    declaration: u32,
    root: w::Handle,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DefinitionWire {
    profile_identity: String,
    profile_revision: String,
    clock_name: String,
    clock_configuration: v2::wire::ClockConfiguration,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OrderWire {
    authority: String,
    key: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ValuationWire {
    node: u32,
    leaf: w::Handle,
    value: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PositionWire {
    identity: String,
    observation: EvidenceRef,
    coordinate: i64,
    order: Option<OrderWire>,
    valuations: Vec<ValuationWire>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CaptureWire {
    Value { anchor: String, value: String },
    Missing,
    Null,
    WrongType,
    Stale,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TriggerWire {
    identity: String,
    receipt: String,
    anchor: String,
    payload: String,
    guard: Option<bool>,
    captures: Vec<CaptureWire>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum EvictionWire {
    Valuation { node: u32, coordinate: i64 },
    Capture { instance: String, capture: u64 },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProgressWire {
    reference: EvidenceRef,
    watermark: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ClosureWire {
    reference: EvidenceRef,
    state: ClosureStateWire,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AxesWire {
    decision_progress: ProgressWire,
    decision_closure: ClosureWire,
    surrounding_progress: ProgressWire,
    surrounding_closure: ClosureWire,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompletenessWire {
    reference: EvidenceRef,
    state: CompletenessStateWire,
    facts: Vec<EvidenceRef>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Wire {
    contract: String,
    identity: String,
    subject: SubjectWire,
    instance: String,
    anchor: String,
    correspondence: EvidenceRef,
    definition: DefinitionWire,
    positions: Vec<PositionWire>,
    triggers: Vec<TriggerWire>,
    trigger_evidence: EvidenceWire,
    trigger_scope: ClosureStateWire,
    axes: AxesWire,
    execution: ExecutionWire,
    completeness: CompletenessWire,
    authoritative_origin: bool,
    evicted: Vec<EvictionWire>,
    limits: WireLimits,
}

#[derive(Serialize)]
struct Preimage<'a> {
    contract: &'a str,
    subject: &'a SubjectWire,
    instance: &'a str,
    anchor: &'a str,
    correspondence: &'a EvidenceRef,
    definition: &'a DefinitionWire,
    positions: &'a [PositionWire],
    triggers: &'a [TriggerWire],
    trigger_evidence: EvidenceWire,
    trigger_scope: ClosureStateWire,
    axes: &'a AxesWire,
    execution: ExecutionWire,
    completeness: &'a CompletenessWire,
    authoritative_origin: bool,
    evicted: &'a [EvictionWire],
    limits: WireLimits,
}

#[derive(Serialize)]
struct PositionPreimage<'a> {
    observation: &'a EvidenceRef,
    coordinate: i64,
    order: &'a Option<OrderWire>,
    valuations: &'a [ValuationWire],
}

fn capture_into_wire(value: temporal::CaptureInput) -> CaptureWire {
    match value {
        temporal::CaptureInput::Value { anchor, value } => CaptureWire::Value { anchor, value },
        temporal::CaptureInput::Missing => CaptureWire::Missing,
        temporal::CaptureInput::Null => CaptureWire::Null,
        temporal::CaptureInput::WrongType => CaptureWire::WrongType,
        temporal::CaptureInput::Stale => CaptureWire::Stale,
    }
}

fn capture_from_wire(value: &CaptureWire) -> temporal::CaptureInput {
    match value {
        CaptureWire::Value { anchor, value } => temporal::CaptureInput::Value {
            anchor: anchor.clone(),
            value: value.clone(),
        },
        CaptureWire::Missing => temporal::CaptureInput::Missing,
        CaptureWire::Null => temporal::CaptureInput::Null,
        CaptureWire::WrongType => temporal::CaptureInput::WrongType,
        CaptureWire::Stale => temporal::CaptureInput::Stale,
    }
}

fn trigger_into_wire(value: temporal::Trigger) -> Result<TriggerWire, Error> {
    let mut captures = Vec::new();
    captures
        .try_reserve(value.captures.len())
        .map_err(|_| error(ErrorCode::Allocation, "triggers.captures"))?;
    captures.extend(value.captures.into_iter().map(capture_into_wire));
    Ok(TriggerWire {
        identity: value.identity,
        receipt: value.receipt,
        anchor: value.anchor,
        payload: value.payload,
        guard: value.guard,
        captures,
    })
}

fn trigger_from_wire(value: &TriggerWire) -> temporal::Trigger {
    temporal::Trigger {
        identity: value.identity.clone(),
        receipt: value.receipt.clone(),
        anchor: value.anchor.clone(),
        payload: value.payload.clone(),
        guard: value.guard,
        captures: value.captures.iter().map(capture_from_wire).collect(),
    }
}

fn eviction_from_wire(value: &EvictionWire) -> Result<temporal::Eviction, Error> {
    Ok(match value {
        EvictionWire::Valuation { node, coordinate } => temporal::Eviction::Valuation {
            node: *node,
            coordinate: *coordinate,
        },
        EvictionWire::Capture { instance, capture } => temporal::Eviction::Capture {
            instance: instance.clone(),
            capture: usize::try_from(*capture).map_err(|_| invalid("evicted.capture"))?,
        },
    })
}

fn preimage(wire: &Wire, limits: Limits) -> Result<Vec<u8>, Error> {
    encode(
        &Preimage {
            contract: &wire.contract,
            subject: &wire.subject,
            instance: &wire.instance,
            anchor: &wire.anchor,
            correspondence: &wire.correspondence,
            definition: &wire.definition,
            positions: &wire.positions,
            triggers: &wire.triggers,
            trigger_evidence: wire.trigger_evidence,
            trigger_scope: wire.trigger_scope,
            axes: &wire.axes,
            execution: wire.execution,
            completeness: &wire.completeness,
            authoritative_origin: wire.authoritative_origin,
            evicted: &wire.evicted,
            limits: wire.limits,
        },
        limits,
    )
}

fn required_leaves(subject: &ValidatedTemporalSubject) -> BTreeMap<u32, w::Handle> {
    subject
        .temporal_nodes()
        .filter_map(|(node, value)| match &value.operation {
            w::TemporalOperation::Holds { value } => Some((node, value.clone())),
            _ => None,
        })
        .collect()
}

fn formula_metrics(subject: &ValidatedTemporalSubject) -> Result<(usize, usize, usize), Error> {
    let nodes: BTreeMap<_, _> = subject.temporal_nodes().collect();
    let mut stack = vec![(subject.root().index, 1usize)];
    let mut depths = BTreeMap::<u32, usize>::new();
    while let Some((index, depth)) = stack.pop() {
        if depths.get(&index).is_some_and(|known| *known >= depth) {
            continue;
        }
        let node = nodes
            .get(&index)
            .ok_or_else(|| invalid("formula.reference"))?;
        depths.insert(index, depth);
        let next = depth
            .checked_add(1)
            .ok_or_else(|| exhausted("limits.formula_depth"))?;
        match &node.operation {
            w::TemporalOperation::Group { value } | w::TemporalOperation::Unary { value, .. } => {
                stack.push((value.index, next))
            }
            w::TemporalOperation::Binary { left, right, .. } => {
                stack.push((left.index, next));
                stack.push((right.index, next));
            }
            w::TemporalOperation::Constant { .. } | w::TemporalOperation::Holds { .. } => {}
        }
    }
    let history_span = subject
        .required_history()
        .unwrap_or_default()
        .iter()
        .map(|interval| match interval.upper.checked() {
            Ok(crate::protocol_artifact::ProtocolNumber::Integer(value)) => {
                usize::try_from(value.value()).map_err(|_| invalid("formula.interval"))
            }
            _ => Err(invalid("formula.interval")),
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(0);
    Ok((
        depths.len(),
        depths.values().copied().max().unwrap_or(0),
        history_span,
    ))
}

fn validate_unique_evidence<'a>(
    values: impl IntoIterator<Item = &'a EvidenceRef>,
) -> Result<BTreeSet<&'a str>, Error> {
    let mut identities = BTreeSet::new();
    let mut digests = BTreeSet::new();
    for value in values {
        if !identities.insert(value.identity()) || !digests.insert(value.digest()) {
            return Err(invalid("evidence.duplicate"));
        }
    }
    Ok(identities)
}

fn validate_limits(document: Limits, reader: Limits) -> Result<(), Error> {
    macro_rules! within { ($($field:ident),* $(,)?) => { $(if document.$field > reader.$field { return Err(exhausted(concat!("limits.", stringify!($field)))); })* }; }
    within!(
        formula_nodes,
        formula_depth,
        positions,
        valuations,
        captures,
        support,
        history_span,
        evaluation_steps,
        lineage,
        visited
    );
    Ok(())
}

fn subject_wire(subject: &ValidatedTemporalSubject) -> SubjectWire {
    SubjectWire {
        contract: "quire.checked-temporal-subject/v1".into(),
        identity: subject.document().identity().into(),
        digest: format!("{:x}", subject.document().digest()),
        package_digest: format!("{:x}", subject.package_digest()),
        declaration: subject.declaration(),
        root: subject.root().clone(),
    }
}

fn expected_definition(
    subject: &ValidatedTemporalSubject,
) -> Result<(String, String, String, v2::wire::ClockConfiguration), Error> {
    let package = &subject.package().package().inherited;
    let declaration = package
        .declarations
        .get(usize::try_from(subject.declaration()).map_err(|_| invalid("subject.declaration"))?)
        .ok_or_else(|| invalid("subject.declaration"))?;
    let definition = package
        .definitions
        .get(usize::try_from(declaration.profile).map_err(|_| invalid("definition.profile"))?)
        .ok_or_else(|| invalid("definition.profile"))?;
    let (clock_index, configuration) =
        subject.clock().ok_or_else(|| invalid("definition.clock"))?;
    let clock = declaration
        .bindings
        .get(usize::try_from(clock_index).map_err(|_| invalid("definition.clock"))?)
        .and_then(|binding| binding.name.strip_prefix("clock:"))
        .ok_or_else(|| invalid("definition.clock"))?;
    Ok((
        definition.identity.clone(),
        definition.revision.value.clone(),
        clock.into(),
        configuration.clone(),
    ))
}

fn validate_wire(
    wire: &Wire,
    subject: &ValidatedTemporalSubject,
    reader_limits: Limits,
    usage: &mut Usage,
) -> Result<temporal::Trace, Error> {
    if wire.contract != CONTRACT {
        return Err(invalid("contract"));
    }
    validate_digest(&wire.identity, "identity")?;
    if wire.subject != subject_wire(subject) {
        return Err(invalid("subject"));
    }
    let document_limits = Limits::try_from(wire.limits)?;
    validate_limits(document_limits, reader_limits)?;
    validate_string(&wire.instance, false, document_limits, usage, "instance")?;
    validate_string(&wire.anchor, false, document_limits, usage, "anchor")?;
    validate_string(
        &wire.definition.profile_identity,
        false,
        document_limits,
        usage,
        "definition.profile_identity",
    )?;
    validate_string(
        &wire.definition.profile_revision,
        false,
        document_limits,
        usage,
        "definition.profile_revision",
    )?;
    validate_string(
        &wire.definition.clock_name,
        false,
        document_limits,
        usage,
        "definition.clock_name",
    )?;
    wire.correspondence
        .validate("correspondence", document_limits, usage)?;
    wire.axes
        .decision_progress
        .reference
        .validate("decision-progress", document_limits, usage)?;
    wire.axes
        .decision_closure
        .reference
        .validate("decision-closure", document_limits, usage)?;
    wire.axes.surrounding_progress.reference.validate(
        "surrounding-progress",
        document_limits,
        usage,
    )?;
    wire.axes.surrounding_closure.reference.validate(
        "surrounding-closure",
        document_limits,
        usage,
    )?;
    wire.completeness
        .reference
        .validate("completeness", document_limits, usage)?;
    for fact in &wire.completeness.facts {
        fact.validate("completeness-fact", document_limits, usage)?;
    }
    let evidence_identities = validate_unique_evidence(
        [
            &wire.correspondence,
            &wire.axes.decision_progress.reference,
            &wire.axes.decision_closure.reference,
            &wire.axes.surrounding_progress.reference,
            &wire.axes.surrounding_closure.reference,
            &wire.completeness.reference,
        ]
        .into_iter()
        .chain(wire.completeness.facts.iter())
        .chain(wire.positions.iter().map(|position| &position.observation)),
    )?;
    let (profile_identity, profile_revision, clock_name, configuration) =
        expected_definition(subject)?;
    if wire.definition.profile_identity != profile_identity
        || wire.definition.profile_revision != profile_revision
        || wire.definition.clock_name != clock_name
        || wire.definition.clock_configuration != configuration
    {
        return Err(invalid("definition"));
    }
    let leaves = required_leaves(subject);
    let (formula_nodes, formula_depth, history_span) = formula_metrics(subject)?;
    usage.formula_nodes = formula_nodes;
    if usage.formula_nodes > document_limits.formula_nodes {
        return Err(exhausted("limits.formula_nodes"));
    }
    if formula_depth > document_limits.formula_depth {
        return Err(exhausted("limits.formula_depth"));
    }
    if history_span > document_limits.history_span {
        return Err(exhausted("limits.history_span"));
    }
    if wire.positions.len() > document_limits.positions {
        return Err(exhausted("limits.positions"));
    }
    let triggers_valid = match wire.trigger_evidence {
        EvidenceWire::Admitted => {
            wire.triggers.len() <= 1
                && wire
                    .triggers
                    .first()
                    .is_none_or(|trigger| trigger.identity == wire.instance)
        }
        EvidenceWire::Missing | EvidenceWire::Refused => wire.triggers.is_empty(),
    };
    if !triggers_valid {
        return Err(invalid("triggers.instance"));
    }
    let expected_captures = subject
        .captures()
        .ok_or_else(|| invalid("subject.captures"))?
        .len();
    let mut receipts = BTreeSet::new();
    let mut captures = 0usize;
    for trigger in &wire.triggers {
        validate_string(
            &trigger.identity,
            false,
            document_limits,
            usage,
            "triggers.identity",
        )?;
        validate_string(
            &trigger.receipt,
            false,
            document_limits,
            usage,
            "triggers.receipt",
        )?;
        validate_string(
            &trigger.anchor,
            false,
            document_limits,
            usage,
            "triggers.anchor",
        )?;
        validate_string(
            &trigger.payload,
            true,
            document_limits,
            usage,
            "triggers.payload",
        )?;
        for capture in &trigger.captures {
            if let CaptureWire::Value { anchor, value } = capture {
                validate_string(
                    anchor,
                    false,
                    document_limits,
                    usage,
                    "triggers.captures.anchor",
                )?;
                validate_string(
                    value,
                    true,
                    document_limits,
                    usage,
                    "triggers.captures.value",
                )?;
            }
        }
        if trigger.anchor != wire.anchor
            || trigger.captures.len() != expected_captures
            || !receipts.insert(trigger.receipt.as_str())
        {
            return Err(invalid("triggers"));
        }
        captures = captures
            .checked_add(trigger.captures.len())
            .ok_or_else(|| exhausted("limits.captures"))?;
    }
    if captures > document_limits.captures {
        return Err(exhausted("limits.captures"));
    }
    usage.captures = captures;
    let origin = subject.history_boundary() == Some("execution-origin");
    if wire.authoritative_origin != origin {
        return Err(invalid("authoritative_origin"));
    }
    let mut identities = BTreeSet::new();
    let mut valuations = 0usize;
    let mut previous: Option<&PositionWire> = None;
    for position in &wire.positions {
        position
            .observation
            .validate("observation", document_limits, usage)?;
        if !identities.insert(position.identity.as_str())
            || evidence_identities.contains(position.identity.as_str())
        {
            return Err(invalid("positions.identity"));
        }
        validate_digest(&position.identity, "positions.identity")?;
        if let Some(order) = &position.order {
            validate_string(
                &order.authority,
                false,
                document_limits,
                usage,
                "positions.order.authority",
            )?;
        }
        if let Some(left) = previous {
            let ordered = left.coordinate < position.coordinate
                || (left.coordinate == position.coordinate
                    && match (&left.order, &position.order) {
                        (Some(left), Some(right)) => {
                            left.authority == right.authority && left.key < right.key
                        }
                        _ => false,
                    });
            if !ordered {
                return Err(invalid("positions.order"));
            }
        }
        previous = Some(position);
        if position.valuations.len() != leaves.len() {
            return Err(invalid("positions.valuations"));
        }
        for (valuation, (node, leaf)) in position.valuations.iter().zip(&leaves) {
            if valuation.node != *node || valuation.leaf != *leaf {
                return Err(invalid("positions.valuations"));
            }
        }
        valuations = valuations
            .checked_add(position.valuations.len())
            .ok_or_else(|| exhausted("limits.valuations"))?;
        let expected = identity(
            "quire.native-temporal-position/v1",
            &encode(
                &PositionPreimage {
                    observation: &position.observation,
                    coordinate: position.coordinate,
                    order: &position.order,
                    valuations: &position.valuations,
                },
                document_limits,
            )?,
        );
        if position.identity != expected {
            return Err(invalid("positions.identity"));
        }
    }
    if valuations > document_limits.valuations {
        return Err(exhausted("limits.valuations"));
    }
    usage.positions = wire.positions.len();
    usage.valuations = valuations;
    if wire.completeness.facts.len() > document_limits.valuations {
        return Err(exhausted("limits.valuations"));
    }
    if wire.correspondence.population() != 1
        || wire.axes.decision_progress.reference.population() != 1
        || wire.axes.decision_closure.reference.population() != 1
        || wire.axes.surrounding_progress.reference.population() != 1
        || wire.axes.surrounding_closure.reference.population() != 1
        || wire.completeness.reference.population()
            != u64::try_from(wire.completeness.facts.len())
                .map_err(|_| invalid("completeness.population"))?
        || wire.completeness.facts.len() != wire.positions.len()
    {
        return Err(invalid("evidence.population"));
    }
    for (position, fact) in wire.positions.iter().zip(&wire.completeness.facts) {
        let population = u64::try_from(position.valuations.len())
            .map_err(|_| invalid("positions.population"))?;
        if position.observation.population() != population || fact.population() != population {
            return Err(invalid("evidence.population"));
        }
    }
    usage.visited = usage
        .visited
        .checked_add(formula_nodes)
        .and_then(|value| value.checked_add(wire.positions.len()))
        .and_then(|value| value.checked_add(valuations))
        .and_then(|value| value.checked_add(captures))
        .and_then(|value| value.checked_add(wire.evicted.len()))
        .filter(|value| *value <= document_limits.visited)
        .ok_or_else(|| exhausted("limits.visited"))?;
    let positions = wire
        .positions
        .iter()
        .map(|value| temporal::Position {
            coordinate: value.coordinate,
            order: value.order.as_ref().map(|order| temporal::OrderKey {
                authority: order.authority.clone(),
                key: order.key,
            }),
            valuations: value
                .valuations
                .iter()
                .map(|value| (value.node, value.value))
                .collect(),
        })
        .collect();
    Ok(temporal::Trace {
        clock: temporal::ClockBinding {
            name: wire.definition.clock_name.clone(),
            profile_identity: wire.definition.profile_identity.clone(),
            profile_revision: wire.definition.profile_revision.clone(),
            parameters: clock_parameters(&wire.definition.clock_configuration)?,
        },
        positions,
        anchor: wire.anchor.clone(),
        triggers: wire.triggers.iter().map(trigger_from_wire).collect(),
        trigger_evidence: wire.trigger_evidence.into(),
        trigger_scope: wire.trigger_scope.into(),
        decision_scope: wire.axes.decision_closure.state.into(),
        surrounding_execution: wire.axes.surrounding_closure.state.into(),
        execution: wire.execution.into(),
        completeness: wire.completeness.state.into(),
        authoritative_origin: wire.authoritative_origin,
        watermark: wire.axes.decision_progress.watermark,
        evicted: wire
            .evicted
            .iter()
            .map(eviction_from_wire)
            .collect::<Result<_, _>>()?,
    })
}

fn clock_parameters(
    clock: &v2::wire::ClockConfiguration,
) -> Result<BTreeMap<String, String>, Error> {
    let mut output = BTreeMap::new();
    match clock {
        v2::wire::ClockConfiguration::EventPosition { sequence_authority } => {
            output.insert("sequence_authority".into(), sequence_authority.clone());
        }
        v2::wire::ClockConfiguration::FixedSample {
            epoch,
            period,
            unit,
        } => {
            output.insert(
                "epoch".into(),
                serde_json::to_string(&epoch.checked().map_err(|_| invalid("definition.clock"))?)
                    .map_err(|_| invalid("definition.clock"))?,
            );
            output.insert(
                "period".into(),
                serde_json::to_string(&period.checked().map_err(|_| invalid("definition.clock"))?)
                    .map_err(|_| invalid("definition.clock"))?,
            );
            output.insert("unit".into(), unit.clone());
        }
        v2::wire::ClockConfiguration::TimestampedEvent { .. } => {
            return Err(error(ErrorCode::Unsupported, "definition.clock"))
        }
    }
    Ok(output)
}

fn build_wire(
    subject: &ValidatedTemporalSubject,
    input: Input,
    limits: Limits,
) -> Result<Wire, Error> {
    let limits = limits.bounded();
    let Input {
        instance,
        correspondence,
        positions: observed_positions,
        anchor,
        triggers: input_triggers,
        trigger_evidence,
        trigger_scope,
        decision_progress,
        decision_closure,
        surrounding_progress,
        surrounding_closure,
        execution,
        completeness,
        authoritative_origin,
        evicted: input_evicted,
    } = input;
    if observed_positions.len() > limits.positions {
        return Err(exhausted("limits.positions"));
    }
    if input_triggers.len() > 1 {
        return Err(invalid("triggers.instance"));
    }
    let capture_count = input_triggers.iter().try_fold(0usize, |total, trigger| {
        total
            .checked_add(trigger.captures.len())
            .ok_or_else(|| exhausted("limits.captures"))
    })?;
    if capture_count > limits.captures {
        return Err(exhausted("limits.captures"));
    }
    let leaves = required_leaves(subject);
    let valuation_count = leaves
        .len()
        .checked_mul(observed_positions.len())
        .ok_or_else(|| exhausted("limits.valuations"))?;
    if valuation_count > limits.valuations {
        return Err(exhausted("limits.valuations"));
    }
    let mut positions = Vec::new();
    positions
        .try_reserve(observed_positions.len())
        .map_err(|_| error(ErrorCode::Allocation, "positions"))?;
    for observed in observed_positions {
        let mut valuations = Vec::new();
        valuations
            .try_reserve(leaves.len())
            .map_err(|_| error(ErrorCode::Allocation, "valuations"))?;
        if observed.position.valuations.len() != leaves.len() {
            return Err(invalid("positions.valuations"));
        }
        for (node, leaf) in &leaves {
            let value = observed
                .position
                .valuations
                .get(node)
                .copied()
                .ok_or_else(|| invalid("positions.valuations"))?;
            valuations.push(ValuationWire {
                node: *node,
                leaf: leaf.clone(),
                value,
            });
        }
        let order = observed.position.order.map(|order| OrderWire {
            authority: order.authority,
            key: order.key,
        });
        let position_identity = identity(
            "quire.native-temporal-position/v1",
            &encode(
                &PositionPreimage {
                    observation: &observed.observation,
                    coordinate: observed.position.coordinate,
                    order: &order,
                    valuations: &valuations,
                },
                limits,
            )?,
        );
        positions.push(PositionWire {
            identity: position_identity,
            observation: observed.observation,
            coordinate: observed.position.coordinate,
            order,
            valuations,
        });
    }
    positions.sort_by(|left, right| {
        left.coordinate
            .cmp(&right.coordinate)
            .then_with(|| match (&left.order, &right.order) {
                (Some(left), Some(right)) if left.authority == right.authority => {
                    left.key.cmp(&right.key)
                }
                _ => std::cmp::Ordering::Equal,
            })
    });
    let (profile_identity, profile_revision, clock_name, clock_configuration) =
        expected_definition(subject)?;
    let mut evicted = Vec::new();
    evicted
        .try_reserve(input_evicted.len())
        .map_err(|_| error(ErrorCode::Allocation, "evicted"))?;
    for value in input_evicted {
        evicted.push(match value {
            temporal::Eviction::Valuation { node, coordinate } => {
                EvictionWire::Valuation { node, coordinate }
            }
            temporal::Eviction::Capture { instance, capture } => EvictionWire::Capture {
                instance,
                capture: u64::try_from(capture).map_err(|_| invalid("evicted.capture"))?,
            },
        });
    }
    let mut triggers = Vec::new();
    triggers
        .try_reserve(input_triggers.len())
        .map_err(|_| error(ErrorCode::Allocation, "triggers"))?;
    for trigger in input_triggers {
        triggers.push(trigger_into_wire(trigger)?);
    }
    let mut wire = Wire {
        contract: CONTRACT.into(),
        identity: String::new(),
        subject: subject_wire(subject),
        instance,
        anchor,
        correspondence,
        definition: DefinitionWire {
            profile_identity,
            profile_revision,
            clock_name,
            clock_configuration,
        },
        positions,
        triggers,
        trigger_evidence: trigger_evidence.into(),
        trigger_scope: trigger_scope.into(),
        axes: AxesWire {
            decision_progress: ProgressWire {
                reference: decision_progress.reference,
                watermark: decision_progress.watermark,
            },
            decision_closure: ClosureWire {
                reference: decision_closure.reference,
                state: decision_closure.state.into(),
            },
            surrounding_progress: ProgressWire {
                reference: surrounding_progress.reference,
                watermark: surrounding_progress.watermark,
            },
            surrounding_closure: ClosureWire {
                reference: surrounding_closure.reference,
                state: surrounding_closure.state.into(),
            },
        },
        execution: execution.into(),
        completeness: CompletenessWire {
            reference: completeness.reference,
            state: completeness.state.into(),
            facts: completeness.facts,
        },
        authoritative_origin,
        evicted,
        limits: limits.try_into()?,
    };
    wire.identity = identity(CONTRACT, &preimage(&wire, limits)?);
    Ok(wire)
}

fn finish(
    wire: Wire,
    trace: temporal::Trace,
    limits: Limits,
    usage: &mut Usage,
) -> Result<Document, Error> {
    let bytes = encode(&wire, limits)?;
    usage.output_bytes = bytes.len();
    Ok(Document {
        digest: raw_digest(&bytes),
        bytes,
        wire,
        trace,
        limits,
    })
}

/// Produces one canonical request from a strict FR-051 subject and complete input.
pub fn produce(
    subject: &ValidatedTemporalSubject,
    input: Input,
    limits: Limits,
) -> Report<Document> {
    let limits = limits.bounded();
    let mut usage = Usage::default();
    let result = (|| {
        let wire = build_wire(subject, input, limits)?;
        let trace = validate_wire(&wire, subject, limits, &mut usage)?;
        finish(wire, trace, limits, &mut usage)
    })();
    report(limits, usage, result)
}

/// Strictly reads canonical request bytes against the exact checked subject.
pub fn read(
    bytes: &[u8],
    subject: &ValidatedTemporalSubject,
    limits: Limits,
) -> Report<ValidatedRequest> {
    let limits = limits.bounded();
    let mut usage = Usage::default();
    let result = (|| {
        let wire: Wire = decode(bytes, limits, &mut usage)?;
        let trace = validate_wire(&wire, subject, limits, &mut usage)?;
        let document_limits = Limits::try_from(wire.limits)?;
        if bytes.len() > document_limits.output_bytes {
            return Err(exhausted("limits.output_bytes"));
        }
        if wire.identity != identity(CONTRACT, &preimage(&wire, limits)?) {
            return Err(invalid("identity"));
        }
        let canonical = encode(&wire, limits)?;
        if canonical != bytes {
            return Err(error(ErrorCode::NonCanonical, "document"));
        }
        usage.output_bytes = canonical.len();
        Ok(ValidatedRequest {
            document: Document {
                digest: raw_digest(bytes),
                bytes: canonical,
                wire,
                trace,
                limits: document_limits,
            },
            package: subject.package_arc(),
            declaration: subject.declaration(),
        })
    })();
    report(limits, usage, result)
}
