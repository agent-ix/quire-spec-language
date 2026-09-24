// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked node keys (QSL-156): the FR-322 `quire.application-node/v1`
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
//! The body is typed ([`SemanticTerm`] and its parts mirror the v2 schema's
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
//! implementation (ADR-013 §2, ADR-013:113), which orders members itself. The
//! group order's shapes are the one place a preimage is rewritten rather than
//! built: its canonical bytes are read back into a `serde_json::Value`, whose
//! placeholders are removed (and, for the anonymous shape, whose `owner` and
//! `declaration` are cleared) before the shape is encoded again. The only
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
//! The body walk is bounded: a body nested deeper than
//! [`MAX_CHECKING_DEPTH`] terms is refused, whatever path built it.
//!
//! Conformance against QSpec's own published `operation_vectors` is the
//! opt-in `conformance` test (`make conformance` with `QSPEC_DIR` pointing at
//! a quire-specification checkout). QSpec is not public yet, so nothing of it
//! is copied here; the test reads the vectors at run time.

use std::collections::{BTreeMap, BTreeSet};

use qsl_foundation::ByteDigest;
use serde::Serialize;

use crate::value::semantic_node::IDENTITY_LIMITS as LIMITS;

use qsl_foundation::absence::AbsenceMode;
use quire_exact::{Identifier, Integer, Rational, RoundingMode, TextProfile};

use super::MAX_CHECKING_DEPTH;
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
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SourceOwner {
    authority: String,
    identity: String,
    kind: SourceOwnerKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
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
/// `ModelOwner{kind: "model", identity, version, node}`. `identity` and
/// `version` are the declaring domain package's `DomainPackageRef`'s;
/// `node` is the declaration's IR node identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ModelOwner {
    identity: String,
    kind: ModelOwnerKind,
    node: String,
    version: String,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
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
    /// `version` is empty.
    #[error("the model owner's version is empty")]
    EmptyVersion,
    /// `node` is empty.
    #[error("the model owner's node is empty")]
    EmptyNode,
}

impl ModelOwner {
    /// The model owner `{kind: "model", identity, version, node}`.
    pub fn new(
        identity: impl Into<String>,
        version: impl Into<String>,
        node: impl Into<String>,
    ) -> Result<Self, InvalidModelOwner> {
        let identity = identity.into();
        let version = version.into();
        let node = node.into();
        if identity.is_empty() {
            return Err(InvalidModelOwner::EmptyIdentity);
        }
        if version.is_empty() {
            return Err(InvalidModelOwner::EmptyVersion);
        }
        if node.is_empty() {
            return Err(InvalidModelOwner::EmptyNode);
        }
        Ok(Self {
            identity,
            kind: ModelOwnerKind::Model,
            node,
            version,
        })
    }

    /// The declaring domain package's identity.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// The declaring domain package's version.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// The declaration's IR node identity.
    pub fn node(&self) -> &str {
        &self.node
    }
}

/// A structural node's `owner` (FR-092, FR-094): a source-declared node's
/// [`SourceOwner`] or a model-owned node's [`ModelOwner`]. Serialized as the
/// owner itself; its `kind` member tells the two apart.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
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
        #[derive(Serialize)]
        struct Wire<'a> {
            domain: &'static str,
            digest: &'a str,
        }
        // `NodeKey`'s lowercase-hex spelling, written on the stack: a
        // preimage names many keys, and each is spelled without
        // allocating or formatting (QSL-221).
        let digits = HexDigest::of(self.0.as_bytes());
        Wire {
            domain: NODE_KEY_DOMAIN,
            digest: digits.as_str(),
        }
        .serialize(serializer)
    }
}

/// The preimage schema's closed `node_tag` vocabulary.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
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

