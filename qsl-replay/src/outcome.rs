// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-286 (ADR-029 CB-4): the one `quire-outcome/1` JSON outcome document
//! every QSL lifecycle operation's outcome serializes to, and its one
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
//! library and the CLI give one document for one request (QSpec
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

use crate::hybrid::{BoundDimension, SolverHorizon};
use qsl_cst::format::FormatRefusal;
use qsl_foundation::diagnostic::{Code, Locus, StageFailure, Staged};
use qsl_foundation::source::provenance::{OccurrenceKey, RawSourceRef};
use qsl_foundation::{LocatedSpan, RequestIndex};
use qsl_semantics::model::observation::AdmissionFailure;
use quire_exact::{CancelCause, NodeKey};
use quire_semantic_value::catalog::Category;
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
use crate::result::ReplayResult;
use crate::spine::kernel_undefined_reason;
use crate::spine::{
    CallIncomplete, CallOutcome, CallRefusal, CallValue, CheckedUnit, CompileRefusal, EmittedUnit,
    FrontEndFailure, ParsedSource, RunRefusal, SpineStage,
};
use crate::statistical::{self, Decision, StatisticalVerdict};
use crate::ReplayRefusal;
use qsl_eval::temporal::UndefinedCause;
use quire_semantic_value::fairness::PropertyForm;

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
            diagnostic: OutcomeDiagnostic::new(
                Some(diagnostic.cause.as_str()),
                diagnostic.code.as_str(),
                diagnostic
                    .region
                    .as_ref()
                    .map(|region| OutcomeLocus::from(&Locus::Region(region.clone()))),
                diagnostic.message.clone(),
            ),
        }
    }
}

/// The document's `format` member.
pub const OUTCOME_FORMAT: &str = "quire-outcome/1";

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
            SpineStage::Intake => Self::I1,
            SpineStage::Assembly | SpineStage::Check => Self::S3,
            SpineStage::Emit => Self::S4,
        }
    }
}

/// A proof item's result label (FR-331). It has no `undefined` member: an
/// undefined claim evaluation is `refuted` with the `undefined-evaluation`
/// cause ([`ItemCause::undefined_evaluation`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
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
            TerminalValue::Failed => Self::Failed,
            TerminalValue::Measured(_) => Self::Measured,
        }
    }
}

/// Why a replay settled inconclusive: its disagreement reason and the two
/// verdicts' categories.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ParityReason {
    reason: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    proved: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replayed: Option<&'static str>,
}

/// A checked-package node id on the wire: the structural
/// `{"package", "path", "ordinal"}` object (QSpec FR-322), a path segment
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
        let key = qsl_foundation::node_wire::node_key_from_json(&value);
        qsl_foundation::node_wire::discard_node_json(value);
        key.map(Self).map_err(serde::de::Error::custom)
    }
}

impl Serialize for WireNode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let value = qsl_foundation::node_wire::node_key_to_json(&self.0)
            .map_err(serde::ser::Error::custom)?;
        let written = Serialize::serialize(&value, serializer);
        qsl_foundation::node_wire::discard_node_json(value);
        written
    }
}

/// The typed cause an item's terminal record carries: its FR-331 spelling in
/// `kind`, and the members that spelling names.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ItemCause {
    kind: &'static str,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cause: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<u64>,
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
    /// state, in initial-state order (QSpec FR-391).
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
    /// member (QSpec FR-439, FR-428).
    #[serde(skip_serializing_if = "Option::is_none")]
    bound: Option<&'static str>,
    /// A `memory-bound-reached` or `instance-bound-reached` cause's count of
    /// limited states.
    #[serde(skip_serializing_if = "Option::is_none")]
    states: Option<u64>,
    /// Every `weak-access-shape` failure, with its shape written as JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    failures: Option<Vec<ItemWeakAccessFailure>>,
    /// A `recovery-view` cause's template, by its qualified name.
    #[serde(skip_serializing_if = "Option::is_none")]
    predicate: Option<String>,
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
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct TimedStateWire {
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

impl From<&qsl_eval::simulation::StateKey> for TimedStateWire {
    fn from(key: &qsl_eval::simulation::StateKey) -> Self {
        use base64::Engine;
        Self {
            key: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key.as_bytes()),
        }
    }
}

