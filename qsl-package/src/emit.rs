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

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use quire_contract_model::{
    encode_checked_package_v2, CheckedArtifactRef, CheckedCapability, CheckedCapabilityDisposition,
    CheckedDeclaration, CheckedDependencySelection, CheckedDiagnosticsV2, CheckedDomainPackageRef,
    CheckedNodeId, CheckedNodeKind, CheckedNodeOwner, CheckedNodeTag, CheckedOccurrence,
    CheckedOccurrenceRole, CheckedPackageEvidence, CheckedPackageIdentityPreimageRefV2,
    CheckedPackageLockV2, CheckedRational, CheckedSelection, CheckedSelectionRole,
    CheckedSemanticGraphV2, CheckedSemanticId, CheckedSemanticNodeV2, CheckedSourceMapEntry,
    CheckedSourceRef, CheckedSourceRegion, NominalIdentityPreimage, NominalOwner, ProtocolForm,
    ValueForm, DOMAIN_PACKAGE_DIGEST, PACKAGE_DOMAIN_V2,
};

use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::{RawSourceRef, SourceRegion};
use qsl_foundation::Code;
use qsl_semantics::check::{
    BodyTerm, CheckedGraph, LawRole, LeafTerm, NodeTag, NominalNode, OperationLaw, Owner,
    SemanticNode,
};
use qsl_semantics::library::PackageId;
use qsl_semantics::model::key::hex;
use qsl_semantics::value::{
    native_diagnostics_catalog, CatalogEntry, CatalogRole, DefinitionLock, DefinitionReference,
    HeaderLayer, Member, ModelEntry, NodeOwner,
};
use quire_exact::{Cancel, Integer, NodeKey, Origin, NODE_KEY_DOMAIN};
use quire_semantic_value::location::Location;

use super::{CheckedPackage, EmittedPackage};

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
    /// The caller's [`quire_exact::Cancel`] handle was cancelled, and the
    /// emitter stopped at its next charge (FR-276).
    #[error("the emitter was cancelled ({cause:?})")]
    Cancelled {
        /// Why the handle was cancelled.
        cause: quire_exact::CancelCause,
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
            Self::StaleProtocolReference { .. } => Code::MissingDeclaration,
            Self::Cancelled { .. } => Code::Cancelled,
        }
    }

    /// The catalog cause, where the code has one (FR-010).
    pub fn cause(&self) -> Option<&'static str> {
        match self {
            Self::StaleProtocolReference { .. } => Some("missing-name"),
            Self::NothingToEmit { .. }
            | Self::UnlocatedOccurrence { .. }
            | Self::UnknownOccurrenceRole { .. }
            | Self::Encoding { .. }
            | Self::Cancelled { .. } => None,
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
            body: node.wire_body().map_err(encoding)?,
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
    let mut groups: BTreeMap<String, Vec<&'c Candidate<'g>>> = BTreeMap::new();
    for candidate in kept {
        if let Some(recursion) = candidate.node.recursion() {
            groups.entry(recursion.label()).or_default().push(candidate);
        }
    }
    for members in groups.values_mut() {
        members.sort_by_key(|member| member.node.recursion().map(|group| group.ordinal()));
    }
    let mut order = Vec::with_capacity(kept.len());
    for candidate in kept {
        match candidate.node.recursion() {
            None => order.push(*candidate),
            Some(recursion) => {
                if let Some(members) = groups.remove(&recursion.label()) {
                    order.extend(members);
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
/// QSpec FR-001 selects for those layers; then each law definition the
/// bodies name that is not already selected.
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

/// `origin`'s role as FR-322 spells it.
fn occurrence_role(node: NodeKey, origin: &Origin) -> Result<CheckedOccurrenceRole, EmitRefusal> {
    crate::checked_v2::occurrence_role(origin.role().as_str()).ok_or_else(|| {
        EmitRefusal::UnknownOccurrenceRole {
            node,
            role: origin.role().to_string(),
        }
    })
}

/// `package`'s FR-322 `dependency_selections`, in its closure's ascending
/// UTF-8 byte order of identity.
fn dependency_selections(package: &CheckedPackage) -> Vec<CheckedDependencySelection> {
    package
        .dependency_selections()
        .iter()
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
pub fn emit_checked(package: &CheckedPackage) -> Result<Emission, EmitRefusal> {
    emit_checked_with_cancel(package, &Cancel::new())
}

/// [`emit_checked`] under the caller's [`Cancel`] handle, polled once at
/// entry and at every node it writes (FR-276). A cancelled handle stops the
/// emitter with [`EmitRefusal::Cancelled`] and writes no bytes.
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
    if emission.omitted().is_empty() {
        package.remember_emitted_id(emission.package().package_id());
    }
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
fn candidates_of(
    graph: &CheckedGraph,
) -> Result<BTreeMap<CheckedNodeId, Candidate<'_>>, EmitRefusal> {
    let mut recorded = recorded_occurrences(graph)?;
    Ok(graph
        .semantic_graph()
        .nodes()
        .map(|node| {
            let occurrences = recorded.remove(&node.key()).unwrap_or_default();
            let candidate = Candidate::of(node, occurrences);
            (candidate.id.clone(), candidate)
        })
        .collect())
}

/// The body shared by [`emit_package`] and [`emit_package_with_fault`].
/// `fault` runs against each node's id, in graph order, just before that
/// node would be written; every production call site is `emit_package`,
/// which passes `|_| None`, so this behaves exactly as `emit_package`
/// always has for every non-test caller.
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
    let candidates = candidates_of(graph)?;
    let omitted = omissions(&candidates, graph.source(), decodes);
    stale_protocol_reference(&candidates, &omitted)?;
    let dropped: BTreeSet<&CheckedNodeId> = omitted.iter().map(|omission| &omission.node).collect();
    let kept: Vec<&Candidate<'_>> = candidates
        .values()
        .filter(|candidate| !dropped.contains(&candidate.id))
        .collect();
    if kept.is_empty() {
        return Err(EmitRefusal::NothingToEmit { omitted });
    }
    let order = graph_order(&kept);
    let (source_map, mut sources) = source_map(&order, &regions)?;
    sources.insert(source_artifact(graph.source()));
    let nodes = order
        .iter()
        .map(
            |candidate| match denied().or_else(|| fault(&candidate.id)) {
                Some(refusal) => Err(refusal),
                None => candidate.wire_node(),
            },
        )
        .collect::<Result<Vec<_>, _>>()?;
    let (profile_selections, laws) = split_laws(&order);

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
            &source_map,
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
