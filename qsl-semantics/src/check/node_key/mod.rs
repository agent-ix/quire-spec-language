// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked node keys: the FR-322 `quire.application-node/v1`
//! preimage and QSL's FR-092 `quire.structural-node/v1` preimage, and the
//! `quire.checked-semantic-node/v1` key each hashes to.
//!
//! FR-092 "Which preimage keys a node": a node whose body contains an
//! `application` term is keyed by FR-322's `application_node_preimage`,
//! `{version: "quire.application-node/v1", node_tag, semantic_form,
//! semantic_type, declaration, recursion, body}`; every other node the
//! `Value` family builds is keyed by `quire.structural-node/v1`, the same
//! members with that version, an `owner` member exactly when `declaration`
//! is not `null` (a `SourceOwner`) or the node is model-owned (a
//! `ModelOwner` with a `null` `declaration`, FR-094), and a `null`
//! `semantic_type` for a self-typed node.
//! [`node_key`] makes that choice from the body alone, so the two preimages
//! cannot disagree about which applies. The nominal enum, enum member,
//! dimension and unit preimages (FR-092 rule 1) are QSpec's own, built by
//! `value::enumeration` and `value::unit`.
//!
//! `declaration` is the node's `declaration` member or `null`. [`node_key`]
//! keys a node outside every recursion group (`recursion` `null`);
//! [`group_keys`] keys a group's members by FR-092's group order: shapes,
//! refinement signatures, classes, ordinals and the group digest. An
//! in-group node's `recursion` is `{size, ordinal}` (application) or
//! `{group, size, ordinal}` (structural), and each position naming a member
//! of its group, a body `reference` or its `semantic_type`, becomes
//! `{term: "group_reference", ordinal}`.
//!
//! The body is typed ([`BodyTerm`] and its strata mirror the v2 schema's
//! closed `SemanticTerm`, `Operation`, `OperationLaw`, `OperationMode` and
//! `OperationLeaf` definitions), so the preimage has a field list the compiler
//! checks rather than a `serde_json::Value` assembled by hand. Mode values are
//! the crate's own domain enums (`quire_exact::RoundingMode`,
//! `quire_exact::TextProfile`, `qsl_foundation::absence::AbsenceMode`),
//! spelled through their `as_str`. A literal's value is typed by its kind
//! ([`LiteralValue`]); FR-092 spells an integer as its canonical decimal
//! string and a rational as `"n/d"`, never as a JSON number.
//!
//! Canonical bytes: the typed preimage, each signature round and the group
//! digest's array are encoded by `quire-canonical`, the one RFC 8785
//! implementation (ADR-013 §2, ADR-013:113), which orders members itself.
//! The preimage's body nests as deep as its term, so it is written through
//! the encoder's event API from an explicit stack; every other part has a
//! fixed shape and takes the encoder's `FixedShape` serde path. The group
//! order's shapes are the one place a preimage is rewritten rather than
//! built: its canonical bytes are read back through `quire-canonical`'s
//! reader and written again with the placeholders' ordinals left out (and,
//! for the anonymous shape, `owner` left out and `declaration` cleared). The only
//! JSON numbers in a preimage are
//! `recursion`'s size and ordinal, a `group_reference` ordinal and an
//! `operation.member` position; each is refused when RFC 8785 cannot render
//! it exactly (outside the IEEE-754 safe range) rather than hashed. No
//! `Debug` or `Display` formatting of a Rust value is on the path except the
//! canonical wire spelling `Integer` (the complete-V1 canonical decimal)
//! documents as a contract. A node key, a group digest and a signature are
//! spelled by this module's own lowercase hex ([`HexDigest`]), which is
//! `NodeKey`'s wire spelling. The pinned-bytes tests fix the exact encoding.
//!
//! The body is stratified ([`BodyTerm`], [`MemberTerm`], [`GroupTerm`],
//! [`LeafTerm`]): no stratum names itself or a higher one, so keying a body
//! is a fixed-depth match and every preimage nests a depth the grammar fixes,
//! whatever the expression's depth (ADR-030 D-2, FR-258).
//!
//! Conformance against QSpec's own published `operation_vectors` is the
//! opt-in `conformance` test (`make conformance` with `QSPEC_DIR` pointing at
//! a quire-specification checkout). QSpec is not public yet, so nothing of it
//! is copied here; the test reads the vectors at run time.

use std::collections::{BTreeMap, BTreeSet};

use qsl_foundation::ByteDigest;
use quire_canonical::{Encode, FixedShape, Sink, Writer};
use serde::Serialize;

use crate::value::semantic_node::{NodeIdentityPreimage, NodeOwner, NominalRefusal, OwnerSubject};
use quire_semantic_value::semantic_node::IDENTITY_LIMITS as LIMITS;

use qsl_foundation::absence::AbsenceMode;
use quire_exact::{Identifier, Integer, Rational, RoundingMode, TextProfile};

use crate::value::definition::DefinitionReference;
use crate::value::member::Member;
use quire_exact::{NodeKey, NODE_KEY_DOMAIN};

/// FR-322's application-node preimage version.
pub(crate) const APPLICATION_NODE_VERSION: &str = "quire.application-node/v1";

/// FR-092's structural-node preimage version.
pub(crate) const STRUCTURAL_NODE_VERSION: &str = "quire.structural-node/v1";

/// The largest integer magnitude RFC 8785 renders exactly (2^53 - 1).
const JCS_SAFE_INTEGER: u64 = (1 << 53) - 1;

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
    pub(crate) fn node_owner(&self) -> NodeOwner {
        NodeOwner::Source(OwnerSubject {
            authority: self.authority.clone(),
            identity: self.identity.clone(),
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
/// lock's `model_selections` selects the package by identity and digest, so
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
}

/// A `NodeRef` (`{domain, digest}`) naming a checked node by key.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeRef(pub NodeKey);

impl Serialize for NodeRef {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // `NodeKey`'s lowercase-hex spelling, written on the stack: a
        // preimage names many keys, and each is spelled without
        // allocating or formatting.
        let digits = HexDigest::of(self.0.as_bytes());
        NodeIdWire {
            domain: NODE_KEY_DOMAIN,
            digest: digits.as_str(),
        }
        .serialize(serializer)
    }
}

/// A [`NodeRef`] serializes as a `NodeIdWire`, so it nests as deep.
impl FixedShape for NodeRef {
    const DEPTH: usize = <NodeIdWire<'static> as FixedShape>::DEPTH;
}

/// A node id as a preimage or term writes it: `{domain, digest}`, the
/// digest in lowercase hex.
#[derive(Serialize, FixedShape)]
struct NodeIdWire<'a> {
    domain: &'static str,
    digest: &'a str,
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
}

/// The Leaf stratum of QSpec FR-322's v2 body grammar (ADR-030 D-2): a
/// term that holds no other term.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
// No `deny_unknown_fields`: serde does not combine it with the literal's
// `flatten`, and the conformance test compares re-encoded bytes with the
// published ones, so a dropped member fails there instead.
#[serde(tag = "term", rename_all = "snake_case")]
pub enum LeafTerm {
    /// A typed literal: its kind and JSON value come from [`LiteralValue`].
    Literal {
        /// The literal's type node.
        #[serde(rename = "type")]
        ty: NodeRef,
        /// The literal's value.
        #[serde(flatten)]
        value: LiteralValue,
    },
    /// A reference to another node.
    Reference {
        /// The referenced node.
        target: NodeRef,
    },
    /// A reference to a node of a dependency package (ADR-015 D-5, QSpec
    /// FR-322): the dependency's `package_id` and the node's id there. It
    /// enters the referencing node's identity and is never one of its
    /// `dependencies` (FR-322-AC-36, FR-322-AC-37).
    DependencyReference {
        /// The dependency's `package_id`.
        package: PackageRef,
        /// The node's id in the dependency.
        node: WireNodeRef,
    },
}

