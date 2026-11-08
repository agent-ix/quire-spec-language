// SPDX-License-Identifier: AGPL-3.0-or-later
//! The S4 checked-package/v2 emitter (QSpec FR-322, ADR-011 T-8).
//!
//! It writes the checked semantic graph with structural node identities
//! `{package, path, ordinal}` and converts checked body terms into IR's typed
//! term vocabulary. A pending lowering reference refuses emission. The
//! package name is the checked graph's required manifest name.
//!
//! The emitter preserves graph order, occurrences, source regions, lock
//! selections and omission causes. The IR encoder writes the envelope.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use quire_contract_model::{
    encode_checked_package_v2, ApplicationOperator, CheckedAbstractionField,
    CheckedAbstractionFrame, CheckedAbstractionObject, CheckedAbstractionParameter,
    CheckedAbstractionPopulation, CheckedArtifactRef, CheckedCapability,
    CheckedCapabilityDisposition, CheckedComponentClause, CheckedCompositionComponent,
    CheckedCompositionPremise, CheckedDeclaration, CheckedDependencySelection,
    CheckedDiagnosticsV2, CheckedDimensionOperand, CheckedEnvironmentEntry,
    CheckedEnvironmentSupplies, CheckedFairnessGranularity, CheckedFairnessKind,
    CheckedFairnessSupply, CheckedFieldScope, CheckedFrameModification, CheckedGuaranteeMeasure,
    CheckedGuaranteeProgress, CheckedNodeId, CheckedNodeKind, CheckedNodeOwner, CheckedNodeTag,
    CheckedOccurrence, CheckedOccurrenceRole, CheckedOperandEntry, CheckedOperationAnchorBody,
    CheckedPackageEvidence, CheckedPackageLockV2, CheckedPackageSidecarsRef,
    CheckedPackageSourceCorrespondenceRef, CheckedPackageV2, CheckedRational,
    CheckedRefinementStepRowRegion, CheckedScopedDeletion, CheckedSelection, CheckedSelectionRole,
    CheckedSemanticGraphV2, CheckedSemanticNodeV2, CheckedSourceMapEntry, CheckedSourceRef,
    CheckedSourceRegion, CheckedTerm, DimensionTerm, LiteralKind, NominalBody, ProtocolForm,
    ScalarTypeForm, StateForm, ValueForm,
};
use serde::Deserialize;

use ix_trace_rs::string_edge;
use qsl_foundation::source::node_table::{scoped, DebugTable, DebugWith, With};
use qsl_foundation::source::provenance::{
    OccurrenceKey, PackageSourceMap, RawSourceRef, SourceRegion,
};
use qsl_foundation::Code;
use qsl_semantics::check::wire_id::StructuralIds;
use qsl_semantics::check::{
    AbstractionNodeRefusal, AbstractionNodes, Arrival, BindingValue, BodyTerm, CheckedGraph,
    CheckedRefinement, CheckedScheduleClaim, CheckedTaskSets, FrameDeletesScoped, FrameModifies,
    FrameScope, FrameTerm, GroupMember, GroupTerm, LawRole, LeafTerm, LiteralValue, MeasureEntry,
    MemberTerm, NodeRef, NodeTag, OperationLaw, Operator, Owner, ScheduleClaim, SchedulingPolicy,
    SemanticNode, SuppliesEntry, TemporalProfile, TimeNominalForm, TimeNominalOrigin, TupleMember,
    TupleTerm, Wcet,
};
use qsl_semantics::value::nominal_payload::NominalPayload;
use qsl_semantics::value::{
    native_diagnostics_catalog, DefinitionReference, DefinitionRole, HeaderLayer, Member,
    ModelRole, NodeOwner, TemporalRole,
};
use quire_exact::{Cancel, NodeKey, Origin, PathSegment, Role};
use quire_semantic_value::catalog::Category;
use quire_semantic_value::location::ExpressionLocation as Location;

use super::{CheckedPackage, EmittedPackage};
use crate::checked::EmitLimits;
use crate::checked_v2::{
    CheckedClockConstraint, CheckedClockGuard, CheckedClockPlace, CheckedClockReset,
    CheckedConstraintTarget, CheckedGuardPart, CheckedResetTarget, CheckedTimeInvariant,
    CheckedTimedModel, CheckedUrgency, ClockComparison, TimedNodeMetadata,
};
use crate::wire_form::{OperationSuffix, WireForm};

/// Whether a member binding is named `body`, the binding that carries a
/// function's body term.
#[string_edge]
fn is_body_binding(name: &str) -> bool {
    name == "body"
}

/// The FR-322 graph schema version every node carries.
const GRAPH_V2: &str = "quire.checked-semantic-graph/v2";

/// The diagnostics catalog the package's (empty) diagnostics are qualified
/// by, written at `diagnostics.catalog`: QSpec's `quire.native.diagnostics/v1`
/// `DefinitionRef`.
fn diagnostics_catalog() -> CheckedArtifactRef {
    artifact(&native_diagnostics_catalog())
}

/// Why the v2 emitter writes no bytes for `package` at all. A node that
/// cannot be written is omitted instead ([`OmittedNode`]).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum EmitRefusal {
    /// No node of the package can be written, and FR-322 admits no empty
    /// semantic graph.
    #[error("no node of the package can be written to a checked-package/v2 graph")]
    NothingToEmit {
        /// Every node the wire would omit, with its cause.
        omitted: Vec<OmittedNode>,
    },
    /// The graph has protocol-local refusals, so only its per-protocol result
    /// API may expose the successful protocol nodes.
    #[error("per-protocol refusals require emit_checked_with_protocol_results")]
    ProtocolResultsRequired,
    /// The caller's region conversion places no region, or an empty one,
    /// for one occurrence.
    #[error("occurrence {role:?}/{ordinal} of node {node} has no source region")]
    UnlocatedOccurrence {
        /// The node.
        node: NodeKey,
        /// The occurrence's role.
        role: CheckedOccurrenceRole,
        /// The occurrence's ordinal.
        ordinal: u64,
    },
    /// `check` recorded an occurrence role FR-322 does not name.
    #[error("occurrence role {role} of node {node} is not an FR-322 occurrence role")]
    UnknownOccurrenceRole {
        /// The node.
        node: NodeKey,
        /// The role as `check` spelled it.
        role: String,
    },
    /// A node body or the wire could not be encoded.
    #[error("the checked-package/v2 wire could not be encoded: {reason}")]
    Encoding {
        /// The encoder's message.
        reason: String,
    },
    /// The checker's retained protocol results and emitted node identities
    /// do not agree.
    #[error("the checked graph's per-protocol result mapping is incomplete")]
    ProtocolResultInvariant,
    /// The package's canonical bytes would exceed the caller's
    /// `emit.output_bytes` ceiling (FR-264 Behavior 5). The emitter writes no
    /// bytes.
    #[error("{0}")]
    OutputLimit(qsl_foundation::diagnostic::LimitExceeded),
    /// A protocol reference is bound to a key that is not the key of a node
    /// the emitter writes (FR-218 stale target): a reference binds a key,
    /// never a name, and a name is never looked up again.
    #[error(
        "protocol reference {reference:?} is bound to {target:?}, which no written node carries"
    )]
    StaleProtocolReference {
        /// The reference's node.
        reference: CheckedNodeId,
        /// The key it is bound to.
        target: CheckedNodeId,
    },
    /// The package's abstraction relation has no node to write (FR-306).
    #[error("the abstraction relation cannot be written: {0}")]
    Abstraction(AbstractionNodeRefusal),
    /// The caller's [`quire_exact::Cancel`] handle was cancelled, and the
    /// emitter stopped at its next charge (FR-276).
    #[error("the emitter was cancelled ({cause:?})")]
    Cancelled {
        /// Why the handle was cancelled.
        cause: quire_exact::CancelCause,
    },
    /// The package declares checked random parameters, rewards or
    /// workloads (FR-185 "Package emission"), which FR-322's v2 vocabulary
    /// has no node for: the emitter refuses rather than write a package
    /// without them, naming the first such declaration in source order.
    #[error(
        "{} `{declaration}` cannot be written to a checked-package/v2 graph yet",
        feature.name()
    )]
    ProbabilisticModelNotEmitted {
        /// The first such declaration's name: `operation(parameter)` for a
        /// random parameter, the declared name of a reward or a workload.
        declaration: String,
        /// Its feature.
        feature: qsl_semantics::check::ProbabilisticFeature,
        /// The unit's selected header profile identity, when it selects one.
        profile: Option<&'static str>,
    },
    /// Interim fail-closed guard: this emission path does not yet map a
    /// checked time source into `timed_model` nodes, and it cannot omit one
    /// because the time source is part of the package's meaning.
    /// [`emit_timed_models`] writes typed timed models; QSL-1032 maps
    /// `CheckedGraph::time_sources` into them here and removes this guard
    /// (FR-230-AC-32, TC-685 step 26).
    #[error(
        "timed model `{model}` is not yet mapped to checked-package/v2 timed nodes on this path (QSL-1032)"
    )]
    TimedModelNotEmitted {
        /// The first timed model's alias.
        model: String,
    },
    /// The typed timed models do not describe exactly the timed nodes in the
    /// already-admitted base package.
    #[error("typed timed models do not match the admitted package's timed-node set")]
    TimedModelNodeSetMismatch,
    /// A checked probabilistic claim has no emitted node or `claim`
    /// occurrence to write its FR-322 row from (FR-498): lowering keys every
    /// claim of an assembled package, so this is a broken invariant.
    #[error("probabilistic claim `{claim}` has no emitted claim node and occurrence")]
    UnkeyedProbabilisticClaim {
        /// The claim's declared name (diagnostic text only).
        claim: String,
    },
    /// The timed nodes cannot be put in QSpec FR-322's declared order.
    #[error("timed aggregate order: {0}")]
    TimedOrder(#[source] timed_order::OrderFault),
    /// A typed timed model cannot be serialized with its available literals.
    #[error("typed timed-model writer invariant: {0}")]
    TimedModelWriterInvariant(&'static str),
    /// The plan-root writer found the package missing a node it needs
    /// (QSpec FR-487).
    #[error("the tick-monitor plan cannot be written: {0}")]
    MonitorPlanInvariant(&'static str),
    /// The plan-root writer found a name or selection declared more than
    /// once and refuses to bind the first (QSpec FR-487).
    #[error("the tick-monitor plan cannot be written: {name} is declared more than once")]
    MonitorPlanAmbiguous {
        /// The ambiguous declared name.
        name: String,
    },
    /// A checked refinement has no selected subject key to bind.
    #[error("a checked refinement has no selected state-machine subject node")]
    RefinementSubjectNotEmitted,
    /// The refinement identity preimage exceeded the emitter's canonical
    /// encoding limit or could not be encoded.
    #[error("the checked refinement identity preimage could not be encoded")]
    RefinementIdentityEncoding,
    /// A checked refinement is not supported by the current emission slice.
    #[error("the checked refinement body is not supported by the current V2 emission slice")]
    RefinementNotEmitted,
    /// A checked refinement's declaration occurrence has no original source region.
    #[error("the checked refinement declaration has no source region")]
    UnlocatedRefinementDeclaration,
    /// The emitted source correspondence could not be represented in-process.
    #[error("the emitted source map violated its typed correspondence invariant")]
    SourceMapInvariant,
    /// A lowering-local placeholder reached the wire boundary unresolved.
    #[error("a checked node body contains an unresolved node reference")]
    UnresolvedNodeReference,
    /// A written node's path names an operand node the graph does not hold,
    /// so the operand has no index segment (FR-322 "Operand index segments").
    #[error("node {node:?} names operand {operand:?}, which the graph does not hold")]
    OperandNotInGraph {
        /// The written node.
        node: Box<CheckedNodeId>,
        /// Its operand.
        operand: Box<CheckedNodeId>,
    },
    /// A dimension equivalence names a dimension of another package. IR
    /// admits that only as a Package operand against a direct dependency
    /// selection, which the emitter does not yet establish (QSL-1177); a
    /// Local operand must not carry a foreign package.
    #[error("dimension equivalence {node:?} names operand {operand:?} of another package")]
    ForeignDimensionOperand {
        /// The dimension-equivalence node.
        node: Box<CheckedNodeId>,
        /// The operand from another package.
        operand: Box<CheckedNodeId>,
    },
    /// An equality producer identity row has no unique emitted application
    /// occurrence to which it can be attached.
    #[error("equality producer identity for application {application:?} has no unique emitted occurrence")]
    MissingEqualityIdentityContext {
        /// The exact checked application key retained by QSL lowering.
        application: NodeKey,
    },
    /// A temporal declaration whose selected profile requires a clock
    /// configuration has no exact producer selection.
    #[error(
        "temporal declaration at {source_ref:?} bytes {span:?} has no selected clock configuration"
    )]
    MissingTemporalClockSelection {
        /// The checked source document.
        source_ref: qsl_foundation::source::provenance::RawSourceRef,
        /// The exact declaration locus.
        span: qsl_foundation::Span,
    },
    /// A temporal clock selection does not name a configured declaration in
    /// the checked source.
    #[error(
        "temporal clock selection at {source_ref:?} bytes {span:?} has no configured declaration"
    )]
    SurplusTemporalClockSelection {
        /// The selected source document.
        source_ref: qsl_foundation::source::provenance::RawSourceRef,
        /// The selected declaration locus.
        span: qsl_foundation::Span,
    },
    /// A temporal clock selection repeats one checked source locus.
    #[error("temporal clock selection at {source_ref:?} bytes {span:?} is duplicated")]
    DuplicateTemporalClockSelection {
        /// The selected source document.
        source_ref: qsl_foundation::source::provenance::RawSourceRef,
        /// The selected declaration locus.
        span: qsl_foundation::Span,
    },
    /// The selected configuration alternative does not match the checked
    /// declaration's selected temporal profile.
    #[error("temporal clock configuration at {source_ref:?} bytes {span:?} does not match profile {profile}")]
    TemporalClockProfileMismatch {
        /// The checked source document.
        source_ref: qsl_foundation::source::provenance::RawSourceRef,
        /// The exact declaration locus.
        span: qsl_foundation::Span,
        /// The selected temporal profile identity.
        profile: &'static str,
    },
    /// A selected clock name or one of its exact configuration fields is
    /// empty, oversized, noncanonical or outside its declared range.
    #[error("temporal clock selection at {source_ref:?} bytes {span:?} has an invalid {field}")]
    InvalidTemporalClockValue {
        /// The checked source document.
        source_ref: qsl_foundation::source::provenance::RawSourceRef,
        /// The exact declaration locus.
        span: qsl_foundation::Span,
        /// The invalid clock member.
        field: &'static str,
    },
    /// A dispatch producer decision has no unique emitted call occurrence or
    /// one of its exact model/function identities is not in the emitted graph.
    #[error("dispatch producer decision for application {application:?} has no complete emitted identity context")]
    MissingDispatchIdentityContext {
        /// The exact checked application key retained by QSL lowering.
        application: NodeKey,
    },
    /// A term of an emitted protocol slot expression has no single emitted
    /// application, recorded occurrence or slot site name to write its
    /// FR-383 `protocol_locations` row from.
    #[error("protocol slot term {application:?} has no complete emitted location context")]
    MissingProtocolLocationContext {
        /// The checked application key lowered at the term.
        application: NodeKey,
    },
    /// A term of an emitted protocol slot expression whose kind always
    /// lowers to an application has no application recorded at its
    /// `Location`, so FR-383's one row per term cannot be written.
    #[error("protocol slot term at {location:?} has no recorded application")]
    UnrecordedProtocolTerm {
        /// The checker's `Location` of the term.
        location: Location,
    },
}

/// One explicit clock configuration selected for an exact checked temporal
/// declaration. The emitter obtains the clock name and structural
/// declaration identity from the checked graph; this selection supplies only
/// the independent typed clock configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalClockSelection {
    /// The exact source document containing the checked declaration.
    pub source: qsl_foundation::source::provenance::RawSourceRef,
    /// The declaration's exact half-open source byte span.
    pub declaration_span: qsl_foundation::Span,
    /// The independently selected typed clock configuration.
    pub clock: quire_contract_model::ClockConfiguration,
    /// The selected temporal definition identity. Emission requires it to
    /// equal the declaration's `temporal_profile` law, which the lock's
    /// `profile_selections` row names (QSpec FR-322 "Temporal clock
    /// bindings"); the wire carries no second copy.
    pub definition: quire_contract_model::CheckedArtifactRef,
}

impl TemporalClockSelection {
    /// The selection of `clock` for the declaration at `declaration_span`,
    /// naming the temporal definition `clock`'s alternative selects
    /// (QSpec FR-322 "Temporal clock bindings": the selected definition
    /// determines exactly the `clock.kind`).
    pub fn new(
        source: qsl_foundation::source::provenance::RawSourceRef,
        declaration_span: qsl_foundation::Span,
        clock: quire_contract_model::ClockConfiguration,
    ) -> Self {
        Self {
            definition: profile_definition(clock_profile(&clock)),
            source,
            declaration_span,
            clock,
        }
    }
}

impl From<quire_canonical::Error> for EmitRefusal {
    fn from(error: quire_canonical::Error) -> Self {
        match error {
            quire_canonical::Error::Limit(limit) => {
                Self::OutputLimit(output_limit(limit.bound, u128::from(limit.required)))
            }
            other => encoding(other),
        }
    }
}

/// The `emit.output_bytes` outcome for a package whose canonical bytes reach
/// `actual` under `bound`.
fn output_limit(bound: u64, actual: u128) -> qsl_foundation::diagnostic::LimitExceeded {
    qsl_foundation::diagnostic::LimitExceeded::new(
        qsl_foundation::Setting::EmitOutputBytes,
        bound,
        actual,
    )
}

impl EmitRefusal {
    /// The catalog code (FR-010).
    pub fn code(&self) -> Code {
        match self {
            Self::NothingToEmit { .. }
            | Self::UnlocatedOccurrence { .. }
            | Self::UnknownOccurrenceRole { .. }
            | Self::ProtocolResultsRequired => Code::UnsupportedProjection,
            Self::MissingTemporalClockSelection { .. } => Code::MissingDeclaration,
            Self::SurplusTemporalClockSelection { .. }
            | Self::DuplicateTemporalClockSelection { .. }
            | Self::TemporalClockProfileMismatch { .. } => Code::InvalidPackage,
            Self::InvalidTemporalClockValue { .. } => Code::InvalidPackage,
            Self::Encoding { .. } => Code::OutputFailure,
            Self::Abstraction(AbstractionNodeRefusal::Internal(_)) => Code::RuntimeInvariant,
            Self::Abstraction(_) => Code::UnsupportedProjection,
            Self::StaleProtocolReference { .. } => Code::MissingDeclaration,
            Self::Cancelled { .. } => Code::Cancelled,
            Self::ProbabilisticModelNotEmitted { .. } | Self::TimedModelNotEmitted { .. } => {
                Code::UnsupportedConstruct
            }
            Self::MonitorPlanAmbiguous { .. } => Code::InvalidProjectionCorrespondence,
            Self::TimedModelNodeSetMismatch
            | Self::UnkeyedProbabilisticClaim { .. }
            | Self::TimedOrder(_) => Code::RuntimeInvariant,
            Self::TimedModelWriterInvariant(_)
            | Self::MonitorPlanInvariant(_)
            | Self::RefinementSubjectNotEmitted => Code::RuntimeInvariant,
            Self::RefinementIdentityEncoding => Code::OutputFailure,
            Self::RefinementNotEmitted => Code::UnsupportedConstruct,
            Self::UnlocatedRefinementDeclaration => Code::UnsupportedProjection,
            Self::SourceMapInvariant
            | Self::UnresolvedNodeReference
            | Self::OperandNotInGraph { .. }
            | Self::MissingEqualityIdentityContext { .. } => Code::RuntimeInvariant,
            Self::ForeignDimensionOperand { .. } => Code::UnsupportedConstruct,
            Self::MissingDispatchIdentityContext { .. }
            | Self::MissingProtocolLocationContext { .. }
            | Self::UnrecordedProtocolTerm { .. } => Code::RuntimeInvariant,
            Self::ProtocolResultInvariant => Code::RuntimeInvariant,
            Self::OutputLimit(_) => Code::StageLimitExceeded,
        }
    }

    /// The catalog cause, where the code has one (FR-010).
    pub fn cause(&self) -> Option<&'static str> {
        match self {
            Self::StaleProtocolReference { .. } => Some("missing-name"),
            Self::ProbabilisticModelNotEmitted { .. } | Self::TimedModelNotEmitted { .. } => {
                Some("not-yet-implemented")
            }
            Self::RefinementNotEmitted => Some("not-yet-implemented"),
            Self::NothingToEmit { .. }
            | Self::ProtocolResultsRequired
            | Self::UnlocatedOccurrence { .. }
            | Self::UnknownOccurrenceRole { .. }
            | Self::Encoding { .. }
            | Self::Abstraction(_)
            | Self::Cancelled { .. }
            | Self::TimedModelNodeSetMismatch
            | Self::UnkeyedProbabilisticClaim { .. }
            | Self::TimedOrder(_)
            | Self::TimedModelWriterInvariant(_)
            | Self::MonitorPlanInvariant(_)
            | Self::MonitorPlanAmbiguous { .. }
            | Self::RefinementSubjectNotEmitted
            | Self::RefinementIdentityEncoding
            | Self::UnlocatedRefinementDeclaration
            | Self::SourceMapInvariant
            | Self::UnresolvedNodeReference
            | Self::OperandNotInGraph { .. }
            | Self::MissingEqualityIdentityContext { .. } => None,
            Self::MissingTemporalClockSelection { .. } => Some("missing-selection"),
            Self::SurplusTemporalClockSelection { .. }
            | Self::DuplicateTemporalClockSelection { .. }
            | Self::TemporalClockProfileMismatch { .. } => Some("timed-node-reference"),
            Self::InvalidTemporalClockValue { .. } => Some("invalid-value"),
            Self::ForeignDimensionOperand { .. } => Some("not-yet-implemented"),
            Self::MissingDispatchIdentityContext { .. }
            | Self::MissingProtocolLocationContext { .. }
            | Self::UnrecordedProtocolTerm { .. } => None,
            Self::ProtocolResultInvariant => None,
            Self::OutputLimit(limit) => Some(limit.kind().catalog_cause()),
        }
    }
}

/// One node the wire omits, and why.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OmittedNode {
    /// The omitted node.
    pub node: CheckedNodeId,
    /// Why it is omitted.
    pub cause: OmissionCause,
}

/// Why a node is omitted from the wire.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OmissionCause {
    /// IR's v2 vocabulary has no such form for the tag.
    UnsupportedForm {
        /// The node's tag.
        node_tag: NodeTag,
        /// The node's form.
        semantic_form: &'static str,
    },
    /// The node carries a `declaration` and no `declaration` occurrence, or
    /// the reverse; FR-322 requires both or neither.
    DeclarationOccurrenceMismatch,
    /// A nominal node whose owner is not the checked unit's source, so the
    /// lock the emitter writes selects no owner it joins.
    UnlockedOwner,
    /// The node names a node the checked graph does not hold.
    NamesAbsentNode(CheckedNodeId),
    /// The node names a node the wire omits.
    NamesOmittedNode(CheckedNodeId),
}

// The typed timed graph the last emission on this thread built, for tests:
// IR's typed bodies cannot be decoded from JSON outside its reader.
#[cfg(test)]
thread_local! {
    pub(crate) static TIMED_GRAPH: std::cell::RefCell<
        Option<(Vec<CheckedSemanticNodeV2>, Vec<CheckedSourceMapEntry>)>,
    > = const { std::cell::RefCell::new(None) };
}

/// One source-owned record's QSL-stage declaration witness (FR-092 "Source
/// record identity rows"). The checking stage identifies a source record's
/// fields by declared name and issues them no member identity, so this row
/// has no `MemberId`; only model records carry one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedSourceRecordRow {
    /// The record declaration's emitted `composite_type` node.
    pub structural_node: CheckedNodeId,
    /// Authority of the selected raw source.
    pub source_authority: Box<str>,
    /// Identity of the selected raw source.
    pub source_identity: Box<str>,
    /// Original qualified declaration segments in that source.
    pub qualified_declaration: Vec<Box<str>>,
    /// The checked declaration's key, which every `ValueType::Composite` of
    /// the record carries.
    pub record_key: NodeKey,
    /// Every member in declaration order, unselected members included.
    pub members: Vec<SelectedSourceRecordMemberRow>,
}

/// One member of a [`SelectedSourceRecordRow`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedSourceRecordMemberRow {
    /// The declared field name, read from the checked declaration.
    pub original_member_identity: Box<str>,
    /// The record node's binding name, read from the emitted node.
    pub admitted_name: Box<str>,
    /// The declared presence, never inferred from an `Option` type.
    pub presence: quire_exact::Presence,
    /// The emitted type node the binding references.
    pub type_node: CheckedNodeId,
    /// The checked field type after presence handling.
    pub value_type: quire_exact::ValueType,
}

/// One model-owned enum (a variant type of a selected domain model)'s checked
/// variant shape, joined to the final emitted declaration and case nodes
/// (FR-092 "Model enum shape rows"). It carries the checker's
/// [`quire_exact::EnumShape`] and each case's [`quire_exact::VariantId`] with
/// the original model identities, and derives no variant from a node key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedModelEnumShapeRow {
    /// The variant type declaration's emitted node.
    pub structural_node: CheckedNodeId,
    /// The checked declaration's key, which every enum type of it carries.
    pub node_key: NodeKey,
    /// The selected domain model document.
    pub selected_document: Box<str>,
    /// The declaration's original IR identity in that document.
    pub original_identity: Box<str>,
    /// The checked shape: the ordered flag and the canonical variant sequence.
    pub shape: quire_exact::EnumShape,
    /// Every case in the shape's canonical order.
    pub cases: Vec<SelectedModelEnumShapeCaseRow>,
}

/// One case of a [`SelectedModelEnumShapeRow`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedModelEnumShapeCaseRow {
    /// The selected case label.
    pub case: Box<str>,
    /// The case's emitted member node.
    pub structural_node: CheckedNodeId,
    /// The case's checked member key.
    pub member_key: NodeKey,
    /// The case's original IR identity in the selected document.
    pub member_original_identity: Box<str>,
    /// The checked variant the shape holds at this case's rank.
    pub variant: quire_exact::VariantId,
}

/// One collection type node the package carries, paired with the checker's
/// collection type (FR-092 "Source collection rows"). A collection is
/// anonymous, so the row has a node and no declaration or source owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedSourceCollectionRow {
    /// The collection type's emitted node: the `collection_bounds` domain of a
    /// bounded collection, otherwise the collection composite node itself.
    pub structural_node: CheckedNodeId,
    /// The emitted collection composite node holding the element reference.
    pub base_node: CheckedNodeId,
    /// The emitted element type node the base references; a map's value
    /// type node.
    pub element_node: CheckedNodeId,
    /// A map's emitted key type node, which the base references before the
    /// value; `None` for every other collection.
    pub key_node: Option<CheckedNodeId>,
    /// The checker's `ValueType::Collection` or `ValueType::Map`, kind,
    /// element and bound included.
    pub value_type: quire_exact::ValueType,
}

/// The emitted package and the nodes it omits.
#[derive(Clone)]
pub struct Emission {
    /// The v2 bytes and their manifest package name.
    pub(crate) package: EmittedPackage,
    /// Every node of the checked graph the wire omits, ascending by node id.
    pub(crate) omitted: Vec<OmittedNode>,
    /// The lock's required features: the evidence IR's reader needs when
    /// these bytes are read back as a dependency's import view (ADR-015 D-1
    /// step 6).
    pub(crate) evidence: CheckedPackageEvidence,
    /// FR-322's feature-level report retained as typed data for inspection.
    pub(crate) capability_report: Vec<CheckedCapability>,
    /// The exact source correspondence emitted with these package bytes.
    pub(crate) source_map: PackageSourceMap,
    /// QSL's selected model-operation witnesses, keyed by the admitted anchor
    /// identities written in these bytes.
    pub(crate) selected_operation_member_rows:
        Vec<quire_contract_model::SelectedOperationMemberRow>,
    /// QSL-stage tuple identities keyed by admitted model tuple nodes.
    pub(crate) selected_tuple_declaration_rows:
        Vec<quire_contract_model::SelectedTupleDeclarationRow>,
    /// QSL-stage source-unit identities keyed by admitted source unit nodes.
    pub(crate) selected_source_unit_rows: Vec<quire_contract_model::SelectedSourceUnitRow>,
    /// QSL-stage identities for selected model-owned enum declarations/cases.
    pub(crate) selected_model_enum_declaration_rows:
        Vec<quire_contract_model::SelectedModelEnumDeclarationRow>,
    pub(crate) selected_model_enum_member_rows:
        Vec<quire_contract_model::SelectedModelEnumMemberRow>,
    /// QSL-stage identities for emitted source-owned enum declarations and
    /// their cases (FR-092 "Source enum identity rows").
    pub(crate) selected_enum_declaration_rows:
        Vec<quire_contract_model::SelectedEnumDeclarationRow>,
    pub(crate) selected_enum_member_rows: Vec<quire_contract_model::SelectedEnumMemberRow>,
    /// QSL-stage identities for emitted source-owned unions: declaration,
    /// sealed members and ordered payload nodes (FR-092 "Source union
    /// identity rows").
    pub(crate) selected_source_union_rows: Vec<quire_contract_model::SelectedSourceUnionRow>,
    /// QSL-stage declaration witnesses for emitted source-owned records
    /// (FR-092 "Source record identity rows").
    pub(crate) selected_source_record_rows: Vec<SelectedSourceRecordRow>,
    /// QSL-stage declaration and ordered position witnesses for emitted
    /// source-owned tuples (FR-092 "Source tuple identity rows").
    pub(crate) selected_source_tuple_rows: Vec<quire_contract_model::SelectedSourceTupleRow>,
    /// QSL-stage checked variant shapes of emitted model-owned enums.
    pub(crate) selected_model_enum_shape_rows: Vec<SelectedModelEnumShapeRow>,
    /// QSL-stage collection type facts for emitted collection nodes.
    pub(crate) selected_source_collection_rows: Vec<SelectedSourceCollectionRow>,
    /// The checking stage's selected model declaration rows (record, object,
    /// declared unit), retained from the acquisition these bytes were
    /// emitted from. A collection type is structural and has no declaration row.
    pub(crate) selected_exact_declaration_rows:
        Vec<quire_contract_model::SelectedExactDeclarationRow>,
    /// The checking stage's selected record-field rows, retained from the same
    /// acquisition.
    pub(crate) selected_exact_field_rows: Vec<quire_contract_model::SelectedExactFieldRow>,
    /// Exact QSL identity witnesses for emitted equality occurrences.
    // Final-emission producer membership, computed before budgeted materialization.
    identity_row_selection: identity_rows::IdentityRowSelection,
    pub(crate) equality_producer_identity_rows: Vec<EqualityProducerEmissionRow>,
    /// Per-fact equality witnesses joined to final structural node ids.
    pub(crate) equality_fact_witness_rows: Vec<EqualityFactWitnessRow>,
    /// The typed graph nodes exactly as encoded into [`Self::package`], with
    /// package-level index segments. IR's typed bodies (an operation anchor's
    /// bare execution point) are not decodable through serde outside its
    /// admitting reader, so protocol results read these rather than the bytes.
    pub(crate) wire_nodes: Vec<CheckedSemanticNodeV2>,
    /// The `temporal_bindings` rows of the checked graph's temporal and
    /// hyper clauses, keyed by the graph's own clause identity.
    pub(crate) temporal_binding_rows: TemporalBindingRows,
    /// The clause references, `profile_selections` and `temporal_bindings`
    /// rows exactly as written into [`Self::package`] (FR-337).
    pub(crate) temporal_rows: TemporalRows,
}

/// One declaration of a checked temporal clause as the emitter wrote it
/// (FR-337): the id the emitted bytes assign its node, its claim occurrence
/// ordinal, and the position of the clause in the checked graph's temporal
/// clauses. The pair `declaration`, `occurrence` equals a
/// `temporal_clause_names` row of the emitted bytes; no other derivation of
/// the id exists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalClauseRef {
    /// The emitted clause node.
    pub declaration: CheckedNodeId,
    /// The claim occurrence ordinal of this declaration.
    pub occurrence: u64,
    /// The position of the clause in the checked graph's temporal clauses.
    pub clause: usize,
}

/// The temporal rows of an emission, exactly as written (FR-337, FR-515).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TemporalRows {
    clauses: Vec<TemporalClauseRef>,
    profile_selections: Vec<quire_contract_model::CheckedSelection>,
    bindings: Vec<quire_contract_model::TemporalBinding>,
}

impl TemporalRows {
    /// One reference per claim occurrence of every checked temporal clause,
    /// ascending by `declaration` and `occurrence` like the name rows.
    pub fn clauses(&self) -> &[TemporalClauseRef] {
        &self.clauses
    }

    /// The lock's `profile_selections` rows.
    pub fn profile_selections(&self) -> &[quire_contract_model::CheckedSelection] {
        &self.profile_selections
    }

    /// The package's `temporal_bindings` rows, ascending by `declaration`
    /// and `occurrence`; empty when no clause has a clock configuration.
    pub fn bindings(&self) -> &[quire_contract_model::TemporalBinding] {
        &self.bindings
    }
}

/// The `temporal_bindings` rows (FR-050, FR-515) an emission wrote for the
/// checked graph's clauses, keyed by the clause's [`NodeKey`] and claim
/// occurrence ordinal, so a reader of the in-process package finds a
/// declaration's row without the wire's node identities (FR-522
/// "Authentication").
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TemporalBindingRows {
    rows: Vec<(NodeKey, quire_contract_model::TemporalBinding)>,
    declarations: Vec<(NodeKey, u64, quire_contract_model::CheckedNodeId)>,
}

impl TemporalBindingRows {
    /// The retained checked clause keys, occurrence ordinals and emitted
    /// declaration identities, borrowed in their original row order.
    ///
    /// Callers with an explicit work allowance can charge each comparison
    /// before searching, without copying or reconstructing this table.
    pub fn declaration_rows(&self) -> &[(NodeKey, u64, quire_contract_model::CheckedNodeId)] {
        &self.declarations
    }

    /// The row of `declaration`'s claim occurrence `occurrence`, `None` when
    /// the lock selects no clock configuration for it.
    pub fn get(
        &self,
        declaration: &NodeKey,
        occurrence: u64,
    ) -> Option<&quire_contract_model::TemporalBinding> {
        self.rows
            .iter()
            .find(|(key, row)| key == declaration && row.occurrence == occurrence)
            .map(|(_, row)| row)
    }

    /// The checked-package declaration identity for one exact checked
    /// temporal clause occurrence.
    pub fn declaration(
        &self,
        declaration: &NodeKey,
        occurrence: u64,
    ) -> Option<&quire_contract_model::CheckedNodeId> {
        self.declarations
            .iter()
            .find(|(key, ordinal, _)| key == declaration && *ordinal == occurrence)
            .map(|(_, _, checked)| checked)
    }

    /// The number of keyed rows.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether no clause has a row.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// A protocol's result from checked-package emission (FR-383).
#[derive(Clone, Debug, PartialEq)]
pub enum ProtocolEmissionResult {
    /// The declaration emitted its checked clause and owned nodes. The node
    /// records retain structural operand ids, so a sibling protocol cannot
    /// change their bytes by shifting package-level index segments.
    Emitted {
        /// The authored protocol declaration name.
        declaration: String,
        /// The structural identity of the emitted clause.
        clause_key: NodeKey,
        /// The protocol-owned semantic nodes in canonical package order,
        /// with structural operand ids and no package-level index segments.
        nodes: Vec<serde_json::Value>,
    },
    /// The declaration was refused during checking. Independent results are
    /// retained alongside this refusal.
    Refused {
        /// The authored protocol declaration name.
        declaration: String,
        /// The declaration's index in the original source unit.
        source_index: usize,
        /// The span of the declaration's name.
        declaration_span: qsl_foundation::Span,
        /// The first typed refusal attributed to this declaration.
        refusal: qsl_semantics::check::CheckRefusal,
    },
    /// The protocol could not finish within its shared checking budget.
    Incomplete {
        /// The authored protocol declaration name.
        declaration: String,
        /// The typed S3 work-limit refusal.
        limit: qsl_semantics::check::StageLimitCause,
    },
}

/// The unit-level result of emitting independent protocol results.
#[derive(Clone, Debug, PartialEq)]
pub enum ProtocolEmissionAggregate {
    /// Every protocol declaration emitted successfully.
    Emitted,
    /// At least one protocol was refused; no package is admitted.
    Refused {
        /// The first refused protocol's primary record.
        primary: qsl_semantics::check::CheckRefusal,
    },
    /// No protocol was refused, but at least one is incomplete.
    Incomplete {
        /// The first incomplete protocol's typed limit.
        primary: qsl_semantics::check::StageLimitCause,
    },
}

/// Per-protocol results and the aggregate emission decision.
#[derive(Clone)]
pub struct ProtocolEmission {
    results: Vec<ProtocolEmissionResult>,
    aggregate: ProtocolEmissionAggregate,
    emission: Option<Emission>,
}

impl ProtocolEmission {
    /// Protocol results in source declaration order.
    pub fn results(&self) -> &[ProtocolEmissionResult] {
        &self.results
    }

