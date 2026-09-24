// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    dead_code,
    reason = "no production caller yet: the S4 pipeline that calls emit_package with source regions is ADR-011 §4's round trip (QSL-6 slice S3); until then only this crate's tests call it"
)]
//! ADR-011 T-8 (M-4, QSL-6), slice S1b: the S4 v2 emitter, [`CheckedPackage`]
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
//! - `edition` and `definition_selections` come from QSL's `DefinitionLock`
//!   catalog: the `edition` role, every other always-selected role, and each
//!   law `DefinitionRef` a node body names.
//! - `sources` are the checked unit's `RawSourceRef` (`CheckedGraph::source`)
//!   and any other raw source an occurrence region names.
//! - `required_features` is `quire.value.complete/v1` (ADR-011 §2.4) and
//!   `capability_report` reports it available.
//! - `profile_selections` and `model_selections` are empty: the `Value`
//!   family selects no profile, and a model-bearing package's domain
//!   packages are not carried on `CheckedGraph`.
//! - `dependency_selections` is empty (ADR-011 §2.4: the QSpec schema types
//!   it as a definition selection, which is a schema defect).
//!
//! # Source regions
//!
//! FR-322 maps every occurrence to at least one byte region. `check` records
//! each occurrence at a `check::Location` (a declaration and a child path),
//! and the S2 form spans that turn a `Location` into a region do not exist
//! yet (ADR-013 O-12). The caller therefore supplies that conversion; an
//! occurrence it cannot place, or places at an empty region, refuses the
//! whole emission ([`EmitRefusal::UnlocatedOccurrence`]).
//!
//! # Omitted nodes
//!
//! A node whose (`node_tag`, `semantic_form`) IR's v2 vocabulary does not
//! hold is omitted: at the pinned IR revision that is `value`/`parameter`
//! (IR-280) and `scalar_type`/`compound_unit`. So is a node that names a node
//! the checked graph does not hold (an enum declaration or unit node, which
//! lowering names by key but does not build), an application node inside a
//! recursion group (the pinned IR keys it by the bare group label, not
//! FR-322's `{ordinal, size}`), and every node that names an omitted one. Every other node is written (FR-062-AC-9: a refusal is per
//! item). [`Emission::omitted`] lists each omitted node and its cause.
//!
//! The wire envelope is a local struct: IR keeps its own
//! `CheckedPackageWireV2` private (IR-238). IR's reader admits exactly its
//! member set, so the round-trip tests fail if the two diverge.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use quire_contract_ir::{
    CheckedArtifactRef, CheckedCapability, CheckedDeclaration, CheckedDiagnosticsV2, CheckedNodeId,
    CheckedNodeTag, CheckedOccurrence, CheckedOccurrenceRole, CheckedPackageIdentityPreimageV2,
    CheckedPackageLockV2, CheckedRevision, CheckedSelection, CheckedSemanticGraphV2,
    CheckedSemanticId, CheckedSemanticNodeV2, CheckedSourceMapEntry, CheckedSourceRegion,
    CHECKED_PACKAGE_V2, PACKAGE_DOMAIN_V2,
};
use serde::Serialize;

use qsl_foundation::source::provenance::{RawSourceRef, SourceRegion};
use qsl_foundation::Code;
use qsl_semantics::check::{CheckedGraph, Location, NodeTag, SemanticNode, SemanticTerm};
use qsl_semantics::library::PackageId;
use qsl_semantics::value::IDENTITY_LIMITS;
use qsl_semantics::value::{
    CatalogEntry, CatalogRole, DefinitionLock, DefinitionReference, Member,
};
use quire_exact::{NodeKey, Origin, NODE_KEY_DOMAIN};

use super::{CheckedPackage, EmittedPackage};

/// The `value` form whose node never carries a `declaration` (FR-322).
const ENUM_VALUE_FORM: &str = "enum_value";

/// The FR-322 graph schema version every node carries.
const GRAPH_V2: &str = "quire.checked-semantic-graph/v2";

/// The identity preimage's own version (FR-322).
const IDENTITY_PREIMAGE_V2: &str = qsl_semantics::library::PACKAGE_ID_VERSION;

/// The diagnostics catalog the package's (empty) diagnostics are qualified
/// by: QSpec's `quire.native.diagnostics/v1` at `1-draft.6`.
fn diagnostics_catalog() -> CheckedArtifactRef {
    CheckedArtifactRef {
        authority: "agent-ix".into(),
        identity: "quire.native.diagnostics/v1".into(),
        revision: CheckedRevision {
            namespace: "quire-draft".into(),
            value: "1-draft.6".into(),
        },
        digest_domain: "quire.definition.bytes/v1".into(),
        // Informational; no reader verifies it yet.
        digest: "9a55506569ff8f1aa4c97d04fe8c8d14aa07ecb40452f5ea9409f38da3a41365".into(),
        export: None,
    }
}