/// `{term: "binding", name, value}`: a named `value` of a lower stratum.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(tag = "term", rename = "binding")]
pub struct Binding<V> {
    /// The bound name (schema `Nonempty`; an empty name is refused).
    pub name: String,
    /// The bound term.
    pub value: V,
}

/// A member of a [`GroupTerm`]: a Leaf, or a binding whose value is a Leaf.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(untagged)]
pub enum GroupMember {
    /// A Leaf.
    Leaf(LeafTerm),
    /// A binding of a Leaf.
    Binding(Binding<LeafTerm>),
}

/// The Group stratum: an `aggregate` whose members are each a Leaf or a
/// binding of a Leaf.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(tag = "term", rename = "aggregate")]
pub struct GroupTerm {
    /// The members, in order.
    pub members: Vec<GroupMember>,
}

/// A member of a [`TupleTerm`]: a Leaf or a Group.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(untagged)]
pub enum TupleMember {
    /// A Leaf.
    Leaf(LeafTerm),
    /// A Group.
    Group(GroupTerm),
}

/// The Tuple stratum: an `aggregate` whose members are each a Leaf or a
/// Group.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(tag = "term", rename = "aggregate")]
pub struct TupleTerm {
    /// The members, in order.
    pub members: Vec<TupleMember>,
}

/// The value of a [`MemberTerm`] binding: a Leaf, a Group or a Tuple. A
/// binding never holds another binding (FR-322 "Body grammar"): FR-092 gives
/// an optional record field as `f` bound to the Group `{optional:
/// reference}`, and FR-093 a fold's step as the accumulator bound to the
/// Group `{x: reference}`.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(untagged)]
pub enum BindingValue {
    /// A Leaf.
    Leaf(LeafTerm),
    /// A Group.
    Group(GroupTerm),
    /// A Tuple.
    Tuple(TupleTerm),
}

/// The Member stratum: an application argument or a body aggregate's
/// member.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(untagged)]
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
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(tag = "term", rename = "application")]
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
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(tag = "term", rename = "aggregate")]
pub struct AggregateTerm {
    /// The members, in order.
    pub members: Vec<MemberTerm>,
}

/// A `state`/`frame` node's own body shape (QSpec FR-340), valid only as a
/// node's body. It contains no `application`, so `node_key` always keys a
/// frame node through the FR-092 structural preimage, never the application
/// one.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(tag = "term", rename = "frame")]
pub struct FrameTerm {
    /// Every field or relationship the operation may write, ascending by
    /// (declaring node digest, field name) (FR-340).
    pub modifies: Vec<FrameField>,
    /// Every object type or process the operation may create, ascending by
    /// node digest.
    pub creates: Vec<NodeRef>,
    /// Every object type or process the operation may delete, ascending by
    /// node digest.
    pub deletes: Vec<NodeRef>,
}

/// A node's body: the Body stratum of QSpec FR-322's v2 body grammar
/// (ADR-030 D-2, FR-258). No stratum names itself or a higher one, so every
/// body, and its node-key preimage, nests a depth the grammar fixes, and
/// every composite subterm is its own node, reached by `reference`.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(untagged)]
pub enum BodyTerm {
    /// A Leaf.
    Leaf(LeafTerm),
    /// An operation application.
    Application(ApplicationTerm),
    /// An ordered aggregate of Members.
    Aggregate(AggregateTerm),
    /// A `state`/`frame` node's body.
    Frame(FrameTerm),
}

/// One `modifies` entry of a `state`/`frame` node's body (QSpec FR-340): a
/// field this operation may write, named by its declaring node and the
/// field's own name. The schema's `FrameModifiesEntry` union also admits a
/// bare relationship entry (`{kind: "relationship", declaration}`); QSL's
/// checker records no relationship modifies yet (FR-104's own scope is field
/// effects only), so this type carries only the field form its one producer
/// (the checker's own `frame_node` lowering step) ever builds -- adding
/// relationship modifies later is a new constructor here, not a wire-shape
/// change.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FrameField {
    declaration: NodeRef,
    name: String,
}

impl FrameField {
    /// `{kind: "field", declaration, name}`.
    pub(crate) fn new(declaration: NodeKey, name: impl Into<String>) -> Self {
        Self {
            declaration: NodeRef(declaration),
            name: name.into(),
        }
    }

    /// The field's declaring node.
    pub fn declaration(&self) -> NodeRef {
        self.declaration
    }

    /// The field's own name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Serialize for FrameField {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        FrameFieldWire {
            kind: "field",
            declaration: self.declaration,
            name: &self.name,
        }
        .serialize(serializer)
    }
}

/// A [`FrameField`] serializes as a `FrameFieldWire`, so it nests as deep.
impl FixedShape for FrameField {
    const DEPTH: usize = <FrameFieldWire<'static> as FixedShape>::DEPTH;
}

/// `{kind: "field", declaration, name}`.
#[derive(Serialize, FixedShape)]
struct FrameFieldWire<'a> {
    kind: &'static str,
    declaration: NodeRef,
    name: &'a str,
}

// Test-only, matching `LiteralValue`'s own pattern: QSpec's operation
// vectors carry no frame body (a frame's modifies/creates/deletes are the
// `node-identity-vectors.json` fixture's own, separate shape), so the
// conformance decoder never needs one.
#[cfg(test)]
impl<'de> serde::Deserialize<'de> for FrameField {
    fn deserialize<D: serde::Deserializer<'de>>(_: D) -> Result<Self, D::Error> {
        Err(serde::de::Error::custom(
            "frame fields are not decoded from vectors",
        ))
    }
}

/// A dependency's `package_id` as a term writes it: a
/// `quire.package.semantic/v2` SHA-256 semantic id.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PackageRef(pub crate::library::PackageId);

impl Serialize for PackageRef {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let digits = HexDigest::of(self.0.as_bytes());
        PackageIdWire {
            domain: PACKAGE_DOMAIN_V2,
            algorithm: "sha256",
            digest: digits.as_str(),
        }
        .serialize(serializer)
    }
}

/// A [`PackageRef`] serializes as a `PackageIdWire`, so it nests as deep.
impl FixedShape for PackageRef {
    const DEPTH: usize = <PackageIdWire<'static> as FixedShape>::DEPTH;
}

/// `{domain, algorithm: "sha256", digest}`.
#[derive(Serialize, FixedShape)]
struct PackageIdWire<'a> {
    domain: &'static str,
    algorithm: &'static str,
    digest: &'a str,
}

/// A dependency node's id as a term writes it: a node id in the node
/// domain, the spelling the dependency's own graph gives it.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WireNodeRef(pub qsl_foundation::digest::WireNodeId);

impl Serialize for WireNodeRef {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let digits = HexDigest::of(self.0.as_bytes());
        NodeIdWire {
            domain: NODE_KEY_DOMAIN,
            digest: digits.as_str(),
        }
        .serialize(serializer)
    }
}

/// A [`WireNodeRef`] serializes as a `NodeIdWire`, so it nests as deep.
impl FixedShape for WireNodeRef {
    const DEPTH: usize = <NodeIdWire<'static> as FixedShape>::DEPTH;
}

