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
    CheckedCapabilityDisposition, CheckedDeclaration, CheckedDependencySelection,
    CheckedDiagnosticsV2, CheckedFieldScope, CheckedFrameModification, CheckedNodeId,
    CheckedNodeKind, CheckedNodeOwner, CheckedNodeTag, CheckedOccurrence, CheckedOccurrenceRole,
    CheckedOperationAnchorBody, CheckedPackageEvidence, CheckedPackageLockV2,
    CheckedPackageSourceCorrespondenceRef, CheckedPackageV2, CheckedRational,
    CheckedRefinementStepRowRegion, CheckedScopedDeletion, CheckedSelection, CheckedSelectionRole,
    CheckedSemanticGraphV2, CheckedSemanticNodeV2, CheckedSourceMapEntry, CheckedSourceRef,
    CheckedSourceRegion, CheckedTerm, DimensionTerm, LiteralKind, NominalBody, ProtocolForm,
    StateForm, ValueForm,
};
use serde::Deserialize;

use ix_trace_rs::string_edge;
use qsl_foundation::source::provenance::{
    OccurrenceKey, PackageSourceMap, RawSourceRef, SourceRegion,
};
use qsl_foundation::Code;
use qsl_semantics::check::wire_id::StructuralIds;
use qsl_semantics::check::{
    AbstractionNodeRefusal, AbstractionNodes, BindingValue, BodyTerm, CheckedGraph,
    CheckedRefinement, FrameDeletesScoped, FrameModifies, FrameScope, FrameTerm, GroupMember,
    GroupTerm, LawRole, LeafTerm, LiteralValue, MemberTerm, NodeRef, NodeTag, OperationLaw,
    Operator, Owner, SemanticNode, TemporalProfile, TimeNominalOrigin, TupleMember, TupleTerm,
};
use qsl_semantics::value::nominal_payload::NominalPayload;
use qsl_semantics::value::{
    native_diagnostics_catalog, DefinitionReference, DefinitionRole, HeaderLayer, Member,
    ModelRole, NodeOwner, TemporalRole,
};
use quire_exact::{Cancel, NodeKey, Origin, PathSegment, Role};
use quire_semantic_value::location::Location;

