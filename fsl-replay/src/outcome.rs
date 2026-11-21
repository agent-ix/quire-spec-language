// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-286 (ADR-029 CB-4): the one `filament-outcome/1` JSON outcome document
//! every FSL lifecycle operation's outcome serializes to, and its one
//! serializer.
//!
//! A driver writes a document with a constructor over the operation's own
//! outcome ([`OutcomeDocument::from_parse`], [`OutcomeDocument::from_format`],
//! [`OutcomeDocument::from_check`],
//! [`OutcomeDocument::from_package`],
//! [`OutcomeDocument::from_replay`]), or, for the operations whose outcome
//! types live above this crate (`prove`, `analyze`, `monitor`), with
//! [`OutcomeDocument::settled`] over their items; [`OutcomeDocument::to_bytes`]
//! writes it. The CLI in machine mode writes exactly these bytes, so the
//! library and the CLI give one document for one request (FSpec
//! FR-300-AC-3).
//!
//! The serializer is pure: it runs no stage and reads no clock. The document
//! is a typed value, so its members are the types below, not keys inserted by
//! hand, and the same value serializes to the same bytes every time. Every
//! cause spelling is its typed cause's own. A proof item's label type has no
//! `undefined` member, so an undefined claim evaluation can only be written as
//! a `refuted` item with the `undefined-evaluation` cause and category
//! violation; the label `undefined` appears only as the category of a
//! non-proof evaluation outcome such as `execute`.

mod counterexample;
pub mod hint;
pub use counterexample::{
    assignments_wire, graph_evidence_wire, hyper_counterexample_wire, probabilistic_undefined_wire,
    probabilistic_wire, protocol_temporal_wire, read_task_temporal_counterexample,
    read_task_temporal_document, read_temporal_counterexample, replay_model_trace_document,
    replay_task_product_trace, sampled_witness_wire, state_key_wire, task_product_transition_wire,
    task_source_transition_wire, task_temporal_wire, task_temporal_wire_with_queue_growth,
    task_transition_wire, temporal_wire, CounterexampleWireError, ProtocolWire, ReadCounterexample,
    TaskProductCounterexampleDocument, TaskProductTraceReplayRefusal,
    TemporalCounterexampleReadRefusal, TraceAlphabet,
};

use crate::hybrid::{BoundDimension, SolverHorizon};
use filament_exact::{CancelCause, Integer, NodeKey};
use filament_semantic_value::catalog::Category;
use filament_semantic_value::location::ExpressionLocation;
use fsl_cst::format::FormatRefusal;
use fsl_foundation::diagnostic::{Code, Locus, StageFailure, Staged, Stopped};
use fsl_foundation::source::provenance::{OccurrenceKey, RawSourceRef};
use fsl_foundation::source::{SourceLabel, SourceReadCause};
use fsl_foundation::{LocatedSpan, RequestIndex};
use fsl_semantics::model::observation::AdmissionFailure;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};

use crate::certificate::{CertificateLocus, CertificateRule};
use crate::model_check::state_graph::WitnessSource;
use crate::proof_result::{
    Certification, DeclineCode, IncompleteCause, InconclusiveCause, InstanceBound, ProofBasis,
    ReportedInconclusiveCause, StaleIdentity, SymmetryBrokenCause, TerminalRecord, TerminalValue,
    UnavailabilityCause, WeakAccessFailure,
};
use crate::result::{DisagreementCause, ReplayResult};
use crate::spine::kernel_undefined_reason;
use crate::spine::{
    classify_clause_run, item_record, CallIncomplete, CallOutcome, CallRefusal, CheckedUnit,
    ClauseDisposition, ClauseRunReport, CompileRefusal, EmittedUnit, FrontEndFailure,
    ParsedUnitSource, ReasonWire, RunRefusal, SpineStage, TemporalUndefinedCause, TemporalVerdict,
};
use crate::statistical::{self, Decision, StatisticalVerdict};
use crate::ReplayRefusal;
use filament_semantic_value::fairness::PropertyForm;
use fsl_eval::temporal::UndefinedCause;

/// The format refusal projection consumed by the replay outcome facade.
///
/// This keeps the public facade signature in the replay vocabulary while the
/// conversion below preserves the formatter's typed diagnostic.
#[doc(hidden)]
pub struct FormatOutcomeRefusal {
    category: Category,
    diagnostic: OutcomeDiagnostic,
}

impl From<&FormatRefusal> for FormatOutcomeRefusal {
    fn from(refusal: &FormatRefusal) -> Self {
        let diagnostic = refusal.diagnostic();
        Self {
            category: diagnostic.code.category(),
            diagnostic: OutcomeDiagnostic::project_source_diagnostic(diagnostic),
        }
    }
}

/// The document's `format` member.
pub const OUTCOME_FORMAT: &str = "filament-outcome/1";
/// FR-286's published draft 2020-12 schema for `filament-outcome/1`.
pub const OUTCOME_SCHEMA_1: &str = include_str!("../schemas/filament-outcome-1.schema.json");

/// The library operation that produced the document (ADR-029 LC-1).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    /// `parse`.
    Parse,
    /// `format`.
    Format,
    /// `select`.
    Select,
    /// `check`.
    Check,
    /// `check_fences`.
    CheckFences,
    /// `package`.
    Package,
    /// `execute`.
    Execute,
    /// `monitor`.
    Monitor,
    /// `analyze`.
    Analyze,
    /// `lower`.
    Lower,
    /// `generate`.
    Generate,
    /// `prove`.
    Prove,
    /// `replay`.
    Replay,
    /// `inspect`.
    Inspect,
    /// `render`.
    Render,
    /// `export`.
    Export,
}

/// An ADR-011 stage, the last one an operation reached.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum OutcomeStage {
    /// S0, source bytes.
    S0,
    /// S1, the lossless CST.
    S1,
    /// S2, parsed semantic forms.
    S2,
    /// I1, domain-package intake.
    I1,
    /// S3, the checked semantic graph.
    S3,
    /// S4, the linked checked package.
    S4,
    /// S5, contract IR.
    S5,
    /// S6a, reference execution.
    S6a,
    /// S6b, bounded proof.
    S6b,
    /// S6c, native model checking.
    S6c,
    /// S7, the typed witness.
    S7,
    /// S8, native replay.
    S8,
}

impl From<SpineStage> for OutcomeStage {
    fn from(stage: SpineStage) -> Self {
        match stage {
            SpineStage::Source => Self::S1,
            SpineStage::Forms => Self::S2,
            SpineStage::Intake | SpineStage::Compile => Self::I1,
            SpineStage::Assembly | SpineStage::Check => Self::S3,
            SpineStage::Emit => Self::S4,
        }
    }
}

/// An item's result label. The FR-331 terminal labels come from a terminal
/// record ([`ItemLabel::of`]) and never include `undefined`: an undefined
/// claim evaluation is `refuted` with the `undefined-evaluation` cause
/// ([`ItemCause::undefined_evaluation`]). `pending`, `indeterminate` and
/// `undefined` are the catalog's category labels (FR-286 "Monitor documents
/// from clause runs"), written only on an item whose category is that
/// category, which only a clause run gives
/// ([`OutcomeItem::from_clause_run`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemLabel {
    /// The claim was proved.
    Proved,
    /// The claim was tested, never promoted to proved.
    Tested,
    /// The claim does not hold.
    Refuted,
    /// The request was declined.
    Declined,
    /// No engine or backend supplies the claim.
    Unsupported,
    /// The run did not complete.
    Incomplete,
    /// Neither proved nor refuted, a vacuous proof included.
    Inconclusive,
    /// The tool failed.
    Failed,
    /// A completed statistical run's measurement (FR-192), never proof
    /// evidence.
    Measured,
    /// A clause still pending when its supplied trace ends (category
    /// pending).
    Pending,
    /// A clause whose finite-window evidence cannot decide it (category
    /// indeterminate).
    Indeterminate,
    /// A clause with insufficient data (category undefined, FR-283).
    Undefined,
}

impl ItemLabel {
    /// The result label `value` settles. A vacuous proof is `inconclusive`,
    /// never `proved`.
    pub fn of(value: &TerminalValue) -> Self {
        match value {
            TerminalValue::Proved {
                basis: ProofBasis::Checks { success_checks: 0 },
                ..
            }
            | TerminalValue::Inconclusive(_) => Self::Inconclusive,
            TerminalValue::Proved { .. } => Self::Proved,
            TerminalValue::Tested => Self::Tested,
            TerminalValue::Refuted => Self::Refuted,
            TerminalValue::Declined { .. } => Self::Declined,
            TerminalValue::Unsupported(_) => Self::Unsupported,
            TerminalValue::Incomplete(_) => Self::Incomplete,
            TerminalValue::Failed | TerminalValue::ToolFailed(_) => Self::Failed,
            TerminalValue::Measured(_) => Self::Measured,
        }
    }
}

/// Why a clause run is not an outcome item.
#[derive(Debug, thiserror::Error)]
pub enum ClauseRunNotAnItem {
    /// The run's category has no item: an evaluator fault fails the whole
    /// request (FR-283), and no clause run settles inconclusive.
    #[error("a clause run of category {} is not an outcome item", .0.as_str())]
    Category(Category),
    /// The run's clause-run record could not be written.
    #[error("the clause-run record could not be written: {0}")]
    Record(#[from] crate::spine::NativeRunResultError),
}

/// Why an `analyze` or `prove` record is not an outcome item (FR-286).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum OutcomeItemError {
    /// A `refuted` record arrived without its one `ItemCounterexample`.
    #[error("a refuted record needs its one counterexample")]
    MissingCounterexample,
    /// The record's counterexample has no wire spelling.
    #[error(transparent)]
    Unspelled(#[from] CounterexampleWireError),
}

/// The `type` of a `hybrid-trace` disagreement (FR-450 "Settlement").
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HybridDisagreementType {
    #[serde(rename = "type")]
    kind: &'static str,
}

/// Why a replay settled inconclusive: its disagreement reason and the two
/// verdicts' categories.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ParityReason {
    reason: &'static str,
    /// A `hybrid-trace` reason's FR-450 disagreement: why exact replay did
    /// not reproduce the provider's counterexample.
    #[serde(skip_serializing_if = "Option::is_none")]
    disagreement: Option<HybridDisagreementType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proved: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replayed: Option<&'static str>,
}

/// A checked-package node id on the wire: the structural
/// `{"package", "path", "ordinal"}` object (FSpec FR-322), a path segment
/// being a string or a nested node id.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireNode(NodeKey);

impl WireNode {
    pub(crate) fn of(key: &NodeKey) -> Self {
        Self(key.clone())
    }
}

impl WireNode {
    #[cfg(test)]
    pub(crate) fn into_key(self) -> NodeKey {
        self.0
    }
}

impl<'de> serde::Deserialize<'de> for WireNode {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // `discard_node_json` drops the value iteratively, so a deep key
        // meets no recursion limit on the way out.
        let value = <serde_json::Value as serde::Deserialize>::deserialize(deserializer)?;
        let key = fsl_foundation::node_wire::node_key_from_json(&value);
        fsl_foundation::node_wire::discard_node_json(value);
        key.map(Self).map_err(serde::de::Error::custom)
    }
}

impl Serialize for WireNode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let value = fsl_foundation::node_wire::node_key_to_json(&self.0)
            .map_err(serde::ser::Error::custom)?;
        let written = Serialize::serialize(&value, serializer);
        fsl_foundation::node_wire::discard_node_json(value);
        written
    }
}

/// A claim's checked-package `{node, occurrence}` locus on the wire (FSpec
/// FR-331's Locus); `node` is the claim's declaration `NodeRef`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct WireLocus {
    node: WireNode,
    occurrence: WireOccurrence,
}

/// A node occurrence's `{role, ordinal}`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct WireOccurrence {
    role: String,
    ordinal: u64,
}

impl WireLocus {
    fn of(claim: &fsl_foundation::source::provenance::OccurrenceKey) -> Self {
        Self {
            node: WireNode::of(claim.node()),
            occurrence: WireOccurrence {
                role: claim.origin().role().as_str().to_owned(),
                ordinal: claim.origin().ordinal(),
            },
        }
    }
}

/// The FSpec FR-331 `{node, occurrence}` locus of `claim`, the spelling a
/// request item's `claim` member takes.
///
/// # Errors
///
/// When the node key has no wire spelling.
pub fn claim_locus_wire(
    claim: &fsl_foundation::source::provenance::OccurrenceKey,
) -> Result<serde_json::Value, serde_json::Error> {
    serde_json::to_value(WireLocus::of(claim))
}

/// FSpec FR-412's structural `CertificateIdentity` `{node_id, model,
/// ordinal}` of the obligation `claim` names over the subject model
/// `model`: `node_id` the claim's node, `model` the owning model's identity
/// text and `ordinal` the claim
/// occurrence's ordinal (FSpec FR-418-AC-9), the identity
/// [`crate::ObligationIdentity`] mints for a claim occurrence.
///
/// # Errors
///
/// When the node key has no wire spelling.
pub fn certificate_identity_wire(
    claim: &fsl_foundation::source::provenance::OccurrenceKey,
    model: &str,
) -> Result<serde_json::Value, serde_json::Error> {
    #[derive(Serialize)]
    struct Identity<'a> {
        node_id: WireNode,
        model: &'a str,
        ordinal: u32,
    }
    let identity = crate::ObligationIdentity::from_claim_occurrence(claim);
    serde_json::to_value(Identity {
        node_id: WireNode::of(identity.node()),
        model,
        ordinal: identity.ordinal(),
    })
}

/// The value of an item cause's `limit` member.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
enum CauseLimit {
    Name(&'static str),
    Bytes(u64),
}

/// The FR-867 `where` member: a legacy textual position or a checked
/// expression location with its declaration and child path.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
enum ItemWhere {
    Text(String),
    Expression(serde_json::Value),
}

/// The diagnostic a disposition-settled item carries: its catalog code and
/// cause, and its `detail` keyed by FR-272 payload names (FSpec FR-331).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ItemDiagnostic {
    code: &'static str,
    cause: &'static str,
    detail: serde_json::Map<String, serde_json::Value>,
}

/// The typed cause an item's terminal record carries: its FR-331 spelling in
/// `kind`, and the members that spelling names.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ItemCause {
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<&'static str>,
    /// A `bounded-search-incomplete` cause's reason.
    #[serde(rename = "reason", skip_serializing_if = "Option::is_none")]
    search_reason: Option<&'static str>,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    at: Option<ItemWhere>,
    /// A `timed-form-shape` cause's claim: its checked-package
    /// `{declaration, occurrence}` (FR-498).
    #[serde(skip_serializing_if = "Option::is_none")]
    locus: Option<ItemLocus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cause: Option<String>,
    /// A refusal's action-oriented explanation, when the source provides one.
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parity: Option<ParityReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    depth: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    horizon: Option<SolverHorizon>,
    #[serde(skip_serializing_if = "Option::is_none")]
    jumps: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exhaustive: Option<BTreeSet<BoundDimension>>,
    #[serde(
        serialize_with = "cancel_source",
        skip_serializing_if = "Option::is_none"
    )]
    source: Option<CancelCause>,
    /// A `hypotheses-not-established` cause's invariant clause node.
    #[serde(skip_serializing_if = "Option::is_none")]
    hypothesis: Option<WireNode>,
    /// A `hypotheses-not-established` cause's hypothesis item result.
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<ItemLabel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rule: Option<CertificateRule>,
    #[serde(rename = "at", skip_serializing_if = "Option::is_none")]
    certificate_at: Option<CertificateLocus>,
    /// The `limit` member: a limit's name, or a byte-limit `tool-failure`
    /// cause's configured maximum. One field, so the two cannot collide.
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<CauseLimit>,
    /// A `tool-failure` cause's FR-462 `defect`.
    #[serde(skip_serializing_if = "Option::is_none")]
    defect: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<u64>,
    /// An `insufficient-data` cause's window count.
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u64>,
    /// An `insufficient-data` cause's `min_count`.
    #[serde(skip_serializing_if = "Option::is_none")]
    min_count: Option<u64>,
    /// An accounting limit's attempted counter value, as a decimal string.
    #[serde(skip_serializing_if = "Option::is_none")]
    counter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lower: Option<crate::proof_result::ExactRational>,
    #[serde(skip_serializing_if = "Option::is_none")]
    upper: Option<crate::proof_result::ExactRational>,
    #[serde(skip_serializing_if = "Option::is_none")]
    setting: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    boundary_states: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_depth: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reduction: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    form: Option<WireForm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    population: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    initial: Option<u64>,
    /// The generator's moved keys, each `[from, to]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    generator: Option<Vec<(String, String)>>,
    /// A `well-definedness-unchecked` cause's witness source per initial
    /// state, in initial-state order (FSpec FR-391).
    #[serde(skip_serializing_if = "Option::is_none")]
    sources: Option<Vec<WitnessSource>>,
    /// The EN-4 refusal an `unsupported-requested-capability` cause names
    /// (FR-192).
    #[serde(skip_serializing_if = "Option::is_none")]
    refusal: Option<&'static str>,
    /// A `total-above-draw-range` cause's scaled total, as a decimal string
    /// (FR-189-AC-7).
    #[serde(skip_serializing_if = "Option::is_none")]
    total: Option<String>,
    /// A `memory-bound-reached` or `instance-bound-reached` cause's request
    /// member (FSpec FR-439, FR-428).
    #[serde(skip_serializing_if = "Option::is_none")]
    bound: Option<String>,
    /// An accounting limit's `filament.value.accounting/v1` field.
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<&'static str>,
    /// A `memory-bound-reached` or `instance-bound-reached` cause's count of
    /// limited states.
    #[serde(skip_serializing_if = "Option::is_none")]
    states: Option<ItemCauseStates>,
    /// Every `weak-access-shape` failure, with its shape written as JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    failures: Option<Vec<ItemWeakAccessFailure>>,
    /// A `recovery-view` cause's template, by its qualified name.
    #[serde(skip_serializing_if = "Option::is_none")]
    predicate: Option<String>,
    /// An unsupported delay distribution's operation identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    operation: Option<String>,
    /// The requested tagged identity of `stale-dependency`.
    #[serde(skip_serializing_if = "Option::is_none")]
    requested: Option<StaleIdentityWire>,
    /// The current tagged identity of `stale-dependency`.
    #[serde(skip_serializing_if = "Option::is_none")]
    recompiled: Option<StaleIdentityWire>,
    /// The timed state named by FR-331's sampling refusal causes.
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<TimedStateWire>,
    /// The typed transition identity named by `not-stochastic`.
    #[serde(skip_serializing_if = "Option::is_none")]
    identity: Option<TimedTransitionWire>,
    /// A clause run's FR-501 indeterminate reason, in the result report's
    /// own spelling; it supplies the cause's `kind`.
    #[serde(flatten)]
    reason: Option<ReasonWire>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