/// The `package_id` domain a `dependency_reference` names (QSpec FR-322).
const PACKAGE_DOMAIN_V2: &str = "quire.package.semantic/v2";

// Test-only: QSpec's operation vectors carry no `dependency_reference`, so
// the conformance decoder never needs one.
#[cfg(test)]
impl<'de> serde::Deserialize<'de> for PackageRef {
    fn deserialize<D: serde::Deserializer<'de>>(_: D) -> Result<Self, D::Error> {
        Err(serde::de::Error::custom(
            "dependency references are not decoded from vectors",
        ))
    }
}

#[cfg(test)]
impl<'de> serde::Deserialize<'de> for WireNodeRef {
    fn deserialize<D: serde::Deserializer<'de>>(_: D) -> Result<Self, D::Error> {
        Err(serde::de::Error::custom(
            "dependency references are not decoded from vectors",
        ))
    }
}

impl LeafTerm {
    /// `{term: "reference", target}`.
    pub(crate) fn reference(target: NodeKey) -> Self {
        Self::Reference {
            target: NodeRef(target),
        }
    }

    /// `{term: "literal", type, value_kind, value}`.
    pub(crate) fn literal(ty: NodeKey, value: LiteralValue) -> Self {
        Self::Literal {
            ty: NodeRef(ty),
            value,
        }
    }

    /// The node key this leaf names in its own graph: a `reference`'s
    /// target or a literal's `type`. A dependency's node is no key of this
    /// graph.
    pub fn key(&self) -> Option<NodeKey> {
        match self {
            Self::Literal { ty, .. } => Some(ty.0),
            Self::Reference { target } => Some(target.0),
            Self::DependencyReference { .. } => None,
        }
    }

    /// This leaf with the node key it names replaced by `map`'s image.
    fn map_keys(&self, map: &mut impl FnMut(NodeKey) -> NodeKey) -> Self {
        match self {
            Self::Literal { ty, value } => Self::Literal {
                ty: NodeRef(map(ty.0)),
                value: value.clone(),
            },
            Self::Reference { target } => Self::Reference {
                target: NodeRef(map(target.0)),
            },
            Self::DependencyReference { package, node } => Self::DependencyReference {
                package: *package,
                node: *node,
            },
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
    fn map_keys(&self, map: &mut impl FnMut(NodeKey) -> NodeKey) -> Self {
        Self::new(self.name.clone(), self.value.map_keys(map))
    }
}

impl GroupMember {
    /// The member's Leaf, bound or not.
    pub fn leaf(&self) -> &LeafTerm {
        match self {
            Self::Leaf(leaf) | Self::Binding(Binding { value: leaf, .. }) => leaf,
        }
    }

    fn map_keys(&self, map: &mut impl FnMut(NodeKey) -> NodeKey) -> Self {
        match self {
            Self::Leaf(leaf) => Self::Leaf(leaf.map_keys(map)),
            Self::Binding(binding) => Self::Binding(binding.map_keys(map)),
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
        Self::new(
            targets
                .into_iter()
                .map(|target| GroupMember::Leaf(LeafTerm::reference(target)))
                .collect(),
        )
    }

    /// Every Leaf of this Group, in order.
    pub fn leaves(&self) -> impl Iterator<Item = &LeafTerm> {
        self.members.iter().map(GroupMember::leaf)
    }

    fn map_keys(&self, map: &mut impl FnMut(NodeKey) -> NodeKey) -> Self {
        Self::new(
            self.members
                .iter()
                .map(|member| member.map_keys(map))
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

    fn map_keys(&self, map: &mut impl FnMut(NodeKey) -> NodeKey) -> Self {
        match self {
            Self::Leaf(leaf) => Self::Leaf(leaf.map_keys(map)),
            Self::Group(group) => Self::Group(group.map_keys(map)),
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

    fn map_keys(&self, map: &mut impl FnMut(NodeKey) -> NodeKey) -> Self {
        Self::new(
            self.members
                .iter()
                .map(|member| member.map_keys(map))
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
        }
    }

    fn map_keys(&self, map: &mut impl FnMut(NodeKey) -> NodeKey) -> Self {
        match self {
            Self::Leaf(leaf) => Self::Leaf(leaf.map_keys(map)),
            Self::Group(group) => Self::Group(group.map_keys(map)),
            Self::Tuple(tuple) => Self::Tuple(tuple.map_keys(map)),
        }
    }
}

impl MemberTerm {
    /// `{term: "reference", target}` as a Member.
    pub(crate) fn reference(target: NodeKey) -> Self {
        Self::Leaf(LeafTerm::reference(target))
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

    fn map_keys(&self, map: &mut impl FnMut(NodeKey) -> NodeKey) -> Self {
        match self {
            Self::Leaf(leaf) => Self::Leaf(leaf.map_keys(map)),
            Self::Group(group) => Self::Group(group.map_keys(map)),
            Self::Binding(binding) => Self::Binding(Binding::new(
                binding.name.clone(),
                binding.value.map_keys(map),
            )),
        }
    }
}

impl BodyTerm {
    /// `{term: "literal", type, value_kind, value}` as a body.
    pub(crate) fn literal(ty: NodeKey, value: LiteralValue) -> Self {
        Self::Leaf(LeafTerm::literal(ty, value))
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
            result_type: NodeRef(result_type),
            arguments,
        })
    }

    /// `{term: "frame", modifies, creates, deletes}` (QSpec FR-340).
    /// `creates` and `deletes` are each an ascending-by-digest list of
    /// object type or process node keys.
    pub(crate) fn frame(
        modifies: Vec<FrameField>,
        creates: Vec<NodeKey>,
        deletes: Vec<NodeKey>,
    ) -> Self {
        Self::Frame(FrameTerm {
            modifies,
            creates: creates.into_iter().map(NodeRef).collect(),
            deletes: deletes.into_iter().map(NodeRef).collect(),
        })
    }

    /// The application this body is, when it is one.
    pub fn application_term(&self) -> Option<&ApplicationTerm> {
        match self {
            Self::Application(application) => Some(application),
            Self::Leaf(_) | Self::Aggregate(_) | Self::Frame(_) => None,
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
            Self::Frame(_) => Vec::new(),
        }
    }

    /// Call `visit` with every node key this body names (FR-092 "names"):
    /// each `reference` target, literal `type`, application `result_type`
    /// and operation member `declaration`, and a frame's entries.
    pub(crate) fn for_each_key(&self, visit: &mut impl FnMut(NodeKey)) {
        match self {
            Self::Application(ApplicationTerm {
                operation,
                result_type,
                ..
            }) => {
                if let Some(declaration) = operation.member.as_ref().and_then(Member::declaration) {
                    visit(declaration);
                }
                visit(result_type.0);
            }
            Self::Frame(FrameTerm {
                modifies,
                creates,
                deletes,
            }) => {
                for field in modifies {
                    visit(field.declaration.0);
                }
                for reference in creates.iter().chain(deletes) {
                    visit(reference.0);
                }
            }
            Self::Leaf(_) | Self::Aggregate(_) => {}
        }
        for key in self.leaves().into_iter().filter_map(LeafTerm::key) {
            visit(key);
        }
    }

    /// This body with every node key it names replaced by `map`'s image.
    pub(crate) fn map_keys(&self, map: &mut impl FnMut(NodeKey) -> NodeKey) -> Self {
        match self {
            Self::Leaf(leaf) => Self::Leaf(leaf.map_keys(map)),
            Self::Application(ApplicationTerm {
                operator,
                operation,
                result_type,
                arguments,
            }) => {
                let mut operation = operation.clone();
                operation.member = operation.member.map(|member| map_member(member, map));
                Self::Application(ApplicationTerm {
                    operator: *operator,
                    operation,
                    result_type: NodeRef(map(result_type.0)),
                    arguments: arguments
                        .iter()
                        .map(|argument| argument.map_keys(map))
                        .collect(),
                })
            }
            Self::Aggregate(AggregateTerm { members }) => Self::Aggregate(AggregateTerm {
                members: members.iter().map(|member| member.map_keys(map)).collect(),
            }),
            Self::Frame(FrameTerm {
                modifies,
                creates,
                deletes,
            }) => Self::Frame(FrameTerm {
                modifies: modifies
                    .iter()
                    .map(|field| FrameField {
                        declaration: NodeRef(map(field.declaration.0)),
                        name: field.name.clone(),
                    })
                    .collect(),
                creates: creates.iter().map(|node| NodeRef(map(node.0))).collect(),
                deletes: deletes.iter().map(|node| NodeRef(map(node.0))).collect(),
            }),
        }
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
    /// `rational`: `"n/d"`, reduced, positive denominator.
    Rational(Rational),
    /// `text`: the text as a JSON string.
    Text(String),
    /// `none`: `null`.
    None,
    /// `enum`: the member's case identifier, the body of an enum member's
    /// QSpec `value`/`enum_value` node.
    Enum(String),
}

impl LiteralValue {
    /// This literal's `{value_kind, value}` as written.
    fn wire(&self) -> LiteralWire<'_> {
        let (value_kind, value) = match self {
            Self::Boolean(value) => ("boolean", LiteralJson::Boolean(*value)),
            Self::Integer(value) => ("integer", LiteralJson::Spelled(value.to_string())),
            Self::Rational(value) => (
                "rational",
                LiteralJson::Spelled(format!("{}/{}", value.numerator(), value.denominator())),
            ),
            Self::Text(value) => ("text", LiteralJson::Text(value)),
            Self::None => ("none", LiteralJson::Null(())),
            Self::Enum(case) => ("enum", LiteralJson::Text(case)),
        };
        LiteralWire { value_kind, value }
    }
}

impl Serialize for LiteralValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.wire().serialize(serializer)
    }
}

/// A [`LiteralValue`] serializes as a `LiteralWire`, so it nests as deep.
impl FixedShape for LiteralValue {
    const DEPTH: usize = <LiteralWire<'static> as FixedShape>::DEPTH;
}

/// A literal's `value_kind` and its JSON `value`.
#[derive(Serialize, FixedShape)]
struct LiteralWire<'a> {
    value_kind: &'static str,
    value: LiteralJson<'a>,
}

/// A literal's JSON `value`: a boolean, a string, or `null`.
#[derive(Serialize, FixedShape)]
#[serde(untagged)]
enum LiteralJson<'a> {
    Boolean(bool),
    Text(&'a str),
    Spelled(String),
    Null(()),
}

// Test-only: QSpec's operation vectors carry no literal term, so the
// conformance decoder never needs one; a literal in a decoded vector refuses
// rather than guessing a kind.
#[cfg(test)]
impl<'de> serde::Deserialize<'de> for LiteralValue {
    fn deserialize<D: serde::Deserializer<'de>>(_: D) -> Result<Self, D::Error> {
        Err(serde::de::Error::custom(
            "literal terms are not decoded from vectors",
        ))
    }
}

/// The schema's closed application `operator` vocabulary.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, FixedShape)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(rename_all = "snake_case")]
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
    /// `temporal`.
    Temporal,
    /// `protocol_control`.
    ProtocolControl,
    /// `state_transition`.
    StateTransition,
    /// `state_clause`.
    StateClause,
    /// `claim`.
    Claim,
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
            Self::Temporal => "temporal",
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
        }
    }
}

