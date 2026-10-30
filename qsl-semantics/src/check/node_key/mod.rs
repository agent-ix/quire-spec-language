// SPDX-License-Identifier: AGPL-3.0-or-later
//! Structural references used by the checked semantic graph.
//!
//! QSpec FR-322 defines a node identity as `{package, path, ordinal}`.
//! Lowering carries source or producer-assigned identities and resolves local
//! placeholders before admission. Body terms retain typed semantic content;
//! they no longer derive identities from canonical preimages or digests.

use quire_canonical::FixedShape;
use serde::Serialize;

use crate::value::semantic_node::{NodeOwner, OwnerSubject};
use quire_semantic_value::semantic_node::IdentityRefusal;

use qsl_foundation::absence::AbsenceMode;
use quire_exact::{Identifier, Integer, Rational, RoundingMode, TextProfile, Timestamp, Uuid};

use crate::value::definition::DefinitionReference;
use crate::value::member::{AcceptedPoint, Member};
use quire_exact::NodeKey;

/// The owner of a declared node (ADR-013 O-04): the declaring source unit's
/// `SourceOwner{kind: "source", authority, identity}`, a required E3 input
/// (FR-091). A builtin or anonymous node carries none.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, FixedShape)]
pub struct SourceOwner {
    authority: String,
    identity: String,
    kind: SourceOwnerKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, FixedShape)]
#[serde(rename_all = "snake_case")]
enum SourceOwnerKind {
    Source,
}

/// Why a [`SourceOwner`] cannot be formed: QSpec's `SourceOwner` members
/// are `Nonempty`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum InvalidSourceOwner {
    /// `authority` is empty.
    #[error("the source owner's authority is empty")]
    EmptyAuthority,
    /// `identity` is empty.
    #[error("the source owner's identity is empty")]
    EmptyIdentity,
}

/// FR-001: a unit's owner is the authority and identity of the
/// `RawSourceRef` it was admitted under; its revision and digest take no
/// part. A `RawSourceRef`'s members are non-empty, so this cannot fail.
impl From<&qsl_foundation::source::provenance::RawSourceRef> for SourceOwner {
    fn from(source: &qsl_foundation::source::provenance::RawSourceRef) -> Self {
        Self {
            authority: source.authority().to_owned(),
            identity: source.identity().to_owned(),
            kind: SourceOwnerKind::Source,
        }
    }
}

impl SourceOwner {
    /// The source owner `{kind: "source", authority, identity}`.
    pub fn new(
        authority: impl Into<String>,
        identity: impl Into<String>,
    ) -> Result<Self, InvalidSourceOwner> {
        let authority = authority.into();
        let identity = identity.into();
        if authority.is_empty() {
            return Err(InvalidSourceOwner::EmptyAuthority);
        }
        if identity.is_empty() {
            return Err(InvalidSourceOwner::EmptyIdentity);
        }
        Ok(Self {
            authority,
            identity,
            kind: SourceOwnerKind::Source,
        })
    }

    /// This owner as the nominal preimages' `{kind: "source", authority,
    /// identity}` owner subject (FR-091 "Enum declarations").
    pub(crate) fn node_owner(&self, package_name: &str) -> NodeOwner {
        NodeOwner::Source(OwnerSubject {
            authority: self.authority.clone(),
            identity: self.identity.clone(),
            package_name: package_name.to_owned(),
        })
    }

    /// The owner's authority.
    pub fn authority(&self) -> &str {
        &self.authority
    }

    /// The owner's identity.
    pub fn identity(&self) -> &str {
        &self.identity
    }
}

/// The owner of a model-owned node (FR-094, ADR-013 O-04, C-02): QSpec's
/// content-only `ModelOwner{kind: "model", identity, node}`. `identity` is
/// the declaring domain package's `DomainPackageRef` identity; `node` is the
/// declaration's IR node identity. The owner carries no version, and the
/// source `model` selects the package by identity alone, so
/// a change of the domain package that leaves a node's content unchanged keys
/// the same node.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, FixedShape)]
pub struct ModelOwner {
    identity: String,
    kind: ModelOwnerKind,
    node: String,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, FixedShape)]
#[serde(rename_all = "snake_case")]
enum ModelOwnerKind {
    Model,
}

/// Why a [`ModelOwner`] cannot be formed: QSpec's `ModelOwner` members are
/// `Nonempty`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
pub enum InvalidModelOwner {
    /// `identity` is empty.
    #[error("the model owner's identity is empty")]
    EmptyIdentity,
    /// `node` is empty.
    #[error("the model owner's node is empty")]
    EmptyNode,
}

impl ModelOwner {
    /// The model owner `{kind: "model", identity, node}`.
    pub fn new(
        identity: impl Into<String>,
        node: impl Into<String>,
    ) -> Result<Self, InvalidModelOwner> {
        let identity = identity.into();
        let node = node.into();
        if identity.is_empty() {
            return Err(InvalidModelOwner::EmptyIdentity);
        }
        if node.is_empty() {
            return Err(InvalidModelOwner::EmptyNode);
        }
        Ok(Self {
            identity,
            kind: ModelOwnerKind::Model,
            node,
        })
    }

    /// The declaring domain package's identity.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// The declaration's IR node identity.
    pub fn node(&self) -> &str {
        &self.node
    }
}

/// The owner of a selected model package's `model_root` node (QSpec FR-322
/// `ModelRootOwner`): the selected package's identity, never a declaration
/// or an alias.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, FixedShape)]
pub struct ModelRootOwner {
    identity: String,
    kind: ModelRootOwnerKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, FixedShape)]
#[serde(rename_all = "snake_case")]
enum ModelRootOwnerKind {
    ModelRoot,
}

impl ModelRootOwner {
    /// The root owner `{kind: "model_root", identity}`.
    pub fn new(identity: impl Into<String>) -> Result<Self, InvalidModelOwner> {
        let identity = identity.into();
        if identity.is_empty() {
            return Err(InvalidModelOwner::EmptyIdentity);
        }
        Ok(Self {
            identity,
            kind: ModelRootOwnerKind::ModelRoot,
        })
    }

    /// The selected package's identity.
    pub fn identity(&self) -> &str {
        &self.identity
    }
}

