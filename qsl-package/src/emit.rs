// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 T-8 (M-4), slice S1b: the S4 v2 emitter, [`CheckedPackage`]
//! -> [`EmittedPackage`] (the `quire.checked-package/v2` bytes with their own
//! `package_id`, QSpec FR-322).
//!
//! The emitter serializes the nodes `check` lowered and keyed
//! ([`CheckedGraph::semantic_graph`], FR-092 to FR-094). It builds no body
//! term and mints no node key (FR-093-CON-2). To each node it adds what
//! FR-093 gives the emission arm: its `node_id`, its `dependencies` (FR-093
//! "Node dependencies", rules 1 to 3), its occurrences, its
//! `recursion_group` label and the graph order, which writes each recursion
//! group's members in ordinal order.
//!
//! # Lock
//!
//! - `edition` and `definition_selections` come from the `DefinitionLock`
//!   catalog: the `edition` role, every other always-selected role, and each
//!   law `DefinitionRef` a node body names. Each is a QSpec `DefinitionRef`,
//!   exactly `{authority, identity}`.
//! - `sources` are the checked unit's `RawSourceRef` (`CheckedGraph::source`)
//!   and any other raw source an occurrence region names, each with its
//!   `quire.source.bytes/v1` digest.
//! - `required_features` is `quire.value.complete/v1` (ADR-011 §2.4) and
//!   `capability_report` reports it available.
//! - `profile_selections` holds one row per clause-profile law a node body
//!   names (`protocol_profile`, FR-092); it is empty for a package of
//!   `Value`-family nodes only.
//! - `model_selections` is `CheckedGraph::model_selections`: each domain
//!   package the package was checked against, by identity and
//!   `sha256-jcs` digest (ADR-011 §2.4).
//! - `dependency_selections` is [`CheckedPackage::dependency_selections`]:
//!   the resolved library closure, one `{identity, package_id}`
//!   entry per library identity in ascending UTF-8 byte order of `identity`
//!   (FR-322, FR-307). The same entries are the identity preimage's, so each
//!   dependency's `package_id` enters this package's.
//! - `diagnostics.catalog` is QSpec's `quire.native.diagnostics/v1`
//!   catalog, named by its `DefinitionRef`.
//!
//! # Source regions
//!
//! FR-322 maps every occurrence to at least one byte region. `check` records
//! each occurrence at a `quire_semantic_value::location::Location` (a
//! declaration and a child path).
//! [`emit_checked`] places it through the checked unit's own form spans
//! (`CheckedGraph::region`, FR-096, ADR-013 O-12); [`emit_package`] takes
//! the conversion as a parameter. An occurrence that cannot be placed, or is
//! placed at an empty region, refuses the whole emission
//! ([`EmitRefusal::UnlocatedOccurrence`]).
//!
//! # Omitted nodes
//!
//! A node whose (`node_tag`, `semantic_form`) IR's v2 vocabulary does not
//! hold is omitted. So is a node that names a node the checked graph does
//! not hold, a nominal node whose owner the lock does not select (a
//! definition-owned unit or dimension node, so a
//! `scalar_type`/`compound_unit` node over one), and every node that names
//! an omitted one. Every other node is written (FR-062-AC-9: all-or-nothing
//! over the nodes a node names), `value`/`parameter` nodes and
//! recursion-group application nodes included (IR-280, IR-242).
//! [`Emission::omitted`] lists each omitted node and its cause. A
//! `protocol_reference` carrier whose target the wire does not write is not
//! omitted: it refuses the emission ([`EmitRefusal::StaleProtocolReference`]).
//!
//! # Abstraction relation
//!
//! A package that carries a checked abstraction relation
//! ([`CheckedPackage::abstraction`], FR-304) writes it as the one
//! `correspondence`/`abstraction_relation` node of QSpec FR-451 (FR-306),
//! with the `model`/`object_type` and `relation`/`population` nodes it
//! names that the graph does not hold.
//! `check` builds and keys these ([`CheckedGraph::abstraction_nodes`]); the
//! relation node has one `declaration` occurrence per declaration, the
//! others one `generated`, at [`AbstractionNodes::sites`].
//!
//! # Nominal nodes
//!
//! An enum declaration or enum member node carries the QSpec nominal
//! preimage its key hashes (`nominal_identity_preimage`, FR-092 rule 1).
//! An enum member node depends on its declaration, as QSpec's
//! `quire.enum-member-node/v1` rule requires.
//!
//! The wire envelope is written by IR's typed encoder,
//! [`encode_checked_package_v2`]; this module defines no envelope type of its
//! own, and IR's reader admits exactly the bytes that encoder writes.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use quire_contract_model::{
    derive_structural_candidates, encode_checked_package_v2, CheckedArtifactRef, CheckedCapability,
    CheckedCapabilityDisposition, CheckedDeclaration, CheckedDependencySelection,
    CheckedDiagnosticsV2, CheckedDomainPackageRef, CheckedNodeId, CheckedNodeKind,
    CheckedNodeOwner, CheckedNodeTag, CheckedOccurrence, CheckedOccurrenceRole,
    CheckedPackageEvidence, CheckedPackageIdentityPreimageRefV2, CheckedPackageLockV2,
    CheckedPackageSourceCorrespondenceRef, CheckedPackageV2, CheckedRational, CheckedSelection,
    CheckedSelectionRole, CheckedSemanticGraphV2, CheckedSemanticId, CheckedSemanticNodeV2,
    CheckedSourceMapEntry, CheckedSourceRef, CheckedSourceRegion, CheckedTerm,
    NominalIdentityPreimage, NominalOwner, ProtocolForm, ValueForm, DOMAIN_PACKAGE_DIGEST,
    PACKAGE_DOMAIN_V2,
};
use serde::Deserialize;