/// Why the v2 emitter writes no bytes for `package` at all. A node that
/// cannot be written is omitted instead ([`OmittedNode`]).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum EmitRefusal {
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
}

impl From<quire_canonical::Error> for EmitRefusal {
    fn from(error: quire_canonical::Error) -> Self {
        encoding(error)
    }
}

impl EmitRefusal {
    /// The catalog code (FR-010).
    pub(crate) fn code(&self) -> Code {
        match self {
            Self::NothingToEmit { .. }
            | Self::UnlocatedOccurrence { .. }
            | Self::UnknownOccurrenceRole { .. } => Code::UnsupportedProjection,
            Self::Encoding { .. } => Code::OutputFailure,
        }
    }
}

/// One node the wire omits, and why.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OmittedNode {
    /// The omitted node.
    pub(crate) node: CheckedNodeId,
    /// Why it is omitted.
    pub(crate) cause: OmissionCause,
}

/// Why a node is omitted from the wire.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum OmissionCause {
    /// IR's v2 vocabulary has no such form for the tag (IR-280: no
    /// `value`/`parameter`).
    UnsupportedForm {
        /// The node's tag.
        node_tag: NodeTag,
        /// The node's form.
        semantic_form: &'static str,
    },
    /// The node carries a `declaration` and no `declaration` occurrence, or
    /// the reverse; FR-322 requires both or neither. `check` records no
    /// declaration occurrence for a declared type (it has no location for
    /// one), so a declared record or tuple is omitted.
    DeclarationOccurrenceMismatch,
    /// An application node inside a recursion group. The pinned IR reader
    /// re-derives its key from the bare `recursion_group` label rather than
    /// FR-322's `{ordinal, size}` and `group_reference` terms, so it would
    /// refuse the package as `stale-node-key`. The whole group is omitted
    /// through [`Self::NamesOmittedNode`].
    RecursiveApplication,
    /// The node names a node the checked graph does not hold.
    NamesAbsentNode(CheckedNodeId),
    /// The node names a node the wire omits.
    NamesOmittedNode(CheckedNodeId),
}

/// The emitted package and the nodes it omits.
#[derive(Clone, Debug)]
pub(crate) struct Emission {
    /// The v2 bytes and their `package_id`.
    pub(crate) package: EmittedPackage,
    /// Every node of the checked graph the wire omits, ascending by node id.
    pub(crate) omitted: Vec<OmittedNode>,
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

/// What one node body names: the FR-093 rule 1 and 2 dependencies, the
/// type annotations (literal `type`s and application `result_type`s), and
/// the law definitions of its applications.
#[derive(Default)]
struct BodyNames {
    dependencies: BTreeSet<CheckedNodeId>,
    annotations: BTreeSet<CheckedNodeId>,
    laws: Vec<DefinitionReference>,
}

impl BodyNames {
    /// Walk `body`, every term at any depth.
    fn of(body: &SemanticTerm) -> Self {
        let mut names = Self::default();
        let mut pending = vec![body];
        while let Some(term) = pending.pop() {
            match term {
                SemanticTerm::Literal { ty, .. } => {
                    names.annotations.insert(node_id(ty.0));
                }
                SemanticTerm::Reference { target } => {
                    names.dependencies.insert(node_id(target.0));
                }
                SemanticTerm::Application {
                    operation,
                    result_type,
                    arguments,
                    ..
                } => {
                    names.annotations.insert(node_id(result_type.0));
                    if let Some(declaration) = operation.member().and_then(Member::declaration) {
                        names.dependencies.insert(node_id(declaration));
                    }
                    let leaf_laws = operation.leaves().iter().flat_map(|leaf| &leaf.laws);
                    names.laws.extend(
                        operation
                            .laws()
                            .iter()
                            .chain(leaf_laws)
                            .map(|law| law.definition.clone()),
                    );
                    pending.extend(arguments);
                }
                SemanticTerm::Aggregate { members } => pending.extend(members),
                SemanticTerm::Binding { value, .. } => pending.push(value),
            }
        }
        names
    }
}

/// A node's recorded occurrences, each with the location `check` recorded
/// it at.
type Recorded<'g> = Vec<(CheckedOccurrence, &'g Location)>;

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
    laws: Vec<DefinitionReference>,
    /// The occurrences `check` recorded for the node, each with the
    /// location it was recorded at.
    occurrences: Recorded<'g>,
}