/// A structural node's `owner` (FR-092, FR-094): a source-declared node's
/// [`SourceOwner`] or a model-owned node's [`ModelOwner`]. Serialized as the
/// owner itself; its `kind` member tells the two apart.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, FixedShape)]
#[serde(untagged)]
pub enum Owner {
    /// A node declared by a QSL source unit.
    Source(SourceOwner),
    /// A node standing for a domain package's declaration or clause.
    Model(ModelOwner),
    /// A selected model package's root node.
    ModelRoot(ModelRootOwner),
}

/// A local draft handle; its value never reaches a checked package.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct PlaceholderId(pub(crate) usize);

/// A node reference during lowering. The pending form must be resolved to
/// an admitted structural identity before the body crosses a wire boundary.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum NodeRef {
    /// A source-supplied or producer-assigned structural node identity.
    Admitted(NodeKey),
    /// A local group member whose structural identity is not yet assigned.
    Pending(PlaceholderId),
}

impl From<NodeKey> for NodeRef {
    fn from(node: NodeKey) -> Self {
        Self::Admitted(node)
    }
}

impl NodeRef {
    /// Borrow the admitted node identity after placeholder resolution.
    pub fn admitted(&self) -> Option<&NodeKey> {
        match self {
            Self::Admitted(node) => Some(node),
            Self::Pending(_) => None,
        }
    }

    /// Require a resolved structural identity at an admission boundary.
    fn require_admitted(&self) -> Result<&NodeKey, NodeKeyRefusal> {
        self.admitted().ok_or(NodeKeyRefusal::UnresolvedPlaceholder)
    }

    /// Apply a graph key substitution only to an admitted reference. Pending
    /// handles remain local until the group allocator resolves them.
    fn map_admitted(&self, map: &mut impl FnMut(NodeKey) -> NodeKey) -> Self {
        match self {
            Self::Admitted(node) => Self::Admitted(map(node.clone())),
            Self::Pending(handle) => Self::Pending(*handle),
        }
    }
}

/// The preimage schema's closed `node_tag` vocabulary.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, FixedShape)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(rename_all = "snake_case")]
pub enum NodeTag {
    /// `scalar_type`.
    ScalarType,
    /// `composite_type`.
    CompositeType,
    /// `bounded_domain`.
    BoundedDomain,
    /// `value`.
    Value,
    /// `expression`.
    Expression,
    /// `function`.
    Function,
    /// `model`.
    Model,
    /// `relation`.
    Relation,
    /// `state`.
    State,
    /// `temporal`.
    Temporal,
    /// `protocol`.
    Protocol,
    /// `claim`.
    Claim,
    /// `correspondence`.
    Correspondence,
    /// `hybrid`: a hybrid model, mode or transition (QSpec FR-481).
    Hybrid,
    /// `timed_model`: a timed model's `clock_place` template, which a
    /// `presence_read` over the place references (QSpec FR-322
    /// "Presence").
    TimedModel,
}

impl NodeTag {
    /// The closed v2 node tag spelling used as a derived path segment.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ScalarType => "scalar_type",
            Self::CompositeType => "composite_type",
            Self::BoundedDomain => "bounded_domain",
            Self::Value => "value",
            Self::Expression => "expression",
            Self::Function => "function",
            Self::Model => "model",
            Self::Relation => "relation",
            Self::State => "state",
            Self::Temporal => "temporal",
            Self::Protocol => "protocol",
            Self::Claim => "claim",
            Self::Correspondence => "correspondence",
            Self::Hybrid => "hybrid",
            Self::TimedModel => "timed_model",
        }
    }
}

/// The Leaf stratum of QSpec FR-322's v2 body grammar (ADR-030 D-2): a
/// term that holds no other term. Its depth is fixed, so it encodes through
/// `quire-canonical`'s serde path (its `flatten` and tag only make the
/// derived depth an overestimate).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
// No `deny_unknown_fields`: serde does not combine it with the literal's
// `flatten`, and the conformance test compares re-encoded bytes with the
// published ones, so a dropped member fails there instead.
pub enum LeafTerm {
    /// A typed literal: its kind and JSON value come from [`LiteralValue`].
    Literal {
        /// The literal's type node.
        ty: NodeRef,
        /// The literal's value.
        value: LiteralValue,
    },
    /// A reference to another node.
    Reference {
        /// The referenced node.
        target: NodeRef,
    },
    /// A reference to an imported node, retaining that node's own package
    /// name inside its structural identity.
    DependencyReference {
        /// The node's structural identity in its owning package.
        node: NodeRef,
    },
}

/// `{term: "binding", name, value}`: a named `value` of a lower stratum.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Binding<V> {
    /// The bound name (schema `Nonempty`; an empty name is refused).
    pub name: String,
    /// The bound term.
    pub value: V,
}

/// A member of a [`GroupTerm`]: a Leaf, or a binding whose value is a Leaf.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum GroupMember {
    /// A Leaf.
    Leaf(LeafTerm),
    /// A binding of a Leaf.
    Binding(Binding<LeafTerm>),
}

/// The Group stratum: an `aggregate` whose members are each a Leaf or a
/// binding of a Leaf.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GroupTerm {
    /// The members, in order.
    pub members: Vec<GroupMember>,
}

/// A member of a [`TupleTerm`]: a Leaf or a Group.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TupleMember {
    /// A Leaf.
    Leaf(LeafTerm),
    /// A Group.
    Group(GroupTerm),
}

/// The Tuple stratum: an `aggregate` whose members are each a Leaf or a
/// Group.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TupleTerm {
    /// The members, in order.
    pub members: Vec<TupleMember>,
}

/// The value of a [`MemberTerm`] binding: a Leaf, a Group, a Tuple, or a
/// closed nested aggregate where a QSpec body production requires one (for
/// example FR-477's execution-entry bindings). Other bindings retain the
/// FR-322 grammar's existing strata.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum BindingValue {
    /// A Leaf.
    Leaf(LeafTerm),
    /// A Group.
    Group(GroupTerm),
    /// A Tuple.
    Tuple(TupleTerm),
    /// An aggregate with member-level bindings. This is needed by nested
    /// closed body productions such as FR-477's execution entries.
    Aggregate(AggregateTerm),
    /// The accepted `ClauseBinding.execution_point` of an `operation_anchor`
    /// body's `execution_point` binding (QSpec FR-342): the IR's closed
    /// four-variant shape, which is not a term and holds no node reference.
    ExecutionPoint(AcceptedPoint),
}

