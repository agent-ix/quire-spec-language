// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    dead_code,
    reason = "no production caller yet: QSL-156 slice A4b switches node identity to this key once the lock-evidence and type-node rulings land; until then only this module's own tests call it"
)]
//! QSpec FR-322 `application_node_preimage` (QSL-156 slice A4a): the node key
//! of every checked node whose body contains an application.
//!
//! FR-322 keys such a node by the JCS SHA-256 of
//! `{version: "quire.application-node/v1", node_tag, semantic_form,
//! semantic_type, declaration, recursion, body}`, where `declaration` is the
//! node's `declaration` member or `null`, `recursion` is `null` or
//! `{size, ordinal}` of the node inside its recursion group in graph order,
//! and a body `reference` to a member of that group becomes
//! `{term: "group_reference", ordinal}`. [`application_node_key`] builds
//! exactly that preimage from an [`ApplicationNode`] and hashes it.
//!
//! The body is typed ([`SemanticTerm`] and its parts mirror the v2 schema's
//! closed `SemanticTerm`, `Operation`, `OperationLaw`, `OperationMode` and
//! `OperationLeaf` definitions), so the preimage has a field list the compiler
//! checks rather than a `serde_json::Value` assembled by hand. Type-node keys
//! (`semantic_type`, `result_type`, literal `type`) are inputs, used exactly as
//! given: how builtin and anonymous type nodes are keyed is a separate ruling
//! (OQ-7), wired in by A4b.
//!
//! Canonical bytes: the typed preimage is converted to a `serde_json::Value`
//! and serialized. `serde_json`'s map is ordered by key (this crate does not
//! enable `preserve_order`), every preimage key is a fixed ASCII schema name,
//! so byte order equals RFC 8785's UTF-16 code-unit order; strings use
//! `serde_json`'s escaping, which matches RFC 8785 for the ASCII escapes it
//! emits. The only numbers are `recursion`'s counts and literal integers;
//! an integer RFC 8785 cannot render exactly (outside the IEEE-754 safe
//! range) is refused rather than hashed. `tests::preimage_bytes_are_pinned`
//! pins the exact bytes.
//!
//! Conformance against QSpec's own published `operation_vectors` is the
//! opt-in `conformance` test below (`make conformance` with `QSPEC_DIR`
//! pointing at a quire-specification checkout). QSpec is not public yet, so
//! nothing of it is copied here; the test reads the vectors at run time.

use serde::Serialize;
use sha2::{Digest, Sha256};

use quire_exact::Identifier;

use super::definition::DefinitionReference;
use super::member::Member;
use super::node::{NodeKey, NODE_KEY_DOMAIN};

/// FR-322's application-node preimage version.
pub(crate) const APPLICATION_NODE_VERSION: &str = "quire.application-node/v1";

/// The largest integer magnitude RFC 8785 renders exactly (2^53 - 1).
const JCS_SAFE_INTEGER: i64 = (1 << 53) - 1;

/// A `NodeRef` (`{domain, digest}`) naming a checked node by key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NodeRef(pub(crate) NodeKey);

impl Serialize for NodeRef {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire {
            domain: &'static str,
            digest: String,
        }
        Wire {
            domain: NODE_KEY_DOMAIN,
            digest: self.0.to_string(),
        }
        .serialize(serializer)
    }
}

/// The v2 schema's closed `SemanticTerm` union.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(tag = "term", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum SemanticTerm {
    /// A typed literal.
    Literal {
        /// The literal's type node.
        #[serde(rename = "type")]
        ty: NodeRef,
        /// The literal's value kind.
        value_kind: LiteralKind,
        /// The literal's JSON value; `None` is JSON `null`.
        value: Option<LiteralValue>,
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
        /// The bound name (schema `Nonempty`).
        name: String,
        /// The bound term.
        value: Box<SemanticTerm>,
    },
}

impl SemanticTerm {
    /// Whether this term is, or contains, an application.
    fn contains_application(&self) -> bool {
        match self {
            Self::Application { .. } => true,
            Self::Aggregate { members } => members.iter().any(Self::contains_application),
            Self::Binding { value, .. } => value.contains_application(),
            Self::Literal { .. } | Self::Reference { .. } => false,
        }
    }
}

/// The schema's closed literal `value_kind` vocabulary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(rename_all = "snake_case")]
pub(crate) enum LiteralKind {
    /// `boolean`.
    Boolean,
    /// `integer`.
    Integer,
    /// `rational`.
    Rational,
    /// `decimal`.
    Decimal,
    /// `float32_bits`.
    Float32Bits,
    /// `float64_bits`.
    Float64Bits,
    /// `text`.
    Text,
    /// `enum`.
    Enum,
    /// `none`.
    None,
}