impl<'g> Candidate<'g> {
    fn of(node: &'g SemanticNode, occurrences: Recorded<'g>) -> Self {
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
        }
    }

    fn form_is_supported(&self) -> bool {
        self.tag.forms().contains(&self.node.semantic_form())
    }

    /// FR-322's declaration rule, as IR applies it: `declaration` is absent
    /// on an `expression`, `relation`, `state`, `temporal` or
    /// `correspondence` node and on a `value`/`enum_value` node, and on every
    /// other node present exactly when it has a `declaration` occurrence.
    fn declaration_matches_occurrences(&self) -> bool {
        let forced_absent = matches!(
            self.tag,
            CheckedNodeTag::Expression
                | CheckedNodeTag::Relation
                | CheckedNodeTag::State
                | CheckedNodeTag::Temporal
                | CheckedNodeTag::Correspondence
        ) || (self.tag == CheckedNodeTag::Value
            && self.node.semantic_form() == ENUM_VALUE_FORM);
        let declared = !forced_absent
            && self
                .occurrences
                .iter()
                .any(|(occurrence, _)| occurrence.role == CheckedOccurrenceRole::Declaration);
        declared == self.node.declaration().is_some()
    }

    fn is_recursive_application(&self) -> bool {
        self.node.recursion().is_some()
            && matches!(self.node.body(), SemanticTerm::Application { .. })
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
            nominal_identity_preimage: None,
            declaration: node.declaration().map(|segments| CheckedDeclaration {
                qualified_name: segments
                    .iter()
                    .map(|segment| segment.as_str().into())
                    .collect(),
            }),
            body: node.wire_body().map_err(encoding)?,
        })
    }
}