use ix_trace_rs::string_edge;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::{
    OccurrenceKey, PackageSourceMap, RawSourceRef, SourceRegion,
};
use qsl_foundation::Code;
use qsl_semantics::check::{
    AbstractionNodeRefusal, AbstractionNodes, BodyTerm, CheckedGraph, CheckedRefinement, LawRole,
    LeafTerm, NodeTag, NominalNode, OperationLaw, Owner, SemanticNode,
};
use qsl_semantics::library::PackageId;
use qsl_semantics::model::key::hex;
use qsl_semantics::value::{
    native_diagnostics_catalog, CatalogEntry, CatalogRole, DefinitionLock, DefinitionReference,
    HeaderLayer, Member, ModelEntry, NodeOwner, TemporalEntry,
};
use quire_exact::{Cancel, Integer, NodeKey, Origin, Role, NODE_KEY_DOMAIN};
use quire_semantic_value::location::Location;

use super::{CheckedPackage, EmittedPackage};
use crate::checked_v2::CheckedTimedModel;

/// The FR-322 graph schema version every node carries.
const GRAPH_V2: &str = "quire.checked-semantic-graph/v2";

/// The identity preimage's own version (FR-322).
const IDENTITY_PREIMAGE_V2: &str = qsl_semantics::library::PACKAGE_ID_VERSION;

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
    #[error("protocol reference {} is bound to {}, which no written node carries", reference.digest, target.digest)]
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
            Self::TimedModelNodeSetMismatch => Code::RuntimeInvariant,
            Self::TimedModelWriterInvariant(_) | Self::RefinementSubjectNotEmitted => {
                Code::RuntimeInvariant
            }
            Self::RefinementIdentityEncoding => Code::OutputFailure,
            Self::RefinementNotEmitted => Code::UnsupportedConstruct,
            Self::UnlocatedRefinementDeclaration => Code::UnsupportedProjection,
            Self::SourceMapInvariant => Code::RuntimeInvariant,
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
            | Self::TimedModelWriterInvariant(_)
            | Self::RefinementSubjectNotEmitted
            | Self::RefinementIdentityEncoding
            | Self::UnlocatedRefinementDeclaration
            | Self::SourceMapInvariant => None,
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

/// The emitted package and the nodes it omits.
#[derive(Clone, Debug)]
pub struct Emission {
    /// The v2 bytes and their `package_id`.
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
}

impl Emission {
    /// The v2 bytes and their `package_id`.
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
    }
}

/// A `NodeRef` to `key`.
fn node_id(key: NodeKey) -> CheckedNodeId {
    CheckedNodeId {
        domain: NODE_KEY_DOMAIN.into(),
        digest: key.to_string().into(),
    }
}

/// The Contract IR node id of the wire node id `node`: the same id
/// [`emit_checked`] writes for the node.
pub fn checked_node_id(node: WireNodeId) -> CheckedNodeId {
    CheckedNodeId {
        domain: NODE_KEY_DOMAIN.into(),
        digest: node.to_string().into(),
    }
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
    dependencies: BTreeSet<CheckedNodeId>,
    annotations: BTreeSet<CheckedNodeId>,
    laws: Vec<OperationLaw>,
}