/// A non-null literal JSON value: the schema admits boolean, string or
/// integer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(untagged)]
pub(crate) enum LiteralValue {
    /// A JSON boolean.
    Boolean(bool),
    /// A JSON integer.
    Integer(i64),
    /// A JSON string.
    Text(String),
}

/// The schema's closed application `operator` vocabulary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(rename_all = "snake_case")]
pub(crate) enum Operator {
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

/// The schema's `Operation`: a catalogued operation with its laws, mode,
/// member and leaves.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(deny_unknown_fields)]
pub(crate) struct Operation {
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

/// The schema's `OperationLaw`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(deny_unknown_fields)]
pub(crate) struct OperationLaw {
    /// The law's role.
    pub(crate) role: LawRole,
    /// The selected definition, digest included.
    pub(crate) definition: DefinitionReference,
}

/// The schema's closed `OperationLaw.role` vocabulary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(rename_all = "snake_case")]
pub(crate) enum LawRole {
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

/// The schema's non-null `OperationMode` (`{kind, value}`).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(crate) enum OperationMode {
    /// `rounding`.
    Rounding(RoundingMode),
    /// `text_profile`.
    TextProfile(TextProfileMode),
    /// `absence`.
    Absence(AbsenceMode),
}

/// The `rounding` mode values.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RoundingMode {
    /// `exact`.
    Exact,
    /// `toward-zero`.
    TowardZero,
    /// `toward-positive`.
    TowardPositive,
    /// `toward-negative`.
    TowardNegative,
    /// `nearest-even`.
    NearestEven,
    /// `nearest-away`.
    NearestAway,
}

/// The `text_profile` mode values.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(rename_all = "kebab-case")]
pub(crate) enum TextProfileMode {
    /// `unicode-scalars`.
    UnicodeScalars,
    /// `nfc`.
    Nfc,
    /// `nfd`.
    Nfd,
    /// `nfkc`.
    Nfkc,
    /// `nfkd`.
    Nfkd,
    /// `binary-utf8`.
    BinaryUtf8,
}

/// The `absence` mode values.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AbsenceMode {
    /// `undefined`.
    Undefined,
    /// `empty`.
    Empty,
    /// `refused`.
    Refused,
}

/// The schema's `OperationLeaf`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(deny_unknown_fields)]
pub(crate) struct OperationLeaf {
    /// The leaf's path from the compared type.
    pub(crate) path: Vec<LeafSegment>,
    /// The leaf's profile laws.
    pub(crate) laws: Vec<OperationLaw>,
    /// The leaf's mode, or `None` (JSON `null`).
    pub(crate) mode: Option<OperationMode>,
}

/// One schema `LeafSegment`: `field:<identifier>`, `position:<n>` or
/// `inner`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LeafSegment {
    /// `field:<name>`.
    Field(Identifier),
    /// `position:<n>`.
    Position(u64),
    /// `inner`.
    Inner,
}

impl Serialize for LeafSegment {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Field(name) => serializer.collect_str(&format_args!("field:{}", name.as_str())),
            Self::Position(position) => {
                serializer.collect_str(&format_args!("position:{position}"))
            }
            Self::Inner => serializer.serialize_str("inner"),
        }
    }
}

/// A node's place in its recursion group: the group's member keys in graph
/// order and this node's ordinal among them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RecursionGroup<'a> {
    members: &'a [NodeKey],
    ordinal: usize,
}

impl<'a> RecursionGroup<'a> {
    /// The group `members` (graph order) with this node at `ordinal`, or
    /// `None` when `ordinal` is not a position in `members`.
    pub(crate) fn new(members: &'a [NodeKey], ordinal: usize) -> Option<Self> {
        (ordinal < members.len()).then_some(Self { members, ordinal })
    }
}

/// The inputs of one application-bearing node's FR-322 key.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ApplicationNode<'a> {
    /// The node's v2 `node_tag`, spelled as on the wire.
    pub(crate) node_tag: &'a str,
    /// The node's v2 `semantic_form`, spelled as on the wire.
    pub(crate) semantic_form: &'a str,
    /// The node's semantic type key.
    pub(crate) semantic_type: NodeKey,
    /// The node's `declaration.qualified_name`, when it carries one.
    pub(crate) declaration: Option<&'a [Identifier]>,
    /// The node's recursion group, when it is in one.
    pub(crate) recursion: Option<RecursionGroup<'a>>,
    /// The node's body.
    pub(crate) body: &'a SemanticTerm,
}

/// The canonical preimage bytes and the key they hash to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ApplicationKey {
    /// The RFC 8785 JCS bytes of the `quire.application-node/v1` preimage.
    pub(crate) preimage: Vec<u8>,
    /// SHA-256 of [`Self::preimage`].
    pub(crate) key: NodeKey,
}