/// The Member stratum: an application argument or a body aggregate's
/// member.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum MemberTerm {
    /// A Leaf.
    Leaf(LeafTerm),
    /// A Group.
    Group(GroupTerm),
    /// A binding of a Leaf, a Group or a Tuple.
    Binding(Binding<BindingValue>),
}

/// An operation application: a Body production whose arguments are each a
/// Member.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ApplicationTerm {
    /// The operator class.
    pub operator: Operator,
    /// The catalogued operation and its laws.
    pub operation: Operation,
    /// The application's result type node.
    pub result_type: NodeRef,
    /// The operands, in order.
    pub arguments: Vec<MemberTerm>,
}

/// An ordered aggregate: a Body production whose members are each a Member.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AggregateTerm {
    /// The members, in order.
    pub members: Vec<MemberTerm>,
}

/// A `state`/`frame` node's own body shape (QSpec FR-340), valid only as a
/// node's body. It contains no `application`, so `node_key` always keys a
/// frame node through the FR-092 structural preimage, never the application
/// one.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FrameTerm {
    /// Every field or relationship the operation may write, ascending by
    /// (declaring node digest, field name), a relationship's name being
    /// empty (FR-340).
    pub modifies: Vec<FrameModifies>,
    /// Every object type or process the operation may create, ascending by
    /// node digest.
    pub creates: Vec<NodeRef>,
    /// Every object type or process the operation may delete, ascending by
    /// node digest.
    pub deletes: Vec<NodeRef>,
    /// The scoped deletes: `receiver` first, then each reference parameter
    /// by name as UTF-8 bytes (QSpec FR-340). Always present.
    pub deletes_scoped: Vec<FrameDeletesScoped>,
}

/// A node's body: the Body stratum of QSpec FR-322's v2 body grammar
/// (ADR-030 D-2, FR-258). No stratum names itself or a higher one, so every
/// body, and its node-key preimage, nests a depth the grammar fixes, and
/// every composite subterm is its own node, reached by `reference`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum BodyTerm {
    /// A Leaf.
    Leaf(LeafTerm),
    /// An operation application.
    Application(ApplicationTerm),
    /// An ordered aggregate of Members.
    Aggregate(AggregateTerm),
    /// A `state`/`frame` node's body.
    Frame(FrameTerm),
    /// A `correspondence`/`abstraction_relation` node's body (QSpec FR-451).
    Abstraction(AbstractionTerm),
}

/// One `modifies` entry of a `state`/`frame` node's body (QSpec FR-340's
/// `FrameModifiesEntry`): a field or a navigation relationship the
/// operation may write.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum FrameModifies {
    /// `{kind: "field", declaration, name, scope}`.
    Field(FrameField),
    /// `{kind: "relationship", declaration}`: the node key of the declared
    /// `relation`/`relationship` node, standing for every link of it.
    Relationship(NodeRef),
}

impl FrameModifies {
    /// The node this entry names: a field's declaring node, or the
    /// relationship's own node.
    pub fn declaration(&self) -> NodeRef {
        match self {
            Self::Field(field) => field.declaration.clone(),
            Self::Relationship(declaration) => declaration.clone(),
        }
    }

    /// The field's own name; `None` for a relationship.
    pub fn field_name(&self) -> Option<&str> {
        match self {
            Self::Field(field) => Some(field.name()),
            Self::Relationship(_) => None,
        }
    }
}

/// The scope of a frame field entry (QSpec FR-340): `every-object` sorts
/// before `receiver`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FrameScope {
    /// `modifies f`: every object whose effective type has the field.
    EveryObject,
    /// `modifies self.f`: the operation's receiver only.
    Receiver,
}

/// One `deletes_scoped` entry of a `state`/`frame` node's body (QSpec
/// FR-340): `deletes self` or `deletes p`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FrameDeletesScoped {
    /// `{kind: "receiver"}`.
    Receiver,
    /// `{kind: "parameter", name}`.
    Parameter(String),
}

/// A `modifies` entry naming a field: the field's declaring node, its own
/// name and its scope.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FrameField {
    declaration: NodeRef,
    name: String,
    scope: FrameScope,
}

impl FrameField {
    /// `{kind: "field", declaration, name, scope}`.
    pub(crate) fn new(declaration: NodeKey, name: impl Into<String>, scope: FrameScope) -> Self {
        Self {
            declaration: NodeRef::Admitted(declaration),
            name: name.into(),
            scope,
        }
    }

    /// The entry's scope.
    pub fn scope(&self) -> FrameScope {
        self.scope
    }

    /// The field's declaring node.
    pub fn declaration(&self) -> NodeRef {
        self.declaration.clone()
    }

    /// The field's own name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl LeafTerm {
    /// `{term: "reference", target}`.
    pub(crate) fn reference(target: NodeKey) -> Self {
        Self::reference_node(target.into())
    }

    /// A reference to an admitted identity or lowering-local placeholder.
    pub(crate) fn reference_node(target: NodeRef) -> Self {
        Self::Reference { target }
    }

    /// `{term: "literal", type, value_kind, value}`.
    pub(crate) fn literal(ty: NodeKey, value: LiteralValue) -> Self {
        Self::literal_node(ty.into(), value)
    }

    /// A literal typed by an admitted identity or lowering-local placeholder.
    pub(crate) fn literal_node(ty: NodeRef, value: LiteralValue) -> Self {
        Self::Literal { ty, value }
    }

    /// The node key this leaf names in its own graph: a `reference`'s
    /// target or a literal's `type`. A dependency's node is no key of this
    /// graph.
    pub fn key(&self) -> Option<NodeKey> {
        match self {
            Self::Literal { ty, .. } => ty.admitted().cloned(),
            Self::Reference { target } => target.admitted().cloned(),
            Self::DependencyReference { .. } => None,
        }
    }