impl From<&quire_semantic_value::model_transition::ModelTransition> for TimedTransitionWire {
    fn from(identity: &quire_semantic_value::model_transition::ModelTransition) -> Self {
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

/// A property form on the outcome wire, spelled as quire-semantic-value
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
            kind,
            at: None,
            cause: None,
            code: None,
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
            value: None,
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
            states: None,
            failures: None,
            predicate: None,
            requested: None,
            recompiled: None,
            state: None,
            identity: None,
        }
    }

    /// The claim's evaluation was undefined at `at`, for `cause`: the item
    /// is refuted, category violation (FR-281, FR-283).
    pub fn undefined_evaluation(at: impl Into<String>, cause: impl Into<String>) -> Self {
        Self {
            at: Some(at.into()),
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
                | InconclusiveCause::NoInitialState
                | InconclusiveCause::NoInstance
                | InconclusiveCause::KaniVacuousProof
                | InconclusiveCause::RefutedIgnoringConstraints
                | InconclusiveCause::MatchUndetermined
                | InconclusiveCause::VacuousMatch
                | InconclusiveCause::MappingUndetermined
                | InconclusiveCause::ProtocolAssumptionNotEstablished(_)
                | InconclusiveCause::SufficientTestFailed
                | InconclusiveCause::NoAdmittedBehaviour
                | InconclusiveCause::LassoNotConcretized
                | InconclusiveCause::SolverInconclusive { .. },
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
                item.bound = Some(bound.member.as_str());
                item.value = Some(bound.value);
                item.states = Some(*states);
            }
            ReportedInconclusiveCause::Cause(InconclusiveCause::InstanceBoundReached {
                bound,
                states,
            }) => {
                item.bound = Some(InstanceBound::MEMBER);
                item.value = Some(bound.value);
                item.states = Some(*states);
            }
            ReportedInconclusiveCause::KaniVacuousProof => {}
            ReportedInconclusiveCause::Cause(InconclusiveCause::ReplayParity(parity)) => {
                item.parity = Some(ParityReason {
                    reason: parity.as_str(),
                    proved: parity.proved().map(|verdict| verdict.category().as_str()),
                    replayed: parity.replayed().map(|verdict| verdict.category().as_str()),
                });
            }
            ReportedInconclusiveCause::Cause(InconclusiveCause::ReplayRefused(code)) => {
                item.code = Some(code.as_str().to_owned());
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

/// One requested item of a `prove`, `analyze` or `monitor` outcome: its
/// terminal record and its category.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OutcomeItem {
    request_index: usize,
    result: ItemLabel,
    cause: Option<ItemCause>,
    #[serde(skip_serializing_if = "Option::is_none")]
    basis: Option<ProofBasis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    certification: Option<Certification>,
    /// A `measured` item's QSpec FR-409 decision: `accepted`, `rejected` or
    /// `undecided`.
    #[serde(skip_serializing_if = "Option::is_none")]
    measured: Option<&'static str>,
    /// The state-graph settlement method, when this item came from FR-391.
    #[serde(skip_serializing_if = "Option::is_none")]
    method: Option<&'static str>,
    /// The state-graph search horizon, a method parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    max_depth: Option<Option<u64>>,
    /// The state-graph phase-0 seed actually used.
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(serialize_with = "category_str")]
    category: Category,
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

/// QSpec FR-409's `measured` label and FR-331 cause of a statistical
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
            TerminalValue::Declined { cause, code } => Some(ItemCause {
                code: Some(match code {
                    DeclineCode::Qsl(code) => code.as_str().to_owned(),
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
                    UnavailabilityCause::ValueCheck { reason, .. } => {
                        item.refusal = Some(reason.as_str());
                    }
                    UnavailabilityCause::ContractCheck { reason, .. } => {
                        item.refusal = Some(reason.as_str());
                    }
                    UnavailabilityCause::SolverAbsent
                    | UnavailabilityCause::HybridProviderAbsent
                    | UnavailabilityCause::BackendAbsent
                    | UnavailabilityCause::MissingFairnessPremise { .. }
                    | UnavailabilityCause::UnsupportedReduction { .. }
                    | UnavailabilityCause::HiddenFieldLiveness
                    | UnavailabilityCause::UnsupportedHyperForm { .. }
                    | UnavailabilityCause::UnsupportedSmtFragment
                    | UnavailabilityCause::StrongAbstractFairness
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
                        item.limit = Some(limit.as_str());
                        item.value = Some(*value);
                        item.setting = Some(limit.setting());
                    }
                    IncompleteCause::PrecisionBudget { lower, upper } => {
                        item.lower = Some((**lower).clone());
                        item.upper = Some((**upper).clone());
                    }
                    IncompleteCause::TimedOut | IncompleteCause::ResourceExhausted => {}
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
            | TerminalValue::Failed => None,
        };
        Self {
            request_index: record.request_index().get(),
            result,
            cause,
            basis,
            certification,
            measured,
            method: None,
            max_depth: None,
            seed: None,
            witness_samples: None,
            limits: None,
            limit_reached: None,
            category: value.category(),
        }
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

    /// The item whose claim evaluated to an undefined value at `at` for
    /// `cause`: refuted, category violation, never labelled `undefined`.
    pub fn undefined_evaluation(
        request_index: RequestIndex,
        at: impl Into<String>,
        cause: impl Into<String>,
    ) -> Self {
        Self {
            request_index: request_index.get(),
            result: ItemLabel::Refuted,
            cause: Some(ItemCause::undefined_evaluation(at, cause)),
            basis: None,
            certification: None,
            measured: None,
            method: None,
            max_depth: None,
            seed: None,
            witness_samples: None,
            limits: None,
            limit_reached: None,
            category: Category::Violation,
        }
    }

    /// The item's O-16 category.
    pub fn category(&self) -> Category {
        self.category
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
        /// The RFC 6901 pointer into the artifact.
        pointer: String,
    },
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
            Locus::Artifact { pointer } => Self::Artifact {
                pointer: pointer.to_string(),
            },
        }
    }
}