/// The v2 schema's closed `SemanticTerm` union.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
// No `deny_unknown_fields`: serde does not combine it with the literal's
// `flatten`, and the conformance test compares re-encoded bytes with the
// published ones, so a dropped member fails there instead.
#[serde(tag = "term", rename_all = "snake_case")]
pub enum SemanticTerm {
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
    /// An operation application.
    Application {
        /// The operator class.
        operator: Operator,
        /// The catalogued operation and its laws.
        operation: Operation,
        /// The application's result type node.
        result_type: NodeRef,
        /// The operands, in order.
        arguments: Vec<SemanticTerm>,
    },
    /// An ordered aggregate of terms.
    Aggregate {
        /// The members, in order.
        members: Vec<SemanticTerm>,
    },
    /// A named binding of a term.
    Binding {
        /// The bound name (schema `Nonempty`; an empty name is refused).
        name: String,
        /// The bound term.
        value: Box<SemanticTerm>,
    },
}

impl SemanticTerm {
    /// `{term: "reference", target}`.
    pub(crate) fn reference(target: NodeKey) -> Self {
        Self::Reference {
            target: NodeRef(target),
        }
    }

    /// `{term: "binding", name, value}`.
    pub(crate) fn binding(name: impl Into<String>, value: Self) -> Self {
        Self::Binding {
            name: name.into(),
            value: Box::new(value),
        }
    }

    /// `{term: "literal", type, value_kind, value}`.
    pub(crate) fn literal(ty: NodeKey, value: LiteralValue) -> Self {
        Self::Literal {
            ty: NodeRef(ty),
            value,
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
}

impl Serialize for LiteralValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(2))?;
        match self {
            Self::Boolean(value) => {
                map.serialize_entry("value_kind", "boolean")?;
                map.serialize_entry("value", value)?;
            }
            Self::Integer(value) => {
                map.serialize_entry("value_kind", "integer")?;
                map.serialize_entry("value", &value.to_string())?;
            }
            Self::Rational(value) => {
                map.serialize_entry("value_kind", "rational")?;
                map.serialize_entry(
                    "value",
                    &format!("{}/{}", value.numerator(), value.denominator()),
                )?;
            }
            Self::Text(value) => {
                map.serialize_entry("value_kind", "text")?;
                map.serialize_entry("value", value)?;
            }
            Self::None => {
                map.serialize_entry("value_kind", "none")?;
                map.serialize_entry("value", &())?;
            }
        }
        map.end()
    }
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
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
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
            Self::Claim => "claim",
        }
    }
}

/// The schema's `Operation`: a catalogued operation with its laws, mode,
/// member and leaves.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(deny_unknown_fields)]
pub struct OperationLaw {
    /// The law's role.
    pub role: LawRole,
    /// The selected definition, digest included.
    pub definition: DefinitionReference,
}