/// The schema's `Operation`: a catalogued operation with its laws, mode,
/// member and leaves.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, FixedShape)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(deny_unknown_fields)]
pub struct Operation {
    /// The `quire.checked-operation-catalog/v1` identity; catalog membership
    /// is the reader's check, not the key's.
    pub(crate) identity: String,
    /// The operation's profile laws.
    pub(crate) laws: Vec<OperationLaw>,
    /// The operation's mode, or `None` (JSON `null`).
    pub(crate) mode: Option<OperationMode>,
    /// The operation's member, or `None` (JSON `null`).
    #[cfg_attr(test, serde(deserialize_with = "tests::member"))]
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
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, FixedShape)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(deny_unknown_fields)]
pub struct OperationLaw {
    /// The law's role.
    pub role: LawRole,
    /// The selected definition, digest included.
    pub definition: DefinitionReference,
}

/// The schema's closed `OperationLaw.role` vocabulary.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, FixedShape)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(rename_all = "snake_case")]
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

impl Serialize for OperationMode {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let wire = match self {
            Self::Rounding(mode) => OperationModeWire::Rounding(mode.as_str()),
            Self::TextProfile(profile) => OperationModeWire::TextProfile(profile.as_str()),
            Self::Absence(mode) => OperationModeWire::Absence(mode.as_str()),
        };
        wire.serialize(serializer)
    }
}

/// An [`OperationMode`] serializes as an `OperationModeWire`, so it nests
/// as deep.
impl FixedShape for OperationMode {
    const DEPTH: usize = <OperationModeWire as FixedShape>::DEPTH;
}

/// `{kind, value}`, the mode spelled through its `as_str`.
#[derive(Serialize, FixedShape)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum OperationModeWire {
    Rounding(&'static str),
    TextProfile(&'static str),
    Absence(&'static str),
}

/// The schema's `OperationLeaf`.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, FixedShape)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(deny_unknown_fields)]
pub struct OperationLeaf {
    /// The leaf's path from the compared type.
    pub path: Vec<LeafSegment>,
    /// The leaf's profile laws.
    pub laws: Vec<OperationLaw>,
    /// The leaf's mode, or `None` (JSON `null`).
    pub mode: Option<OperationMode>,
}

/// One schema `LeafSegment`: `field:<identifier>`, `position:<n>` or
/// `inner`, and FR-093's `recursion:<d>`, which ends a recursion leaf's path
/// (a QSL proposal, ADR-013 QC-24).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum LeafSegment {
    /// `field:<name>`.
    Field(Identifier),
    /// `position:<n>`.
    Position(u64),
    /// `inner`.
    Inner,
    /// `recursion:<d>`: the path reenters the composite it entered after
    /// `d` segments.
    Recursion(usize),
}

impl Serialize for LeafSegment {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Field(name) => serializer.collect_str(&format_args!("field:{}", name.as_str())),
            Self::Position(position) => {
                serializer.collect_str(&format_args!("position:{position}"))
            }
            Self::Inner => serializer.serialize_str("inner"),
            Self::Recursion(depth) => serializer.collect_str(&format_args!("recursion:{depth}")),
        }
    }
}

/// A [`LeafSegment`] serializes as one string.
impl FixedShape for LeafSegment {
    const DEPTH: usize = <str as FixedShape>::DEPTH;
}

/// The inputs of one node's key: every member of either preimage.
#[derive(Clone, Copy, Debug)]
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
}