/// One diagnostic: its typed cause, catalog code, locus and message.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OutcomeDiagnostic {
    cause: Option<&'static str>,
    code: String,
    locus: Option<OutcomeLocus>,
    message: String,
}

impl OutcomeDiagnostic {
    /// A diagnostic with the catalog `code` and its typed `cause` (the
    /// catalog's cause tag, where the code has one), raised at `locus`.
    pub fn new(
        cause: Option<&'static str>,
        code: impl Into<String>,
        locus: Option<OutcomeLocus>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            cause,
            code: code.into(),
            locus,
            message: message.into(),
        }
    }

    /// The diagnostic of a spine compile refusal: its catalog code, cause,
    /// region and message.
    pub fn from_compile_refusal(refusal: &CompileRefusal) -> Self {
        Self::new(
            refusal.cause(),
            refusal.code().as_str(),
            refusal
                .region()
                .map(|region| OutcomeLocus::from(&Locus::Region(region.clone()))),
            refusal.to_string(),
        )
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
    pub fn cause(&self) -> Option<&'static str> {
        self.cause
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

/// A completed call's value, in FR-100's rendering.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResultValue {
    /// A `Boolean` value.
    Boolean {
        /// The value.
        value: bool,
    },
    /// An integer value, in FR-038's ASCII decimal spelling.
    Integer {
        /// The decimal digits.
        decimal: String,
    },
    /// An exact model reference identity triple.
    Reference {
        /// The object universe identity.
        universe: String,
        /// The most-specific object type identity.
        #[serde(rename = "type")]
        object_type: String,
        /// The authored object identity.
        object_identity: String,
    },
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
            setting: incomplete.setting().name(),
        }
    }
}

/// The `quire-outcome/1` document (FR-286).
///
/// `items` is always written, an empty array for an operation with no
/// requested items. `last_stage` is `null` when no stage is known to have
/// run. `result` is always written: `null` except for a completed,
/// undefined or incomplete `execute` outcome. `artifacts` is always written
/// as an empty array.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutcomeDocument {
    format: &'static str,
    operation: Operation,
    last_stage: Option<OutcomeStage>,
    category: Category,
    items: Vec<OutcomeItem>,
    diagnostics: Vec<OutcomeDiagnostic>,
    result: Option<ExecuteResult>,
}