    /// The aggregate admission result.
    pub fn aggregate(&self) -> &ProtocolEmissionAggregate {
        &self.aggregate
    }

    /// The package, present only when every protocol result is emitted.
    pub fn emission(&self) -> Option<&Emission> {
        self.emission.as_ref()
    }

    /// One primary diagnostic for each refused protocol, in declaration
    /// order (FR-383 diagnostics rule).
    pub fn diagnostics(&self) -> impl Iterator<Item = &qsl_semantics::check::CheckRefusal> + '_ {
        self.results.iter().filter_map(|result| match result {
            ProtocolEmissionResult::Emitted { .. } => None,
            ProtocolEmissionResult::Refused { refusal, .. } => Some(refusal),
            ProtocolEmissionResult::Incomplete { .. } => None,
        })
    }

    /// The unit exit code under FR-285's multi-item severity ordering.
    pub fn exit_code(&self) -> u8 {
        Category::most_severe(self.results.iter().map(|result| match result {
            ProtocolEmissionResult::Emitted { .. } => Category::Success.exit_code(),
            ProtocolEmissionResult::Refused { refusal, .. } => {
                refusal.cause.code().category().exit_code()
            }
            ProtocolEmissionResult::Incomplete { .. } => Category::Incomplete.exit_code(),
        }))
        .unwrap_or_else(|| Category::Success.exit_code())
    }
}

/// Debug writes the source map's node keys into the enclosing table, so a
/// package that retains its emission stays linear.
impl DebugWith for Emission {
    fn fmt_with(&self, f: &mut std::fmt::Formatter<'_>, table: &DebugTable) -> std::fmt::Result {
        let Self {
            package,
            omitted,
            evidence,
            capability_report,
            source_map,
            selected_operation_member_rows,
            selected_tuple_declaration_rows,
            selected_source_unit_rows,
            selected_model_enum_declaration_rows,
            selected_model_enum_member_rows,
            selected_enum_declaration_rows,
            selected_enum_member_rows,
            selected_source_union_rows,
            selected_source_record_rows,
            selected_source_tuple_rows,
            selected_model_enum_shape_rows,
            selected_source_collection_rows,
            selected_exact_declaration_rows,
            selected_exact_field_rows,
            identity_row_selection: _,
            equality_producer_identity_rows,
            equality_fact_witness_rows,
            wire_nodes,
            temporal_binding_rows,
            temporal_rows,
        } = self;
        f.debug_struct("Emission")
            .field("package", package)
            .field("omitted", omitted)
            .field("evidence", evidence)
            .field("capability_report", capability_report)
            .field("source_map", &With(source_map, table))
            .field(
                "selected_operation_member_rows",
                selected_operation_member_rows,
            )
            .field(
                "selected_tuple_declaration_rows",
                selected_tuple_declaration_rows,
            )
            .field("selected_source_unit_rows", selected_source_unit_rows)
            .field(
                "selected_model_enum_declaration_rows",
                selected_model_enum_declaration_rows,
            )
            .field(
                "selected_model_enum_member_rows",
                selected_model_enum_member_rows,
            )
            .field(
                "selected_enum_declaration_rows",
                selected_enum_declaration_rows,
            )
            .field("selected_enum_member_rows", selected_enum_member_rows)
            .field("selected_source_union_rows", selected_source_union_rows)
            .field("selected_source_record_rows", selected_source_record_rows)
            .field("selected_source_tuple_rows", selected_source_tuple_rows)
            .field(
                "selected_model_enum_shape_rows",
                selected_model_enum_shape_rows,
            )
            .field(
                "selected_source_collection_rows",
                selected_source_collection_rows,
            )
            .field(
                "selected_exact_declaration_rows",
                selected_exact_declaration_rows,
            )
            .field("selected_exact_field_rows", selected_exact_field_rows)
            .field(
                "equality_producer_identity_rows",
                equality_producer_identity_rows,
            )
            .field("equality_fact_witness_rows", equality_fact_witness_rows)
            .field("wire_node_count", &wire_nodes.len())
            .field("temporal_binding_row_count", &temporal_binding_rows.len())
            .field("temporal_rows", temporal_rows)
            .finish()
    }
}

impl std::fmt::Debug for Emission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        scoped(f, self)
    }
}

/// A producer-side equality fact paired with its exact structural key and
/// final checked id. These rows do not claim that the IR reader admits them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EqualityFactWitnessRow {
    application: NodeKey,
    checked_application: CheckedNodeId,
    occurrence: CheckedOccurrence,
    source_map_entry: CheckedSourceMapEntry,
    structural_key: NodeKey,
    structural_node: CheckedNodeId,
    owner: Option<CheckedNodeOwner>,
    value_type: Option<quire_exact::ValueType>,
    fact: EqualityFactWitness,
    // Derived eligibility in this final emission, not an admission claim. It
    // avoids building a temporary identity index in the budgeted row adapter.
    selected_for_emission: bool,
}

/// The exact producer fact represented by an equality witness row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EqualityFactWitness {
    /// A quantity operand's source or conversion target type.
    OperandType {
        position: usize,
        role: EqualityOperandTypeRole,
    },
    /// A resolved unit paired to its quantity type and role.
    Unit {
        unit_id: quire_exact::UnitId,
        quantity_type: CheckedNodeId,
        operand_position: usize,
        role: EqualityUnitRole,
    },
    /// A dimension declaration's role and stable position within its unit.
    DimensionDeclaration {
        unit_id: quire_exact::UnitId,
        operand_position: usize,
        role: qsl_semantics::check::EqualityDimensionRole,
        ordinal: usize,
    },
    /// An ordered direct dimension term, retaining both structural endpoints.
    DimensionTerm {
        unit_id: quire_exact::UnitId,
        operand_position: usize,
        ordinal: usize,
        base: NodeKey,
        base_node: CheckedNodeId,
        exponent: Box<str>,
    },
    /// An ordered conversion edge, retaining source and target structural IDs.
    ConversionEdge {
        unit_id: quire_exact::UnitId,
        operand_position: usize,
        ordinal: usize,
        target: NodeKey,
        target_node: CheckedNodeId,
        scale: CheckedRational,
        offset: CheckedRational,
    },
    /// A selected Text/Plan operand type and its comparison role.
    TextPlanType {
        position: usize,
        role: EqualityOperandTypeRole,
    },
    /// One selected enum member, joined to its exact source and lowered
    /// declarations and the member's checked-package node.
    EnumMember {
        source_declaration: NodeKey,
        source_declaration_node: CheckedNodeId,
        declaration: NodeKey,
        declaration_node: CheckedNodeId,
        member: NodeKey,
        member_node: CheckedNodeId,
        case: Box<str>,
        ordinal: usize,
        ordered: bool,
    },
}

/// The operand role attached to a retained type identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EqualityOperandTypeRole {
    Source,
    ConversionTarget,
}

/// Why a resolved UnitId is present in this equality's unit table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EqualityUnitRole {
    Source,
    ConversionTarget,
}

/// One equality's QSL producer identities joined to its emitted checked node
/// and exact occurrence. This is in-memory producer data, not wire data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EqualityProducerEmissionRow {
    application: NodeKey,
    checked_node: CheckedNodeId,
    occurrence: CheckedOccurrence,
    location: quire_exact::Location,
    source_map_entry: CheckedSourceMapEntry,
    producer_identity: qsl_semantics::check::EqualityProducerIdentity,
    producer_data: qsl_semantics::check::EqualityProducerData,
}

impl EqualityProducerEmissionRow {
    /// The exact QSL application key retained during lowering.
    pub fn application(&self) -> &NodeKey {
        &self.application
    }

    /// The corresponding checked-package node id produced by the emitter.
    pub fn checked_node(&self) -> &CheckedNodeId {
        &self.checked_node
    }

    /// The emitted checked-package occurrence.
    pub fn occurrence(&self) -> &CheckedOccurrence {
        &self.occurrence
    }

    /// The exact checked structural node occurrence from the source graph.
    pub fn location(&self) -> &quire_exact::Location {
        &self.location
    }

    /// The selected source-map context for this exact emitted occurrence.
    pub fn source_map_entry(&self) -> &CheckedSourceMapEntry {
        &self.source_map_entry
    }

    /// The QSL-owned typed producer identities for this occurrence.
    pub fn producer_identity(&self) -> &qsl_semantics::check::EqualityProducerIdentity {
        &self.producer_identity
    }

    /// The checker's exact equality schedule and typed producer values.
    pub fn producer_data(&self) -> &qsl_semantics::check::EqualityProducerData {
        &self.producer_data
    }
}

impl Emission {
    /// The v2 bytes and their manifest package name.
    pub fn package(&self) -> &EmittedPackage {
        &self.package
    }

    /// The `temporal_bindings` rows of the checked graph's clauses, keyed by
    /// clause identity (FR-522). An emission re-encoded from a read package
    /// (timed models, a monitor plan) keys none: the read package's rows are
    /// keyed by wire identity only.
    pub fn temporal_binding_rows(&self) -> &TemporalBindingRows {
        &self.temporal_binding_rows
    }

    /// The emission with `bytes` as its package: how a test presents a
    /// package no emitter produces (FR-515-AC-20, FR-516-AC-32), such as twins
    /// whose occurrence ordinals the emitter would not assign. Every other
    /// member is the emission's own.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn with_package_bytes(mut self, bytes: Vec<u8>) -> Self {
        let name = self.package.name().to_owned();
        self.package =
            match EmittedPackage::new(&name, |_| Ok::<_, std::convert::Infallible>(bytes)) {
                Ok(package) => package,
                Err(never) => match never {},
            };
        self
    }

    /// The emission with `clauses` as its clause references: how a test
    /// presents an emission whose references name a clause position the
    /// emitter did not write. Every other member is the emission's own.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn with_temporal_clause_refs(mut self, clauses: Vec<TemporalClauseRef>) -> Self {
        self.temporal_rows.clauses = clauses;
        self
    }

    /// The emission with `profile_selections` as its profile selection rows,
    /// as [`Self::with_temporal_clause_refs`] presents other references.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn with_temporal_profile_selections(
        mut self,
        profile_selections: Vec<quire_contract_model::CheckedSelection>,
    ) -> Self {
        self.temporal_rows.profile_selections = profile_selections;
        self
    }

    /// The emission with `bindings` as its `temporal_bindings` rows, as
    /// [`Self::with_temporal_clause_refs`] presents other references.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn with_temporal_bindings(
        mut self,
        bindings: Vec<quire_contract_model::TemporalBinding>,
    ) -> Self {
        self.temporal_rows.bindings = bindings;
        self
    }

    /// The temporal clause references, `profile_selections` and
    /// `temporal_bindings` rows as written into [`Self::package`] (FR-337).
    pub fn temporal_rows(&self) -> &TemporalRows {
        &self.temporal_rows
    }

    /// Every node of the checked graph the wire omits, ascending by node id.
    pub fn omitted(&self) -> &[OmittedNode] {
        &self.omitted
    }

    /// The evidence IR's reader needs to read these bytes: the artifacts the
    /// emission compiled against and its required features. A caller that
    /// reads the bytes itself adds the domain package documents.
    pub fn evidence(&self) -> &CheckedPackageEvidence {
        &self.evidence
    }

    /// QSL-stage rows for model-owned operation anchors in this package.
    pub fn selected_operation_member_rows(
        &self,
    ) -> &[quire_contract_model::SelectedOperationMemberRow] {
        &self.selected_operation_member_rows
    }

    /// Selected model tuple identity rows carried by this emission.
    pub fn selected_tuple_declaration_rows(
        &self,
    ) -> &[quire_contract_model::SelectedTupleDeclarationRow] {
        &self.selected_tuple_declaration_rows
    }

    /// Source-owned declared-unit identity rows carried by this emission.
    pub fn selected_source_unit_rows(&self) -> &[quire_contract_model::SelectedSourceUnitRow] {
        &self.selected_source_unit_rows
    }

    /// Selected model enum declaration rows carried by this emission.
    pub fn selected_model_enum_declaration_rows(
        &self,
    ) -> &[quire_contract_model::SelectedModelEnumDeclarationRow] {
        &self.selected_model_enum_declaration_rows
    }

    /// Selected model enum member rows carried by this emission.
    pub fn selected_model_enum_member_rows(
        &self,
    ) -> &[quire_contract_model::SelectedModelEnumMemberRow] {
        &self.selected_model_enum_member_rows
    }

    /// Source-owned enum declaration rows carried by this emission.
    pub fn selected_enum_declaration_rows(
        &self,
    ) -> &[quire_contract_model::SelectedEnumDeclarationRow] {
        &self.selected_enum_declaration_rows
    }

    /// Source-owned enum member rows carried by this emission.
    pub fn selected_enum_member_rows(&self) -> &[quire_contract_model::SelectedEnumMemberRow] {
        &self.selected_enum_member_rows
    }

    /// Source-owned union rows carried by this emission: each names the
    /// emitted union node, its sealed members and their ordered payload nodes.
    pub fn selected_source_union_rows(&self) -> &[quire_contract_model::SelectedSourceUnionRow] {
        &self.selected_source_union_rows
    }

    /// Source-owned record rows carried by this emission, ascending by the
    /// record nodes' keys.
    pub fn selected_source_record_rows(&self) -> &[SelectedSourceRecordRow] {
        &self.selected_source_record_rows
    }

    /// Source-owned tuple rows carried by this emission, ascending by the
    /// tuple nodes' keys: each names the emitted tuple node and its positions
    /// in declaration order with their final type nodes and checked types.
    pub fn selected_source_tuple_rows(&self) -> &[quire_contract_model::SelectedSourceTupleRow] {
        &self.selected_source_tuple_rows
    }

    /// The model-owned enum shape rows (FR-092 "Model enum shape rows"), in
    /// ascending declaration-key order.
    pub fn selected_model_enum_shape_rows(&self) -> &[SelectedModelEnumShapeRow] {
        &self.selected_model_enum_shape_rows
    }

    /// Collection type rows carried by this emission, ascending by the
    /// collection nodes' keys.
    pub fn selected_source_collection_rows(&self) -> &[SelectedSourceCollectionRow] {
        &self.selected_source_collection_rows
    }

    /// Selected model declaration rows (record, object, declared unit) of the
    /// acquisition these bytes were emitted from.
    pub fn selected_exact_declaration_rows(
        &self,
    ) -> &[quire_contract_model::SelectedExactDeclarationRow] {
        &self.selected_exact_declaration_rows
    }

    /// Selected record-field rows of the acquisition these bytes were emitted
    /// from.
    pub fn selected_exact_field_rows(&self) -> &[quire_contract_model::SelectedExactFieldRow] {
        &self.selected_exact_field_rows
    }

    /// The required identity handoff facts from this final emission. This
    /// preserves original opaque source keys separately from graph references.
    ///
    /// # Errors
    ///
    /// The native typed stage-row refusal when declaration or field claims do
    /// not join the admitted package, or an inline
    /// [`crate::IdentityFileRefusal::Rows`] denial before an unpaid producer copy.
    /// The same caller budget pays the producer inputs, one native join and
    /// the returned owned rows without restoring prior consumption.
    pub fn identity_rows(
        &self,
        package: &CheckedPackageV2,
        budget: &mut quire_contract_model::NativeJoinBudget,
    ) -> Result<crate::ReadIdentityRows, crate::IdentityFileRefusal> {
        self.materialize_identity_rows(package, budget)
    }

    /// QSL-owned identity rows keyed by equality application and occurrence.
    pub fn equality_producer_identity_rows(&self) -> &[EqualityProducerEmissionRow] {
        &self.equality_producer_identity_rows
    }

    /// Exact producer facts joined to final structural IDs for equality.
    pub fn equality_fact_witness_rows(&self) -> &[EqualityFactWitnessRow] {
        &self.equality_fact_witness_rows
    }

    /// Validates the selected model enum rows against this admitted package.
    pub fn selected_model_enums<'a>(
        &'a self,
        package: &'a quire_contract_model::CheckedPackageV2,
        work_limit: u64,
    ) -> Result<
        quire_contract_model::SelectedModelEnums<'a>,
        quire_contract_model::SelectedModelEnumError,
    > {
        quire_contract_model::SelectedModelEnums::from_stage_rows(
            package,
            self.selected_model_enum_declaration_rows.clone(),
            self.selected_model_enum_member_rows.clone(),
            work_limit,
        )
    }

    /// Validates the operation-member rows against this emission's admitted IR
    /// package, returning a typed refusal for any absent or mismatched row.
    pub fn selected_operation_members<'a>(
        &'a self,
        package: &'a quire_contract_model::CheckedPackageV2,
        work_limit: u64,
    ) -> Result<
        quire_contract_model::SelectedOperationMembers<'a>,
        quire_contract_model::SelectedOperationMemberError,
    > {
        quire_contract_model::SelectedOperationMembers::from_stage_rows(
            package,
            self.selected_operation_member_rows.clone(),
            work_limit,
        )
    }

    /// The FR-322 feature-level capability report, in the emitted order.
    pub fn capability_report(&self) -> &[CheckedCapability] {
        &self.capability_report
    }

    /// The source map emitted with these package bytes.
    pub fn source_map(&self) -> &PackageSourceMap {
        &self.source_map
    }
}

impl EqualityFactWitnessRow {
    /// The exact checked application key retained during lowering.
    pub fn application(&self) -> &NodeKey {
        &self.application
    }

    /// The emitted checked id of the equality application.
    pub fn checked_application(&self) -> &CheckedNodeId {
        &self.checked_application
    }

    /// The exact occurrence of the equality application.
    pub fn occurrence(&self) -> &CheckedOccurrence {
        &self.occurrence
    }

    /// The selected source context for the equality occurrence.
    pub fn source_map_entry(&self) -> &CheckedSourceMapEntry {
        &self.source_map_entry
    }

    /// The structural key of the fact node.
    pub fn structural_key(&self) -> &NodeKey {
        &self.structural_key
    }

    /// The final checked id of the exact fact node.
    pub fn structural_node(&self) -> &CheckedNodeId {
        &self.structural_node
    }

    /// The emitted source/model owner context of the fact node, if present.
    pub fn owner(&self) -> Option<&CheckedNodeOwner> {
        self.owner.as_ref()
    }

    /// The checker's exact static type for this producer row when it names a
    /// typed operand or comparison type.
    pub fn value_type(&self) -> Option<&quire_exact::ValueType> {
        self.value_type.as_ref()
    }

    /// The exact producer fact retained for this row.
    pub fn fact(&self) -> &EqualityFactWitness {
        &self.fact
    }
}

/// IR's closed node family for `tag`. Exhaustive: a new QSL node tag does
/// not compile until it has a v2 arm (ADR-013 C-03).
fn ir_tag(tag: NodeTag) -> CheckedNodeTag {
    match tag {
        NodeTag::ScalarType => CheckedNodeTag::ScalarType,
        NodeTag::CompositeType => CheckedNodeTag::CompositeType,
        NodeTag::BoundedDomain => CheckedNodeTag::BoundedDomain,
        NodeTag::Value => CheckedNodeTag::Value,
        NodeTag::Expression => CheckedNodeTag::Expression,
        NodeTag::Function => CheckedNodeTag::Function,
        NodeTag::Model => CheckedNodeTag::Model,
        NodeTag::Relation => CheckedNodeTag::Relation,
        NodeTag::State => CheckedNodeTag::State,
        NodeTag::Temporal => CheckedNodeTag::Temporal,
        NodeTag::Protocol => CheckedNodeTag::Protocol,
        NodeTag::Claim => CheckedNodeTag::Claim,
        NodeTag::Correspondence => CheckedNodeTag::Correspondence,
        NodeTag::Hybrid => CheckedNodeTag::Hybrid,
        NodeTag::TimedModel => CheckedNodeTag::TimedModel,
        NodeTag::Extension => CheckedNodeTag::Extension,
    }
}

/// The structural V2 identity of a checked key (FR-322 "Operand index
/// segments"): each operand is the operand node's own id, shared where it is
/// named twice. The package writer holds each operand as its index segment.
pub(crate) fn node_id(
    ids: &StructuralIds,
    key: impl std::borrow::Borrow<NodeKey>,
) -> CheckedNodeId {
    ids.id(key.borrow())
}

fn admitted_id(ids: &StructuralIds, reference: &NodeRef) -> Result<CheckedNodeId, EmitRefusal> {
    admitted(reference).map(|key| node_id(ids, key))
}

/// The admitted kernel key `reference` names.
fn admitted(reference: &NodeRef) -> Result<NodeKey, EmitRefusal> {
    reference
        .admitted()
        .cloned()
        .ok_or(EmitRefusal::UnresolvedNodeReference)
}

/// The structural kernel key of an id this emitter built, which holds no
/// index segment. An id read from a package resolves against that package's
/// graph instead ([`qsl_semantics::check::wire_id::WireGraph::key`]).
pub(crate) fn admitted_key(id: &CheckedNodeId) -> Result<NodeKey, EmitRefusal> {
    qsl_semantics::check::wire_id::node_key(id).map_err(|_| EmitRefusal::SourceMapInvariant)
}

/// The Contract IR occurrence `origin` names, as [`emit_checked`] writes it
/// on a node. `None` for a role FR-322 does not name.
pub fn checked_occurrence(origin: &Origin) -> Option<CheckedOccurrence> {
    Some(CheckedOccurrence {
        role: crate::checked_v2::occurrence_role(origin.role().as_str())?,
        ordinal: origin.ordinal(),
    })
}

/// What one node body names: the FR-093 rule 1 and 2 dependencies, the
/// type annotations (literal `type`s and application `result_type`s), and
/// the law definitions of its applications.
#[derive(Default)]
struct BodyNames {
    dependencies: HashSet<NodeKey>,
    annotations: HashSet<NodeKey>,
    laws: Vec<OperationLaw>,
}

impl BodyNames {
    /// Read `body`: its stratified terms have a fixed depth (ADR-030 D-2),
    /// so this is a fixed-depth match over its leaves.
    fn of(body: &BodyTerm) -> Result<Self, EmitRefusal> {
        let mut names = Self::default();
        match body {
            BodyTerm::Application(application) => {
                let operation = &application.operation;
                names
                    .annotations
                    .insert(admitted(&application.result_type)?);
                if let Some(declaration) = operation.member().and_then(Member::declaration) {
                    names.dependencies.insert(declaration.clone());
                }
                // FR-380: the population node a reachability links to.
                if let Some(population) = operation.member().and_then(Member::population) {
                    names.dependencies.insert(population.clone());
                }
                let leaf_laws = operation.leaves().iter().flat_map(|leaf| &leaf.laws);
                names
                    .laws
                    .extend(operation.laws().iter().chain(leaf_laws).cloned());
            }
            // QSpec FR-340: every `modifies` declaration and every
            // `creates`/`deletes` entry is a declared dependency of the
            // frame node, so a reader can join each entry to the node it
            // names among the frame's own `dependencies`.
            BodyTerm::Frame(frame) => {
                for entry in &frame.modifies {
                    names.dependencies.insert(admitted(&entry.declaration())?);
                }
                for node in frame.creates.iter().chain(&frame.deletes) {
                    names.dependencies.insert(admitted(node)?);
                }
            }
            // QSpec FR-451: `dependencies` are exactly the unique set of every
            // `type`, `population` and `context` target of the body.
            BodyTerm::Abstraction(term) => {
                names
                    .dependencies
                    .extend(term.targets().map_err(encoding)?.iter().cloned());
            }
            // QSpec FR-469, FR-490, FR-491: `dependencies` are exactly the
            // unique targets of the form's references.
            BodyTerm::Extension(term) => {
                names
                    .dependencies
                    .extend(term.targets().map_err(encoding)?.iter().cloned());
            }
            BodyTerm::ImplementationGuarantee(term) => {
                names
                    .dependencies
                    .extend(term.targets().map_err(encoding)?.iter().cloned());
            }
            BodyTerm::Composition(term) => {
                names
                    .dependencies
                    .extend(term.targets().map_err(encoding)?.iter().cloned());
            }
            BodyTerm::Leaf(_) | BodyTerm::Aggregate(_) => {}
        }
        for leaf in body.leaves() {
            match leaf {
                LeafTerm::Literal { ty, .. } => {
                    names.annotations.insert(admitted(ty)?);
                }
                LeafTerm::Reference { target } => {
                    names.dependencies.insert(admitted(target)?);
                }
                // FR-322-AC-37: a `dependency_reference` is never one of the
                // node's `dependencies`, and names no node of this graph.
                LeafTerm::DependencyReference { .. } => {}
            }
        }
        Ok(names)
    }
}

/// A node's recorded occurrences, each with the location `check` recorded
/// it at.
type Recorded<'g> = Vec<(CheckedOccurrence, RecordedLocation<'g>)>;

/// Parameters retain their own source regions; expression sites use the
/// expression-location converter. An unlocated parameter never takes its
/// enclosing body's region.
enum RecordedLocation<'g> {
    Expression(&'g Location),
    Parameter(Option<SourceRegion>),
}

/// One checked node, read for the wire.
struct Candidate<'g> {
    node: &'g SemanticNode,
    key: NodeKey,
    id: CheckedNodeId,
    tag: CheckedNodeTag,
    semantic_type: CheckedNodeId,
    /// FR-093 rules 1 to 3, ascending by structural node identity.
    dependencies: Vec<CheckedNodeId>,
    /// Every node the written node names: its dependencies, its semantic
    /// type and its body's type annotations. Held by hash: two keys that
    /// share a deep operand chain cost its depth to order, never to hash.
    names: HashSet<NodeKey>,
    laws: Vec<OperationLaw>,
    /// The occurrences `check` recorded for the node, each with the
    /// location it was recorded at.
    occurrences: Recorded<'g>,
}

impl<'g> Candidate<'g> {
    fn of(
        ids: &StructuralIds,
        foreign: &ForeignNodes,
        node: &'g SemanticNode,
        occurrences: Recorded<'g>,
    ) -> Result<Self, EmitRefusal> {
        let key = node.key();
        let id = node_id(ids, &key);
        // A self-typed node names itself as its semantic type (FR-322).
        let type_key = node.semantic_type().unwrap_or_else(|| key.clone());
        let semantic_type = node_id(ids, &type_key);
        let BodyNames {
            mut dependencies,
            annotations,
            laws,
        } = BodyNames::of(node.body())?;
        // QSpec FR-464 "owner_model": a protocol clause depends on its exact
        // owner model, besides the nodes its body reaches.
        dependencies.extend(node.owner_model());
        // Rule 3: a `bounded_domain` node depends on the type it bounds.
        if node.node_tag() == NodeTag::BoundedDomain {
            dependencies.insert(type_key.clone());
        }
        // QSpec FR-322-AC-384: a `bounded_quantity` depends on its unit and on
        // the rational type of its exact endpoints.
        if CheckedNodeKind::decode(ir_tag(node.node_tag()), node.semantic_form())
            == Some(CheckedNodeKind::ScalarType(ScalarTypeForm::BoundedQuantity))
        {
            dependencies.extend(annotations.iter().cloned());
        }
        if CheckedNodeKind::decode(ir_tag(node.node_tag()), node.semantic_form())
            == Some(CheckedNodeKind::Value(ValueForm::EnumValue))
        {
            dependencies.insert(type_key.clone());
        }
        if let Some(nominal) = node.nominal_payload() {
            match nominal {
                // An `enum_value` depends on its enumeration declaration.
                NominalPayload::Enum(_) => {}
                NominalPayload::Dimension(dimension) => {
                    dependencies.extend(dimension.terms().iter().map(|(base, _)| base.clone()));
                }
                NominalPayload::Unit(unit) => {
                    dependencies.insert(unit.dimension().clone());
                    dependencies.extend(unit.target().cloned());
                }
                NominalPayload::DimensionEquivalence(equivalence) => {
                    dependencies.extend(equivalence.dimensions().iter().cloned());
                }
            }
        }
        let mut names = dependencies.clone();
        names.extend(annotations);
        names.insert(type_key);
        names.remove(&key);
        // A dependency's node is written as the dependency position of its
        // package, never as a node of this graph.
        names.retain(|name| !foreign.holds(name));
        // A node has a few dependencies, so sorting them by structural id is
        // cheap; they are rarely deep chains of each other.
        let mut dependencies: Vec<CheckedNodeId> =
            dependencies.iter().map(|key| node_id(ids, key)).collect();
        dependencies.sort_unstable();
        Ok(Self {
            node,
            tag: ir_tag(node.node_tag()),
            key,
            id,
            semantic_type,
            dependencies,
            names,
            laws,
            occurrences,
        })
    }

    /// Whether `decodes` admits the node's (`node_tag`, `semantic_form`).
    fn form_is_supported(&self, decodes: &dyn Fn(CheckedNodeTag, &str) -> bool) -> bool {
        decodes(self.tag, self.node.semantic_form())
    }

    /// FR-322's declaration rule, as IR applies it: `declaration` is absent
    /// on an `expression`, `relation`, `state`, `temporal` or
    /// `correspondence` node and on a `value`/`enum_value` node, and on every
    /// other node present exactly when it has a `declaration` occurrence.
    fn declaration_matches_occurrences(&self) -> bool {
        // The `EnumValue` arm is not distinguished by any fixture today:
        // occurrence-recording never attaches a `Declaration`-role
        // occurrence to an enum-value node under current lowering, so
        // `declared` already comes out `false` for one without this arm
        // (SR-758 FND-006 mutation-tested this and found no fixture that
        // depends on it). It is kept because FR-322 states the rule as a
        // property of the node kind, not as a derived fact about today's
        // occurrence-recording: if recording ever starts attaching a
        // `Declaration` occurrence to an enum-value node (for example while
        // building out a future enum-value declaration form), this arm is
        // what keeps that node refused instead of silently accepted.
        let forced_absent = matches!(
            self.tag,
            CheckedNodeTag::Expression
                | CheckedNodeTag::Relation
                | CheckedNodeTag::State
                | CheckedNodeTag::Temporal
                | CheckedNodeTag::Correspondence
        ) || matches!(
            CheckedNodeKind::decode(self.tag, self.node.semantic_form()),
            Some(CheckedNodeKind::Value(ValueForm::EnumValue))
        );
        let declared = !forced_absent
            && self
                .occurrences
                .iter()
                .any(|(occurrence, _)| occurrence.role == CheckedOccurrenceRole::Declaration);
        declared == self.node.declaration().is_some()
    }

    /// The node as FR-322 writes it.
    fn wire_node(&self, ids: &StructuralIds) -> Result<CheckedSemanticNodeV2, EmitRefusal> {
        let node = self.node;
        Ok(CheckedSemanticNodeV2 {
            node_id: self.id.clone(),
            schema_version: GRAPH_V2.into(),
            node_tag: self.tag.as_wire().into(),
            semantic_form: node.semantic_form().into(),
            semantic_type: self.semantic_type.clone(),
            dependencies: self.dependencies.clone(),
            occurrences: self
                .occurrences
                .iter()
                .map(|(occurrence, _)| occurrence.clone())
                .collect(),
            recursion_group: node
                .recursion()
                .map(|group| {
                    let value = qsl_foundation::node_wire::node_key_to_json(group.anchor())
                        .map_err(encoding)?;
                    let label = serde_json::to_string(&value);
                    qsl_foundation::node_wire::discard_node_json(value);
                    Ok::<Box<str>, EmitRefusal>(label.map_err(encoding)?.into())
                })
                .transpose()?,
            declaration: node.declaration().map(|segments| CheckedDeclaration {
                qualified_name: segments
                    .iter()
                    .map(|segment| segment.as_str().into())
                    .collect(),
            }),
            owner: node.owner().map(wire_owner),
            owner_model: node.owner_model().map(|key| node_id(ids, key)),
            body: checked_body(ids, node, self.tag)?,
        })
    }
}

/// Convert the checked body grammar to IR's typed term vocabulary.  Every
/// reference is required to have an admitted structural identity here.
fn checked_body(
    ids: &StructuralIds,
    node: &SemanticNode,
    tag: CheckedNodeTag,
) -> Result<CheckedTerm, EmitRefusal> {
    // A model-owned nominal node's values are read from the selected
    // document: its body is the empty aggregate (FR-322 "Model-owned
    // members"). Other nominal nodes are emitted from their payload.
    if let (Some(nominal), false) = (
        node.nominal_payload(),
        matches!(node.owner(), Some(Owner::Model(_))),
    ) {
        return nominal_term(ids, &node.key(), nominal);
    }
    let BodyTerm::Aggregate(aggregate) = node.body() else {
        return checked_term(ids, node.body());
    };
    if CheckedNodeKind::decode(tag, node.semantic_form())
        == Some(CheckedNodeKind::State(StateForm::OperationAnchor))
        && matches!(aggregate.members.last(), Some(MemberTerm::Binding(binding)) if
            binding.name == "execution_point")
    {
        let [context, operation, frame, point] = aggregate.members.as_slice() else {
            return Err(encoding(
                "point-bearing operation-anchor body does not have four members",
            ));
        };
        let MemberTerm::Binding(point) = point else {
            return Err(encoding(
                "operation-anchor execution point is not a binding",
            ));
        };
        let BindingValue::ExecutionPoint(point) = &point.value else {
            return Err(encoding(
                "operation-anchor execution point has no typed point",
            ));
        };
        return Ok(CheckedTerm::OperationAnchorBody(Box::new(
            CheckedOperationAnchorBody {
                members: [
                    member_term(ids, context)?,
                    member_term(ids, operation)?,
                    member_term(ids, frame)?,
                ],
                execution_point: Some(point.point().clone()),
            },
        )));
    }
    checked_term(ids, node.body())
}

fn checked_rational(value: &quire_exact::Rational) -> CheckedRational {
    CheckedRational {
        numerator: value.numerator().to_string().into(),
        denominator: value.denominator().to_string().into(),
    }
}

/// Convert admitted nominal facts without reading an identity back out of a
/// structural key. Enum ordering and conversion factors are payload facts.
fn nominal_term(
    ids: &StructuralIds,
    node: &NodeKey,
    payload: &NominalPayload,
) -> Result<CheckedTerm, EmitRefusal> {
    let body = match payload {
        NominalPayload::Enum(enumeration) => NominalBody::try_enum(
            enumeration.is_ordered(),
            enumeration
                .members()
                .iter()
                .map(|member| member.as_str().into())
                .collect(),
        ),
        NominalPayload::Dimension(dimension) => {
            let terms: Vec<DimensionTerm> = dimension
                .terms()
                .iter()
                .map(|(base, exponent)| DimensionTerm {
                    dimension_node_id: node_id(ids, base),
                    exponent: exponent.to_string().into(),
                })
                .collect();
            NominalBody::try_dimension(dimension.is_base(), terms)
        }
        NominalPayload::Unit(unit) => NominalBody::try_unit(
            node_id(ids, unit.dimension()),
            unit.target().map(|key| node_id(ids, key)),
            checked_rational(unit.factor()),
            checked_rational(unit.offset()),
        ),
        NominalPayload::DimensionEquivalence(equivalence) => {
            // A Local operand is a node of the importing package's own graph
            // (QSpec FR-322-AC-299). A dimension of another package is a
            // Package operand, which IR checks against a direct dependency
            // selection; the emitter does not establish that selection here
            // (QSL-1177 owns it), so it refuses rather than mint a foreign
            // package into a Local id.
            for operand in equivalence.dimensions() {
                if operand.package() != node.package() {
                    return Err(EmitRefusal::ForeignDimensionOperand {
                        node: Box::new(node_id(ids, node)),
                        operand: Box::new(node_id(ids, operand)),
                    });
                }
            }
            NominalBody::try_dimension_equivalence(
                [
                    CheckedDimensionOperand::Local(node_id(ids, &equivalence.dimensions()[0])),
                    CheckedDimensionOperand::Local(node_id(ids, &equivalence.dimensions()[1])),
                ],
                checked_rational(equivalence.scale()),
            )
        }
    };
    Ok(CheckedTerm::NominalBody(Box::new(body.map_err(encoding)?)))
}