/// The inputs of one application-bearing node's FR-322 key: the
/// conformance test's view of [`NodeInput`].
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct ApplicationNode<'a> {
    /// The node's v2 `node_tag`.
    pub(crate) node_tag: NodeTag,
    /// The node's v2 `semantic_form`.
    pub(crate) semantic_form: &'a str,
    /// The node's semantic type key.
    pub(crate) semantic_type: NodeKey,
    /// The node's `declaration.qualified_name`, when it carries one.
    pub(crate) declaration: Option<&'a [Identifier]>,
    /// The node's body.
    pub(crate) body: &'a BodyTerm,
}

/// The canonical preimage bytes and the key they hash to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeyedPreimage {
    /// The canonical JCS bytes of the node's preimage.
    pub(crate) preimage: Vec<u8>,
    /// SHA-256 of [`Self::preimage`].
    pub(crate) key: NodeKey,
}

/// Where a preimage number sits.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum IntegerSite {
    /// An `operation.member` position.
    MemberPosition,
    /// The `recursion` group size.
    RecursionSize,
}

/// Why no key exists for a node.
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
pub enum NodeKeyRefusal {
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
    /// A structural node's `owner` and `declaration` are not both present
    /// or both absent.
    #[error("a declared node's owner and declaration disagree")]
    OwnerDeclarationMismatch,
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
    /// A number lies outside the range RFC 8785 renders exactly.
    #[error("{site:?} {value} is outside the RFC 8785 exact-integer range")]
    UnsafeInteger {
        /// Where the number sits.
        site: IntegerSite,
        /// The offending value.
        value: u64,
    },
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
    /// `quire-canonical` refused to encode a preimage, a shape or a
    /// signature round. Unreachable for these types (every member name is a
    /// fixed string, every number is checked against RFC 8785's exact range
    /// first, and no `Serialize` impl here errors) short of a failed heap
    /// reservation, but encoding is fallible and this module does not panic
    /// on an input path.
    #[error("the preimage has no RFC 8785 encoding: {reason}")]
    Encode {
        /// The encoder's own message.
        reason: String,
    },
}

/// Refuse `value` at `site` when RFC 8785 cannot render it exactly.
fn exact_integer(site: IntegerSite, value: u64) -> Result<(), NodeKeyRefusal> {
    if value <= JCS_SAFE_INTEGER {
        Ok(())
    } else {
        Err(NodeKeyRefusal::UnsafeInteger { site, value })
    }
}

/// The FR-322 `application_node_preimage` key of `node`, refusing a body
/// with no application.
// Production keys every node through `node_key`; the conformance test and
// the application-only unit tests call this one.
#[cfg(test)]
pub(crate) fn application_node_key(
    node: &ApplicationNode<'_>,
) -> Result<KeyedPreimage, NodeKeyRefusal> {
    let input = NodeInput {
        owner: None,
        node_tag: node.node_tag,
        semantic_form: node.semantic_form,
        semantic_type: Some(node.semantic_type),
        declaration: node.declaration,
        body: node.body,
    };
    let (_, has_application) = Walk::OUTSIDE.body(node.body)?;
    if !has_application {
        return Err(NodeKeyRefusal::NoApplication);
    }
    node_key(&input)
}

/// The key of `node`, a node outside every recursion group (FR-092 "Which
/// preimage keys a node"): the FR-322 application-node preimage when its
/// body contains an application, else the FR-092 structural-node preimage,
/// with `recursion` `null`.
pub(crate) fn node_key(node: &NodeInput<'_>) -> Result<KeyedPreimage, NodeKeyRefusal> {
    let (preimage, _) = typed_preimage(node, Walk::OUTSIDE, None)?;
    keyed(&preimage)
}

/// The keys of one recursion group's members (FR-092 "Recursion groups").
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GroupKeys {
    /// Each member's preimage and key, in input order. Members of one class
    /// have equal preimages.
    pub(crate) members: Vec<KeyedPreimage>,
    /// Each member's ordinal (its class's rank), in input order.
    pub(crate) ordinals: Vec<usize>,
    /// The number of classes.
    pub(crate) size: usize,
    /// The group digest.
    pub(crate) digest: [u8; 32],
    /// Each member's anonymous-pass signature, lowercase hex, in input order.
    pub(crate) anonymous: Vec<String>,
    /// Each member's full-pass signature, lowercase hex, in input order.
    pub(crate) full: Vec<String>,
}

/// Key the recursion group `members` (FR-092 "The group order"). Each
/// member's references to another member name it by its handle, the
/// matching entry of `handles`: any key that is not a member's key, since
/// no member's key exists before its group is keyed. `semantic_form` is the
/// member's own (a function member's is `recursive_function`).
///
/// No step reads a handle's value: the shapes write every in-group position
/// as `{term: "group_reference"}`, so the order, ordinals, digest and keys
/// are the same whatever handles name the members.
///
/// `charge` is called with the number of preimages or signature rounds
/// each step hashes, before it hashes them, so a caller bounds the work
/// (a refinement pass is up to `n` rounds of `n` hashes); its refusal ends
/// the keying.
pub(crate) fn group_keys(
    members: &[NodeInput<'_>],
    handles: &[NodeKey],
    charge: &mut dyn FnMut(u64) -> Result<(), NodeKeyRefusal>,
) -> Result<GroupKeys, NodeKeyRefusal> {
    let positions: BTreeMap<NodeKey, usize> = handles
        .iter()
        .enumerate()
        .map(|(position, handle)| (*handle, position))
        .collect();
    if members.is_empty() || members.len() != handles.len() || positions.len() != handles.len() {
        return Err(NodeKeyRefusal::InvalidGroup);
    }
    let count = members.len();
    let work = u64::try_from(count).map_err(|_| NodeKeyRefusal::InvalidGroup)?;
    // 1-2. Each member's full and anonymous shapes and its targets: the
    // placeholders carry the target's input position as their ordinal,
    // which `shape_targets` removes in RFC 8785 order.
    charge(work)?;
    let mut full_shapes = Vec::with_capacity(count);
    let mut anonymous_shapes = Vec::with_capacity(count);
    let mut targets = Vec::with_capacity(count);
    for member in members {
        let (preimage, _) = typed_preimage(member, Walk { group: &positions }, None)?;
        let shapes = shape::member_shapes(&canonical_bytes(&preimage)?, count)?;
        full_shapes.push(shapes.full);
        anonymous_shapes.push(shapes.anonymous);
        targets.push(shapes.targets);
    }
    // 3-4. One refinement pass per kind of shape, then the order.
    let anonymous = refine(&anonymous_shapes, &targets, work, charge)?;
    let full = refine(&full_shapes, &targets, work, charge)?;
    let mut order: Vec<usize> = (0..count).collect();
    order.sort_by(|left, right| {
        (&anonymous[*left], &full[*left]).cmp(&(&anonymous[*right], &full[*right]))
    });
    // 5. Equal full signatures are one class; a class's ordinal is its rank,
    // and its first member in the order represents it.
    let mut classes: BTreeMap<&str, usize> = BTreeMap::new();
    let mut representatives = Vec::new();
    let mut ordinals = vec![0; count];
    for member in order {
        let next = classes.len();
        let ordinal = *classes.entry(full[member].as_str()).or_insert(next);
        if ordinal == next {
            representatives.push(member);
        }
        ordinals[member] = ordinal;
    }
    let size = classes.len();
    exact_integer(
        IntegerSite::RecursionSize,
        u64::try_from(size).map_err(|_| NodeKeyRefusal::InvalidGroup)?,
    )?;
    let ordinal_of: BTreeMap<NodeKey, usize> = handles
        .iter()
        .zip(&ordinals)
        .map(|(handle, ordinal)| (*handle, *ordinal))
        .collect();
    // The group digest: over each class's group-local preimage, in ordinal
    // order.
    charge(work)?;
    let mut local = Vec::with_capacity(size);
    for (ordinal, member) in representatives.iter().enumerate() {
        let recursion = RecursionPreimage {
            group: None,
            ordinal,
            size,
        };
        let (preimage, _) = typed_preimage(
            &members[*member],
            Walk { group: &ordinal_of },
            Some(recursion),
        )?;
        local.push(hex(&canonical_sha256(&preimage)?));
    }
    let digest = canonical_sha256(&local)?;
    let digest_hex = hex(&digest);
    charge(work)?;
    let mut keys = Vec::with_capacity(count);
    for (member, ordinal) in members.iter().zip(&ordinals) {
        let recursion = RecursionPreimage {
            group: Some(digest_hex.clone()),
            ordinal: *ordinal,
            size,
        };
        let (preimage, _) = typed_preimage(member, Walk { group: &ordinal_of }, Some(recursion))?;
        keys.push(keyed(&preimage)?);
    }
    Ok(GroupKeys {
        members: keys,
        ordinals,
        size,
        digest,
        anonymous,
        full,
    })
}

/// A refinement pass (FR-092 "The group order", step 3): each member's
/// signature over `shapes`, lowercase hex. Each round charges `work`, the
/// member count.
fn refine(
    shapes: &[String],
    targets: &[Vec<usize>],
    work: u64,
    charge: &mut dyn FnMut(u64) -> Result<(), NodeKeyRefusal>,
) -> Result<Vec<String>, NodeKeyRefusal> {
    charge(work)?;
    let initial: Vec<String> = shapes
        .iter()
        .map(|shape| hex(&ByteDigest::of(shape.as_bytes()).as_bytes()))
        .collect();
    let distinct = |signatures: &[String]| signatures.iter().collect::<BTreeSet<_>>().len();
    let mut previous = initial.clone();
    // Each round before the last adds a distinct value, so at most one
    // round per member runs.
    for _ in 0..shapes.len() {
        charge(work)?;
        let mut next = Vec::with_capacity(initial.len());
        for (shape, member_targets) in initial.iter().zip(targets) {
            let round_targets = member_targets
                .iter()
                .map(|target| previous.get(*target).ok_or(NodeKeyRefusal::InvalidGroup))
                .collect::<Result<Vec<_>, _>>()?;
            let round = SignatureRound {
                shape,
                targets: round_targets,
            };
            next.push(hex(&canonical_sha256(&round)?));
        }
        let stable = distinct(&next) == distinct(&previous);
        previous = next;
        if stable {
            break;
        }
    }
    Ok(previous)
}

/// One refinement round's input: `{shape, targets}`, the member's initial
/// signature and its targets' previous-round signatures.
#[derive(Serialize, FixedShape)]
struct SignatureRound<'a> {
    shape: &'a str,
    targets: Vec<&'a String>,
}