    /// This leaf with the node key it names replaced by `map`'s image.
    fn map_refs(&self, map: &mut impl FnMut(&NodeRef) -> NodeRef) -> Self {
        match self {
            Self::Literal { ty, value } => Self::Literal {
                ty: map(&ty),
                value: value.clone(),
            },
            Self::Reference { target } => Self::Reference {
                target: map(&target),
            },
            Self::DependencyReference { node } => Self::DependencyReference { node: map(node) },
        }
    }
}

impl<V> Binding<V> {
    /// `{term: "binding", name, value}`.
    pub(crate) fn new(name: impl Into<String>, value: V) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

impl Binding<LeafTerm> {
    fn map_refs(&self, map: &mut impl FnMut(&NodeRef) -> NodeRef) -> Self {
        Self::new(self.name.clone(), self.value.map_refs(map))
    }
}

impl GroupMember {
    /// The member's Leaf, bound or not.
    pub fn leaf(&self) -> &LeafTerm {
        match self {
            Self::Leaf(leaf) | Self::Binding(Binding { value: leaf, .. }) => leaf,
        }
    }

    fn map_refs(&self, map: &mut impl FnMut(&NodeRef) -> NodeRef) -> Self {
        match self {
            Self::Leaf(leaf) => Self::Leaf(leaf.map_refs(map)),
            Self::Binding(binding) => Self::Binding(binding.map_refs(map)),
        }
    }
}

impl GroupTerm {
    /// `{term: "aggregate", members}` over Group members.
    pub(crate) fn new(members: Vec<GroupMember>) -> Self {
        Self { members }
    }

    /// An aggregate of the references to `targets`, in order.
    pub(crate) fn references(targets: impl IntoIterator<Item = NodeKey>) -> Self {
        Self::references_nodes(targets.into_iter().map(NodeRef::from))
    }

    /// An aggregate of admitted references and pending local handles.
    pub(crate) fn references_nodes(targets: impl IntoIterator<Item = NodeRef>) -> Self {
        Self::new(
            targets
                .into_iter()
                .map(|target| GroupMember::Leaf(LeafTerm::reference_node(target)))
                .collect(),
        )
    }

    /// Every Leaf of this Group, in order.
    pub fn leaves(&self) -> impl Iterator<Item = &LeafTerm> {
        self.members.iter().map(GroupMember::leaf)
    }

    fn map_refs(&self, map: &mut impl FnMut(&NodeRef) -> NodeRef) -> Self {
        Self::new(
            self.members
                .iter()
                .map(|member| member.map_refs(map))
                .collect(),
        )
    }
}

impl TupleMember {
    /// Every Leaf of this member, in order.
    pub fn leaves(&self) -> Vec<&LeafTerm> {
        match self {
            Self::Leaf(leaf) => vec![leaf],
            Self::Group(group) => group.leaves().collect(),
        }
    }

    fn map_refs(&self, map: &mut impl FnMut(&NodeRef) -> NodeRef) -> Self {
        match self {
            Self::Leaf(leaf) => Self::Leaf(leaf.map_refs(map)),
            Self::Group(group) => Self::Group(group.map_refs(map)),
        }
    }
}

impl TupleTerm {
    /// `{term: "aggregate", members}` over Tuple members.
    pub(crate) fn new(members: Vec<TupleMember>) -> Self {
        Self { members }
    }

    /// Every Leaf of this Tuple, in order.
    pub fn leaves(&self) -> Vec<&LeafTerm> {
        self.members.iter().flat_map(TupleMember::leaves).collect()
    }

    fn map_refs(&self, map: &mut impl FnMut(&NodeRef) -> NodeRef) -> Self {
        Self::new(
            self.members
                .iter()
                .map(|member| member.map_refs(map))
                .collect(),
        )
    }
}

impl BindingValue {
    /// A Group of one named Leaf: `{name: leaf}`, the shape of an optional
    /// record field's and a fold step's value.
    pub(crate) fn named_leaf(name: impl Into<String>, leaf: LeafTerm) -> Self {
        Self::Group(GroupTerm::new(vec![GroupMember::Binding(Binding::new(
            name, leaf,
        ))]))
    }

    /// Every Leaf this value holds, in order.
    pub fn leaves(&self) -> Vec<&LeafTerm> {
        match self {
            Self::Leaf(leaf) => vec![leaf],
            Self::Group(group) => group.leaves().collect(),
            Self::Tuple(tuple) => tuple.leaves(),
            Self::Aggregate(aggregate) => aggregate
                .members
                .iter()
                .flat_map(MemberTerm::leaves)
                .collect(),
            Self::ExecutionPoint(_) => Vec::new(),
        }
    }

    fn map_refs(&self, map: &mut impl FnMut(&NodeRef) -> NodeRef) -> Self {
        match self {
            Self::Leaf(leaf) => Self::Leaf(leaf.map_refs(map)),
            Self::Group(group) => Self::Group(group.map_refs(map)),
            Self::Tuple(tuple) => Self::Tuple(tuple.map_refs(map)),
            Self::Aggregate(aggregate) => Self::Aggregate(AggregateTerm {
                members: aggregate
                    .members
                    .iter()
                    .map(|member| member.map_refs(map))
                    .collect(),
            }),
            Self::ExecutionPoint(point) => Self::ExecutionPoint(point.clone()),
        }
    }
}

impl MemberTerm {
    /// `{term: "reference", target}` as a Member.
    pub(crate) fn reference(target: NodeKey) -> Self {
        Self::reference_node(target.into())
    }

    /// A Member reference to an admitted identity or pending local handle.
    pub(crate) fn reference_node(target: NodeRef) -> Self {
        Self::Leaf(LeafTerm::reference_node(target))
    }

    /// `{term: "binding", name, value}` as a Member.
    pub(crate) fn binding(name: impl Into<String>, value: BindingValue) -> Self {
        Self::Binding(Binding::new(name, value))
    }

    /// `{term: "binding", name, value}` of a Leaf, as a Member.
    pub(crate) fn bound(name: impl Into<String>, value: LeafTerm) -> Self {
        Self::binding(name, BindingValue::Leaf(value))
    }

