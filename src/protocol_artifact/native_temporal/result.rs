// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical `quire.native-temporal-result/v1` evaluation and strict reading.

use serde::{Deserialize, Serialize};

use super::{
    common::{
        decode, encode, error, exhausted, identity, invalid, raw_digest, report, validate_digest,
        Error, ErrorCode, EvidenceRef, Limits, Report, Usage, WireLimits, RESULT_CONTRACT,
    },
    request::ValidatedRequest,
};
use crate::temporal;
use qsl_foundation::ByteDigest;

/// Canonical immutable JSON Schema bytes for [`CONTRACT`].
pub const SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../schemas/native-temporal-result-v1.schema.json");
/// SHA-256 digest of [`SCHEMA_BYTES`].
pub const SCHEMA_SHA256: &str = "e55e15cc852f0145244d233ca5c88381e0969daf25f644e45a4da3362b611e28";
/// Exact result contract selection.
pub const CONTRACT: &str = RESULT_CONTRACT;

#[derive(Clone, Copy, Debug)]
/// Immutable result lineage selected by a producer.
pub enum Relation<'a> {
    /// This result has no predecessor; it is the first in its lineage.
    Original,
    /// This result supersedes the given predecessor, replacing it as the
    /// current answer within the same lineage.
    Superseding(&'a ValidatedResult),
    /// This result invalidates the given predecessor, retracting it within
    /// the same lineage.
    Invalidating(&'a ValidatedResult),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Which lineage relation a result stands in relative to a predecessor.
pub enum RelationKind {
    /// The result has no predecessor.
    Original,
    /// The result supersedes and replaces a predecessor.
    Superseding,
    /// The result invalidates and retracts a predecessor.
    Invalidating,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Whether the trigger or execution origin governing a result was activated.
pub enum ActivationState {
    /// No trigger was admitted within a closed, complete trigger scope; not a
    /// true obligation to assess.
    Inactive,
    /// The trigger scope was open, or trigger evidence was missing or
    /// refused, so activation could not be determined.
    Unknown,
    /// One semantic trigger was admitted, or the whole-execution origin
    /// activated the obligation.
    Active,
    /// Activation could not be reported because the request was refused,
    /// incomplete, or its obligation was never activated.
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Bounded temporal truth. `Pending` is a distinct outcome, never a Boolean.
pub enum Truth {
    /// The decision evaluated to true.
    True,
    /// The decision evaluated to false.
    False,
    /// The decision has not yet settled to true or false.
    Pending,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Why a truth was settled, using the shared temporal settlement vocabulary.
pub enum Settlement {
    /// A complete closed decision scope authorized the closed-boundary rule.
    ClosedScope,
    /// An open scope settled true from a witness preserved by every admitted
    /// continuation.
    DecisiveWitness,
    /// An open scope settled false from a counterexample preserved by every
    /// admitted continuation.
    DecisiveCounterexample,
    /// An open future whose admitted continuations do not preserve a
    /// Boolean.
    Unsettled,
    /// A fact inside the decision-support set is missing, so the truth is
    /// not available.
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Why a result carries no truth value.
pub enum NonValueKind {
    /// No value is reported because activation was inactive.
    Inactive,
    /// No value is reported because activation could not be determined.
    ActivationUnknown,
    /// No value is reported because a required fact was missing from the
    /// decision support.
    Missing,
    /// No value is reported because evaluation was refused for the stated
    /// dimension or reason.
    Refused,
    /// No value is reported because the obligation was never activated.
    Unactivated,
}

#[derive(Clone, Debug)]
/// Canonical result bytes and their raw and semantic identities.
pub struct Document {
    bytes: Vec<u8>,
    digest: ByteDigest,
    wire: Wire,
    limits: Limits,
}

impl Document {
    /// Returns the canonical encoded result bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Returns the raw digest of the canonical bytes.
    pub const fn digest(&self) -> ByteDigest {
        self.digest
    }
    /// Returns the result's canonical content-addressed identity.
    pub fn identity(&self) -> &str {
        &self.wire.identity
    }
    /// Returns the monotonically increasing revision number within this
    /// result's lineage.
    pub const fn revision(&self) -> u64 {
        self.wire.revision
    }
}

#[derive(Clone, Debug)]
/// Constructor-private result admitted only through evaluation or strict re-evaluation.
///
/// ```compile_fail
/// use quire_spec_language::protocol_artifact::native_temporal::result::ValidatedResult;
/// let _ = ValidatedResult;
/// ```
pub struct ValidatedResult(Document);

impl ValidatedResult {
    /// Returns the underlying canonical document.
    pub fn document(&self) -> &Document {
        &self.0
    }
    /// Returns which lineage relation this result stands in relative to a
    /// predecessor.
    pub fn relation(&self) -> RelationKind {
        self.0.wire.relation.kind
    }
    /// Returns the identity of the request this result was evaluated from.
    pub fn request_identity(&self) -> &str {
        &self.0.wire.request.identity
    }
    /// Returns the identity of the observation subject this result concerns.
    pub fn subject_identity(&self) -> &str {
        &self.0.wire.subject_identity
    }
    /// Returns the semantic instance identity this result was evaluated
    /// against.
    pub fn instance(&self) -> &str {
        &self.0.wire.instance
    }
    /// Returns the evidence correspondence this result was bound to.
    pub fn correspondence(&self) -> &EvidenceRef {
        &self.0.wire.correspondence
    }
    /// Returns whether the governing trigger or execution origin was
    /// activated.
    pub fn activation(&self) -> ActivationState {
        self.0.wire.activation
    }
    /// Returns the assessment-execution disposition, independent of truth.
    pub fn execution(&self) -> temporal::Execution {
        self.0.wire.execution.into()
    }
    /// Returns the settled truth, or `None` when no truth value is reported.
    pub fn truth(&self) -> Option<Truth> {
        self.0.wire.truth
    }
    /// Returns why no truth value is reported, or `None` when a truth value
    /// is present.
    pub fn non_value(&self) -> Option<NonValueView<'_>> {
        self.0.wire.non_value.as_ref().map(NonValueView)
    }
    /// Returns why the truth was settled, or `None` when no truth value is
    /// reported.
    pub fn settlement(&self) -> Option<Settlement> {
        self.0.wire.settlement
    }
    /// Returns the identities of the trace positions whose valuations
    /// established the reported truth.
    pub fn support(&self) -> impl ExactSizeIterator<Item = &str> {
        self.0.wire.support.iter().map(String::as_str)
    }
    /// Returns the admitted decision-progress evidence and watermark.
    pub fn decision_progress(&self) -> ProgressView<'_> {
        ProgressView(&self.0.wire.axes.decision_progress)
    }
    /// Returns the admitted decision-closure evidence and state.
    pub fn decision_closure(&self) -> ClosureView<'_> {
        ClosureView(&self.0.wire.axes.decision_closure)
    }
    /// Returns the admitted surrounding-progress evidence and watermark.
    pub fn surrounding_progress(&self) -> ProgressView<'_> {
        ProgressView(&self.0.wire.axes.surrounding_progress)
    }
    /// Returns the admitted surrounding-closure evidence and state.
    pub fn surrounding_closure(&self) -> ClosureView<'_> {
        ClosureView(&self.0.wire.axes.surrounding_closure)
    }
    /// Returns the admitted completeness authority, state, and fact
    /// population.
    pub fn completeness(&self) -> CompletenessView<'_> {
        CompletenessView(&self.0.wire.completeness)
    }
    /// Returns the effective limits this result was evaluated and encoded
    /// under.
    pub fn limits(&self) -> Limits {
        self.0.limits
    }
    /// Returns the predecessor's identity and digest, or `None` for an
    /// original result with no predecessor.
    pub fn predecessor(&self) -> Option<(&str, &str)> {
        self.0
            .wire
            .relation
            .predecessor
            .as_ref()
            .map(|value| (value.identity.as_str(), value.digest.as_str()))
    }
    /// Returns the monotonically increasing lineage counter, incremented on
    /// each correction.
    pub const fn lineage(&self) -> u64 {
        self.0.wire.lineage
    }
}