/// Which candidates the wire omits, and why: an unsupported form, a
/// declaration without its occurrence, a name outside the graph, and then,
/// transitively, a name of an omitted node.
fn omissions(candidates: &BTreeMap<CheckedNodeId, Candidate<'_>>) -> Vec<OmittedNode> {
    let mut omitted: BTreeMap<CheckedNodeId, OmissionCause> = BTreeMap::new();
    for (id, candidate) in candidates {
        let cause = if !candidate.form_is_supported() {
            Some(OmissionCause::UnsupportedForm {
                node_tag: candidate.node.node_tag(),
                semantic_form: candidate.node.semantic_form(),
            })
        } else if !candidate.declaration_matches_occurrences() {
            Some(OmissionCause::DeclarationOccurrenceMismatch)
        } else if candidate.is_recursive_application() {
            Some(OmissionCause::RecursiveApplication)
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

fn artifact(reference: &DefinitionReference) -> CheckedArtifactRef {
    CheckedArtifactRef {
        authority: reference.authority.as_str().into(),
        identity: reference.identity.as_str().into(),
        revision: CheckedRevision {
            namespace: reference.revision.namespace.as_str().into(),
            value: reference.revision.value.as_str().into(),
        },
        digest_domain: reference.digest_domain.as_str().into(),
        digest: reference.digest.as_str().into(),
        export: None,
    }
}

fn source_artifact(source: &RawSourceRef) -> CheckedArtifactRef {
    let digest = source.digest();
    CheckedArtifactRef {
        authority: source.authority().into(),
        identity: source.identity().into(),
        revision: CheckedRevision {
            namespace: source.revision().namespace().into(),
            value: source.revision().value().into(),
        },
        digest_domain: digest.domain().to_string().into(),
        digest: digest.hex().into(),
        export: None,
    }
}

/// The catalog's row for `role`. The closed catalog has a row for every
/// role (`the_catalog_covers_every_role_exactly_once`).
fn catalog_entry(role: CatalogRole) -> &'static CatalogEntry {
    DefinitionLock::pinned()
        .entry(role)
        .expect("the closed catalog has a row for every role")
}

/// The lock's `edition` and `definition_selections`: the catalog's edition,
/// then its other always-selected roles in catalog order, then each law
/// definition the bodies name that is not already selected.
fn catalog_selections(laws: &[DefinitionReference]) -> (CheckedSelection, Vec<CheckedArtifactRef>) {
    // Informational; no reader verifies these digests yet.
    let edition = CheckedSelection {
        role: CatalogRole::Edition.as_str().into(),
        definition: artifact(&catalog_entry(CatalogRole::Edition).reference()),
    };
    let mut definitions: Vec<CheckedArtifactRef> = DefinitionLock::pinned()
        .always_roles()
        .iter()
        .filter(|role| **role != CatalogRole::Edition)
        .map(|role| artifact(&catalog_entry(*role).reference()))
        .collect();
    for law in laws {
        let law = artifact(law);
        if !definitions.contains(&law) {
            definitions.push(law);
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
        by_node.entry(key).or_default().push((occurrence, location));
    }
    Ok(by_node)
}

/// The source map of the written nodes, in graph order, and the raw sources
/// its regions name.
fn source_map(
    order: &[&Candidate<'_>],
    regions: &impl Fn(&Location) -> Option<SourceRegion>,
) -> Result<(Vec<CheckedSourceMapEntry>, BTreeSet<CheckedArtifactRef>), EmitRefusal> {
    let mut entries = Vec::new();
    let mut sources = BTreeSet::new();
    for candidate in order {
        for (occurrence, location) in &candidate.occurrences {
            // FR-322 regions are non-empty half-open intervals: an empty
            // region places the occurrence nowhere.
            let region = regions(location)
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

/// The `quire.checked-package/v2` envelope. See the module doc for why it is
/// local.
#[derive(Serialize)]
struct WireV2<'a> {
    contract_version: &'static str,
    identity_preimage: &'a CheckedPackageIdentityPreimageV2,
    package_id: CheckedSemanticId,
    lock: &'a CheckedPackageLockV2,
    semantic_graph: &'a CheckedSemanticGraphV2,
    source_map: &'a [CheckedSourceMapEntry],
    capability_report: &'a [CheckedCapability],
    diagnostics: &'a CheckedDiagnosticsV2,
}

fn encoding(error: impl std::fmt::Display) -> EmitRefusal {
    EmitRefusal::Encoding {
        reason: error.to_string(),
    }
}

/// Emit `package` as `quire.checked-package/v2` bytes (FR-322). `regions`
/// places each occurrence `check` recorded (see the module doc).
pub(crate) fn emit_package(
    package: &CheckedPackage,
    regions: impl Fn(&Location) -> Option<SourceRegion>,
) -> Result<Emission, EmitRefusal> {
    let graph = package.graph();
    let mut recorded = recorded_occurrences(graph)?;
    let candidates: BTreeMap<CheckedNodeId, Candidate<'_>> = graph
        .semantic_graph()
        .nodes()
        .map(|node| {
            let occurrences = recorded.remove(&node.key()).unwrap_or_default();
            let candidate = Candidate::of(node, occurrences);
            (candidate.id.clone(), candidate)
        })
        .collect();
    let omitted = omissions(&candidates);
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
        .map(|candidate| candidate.wire_node())
        .collect::<Result<Vec<_>, _>>()?;
    let laws: Vec<DefinitionReference> = order
        .iter()
        .flat_map(|candidate| candidate.laws.iter().cloned())
        .collect();

    let (edition, definition_selections) = catalog_selections(&laws);
    let root = catalog_entry(CatalogRole::Root).identity;
    let lock = CheckedPackageLockV2 {
        sources: sources.into_iter().collect(),
        edition: edition.clone(),
        profile_selections: Vec::new(),
        definition_selections: definition_selections.clone(),
        model_selections: Vec::new(),
        required_features: vec![root.into()],
        dependency_selections: Vec::new(),
    };
    let preimage = CheckedPackageIdentityPreimageV2 {
        version: IDENTITY_PREIMAGE_V2.into(),
        edition,
        profile_selections: Vec::new(),
        definition_selections,
        model_selections: Vec::new(),
        required_features: lock.required_features.clone(),
        dependency_selections: Vec::new(),
        identity_projection: nodes.iter().map(Into::into).collect(),
    };
    let semantic_graph = CheckedSemanticGraphV2 {
        graph_version: GRAPH_V2.into(),
        nodes,
    };
    let capability_report = [CheckedCapability {
        feature: root.into(),
        disposition: "available".into(),
    }];
    let diagnostics = CheckedDiagnosticsV2 {
        catalog: diagnostics_catalog(),
        entries: Vec::new(),
    };
    let package = EmittedPackage::new(&preimage, |package_id: PackageId| {
        let wire = WireV2 {
            contract_version: CHECKED_PACKAGE_V2,
            identity_preimage: &preimage,
            package_id: CheckedSemanticId {
                domain: PACKAGE_DOMAIN_V2.into(),
                algorithm: "sha256".into(),
                digest: package_id.hex().into(),
            },
            lock: &lock,
            semantic_graph: &semantic_graph,
            source_map: &source_map,
            capability_report: &capability_report,
            diagnostics: &diagnostics,
        };
        quire_canonical::to_vec(&wire, IDENTITY_LIMITS).map_err(EmitRefusal::from)
    })?;
    Ok(Emission { package, omitted })
}

#[cfg(test)]
mod tests;