impl Serialize for OutcomeDocument {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut document = serializer.serialize_struct("OutcomeDocument", 8)?;
        document.serialize_field("format", &self.format)?;
        document.serialize_field("operation", &self.operation)?;
        document.serialize_field("last_stage", &self.last_stage)?;
        document.serialize_field("category", self.category.as_str())?;
        document.serialize_field("items", &self.items)?;
        document.serialize_field("diagnostics", &self.diagnostics)?;
        document.serialize_field("artifacts", &[(); 0])?;
        document.serialize_field("result", &self.result)?;
        document.end()
    }
}

/// Why a document could not be written.
#[derive(Debug, thiserror::Error)]
#[error("the outcome document could not be encoded: {0}")]
pub struct OutcomeWriteError(#[from] serde_json::Error);

impl OutcomeDocument {
    /// A document of `operation` that last reached `last_stage` with the
    /// outcome's `category`, and no items, diagnostics or artifacts.
    pub fn new(operation: Operation, last_stage: Option<OutcomeStage>, category: Category) -> Self {
        Self {
            format: OUTCOME_FORMAT,
            operation,
            last_stage,
            category,
            items: Vec::new(),
            diagnostics: Vec::new(),
            result: None,
        }
    }

    /// A document of `operation` over its requested `items`, in request
    /// order. Its category is the most severe item's under ADR-029 CB-4
    /// (FR-285's order 30, 20, 21, 22, 10, 0), the first such item's on a
    /// tie, and `success` with no items. A `monitor` clause still pending
    /// when the trace ends takes the trace row.
    pub fn settled(
        operation: Operation,
        last_stage: Option<OutcomeStage>,
        items: Vec<OutcomeItem>,
    ) -> Self {
        let code = |item: &OutcomeItem| match operation {
            Operation::Monitor => item.category.trace_exit_code(),
            _ => item.category.exit_code(),
        };
        let category = Category::most_severe(items.iter().map(code))
            .and_then(|worst| items.iter().find(|item| code(item) == worst))
            .map_or(Category::Success, |item| item.category);
        Self::new(operation, last_stage, category).with_items(items)
    }

    /// This document with one entry per requested item, in request order.
    #[must_use]
    pub fn with_items(mut self, items: Vec<OutcomeItem>) -> Self {
        self.items = items;
        self
    }

    /// This document with its diagnostics.
    #[must_use]
    pub fn with_diagnostics(mut self, diagnostics: Vec<OutcomeDiagnostic>) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    /// The outcome's O-16 category, from which FR-285 derives the exit code.
    pub fn category(&self) -> Category {
        self.category
    }

    /// The document's items, in request order.
    pub fn items(&self) -> &[OutcomeItem] {
        &self.items
    }

    /// The document's diagnostics.
    pub fn diagnostics(&self) -> &[OutcomeDiagnostic] {
        &self.diagnostics
    }

    /// The document of a `parse` outcome: success at S2, with no items,
    /// diagnostics, artifacts or evaluation result; a failure uses the
    /// front-end's category, stage and diagnostic without running a stage.
    pub fn from_parse(result: &Result<Staged<ParsedSource>, FrontEndFailure>) -> Self {
        match result {
            Ok(_) => Self::new(Operation::Parse, Some(OutcomeStage::S2), Category::Success),
            Err(failure) => Self::from_failure(Operation::Parse, failure),
        }
    }

    /// The document of a `format` outcome. The complete UTF-8 text stays in
    /// the staged success value; this non-execute document has no result or
    /// artifacts. S2 is the reporting default, not a stage run by this adapter.
    /// Refusals retain their original typed diagnostic and catalog category;
    /// internal failures report no stage (FR-286).
    pub fn from_format<E>(result: &Result<Staged<String>, E>) -> Self
    where
        for<'a> &'a E: Into<FormatOutcomeRefusal>,
    {
        match result {
            Ok(_) => Self::new(Operation::Format, Some(OutcomeStage::S2), Category::Success),
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
    /// identity, so no artifact; a failure names its category and one
    /// diagnostic.
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
            .with_diagnostics(vec![OutcomeDiagnostic::from_compile_refusal(refusal)]),
            StageFailure::Limit(limit) => {
                let refusal = CompileRefusal::Limit(limit.clone());
                Self::new(
                    operation,
                    Some(refusal.stage().into()),
                    Category::Incomplete,
                )
                .with_diagnostics(vec![OutcomeDiagnostic::from_compile_refusal(&refusal)])
            }
            StageFailure::Cancelled(cause) => Self::cancelled(operation, *cause),
            StageFailure::Fault(fault) => Self::faulted(operation, fault),
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
    fn faulted(operation: Operation, fault: &quire_semantic_value::catalog::InternalFault) -> Self {
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
            Ok(ReplayResult::Witness(arm)) => {
                Self::new(Operation::Replay, Some(OutcomeStage::S8), arm.category())
            }
            Ok(ReplayResult::Input(arm)) => {
                Self::new(Operation::Replay, Some(OutcomeStage::S8), arm.category())
            }
            Ok(ReplayResult::ObservedTemporal(arm)) => {
                Self::new(Operation::Replay, Some(OutcomeStage::S8), arm.category())
            }
            Ok(ReplayResult::ModelGraph(arm)) => Self::new(
                Operation::Replay,
                Some(OutcomeStage::S8),
                ReplayResult::ModelGraph(arm.clone()).category(),
            ),
            Err(refusal) => Self::new(
                Operation::Replay,
                replay_stage(refusal),
                refusal.code().category(),
            )
            .with_diagnostics(vec![OutcomeDiagnostic::new(
                refusal.cause(),
                refusal.code().as_str(),
                None,
                refusal.to_string(),
            )]),
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
                document.result = Some(ExecuteResult::Completed {
                    value: match value {
                        CallValue::Boolean(value) => ResultValue::Boolean { value: *value },
                        CallValue::Integer(value) => ResultValue::Integer {
                            decimal: value.to_string(),
                        },
                        CallValue::Reference(reference) => ResultValue::Reference {
                            universe: reference.universe().root_type().to_string(),
                            object_type: crate::spine::effective_spelling(reference.object_type()),
                            object_identity: reference.object().as_str().to_owned(),
                        },
                    },
                });
            }
            CallOutcome::Undefined { reason } => {
                document.result = Some(ExecuteResult::Undefined { reason });
            }
            CallOutcome::Incomplete(incomplete) => {
                document.result = Some(ExecuteResult::Incomplete {
                    limit: ResultLimit::from(incomplete),
                });
            }
        }
        document
    }

    /// The document of a `run`: a call that ran is [`Self::from_call`]'s
    /// document; a refusal is an
    /// `execute` document at the stage it reached with the refusal's
    /// category and one diagnostic; a cancel is incomplete; a fault is
    /// internal failure with its catalog code.
    pub fn from_run(result: &Result<CallOutcome, Box<RunRefusal>>) -> Self {
        match result {
            Ok(outcome) => Self::from_call(outcome),
            Err(refusal) => match refusal.as_ref() {
                RunRefusal::Compile(compile) => Self::new(
                    Operation::Execute,
                    Some(compile.stage().into()),
                    refusal.category(),
                )
                .with_diagnostics(vec![OutcomeDiagnostic::from_compile_refusal(compile)]),
                RunRefusal::Cancelled(cause) => Self::cancelled(Operation::Execute, *cause),
                RunRefusal::Fault(fault) => Self::faulted(Operation::Execute, fault),
                // FR-277: a supplied-phase stop is phase-marked, not a
                // catalog record. Its category is the document's; the shared
                // result contract has no field for the phase context yet, so
                // nothing is invented for it here.
                RunRefusal::Supplied(stop) => match stop.cause {
                    quire_semantic_value::supplied_stop::SuppliedCause::Cancelled(cause) => {
                        Self::cancelled(Operation::Execute, cause)
                    }
                    quire_semantic_value::supplied_stop::SuppliedCause::Limit(_) => Self::new(
                        Operation::Execute,
                        Some(OutcomeStage::S6a),
                        refusal.category(),
                    ),
                    quire_semantic_value::supplied_stop::SuppliedCause::Input { .. }
                    | quire_semantic_value::supplied_stop::SuppliedCause::Storage(_)
                    | quire_semantic_value::supplied_stop::SuppliedCause::Capacity(_) => Self::new(
                        Operation::Execute,
                        Some(OutcomeStage::S6a),
                        refusal.category(),
                    )
                    .with_diagnostics(vec![OutcomeDiagnostic::new(
                        qsl_eval::value::supplied_stop_cause(stop),
                        qsl_eval::value::supplied_stop_code(stop).as_str(),
                        None,
                        refusal.to_string(),
                    )]),
                },
                RunRefusal::UnsupportedHelper(_) => Self::new(
                    Operation::Execute,
                    Some(OutcomeStage::S6a),
                    refusal.category(),
                ),
                RunRefusal::MissingDeclaration { .. }
                | RunRefusal::UnsupportedResult { .. }
                | RunRefusal::UnknownParameter { .. }
                | RunRefusal::DuplicateArgument { .. }
                | RunRefusal::UnboundParameter { .. }
                | RunRefusal::WrongValueKind { .. }
                | RunRefusal::Admission { .. }
                | RunRefusal::Storage { .. }
                | RunRefusal::DanglingReference { .. } => Self::new(
                    Operation::Execute,
                    Some(OutcomeStage::S6a),
                    refusal.category(),
                )
                .with_diagnostics(vec![OutcomeDiagnostic::new(
                    None,
                    refusal.code().as_str(),
                    None,
                    refusal.to_string(),
                )]),
            },
        }
    }

    /// The document's bytes: the same document gives the same bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, OutcomeWriteError> {
        Ok(serde_json::to_vec(self)?)
    }
}