fn checked_term(ids: &StructuralIds, term: &BodyTerm) -> Result<CheckedTerm, EmitRefusal> {
    match term {
        BodyTerm::Leaf(leaf) => leaf_term(ids, leaf),
        BodyTerm::Aggregate(aggregate) => Ok(CheckedTerm::Aggregate {
            members: aggregate
                .members
                .iter()
                .map(|value| member_term(ids, value))
                .collect::<Result<_, _>>()?,
        }),
        BodyTerm::Application(application) => Ok(CheckedTerm::Application {
            operator: application_operator(application.operator),
            operation: operation_term(ids, &application.operation)?,
            result_type: admitted_id(ids, &application.result_type)?,
            arguments: application
                .arguments
                .iter()
                .map(|value| member_term(ids, value))
                .collect::<Result<_, _>>()?,
        }),
        BodyTerm::Frame(frame) => frame_term(ids, frame),
        BodyTerm::Abstraction(abstraction) => abstraction_term(ids, abstraction),
        BodyTerm::Extension(extension) => extension_term(ids, extension),
        BodyTerm::ImplementationGuarantee(guarantee) => guarantee_term(ids, guarantee),
        BodyTerm::Composition(composition) => composition_term(ids, composition),
    }
}

/// QSpec FR-490: the `implementation_guarantee` term.
fn guarantee_term(
    ids: &StructuralIds,
    guarantee: &qsl_semantics::check::ImplementationGuaranteeTerm,
) -> Result<CheckedTerm, EmitRefusal> {
    let progress = guarantee
        .progress()
        .iter()
        .map(|entry| {
            Ok(CheckedGuaranteeProgress {
                context: admitted_id(ids, entry.context())?,
                operation: entry.operation().into(),
                measure: match entry.measure() {
                    MeasureEntry::Decreases(segments) => CheckedGuaranteeMeasure::Decreases(
                        segments
                            .iter()
                            .map(|segment| segment.as_str().into())
                            .collect(),
                    ),
                    MeasureEntry::Bound(count) => CheckedGuaranteeMeasure::Bound(*count),
                },
            })
        })
        .collect::<Result<_, EmitRefusal>>()?;
    Ok(CheckedTerm::ImplementationGuarantee {
        clause: admitted_id(ids, guarantee.clause())?,
        relation: admitted_id(ids, guarantee.relation())?,
        initial: guarantee
            .initial()
            .iter()
            .map(|segment| segment.as_str().into())
            .collect(),
        progress,
        environment: environment_terms(ids, guarantee.environment())?,
    })
}

/// QSpec FR-491: the `component_composition` term. A component's subject and
/// clauses are nodes of a dependency; [`write_index_segments`] names an
/// operand of theirs by its dependency position.
fn composition_term(
    ids: &StructuralIds,
    composition: &qsl_semantics::check::CompositionTerm,
) -> Result<CheckedTerm, EmitRefusal> {
    let clause = |entry: &qsl_semantics::check::ClauseEntry| {
        Ok(CheckedComponentClause {
            component: entry.component().into(),
            clause: admitted_id(ids, entry.clause())?,
        })
    };
    let premise = |entry: &qsl_semantics::check::PremiseEntry| {
        Ok(CheckedCompositionPremise {
            guarantee: clause(entry.guarantee())?,
            on: clause(entry.on())?,
        })
    };
    Ok(CheckedTerm::ComponentComposition {
        rule: composition.rule().as_str().into(),
        components: composition
            .components()
            .iter()
            .map(|entry| {
                Ok(CheckedCompositionComponent {
                    name: entry.name().into(),
                    package: entry.package().into(),
                    subject: admitted_id(ids, entry.subject())?,
                })
            })
            .collect::<Result<_, EmitRefusal>>()?,
        guarantees: composition
            .guarantees()
            .iter()
            .map(clause)
            .collect::<Result<_, EmitRefusal>>()?,
        relies: composition
            .relies()
            .iter()
            .map(premise)
            .collect::<Result<_, EmitRefusal>>()?,
        liveness_dependencies: composition
            .liveness_dependencies()
            .iter()
            .map(premise)
            .collect::<Result<_, EmitRefusal>>()?,
        supplies: composition
            .supplies()
            .iter()
            .map(|entry| {
                fairness_supply(
                    ids,
                    entry.kind(),
                    entry.granularity(),
                    entry.context(),
                    entry.operation(),
                )
            })
            .collect::<Result<_, EmitRefusal>>()?,
        environment: environment_terms(ids, composition.environment())?,
    })
}

/// The `environment` entries of a guarantee or composition body.
fn environment_terms(
    ids: &StructuralIds,
    entries: &[qsl_semantics::check::EnvironmentEntry],
) -> Result<Vec<CheckedEnvironmentEntry>, EmitRefusal> {
    entries
        .iter()
        .map(|entry| {
            Ok(CheckedEnvironmentEntry {
                name: entry.name().into(),
                supplier: entry.supplier().into(),
                supplies: match entry.supplies() {
                    SuppliesEntry::Fairness {
                        kind,
                        granularity,
                        context,
                        operation,
                    } => CheckedEnvironmentSupplies::Fairness(fairness_supply(
                        ids,
                        kind,
                        granularity,
                        context,
                        operation,
                    )?),
                    SuppliesEntry::Assumption(node) => {
                        CheckedEnvironmentSupplies::Assumption(admitted_id(ids, node)?)
                    }
                },
            })
        })
        .collect()
}

/// One scheduling constraint, its `kind` and `granularity` read from their
/// spellings.
#[string_edge]
fn fairness_supply(
    ids: &StructuralIds,
    kind: &str,
    granularity: &str,
    context: &NodeRef,
    operation: &str,
) -> Result<CheckedFairnessSupply, EmitRefusal> {
    Ok(CheckedFairnessSupply {
        kind: match kind {
            "weak" => CheckedFairnessKind::Weak,
            "strong" => CheckedFairnessKind::Strong,
            other => return Err(encoding(format!("fairness kind {other:?}"))),
        },
        granularity: match granularity {
            "whole" => CheckedFairnessGranularity::Whole,
            "each" => CheckedFairnessGranularity::Each,
            other => return Err(encoding(format!("fairness granularity {other:?}"))),
        },
        context: admitted_id(ids, context)?,
        operation: operation.into(),
    })
}

/// QSpec FR-469: `{term: "extension_form", definition, node_kind, clauses}`,
/// each clause a `binding` of the clause name to a `reference` to the node of
/// its checked value.
fn extension_term(
    ids: &StructuralIds,
    extension: &qsl_semantics::check::ExtensionTerm,
) -> Result<CheckedTerm, EmitRefusal> {
    Ok(CheckedTerm::ExtensionForm {
        definition: CheckedArtifactRef {
            authority: extension.definition().authority().into(),
            identity: extension.definition().identity().into(),
        },
        node_kind: extension.node_kind().into(),
        clauses: extension
            .clauses()
            .iter()
            .map(|clause| {
                Ok(CheckedTerm::Binding {
                    name: clause.name().into(),
                    value: Box::new(CheckedTerm::Reference {
                        target: admitted_id(ids, clause.value())?,
                    }),
                })
            })
            .collect::<Result<_, EmitRefusal>>()?,
    })
}

fn leaf_term(ids: &StructuralIds, leaf: &LeafTerm) -> Result<CheckedTerm, EmitRefusal> {
    Ok(match leaf {
        LeafTerm::Literal { ty, value } => {
            let (value_kind, value) = literal_value(value);
            CheckedTerm::Literal {
                type_id: admitted_id(ids, ty)?,
                value_kind,
                value,
            }
        }
        LeafTerm::Reference { target } => CheckedTerm::Reference {
            target: admitted_id(ids, target)?,
        },
        LeafTerm::DependencyReference { node } => {
            let key = node
                .admitted()
                .ok_or(EmitRefusal::UnresolvedNodeReference)?;
            CheckedTerm::DependencyReference {
                package: key.view().package().into(),
                node: node_id(ids, key),
            }
        }
    })
}

fn group_term(ids: &StructuralIds, group: &GroupTerm) -> Result<CheckedTerm, EmitRefusal> {
    Ok(CheckedTerm::Aggregate {
        members: group
            .members
            .iter()
            .map(|member| match member {
                GroupMember::Leaf(leaf) => leaf_term(ids, leaf),
                GroupMember::Binding(binding) => Ok(CheckedTerm::Binding {
                    name: binding.name.as_str().into(),
                    value: Box::new(leaf_term(ids, &binding.value)?),
                }),
            })
            .collect::<Result<_, _>>()?,
    })
}

fn tuple_term(ids: &StructuralIds, tuple: &TupleTerm) -> Result<CheckedTerm, EmitRefusal> {
    Ok(CheckedTerm::Aggregate {
        members: tuple
            .members
            .iter()
            .map(|member| match member {
                TupleMember::Leaf(leaf) => leaf_term(ids, leaf),
                TupleMember::Group(group) => group_term(ids, group),
            })
            .collect::<Result<_, _>>()?,
    })
}

fn member_term(ids: &StructuralIds, member: &MemberTerm) -> Result<CheckedTerm, EmitRefusal> {
    Ok(match member {
        MemberTerm::Leaf(leaf) => leaf_term(ids, leaf)?,
        MemberTerm::Group(group) => group_term(ids, group)?,
        MemberTerm::Binding(binding) => {
            let value = match &binding.value {
                BindingValue::Leaf(leaf) => leaf_term(ids, leaf)?,
                BindingValue::Group(group) => group_term(ids, group)?,
                BindingValue::Tuple(tuple) => tuple_term(ids, tuple)?,
                BindingValue::Aggregate(aggregate) => CheckedTerm::Aggregate {
                    members: aggregate
                        .members
                        .iter()
                        .map(|value| member_term(ids, value))
                        .collect::<Result<_, _>>()?,
                },
                BindingValue::ExecutionPoint(_) => {
                    return Err(encoding(
                        "execution point appears outside operation-anchor body",
                    ))
                }
            };
            CheckedTerm::Binding {
                name: binding.name.as_str().into(),
                value: Box::new(value),
            }
        }
    })
}

fn literal_value(value: &LiteralValue) -> (LiteralKind, serde_json::Value) {
    use serde_json::json;
    fn rational(value: &quire_exact::Rational) -> String {
        if value.is_integer() {
            value.numerator().to_string()
        } else {
            format!("{}/{}", value.numerator(), value.denominator())
        }
    }
    match value {
        LiteralValue::Boolean(value) => (LiteralKind::Boolean, json!(value)),
        LiteralValue::Integer(value) => (LiteralKind::Integer, json!(value.to_string())),
        LiteralValue::Count(value) => (LiteralKind::Integer, json!(value)),
        LiteralValue::Rational(value) => (LiteralKind::Rational, json!(rational(value))),
        LiteralValue::Decimal(value) => (LiteralKind::Decimal, json!(value)),
        LiteralValue::Float32Bits(value) => {
            (LiteralKind::Float32Bits, json!(format!("{value:08x}")))
        }
        LiteralValue::Float64Bits(value) => {
            (LiteralKind::Float64Bits, json!(format!("{value:016x}")))
        }
        LiteralValue::Quantity(value) => (LiteralKind::Quantity, json!(rational(value))),
        LiteralValue::Text(value) => (LiteralKind::Text, json!(value)),
        LiteralValue::String(value) => (LiteralKind::String, json!(value)),
        LiteralValue::Uuid(value) => (LiteralKind::Uuid, json!(value.to_string())),
        LiteralValue::Timestamp(value) => (LiteralKind::Timestamp, json!(value.to_string())),
        LiteralValue::None => (LiteralKind::None, serde_json::Value::Null),
        LiteralValue::Enum(value) => (LiteralKind::Enum, json!(value)),
    }
}

fn application_operator(operator: Operator) -> ApplicationOperator {
    match operator {
        Operator::Call => ApplicationOperator::Call,
        Operator::Unary => ApplicationOperator::Unary,
        Operator::Binary => ApplicationOperator::Binary,
        Operator::Conditional => ApplicationOperator::Conditional,
        Operator::Let => ApplicationOperator::Let,
        Operator::Quantify => ApplicationOperator::Quantify,
        Operator::Collection => ApplicationOperator::Collection,
        Operator::Query => ApplicationOperator::Query,
        Operator::Convert => ApplicationOperator::Convert,
        Operator::Pre => ApplicationOperator::Pre,
        Operator::Present => ApplicationOperator::Present,
        Operator::Value => ApplicationOperator::Value,
        Operator::Deref => ApplicationOperator::Deref,
        Operator::Reaches => ApplicationOperator::Reaches,
        Operator::Temporal => ApplicationOperator::Temporal,
        Operator::TemporalFormula => ApplicationOperator::TemporalFormula,
        Operator::TemporalFairness => ApplicationOperator::TemporalFairness,
        Operator::ProtocolControl => ApplicationOperator::ProtocolControl,
        Operator::StateTransition => ApplicationOperator::StateTransition,
        Operator::StateClause => ApplicationOperator::StateClause,
        Operator::Claim => ApplicationOperator::Claim,
        Operator::Case => ApplicationOperator::Case,
    }
}

fn operation_term(
    ids: &StructuralIds,
    operation: &qsl_semantics::check::Operation,
) -> Result<quire_contract_model::CheckedApplicationOperation, EmitRefusal> {
    use serde_json::json;
    let laws = |laws: &[OperationLaw]| {
        laws.iter()
            .map(|law| {
                json!({
                    "role": law.role.as_str(),
                    "definition": law.definition,
                })
            })
            .collect::<Vec<_>>()
    };
    let mode = |mode: Option<&qsl_semantics::check::OperationMode>| {
        mode.map(|mode| match mode {
            qsl_semantics::check::OperationMode::Rounding(value) => {
                json!({"kind":"rounding", "value":value.as_str()})
            }
            qsl_semantics::check::OperationMode::TextProfile(value) => {
                json!({"kind":"text_profile", "value":value.as_str()})
            }
            qsl_semantics::check::OperationMode::Absence(value) => {
                json!({"kind":"absence", "value":value.as_str()})
            }
        })
    };
    let leaves = operation
        .leaves()
        .iter()
        .map(|leaf| {
            let path = leaf
                .path
                .iter()
                .map(|segment| match segment {
                    qsl_semantics::check::LeafSegment::Field(name) => {
                        format!("field:{}", name.as_str())
                    }
                    qsl_semantics::check::LeafSegment::Position(position) => {
                        format!("position:{position}")
                    }
                    qsl_semantics::check::LeafSegment::Inner => "inner".into(),
                    qsl_semantics::check::LeafSegment::Member(name) => {
                        format!("member:{}", name.as_str())
                    }
                    qsl_semantics::check::LeafSegment::Recursion(depth) => {
                        format!("recursion:{depth}")
                    }
                })
                .collect::<Vec<_>>();
            json!({"path":path, "laws":laws(&leaf.laws), "mode":mode(leaf.mode.as_ref())})
        })
        .collect::<Vec<_>>();
    let member = operation
        .member()
        .map(|value| member_value(ids, value))
        .transpose()?;
    quire_contract_model::CheckedApplicationOperation::deserialize(json!({
        "identity": operation.identity(), "laws": laws(operation.laws()),
        "mode": mode(operation.mode()), "member": member, "leaves": leaves,
    }))
    .map_err(encoding)
}

fn member_value(ids: &StructuralIds, member: &Member) -> Result<serde_json::Value, EmitRefusal> {
    use serde_json::json;
    Ok(match member {
        Member::Field {
            declaration,
            name,
            explicit_deref,
        } => {
            let mut member = json!({
                "kind":"field",
                "declaration":node_id(ids, declaration),
                "name":name.as_str()
            });
            if let Some(explicit_deref) = explicit_deref {
                let read_form = if *explicit_deref {
                    "explicit_deref"
                } else {
                    "bare"
                };
                member["read_form"] = json!(read_form);
            }
            member
        }
        Member::Position {
            declaration,
            position,
        } => json!({"kind":"position","declaration":node_id(ids, declaration),"position":position}),
        Member::Element { declaration } => {
            json!({"kind":"element","declaration":node_id(ids, declaration)})
        }
        Member::RelationshipEnd {
            declaration,
            name,
            destination_type,
            population,
        } => {
            let mut record = json!({"kind":"relationship_end","declaration":node_id(ids, declaration),"name":name.as_str(),"destination_type":node_id(ids, destination_type)});
            if let Some(population) = population {
                record["population"] = json!(node_id(ids, population));
            }
            record
        }
        Member::Operation { declaration, name } => {
            json!({"kind":"operation","declaration":node_id(ids, declaration),"name":name.as_str()})
        }
        Member::TypeArgument { declaration } => {
            json!({"kind":"type_argument","declaration":node_id(ids, declaration)})
        }
        Member::ProfileOperator { operator } => {
            json!({"kind":"profile_operator","operator":operator.as_str()})
        }
        Member::StateClause {
            execution_point, ..
        } => {
            let clause = member
                .state_clause_wire_kind()
                .ok_or_else(|| encoding("a state clause member has no checked wire kind"))?;
            let mut record = json!({"kind":"state_clause","clause":clause});
            if let Some(point) = execution_point {
                record["execution_point"] = json!(point.point());
            }
            record
        }
        Member::TemporalInterval { interval } => json!({
            "kind":"temporal_interval",
            "interval":interval.map(|interval| json!({
                "lower":interval.lower().to_string(),
                "upper": match interval.upper() {
                    qsl_foundation::bound::TemporalUpper::Finite(value) => Some(value.to_string()),
                    qsl_foundation::bound::TemporalUpper::Open => None,
                },
            })),
        }),
        Member::TimedInterval(timed) => {
            let rational = |value: &quire_exact::Rational| {
                json!({
                    "numerator":value.numerator().to_string(),
                    "denominator":value.denominator().to_string(),
                })
            };
            json!({"kind":"temporal_interval","interval":{
                "lower":rational(&timed.lower),
                "lower_end":if timed.lower_closed {"closed"} else {"open"},
                "upper":timed.upper.as_ref().map(rational),
                "upper_end":if timed.upper_closed {"closed"} else {"open"},
            }})
        }
        Member::Fairness {
            kind,
            granularity,
            target,
        } => json!({
            "kind":"fairness",
            "fairness_kind":match kind { quire_semantic_value::fairness::FairnessKind::Weak => "weak", quire_semantic_value::fairness::FairnessKind::Strong => "strong" },
            "granularity":match granularity { quire_semantic_value::fairness::FairnessGranularity::Whole => "whole", quire_semantic_value::fairness::FairnessGranularity::Each => "each" },
            "target":match target {
                qsl_semantics::value::FairnessTarget::Operation { declaration, name } => json!({"kind":"operation", "declaration":node_id(ids, declaration), "name":name.as_str()}),
                qsl_semantics::value::FairnessTarget::ProtocolRoot { node } => json!({"kind":"protocol_root", "node":node_id(ids, node)}),
                qsl_semantics::value::FairnessTarget::AttemptNode { node } => json!({"kind":"attempt_node", "node":node_id(ids, node)}),
                qsl_semantics::value::FairnessTarget::ParallelBranch { parallel, branch } => json!({"kind":"parallel_branch", "parallel":node_id(ids, parallel), "branch":branch.as_str()}),
                qsl_semantics::value::FairnessTarget::CompensationTemplate { node } => json!({"kind":"compensation_template", "node":node_id(ids, node)}),
                qsl_semantics::value::FairnessTarget::HybridTransition { declaration, name } => json!({"kind":"hybrid_transition", "declaration":node_id(ids, declaration), "name":name.as_str()}),
            }
        }),
    })
}

/// Whether every node an `operation_effects` row names is a node of the
/// package. An omitted node (an unsupported form, or a node naming one) makes
/// the effect unavailable, and a present member must cover every operation, so
/// the whole member is then left out rather than refused by the reader
/// (FR-496).
fn operation_effects_are_carried(
    rows: &[quire_contract_model::CheckedOperationEffectRow],
    nodes: &[CheckedSemanticNodeV2],
) -> bool {
    let carried: BTreeSet<&CheckedNodeId> = nodes.iter().map(|node| &node.node_id).collect();
    rows.iter().all(|row| {
        let modified = row.effect.modifies.iter().map(|entry| match entry {
            CheckedFrameModification::Field { declaration, .. }
            | CheckedFrameModification::Relationship { declaration } => declaration,
        });
        std::iter::once(&row.declaring)
            .chain(modified)
            .chain(&row.effect.creates)
            .chain(&row.effect.deletes)
            .all(|id| carried.contains(id))
    })
}

fn frame_term(ids: &StructuralIds, frame: &FrameTerm) -> Result<CheckedTerm, EmitRefusal> {
    let modifies = frame
        .modifies
        .iter()
        .map(|entry| match entry {
            FrameModifies::Field(field) => Ok(CheckedFrameModification::Field {
                declaration: admitted_id(ids, &field.declaration())?,
                name: field.name().into(),
                scope: match field.scope() {
                    FrameScope::EveryObject => CheckedFieldScope::EveryObject,
                    FrameScope::Receiver => CheckedFieldScope::Receiver,
                },
            }),
            FrameModifies::Relationship(target) => Ok(CheckedFrameModification::Relationship {
                declaration: admitted_id(ids, target)?,
            }),
        })
        .collect::<Result<Vec<_>, EmitRefusal>>()?;
    let deletes_scoped = frame
        .deletes_scoped
        .iter()
        .map(|entry| match entry {
            FrameDeletesScoped::Receiver => CheckedScopedDeletion::Receiver,
            FrameDeletesScoped::Parameter(name) => CheckedScopedDeletion::Parameter {
                name: name.as_str().into(),
            },
        })
        .collect();
    Ok(CheckedTerm::Frame {
        modifies,
        creates: frame
            .creates
            .iter()
            .map(|reference| admitted_id(ids, reference))
            .collect::<Result<_, _>>()?,
        deletes: frame
            .deletes
            .iter()
            .map(|reference| admitted_id(ids, reference))
            .collect::<Result<_, _>>()?,
        deletes_scoped,
    })
}