#[derive(Clone, Copy)]
/// Read-only view over one progress axis's evidence reference and watermark.
pub struct ProgressView<'a>(&'a ProgressWire);
impl<'a> ProgressView<'a> {
    /// Returns the evidence reference this progress axis is bound to.
    pub fn reference(self) -> &'a EvidenceRef {
        &self.0.reference
    }
    /// Returns the progress watermark in the profile's clock domain.
    pub const fn watermark(self) -> i64 {
        self.0.watermark
    }
}

#[derive(Clone, Copy)]
/// Read-only view over one closure axis's evidence reference and state.
pub struct ClosureView<'a>(&'a ClosureRecord);
impl<'a> ClosureView<'a> {
    /// Returns the evidence reference this closure axis is bound to.
    pub fn reference(self) -> &'a EvidenceRef {
        &self.0.reference
    }
    /// Returns whether this closure axis is open or closed.
    pub fn state(self) -> temporal::Closure {
        match self.0.state {
            ClosureWire::Open => temporal::Closure::Open,
            ClosureWire::Closed => temporal::Closure::Closed,
        }
    }
}

#[derive(Clone, Copy)]
/// Read-only view over the admitted completeness authority, state, and fact
/// population.
pub struct CompletenessView<'a>(&'a CompletenessRecord);
impl<'a> CompletenessView<'a> {
    /// Returns the evidence reference this completeness authority is bound
    /// to.
    pub fn reference(self) -> &'a EvidenceRef {
        &self.0.reference
    }
    /// Returns whether the input is complete or incomplete.
    pub fn state(self) -> temporal::Completeness {
        match self.0.state {
            CompletenessWire::Complete => temporal::Completeness::Complete,
            CompletenessWire::Incomplete => temporal::Completeness::Incomplete,
        }
    }
    /// Returns the exact fact population this completeness authority
    /// covers.
    pub fn facts(self) -> impl ExactSizeIterator<Item = &'a EvidenceRef> {
        self.0.facts.iter()
    }
}