impl BodyNames {
    /// Read `body`: its stratified terms have a fixed depth (ADR-030 D-2),
    /// so this is a fixed-depth match over its leaves.
    fn of(body: &BodyTerm) -> Self {
        let mut names = Self::default();
        match body {
            BodyTerm::Application(application) => {
                let operation = &application.operation;
                names.annotations.insert(node_id(application.result_type.0));
                if let Some(declaration) = operation.member().and_then(Member::declaration) {
                    names.dependencies.insert(node_id(declaration));
                }
                // FR-380: the population node a reachability links to.
                if let Some(population) = operation.member().and_then(Member::population) {
                    names.dependencies.insert(node_id(population));
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
                names.dependencies.extend(
                    frame
                        .modifies
                        .iter()
                        .map(|entry| node_id(entry.declaration().0)),
                );
                names.dependencies.extend(
                    frame
                        .creates
                        .iter()
                        .chain(&frame.deletes)
                        .map(|node| node_id(node.0)),
                );
            }
            // QSpec FR-451: `dependencies` are exactly the unique set of every
            // `type`, `population` and `context` target of the body.
            BodyTerm::Abstraction(term) => {
                names
                    .dependencies
                    .extend(term.targets().into_iter().map(node_id));
            }
            BodyTerm::Leaf(_) | BodyTerm::Aggregate(_) => {}
        }
        for leaf in body.leaves() {
            match leaf {
                LeafTerm::Literal { ty, .. } => {
                    names.annotations.insert(node_id(ty.0));
                }
                LeafTerm::Reference { target } => {
                    names.dependencies.insert(node_id(target.0));
                }
                // FR-322-AC-37: a `dependency_reference` is never one of the
                // node's `dependencies`, and names no node of this graph.
                LeafTerm::DependencyReference { .. } => {}
            }
        }
        names
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
    id: CheckedNodeId,
    tag: CheckedNodeTag,
    semantic_type: CheckedNodeId,
    /// FR-093 rules 1 to 3, ascending by digest.
    dependencies: BTreeSet<CheckedNodeId>,
    /// Every node the written node names: its dependencies, its semantic
    /// type and its body's type annotations.
    names: BTreeSet<CheckedNodeId>,
    laws: Vec<OperationLaw>,
    /// The occurrences `check` recorded for the node, each with the
    /// location it was recorded at.
    occurrences: Recorded<'g>,
    /// The QSpec nominal preimage the node is keyed by, when it is nominal.
    nominal: Option<NominalIdentityPreimage>,
}

impl<'g> Candidate<'g> {
    fn of(node: &'g SemanticNode, occurrences: Recorded<'g>) -> Self {
        let nominal = nominal_preimage(node);
        let id = node_id(node.key());
        // A self-typed node names itself as its semantic type (FR-322).
        let semantic_type = node_id(node.semantic_type().unwrap_or(node.key()));
        let BodyNames {
            mut dependencies,
            annotations,
            laws,
        } = BodyNames::of(node.body());
        // QSpec FR-464 "owner_model": a protocol clause depends on its exact
        // owner model, besides the nodes its body reaches.
        dependencies.extend(node.owner_model().map(node_id));
        // Rule 3: a `bounded_domain` node depends on the type it bounds.
        if node.node_tag() == NodeTag::BoundedDomain {
            dependencies.insert(semantic_type.clone());
        }
        // FR-093 rule 4, QSpec's nominal joins: an enum member node depends
        // on its declaration, a dimension on its base dimensions, and a unit
        // on its dimension and target unit.
        match &nominal {
            Some(NominalIdentityPreimage::EnumMember(member)) => {
                dependencies.insert(member.declaration_node_id.clone());
            }
            Some(NominalIdentityPreimage::Dimension(dimension)) => {
                dependencies.extend(
                    dimension
                        .terms
                        .iter()
                        .map(|term| term.dimension_node_id.clone()),
                );
            }
            Some(NominalIdentityPreimage::Unit(unit)) => {
                dependencies.insert(unit.dimension_node_id.clone());
                dependencies.extend(unit.target_unit_node_id.clone());
            }
            Some(NominalIdentityPreimage::DimensionEquivalence(equivalence)) => {
                // QSpec FR-322: an equivalence depends on exactly the two
                // dimension nodes it relates.
                dependencies.extend(equivalence.dimension_node_ids.iter().cloned());
            }
            Some(NominalIdentityPreimage::EnumDeclaration(_)) | None => {}
        }
        let mut names = dependencies.clone();
        names.extend(annotations);
        names.insert(semantic_type.clone());
        names.remove(&id);
        Self {
            node,
            tag: ir_tag(node.node_tag()),
            id,
            semantic_type,
            dependencies,
            names,
            laws,
            occurrences,
            nominal,
        }
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
    fn wire_node(&self) -> Result<CheckedSemanticNodeV2, EmitRefusal> {
        let node = self.node;
        Ok(CheckedSemanticNodeV2 {
            node_id: self.id.clone(),
            schema_version: GRAPH_V2.into(),
            node_tag: self.tag.as_wire().into(),
            semantic_form: node.semantic_form().into(),
            semantic_type: self.semantic_type.clone(),
            dependencies: self.dependencies.iter().cloned().collect(),
            occurrences: self
                .occurrences
                .iter()
                .map(|(occurrence, _)| occurrence.clone())
                .collect(),
            recursion_group: node.recursion().map(|group| group.label().into()),
            nominal_identity_preimage: self.nominal.clone(),
            declaration: node.declaration().map(|segments| CheckedDeclaration {
                qualified_name: segments
                    .iter()
                    .map(|segment| segment.as_str().into())
                    .collect(),
            }),
            owner: node.owner().map(wire_owner),
            owner_model: node.owner_model().map(node_id),
            body: CheckedTerm::deserialize(node.wire_body().map_err(encoding)?)
                .map_err(encoding)?,
        })
    }
}

/// A structural node's `owner` as the v2 wire writes it (FR-093): exactly the
/// owner its `quire.structural-node/v1` preimage carries.
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
            digest: subject.digest().into(),
        },
    }
}

/// The node's QSpec nominal preimage as the v2 wire writes it, or `None`
/// for a node that is not nominal or whose owner the check did not read
/// back.
fn nominal_preimage(node: &SemanticNode) -> Option<NominalIdentityPreimage> {
    let preimage = match node.nominal() {
        None => return None,
        Some(NominalNode::EnumDeclaration(preimage)) => NominalIdentityPreimage::EnumDeclaration(
            quire_contract_model::EnumDeclarationPreimage {
                owner: nominal_owner(preimage.owner()),
                qualified_declaration: preimage
                    .qualified_declaration()
                    .iter()
                    .map(|segment| segment.as_str().into())
                    .collect(),
                ordered: preimage.is_ordered(),
                members: preimage
                    .members()
                    .iter()
                    .map(|case| case.as_str().into())
                    .collect(),
            },
        ),
        Some(NominalNode::EnumMember { declaration, case }) => {
            NominalIdentityPreimage::EnumMember(quire_contract_model::EnumMemberPreimage {
                declaration_node_id: node_id(*declaration),
                case: case.as_str().into(),
            })
        }
        Some(NominalNode::Dimension(Some(preimage))) => {
            NominalIdentityPreimage::Dimension(quire_contract_model::DimensionPreimage {
                owner: nominal_owner(preimage.owner()),
                qualified_declaration: qualified(preimage.qualified_declaration()),
                terms: preimage
                    .terms()
                    .iter()
                    .map(|(base, exponent)| quire_contract_model::DimensionTerm {
                        dimension_node_id: wire_id(*base),
                        exponent: exponent.to_string().into(),
                    })
                    .collect(),
            })
        }
        Some(NominalNode::Unit(Some(preimage))) => {
            let rational = |(numerator, denominator): (&Integer, &Integer)| CheckedRational {
                numerator: numerator.to_string().into(),
                denominator: denominator.to_string().into(),
            };
            NominalIdentityPreimage::Unit(quire_contract_model::UnitPreimage {
                owner: nominal_owner(preimage.owner()),
                qualified_declaration: qualified(preimage.qualified_declaration()),
                dimension_node_id: wire_id(preimage.dimension()),
                target_unit_node_id: preimage.target().map(wire_id),
                scale: rational(preimage.scale()),
                offset: rational(preimage.offset()),
            })
        }
        // Another owner's dimension or unit: the emission omits it
        // (`owner_is_locked`), so no preimage is written for it.
        Some(NominalNode::Dimension(None) | NominalNode::Unit(None)) => return None,
    };
    Some(preimage)
}