/// The last stage a replay refusal reached: a recompile refusal's own spine
/// stage, S8 for every other refusal (the replay itself refused), and none
/// for a fault, whose stage is untyped.
fn replay_stage(refusal: &ReplayRefusal) -> Option<OutcomeStage> {
    match refusal {
        ReplayRefusal::Recompile(refusal) => Some(refusal.stage().into()),
        ReplayRefusal::Fault(_)
        | ReplayRefusal::InvalidPackageName(_)
        | ReplayRefusal::ObservedTemporalRefusal(qsl_eval::temporal::TemporalRefusal::Fault(_))
        | ReplayRefusal::Admission(AdmissionFailure::Fault(_)) => None,
        ReplayRefusal::Request(_)
        | ReplayRefusal::DomainPackages(_)
        | ReplayRefusal::LimitAboveReader(_)
        | ReplayRefusal::SourceCount(_)
        | ReplayRefusal::DependencySelections(_)
        | ReplayRefusal::DependencyInput(_)
        | ReplayRefusal::UnknownFunction { .. }
        | ReplayRefusal::ClaimOccurrence { .. }
        | ReplayRefusal::UnknownOperation { .. }
        | ReplayRefusal::WrongReplayTarget { .. }
        | ReplayRefusal::WrongObservedTemporalTarget { .. }
        | ReplayRefusal::WrongObservedTemporalSource
        | ReplayRefusal::WrongModelGraphEntry
        | ReplayRefusal::ModelGraphMismatch { .. }
        | ReplayRefusal::WrongModelGraphSource
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

    use ix_trace_rs::trace;
    use qsl_foundation::{Position, SourceIdentity};

    use qsl_semantics::model::object_environment::ObjectEnvironment;
    use quire_exact::{Cancel, CancelCause, Identifier, ScalarLimits};
    use quire_semantic_value::catalog::{CatalogCode, InternalFault};
    use serde_json::{json, Value};

    use super::*;
    use crate::identity::QualifiedName;

    #[test]
    fn timed_statistical_refusal_carries_typed_state_and_transition() {
        let state = qsl_eval::simulation::StateKey::from_canonical_bytes(b"[0,[]]".to_vec());
        let transition = quire_semantic_value::model_transition::ModelTransition::new(
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

    /// QSL-381: a node id nested 200 deep, past serde_json's 128-level
    /// recursion limit, writes and reads back without a depth cap.
    #[test]
    fn a_wire_node_nested_two_hundred_deep_round_trips() {
        let mut key = NodeKey::new(
            "fixture",
            vec![quire_exact::PathSegment::Text("leaf".into())],
            0,
        )
        .expect("a fixture key");
        for ordinal in 1..=200 {
            key = NodeKey::new(
                "fixture",
                vec![quire_exact::PathSegment::Node(key)],
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
        profile v = \"quire.value.complete/v1\";\n\
        type Digit = Int[0, 9];\n\
        function inv using v(x: Digit): Boolean pure { 1 / x > 0 }\n";

    type Checked = Result<Staged<CheckedUnit>, FrontEndFailure>;
    type Packaged = Result<Staged<EmittedUnit>, FrontEndFailure>;

    /// `check`, then `package`, over `text`.
    fn compiled(text: &str) -> (Checked, Option<Packaged>) {
        let source = SourceIdentity::new("agent-ix", "test:outcome", "fixture", "fixture:1");
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

    fn record(index: usize, value: TerminalValue) -> OutcomeItem {
        OutcomeItem::from_terminal(&TerminalRecord::from_value(RequestIndex::new(index), value))
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
                "format": "quire-outcome/1",
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
                "format": "quire-outcome/1",
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
                "format": "quire-outcome/1",
                "operation": "check",
                "last_stage": "S3",
                "category": "refusal",
                "items": [],
                "diagnostics": [{
                    "cause": "ambiguous-literal",
                    "code": "ill_typed",
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
        let expected = concat!(
            r#"{"format":"quire-outcome/1","operation":"analyze","last_stage":"S6c","#,
            r#""category":"unsupported","items":["#,
            r#"{"request_index":0,"result":"unsupported","cause":{"kind":"solver-absent"},"category":"unsupported"},"#,
            r#"{"request_index":1,"result":"proved","cause":null,"basis":{"type":"bounded-proof","checks":3},"certification":"certified","category":"success"},"#,
            r#"{"request_index":2,"result":"refuted","cause":null,"category":"violation"}"#,
            r#"],"diagnostics":[],"artifacts":[],"result":null}"#
        );
        assert_eq!(text_of(&document), expected);
        assert_eq!(document.to_bytes().unwrap(), document.to_bytes().unwrap());
        let (refused, _) = compiled(ILL_TYPED);
        assert_eq!(
            OutcomeDocument::from_check(&refused).to_bytes().unwrap(),
            OutcomeDocument::from_check(&refused).to_bytes().unwrap()
        );
    }

    /// QSpec FR-331-AC-8: a proof with no SUCCESS check is `inconclusive`
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

    /// QSpec FR-483: ValueValidity's no-assignment outcome is an
    /// `Inconclusive` terminal result carrying the same wire cause as a
    /// zero-check Kani proof, while remaining distinct in the typed record.
    #[trace("QSpec-TC-972", "QSpec-FR-483-AC-2")]
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
        use qsl_eval::simulation::Permutation;
        use qsl_foundation::ReductionKind;
        use quire_checked::key::DeclarationKey;
        use quire_semantic_value::fairness::PropertyForm;

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

    /// FR-169-AC-8 on the wire (QSpec FR-391): a `well-definedness-unchecked`
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
                        "format": "quire-outcome/1",
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
                assert_eq!(
                    item.cause.as_ref().unwrap().certificate_at,
                    Some(at.clone())
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
                    .expect("proof-step rejection document encodes");
                assert_eq!(document.to_bytes().unwrap(), bytes);
                let written: Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(
                    written,
                    json!({
                        "format": "quire-outcome/1",
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
                assert_eq!(
                    document.items()[2].cause.as_ref().unwrap().certificate_at,
                    Some(at)
                );
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
                    code: DeclineCode::Qsl(Code::IllTyped),
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
                    hypothesis: quire_exact::NodeKey::new(
                        "fixture",
                        vec![quire_exact::PathSegment::Text("node".into())],
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
                    reduction: qsl_foundation::ReductionKind::Symmetry,
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
                            obligation: crate::identity::ObligationIdentity::new(
                                crate::fixture_node(1),
                                None,
                                0,
                            ),
                            package_name: "fixture".to_owned(),
                            function: Some(
                                QualifiedName::new(vec![Identifier::new("f").unwrap()]).unwrap(),
                            ),
                            source: crate::ReplaySource::Input(Vec::new()),
                            limits: quire_exact::ScalarLimits {
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
                            supplied: quire_semantic_value::supplied_stop::SuppliedLimits::default(
                            ),
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
            quire_exact::Rational::new(1_i64.into(), 3_i64.into()).expect("nonzero divisor"),
        );
        let upper = crate::proof_result::ExactRational::new(
            quire_exact::Rational::new(2_i64.into(), 3_i64.into()).expect("nonzero divisor"),
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
                    "format": "quire-outcome/1",
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
        });
        assert_eq!(
            json_of(&execute),
            json!({
                "format": "quire-outcome/1",
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
                function: "seven".to_owned(),
                arguments: Vec::new(),
                accounting,
                supplied: Default::default(),
            };
            let outcome = run(
                SourceIdentity::new("agent-ix", "test:outcome", "fixture", "fixture:1"),
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
            json!({"kind": "completed", "value": {"kind": "integer", "decimal": "7"}})
        );
    }

    /// FR-286-AC-5: `seven` completes with integer 7 in the `result`
    /// member, and a `check` under an already cancelled handle is incomplete
    /// with no last stage and a null `result`.
    #[trace("TC-770", "FR-286-AC-5")]
    #[test]
    fn execute_result_and_cancelled_check_documents() {
        let call = Call {
            function: "seven".to_owned(),
            arguments: Vec::new(),
            accounting: default_accounting(1_000_000),
            supplied: Default::default(),
        };
        let outcome = run(
            SourceIdentity::new("agent-ix", "test:outcome", "fixture", "fixture:1"),
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
                "format": "quire-outcome/1",
                "operation": "execute",
                "last_stage": "S6a",
                "category": "success",
                "items": [],
                "diagnostics": [],
                "artifacts": [],
                "result": {"kind": "completed", "value": {"kind": "integer", "decimal": "7"}},
            })
        );

        let starved = Call {
            accounting: default_accounting(0),
            supplied: Default::default(),
            ..call
        };
        let outcome = run(
            SourceIdentity::new("agent-ix", "test:outcome", "fixture", "fixture:1"),
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
                "format": "quire-outcome/1",
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

        let source = SourceIdentity::new("agent-ix", "test:outcome", "fixture", "fixture:1");
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

    /// The whole-outcome category is the most severe item's, and a monitor
    /// clause pending at the end of the trace stays below a violation.
    #[trace("TC-770", "FR-286-AC-3")]
    #[test]
    fn the_document_category_is_the_most_severe_items() {
        let declined = TerminalValue::Declined {
            cause: ProofRefusalCause::Refused,
            code: DeclineCode::Qsl(Code::IllTyped),
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
        let pending = || {
            vec![record(
                0,
                TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(
                    Code::StaleDependency,
                )),
            )]
        };
        let monitored = OutcomeDocument::settled(Operation::Monitor, None, pending());
        assert_eq!(monitored.category(), Category::Inconclusive);
        assert_eq!(monitored.category().trace_exit_code(), 0);
        let violated = OutcomeDocument::settled(
            Operation::Monitor,
            None,
            pending()
                .into_iter()
                .chain([record(1, TerminalValue::Refuted)])
                .collect(),
        );
        assert_eq!(violated.category(), Category::Violation);
        assert_eq!(
            OutcomeDocument::settled(Operation::Prove, None, pending()).category(),
            Category::Inconclusive
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
        let span = qsl_foundation::LocatedSpan {
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
        assert_eq!(
            json_of(&OutcomeDocument::from_replay(&Ok(ReplayResult::Input(arm)))),
            json!({
                "format": "quire-outcome/1",
                "operation": "replay",
                "last_stage": "S8",
                "category": "success",
                "items": [],
                "diagnostics": [],
                "artifacts": [],
                "result": null,
            })
        );
        let unknown = ReplayRefusal::UnknownFunction {
            selection: QualifiedName::new(vec![Identifier::new("seven").expect("an identifier")])
                .expect("a qualified name"),
            package: qsl_semantics::library::LibraryName::new("fixture").expect("a library name"),
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

    fn run_source(
        text: &str,
        function: &str,
        cancel: &Cancel,
    ) -> Result<CallOutcome, Box<RunRefusal>> {
        run(
            SourceIdentity::new("agent-ix", "test:outcome", "fixture", "fixture:1"),
            "fixture".to_owned(),
            "unit.native",
            text.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            SpineLimits::default(),
            &Call {
                function: function.to_owned(),
                arguments: Vec::new(),
                accounting: default_accounting(1_000_000),
                supplied: Default::default(),
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
                "format": "quire-outcome/1",
                "operation": "execute",
                "last_stage": "S6a",
                "category": "success",
                "items": [],
                "diagnostics": [],
                "artifacts": [],
                "result": {"kind": "completed", "value": {"kind": "integer", "decimal": "7"}},
            })
        );
        assert_eq!(
            document["result"],
            json_of(&OutcomeDocument::from_call(call_outcome))["result"]
        );

        // Undefined: the call's undefined result.
        let undefined = Ok(CallOutcome::Undefined {
            reason: "division-by-zero",
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
}