#[derive(Clone, Copy)]
/// Read-only view over why a result carries no truth value.
pub struct NonValueView<'a>(&'a NonValueWire);
impl<'a> NonValueView<'a> {
    /// Returns the category of reason no truth value is reported.
    pub const fn kind(self) -> NonValueKind {
        self.0.kind
    }
    /// Returns the exact machine-readable reason code.
    pub fn code(self) -> &'a str {
        &self.0.code
    }
    /// Returns the dimension the reason concerns, when one applies.
    pub fn dimension(self) -> Option<&'a str> {
        self.0.dimension.as_deref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ExecutionWire {
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
enum ClosureWire {
    Open,
    Closed,
}
impl From<temporal::Closure> for ClosureWire {
    fn from(value: temporal::Closure) -> Self {
        match value {
            temporal::Closure::Open => Self::Open,
            temporal::Closure::Closed => Self::Closed,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CompletenessWire {
    Complete,
    Incomplete,
}
impl From<temporal::Completeness> for CompletenessWire {
    fn from(value: temporal::Completeness) -> Self {
        match value {
            temporal::Completeness::Complete => Self::Complete,
            temporal::Completeness::Incomplete => Self::Incomplete,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestBindingWire {
    identity: String,
    digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PredecessorWire {
    identity: String,
    digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RelationWire {
    kind: RelationKind,
    predecessor: Option<PredecessorWire>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProgressWire {
    reference: EvidenceRef,
    watermark: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClosureRecord {
    reference: EvidenceRef,
    state: ClosureWire,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AxesWire {
    decision_progress: ProgressWire,
    decision_closure: ClosureRecord,
    surrounding_progress: ProgressWire,
    surrounding_closure: ClosureRecord,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompletenessRecord {
    reference: EvidenceRef,
    state: CompletenessWire,
    facts: Vec<EvidenceRef>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NonValueWire {
    kind: NonValueKind,
    code: String,
    dimension: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    contract: String,
    identity: String,
    revision: u64,
    relation: RelationWire,
    request: RequestBindingWire,
    subject_identity: String,
    instance: String,
    correspondence: EvidenceRef,
    activation: ActivationState,
    execution: ExecutionWire,
    truth: Option<Truth>,
    non_value: Option<NonValueWire>,
    settlement: Option<Settlement>,
    axes: AxesWire,
    completeness: CompletenessRecord,
    support: Vec<String>,
    limits: WireLimits,
    lineage: u64,
}

#[derive(Serialize)]
struct Preimage<'a> {
    contract: &'a str,
    revision: u64,
    relation: &'a RelationWire,
    request: &'a RequestBindingWire,
    subject_identity: &'a str,
    instance: &'a str,
    correspondence: &'a EvidenceRef,
    activation: ActivationState,
    execution: ExecutionWire,
    truth: Option<Truth>,
    non_value: &'a Option<NonValueWire>,
    settlement: Option<Settlement>,
    axes: &'a AxesWire,
    completeness: &'a CompletenessRecord,
    support: &'a [String],
    limits: WireLimits,
    lineage: u64,
}

fn preimage(wire: &Wire, limits: Limits) -> Result<Vec<u8>, Error> {
    encode(
        &Preimage {
            contract: &wire.contract,
            revision: wire.revision,
            relation: &wire.relation,
            request: &wire.request,
            subject_identity: &wire.subject_identity,
            instance: &wire.instance,
            correspondence: &wire.correspondence,
            activation: wire.activation,
            execution: wire.execution,
            truth: wire.truth,
            non_value: &wire.non_value,
            settlement: wire.settlement,
            axes: &wire.axes,
            completeness: &wire.completeness,
            support: &wire.support,
            limits: wire.limits,
            lineage: wire.lineage,
        },
        limits,
    )
}

fn dimension(value: temporal::Dimension) -> &'static str {
    use temporal::Dimension as D;
    match value {
        D::Profile => "profile",
        D::ProfileRevision => "profile_revision",
        D::Clock => "clock",
        D::SamplePeriod => "sample_period",
        D::Epoch => "epoch",
        D::TimestampUnit => "timestamp_unit",
        D::SequenceAuthority => "sequence_authority",
        D::ClockUnit => "clock_unit",
        D::ClockParameters => "clock_parameters",
        D::AdmittedOrder => "admitted_order",
        D::Interval => "interval",
        D::Capture => "capture",
        D::Guard => "guard",
        D::TriggerIdentity => "trigger_identity",
        D::Anchor => "anchor",
        D::Valuation => "valuation",
        D::History => "history",
        D::PastOperator => "past_operator",
        D::FiniteWindow => "finite_window",
        D::OpenPrefix => "open_prefix",
        D::Watermark => "watermark",
        D::Completeness => "completeness",
    }
}

fn refusal(value: &temporal::Refusal) -> (&'static str, Option<&'static str>) {
    match value {
        temporal::Refusal::Binding {
            dimension: value, ..
        } => ("binding", Some(dimension(*value))),
        temporal::Refusal::Order { .. } => ("order", Some("admitted_order")),
        temporal::Refusal::Capture {
            dimension: value, ..
        } => ("capture", Some(dimension(*value))),
        temporal::Refusal::Contradiction { .. } => ("contradiction", Some("trigger_identity")),
        temporal::Refusal::Progress {
            dimension: value, ..
        } => ("progress", Some(dimension(*value))),
        temporal::Refusal::Reference { .. } => ("reference", None),
    }
}

fn activation(value: temporal::Activation) -> ActivationState {
    match value {
        temporal::Activation::Inactive => ActivationState::Inactive,
        temporal::Activation::Unknown { .. } => ActivationState::Unknown,
        temporal::Activation::Active => ActivationState::Active,
    }
}

fn truth(value: temporal::Truth) -> Truth {
    match value {
        temporal::Truth::True => Truth::True,
        temporal::Truth::False => Truth::False,
        temporal::Truth::Pending => Truth::Pending,
    }
}

fn settlement(value: temporal::Basis) -> Settlement {
    match value {
        temporal::Basis::ClosedScope => Settlement::ClosedScope,
        temporal::Basis::DecisiveWitness => Settlement::DecisiveWitness,
        temporal::Basis::DecisiveCounterexample => Settlement::DecisiveCounterexample,
        temporal::Basis::Unsettled => Settlement::Unsettled,
        temporal::Basis::Unavailable => Settlement::Unavailable,
    }
}

type Outcome = (
    ActivationState,
    ExecutionWire,
    Option<Truth>,
    Option<NonValueWire>,
    Option<Settlement>,
    Vec<usize>,
);

fn outcome(report: &temporal::Report) -> Result<Outcome, Error> {
    match report.result() {
        Ok([temporal::Obligation::Assessed(assessment)]) => {
            let activation = activation(assessment.activation);
            let non_value = if let Some(incomplete) = &assessment.incomplete {
                Some(NonValueWire {
                    kind: NonValueKind::Missing,
                    code: "missing".into(),
                    dimension: Some(dimension(incomplete.dimension).into()),
                })
            } else if assessment.truth.is_none() {
                Some(NonValueWire {
                    kind: match assessment.activation {
                        temporal::Activation::Inactive => NonValueKind::Inactive,
                        _ => NonValueKind::ActivationUnknown,
                    },
                    code: match assessment.activation {
                        temporal::Activation::Inactive => "inactive",
                        _ => "activation_unknown",
                    }
                    .into(),
                    dimension: None,
                })
            } else {
                None
            };
            Ok((
                activation,
                assessment.premises.execution.into(),
                assessment.truth.map(truth),
                non_value,
                assessment.basis.map(settlement),
                assessment.support.clone(),
            ))
        }
        Ok([temporal::Obligation::Unactivated { error, .. }]) => {
            let (code, dimension) = match error {
                temporal::Error::Refused(value) => refusal(value),
                temporal::Error::Incomplete(value) => {
                    ("incomplete", Some(dimension(value.dimension)))
                }
                temporal::Error::Exhausted(_) => return Err(exhausted("evaluation")),
            };
            Ok((
                ActivationState::Unavailable,
                ExecutionWire::Failed,
                None,
                Some(NonValueWire {
                    kind: NonValueKind::Unactivated,
                    code: code.into(),
                    dimension: dimension.map(str::to_owned),
                }),
                None,
                Vec::new(),
            ))
        }
        Ok(_) => Err(invalid("evaluation.obligations")),
        Err(temporal::Error::Exhausted(_)) => Err(exhausted("evaluation")),
        Err(temporal::Error::Incomplete(value)) => Ok((
            ActivationState::Unavailable,
            ExecutionWire::Failed,
            None,
            Some(NonValueWire {
                kind: NonValueKind::Missing,
                code: "incomplete".into(),
                dimension: Some(dimension(value.dimension).into()),
            }),
            None,
            Vec::new(),
        )),
        Err(temporal::Error::Refused(value)) => {
            let (code, dimension) = refusal(value);
            Ok((
                ActivationState::Unavailable,
                ExecutionWire::Failed,
                None,
                Some(NonValueWire {
                    kind: NonValueKind::Refused,
                    code: code.into(),
                    dimension: dimension.map(str::to_owned),
                }),
                None,
                Vec::new(),
            ))
        }
    }
}

fn relation(
    value: Relation<'_>,
    request: &ValidatedRequest,
    limits: Limits,
) -> Result<(u64, RelationWire, u64), Error> {
    match value {
        Relation::Original => Ok((
            1,
            RelationWire {
                kind: RelationKind::Original,
                predecessor: None,
            },
            0,
        )),
        Relation::Superseding(predecessor) => {
            corrected_relation(predecessor, request, limits, RelationKind::Superseding)
        }
        Relation::Invalidating(predecessor) => {
            corrected_relation(predecessor, request, limits, RelationKind::Invalidating)
        }
    }
}

fn corrected_relation(
    predecessor: &ValidatedResult,
    request: &ValidatedRequest,
    limits: Limits,
    kind: RelationKind,
) -> Result<(u64, RelationWire, u64), Error> {
    if predecessor.request_identity() == request.document().identity()
        || predecessor.subject_identity() != request.subject_identity()
        || predecessor.instance() != request.instance()
        || predecessor.correspondence() != request.correspondence()
        || raw_digest(predecessor.document().bytes()) != predecessor.document().digest()
    {
        return Err(error(ErrorCode::InvalidRelation, "relation.predecessor"));
    }
    let revision = predecessor
        .document()
        .revision()
        .checked_add(1)
        .ok_or_else(|| error(ErrorCode::InvalidRelation, "revision"))?;
    let lineage = predecessor
        .lineage()
        .checked_add(1)
        .ok_or_else(|| exhausted("limits.lineage"))?;
    if lineage > u64::try_from(limits.lineage).map_err(|_| exhausted("limits.lineage"))? {
        return Err(exhausted("limits.lineage"));
    }
    Ok((
        revision,
        RelationWire {
            kind,
            predecessor: Some(PredecessorWire {
                identity: predecessor.document().identity().into(),
                digest: format!("{:x}", predecessor.document().digest()),
            }),
        },
        lineage,
    ))
}

fn build(
    request: &ValidatedRequest,
    relation_input: Relation<'_>,
    limits: Limits,
    usage: &mut Usage,
) -> Result<Wire, Error> {
    let limits = limits.bounded();
    if limits != request.limits() {
        return Err(invalid("limits.request"));
    }
    if raw_digest(request.document().bytes()) != request.document().digest() {
        return Err(invalid("request.digest"));
    }
    let (revision, relation, lineage) = relation(relation_input, request, limits)?;
    let evaluated = temporal::evaluate_v2(
        request.subject_package(),
        usize::try_from(request.subject_declaration())
            .map_err(|_| invalid("subject.declaration"))?,
        request.trace(),
        limits.evaluation(),
    );
    usage.evaluation = evaluated.usage();
    let (activation, execution, truth, non_value, settlement, indices) = outcome(&evaluated)?;
    if indices.len() > limits.support {
        return Err(exhausted("limits.support"));
    }
    let positions: Vec<_> = request
        .positions()
        .map(|position| position.identity())
        .collect();
    let mut support = Vec::new();
    support
        .try_reserve(indices.len())
        .map_err(|_| error(ErrorCode::Allocation, "support"))?;
    for index in indices {
        support.push(
            positions
                .get(index)
                .ok_or_else(|| invalid("support"))?
                .to_string(),
        );
    }
    usage.support = support.len();
    usage.lineage = usize::try_from(lineage).map_err(|_| exhausted("limits.lineage"))?;
    let decision_progress = request.decision_progress();
    let decision_closure = request.decision_closure();
    let surrounding_progress = request.surrounding_progress();
    let surrounding_closure = request.surrounding_closure();
    let completeness = request.completeness();
    let mut wire = Wire {
        contract: CONTRACT.into(),
        identity: String::new(),
        revision,
        relation,
        request: RequestBindingWire {
            identity: request.document().identity().into(),
            digest: format!("{:x}", request.document().digest()),
        },
        subject_identity: request.subject_identity().into(),
        instance: request.instance().into(),
        correspondence: request.correspondence().clone(),
        activation,
        execution,
        truth,
        non_value,
        settlement,
        axes: AxesWire {
            decision_progress: ProgressWire {
                reference: decision_progress.reference().clone(),
                watermark: decision_progress.watermark(),
            },
            decision_closure: ClosureRecord {
                reference: decision_closure.reference().clone(),
                state: decision_closure.state().into(),
            },
            surrounding_progress: ProgressWire {
                reference: surrounding_progress.reference().clone(),
                watermark: surrounding_progress.watermark(),
            },
            surrounding_closure: ClosureRecord {
                reference: surrounding_closure.reference().clone(),
                state: surrounding_closure.state().into(),
            },
        },
        completeness: CompletenessRecord {
            reference: completeness.reference().clone(),
            state: completeness.state().into(),
            facts: completeness.facts().cloned().collect(),
        },
        support,
        limits: limits.try_into()?,
        lineage,
    };
    wire.identity = identity(CONTRACT, &preimage(&wire, limits)?);
    Ok(wire)
}

fn finish(wire: Wire, limits: Limits, usage: &mut Usage) -> Result<Document, Error> {
    let bytes = encode(&wire, limits)?;
    usage.output_bytes = bytes.len();
    Ok(Document {
        digest: raw_digest(&bytes),
        bytes,
        wire,
        limits,
    })
}

/// Evaluates the exact validated request and emits one formula-wide owner result.
pub fn evaluate(
    request: &ValidatedRequest,
    relation: Relation<'_>,
    limits: Limits,
) -> Report<Document> {
    let limits = limits.bounded();
    let mut usage = Usage::default();
    let result = build(request, relation, limits, &mut usage)
        .and_then(|wire| finish(wire, limits, &mut usage));
    report(limits, usage, result)
}

/// Strictly reads by re-evaluating the exact request and relation before byte comparison.
pub fn read(
    bytes: &[u8],
    request: &ValidatedRequest,
    relation: Relation<'_>,
    limits: Limits,
) -> Report<ValidatedResult> {
    let limits = limits.bounded();
    let mut usage = Usage::default();
    let result = (|| {
        let offered: Wire = decode(bytes, limits, &mut usage)?;
        if offered.contract != CONTRACT {
            return Err(invalid("contract"));
        }
        validate_digest(&offered.identity, "identity")?;
        let expected = build(request, relation, limits, &mut usage)?;
        if offered != expected
            || offered.identity != identity(CONTRACT, &preimage(&offered, limits)?)
        {
            return Err(error(ErrorCode::NonCanonical, "document"));
        }
        let document = finish(expected, limits, &mut usage)?;
        if document.bytes() != bytes {
            return Err(error(ErrorCode::NonCanonical, "document"));
        }
        Ok(ValidatedResult(document))
    })();
    report(limits, usage, result)
}