/// The schema's closed `OperationLaw.role` vocabulary.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
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
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum OperationMode {
    /// `rounding`.
    Rounding(#[serde(serialize_with = "rounding_spelling")] RoundingMode),
    /// `text_profile`.
    TextProfile(#[serde(serialize_with = "text_profile_spelling")] TextProfile),
    /// `absence`.
    Absence(#[serde(serialize_with = "absence_spelling")] AbsenceMode),
}

fn rounding_spelling<S: serde::Serializer>(
    mode: &RoundingMode,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(mode.as_str())
}

fn text_profile_spelling<S: serde::Serializer>(
    profile: &TextProfile,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(profile.as_str())
}

fn absence_spelling<S: serde::Serializer>(
    mode: &AbsenceMode,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(mode.as_str())
}

/// The schema's `OperationLeaf`.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
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
    pub(crate) body: &'a SemanticTerm,
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
    pub(crate) body: &'a SemanticTerm,
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
    /// The body nests deeper than [`MAX_CHECKING_DEPTH`] terms.
    #[error("the body nests deeper than {limit} terms")]
    TooDeep {
        /// The depth limit.
        limit: u64,
        /// The depth the refused term is at.
        actual: u64,
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
    let (_, has_application) = Walk::OUTSIDE.term(node.body, 1)?;
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
#[derive(Serialize)]
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

/// `preimage`'s RFC 8785 bytes and the node key they hash to.
fn keyed(preimage: &Preimage<'_>) -> Result<KeyedPreimage, NodeKeyRefusal> {
    let preimage = canonical_bytes(preimage)?;
    let key = NodeKey::from_digest(ByteDigest::of(&preimage).as_bytes());
    Ok(KeyedPreimage { preimage, key })
}

/// `value`'s RFC 8785 bytes, from `quire-canonical` (ADR-013 §2,
/// ADR-013:113: the one RFC 8785 implementation).
pub(super) fn canonical_bytes(value: &impl Serialize) -> Result<Vec<u8>, NodeKeyRefusal> {
    quire_canonical::to_vec(value, LIMITS).map_err(|error| NodeKeyRefusal::Encode {
        reason: error.to_string(),
    })
}

/// The SHA-256 of `value`'s RFC 8785 bytes, hashed by `quire-canonical` as
/// it encodes (ADR-013:113).
fn canonical_sha256(value: &impl Serialize) -> Result<[u8; 32], NodeKeyRefusal> {
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
    let (body, has_application) = walk.term(node.body, 1)?;
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
            Some(ordinal) => PreimageTerm::GroupReference { ordinal },
            None => PreimageTerm::Reference {
                target: NodeRef(semantic_type),
            },
        });
    let preimage = Preimage {
        version,
        owner: node.owner,
        node_tag: node.node_tag,
        semantic_form: node.semantic_form,
        semantic_type: semantic_type.map(SemanticTypePreimage),
        declaration: node.declaration.map(|segments| DeclarationPreimage {
            qualified_name: segments.iter().map(Identifier::as_str).collect(),
        }),
        recursion,
        body,
    };
    Ok((preimage, has_application))
}

#[derive(Serialize)]
struct Preimage<'a> {
    version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner: Option<&'a Owner>,
    node_tag: NodeTag,
    semantic_form: &'a str,
    semantic_type: Option<SemanticTypePreimage<'a>>,
    declaration: Option<DeclarationPreimage<'a>>,
    recursion: Option<RecursionPreimage>,
    body: PreimageTerm<'a>,
}

/// A `semantic_type` as it enters the preimage: a `NodeRef`, or a
/// `group_reference` when it names a member of the node's own group (G9).
struct SemanticTypePreimage<'a>(PreimageTerm<'a>);

impl Serialize for SemanticTypePreimage<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match &self.0 {
            PreimageTerm::Reference { target } => target.serialize(serializer),
            term => term.serialize(serializer),
        }
    }
}

#[derive(Serialize)]
struct DeclarationPreimage<'a> {
    qualified_name: Vec<&'a str>,
}

#[derive(Serialize)]
struct RecursionPreimage {
    #[serde(skip_serializing_if = "Option::is_none")]
    group: Option<String>,
    ordinal: usize,
    size: usize,
}

/// A body term as it enters the preimage: [`SemanticTerm`] with references
/// to recursion-group members replaced by their group ordinal.
#[derive(Serialize)]
#[serde(tag = "term", rename_all = "snake_case")]
enum PreimageTerm<'a> {
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

impl SemanticTerm {
    /// Call `visit` with every node key this term names (FR-092 "names"):
    /// each `reference` target, literal `type`, application `result_type`
    /// and operation member `declaration`.
    pub(crate) fn for_each_key(&self, visit: &mut impl FnMut(NodeKey)) {
        match self {
            Self::Literal { ty, .. } => visit(ty.0),
            Self::Reference { target } => visit(target.0),
            Self::Application {
                operation,
                result_type,
                arguments,
                ..
            } => {
                if let Some(declaration) = operation.member.as_ref().and_then(Member::declaration) {
                    visit(declaration);
                }
                visit(result_type.0);
                for argument in arguments {
                    argument.for_each_key(visit);
                }
            }
            Self::Aggregate { members } => {
                for member in members {
                    member.for_each_key(visit);
                }
            }
            Self::Binding { value, .. } => value.for_each_key(visit),
        }
    }