    /// Every Leaf this Member holds, in order.
    pub fn leaves(&self) -> Vec<&LeafTerm> {
        match self {
            Self::Leaf(leaf) => vec![leaf],
            Self::Group(group) => group.leaves().collect(),
            Self::Binding(binding) => binding.value.leaves(),
        }
    }

    fn map_refs(&self, map: &mut impl FnMut(&NodeRef) -> NodeRef) -> Self {
        match self {
            Self::Leaf(leaf) => Self::Leaf(leaf.map_refs(map)),
            Self::Group(group) => Self::Group(group.map_refs(map)),
            Self::Binding(binding) => Self::Binding(Binding::new(
                binding.name.clone(),
                binding.value.map_refs(map),
            )),
        }
    }
}

impl AggregateTerm {
    /// An aggregate whose members may bind leaves, groups, tuples or further
    /// aggregates, as required by closed nested body productions.
    pub(crate) fn new(members: Vec<MemberTerm>) -> Self {
        Self { members }
    }
}

impl BodyTerm {
    /// `{term: "literal", type, value_kind, value}` as a body.
    pub(crate) fn literal(ty: NodeKey, value: LiteralValue) -> Self {
        Self::literal_node(ty.into(), value)
    }

    /// A literal body typed by an admitted identity or pending handle.
    pub(crate) fn literal_node(ty: NodeRef, value: LiteralValue) -> Self {
        Self::Leaf(LeafTerm::literal_node(ty, value))
    }

    /// `{term: "aggregate", members}` as a body.
    pub(crate) fn aggregate(members: Vec<MemberTerm>) -> Self {
        Self::Aggregate(AggregateTerm { members })
    }

    /// `{term: "application", operator, operation, result_type,
    /// arguments}` as a body.
    pub(crate) fn application(
        operator: Operator,
        operation: Operation,
        result_type: NodeKey,
        arguments: Vec<MemberTerm>,
    ) -> Self {
        Self::Application(ApplicationTerm {
            operator,
            operation,
            result_type: NodeRef::Admitted(result_type),
            arguments,
        })
    }

    /// `{term: "frame", modifies, creates, deletes, deletes_scoped}` (QSpec FR-340).
    /// `creates` and `deletes` are each ordered lists of object type or
    /// process node keys.
    pub(crate) fn frame(
        modifies: Vec<FrameModifies>,
        creates: Vec<NodeKey>,
        deletes: Vec<NodeKey>,
        deletes_scoped: Vec<FrameDeletesScoped>,
    ) -> Self {
        Self::Frame(FrameTerm {
            modifies,
            creates: creates.into_iter().map(NodeRef::Admitted).collect(),
            deletes: deletes.into_iter().map(NodeRef::Admitted).collect(),
            deletes_scoped,
        })
    }

    /// The application this body is, when it is one.
    pub fn application_term(&self) -> Option<&ApplicationTerm> {
        match self {
            Self::Application(application) => Some(application),
            Self::Leaf(_) | Self::Aggregate(_) | Self::Frame(_) | Self::Abstraction(_) => None,
        }
    }

    /// Every Leaf of this body, in order: the body itself, an
    /// application's arguments' or an aggregate's members' leaves. A frame
    /// body holds none. The body's depth is fixed, so this is a fixed-depth
    /// match, not a walk.
    pub fn leaves(&self) -> Vec<&LeafTerm> {
        match self {
            Self::Leaf(leaf) => vec![leaf],
            Self::Application(ApplicationTerm { arguments, .. })
            | Self::Aggregate(AggregateTerm { members: arguments }) => {
                arguments.iter().flat_map(MemberTerm::leaves).collect()
            }
            Self::Frame(_) | Self::Abstraction(_) => Vec::new(),
        }
    }

    /// Call `visit` with every node key this body names (FR-092 "names"):
    /// each `reference` target, literal `type`, application `result_type`
    /// and operation member `declaration`, and a frame's entries.
    pub(crate) fn for_each_key(
        &self,
        visit: &mut impl FnMut(NodeKey),
    ) -> Result<(), NodeKeyRefusal> {
        match self {
            Self::Application(ApplicationTerm {
                operation,
                result_type,
                ..
            }) => {
                if let Some(declaration) = operation.member.as_ref().and_then(Member::declaration) {
                    visit(declaration);
                }
                if let Some(population) = operation.member.as_ref().and_then(Member::population) {
                    visit(population);
                }
                visit(result_type.require_admitted()?.clone());
            }
            Self::Frame(FrameTerm {
                modifies,
                creates,
                deletes,
                deletes_scoped: _,
            }) => {
                for entry in modifies {
                    visit(entry.declaration().require_admitted()?.clone());
                }
                for reference in creates.iter().chain(deletes) {
                    visit(reference.require_admitted()?.clone());
                }
            }
            Self::Abstraction(term) => {
                for target in term.targets()? {
                    visit(target);
                }
            }
            Self::Leaf(_) | Self::Aggregate(_) => {}
        }
        for leaf in self.leaves() {
            match leaf {
                LeafTerm::Literal { ty, .. } => visit(ty.require_admitted()?.clone()),
                LeafTerm::Reference { target } => visit(target.require_admitted()?.clone()),
                LeafTerm::DependencyReference { .. } => {}
            }
        }
        Ok(())
    }

    /// Call `visit` with every node reference this body holds, admitted or
    /// pending (a reference, a literal or result type, a dependency
    /// reference, a frame entry, an abstraction target, an operation
    /// member's declaration or population), without copying the body.
    pub(crate) fn for_each_ref(&self, visit: &mut impl FnMut(&NodeRef)) {
        match self {
            Self::Application(ApplicationTerm {
                operation,
                result_type,
                ..
            }) => {
                if let Some(member) = operation.member.as_ref() {
                    for key in [member.declaration(), member.population()]
                        .into_iter()
                        .flatten()
                    {
                        visit(&NodeRef::Admitted(key));
                    }
                }
                visit(result_type);
            }
            Self::Frame(FrameTerm {
                modifies,
                creates,
                deletes,
                deletes_scoped: _,
            }) => {
                for entry in modifies {
                    visit(&entry.declaration());
                }
                for reference in creates.iter().chain(deletes) {
                    visit(reference);
                }
            }
            Self::Abstraction(term) => term.for_each_ref(visit),
            Self::Leaf(_) | Self::Aggregate(_) => {}
        }
        for leaf in self.leaves() {
            match leaf {
                LeafTerm::Literal { ty, .. } => visit(ty),
                LeafTerm::Reference { target } => visit(target),
                LeafTerm::DependencyReference { node } => visit(node),
            }
        }
    }