/// Why no FR-322 application key exists for a node.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum ApplicationKeyRefusal {
    /// The body contains no application term, so FR-322's application
    /// preimage does not key this node.
    #[error("the node body contains no application term")]
    NoApplication,
    /// A literal integer lies outside the range RFC 8785 renders exactly.
    #[error("literal integer {value} is outside the RFC 8785 exact-integer range")]
    UnsafeInteger {
        /// The offending literal value.
        value: i64,
    },
    /// The typed preimage failed to serialize.
    #[error("the preimage failed to serialize: {reason}")]
    Serialize {
        /// `serde_json`'s own message.
        reason: String,
    },
}

/// The FR-322 `application_node_preimage` key of `node`.
pub(crate) fn application_node_key(
    node: &ApplicationNode<'_>,
) -> Result<ApplicationKey, ApplicationKeyRefusal> {
    if !node.body.contains_application() {
        return Err(ApplicationKeyRefusal::NoApplication);
    }
    let group = node.recursion.map_or(&[][..], |group| group.members);
    let preimage = Preimage {
        version: APPLICATION_NODE_VERSION,
        node_tag: node.node_tag,
        semantic_form: node.semantic_form,
        semantic_type: NodeRef(node.semantic_type),
        declaration: node.declaration.map(|segments| DeclarationPreimage {
            qualified_name: segments.iter().map(Identifier::as_str).collect(),
        }),
        recursion: node.recursion.map(|group| RecursionPreimage {
            size: group.members.len(),
            ordinal: group.ordinal,
        }),
        body: preimage_term(node.body, group)?,
    };
    let serialize = |error: serde_json::Error| ApplicationKeyRefusal::Serialize {
        reason: error.to_string(),
    };
    let canonical = serde_json::to_value(&preimage).map_err(serialize)?;
    let bytes = serde_json::to_vec(&canonical).map_err(serialize)?;
    let key = NodeKey::from_digest(Sha256::digest(&bytes).into());
    Ok(ApplicationKey {
        preimage: bytes,
        key,
    })
}

#[derive(Serialize)]
struct Preimage<'a> {
    version: &'static str,
    node_tag: &'a str,
    semantic_form: &'a str,
    semantic_type: NodeRef,
    declaration: Option<DeclarationPreimage<'a>>,
    recursion: Option<RecursionPreimage>,
    body: PreimageTerm<'a>,
}

#[derive(Serialize)]
struct DeclarationPreimage<'a> {
    qualified_name: Vec<&'a str>,
}

#[derive(Serialize)]
struct RecursionPreimage {
    size: usize,
    ordinal: usize,
}

/// A body term as it enters the preimage: [`SemanticTerm`] with references
/// to recursion-group members replaced by their group ordinal.
#[derive(Serialize)]
#[serde(tag = "term", rename_all = "snake_case")]
enum PreimageTerm<'a> {
    Literal {
        #[serde(rename = "type")]
        ty: NodeRef,
        value_kind: LiteralKind,
        value: &'a Option<LiteralValue>,
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

/// `term` in preimage form over the recursion group `group` (graph order).
fn preimage_term<'a>(
    term: &'a SemanticTerm,
    group: &[NodeKey],
) -> Result<PreimageTerm<'a>, ApplicationKeyRefusal> {
    let terms = |terms: &'a [SemanticTerm]| {
        terms
            .iter()
            .map(|term| preimage_term(term, group))
            .collect::<Result<Vec<_>, _>>()
    };
    Ok(match term {
        SemanticTerm::Literal {
            ty,
            value_kind,
            value,
        } => {
            if let Some(LiteralValue::Integer(integer)) = value {
                if !(-JCS_SAFE_INTEGER..=JCS_SAFE_INTEGER).contains(integer) {
                    return Err(ApplicationKeyRefusal::UnsafeInteger { value: *integer });
                }
            }
            PreimageTerm::Literal {
                ty: *ty,
                value_kind: *value_kind,
                value,
            }
        }
        SemanticTerm::Reference { target } => {
            match group.iter().position(|member| *member == target.0) {
                Some(ordinal) => PreimageTerm::GroupReference { ordinal },
                None => PreimageTerm::Reference { target: *target },
            }
        }
        SemanticTerm::Application {
            operator,
            operation,
            result_type,
            arguments,
        } => PreimageTerm::Application {
            operator: *operator,
            operation,
            result_type: *result_type,
            arguments: terms(arguments)?,
        },
        SemanticTerm::Aggregate { members } => PreimageTerm::Aggregate {
            members: terms(members)?,
        },
        SemanticTerm::Binding { name, value } => PreimageTerm::Binding {
            name,
            value: Box::new(preimage_term(value, group)?),
        },
    })
}

#[cfg(test)]
mod tests;