/// A 32-byte digest's lowercase hex, the spelling FR-092 gives every digest
/// inside a preimage or a signature round, and `NodeKey`'s wire spelling.
struct HexDigest([u8; 64]);

impl HexDigest {
    fn of(bytes: &[u8; 32]) -> Self {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut digits = [0; 64];
        let (pairs, _) = digits.as_chunks_mut::<2>();
        for (pair, byte) in pairs.iter_mut().zip(bytes) {
            *pair = [
                DIGITS[usize::from(byte >> 4)],
                DIGITS[usize::from(byte & 0x0f)],
            ];
        }
        Self(digits)
    }

    /// The digits as text.
    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("every hex digit is ASCII, so the digits are UTF-8")
    }
}

/// [`HexDigest`] as an owned string.
pub(super) fn hex(bytes: &[u8; 32]) -> String {
    String::from(HexDigest::of(bytes).as_str())
}

/// The handle a record or tuple declared in source is named by in
/// `ValueType::Composite` before lowering keys its node (FR-091 "Node key
/// owner"): the SHA-256 of `{version, owner, name}`'s RFC 8785 bytes. It is
/// a checker-internal name for the declaration, stable for an unchanged
/// declaration under one owner and different under another; lowering maps
/// it to the node's FR-092 key, which is what the wire carries.
pub(crate) fn declared_type_handle(
    owner: &SourceOwner,
    name: &str,
) -> Result<NodeKey, NodeKeyRefusal> {
    #[derive(Serialize, FixedShape)]
    struct Handle<'a> {
        version: &'static str,
        owner: &'a SourceOwner,
        name: &'a str,
    }
    canonical_sha256(&Handle {
        version: "quire.qsl.declared-type-handle/v1",
        owner,
        name,
    })
    .map(NodeKey::from_digest)
}

/// The key of a nominal node (an enum declaration or member, a dimension
/// or a unit): the SHA-256 of its owner-bearing preimage's RFC 8785 bytes
/// (FR-091 "Enum declarations"; ADR-013 O-04). It is the one function that
/// mints a nominal node's key, and only `check` calls it (FB-13).
/// [`node_key`] keys structural and application nodes and never a nominal
/// preimage. Encoding a preimage can refuse, and the refusal is the
/// caller's nominal-admission fault.
pub(crate) fn nominal_key(preimage: &impl NodeIdentityPreimage) -> Result<NodeKey, NominalRefusal> {
    preimage.digest().map(NodeKey::from_digest)
}

/// `preimage`'s RFC 8785 bytes and the node key they hash to.
fn keyed(preimage: &Preimage<'_>) -> Result<KeyedPreimage, NodeKeyRefusal> {
    let preimage = canonical_bytes(preimage)?;
    let key = NodeKey::from_digest(ByteDigest::of(&preimage).as_bytes());
    Ok(KeyedPreimage { preimage, key })
}

/// `value`'s RFC 8785 bytes, from `quire-canonical` (ADR-013 §2,
/// ADR-013:113: the one RFC 8785 implementation).
pub(super) fn canonical_bytes(value: &(impl Encode + ?Sized)) -> Result<Vec<u8>, NodeKeyRefusal> {
    quire_canonical::to_vec(value, LIMITS).map_err(|error| NodeKeyRefusal::Encode {
        reason: error.to_string(),
    })
}

/// The SHA-256 of `value`'s RFC 8785 bytes, hashed by `quire-canonical` as
/// it encodes (ADR-013:113).
fn canonical_sha256(value: &(impl Encode + ?Sized)) -> Result<[u8; 32], NodeKeyRefusal> {
    quire_canonical::sha256(value, LIMITS)
        .map(|digest| *digest.as_bytes())
        .map_err(|error| NodeKeyRefusal::Encode {
            reason: error.to_string(),
        })
}