    /// This body with every admitted node key it names replaced by `map`'s
    /// image; pending handles are kept.
    pub(crate) fn map_keys(&self, map: &mut impl FnMut(NodeKey) -> NodeKey) -> Self {
        // An admitted reference always maps to an admitted one here, so the
        // refusal of `map_refs` cannot arise.
        self.map_refs(&mut |reference| reference.map_admitted(&mut *map))
            .unwrap_or_else(|_| self.clone())
    }

    /// This body with every node reference it holds replaced by `map`'s
    /// image. An operation member's declaration or population is a node
    /// key: it takes the image only when that is admitted.
    ///
    /// # Errors
    ///
    /// [`NodeKeyRefusal::UnresolvedPlaceholder`] when `map` returns a pending
    /// handle for an operation member's declaration or population, which are
    /// always admitted node keys.
    pub(crate) fn map_refs(
        &self,
        map: &mut impl FnMut(&NodeRef) -> NodeRef,
    ) -> Result<Self, NodeKeyRefusal> {
        let mut pending_member = false;
        let mapped = self.map_refs_in(map, &mut pending_member);
        if pending_member {
            Err(NodeKeyRefusal::UnresolvedPlaceholder)
        } else {
            Ok(mapped)
        }
    }

    fn map_refs_in(
        &self,
        map: &mut impl FnMut(&NodeRef) -> NodeRef,
        pending_member: &mut bool,
    ) -> Self {
        match self {
            Self::Leaf(leaf) => Self::Leaf(leaf.map_refs(map)),
            Self::Application(ApplicationTerm {
                operator,
                operation,
                result_type,
                arguments,
            }) => {
                let mut operation = operation.clone();
                operation.member = operation
                    .member
                    .map(|member| map_member(member, map, pending_member));
                Self::Application(ApplicationTerm {
                    operator: *operator,
                    operation,
                    result_type: map(&result_type),
                    arguments: arguments
                        .iter()
                        .map(|argument| argument.map_refs(map))
                        .collect(),
                })
            }
            Self::Aggregate(AggregateTerm { members }) => Self::Aggregate(AggregateTerm {
                members: members.iter().map(|member| member.map_refs(map)).collect(),
            }),
            Self::Frame(FrameTerm {
                modifies,
                creates,
                deletes,
                deletes_scoped,
            }) => Self::Frame(FrameTerm {
                modifies: modifies
                    .iter()
                    .map(|entry| match entry {
                        FrameModifies::Field(field) => FrameModifies::Field(FrameField {
                            declaration: map(&field.declaration),
                            name: field.name.clone(),
                            scope: field.scope,
                        }),
                        FrameModifies::Relationship(declaration) => {
                            FrameModifies::Relationship(map(&declaration))
                        }
                    })
                    .collect(),
                creates: creates.iter().map(|node| map(&node)).collect(),
                deletes: deletes.iter().map(|node| map(&node)).collect(),
                deletes_scoped: deletes_scoped.clone(),
            }),
            Self::Abstraction(term) => Self::Abstraction(term.map_refs(map)),
        }
    }
}

/// Remap only the admitted declaration references of an operation member.
fn map_key(
    map: &mut impl FnMut(&NodeRef) -> NodeRef,
    pending_member: &mut bool,
    key: NodeKey,
) -> NodeKey {
    match map(&NodeRef::Admitted(key.clone())) {
        NodeRef::Admitted(mapped) => mapped,
        NodeRef::Pending(_) => {
            *pending_member = true;
            key
        }
    }
}

fn map_member(
    member: Member,
    map: &mut impl FnMut(&NodeRef) -> NodeRef,
    pending_member: &mut bool,
) -> Member {
    match member {
        Member::Field { declaration, name } => Member::Field {
            declaration: map_key(map, pending_member, declaration),
            name,
        },
        Member::Position {
            declaration,
            position,
        } => Member::Position {
            declaration: map_key(map, pending_member, declaration),
            position,
        },
        Member::Element { declaration } => Member::Element {
            declaration: map_key(map, pending_member, declaration),
        },
        Member::RelationshipEnd {
            declaration,
            name,
            population,
        } => Member::RelationshipEnd {
            declaration: map_key(map, pending_member, declaration),
            name,
            population: population.map(|population| map_key(map, pending_member, population)),
        },
        Member::Operation { declaration, name } => Member::Operation {
            declaration: map_key(map, pending_member, declaration),
            name,
        },
        Member::TypeArgument { declaration } => Member::TypeArgument {
            declaration: map_key(map, pending_member, declaration),
        },
        Member::ProfileOperator { operator } => Member::ProfileOperator { operator },
        Member::StateClause {
            clause,
            execution_point,
        } => Member::StateClause {
            clause,
            execution_point,
        },
        Member::TemporalInterval { interval } => Member::TemporalInterval { interval },
        timed @ Member::TimedInterval(_) => timed,
        Member::Fairness {
            kind,
            granularity,
            declaration,
            name,
        } => Member::Fairness {
            kind,
            granularity,
            declaration: map_key(map, pending_member, declaration),
            name,
        },
    }
}

/// A literal's `value_kind` and `value`, typed together so the two cannot
/// disagree. FR-092 fixes each spelling.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum LiteralValue {
    /// `boolean`: a JSON boolean.
    Boolean(bool),
    /// `integer`: the canonical decimal string.
    Integer(Integer),
    /// `integer`: a JSON number, as QSpec FR-322 "Timed-model nodes" spells
    /// a `clock_place`'s path positions, collection bounds and row
    /// positions.
    Count(u64),
    /// `rational`: `"n/d"`, reduced, positive denominator.
    Rational(Rational),
    /// `float32_bits`: the eight lowercase hexadecimal binary32 bits.
    Float32Bits(u32),
    /// `float64_bits`: the sixteen lowercase hexadecimal binary64 bits.
    Float64Bits(u64),
    /// `quantity`: the exact magnitude, spelled as the decimal integer when
    /// the denominator is 1 and otherwise `"n/d"` in lowest terms (QSpec
    /// FR-322 "Scalar literal values").
    Quantity(Rational),
    /// `text`: the text as a JSON string.
    Text(String),
    /// `string`: the exact scalar sequence as a JSON string, with no
    /// normalization.
    String(String),
    /// `uuid`: the canonical lowercase `8-4-4-4-12` text.
    Uuid(Uuid),
    /// `timestamp`: the canonical decimal string of the signed 128-bit
    /// nanosecond count.
    Timestamp(Timestamp),
    /// `none`: `null`.
    None,
    /// `enum`: the member's case identifier, the body of an enum member's
    /// QSpec `value`/`enum_value` node.
    Enum(String),
}