    /// This term with every node key it names replaced by `map`'s image.
    pub(crate) fn map_keys(&self, map: &mut impl FnMut(NodeKey) -> NodeKey) -> Self {
        match self {
            Self::Literal { ty, value } => Self::Literal {
                ty: NodeRef(map(ty.0)),
                value: value.clone(),
            },
            Self::Reference { target } => Self::Reference {
                target: NodeRef(map(target.0)),
            },
            Self::Application {
                operator,
                operation,
                result_type,
                arguments,
            } => {
                let mut operation = operation.clone();
                operation.member = operation.member.map(|member| map_member(member, map));
                Self::Application {
                    operator: *operator,
                    operation,
                    result_type: NodeRef(map(result_type.0)),
                    arguments: arguments
                        .iter()
                        .map(|argument| argument.map_keys(map))
                        .collect(),
                }
            }
            Self::Aggregate { members } => Self::Aggregate {
                members: members.iter().map(|member| member.map_keys(map)).collect(),
            },
            Self::Binding { name, value } => Self::Binding {
                name: name.clone(),
                value: Box::new(value.map_keys(map)),
            },
        }
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
    }
}

/// One pass over a body: builds its preimage form, reports whether it
/// contains an application, and enforces the depth and number bounds.
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

    /// `term` at nesting `depth` (the body root is depth 1) in preimage form,
    /// and whether it is or contains an application.
    fn term<'a>(
        &self,
        term: &'a SemanticTerm,
        depth: u64,
    ) -> Result<(PreimageTerm<'a>, bool), NodeKeyRefusal> {
        if depth > MAX_CHECKING_DEPTH {
            return Err(NodeKeyRefusal::TooDeep {
                limit: MAX_CHECKING_DEPTH,
                actual: depth,
            });
        }
        let terms = |terms: &'a [SemanticTerm]| {
            terms.iter().try_fold(
                (Vec::with_capacity(terms.len()), false),
                |(mut mapped, any), term| {
                    let (preimage, has) = self.term(term, depth + 1)?;
                    mapped.push(preimage);
                    Ok::<_, NodeKeyRefusal>((mapped, any || has))
                },
            )
        };
        Ok(match term {
            SemanticTerm::Literal { ty, value } => {
                self.type_position(ty.0)?;
                (PreimageTerm::Literal { ty: *ty, value }, false)
            }
            SemanticTerm::Reference { target } => {
                let preimage = match self.ordinal(target.0) {
                    Some(ordinal) => PreimageTerm::GroupReference { ordinal },
                    None => PreimageTerm::Reference { target: *target },
                };
                (preimage, false)
            }
            SemanticTerm::Application {
                operator,
                operation,
                result_type,
                arguments,
            } => {
                if let Some(Member::Position { position, .. }) = &operation.member {
                    exact_integer(IntegerSite::MemberPosition, *position)?;
                }
                if let Some(declaration) = operation.member.as_ref().and_then(Member::declaration) {
                    self.type_position(declaration)?;
                }
                self.type_position(result_type.0)?;
                let (arguments, _) = terms(arguments)?;
                let preimage = PreimageTerm::Application {
                    operator: *operator,
                    operation,
                    result_type: *result_type,
                    arguments,
                };
                (preimage, true)
            }
            SemanticTerm::Aggregate { members } => {
                let (members, has) = terms(members)?;
                (PreimageTerm::Aggregate { members }, has)
            }
            SemanticTerm::Binding { name, value } => {
                if name.is_empty() {
                    return Err(NodeKeyRefusal::EmptyBindingName);
                }
                let (value, has) = self.term(value, depth + 1)?;
                let preimage = PreimageTerm::Binding {
                    name,
                    value: Box::new(value),
                };
                (preimage, has)
            }
        })
    }
}

mod shape;

#[cfg(test)]
mod tests;