/// `node`'s typed preimage, with `walk` rewriting references to its group's
/// members, and whether it has an application. A structural preimage's
/// `recursion` keeps `group`; an application preimage's drops it.
fn typed_preimage<'a>(
    node: &NodeInput<'a>,
    walk: Walk<'_>,
    recursion: Option<RecursionPreimage>,
) -> Result<(Preimage<'a>, bool), NodeKeyRefusal> {
    if node.semantic_form.is_empty() {
        return Err(NodeKeyRefusal::EmptySemanticForm);
    }
    if node.declaration.is_some_and(<[Identifier]>::is_empty) {
        return Err(NodeKeyRefusal::EmptyQualifiedName);
    }
    let (body, has_application) = walk.body(node.body)?;
    let (version, recursion) = if has_application {
        if node.owner.is_some() {
            return Err(NodeKeyRefusal::OwnedApplication);
        }
        if node.semantic_type.is_none() {
            return Err(NodeKeyRefusal::UntypedApplication);
        }
        // FR-322: an application node's `recursion` is `{size, ordinal}`.
        let recursion = recursion.map(|recursion| RecursionPreimage {
            group: None,
            ..recursion
        });
        (APPLICATION_NODE_VERSION, recursion)
    } else {
        // FR-092: a `SourceOwner` exactly when `declaration` is present;
        // FR-094: a model-owned node's `declaration` is `null`.
        let consistent = match node.owner {
            Some(Owner::Source(_)) => node.declaration.is_some(),
            Some(Owner::Model(_)) | None => node.declaration.is_none(),
        };
        if !consistent {
            return Err(NodeKeyRefusal::OwnerDeclarationMismatch);
        }
        (STRUCTURAL_NODE_VERSION, recursion)
    };
    let semantic_type = node
        .semantic_type
        .map(|semantic_type| match walk.ordinal(semantic_type) {
            Some(ordinal) => SemanticTypePreimage::Group(GroupReference { ordinal }),
            None => SemanticTypePreimage::Node(NodeRef(semantic_type)),
        });
    let preimage = Preimage {
        version,
        owner: node.owner,
        node_tag: node.node_tag,
        semantic_form: node.semantic_form,
        semantic_type,
        declaration: node.declaration.map(|segments| DeclarationPreimage {
            qualified_name: segments.iter().map(Identifier::as_str).collect(),
        }),
        recursion,
        body,
    };
    Ok((preimage, has_application))
}

/// A node's preimage. Its `body` nests as deep as the term it keys, so it
/// is written through `quire-canonical`'s event API from an explicit stack
/// ([`PreimageTerm`]'s [`Encode`]); every other member has a fixed shape.
struct Preimage<'a> {
    version: &'static str,
    owner: Option<&'a Owner>,
    node_tag: NodeTag,
    semantic_form: &'a str,
    semantic_type: Option<SemanticTypePreimage>,
    declaration: Option<DeclarationPreimage<'a>>,
    recursion: Option<RecursionPreimage>,
    body: PreimageTerm<'a>,
}

impl Encode for Preimage<'_> {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_object()?;
        writer.name("version")?;
        writer.string(self.version)?;
        if let Some(owner) = self.owner {
            writer.name("owner")?;
            writer.serialize(owner)?;
        }
        writer.name("node_tag")?;
        writer.serialize(&self.node_tag)?;
        writer.name("semantic_form")?;
        writer.string(self.semantic_form)?;
        writer.name("semantic_type")?;
        writer.serialize(&self.semantic_type)?;
        writer.name("declaration")?;
        writer.serialize(&self.declaration)?;
        writer.name("recursion")?;
        writer.serialize(&self.recursion)?;
        writer.name("body")?;
        self.body.encode_into(writer)?;
        writer.end_object()
    }
}

/// A `semantic_type` as it enters the preimage: a `NodeRef`, or a
/// `group_reference` when it names a member of the node's own group (G9).
#[derive(Serialize, FixedShape)]
#[serde(untagged)]
enum SemanticTypePreimage {
    /// A type outside the node's group, by its `NodeRef`.
    Node(NodeRef),
    /// A member of the node's own group, by its group ordinal.
    Group(GroupReference),
}

/// `{"term": "group_reference", "ordinal": n}`: the same text as
/// [`PreimageLeaf::GroupReference`], the one shape a `semantic_type` naming
/// a member of the node's own group has.
#[derive(Serialize, FixedShape)]
#[serde(tag = "term", rename = "group_reference")]
struct GroupReference {
    ordinal: usize,
}

#[derive(Serialize, FixedShape)]
struct DeclarationPreimage<'a> {
    qualified_name: Vec<&'a str>,
}

#[derive(Serialize, FixedShape)]
struct RecursionPreimage {
    #[serde(skip_serializing_if = "Option::is_none")]
    group: Option<String>,
    ordinal: usize,
    size: usize,
}

/// A body term as it enters the preimage: [`SemanticTerm`] with references
/// to recursion-group members replaced by their group ordinal. A leaf has a
/// fixed shape; the three composite terms nest their subterms.
enum PreimageTerm<'a> {
    Leaf(PreimageLeaf<'a>),
    Application {
        operator: Operator,
        operation: &'a Operation,
        result_type: NodeRef,
        arguments: Vec<PreimageTerm<'a>>,
    },
    Aggregate {
        members: Vec<PreimageTerm<'a>>,
    },
    Binding {
        name: &'a str,
        value: Box<PreimageTerm<'a>>,
    },
}

/// A preimage term with no subterm.
#[derive(Serialize, FixedShape)]
#[serde(tag = "term", rename_all = "snake_case")]
enum PreimageLeaf<'a> {
    Literal {
        #[serde(rename = "type")]
        ty: NodeRef,
        #[serde(flatten)]
        value: &'a LiteralValue,
    },
    Reference {
        target: NodeRef,
    },
    GroupReference {
        ordinal: usize,
    },
    DependencyReference {
        package: PackageRef,
        node: WireNodeRef,
    },
    Frame {
        modifies: &'a [FrameField],
        creates: &'a [NodeRef],
        deletes: &'a [NodeRef],
    },
}

/// The term's RFC 8785 text, written from an explicit heap stack of
/// pending subterms and closings, so encoding never recurses with the
/// term's depth.
impl Encode for PreimageTerm<'_> {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        enum Task<'t, 'a> {
            Term(&'t PreimageTerm<'a>),
            EndArray,
            EndObject,
        }
        let mut tasks = vec![Task::Term(self)];
        while let Some(task) = tasks.pop() {
            match task {
                Task::Term(PreimageTerm::Leaf(leaf)) => writer.serialize(leaf)?,
                Task::Term(PreimageTerm::Application {
                    operator,
                    operation,
                    result_type,
                    arguments,
                }) => {
                    writer.begin_object()?;
                    writer.name("term")?;
                    writer.string("application")?;
                    writer.name("operator")?;
                    writer.serialize(operator)?;
                    writer.name("operation")?;
                    writer.serialize(*operation)?;
                    writer.name("result_type")?;
                    writer.serialize(result_type)?;
                    writer.name("arguments")?;
                    writer.begin_array()?;
                    tasks.push(Task::EndObject);
                    tasks.push(Task::EndArray);
                    tasks.extend(arguments.iter().rev().map(Task::Term));
                }
                Task::Term(PreimageTerm::Aggregate { members }) => {
                    writer.begin_object()?;
                    writer.name("term")?;
                    writer.string("aggregate")?;
                    writer.name("members")?;
                    writer.begin_array()?;
                    tasks.push(Task::EndObject);
                    tasks.push(Task::EndArray);
                    tasks.extend(members.iter().rev().map(Task::Term));
                }
                Task::Term(PreimageTerm::Binding { name, value }) => {
                    writer.begin_object()?;
                    writer.name("term")?;
                    writer.string("binding")?;
                    writer.name("name")?;
                    writer.string(name)?;
                    writer.name("value")?;
                    tasks.push(Task::EndObject);
                    tasks.push(Task::Term(value));
                }
                Task::EndArray => writer.end_array()?,
                Task::EndObject => writer.end_object()?,
            }
        }
        Ok(())
    }
}