enum ItemLocus {
    Claim(WireLocus),
    Outcome(OutcomeLocus),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
enum ItemCauseStates {
    Count(u64),
    Cycle(Vec<String>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct TimedStateWire {
    key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct TimedTransitionWire {
    #[serde(rename = "type")]
    kind: &'static str,
    operation: serde_json::Value,
    receiver: Option<serde_json::Value>,
    arguments: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct TaskSourceTransitionWire {
    #[serde(rename = "type")]
    kind: &'static str,
    operation: String,
    receiver: Option<serde_json::Value>,
    arguments: Vec<serde_json::Value>,
}

impl From<&filament_semantic_value::model_transition::ModelTransition>
    for TaskSourceTransitionWire
{
    fn from(identity: &filament_semantic_value::model_transition::ModelTransition) -> Self {
        let operation: serde_json::Value = serde_json::from_str(identity.operation())
            .expect("canonical source operation identity");
        let operation = operation
            .as_str()
            .or_else(|| operation.get("name").and_then(serde_json::Value::as_str))
            // Checked model transitions carry the canonical node identity,
            // not the authored operation spelling. FR-364's source arm
            // keeps that stable identity as its operation string.
            .or_else(|| operation.get("node").and_then(serde_json::Value::as_str))
            .map(str::to_owned)
            // ModelTransition validates canonical JSON, not a particular
            // operation-identity object shape. Preserve any other valid
            // canonical identity instead of panicking on it.
            .unwrap_or_else(|| identity.operation().to_owned());
        let value = |text: &str| serde_json::from_str(text).expect("canonical typed value");
        Self {
            kind: "transition",
            operation,
            receiver: identity.receiver().map(value),
            arguments: identity
                .arguments()
                .iter()
                .map(|text| value(text))
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct TaskTransitionWire {
    #[serde(rename = "type")]
    kind: &'static str,
    action: &'static str,
    taskset: String,
    task: String,
    job: u64,
    slot: u64,
    generation: u64,
    release: TaskSourceTransitionWire,
    source_transition: Option<TaskSourceTransitionWire>,
    demand: serde_json::Value,
    jitter: serde_json::Value,
    deadline: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    budget_delta: Option<serde_json::Value>,
}

fn task_rational(value: &filament_exact::Rational) -> serde_json::Value {
    serde_json::json!({
        "numerator": value.numerator().to_string(),
        "denominator": value.denominator().to_string(),
    })
}

impl From<&crate::TaskTransitionIdentity> for TaskTransitionWire {
    fn from(identity: &crate::TaskTransitionIdentity) -> Self {
        use crate::TaskTransitionAction as Action;
        let action = match identity.action {
            Action::Activate => "activate",
            Action::Release => "release",
            Action::Dispatch => "dispatch",
            Action::Preempt => "preempt",
            Action::Resume => "resume",
            Action::Complete => "complete",
            Action::BudgetChange => "budget-change",
            Action::DeadlineMiss => "deadline-miss",
        };
        Self {
            kind: "task-transition",
            action,
            taskset: identity.task_set.clone(),
            task: identity.task.clone(),
            job: identity.job,
            slot: identity.slot,
            generation: identity.generation,
            release: TaskSourceTransitionWire::from(&identity.release),
            source_transition: identity
                .source_transition
                .as_ref()
                .map(TaskSourceTransitionWire::from),
            demand: task_rational(&identity.demand),
            jitter: task_rational(&identity.jitter),
            deadline: task_rational(&identity.deadline),
            priority: identity.priority,
            budget_delta: identity.budget_delta.as_ref().map(task_rational),
        }
    }
}

impl From<&fsl_eval::simulation::StateKey> for TimedStateWire {
    fn from(key: &fsl_eval::simulation::StateKey) -> Self {
        use base64::Engine;
        Self {
            key: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key.as_bytes()),
        }
    }
}

impl From<&filament_semantic_value::model_transition::ModelTransition> for TimedTransitionWire {
    fn from(identity: &filament_semantic_value::model_transition::ModelTransition) -> Self {
        let value = |text: &str| serde_json::from_str(text).expect("canonical typed value");
        Self {
            kind: "transition",
            operation: value(identity.operation()),
            receiver: identity.receiver().map(value),
            arguments: identity
                .arguments()
                .iter()
                .map(|text| value(text))
                .collect(),
        }
    }
}

/// A stale identity on the terminal wire. The explicit discriminator keeps
/// the identity kinds distinct while sharing one cause shape.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum StaleIdentityWire {
    Refinement { name: String },
}

impl From<&StaleIdentity> for StaleIdentityWire {
    fn from(identity: &StaleIdentity) -> Self {
        match identity {
            StaleIdentity::Refinement(name) => Self::Refinement { name: name.clone() },
        }
    }
}

/// One `weak-access-shape` failure in an item cause.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ItemWeakAccessFailure {
    site: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    transition: Option<String>,
    #[serde(serialize_with = "raw_json")]
    shape: Option<String>,
}

impl From<&WeakAccessFailure> for ItemWeakAccessFailure {
    fn from(failure: &WeakAccessFailure) -> Self {
        Self {
            site: failure.site.clone(),
            transition: failure.transition.clone(),
            shape: Some(failure.shape.clone()),
        }
    }
}

/// `text`, canonical JSON, written as the JSON value it spells; text that is
/// not JSON fails the serialization.
fn raw_json<S: Serializer>(text: &Option<String>, serializer: S) -> Result<S::Ok, S::Error> {
    match text {
        Some(text) => serde_json::value::RawValue::from_string(text.clone())
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer),
        None => serializer.serialize_none(),
    }
}

/// A property form on the outcome wire, spelled as filament-semantic-value
/// FR-121 spells it: a string for the four fieldless forms and
/// `{"refinement":{"liveness":<bool>}}` for a refinement, which keeps its own
/// identity and its liveness flag.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
enum WireForm {
    Named(&'static str),
    Refinement { refinement: RefinementForm },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct RefinementForm {
    liveness: bool,
}

fn wire_form(form: PropertyForm) -> WireForm {
    match form {
        PropertyForm::ReachableInvariant => WireForm::Named("reachable-invariant"),
        PropertyForm::BoundedMltl => WireForm::Named("bounded-mltl"),
        PropertyForm::Safety => WireForm::Named("safety"),
        PropertyForm::Liveness => WireForm::Named("liveness"),
        PropertyForm::Refinement { liveness } => WireForm::Refinement {
            refinement: RefinementForm { liveness },
        },
    }
}

fn cancel_source<S: Serializer>(
    source: &Option<CancelCause>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match source {
        Some(CancelCause::Requested) => serializer.serialize_str("requested"),
        Some(CancelCause::Deadline) => serializer.serialize_str("deadline"),
        None => serializer.serialize_none(),
    }
}

impl ItemCause {
    fn kind(kind: &'static str) -> Self {
        Self {
            kind: Some(kind),
            search_reason: None,
            at: None,
            locus: None,
            code: None,
            cause: None,
            message: None,
            parity: None,
            depth: None,
            horizon: None,
            jumps: None,
            exhaustive: None,
            source: None,
            rule: None,
            hypothesis: None,
            result: None,
            certificate_at: None,
            limit: None,
            defect: None,
            value: None,
            count: None,
            min_count: None,
            counter: None,
            lower: None,
            upper: None,
            setting: None,
            boundary_states: None,
            max_depth: None,
            reduction: None,
            form: None,
            population: None,
            initial: None,
            generator: None,
            sources: None,
            refusal: None,
            total: None,
            bound: None,
            field: None,
            states: None,
            failures: None,
            predicate: None,
            operation: None,
            requested: None,
            recompiled: None,
            state: None,
            identity: None,
            reason: None,
        }
    }

    /// The claim's evaluation was undefined at `at`, for `cause`: the item
    /// is refuted, category violation (FR-281, FR-283).
    pub fn undefined_evaluation(at: impl Into<String>, cause: impl Into<String>) -> Self {
        Self {
            at: Some(ItemWhere::Text(at.into())),
            cause: Some(cause.into()),
            ..Self::kind("undefined-evaluation")
        }
    }

    fn undefined_evaluation_at(location: &ExpressionLocation, cause: impl Into<String>) -> Self {
        Self {
            at: Some(ItemWhere::Expression(
                crate::result::expression_location_json(location),
            )),
            cause: Some(cause.into()),
            ..Self::kind("undefined-evaluation")
        }
    }

    fn inconclusive(cause: &ReportedInconclusiveCause) -> Self {
        let mut item = Self::kind(cause.as_str());
        match cause {
            ReportedInconclusiveCause::Cause(InconclusiveCause::CertificateRejected {
                rule,
                at,
            }) => {
                item.rule = Some(*rule);
                item.certificate_at = Some(at.clone());
            }
            ReportedInconclusiveCause::Cause(InconclusiveCause::InsufficientData {
                count,
                min_count,
            }) => {
                item.count = Some(*count);
                item.min_count = Some(*min_count);
            }
            ReportedInconclusiveCause::Cause(InconclusiveCause::HypothesesNotEstablished {
                hypothesis,
                result,
            }) => {
                item.hypothesis = Some(WireNode::of(hypothesis));
                item.result = Some(*result);
            }
            ReportedInconclusiveCause::Cause(
                InconclusiveCause::BoundReached { depth }
                | InconclusiveCause::InductionNotClosed { depth },
            ) => {
                item.depth = Some(*depth);
            }
            ReportedInconclusiveCause::Cause(InconclusiveCause::InsufficientHorizon {
                horizon,
                jumps,
                exhaustive,
            }) => {
                item.horizon = Some(horizon.clone());
                item.jumps = Some(*jumps);
                item.exhaustive = Some(exhaustive.clone());
            }
            ReportedInconclusiveCause::Cause(
                InconclusiveCause::UndecidedSuccessor
                | InconclusiveCause::OpenHistory
                | InconclusiveCause::NoInitialState
                | InconclusiveCause::NoInstance
                | InconclusiveCause::TemporalPending
                | InconclusiveCause::TemporalIndeterminate
                | InconclusiveCause::KaniVacuousProof
                | InconclusiveCause::RefutedIgnoringConstraints
                | InconclusiveCause::MatchUndetermined
                | InconclusiveCause::VacuousMatch
                | InconclusiveCause::MappingUndetermined
                | InconclusiveCause::ProtocolAssumptionNotEstablished(_)
                | InconclusiveCause::SufficientTestFailed
                | InconclusiveCause::NoAdmittedBehaviour
                | InconclusiveCause::LassoNotConcretized
                | InconclusiveCause::SolverInconclusive { .. }
                // Planner-only causes: the planner result writes their payload.
                | InconclusiveCause::PremiseUnproved { .. }
                | InconclusiveCause::TransferNotPreserving { .. }
                | InconclusiveCause::ProtocolAnalysis(_),
            ) => {}
            ReportedInconclusiveCause::Cause(InconclusiveCause::ReductionNotPreserving {
                reduction,
                form,
            }) => {
                item.reduction = Some(reduction.as_str());
                item.form = Some(wire_form(*form));
            }
            ReportedInconclusiveCause::Cause(
                InconclusiveCause::StateGraphReductionNotPreserving { reduction, form },
            ) => {
                item.reduction = Some(reduction.as_str());
                item.form = Some(WireForm::Named(form.as_str()));
            }
            ReportedInconclusiveCause::Cause(InconclusiveCause::SymmetryBroken {
                population,
                cause: SymmetryBrokenCause::InitialStatesNotClosed { initial, generator },
            }) => {
                item.population = Some(population.node.clone());
                item.initial = Some(*initial);
                item.generator = Some(
                    generator
                        .maps()
                        .values()
                        .flatten()
                        .map(|(from, to)| (from.clone(), to.clone()))
                        .collect(),
                );
            }
            ReportedInconclusiveCause::Cause(InconclusiveCause::ConstraintReached {
                boundary_states,
            }) => {
                item.boundary_states = Some(*boundary_states);
            }
            ReportedInconclusiveCause::Cause(InconclusiveCause::ReductionHorizon { max_depth }) => {
                item.max_depth = Some(*max_depth);
            }
            ReportedInconclusiveCause::Cause(InconclusiveCause::MemoryBoundReached {
                bound,
                states,
            }) => {
                item.bound = Some(bound.member.as_str().to_owned());
                item.value = Some(bound.value);
                item.states = Some(ItemCauseStates::Count(*states));
            }
            ReportedInconclusiveCause::Cause(InconclusiveCause::InstanceBoundReached {
                bound,
                states,
            }) => {
                item.bound = Some(InstanceBound::MEMBER.to_owned());
                item.value = Some(bound.value);
                item.states = Some(ItemCauseStates::Count(*states));
            }
            ReportedInconclusiveCause::KaniVacuousProof => {}
            ReportedInconclusiveCause::Cause(InconclusiveCause::ReplayParity(parity)) => {
                item.parity = Some(ParityReason {
                    reason: parity.as_str(),
                    disagreement: match parity {
                        DisagreementCause::HybridTrace(disagreement) => {
                            Some(HybridDisagreementType {
                                kind: disagreement.as_str(),
                            })
                        }
                        _ => None,
                    },
                    proved: parity.proved().map(|verdict| verdict.category().as_str()),
                    replayed: parity.replayed().map(|verdict| verdict.category().as_str()),
                });
            }
            ReportedInconclusiveCause::Cause(InconclusiveCause::ReplayRefused(code)) => {
                item.code = Some(code.as_str().to_owned());
            }
            ReportedInconclusiveCause::Cause(InconclusiveCause::ReplayRefusedAtBound {
                code,
                cause,
                setting,
                exceeded,
            }) => {
                item.code = Some(code.as_str().to_owned());
                item.cause = Some((*cause).to_owned());
                item.setting = Some(setting);
                item.bound = Some(exceeded.bound.to_string());
                item.message = Some(exceeded.to_string());
            }
            ReportedInconclusiveCause::Cause(InconclusiveCause::StaleDependency {
                requested,
                recompiled,
            }) => {
                item.requested = Some(requested.into());
                item.recompiled = Some(recompiled.into());
            }
            // The kind alone: the claim identity and outcome stay on the
            // typed cause, which holds them for the caller.
            ReportedInconclusiveCause::Cause(InconclusiveCause::ScalarAgrees(_)) => {}
            ReportedInconclusiveCause::Cause(InconclusiveCause::WellDefinednessUnchecked {
                sources,
            }) => {
                item.sources = Some(sources.clone());
            }
        }
        item
    }
}

/// Most outcome items retain the existing nullable cause member. A request
/// disposition's unsupported item instead carries a diagnostic and omits
/// cause entirely (FR-286, FSpec FR-331).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
enum ItemCauseMember {
    Present { cause: Option<Box<ItemCause>> },
    Omitted {},
}

/// One requested item of a `prove`, `analyze` or `monitor` outcome: its
/// terminal record and its category.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OutcomeItem {
    request_index: usize,
    /// The record's `result`; absent on a record settled by an
    /// `invalid-request` disposition, which carries no result (FSpec
    /// FR-331, FR-621).
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<ItemLabel>,
    /// The item's result label, which an `invalid-request` disposition
    /// record does not write.
    #[serde(skip)]
    label: ItemLabel,
    #[serde(flatten)]
    cause: ItemCauseMember,
    /// The FR-272 diagnostic of a record settled by an `unsupported` or
    /// `invalid-request` disposition (FR-286), which carries it and no `cause`.
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostic: Option<ItemDiagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    basis: Option<ProofBasis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    certification: Option<Certification>,
    /// A `measured` item's FSpec FR-409 decision: `accepted`, `rejected` or
    /// `undecided`.
    #[serde(skip_serializing_if = "Option::is_none")]
    measured: Option<&'static str>,
    /// The FR-410 StatisticalVerdict of a completed measured result.
    #[serde(skip_serializing_if = "Option::is_none")]
    verdict: Option<serde_json::Value>,
    /// The state-graph settlement method, when this item came from FR-391.
    #[serde(skip_serializing_if = "Option::is_none")]
    method: Option<&'static str>,
    /// The state-graph search horizon, a method parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    max_depth: Option<Option<u64>>,
    /// The state-graph phase-0 seed actually used, written as FSpec FR-331's
    /// `U64String` (a decimal string), so every `u64` seed stays exact under
    /// RFC 8785 (FR-286 "Numbers").
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_u64_string"
    )]
    seed: Option<u64>,
    /// The number of state-graph witness walks per initial state.
    #[serde(skip_serializing_if = "Option::is_none")]
    witness_samples: Option<u64>,
    /// The effective state-graph limits used by the run.
    #[serde(skip_serializing_if = "Option::is_none")]
    limits: Option<BTreeMap<&'static str, u64>>,
    /// The state-graph limit reached, when the run stopped at one.
    #[serde(skip_serializing_if = "Option::is_none")]
    limit_reached: Option<ReachedLimitWire>,
    /// A clause run's trace position (FR-283's record).
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<u64>,
    /// A violation's or undefined evaluation's `TemporalCounterexample`.
    #[serde(skip_serializing_if = "Option::is_none")]
    counterexample: Option<serde_json::Value>,
    /// A temporal violation's FR-283 `decisive`, a sibling of
    /// `counterexample`.
    #[serde(skip_serializing_if = "Option::is_none")]
    decisive: Option<serde_json::Value>,
    /// A violation's FSpec FR-351 separating witness record, when the report
    /// holds one.
    #[serde(skip_serializing_if = "Option::is_none")]
    witness: Option<serde_json::Value>,
    /// A state-graph item's FSpec FR-393 witness evidence and its sampled
    /// witnesses (FR-628).
    #[serde(skip_serializing_if = "Option::is_none")]
    evidence: Option<serde_json::Value>,
    /// A state-graph item's checker-accepted certificate document, carried
    /// internally to the FR-286 item writer and nested under `certificate`.
    #[serde(skip_serializing_if = "Option::is_none")]
    certificate_document: Option<serde_json::Value>,
    /// The values an insufficient-data window held.
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u64>,
    /// An insufficient-data window's `min_count`.
    #[serde(skip_serializing_if = "Option::is_none")]
    min_count: Option<u64>,
    /// The qualified name of the clause a clause run settled (FR-286).
    #[serde(skip_serializing_if = "Option::is_none")]
    clause: Option<String>,
    /// FR-622: the four outcome axes of a `monitor` item, copied from its
    /// clause run; absent on every item not built from a clause run.
    #[serde(skip_serializing_if = "Option::is_none")]
    axes: Option<crate::OutcomeAxes>,
    /// The FR-990 per-obligation, closure and global conformance result.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    protocol_conformance: Option<ProtocolConformanceOutput>,
    #[serde(serialize_with = "category_str")]
    category: Category,
}

/// The extra FSpec FR-059 result members of a `global-conformance` item.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProtocolConformanceOutput {
    /// One result per protocol obligation.
    pub obligations: Vec<ConformanceObligationOutput>,
    /// The complete-global-conformance closure record.
    pub closure: ConformanceClosureOutput,
    /// The result joined from the obligations and closure.
    pub global: ConformanceGlobalOutput,
    /// The protocol's per-channel `max_messages` bound and whether a send
    /// reached it, when the run has a channel (FR-225, FR-636).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_messages: Option<ConformanceMessageBoundOutput>,
    /// Per-role projection results, absent in recording mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projections: Option<Vec<ConformanceProjectionOutput>>,
    /// The provisioned FR-737 recording identity, present in recording mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording: Option<ConformanceRecordingIdentity>,
    /// Whether the global counterexample replay reproduced its violation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay: Option<ConformanceReplayOutput>,
}

/// The per-channel `max_messages` method bound a protocol run used.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct ConformanceMessageBoundOutput {
    /// The configured bound (4 when unset).
    pub value: u64,
    /// Whether an offered send reached the bound.
    pub reached: bool,
}

/// The command-level identity under which a protocol recording was provisioned.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConformanceRecordingIdentity {
    /// The authority that provisioned the recording.
    pub authority: String,
    /// The provisioned recording identity.
    pub identity: String,
}

/// The in-process replay result for a global-conformance counterexample.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConformanceReplayOutput {
    /// Whether replay reproduced the claimed violation.
    pub reproduced: bool,
}

/// One FR-990 verdict and its evidence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConformanceObligationOutput {
    /// The obligation's typed protocol identity.
    pub obligation: serde_json::Value,
    /// `satisfied`, `violated` or `pending`.
    pub verdict: String,
    /// The FSpec FR-059 cause of a violation, when one is reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
    /// The basis for the decision.
    pub basis: String,
    /// The witness index or recording observations supporting a result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub witness: Option<serde_json::Value>,
}

/// The independent FR-990 closure record.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConformanceClosureOutput {
    /// `closed`, `open`, `incomplete` or `contradicted`.
    pub state: String,
    /// The complete set of closure premises.
    pub premises: Vec<serde_json::Value>,
    /// `engine` or `recording`.
    pub producer: String,
}

/// The joined FR-990 global result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConformanceGlobalOutput {
    /// The FSpec FR-360 result label.
    pub label: String,
    /// The basis of the result.
    pub basis: String,
    /// Whether the global result is complete success.
    pub complete: bool,
    /// The cause for an inconclusive or incomplete result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
}

/// One FR-890 projection result beside, and independent from, conformance.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConformanceProjectionOutput {
    /// The role the projection is over.
    pub role: String,
    /// The projection's FSpec FR-360 result label.
    pub label: String,
    /// The projection's counterexample, when refuted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterexample: Option<serde_json::Value>,
}

/// A reached state-graph limit on FR-331's accounting record.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct ReachedLimitWire {
    limit: &'static str,
    value: u64,
    setting: &'static str,
}

/// The catalogued reason of an undefined value: the kernel's own spelling,
/// a family record's reason, or `insufficient-data` for a window.
pub fn undefined_reason(cause: &UndefinedCause) -> &'static str {
    match cause {
        UndefinedCause::Kernel(reason) => kernel_undefined_reason(reason.clone()),
        UndefinedCause::Family(record) => record.reason.as_str(),
        UndefinedCause::InsufficientData(_) => "insufficient-data",
    }
}

/// FSpec FR-409's `measured` label and FR-331 cause of a statistical
/// verdict: `undefined-evaluation{where, cause}` for a rejection by an
/// undefined sample, `where` naming its test, trace and position, the
/// undecided cause for `undecided`, no cause otherwise.
fn measured(verdict: &StatisticalVerdict) -> (&'static str, Option<ItemCause>) {
    match verdict.decision {
        Decision::Accepted => ("accepted", None),
        Decision::Rejected => (
            "rejected",
            verdict
                .tests
                .iter()
                .find_map(|test| test.undefined.as_ref())
                .map(|undefined| {
                    ItemCause::undefined_evaluation(
                        format!(
                            "test {}, trace {}, position {}",
                            undefined.test, undefined.trace, undefined.evaluation.site.position
                        ),
                        undefined_reason(&undefined.evaluation.cause),
                    )
                }),
        ),
        Decision::Undecided(cause) => (
            "undecided",
            Some(ItemCause::kind(match cause {
                statistical::UndecidedCause::IndifferenceRegion => "indifference-region",
                statistical::UndecidedCause::UndecidedSuccessor => "undecided-successor",
            })),
        ),
    }
}