/// The schema's closed application `operator` vocabulary.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Operator {
    /// `call`.
    Call,
    /// `unary`.
    Unary,
    /// `binary`.
    Binary,
    /// `conditional`.
    Conditional,
    /// `let`.
    Let,
    /// `quantify`.
    Quantify,
    /// `collection`.
    Collection,
    /// `query`.
    Query,
    /// `convert`.
    Convert,
    /// `pre`.
    Pre,
    /// `present`.
    Present,
    /// `value`.
    Value,
    /// `deref`.
    Deref,
    /// `reaches`.
    Reaches,
    /// `temporal`: the body root of a `temporal`/`temporal_clause` node
    /// (QSpec FR-370).
    Temporal,
    /// `temporal_formula`: the body root of a `temporal`/`formula` node
    /// (QSpec FR-370).
    TemporalFormula,
    /// `temporal_fairness`: the body root of a `temporal`/`fairness` node
    /// (QSpec FR-370).
    TemporalFairness,
    /// `protocol_control`.
    ProtocolControl,
    /// `state_transition`.
    StateTransition,
    /// `state_clause`.
    StateClause,
    /// `claim`.
    Claim,
    /// `case`: the body root of an `expression`/`case` node (QSpec FR-440).
    Case,
}

impl Operator {
    /// The `semantic_form` an application node rooted at this operator class
    /// carries (FR-093 "One node per checked expression").
    pub(crate) fn semantic_form(self) -> &'static str {
        match self {
            Self::Call => "call",
            Self::Unary => "unary",
            Self::Binary => "binary",
            Self::Conditional => "conditional",
            Self::Let => "let",
            Self::Quantify => "quantify",
            Self::Collection => "collection",
            Self::Query => "query",
            Self::Convert => "conversion",
            Self::Pre => "pre_read",
            Self::Present => "presence_read",
            Self::Value => "value_read",
            Self::Deref => "deref",
            Self::Reaches => "reachability",
            Self::Temporal => "temporal_clause",
            Self::TemporalFormula => "formula",
            Self::TemporalFairness => "fairness",
            Self::ProtocolControl => "protocol_control",
            Self::StateTransition => "state_transition",
            // Never reached today: a `state_clause` node is inserted
            // directly with its own explicit `NodeTag::State`/
            // `"state_clause"` (`Lowering::state_clause`, `check/lowering/
            // state.rs`), not through the generic expression-application
            // pipeline this function serves. The arm exists because
            // `Operator` is one closed enum matched exhaustively here
            // (FR-088-AC-4's own pattern): a variant added to it fails to
            // compile until every match over it, including this one, names
            // its own case.
            Self::StateClause => "state_clause",
            Self::Claim => "claim",
            Self::Case => "case",
        }
    }
}

/// The schema's `Operation`: a catalogued operation with its laws, mode,
/// member and leaves.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Operation {
    /// The `quire.checked-operation-catalog/v1` identity; catalog membership
    /// is the reader's check, not the key's.
    pub(crate) identity: String,
    /// The operation's profile laws.
    pub(crate) laws: Vec<OperationLaw>,
    /// The operation's mode, or `None` (JSON `null`).
    pub(crate) mode: Option<OperationMode>,
    /// The operation's member, or `None` (JSON `null`).
    pub(crate) member: Option<Member>,
    /// The operation's leaves.
    pub(crate) leaves: Vec<OperationLeaf>,
}

impl Operation {
    /// The catalogued operation identity.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// The operation's profile laws, in order.
    pub fn laws(&self) -> &[OperationLaw] {
        &self.laws
    }

    /// The operation's mode, when it has one.
    pub fn mode(&self) -> Option<&OperationMode> {
        self.mode.as_ref()
    }

    /// The operation's member, when it names one.
    pub fn member(&self) -> Option<&Member> {
        self.member.as_ref()
    }

    /// The operation's leaves, in order.
    pub fn leaves(&self) -> &[OperationLeaf] {
        &self.leaves
    }

    /// The catalogued operation `identity` with no law, mode, member or leaf.
    pub(crate) fn plain(identity: &str) -> Self {
        Self {
            identity: identity.to_owned(),
            laws: Vec::new(),
            mode: None,
            member: None,
            leaves: Vec::new(),
        }
    }
}

/// The schema's `OperationLaw`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct OperationLaw {
    /// The law's role.
    pub role: LawRole,
    /// The selected definition, digest included.
    pub definition: DefinitionReference,
}

/// The schema's closed `OperationLaw.role` vocabulary.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LawRole {
    /// `integer_division`.
    IntegerDivision,
    /// `ieee_profile`.
    IeeeProfile,
    /// `text_profile`.
    TextProfile,
    /// `temporal_profile`.
    TemporalProfile,
    /// `protocol_profile`.
    ProtocolProfile,
}

impl LawRole {
    /// The role's schema spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IntegerDivision => "integer_division",
            Self::IeeeProfile => "ieee_profile",
            Self::TextProfile => "text_profile",
            Self::TemporalProfile => "temporal_profile",
            Self::ProtocolProfile => "protocol_profile",
        }
    }
}

/// The schema's non-null `OperationMode` (`{kind, value}`), carrying the
/// crate's own mode enums.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum OperationMode {
    /// `rounding`.
    Rounding(RoundingMode),
    /// `text_profile`.
    TextProfile(TextProfile),
    /// `absence`.
    Absence(AbsenceMode),
}