/// `member` with its declaring node replaced by `map`'s image.
fn map_member(member: Member, map: &mut impl FnMut(NodeKey) -> NodeKey) -> Member {
    match member {
        Member::Field { declaration, name } => Member::Field {
            declaration: map(declaration),
            name,
        },
        Member::Position {
            declaration,
            position,
        } => Member::Position {
            declaration: map(declaration),
            position,
        },
        Member::Element { declaration } => Member::Element {
            declaration: map(declaration),
        },
        Member::RelationshipEnd { declaration, name } => Member::RelationshipEnd {
            declaration: map(declaration),
            name,
        },
        Member::Operation { declaration, name } => Member::Operation {
            declaration: map(declaration),
            name,
        },
        Member::TypeArgument { declaration } => Member::TypeArgument {
            declaration: map(declaration),
        },
        Member::ProfileOperator { operator } => Member::ProfileOperator { operator },
        // No `declaration`: nothing for `map` to remap.
        Member::StateClause { clause } => Member::StateClause { clause },
    }
}

/// One pass over a body: builds its preimage form, reports whether it is
/// an application, and enforces the number bounds.
#[derive(Clone, Copy)]
struct Walk<'g> {
    /// The recursion group's members by handle, each with the ordinal its
    /// `group_reference` carries.
    group: &'g BTreeMap<NodeKey, usize>,
}

/// The empty group of a node outside every recursion group.
static NO_GROUP: BTreeMap<NodeKey, usize> = BTreeMap::new();

impl Walk<'_> {
    /// The walk of a node outside every group.
    const OUTSIDE: Walk<'static> = Walk { group: &NO_GROUP };

    /// The `group_reference` ordinal of `key`, when it names a member.
    fn ordinal(&self, key: NodeKey) -> Option<usize> {
        self.group.get(&key).copied()
    }

    /// Refuse `key` at a type position when it names a member.
    fn type_position(&self, key: NodeKey) -> Result<(), NodeKeyRefusal> {
        match self.ordinal(key) {
            Some(_) => Err(NodeKeyRefusal::GroupMemberAtTypePosition),
            None => Ok(()),
        }
    }

    /// `leaf` in preimage form.
    fn leaf<'a>(&self, leaf: &'a LeafTerm) -> Result<PreimageTerm<'a>, NodeKeyRefusal> {
        Ok(match leaf {
            LeafTerm::Literal { ty, value } => {
                self.type_position(ty.0)?;
                PreimageTerm::Leaf(PreimageLeaf::Literal { ty: *ty, value })
            }
            LeafTerm::Reference { target } => match self.ordinal(target.0) {
                Some(ordinal) => PreimageTerm::Leaf(PreimageLeaf::GroupReference { ordinal }),
                None => PreimageTerm::Leaf(PreimageLeaf::Reference { target: *target }),
            },
            // ADR-015 D-5: the dependency's `package_id` and node id enter
            // the preimage as written; neither names a node of this graph.
            LeafTerm::DependencyReference { package, node } => {
                PreimageTerm::Leaf(PreimageLeaf::DependencyReference {
                    package: *package,
                    node: *node,
                })
            }
        })
    }

    /// `binding` of a value already in preimage form.
    fn binding<'a>(
        name: &'a str,
        value: PreimageTerm<'a>,
    ) -> Result<PreimageTerm<'a>, NodeKeyRefusal> {
        if name.is_empty() {
            return Err(NodeKeyRefusal::EmptyBindingName);
        }
        Ok(PreimageTerm::Binding {
            name,
            value: Box::new(value),
        })
    }

    /// A binding of a Leaf in preimage form.
    fn bound_leaf<'a>(
        &self,
        binding: &'a Binding<LeafTerm>,
    ) -> Result<PreimageTerm<'a>, NodeKeyRefusal> {
        Self::binding(&binding.name, self.leaf(&binding.value)?)
    }

    /// `group` in preimage form.
    fn group<'a>(&self, group: &'a GroupTerm) -> Result<PreimageTerm<'a>, NodeKeyRefusal> {
        let members = group
            .members
            .iter()
            .map(|member| match member {
                GroupMember::Leaf(leaf) => self.leaf(leaf),
                GroupMember::Binding(binding) => self.bound_leaf(binding),
            })
            .collect::<Result<_, _>>()?;
        Ok(PreimageTerm::Aggregate { members })
    }

    /// `tuple` in preimage form.
    fn tuple<'a>(&self, tuple: &'a TupleTerm) -> Result<PreimageTerm<'a>, NodeKeyRefusal> {
        let members = tuple
            .members
            .iter()
            .map(|member| match member {
                TupleMember::Leaf(leaf) => self.leaf(leaf),
                TupleMember::Group(group) => self.group(group),
            })
            .collect::<Result<_, _>>()?;
        Ok(PreimageTerm::Aggregate { members })
    }

    /// `member` in preimage form.
    fn member<'a>(&self, member: &'a MemberTerm) -> Result<PreimageTerm<'a>, NodeKeyRefusal> {
        match member {
            MemberTerm::Leaf(leaf) => self.leaf(leaf),
            MemberTerm::Group(group) => self.group(group),
            MemberTerm::Binding(binding) => {
                let value = match &binding.value {
                    BindingValue::Leaf(leaf) => self.leaf(leaf)?,
                    BindingValue::Group(group) => self.group(group)?,
                    BindingValue::Tuple(tuple) => self.tuple(tuple)?,
                };
                Self::binding(&binding.name, value)
            }
        }
    }

    /// `body` in preimage form, and whether it is an application. Each
    /// stratum's function calls only the strata below it, so the walk is a
    /// fixed-depth match whatever the expression's depth (FR-258).
    fn body<'a>(&self, body: &'a BodyTerm) -> Result<(PreimageTerm<'a>, bool), NodeKeyRefusal> {
        let members = |members: &'a [MemberTerm]| {
            members
                .iter()
                .map(|member| self.member(member))
                .collect::<Result<Vec<_>, _>>()
        };
        Ok(match body {
            BodyTerm::Leaf(leaf) => (self.leaf(leaf)?, false),
            BodyTerm::Application(ApplicationTerm {
                operator,
                operation,
                result_type,
                arguments,
            }) => {
                if let Some(Member::Position { position, .. }) = &operation.member {
                    exact_integer(IntegerSite::MemberPosition, *position)?;
                }
                if let Some(declaration) = operation.member.as_ref().and_then(Member::declaration) {
                    self.type_position(declaration)?;
                }
                self.type_position(result_type.0)?;
                let preimage = PreimageTerm::Application {
                    operator: *operator,
                    operation,
                    result_type: *result_type,
                    arguments: members(arguments)?,
                };
                (preimage, true)
            }
            BodyTerm::Aggregate(AggregateTerm { members: terms }) => (
                PreimageTerm::Aggregate {
                    members: members(terms)?,
                },
                false,
            ),
            // QSpec FR-340: a frame body holds no application, and its
            // `modifies`/`creates`/`deletes` entries are plain `NodeRef`
            // members, never `reference` terms -- so, unlike every other
            // position above, group substitution does not apply here. A
            // `state`/`frame` node is never part of a recursion group (only
            // a function's own body can be), so this is a direct passthrough,
            // not a gap in the group-reference rule.
            BodyTerm::Frame(FrameTerm {
                modifies,
                creates,
                deletes,
            }) => (
                PreimageTerm::Leaf(PreimageLeaf::Frame {
                    modifies,
                    creates,
                    deletes,
                }),
                false,
            ),
        })
    }
}

mod shape;

#[cfg(test)]
mod tests;