impl OutcomeItem {
    /// The item an FR-331 terminal record settles. A vacuous proof is
    /// `inconclusive` with its vacuity cause, never `proved`.
    pub fn from_terminal(record: &TerminalRecord) -> Self {
        let value = record.value();
        let result = ItemLabel::of(value);
        let cause = match value {
            TerminalValue::Proved {
                basis: ProofBasis::Checks { success_checks: 0 },
                ..
            }
            | TerminalValue::Inconclusive(_) => value
                .inconclusive_cause()
                .map(|cause| ItemCause::inconclusive(&cause)),
            TerminalValue::Proved { .. } | TerminalValue::Tested | TerminalValue::Refuted => None,
            TerminalValue::Unsupported(UnavailabilityCause::Disposition) => None,
            TerminalValue::Declined { cause, code } => Some(ItemCause {
                code: Some(match code {
                    DeclineCode::Fsl(code) => code.as_str().to_owned(),
                    DeclineCode::Std001(code) => code.as_str().to_owned(),
                }),
                ..ItemCause::kind(cause.as_str())
            }),
            TerminalValue::Unsupported(cause) => {
                let mut item = ItemCause {
                    code: cause.code().map(str::to_owned),
                    ..ItemCause::kind(cause.as_str())
                };
                match cause {
                    UnavailabilityCause::StatisticalRequest(refusal) => {
                        item.refusal = Some(refusal.as_str());
                    }
                    UnavailabilityCause::TotalAboveDrawRange { total } => {
                        item.total = Some(total.to_string());
                    }
                    UnavailabilityCause::WeakAccessShape { failures } => {
                        item.failures = Some(failures.iter().map(Into::into).collect());
                    }
                    UnavailabilityCause::RecoveryView { predicate } => {
                        item.predicate = Some(predicate.clone());
                    }
                    UnavailabilityCause::StrictClockConstraint { span } => {
                        item.at = Some(ItemWhere::Text(format!("{}..{}", span.start, span.end)));
                    }
                    UnavailabilityCause::DelayDistribution { operation } => {
                        item.operation = Some(operation.clone());
                    }
                    UnavailabilityCause::ZeroDelayCycle { states } => {
                        item.states = Some(ItemCauseStates::Cycle(states.clone()));
                    }
                    UnavailabilityCause::TimedFormShape { span, claim, .. } => match claim {
                        Some(claim) => item.locus = Some(ItemLocus::Claim(WireLocus::of(claim))),
                        None => {
                            item.at = Some(ItemWhere::Text(format!("{}..{}", span.start, span.end)))
                        }
                    },
                    UnavailabilityCause::ValueCheck { reason, .. } => {
                        item.refusal = Some(reason.as_str());
                    }
                    UnavailabilityCause::ContractCheck { reason, .. } => {
                        item.refusal = Some(reason.as_str());
                    }
                    UnavailabilityCause::ProtocolAnalysis(unsupported) => {
                        item.at = crate::protocol_analysis::node_text(&unsupported.node)
                            .ok()
                            .map(ItemWhere::Text);
                        item.message.clone_from(&unsupported.detail);
                    }
                    UnavailabilityCause::SolverAbsent
                    | UnavailabilityCause::HybridProviderAbsent
                    | UnavailabilityCause::BackendAbsent
                    | UnavailabilityCause::Disposition
                    | UnavailabilityCause::MissingFairnessPremise { .. }
                    | UnavailabilityCause::UnsupportedReduction { .. }
                    | UnavailabilityCause::UnsupportedHyperForm { .. }
                    | UnavailabilityCause::UnsupportedSmtFragment
                    | UnavailabilityCause::NotMarkov
                    | UnavailabilityCause::ConfidenceBeyondPrecision
                    | UnavailabilityCause::PunctualInterval { .. } => {}
                    UnavailabilityCause::NotStochastic { state, identity } => {
                        item.state = Some(state.into());
                        item.identity = Some(identity.into());
                    }
                    UnavailabilityCause::TimeLockedSample { state } => {
                        item.state = Some(state.into());
                    }
                }
                Some(item)
            }
            TerminalValue::Incomplete(cause) => {
                let mut item = ItemCause::kind(cause.as_str());
                match cause {
                    IncompleteCause::Cancelled { source } => item.source = Some(*source),
                    IncompleteCause::LimitReached { limit, value } => {
                        item.limit = Some(CauseLimit::Name(limit.as_str()));
                        item.value = Some(*value);
                        item.setting = Some(limit.setting());
                    }
                    IncompleteCause::PrecisionBudget { lower, upper } => {
                        item.lower = Some((**lower).clone());
                        item.upper = Some((**upper).clone());
                    }
                    IncompleteCause::UnresolvedDynamics => {
                        item.search_reason = Some("unresolved-dynamics");
                    }
                    IncompleteCause::CertificateUnchecked => {
                        item.search_reason = Some("certificate-unchecked");
                    }
                    IncompleteCause::TimedOut | IncompleteCause::ResourceExhausted => {}
                }
                Some(item)
            }
            TerminalValue::ToolFailed(failure) => {
                let mut item = ItemCause::kind("tool-failure");
                item.defect = Some(failure.defect());
                if let Some((limit, value)) = failure.byte_counts() {
                    item.limit = Some(CauseLimit::Bytes(limit));
                    item.value = Some(value);
                }
                Some(item)
            }
            TerminalValue::Failed => record.diagnostic().map(|diagnostic| ItemCause {
                code: Some(diagnostic.code().to_owned()),
                ..ItemCause::kind(diagnostic.cause())
            }),
            TerminalValue::Measured(verdict) => measured(verdict).1,
        };
        let (basis, certification) = match value {
            TerminalValue::Proved {
                basis,
                certification,
            } => {
                if result == ItemLabel::Proved {
                    (Some(basis.clone()), Some(*certification))
                } else {
                    (None, None)
                }
            }
            TerminalValue::Tested
            | TerminalValue::Refuted
            | TerminalValue::Declined { .. }
            | TerminalValue::Unsupported(_)
            | TerminalValue::Incomplete(_)
            | TerminalValue::Inconclusive(_)
            | TerminalValue::Failed
            | TerminalValue::ToolFailed(_)
            | TerminalValue::Measured(_) => (None, None),
        };
        let measured = match value {
            TerminalValue::Measured(verdict) => Some(measured(verdict).0),
            TerminalValue::Proved { .. }
            | TerminalValue::Tested
            | TerminalValue::Refuted
            | TerminalValue::Declined { .. }
            | TerminalValue::Unsupported(_)
            | TerminalValue::Incomplete(_)
            | TerminalValue::Inconclusive(_)
            | TerminalValue::Failed
            | TerminalValue::ToolFailed(_) => None,
        };
        let diagnostic = match (
            record
                .disposition_detail()
                .or_else(|| record.diagnostic_detail()),
            record.diagnostic(),
        ) {
            (Some(detail), Some(code)) => Some(ItemDiagnostic {
                code: code.code(),
                cause: code.cause(),
                detail: detail.clone(),
            }),
            _ => None,
        };
        // FR-286, FR-621: a record settled by an `invalid-request`
        // disposition carries its diagnostic, and no `result` and no `cause`.
        let invalid_request = matches!(value, TerminalValue::Declined { .. })
            && record.disposition_detail().is_some();
        // A verdict with a member no spelling exists for is an internal
        // failure of the item, never a document with the member missing.
        let (verdict, result, cause, category) = match value {
            TerminalValue::Measured(measured_verdict) => {
                match statistical_verdict_wire(measured_verdict) {
                    Ok(wire) => (Some(wire), result, cause, value.category()),
                    Err(_) => (
                        None,
                        ItemLabel::Failed,
                        Some(ItemCause::kind("internal-fault")),
                        TerminalValue::Failed.category(),
                    ),
                }
            }
            _ => (None, result, cause, value.category()),
        };
        let measured = measured.filter(|_| verdict.is_some());
        Self {
            request_index: record.request_index().get(),
            result: (!invalid_request).then_some(result),
            label: result,
            cause: if invalid_request
                || matches!(
                    value,
                    TerminalValue::Unsupported(UnavailabilityCause::Disposition)
                ) {
                ItemCauseMember::Omitted {}
            } else {
                ItemCauseMember::Present {
                    cause: cause.map(Box::new),
                }
            },
            diagnostic,
            basis,
            certification,
            measured,
            verdict,
            method: None,
            max_depth: None,
            seed: None,
            witness_samples: None,
            limits: None,
            limit_reached: None,
            position: None,
            counterexample: None,
            decisive: None,
            witness: None,
            evidence: None,
            certificate_document: None,
            count: None,
            min_count: None,
            clause: None,
            axes: None,
            protocol_conformance: None,
            category,
        }
    }

    /// The item an `analyze` or `prove` terminal record settles, with the
    /// wire spelling of the record's one counterexample (FR-286 "Analyze
    /// items and their counterexamples"). Only a `refuted` item carries it. A
    /// refutation by a claim evaluation undefined at a state, given as
    /// `undefined` (where, why), is `refuted` with cause `undefined-evaluation`
    /// and never the `undefined` label (FR-281).
    ///
    /// # Errors
    ///
    /// [`OutcomeItemError::MissingCounterexample`] for a `refuted` record
    /// given no counterexample; no item is built.
    pub fn from_analyze(
        record: &TerminalRecord,
        counterexample: Option<serde_json::Value>,
        undefined: Option<(String, String)>,
    ) -> Result<Self, OutcomeItemError> {
        let mut item = match undefined {
            Some((at, cause)) if matches!(record.value(), TerminalValue::Refuted) => {
                Self::undefined_evaluation(record.request_index(), at, cause)
            }
            _ => Self::from_terminal(record),
        };
        if item.label == ItemLabel::Refuted {
            item.counterexample =
                Some(counterexample.ok_or(OutcomeItemError::MissingCounterexample)?);
        }
        Ok(item)
    }

    /// Attach the typed FR-990 result members to a global-conformance item.
    #[must_use]
    pub fn with_protocol_conformance(mut self, output: ProtocolConformanceOutput) -> Self {
        self.protocol_conformance = Some(output);
        self
    }

    /// The FR-331 item for one settled state-graph claim instance, including
    /// FR-391's method parameters and effective run limits.
    pub fn from_state_graph(request_index: RequestIndex, record: &crate::StateGraphRecord) -> Self {
        let terminal = TerminalRecord::from_value(request_index, record.value.clone());
        let mut item = Self::from_terminal(&terminal);
        item.method = Some(record.method);
        item.max_depth = Some(record.max_depth);
        item.seed = Some(record.seed);
        item.witness_samples = Some(record.witness_samples);
        item.limits = Some(
            record
                .limits
                .iter()
                .map(|(limit, value)| (limit.as_str(), *value))
                .collect(),
        );
        item.limit_reached = record.limit_reached.map(|reached| ReachedLimitWire {
            limit: reached.limit.as_str(),
            value: reached.value,
            setting: reached.limit.setting(),
        });
        item
    }

    /// This state-graph item carrying the FR-393 spelling of its refuting
    /// evidence as `counterexample` and of its witnesses as `evidence`.
    #[must_use]
    pub fn with_graph_evidence(
        mut self,
        counterexample: Option<serde_json::Value>,
        evidence: Option<serde_json::Value>,
    ) -> Self {
        self.counterexample = counterexample;
        self.evidence = evidence;
        self
    }

    /// Attach the accepted state-graph certificate's public document.
    #[must_use]
    pub fn with_graph_certificate_document(mut self, document: Option<serde_json::Value>) -> Self {
        self.certificate_document = document;
        self
    }

    /// The item whose claim evaluated to an undefined value at `at` for
    /// `cause`: refuted, category violation, never labelled `undefined`.
    pub fn undefined_evaluation(
        request_index: RequestIndex,
        at: impl Into<String>,
        cause: impl Into<String>,
    ) -> Self {
        Self {
            request_index: request_index.get(),
            result: Some(ItemLabel::Refuted),
            label: ItemLabel::Refuted,
            cause: ItemCauseMember::Present {
                cause: Some(Box::new(ItemCause::undefined_evaluation(at, cause))),
            },
            diagnostic: None,
            basis: None,
            certification: None,
            measured: None,
            verdict: None,
            method: None,
            max_depth: None,
            seed: None,
            witness_samples: None,
            limits: None,
            limit_reached: None,
            position: None,
            counterexample: None,
            decisive: None,
            witness: None,
            evidence: None,
            certificate_document: None,
            count: None,
            min_count: None,
            clause: None,
            axes: None,
            protocol_conformance: None,
            category: Category::Violation,
        }
    }

    /// The item's O-16 category.
    pub fn category(&self) -> Category {
        self.category
    }

    /// The item one clause run settles (FR-286 "Monitor documents from clause
    /// runs"). The category is `report.category()`, the catalog's, and the
    /// result label follows from it; a pending or indeterminate run carries
    /// the catalog label and no FR-331 terminal record. The item names its
    /// `clause`.
    ///
    /// # Errors
    ///
    /// [`ClauseRunNotAnItem`] for an internal failure (an evaluator fault
    /// fails the whole request) and for the category inconclusive, which no
    /// clause run settles.
    pub fn from_clause_run(
        request_index: RequestIndex,
        clause: &str,
        report: &ClauseRunReport,
    ) -> Result<Self, ClauseRunNotAnItem> {
        let category = report.category();
        let result = match category {
            Category::Success => ItemLabel::Tested,
            Category::Violation => ItemLabel::Refuted,
            Category::Pending => ItemLabel::Pending,
            Category::Indeterminate => ItemLabel::Indeterminate,
            Category::Incomplete => ItemLabel::Incomplete,
            Category::Unsupported => ItemLabel::Unsupported,
            Category::Refusal => ItemLabel::Declined,
            Category::Undefined => ItemLabel::Undefined,
            Category::Inconclusive | Category::InternalFailure => {
                return Err(ClauseRunNotAnItem::Category(category));
            }
        };
        let record = item_record(report)?;
        Ok(Self {
            request_index: request_index.get(),
            result: Some(result),
            label: result,
            diagnostic: clause_run_diagnostic(report),
            cause: ItemCauseMember::Present {
                cause: clause_run_cause(report, clause).map(Box::new),
            },
            position: record.position,
            counterexample: record.counterexample,
            decisive: record.decisive,
            witness: record.witness,
            evidence: None,
            certificate_document: None,
            count: record.count,
            min_count: record.min_count,
            basis: None,
            certification: None,
            measured: None,
            verdict: None,
            method: None,
            max_depth: None,
            seed: None,
            witness_samples: None,
            limits: None,
            limit_reached: None,
            clause: Some(clause.to_owned()),
            axes: Some(report.axes()),
            protocol_conformance: None,
            category,
        })
    }

    /// The qualified name of the clause a clause run settled, when this item
    /// came from one.
    pub fn clause(&self) -> Option<&str> {
        self.clause.as_deref()
    }

    /// The item's result label.
    pub fn result(&self) -> ItemLabel {
        self.label
    }

    /// The zero-based index of the request this item settles.
    pub fn request_index(&self) -> RequestIndex {
        RequestIndex::new(self.request_index)
    }

    /// Who stands behind a `proved` result; `None` on every other label.
    pub fn certification(&self) -> Option<Certification> {
        self.certification
    }
}

fn rational_wire(value: &filament_exact::Rational) -> serde_json::Value {
    serde_json::json!({
        "numerator": value.numerator().to_string(),
        "denominator": value.denominator().to_string(),
    })
}

fn statistical_method_wire(method: statistical::StatisticalMethod) -> serde_json::Value {
    match method {
        statistical::StatisticalMethod::Okamoto => serde_json::json!({"type": "okamoto"}),
        statistical::StatisticalMethod::Sprt => serde_json::json!({"type": "sprt"}),
        statistical::StatisticalMethod::ClopperPearson => {
            serde_json::json!({"type": "clopper-pearson"})
        }
        statistical::StatisticalMethod::Regenerative { min_cycles } => {
            serde_json::json!({"type": "regenerative", "min_cycles": min_cycles})
        }
    }
}

fn statistical_decision_wire(
    decision: Decision,
    undefined: Option<&statistical::SampledUndefined>,
) -> Result<serde_json::Value, CounterexampleWireError> {
    Ok(match decision {
        Decision::Accepted => serde_json::json!({"type": "accepted"}),
        Decision::Rejected => {
            let mut rejected = serde_json::json!({"type": "rejected"});
            if let Some(found) = undefined {
                rejected["cause"] = counterexample::sampled_undefined_wire(found)?;
            }
            rejected
        }
        Decision::Undecided(cause) => serde_json::json!({
            "type": "undecided",
            "cause": {"type": match cause {
                statistical::UndecidedCause::IndifferenceRegion => "indifference-region",
                statistical::UndecidedCause::UndecidedSuccessor => "undecided-successor",
            }}
        }),
    })
}

/// FSpec FR-410's `estimate`. The backend-provider `Estimate` schema has no
/// spelling for an absent estimate or a censored quantile, so neither is
/// written: `None` for no estimate, and no `quantile` member for a censored one.
fn statistical_estimate_wire(
    estimate: Option<&filament_exact::Rational>,
    quantile: Option<&statistical::MeasureValue>,
) -> Option<serde_json::Value> {
    let mut value = serde_json::Map::new();
    value.insert("value".to_owned(), rational_wire(estimate?));
    if let Some(statistical::MeasureValue::Value(quantile)) = quantile {
        value.insert(
            "quantile".to_owned(),
            serde_json::json!({"value": rational_wire(quantile)}),
        );
    }
    Some(serde_json::Value::Object(value))
}

fn statistical_interval_wire(
    interval: Option<&(filament_exact::Rational, filament_exact::Rational)>,
) -> Option<serde_json::Value> {
    interval.map(|(lower, upper)| {
        serde_json::json!({
            "lower": rational_wire(lower),
            "upper": rational_wire(upper),
        })
    })
}

/// Why a sampled witness is not in a verdict's `witnesses` (FSpec FR-410).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WitnessOmission {
    /// A step of the witness is the terminal stutter step, which has no FR-181
    /// transition identity.
    TerminalStutterStep,
    /// The witness's measure is censored; `Quantity` has no spelling for it.
    CensoredMeasure,
    /// The schema requires each step's `scheduled` as a `TypedValue`, which
    /// cannot hold the scheduled identity (operation, receiver, non-random
    /// arguments), and the verdict records only the choice indexes.
    ScheduledIdentityNotSpellable,
}

impl WitnessOmission {
    /// The reason's spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TerminalStutterStep => "terminal-stutter-step",
            Self::CensoredMeasure => "censored-measure",
            Self::ScheduledIdentityNotSpellable => "scheduled-identity-not-spellable",
        }
    }
}

/// One witness a verdict leaves out, and why.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OmittedWitness {
    /// The witness's test index.
    pub test: u32,
    /// The witness's trace index.
    pub trace: u64,
    /// Why it is not written.
    pub reason: WitnessOmission,
}

/// Every witness of `verdict` that is not written to its `witnesses`, with the
/// reason. A witness is written only when every member of it is schema-valid.
pub fn omitted_witnesses(verdict: &StatisticalVerdict) -> Vec<OmittedWitness> {
    verdict
        .witnesses
        .iter()
        .map(|witness| OmittedWitness {
            test: witness.test,
            trace: witness.trace,
            reason: counterexample::witness_omission(witness),
        })
        .collect()
}

/// The FSpec FR-410 `StatisticalVerdict` of `verdict`.
///
/// # Errors
///
/// [`CounterexampleWireError`] when a member has no spelling.
fn statistical_verdict_wire(
    verdict: &StatisticalVerdict,
) -> Result<serde_json::Value, CounterexampleWireError> {
    let method = statistical_method_wire(verdict.basis.method);
    let rejection = verdict
        .tests
        .iter()
        .find_map(|test| test.undefined.as_ref());
    let decision = statistical_decision_wire(verdict.decision, rejection)?;
    let mut tests = Vec::with_capacity(verdict.tests.len());
    for test in &verdict.tests {
        let mut result = serde_json::Map::new();
        result.insert("index".to_owned(), serde_json::json!(test.index));
        result.insert(
            "decision".to_owned(),
            statistical_decision_wire(test.decision, test.undefined.as_ref())?,
        );
        // A test an undefined sample rejected, or whose first sample reached
        // an undecided successor, has no estimate and the schema has no
        // spelling for that; any other test without one never reaches
        // a verdict (it settles `incomplete`).
        match statistical_estimate_wire(test.estimate.as_ref(), test.quantile.as_ref()) {
            Some(estimate) => {
                result.insert("estimate".to_owned(), estimate);
            }
            None if test.undefined.is_some()
                || test.decision
                    == Decision::Undecided(statistical::UndecidedCause::UndecidedSuccessor) => {}
            None => return Err(CounterexampleWireError::Unspelled("test estimate")),
        }
        if let Some(interval) = statistical_interval_wire(test.interval.as_ref()) {
            result.insert("interval".to_owned(), interval);
        }
        result.insert("samples".to_owned(), serde_json::json!(test.samples));
        result.insert("draws".to_owned(), serde_json::json!(test.draws));
        tests.push(serde_json::Value::Object(result));
    }
    let mut provenance_tests = Vec::with_capacity(verdict.provenance.tests.len());
    for test in &verdict.provenance.tests {
        let last = test.traces.first.saturating_add(
            test.traces
                .count
                .saturating_sub(1)
                .saturating_mul(test.traces.step),
        );
        provenance_tests.push(serde_json::json!({
            "index": test.index,
            "initial_state": test.initial,
            "binding": counterexample::binding_wire(test.reference.as_ref())?,
            "instance": null,
            "first_trace_index": test.traces.first.to_string(),
            "last_trace_index": last.to_string(),
        }));
    }
    let mut basis = serde_json::json!({
        "method": method,
        "confidence": {
            "alpha": rational_wire(verdict.basis.confidence.alpha()),
            "beta": rational_wire(verdict.basis.confidence.beta()),
            "iota": rational_wire(verdict.basis.confidence.indifference()),
        },
        "samples": verdict.basis.samples,
        "draws": verdict.basis.draws,
        "tests": {
            "m": verdict.basis.tests,
            "alpha": rational_wire(&verdict.basis.alpha_per_test),
            "beta": rational_wire(&verdict.basis.beta_per_test),
        },
        "coverage": match verdict.basis.coverage {
            statistical::Coverage::FiniteSample => "finite-sample",
            statistical::Coverage::Asymptotic => "asymptotic",
        },
    });
    if let Some(grid) = verdict.basis.delay_grid_bits {
        basis["grid"] = serde_json::json!(grid);
    }
    // The schema's `provenance` closes its members: `timed_horizon`,
    // `delay_grid_bits` and `max_sample_steps` are not written.
    let provenance = serde_json::json!({
        "seed": verdict.provenance.seed.to_string(),
        "sampler": "filament.simulation.sampler/v1",
        "tests": provenance_tests,
    });
    // A witness is written only when every member of it is schema-valid;
    // none is today (`omitted_witnesses` says why).
    let mut value = serde_json::json!({
        "decision": decision,
        "basis": basis,
        "provenance": provenance,
        "tests": tests,
        "witnesses": [],
    });
    match statistical_estimate_wire(verdict.estimate.as_ref(), verdict.quantile.as_ref()) {
        Some(estimate) => value["estimate"] = estimate,
        None if rejection.is_some()
            || verdict.decision
                == Decision::Undecided(statistical::UndecidedCause::UndecidedSuccessor) => {}
        None => return Err(CounterexampleWireError::Unspelled("verdict estimate")),
    }
    if let Some(interval) = statistical_interval_wire(verdict.interval.as_ref()) {
        value["interval"] = interval;
    }
    Ok(value)
}

/// The cause an item built from `report` carries: the table's `cause`
/// column of FR-286 "Monitor documents from clause runs". A success, a
/// violated clause, a pending run and a clause with insufficient data
/// (`insufficient-data`) carry the spelling the table gives them.
fn clause_run_diagnostic(report: &ClauseRunReport) -> Option<ItemDiagnostic> {
    let mut detail = serde_json::Map::new();
    let (code, cause) = match &report.disposition {
        ClauseDisposition::NotAPredicate { name, actual, .. } => {
            detail.insert("function".to_owned(), serde_json::json!(name));
            detail.insert("expected".to_owned(), serde_json::json!("Boolean"));
            detail.insert("actual".to_owned(), serde_json::json!(actual));
            ("ill_typed", "type-mismatch")
        }
        ClauseDisposition::MissingName { name } => {
            detail.insert("name".to_owned(), serde_json::json!(name));
            ("missing_declaration", "missing-name")
        }
        _ => return None,
    };
    Some(ItemDiagnostic {
        code,
        cause,
        detail,
    })
}