/// A nominal preimage's qualified declaration as the wire spells it.
fn qualified(segments: &[String]) -> Vec<Box<str>> {
    segments
        .iter()
        .map(|segment| segment.as_str().into())
        .collect()
}

/// The wire id of a node a nominal preimage names by id.
fn wire_id(id: WireNodeId) -> CheckedNodeId {
    CheckedNodeId {
        domain: NODE_KEY_DOMAIN.into(),
        digest: id.to_string().into(),
    }
}

fn nominal_owner(owner: &NodeOwner) -> NominalOwner {
    match owner {
        NodeOwner::Source(subject) => NominalOwner::Source {
            authority: subject.authority.as_str().into(),
            identity: subject.identity.as_str().into(),
        },
        NodeOwner::Definition(subject) => NominalOwner::Definition {
            authority: subject.authority.as_str().into(),
            identity: subject.identity.as_str().into(),
        },
        NodeOwner::Model(subject) => NominalOwner::Model {
            identity: subject.identity.as_str().into(),
            node: subject.node.as_str().into(),
        },
    }
}

/// Whether the lock the emitter writes selects `nominal`'s owner: a nominal
/// node without an owner joins trivially; one with an owner joins only when
/// that owner is the checked unit's `source`, the one lock entry it can
/// always join (FR-322).
fn owner_is_locked(nominal: Option<&NominalNode>, source: &RawSourceRef) -> bool {
    let owner = match nominal {
        None | Some(NominalNode::EnumMember { .. }) => return true,
        Some(NominalNode::Dimension(None) | NominalNode::Unit(None)) => return false,
        Some(NominalNode::EnumDeclaration(preimage)) => preimage.owner(),
        Some(NominalNode::Dimension(Some(preimage))) => preimage.owner(),
        Some(NominalNode::Unit(Some(preimage))) => preimage.owner(),
    };
    matches!(
        owner,
        NodeOwner::Source(subject)
            if subject.authority == source.authority() && subject.identity == source.identity()
    )
}

/// Which candidates the wire omits, and why: an unsupported form, a
/// declaration without its occurrence, a nominal owner the lock does not
/// select, a name outside the graph, and then, transitively, a name of an
/// omitted node.
fn omissions(
    candidates: &BTreeMap<CheckedNodeId, Candidate<'_>>,
    source: &RawSourceRef,
    decodes: &dyn Fn(CheckedNodeTag, &str) -> bool,
) -> Vec<OmittedNode> {
    let mut omitted: BTreeMap<CheckedNodeId, OmissionCause> = BTreeMap::new();
    for (id, candidate) in candidates {
        let cause = if !candidate.form_is_supported(decodes) {
            Some(OmissionCause::UnsupportedForm {
                node_tag: candidate.node.node_tag(),
                semantic_form: candidate.node.semantic_form(),
            })
        } else if !candidate.declaration_matches_occurrences() {
            Some(OmissionCause::DeclarationOccurrenceMismatch)
        } else if !owner_is_locked(candidate.node.nominal(), source) {
            Some(OmissionCause::UnlockedOwner)
        } else {
            candidate
                .names
                .iter()
                .find(|name| !candidates.contains_key(*name))
                .map(|absent| OmissionCause::NamesAbsentNode(absent.clone()))
        };
        if let Some(cause) = cause {
            omitted.insert(id.clone(), cause);
        }
    }
    let mut named_by: BTreeMap<&CheckedNodeId, Vec<&CheckedNodeId>> = BTreeMap::new();
    for (id, candidate) in candidates {
        for name in &candidate.names {
            named_by.entry(name).or_default().push(id);
        }
    }
    let mut pending: VecDeque<CheckedNodeId> = omitted.keys().cloned().collect();
    while let Some(gone) = pending.pop_front() {
        for &dependent in named_by.get(&gone).into_iter().flatten() {
            if !omitted.contains_key(dependent) {
                omitted.insert(
                    dependent.clone(),
                    OmissionCause::NamesOmittedNode(gone.clone()),
                );
                pending.push_back(dependent.clone());
            }
        }
    }
    omitted
        .into_iter()
        .map(|(node, cause)| OmittedNode { node, cause })
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
    candidates: &BTreeMap<CheckedNodeId, Candidate<'_>>,
    omitted: &[OmittedNode],
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
        let target = node_id(target.0);
        let written = candidates.contains_key(&target)
            && !omitted.iter().any(|omission| omission.node == target);
        if !written {
            return Err(EmitRefusal::StaleProtocolReference {
                reference: candidate.id.clone(),
                target,
            });
        }
    }
    Ok(())
}