use super::{CheckedPackage, EmittedPackage};
use crate::checked_v2::{
    CheckedClockConstraint, CheckedClockGuard, CheckedClockPlace, CheckedClockReset,
    CheckedConstraintTarget, CheckedGuardPart, CheckedResetTarget, CheckedTimeInvariant,
    CheckedTimedModel, CheckedUrgency, ClockComparison, TimedNodeMetadata,
};

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
    /// The timed nodes cannot be put in QSpec FR-322's declared order.
    #[error("timed aggregate order: {0}")]
    TimedOrder(#[source] timed_order::OrderFault),
    /// A typed timed model cannot be serialized with its available literals.
    #[error("typed timed-model writer invariant: {0}")]
    TimedModelWriterInvariant(&'static str),
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
    /// An equality producer identity row has no unique emitted application
    /// occurrence to which it can be attached.
    #[error("equality producer identity for application {application:?} has no unique emitted occurrence")]
    MissingEqualityIdentityContext {
        /// The exact checked application key retained by QSL lowering.
        application: NodeKey,
    },
}

impl From<quire_canonical::Error> for EmitRefusal {
    fn from(error: quire_canonical::Error) -> Self {
        encoding(error)
    }
}

impl EmitRefusal {
    /// The catalog code (FR-010).
    pub fn code(&self) -> Code {
        match self {
            Self::NothingToEmit { .. }
            | Self::UnlocatedOccurrence { .. }
            | Self::UnknownOccurrenceRole { .. } => Code::UnsupportedProjection,
            Self::Encoding { .. } => Code::OutputFailure,
            Self::Abstraction(AbstractionNodeRefusal::Internal(_)) => Code::RuntimeInvariant,
            Self::Abstraction(_) => Code::UnsupportedProjection,
            Self::StaleProtocolReference { .. } => Code::MissingDeclaration,
            Self::Cancelled { .. } => Code::Cancelled,
            Self::ProbabilisticModelNotEmitted { .. } | Self::TimedModelNotEmitted { .. } => {
                Code::UnsupportedConstruct
            }
            Self::TimedModelNodeSetMismatch | Self::TimedOrder(_) => Code::RuntimeInvariant,
            Self::TimedModelWriterInvariant(_) | Self::RefinementSubjectNotEmitted => {
                Code::RuntimeInvariant
            }
            Self::RefinementIdentityEncoding => Code::OutputFailure,
            Self::RefinementNotEmitted => Code::UnsupportedConstruct,
            Self::UnlocatedRefinementDeclaration => Code::UnsupportedProjection,
            Self::SourceMapInvariant
            | Self::UnresolvedNodeReference
            | Self::OperandNotInGraph { .. }
            | Self::MissingEqualityIdentityContext { .. } => Code::RuntimeInvariant,
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
            | Self::UnlocatedOccurrence { .. }
            | Self::UnknownOccurrenceRole { .. }
            | Self::Encoding { .. }
            | Self::Abstraction(_)
            | Self::Cancelled { .. }
            | Self::TimedModelNodeSetMismatch
            | Self::TimedOrder(_)
            | Self::TimedModelWriterInvariant(_)
            | Self::RefinementSubjectNotEmitted
            | Self::RefinementIdentityEncoding
            | Self::UnlocatedRefinementDeclaration
            | Self::SourceMapInvariant
            | Self::UnresolvedNodeReference
            | Self::OperandNotInGraph { .. }
            | Self::MissingEqualityIdentityContext { .. } => None,
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

/// The emitted package and the nodes it omits.
#[derive(Clone, Debug)]
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
    /// Exact QSL identity witnesses for emitted equality occurrences.
    pub(crate) equality_producer_identity_rows: Vec<EqualityProducerEmissionRow>,
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
}

impl Emission {
    /// The v2 bytes and their manifest package name.
    pub fn package(&self) -> &EmittedPackage {
        &self.package
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

    /// QSL-owned identity rows keyed by equality application and occurrence.
    pub fn equality_producer_identity_rows(&self) -> &[EqualityProducerEmissionRow] {
        &self.equality_producer_identity_rows
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
        return nominal_term(ids, nominal);
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
fn nominal_term(ids: &StructuralIds, payload: &NominalPayload) -> Result<CheckedTerm, EmitRefusal> {
    let body = match payload {
        NominalPayload::Enum(enumeration) => NominalBody::try_enum(
            enumeration.is_ordered(),
            enumeration
                .members()
                .iter()
                .map(|member| member.as_str().into())
                .collect(),
        ),
        NominalPayload::Dimension(dimension) => NominalBody::try_dimension(
            dimension
                .terms()
                .iter()
                .map(|(base, exponent)| DimensionTerm {
                    dimension_node_id: node_id(ids, base),
                    exponent: exponent.to_string().into(),
                })
                .collect(),
        ),
        NominalPayload::Unit(unit) => NominalBody::try_unit(
            node_id(ids, unit.dimension()),
            unit.target().map(|key| node_id(ids, key)),
            checked_rational(unit.factor()),
            checked_rational(unit.offset()),
        ),
        NominalPayload::DimensionEquivalence(equivalence) => {
            NominalBody::try_dimension_equivalence(
                equivalence
                    .dimensions()
                    .each_ref()
                    .map(|key| node_id(ids, key)),
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
    }
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
/// definition the bodies name that is not already selected.
///
/// A unit whose header selects no layer selects the `always` rows: the
/// lock's `without_layer` rows are for a unit that selects layers none of
/// which is `quire.value.complete/v1`.
fn catalog_selections(
    layers: &[HeaderLayer],
    laws: &[DefinitionReference],
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
        .chain(laws.iter().cloned());
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
fn equality_producer_emission_rows(
    graph: &CheckedGraph,
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
    source_map: &[CheckedSourceMapEntry],
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
            let mut occurrences = graph.occurrences().filter(|(node, _, location)| {
                node == application && *location == expression_location
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
            let package_source = source_artifact(graph.source());
            if source_map_entry.regions.is_empty()
                || source_map_entry
                    .regions
                    .iter()
                    .any(|region| region.source != package_source)
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
                operation.ends_with(".eq")
            }
            quire_semantic_value::declaration::EqualityOperator::NotEqual => {
                operation.ends_with(".ne")
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
                        if convert.operation.identity().ends_with(".convert") {
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
    }))
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
                    || member_node.semantic_form() != "enum_value"
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
        let selected = data.units().ids().find_map(|id| {
            if !identity.unit_ids().contains(&id) {
                return None;
            }
            let quantity_unit = data.units().get(id.clone())?;
            match quantity_unit {
                quire_semantic_value::quantity::QuantityUnit::Declared(declared)
                    if declared.key() == quantity_type =>
                {
                    Some((id, quantity_unit.clone()))
                }
                quire_semantic_value::quantity::QuantityUnit::Compound(compound)
                    if quantity_node.semantic_form() == "compound_unit"
                        && compound_terms_match(quantity_node.body(), compound) =>
                {
                    Some((id, quantity_unit.clone()))
                }
                _ => None,
            }
        });
        let Some((selected_id, selected_unit)) = selected else {
            return Err(EmitRefusal::MissingEqualityIdentityContext {
                application: application.clone(),
            });
        };
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
                if !identity.dimension_declarations().contains(&dimension) {
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
            for edge in unit.path() {
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
                if !identity.conversion_edge_targets().contains(&target) {
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

fn compound_terms_match(
    body: &qsl_semantics::check::BodyTerm,
    compound: &quire_semantic_value::unit::CompoundUnit,
) -> bool {
    let qsl_semantics::check::BodyTerm::Aggregate(aggregate) = body else {
        return false;
    };
    let mut actual = Vec::with_capacity(aggregate.members.len());
    for member in &aggregate.members {
        let qsl_semantics::check::MemberTerm::Group(group) = member else {
            return false;
        };
        let (
            Some(qsl_semantics::check::GroupMember::Binding(unit)),
            Some(qsl_semantics::check::GroupMember::Binding(exponent)),
        ) = (group.members.first(), group.members.get(1))
        else {
            return false;
        };
        let qsl_semantics::check::LeafTerm::Reference { target } = &unit.value else {
            return false;
        };
        let qsl_semantics::check::LeafTerm::Literal {
            value: qsl_semantics::check::LiteralValue::Integer(exponent),
            ..
        } = &exponent.value
        else {
            return false;
        };
        let Some(root) = target.admitted() else {
            return false;
        };
        actual.push((root.clone(), exponent.clone()));
    }
    let expected = compound
        .terms()
        .map(|(root, exponent)| (root, exponent.clone()))
        .collect::<Vec<_>>();
    actual == expected
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

/// Emit `package` as `quire.checked-package/v2` bytes (FR-322), placing each
/// occurrence at the region of the checked unit its location names
/// (`CheckedGraph::region`, FR-096).
///
/// A package emitted before (by this function or
/// [`emit_checked_with_cancel`]) returns its remembered emission: a checked
/// package does not change, so a dependency reached again, or a caller that
/// emits what the lifecycle already emitted, does not emit it twice.
pub fn emit_checked(package: &CheckedPackage) -> Result<Emission, EmitRefusal> {
    if let Some(emission) = package.remembered_emission() {
        return Ok(emission.clone());
    }
    emit_checked_with_cancel(package, &Cancel::new())
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
        if let Some(held) = model
            .invariants
            .iter_mut()
            .find(|held| held.predicate == predicate && held.bounds == bounds)
        {
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

/// QSpec FR-322 `model_root`, FR-322-AC-132 and AC-134: for each selected
/// domain package, in `CheckedGraph::model_selections` order, one self-typed `model` node
/// keyed by the selection's identity alone, appended after every
/// other node, with one `generated` source-map entry appended after every
/// other entry. The selection records no selecting occurrence, so the entry's
/// region is the first byte of the checked unit (QSpec: the entry is a
/// producer assertion no reader verifies, and the published regions are
/// one-byte placeholders).
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
) -> Result<Emission, EmitRefusal> {
    let ids = &StructuralIds::default();
    let expected = base
        .graph()
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
        .graph()
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
    write_index_segments(&mut nodes, &mut source_map, &mut [])?;
    let semantic_graph = CheckedSemanticGraphV2 {
        graph_version: base.graph().graph_version.clone(),
        nodes,
    };
    let capability_report = base.capability_report().to_vec();
    let diagnostics = base.diagnostics().clone();
    let package = EmittedPackage::new(base.name(), |name| {
        encode_checked_package_v2(
            name,
            &lock,
            &semantic_graph,
            CheckedPackageSourceCorrespondenceRef {
                source_map: &source_map,
                refinement_step_rows: None,
            },
            None,
            &capability_report,
            &diagnostics,
            crate::checked::EMIT_LIMITS,
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
        equality_producer_identity_rows: Vec::new(),
    })
}

/// [`emit_checked`] under the caller's [`Cancel`] handle, polled once at
/// entry and at every node it writes (FR-276). A cancelled handle stops the
/// emitter with [`EmitRefusal::Cancelled`] and writes no bytes. It always
/// emits, so every node is polled; the emission it returns is remembered for
/// [`emit_checked`].
pub fn emit_checked_with_cancel(
    package: &CheckedPackage,
    cancel: &Cancel,
) -> Result<Emission, EmitRefusal> {
    let graph = package.graph();
    let emission = emit_package_inner(
        package,
        graph.memoized_regions(),
        |_| None,
        cancel,
        &ir_decodes,
    )?;
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
) -> Result<Emission, EmitRefusal> {
    emit_package_inner(package, regions, |_| None, &Cancel::new(), &ir_decodes)
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
) -> Result<Emission, EmitRefusal> {
    emit_package_inner(package, regions, fault, &Cancel::new(), &ir_decodes)
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
) -> Result<Emission, EmitRefusal> {
    emit_package_inner(package, regions, |_| None, &Cancel::new(), decodes)
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
) -> Result<Emission, EmitRefusal> {
    let graph = package.graph();
    emit_package_with_fault(package, graph.memoized_regions(), fault)
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
    graph: &'g CheckedGraph,
    abstraction: Option<&'g AbstractionNodes>,
) -> Result<Candidates<'g>, EmitRefusal> {
    let mut recorded = recorded_occurrences(graph)?;
    let mut candidates: Vec<Candidate<'g>> = graph
        .semantic_graph()
        .nodes()
        .map(|node| {
            let occurrences = recorded.remove(&node.key()).unwrap_or_default();
            Candidate::of(ids, node, occurrences)
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
                let candidate = Candidate::of(ids, node, occurrences)?;
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
    let candidates = candidates_of(ids, graph, abstraction.as_ref())?;
    let model_roots: HashSet<NodeKey> = graph.model_roots().map(|(_, key)| key).collect();
    let selected_models: BTreeSet<&str> = graph
        .model_roots()
        .map(|(selection, _)| selection.identity.as_str())
        .collect();
    let omitted = omissions(
        ids,
        &candidates,
        graph.source(),
        decodes,
        &model_roots,
        &selected_models,
    );
    let dropped: HashSet<usize> = omitted.iter().map(|(position, _)| *position).collect();
    let omitted: Vec<OmittedNode> = omitted.into_iter().map(|(_, omission)| omission).collect();
    stale_protocol_reference(ids, &candidates, &dropped)?;
    let kept: Vec<&Candidate<'_>> = candidates
        .values()
        .enumerate()
        .filter(|(position, _)| !dropped.contains(position))
        .map(|(_, candidate)| candidate)
        .collect();
    if kept.is_empty() && graph.refinements().is_empty() {
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
    sources.insert(source_artifact(graph.source()));
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
            model.clock_places = graph
                .timed_models()
                .iter()
                .filter(|timed| timed.model() == source.model)
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
            for timed in graph
                .timed_models()
                .iter()
                .filter(|timed| timed.model() == source.model)
            {
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
            if let Some(clauses) = graph
                .timed_models()
                .iter()
                .find(|timed| timed.model() == source.model)
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
                let id = node_id(ids, &nominal.key);
                if nodes.iter().any(|node| node.node_id == id) {
                    continue;
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
                    match (nominal.form.as_str(), nominal.nominal_payload()) {
                        ("dimension", Some(payload @ NominalPayload::Dimension(_)))
                        | ("unit", Some(payload @ NominalPayload::Unit(_))) => {
                            nominal_term(ids, payload)?
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
                let semantic_type = if nominal.form == "unit" {
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
                // so its `generated` occurrence takes the timed source's
                // region.
                let region = match nominal.name_span() {
                    Some(span) => SourceRegion::new(
                        graph.source().clone(),
                        span.start as u64,
                        span.end as u64,
                    )
                    .map_err(encoding)?,
                    None => source_region.clone(),
                };
                source_map.push(CheckedSourceMapEntry {
                    node_id: id.clone(),
                    role: occurrence.role.clone(),
                    ordinal: occurrence.ordinal,
                    regions: vec![CheckedSourceRegion {
                        source: source_artifact(graph.source()),
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
    append_model_roots(
        ids,
        graph.model_roots(),
        graph.source(),
        &mut nodes,
        &mut source_map,
    );
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

    let (edition, definition_selections) = catalog_selections(graph.header_layers(), &laws);
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
    let emitted_node_ids = write_index_segments(&mut nodes, &mut source_map, &mut step_rows)?;
    let equality_producer_identity_rows =
        equality_producer_emission_rows(graph, &emitted_node_ids, &source_map)?;
    let checked_equality_term_data = checked_equality_term_data(
        graph,
        &emitted_node_ids,
        &selected_model_enum_member_rows,
        &equality_producer_identity_rows,
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
    let semantic_graph = CheckedSemanticGraphV2 {
        graph_version: GRAPH_V2.into(),
        nodes,
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
            checked_equality_term_data.as_ref(),
            &capability_report,
            &diagnostics,
            crate::checked::EMIT_LIMITS,
        )
        .map_err(EmitRefusal::from)
    })?;
    let evidence = own_evidence(&lock);
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
        equality_producer_identity_rows,
    })
}

/// The ids `node` holds that this emitter can reach through IR's public
/// types: its own id, `semantic_type`, `owner_model`, `dependencies`, and
/// every id of a body term with public members (reference targets, literal
/// and result types, frame nodes, dependency nodes, and the terms of
/// aggregates, bindings, extension clauses and operation anchors). A member
/// declaration or population an operation record names is a dependency
/// (FR-093 rule 2). An id held anywhere else stays structural, and IR's
/// encoder then refuses the package, so a gap fails loudly, never silently.
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
            CheckedTerm::DependencyReference { node, .. } => ids.push(node),
            CheckedTerm::Frame {
                creates, deletes, ..
            } => ids.extend(creates.iter_mut().chain(deletes.iter_mut())),
            CheckedTerm::ExtensionForm { clauses, .. } => pending.extend(clauses.iter_mut()),
            CheckedTerm::OperationAnchorBody(body) => pending.extend(body.members.iter_mut()),
            _ => {}
        }
    }
    ids
}

/// Whether `id` holds a segment `holds` matches.
fn holds_segment(
    id: &CheckedNodeId,
    holds: impl Fn(&quire_contract_model::CheckedNodePathSegment) -> bool,
) -> bool {
    id.path.iter().any(holds)
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

/// The evidence IR's reader needs to read this emission back: each of the
/// lock's required features, declared as supported.
fn own_evidence(lock: &CheckedPackageLockV2) -> CheckedPackageEvidence {
    let mut evidence = CheckedPackageEvidence::new();
    for feature in &lock.required_features {
        evidence.support_feature(feature.clone());
    }
    evidence
}

#[cfg(test)]
pub(crate) mod tests;

/// ADR-014 §4 (TC-440): QSL's extent classification agrees with
/// IR's `requires-bound` at the pinned IR revision.
#[cfg(test)]
mod extent_agreement;
mod timed_order;
pub use timed_order::OrderFault as TimedOrderFault;