fn clause_run_cause(report: &ClauseRunReport, clause: &str) -> Option<ItemCause> {
    let catalog = |code: &'static str, cause: &'static str| ItemCause {
        code: Some(code.to_owned()),
        ..ItemCause::kind(cause)
    };
    let limit = |kind: &'static str, value: u64, setting: &'static str| ItemCause {
        limit: Some(CauseLimit::Name(kind)),
        value: Some(value),
        setting: Some(setting),
        ..ItemCause::kind("limit-reached")
    };
    match &report.disposition {
        ClauseDisposition::Temporal(verdict) => match verdict {
            TemporalVerdict::Inactive
            | TemporalVerdict::Holds
            | TemporalVerdict::Violated { .. }
            | TemporalVerdict::Pending
            | TemporalVerdict::PendingWindow { .. }
            | TemporalVerdict::PendingNextEvent { .. } => None,
            TemporalVerdict::Undefined {
                cause: TemporalUndefinedCause::InsufficientData { .. },
                ..
            } => Some(ItemCause::kind("insufficient-data")),
            TemporalVerdict::Undefined { at, cause } => Some(ItemCause::undefined_evaluation(
                format!("position {at}"),
                cause.reason(),
            )),
            TemporalVerdict::Indeterminate { at, reason } => Some(ItemCause {
                at: Some(ItemWhere::Text(format!("position {at}"))),
                kind: None,
                reason: Some(ReasonWire::from(reason)),
                ..ItemCause::kind("")
            }),
            TemporalVerdict::MissingFairnessPremise { .. } => Some(catalog(
                "unsupported_projection",
                "missing-fairness-premise",
            )),
            TemporalVerdict::Incomplete(incomplete) => {
                Some(accounting_limit_cause(incomplete, "limit-reached", None))
            }
            TemporalVerdict::Cancelled(source) => Some(ItemCause {
                source: Some(*source),
                ..ItemCause::kind("cancelled")
            }),
            TemporalVerdict::ActivationIncomplete { at, .. } => Some(ItemCause {
                at: Some(ItemWhere::Text(format!("position {at}"))),
                ..ItemCause::kind("activation-incomplete")
            }),
            TemporalVerdict::WindowLimitReached {
                limit: kind,
                value,
                setting,
            } => Some(limit(kind, *value, setting)),
            TemporalVerdict::ProgressLimitReached(reached) => {
                Some(limit(reached.limit, reached.value, reached.setting))
            }
            TemporalVerdict::ExpansionLimitReached(reached) => Some(limit(
                reached.limit.as_str(),
                reached.value,
                reached.limit.setting(),
            )),
        },
        ClauseDisposition::TemporalRefusal(refusal) => {
            let code = refusal.catalog_code();
            Some(catalog(code.code(), code.cause()))
        }
        ClauseDisposition::TraceRefused(refusal) => Some(ItemCause {
            message: Some(refusal.message.clone()),
            ..catalog("invalid_runtime_input", "invalid-value")
        }),
        ClauseDisposition::ProgressContradiction(refusal) => {
            let code = &refusal.record.catalog;
            Some(catalog(code.code(), code.cause()))
        }
        ClauseDisposition::FrameViolation(witness) => {
            Some(catalog(witness.code.as_str(), witness.cause))
        }
        ClauseDisposition::Admit(
            AdmissionFailure::Incomplete(record) | AdmissionFailure::Refused(record),
        ) if record.fields.get("setting").map(String::as_str)
            == Some("observation.input_bytes") =>
        {
            let bound = record.fields.get("bound")?;
            let actual = record.fields.get("actual")?;
            Some(ItemCause {
                code: Some(record.catalog.code().to_owned()),
                limit: Some(CauseLimit::Name("input-bytes-exceeded")),
                value: bound.parse().ok(),
                counter: Some(actual.clone()),
                setting: Some("observation.input_bytes"),
                bound: Some(bound.clone()),
                ..ItemCause::kind("input-bytes-exceeded")
            })
        }
        ClauseDisposition::SnapshotAdmission {
            failure:
                fsl_semantics::model::observation::AdmissionFailure::Refused(record)
                | fsl_semantics::model::observation::AdmissionFailure::Incomplete(record),
            document,
            path,
        } if record.fields.get("setting").map(String::as_str)
            == Some("observation.input_bytes") =>
        {
            let bound = record.fields.get("bound")?;
            let actual = record.fields.get("actual")?;
            Some(ItemCause {
                code: Some(record.catalog.code().to_owned()),
                limit: Some(CauseLimit::Name("input-bytes-exceeded")),
                value: bound.parse().ok(),
                counter: Some(actual.clone()),
                setting: Some("observation.input_bytes"),
                bound: Some(bound.clone()),
                locus: snapshot_admission_locus(record, document, path.as_deref())
                    .map(ItemLocus::Outcome),
                ..ItemCause::kind("input-bytes-exceeded")
            })
        }
        ClauseDisposition::SnapshotAdmission {
            failure:
                fsl_semantics::model::observation::AdmissionFailure::Refused(record)
                | fsl_semantics::model::observation::AdmissionFailure::Incomplete(record),
            document,
            path,
        } => Some(ItemCause {
            kind: Some(record.catalog.cause()),
            code: Some(record.catalog.code().to_owned()),
            locus: snapshot_admission_locus(record, document, path.as_deref())
                .map(ItemLocus::Outcome),
            ..ItemCause::kind("")
        }),
        ClauseDisposition::Evaluate(CallOutcome::Undefined { reason, location }) => {
            Some(location.as_ref().map_or_else(
                || ItemCause::undefined_evaluation(clause, *reason),
                |location| ItemCause::undefined_evaluation_at(location, *reason),
            ))
        }
        ClauseDisposition::Compile(refusal) if has_timed_trace_subject_refusal(refusal) => {
            classify_clause_run(report)
                .classification
                .causes
                .first()
                .map(|cause| ItemCause {
                    code: Some(cause.code.as_str().to_owned()),
                    message: Some(refusal.to_string()),
                    ..ItemCause::kind(cause.cause.unwrap_or_else(|| cause.code.as_str()))
                })
        }
        ClauseDisposition::Evaluate(CallOutcome::Incomplete(incomplete)) => {
            Some(accounting_limit_cause(
                &incomplete.record,
                "resource_exhausted",
                Some("resource_exhausted"),
            ))
        }
        ClauseDisposition::Evaluate(CallOutcome::Refused(refusal)) => {
            let (CallRefusal::Record { code, .. } | CallRefusal::Family { code, .. }) = refusal;
            Some(catalog(code.code(), code.cause()))
        }
        _ => classify_clause_run(report)
            .classification
            .causes
            .first()
            .map(|cause| ItemCause {
                code: Some(cause.code.as_str().to_owned()),
                ..ItemCause::kind(cause.cause.unwrap_or_else(|| cause.code.as_str()))
            }),
    }
}

fn has_timed_trace_subject_refusal(refusal: &CompileRefusal) -> bool {
    let CompileRefusal::Check { refusals, .. } = refusal else {
        return false;
    };
    refusals.iter().any(|refusal| {
        matches!(
            &refusal.cause,
            fsl_semantics::check::CheckCause::Temporal(cause)
                if matches!(
                    cause.as_ref(),
                    fsl_semantics::check::TemporalCause::TimedTraceSubject { .. }
                )
        )
    })
}

fn accounting_limit_cause(
    incomplete: &filament_exact::Incomplete,
    kind: &'static str,
    code: Option<&'static str>,
) -> ItemCause {
    let counter = if incomplete.limit_kind.is_cumulative() {
        Integer::from(incomplete.consumed).add(&incomplete.next_charge)
    } else {
        incomplete.next_charge.clone()
    };
    let setting = crate::limits::AccountingLimits::setting_name(incomplete.limit_kind);
    ItemCause {
        code: code.map(str::to_owned),
        limit: Some(CauseLimit::Name(incomplete.limit_kind.as_str())),
        value: Some(incomplete.limit),
        counter: Some(counter.to_string()),
        bound: Some(incomplete.limit.to_string()),
        field: Some(incomplete.limit_kind.as_str()),
        setting: Some(setting),
        ..ItemCause::kind(kind)
    }
}

/// A diagnostic's position (ADR-013 T-5), as the document writes it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OutcomeLocus {
    /// An O-07 source region.
    Region {
        /// The source the offsets index.
        source: RawSourceRef,
        /// The region's start offset.
        start: u64,
        /// The region's end offset.
        end: u64,
    },
    /// A region rendered over its source, as an evaluation records it.
    Located {
        /// The authored path of the source.
        file: String,
        /// The region's span.
        span: LocatedSpan,
    },
    /// A kernel occurrence key, as its display form.
    Occurrence {
        /// The occurrence key's display form.
        occurrence: String,
    },
    /// An artifact and a JSON pointer into it.
    Artifact {
        /// The FR-001 identity of the artifact.
        identity: RawSourceRef,
        /// The RFC 6901 pointer into the artifact.
        pointer: String,
    },
    /// A position in an input named by the caller, not an FR-001 artifact.
    Input {
        /// The operand exactly as the caller wrote it.
        operand: String,
        /// The member file relative to a directory operand, when present.
        member: Option<String>,
        /// The RFC 6901 pointer into the input, when present.
        pointer: Option<String>,
    },
    /// A refusal located in a snapshot or invocation document.
    Document {
        /// The document's FR-001 labels.
        document: Option<RawSourceRef>,
        /// The caller's path for the document, when retained.
        path: Option<String>,
        /// The RFC 6901 pointer to the failing member or containing object.
        pointer: String,
        /// The population and key of the object, when the refusal names one.
        object: Option<DocumentObjectLocus>,
        /// The failing member, when the refusal names one.
        member: Option<String>,
    },
}

/// The population and snapshot object key named by a document locus.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DocumentObjectLocus {
    /// The model population.
    pub population: String,
    /// The snapshot object key.
    pub key: String,
}

pub(crate) fn snapshot_admission_locus(
    record: &fsl_semantics::model::observation::AdmissionRecord,
    document: &fsl_semantics::model::observation::DocumentRef,
    path: Option<&str>,
) -> Option<OutcomeLocus> {
    use filament_semantic_value::admission::PathStep;
    // The snapshot refusal's `path` member is the RFC 6901 pointer to the
    // failing member in the document's own spelling (FR-663); a refusal that
    // names none is located by its typed steps.
    let folded = || {
        record.path.iter().fold(String::new(), |mut pointer, step| {
            match step {
                PathStep::Field(member) => {
                    pointer.push('/');
                    pointer.push_str(&member.replace('~', "~0").replace('/', "~1"));
                }
                PathStep::Index(index) => {
                    pointer.push('/');
                    pointer.push_str(&index.to_string());
                }
                PathStep::OptionValue => {}
            }
            pointer
        })
    };
    let pointer = match record.fields.get("path") {
        Some(path) if path.starts_with('/') => path.clone(),
        _ => folded(),
    };
    let identity = fsl_foundation::source::provenance::RawSourceRef::new(
        document.authority.clone(),
        document.identity.clone(),
    )
    .ok();
    let object = record
        .fields
        .get("object")
        .zip(record.fields.get("population"))
        .map(|(key, population)| DocumentObjectLocus {
            population: population.clone(),
            key: key.clone(),
        });
    Some(OutcomeLocus::Document {
        document: identity,
        path: path.map(str::to_owned),
        pointer,
        object,
        member: record.fields.get("member").cloned(),
    })
}

fn pointer_member(pointer: &str) -> Option<String> {
    let token = pointer.rsplit('/').next()?;
    if token.is_empty() && pointer.is_empty() {
        return None;
    }
    Some(token.replace("~1", "/").replace("~0", "~"))
}

impl From<&Locus> for OutcomeLocus {
    fn from(locus: &Locus) -> Self {
        match locus {
            Locus::Region(region) => Self::Region {
                source: region.source().clone(),
                start: region.start(),
                end: region.end(),
            },
            Locus::Occurrence(location) => Self::Occurrence {
                occurrence: OccurrenceKey::from(location).to_string(),
            },
            Locus::Artifact { identity, pointer } => Self::Artifact {
                identity: identity.clone(),
                pointer: pointer.to_string(),
            },
            Locus::Input {
                operand,
                member,
                pointer,
            } => Self::Input {
                operand: operand.clone(),
                member: member.clone(),
                pointer: pointer.as_ref().map(ToString::to_string),
            },
        }
    }
}

/// One diagnostic: its typed cause, catalog code, locus and message.
/// A source blank-label refusal also retains the owner's typed label.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OutcomeDiagnostic {
    cause: Option<Box<str>>,
    code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    construct: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    declaration: Option<String>,
    /// FR-286 "Diagnostic details": `null`, or the closed object that locates
    /// the refusal inside the request and the call.
    details: Option<serde_json::Map<String, serde_json::Value>>,
    /// FR-286's catalog hint for the diagnostic's code and cause.
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<&'static str>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_source_label"
    )]
    label: Option<SourceLabel>,
    locus: Option<OutcomeLocus>,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
}

/// FSpec FR-331's `U64String`: an unsigned 64-bit integer as its canonical
/// decimal string.
fn serialize_u64_string<S>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    value.map(|value| value.to_string()).serialize(serializer)
}

fn serialize_source_label<S>(label: &Option<SourceLabel>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    label.map(SourceLabel::as_str).serialize(serializer)
}

impl OutcomeDiagnostic {
    /// A diagnostic with the catalog `code` and its typed `cause` (the
    /// catalog's cause tag, where the code has one), raised at `locus`.
    pub fn new(
        cause: Option<&str>,
        code: impl Into<String>,
        locus: Option<OutcomeLocus>,
        message: impl Into<String>,
    ) -> Self {
        let code = code.into();
        let hint = Code::from_code(&code).and_then(|code| hint::hint_for(code, cause));
        Self {
            cause: cause.map(Into::into),
            code,
            construct: None,
            declaration: None,
            hint,
            label: None,
            locus,
            message: message.into(),
            profile: None,
            text: None,
            details: None,
        }
    }