/// The schema's `OperationLeaf`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct OperationLeaf {
    /// The leaf's path from the compared type.
    pub path: Vec<LeafSegment>,
    /// The leaf's profile laws.
    pub laws: Vec<OperationLaw>,
    /// The leaf's mode, or `None` (JSON `null`).
    pub mode: Option<OperationMode>,
}

/// One schema `LeafSegment`: `field:<identifier>`, `member:<identifier>`,
/// `position:<n>` or `inner`, and FR-093's `recursion:<d>`, which ends a recursion leaf's path
/// (a QSL proposal, ADR-013 QC-24).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum LeafSegment {
    /// `field:<name>`.
    Field(Identifier),
    /// `position:<n>`.
    Position(u64),
    /// `inner`.
    Inner,
    /// `member:<name>`: a union member, QSpec FR-440's "Structural leaves".
    Member(Identifier),
    /// `recursion:<d>`: the path reenters the composite it entered after
    /// `d` segments.
    Recursion(usize),
}

/// A [`LeafSegment`] serializes as one string.

/// Semantic content retained during lowering before a producer-assigned
/// structural identity is attached.
#[derive(Clone, Debug)]
pub(crate) struct NodeInput<'a> {
    /// The node's owner: for a structural node, a [`SourceOwner`] exactly
    /// when `declaration` is present, a [`ModelOwner`] for a model-owned node
    /// (whose `declaration` is absent), else none; an application node has
    /// none.
    pub(crate) owner: Option<&'a Owner>,
    /// The node's v2 `node_tag`.
    pub(crate) node_tag: NodeTag,
    /// The node's v2 `semantic_form`, spelled as on the wire (schema
    /// `Nonempty`; an empty form is refused).
    pub(crate) semantic_form: &'a str,
    /// The node's semantic type key, or `None` for a self-typed node
    /// (structural preimage only).
    pub(crate) semantic_type: Option<NodeKey>,
    /// The node's `declaration.qualified_name`, when it carries one (schema
    /// `minItems: 1`; an empty name is refused).
    pub(crate) declaration: Option<&'a [Identifier]>,
    /// The node's body.
    pub(crate) body: &'a BodyTerm,
    /// QSpec FR-464's `owner_model`: the exact model declaration node a
    /// `protocol`/`protocol_clause` node is declared over, `Some` for that
    /// node and `None` for every other; it enters the clause's
    /// `quire.application-node/v1` preimage.
    pub(crate) owner_model: Option<NodeKey>,
}

/// Why no key exists for a node.
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
pub enum NodeKeyRefusal {
    /// A lowering-local group handle reached an admission boundary unresolved.
    #[error("a lowering placeholder has no admitted structural node identity")]
    UnresolvedPlaceholder,
    /// The body contains no application term, so FR-322's application
    /// preimage does not key this node.
    #[error("the node body contains no application term")]
    NoApplication,
    /// A body with an application names no semantic type.
    #[error("an application node has no semantic type")]
    UntypedApplication,
    /// A body with an application belongs to a declared (owned) node; the
    /// application preimage has no owner member.
    #[error("an application node carries an owner")]
    OwnedApplication,
    /// An `owner_model` on a node that is not a protocol clause, or a protocol
    /// clause without one (QSpec FR-464 "owner_model").
    #[error("owner_model is present exactly on a protocol clause")]
    OwnerModelPresence,
    /// A structural node's `owner` and `declaration` are not both present
    /// or both absent.
    #[error("a declared node's owner and declaration disagree")]
    OwnerDeclarationMismatch,
    /// The body is an abstraction relation's, which QSpec FR-451 keys by its
    /// own preimage.
    #[error("an abstraction relation body is keyed by its own preimage")]
    AbstractionBody,
    /// The `semantic_form` is empty.
    #[error("the semantic form is empty")]
    EmptySemanticForm,
    /// The `declaration.qualified_name` has no segment.
    #[error("the declaration's qualified name is empty")]
    EmptyQualifiedName,
    /// A `declaration.qualified_name` segment is not an identifier
    /// (`^[A-Za-z_][A-Za-z0-9_]*$`).
    #[error("qualified name segment {segment:?} is not an identifier")]
    InvalidQualifiedNameSegment {
        /// The offending segment.
        segment: String,
    },
    /// A `binding` term's name is empty.
    #[error("a binding name is empty")]
    EmptyBindingName,
    /// A recursion group has no member, names one member twice, or a
    /// placeholder names no member: an internal fault of the caller.
    #[error("a recursion group is empty, names a member twice, or names no member")]
    InvalidGroup,
    /// The checking stage's work budget cannot pay for keying a group.
    #[error("keying the recursion group exceeds the work budget {limit}")]
    WorkBudget {
        /// The work budget.
        limit: u64,
        /// The cumulative spend the refused charge would have reached.
        actual: u128,
    },
    /// A literal's `type`, an application's `result_type` or an operation
    /// member's `declaration` names a member of the node's own recursion
    /// group. FR-092: those positions name type and model nodes, which are
    /// never in an expression's, value's or function's group, and a type
    /// node's literals are typed at builtin scalars.
    #[error("a type position names a member of the node's own recursion group")]
    GroupMemberAtTypePosition,
    /// `quire-canonical` found no RFC 8785 encoding of a preimage, a shape
    /// or a signature round. Unreachable for these types (every member name
    /// is a fixed string, every number is checked against RFC 8785's exact
    /// range first, and no `Serialize` impl here errors), but encoding is
    /// fallible and this module does not panic on an input path. A byte
    /// limit or a failed reservation is [`Self::Identity`].
    #[error("the preimage has no RFC 8785 encoding: {reason}")]
    Encode {
        /// The encoder's own message.
        reason: String,
    },
    /// The preimage's canonical bytes would exceed `identity.input_bytes`,
    /// or a heap reservation for them failed (FR-259 Behaviors 4 and 6):
    /// a stage limit or an allocation failure, never a malformed preimage.
    #[error(transparent)]
    Identity(IdentityRefusal),
}

mod abstraction;

pub use abstraction::AbstractionTerm;
pub(crate) use abstraction::{FrameEntry, ObjectEntry, PopulationEntry};

#[cfg(test)]
mod ref_tests;