fn abstraction_term(
    ids: &StructuralIds,
    abstraction: &qsl_semantics::check::AbstractionTerm,
) -> Result<CheckedTerm, EmitRefusal> {
    let objects = abstraction
        .objects()
        .iter()
        .map(|object| {
            Ok(CheckedAbstractionObject {
                type_id: admitted_id(ids, object.ty())?,
                type_path: object
                    .rust_type()
                    .iter()
                    .map(|part| part.as_str().into())
                    .collect(),
                fields: object
                    .fields()
                    .map(|(name, rust_field)| CheckedAbstractionField {
                        name: name.into(),
                        field_ident: rust_field.into(),
                    })
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, EmitRefusal>>()?;
    let populations = abstraction
        .populations()
        .iter()
        .map(|population| {
            Ok(CheckedAbstractionPopulation {
                population: admitted_id(ids, population.population())?,
                collection: population
                    .collection()
                    .iter()
                    .map(|part| part.as_str().into())
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, EmitRefusal>>()?;
    let frames = abstraction
        .frames()
        .iter()
        .map(|frame| {
            Ok(CheckedAbstractionFrame {
                context: admitted_id(ids, frame.context())?,
                operation: frame.operation().into(),
                function: frame
                    .function()
                    .iter()
                    .map(|part| part.as_str().into())
                    .collect(),
                receiver: frame.receiver().into(),
                parameters: frame
                    .parameters()
                    .map(|(name, rust_parameter)| CheckedAbstractionParameter {
                        name: name.into(),
                        parameter_ident: rust_parameter.into(),
                    })
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, EmitRefusal>>()?;
    Ok(CheckedTerm::AbstractionRelation {
        objects,
        populations,
        frames,
    })
}

/// A structural node's admitted `owner` as the v2 wire writes it (FR-093).
fn wire_owner(owner: &Owner) -> CheckedNodeOwner {
    match owner {
        Owner::Source(subject) => CheckedNodeOwner::Source {
            authority: subject.authority().into(),
            identity: subject.identity().into(),
        },
        Owner::Model(subject) => CheckedNodeOwner::Model {
            identity: subject.identity().into(),
            node: subject.node().into(),
        },
        Owner::ModelRoot(subject) => CheckedNodeOwner::ModelRoot {
            identity: subject.identity().into(),
        },
    }
}

/// Whether the lock the emitter writes selects the owner `node` joins. A
/// node without a nominal payload (an enum member, a lowering-built unit or
/// dimension aside) joins trivially; a nominal node joins only when its
/// owner is the checked unit's `source`, the one lock entry it can always
/// join, or a model the unit selects (`models`, the identities of its
/// `model_root` nodes) (FR-322).
fn owner_is_locked(node: &SemanticNode, source: &RawSourceRef, models: &BTreeSet<&str>) -> bool {
    if node.nominal_payload().is_none() {
        return !matches!(node.semantic_form(), "dimension" | "unit");
    }
    // The declaring owner the payload retains is the truth; a payload built
    // without one (the timed path) joins by the node's own owner.
    match node.nominal_payload().and_then(NominalPayload::owner) {
        Some(NodeOwner::Model(subject)) => models.contains(subject.identity.as_str()),
        Some(owner) => matches!(
            owner,
            NodeOwner::Source(subject)
                if subject.authority == source.authority() && subject.identity == source.identity()
        ),
        None => match node.owner() {
            Some(Owner::Model(subject)) => models.contains(subject.identity()),
            owner => matches!(
                owner,
                Some(Owner::Source(subject))
                    if subject.authority() == source.authority()
                        && subject.identity() == source.identity()
            ),
        },
    }
}

/// Which candidates the wire omits, and why: an unsupported form, a
/// declaration without its occurrence, a nominal owner the lock does not
/// select, a name outside the graph, and then, transitively, a name of an
/// omitted node.
fn omissions(
    ids: &StructuralIds,
    candidates: &Candidates<'_>,
    source: &RawSourceRef,
    decodes: &dyn Fn(CheckedNodeTag, &str) -> bool,
    external: &HashSet<NodeKey>,
    models: &BTreeSet<&str>,
) -> Vec<(usize, OmittedNode)> {
    let mut omitted: Vec<Option<OmissionCause>> = vec![None; candidates.order.len()];
    for (position, candidate) in candidates.values().enumerate() {
        omitted[position] = if !candidate.form_is_supported(decodes) {
            Some(OmissionCause::UnsupportedForm {
                node_tag: candidate.node.node_tag(),
                semantic_form: candidate.node.semantic_form(),
            })
        } else if !candidate.declaration_matches_occurrences() {
            Some(OmissionCause::DeclarationOccurrenceMismatch)
        } else if !owner_is_locked(candidate.node, source, models) {
            Some(OmissionCause::UnlockedOwner)
        } else {
            // The least absent name: a node names few, and absent ones are
            // rarer still, so ordering them is cheap.
            candidate
                .names
                .iter()
                .filter(|name| !candidates.contains(name) && !external.contains(*name))
                .min()
                .map(|absent| OmissionCause::NamesAbsentNode(node_id(ids, absent)))
        };
    }
    let mut named_by: HashMap<&NodeKey, Vec<usize>> = HashMap::new();
    for (position, candidate) in candidates.values().enumerate() {
        for name in &candidate.names {
            named_by.entry(name).or_default().push(position);
        }
    }
    let mut pending: VecDeque<usize> = (0..omitted.len())
        .filter(|&position| omitted[position].is_some())
        .collect();
    while let Some(gone) = pending.pop_front() {
        let gone_key = &candidates.order[gone].key;
        for &dependent in named_by.get(gone_key).into_iter().flatten() {
            if omitted[dependent].is_none() {
                omitted[dependent] = Some(OmissionCause::NamesOmittedNode(
                    candidates.order[gone].id.clone(),
                ));
                pending.push_back(dependent);
            }
        }
    }
    omitted
        .into_iter()
        .enumerate()
        .filter_map(|(position, cause)| {
            cause.map(|cause| {
                let node = candidates.order[position].id.clone();
                (position, OmittedNode { node, cause })
            })
        })
        .collect()
}

/// The role a clause-profile law is selected under in the lock's
/// `profile_selections`, or `None` for a value law, which the lock selects in
/// `definition_selections` (QSpec FR-322).
fn profile_role(role: LawRole) -> Option<CheckedSelectionRole> {
    match role {
        LawRole::TemporalProfile => Some(CheckedSelectionRole::TemporalProfile),
        LawRole::ProtocolProfile => Some(CheckedSelectionRole::ProtocolProfile),
        LawRole::IntegerDivision | LawRole::IeeeProfile | LawRole::TextProfile => None,
    }
}

/// The written nodes' laws: the lock's `profile_selections` rows for the
/// clause-profile laws, each once in first-use order, and the value laws'
/// definitions for `definition_selections`.
fn split_laws(order: &[&Candidate<'_>]) -> (Vec<CheckedSelection>, Vec<DefinitionReference>) {
    let mut profiles: Vec<CheckedSelection> = Vec::new();
    let mut definitions = Vec::new();
    for law in order.iter().flat_map(|candidate| &candidate.laws) {
        match profile_role(law.role) {
            Some(role) => {
                let selection = CheckedSelection {
                    role,
                    definition: artifact(&law.definition),
                };
                if !profiles.contains(&selection) {
                    profiles.push(selection);
                }
            }
            None => definitions.push(law.definition.clone()),
        }
    }
    (profiles, definitions)
}

/// FR-218: a `protocol_reference` carrier whose bound target is no node the
/// wire writes -- absent from the graph, or omitted from it -- refuses the
/// whole emission, never an omission: the reference binds a key, and the key
/// names nothing.
fn stale_protocol_reference(
    ids: &StructuralIds,
    candidates: &Candidates<'_>,
    dropped: &HashSet<usize>,
) -> Result<(), EmitRefusal> {
    for candidate in candidates.values() {
        let node = candidate.node;
        let carrier = CheckedNodeKind::decode(candidate.tag, node.semantic_form())
            == Some(CheckedNodeKind::Protocol(ProtocolForm::ProtocolReference));
        if !carrier {
            continue;
        }
        let BodyTerm::Leaf(LeafTerm::Reference { target }) = node.body() else {
            continue;
        };
        let target = admitted(target)?;
        let written = candidates
            .index
            .get(&target)
            .is_some_and(|position| !dropped.contains(position));
        if !written {
            return Err(EmitRefusal::StaleProtocolReference {
                reference: candidate.id.clone(),
                target: node_id(ids, &target),
            });
        }
    }
    Ok(())
}

/// FR-093's graph order: ascending by node id, except that a recursion
/// group's members are written together, in ordinal order, where the group's
/// first member by node id would be.
fn graph_order<'c, 'g>(kept: &[&'c Candidate<'g>]) -> Vec<&'c Candidate<'g>> {
    let mut groups: HashMap<NodeKey, Vec<Option<&'c Candidate<'g>>>> = HashMap::new();
    for candidate in kept {
        let Some(recursion) = candidate.node.recursion() else {
            continue;
        };
        let members = groups
            .entry(recursion.anchor().clone())
            .or_insert_with(|| vec![None; recursion.size()]);
        if let Some(member) = members.get_mut(recursion.ordinal()) {
            *member = Some(*candidate);
        }
    }
    let mut order = Vec::with_capacity(kept.len());
    let mut emitted_groups = HashSet::new();
    for candidate in kept {
        match candidate.node.recursion() {
            None => order.push(*candidate),
            Some(recursion) => {
                let group = recursion.anchor().clone();
                if emitted_groups.insert(group.clone()) {
                    if let Some(members) = groups.remove(&group) {
                        order.extend(members.into_iter().flatten());
                    }
                }
            }
        }
    }
    order
}

/// `reference` as the wire's `DefinitionRef`, `{authority, identity}`.
fn artifact(reference: &DefinitionReference) -> CheckedArtifactRef {
    CheckedArtifactRef {
        authority: reference.authority.as_str().into(),
        identity: reference.identity.as_str().into(),
    }
}

/// `source` as the wire's `RawSourceRef`: authority and identity.
fn source_artifact(source: &RawSourceRef) -> CheckedSourceRef {
    CheckedSourceRef {
        authority: source.authority().into(),
        identity: source.identity().into(),
    }
}

/// The lock's `edition` and `definition_selections` (FR-110 "Lock rows"):
/// the catalog's edition; then the header-selected layers and every layer
/// their `requires` closure names, in the lock's row order, and no layer
/// that requires a selected layer; then the qualification-catalog rows
/// QSpec FR-001 selects for those layers; then the complete-model and
/// complete-temporal lock rows when their layers are selected; then each law
/// definition the bodies name that is not already selected; then each
/// selected extension definition, ascending by `(authority, identity)`.
///
/// A unit whose header selects no layer selects the `always` rows: the
/// lock's `without_layer` rows are for a unit that selects layers none of
/// which is `quire.value.complete/v1`.
fn catalog_selections(
    layers: &[HeaderLayer],
    laws: &[DefinitionReference],
    extensions: &[qsl_foundation::selection::DefinitionRef],
) -> (CheckedSelection, Vec<CheckedArtifactRef>) {
    // A law's `DefinitionRef` is also part of the application-node keys that
    // name the law.
    let edition = CheckedSelection {
        role: CheckedSelectionRole::Edition,
        definition: artifact(&DefinitionRole::Edition.reference()),
    };
    let closure = HeaderLayer::closure(layers);
    let qualification = if layers.is_empty() {
        qsl_semantics::value::always_roles()
    } else {
        qsl_semantics::value::qualification_roles(&closure)
    };
    let selected = closure
        .iter()
        .map(|layer| layer.reference())
        .chain(
            qualification
                .iter()
                .filter(|role| **role != DefinitionRole::Edition)
                .map(|role| role.reference()),
        )
        // FR-361 item 3: the complete-model lock's `package_selection.always`
        // rows, for a unit whose layers hold the model layer.
        .chain(
            closure
                .contains(&HeaderLayer::ModelComplete)
                .then(|| ModelRole::ALL.into_iter().map(ModelRole::reference))
                .into_iter()
                .flatten(),
        )
        .chain(
            closure
                .contains(&HeaderLayer::TemporalComplete)
                .then(|| TemporalRole::ALL.into_iter().map(TemporalRole::reference))
                .into_iter()
                .flatten(),
        )
        .chain(laws.iter().cloned())
        // FR-110 "Lock rows" item 3: the extensions, last, already ascending.
        .chain(extensions.iter().map(|extension| DefinitionReference {
            authority: extension.authority().to_owned(),
            identity: extension.identity().to_owned(),
        }));
    let mut definitions: Vec<CheckedArtifactRef> = Vec::new();
    for definition in selected {
        let definition = artifact(&definition);
        if !definitions.contains(&definition) {
            definitions.push(definition);
        }
    }
    (edition, definitions)
}

/// Every occurrence `check` recorded, by node, with its FR-322 role and the
/// location it was recorded at.
fn recorded_occurrences(
    graph: &CheckedGraph,
) -> Result<HashMap<NodeKey, Recorded<'_>>, EmitRefusal> {
    let mut by_node: HashMap<NodeKey, Recorded<'_>> = HashMap::new();
    for (key, origin, location) in graph.occurrences() {
        let occurrence = CheckedOccurrence {
            role: occurrence_role(&key, &origin)?,
            ordinal: origin.ordinal(),
        };
        let location = if graph.parameter_span(key.clone(), &origin).is_some() {
            RecordedLocation::Parameter(graph.occurrence_region(key.clone(), &origin))
        } else {
            RecordedLocation::Expression(location)
        };
        by_node.entry(key).or_default().push((occurrence, location));
    }
    Ok(by_node)
}

/// Joins each equality producer row to the exact occurrence and checked id
/// emitted for its application. The join starts from QSL's retained key and
/// expression location; it never reconstructs a key from a checked id.
#[cfg(test)]
fn equality_producer_emission_rows(
    graph: &CheckedGraph,
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
    source_map: &[CheckedSourceMapEntry],
) -> Result<Vec<EqualityProducerEmissionRow>, EmitRefusal> {
    equality_producer_emission_rows_excluding(graph, emitted_node_ids, source_map, &HashSet::new())
}

fn equality_producer_emission_rows_excluding(
    graph: &CheckedGraph,
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
    source_map: &[CheckedSourceMapEntry],
    omitted_nodes: &HashSet<NodeKey>,
) -> Result<Vec<EqualityProducerEmissionRow>, EmitRefusal> {
    let producer_data = graph.equality_producer_data();
    let producer_identities = graph.equality_producer_identities();
    if producer_data.len() != producer_identities.len()
        || producer_data.keys().ne(producer_identities.keys())
    {
        let application = producer_data
            .keys()
            .next()
            .or_else(|| producer_identities.keys().next())
            .map(|(application, _)| application.clone())
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        return Err(EmitRefusal::MissingEqualityIdentityContext { application });
    }
    graph
        .equality_producer_identities()
        .iter()
        .filter(|((application, _), _)| !omitted_nodes.contains(application))
        .map(|((application, expression_location), producer_identity)| {
            let Some(checked_node) = emitted_node_ids.get(application).cloned() else {
                return Err(EmitRefusal::MissingEqualityIdentityContext {
                    application: application.clone(),
                });
            };
            let Some(producer_data) = producer_data
                .get(&(application.clone(), expression_location.clone()))
                .cloned()
            else {
                return Err(EmitRefusal::MissingEqualityIdentityContext {
                    application: application.clone(),
                });
            };
            // A Boolean function body can also carry claim and declaration
            // occurrences at this same node and source location. Equality
            // producer identity belongs to its expression occurrence only.
            let expression_role = Role::new("expression");
            let mut occurrences = graph.occurrences().filter(|(node, origin, location)| {
                node == application
                    && origin.role() == &expression_role
                    && *location == expression_location
            });
            let Some((node, origin, _)) = occurrences.next() else {
                return Err(EmitRefusal::MissingEqualityIdentityContext {
                    application: application.clone(),
                });
            };
            if occurrences.next().is_some() {
                return Err(EmitRefusal::MissingEqualityIdentityContext {
                    application: application.clone(),
                });
            }
            let occurrence =
                checked_occurrence(&origin).ok_or_else(|| EmitRefusal::UnknownOccurrenceRole {
                    node: application.clone(),
                    role: origin.role().to_string(),
                })?;
            let mut source_entries = source_map.iter().filter(|entry| {
                entry.node_id == checked_node
                    && entry.role == occurrence.role
                    && entry.ordinal == occurrence.ordinal
            });
            let Some(source_map_entry) = source_entries.next().cloned() else {
                return Err(EmitRefusal::MissingEqualityIdentityContext {
                    application: application.clone(),
                });
            };
            if source_entries.next().is_some() {
                return Err(EmitRefusal::MissingEqualityIdentityContext {
                    application: application.clone(),
                });
            }
            let package_sources: Vec<_> =
                graph.sources().into_iter().map(source_artifact).collect();
            if source_map_entry.regions.is_empty()
                || source_map_entry
                    .regions
                    .iter()
                    .any(|region| !package_sources.contains(&region.source))
            {
                return Err(EmitRefusal::MissingEqualityIdentityContext {
                    application: application.clone(),
                });
            }
            Ok(EqualityProducerEmissionRow {
                application: application.clone(),
                checked_node,
                occurrence,
                location: quire_exact::Location::new(node, origin),
                source_map_entry,
                producer_identity: producer_identity.clone(),
                producer_data,
            })
        })
        .collect()
}

/// Resolves each enum witness key independently through the emitter's exact
/// structural-key map. Keeping this join in one helper lets tests exercise
/// source, lowered declaration, and member associations with distinct keys.
fn enum_member_node_ids(
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
    source_declaration: &NodeKey,
    declaration: &NodeKey,
    member: &NodeKey,
    application: &NodeKey,
) -> Result<(CheckedNodeId, CheckedNodeId, CheckedNodeId), EmitRefusal> {
    let lookup = |key: &NodeKey| {
        emitted_node_ids.get(key).cloned().ok_or_else(|| {
            EmitRefusal::MissingEqualityIdentityContext {
                application: application.clone(),
            }
        })
    };
    Ok((
        lookup(source_declaration)?,
        lookup(declaration)?,
        lookup(member)?,
    ))
}

fn equality_fact_witness_rows(
    graph: &CheckedGraph,
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
    wire_nodes: &[CheckedSemanticNodeV2],
    rows: &[EqualityProducerEmissionRow],
) -> Result<Vec<EqualityFactWitnessRow>, EmitRefusal> {
    let graph_nodes = graph
        .semantic_graph()
        .nodes()
        .map(|node| (node.key(), node))
        .collect::<HashMap<_, _>>();
    let mut witnesses = Vec::new();
    for row in rows {
        let mut add = |key: &NodeKey,
                       fact: EqualityFactWitness,
                       value_type: Option<quire_exact::ValueType>| {
            // A selected Exact identity can name a declaration of a
            // dependency package. IR's typed equality reader accepts only
            // package-local structural facts, so rows whose key belongs to
            // another package than the application are the one deliberate
            // omission; every package-local key must still resolve.
            if key.package() != row.application().package() {
                return Ok(());
            }
            let Some(structural_node) = emitted_node_ids.get(key).cloned() else {
                return Err(EmitRefusal::MissingEqualityIdentityContext {
                    application: row.application().clone(),
                });
            };
            let Some(wire_node) = wire_nodes
                .iter()
                .find(|node| node.node_id == structural_node)
            else {
                return Err(EmitRefusal::MissingEqualityIdentityContext {
                    application: row.application().clone(),
                });
            };
            let owner = wire_node.owner.clone();
            witnesses.push(EqualityFactWitnessRow {
                application: row.application().clone(),
                checked_application: row.checked_node().clone(),
                occurrence: row.occurrence().clone(),
                source_map_entry: row.source_map_entry().clone(),
                structural_key: key.clone(),
                structural_node,
                owner,
                value_type,
                fact,
                selected_for_emission: true,
            });
            Ok(())
        };

        let identity_type_nodes = row.producer_identity.operand_type_nodes();
        if identity_type_nodes.len() != row.producer_data.operand_type_facts().len() {
            return Err(EmitRefusal::MissingEqualityIdentityContext {
                application: row.application().clone(),
            });
        }
        for (identity_fact, producer_fact) in identity_type_nodes
            .iter()
            .zip(row.producer_data.operand_type_facts())
        {
            if identity_fact.operand_position != producer_fact.operand_position
                || identity_fact.role != producer_fact.role
                || identity_fact.operand_position >= 2
            {
                return Err(EmitRefusal::MissingEqualityIdentityContext {
                    application: row.application().clone(),
                });
            }
            add(
                &identity_fact.node,
                EqualityFactWitness::OperandType {
                    position: identity_fact.operand_position,
                    role: match identity_fact.role {
                        qsl_semantics::check::EqualityUnitOperandRole::Source => {
                            EqualityOperandTypeRole::Source
                        }
                        qsl_semantics::check::EqualityUnitOperandRole::ConversionTarget => {
                            EqualityOperandTypeRole::ConversionTarget
                        }
                    },
                },
                Some(producer_fact.value_type.clone()),
            )?;
        }
        if matches!(
            row.producer_data.schedule(),
            quire_semantic_value::declaration::EqualitySchedule::Text
                | quire_semantic_value::declaration::EqualitySchedule::Plan
        ) {
            for (position, (ordinal, key)) in row
                .producer_identity
                .text_plan_type_nodes()
                .iter()
                .enumerate()
            {
                if position >= 2 {
                    return Err(EmitRefusal::MissingEqualityIdentityContext {
                        application: row.application().clone(),
                    });
                }
                let expected_role = if row.producer_data.authored_operands()[position]
                    .target()
                    .is_some()
                {
                    qsl_semantics::check::EqualityUnitOperandRole::ConversionTarget
                } else {
                    qsl_semantics::check::EqualityUnitOperandRole::Source
                };
                if *ordinal != position
                    || !identity_type_nodes.iter().any(|fact| {
                        fact.operand_position == position
                            && fact.role == expected_role
                            && fact.node == *key
                    })
                {
                    return Err(EmitRefusal::MissingEqualityIdentityContext {
                        application: row.application().clone(),
                    });
                }
                let mut typed_facts = identity_type_nodes
                    .iter()
                    .zip(row.producer_data.operand_type_facts())
                    .filter(|(identity_fact, _)| {
                        identity_fact.operand_position == position
                            && identity_fact.role == expected_role
                            && identity_fact.node == *key
                    });
                let Some((_, typed_fact)) = typed_facts.next() else {
                    return Err(EmitRefusal::MissingEqualityIdentityContext {
                        application: row.application().clone(),
                    });
                };
                if typed_facts.next().is_some() {
                    return Err(EmitRefusal::MissingEqualityIdentityContext {
                        application: row.application().clone(),
                    });
                }
                add(
                    key,
                    EqualityFactWitness::TextPlanType {
                        position,
                        role: match expected_role {
                            qsl_semantics::check::EqualityUnitOperandRole::Source => {
                                EqualityOperandTypeRole::Source
                            }
                            qsl_semantics::check::EqualityUnitOperandRole::ConversionTarget => {
                                EqualityOperandTypeRole::ConversionTarget
                            }
                        },
                    },
                    Some(typed_fact.value_type.clone()),
                )?;
            }
        }

        if row.producer_data.schedule() == quire_semantic_value::declaration::EqualitySchedule::Enum
        {
            let enum_types = row
                .producer_data
                .operand_type_facts()
                .iter()
                .filter(|fact| matches!(fact.value_type, quire_exact::ValueType::Enum(_)))
                .map(|fact| &fact.value_type)
                .collect::<Vec<_>>();
            let Some(enum_type) = enum_types.first() else {
                return Err(EmitRefusal::MissingEqualityIdentityContext {
                    application: row.application().clone(),
                });
            };
            if enum_types.iter().any(|other| *other != *enum_type) {
                return Err(EmitRefusal::MissingEqualityIdentityContext {
                    application: row.application().clone(),
                });
            }
            for (source_declaration, declaration, member, case, ordinal, ordered) in
                row.producer_identity.enum_member_facts()
            {
                let (source_declaration_node, declaration_node, member_node) =
                    enum_member_node_ids(
                        emitted_node_ids,
                        source_declaration,
                        declaration,
                        member,
                        row.application(),
                    )?;
                add(
                    member,
                    EqualityFactWitness::EnumMember {
                        source_declaration: source_declaration.clone(),
                        source_declaration_node,
                        declaration: declaration.clone(),
                        declaration_node,
                        member: member.clone(),
                        member_node,
                        case: case.clone(),
                        ordinal: *ordinal,
                        ordered: *ordered,
                    },
                    Some((*enum_type).clone()),
                )?;
            }
        }

        if row.producer_data.schedule()
            == quire_semantic_value::declaration::EqualitySchedule::Quantity
        {
            let identity = row.producer_identity();
            for unit_fact in identity.unit_facts() {
                let Some(unit) = row.producer_data.units().get(unit_fact.unit_id.clone()) else {
                    return Err(EmitRefusal::MissingEqualityIdentityContext {
                        application: row.application().clone(),
                    });
                };
                let quantity_key = &unit_fact.structural_node;
                let exact_unit_key = match unit {
                    quire_semantic_value::quantity::QuantityUnit::Declared(unit) => {
                        unit.key() == *quantity_key
                    }
                    quire_semantic_value::quantity::QuantityUnit::Compound(_) => {
                        graph_nodes.get(quantity_key).is_some_and(|node| {
                            WireForm::from_wire(node.semantic_form())
                                == Some(WireForm::CompoundUnit)
                        })
                    }
                };
                if !exact_unit_key {
                    return Err(EmitRefusal::MissingEqualityIdentityContext {
                        application: row.application().clone(),
                    });
                }
                let operand_position = unit_fact.operand_position;
                let role = match unit_fact.operand_role {
                    qsl_semantics::check::EqualityUnitOperandRole::Source => {
                        EqualityUnitRole::Source
                    }
                    qsl_semantics::check::EqualityUnitOperandRole::ConversionTarget => {
                        EqualityUnitRole::ConversionTarget
                    }
                };
                {
                    let type_id = emitted_node_ids.get(quantity_key).cloned().ok_or_else(|| {
                        EmitRefusal::MissingEqualityIdentityContext {
                            application: row.application().clone(),
                        }
                    })?;
                    add(
                        quantity_key,
                        EqualityFactWitness::Unit {
                            unit_id: unit_fact.unit_id.clone(),
                            quantity_type: type_id,
                            operand_position,
                            role,
                        },
                        None,
                    )?;
                    for dimension in &unit_fact.dimensions {
                        let exact_role = match &unit {
                            quire_semantic_value::quantity::QuantityUnit::Declared(unit) => {
                                match dimension.role {
                                    qsl_semantics::check::EqualityDimensionRole::UnitDimension => {
                                        dimension.node == unit.dimension_node()
                                            && dimension.ordinal == 0
                                    }
                                    qsl_semantics::check::EqualityDimensionRole::ConversionDimension => {
                                        unit_fact.conversion_edges.iter().any(|edge| {
                                            graph_nodes
                                                .get(&edge.target)
                                                .and_then(|node| node.nominal_payload())
                                                .is_some_and(|payload| matches!(
                                                    payload,
                                                    NominalPayload::Unit(target_unit)
                                                        if target_unit.dimension() == &dimension.node
                                                ))
                                        })
                                    }
                                    qsl_semantics::check::EqualityDimensionRole::BaseDimension => {
                                        unit_fact
                                            .dimension_terms
                                            .iter()
                                            .any(|term| term.base == dimension.node)
                                    }
                                    qsl_semantics::check::EqualityDimensionRole::CompoundBase => {
                                        false
                                    }
                                }
                            }
                            quire_semantic_value::quantity::QuantityUnit::Compound(compound) => {
                                dimension.role
                                    == qsl_semantics::check::EqualityDimensionRole::CompoundBase
                                    && compound
                                        .dimension()
                                        .exponents()
                                        .any(|(key, _)| key == dimension.node)
                            }
                        };
                        let exact_dimension = graph_nodes
                            .get(&dimension.node)
                            .and_then(|node| node.nominal_payload())
                            .is_some_and(|payload| matches!(payload, NominalPayload::Dimension(_)));
                        if dimension.unit_id != unit_fact.unit_id || !exact_dimension || !exact_role
                        {
                            return Err(EmitRefusal::MissingEqualityIdentityContext {
                                application: row.application().clone(),
                            });
                        }
                        add(
                            &dimension.node,
                            EqualityFactWitness::DimensionDeclaration {
                                unit_id: unit_fact.unit_id.clone(),
                                operand_position,
                                role: dimension.role,
                                ordinal: dimension.ordinal,
                            },
                            None,
                        )?;
                        // A unit of a base dimension carries that dimension
                        // as its canonical base-dimension term. The IR
                        // equality reader joins this separately from the
                        // authored UnitDimension declaration row.
                        if dimension.role
                            == qsl_semantics::check::EqualityDimensionRole::UnitDimension
                            && matches!(
                                &unit,
                                quire_semantic_value::quantity::QuantityUnit::Declared(_)
                            )
                            && graph_nodes
                                .get(&dimension.node)
                                .and_then(|node| node.nominal_payload())
                                .and_then(|payload| match payload {
                                    NominalPayload::Dimension(dimension) => {
                                        Some(dimension.terms().is_empty())
                                    }
                                    _ => None,
                                })
                                == Some(true)
                        {
                            add(
                                &dimension.node,
                                EqualityFactWitness::DimensionDeclaration {
                                    unit_id: unit_fact.unit_id.clone(),
                                    operand_position,
                                    role:
                                        qsl_semantics::check::EqualityDimensionRole::BaseDimension,
                                    ordinal: 0,
                                },
                                None,
                            )?;
                        }
                    }
                    for term in &unit_fact.dimension_terms {
                        let exact_term = graph_nodes
                            .get(&term.declaration)
                            .and_then(|node| node.nominal_payload())
                            .and_then(|payload| match payload {
                                NominalPayload::Dimension(dimension) => {
                                    dimension.terms().get(term.ordinal)
                                }
                                _ => None,
                            })
                            .is_some_and(|(base, exponent)| {
                                base == &term.base && exponent == &term.exponent
                            });
                        if term.unit_id != unit_fact.unit_id || !exact_term {
                            return Err(EmitRefusal::MissingEqualityIdentityContext {
                                application: row.application().clone(),
                            });
                        }
                        let base_node =
                            emitted_node_ids.get(&term.base).cloned().ok_or_else(|| {
                                EmitRefusal::MissingEqualityIdentityContext {
                                    application: row.application().clone(),
                                }
                            })?;
                        add(
                            &term.declaration,
                            EqualityFactWitness::DimensionTerm {
                                unit_id: unit_fact.unit_id.clone(),
                                operand_position,
                                ordinal: term.ordinal,
                                base: term.base.clone(),
                                base_node,
                                exponent: term.exponent.to_string().into_boxed_str(),
                            },
                            None,
                        )?;
                    }
                    for edge in &unit_fact.conversion_edges {
                        let exact_target = graph_nodes
                            .get(&edge.source)
                            .and_then(|node| node.nominal_payload())
                            .and_then(|payload| match payload {
                                NominalPayload::Unit(unit) => unit.target(),
                                _ => None,
                            })
                            .is_some_and(|target| target == &edge.target);
                        if edge.unit_id != unit_fact.unit_id || !exact_target {
                            return Err(EmitRefusal::MissingEqualityIdentityContext {
                                application: row.application().clone(),
                            });
                        }
                        let Some(NominalPayload::Unit(payload)) = graph_nodes
                            .get(&edge.source)
                            .and_then(|node| node.nominal_payload())
                        else {
                            return Err(EmitRefusal::MissingEqualityIdentityContext {
                                application: row.application().clone(),
                            });
                        };
                        let target_node =
                            emitted_node_ids.get(&edge.target).cloned().ok_or_else(|| {
                                EmitRefusal::MissingEqualityIdentityContext {
                                    application: row.application().clone(),
                                }
                            })?;
                        add(
                            &edge.source,
                            EqualityFactWitness::ConversionEdge {
                                unit_id: unit_fact.unit_id.clone(),
                                operand_position,
                                ordinal: edge.ordinal,
                                target: edge.target.clone(),
                                target_node,
                                scale: checked_rational(payload.factor()),
                                offset: checked_rational(payload.offset()),
                            },
                            None,
                        )?;
                    }
                }
            }
        }
    }
    Ok(witnesses)
}

/// Turns the checked equality decision and its exact occurrence witness into
/// FR-322's wire sidecar. The structural graph supplies operand type nodes;
/// the producer rows supply the schedule and occurrence-specific decision.
fn checked_equality_term_data(
    graph: &CheckedGraph,
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
    selected_model_members: &[(
        NodeKey,
        NodeKey,
        quire_contract_model::SelectedModelEnumMemberRow,
    )],
    rows: &[EqualityProducerEmissionRow],
) -> Result<Option<quire_contract_model::CheckedTermDataV2>, EmitRefusal> {
    if rows.is_empty() {
        return Ok(None);
    }
    let graph_nodes = graph
        .semantic_graph()
        .nodes()
        .map(|node| (node.key(), node))
        .collect::<HashMap<_, _>>();
    let mut equality_decisions = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(application) = graph_nodes.get(row.application()) else {
            return Err(EmitRefusal::MissingEqualityIdentityContext {
                application: row.application().clone(),
            });
        };
        let qsl_semantics::check::BodyTerm::Application(application_body) = application.body()
        else {
            return Err(EmitRefusal::MissingEqualityIdentityContext {
                application: row.application().clone(),
            });
        };
        let operation = application_body.operation.identity();
        let operator_matches = match row.producer_data.operator() {
            quire_semantic_value::declaration::EqualityOperator::Equal => {
                OperationSuffix::of(operation) == Some(OperationSuffix::Eq)
            }
            quire_semantic_value::declaration::EqualityOperator::NotEqual => {
                OperationSuffix::of(operation) == Some(OperationSuffix::Ne)
            }
        };
        if !operator_matches || application_body.arguments.len() != 2 {
            return Err(EmitRefusal::MissingEqualityIdentityContext {
                application: row.application().clone(),
            });
        }
        let mut operands = Vec::with_capacity(2);
        let mut compared_type_keys = Vec::with_capacity(2);
        let mut quantity_type_keys = Vec::with_capacity(4);
        for (position, authored) in row.producer_data.authored_operands().iter().enumerate() {
            let Some(qsl_semantics::check::MemberTerm::Leaf(operand)) =
                application_body.arguments.get(position)
            else {
                return Err(EmitRefusal::MissingEqualityIdentityContext {
                    application: row.application().clone(),
                });
            };
            let (operand_node, compared_type_key) = match operand {
                qsl_semantics::check::LeafTerm::Reference { target } => {
                    let Some(operand_node) = target.admitted().and_then(|key| graph_nodes.get(key))
                    else {
                        return Err(EmitRefusal::MissingEqualityIdentityContext {
                            application: row.application().clone(),
                        });
                    };
                    let compared_type_key = operand_node.semantic_type().ok_or_else(|| {
                        EmitRefusal::MissingEqualityIdentityContext {
                            application: row.application().clone(),
                        }
                    })?;
                    (Some(*operand_node), compared_type_key)
                }
                qsl_semantics::check::LeafTerm::Literal { ty, .. } => {
                    let Some(ty) = ty.admitted() else {
                        return Err(EmitRefusal::MissingEqualityIdentityContext {
                            application: row.application().clone(),
                        });
                    };
                    (None, ty.clone())
                }
                qsl_semantics::check::LeafTerm::DependencyReference { .. } => {
                    return Err(EmitRefusal::MissingEqualityIdentityContext {
                        application: row.application().clone(),
                    });
                }
            };
            compared_type_keys.push(compared_type_key.clone());
            let conversion_target_key = compared_type_key.clone();
            let mut source_type = compared_type_key.clone();
            if authored.target().is_some() {
                if let Some(operand_node) = operand_node {
                    if let qsl_semantics::check::BodyTerm::Application(convert) =
                        operand_node.body()
                    {
                        if OperationSuffix::of(convert.operation.identity())
                            == Some(OperationSuffix::Convert)
                        {
                            let Some(qsl_semantics::check::MemberTerm::Leaf(
                                qsl_semantics::check::LeafTerm::Reference { target },
                            )) = convert.arguments.first()
                            else {
                                return Err(EmitRefusal::MissingEqualityIdentityContext {
                                    application: row.application().clone(),
                                });
                            };
                            let Some(source_node) =
                                target.admitted().and_then(|key| graph_nodes.get(key))
                            else {
                                return Err(EmitRefusal::MissingEqualityIdentityContext {
                                    application: row.application().clone(),
                                });
                            };
                            source_type = source_node.semantic_type().ok_or_else(|| {
                                EmitRefusal::MissingEqualityIdentityContext {
                                    application: row.application().clone(),
                                }
                            })?;
                        }
                    }
                }
            }
            let source_type_key = source_type;
            let source_type = emitted_node_ids
                .get(&source_type_key)
                .cloned()
                .ok_or_else(|| EmitRefusal::MissingEqualityIdentityContext {
                    application: row.application().clone(),
                })?;
            if row.producer_data.schedule()
                == quire_semantic_value::declaration::EqualitySchedule::Quantity
            {
                quantity_type_keys.push(source_type_key.clone());
                if authored.target().is_some() {
                    quantity_type_keys.push(compared_type_key);
                }
            }
            let conversion_target = if authored.target().is_some() {
                Some(
                    emitted_node_ids
                        .get(&conversion_target_key)
                        .cloned()
                        .ok_or_else(|| EmitRefusal::MissingEqualityIdentityContext {
                            application: row.application().clone(),
                        })?,
                )
            } else {
                None
            };
            operands.push(quire_contract_model::CheckedEqualityOperandDecision {
                operand: u8::try_from(position).map_err(|_| EmitRefusal::SourceMapInvariant)?,
                source_type,
                conversion_target,
            });
        }
        let mut operands = operands.into_iter();
        let left = operands.next().ok_or(EmitRefusal::SourceMapInvariant)?;
        let right = operands.next().ok_or(EmitRefusal::SourceMapInvariant)?;
        let operator = match row.producer_data.operator() {
            quire_semantic_value::declaration::EqualityOperator::Equal => {
                quire_contract_model::CheckedEqualityOperator::Equal
            }
            quire_semantic_value::declaration::EqualityOperator::NotEqual => {
                quire_contract_model::CheckedEqualityOperator::NotEqual
            }
        };
        if matches!(
            row.producer_data.schedule(),
            quire_semantic_value::declaration::EqualitySchedule::Text
                | quire_semantic_value::declaration::EqualitySchedule::Plan
        ) {
            let identity_types = row.producer_identity.text_plan_type_nodes();
            if identity_types.len() != 2
                || identity_types
                    .iter()
                    .enumerate()
                    .any(|(position, (ordinal, key))| {
                        *ordinal != position || compared_type_keys.get(position) != Some(key)
                    })
            {
                return Err(EmitRefusal::MissingEqualityIdentityContext {
                    application: row.application.clone(),
                });
            }
        }
        let schedule = match row.producer_data.schedule() {
            quire_semantic_value::declaration::EqualitySchedule::Text => {
                quire_contract_model::CheckedEqualitySchedule::Text
            }
            quire_semantic_value::declaration::EqualitySchedule::Enum => {
                quire_contract_model::CheckedEqualitySchedule::Enum
            }
            quire_semantic_value::declaration::EqualitySchedule::Quantity => {
                quire_contract_model::CheckedEqualitySchedule::Quantity
            }
            quire_semantic_value::declaration::EqualitySchedule::Plan => {
                quire_contract_model::CheckedEqualitySchedule::Plan
            }
        };
        let enum_members = if row.producer_data.schedule()
            == quire_semantic_value::declaration::EqualitySchedule::Enum
        {
            equality_enum_members(
                &graph_nodes,
                emitted_node_ids,
                selected_model_members,
                &row.application,
                compared_type_keys
                    .first()
                    .ok_or(EmitRefusal::SourceMapInvariant)?,
                &row.producer_identity,
            )?
        } else {
            Vec::new()
        };
        let units = if row.producer_data.schedule()
            == quire_semantic_value::declaration::EqualitySchedule::Quantity
        {
            equality_units(
                &graph_nodes,
                emitted_node_ids,
                &row.application,
                &row.producer_data,
                &row.producer_identity,
                quantity_type_keys,
            )?
        } else {
            Vec::new()
        };
        equality_decisions.push(quire_contract_model::CheckedEqualityDecision {
            application: row.checked_node().clone(),
            occurrence: row.occurrence().clone(),
            term_path: Vec::new(),
            operator,
            left,
            right,
            schedule,
            units,
            enum_members,
        });
    }
    Ok(Some(quire_contract_model::CheckedTermDataV2 {
        equality_decisions,
        protocol_locations: Vec::new(),
        dispatch_tables: Vec::new(),
        probabilistic_claims: Vec::new(),
    }))
}

/// Convert the retained FR-150 identity without looking up a display name or
/// traversing the checked graph.
fn checked_effective_id(
    identity: &quire_exact::EffectiveId,
) -> quire_contract_model::CheckedEffectiveId {
    let mut current = None;
    for original in identity.owner_chain() {
        current = Some(quire_contract_model::CheckedEffectiveId {
            owner_effective_type: current.map(Box::new),
            original: quire_contract_model::CheckedOriginalDeclarationId {
                package: original.package().into(),
                node: original.node().into(),
            },
        });
    }
    current.expect("EffectiveId always has an original declaration")
}

/// Emit exact dispatch decisions retained while lowering each typed dispatch
/// application. The only joins here are producer keys to their emitted ids
/// and occurrence records.
fn checked_dispatch_tables(
    graph: &CheckedGraph,
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
    source_map: &[CheckedSourceMapEntry],
) -> Result<Vec<quire_contract_model::CheckedDispatchCandidateTable>, EmitRefusal> {
    let mut rows = Vec::with_capacity(graph.dispatch_producer_decisions().len());
    for ((application, location), decision) in graph.dispatch_producer_decisions() {
        let refusal = || EmitRefusal::MissingDispatchIdentityContext {
            application: application.clone(),
        };
        let checked_application = emitted_node_ids
            .get(application)
            .cloned()
            .ok_or_else(refusal)?;
        let mut occurrences = graph
            .occurrences()
            .filter(|(node, _, found_location)| node == application && *found_location == location);
        let Some((_, origin, _)) = occurrences.next() else {
            return Err(refusal());
        };
        if occurrences.next().is_some() {
            return Err(refusal());
        }
        let occurrence = checked_occurrence(&origin).ok_or_else(refusal)?;
        let mut source_rows = source_map.iter().filter(|entry| {
            entry.node_id == checked_application
                && entry.role == occurrence.role
                && entry.ordinal == occurrence.ordinal
        });
        let Some(source_row) = source_rows.next() else {
            return Err(refusal());
        };
        if source_rows.next().is_some()
            || source_row.regions.is_empty()
            || source_row
                .regions
                .iter()
                .any(|region| region.source != source_artifact(graph.source()))
        {
            return Err(refusal());
        }
        let operation = quire_contract_model::CheckedSelectedOperationWitness {
            declaring: emitted_node_ids
                .get(&decision.declaring_node)
                .cloned()
                .ok_or_else(refusal)?,
            operation: decision.operation.selected_operation.as_str().into(),
        };
        let context = emitted_node_ids
            .get(&decision.context_node)
            .cloned()
            .ok_or_else(refusal)?;
        let mut candidates = Vec::with_capacity(decision.table.entries().len());
        for (selection_key, candidate) in decision.table.entries() {
            let operation_body = graph
                .function_identity_at(candidate.body)
                .and_then(|identity| emitted_node_ids.get(&identity).cloned())
                .ok_or_else(refusal)?;
            let effective_precondition = candidate
                .precondition
                .map(|index| {
                    graph
                        .function_identity_at(index)
                        .and_then(|identity| emitted_node_ids.get(&identity).cloned())
                        .ok_or_else(refusal)
                })
                .transpose()?;
            candidates.push(quire_contract_model::CheckedDispatchCandidate {
                selected_operation: operation.clone(),
                selection_key: checked_effective_id(selection_key),
                operation_body,
                effective_precondition,
            });
        }
        rows.push(quire_contract_model::CheckedDispatchCandidateTable {
            application: checked_application,
            occurrence,
            operation,
            context,
            candidates,
        });
    }
    rows.sort_by(|left, right| {
        left.application
            .package
            .cmp(&right.application.package)
            .then_with(|| left.application.path.cmp(&right.application.path))
            .then_with(|| left.application.ordinal.cmp(&right.application.ordinal))
            .then_with(|| left.occurrence.cmp(&right.occurrence))
    });
    Ok(rows)
}

/// Combine the independently retained producer decision families into the
/// one FR-322 checked-term carrier.
fn checked_term_data(
    graph: &CheckedGraph,
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
    selected_model_members: &[(
        NodeKey,
        NodeKey,
        quire_contract_model::SelectedModelEnumMemberRow,
    )],
    equality_rows: &[EqualityProducerEmissionRow],
    source_map: &[CheckedSourceMapEntry],
) -> Result<Option<quire_contract_model::CheckedTermDataV2>, EmitRefusal> {
    let mut term_data = checked_equality_term_data(
        graph,
        emitted_node_ids,
        selected_model_members,
        equality_rows,
    )?
    .unwrap_or_default();
    term_data.dispatch_tables = checked_dispatch_tables(graph, emitted_node_ids, source_map)?;
    term_data.protocol_locations =
        protocol_locations::checked_protocol_locations(graph, emitted_node_ids)?;
    term_data.probabilistic_claims = checked_probabilistic_claims(graph, emitted_node_ids)?;
    Ok((!term_data.equality_decisions.is_empty()
        || !term_data.protocol_locations.is_empty()
        || !term_data.dispatch_tables.is_empty()
        || !term_data.probabilistic_claims.is_empty())
    .then_some(term_data))
}

/// FR-498: one `{declaration, occurrence}` row per checked probabilistic
/// claim, naming the node and `claim` occurrence lowering keyed it at, in
/// ascending structural `NodeRef` order, then occurrence order. Nothing is
/// read from the claim's name, span or form.
fn checked_probabilistic_claims(
    graph: &CheckedGraph,
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
) -> Result<Vec<quire_contract_model::CheckedProbabilisticClaim>, EmitRefusal> {
    let claims = graph.probabilistic_model().claims();
    let mut rows = Vec::with_capacity(claims.len());
    for claim in claims {
        let unkeyed = || EmitRefusal::UnkeyedProbabilisticClaim {
            claim: claim.name.clone(),
        };
        let occurrence = claim.occurrence().ok_or_else(unkeyed)?;
        let declaration = emitted_node_ids
            .get(occurrence.node())
            .cloned()
            .ok_or_else(unkeyed)?;
        let occurrence = checked_occurrence(occurrence.origin()).ok_or_else(unkeyed)?;
        rows.push(quire_contract_model::CheckedProbabilisticClaim {
            declaration,
            occurrence,
        });
    }
    rows.sort_by(|left, right| {
        left.declaration
            .package
            .cmp(&right.declaration.package)
            .then_with(|| left.declaration.path.cmp(&right.declaration.path))
            .then_with(|| left.declaration.ordinal.cmp(&right.declaration.ordinal))
            .then_with(|| left.occurrence.cmp(&right.occurrence))
    });
    Ok(rows)
}

fn equality_enum_members(
    graph_nodes: &HashMap<NodeKey, &qsl_semantics::check::SemanticNode>,
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
    selected_model_members: &[(
        NodeKey,
        NodeKey,
        quire_contract_model::SelectedModelEnumMemberRow,
    )],
    application: &NodeKey,
    enum_type: &NodeKey,
    identity: &qsl_semantics::check::EqualityProducerIdentity,
) -> Result<Vec<quire_contract_model::CheckedEqualityEnumMember>, EmitRefusal> {
    let member_facts = identity
        .enum_member_facts()
        .iter()
        .filter(|(source_declaration, _, _, _, _, _)| source_declaration == enum_type)
        .collect::<Vec<_>>();
    if member_facts.is_empty() {
        return Err(EmitRefusal::MissingEqualityIdentityContext {
            application: application.clone(),
        });
    }
    member_facts
        .into_iter()
        .map(|(_, declaration, member, case, ordinal, ordered)| {
            let member_id = if let Some(member_node) = graph_nodes.get(member) {
                if member_node.semantic_type().as_ref() != Some(declaration)
                    || WireForm::from_wire(member_node.semantic_form()) != Some(WireForm::EnumValue)
                    || !matches!(
                        member_node.body(),
                        qsl_semantics::check::BodyTerm::Leaf(
                            qsl_semantics::check::LeafTerm::Literal {
                                value: qsl_semantics::check::LiteralValue::Enum(value),
                                ..
                            }
                        ) if value == case.as_ref()
                    )
                {
                    return Err(EmitRefusal::MissingEqualityIdentityContext {
                        application: application.clone(),
                    });
                }
                emitted_node_ids.get(member).cloned()
            } else {
                let selected = selected_model_members
                    .iter()
                    .filter(|(key, declaration_key, row)| {
                        key == member
                            && declaration_key == declaration
                            && row.member_key == *member
                            && row.case == *case
                    })
                    .collect::<Vec<_>>();
                if selected.len() != 1 {
                    return Err(EmitRefusal::MissingEqualityIdentityContext {
                        application: application.clone(),
                    });
                }
                Some(selected[0].2.structural_node.clone())
            }
            .ok_or_else(|| EmitRefusal::MissingEqualityIdentityContext {
                application: application.clone(),
            })?;
            Ok(quire_contract_model::CheckedEqualityEnumMember {
                enum_type: emitted_node_ids.get(enum_type).cloned().ok_or_else(|| {
                    EmitRefusal::MissingEqualityIdentityContext {
                        application: application.clone(),
                    }
                })?,
                member: member_id,
                case: case.clone(),
                ordinal: u64::try_from(*ordinal).map_err(|_| EmitRefusal::SourceMapInvariant)?,
                ordered: *ordered,
            })
        })
        .collect()
}

fn equality_units(
    graph_nodes: &HashMap<NodeKey, &qsl_semantics::check::SemanticNode>,
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
    application: &NodeKey,
    data: &qsl_semantics::check::EqualityProducerData,
    identity: &qsl_semantics::check::EqualityProducerIdentity,
    quantity_type_keys: Vec<NodeKey>,
) -> Result<Vec<quire_contract_model::CheckedResolvedEqualityUnit>, EmitRefusal> {
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    for quantity_type in quantity_type_keys {
        if !seen.insert(quantity_type.clone()) {
            continue;
        }
        let Some(quantity_node) = graph_nodes.get(&quantity_type) else {
            return Err(EmitRefusal::MissingEqualityIdentityContext {
                application: application.clone(),
            });
        };
        let unit_fact =
            equality_unit_identity_for_type(identity.unit_facts(), &quantity_type, application)?;
        let selected_id = unit_fact.unit_id.clone();
        if !identity.unit_ids().contains(&selected_id) {
            return Err(EmitRefusal::MissingEqualityIdentityContext {
                application: application.clone(),
            });
        }
        let selected_unit = data
            .units()
            .get(selected_id.clone())
            .cloned()
            .ok_or_else(|| EmitRefusal::MissingEqualityIdentityContext {
                application: application.clone(),
            })?;
        let exact_unit_type = match &selected_unit {
            quire_semantic_value::quantity::QuantityUnit::Declared(unit) => {
                unit.key() == quantity_type
            }
            quire_semantic_value::quantity::QuantityUnit::Compound(_) => {
                WireForm::from_wire(quantity_node.semantic_form()) == Some(WireForm::CompoundUnit)
            }
        };
        if !exact_unit_type {
            return Err(EmitRefusal::MissingEqualityIdentityContext {
                application: application.clone(),
            });
        }
        let unit_identity = unit_fact;
        let compound = match &selected_unit {
            quire_semantic_value::quantity::QuantityUnit::Compound(compound) => Some(compound),
            quire_semantic_value::quantity::QuantityUnit::Declared(_) => None,
        };
        let quantity_type_id = emitted_node_ids
            .get(&quantity_type)
            .cloned()
            .ok_or_else(|| EmitRefusal::MissingEqualityIdentityContext {
                application: application.clone(),
            })?;
        let dimension_terms = selected_unit
            .dimension()
            .exponents()
            .map(|(dimension, exponent)| {
                if !unit_identity.dimensions.iter().any(|fact| {
                    fact.node == dimension
                        || unit_identity
                            .dimension_terms
                            .iter()
                            .any(|term| term.base == dimension)
                }) {
                    return Err(EmitRefusal::MissingEqualityIdentityContext {
                        application: application.clone(),
                    });
                }
                Ok(quire_contract_model::CheckedUnitDimensionTerm {
                    dimension: emitted_node_ids.get(&dimension).cloned().ok_or_else(|| {
                        EmitRefusal::MissingEqualityIdentityContext {
                            application: application.clone(),
                        }
                    })?,
                    exponent: exponent.to_string().into_boxed_str(),
                })
            })
            .collect::<Result<Vec<_>, EmitRefusal>>()?;
        let form = if let Some(compound) = compound {
            quire_contract_model::CheckedResolvedEqualityUnitForm::Compound {
                dimension_terms,
                terms: compound
                    .terms()
                    .map(|(root_unit, exponent)| {
                        Ok(quire_contract_model::CheckedCompoundEqualityUnitTerm {
                            root_unit: emitted_node_ids.get(&root_unit).cloned().ok_or_else(
                                || EmitRefusal::MissingEqualityIdentityContext {
                                    application: application.clone(),
                                },
                            )?,
                            exponent: exponent.to_string().into_boxed_str(),
                        })
                    })
                    .collect::<Result<Vec<_>, EmitRefusal>>()?,
            }
        } else {
            let quire_semantic_value::quantity::QuantityUnit::Declared(unit) = &selected_unit
            else {
                return Err(EmitRefusal::SourceMapInvariant);
            };
            let mut path = Vec::new();
            let mut current = unit.key();
            for (ordinal, edge) in unit.path().iter().enumerate() {
                let Some(node) = graph_nodes.get(&current) else {
                    return Err(EmitRefusal::MissingEqualityIdentityContext {
                        application: application.clone(),
                    });
                };
                let Some(qsl_semantics::value::nominal_payload::NominalPayload::Unit(payload)) =
                    node.nominal_payload()
                else {
                    return Err(EmitRefusal::MissingEqualityIdentityContext {
                        application: application.clone(),
                    });
                };
                let Some(target) = payload.target().cloned() else {
                    return Err(EmitRefusal::MissingEqualityIdentityContext {
                        application: application.clone(),
                    });
                };
                if !unit_identity.conversion_edges.iter().any(|fact| {
                    fact.source == current && fact.ordinal == ordinal && fact.target == target
                }) {
                    return Err(EmitRefusal::MissingEqualityIdentityContext {
                        application: application.clone(),
                    });
                }
                path.push(quire_contract_model::CheckedUnitConversionEdge {
                    target: emitted_node_ids.get(&target).cloned().ok_or_else(|| {
                        EmitRefusal::MissingEqualityIdentityContext {
                            application: application.clone(),
                        }
                    })?,
                    scale: checked_rational(edge.scale()),
                    offset: checked_rational(edge.offset()),
                });
                current = target;
            }
            quire_contract_model::CheckedResolvedEqualityUnitForm::Declared {
                // Every referenced declaration is backed by QSL's exact
                // check-stage identity row before its checked node id is used.
                dimension_node: emitted_node_ids
                    .get(&unit.dimension_node())
                    .cloned()
                    .ok_or_else(|| EmitRefusal::MissingEqualityIdentityContext {
                        application: application.clone(),
                    })?,
                dimension_terms,
                root_unit: emitted_node_ids.get(&unit.root()).cloned().ok_or_else(|| {
                    EmitRefusal::MissingEqualityIdentityContext {
                        application: application.clone(),
                    }
                })?,
                path,
            }
        };
        let unit_id = match compound {
            Some(_) => quantity_type_id.clone(),
            None => match &selected_unit {
                quire_semantic_value::quantity::QuantityUnit::Declared(unit) => emitted_node_ids
                    .get(&unit.key())
                    .cloned()
                    .ok_or_else(|| EmitRefusal::MissingEqualityIdentityContext {
                        application: application.clone(),
                    })?,
                quire_semantic_value::quantity::QuantityUnit::Compound(_) => {
                    return Err(EmitRefusal::SourceMapInvariant)
                }
            },
        };
        if !identity.unit_ids().contains(&selected_id) {
            return Err(EmitRefusal::MissingEqualityIdentityContext {
                application: application.clone(),
            });
        }
        rows.push(quire_contract_model::CheckedResolvedEqualityUnit {
            quantity_type: quantity_type_id,
            unit: unit_id,
            form,
        });
    }
    Ok(rows)
}

fn equality_unit_identity_for_type<'a>(
    facts: &'a [qsl_semantics::check::EqualityUnitIdentityFact],
    quantity_type: &NodeKey,
    application: &NodeKey,
) -> Result<&'a qsl_semantics::check::EqualityUnitIdentityFact, EmitRefusal> {
    let mut matching = facts
        .iter()
        .filter(|fact| fact.structural_node == *quantity_type);
    let Some(selected) = matching.next() else {
        return Err(EmitRefusal::MissingEqualityIdentityContext {
            application: application.clone(),
        });
    };
    if matching.any(|fact| fact.unit_id != selected.unit_id) {
        return Err(EmitRefusal::MissingEqualityIdentityContext {
            application: application.clone(),
        });
    }
    Ok(selected)
}

/// The source map of the written nodes, in graph order, and the raw sources
/// its regions name.
fn source_map(
    order: &[&Candidate<'_>],
    regions: &impl Fn(&Location) -> Option<SourceRegion>,
) -> Result<(Vec<CheckedSourceMapEntry>, BTreeSet<CheckedSourceRef>), EmitRefusal> {
    let mut entries = Vec::new();
    let mut sources = BTreeSet::new();
    for candidate in order {
        for (occurrence, location) in &candidate.occurrences {
            // FR-322 regions are non-empty half-open intervals: an empty
            // region places the occurrence nowhere.
            let region = match location {
                RecordedLocation::Expression(location) => regions(location),
                RecordedLocation::Parameter(region) => region.clone(),
            }
            .filter(|region| region.start() < region.end())
            .ok_or_else(|| EmitRefusal::UnlocatedOccurrence {
                node: candidate.node.key(),
                role: occurrence.role.clone(),
                ordinal: occurrence.ordinal,
            })?;
            let source = source_artifact(region.source());
            sources.insert(source.clone());
            entries.push(CheckedSourceMapEntry {
                node_id: candidate.id.clone(),
                role: occurrence.role.clone(),
                ordinal: occurrence.ordinal,
                regions: vec![CheckedSourceRegion {
                    source,
                    start: region.start(),
                    end: region.end(),
                }],
            });
        }
    }
    Ok((entries, sources))
}

/// The exact typed source map represented by the just-built v2 entries.
fn inspection_source_map(
    ids: &StructuralIds,
    entries: &[CheckedSourceMapEntry],
) -> Result<PackageSourceMap, EmitRefusal> {
    let mut mapped = Vec::with_capacity(entries.len());
    // One memo for every entry: a nested operand is converted once, so the
    // map is linear in the package, not in the sum of each id's depth.
    let mut keys = ids.key_reader();
    for entry in entries {
        let node = keys
            .key(&entry.node_id)
            .map_err(|_| EmitRefusal::SourceMapInvariant)?;
        let key = OccurrenceKey::new(
            node,
            Origin::new(
                Role::new(crate::checked_v2::role_spelling(&entry.role)),
                entry.ordinal,
            ),
        );
        let mut regions = Vec::with_capacity(entry.regions.len());
        for region in &entry.regions {
            let source = crate::checked_v2::raw_source_ref(&region.source)
                .map_err(|_| EmitRefusal::SourceMapInvariant)?;
            regions.push(
                SourceRegion::new(source, region.start, region.end)
                    .map_err(|_| EmitRefusal::SourceMapInvariant)?,
            );
        }
        mapped.push((key, regions));
    }
    PackageSourceMap::from_entries(mapped).map_err(|_| EmitRefusal::SourceMapInvariant)
}

/// A refinement's V2 node (QSpec FR-375), its source-map entries and its
/// `refinement_step_rows` entries. Its `declaration` occurrence is at
/// `region`; a proof-eligible owner also has one `generated` occurrence per
/// step-simulation obligation, the initial one at `region` and the one of
/// `steps[i]` at that row's own region (FR-322-AC-87, AC-88). Every step
/// row has a sidecar entry at its region, eligible or not (FR-322-AC-129).
fn refinement_node(
    ids: &StructuralIds,
    graph: &CheckedGraph,
    refinement: &CheckedRefinement,
    region: &SourceRegion,
) -> Result<
    (
        CheckedSemanticNodeV2,
        Vec<CheckedSourceMapEntry>,
        Vec<CheckedRefinementStepRowRegion>,
    ),
    EmitRefusal,
> {
    let emission = refinement.v2_emission().map_err(|error| match error {
        qsl_semantics::check::RefinementV2EncodingError::MissingSubject
        | qsl_semantics::check::RefinementV2EncodingError::MissingRows => {
            EmitRefusal::RefinementSubjectNotEmitted
        }
        qsl_semantics::check::RefinementV2EncodingError::Body(_)
        | qsl_semantics::check::RefinementV2EncodingError::Term(_)
        | qsl_semantics::check::RefinementV2EncodingError::Identity(_) => {
            EmitRefusal::RefinementIdentityEncoding
        }
    })?;
    let concrete_node = node_id(
        ids,
        refinement
            .concrete_subject_node()
            .ok_or(EmitRefusal::RefinementSubjectNotEmitted)?,
    );
    let id = node_id(ids, emission.identity);
    let wire_region = |region: &SourceRegion| CheckedSourceRegion {
        source: source_artifact(region.source()),
        start: region.start(),
        end: region.end(),
    };
    let declaration_region = wire_region(region);
    let mut row_regions = Vec::with_capacity(emission.step_sources.len());
    let mut step_rows = Vec::with_capacity(emission.step_sources.len());
    for (ordinal, source) in emission.step_sources.iter().enumerate() {
        let row = graph
            .refinement_step_region(refinement, *source)
            .filter(|row| row.start() < row.end())
            .ok_or(EmitRefusal::UnlocatedRefinementDeclaration)?;
        let row = wire_region(&row);
        step_rows.push(CheckedRefinementStepRowRegion {
            node_id: id.clone(),
            ordinal: u64::try_from(ordinal).map_err(encoding)?,
            region: row.clone(),
        });
        row_regions.push(row);
    }
    let mut occurrences = vec![(
        CheckedOccurrence {
            role: CheckedOccurrenceRole::Declaration,
            ordinal: 0,
        },
        declaration_region.clone(),
    )];
    if emission.proof_eligible {
        let generated = std::iter::once(declaration_region).chain(row_regions);
        for (ordinal, region) in generated.enumerate() {
            occurrences.push((
                CheckedOccurrence {
                    role: CheckedOccurrenceRole::Generated,
                    ordinal: u64::try_from(ordinal).map_err(encoding)?,
                },
                region,
            ));
        }
    }
    let source_entries = occurrences
        .iter()
        .map(|(occurrence, region)| CheckedSourceMapEntry {
            node_id: id.clone(),
            role: occurrence.role.clone(),
            ordinal: occurrence.ordinal,
            regions: vec![region.clone()],
        })
        .collect();
    Ok((
        CheckedSemanticNodeV2 {
            owner_model: None,
            node_id: id,
            schema_version: GRAPH_V2.into(),
            node_tag: CheckedNodeTag::Relation.as_wire().into(),
            semantic_form: "refinement".into(),
            semantic_type: concrete_node,
            dependencies: emission
                .dependencies
                .into_iter()
                .map(|key| node_id(ids, key))
                .collect(),
            occurrences: occurrences
                .into_iter()
                .map(|(occurrence, _)| occurrence)
                .collect(),
            recursion_group: None,
            declaration: None,
            owner: None,
            body: emission.body,
        },
        source_entries,
        step_rows,
    ))
}

/// `origin`'s role as FR-322 spells it.
fn occurrence_role(node: &NodeKey, origin: &Origin) -> Result<CheckedOccurrenceRole, EmitRefusal> {
    crate::checked_v2::occurrence_role(origin.role().as_str()).ok_or_else(|| {
        EmitRefusal::UnknownOccurrenceRole {
            node: node.clone(),
            role: origin.role().to_string(),
        }
    })
}

/// `package`'s FR-322 direct `dependency_selections`, in ascending
/// UTF-8 byte order of identity.
fn dependency_selections(package: &CheckedPackage) -> Vec<CheckedDependencySelection> {
    package
        .direct_dependency_selections()
        .map(|(identity, _)| CheckedDependencySelection {
            identity: identity.as_str().into(),
        })
        .collect()
}

fn encoding(error: impl std::fmt::Display) -> EmitRefusal {
    EmitRefusal::Encoding {
        reason: error.to_string(),
    }
}

fn require_protocol_results_for_refusals(package: &CheckedPackage) -> Result<(), EmitRefusal> {
    if package.graph().protocol_refusals().is_empty() {
        Ok(())
    } else {
        Err(EmitRefusal::ProtocolResultsRequired)
    }
}

/// Emit `package` as `quire.checked-package/v2` bytes (FR-322), placing each
/// occurrence at the region of the checked unit its location names
/// (`CheckedGraph::region`, FR-096).
///
/// A package emitted before (by this function or
/// [`emit_checked_with_cancel`]) returns its remembered emission: a checked
/// package does not change, so a dependency reached again, or a caller that
/// emits what the lifecycle already emitted, does not emit it twice.
///
/// `limits` is the caller's ceiling on the package's canonical bytes
/// (`emit.output_bytes`); a remembered emission longer than it is refused
/// with [`EmitRefusal::OutputLimit`] in place of being returned.
pub fn emit_checked(package: &CheckedPackage, limits: EmitLimits) -> Result<Emission, EmitRefusal> {
    require_protocol_results_for_refusals(package)?;
    if let Some(emission) = package.remembered_emission() {
        return remembered_within(emission, limits);
    }
    emit_checked_with_cancel(package, &Cancel::new(), limits)
}

/// A remembered `emission`, when its canonical bytes fit `limits`.
fn remembered_within(emission: &Emission, limits: EmitLimits) -> Result<Emission, EmitRefusal> {
    let actual = u64::try_from(emission.package().bytes().len()).unwrap_or(u64::MAX);
    if actual > limits.output_bytes {
        return Err(EmitRefusal::OutputLimit(output_limit(
            limits.output_bytes,
            u128::from(actual),
        )));
    }
    Ok(emission.clone())
}

/// Emit the checked graph without reading or populating `CheckedPackage`'s
/// remembered emission. Per-protocol results use this before deciding
/// whether the aggregate is admissible.
fn emit_checked_uncached(
    package: &CheckedPackage,
    cancel: &Cancel,
    limits: EmitLimits,
) -> Result<Emission, EmitRefusal> {
    let graph = package.graph();
    emit_package_inner(
        package,
        graph.memoized_regions(),
        |_| None,
        cancel,
        &ir_decodes,
        &[],
        limits,
    )
}

/// Emit checked protocols independently and retain each protocol's result
/// when another protocol was refused by S3 (FR-383 AC-18, AC-28, AC-29).
/// The returned aggregate carries a package only when every protocol emits.
pub fn emit_checked_with_protocol_results(
    package: &CheckedPackage,
    limits: EmitLimits,
) -> Result<ProtocolEmission, EmitRefusal> {
    emit_checked_with_protocol_results_and_temporal_selections_and_cancel(
        package,
        &[],
        &Cancel::new(),
        limits,
    )
}

/// Emit each protocol independently while applying the package's explicit
/// temporal selections and polling the caller's cancellation handle.
pub fn emit_checked_with_protocol_results_and_temporal_selections_and_cancel(
    package: &CheckedPackage,
    selections: &[TemporalClockSelection],
    cancel: &Cancel,
    limits: EmitLimits,
) -> Result<ProtocolEmission, EmitRefusal> {
    let graph = package.graph();
    let emission = match emit_package_inner(
        package,
        graph.memoized_regions(),
        |_| None,
        cancel,
        &ir_decodes,
        selections,
        limits,
    ) {
        Ok(emission) => Some(emission),
        Err(EmitRefusal::NothingToEmit { .. }) if !graph.protocol_refusals().is_empty() => None,
        Err(refusal) => return Err(refusal),
    };
    let structural_nodes = emission
        .as_ref()
        .map(|emission| emission.wire_nodes.clone());
    let ids = StructuralIds::default();
    let mut structural_nodes = structural_nodes;
    if let Some(nodes) = structural_nodes.as_mut() {
        read_index_segments(&ids, nodes, &mut Vec::new())?;
    }
    let mut results = Vec::with_capacity(graph.protocol_order().len());
    let mut first_refusal = None;
    let mut first_incomplete = None;
    for declaration in graph.protocol_order() {
        if let Some(refused) = graph
            .protocol_refusals()
            .iter()
            .find(|refused| refused.declaration == *declaration)
        {
            let refusal = refused
                .refusals
                .first()
                .cloned()
                .ok_or(EmitRefusal::ProtocolResultInvariant)?;
            if let qsl_semantics::check::CheckCause::ResourceExhausted(limit) = &refusal.cause {
                if limit.kind == qsl_semantics::check::CheckingLimitKind::WorkBudget {
                    first_incomplete.get_or_insert_with(|| (**limit).clone());
                    results.push(ProtocolEmissionResult::Incomplete {
                        declaration: declaration.clone(),
                        limit: (**limit).clone(),
                    });
                    continue;
                }
            }
            first_refusal.get_or_insert_with(|| refusal.clone());
            results.push(ProtocolEmissionResult::Refused {
                declaration: declaration.clone(),
                source_index: refused.source_index,
                declaration_span: refused.declaration_span,
                refusal,
            });
            continue;
        }
        let keys = graph
            .protocol_node_keys(declaration)
            .ok_or(EmitRefusal::ProtocolResultInvariant)?;
        let clause_key = keys
            .clause
            .clone()
            .ok_or(EmitRefusal::ProtocolResultInvariant)?;
        let mut owned_ids = Vec::new();
        for key in keys
            .nodes
            .values()
            .chain(keys.channels.values())
            .chain(keys.roles.values())
            .chain(keys.compensations.values())
            .chain(keys.terminal.iter())
        {
            owned_ids.push(node_id(&ids, key));
        }
        owned_ids.push(node_id(&ids, &clause_key));
        let nodes = if let Some(structural_nodes) = structural_nodes.as_ref() {
            let nodes: Vec<serde_json::Value> = structural_nodes
                .iter()
                .filter(|node| owned_ids.contains(&node.node_id))
                .map(serde_json::to_value)
                .collect::<Result<_, _>>()
                .map_err(encoding)?;
            if !nodes.iter().any(|node| {
                node.get("semantic_form")
                    .and_then(serde_json::Value::as_str)
                    == Some("protocol_clause")
            }) {
                return Err(EmitRefusal::ProtocolResultInvariant);
            }
            nodes
        } else {
            return Err(EmitRefusal::ProtocolResultInvariant);
        };
        results.push(ProtocolEmissionResult::Emitted {
            declaration: declaration.clone(),
            clause_key,
            nodes,
        });
    }
    let aggregate = if let Some(primary) = first_refusal {
        ProtocolEmissionAggregate::Refused { primary }
    } else if let Some(primary) = first_incomplete {
        ProtocolEmissionAggregate::Incomplete { primary }
    } else {
        ProtocolEmissionAggregate::Emitted
    };
    let emission = match aggregate {
        ProtocolEmissionAggregate::Emitted => emission,
        ProtocolEmissionAggregate::Refused { .. }
        | ProtocolEmissionAggregate::Incomplete { .. } => None,
    };
    Ok(ProtocolEmission {
        results,
        aggregate,
        emission,
    })
}

/// Emit a checked package with explicit clock configurations for every
/// temporal declaration whose selected profile requires one (FR-050,
/// FR-322-AC-294). Each selection is joined to the checked graph by its exact
/// source document and declaration byte span. The checked clause supplies the
/// clock name, and the emitter supplies its final structural declaration ID.
///
/// This entry point does not use or populate the package's remembered
/// emission: the explicit selections are part of this emission's identity.
pub fn emit_checked_with_temporal_selections(
    package: &CheckedPackage,
    selections: &[TemporalClockSelection],
    limits: EmitLimits,
) -> Result<Emission, EmitRefusal> {
    emit_checked_with_temporal_selections_and_cancel(package, selections, &Cancel::new(), limits)
}

/// Emit with exact temporal selections while polling the caller's cancellation
/// handle at the same points as [`emit_checked_with_cancel`].
pub fn emit_checked_with_temporal_selections_and_cancel(
    package: &CheckedPackage,
    selections: &[TemporalClockSelection],
    cancel: &Cancel,
    limits: EmitLimits,
) -> Result<Emission, EmitRefusal> {
    require_protocol_results_for_refusals(package)?;
    let graph = package.graph();
    emit_package_inner(
        package,
        graph.memoized_regions(),
        |_| None,
        cancel,
        &ir_decodes,
        selections,
        limits,
    )
}

/// The metadata of a timed clause node with one `expression` occurrence,
/// keyed `id` until the emitter rekeys it.
fn clause_metadata(id: CheckedNodeId) -> TimedNodeMetadata {
    TimedNodeMetadata {
        node_id: id.clone(),
        schema_version: GRAPH_V2.into(),
        semantic_type: id,
        dependencies: Vec::new(),
        occurrences: vec![CheckedOccurrence {
            role: CheckedOccurrenceRole::Expression,
            ordinal: 0,
        }],
        recursion_group: None,
        declaration: None,
        owner: None,
    }
}

/// `span` as an emitted region's `(start, end)`.
fn span_region(span: qsl_foundation::Span) -> (u64, u64) {
    (span.start as u64, span.end as u64)
}

/// `value` as a reduced `numerator/denominator` quantity spelling.
fn quantity_spelling(value: &quire_exact::Rational) -> String {
    format!("{}/{}", value.numerator(), value.denominator())
}

fn claim_binding(name: &str, value: CheckedTerm) -> CheckedTerm {
    CheckedTerm::Binding {
        name: name.into(),
        value: Box::new(value),
    }
}

fn claim_aggregate(members: Vec<CheckedTerm>) -> CheckedTerm {
    CheckedTerm::Aggregate { members }
}

fn claim_literal(type_id: &CheckedNodeId, kind: LiteralKind, value: String) -> CheckedTerm {
    CheckedTerm::Literal {
        type_id: type_id.clone(),
        value_kind: kind,
        value: serde_json::Value::String(value),
    }
}

fn claim_text(type_id: &CheckedNodeId, value: &str) -> CheckedTerm {
    claim_literal(type_id, LiteralKind::Text, value.to_owned())
}

fn claim_quantity(type_id: &CheckedNodeId, value: &quire_exact::Rational) -> CheckedTerm {
    claim_literal(type_id, LiteralKind::Quantity, quantity_spelling(value))
}

fn schedulability_claim_body(
    ids: &StructuralIds,
    task_set: &qsl_semantics::check::TaskSet,
    claim: &CheckedScheduleClaim,
    text_type: &CheckedNodeId,
    integer_type: &CheckedNodeId,
) -> Result<CheckedTerm, EmitRefusal> {
    let unit = task_set
        .unit
        .as_ref()
        .ok_or(EmitRefusal::SourceMapInvariant)?;
    let unit_type = node_id(ids, &unit.node);
    let (kind, policy, task, bound) = match &claim.claim {
        ScheduleClaim::Schedulable(policy) => (
            "schedulable",
            *policy,
            claim_aggregate(Vec::new()),
            claim_aggregate(Vec::new()),
        ),
        ScheduleClaim::Response { task, bound } => (
            "response",
            SchedulingPolicy::FixedPriority,
            claim_text(text_type, task),
            claim_quantity(&unit_type, bound),
        ),
    };
    let policy = match policy {
        SchedulingPolicy::FixedPriority => "fixed-priority",
        SchedulingPolicy::Edf => "edf",
        SchedulingPolicy::Amc => "amc",
    };
    let mut tasks = task_set.tasks.iter().collect::<Vec<_>>();
    tasks.sort_by(|left, right| left.name.cmp(&right.name));
    let tasks = tasks
        .into_iter()
        .map(|task| {
            let (arrival, period) = match &task.arrival {
                Arrival::Periodic { period } => ("periodic", period),
                Arrival::Sporadic { min_interarrival } => ("sporadic", min_interarrival),
            };
            let (wcet, wcet_hi) = match &task.wcet {
                Wcet::Single(wcet) => (wcet, claim_aggregate(Vec::new())),
                Wcet::Dual { lo, hi } => (lo, claim_quantity(&unit_type, hi)),
            };
            let criticality = match task.criticality {
                qsl_semantics::check::Criticality::Lo => "lo",
                qsl_semantics::check::Criticality::Hi => "hi",
            };
            let priority = task.priority.map_or_else(
                || claim_aggregate(Vec::new()),
                |value| claim_literal(integer_type, LiteralKind::Integer, value.to_string()),
            );
            Ok(claim_aggregate(vec![
                claim_binding("name", claim_text(text_type, &task.name)),
                claim_binding("arrival", claim_text(text_type, arrival)),
                claim_binding("period", claim_quantity(&unit_type, period)),
                claim_binding("deadline", claim_quantity(&unit_type, &task.deadline)),
                claim_binding("bcet", claim_quantity(&unit_type, &task.bcet)),
                claim_binding("wcet", claim_quantity(&unit_type, wcet)),
                claim_binding("wcet_hi", wcet_hi),
                claim_binding("criticality", claim_text(text_type, criticality)),
                claim_binding("priority", priority),
                claim_binding("jitter", claim_quantity(&unit_type, &task.jitter)),
                claim_binding("blocking", claim_quantity(&unit_type, &task.blocking)),
            ]))
        })
        .collect::<Result<Vec<_>, EmitRefusal>>()?;
    Ok(claim_aggregate(vec![
        claim_binding("kind", claim_text(text_type, kind)),
        claim_binding("policy", claim_text(text_type, policy)),
        claim_binding("task", task),
        claim_binding("bound", bound),
        claim_binding("tasks", claim_aggregate(tasks)),
    ]))
}

/// The checked claims grouped by the node their normalized body keys. A
/// checked claim without a node key would be silently unemitted, so it is
/// refused as a broken invariant of the checker.
fn claims_by_node(
    claims: &[CheckedScheduleClaim],
) -> Result<BTreeMap<NodeKey, Vec<&CheckedScheduleClaim>>, EmitRefusal> {
    let mut by_node: BTreeMap<NodeKey, Vec<&CheckedScheduleClaim>> = BTreeMap::new();
    for claim in claims {
        let id = claim.node_id().ok_or(EmitRefusal::SourceMapInvariant)?;
        by_node.entry(id.clone()).or_default().push(claim);
    }
    Ok(by_node)
}

fn append_schedulability_claims(
    ids: &StructuralIds,
    graph: &CheckedGraph,
    task_sets: &CheckedTaskSets,
    source: &RawSourceRef,
    nodes: &mut Vec<CheckedSemanticNodeV2>,
    source_map: &mut Vec<CheckedSourceMapEntry>,
) -> Result<Vec<quire_contract_model::CheckedSchedulabilityClaimRow>, EmitRefusal> {
    let by_node = claims_by_node(task_sets.claims())?;
    if by_node.is_empty() {
        return Ok(Vec::new());
    }
    let mut nominal_nodes = graph.source_time_nominal_nodes().collect::<Vec<_>>();
    nominal_nodes.sort_by(|left, right| left.key.cmp(&right.key));
    for nominal in nominal_nodes {
        let id = node_id(ids, &nominal.key);
        if nodes.iter().any(|node| node.node_id == id) {
            continue;
        }
        let span = nominal.name_span().ok_or(EmitRefusal::SourceMapInvariant)?;
        let region = SourceRegion::new(source.clone(), span.start as u64, span.end as u64)
            .map_err(encoding)?;
        let owner = match &nominal.origin {
            TimeNominalOrigin::Source { owner, .. } => CheckedNodeOwner::Source {
                authority: owner.authority.as_str().into(),
                identity: owner.identity.as_str().into(),
            },
            TimeNominalOrigin::Model(_) => return Err(EmitRefusal::SourceMapInvariant),
        };
        let declaration = nominal.declaration().map(|segments| CheckedDeclaration {
            qualified_name: segments
                .iter()
                .map(|segment| segment.as_str().into())
                .collect(),
        });
        let semantic_type = if nominal.form == TimeNominalForm::Unit {
            nominal
                .dependencies
                .first()
                .map(|key| node_id(ids, key))
                .ok_or(EmitRefusal::SourceMapInvariant)?
        } else {
            id.clone()
        };
        let occurrence = CheckedOccurrence {
            role: CheckedOccurrenceRole::Declaration,
            ordinal: 0,
        };
        source_map.push(CheckedSourceMapEntry {
            node_id: id.clone(),
            role: occurrence.role.clone(),
            ordinal: occurrence.ordinal,
            regions: vec![CheckedSourceRegion {
                source: source_artifact(source),
                start: region.start(),
                end: region.end(),
            }],
        });
        let body = nominal
            .nominal_payload()
            .map(|payload| nominal_term(ids, &nominal.key, payload))
            .transpose()?
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let node = CheckedSemanticNodeV2 {
            owner_model: None,
            node_id: id.clone(),
            schema_version: GRAPH_V2.into(),
            node_tag: CheckedNodeTag::ScalarType.as_wire().into(),
            semantic_form: nominal.form.as_str().into(),
            semantic_type,
            dependencies: nominal
                .dependencies
                .iter()
                .map(|key| node_id(ids, key))
                .collect(),
            occurrences: vec![occurrence],
            recursion_group: None,
            declaration,
            owner: Some(owner),
            body,
        };
        let at = nodes.partition_point(|existing| existing.node_id < node.node_id);
        nodes.insert(at, node);
    }
    let package = by_node
        .keys()
        .next()
        .ok_or(EmitRefusal::SourceMapInvariant)?
        .package();
    let text_type = node_id(
        ids,
        qsl_semantics::check::builtin_text_type_key(package).map_err(encoding)?,
    );
    let integer_type = node_id(
        ids,
        qsl_semantics::check::builtin_integer_type_key(package).map_err(encoding)?,
    );
    let boolean_key = NodeKey::new(
        package,
        vec![
            PathSegment::Text("scalar_type".into()),
            PathSegment::Text("boolean".into()),
        ],
        0,
    )
    .map_err(encoding)?;
    let boolean_type = node_id(ids, boolean_key);
    let span = by_node
        .values()
        .next()
        .and_then(|claims| claims.first())
        .ok_or(EmitRefusal::SourceMapInvariant)?
        .span;
    let generated_region =
        SourceRegion::new(source.clone(), span.start as u64, span.end as u64).map_err(encoding)?;
    let generated = CheckedSourceRegion {
        source: source_artifact(source),
        start: generated_region.start(),
        end: generated_region.end(),
    };
    push_scalar_type(nodes, source_map, &text_type, "text", &generated);
    push_scalar_type(nodes, source_map, &integer_type, "integer", &generated);
    push_scalar_type(nodes, source_map, &boolean_type, "boolean", &generated);
    let mut rows = Vec::new();
    for (key, claims) in by_node {
        let first = *claims.first().ok_or(EmitRefusal::SourceMapInvariant)?;
        let task_set = task_sets
            .task_set_of(first)
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let id = node_id(ids, &key);
        let mut occurrences = claims
            .iter()
            .map(|claim| {
                claim
                    .occurrence()
                    .map(|occurrence| occurrence.origin().ordinal())
                    .ok_or(EmitRefusal::SourceMapInvariant)
            })
            .collect::<Result<Vec<_>, _>>()?;
        occurrences.sort_unstable();
        occurrences.dedup();
        for ordinal in &occurrences {
            let claim = claims
                .iter()
                .copied()
                .find(|claim| {
                    claim
                        .occurrence()
                        .is_some_and(|occurrence| occurrence.origin().ordinal() == *ordinal)
                })
                .ok_or(EmitRefusal::SourceMapInvariant)?;
            let region = SourceRegion::new(
                source.clone(),
                claim.span.start as u64,
                claim.span.end as u64,
            )
            .map_err(encoding)?;
            source_map.push(CheckedSourceMapEntry {
                node_id: id.clone(),
                role: CheckedOccurrenceRole::Claim,
                ordinal: *ordinal,
                regions: vec![CheckedSourceRegion {
                    source: source_artifact(source),
                    start: region.start(),
                    end: region.end(),
                }],
            });
            rows.push(quire_contract_model::CheckedSchedulabilityClaimRow {
                declaration: id.clone(),
                occurrence: *ordinal,
            });
        }
        let node = CheckedSemanticNodeV2 {
            owner_model: None,
            node_id: id.clone(),
            schema_version: GRAPH_V2.into(),
            node_tag: CheckedNodeTag::Claim.as_wire().into(),
            semantic_form: "schedulability".into(),
            semantic_type: boolean_type.clone(),
            dependencies: Vec::new(),
            occurrences: occurrences
                .iter()
                .map(|ordinal| CheckedOccurrence {
                    role: CheckedOccurrenceRole::Claim,
                    ordinal: *ordinal,
                })
                .collect(),
            recursion_group: None,
            declaration: None,
            owner: None,
            body: schedulability_claim_body(ids, task_set, first, &text_type, &integer_type)?,
        };
        let at = nodes.partition_point(|existing| existing.node_id < node.node_id);
        nodes.insert(at, node);
    }
    rows.sort_by(|left, right| {
        (&left.declaration, left.occurrence).cmp(&(&right.declaration, right.occurrence))
    });
    Ok(rows)
}

/// The wire comparison of a checked clock comparison.
fn clock_comparison(op: qsl_semantics::check::ClockOp) -> ClockComparison {
    use qsl_semantics::check::ClockOp;
    match op {
        ClockOp::Less => ClockComparison::Lt,
        ClockOp::LessOrEqual => ClockComparison::Le,
        ClockOp::Equal => ClockComparison::Eq,
        ClockOp::GreaterOrEqual => ClockComparison::Ge,
        ClockOp::Greater => ClockComparison::Gt,
    }
}

/// Add the node `metadata` stands for, or a further occurrence of an equal
/// node already written, located at `region`; returns the node's id.
fn occur(
    metadata: &mut TimedNodeMetadata,
    region: (u64, u64),
    regions: &mut BTreeMap<CheckedNodeId, Vec<(u64, u64)>>,
    fresh: bool,
) -> CheckedNodeId {
    if !fresh {
        let ordinal = metadata.occurrences.len() as u64;
        metadata.occurrences.push(CheckedOccurrence {
            role: CheckedOccurrenceRole::Expression,
            ordinal,
        });
    }
    regions
        .entry(metadata.node_id.clone())
        .or_default()
        .push(region);
    metadata.node_id.clone()
}

/// QSpec FR-322 "Clause nodes": the `clock_constraint`, `clock_guard`,
/// `clock_reset` and `time_invariant` nodes of `clauses` added to `model`,
/// each located at its own source span. `places` holds each clock's
/// `clock_place` by clock index and `unit` is the root's unit node. Clause
/// nodes of equal content are one node with one `expression` occurrence at
/// each source (FR-322: a node has one or more occurrences), listed once;
/// the root lists every constraint a part or an invariant names.
fn write_clauses(
    ids: &StructuralIds,
    model: &mut CheckedTimedModel,
    clauses: &qsl_semantics::check::TimedClauseNodes,
    places: &[CheckedNodeId],
    unit: &CheckedNodeId,
    placeholder: &dyn Fn(&str) -> Result<CheckedNodeId, EmitRefusal>,
    regions: &mut BTreeMap<CheckedNodeId, Vec<(u64, u64)>>,
) -> Result<(), EmitRefusal> {
    use qsl_semantics::check::{ConstraintTargetNodes, ResetTargetNodes};
    let place = |clock: qsl_semantics::check::ClockId| {
        places
            .get(clock.index())
            .cloned()
            .ok_or(EmitRefusal::TimedModelWriterInvariant(
                "a clause names a clock with no place",
            ))
    };
    let constraint = |model: &mut CheckedTimedModel,
                      regions: &mut BTreeMap<CheckedNodeId, Vec<(u64, u64)>>,
                      written: &qsl_semantics::check::ConstraintNodes|
     -> Result<CheckedNodeId, EmitRefusal> {
        let target = match &written.target {
            ConstraintTargetNodes::Place {
                clock,
                owner,
                slots,
            } => CheckedConstraintTarget::Place {
                place: place(*clock)?,
                owner: node_id(ids, owner),
                slots: slots.iter().map(|key| node_id(ids, key)).collect(),
            },
            ConstraintTargetNodes::Parameter {
                operation,
                parameter,
            } => CheckedConstraintTarget::Parameter {
                operation: node_id(ids, operation),
                parameter: parameter.clone(),
            },
        };
        let comparison = clock_comparison(written.comparison);
        let bound = quantity_spelling(&written.bound);
        let region = span_region(written.span);
        if let Some(held) = model.constraints.iter_mut().find(|held| {
            held.target == target && held.comparison == comparison && held.bound == bound
        }) {
            return Ok(occur(&mut held.metadata, region, regions, false));
        }
        let mut metadata = clause_metadata(placeholder(&format!(
            "constraint-{}",
            model.constraints.len()
        ))?);
        let id = occur(&mut metadata, region, regions, true);
        model.constraints.push(CheckedClockConstraint {
            metadata,
            target,
            comparison,
            bound,
            bound_type: unit.clone(),
        });
        Ok(id)
    };
    for guard in &clauses.guards {
        let mut parts = Vec::with_capacity(guard.parts.len());
        for part in &guard.parts {
            let mut constraints = Vec::with_capacity(part.constraints.len());
            for written in &part.constraints {
                let id = constraint(model, regions, written)?;
                if !constraints.contains(&id) {
                    constraints.push(id);
                }
            }
            parts.push(CheckedGuardPart {
                predicate: node_id(ids, &part.predicate),
                binders: part.binders.iter().map(|key| node_id(ids, key)).collect(),
                constraints,
            });
        }
        let mut metadata = clause_metadata(placeholder(&format!("guard-{}", model.guards.len()))?);
        occur(&mut metadata, span_region(guard.span), regions, true);
        model.guards.push(CheckedClockGuard {
            metadata,
            operation: node_id(ids, &guard.operation),
            parts,
        });
    }
    for invariant in &clauses.invariants {
        let mut bounds = Vec::with_capacity(invariant.bounds.len());
        for written in &invariant.bounds {
            let id = constraint(model, regions, written)?;
            if !bounds.contains(&id) {
                bounds.push(id);
            }
        }
        let predicate = node_id(ids, &invariant.predicate);
        let region = span_region(invariant.span);
        if let Some(held) = model.invariants.iter_mut().find(|held| {
            held.predicate == predicate
                && held.binders
                    == invariant
                        .binders
                        .iter()
                        .map(|key| node_id(ids, key))
                        .collect::<Vec<_>>()
                && held.bounds == bounds
        }) {
            occur(&mut held.metadata, region, regions, false);
            continue;
        }
        let mut metadata = clause_metadata(placeholder(&format!(
            "invariant-{}",
            model.invariants.len()
        ))?);
        occur(&mut metadata, region, regions, true);
        model.invariants.push(CheckedTimeInvariant {
            metadata,
            predicate,
            binders: invariant
                .binders
                .iter()
                .map(|key| node_id(ids, key))
                .collect(),
            bounds,
        });
    }
    for reset in &clauses.resets {
        let target = match &reset.target {
            ResetTargetNodes::Place {
                clock,
                owner,
                slots,
                binders,
                condition,
            } => CheckedResetTarget::Place {
                place: place(*clock)?,
                owner: node_id(ids, owner),
                slots: slots.iter().map(|key| node_id(ids, key)).collect(),
                binders: binders.iter().map(|key| node_id(ids, key)).collect(),
                condition: node_id(ids, condition),
            },
            ResetTargetNodes::Parameter { parameter } => CheckedResetTarget::Parameter {
                parameter: parameter.clone(),
            },
        };
        let operation = node_id(ids, &reset.operation);
        let value = quantity_spelling(&reset.value);
        let region = span_region(reset.span);
        if let Some(held) = model.resets.iter_mut().find(|held| {
            held.operation == operation && held.target == target && held.value == value
        }) {
            occur(&mut held.metadata, region, regions, false);
            continue;
        }
        let mut metadata = clause_metadata(placeholder(&format!("reset-{}", model.resets.len()))?);
        occur(&mut metadata, region, regions, true);
        model.resets.push(CheckedClockReset {
            metadata,
            operation,
            target,
            value,
            value_type: unit.clone(),
        });
    }
    Ok(())
}

/// Add the built-in scalar type node `id` of `form` (QSpec FR-092) to
/// `nodes`, with a `generated` source-map entry at `region`, unless the
/// graph holds it already: a timed node's literal or selector type.
fn push_scalar_type(
    nodes: &mut Vec<CheckedSemanticNodeV2>,
    source_map: &mut Vec<CheckedSourceMapEntry>,
    id: &CheckedNodeId,
    form: &str,
    region: &CheckedSourceRegion,
) {
    if nodes.iter().any(|node| node.node_id == *id) {
        return;
    }
    let occurrence = CheckedOccurrence {
        role: CheckedOccurrenceRole::Generated,
        ordinal: 0,
    };
    source_map.push(CheckedSourceMapEntry {
        node_id: id.clone(),
        role: occurrence.role.clone(),
        ordinal: occurrence.ordinal,
        regions: vec![region.clone()],
    });
    nodes.push(CheckedSemanticNodeV2 {
        owner_model: None,
        node_id: id.clone(),
        schema_version: GRAPH_V2.into(),
        node_tag: CheckedNodeTag::ScalarType.as_wire().into(),
        semantic_form: form.into(),
        semantic_type: id.clone(),
        dependencies: Vec::new(),
        occurrences: vec![occurrence],
        recursion_group: None,
        declaration: None,
        owner: None,
        body: CheckedTerm::Aggregate {
            members: Vec::new(),
        },
    });
}

/// Replace producer-local temporary timed identities with their structural
/// identities. Timed graph edges are acyclic, so each pass resolves another
/// level of references until every node and source-map entry agrees.
fn rekey_timed_model_nodes(
    ids: &StructuralIds,
    nodes: &mut [CheckedSemanticNodeV2],
    source_map: &mut [CheckedSourceMapEntry],
) -> Result<(), EmitRefusal> {
    let timed_count = nodes
        .iter()
        .filter(|node| {
            CheckedNodeTag::from_wire(&node.node_tag) == Some(CheckedNodeTag::TimedModel)
        })
        .count();
    for _ in 0..=timed_count.saturating_mul(timed_count).max(1) {
        let mut changes = Vec::new();
        for node in nodes.iter_mut().filter(|node| {
            CheckedNodeTag::from_wire(&node.node_tag) == Some(CheckedNodeTag::TimedModel)
        }) {
            node.dependencies = body_reference_targets(&node.body);
            let old = node.node_id.clone();
            let new = timed_model_node_id(ids, node)?;
            if new != old {
                changes.push((old, new));
            }
        }
        if changes.is_empty() {
            return Ok(());
        }
        // FR-322 "Operand index segments": a node whose own path holds a
        // rekeyed node as an operand is rekeyed with it, so every operand
        // stays a node of the graph.
        let mut next = 0;
        while let Some((old, new)) = changes.get(next).cloned() {
            next += 1;
            for node in nodes.iter() {
                if changes.iter().any(|(changed, _)| *changed == node.node_id) {
                    continue;
                }
                if let Some(rekeyed) = with_operand(&node.node_id, &old, &new) {
                    changes.push((node.node_id.clone(), rekeyed));
                }
            }
        }
        for (old, new) in changes {
            for node in nodes.iter_mut() {
                node.replace_node_id(&old, &new);
            }
            for entry in source_map.iter_mut() {
                if entry.node_id == old {
                    entry.node_id = new.clone();
                }
            }
        }
    }
    Err(EmitRefusal::TimedModelWriterInvariant(
        "timed-node identity propagation did not converge",
    ))
}

/// `id` with each direct operand equal to `old` replaced by `new`, or `None`
/// when `id` holds no such operand.
fn with_operand(
    id: &CheckedNodeId,
    old: &CheckedNodeId,
    new: &CheckedNodeId,
) -> Option<CheckedNodeId> {
    use quire_contract_model::CheckedNodePathSegment as Segment;
    let mut changed = false;
    let path = id
        .path
        .iter()
        .map(|segment| match segment {
            Segment::Node(operand) if **operand == *old => {
                changed = true;
                Segment::Node(std::sync::Arc::new(new.clone()))
            }
            other => other.clone(),
        })
        .collect();
    changed.then(|| CheckedNodeId {
        package: id.package.clone(),
        path,
        ordinal: id.ordinal,
    })
}

/// The QSpec FR-322 producer identity for a timed node. Declared roots keep
/// their qualified declaration path; derived forms flatten their typed body
/// operands after the family and form path segments.
fn timed_model_node_id(
    ids: &StructuralIds,
    node: &CheckedSemanticNodeV2,
) -> Result<CheckedNodeId, EmitRefusal> {
    if node.recursion_group.is_some() {
        return Err(EmitRefusal::TimedModelWriterInvariant(
            "timed nodes cannot belong to a recursion group",
        ));
    }
    let package = node.node_id.package.as_ref();
    let ordinal = node.node_id.ordinal;
    let key = if let Some(declaration) = &node.declaration {
        let path = declaration
            .qualified_name
            .iter()
            .map(|part| PathSegment::Text(part.to_string()))
            .collect();
        qsl_semantics::check::structural_node_key(package, path, ordinal)
    } else {
        let mut operands = Vec::new();
        timed_body_operands(&node.body, &mut operands)?;
        qsl_semantics::check::derived_node_key(
            package,
            NodeTag::TimedModel,
            &node.semantic_form,
            operands,
            ordinal,
        )
    }
    .map_err(encoding)?;
    Ok(node_id(ids, key))
}

/// Append the literal values, binding names and reference identities of a
/// closed timed body in body order. An IR-validated timed node uses these four
/// term shapes only; any other shape is an internal producer disagreement.
fn timed_body_operands(
    term: &CheckedTerm,
    operands: &mut Vec<PathSegment>,
) -> Result<(), EmitRefusal> {
    match term {
        CheckedTerm::Aggregate { members } => {
            for member in members {
                timed_body_operands(member, operands)?;
            }
        }
        CheckedTerm::Binding { name, value } => {
            operands.push(PathSegment::Text(name.as_ref().to_owned()));
            timed_body_operands(value, operands)?;
        }
        CheckedTerm::Reference { target } => {
            operands.push(PathSegment::Node(admitted_key(target)?));
        }
        CheckedTerm::Literal { type_id, value, .. } => {
            operands.push(PathSegment::Node(admitted_key(type_id)?));
            let text = value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string());
            operands.push(PathSegment::Text(text));
        }
        _ => {
            return Err(EmitRefusal::TimedModelWriterInvariant(
                "timed body has a term outside the closed path grammar",
            ))
        }
    }
    Ok(())
}

/// QSpec FR-322 `model_root`, FR-322-AC-132 and AC-134: append one self-typed
/// root and generated source-map entry per selected domain package, in
/// selection order. The placeholder region is the checked unit's first byte.
fn append_model_roots<'a>(
    ids: &StructuralIds,
    roots: impl Iterator<
        Item = (
            &'a qsl_semantics::model::domain_package::DomainPackageRef,
            NodeKey,
        ),
    >,
    source: &RawSourceRef,
    nodes: &mut Vec<CheckedSemanticNodeV2>,
    source_map: &mut Vec<CheckedSourceMapEntry>,
) {
    for (selection, key) in roots {
        let root = model_root_node(ids, selection, key);
        source_map.push(CheckedSourceMapEntry {
            node_id: root.node_id.clone(),
            role: CheckedOccurrenceRole::Generated,
            ordinal: 0,
            regions: vec![CheckedSourceRegion {
                source: source_artifact(source),
                start: 0,
                end: 1,
            }],
        });
        nodes.push(root);
    }
}

/// The `model_root` node for one selection row, under the key the check
/// derived (`CheckedGraph::model_roots`), the key a refinement over the model
/// names.
fn model_root_node(
    ids: &StructuralIds,
    selection: &qsl_semantics::model::domain_package::DomainPackageRef,
    key: NodeKey,
) -> CheckedSemanticNodeV2 {
    let id = node_id(ids, key);
    CheckedSemanticNodeV2 {
        owner_model: None,
        node_id: id.clone(),
        schema_version: GRAPH_V2.into(),
        node_tag: CheckedNodeTag::Model.as_wire().into(),
        semantic_form: "model_root".into(),
        semantic_type: id,
        dependencies: Vec::new(),
        occurrences: vec![CheckedOccurrence {
            role: CheckedOccurrenceRole::Generated,
            ordinal: 0,
        }],
        recursion_group: None,
        declaration: None,
        owner: Some(CheckedNodeOwner::ModelRoot {
            identity: selection.identity.as_str().into(),
        }),
        body: CheckedTerm::Aggregate {
            members: Vec::new(),
        },
    }
}

/// The unique structural reference targets directly named by a body.
fn body_reference_targets(body: &CheckedTerm) -> Vec<CheckedNodeId> {
    let mut targets = body
        .reference_targets()
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    targets.sort();
    targets.dedup();
    targets
}

/// The checked primitive type used by an existing literal of `kind`.
fn literal_type(nodes: &[CheckedSemanticNodeV2], kind: &str) -> Option<CheckedNodeId> {
    fn visit(term: &CheckedTerm, kind: &str) -> Option<CheckedNodeId> {
        match term {
            CheckedTerm::Literal {
                type_id,
                value_kind,
                ..
            } if value_kind.as_wire() == kind => Some(type_id.clone()),
            CheckedTerm::Aggregate { members } => {
                members.iter().find_map(|member| visit(member, kind))
            }
            CheckedTerm::Binding { value, .. } => visit(value, kind),
            CheckedTerm::Application { arguments, .. } => {
                arguments.iter().find_map(|argument| visit(argument, kind))
            }
            CheckedTerm::ExtensionForm { clauses, .. } => {
                clauses.iter().find_map(|clause| visit(clause, kind))
            }
            _ => None,
        }
    }
    nodes.iter().find_map(|node| visit(&node.body, kind))
}

/// The checked timed models `source` times. Two units may bind one model
/// alias to two packages (FR-480 "Selections and imports"), so a time
/// source names its models through the model node, never the alias.
pub(crate) fn timed_models_of<'g>(
    graph: &'g qsl_semantics::check::CheckedGraph,
    source: &'g qsl_semantics::check::ModelTimeSource,
) -> impl Iterator<Item = &'g qsl_semantics::check::TimedModel> {
    graph
        .timed_models()
        .iter()
        .filter(move |timed| timed.is_timed_by(source))
}

/// Emit a complete admitted package using typed timed models for its timed
/// nodes. The base package supplies the already-checked non-timed graph,
/// lock, source map, capability report and diagnostics; every timed node is
/// rewritten from `models` in its original graph position. The supplied
/// models must cover the base package's timed-node identities exactly.
///
/// Timed-node identities are recomputed from their written bodies, dependent
/// timed references and source-map entries are updated, and the package
/// identity is recomputed before the IR encoder writes the package.
pub fn emit_timed_models(
    base: &CheckedPackageV2,
    models: &[CheckedTimedModel],
    limits: EmitLimits,
) -> Result<Emission, EmitRefusal> {
    let ids = &StructuralIds::default();
    let expected = base
        .resolved_graph()
        .nodes
        .iter()
        .filter(|node| {
            CheckedNodeTag::from_wire(&node.node_tag) == Some(CheckedNodeTag::TimedModel)
        })
        .map(|node| node.node_id.clone())
        .collect::<BTreeSet<_>>();
    let mut timed = BTreeMap::new();
    for model in models {
        let written = model
            .write_nodes()
            .map_err(EmitRefusal::TimedModelWriterInvariant)?;
        for node in written {
            if CheckedNodeTag::from_wire(&node.node_tag) != Some(CheckedNodeTag::TimedModel)
                || timed.insert(node.node_id.clone(), node).is_some()
            {
                return Err(EmitRefusal::TimedModelNodeSetMismatch);
            }
        }
    }
    if expected != timed.keys().cloned().collect() {
        return Err(EmitRefusal::TimedModelNodeSetMismatch);
    }

    let mut nodes = base
        .resolved_graph()
        .nodes
        .iter()
        .map(|node| {
            if CheckedNodeTag::from_wire(&node.node_tag) == Some(CheckedNodeTag::TimedModel) {
                timed
                    .remove(&node.node_id)
                    .ok_or(EmitRefusal::TimedModelNodeSetMismatch)
            } else {
                Ok(node.clone())
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !timed.is_empty() {
        return Err(EmitRefusal::TimedModelNodeSetMismatch);
    }

    let lock = base.lock().clone();
    let mut source_map = base.source_map().to_vec();
    read_index_segments(ids, &mut nodes, &mut source_map)?;
    rekey_timed_model_nodes(ids, &mut nodes, &mut source_map)?;
    let inspection_source_map = inspection_source_map(ids, &source_map)?;
    write_index_segments(
        &mut nodes,
        &mut source_map,
        &mut [],
        &mut [],
        &ForeignNodes::default(),
    )?;
    let operand_rows = write_operand_slots(&mut nodes, limits)?;
    let semantic_graph = CheckedSemanticGraphV2 {
        graph_version: base.graph().graph_version.clone(),
        nodes,
        operand_rows,
    };
    let capability_report = base.capability_report().to_vec();
    let diagnostics = base.diagnostics().clone();
    let operation_effects = base.operation_effects().map(|rows| {
        rows.map(|row| quire_contract_model::CheckedOperationEffectRow {
            document: row.document().into(),
            declaring: row.declaring().clone(),
            operation: row.operation().into(),
            effect: row.effect().clone(),
            provenance: row.provenance().clone(),
        })
        .collect::<Vec<_>>()
    });
    let package = EmittedPackage::new(base.name(), |name| {
        encode_checked_package_v2(
            name,
            &lock,
            &semantic_graph,
            CheckedPackageSourceCorrespondenceRef {
                source_map: &source_map,
                refinement_step_rows: None,
            },
            CheckedPackageSidecarsRef {
                checked_term_data: None,
                temporal_bindings: base.temporal_bindings(),
                temporal_clause_names: base.temporal_clause_names(),
                operation_effects: operation_effects.as_deref(),
                schedulability_claims: None,
            },
            &capability_report,
            &diagnostics,
            limits.canonical(),
        )
        .map_err(EmitRefusal::from)
    })?;
    Ok(Emission {
        package,
        omitted: Vec::new(),
        evidence: own_evidence(&lock),
        capability_report,
        source_map: inspection_source_map,
        selected_operation_member_rows: Vec::new(),
        selected_tuple_declaration_rows: Vec::new(),
        selected_source_unit_rows: Vec::new(),
        selected_model_enum_declaration_rows: Vec::new(),
        selected_model_enum_member_rows: Vec::new(),
        selected_enum_declaration_rows: Vec::new(),
        selected_enum_member_rows: Vec::new(),
        selected_source_union_rows: Vec::new(),
        selected_source_record_rows: Vec::new(),
        selected_source_tuple_rows: Vec::new(),
        selected_model_enum_shape_rows: Vec::new(),
        selected_source_collection_rows: Vec::new(),
        selected_exact_declaration_rows: Vec::new(),
        selected_exact_field_rows: Vec::new(),
        identity_row_selection: identity_rows::IdentityRowSelection::default(),
        equality_producer_identity_rows: Vec::new(),
        equality_fact_witness_rows: Vec::new(),
        wire_nodes: document_nodes(semantic_graph)?,
        temporal_binding_rows: TemporalBindingRows::default(),
        temporal_rows: TemporalRows::default(),
    }
    .with_identity_row_selection())
}

/// [`emit_checked`] under the caller's [`Cancel`] handle, polled once at
/// entry and at every node it writes (FR-276). A cancelled handle stops the
/// emitter with [`EmitRefusal::Cancelled`] and writes no bytes. It always
/// emits, so every node is polled; the emission it returns is remembered for
/// [`emit_checked`].
pub fn emit_checked_with_cancel(
    package: &CheckedPackage,
    cancel: &Cancel,
    limits: EmitLimits,
) -> Result<Emission, EmitRefusal> {
    require_protocol_results_for_refusals(package)?;
    let emission = emit_checked_uncached(package, cancel, limits)?;
    package.remember_emission(&emission);
    Ok(emission)
}

/// Emit `package` as `quire.checked-package/v2` bytes (FR-322). `regions`
/// places each occurrence `check` recorded (see the module doc). The entry
/// point for a package built from forms that carry no spans, whose caller
/// places every occurrence itself.
pub fn emit_package(
    package: &CheckedPackage,
    regions: impl Fn(&Location) -> Option<SourceRegion>,
    limits: EmitLimits,
) -> Result<Emission, EmitRefusal> {
    require_protocol_results_for_refusals(package)?;
    emit_package_inner(
        package,
        regions,
        |_| None,
        &Cancel::new(),
        &ir_decodes,
        &[],
        limits,
    )
}

/// Emit a checked package with caller-placed source regions and explicit
/// temporal clock selections. The table is joined to the checked source and
/// declaration spans before any bytes are encoded.
pub fn emit_package_with_temporal_selections(
    package: &CheckedPackage,
    regions: impl Fn(&Location) -> Option<SourceRegion>,
    selections: &[TemporalClockSelection],
    limits: EmitLimits,
) -> Result<Emission, EmitRefusal> {
    require_protocol_results_for_refusals(package)?;
    emit_package_inner(
        package,
        regions,
        |_| None,
        &Cancel::new(),
        &ir_decodes,
        selections,
        limits,
    )
}

/// As [`emit_package`], but lets a caller force a specific node's wire
/// encoding to fail in place of writing it, in graph order
/// ([`emit_package_inner`]'s `nodes` step): the injectable failure point
/// FR-105-AC-6 requires to prove that node emission is all-or-nothing.
/// No non-test caller exists; this compiles into no non-test
/// build (`#[cfg(any(test, feature = "test-support"))]`), and
/// `emit_package`/`emit_checked` never pass a `fault` that fires. Only this
/// module's own tests call it directly; a dependent crate's tests go through
/// [`emit_checked_with_fault`], the `test-support`-gated public wrapper
/// below, so a private `fn` is enough here.
#[cfg(any(test, feature = "test-support"))]
fn emit_package_with_fault(
    package: &CheckedPackage,
    regions: impl Fn(&Location) -> Option<SourceRegion>,
    fault: impl Fn(&CheckedNodeId) -> Option<EmitRefusal>,
    limits: EmitLimits,
) -> Result<Emission, EmitRefusal> {
    require_protocol_results_for_refusals(package)?;
    emit_package_inner(
        package,
        regions,
        fault,
        &Cancel::new(),
        &ir_decodes,
        &[],
        limits,
    )
}

/// As [`emit_package`], but against an IR whose v2 vocabulary `decodes`
/// describes in place of the one this build links: a node whose tag and form
/// it refuses is omitted with [`OmissionCause::UnsupportedForm`], together
/// with every node that names it (FR-320-AC-2). Only this module's own tests
/// call it.
#[cfg(test)]
fn emit_package_decoding(
    package: &CheckedPackage,
    regions: impl Fn(&Location) -> Option<SourceRegion>,
    decodes: &dyn Fn(CheckedNodeTag, &str) -> bool,
    limits: EmitLimits,
) -> Result<Emission, EmitRefusal> {
    emit_package_inner(
        package,
        regions,
        |_| None,
        &Cancel::new(),
        decodes,
        &[],
        limits,
    )
}

/// As [`emit_checked`], but lets a caller force a specific node's wire
/// encoding to fail, by [`CheckedNodeId`] (FR-105-AC-6). Public only
/// under `test-support`, so it never reaches a shipped build: a dependent
/// crate's own tests (e.g. qsl-replay's frame-node fixtures) enable the
/// feature only from a `[dev-dependencies]` edge, which
/// `no_shipped_dependency_enables_test_support` checks.
#[cfg(feature = "test-support")]
#[doc(hidden)]
pub fn emit_checked_with_fault(
    package: &CheckedPackage,
    fault: impl Fn(&CheckedNodeId) -> Option<EmitRefusal>,
    limits: EmitLimits,
) -> Result<Emission, EmitRefusal> {
    require_protocol_results_for_refusals(package)?;
    let graph = package.graph();
    emit_package_with_fault(package, graph.memoized_regions(), fault, limits)
}

/// The [`CheckedNodeId`] [`emit_checked_with_fault`]'s `fault` closure
/// receives for the graph node `key` names ([`node_id`]'s own conversion),
/// so a dependent crate's test can target one specific node (e.g. a
/// `frame` node found via [`qsl_semantics::check::SemanticNode::key`])
/// without reimplementing the conversion. Public only under `test-support`;
/// see [`emit_checked_with_fault`].
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub fn checked_node_id_of(key: &NodeKey) -> CheckedNodeId {
    qsl_semantics::check::wire_id::structural_id(key)
}

/// Whether IR's v2 vocabulary has the form `semantic_form` under `tag`.
fn ir_decodes(tag: CheckedNodeTag, semantic_form: &str) -> bool {
    CheckedNodeKind::decode(tag, semantic_form).is_some()
}

/// Every node of `graph` read for the wire, ascending by key, and each
/// key's position.
///
/// The graph yields its nodes in key order already ranked, so no two keys
/// are compared here: comparing two keys that share a deep operand chain
/// costs the chain's depth.
struct Candidates<'g> {
    order: Vec<Candidate<'g>>,
    index: HashMap<NodeKey, usize>,
}

impl<'g> Candidates<'g> {
    fn new(order: Vec<Candidate<'g>>) -> Self {
        let index = (0..)
            .zip(&order)
            .map(|(position, candidate)| (candidate.key.clone(), position))
            .collect();
        Self { order, index }
    }

    fn contains(&self, key: &NodeKey) -> bool {
        self.index.contains_key(key)
    }

    fn values(&self) -> std::slice::Iter<'_, Candidate<'g>> {
        self.order.iter()
    }
}

fn candidates_of<'g>(
    ids: &StructuralIds,
    foreign: &ForeignNodes,
    graph: &'g CheckedGraph,
    abstraction: Option<&'g AbstractionNodes>,
    declared_forms: &'g [(SemanticNode, Location)],
) -> Result<Candidates<'g>, EmitRefusal> {
    let mut recorded = recorded_occurrences(graph)?;
    let mut candidates: Vec<Candidate<'g>> = graph
        .semantic_graph()
        .nodes()
        .map(|node| {
            let occurrences = recorded.remove(&node.key()).unwrap_or_default();
            Candidate::of(ids, foreign, node, occurrences)
        })
        .collect::<Result<_, _>>()?;
    let graph_keys: HashSet<NodeKey> = candidates.iter().map(|c| c.key.clone()).collect();
    // QSpec FR-451: the relation node has one `declaration` occurrence per
    // declaration it was read from; the nodes it names that the graph does not
    // hold are `generated`, at the first.
    if let Some(nodes) = abstraction {
        // The few abstraction nodes the graph does not hold go in at their
        // place in key order.
        let mut added: HashSet<NodeKey> = HashSet::new();
        let mut add =
            |node: &'g SemanticNode, occurrences: Recorded<'g>| -> Result<_, EmitRefusal> {
                let key = node.key();
                if graph_keys.contains(&key) || !added.insert(key.clone()) {
                    return Ok(());
                }
                let candidate = Candidate::of(ids, foreign, node, occurrences)?;
                let at = candidates.partition_point(|existing| existing.key < candidate.key);
                candidates.insert(at, candidate);
                Ok(())
            };
        let declared = (0_u64..)
            .zip(nodes.sites())
            .map(|(ordinal, site)| {
                (
                    CheckedOccurrence {
                        role: CheckedOccurrenceRole::Declaration,
                        ordinal,
                    },
                    RecordedLocation::Expression(site),
                )
            })
            .collect();
        add(nodes.relation(), declared)?;
        for node in nodes.support() {
            let generated = nodes
                .sites()
                .first()
                .map(|site| {
                    (
                        CheckedOccurrence {
                            role: CheckedOccurrenceRole::Generated,
                            ordinal: 0,
                        },
                        RecordedLocation::Expression(site),
                    )
                })
                .into_iter()
                .collect();
            add(node, generated)?;
        }
    }
    // QSpec FR-490: a guarantee node has one
    // `declaration` occurrence, at its declaration.
    for (node, location) in declared_forms {
        let declared = vec![(
            CheckedOccurrence {
                role: CheckedOccurrenceRole::Declaration,
                ordinal: 0,
            },
            RecordedLocation::Expression(location),
        )];
        let candidate = Candidate::of(ids, foreign, node, declared)?;
        let at = candidates.partition_point(|existing| existing.key < candidate.key);
        candidates.insert(at, candidate);
    }
    Ok(Candidates::new(candidates))
}

/// The temporal profile the lock selects for the clock-reading control nodes
/// actually written into `nodes` (QSpec FR-464 "Profiles and laws": the lock
/// selects a `temporal_profile` for an `await` or `compensate` node). A
/// protocol whose controls are not emitted, for want of a protocol profile or
/// a refused construct, selects nothing, so the package identity of a unit
/// that writes no such node is unchanged. The lock holds one row per role
/// (FR-322), so the first emitted node's profile in source order is the one
/// selected. QSL-1127 adds `compensate` templates to the same trigger.
fn clock_control_profile(
    graph: &CheckedGraph,
    nodes: &[CheckedSemanticNodeV2],
) -> Option<TemporalProfile> {
    let awaits = graph.protocols().iter().flat_map(|protocol| {
        protocol.awaits.iter().filter_map(move |awaited| {
            let control = protocol.controls.control(awaited.node)?;
            let mut path = vec![protocol.name.as_str(), "run"];
            path.extend(control.path.iter().map(String::as_str));
            Some((path, awaited.profile?))
        })
    });
    let compensations = graph.protocols().iter().flat_map(|protocol| {
        protocol
            .obligations
            .compensations
            .iter()
            .filter_map(move |template| {
                let profile = template.temporal_profile?;
                Some((
                    vec![protocol.name.as_str(), "compensate", template.name.as_str()],
                    profile,
                ))
            })
    });
    awaits
        .chain(compensations)
        .find(|(path, _)| {
            nodes.iter().any(|node| {
                is_control_node(node)
                    || is_compensation_node(node)
                        && node.declaration.as_ref().is_some_and(|declaration| {
                            declaration
                                .qualified_name
                                .iter()
                                .map(AsRef::as_ref)
                                .eq(path.iter().copied())
                        })
            })
        })
        .map(|(_, profile)| profile)
}

/// Whether a written node is a protocol `control`, decoded through IR's closed
/// vocabulary rather than compared by spelling.
fn is_control_node(node: &CheckedSemanticNodeV2) -> bool {
    CheckedNodeTag::from_wire(&node.node_tag)
        .and_then(|tag| CheckedNodeKind::decode(tag, &node.semantic_form))
        == Some(CheckedNodeKind::Protocol(ProtocolForm::Control))
}

/// Whether a written node is a protocol compensation template.
fn is_compensation_node(node: &CheckedSemanticNodeV2) -> bool {
    CheckedNodeTag::from_wire(&node.node_tag)
        .and_then(|tag| CheckedNodeKind::decode(tag, &node.semantic_form))
        == Some(CheckedNodeKind::Protocol(ProtocolForm::Compensation))
}

/// The body shared by [`emit_package`] and [`emit_package_with_fault`].
/// `fault` runs against each node's id, in graph order, just before that
/// node would be written; every production call site is `emit_package`,
/// which passes `|_| None`, so this behaves exactly as `emit_package`
/// always has for every non-test caller.
/// `#[string_edge]`: the admitted timed nominal form selects the unit-node
/// emission branch.
#[string_edge]
fn emit_package_inner(
    package: &CheckedPackage,
    regions: impl Fn(&Location) -> Option<SourceRegion>,
    fault: impl Fn(&CheckedNodeId) -> Option<EmitRefusal>,
    cancel: &Cancel,
    decodes: &dyn Fn(CheckedNodeTag, &str) -> bool,
    temporal_selections: &[TemporalClockSelection],
    limits: EmitLimits,
) -> Result<Emission, EmitRefusal> {
    let ids = &StructuralIds::default();
    let denied = || {
        cancel
            .poll()
            .then(|| cancel.cause())
            .flatten()
            .map(|cause| EmitRefusal::Cancelled { cause })
    };
    if let Some(refusal) = denied() {
        return Err(refusal);
    }
    let graph = package.graph();
    // QSpec FR-490, FR-491: each guarantee and composition declaration is a
    // `correspondence` node beside the graph's own.
    let declared_forms: Vec<(SemanticNode, Location)> = graph
        .implementation_guarantees()
        .iter()
        .map(|guarantee| guarantee.semantic_node())
        .chain(
            graph
                .compositions()
                .iter()
                .map(|composition| composition.semantic_node()),
        )
        .collect();
    // QSpec FR-491: a component's subject and clauses are nodes of a
    // dependency, whose operands this package names by their position there.
    let foreign = ForeignNodes::of(package, limits)?;
    let abstraction = package
        .abstraction()
        .map(|relation| graph.abstraction_nodes(relation))
        .transpose()
        .map_err(EmitRefusal::Abstraction)?;
    if let Some((feature, declaration)) = graph.probabilistic_model().first_declaration() {
        return Err(EmitRefusal::ProbabilisticModelNotEmitted {
            declaration,
            feature,
            // The header layers ascend; the widest is the profile the unit
            // selects its probabilistic declarations under.
            profile: graph.header_layers().last().map(|layer| layer.identity()),
        });
    }
    let candidates = candidates_of(ids, &foreign, graph, abstraction.as_ref(), &declared_forms)?;
    // Nodes the emitter writes beside the graph's own: each model root, and
    // each model-owned time unit node a `duration` window is typed by.
    let external: HashSet<NodeKey> = graph
        .model_roots()
        .map(|(_, key)| key)
        .chain(graph.temporal_clauses().iter().flat_map(|temporal| {
            temporal
                .clause()
                .duration_window_model_nodes()
                .map(|nominal| nominal.key.clone())
        }))
        .collect();
    let selected_models: BTreeSet<&str> = graph
        .model_roots()
        .map(|(selection, _)| selection.identity.as_str())
        .collect();
    let omitted = omissions(
        ids,
        &candidates,
        graph.source(),
        decodes,
        &external,
        &selected_models,
    );
    let omitted_keys = omitted
        .iter()
        .map(|(position, _)| candidates.order[*position].key.clone())
        .collect::<HashSet<_>>();
    let dropped: HashSet<usize> = omitted.iter().map(|(position, _)| *position).collect();
    let omitted: Vec<OmittedNode> = omitted.into_iter().map(|(_, omission)| omission).collect();
    stale_protocol_reference(ids, &candidates, &dropped)?;
    let kept: Vec<&Candidate<'_>> = candidates
        .values()
        .enumerate()
        .filter(|(position, _)| !dropped.contains(position))
        .map(|(_, candidate)| candidate)
        .collect();
    if kept.is_empty() && graph.refinements().is_empty() && graph.task_sets().claims().is_empty() {
        return Err(EmitRefusal::NothingToEmit { omitted });
    }
    let order = graph_order(&kept);
    let emitted_keys: HashSet<NodeKey> =
        kept.iter().map(|candidate| candidate.key.clone()).collect();
    let selected_operation_member_rows: Vec<(
        NodeKey,
        quire_contract_model::SelectedOperationMemberRow,
    )> = graph
        .semantic_graph()
        .selected_operation_members()
        .filter(|(anchor, _)| emitted_keys.contains(*anchor))
        .map(|(anchor, member)| {
            (
                anchor.clone(),
                quire_contract_model::SelectedOperationMemberRow {
                    anchor: node_id(ids, anchor),
                    selected_document: member.selected_document.clone(),
                    declaring_form: member.declaring_form,
                    original_model_identity: member.original_model_identity.clone(),
                    original_operation_identity: member.original_operation_identity.clone(),
                    selected_operation_identity: member.selected_operation_identity.clone(),
                },
            )
        })
        .collect();
    let selected_tuple_declaration_rows: Vec<quire_contract_model::SelectedTupleDeclarationRow> =
        graph
            .selected_tuple_declaration_rows()
            .iter()
            .filter(|row| emitted_keys.contains(&row.node_key))
            .cloned()
            .collect();
    let selected_model_enum_declaration_rows: Vec<
        quire_contract_model::SelectedModelEnumDeclarationRow,
    > = graph
        .semantic_graph()
        .selected_model_enum_declarations()
        .filter(|(key, _)| emitted_keys.contains(*key))
        .map(|(key, selected)| {
            quire_contract_model::SelectedModelEnumDeclarationRow::new(
                node_id(ids, key),
                selected.selected_document.clone(),
                selected.original_identity.clone(),
                key.clone(),
            )
        })
        .collect();
    let selected_model_enum_member_rows: Vec<(
        NodeKey,
        NodeKey,
        quire_contract_model::SelectedModelEnumMemberRow,
    )> = graph
        .semantic_graph()
        .selected_model_enum_members()
        .filter(|(key, _)| emitted_keys.contains(*key))
        .map(|(key, selected)| {
            (
                key.clone(),
                selected.declaration_key.clone(),
                quire_contract_model::SelectedModelEnumMemberRow::new(
                    node_id(ids, key),
                    node_id(ids, &selected.declaration_key),
                    selected.selected_document.clone(),
                    selected.enum_original_identity.clone(),
                    selected.member_original_identity.clone(),
                    selected.case.clone(),
                    key.clone(),
                ),
            )
        })
        .collect();
    let (mut source_map, mut sources) = source_map(&order, &regions)?;
    sources.extend(graph.sources().into_iter().map(source_artifact));
    let wire_nodes = order
        .iter()
        .map(
            |candidate| match denied().or_else(|| fault(&candidate.id)) {
                Some(refusal) => Err(refusal),
                None => candidate.wire_node(ids),
            },
        )
        .collect::<Result<Vec<_>, _>>()?;
    let mut nodes = wire_nodes;
    let mut emitted_refinements = Vec::new();
    let mut step_rows = Vec::new();
    for refinement in graph.refinements() {
        if !refinement.v2_emittable() {
            return Err(EmitRefusal::RefinementNotEmitted);
        }
        let region = graph
            .refinement_declaration_region(refinement)
            .filter(|region| region.start() < region.end())
            .ok_or(EmitRefusal::UnlocatedRefinementDeclaration)?;
        let (node, entries, rows) = refinement_node(ids, graph, refinement, &region)?;
        step_rows.extend(rows);
        let insertion = nodes.partition_point(|existing| existing.node_id < node.node_id);
        nodes.insert(insertion, node.clone());
        source_map.extend(entries);
        sources.insert(source_artifact(region.source()));
        emitted_refinements.push(refinement);
    }
    // FR-322: the sidecar is ordered by owner node id, then ordinal.
    step_rows
        .sort_by(|left, right| (&left.node_id, left.ordinal).cmp(&(&right.node_id, right.ordinal)));
    if !graph.time_sources().is_empty() {
        let text_type = match literal_type(&nodes, "text") {
            Some(id) => id,
            None => node_id(
                ids,
                qsl_semantics::check::builtin_text_type_key(graph.package_name())
                    .map_err(encoding)?,
            ),
        };
        let rational_type = node_id(
            ids,
            qsl_semantics::check::builtin_rational_type_key(graph.package_name())
                .map_err(encoding)?,
        );
        for (index, source) in graph.time_sources().iter().enumerate() {
            let placeholder = |kind: &str| {
                qsl_semantics::check::structural_node_key(
                    graph.package_name(),
                    vec![PathSegment::Text(format!("timed-pending-{kind}-{index}"))],
                    0,
                )
                .map(|key| node_id(ids, key))
                .map_err(encoding)
            };
            let mut model = CheckedTimedModel::from_checked_time_source(
                source,
                placeholder("source")?,
                placeholder("root")?,
                CheckedNodeOwner::Source {
                    authority: graph.source().authority().into(),
                    identity: graph.source().identity().into(),
                },
                text_type.clone(),
                rational_type.clone(),
                &source.model,
            )
            .ok_or(EmitRefusal::TimedModelWriterInvariant(
                "checked tick source has no operation anchor",
            ))?;
            let source_region = SourceRegion::new(
                graph.source().clone(),
                source.span.start as u64,
                source.span.end as u64,
            )
            .map_err(encoding)?;
            let generated = CheckedSourceRegion {
                source: source_artifact(graph.source()),
                start: source_region.start(),
                end: source_region.end(),
            };
            push_scalar_type(&mut nodes, &mut source_map, &text_type, "text", &generated);
            if matches!(
                &source.source,
                qsl_semantics::check::TimeSourceKind::Tick { .. }
            ) {
                push_scalar_type(
                    &mut nodes,
                    &mut source_map,
                    &rational_type,
                    "rational",
                    &generated,
                );
            }
            // QSpec FR-322 "Timed-model nodes": the template `clock_place`
            // of each clock path, which S3 wrote so that a `presence_read`
            // names it, read back from its emitted node.
            let integer_type = node_id(
                ids,
                qsl_semantics::check::builtin_integer_type_key(graph.package_name())
                    .map_err(encoding)?,
            );
            model.clock_places = timed_models_of(graph, source)
                .flat_map(|timed| timed.place_nodes())
                .map(|key| {
                    let id = node_id(ids, key);
                    let written = nodes.iter().find(|node| node.node_id == id).ok_or(
                        EmitRefusal::TimedModelWriterInvariant("a clock place S3 wrote is emitted"),
                    )?;
                    CheckedClockPlace::project(written)
                        .map_err(EmitRefusal::TimedModelWriterInvariant)
                })
                .collect::<Result<_, _>>()?;
            // QSpec FR-322 "Timed-model nodes": one `urgency` node for each
            // `urgent` declaration, in declared order (FR-322 "Aggregate
            // order"), each located at its declaration so that its order key
            // is its own region. S3 refuses a model whose `urgent when`
            // predicate it has not lowered (FR-230-AC-50), so each `when`
            // here names its predicate's expression node. Declarations of
            // equal content are one node (its key is its content) with one
            // `expression` occurrence at each declaration (FR-322: every node
            // has one or more source occurrences); the root lists it once.
            let mut regions_of: BTreeMap<CheckedNodeId, Vec<(u64, u64)>> = BTreeMap::new();
            let mut declared = Vec::new();
            for timed in timed_models_of(graph, source) {
                for (at, urgency) in timed.urgency().iter().enumerate() {
                    let id = placeholder(&format!("urgency-{at}"))?;
                    let metadata = TimedNodeMetadata {
                        node_id: id.clone(),
                        schema_version: GRAPH_V2.into(),
                        semantic_type: id.clone(),
                        dependencies: Vec::new(),
                        occurrences: vec![CheckedOccurrence {
                            role: CheckedOccurrenceRole::Expression,
                            ordinal: 0,
                        }],
                        recursion_group: None,
                        declaration: None,
                        owner: None,
                    };
                    let node = match urgency {
                        qsl_semantics::check::Urgency::When { .. } => CheckedUrgency::When {
                            metadata,
                            predicate: timed
                                .urgency_predicate_node(at)
                                .map(|key| node_id(ids, key))
                                .ok_or(EmitRefusal::TimedModelWriterInvariant(
                                    "an emitted `urgent when` has a lowered predicate",
                                ))?,
                        },
                        qsl_semantics::check::Urgency::Operation { operation_name, .. } => {
                            CheckedUrgency::Operation {
                                metadata,
                                model: model.model.clone(),
                                operation_name: operation_name.clone(),
                            }
                        }
                    };
                    let span = urgency.span();
                    declared.push(((span.start as u64, span.end as u64), node));
                }
            }
            declared.sort_by_key(|(region, _)| *region);
            let mut urgencies: Vec<CheckedUrgency> = Vec::new();
            for (region, node) in declared {
                let same = urgencies
                    .iter_mut()
                    .find(|written| match (&**written, &node) {
                        (
                            CheckedUrgency::When { predicate, .. },
                            CheckedUrgency::When {
                                predicate: other, ..
                            },
                        ) => predicate == other,
                        (
                            CheckedUrgency::Operation {
                                model,
                                operation_name,
                                ..
                            },
                            CheckedUrgency::Operation {
                                model: other_model,
                                operation_name: other_name,
                                ..
                            },
                        ) => model == other_model && operation_name == other_name,
                        _ => false,
                    });
                let (written, metadata) = match same {
                    Some(written) => {
                        let metadata = match written {
                            CheckedUrgency::When { metadata, .. }
                            | CheckedUrgency::Operation { metadata, .. } => metadata,
                        };
                        let ordinal = metadata.occurrences.len() as u64;
                        metadata.occurrences.push(CheckedOccurrence {
                            role: CheckedOccurrenceRole::Expression,
                            ordinal,
                        });
                        (true, metadata.node_id.clone())
                    }
                    None => {
                        let id = match &node {
                            CheckedUrgency::When { metadata, .. }
                            | CheckedUrgency::Operation { metadata, .. } => {
                                metadata.node_id.clone()
                            }
                        };
                        (false, id)
                    }
                };
                regions_of.entry(metadata).or_default().push(region);
                if !written {
                    urgencies.push(node);
                }
            }
            model.urgencies = urgencies;
            // QSpec FR-322 "Clause nodes": the guards, resets, invariants
            // and constraints S3 lowered, each located at its own source.
            if let Some(clauses) = timed_models_of(graph, source)
                .next()
                .and_then(qsl_semantics::check::TimedModel::clause_nodes)
            {
                let places: Vec<CheckedNodeId> = model
                    .clock_places
                    .iter()
                    .map(|place| place.metadata.node_id.clone())
                    .collect();
                let unit = model.source.unit.clone();
                write_clauses(
                    ids,
                    &mut model,
                    clauses,
                    &places,
                    &unit,
                    &placeholder,
                    &mut regions_of,
                )?;
            }
            if !model.clock_places.is_empty() {
                push_scalar_type(
                    &mut nodes,
                    &mut source_map,
                    &integer_type,
                    "integer",
                    &generated,
                );
                model.literal_types.integer = Some(integer_type);
            }
            for nominal in &source.nominal_nodes {
                write_time_nominal(
                    ids,
                    graph,
                    nominal,
                    &source_region,
                    &mut nodes,
                    &mut source_map,
                )?;
            }
            for node in model
                .write_nodes()
                .map_err(EmitRefusal::TimedModelWriterInvariant)?
            {
                // A clock place is S3's node, emitted with the graph.
                if nodes.iter().any(|written| written.node_id == node.node_id) {
                    continue;
                }
                if node.occurrences.is_empty() {
                    return Err(EmitRefusal::TimedModelWriterInvariant(
                        "timed nodes must retain their source occurrence",
                    ));
                }
                let regions = regions_of.get(&node.node_id);
                for (at, occurrence) in node.occurrences.iter().enumerate() {
                    let (start, end) = regions
                        .and_then(|regions| regions.get(at))
                        .copied()
                        .unwrap_or((source_region.start(), source_region.end()));
                    source_map.push(CheckedSourceMapEntry {
                        node_id: node.node_id.clone(),
                        role: occurrence.role.clone(),
                        ordinal: occurrence.ordinal,
                        regions: vec![CheckedSourceRegion {
                            source: source_artifact(graph.source()),
                            start,
                            end,
                        }],
                    });
                }
                nodes.push(node);
            }
        }
        rekey_timed_model_nodes(ids, &mut nodes, &mut source_map)?;
        // QSpec FR-322 "Aggregate order": the lists are written in declared
        // order, which reads the nodes' regions and structure alone, and
        // the nodes whose lists moved are rekeyed.
        if timed_order::order_timed_aggregates(&mut nodes, &source_map)
            .map_err(EmitRefusal::TimedOrder)?
        {
            rekey_timed_model_nodes(ids, &mut nodes, &mut source_map)?;
        }
        #[cfg(test)]
        TIMED_GRAPH.with(|held| *held.borrow_mut() = Some((nodes.clone(), source_map.clone())));
    }
    // QSpec FR-370: a `duration` window typed by a model-owned time unit
    // names that unit's nodes, which S3 does not lower; each is written once,
    // located at the clause that names it.
    for temporal in graph.temporal_clauses() {
        let mut model_nodes = temporal.clause().duration_window_model_nodes().peekable();
        if model_nodes.peek().is_none() {
            continue;
        }
        let location = graph
            .occurrence(temporal.identity(), temporal.claim())
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let region = graph
            .region(location)
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        for nominal in model_nodes {
            write_time_nominal(ids, graph, nominal, &region, &mut nodes, &mut source_map)?;
        }
    }
    append_model_roots(
        ids,
        graph.model_roots(),
        graph.source(),
        &mut nodes,
        &mut source_map,
    );
    let mut schedulability_claim_rows = append_schedulability_claims(
        ids,
        graph,
        graph.task_sets(),
        graph.source(),
        &mut nodes,
        &mut source_map,
    )?;
    let (mut profile_selections, mut laws) = split_laws(&order);
    for refinement in emitted_refinements {
        let selection = CheckedSelection {
            role: CheckedSelectionRole::TemporalProfile,
            definition: CheckedArtifactRef {
                authority: "agent-ix".into(),
                identity: refinement.profile().identity.as_str().into(),
            },
        };
        if !profile_selections.contains(&selection) {
            profile_selections.push(selection);
        }
    }
    // A clock-reading control node needs the lock to select a temporal
    // profile (QSpec FR-464 "Profiles and laws").
    if let Some(profile) = clock_control_profile(graph, &nodes) {
        let selection = CheckedSelection {
            role: CheckedSelectionRole::TemporalProfile,
            definition: CheckedArtifactRef {
                authority: "agent-ix".into(),
                identity: profile.identity().into(),
            },
        };
        if !profile_selections.contains(&selection) {
            profile_selections.push(selection);
        }
    }
    // The IEEE trigger includes a declared float with no IEEE application.
    // Keep the selected, admitted policy even when no node needs its law.
    if let Some(profile) = graph.scope().ieee_profile_selection() {
        laws.push(profile.definition().clone());
    }

    let (edition, definition_selections) =
        catalog_selections(graph.header_layers(), &laws, graph.extension_selections());
    let root = DefinitionRole::Root.identity();
    let lock = CheckedPackageLockV2 {
        sources: sources.into_iter().collect(),
        edition: edition.clone(),
        profile_selections: profile_selections.clone(),
        definition_selections: definition_selections.clone(),
        required_features: vec![root.into()],
        dependency_selections: dependency_selections(package),
        refinement_internals: None,
    };
    let inspection_source_map = inspection_source_map(ids, &source_map)?;
    let emitted_node_ids = write_index_segments(
        &mut nodes,
        &mut source_map,
        &mut step_rows,
        &mut schedulability_claim_rows,
        &foreign,
    )?;
    let WrittenTemporalRows {
        bindings: temporal_bindings,
        names: temporal_clause_names,
        keyed: temporal_binding_rows,
        refs: temporal_clause_refs,
    } = temporal_binding_rows(graph, &emitted_node_ids, &nodes, temporal_selections)?;
    let equality_producer_identity_rows = equality_producer_emission_rows_excluding(
        graph,
        &emitted_node_ids,
        &source_map,
        &omitted_keys,
    )?;
    let equality_fact_witness_rows = equality_fact_witness_rows(
        graph,
        &emitted_node_ids,
        &nodes,
        &equality_producer_identity_rows,
    )?;
    let checked_term_data = checked_term_data(
        graph,
        &emitted_node_ids,
        &selected_model_enum_member_rows,
        &equality_producer_identity_rows,
        &source_map,
    )?;
    let selected_operation_member_rows = selected_operation_member_rows
        .into_iter()
        .map(|(key, mut row)| {
            row.anchor = emitted_node_ids
                .get(&key)
                .cloned()
                .ok_or(EmitRefusal::SourceMapInvariant)?;
            Ok(row)
        })
        .collect::<Result<Vec<_>, EmitRefusal>>()?;
    let selected_tuple_declaration_rows = selected_tuple_declaration_rows
        .into_iter()
        .map(|mut row| {
            row.structural_node = emitted_node_ids
                .get(&row.node_key)
                .cloned()
                .ok_or(EmitRefusal::SourceMapInvariant)?;
            Ok(row)
        })
        .collect::<Result<Vec<_>, EmitRefusal>>()?;
    let selected_model_enum_declaration_rows = selected_model_enum_declaration_rows
        .into_iter()
        .map(|mut row| {
            row.structural_node = emitted_node_ids
                .get(&row.node_key)
                .cloned()
                .ok_or(EmitRefusal::SourceMapInvariant)?;
            Ok(row)
        })
        .collect::<Result<Vec<_>, EmitRefusal>>()?;
    let selected_model_enum_member_rows = selected_model_enum_member_rows
        .into_iter()
        .map(|(key, declaration_key, mut row)| {
            row.structural_node = emitted_node_ids
                .get(&key)
                .cloned()
                .ok_or(EmitRefusal::SourceMapInvariant)?;
            row.declaration_node = emitted_node_ids
                .get(&declaration_key)
                .cloned()
                .ok_or(EmitRefusal::SourceMapInvariant)?;
            Ok(row)
        })
        .collect::<Result<Vec<_>, EmitRefusal>>()?;
    let selected_source_unit_rows = graph
        .source_unit_nodes()
        .filter_map(|unit| {
            let structural_node = emitted_node_ids.get(&unit.key)?.clone();
            let TimeNominalOrigin::Source { owner, .. } = &unit.origin else {
                return None;
            };
            Some(quire_contract_model::SelectedSourceUnitRow::new(
                structural_node,
                owner.authority.as_str(),
                owner.identity.as_str(),
                unit.declaration()?
                    .iter()
                    .map(|segment| segment.as_str().into())
                    .collect(),
                quire_exact::UnitId::declared(unit.key.clone()),
            ))
        })
        .collect();
    let (selected_enum_declaration_rows, selected_enum_member_rows) =
        source_enum_rows(graph, &emitted_node_ids)?;
    let selected_model_enum_shape_rows = model_enum_shape_rows(graph, &emitted_node_ids)?;
    let selected_source_union_rows = source_union_rows(graph, &emitted_node_ids)?;
    let selected_source_record_rows = source_record_rows(graph, &emitted_node_ids)?;
    let selected_source_tuple_rows = source_tuple_rows(graph, &emitted_node_ids)?;
    let selected_source_collection_rows = source_collection_rows(graph, &emitted_node_ids)?;
    let operation_effects = graph
        .operation_effects()
        .filter(|rows| operation_effects_are_carried(rows, &nodes));
    let operand_rows = write_operand_slots(&mut nodes, limits)?;
    let semantic_graph = CheckedSemanticGraphV2 {
        graph_version: GRAPH_V2.into(),
        nodes,
        operand_rows,
    };
    let capability_report = [CheckedCapability {
        feature: root.into(),
        disposition: CheckedCapabilityDisposition::Available,
    }];
    let diagnostics = CheckedDiagnosticsV2 {
        catalog: diagnostics_catalog(),
        entries: Vec::new(),
    };
    let package = EmittedPackage::new(graph.package_name(), |name| {
        encode_checked_package_v2(
            name,
            &lock,
            &semantic_graph,
            CheckedPackageSourceCorrespondenceRef {
                source_map: &source_map,
                refinement_step_rows: (!step_rows.is_empty()).then_some(step_rows.as_slice()),
            },
            CheckedPackageSidecarsRef {
                checked_term_data: checked_term_data.as_ref(),
                temporal_bindings: (!temporal_bindings.is_empty())
                    .then_some(temporal_bindings.as_slice()),
                temporal_clause_names: (!temporal_clause_names.is_empty())
                    .then_some(temporal_clause_names.as_slice()),
                operation_effects,
                schedulability_claims: (!schedulability_claim_rows.is_empty())
                    .then_some(schedulability_claim_rows.as_slice()),
            },
            &capability_report,
            &diagnostics,
            limits.canonical(),
        )
        .map_err(EmitRefusal::from)
    })?;
    let mut evidence = own_evidence(&lock);
    supply_extension_definitions(&mut evidence, graph, &order)?;
    Ok(Emission {
        package,
        omitted,
        evidence,
        capability_report: capability_report.to_vec(),
        source_map: inspection_source_map,
        selected_operation_member_rows,
        selected_tuple_declaration_rows,
        selected_source_unit_rows,
        selected_model_enum_declaration_rows,
        selected_model_enum_member_rows,
        selected_enum_declaration_rows,
        selected_enum_member_rows,
        selected_source_union_rows,
        selected_source_record_rows,
        selected_source_tuple_rows,
        selected_model_enum_shape_rows,
        selected_source_collection_rows,
        selected_exact_declaration_rows: graph.selected_exact_declaration_rows().to_vec(),
        selected_exact_field_rows: graph.selected_exact_field_rows().to_vec(),
        identity_row_selection: identity_rows::IdentityRowSelection::default(),
        equality_producer_identity_rows,
        equality_fact_witness_rows,
        wire_nodes: document_nodes(semantic_graph)?,
        temporal_binding_rows,
        temporal_rows: TemporalRows {
            clauses: temporal_clause_refs,
            profile_selections: lock.profile_selections,
            bindings: temporal_bindings,
        },
    }
    .with_identity_row_selection())
}

/// FR-092 "Source enum identity rows": one declaration row per emitted
/// source-owned enum and one member row per emitted case, joined to the final
/// structural ids. The declaration row holds the admitted binding's checked
/// `EnumShape` and each member row the shape's variant at its case's rank,
/// read from the binding and never rebuilt from keys. A declaration bound
/// twice is one row; an emitted declaration whose case node is not emitted
/// is a writer fault.
pub(crate) fn source_enum_rows(
    graph: &CheckedGraph,
    emitted: &HashMap<NodeKey, CheckedNodeId>,
) -> Result<
    (
        Vec<quire_contract_model::SelectedEnumDeclarationRow>,
        Vec<quire_contract_model::SelectedEnumMemberRow>,
    ),
    EmitRefusal,
> {
    let semantic = graph.semantic_graph();
    let mut declarations = std::collections::BTreeMap::new();
    let mut members = std::collections::BTreeMap::new();
    for binding in graph.scope().enums() {
        let key = binding.declaration.key();
        let Some(structural_node) = emitted.get(&key) else {
            continue;
        };
        if declarations.contains_key(&key) {
            continue;
        }
        let declaration = semantic
            .source_enum_declaration(&key)
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let shape = binding.shape();
        let variants: Vec<quire_exact::VariantId> = shape.variants().cloned().collect();
        if variants.len() != binding.members.len() {
            return Err(EmitRefusal::SourceMapInvariant);
        }
        for (member, variant) in binding.members.iter().zip(variants) {
            let member_key = member.member();
            let node = emitted
                .get(&member_key)
                .ok_or(EmitRefusal::SourceMapInvariant)?;
            members.insert(
                member_key.clone(),
                quire_contract_model::SelectedEnumMemberRow::new(
                    node.clone(),
                    declaration.owner.authority(),
                    declaration.owner.identity(),
                    structural_node.clone(),
                    declaration.qualified_declaration.clone(),
                    member.case(),
                    member_key,
                    variant,
                ),
            );
        }
        declarations.insert(
            key.clone(),
            quire_contract_model::SelectedEnumDeclarationRow::new(
                structural_node.clone(),
                declaration.owner.authority(),
                declaration.owner.identity(),
                declaration.qualified_declaration.clone(),
                key,
                shape,
            ),
        );
    }
    Ok((
        declarations.into_values().collect(),
        members.into_values().collect(),
    ))
}

/// FR-092 "Source union identity rows": one row per emitted source-owned
/// union, joined to its final structural id. Each sealed member is the one
/// the check minted (read from the checked type environment, never rebuilt)
/// and carries the final ids of its payload type nodes in position order,
/// read from the union node's own member bindings. A union the unit declares
/// but no emitted node reaches has no row; an emitted union whose members or
/// payload nodes cannot be joined is a writer fault.
fn source_union_rows(
    graph: &CheckedGraph,
    emitted: &HashMap<NodeKey, CheckedNodeId>,
) -> Result<Vec<quire_contract_model::SelectedSourceUnionRow>, EmitRefusal> {
    use quire_semantic_value::declaration::CompositeShape;

    let mut rows = Vec::new();
    for declaration in graph.scope().types().composites() {
        let CompositeShape::Union(declared) = declaration.shape() else {
            continue;
        };
        let sealed = declared
            .iter()
            .map(|member| member.member().ok_or(EmitRefusal::SourceMapInvariant))
            .collect::<Result<Vec<_>, _>>()?;
        let Some(settled_key) = sealed.first().map(|member| member.declaration().clone()) else {
            continue;
        };
        let (Some(node), Some(structural_node)) = (
            graph.semantic_graph().node(settled_key.clone()),
            emitted.get(&settled_key),
        ) else {
            continue;
        };
        let (Some(Owner::Source(owner)), Some(qualified)) = (node.owner(), node.declaration())
        else {
            continue;
        };
        let payloads = node
            .union_member_payload_nodes()
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let members = union_member_rows(declared, &sealed, payloads, emitted)?;
        rows.push(quire_contract_model::SelectedSourceUnionRow::new(
            structural_node.clone(),
            owner.authority(),
            owner.identity(),
            qualified
                .iter()
                .map(|segment| segment.as_str().into())
                .collect(),
            declaration.key(),
            settled_key,
            members,
        ));
    }
    Ok(rows)
}

/// FR-092 "Source record identity rows": one row per emitted source-owned
/// record, joined to its final structural id. Declared names, presence and
/// field types come from the checked declaration; binding names and type
/// nodes come from the emitted record node; the two are joined by position.
fn source_record_rows(
    graph: &CheckedGraph,
    emitted: &HashMap<NodeKey, CheckedNodeId>,
) -> Result<Vec<SelectedSourceRecordRow>, EmitRefusal> {
    source_record_rows_over(graph, emitted, graph.scope().types().composites())
}

/// [`source_record_rows`] over the record declarations `declarations`. An
/// emitted source record node that no declaration of `declarations` joins to,
/// a declared record whose node is absent from the graph or is a source
/// composite without an owner or declaration, and a binding list or type node
/// that cannot be joined are writer faults; a record node that was not
/// emitted, and a model-owned record, have no row.
fn source_record_rows_over<'d>(
    graph: &CheckedGraph,
    emitted: &HashMap<NodeKey, CheckedNodeId>,
    declarations: impl Iterator<Item = &'d quire_semantic_value::declaration::CompositeDeclaration>,
) -> Result<Vec<SelectedSourceRecordRow>, EmitRefusal> {
    use quire_semantic_value::declaration::CompositeShape;

    let semantic = graph.semantic_graph();
    let mut rows = Vec::new();
    for declaration in declarations {
        let CompositeShape::Record(declared) = declaration.shape() else {
            continue;
        };
        let Some(node_key) = semantic.composite_node(&declaration.key()) else {
            continue;
        };
        let node = semantic
            .node(node_key.clone())
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        if node.node_tag() != NodeTag::CompositeType {
            // A selected model record uses its model-owned node.
            continue;
        }
        let (Some(Owner::Source(owner)), Some(qualified)) = (node.owner(), node.declaration())
        else {
            return Err(EmitRefusal::SourceMapInvariant);
        };
        let Some(structural_node) = emitted.get(node_key) else {
            continue;
        };
        let bindings = node
            .record_member_nodes()
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let members = record_member_rows(declared, bindings, emitted)?;
        rows.push((
            node_key.clone(),
            SelectedSourceRecordRow {
                structural_node: structural_node.clone(),
                source_authority: owner.authority().into(),
                source_identity: owner.identity().into(),
                qualified_declaration: qualified
                    .iter()
                    .map(|segment| segment.as_str().into())
                    .collect(),
                record_key: declaration.key(),
                members,
            },
        ));
    }
    let rowed: std::collections::BTreeSet<&NodeKey> = rows.iter().map(|(key, _)| key).collect();
    let unmatched = semantic.nodes().any(|node| {
        node.node_tag() == NodeTag::CompositeType
            && matches!(node.owner(), Some(Owner::Source(_)))
            && node
                .record_member_nodes()
                .is_some_and(|bindings| !bindings.is_empty())
            && emitted.contains_key(&node.key())
            && !rowed.contains(&node.key())
    });
    if unmatched {
        return Err(EmitRefusal::SourceMapInvariant);
    }
    rows.sort_by(|(left, _), (right, _)| left.cmp(right));
    Ok(rows.into_iter().map(|(_, row)| row).collect())
}

/// FR-092 "Source tuple identity rows": one row per emitted source-owned
/// tuple, joined to its final structural id. Positions come from the checked
/// declaration (type) and the lowered tuple node (type node), joined by
/// ordinal.
fn source_tuple_rows(
    graph: &CheckedGraph,
    emitted: &HashMap<NodeKey, CheckedNodeId>,
) -> Result<Vec<quire_contract_model::SelectedSourceTupleRow>, EmitRefusal> {
    source_tuple_rows_over(graph, emitted, graph.scope().types().composites())
}

/// [`source_tuple_rows`] over the declarations `declarations`. An emitted
/// source tuple node that no declaration of `declarations` joins to, a
/// declared tuple whose node is absent or is a source composite without an
/// owner or declaration, and a position list or type node that cannot be
/// joined are writer faults; a tuple node that was not emitted has no row.
fn source_tuple_rows_over<'d>(
    graph: &CheckedGraph,
    emitted: &HashMap<NodeKey, CheckedNodeId>,
    declarations: impl Iterator<Item = &'d quire_semantic_value::declaration::CompositeDeclaration>,
) -> Result<Vec<quire_contract_model::SelectedSourceTupleRow>, EmitRefusal> {
    use quire_semantic_value::declaration::CompositeShape;

    let semantic = graph.semantic_graph();
    let mut rows = Vec::new();
    for declaration in declarations {
        let CompositeShape::Tuple(declared) = declaration.shape() else {
            continue;
        };
        let Some(node_key) = semantic.composite_node(&declaration.key()) else {
            continue;
        };
        let node = semantic
            .node(node_key.clone())
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        if node.node_tag() != NodeTag::CompositeType {
            continue;
        }
        let (Some(Owner::Source(owner)), Some(qualified)) = (node.owner(), node.declaration())
        else {
            return Err(EmitRefusal::SourceMapInvariant);
        };
        let Some(structural_node) = emitted.get(node_key) else {
            continue;
        };
        let nodes = node
            .tuple_position_nodes()
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let positions = tuple_position_rows(declared, nodes, emitted)?;
        rows.push((
            node_key.clone(),
            quire_contract_model::SelectedSourceTupleRow {
                structural_node: structural_node.clone(),
                source_authority: owner.authority().into(),
                source_identity: owner.identity().into(),
                qualified_declaration: qualified
                    .iter()
                    .map(|segment| segment.as_str().into())
                    .collect(),
                type_handle: declaration.key(),
                positions,
            },
        ));
    }
    let rowed: std::collections::BTreeSet<&NodeKey> = rows.iter().map(|(key, _)| key).collect();
    let unmatched = semantic.nodes().any(|node| {
        node.node_tag() == NodeTag::CompositeType
            && matches!(node.owner(), Some(Owner::Source(_)))
            && node
                .tuple_position_nodes()
                .is_some_and(|nodes| !nodes.is_empty())
            && emitted.contains_key(&node.key())
            && !rowed.contains(&node.key())
    });
    if unmatched {
        return Err(EmitRefusal::SourceMapInvariant);
    }
    rows.sort_by(|(left, _), (right, _)| left.cmp(right));
    Ok(rows.into_iter().map(|(_, row)| row).collect())
}

/// The position entries of one tuple row, joining the declared position
/// types to the tuple node's references by ordinal. A reference list that
/// disagrees with the declaration in count, or names a type node that was not
/// emitted, is a writer fault.
fn tuple_position_rows(
    declared: &[quire_exact::ValueType],
    nodes: Vec<NodeKey>,
    emitted: &HashMap<NodeKey, CheckedNodeId>,
) -> Result<Vec<quire_contract_model::SelectedTuplePositionRow>, EmitRefusal> {
    if nodes.len() != declared.len() {
        return Err(EmitRefusal::SourceMapInvariant);
    }
    (0_u64..)
        .zip(declared.iter().zip(nodes))
        .map(|(ordinal, (value_type, key))| {
            Ok(quire_contract_model::SelectedTuplePositionRow {
                ordinal,
                type_node: emitted
                    .get(&key)
                    .cloned()
                    .ok_or(EmitRefusal::SourceMapInvariant)?,
                value_type: value_type.clone(),
            })
        })
        .collect()
}

/// FR-092 "Model enum shape rows": one row per emitted model-owned enum
/// declaration, holding the checker's `EnumShape` and each case's variant,
/// joined to the final declaration and case node ids, cases in the shape's
/// canonical order. A declaration whose case set differs from its shape, a
/// case whose variant is not at its rank, or a case node that was not emitted
/// is a writer fault.
fn model_enum_shape_rows(
    graph: &CheckedGraph,
    emitted: &HashMap<NodeKey, CheckedNodeId>,
) -> Result<Vec<SelectedModelEnumShapeRow>, EmitRefusal> {
    let semantic = graph.semantic_graph();
    let mut rows = Vec::new();
    for (key, declaration) in semantic.selected_model_enum_declarations() {
        let Some(structural_node) = emitted.get(key) else {
            continue;
        };
        let variants: Vec<quire_exact::VariantId> = declaration.shape.variants().cloned().collect();
        let mut members: Vec<_> = semantic
            .selected_model_enum_members()
            .filter(|(_, member)| &member.declaration_key == key)
            .collect();
        if members.len() != variants.len() {
            return Err(EmitRefusal::SourceMapInvariant);
        }
        members.sort_by_key(|(_, member)| {
            variants
                .iter()
                .position(|variant| variant == &member.variant)
                .unwrap_or(usize::MAX)
        });
        let cases = members
            .into_iter()
            .zip(&variants)
            .map(|((member_key, member), variant)| {
                if &member.variant != variant {
                    return Err(EmitRefusal::SourceMapInvariant);
                }
                Ok(SelectedModelEnumShapeCaseRow {
                    case: member.case.clone(),
                    structural_node: emitted
                        .get(member_key)
                        .cloned()
                        .ok_or(EmitRefusal::SourceMapInvariant)?,
                    member_key: member_key.clone(),
                    member_original_identity: member.member_original_identity.clone(),
                    variant: variant.clone(),
                })
            })
            .collect::<Result<Vec<_>, EmitRefusal>>()?;
        rows.push(SelectedModelEnumShapeRow {
            structural_node: structural_node.clone(),
            node_key: key.clone(),
            selected_document: declaration.selected_document.clone(),
            original_identity: declaration.original_identity.clone(),
            shape: declaration.shape.clone(),
            cases,
        });
    }
    Ok(rows)
}

/// FR-092 "Source collection rows": one row per emitted collection type node,
/// pairing it with the checker's collection type and the emitted element
/// node. A collection whose node is not emitted has no row.
fn source_collection_rows(
    graph: &CheckedGraph,
    emitted: &HashMap<NodeKey, CheckedNodeId>,
) -> Result<Vec<SelectedSourceCollectionRow>, EmitRefusal> {
    let semantic = graph.semantic_graph();
    semantic
        .source_collections()
        .filter_map(|(key, collection)| Some((collection, emitted.get(key)?)))
        .map(|(collection, structural_node)| {
            // A collection node's body is one bare reference, its element; a
            // map's is its `key` then `value` bindings.
            let base =
                semantic
                    .node(collection.base.clone())
                    .and_then(|node| match collection.key {
                        None => node.tuple_position_nodes(),
                        Some(_) => node.map_operand_nodes(),
                    });
            collection_row(structural_node, collection, base, emitted)
        })
        .collect()
}

/// One collection row joined to `emitted`. A value type that is neither a
/// collection nor a map, a key node on a collection or none on a map, a base
/// node that does not reference exactly the element (a map's: the key, then
/// the value), and a node that was not emitted are writer faults.
fn collection_row(
    structural_node: &CheckedNodeId,
    collection: &qsl_semantics::check::SourceCollection,
    base_references: Option<Vec<NodeKey>>,
    emitted: &HashMap<NodeKey, CheckedNodeId>,
) -> Result<SelectedSourceCollectionRow, EmitRefusal> {
    let expected: Vec<NodeKey> = match (&collection.value_type, &collection.key) {
        (quire_exact::ValueType::Collection(_), None) => vec![collection.element.clone()],
        (quire_exact::ValueType::Map(_), Some(key)) => {
            vec![key.clone(), collection.element.clone()]
        }
        _ => return Err(EmitRefusal::SourceMapInvariant),
    };
    if base_references.as_deref() != Some(expected.as_slice()) {
        return Err(EmitRefusal::SourceMapInvariant);
    }
    let node = |key: &NodeKey| {
        emitted
            .get(key)
            .cloned()
            .ok_or(EmitRefusal::SourceMapInvariant)
    };
    Ok(SelectedSourceCollectionRow {
        structural_node: structural_node.clone(),
        base_node: node(&collection.base)?,
        element_node: node(&collection.element)?,
        key_node: collection.key.as_ref().map(node).transpose()?,
        value_type: collection.value_type.clone(),
    })
}

/// The member entries of one record row, joining the declared fields to the
/// record node's bindings position by position and keeping both the declared
/// name and the node's binding name. A binding list that disagrees with the
/// declaration in member count, or that permutes the declared names, or names a type node that was not emitted, is
/// a writer fault.
fn record_member_rows(
    declared: &[quire_semantic_value::declaration::FieldDeclaration],
    bindings: Vec<(&str, NodeKey)>,
    emitted: &HashMap<NodeKey, CheckedNodeId>,
) -> Result<Vec<SelectedSourceRecordMemberRow>, EmitRefusal> {
    if bindings.len() != declared.len() {
        return Err(EmitRefusal::SourceMapInvariant);
    }
    // A rename to a name outside the declared set is legal; the same names in
    // another order are not, since position would then pair a declared field
    // with another field's binding.
    let mut declared_names: Vec<&str> = declared.iter().map(|field| field.name()).collect();
    let mut binding_names: Vec<&str> = bindings.iter().map(|(name, _)| *name).collect();
    let in_declared_order = declared_names.iter().eq(binding_names.iter());
    declared_names.sort_unstable();
    binding_names.sort_unstable();
    if declared_names == binding_names && !in_declared_order {
        return Err(EmitRefusal::SourceMapInvariant);
    }
    declared
        .iter()
        .zip(bindings)
        .map(|(field, (name, type_key))| {
            Ok(SelectedSourceRecordMemberRow {
                original_member_identity: field.name().into(),
                admitted_name: name.into(),
                presence: field.presence(),
                type_node: emitted
                    .get(&type_key)
                    .cloned()
                    .ok_or(EmitRefusal::SourceMapInvariant)?,
                value_type: field.value_type().clone(),
            })
        })
        .collect()
}

/// The member entries of one union row: the sealed members, the payload node
/// ids and the payload types, joined position by position. A payload list
/// that disagrees with the declaration in member count, member name or
/// payload count, or names a node that was not emitted, is a writer fault.
fn union_member_rows(
    declared: &[quire_semantic_value::declaration::UnionMemberDeclaration],
    sealed: &[&quire_exact::UnionMember],
    payloads: Vec<(&str, Vec<NodeKey>)>,
    emitted: &HashMap<NodeKey, CheckedNodeId>,
) -> Result<Vec<quire_contract_model::SelectedUnionMemberRow>, EmitRefusal> {
    if payloads.len() != declared.len() {
        return Err(EmitRefusal::SourceMapInvariant);
    }
    declared
        .iter()
        .zip(sealed)
        .zip(payloads)
        .map(|((declared, sealed), (name, payload))| {
            if name != sealed.identifier() || payload.len() != declared.positions().len() {
                return Err(EmitRefusal::SourceMapInvariant);
            }
            let payload_nodes = payload
                .iter()
                .map(|key| {
                    emitted
                        .get(key)
                        .cloned()
                        .ok_or(EmitRefusal::SourceMapInvariant)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(quire_contract_model::SelectedUnionMemberRow::new(
                (*sealed).clone(),
                payload_nodes,
                declared.positions().to_vec(),
            ))
        })
        .collect()
}

/// The temporal profile `clock`'s alternative configures.
fn clock_profile(
    clock: &quire_contract_model::ClockConfiguration,
) -> qsl_semantics::check::TemporalProfile {
    use qsl_semantics::check::TemporalProfile;
    use quire_contract_model::ClockConfiguration;

    match clock {
        ClockConfiguration::EventPosition { .. } => TemporalProfile::EventPositionFalseExtension,
        ClockConfiguration::FixedSample { .. } => TemporalProfile::FixedSampleFalseExtension,
        ClockConfiguration::TimestampedEvent { .. } => {
            TemporalProfile::TimestampedEventFiniteWindow
        }
    }
}

/// The definition a lock's `profile_selections` row names for `profile`.
fn profile_definition(
    profile: qsl_semantics::check::TemporalProfile,
) -> quire_contract_model::CheckedArtifactRef {
    quire_contract_model::CheckedArtifactRef {
        authority: "agent-ix".into(),
        identity: profile.identity().into(),
    }
}

/// Refuses a selection whose clock alternative is not the declaration's
/// profile's, or whose selected definition identity is not the one the
/// declaration's `temporal_profile` law names.
fn check_selected_definition(
    source_ref: &RawSourceRef,
    span: qsl_foundation::Span,
    profile: qsl_semantics::check::TemporalProfile,
    selection: &TemporalClockSelection,
) -> Result<(), EmitRefusal> {
    if clock_profile(&selection.clock) != profile {
        return Err(EmitRefusal::TemporalClockProfileMismatch {
            source_ref: source_ref.clone(),
            span,
            profile: profile.identity(),
        });
    }
    if selection.definition != profile_definition(profile) {
        return Err(EmitRefusal::InvalidTemporalClockValue {
            source_ref: source_ref.clone(),
            span,
            field: "definition",
        });
    }
    Ok(())
}

/// The wrapper node a hyper clause's `body` member references.
fn hyper_wrapper(graph: &CheckedGraph, hyper: &NodeKey) -> Result<NodeKey, EmitRefusal> {
    let hyper_node = graph
        .semantic_graph()
        .node(hyper.clone())
        .ok_or(EmitRefusal::SourceMapInvariant)?;
    let BodyTerm::Aggregate(aggregate) = hyper_node.body() else {
        return Err(EmitRefusal::SourceMapInvariant);
    };
    aggregate
        .members
        .iter()
        .find_map(|member| {
            let MemberTerm::Binding(binding) = member else {
                return None;
            };
            if !is_body_binding(&binding.name) {
                return None;
            }
            let BindingValue::Leaf(LeafTerm::Reference { target }) = &binding.value else {
                return None;
            };
            target.admitted().cloned()
        })
        .ok_or(EmitRefusal::SourceMapInvariant)
}

fn canonical_clock_integer(value: &str) -> Option<i64> {
    let parsed = value.parse::<i64>().ok()?;
    (parsed.to_string() == value).then_some(parsed)
}

fn canonical_clock_number(number: &quire_contract_model::ClockExactNumber, positive: bool) -> bool {
    use quire_contract_model::ClockExactNumber;

    match number {
        ClockExactNumber::Integer { decimal } => {
            canonical_clock_integer(decimal).is_some_and(|value| !positive || value > 0)
        }
        ClockExactNumber::Rational {
            numerator,
            denominator,
        } => {
            let Some(numerator) = canonical_clock_integer(numerator) else {
                return false;
            };
            let Some(denominator) = canonical_clock_integer(denominator) else {
                return false;
            };
            if denominator <= 0 || (positive && numerator <= 0) {
                return false;
            }
            let mut left = numerator.unsigned_abs();
            let mut right = denominator as u64;
            while right != 0 {
                (left, right) = (right, left % right);
            }
            left == 1
        }
    }
}

fn temporal_clock_invalid_field(
    clock_name: &str,
    clock: &quire_contract_model::ClockConfiguration,
) -> Option<&'static str> {
    use quire_contract_model::ClockConfiguration;

    let valid_name = |value: &str| qsl_foundation::selection::valid_selection_identity(value);
    if !valid_name(clock_name) {
        return Some("clock_name");
    }
    match clock {
        ClockConfiguration::EventPosition { sequence_authority } => {
            (!valid_name(sequence_authority)).then_some("sequence_authority")
        }
        ClockConfiguration::FixedSample {
            epoch,
            period,
            unit,
        } => {
            if !canonical_clock_number(epoch, false) {
                Some("epoch")
            } else if !canonical_clock_number(period, true) {
                Some("period")
            } else if !valid_name(unit) {
                Some("unit")
            } else {
                None
            }
        }
        ClockConfiguration::TimestampedEvent { timestamp_unit } => {
            (!valid_name(timestamp_unit)).then_some("timestamp_unit")
        }
    }
}

/// Write the nominal dimension or unit node `nominal` a timed construct
/// names, unless the graph already holds it: a source node located at its
/// declaration, a model-owned node with an empty body, no declaration and a
/// `generated` occurrence at `generated` (FR-322 "Model-owned members").
fn write_time_nominal(
    ids: &StructuralIds,
    graph: &CheckedGraph,
    nominal: &qsl_semantics::check::TimeNominalNode,
    generated: &SourceRegion,
    nodes: &mut Vec<CheckedSemanticNodeV2>,
    source_map: &mut Vec<CheckedSourceMapEntry>,
) -> Result<(), EmitRefusal> {
    let id = node_id(ids, &nominal.key);
    if nodes.iter().any(|node| node.node_id == id) {
        return Ok(());
    }
    let declaration = nominal.declaration().map(|segments| CheckedDeclaration {
        qualified_name: segments
            .iter()
            .map(|segment| segment.as_str().into())
            .collect(),
    });
    let (owner, source_owned) = match &nominal.origin {
        TimeNominalOrigin::Source { owner, .. } => (
            CheckedNodeOwner::Source {
                authority: owner.authority.as_str().into(),
                identity: owner.identity.as_str().into(),
            },
            true,
        ),
        TimeNominalOrigin::Model(subject) => (
            CheckedNodeOwner::Model {
                identity: subject.identity.as_str().into(),
                node: subject.node.as_str().into(),
            },
            false,
        ),
    };
    let body = if source_owned {
        match (nominal.form, nominal.nominal_payload()) {
            (TimeNominalForm::Dimension, Some(payload @ NominalPayload::Dimension(_)))
            | (TimeNominalForm::Unit, Some(payload @ NominalPayload::Unit(_))) => {
                nominal_term(ids, &nominal.key, payload)?
            }
            _ => {
                return Err(EmitRefusal::TimedModelWriterInvariant(
                    "a source-owned timed nominal must carry its typed body",
                ));
            }
        }
    } else {
        CheckedTerm::Aggregate {
            members: Vec::new(),
        }
    };
    let semantic_type = if nominal.form == TimeNominalForm::Unit {
        nominal
            .dependencies
            .first()
            .map(|key| node_id(ids, key))
            .ok_or(EmitRefusal::TimedModelWriterInvariant(
                "a unit node must name its dimension",
            ))?
    } else {
        id.clone()
    };
    // A model-owned node carries no declaration, so it is
    // `generated` (like a `model_root`); every node has at
    // least one occurrence.
    let occurrence = CheckedOccurrence {
        role: if source_owned {
            CheckedOccurrenceRole::Declaration
        } else {
            CheckedOccurrenceRole::Generated
        },
        ordinal: 0,
    };
    // FR-322: a source nominal's occurrence is located at its
    // own declaration; a model-owned node has no source form,
    // so its `generated` occurrence takes `generated`, the
    // region of the construct that names it.
    let region = match nominal.name_span() {
        Some(span) => SourceRegion::new(graph.source().clone(), span.start as u64, span.end as u64)
            .map_err(encoding)?,
        None => generated.clone(),
    };
    source_map.push(CheckedSourceMapEntry {
        node_id: id.clone(),
        role: occurrence.role.clone(),
        ordinal: occurrence.ordinal,
        regions: vec![CheckedSourceRegion {
            source: source_artifact(region.source()),
            start: region.start(),
            end: region.end(),
        }],
    });
    nodes.push(CheckedSemanticNodeV2 {
        owner_model: None,
        node_id: id.clone(),
        schema_version: GRAPH_V2.into(),
        node_tag: CheckedNodeTag::ScalarType.as_wire().into(),
        semantic_form: nominal.form.as_str().into(),
        semantic_type,
        dependencies: nominal
            .dependencies
            .iter()
            .map(|key| node_id(ids, key))
            .collect(),
        occurrences: vec![occurrence],
        recursion_group: None,
        declaration,
        owner: Some(owner),
        body,
    });
    Ok(())
}

fn validate_temporal_clock(
    source_ref: &RawSourceRef,
    span: qsl_foundation::Span,
    clock_name: &str,
    clock: &quire_contract_model::ClockConfiguration,
) -> Result<(), EmitRefusal> {
    if let Some(field) = temporal_clock_invalid_field(clock_name, clock) {
        return Err(EmitRefusal::InvalidTemporalClockValue {
            source_ref: source_ref.clone(),
            span,
            field,
        });
    }
    Ok(())
}

/// The temporal rows one emission writes.
struct WrittenTemporalRows {
    bindings: Vec<quire_contract_model::TemporalBinding>,
    names: Vec<quire_contract_model::TemporalClauseName>,
    keyed: TemporalBindingRows,
    refs: Vec<TemporalClauseRef>,
}

fn temporal_binding_rows(
    graph: &CheckedGraph,
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
    nodes: &[CheckedSemanticNodeV2],
    selections: &[TemporalClockSelection],
) -> Result<WrittenTemporalRows, EmitRefusal> {
    use qsl_semantics::check::TemporalProfile;

    let mut selected = BTreeMap::new();
    for selection in selections {
        let key = (
            selection.source.clone(),
            selection.declaration_span.start,
            selection.declaration_span.end,
        );
        if selected.insert(key, selection).is_some() {
            return Err(EmitRefusal::DuplicateTemporalClockSelection {
                source_ref: selection.source.clone(),
                span: selection.declaration_span,
            });
        }
    }

    let mut rows = Vec::new();
    let mut keyed = TemporalBindingRows::default();
    let mut names = Vec::new();
    for site in graph.hybrid_claim_sites() {
        names.push(quire_contract_model::TemporalClauseName {
            declaration: emitted_node_ids
                .get(&site.identity)
                .cloned()
                .ok_or(EmitRefusal::SourceMapInvariant)?,
            occurrence: site.claim.ordinal(),
            name: site.name.as_str().into(),
        });
    }
    let mut refs = Vec::new();
    let mut missing = Vec::new();
    for (position, temporal) in graph.temporal_clauses().iter().enumerate() {
        let clause = temporal.clause();
        let profile = clause.profile();
        let declaration = emitted_node_ids
            .get(&temporal.identity())
            .cloned()
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        keyed.declarations.push((
            temporal.identity(),
            temporal.claim().ordinal(),
            declaration.clone(),
        ));
        refs.push(TemporalClauseRef {
            declaration: declaration.clone(),
            occurrence: temporal.claim().ordinal(),
            clause: position,
        });
        names.push(quire_contract_model::TemporalClauseName {
            declaration,
            occurrence: temporal.claim().ordinal(),
            name: clause.name().into(),
        });
        if !matches!(
            profile,
            TemporalProfile::EventPositionFalseExtension
                | TemporalProfile::FixedSampleFalseExtension
                | TemporalProfile::TimestampedEventFiniteWindow
        ) {
            continue;
        }
        let location = graph
            .occurrence(temporal.identity(), temporal.claim())
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let region = graph
            .region(location)
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let source = region.source().clone();
        let span = clause.span();
        let key = (source.clone(), span.start, span.end);
        let Some(selection) = selected.remove(&key) else {
            missing.push((source, span));
            continue;
        };
        check_selected_definition(&source, span, profile, selection)?;
        let clock = &selection.clock;
        validate_temporal_clock(&source, span, clause.clock(), clock)?;
        // QSpec FR-370 "Window aggregate atom": a `duration` window's bounds
        // are quantities of the clause's clock unit, the `scalar_type`/`unit`
        // node named by the selected `timestamp_unit` (FR-337 "Aggregate
        // formula emission").
        if let quire_contract_model::ClockConfiguration::TimestampedEvent { timestamp_unit } = clock
        {
            if clause
                .duration_window_units()
                .any(|unit| !qsl_semantics::check::is_clock_unit_node(unit, timestamp_unit))
            {
                return Err(EmitRefusal::InvalidTemporalClockValue {
                    source_ref: source.clone(),
                    span,
                    field: "timestamp_unit",
                });
            }
        }
        let declaration_key = temporal.identity();
        let declaration = emitted_node_ids
            .get(&declaration_key)
            .cloned()
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let row = quire_contract_model::TemporalBinding {
            declaration,
            occurrence: temporal.claim().ordinal(),
            clock_name: clause.clock().into(),
            clock: clock.clone(),
        };
        keyed.rows.push((declaration_key, row.clone()));
        rows.push(row);
    }

    for hyper in graph.hyper_clauses() {
        let occurrence = hyper
            .assembled_occurrence()
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let wrapper = hyper_wrapper(graph, occurrence.node())?;
        names.push(quire_contract_model::TemporalClauseName {
            declaration: emitted_node_ids
                .get(&wrapper)
                .cloned()
                .ok_or(EmitRefusal::SourceMapInvariant)?,
            occurrence: occurrence.origin().ordinal(),
            name: hyper.name().into(),
        });
        let profile = hyper.profile();
        if !matches!(
            profile,
            TemporalProfile::EventPositionFalseExtension
                | TemporalProfile::FixedSampleFalseExtension
                | TemporalProfile::TimestampedEventFiniteWindow
        ) {
            continue;
        }
        let location = graph
            .occurrence(occurrence.node().clone(), occurrence.origin())
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let region = graph
            .region(location)
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let source = region.source().clone();
        let span = hyper.span();
        let key = (source.clone(), span.start, span.end);
        let Some(selection) = selected.remove(&key) else {
            missing.push((source, span));
            continue;
        };
        check_selected_definition(&source, span, profile, selection)?;
        let clock = &selection.clock;
        let clock_name = graph
            .semantic_graph()
            .node(wrapper.clone())
            .and_then(|node| match node.body() {
                BodyTerm::Application(application) => application.arguments.get(1),
                _ => None,
            })
            .and_then(|argument| match argument {
                MemberTerm::Leaf(LeafTerm::Literal {
                    value: LiteralValue::Text(name),
                    ..
                }) => Some(name.as_str()),
                _ => None,
            })
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        validate_temporal_clock(&source, span, clock_name, clock)?;
        let declaration = emitted_node_ids
            .get(&wrapper)
            .cloned()
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let row = quire_contract_model::TemporalBinding {
            declaration,
            occurrence: occurrence.origin().ordinal(),
            clock_name: clock_name.into(),
            clock: clock.clone(),
        };
        keyed.rows.push((wrapper.clone(), row.clone()));
        rows.push(row);
    }

    // QSpec FR-322-AC-305: a `compensate` node reads a clock too, so it is
    // keyed by its own node identity, named by the template's clock text,
    // and shares the one array with the clause rows. Its profile is the one
    // its `using` alias selects; a node of a profile that is not a
    // clock-configured one writes no row.
    let compensation_nodes: HashMap<Vec<&str>, &CheckedNodeId> = nodes
        .iter()
        .filter(|node| is_compensation_node(node))
        .filter_map(|node| {
            let declaration = node.declaration.as_ref()?;
            let path = declaration
                .qualified_name
                .iter()
                .map(AsRef::as_ref)
                .collect();
            Some((path, &node.node_id))
        })
        .collect();
    for protocol in graph.protocols() {
        let source = graph.source().clone();
        for template in &protocol.obligations.compensations {
            let Some(profile) = template.temporal_profile else {
                continue;
            };
            if !matches!(
                profile,
                TemporalProfile::EventPositionFalseExtension
                    | TemporalProfile::FixedSampleFalseExtension
                    | TemporalProfile::TimestampedEventFiniteWindow
            ) {
                continue;
            }
            let path = [protocol.name.as_str(), "compensate", template.name.as_str()];
            let Some(declaration) = compensation_nodes.get(path.as_slice()).copied().cloned()
            else {
                continue;
            };
            let span = template.span;
            let key = (source.clone(), span.start, span.end);
            let Some(selection) = selected.remove(&key) else {
                missing.push((source.clone(), span));
                continue;
            };
            check_selected_definition(&source, span, profile, selection)?;
            let clock = &selection.clock;
            validate_temporal_clock(&source, span, template.clock.as_str(), clock)?;
            for node in nodes.iter().filter(|node| node.node_id == declaration) {
                for ordinal in node
                    .occurrences
                    .iter()
                    .filter(|occurrence| occurrence.role == CheckedOccurrenceRole::Declaration)
                    .map(|occurrence| occurrence.ordinal)
                {
                    rows.push(quire_contract_model::TemporalBinding {
                        declaration: declaration.clone(),
                        occurrence: ordinal,
                        clock_name: template.clock.as_str().into(),
                        clock: clock.clone(),
                    });
                }
            }
        }
    }

    if let Some(((source_ref, start, end), _)) = selected.into_iter().next() {
        return Err(EmitRefusal::SurplusTemporalClockSelection {
            source_ref,
            span: qsl_foundation::Span { start, end },
        });
    }
    if let Some((source_ref, span)) = missing.into_iter().next() {
        return Err(EmitRefusal::MissingTemporalClockSelection { source_ref, span });
    }
    rows.sort_by(|left, right| {
        (&left.declaration, left.occurrence).cmp(&(&right.declaration, right.occurrence))
    });
    names.sort_by(|left, right| {
        (&left.declaration, left.occurrence).cmp(&(&right.declaration, right.occurrence))
    });
    refs.sort_by(|left, right| {
        (&left.declaration, left.occurrence).cmp(&(&right.declaration, right.occurrence))
    });
    Ok(WrittenTemporalRows {
        bindings: rows,
        names,
        keyed,
        refs,
    })
}

/// The ids `node` holds that this emitter can reach through IR's public
/// types: its own id, `semantic_type`, `owner_model`, `dependencies`, and
/// every id of a body term with public members (reference targets, literal
/// and result types, frame nodes, and the terms of
/// aggregates, bindings, extension clauses and operation anchors). A member
/// declaration or population an operation record names is a dependency
/// (FR-093 rule 2). An id held anywhere else stays structural, and IR's
/// encoder then refuses the package, so a gap fails loudly, never silently.
/// Foreign dependency references retain their dependency graph's IDs.
fn reachable_ids(node: &mut CheckedSemanticNodeV2) -> Vec<&mut CheckedNodeId> {
    let CheckedSemanticNodeV2 {
        node_id,
        semantic_type,
        owner_model,
        dependencies,
        body,
        ..
    } = node;
    let mut ids = vec![node_id, semantic_type];
    ids.extend(owner_model.iter_mut());
    ids.extend(dependencies.iter_mut());
    let mut pending = vec![body];
    while let Some(term) = pending.pop() {
        match term {
            CheckedTerm::Literal { type_id, .. } => ids.push(type_id),
            CheckedTerm::Reference { target } => ids.push(target),
            CheckedTerm::Application {
                result_type,
                arguments,
                ..
            } => {
                ids.push(result_type);
                pending.extend(arguments.iter_mut());
            }
            CheckedTerm::Aggregate { members } => pending.extend(members.iter_mut()),
            CheckedTerm::Binding { value, .. } => pending.push(value),
            CheckedTerm::Frame {
                creates, deletes, ..
            } => ids.extend(creates.iter_mut().chain(deletes.iter_mut())),
            CheckedTerm::ExtensionForm { clauses, .. } => pending.extend(clauses.iter_mut()),
            CheckedTerm::OperationAnchorBody(body) => pending.extend(body.members.iter_mut()),
            CheckedTerm::ImplementationGuarantee {
                clause,
                relation,
                progress,
                environment,
                ..
            } => {
                ids.extend([clause, relation]);
                ids.extend(progress.iter_mut().map(|entry| &mut entry.context));
                environment_ids(environment, &mut ids);
            }
            CheckedTerm::ComponentComposition {
                components,
                guarantees,
                relies,
                liveness_dependencies,
                supplies,
                environment,
                ..
            } => {
                ids.extend(components.iter_mut().map(|entry| &mut entry.subject));
                ids.extend(guarantees.iter_mut().map(|entry| &mut entry.clause));
                for premise in relies.iter_mut().chain(liveness_dependencies.iter_mut()) {
                    ids.extend([&mut premise.guarantee.clause, &mut premise.on.clause]);
                }
                ids.extend(supplies.iter_mut().map(|entry| &mut entry.context));
                environment_ids(environment, &mut ids);
            }
            _ => {}
        }
    }
    ids
}

/// The node ids an `environment` list holds.
fn environment_ids<'a>(
    environment: &'a mut [CheckedEnvironmentEntry],
    ids: &mut Vec<&'a mut CheckedNodeId>,
) {
    ids.extend(
        environment
            .iter_mut()
            .map(|entry| match &mut entry.supplies {
                CheckedEnvironmentSupplies::Fairness(supply) => &mut supply.context,
                CheckedEnvironmentSupplies::Assumption(node) => node,
            }),
    );
}

/// Write every node-held operand of `nodes` as an operand slot (FR-322): slot
/// `k` of a node names the `k`th distinct graph index its canonical bytes
/// hold, in byte order, and the returned row of that node lists those
/// indices. On entry, every node-held operand uses a document index.
///
/// # Errors
///
/// Returns the canonical output-limit failure or an identity invariant refusal.
/// Earlier nodes may already be rewritten on failure; discard that conversion.
pub fn write_operand_slots(
    nodes: &mut [CheckedSemanticNodeV2],
    limits: EmitLimits,
) -> Result<Vec<Vec<CheckedOperandEntry>>, EmitRefusal> {
    use quire_contract_model::CheckedNodePathSegment as Segment;
    let mut rows = Vec::with_capacity(nodes.len());
    for node in nodes.iter_mut() {
        let bytes = quire_canonical::to_vec(&*node, limits.canonical())?;
        let mut row: Vec<CheckedOperandEntry> = Vec::new();
        let mut slots = HashMap::new();
        for entry in operand_entries(&bytes, limits)? {
            if let std::collections::hash_map::Entry::Vacant(vacant) = slots.entry(entry.clone()) {
                vacant
                    .insert(u64::try_from(row.len()).map_err(|_| EmitRefusal::SourceMapInvariant)?);
                row.push(entry);
            }
        }
        let mut changes = Vec::new();
        for id in reachable_ids(node) {
            let old = id.clone();
            for segment in &mut id.path {
                let entry = match segment {
                    Segment::Index(index) => CheckedOperandEntry::Node(*index),
                    Segment::Dependency { package, node } => CheckedOperandEntry::Dependency(
                        quire_contract_model::CheckedDependencyOperand {
                            package: package.clone(),
                            node: *node,
                        },
                    ),
                    _ => continue,
                };
                let slot = slots.get(&entry).ok_or(EmitRefusal::SourceMapInvariant)?;
                *segment = Segment::Operand(*slot);
            }
            if old != *id {
                changes.push((old, id.clone()));
            }
        }
        for (old, new) in changes {
            node.replace_node_id(&old, &new);
        }
        rows.push(row);
    }
    Ok(rows)
}

fn operand_entries(
    bytes: &[u8],
    limits: EmitLimits,
) -> Result<Vec<CheckedOperandEntry>, EmitRefusal> {
    qsl_semantics::check::wire_id::canonical_operand_entries(bytes, limits.output_bytes)
        .map_err(|_| EmitRefusal::SourceMapInvariant)
}

/// Retain document-held IDs for producer joins and subsequent rewrites.
pub(crate) fn document_nodes(
    mut graph: CheckedSemanticGraphV2,
) -> Result<Vec<CheckedSemanticNodeV2>, EmitRefusal> {
    use quire_contract_model::CheckedNodePathSegment as Segment;
    for (node, row) in graph.nodes.iter_mut().zip(&graph.operand_rows) {
        let mut changes = Vec::new();
        for id in reachable_ids(node) {
            let old = id.clone();
            for segment in &mut id.path {
                if let Segment::Operand(slot) = segment {
                    let entry = usize::try_from(*slot)
                        .ok()
                        .and_then(|slot| row.get(slot))
                        .ok_or(EmitRefusal::SourceMapInvariant)?;
                    *segment = match entry {
                        CheckedOperandEntry::Node(index) => Segment::Index(*index),
                        CheckedOperandEntry::Dependency(operand) => Segment::Dependency {
                            package: operand.package.clone(),
                            node: operand.node,
                        },
                    };
                }
            }
            if old != *id {
                changes.push((old, id.clone()));
            }
        }
        for (old, new) in changes {
            node.replace_node_id(&old, &new);
        }
    }
    Ok(graph.nodes)
}

/// Whether `id` holds a segment `holds` matches.
fn holds_segment(
    id: &CheckedNodeId,
    holds: impl Fn(&quire_contract_model::CheckedNodePathSegment) -> bool,
) -> bool {
    id.path.iter().any(holds)
}

/// The nodes of a package's dependencies by their position in the graph of
/// the dependency that holds them (QSpec FR-491): the form in which the
/// package names an operand of a dependency's node, without holding a copy of
/// that node.
#[derive(Default)]
struct ForeignNodes {
    positions: HashMap<NodeKey, (Box<str>, u64)>,
}

impl ForeignNodes {
    /// The positions of every node of every dependency of `package`, read
    /// only when the package composes components. A dependency's emission is
    /// the one the link already made.
    fn of(package: &CheckedPackage, limits: EmitLimits) -> Result<Self, EmitRefusal> {
        let mut foreign = Self::default();
        if package.graph().compositions().is_empty() {
            return Ok(foreign);
        }
        for (identity, dependency) in package.dependencies() {
            let emission = emit_checked(dependency, limits)?;
            let graph = qsl_semantics::check::wire_id::WireGraph::new(
                emission.wire_nodes.iter().map(|node| &node.node_id),
            )
            .map_err(|_| EmitRefusal::SourceMapInvariant)?;
            for (at, key) in (0_u64..).zip(graph.keys()) {
                foreign
                    .positions
                    .insert(key.clone(), (identity.as_str().into(), at));
            }
        }
        Ok(foreign)
    }

    /// Whether `key` is a node of a dependency.
    fn holds(&self, key: &NodeKey) -> bool {
        self.positions.contains_key(key)
    }

    /// The dependency position segment naming `key`.
    fn position(&self, key: &NodeKey) -> Option<quire_contract_model::CheckedNodePathSegment> {
        self.positions.get(key).map(|(package, node)| {
            quire_contract_model::CheckedNodePathSegment::Dependency {
                package: package.clone(),
                node: *node,
            }
        })
    }
}

/// FR-322 "Operand index segments": rewrite every structural id the package
/// holds into its wire form, each operand node written as the index segment
/// `{"node": i}` of that node in `nodes`, whose order is now final.
/// Returns each emitted node's final wire id, keyed by its original structural
/// key, for producer rows that must name those same admitted nodes.
///
/// Linear in the package: each node's key is rebuilt once, sharing the key
/// of every operand allocation ([`StructuralKeys`]), and each id a node
/// reaches ([`reachable_ids`]) is rewritten in place. No two ids are
/// compared: comparing two ids that share a deep operand chain costs the
/// chain's depth.
///
/// [`StructuralKeys`]: qsl_semantics::check::wire_id::StructuralKeys
fn write_index_segments(
    nodes: &mut [CheckedSemanticNodeV2],
    source_map: &mut [CheckedSourceMapEntry],
    step_rows: &mut [quire_contract_model::CheckedRefinementStepRowRegion],
    schedulability_rows: &mut [quire_contract_model::CheckedSchedulabilityClaimRow],
    foreign: &ForeignNodes,
) -> Result<HashMap<NodeKey, CheckedNodeId>, EmitRefusal> {
    use quire_contract_model::CheckedNodePathSegment as Segment;
    let is_operand = |segment: &Segment| matches!(segment, Segment::Node(_));
    let mut keys = qsl_semantics::check::wire_id::StructuralKeys::default();
    let invariant = |_| EmitRefusal::SourceMapInvariant;
    let graph_keys = nodes
        .iter()
        .map(|node| keys.key(&node.node_id).map_err(invariant))
        .collect::<Result<Vec<_>, _>>()?;
    let index: HashMap<&NodeKey, u64> = graph_keys
        .iter()
        .enumerate()
        .map(|(at, key)| (key, at as u64))
        .collect();
    let wire_of = |keys: &mut qsl_semantics::check::wire_id::StructuralKeys,
                   holder: &CheckedNodeId,
                   id: &CheckedNodeId|
     -> Result<CheckedNodeId, EmitRefusal> {
        let path = id
            .path
            .iter()
            .map(|segment| match segment {
                Segment::Node(operand) => {
                    let key = keys.key(operand).map_err(invariant)?;
                    index
                        .get(&key)
                        .map(|at| Segment::Index(*at))
                        .or_else(|| foreign.position(&key))
                        .ok_or_else(|| EmitRefusal::OperandNotInGraph {
                            node: Box::new(holder.clone()),
                            operand: Box::new((**operand).clone()),
                        })
                }
                other => Ok(other.clone()),
            })
            .collect::<Result<_, _>>()?;
        Ok(CheckedNodeId {
            package: id.package.clone(),
            path,
            ordinal: id.ordinal,
        })
    };
    // The memo holds every operand allocation it keyed, so rewriting ids in
    // place never reuses an address it has seen.
    let mut graph_wire: HashMap<NodeKey, CheckedNodeId> = HashMap::with_capacity(nodes.len());
    for (node, key) in nodes.iter_mut().zip(&graph_keys) {
        let holder = node.node_id.clone();
        let mut seen: HashSet<NodeKey> = HashSet::new();
        let mut changes: Vec<(CheckedNodeId, CheckedNodeId)> = Vec::new();
        for id in reachable_ids(node) {
            if holds_segment(id, is_operand) {
                let new = wire_of(&mut keys, &holder, id)?;
                if seen.insert(keys.key(id).map_err(invariant)?) {
                    changes.push((id.clone(), new.clone()));
                }
                *id = new;
            }
        }
        // IR's own substitution reaches the ids held where this emitter
        // cannot (an operation's closed record and views, a nominal or
        // refinement body). Every id it can reach already holds index
        // segments, so it never matches a structural id deep in a chain.
        for (old, new) in &changes {
            node.replace_node_id(old, new);
        }
        graph_wire.insert(key.clone(), node.node_id.clone());
    }
    let mut sidecar = |id: &mut CheckedNodeId| -> Result<(), EmitRefusal> {
        if holds_segment(id, is_operand) {
            let key = keys.key(id).map_err(invariant)?;
            *id = graph_wire
                .get(&key)
                .cloned()
                .ok_or(EmitRefusal::SourceMapInvariant)?;
        }
        Ok(())
    };
    for entry in source_map.iter_mut() {
        sidecar(&mut entry.node_id)?;
    }
    for row in step_rows.iter_mut() {
        sidecar(&mut row.node_id)?;
    }
    for row in schedulability_rows {
        sidecar(&mut row.declaration)?;
    }
    Ok(graph_wire)
}

/// The inverse of [`write_index_segments`] over an admitted package's nodes:
/// every wire id each node reaches is replaced, in that node, by the
/// structural id of the node it names, so the emitter's identity work
/// compares structural keys.
fn read_index_segments(
    ids: &StructuralIds,
    nodes: &mut [CheckedSemanticNodeV2],
    source_map: &mut [CheckedSourceMapEntry],
) -> Result<(), EmitRefusal> {
    use quire_contract_model::CheckedNodePathSegment as Segment;
    let is_index = |segment: &Segment| matches!(segment, Segment::Index(_));
    let graph =
        qsl_semantics::check::wire_id::WireGraph::new(nodes.iter().map(|node| &node.node_id))
            .map_err(|_| EmitRefusal::SourceMapInvariant)?;
    let structural = |id: &CheckedNodeId| {
        graph
            .key(id)
            .map(|key| node_id(ids, &key))
            .map_err(|_| EmitRefusal::SourceMapInvariant)
    };
    for node in nodes.iter_mut() {
        let mut changes: Vec<(CheckedNodeId, CheckedNodeId)> = Vec::new();
        for id in reachable_ids(node) {
            if holds_segment(id, is_index) {
                let new = structural(id)?;
                if !changes.iter().any(|(old, _)| old == id) {
                    changes.push((id.clone(), new.clone()));
                }
                *id = new;
            }
        }
        // As in `write_index_segments`: IR substitutes where this reader
        // cannot reach.
        for (old, new) in &changes {
            node.replace_node_id(old, new);
        }
    }
    for entry in source_map.iter_mut() {
        if holds_segment(&entry.node_id, is_index) {
            entry.node_id = structural(&entry.node_id)?;
        }
    }
    Ok(())
}

/// FR-354 "Reading": each held extension definition that an `extension` node
/// of the package names, under its `DefinitionRef`, with exactly its two
/// FR-467 members. Reader input only; no package byte depends on it.
fn supply_extension_definitions(
    evidence: &mut CheckedPackageEvidence,
    graph: &CheckedGraph,
    order: &[&Candidate<'_>],
) -> Result<(), EmitRefusal> {
    for (selection, definition) in graph
        .extension_selections()
        .iter()
        .zip(graph.extension_definitions())
    {
        let named = order.iter().any(|candidate| {
            matches!(
                candidate.node.body(),
                BodyTerm::Extension(extension) if extension.definition() == selection
            )
        });
        if named {
            evidence
                .insert_extension_definition(
                    CheckedArtifactRef {
                        authority: selection.authority().into(),
                        identity: selection.identity().into(),
                    },
                    definition.clone(),
                )
                .map_err(encoding)?;
        }
    }
    Ok(())
}

/// The evidence IR's reader needs to read this emission back: each of the
/// lock's required features, declared as supported.
fn own_evidence(lock: &CheckedPackageLockV2) -> CheckedPackageEvidence {
    let mut evidence = CheckedPackageEvidence::new();
    for feature in &lock.required_features {
        evidence.support_feature(feature.clone());
    }
    evidence
}

mod selected_rows;

#[cfg(test)]
pub(crate) mod tests;

pub(crate) mod equality_reader;
/// ADR-014 §4 (TC-440): QSL's extent classification agrees with
/// IR's `requires-bound` at the pinned IR revision.
#[cfg(test)]
mod extent_agreement;
mod identity_rows;
mod monitor_plan;
pub(crate) mod protocol_locations;
pub use monitor_plan::{declaration_less_function_id, emit_monitor_plan_over, MonitorPlanWire};
mod timed_order;
pub use timed_order::OrderFault as TimedOrderFault;