    /// This diagnostic with `details` (FR-286 "Diagnostic details"): the closed
    /// object that locates the refusal, never empty.
    #[must_use]
    pub fn with_details(
        mut self,
        details: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Self {
        self.details = details.filter(|members| !members.is_empty());
        self
    }

    /// Project a source diagnostic without bytes or an admitted source.
    ///
    /// Retains the typed cause, optional region and human message. A blank
    /// label retains the source owner's exact label; other causes omit it.
    pub(crate) fn project_source_diagnostic(
        diagnostic: &fsl_cst::diagnostic::CompleteDiagnostic,
    ) -> Self {
        let mut projected = Self::new(
            Some(diagnostic.cause.as_str()),
            diagnostic.code.as_str(),
            diagnostic
                .region
                .as_ref()
                .map(|region| OutcomeLocus::from(&Locus::Region(region.clone()))),
            diagnostic.message.clone(),
        );
        projected.label = match diagnostic.cause.identity_cause() {
            Some(SourceReadCause::BlankLabel { label }) => Some(label),
            _ => None,
        };
        projected
    }

    /// The diagnostic of a spine compile refusal: its catalog code, cause,
    /// region and message.
    pub fn from_compile_refusal(refusal: &CompileRefusal) -> Self {
        if let CompileRefusal::Source(diagnostic) = refusal {
            return Self::project_source_diagnostic(diagnostic);
        }
        let mut diagnostic = Self::new(
            refusal.cause(),
            refusal.code().as_str(),
            refusal
                .region()
                .map(|region| OutcomeLocus::from(&Locus::Region(region.clone()))),
            refusal.to_string(),
        );
        if let CompileRefusal::Forms { failure, .. } = refusal {
            match &failure.cause {
                fsl_forms::FormsCause::NoDispatchEntry {
                    construct,
                    text,
                    declaration,
                    profile,
                    ..
                }
                | fsl_forms::FormsCause::UnrepresentedConstruct {
                    construct,
                    text,
                    declaration,
                    profile,
                    ..
                } => {
                    diagnostic.construct = Some(construct.as_str().to_owned());
                    diagnostic.text = Some(text.to_string());
                    diagnostic.declaration = declaration.as_deref().map(str::to_owned);
                    diagnostic.profile = profile.as_deref().map(str::to_owned);
                }
                _ => {}
            }
        }
        diagnostic
    }

    /// Project a compile refusal with only the limit-setting mechanism the
    /// named operation accepts. Replay also accepts request `stage_limits`.
    fn from_compile_refusal_for_operation(operation: Operation, refusal: &CompileRefusal) -> Self {
        let diagnostic = Self::from_compile_refusal(refusal);
        Self::for_operation(operation, diagnostic)
    }

    /// Project every retained compile diagnostic with only the limit-setting
    /// mechanism the named operation accepts.
    fn from_compile_refusal_diagnostics_for_operation(
        operation: Operation,
        refusal: &CompileRefusal,
    ) -> Vec<Self> {
        refusal
            .outcome_diagnostics()
            .into_iter()
            .map(|diagnostic| Self::for_operation(operation, diagnostic))
            .collect()
    }

    fn for_operation(operation: Operation, mut diagnostic: Self) -> Self {
        if operation != Operation::Replay {
            if let Some((message, _)) = diagnostic
                .message
                .split_once(" or the request's `stage_limits` entry ")
            {
                diagnostic.message = message.to_owned();
            }
        }
        diagnostic
    }

    /// The diagnostic of a refused call: its catalog code and cause, and
    /// the resolved region its record carries as locus.
    pub fn from_call_refusal(refusal: &CallRefusal) -> Self {
        let (CallRefusal::Record { code, .. } | CallRefusal::Family { code, .. }) = refusal;
        let locus = match refusal {
            CallRefusal::Record {
                locus: Some(locus), ..
            } => Some(OutcomeLocus::Located {
                file: locus.file.clone(),
                span: locus.span,
            }),
            CallRefusal::Record { locus: None, .. } | CallRefusal::Family { .. } => None,
        };
        Self::new(Some(code.cause()), code.code(), locus, code.to_string())
    }

    /// The catalog code.
    pub fn code(&self) -> &str {
        &self.code
    }

    /// The typed cause tag.
    pub fn cause(&self) -> Option<&str> {
        self.cause.as_deref()
    }

    /// The diagnostic's typed position, when present.
    pub fn locus(&self) -> Option<&OutcomeLocus> {
        self.locus.as_ref()
    }

    /// The message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// A completed call's value, in FR-650's run value form.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ResultValue(serde_json::Value);

impl ResultValue {
    /// The value's JSON form.
    pub fn form(&self) -> &serde_json::Value {
        &self.0
    }
}

/// The evaluation result of an `execute` outcome (FR-286's `result`).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExecuteResult {
    /// The call completed with a value.
    Completed {
        /// The value.
        value: ResultValue,
    },
    /// The call's result is undefined.
    Undefined {
        /// The reason, as FR-100's tables spell it.
        reason: &'static str,
    },
    /// An accounting limit stopped the call.
    Incomplete {
        /// The exhausted limit.
        limit: ResultLimit,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
enum OutcomeResult {
    Execute(ExecuteResult),
    Parsed(ParsedOutcome),
    Formatted(FormattedOutcome),
    Exported(ExportedOutcome),
    Replayed(ReplayedOutcome),
    #[cfg(feature = "test-support")]
    Raw(serde_json::Value),
}

/// FR-286 "Export outcomes": the `exported` result holding the FSpec FR-817
/// export outcome document as the run outcome's `export` member carries it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ExportedOutcome {
    kind: &'static str,
    export: serde_json::Value,
}

/// A replay outcome's `replayed` result: each entry is the row of the FSpec
/// FR-323 `results` document, unchanged and in row order (FR-286).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ReplayedOutcome {
    kind: &'static str,
    results: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ParsedOutcome {
    kind: &'static str,
    source: fsl_foundation::source::provenance::RawSourceRef,
    path: String,
    bytes: String,
    usage: Vec<ParseUsage>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct FormattedOutcome {
    kind: &'static str,
    text: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ParseUsage {
    setting: &'static str,
    bound: String,
    used: String,
}

impl From<&crate::spine::ParseReport> for ParsedOutcome {
    fn from(report: &crate::spine::ParseReport) -> Self {
        let count = |value: usize| value.to_string();
        Self {
            kind: "parsed",
            source: report.source.clone(),
            path: report.path.clone(),
            bytes: count(report.bytes),
            usage: vec![
                ParseUsage {
                    setting: "s1.input_bytes",
                    bound: count(report.limits.source_bytes),
                    used: count(report.bytes),
                },
                ParseUsage {
                    setting: "s1.tokens",
                    bound: count(report.limits.tokens),
                    used: count(report.tokens),
                },
                ParseUsage {
                    setting: "s1.nodes",
                    bound: count(report.limits.nodes),
                    used: count(report.nodes),
                },
                ParseUsage {
                    setting: "s1.work_units",
                    bound: count(report.limits.work_units),
                    used: report.work_units.to_string(),
                },
            ],
        }
    }
}

/// The exhausted limit of an incomplete `execute` result (FR-277): bound and
/// counter (FR-277's counter at the failed charge) are
/// ASCII decimal strings.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ResultLimit {
    kind: &'static str,
    bound: String,
    counter: String,
    field: &'static str,
    setting: &'static str,
}

impl From<&CallIncomplete> for ResultLimit {
    fn from(incomplete: &CallIncomplete) -> Self {
        Self {
            kind: incomplete.record.limit_kind.as_str(),
            bound: incomplete.record.limit.to_string(),
            counter: incomplete.counter().to_string(),
            field: incomplete.limits_field(),
            setting: incomplete.setting_name(),
        }
    }
}

/// The `filament-outcome/1` document (FR-286).
///
/// `items` is always written, an empty array for an operation with no
/// requested items. `last_stage` is `null` when no stage is known to have
/// run. `result` is always written, either `null` or the operation's result
/// payload. `artifacts` is always written as an empty array.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutcomeDocument {
    #[cfg(feature = "quire-extraction")]
    native_fences: Option<crate::spine::NativeFenceReport>,
    format: &'static str,
    operation: Operation,
    last_stage: Option<OutcomeStage>,
    category: Category,
    items: Vec<OutcomeItem>,
    diagnostics: Vec<OutcomeDiagnostic>,
    result: Option<OutcomeResult>,
    requirements: Option<Box<RequirementItems>>,
    /// An `analyze` refusal: written with an empty `aggregate` and `registry` (FR-621).
    empty_analyze_members: bool,
}

/// The settled requirement items of an `execute` call (FR-963): one FR-621
/// item entry per item, written in place of `items`, and FR-635's `registry`.
#[derive(Clone, Debug, Eq, PartialEq)]
struct RequirementItems {
    entries: Vec<serde_json::Value>,
    registry: serde_json::Value,
}

impl Serialize for OutcomeDocument {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let fields = if self.requirements.is_some() {
            9
        } else if self.empty_analyze_members {
            10
        } else {
            8
        };
        let mut document = serializer.serialize_struct("OutcomeDocument", fields)?;
        document.serialize_field("format", &self.format)?;
        document.serialize_field("operation", &self.operation)?;
        document.serialize_field("last_stage", &self.last_stage)?;
        document.serialize_field("category", self.category.as_str())?;
        if let Some(requirements) = &self.requirements {
            document.serialize_field("items", &requirements.entries)?;
        } else {
            #[cfg(feature = "quire-extraction")]
            if let Some(report) = &self.native_fences {
                document.serialize_field("items", &report.fences)?;
            } else {
                document.serialize_field("items", &self.items)?;
            }
            #[cfg(not(feature = "quire-extraction"))]
            document.serialize_field("items", &self.items)?;
        }
        document.serialize_field("diagnostics", &self.diagnostics)?;
        document.serialize_field("artifacts", &[(); 0])?;
        document.serialize_field("result", &self.result)?;
        if let Some(requirements) = &self.requirements {
            document.serialize_field("registry", &requirements.registry)?;
        } else if self.empty_analyze_members {
            document.serialize_field("registry", &[(); 0])?;
            document.serialize_field(
                "aggregate",
                &serde_json::json!({"status": "incomplete", "gaps": []}),
            )?;
        }
        document.end()
    }
}

/// Why a document could not be written.
#[derive(Debug, thiserror::Error)]
#[error("the outcome document could not be encoded: {0}")]
pub struct OutcomeWriteError(#[from] fsl_foundation::canonical_json::CanonicalJsonError);

impl OutcomeDocument {
    /// This document with `last_stage` (FR-286): the greatest stage it entered.
    #[must_use]
    pub fn with_last_stage(mut self, last_stage: Option<OutcomeStage>) -> Self {
        self.last_stage = last_stage;
        self
    }

    /// One check document carrying an independent result for every native fence.
    #[cfg(feature = "quire-extraction")]
    pub fn from_native_fences(report: &crate::spine::NativeFenceReport) -> Self {
        let category =
            Category::most_severe(report.fences.iter().map(|fence| fence.category.exit_code()))
                .and_then(|worst| {
                    report
                        .fences
                        .iter()
                        .find(|fence| fence.category.exit_code() == worst)
                })
                .map_or(Category::Success, |fence| fence.category);
        let mut document = Self::new(Operation::Check, Some(OutcomeStage::S4), category);
        document.native_fences = Some(report.clone());
        document
    }
    /// A document of `operation` that last reached `last_stage` with the
    /// outcome's `category`, and no items, diagnostics or artifacts.
    pub fn new(operation: Operation, last_stage: Option<OutcomeStage>, category: Category) -> Self {
        Self {
            #[cfg(feature = "quire-extraction")]
            native_fences: None,
            format: OUTCOME_FORMAT,
            operation,
            last_stage,
            category,
            items: Vec::new(),
            diagnostics: Vec::new(),
            result: None,
            requirements: None,
            empty_analyze_members: false,
        }
    }

    /// A document of `operation` over its requested `items`, in request
    /// order. Its category is the most severe item's under ADR-029 CB-4
    /// (FR-285's order 30, 20, 21, 22, 10, 11, 0), the first such item's on a
    /// tie, and `success` with no items. A clause still pending when the
    /// trace ends has the category `pending`, whose catalog exit is 0.
    pub fn settled(
        operation: Operation,
        last_stage: Option<OutcomeStage>,
        items: Vec<OutcomeItem>,
    ) -> Self {
        let code = |item: &OutcomeItem| item.category.exit_code();
        let category = Category::most_severe(items.iter().map(code))
            .and_then(|worst| items.iter().find(|item| code(item) == worst))
            .map_or(Category::Success, |item| item.category);
        Self::new(operation, last_stage, category).with_items(items)
    }

    /// The `monitor` document of `runs`, one item per clause run in request
    /// order: each run is `(clause, report)`, its request index its position
    /// in `runs`. The category is [`Self::settled`]'s, so a document whose
    /// only item is a pending clause is pending, exit 0 (FR-286-AC-8).
    ///
    /// # Errors
    ///
    /// [`ClauseRunNotAnItem`] for the first run that is an evaluator fault or
    /// inconclusive: the request fails as a whole (FR-283).
    pub fn from_clause_runs<'a>(
        last_stage: Option<OutcomeStage>,
        runs: impl IntoIterator<Item = (&'a str, &'a ClauseRunReport)>,
    ) -> Result<Self, ClauseRunNotAnItem> {
        let items = runs
            .into_iter()
            .enumerate()
            .map(|(index, (clause, report))| {
                OutcomeItem::from_clause_run(RequestIndex::new(index), clause, report)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self::settled(Operation::Monitor, last_stage, items))
    }

    /// This document with one entry per requested item, in request order.
    #[must_use]
    pub fn with_items(mut self, items: Vec<OutcomeItem>) -> Self {
        #[cfg(feature = "quire-extraction")]
        {
            self.native_fences = None;
        }
        self.items = items;
        self
    }

    /// This `export` document carrying FSpec FR-817's outcome `export` as its
    /// `exported` result (FR-286 "Export outcomes").
    #[must_use]
    pub fn with_export(mut self, export: serde_json::Value) -> Self {
        self.result = Some(OutcomeResult::Exported(ExportedOutcome {
            kind: "exported",
            export,
        }));
        self
    }

    /// This document with its diagnostics.
    #[must_use]
    pub fn with_diagnostics(mut self, diagnostics: Vec<OutcomeDiagnostic>) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    /// This `analyze` document of a whole-request refusal, written with the
    /// empty `aggregate` (no gaps) and empty `registry` FR-621 requires of
    /// every `analyze` document, so it validates against the published schema.
    #[must_use]
    pub fn with_empty_analyze_members(mut self) -> Self {
        self.empty_analyze_members = self.operation == Operation::Analyze;
        self
    }

    /// This `execute` document with the settled requirement items of its
    /// call (FR-963): `items` holds one FR-621 item `entries` entry per item,
    /// in package order, `registry` is FR-635's `{backend, tool}` list the
    /// items were negotiated against, and `category` is the most severe of
    /// the call's and the items' categories. `result` stays the call's.
    #[must_use]
    pub fn with_requirement_items(
        mut self,
        category: Category,
        entries: Vec<serde_json::Value>,
        registry: serde_json::Value,
    ) -> Self {
        self.category = category;
        self.requirements = Some(Box::new(RequirementItems { entries, registry }));
        self
    }

    /// The outcome's O-16 category, from which FR-285 derives the exit code.
    pub fn category(&self) -> Category {
        self.category
    }

    /// The document's terminal-record items, in request order.
    /// Native fence check items are available through `native_fences`.
    pub fn items(&self) -> &[OutcomeItem] {
        &self.items
    }

    /// Native fence check items, when this is a Markdown check document.
    #[cfg(feature = "quire-extraction")]
    pub fn native_fences(&self) -> Option<&crate::spine::NativeFenceReport> {
        self.native_fences.as_ref()
    }

    /// The document's diagnostics.
    pub fn diagnostics(&self) -> &[OutcomeDiagnostic] {
        &self.diagnostics
    }

    /// The document of a `parse` outcome. Completed S1/S2 work carries its
    /// parsed report, including when S2 later refuses recovered syntax.
    pub fn from_parse(result: &Result<Staged<ParsedUnitSource>, FrontEndFailure>) -> Self {
        match result {
            Ok(staged) => {
                let parsed = crate::spine::ParseReport {
                    source: staged.value().source().reference().clone(),
                    path: staged.value().source().path().to_owned(),
                    bytes: staged.value().source().text().len(),
                    limits: staged.value().syntax().effective_limits(),
                    tokens: staged.value().syntax().cst().tokens().len(),
                    nodes: staged.value().syntax().cst().nodes().len(),
                    work_units: staged.work().s1,
                };
                let mut document =
                    Self::new(Operation::Parse, Some(OutcomeStage::S2), Category::Success);
                document.result = Some(OutcomeResult::Parsed(ParsedOutcome::from(&parsed)));
                document
            }
            Err(failure) => {
                let mut document = Self::from_failure(Operation::Parse, failure);
                let parsed = match failure {
                    StageFailure::RefusedWithPayload(_, payload)
                    | StageFailure::LimitWithPayload(_, payload) => payload.parsed.as_deref(),
                    _ => None,
                };
                document.result =
                    parsed.map(|parsed| OutcomeResult::Parsed(ParsedOutcome::from(parsed)));
                document
            }
        }
    }

    /// The document of a `format` outcome. The complete UTF-8 text is carried
    /// in the formatted result; S2 is the reporting default, not a stage run
    /// by this adapter.
    /// Refusals retain their original typed diagnostic and catalog category;
    /// internal failures report no stage (FR-286).
    pub fn from_format<E>(result: &Result<Staged<String>, E>) -> Self
    where
        for<'a> &'a E: Into<FormatOutcomeRefusal>,
    {
        match result {
            Ok(staged) => {
                let mut document =
                    Self::new(Operation::Format, Some(OutcomeStage::S2), Category::Success);
                document.result = Some(OutcomeResult::Formatted(FormattedOutcome {
                    kind: "formatted",
                    text: staged.value().clone(),
                }));
                document
            }
            Err(refusal) => {
                let refusal: FormatOutcomeRefusal = refusal.into();
                let stage = if refusal.category == Category::InternalFailure {
                    None
                } else {
                    Some(OutcomeStage::S2)
                };
                Self::new(Operation::Format, stage, refusal.category)
                    .with_diagnostics(vec![refusal.diagnostic])
            }
        }
    }

    /// The document of a `check` outcome: success at S4, which mints no
    /// identity, so no artifact; a failure names its category and retains
    /// every diagnostic the refused stage reported.
    pub fn from_check(result: &Result<Staged<CheckedUnit>, FrontEndFailure>) -> Self {
        match result {
            Ok(_) => Self::new(Operation::Check, Some(OutcomeStage::S4), Category::Success),
            Err(failure) => Self::from_failure(Operation::Check, failure),
        }
    }

    /// The document of a `package` outcome: success at S4.
    pub fn from_package(result: &Result<Staged<EmittedUnit>, FrontEndFailure>) -> Self {
        match result {
            Ok(_) => Self::new(
                Operation::Package,
                Some(OutcomeStage::S4),
                Category::Success,
            ),
            Err(failure) => Self::from_failure(Operation::Package, failure),
        }
    }

    /// The document of a front-end operation's failure.
    fn from_failure(operation: Operation, failure: &FrontEndFailure) -> Self {
        match failure {
            StageFailure::Refused(refusal) => Self::new(
                operation,
                Some(refusal.stage().into()),
                refusal.code().category(),
            )
            .with_diagnostics(
                OutcomeDiagnostic::from_compile_refusal_diagnostics_for_operation(
                    operation, refusal,
                ),
            ),
            StageFailure::RefusedWithPayload(refusal, _) => Self::new(
                operation,
                Some(refusal.stage().into()),
                refusal.code().category(),
            )
            .with_diagnostics(
                OutcomeDiagnostic::from_compile_refusal_diagnostics_for_operation(
                    operation, refusal,
                ),
            ),
            StageFailure::Limit(limit) => {
                let refusal = CompileRefusal::Limit(limit.clone());
                Self::new(
                    operation,
                    Some(refusal.stage().into()),
                    Category::Incomplete,
                )
                .with_diagnostics(vec![
                    OutcomeDiagnostic::from_compile_refusal_for_operation(operation, &refusal),
                ])
            }
            StageFailure::LimitWithPayload(limit, _) => {
                let refusal = CompileRefusal::Limit(limit.clone());
                Self::new(
                    operation,
                    Some(refusal.stage().into()),
                    Category::Incomplete,
                )
                .with_diagnostics(vec![
                    OutcomeDiagnostic::from_compile_refusal_for_operation(operation, &refusal),
                ])
            }
            StageFailure::Cancelled(cause) => Self::cancelled(operation, *cause),
            StageFailure::Fault(fault) => Self::faulted(operation, fault),
        }
    }

    /// The document of an `operation` whose stage stopped without a result:
    /// a reached limit is incomplete at its stage (as `check` and `package`
    /// write it), a cancel is incomplete at no stage with the cancel's cause,
    /// and a fault is an internal failure at no stage (FR-286).
    pub fn from_stopped(operation: Operation, stopped: Stopped) -> Self {
        match stopped {
            Stopped::Limit(limit) => {
                let refusal = CompileRefusal::Limit(limit);
                Self::new(
                    operation,
                    Some(refusal.stage().into()),
                    Category::Incomplete,
                )
                .with_diagnostics(vec![
                    OutcomeDiagnostic::from_compile_refusal_for_operation(operation, &refusal),
                ])
            }
            Stopped::Cancelled(cause) => Self::cancelled(operation, cause),
            Stopped::Fault(fault) => Self::faulted(operation, &fault),
        }
    }

    /// The document of a cancelled `operation`: incomplete, at no stage, with
    /// the cancel's cause as the diagnostic's.
    fn cancelled(operation: Operation, cause: CancelCause) -> Self {
        Self::new(operation, None, Category::Incomplete).with_diagnostics(vec![
            OutcomeDiagnostic::new(
                Some(match cause {
                    CancelCause::Requested => "requested",
                    CancelCause::Deadline => "deadline",
                }),
                Code::Cancelled.as_str(),
                None,
                "the operation was cancelled",
            ),
        ])
    }

    /// The document of an `operation` that hit a broken internal invariant:
    /// internal failure, at no stage, with the fault's catalog code.
    fn faulted(
        operation: Operation,
        fault: &filament_semantic_value::catalog::InternalFault,
    ) -> Self {
        Self::new(operation, None, fault.category()).with_diagnostics(vec![OutcomeDiagnostic::new(
            Some(fault.catalog_code().cause()),
            fault.catalog_code().code(),
            None,
            format!(
                "internal invariant {} broken in {}",
                fault.invariant(),
                fault.stage()
            ),
        )])
    }

    /// The document of a `replay` outcome: a settled arm's category at S8,
    /// or the refusal's category and diagnostic.
    pub fn from_replay(result: &Result<ReplayResult, ReplayRefusal>) -> Self {
        match result {
            Ok(arm) => {
                let row = arm
                    .to_function_json()
                    .and_then(|row| serde_json::from_str(&row).map_err(Into::into));
                Self::from_replay_row(arm, 0, row)
            }
            Err(refusal) => Self::from_replay_refusal(refusal),
        }
    }

    /// A successful replay outcome document with its already encoded result
    /// row. Callers that need the row in a separate result document can encode
    /// it once and pass the parsed value here.
    pub fn from_replay_with_row(result: &ReplayResult, row: serde_json::Value) -> Self {
        Self::from_replay_row(result, 0, Ok(row))
    }

    /// An FR-286 internal-failure outcome when one FR-323 result row failed
    /// to serialize. The row position identifies which row could not be
    /// written; no partial replay result is retained.
    pub fn from_replay_output_failure(
        row_position: usize,
        error: &crate::result::CauseCodecError,
    ) -> Self {
        replay_row_output_failure(row_position, error)
    }

    /// Supply a raw result solely to exercise malformed outcome handling in
    /// downstream test targets.
    #[cfg(feature = "test-support")]
    #[doc(hidden)]
    pub fn with_raw_result_for_test(mut self, result: serde_json::Value) -> Self {
        self.result = Some(OutcomeResult::Raw(result));
        self
    }

    /// The document of a refused replay.
    pub fn from_replay_refusal(refusal: &ReplayRefusal) -> Self {
        Self::from_replay_refusal_at(refusal, None)
    }

    /// The document of a refused replay, with the caller's request path when known.
    pub fn from_replay_refusal_at(refusal: &ReplayRefusal, request_path: Option<&str>) -> Self {
        Self::from_replay_refusal_with_context(refusal, request_path, None)
    }

    /// The document of a refused replay with its request path and package binding.
    pub fn from_replay_refusal_with_context(
        refusal: &ReplayRefusal,
        request_path: Option<&str>,
        artifact_identity: Option<&fsl_foundation::source::provenance::RawSourceRef>,
    ) -> Self {
        let locus = match refusal {
            ReplayRefusal::Request(crate::ReplayRequestRefusal::PackageMember { .. })
            | ReplayRefusal::PackageSources(_)
            | ReplayRefusal::DependencySources { .. }
            | ReplayRefusal::DependencySelections(_)
            | ReplayRefusal::RefinementInternals { .. } => refusal
                .pointer()
                .map(|pointer| request_document_locus(&pointer, request_path)),
            ReplayRefusal::Package(_) => artifact_identity
                .and_then(|identity| refusal.pointer().map(|pointer| (identity, pointer)))
                .map(|(identity, pointer)| OutcomeLocus::Artifact {
                    identity: identity.clone(),
                    pointer: package_artifact_pointer(&pointer),
                }),
            ReplayRefusal::PackageWithIdentity { identity, .. } => {
                identity.as_ref().and_then(|identity| {
                    refusal.pointer().map(|pointer| OutcomeLocus::Artifact {
                        identity: identity.clone(),
                        pointer: package_artifact_pointer(&pointer),
                    })
                })
            }
            _ => None,
        };
        Self::new(
            Operation::Replay,
            replay_stage(refusal),
            refusal.code().category(),
        )
        .with_diagnostics(vec![OutcomeDiagnostic::new(
            refusal.cause(),
            refusal.code().as_str(),
            locus,
            refusal.to_string(),
        )])
    }

    fn from_replay_row(
        arm: &ReplayResult,
        row_position: usize,
        row: Result<serde_json::Value, crate::result::CauseCodecError>,
    ) -> Self {
        match row {
            Ok(row) => {
                let mut document =
                    Self::new(Operation::Replay, Some(OutcomeStage::S8), arm.category());
                document.result = Some(OutcomeResult::Replayed(ReplayedOutcome {
                    kind: "replayed",
                    results: vec![row],
                }));
                document
            }
            Err(error) => replay_row_output_failure(row_position, &error),
        }
    }

    /// The document of an `execute` outcome, last reaching S6a. A refused
    /// call carries its catalog code, cause and locus as one diagnostic; a
    /// completed or undefined call carries its `result`. An undefined call has category `undefined`, the one place that label
    /// appears.
    pub fn from_call(outcome: &CallOutcome) -> Self {
        let mut document = Self::new(
            Operation::Execute,
            Some(OutcomeStage::S6a),
            outcome.category(),
        );
        match outcome {
            CallOutcome::Refused(refusal) => {
                document.diagnostics = vec![OutcomeDiagnostic::from_call_refusal(refusal)];
            }
            CallOutcome::Completed(value) => {
                document.result = Some(OutcomeResult::Execute(ExecuteResult::Completed {
                    value: ResultValue(value.form()),
                }));
            }
            CallOutcome::Undefined { reason, .. } => {
                document.result = Some(OutcomeResult::Execute(ExecuteResult::Undefined { reason }));
            }
            CallOutcome::Incomplete(incomplete) => {
                document.result = Some(OutcomeResult::Execute(ExecuteResult::Incomplete {
                    limit: ResultLimit::from(incomplete),
                }));
            }
        }
        document
    }

    /// The document of a `run`: a call that ran is [`Self::from_call`]'s
    /// document; a refusal is an
    /// `execute` document at the stage it reached with the refusal's
    /// category and every retained diagnostic; a cancel is incomplete; a
    /// fault is internal failure with its catalog code.
    pub fn from_run(result: &Result<CallOutcome, Box<RunRefusal>>) -> Self {
        match result {
            Ok(outcome) => Self::from_call(outcome),
            Err(refusal) => Self::from_run_refusal(Operation::Execute, refusal),
        }
    }

    /// Project an existing run refusal without taking ownership or executing it.
    /// The caller names the operation whose pipeline produced the refusal.
    pub fn from_run_refusal(operation: Operation, refusal: &RunRefusal) -> Self {
        let mut document = Self::run_refusal_document(operation, refusal);
        if let Some(details) = refusal.details() {
            document.diagnostics = std::mem::take(&mut document.diagnostics)
                .into_iter()
                .map(|diagnostic| diagnostic.with_details(Some(details.clone())))
                .collect();
        }
        document
    }

    fn run_refusal_document(operation: Operation, refusal: &RunRefusal) -> Self {
        match refusal {
            RunRefusal::Compile(compile) => {
                Self::new(operation, Some(compile.stage().into()), refusal.category())
                    .with_diagnostics(
                        OutcomeDiagnostic::from_compile_refusal_diagnostics_for_operation(
                            operation, compile,
                        ),
                    )
            }
            RunRefusal::Cancelled(cause) => Self::cancelled(operation, *cause),
            RunRefusal::Fault(fault) => Self::faulted(operation, fault),
            // FR-277: a supplied-phase stop is phase-marked, not a
            // catalog record. Its category is the document's; the shared
            // result contract has no field for the phase context yet, so
            // nothing is invented for it here.
            RunRefusal::Supplied(stop) => match stop.cause {
                filament_semantic_value::supplied_stop::SuppliedCause::Cancelled(cause) => {
                    Self::cancelled(operation, cause)
                }
                filament_semantic_value::supplied_stop::SuppliedCause::Limit(_) => {
                    Self::new(operation, Some(OutcomeStage::S6a), refusal.category())
                }
                filament_semantic_value::supplied_stop::SuppliedCause::Input { .. }
                | filament_semantic_value::supplied_stop::SuppliedCause::Storage(_)
                | filament_semantic_value::supplied_stop::SuppliedCause::Capacity(_) => {
                    Self::new(operation, Some(OutcomeStage::S6a), refusal.category())
                        .with_diagnostics(vec![OutcomeDiagnostic::new(
                            fsl_eval::value::supplied_stop_cause(stop),
                            fsl_eval::value::supplied_stop_code(stop).as_str(),
                            None,
                            refusal.to_string(),
                        )])
                }
            },
            RunRefusal::UnsupportedHelper(_) => {
                Self::new(operation, Some(OutcomeStage::S6a), refusal.category())
            }
            RunRefusal::ObjectEnvironment(record) => {
                Self::new(operation, Some(OutcomeStage::S6a), refusal.category()).with_diagnostics(
                    vec![OutcomeDiagnostic::new(
                        Some(record.catalog.cause()),
                        refusal.code().as_str(),
                        None,
                        refusal.to_string(),
                    )],
                )
            }
            RunRefusal::Unsupported(entry) => {
                Self::new(operation, Some(OutcomeStage::S6a), refusal.category()).with_diagnostics(
                    vec![OutcomeDiagnostic::new(
                        Some(entry.catalog_code().cause()),
                        refusal.code().as_str(),
                        None,
                        refusal.to_string(),
                    )],
                )
            }
            RunRefusal::SourceNotSupplied { .. } => {
                Self::new(operation, Some(OutcomeStage::S6a), refusal.category()).with_diagnostics(
                    vec![OutcomeDiagnostic::new(
                        Some("missing-member"),
                        refusal.code().as_str(),
                        None,
                        refusal.to_string(),
                    )],
                )
            }
            RunRefusal::Argument { defect, .. } => {
                Self::new(operation, Some(OutcomeStage::S6a), refusal.category()).with_diagnostics(
                    vec![OutcomeDiagnostic::new(
                        Some(defect.cause),
                        refusal.code().as_str(),
                        None,
                        refusal.to_string(),
                    )],
                )
            }
            RunRefusal::UnsupportedResult { .. } | RunRefusal::UnsupportedParameter { .. } => {
                Self::new(operation, Some(OutcomeStage::S6a), refusal.category()).with_diagnostics(
                    vec![OutcomeDiagnostic::new(
                        Some("unsupported-feature"),
                        refusal.code().as_str(),
                        None,
                        refusal.to_string(),
                    )],
                )
            }
            RunRefusal::MissingDeclaration { .. }
            | RunRefusal::UnknownParameter { .. }
            | RunRefusal::DuplicateArgument { .. }
            | RunRefusal::UnboundParameter { .. }
            | RunRefusal::WrongValueKind { .. }
            | RunRefusal::Admission { .. }
            | RunRefusal::Storage { .. }
            | RunRefusal::PrepareLimit(_)
            | RunRefusal::DanglingReference { .. } => {
                Self::new(operation, Some(OutcomeStage::S6a), refusal.category()).with_diagnostics(
                    vec![OutcomeDiagnostic::new(
                        None,
                        refusal.code().as_str(),
                        None,
                        refusal.to_string(),
                    )],
                )
            }
        }
    }

    /// The document's bytes: the same document gives the same bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, OutcomeWriteError> {
        Ok(fsl_foundation::canonical_json::encode(self)?)
    }
}

fn request_document_locus(pointer: &str, request_path: Option<&str>) -> OutcomeLocus {
    let pointer = match request_path {
        Some(_) => format!("/request{pointer}"),
        None => pointer.to_owned(),
    };
    OutcomeLocus::Document {
        document: None,
        path: request_path.map(str::to_owned),
        member: pointer_member(&pointer),
        pointer,
        object: None,
    }
}

fn package_artifact_pointer(pointer: &str) -> String {
    match pointer.strip_prefix("/package") {
        Some("") => String::new(),
        Some(remainder) if remainder.starts_with('/') => remainder.to_owned(),
        _ => pointer.to_owned(),
    }
}

/// The last stage a replay refusal reached: a recompile refusal's own spine
/// stage, S8 for every other refusal (the replay itself refused), and none
/// for a fault, whose stage is untyped.
fn replay_row_output_failure(
    row_position: usize,
    error: &crate::result::CauseCodecError,
) -> OutcomeDocument {
    OutcomeDocument::new(Operation::Replay, None, Category::InternalFailure).with_diagnostics(vec![
        OutcomeDiagnostic::new(
            Some("output-failure"),
            Code::OutputFailure.as_str(),
            None,
            format!("replay result row at position {row_position} did not serialize: {error}"),
        ),
    ])
}

fn replay_stage(refusal: &ReplayRefusal) -> Option<OutcomeStage> {
    match refusal {
        ReplayRefusal::Recompile { refusal, .. } => Some(refusal.stage().into()),
        ReplayRefusal::RecompileWithProtocolResults { refusal, .. }
        | ReplayRefusal::RefinementInternals { refusal, .. } => Some(refusal.stage().into()),
        ReplayRefusal::Fault(_)
        | ReplayRefusal::InvalidPackageName(_)
        | ReplayRefusal::ObservedTemporalRefusal(fsl_eval::temporal::TemporalRefusal::Fault(_))
        | ReplayRefusal::Admission(AdmissionFailure::Fault(_)) => None,
        ReplayRefusal::Request(_)
        | ReplayRefusal::Package(_)
        | ReplayRefusal::PackageWithIdentity { .. }
        | ReplayRefusal::PackageEvidence(_)
        | ReplayRefusal::DomainPackages(_)
        | ReplayRefusal::LimitAboveReader(_)
        | ReplayRefusal::SourceCount(_)
        | ReplayRefusal::PackageSources(_)
        | ReplayRefusal::PackageRead(_)
        | ReplayRefusal::DependencySources { .. }
        | ReplayRefusal::DependencySelections(_)
        | ReplayRefusal::DependencyInput(_)
        | ReplayRefusal::UnknownFunction { .. }
        | ReplayRefusal::UnknownOwner { .. }
        | ReplayRefusal::NotImportable { .. }
        | ReplayRefusal::ImportedFunctionTarget { .. }
        | ReplayRefusal::ClaimOccurrence { .. }
        | ReplayRefusal::UnknownOperation { .. }
        | ReplayRefusal::WrongReplayTarget { .. }
        | ReplayRefusal::WrongObservedTemporalTarget { .. }
        | ReplayRefusal::WrongObservedTemporalSource
        | ReplayRefusal::WrongModelGraphEntry
        | ReplayRefusal::ModelGraphMismatch { .. }
        | ReplayRefusal::WrongModelGraphSource
        | ReplayRefusal::WrongProtocolAnalysisEntry
        | ReplayRefusal::TargetMismatch { .. }
        | ReplayRefusal::MalformedOperationTarget { .. }
        | ReplayRefusal::AmbiguousOperation { .. }
        | ReplayRefusal::FrameIdentity(_)
        | ReplayRefusal::ClauseIdentity(_)
        | ReplayRefusal::UnknownClause { .. }
        | ReplayRefusal::WrongObservation { .. }
        | ReplayRefusal::Admission(_)
        | ReplayRefusal::UnusedObjectEnvironment
        | ReplayRefusal::UnusedProfileSelections { .. }
        | ReplayRefusal::ForeignReference { .. }
        | ReplayRefusal::ObjectEnvironment(_)
        | ReplayRefusal::UnknownParameter(_)
        | ReplayRefusal::DuplicateArgument(_)
        | ReplayRefusal::UnboundParameter(_)
        | ReplayRefusal::Witness { .. }
        | ReplayRefusal::Storage { .. }
        | ReplayRefusal::NotAPredicate { .. }
        | ReplayRefusal::NotAValueFunction { .. }
        | ReplayRefusal::ScalarIdentity(_)
        | ReplayRefusal::ParityBound(_)
        | ReplayRefusal::Supplied(_)
        | ReplayRefusal::UnsupportedHelper(_)
        | ReplayRefusal::Cancelled(_)
        | ReplayRefusal::NativePayload { .. }
        | ReplayRefusal::ModelTrace(_)
        | ReplayRefusal::HybridTrace(_)
        | ReplayRefusal::HybridCertificate(_)
        | ReplayRefusal::UnfairLasso { .. }
        | ReplayRefusal::MissingFairnessPremise { .. }
        | ReplayRefusal::ObservedTemporalMismatch { .. }
        | ReplayRefusal::ObservedTemporalInput { .. }
        | ReplayRefusal::ObservedTemporalRefusal(_)
        | ReplayRefusal::Input(_) => Some(OutcomeStage::S8),
    }
}

fn category_str<S: Serializer>(category: &Category, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(category.as_str())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use fsl_foundation::{BackendId, Position, RequestIndex, SourceIdentity};
    use ix_trace_rs::trace;

    use filament_exact::{Cancel, CancelCause, Identifier, ScalarLimits};
    use filament_semantic_value::catalog::{CatalogCode, InternalFault};
    use fsl_foundation::diagnostic::LimitExceeded;
    use fsl_foundation::Setting;
    use fsl_semantics::model::object_environment::ObjectEnvironment;
    use serde_json::{json, Value};

    use super::*;
    use crate::identity::QualifiedName;

    /// FR-286-AC-28: each diagnostic constructor attaches the exact catalog hint.
    #[trace("TC-3112", "FR-286-AC-28")]
    #[test]
    fn outcome_diagnostic_construction_looks_up_hints_by_exact_pair() {
        let typed = OutcomeDiagnostic::new(
            Some("type-mismatch"),
            Code::IllTyped.as_str(),
            None,
            "types differ",
        );
        assert_eq!(
            typed.hint,
            hint::hint_for(Code::IllTyped, Some("type-mismatch"))
        );

        let io = OutcomeDiagnostic::new(None, Code::IoError.as_str(), None, "cannot read file");
        assert_eq!(io.hint, hint::hint_for(Code::IoError, None));

        let generic =
            OutcomeDiagnostic::new(None, Code::InvalidRequest.as_str(), None, "invalid request");
        assert_eq!(generic.hint, None);
    }

    /// FR-181 source transitions retain their canonical operation node when
    /// the checked identity carries a node/package pair instead of a name.
    #[trace("FR-237-AC-10", "FSpec-FR-364-AC-23")]
    #[test]
    fn task_source_transition_wire_accepts_canonical_operation_nodes() {
        let transition = filament_semantic_value::model_transition::ModelTransition::new(
            r#"{"node":"ix://test/timed/Call/send","package":"test/timed"}"#.to_owned(),
            None,
            Vec::new(),
            quire_canonical::Limits::new(u64::MAX),
        )
        .expect("valid canonical model transition");

        let wire = task_source_transition_wire(&transition).expect("canonical operation node");

        assert_eq!(wire["operation"], "ix://test/timed/Call/send");
    }

    #[trace("FR-237-AC-10")]
    #[test]
    fn task_source_transition_wire_preserves_other_canonical_operation_identities() {
        let transition = filament_semantic_value::model_transition::ModelTransition::new(
            "[]".to_owned(),
            None,
            Vec::new(),
            quire_canonical::Limits::new(u64::MAX),
        )
        .expect("canonical operation identity");

        let wire = task_source_transition_wire(&transition).expect("canonical operation identity");

        assert_eq!(wire["operation"], "[]");
    }

    /// Trace: FR-255-AC-3, the operation hint follows the entry point's accepted inputs.
    #[trace("TC-931", "FR-255-AC-3")]
    #[test]
    fn run_limit_message_omits_stage_limits_while_replay_keeps_it() {
        let run_refusal = RunRefusal::Compile(Box::new(CompileRefusal::Limit(LimitExceeded::new(
            Setting::S1Tokens,
            3,
            4,
        ))));
        let run = OutcomeDocument::from_run_refusal(Operation::Execute, &run_refusal);
        let run_message = run.diagnostics()[0].message();
        assert!(run_message.contains("--limit s1.tokens"), "{run_message}");
        assert!(!run_message.contains("stage_limits"), "{run_message}");

        let replay_refusal = ReplayRefusal::Recompile {
            refusal: Box::new(CompileRefusal::Limit(LimitExceeded::new(
                Setting::S1Tokens,
                3,
                4,
            ))),
            pointer: None,
        };
        let replay = OutcomeDocument::from_replay(&Err(replay_refusal));
        let replay_message = replay.diagnostics()[0].message();
        assert!(replay_message.contains("stage_limits"), "{replay_message}");
    }

    #[test]
    fn timed_statistical_refusal_carries_typed_state_and_transition() {
        let state = fsl_eval::simulation::StateKey::from_canonical_bytes(b"[0,[]]".to_vec());
        let transition = filament_semantic_value::model_transition::ModelTransition::new(
            r#"{"name":"timeout"}"#.to_owned(),
            None,
            vec![r#"{"type":"integer","value":"1"}"#.to_owned()],
            quire_canonical::Limits::new(u64::MAX),
        )
        .expect("canonical transition values");
        let mut item = ItemCause::kind("not-stochastic");
        item.state = Some((&state).into());
        item.identity = Some((&transition).into());

        let value = serde_json::to_value(item).expect("wire cause serializes");
        assert_eq!(value["kind"], "not-stochastic");
        assert_eq!(value["state"]["key"], "WzAsW11d");
        assert_eq!(value["identity"]["type"], "transition");
        assert_eq!(
            value["identity"]["operation"],
            serde_json::json!({"name":"timeout"})
        );
        assert_eq!(value["identity"]["receiver"], serde_json::Value::Null);
        assert_eq!(value["identity"]["arguments"][0]["value"], "1");
    }

    #[trace("TC-639", "FR-204-AC-2", "FR-204-AC-3", "FR-204-AC-4")]
    #[test]
    fn digital_unsupported_causes_keep_their_detail_on_the_wire() {
        let cases = [
            (
                UnavailabilityCause::StrictClockConstraint {
                    span: fsl_foundation::Span { start: 8, end: 17 },
                },
                "strict-clock-constraint",
                "8..17",
                None,
            ),
            (
                UnavailabilityCause::DelayDistribution {
                    operation: "M::Call::send".to_owned(),
                },
                "delay-distribution",
                "M::Call::send",
                None,
            ),
            (
                UnavailabilityCause::ZeroDelayCycle {
                    states: vec!["product-state-a".to_owned(), "product-state-b".to_owned()],
                },
                "zero-delay-cycle",
                "3",
                Some(vec!["product-state-a", "product-state-b"]),
            ),
        ];

        for (cause, kind, detail, states) in cases {
            let record = TerminalRecord::unsupported(
                RequestIndex::new(0),
                BackendId::new("fsl-en-5"),
                cause,
            );
            let wire = serde_json::to_value(OutcomeItem::from_terminal(&record))
                .expect("the FR-331 item serializes");
            assert_eq!(wire["cause"]["kind"], kind);
            if kind == "strict-clock-constraint" {
                assert_eq!(wire["cause"]["where"], detail);
            } else if kind == "delay-distribution" {
                assert_eq!(wire["cause"]["operation"], detail);
            } else {
                assert_eq!(wire["cause"]["states"], serde_json::json!(states.unwrap()));
            }
        }

        let timed_form = TerminalRecord::unsupported(
            RequestIndex::new(1),
            BackendId::new("fsl-en-5"),
            UnavailabilityCause::TimedFormShape {
                span: fsl_foundation::Span { start: 20, end: 32 },
                claim: None,
                member: None,
            },
        );
        let wire = serde_json::to_value(OutcomeItem::from_terminal(&timed_form))
            .expect("the FR-331 item serializes");
        assert_eq!(wire["cause"]["kind"], "timed-form-shape");
        assert_eq!(wire["cause"]["where"], "20..32");
    }

    /// QSL-381: a node id nested 200 deep, past serde_json's 128-level
    /// recursion limit, writes and reads back without a depth cap.
    #[test]
    fn a_wire_node_nested_two_hundred_deep_round_trips() {
        let mut key = NodeKey::new(
            "fixture",
            vec![filament_exact::PathSegment::Text("leaf".into())],
            0,
        )
        .expect("a fixture key");
        for ordinal in 1..=200 {
            key = NodeKey::new(
                "fixture",
                vec![filament_exact::PathSegment::Node(key)],
                ordinal,
            )
            .expect("a nested key");
        }
        let text = serde_json::to_string(&WireNode::of(&key)).expect("writes");
        let mut deserializer = serde_json::Deserializer::from_str(&text);
        deserializer.disable_recursion_limit();
        let read = <WireNode as serde::Deserialize>::deserialize(&mut deserializer).expect("reads");
        assert_eq!(read.into_key(), key);
    }
    use crate::proof_result::{IncompleteCause, ProofRefusalCause, UnavailabilityCause};
    use crate::result::{DisagreementCause, EvaluatedValue, InputArmResult, Verdict, WitnessCheck};
    use crate::spine::{
        check, default_accounting, package, parse, run, select, Call, CallLocus, DependencyInput,
        PackageLimits, ParseRequest, SpineLimits,
    };

    const FIXTURE: &str = include_str!("../../tests/fixtures/spine-compile.native");
    const ILL_TYPED: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.value.complete/v1\";\n\
        type Digit = Int[0, 9];\n\
        function inv using v(x: Digit): Boolean pure { 1 / x > 0 }\n";

    type Checked = Result<Staged<CheckedUnit>, FrontEndFailure>;
    type Packaged = Result<Staged<EmittedUnit>, FrontEndFailure>;

    /// `check`, then `package`, over `text`.
    fn compiled(text: &str) -> (Checked, Option<Packaged>) {
        let source = SourceIdentity::new("agent-ix", "test:outcome");
        let limits = SpineLimits::default();
        let cancel = Cancel::new();
        let request = ParseRequest {
            source: &source,
            path: "unit.native",
            bytes: text.as_bytes(),
        };
        let parsed = parse(&request, limits.source, &cancel)
            .expect("parses")
            .into_value();
        let models = select(&parsed, &BTreeMap::new(), limits.model, &cancel)
            .expect("selects")
            .into_value();
        let checked = check(
            &parsed,
            "fixture".to_owned(),
            &models,
            &DependencyInput::default(),
            limits,
            &cancel,
        );
        let emitted = checked
            .as_ref()
            .ok()
            .map(|staged| package(staged.value(), PackageLimits::default(), &cancel));
        (checked, emitted)
    }

    fn json_of(document: &OutcomeDocument) -> Value {
        serde_json::from_slice(&document.to_bytes().expect("encodes")).expect("is JSON")
    }

    fn text_of(document: &OutcomeDocument) -> String {
        String::from_utf8(document.to_bytes().expect("encodes")).expect("is UTF-8")
    }

    fn clause_document(disposition: ClauseDisposition) -> Value {
        let report = ClauseRunReport {
            disposition,
            usage: Default::default(),
            basis: crate::proof_result::SettlementBasis::Unavailable,
            witness: None,
            state_clause_counterexample: None,
            frame_field_values: None,
            consequent_source_events: Default::default(),
            ledger: None,
            finite_window: None,
            counterexample: None,
            decisive: None,
            authenticated_temporal_selection: None,
            refused_at: None,
        };
        let document =
            OutcomeDocument::from_clause_runs(Some(OutcomeStage::S6c), [("claim", &report)])
                .expect("refusal and undefined dispositions settle as items");
        json_of(&document)
    }

    /// FR-867-AC-3: an undefined function claim carries its retained
    /// expression location in `where` and its evaluator reason in `cause`.
    #[trace("TC-2951", "FR-867-AC-3", "FR-286-AC-13")]
    #[test]
    fn undefined_claim_item_serializes_location_and_cause() {
        use filament_semantic_value::location::{ExpressionLocation, ExpressionOwner};

        let location = ExpressionLocation::root(ExpressionOwner::Body {
            function: "claim".to_owned(),
            index: 0,
        });
        let document = clause_document(ClauseDisposition::Evaluate(CallOutcome::Undefined {
            reason: "division-by-zero",
            location: Some(location),
        }));
        assert_eq!(
            document["items"][0]["cause"],
            json!({
                "kind": "undefined-evaluation",
                "where": {
                    "declaration": {"kind": "body", "function": "claim", "index": 0},
                    "path": [],
                },
                "cause": "division-by-zero",
            })
        );
    }

    /// FR-867-AC-4: an integer function claim emits its type-mismatch detail.
    #[trace("TC-2951", "FR-867-AC-4", "FR-286-AC-13")]
    #[test]
    fn integer_claim_item_serializes_diagnostic_detail() {
        let document = clause_document(ClauseDisposition::NotAPredicate {
            name: "integerClaim".to_owned(),
            actual: "Integer".to_owned(),
            result_type_locus: None,
        });
        assert_eq!(
            document["items"][0]["cause"],
            json!({"kind": "type-mismatch", "code": "ill_typed"})
        );
        assert_eq!(
            document["items"][0]["diagnostic"],
            json!({
                "code": "ill_typed",
                "cause": "type-mismatch",
                "detail": {
                    "function": "integerClaim",
                    "expected": "Boolean",
                    "actual": "Integer",
                },
            })
        );
    }

    /// FR-867-AC-4: a bounded integer function claim preserves its declared
    /// result type in its detail.
    #[trace("TC-2951", "FR-867-AC-4", "FR-286-AC-13")]
    #[test]
    fn bounded_claim_item_serializes_diagnostic_detail() {
        let document = clause_document(ClauseDisposition::NotAPredicate {
            name: "boundedClaim".to_owned(),
            actual: "Int[0, 3]".to_owned(),
            result_type_locus: None,
        });
        assert_eq!(
            document["items"][0]["cause"],
            json!({"kind": "type-mismatch", "code": "ill_typed"})
        );
        assert_eq!(
            document["items"][0]["diagnostic"]["detail"]["actual"],
            "Int[0, 3]"
        );
    }

    /// FR-867-AC-4: an unknown function claim emits missing-name detail.
    #[trace("TC-2951", "FR-867-AC-4", "FR-286-AC-13")]
    #[test]
    fn unknown_claim_item_serializes_diagnostic_detail() {
        let document = clause_document(ClauseDisposition::MissingName {
            name: "missingClaim".to_owned(),
        });
        assert_eq!(
            document["items"][0]["cause"],
            json!({"kind": "missing-name", "code": "missing_declaration"})
        );
        assert_eq!(
            document["items"][0]["diagnostic"],
            json!({
                "code": "missing_declaration",
                "cause": "missing-name",
                "detail": {"name": "missingClaim"},
            })
        );
    }

    fn record(index: usize, value: TerminalValue) -> OutcomeItem {
        OutcomeItem::from_terminal(&TerminalRecord::from_value(RequestIndex::new(index), value))
    }

    /// FR-990 writes the structured result members on the settled item while
    /// retaining its terminal label and category.
    #[trace("TC-2800", "FR-990-AC-1")]
    #[test]
    fn protocol_conformance_members_serialize_at_item_level() {
        let item =
            record(0, TerminalValue::Tested).with_protocol_conformance(ProtocolConformanceOutput {
                obligations: vec![ConformanceObligationOutput {
                    obligation: json!({"kind": "closure"}),
                    verdict: "satisfied".to_owned(),
                    cause: None,
                    basis: "explored".to_owned(),
                    witness: None,
                }],
                closure: ConformanceClosureOutput {
                    state: "closed".to_owned(),
                    premises: vec![json!({"kind": "execution", "identity": "w-1"})],
                    producer: "engine".to_owned(),
                },
                global: ConformanceGlobalOutput {
                    label: "proved".to_owned(),
                    basis: "closed-scope".to_owned(),
                    complete: true,
                    cause: None,
                },
                max_messages: None,
                projections: Some(vec![ConformanceProjectionOutput {
                    role: "Buyer".to_owned(),
                    label: "proved".to_owned(),
                    counterexample: None,
                }]),
                recording: None,
                replay: Some(ConformanceReplayOutput { reproduced: true }),
            });
        let wire = serde_json::to_value(item).expect("serializes");
        assert_eq!(wire["result"], "tested");
        assert_eq!(wire["obligations"][0]["verdict"], "satisfied");
        assert_eq!(wire["closure"]["state"], "closed");
        assert_eq!(wire["global"]["complete"], true);
        assert_eq!(wire["projections"][0]["role"], "Buyer");
        assert_eq!(wire["replay"]["reproduced"], true);
    }

    /// FR-286-AC-1: `check` over the spine fixture is one whole document, at
    /// S4 with no diagnostics; the package identity is `package`'s.
    #[trace("TC-770", "FR-286-AC-1")]
    #[test]
    fn check_and_package_documents_over_the_fixture() {
        let (checked, emitted) = compiled(FIXTURE);
        assert_eq!(
            json_of(&OutcomeDocument::from_check(&checked)),
            json!({
                "format": "filament-outcome/1",
                "operation": "check",
                "last_stage": "S4",
                "category": "success",
                "items": [],
                "diagnostics": [],
                "artifacts": [],
                "result": null,
            })
        );
        let emitted = emitted.expect("the fixture checks");
        assert_eq!(
            json_of(&OutcomeDocument::from_package(&emitted)),
            json!({
                "format": "filament-outcome/1",
                "operation": "package",
                "last_stage": "S4",
                "category": "success",
                "items": [],
                "diagnostics": [],
                "artifacts": [],
                "result": null,
            })
        );
    }

    /// FR-286-AC-2: the `check` outcome of the `inv` source refuses with one
    /// diagnostic equal to the refusal's code, cause, locus and message.
    #[trace("TC-770", "FR-286-AC-2")]
    #[test]
    fn check_refusal_document_carries_the_refusals_diagnostic() {
        let (result, _) = compiled(ILL_TYPED);
        let Err(StageFailure::Refused(refusal)) = &result else {
            panic!("expected a refusal");
        };
        let region = refusal.region().expect("the refusal is located");
        let document = json_of(&OutcomeDocument::from_check(&result));
        assert_eq!(
            document,
            json!({
                "format": "filament-outcome/1",
                "operation": "check",
                "last_stage": "S3",
                "category": "refusal",
                "items": [],
                "diagnostics": [{
                    "cause": "ambiguous-literal",
                    "code": "ill_typed",
                    "details": null,
                    "locus": {
                        "kind": "region",
                        "source": serde_json::to_value(region.source()).expect("a source ref"),
                        "start": region.start(),
                        "end": region.end(),
                    },
                    "message": refusal.to_string(),
                }],
                "artifacts": [],
                "result": null,
            })
        );
    }

    /// FR-286-AC-3: three items serialize in request order with their
    /// records and categories, to exactly these bytes, twice over.
    #[trace("TC-770", "FR-286-AC-3")]
    #[test]
    fn analyze_items_keep_request_order_and_bytes_are_stable() {
        let document = OutcomeDocument::settled(
            Operation::Analyze,
            Some(OutcomeStage::S6c),
            vec![
                record(
                    0,
                    TerminalValue::Unsupported(UnavailabilityCause::SolverAbsent),
                ),
                record(
                    1,
                    TerminalValue::Proved {
                        basis: ProofBasis::Checks { success_checks: 3 },
                        certification: Certification::Certified,
                    },
                ),
                record(2, TerminalValue::Refuted),
            ],
        );
        // FR-286 "Canonical encoding and key order": RFC 8785, members sorted
        // by UTF-16 code units at every depth, independent of the order the
        // document serializes them in and of `serde_json`'s `preserve_order`.
        let expected = concat!(
            r#"{"artifacts":[],"category":"unsupported","diagnostics":[],"#,
            r#""format":"filament-outcome/1","items":["#,
            r#"{"category":"unsupported","cause":{"kind":"solver-absent"},"request_index":0,"result":"unsupported"},"#,
            r#"{"basis":{"checks":3,"type":"bounded-proof"},"category":"success","cause":null,"certification":"certified","request_index":1,"result":"proved"},"#,
            r#"{"category":"violation","cause":null,"request_index":2,"result":"refuted"}"#,
            r#"],"last_stage":"S6c","operation":"analyze","result":null}"#
        );
        assert_eq!(text_of(&document), expected);
        assert_eq!(document.to_bytes().unwrap(), document.to_bytes().unwrap());
        let (refused, _) = compiled(ILL_TYPED);
        assert_eq!(
            OutcomeDocument::from_check(&refused).to_bytes().unwrap(),
            OutcomeDocument::from_check(&refused).to_bytes().unwrap()
        );
    }

    /// FSpec FR-331-AC-8: a proof with no SUCCESS check is `inconclusive`
    /// with its vacuity cause, never `proved`.
    #[trace("TC-770", "FR-286-AC-3")]
    #[test]
    fn a_vacuous_proof_is_inconclusive_never_proved() {
        let item = serde_json::to_value(record(
            4,
            TerminalValue::Proved {
                basis: ProofBasis::Checks { success_checks: 0 },
                certification: Certification::Certified,
            },
        ))
        .expect("serializes");
        assert_eq!(
            item,
            json!({
                "request_index": 4,
                "result": "inconclusive",
                "cause": {"kind": "kani-vacuous-proof"},
                "category": "inconclusive",
            })
        );
    }

    /// FSpec FR-483: ValueValidity's no-assignment outcome is an
    /// `Inconclusive` terminal result carrying the same wire cause as a
    /// zero-check Kani proof, while remaining distinct in the typed record.
    #[trace("FSpec-TC-972", "FSpec-FR-483-AC-2")]
    #[test]
    fn value_validity_vacuity_is_an_inconclusive_terminal_value() {
        let value = TerminalValue::Inconclusive(InconclusiveCause::KaniVacuousProof);
        let item = serde_json::to_value(record(5, value)).expect("serializes");
        assert_eq!(
            item,
            json!({
                "request_index": 5,
                "result": "inconclusive",
                "cause": {"kind": "kani-vacuous-proof"},
                "category": "inconclusive",
            })
        );
    }

    #[trace("TC-522", "FR-127-AC-2", "FR-127-AC-9")]
    #[test]
    fn proof_wire_keeps_exact_basis_and_certification_only_on_proved() {
        let bases = [
            (
                ProofBasis::Checks { success_checks: 3 },
                json!({"type": "bounded-proof", "checks": 3}),
            ),
            (ProofBasis::Exhaustive, json!({"type": "exhaustive"})),
            (
                ProofBasis::BoundedComplete { depth: 5 },
                json!({"type": "bounded-complete", "depth": 5}),
            ),
            (
                ProofBasis::Inductive { depth: 2 },
                json!({"type": "inductive", "depth": 2}),
            ),
        ];
        for (basis, expected_basis) in bases {
            for (certification, expected_certification) in [
                (Certification::Certified, "certified"),
                (Certification::Uncertified, "uncertified"),
                (Certification::Trusted, "trusted"),
            ] {
                let document = OutcomeDocument::settled(
                    Operation::Prove,
                    Some(OutcomeStage::S8),
                    vec![record(
                        6,
                        TerminalValue::Proved {
                            basis: basis.clone(),
                            certification,
                        },
                    )],
                );
                let written: Value = serde_json::from_slice(&document.to_bytes().unwrap()).unwrap();
                assert_eq!(
                    written["items"],
                    json!([{
                        "request_index": 6,
                        "result": "proved",
                        "cause": null,
                        "basis": expected_basis,
                        "certification": expected_certification,
                        "category": "success",
                    }])
                );
            }
        }
        for value in [
            TerminalValue::Proved {
                basis: ProofBasis::Checks { success_checks: 0 },
                certification: Certification::Trusted,
            },
            TerminalValue::Tested,
            TerminalValue::Refuted,
            TerminalValue::Failed,
        ] {
            let written = serde_json::to_value(record(0, value)).unwrap();
            assert!(written.get("certification").is_none(), "{written}");
            assert!(written.get("basis").is_none(), "{written}");
        }
    }

    /// ADR-021 RV-2 to RV-4: the reduction causes keep their payloads on the
    /// wire: the refused reduction and form, the population, initial state
    /// and generator, and the number of boundary states.
    #[test]
    fn reduction_cause_wire_keeps_its_payload() {
        use crate::proof_result::SymmetryBrokenCause;
        use filament_checked::key::DeclarationKey;
        use filament_semantic_value::fairness::PropertyForm;
        use fsl_eval::simulation::Permutation;
        use fsl_foundation::ReductionKind;

        let population = DeclarationKey {
            package: "test/sym".to_owned(),
            node: "ix://test/sym/counters".to_owned(),
        };
        let swap = Permutation::of(
            &population,
            [
                ("a".to_owned(), "b".to_owned()),
                ("b".to_owned(), "a".to_owned()),
            ],
        );
        for (value, cause) in [
            (
                InconclusiveCause::ReductionNotPreserving {
                    reduction: ReductionKind::PartialOrder,
                    form: PropertyForm::BoundedMltl,
                },
                json!({"kind": "reduction-not-preserving", "reduction": "partial-order",
                    "form": "bounded-mltl"}),
            ),
            (
                InconclusiveCause::ReductionNotPreserving {
                    reduction: ReductionKind::PartialOrder,
                    form: PropertyForm::Refinement { liveness: false },
                },
                json!({"kind": "reduction-not-preserving", "reduction": "partial-order",
                    "form": {"refinement": {"liveness": false}}}),
            ),
            (
                InconclusiveCause::ReductionNotPreserving {
                    reduction: ReductionKind::PartialOrder,
                    form: PropertyForm::Refinement { liveness: true },
                },
                json!({"kind": "reduction-not-preserving", "reduction": "partial-order",
                    "form": {"refinement": {"liveness": true}}}),
            ),
            (
                InconclusiveCause::SymmetryBroken {
                    population: population.clone(),
                    cause: SymmetryBrokenCause::InitialStatesNotClosed {
                        initial: 0,
                        generator: swap,
                    },
                },
                json!({"kind": "symmetry-broken", "population": "ix://test/sym/counters",
                    "initial": 0, "generator": [["a", "b"], ["b", "a"]]}),
            ),
            (
                InconclusiveCause::ConstraintReached {
                    boundary_states: 12,
                },
                json!({"kind": "constraint-reached", "boundary_states": 12}),
            ),
            (
                InconclusiveCause::ReductionHorizon { max_depth: 2 },
                json!({"kind": "reduction-horizon", "max_depth": 2}),
            ),
        ] {
            let written =
                serde_json::to_value(record(0, TerminalValue::Inconclusive(value))).unwrap();
            assert_eq!(written["cause"], cause, "{written}");
        }
    }

    /// FR-169-AC-8 on the wire (FSpec FR-391): a `well-definedness-unchecked`
    /// cause carries its witness source per initial state.
    #[trace("TC-614", "FR-169-AC-8")]
    #[test]
    fn well_definedness_unchecked_wire_keeps_its_sources() {
        use crate::model_check::state_graph::WitnessSource;

        let value = TerminalValue::Inconclusive(InconclusiveCause::WellDefinednessUnchecked {
            sources: vec![WitnessSource::Explored, WitnessSource::Explored],
        });
        let written = serde_json::to_value(record(0, value)).unwrap();
        assert_eq!(
            written["cause"],
            json!({"kind": "well-definedness-unchecked",
                "sources": [{"type": "explored"}, {"type": "explored"}]}),
            "{written}"
        );
    }

    #[trace("TC-522", "FR-127-AC-2", "FR-127-AC-4")]
    #[test]
    fn temporal_cause_wire_keeps_depth_and_cancellation_source() {
        for (value, cause, category) in [
            (
                TerminalValue::Inconclusive(InconclusiveCause::BoundReached { depth: 1 }),
                json!({"kind": "bound-reached", "depth": 1}),
                "inconclusive",
            ),
            (
                TerminalValue::Inconclusive(InconclusiveCause::InductionNotClosed { depth: 2 }),
                json!({"kind": "induction-not-closed", "depth": 2}),
                "inconclusive",
            ),
            (
                TerminalValue::Inconclusive(InconclusiveCause::UndecidedSuccessor),
                json!({"kind": "undecided-successor"}),
                "inconclusive",
            ),
            (
                TerminalValue::Inconclusive(InconclusiveCause::NoInitialState),
                json!({"kind": "no-initial-state"}),
                "inconclusive",
            ),
            (
                TerminalValue::Incomplete(IncompleteCause::Cancelled {
                    source: CancelCause::Requested,
                }),
                json!({"kind": "cancelled", "source": "requested"}),
                "incomplete",
            ),
            (
                TerminalValue::Incomplete(IncompleteCause::Cancelled {
                    source: CancelCause::Deadline,
                }),
                json!({"kind": "cancelled", "source": "deadline"}),
                "incomplete",
            ),
        ] {
            let written = serde_json::to_value(record(3, value)).unwrap();
            assert_eq!(
                written,
                json!({
                    "request_index": 3,
                    "result": category,
                    "cause": cause,
                    "category": category,
                })
            );
        }
    }

    #[trace("TC-522", "FR-127-AC-9")]
    #[trace("TC-770", "FR-286-AC-3")]
    #[test]
    fn certificate_query_wire_keeps_every_rule_and_query_part() {
        use crate::QueryPart;

        for (rule, expected_rule) in [
            (CertificateRule::InitialMissing, "initial-missing"),
            (CertificateRule::SuccessorMissing, "successor-missing"),
            (CertificateRule::BadState, "bad-state"),
            (CertificateRule::NotPartition, "not-partition"),
            (CertificateRule::BackwardEdge, "backward-edge"),
            (CertificateRule::WitnessFails, "witness-fails"),
            (CertificateRule::QueryMismatch, "query-mismatch"),
            (CertificateRule::ShapeMismatch, "shape-mismatch"),
            (CertificateRule::ProofStepInvalid, "proof-step-invalid"),
            (CertificateRule::NotRefutation, "not-refutation"),
        ] {
            for (part, expected_part) in [
                (QueryPart::Unrolling, "unrolling"),
                (QueryPart::Base, "base"),
                (QueryPart::Step { row: None }, "step"),
                (QueryPart::Step { row: Some(3) }, "step/3"),
            ] {
                let item = record(
                    2,
                    TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
                        rule,
                        at: CertificateLocus::Query { part },
                    }),
                );
                let document = OutcomeDocument::settled(
                    Operation::Analyze,
                    Some(OutcomeStage::S6c),
                    vec![
                        record(
                            0,
                            TerminalValue::Proved {
                                basis: ProofBasis::Exhaustive,
                                certification: Certification::Trusted,
                            },
                        ),
                        record(
                            1,
                            TerminalValue::Inconclusive(InconclusiveCause::BoundReached {
                                depth: 9,
                            }),
                        ),
                        item,
                    ],
                );
                let bytes = document
                    .to_bytes()
                    .expect("query rejection document encodes");
                assert_eq!(document.to_bytes().unwrap(), bytes);
                let written: Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(
                    written,
                    json!({
                        "format": "filament-outcome/1",
                        "operation": "analyze",
                        "last_stage": "S6c",
                        "category": "inconclusive",
                        "items": [{
                            "request_index": 0,
                            "result": "proved",
                            "cause": null,
                            "basis": {"type": "exhaustive"},
                            "certification": "trusted",
                            "category": "success",
                        }, {
                            "request_index": 1,
                            "result": "inconclusive",
                            "cause": {"kind": "bound-reached", "depth": 9},
                            "category": "inconclusive",
                        }, {
                            "request_index": 2,
                            "result": "inconclusive",
                            "cause": {"kind": "certificate-rejected", "rule": expected_rule, "at": expected_part},
                            "category": "inconclusive",
                        }],
                        "diagnostics": [],
                        "artifacts": [],
                        "result": null,
                    })
                );
            }
        }
    }