/// FR-093's graph order: ascending by node id, except that a recursion
/// group's members are written together, in ordinal order, where the group's
/// first member by node id would be.
fn graph_order<'c, 'g>(kept: &[&'c Candidate<'g>]) -> Vec<&'c Candidate<'g>> {
    let mut groups: HashMap<[u8; 32], Vec<Option<&'c Candidate<'g>>>> = HashMap::new();
    for candidate in kept {
        let Some(recursion) = candidate.node.recursion() else {
            continue;
        };
        let members = groups
            .entry(*recursion.group())
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
                let group = *recursion.group();
                if emitted_groups.insert(group) {
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

/// `source` as the wire's `RawSourceRef`: authority, identity and the
/// digest of its bytes.
fn source_artifact(source: &RawSourceRef) -> CheckedSourceRef {
    let digest = source.digest();
    CheckedSourceRef {
        authority: source.authority().into(),
        identity: source.identity().into(),
        digest_domain: digest.domain().to_string().into(),
        digest: digest.hex().into(),
    }
}

/// The catalog's row for `role`. The closed catalog has a row for every
/// role (`the_catalog_covers_every_role_exactly_once`).
fn catalog_entry(lock: &DefinitionLock, role: CatalogRole) -> &CatalogEntry {
    lock.entry(role)
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
    lock: &DefinitionLock,
    layers: &[HeaderLayer],
    laws: &[DefinitionReference],
) -> (CheckedSelection, Vec<CheckedArtifactRef>) {
    // A law's `DefinitionRef` is also part of the application-node keys that
    // name the law.
    let edition = CheckedSelection {
        role: CheckedSelectionRole::Edition,
        definition: artifact(&catalog_entry(lock, CatalogRole::Edition).reference()),
    };
    let closure = lock.closure(layers);
    let qualification = if layers.is_empty() {
        lock.always_roles()
    } else {
        lock.qualification_roles(&closure)
    };
    let selected = closure
        .iter()
        .map(|layer| lock.layer_reference(*layer))
        .chain(
            qualification
                .iter()
                .filter(|role| **role != CatalogRole::Edition)
                .map(|role| catalog_entry(lock, *role).reference()),
        )
        // FR-361 item 3: the complete-model lock's `package_selection.always`
        // rows, for a unit whose layers hold the model layer.
        .chain(
            closure
                .contains(&HeaderLayer::ModelComplete)
                .then(|| lock.model_catalog().iter().map(ModelEntry::reference))
                .into_iter()
                .flatten(),
        )
        .chain(
            closure
                .contains(&HeaderLayer::TemporalComplete)
                .then(|| lock.temporal_catalog().iter().map(TemporalEntry::reference))
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
) -> Result<BTreeMap<NodeKey, Recorded<'_>>, EmitRefusal> {
    let mut by_node: BTreeMap<NodeKey, Recorded<'_>> = BTreeMap::new();
    for (key, origin, location) in graph.occurrences() {
        let occurrence = CheckedOccurrence {
            role: occurrence_role(key, &origin)?,
            ordinal: origin.ordinal(),
        };
        let location = if graph.parameter_span(key, &origin).is_some() {
            RecordedLocation::Parameter(graph.occurrence_region(key, &origin))
        } else {
            RecordedLocation::Expression(location)
        };
        by_node.entry(key).or_default().push((occurrence, location));
    }
    Ok(by_node)
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
    entries: &[CheckedSourceMapEntry],
) -> Result<PackageSourceMap, EmitRefusal> {
    let mut mapped = Vec::with_capacity(entries.len());
    for entry in entries {
        let node = WireNodeId::from_hex(entry.node_id.digest.as_ref())
            .ok_or(EmitRefusal::SourceMapInvariant)?;
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

/// The first FR-375 emission slice: the complete row-empty model refinement
/// body. Mapped refinements are added by later slices; this body is legal in
/// its own right and its identity is bound to the concrete model's
/// `model_root` node.
fn empty_model_refinement_node(
    refinement: &CheckedRefinement,
    region: &SourceRegion,
) -> Result<(CheckedSemanticNodeV2, Vec<CheckedSourceMapEntry>), EmitRefusal> {
    let abstract_node = node_id(
        refinement
            .abstract_subject_node()
            .ok_or(EmitRefusal::RefinementSubjectNotEmitted)?,
    );
    let concrete_node = node_id(
        refinement
            .concrete_subject_node()
            .ok_or(EmitRefusal::RefinementSubjectNotEmitted)?,
    );
    let (identity, typed_body) =
        refinement
            .v2_empty_model_emission()
            .map_err(|error| match error {
                qsl_semantics::check::RefinementV2EncodingError::MissingSubject => {
                    EmitRefusal::RefinementSubjectNotEmitted
                }
                qsl_semantics::check::RefinementV2EncodingError::Body(_)
                | qsl_semantics::check::RefinementV2EncodingError::Identity(_) => {
                    EmitRefusal::RefinementIdentityEncoding
                }
            })?;
    let semantic_type = concrete_node;
    let id = node_id(identity);
    let declaration = CheckedOccurrence {
        role: CheckedOccurrenceRole::Declaration,
        ordinal: 0,
    };
    let generated = CheckedOccurrence {
        role: CheckedOccurrenceRole::Generated,
        ordinal: 0,
    };
    let source = source_artifact(region.source());
    let source_region = CheckedSourceRegion {
        source,
        start: region.start(),
        end: region.end(),
    };
    let occurrences = vec![declaration.clone(), generated.clone()];
    let source_entries = [declaration, generated]
        .into_iter()
        .map(|occurrence| CheckedSourceMapEntry {
            node_id: id.clone(),
            role: occurrence.role,
            ordinal: occurrence.ordinal,
            regions: vec![source_region.clone()],
        })
        .collect();
    let mut dependencies = vec![abstract_node];
    dependencies.sort();
    dependencies.dedup();
    Ok((
        CheckedSemanticNodeV2 {
            owner_model: None,
            node_id: id,
            schema_version: GRAPH_V2.into(),
            node_tag: CheckedNodeTag::Relation.as_wire().into(),
            semantic_form: "refinement".into(),
            semantic_type,
            dependencies,
            occurrences,
            recursion_group: None,
            nominal_identity_preimage: None,
            declaration: None,
            owner: None,
            body: typed_body,
        },
        source_entries,
    ))
}

/// `origin`'s role as FR-322 spells it.
fn occurrence_role(node: NodeKey, origin: &Origin) -> Result<CheckedOccurrenceRole, EmitRefusal> {
    crate::checked_v2::occurrence_role(origin.role().as_str()).ok_or_else(|| {
        EmitRefusal::UnknownOccurrenceRole {
            node,
            role: origin.role().to_string(),
        }
    })
}

/// `package`'s FR-322 direct `dependency_selections`, in ascending
/// UTF-8 byte order of identity.
fn dependency_selections(package: &CheckedPackage) -> Vec<CheckedDependencySelection> {
    package
        .direct_dependency_selections()
        .map(|(identity, resolved)| CheckedDependencySelection {
            identity: identity.as_str().into(),
            package_id: semantic_id(resolved.package_id),
        })
        .collect()
}

/// `package_id` as FR-322's `PackageId` member.
fn semantic_id(package_id: PackageId) -> CheckedSemanticId {
    CheckedSemanticId {
        domain: PACKAGE_DOMAIN_V2.into(),
        algorithm: "sha256".into(),
        digest: package_id.hex().into(),
    }
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

/// Rebuild the structural identities and references of typed timed nodes
/// after their bodies are serialized. Timed graph edges are acyclic by the
/// closed FR-322 forms, so repeated digest passes propagate child-key changes
/// to their parents.
fn rekey_timed_model_nodes(
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
            let new = timed_model_node_id(node)?;
            if new != old {
                changes.push((old, new));
            }
        }
        if changes.is_empty() {
            return Ok(());
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

/// Derive one ungrouped timed-node identity through IR's public producer API.
fn timed_model_node_id(node: &CheckedSemanticNodeV2) -> Result<CheckedNodeId, EmitRefusal> {
    if node.recursion_group.is_some() {
        return Err(EmitRefusal::TimedModelWriterInvariant(
            "timed nodes cannot belong to a recursion group",
        ));
    }
    // External references retain their identities here; complete-package
    // admission checks their targets after the timed graph is re-keyed.
    let candidates = derive_structural_candidates(
        std::slice::from_ref(node),
        crate::checked::EMIT_LIMITS.max_bytes(),
        u64::MAX,
    )
    .map_err(encoding)?;
    let [candidate] = candidates.nodes() else {
        return Err(EmitRefusal::TimedModelWriterInvariant(
            "a timed node must have one structural identity",
        ));
    };
    Ok(candidate.candidate_node_id().clone())
}

/// QSpec FR-322 `model_root`, FR-322-AC-132 and AC-134: for each selected
/// domain package, in `model_selections` order, one self-typed `model` node
/// keyed by the selection's identity and digest alone, appended after every
/// other node, with one `generated` source-map entry appended after every
/// other entry. The selection records no selecting occurrence, so the entry's
/// region is the first byte of the checked unit (QSpec: the entry is a
/// producer assertion no reader verifies, and the published regions are
/// one-byte placeholders).
fn append_model_roots<'a>(
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
        let root = model_root_node(selection, key);
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
    selection: &qsl_semantics::model::domain_package::DomainPackageRef,
    key: NodeKey,
) -> CheckedSemanticNodeV2 {
    let id = node_id(key);
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
        nominal_identity_preimage: None,
        declaration: None,
        owner: Some(CheckedNodeOwner::ModelRoot {
            identity: selection.identity.as_str().into(),
            digest: hex(&selection.digest).into(),
        }),
        body: CheckedTerm::Aggregate {
            members: Vec::new(),
        },
    }
}

/// The unique digest-ascending reference targets directly named by a body.
fn body_reference_targets(body: &CheckedTerm) -> Vec<CheckedNodeId> {
    let mut targets = body
        .reference_targets()
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    targets.sort_by(|left, right| left.digest.cmp(&right.digest));
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
    rekey_timed_model_nodes(&mut nodes, &mut source_map)?;
    let inspection_source_map = inspection_source_map(&source_map)?;
    let semantic_graph = CheckedSemanticGraphV2 {
        graph_version: base.graph().graph_version.clone(),
        nodes,
    };
    let identity = base.identity_preimage();
    let preimage = CheckedPackageIdentityPreimageRefV2 {
        version: &identity.version,
        edition: &identity.edition,
        profile_selections: &identity.profile_selections,
        definition_selections: &identity.definition_selections,
        model_selections: &identity.model_selections,
        required_features: &identity.required_features,
        dependency_selections: &identity.dependency_selections,
        identity_projection: &semantic_graph.nodes,
    };
    let capability_report = base.capability_report().to_vec();
    let diagnostics = base.diagnostics().clone();
    let package = EmittedPackage::new(&preimage, |package_id: PackageId| {
        encode_checked_package_v2(
            &preimage,
            &semantic_id(package_id),
            &lock,
            &semantic_graph,
            CheckedPackageSourceCorrespondenceRef {
                source_map: &source_map,
                refinement_step_rows: None,
            },
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
#[cfg(feature = "test-support")]
#[doc(hidden)]
pub fn checked_node_id_of(key: NodeKey) -> CheckedNodeId {
    node_id(key)
}

/// Whether IR's v2 vocabulary has the form `semantic_form` under `tag`.
fn ir_decodes(tag: CheckedNodeTag, semantic_form: &str) -> bool {
    CheckedNodeKind::decode(tag, semantic_form).is_some()
}

/// Every node of `graph` read for the wire, by its id.
fn candidates_of<'g>(
    graph: &'g CheckedGraph,
    abstraction: Option<&'g AbstractionNodes>,
) -> Result<BTreeMap<CheckedNodeId, Candidate<'g>>, EmitRefusal> {
    let mut recorded = recorded_occurrences(graph)?;
    let mut candidates: BTreeMap<CheckedNodeId, Candidate<'g>> = graph
        .semantic_graph()
        .nodes()
        .map(|node| {
            let occurrences = recorded.remove(&node.key()).unwrap_or_default();
            let candidate = Candidate::of(node, occurrences);
            (candidate.id.clone(), candidate)
        })
        .collect();
    // QSpec FR-451: the relation node has one `declaration` occurrence per
    // declaration it was read from; the nodes it names that the graph does not
    // hold are `generated`, at the first.
    if let Some(nodes) = abstraction {
        let mut add = |node: &'g SemanticNode, occurrences: Recorded<'g>| {
            let candidate = Candidate::of(node, occurrences);
            candidates.entry(candidate.id.clone()).or_insert(candidate);
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
        add(nodes.relation(), declared);
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
            add(node, generated);
        }
    }
    Ok(candidates)
}

/// The body shared by [`emit_package`] and [`emit_package_with_fault`].
/// `fault` runs against each node's id, in graph order, just before that
/// node would be written; every production call site is `emit_package`,
/// which passes `|_| None`, so this behaves exactly as `emit_package`
/// always has for every non-test caller.
/// `#[string_edge]`: the nominal preimage's `form` wire discriminator selects
/// the unit-node emission branch.
#[string_edge]
fn emit_package_inner(
    package: &CheckedPackage,
    regions: impl Fn(&Location) -> Option<SourceRegion>,
    fault: impl Fn(&CheckedNodeId) -> Option<EmitRefusal>,
    cancel: &Cancel,
    decodes: &dyn Fn(CheckedNodeTag, &str) -> bool,
) -> Result<Emission, EmitRefusal> {
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
    let candidates = candidates_of(graph, abstraction.as_ref())?;
    let omitted = omissions(&candidates, graph.source(), decodes);
    stale_protocol_reference(&candidates, &omitted)?;
    let dropped: BTreeSet<&CheckedNodeId> = omitted.iter().map(|omission| &omission.node).collect();
    let kept: Vec<&Candidate<'_>> = candidates
        .values()
        .filter(|candidate| !dropped.contains(&candidate.id))
        .collect();
    if kept.is_empty() && graph.refinements().is_empty() {
        return Err(EmitRefusal::NothingToEmit { omitted });
    }
    let order = graph_order(&kept);
    let (mut source_map, mut sources) = source_map(&order, &regions)?;
    sources.insert(source_artifact(graph.source()));
    let wire_nodes = order
        .iter()
        .map(
            |candidate| match denied().or_else(|| fault(&candidate.id)) {
                Some(refusal) => Err(refusal),
                None => candidate.wire_node(),
            },
        )
        .collect::<Result<Vec<_>, _>>()?;
    let mut nodes = wire_nodes;
    let mut emitted_refinements = Vec::new();
    for refinement in graph.refinements() {
        if !refinement.v2_empty_model_body() {
            return Err(EmitRefusal::RefinementNotEmitted);
        }
        let region = graph
            .refinement_declaration_region(refinement)
            .filter(|region| region.start() < region.end())
            .ok_or(EmitRefusal::UnlocatedRefinementDeclaration)?;
        let (node, entries) = empty_model_refinement_node(refinement, &region)?;
        let insertion = nodes.partition_point(|existing| {
            existing.node_id.digest.as_bytes() < node.node_id.digest.as_bytes()
        });
        nodes.insert(insertion, node.clone());
        source_map.extend(entries);
        sources.insert(source_artifact(region.source()));
        emitted_refinements.push(refinement);
    }
    if !graph.time_sources().is_empty() {
        let text_type = literal_type(&nodes, "text")
            .unwrap_or_else(|| node_id(qsl_semantics::check::builtin_text_type_key()));
        let rational_type = node_id(qsl_semantics::check::builtin_rational_type_key());
        for (index, source) in graph.time_sources().iter().enumerate() {
            let placeholder = |kind: &str| CheckedNodeId {
                domain: NODE_KEY_DOMAIN.into(),
                digest: format!("timed-{kind}-{index}").into(),
            };
            let model = CheckedTimedModel::from_checked_time_source(
                source,
                placeholder("source"),
                placeholder("root"),
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
            if !nodes.iter().any(|node| node.node_id == text_type) {
                let occurrence = CheckedOccurrence {
                    role: CheckedOccurrenceRole::Generated,
                    ordinal: 0,
                };
                source_map.push(CheckedSourceMapEntry {
                    node_id: text_type.clone(),
                    role: occurrence.role.clone(),
                    ordinal: occurrence.ordinal,
                    regions: vec![CheckedSourceRegion {
                        source: source_artifact(graph.source()),
                        start: source_region.start(),
                        end: source_region.end(),
                    }],
                });
                nodes.push(CheckedSemanticNodeV2 {
                    owner_model: None,
                    node_id: text_type.clone(),
                    schema_version: GRAPH_V2.into(),
                    node_tag: CheckedNodeTag::ScalarType.as_wire().into(),
                    semantic_form: "text".into(),
                    semantic_type: text_type.clone(),
                    dependencies: Vec::new(),
                    occurrences: vec![occurrence],
                    recursion_group: None,
                    nominal_identity_preimage: None,
                    declaration: None,
                    owner: None,
                    body: CheckedTerm::Aggregate {
                        members: Vec::new(),
                    },
                });
            }
            if matches!(
                &source.source,
                qsl_semantics::check::TimeSourceKind::Tick { .. }
            ) && !nodes.iter().any(|node| node.node_id == rational_type)
            {
                let occurrence = CheckedOccurrence {
                    role: CheckedOccurrenceRole::Generated,
                    ordinal: 0,
                };
                source_map.push(CheckedSourceMapEntry {
                    node_id: rational_type.clone(),
                    role: occurrence.role.clone(),
                    ordinal: occurrence.ordinal,
                    regions: vec![CheckedSourceRegion {
                        source: source_artifact(graph.source()),
                        start: source_region.start(),
                        end: source_region.end(),
                    }],
                });
                nodes.push(CheckedSemanticNodeV2 {
                    owner_model: None,
                    node_id: rational_type.clone(),
                    schema_version: GRAPH_V2.into(),
                    node_tag: CheckedNodeTag::ScalarType.as_wire().into(),
                    semantic_form: "rational".into(),
                    semantic_type: rational_type.clone(),
                    dependencies: Vec::new(),
                    occurrences: vec![occurrence],
                    recursion_group: None,
                    nominal_identity_preimage: None,
                    declaration: None,
                    owner: None,
                    body: CheckedTerm::Aggregate {
                        members: Vec::new(),
                    },
                });
            }
            for nominal in &source.nominal_nodes {
                let id = node_id(nominal.key);
                if nodes.iter().any(|node| node.node_id == id) {
                    continue;
                }
                let preimage = NominalIdentityPreimage::deserialize(
                    nominal.preimage_value().map_err(encoding)?,
                )
                .map_err(encoding)?;
                let declaration_name = match &preimage {
                    NominalIdentityPreimage::Dimension(preimage) => {
                        preimage.qualified_declaration.clone()
                    }
                    NominalIdentityPreimage::Unit(preimage) => {
                        preimage.qualified_declaration.clone()
                    }
                    _ => {
                        return Err(EmitRefusal::TimedModelWriterInvariant(
                            "a timed unit graph node must be a dimension or unit",
                        ));
                    }
                };
                let semantic_type = if nominal.form == "unit" {
                    nominal.dependencies.first().copied().map(node_id).ok_or(
                        EmitRefusal::TimedModelWriterInvariant(
                            "a unit node must name its dimension",
                        ),
                    )?
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
                        source: source_artifact(graph.source()),
                        start: source_region.start(),
                        end: source_region.end(),
                    }],
                });
                nodes.push(CheckedSemanticNodeV2 {
                    owner_model: None,
                    node_id: id.clone(),
                    schema_version: GRAPH_V2.into(),
                    node_tag: CheckedNodeTag::ScalarType.as_wire().into(),
                    semantic_form: nominal.form.as_str().into(),
                    semantic_type,
                    dependencies: nominal.dependencies.iter().copied().map(node_id).collect(),
                    occurrences: vec![occurrence],
                    recursion_group: None,
                    nominal_identity_preimage: Some(preimage),
                    declaration: Some(CheckedDeclaration {
                        qualified_name: declaration_name,
                    }),
                    owner: None,
                    body: CheckedTerm::Aggregate {
                        members: Vec::new(),
                    },
                });
            }
            for node in model
                .write_nodes()
                .map_err(EmitRefusal::TimedModelWriterInvariant)?
            {
                let occurrence =
                    node.occurrences
                        .first()
                        .ok_or(EmitRefusal::TimedModelWriterInvariant(
                            "timed nodes must retain their source occurrence",
                        ))?;
                source_map.push(CheckedSourceMapEntry {
                    node_id: node.node_id.clone(),
                    role: occurrence.role.clone(),
                    ordinal: occurrence.ordinal,
                    regions: vec![CheckedSourceRegion {
                        source: source_artifact(graph.source()),
                        start: source_region.start(),
                        end: source_region.end(),
                    }],
                });
                nodes.push(node);
            }
        }
        rekey_timed_model_nodes(&mut nodes, &mut source_map)?;
    }
    append_model_roots(
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
    // The IEEE trigger includes a declared float with no IEEE application.
    // Keep the selected, admitted policy even when no node needs its law.
    if let Some(profile) = graph.scope().ieee_profile_selection() {
        laws.push(profile.definition().clone());
    }

    let catalog = DefinitionLock::pinned();
    let (edition, definition_selections) =
        catalog_selections(&catalog, graph.header_layers(), &laws);
    let model_selections: Vec<CheckedDomainPackageRef> = graph
        .model_selections()
        .iter()
        .map(|selection| CheckedDomainPackageRef {
            identity: selection.identity.as_str().into(),
            digest_domain: DOMAIN_PACKAGE_DIGEST.into(),
            digest: hex(&selection.digest).into(),
        })
        .collect();
    let root = catalog_entry(&catalog, CatalogRole::Root).identity;
    let lock = CheckedPackageLockV2 {
        sources: sources.into_iter().collect(),
        edition: edition.clone(),
        profile_selections: profile_selections.clone(),
        definition_selections: definition_selections.clone(),
        model_selections: model_selections.clone(),
        required_features: vec![root.into()],
        dependency_selections: dependency_selections(package),
    };
    let semantic_graph = CheckedSemanticGraphV2 {
        graph_version: GRAPH_V2.into(),
        nodes,
    };
    let inspection_source_map = inspection_source_map(&source_map)?;
    let preimage = CheckedPackageIdentityPreimageRefV2 {
        version: IDENTITY_PREIMAGE_V2,
        edition: &edition,
        profile_selections: &profile_selections,
        definition_selections: &definition_selections,
        model_selections: &model_selections,
        required_features: &lock.required_features,
        dependency_selections: &lock.dependency_selections,
        identity_projection: &semantic_graph.nodes,
    };
    let capability_report = [CheckedCapability {
        feature: root.into(),
        disposition: CheckedCapabilityDisposition::Available,
    }];
    let diagnostics = CheckedDiagnosticsV2 {
        catalog: diagnostics_catalog(),
        entries: Vec::new(),
    };
    let package = EmittedPackage::new(&preimage, |package_id: PackageId| {
        encode_checked_package_v2(
            &preimage,
            &semantic_id(package_id),
            &lock,
            &semantic_graph,
            CheckedPackageSourceCorrespondenceRef {
                source_map: &source_map,
                refinement_step_rows: None,
            },
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
    })
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