    #[trace("TC-522", "FR-127-AC-9")]
    #[trace("TC-770", "FR-286-AC-3")]
    #[test]
    fn proof_step_wire_preserves_each_part_and_canonical_index() {
        use crate::QueryPart;

        for (part, expected_part) in [
            (QueryPart::Unrolling, "unrolling"),
            (QueryPart::Base, "base"),
            (QueryPart::Step { row: None }, "step"),
            (QueryPart::Step { row: Some(4) }, "step/4"),
        ] {
            for (index, expected_index) in [
                (0, "0"),
                (7, "7"),
                (41, "41"),
                (9_007_199_254_740_992, "9007199254740992"),
                (9_223_372_036_854_775_808, "9223372036854775808"),
                (u64::MAX, "18446744073709551615"),
            ] {
                let at = CertificateLocus::ProofStep { part, index };
                let item = record(
                    2,
                    TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
                        rule: CertificateRule::ProofStepInvalid,
                        at: at.clone(),
                    }),
                );
                assert_eq!(item.category(), Category::Inconclusive);
                let ItemCauseMember::Present { cause: Some(cause) } = &item.cause else {
                    panic!("certificate rejection carries its cause");
                };
                assert_eq!(cause.certificate_at, Some(at.clone()));
                let document = OutcomeDocument::settled(
                    Operation::Analyze,
                    Some(OutcomeStage::S6c),
                    vec![
                        record(
                            0,
                            TerminalValue::Proved {
                                basis: ProofBasis::Exhaustive,
                                certification: Certification::Trusted,
                            },
                        ),
                        record(
                            1,
                            TerminalValue::Inconclusive(InconclusiveCause::BoundReached {
                                depth: 9,
                            }),
                        ),
                        item,
                    ],
                );
                let bytes = document
                    .to_bytes()
                    .expect("proof-step rejection document encodes");
                assert_eq!(document.to_bytes().unwrap(), bytes);
                let written: Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(
                    written,
                    json!({
                        "format": "filament-outcome/1",
                        "operation": "analyze",
                        "last_stage": "S6c",
                        "category": "inconclusive",
                        "items": [{
                            "request_index": 0,
                            "result": "proved",
                            "cause": null,
                            "basis": {"type": "exhaustive"},
                            "certification": "trusted",
                            "category": "success",
                        }, {
                            "request_index": 1,
                            "result": "inconclusive",
                            "cause": {"kind": "bound-reached", "depth": 9},
                            "category": "inconclusive",
                        }, {
                            "request_index": 2,
                            "result": "inconclusive",
                            "cause": {
                                "kind": "certificate-rejected",
                                "rule": "proof-step-invalid",
                                "at": {"part": expected_part, "index": expected_index},
                            },
                            "category": "inconclusive",
                        }],
                        "diagnostics": [],
                        "artifacts": [],
                        "result": null,
                    })
                );
                let ItemCauseMember::Present { cause: Some(cause) } = &document.items()[2].cause
                else {
                    panic!("the document retains the certificate rejection cause");
                };
                assert_eq!(cause.certificate_at, Some(at));
            }
        }
    }

    #[trace("TC-522", "FR-127-AC-10", "FR-072-AC-2")]
    #[test]
    fn replay_verdict_disagreement_writes_exact_replay_parity() {
        let proved = Verdict::from_category(Category::Violation);
        let replayed = Verdict::from_category(Category::Success);
        let replay = crate::WitnessArmResult::settle(
            proved,
            replayed,
            Category::Success,
            Some(EvaluatedValue::Boolean(true)),
            WitnessCheck::Agrees(None),
            Vec::new(),
            default_accounting(0),
        );
        assert_eq!(
            replay.disagreement(),
            Some(&DisagreementCause::Verdicts { proved, replayed })
        );
        let written = serde_json::to_value(record(
            0,
            TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(
                replay.disagreement().unwrap().clone(),
            )),
        ))
        .unwrap();
        assert_eq!(
            written,
            json!({
                "request_index": 0,
                "result": "inconclusive",
                "cause": {"kind": "replay-parity", "parity": {
                    "reason": "verdicts", "proved": "violation", "replayed": "success",
                }},
                "category": "inconclusive",
            })
        );
    }

    /// Each terminal cause is written in its FR-331 spelling, with its
    /// payload.
    #[trace("TC-770", "FR-286-AC-3", "FR-357-AC-12", "FR-127-AC-18", "FR-149-AC-4")]
    #[test]
    fn terminal_causes_use_their_own_spellings() {
        let parity = DisagreementCause::Verdicts {
            proved: Verdict::from_category(Category::Success),
            replayed: Verdict::from_category(Category::Violation),
        };
        let cases = [
            (
                TerminalValue::Declined {
                    cause: ProofRefusalCause::InvalidInput,
                    code: DeclineCode::Fsl(Code::IllTyped),
                },
                json!({"result": "declined", "cause": {"kind": "invalid-input", "code": "ill_typed"}, "category": "refusal"}),
            ),
            (
                TerminalValue::Incomplete(IncompleteCause::Cancelled {
                    source: CancelCause::Requested,
                }),
                json!({"result": "incomplete", "cause": {"kind": "cancelled", "source": "requested"}, "category": "incomplete"}),
            ),
            (
                TerminalValue::Incomplete(IncompleteCause::ResourceExhausted),
                json!({"result": "incomplete", "cause": {"kind": "limit-reached"}, "category": "incomplete"}),
            ),
            (
                TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(
                    Code::StaleDependency,
                )),
                json!({"result": "inconclusive", "cause": {"kind": "replay-refused", "code": "stale_dependency"}, "category": "inconclusive"}),
            ),
            (
                TerminalValue::Inconclusive(InconclusiveCause::HypothesesNotEstablished {
                    hypothesis: filament_exact::NodeKey::new(
                        "fixture",
                        vec![filament_exact::PathSegment::Text("node".into())],
                        0xdc,
                    )
                    .expect("a fixture key"),
                    result: ItemLabel::Refuted,
                }),
                json!({
                    "result": "inconclusive",
                    "cause": {
                        "kind": "hypotheses-not-established",
                        "hypothesis": {"package": "fixture", "path": ["node"], "ordinal": 0xdc},
                        "result": "refuted",
                    },
                    "category": "inconclusive",
                }),
            ),
            (
                TerminalValue::Inconclusive(InconclusiveCause::StaleDependency {
                    requested: StaleIdentity::Refinement("OtherRefinement".to_owned()),
                    recompiled: StaleIdentity::Refinement("CasRefinesCounter".to_owned()),
                }),
                json!({
                    "result": "inconclusive",
                    "cause": {
                        "kind": "stale-dependency",
                        "requested": {"kind": "refinement", "name": "OtherRefinement"},
                        "recompiled": {"kind": "refinement", "name": "CasRefinesCounter"},
                    },
                    "category": "inconclusive",
                }),
            ),
            (
                TerminalValue::Inconclusive(InconclusiveCause::StateGraphReductionNotPreserving {
                    reduction: fsl_foundation::ReductionKind::Symmetry,
                    form: crate::StateGraphPropertyForm::UniquePath,
                }),
                json!({
                    "result": "inconclusive",
                    "cause": {
                        "kind": "reduction-not-preserving",
                        "reduction": "symmetry",
                        "form": "unique-path",
                    },
                    "category": "inconclusive",
                }),
            ),
            (
                TerminalValue::Inconclusive(InconclusiveCause::ScalarAgrees(
                    crate::scalar::ScalarAgreement::new(
                        crate::scalar::ScalarClaim::Function(Box::new(crate::ValueIdentity {
                            obligation: fsl_package::ObligationIdentity::new(
                                crate::fixture_node(1),
                                None,
                                0,
                            ),
                            package: crate::request::raw_package_descriptor("fixture", &[]),
                            function: Some(
                                QualifiedName::new(vec![Identifier::new("f").unwrap()]).unwrap(),
                            ),
                            source: crate::ReplaySource::Input(Vec::new()),
                            limits: filament_exact::ScalarLimits {
                                integer_bits: 1,
                                decimal_digits: 1,
                                scale_expansion: 1,
                                text_input_bytes: 1,
                                text_scalars: 1,
                                normalized_scalars: 1,
                                unit_edges: 1,
                                value_occurrences: 1,
                                work_units: 1,
                                result_units: 1,
                            },
                            occurrence_key: None,
                            supplied:
                                filament_semantic_value::supplied_stop::SuppliedLimits::default(),
                            generated: crate::scalar::ScalarOutcome::OutOfRange,
                        })),
                        crate::scalar::ScalarOutcome::OutOfRange,
                    ),
                )),
                json!({"result": "inconclusive", "cause": {"kind": "scalar-agrees"}, "category": "inconclusive"}),
            ),
            (
                TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(parity)),
                json!({"result": "inconclusive", "cause": {"kind": "replay-parity", "parity": {"reason": "verdicts", "proved": "success", "replayed": "violation"}}, "category": "inconclusive"}),
            ),
            (
                TerminalValue::Failed,
                json!({"result": "failed", "cause": null, "category": "internal-failure"}),
            ),
        ];
        for (value, mut expected) in cases {
            expected["request_index"] = json!(1);
            assert_eq!(
                serde_json::to_value(record(1, value.clone())).expect("serializes"),
                expected,
                "{value:?}"
            );
        }
    }

    #[trace("TC-638", "FR-203-AC-2")]
    #[test]
    fn precision_budget_cause_serializes_its_exact_interval() {
        let lower = crate::proof_result::ExactRational::new(
            filament_exact::Rational::new(1_i64.into(), 3_i64.into()).expect("nonzero divisor"),
        );
        let upper = crate::proof_result::ExactRational::new(
            filament_exact::Rational::new(2_i64.into(), 3_i64.into()).expect("nonzero divisor"),
        );
        let document = OutcomeDocument::settled(
            Operation::Analyze,
            None,
            vec![record(
                0,
                TerminalValue::Incomplete(IncompleteCause::PrecisionBudget {
                    lower: Box::new(lower),
                    upper: Box::new(upper),
                }),
            )],
        );
        assert_eq!(
            json_of(&document)["items"][0],
            json!({
                "request_index": 0,
                "result": "incomplete",
                "cause": {
                    "kind": "precision-budget",
                    "lower": {"numerator": "1", "denominator": "3"},
                    "upper": {"numerator": "2", "denominator": "3"},
                },
                "category": "incomplete",
            })
        );
    }

    /// FR-286-AC-4: an undefined claim evaluation in an `analyze` or
    /// `monitor` item is a violation with cause `undefined-evaluation` and no
    /// `undefined` label; an `execute` outcome that is undefined keeps it.
    #[trace("TC-770", "FR-286-AC-4")]
    #[test]
    fn undefined_label_appears_only_on_a_non_proof_outcome() {
        for (operation, name) in [
            (Operation::Analyze, "analyze"),
            (Operation::Monitor, "monitor"),
        ] {
            let document = OutcomeDocument::settled(
                operation,
                Some(OutcomeStage::S6c),
                vec![OutcomeItem::undefined_evaluation(
                    RequestIndex::new(0),
                    "position 1",
                    "division-by-zero",
                )],
            );
            assert!(!text_of(&document).contains("\"undefined\""));
            assert_eq!(
                json_of(&document),
                json!({
                    "format": "filament-outcome/1",
                    "operation": name,
                    "last_stage": "S6c",
                    "category": "violation",
                    "items": [{
                        "request_index": 0,
                        "result": "refuted",
                        "cause": {
                            "kind": "undefined-evaluation",
                            "where": "position 1",
                            "cause": "division-by-zero",
                        },
                        "category": "violation",
                    }],
                    "diagnostics": [],
                    "artifacts": [],
                    "result": null,
                })
            );
        }
        let execute = OutcomeDocument::from_call(&CallOutcome::Undefined {
            reason: "sum-out-of-domain",
            location: None,
        });
        assert_eq!(
            json_of(&execute),
            json!({
                "format": "filament-outcome/1",
                "operation": "execute",
                "last_stage": "S6a",
                "category": "undefined",
                "items": [],
                "diagnostics": [],
                "artifacts": [],
                "result": {"kind": "undefined", "reason": "sum-out-of-domain"},
            })
        );
    }

    /// FR-255-AC-7: `seven` under the accounting limits the settings
    /// operation returns for `accounting.work_units=0` settles `Incomplete` naming the
    /// counter, bound, count and setting; under `accounting.work_units=1000000` it
    /// completes with 7.
    #[trace("TC-921", "FR-255-AC-7")]
    #[test]
    fn a_reached_budget_set_by_operand_names_its_setting() {
        let run_under = |operand: &str| {
            let accounting = crate::CallerLimits::from_operands([operand])
                .expect("a settings operand")
                .accounting
                .0;
            let call = Call {
                clocks: Default::default(),
                wire_arguments: Vec::new(),
                snapshot: None,
                function: "seven".to_owned(),
                arguments: Vec::new(),
                accounting,
                supplied: Default::default(),
                backend: filament_eval::ExecutionBackendChoice::Interpreter,
                live_occurrences: crate::spine::DEFAULT_LIVE_OCCURRENCES,
            };
            let outcome = run(
                SourceIdentity::new("agent-ix", "test:outcome"),
                "fixture".to_owned(),
                "unit.native",
                FIXTURE.as_bytes(),
                &BTreeMap::new(),
                &DependencyInput::default(),
                SpineLimits::default(),
                &call,
                &ObjectEnvironment::default(),
                &Cancel::new(),
            )
            .expect("seven runs");
            json_of(&OutcomeDocument::from_call(&outcome))["result"].clone()
        };
        assert_eq!(
            run_under("accounting.work_units=0"),
            json!({"kind": "incomplete", "limit": {
                "kind": "work_units", "bound": "0", "counter": "1",
                "field": "work_units", "setting": "accounting.work_units"}})
        );
        assert_eq!(
            run_under("accounting.work_units=1000000"),
            json!({"kind": "completed", "value": {"integer": "7"}})
        );
    }

    /// FR-286-AC-5: `seven` completes with integer 7 in the `result`
    /// member, and a `check` under an already cancelled handle is incomplete
    /// with no last stage and a null `result`.
    #[trace("TC-770", "FR-286-AC-5")]
    #[test]
    fn execute_result_and_cancelled_check_documents() {
        let call = Call {
            clocks: Default::default(),
            wire_arguments: Vec::new(),
            snapshot: None,
            function: "seven".to_owned(),
            arguments: Vec::new(),
            accounting: default_accounting(1_000_000),
            supplied: Default::default(),
            backend: filament_eval::ExecutionBackendChoice::Interpreter,
            live_occurrences: crate::spine::DEFAULT_LIVE_OCCURRENCES,
        };
        let outcome = run(
            SourceIdentity::new("agent-ix", "test:outcome"),
            "fixture".to_owned(),
            "unit.native",
            FIXTURE.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            SpineLimits::default(),
            &call,
            &ObjectEnvironment::default(),
            &Cancel::new(),
        )
        .expect("seven runs");
        assert_eq!(
            json_of(&OutcomeDocument::from_call(&outcome)),
            json!({
                "format": "filament-outcome/1",
                "operation": "execute",
                "last_stage": "S6a",
                "category": "success",
                "items": [],
                "diagnostics": [],
                "artifacts": [],
                "result": {"kind": "completed", "value": {"integer": "7"}},
            })
        );

        let starved = Call {
            accounting: default_accounting(0),
            supplied: Default::default(),
            ..call
        };
        let outcome = run(
            SourceIdentity::new("agent-ix", "test:outcome"),
            "fixture".to_owned(),
            "unit.native",
            FIXTURE.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            SpineLimits::default(),
            &starved,
            &ObjectEnvironment::default(),
            &Cancel::new(),
        )
        .expect("seven runs");
        assert_eq!(
            json_of(&OutcomeDocument::from_call(&outcome)),
            json!({
                "format": "filament-outcome/1",
                "operation": "execute",
                "last_stage": "S6a",
                "category": "incomplete",
                "items": [],
                "diagnostics": [],
                "artifacts": [],
                "result": {"kind": "incomplete", "limit": {
                    "kind": "work_units", "bound": "0", "counter": "1", "field": "work_units",
                    "setting": "accounting.work_units"}},
            })
        );

        let source = SourceIdentity::new("agent-ix", "test:outcome");
        let limits = SpineLimits::default();
        let live = Cancel::new();
        let request = ParseRequest {
            source: &source,
            path: "unit.native",
            bytes: FIXTURE.as_bytes(),
        };
        let parsed = parse(&request, limits.source, &live)
            .expect("parses")
            .into_value();
        let models = select(&parsed, &BTreeMap::new(), limits.model, &live)
            .expect("selects")
            .into_value();
        let cancelled = Cancel::new();
        cancelled.cancel(CancelCause::Requested);
        let result = check(
            &parsed,
            "fixture".to_owned(),
            &models,
            &DependencyInput::default(),
            limits,
            &cancelled,
        );
        let document = json_of(&OutcomeDocument::from_check(&result));
        assert_eq!(document["category"], "incomplete");
        assert_eq!(document["last_stage"], Value::Null);
        assert_eq!(document["items"], json!([]));
        assert_eq!(document["result"], Value::Null);
    }

    /// The whole-outcome category is the most severe item's under FR-285's
    /// order. A monitor document whose only item is a `Category::Pending`
    /// item is `Pending` with exit 0 (FR-285-AC-9); a pending item next to a
    /// violation is a `Violation`; an indeterminate item ranks above pending
    /// and below a violation. An `Inconclusive` proof item (exit 22) is a
    /// different category and is not a pending clause.
    #[trace("TC-770", "FR-286-AC-3")]
    #[trace("TC-769", "FR-285-AC-9")]
    #[test]
    fn the_document_category_is_the_most_severe_items() {
        let declined = TerminalValue::Declined {
            cause: ProofRefusalCause::Refused,
            code: DeclineCode::Fsl(Code::IllTyped),
        };
        let mixed = OutcomeDocument::settled(
            Operation::Prove,
            Some(OutcomeStage::S8),
            vec![
                record(
                    0,
                    TerminalValue::Proved {
                        basis: ProofBasis::Checks { success_checks: 1 },
                        certification: Certification::Certified,
                    },
                ),
                record(1, TerminalValue::Refuted),
                record(2, declined),
                record(3, TerminalValue::Incomplete(IncompleteCause::TimedOut)),
            ],
        );
        assert_eq!(mixed.category(), Category::Refusal);
        // A clause pending at the end of the trace is the catalog's `pending`
        // (exit 0), a category of its own and not `inconclusive` (FR-285-AC-9).
        let pending = || {
            let mut item = record(0, TerminalValue::Refuted);
            item.category = Category::Pending;
            vec![item]
        };
        let monitored = OutcomeDocument::settled(Operation::Monitor, None, pending());
        assert_eq!(monitored.category(), Category::Pending);
        assert_eq!(monitored.category().exit_code(), 0);
        let violated = OutcomeDocument::settled(
            Operation::Monitor,
            None,
            pending()
                .into_iter()
                .chain([record(1, TerminalValue::Refuted)])
                .collect(),
        );
        assert_eq!(violated.category(), Category::Violation);
        let inconclusive = vec![record(
            0,
            TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(Code::StaleDependency)),
        )];
        assert_eq!(
            OutcomeDocument::settled(Operation::Prove, None, inconclusive).category(),
            Category::Inconclusive
        );
        // An indeterminate item exits 11: above a pending one, below a violation.
        let mut indeterminate = record(2, TerminalValue::Refuted);
        indeterminate.category = Category::Indeterminate;
        let ranked = |items: Vec<OutcomeItem>| {
            OutcomeDocument::settled(Operation::Monitor, None, items).category()
        };
        assert_eq!(
            ranked(vec![pending().remove(0), indeterminate.clone()]),
            Category::Indeterminate
        );
        assert_eq!(
            ranked(vec![indeterminate, record(1, TerminalValue::Refuted)]),
            Category::Violation
        );
        assert_eq!(
            OutcomeDocument::settled(Operation::Prove, None, Vec::new()).category(),
            Category::Success
        );
    }

    /// A refused call's diagnostic keeps the refusal's locus.
    #[trace("TC-770", "FR-286-AC-2")]
    #[test]
    fn a_refused_call_keeps_its_locus() {
        let span = fsl_foundation::LocatedSpan {
            start: Position {
                byte: 4,
                line: 1,
                column: 5,
            },
            end: Position {
                byte: 9,
                line: 1,
                column: 10,
            },
        };
        let refusal = CallRefusal::Record {
            code: CatalogCode::new("ill_typed", "type-mismatch"),
            fields: BTreeMap::new(),
            locus: Some(CallLocus {
                file: "fixture.native".to_owned(),
                span,
            }),
            location: None,
        };
        let document = json_of(&OutcomeDocument::from_call(&CallOutcome::Refused(refusal)));
        assert_eq!(document["category"], "refusal");
        assert_eq!(document["diagnostics"][0]["code"], "ill_typed");
        assert_eq!(document["diagnostics"][0]["cause"], "type-mismatch");
        assert_eq!(
            document["diagnostics"][0]["locus"],
            json!({
                "kind": "located",
                "file": "fixture.native",
                "span": serde_json::to_value(span).expect("a span"),
            })
        );
    }

    /// FR-095-AC-7 (TC-422): caller-named input loci preserve operand text
    /// and always write nullable `member` and `pointer` members.
    #[trace("TC-422", "FR-095-AC-7")]
    #[test]
    fn input_loci_preserve_operand_and_nullable_members_on_the_wire() {
        let first = OutcomeLocus::from(&Locus::Input {
            operand: "./out/handoff".to_owned(),
            member: Some("selection.json".to_owned()),
            pointer: None,
        });
        assert_eq!(
            serde_json::to_value(first).expect("input locus serializes"),
            json!({
                "kind": "input",
                "member": "selection.json",
                "operand": "./out/handoff",
                "pointer": null
            })
        );

        let second = OutcomeLocus::from(&Locus::Input {
            operand: "pkg.json".to_owned(),
            member: None,
            pointer: Some("/lock".parse().expect("valid pointer")),
        });
        assert_eq!(
            serde_json::to_value(second).expect("input locus serializes"),
            json!({
                "kind": "input",
                "member": null,
                "operand": "pkg.json",
                "pointer": "/lock"
            })
        );
    }

    /// A replay outcome is its arm's category at S8, or its refusal's.
    #[trace("TC-770", "FR-286-AC-3")]
    #[test]
    fn replay_documents_follow_the_arm_and_the_refusal() {
        let charges = ScalarLimits {
            integer_bits: 64,
            decimal_digits: 34,
            scale_expansion: 8,
            text_input_bytes: 1024,
            text_scalars: 1024,
            normalized_scalars: 1024,
            unit_edges: 4,
            value_occurrences: 16,
            work_units: 100,
            result_units: 10,
        };
        let success = Verdict::from_category(Category::Success);
        let arm = InputArmResult::settle(
            success,
            success,
            Category::Success,
            Some(EvaluatedValue::Boolean(true)),
            &WitnessCheck::Agrees(None),
            Vec::new(),
            charges,
        );
        let row: Value = serde_json::from_str(
            &ReplayResult::Input(arm.clone())
                .to_function_json()
                .expect("a row"),
        )
        .expect("the row is JSON");
        assert_eq!(
            json_of(&OutcomeDocument::from_replay(&Ok(ReplayResult::Input(arm)))),
            json!({
                "format": "filament-outcome/1",
                "operation": "replay",
                "last_stage": "S8",
                "category": "success",
                "items": [],
                "diagnostics": [],
                "artifacts": [],
                "result": {"kind": "replayed", "results": [row]},
            })
        );
        let unknown = ReplayRefusal::UnknownFunction {
            selection: QualifiedName::new(vec![Identifier::new("seven").expect("an identifier")])
                .expect("a qualified name"),
            package: fsl_semantics::library::LibraryName::new("fixture").expect("a library name"),
        };
        let unknown = json_of(&OutcomeDocument::from_replay(&Err(unknown)));
        assert_eq!(unknown["last_stage"], "S8");
        assert_eq!(unknown["diagnostics"][0]["code"], "missing_declaration");
        assert_eq!(unknown["diagnostics"][0]["cause"], "missing-name");
        let fault = ReplayRefusal::Fault(InternalFault::new("replay", "broken"));
        let document = json_of(&OutcomeDocument::from_replay(&Err(fault)));
        assert_eq!(document["category"], "internal-failure");
        assert_eq!(document["last_stage"], Value::Null);
        assert_eq!(document["diagnostics"][0]["code"], "runtime_invariant");
    }

    /// FSpec FR-286-AC-21: a refused replay outcome has no result row.
    #[trace("TC-2963", "FR-286-AC-21")]
    #[test]
    fn a_refused_replay_outcome_has_null_result() {
        let refusal = ReplayRefusal::Request(crate::ReplayRequestRefusal::MissingReplayTarget);
        let document = json_of(&OutcomeDocument::from_replay(&Err(refusal)));
        assert_eq!(document["operation"], "replay");
        assert_eq!(document["result"], Value::Null);
    }

    /// An unwriteable replay row is an internal failure with no reached stage; its one
    /// output-failure diagnostic names the failed row, and the result is null.
    #[test]
    fn replay_row_encoding_failure_is_an_unstaged_output_failure_without_partial_result() {
        let result =
            ReplayResult::ProtocolAnalysis(crate::protocol_analysis::ProtocolReplayResult {
                settlement: crate::protocol_analysis::ProtocolReplaySettlement::Reproduced,
            });
        let document = json_of(&OutcomeDocument::from_replay_row(
            &result,
            2,
            Err(crate::result::CauseCodecError::Empty("result row")),
        ));
        assert_eq!(document["category"], "internal-failure");
        assert_eq!(document["last_stage"], Value::Null);
        assert_eq!(document["result"], Value::Null);
        assert_eq!(document["diagnostics"][0]["code"], "output-failure");
        assert_eq!(document["diagnostics"][0]["cause"], "output-failure");
        assert!(
            document["diagnostics"][0]["message"]
                .as_str()
                .expect("diagnostic message")
                .contains("row at position 2"),
            "{document}"
        );
    }

    fn run_source(
        text: &str,
        function: &str,
        cancel: &Cancel,
    ) -> Result<CallOutcome, Box<RunRefusal>> {
        run(
            SourceIdentity::new("agent-ix", "test:outcome"),
            "fixture".to_owned(),
            "unit.native",
            text.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            SpineLimits::default(),
            &Call {
                clocks: Default::default(),
                wire_arguments: Vec::new(),
                snapshot: None,
                function: function.to_owned(),
                arguments: Vec::new(),
                accounting: default_accounting(1_000_000),
                supplied: Default::default(),
                backend: filament_eval::ExecutionBackendChoice::Interpreter,
                live_occurrences: crate::spine::DEFAULT_LIVE_OCCURRENCES,
            },
            &ObjectEnvironment::default(),
            cancel,
        )
    }

    /// FR-286-AC-6: `from_run` over each arm of `spine::run`'s result.
    #[trace("TC-770", "FR-286-AC-6")]
    #[test]
    fn from_run_documents_every_arm() {
        // Completed: the call's document.
        let completed = run_source(FIXTURE, "seven", &Cancel::new());
        let call_outcome = completed.as_ref().expect("seven runs");
        let document = json_of(&OutcomeDocument::from_run(&completed));
        assert_eq!(
            document,
            json!({
                "format": "filament-outcome/1",
                "operation": "execute",
                "last_stage": "S6a",
                "category": "success",
                "items": [],
                "diagnostics": [],
                "artifacts": [],
                "result": {"kind": "completed", "value": {"integer": "7"}},
            })
        );
        assert_eq!(
            document["result"],
            json_of(&OutcomeDocument::from_call(call_outcome))["result"]
        );

        // Undefined: the call's undefined result.
        let undefined = Ok(CallOutcome::Undefined {
            reason: "division-by-zero",
            location: None,
        });
        let document = json_of(&OutcomeDocument::from_run(&undefined));
        assert_eq!(document["category"], "undefined");
        assert_eq!(
            document["result"],
            json!({"kind": "undefined", "reason": "division-by-zero"})
        );

        // Refused call: its catalog code and cause as one diagnostic.
        let refused = Ok(CallOutcome::Refused(CallRefusal::Family {
            code: CatalogCode::new("ill_typed", "type-mismatch"),
            location: None,
        }));
        let document = json_of(&OutcomeDocument::from_run(&refused));
        assert_eq!(document["category"], "refusal");
        assert_eq!(document["diagnostics"][0]["code"], "ill_typed");
        assert_eq!(document["diagnostics"][0]["cause"], "type-mismatch");

        // Compile refusal: the stage it reached and its own diagnostic.
        let refusal = run_source(ILL_TYPED, "inv", &Cancel::new());
        let Err(boxed) = &refusal else {
            panic!("the source does not type");
        };
        let RunRefusal::Compile(compile) = boxed.as_ref() else {
            panic!("a compile refusal");
        };
        let document = json_of(&OutcomeDocument::from_run(&refusal));
        assert_eq!(document["operation"], "execute");
        assert_eq!(document["last_stage"], "S3");
        assert_eq!(document["category"], "refusal");
        assert_eq!(document["artifacts"], json!([]));
        assert_eq!(document["diagnostics"].as_array().map(Vec::len), Some(1));
        assert_eq!(document["diagnostics"][0]["code"], "ill_typed");
        assert_eq!(document["diagnostics"][0]["message"], compile.to_string());

        // A call-stage refusal: S6a, its code.
        let missing = run_source(FIXTURE, "nope", &Cancel::new());
        let document = json_of(&OutcomeDocument::from_run(&missing));
        assert_eq!(document["last_stage"], "S6a");
        assert_eq!(document["category"], "refusal");
        assert_eq!(document["diagnostics"][0]["code"], "missing_declaration");

        // Fault: internal failure with its catalog code, at no stage.
        let fault = InternalFault::new("call", "broken");
        let code = fault.catalog_code();
        let faulted = Err(Box::new(RunRefusal::Fault(fault)));
        let document = json_of(&OutcomeDocument::from_run(&faulted));
        assert_eq!(document["category"], "internal-failure");
        assert_eq!(document["last_stage"], Value::Null);
        assert_eq!(document["diagnostics"][0]["code"], code.code());
        assert_eq!(document["diagnostics"][0]["cause"], code.cause());

        // Cancelled: incomplete at no stage, naming the cause.
        let cancel = Cancel::new();
        cancel.cancel(CancelCause::Requested);
        let cancelled = run_source(FIXTURE, "seven", &cancel);
        assert!(matches!(
            cancelled.as_ref().err().map(AsRef::as_ref),
            Some(RunRefusal::Cancelled(CancelCause::Requested))
        ));
        let document = json_of(&OutcomeDocument::from_run(&cancelled));
        assert_eq!(document["category"], "incomplete");
        assert_eq!(document["last_stage"], Value::Null);
        assert_eq!(document["diagnostics"][0]["code"], "cancelled");
        assert_eq!(document["diagnostics"][0]["cause"], "requested");

        // A driver's unbuilt engine: `unimplemented_capability`, not
        // `unsupported_construct`, unsupported, no stage, exit 21.
        let driver = OutcomeDocument::new(Operation::Execute, None, Category::Unsupported)
            .with_diagnostics(vec![OutcomeDiagnostic::new(
                None,
                Code::UnimplementedCapability.as_str(),
                None,
                "the AOT engine is not built",
            )]);
        assert_eq!(
            Code::UnimplementedCapability.category(),
            Category::Unsupported
        );
        assert_eq!(Code::UnimplementedCapability.category().exit_code(), 21);
        assert_eq!(driver.category().exit_code(), 21);
        let document = json_of(&driver);
        assert_eq!(document["category"], "unsupported");
        assert_eq!(document["last_stage"], Value::Null);
        assert_eq!(
            document["diagnostics"][0]["code"],
            "unimplemented_capability"
        );
    }

    /// FR-663 (TC-1746): the pointer of a snapshot refusal is its
    /// document-spelled `path` member, which resolves in the document.
    #[trace("FR-663-AC-3")]
    #[test]
    fn a_snapshot_locus_pointer_resolves_in_the_document() {
        use filament_semantic_value::admission::{AdmissionRecord, PathStep};
        let document = json!({"populations": [{"objects": [{"fields": {
            "reach": {"set": [{"n": 1}, {"n": 1}]},
            "total": {"record": {"type": "T", "fields": {"a": 1}}},
        }}]}]});
        let reference = fsl_semantics::model::observation::DocumentRef {
            authority: "test".to_owned(),
            identity: "current".to_owned(),
        };
        let field = |name: &str| PathStep::Field(name.to_owned());
        let prefix = || {
            vec![
                field("populations"),
                PathStep::Index(0),
                field("objects"),
                PathStep::Index(0),
                field("fields"),
            ]
        };
        let pointer_of = |below: Vec<PathStep>, spelled: Option<&str>| {
            let mut path = prefix();
            path.extend(below);
            let record = AdmissionRecord {
                catalog: CatalogCode::new("invalid_runtime_input", "duplicate-member"),
                fields: spelled
                    .map(|spelled| BTreeMap::from([("path", spelled.to_owned())]))
                    .unwrap_or_default(),
                path,
                kernel: None,
            };
            match snapshot_admission_locus(&record, &reference, None) {
                Some(OutcomeLocus::Document { pointer, .. }) => pointer,
                other => panic!("a document locus, not {other:?}"),
            }
        };
        // The typed steps of a duplicate at element 1 of a set are `[1]`; the
        // pointer is the document-spelled `path` member.
        let duplicate = pointer_of(
            vec![field("reach"), PathStep::Index(1)],
            Some("/populations/0/objects/0/fields/reach/set/1"),
        );
        assert_eq!(duplicate, "/populations/0/objects/0/fields/reach/set/1");
        assert_eq!(document.pointer(&duplicate), Some(&json!({"n": 1})));
        // A refusal with no `path` member is located by its typed steps.
        let typed = pointer_of(vec![field("total")], None);
        assert_eq!(typed, "/populations/0/objects/0/fields/total");
        assert!(document.pointer(&typed).is_some());
    }
}
