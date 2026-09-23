// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "no production caller yet: QSL-156 slice A4b switches node identity to this key once the lock-evidence and type-node rulings land; until then only this module's own tests call it"
    )
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
//! checks rather than a `serde_json::Value` assembled by hand. Mode values are
//! the crate's own domain enums (`quire_exact::RoundingMode`,
//! `quire_exact::TextProfile`, `qsl_foundation::absence::AbsenceMode`),
//! spelled through their `as_str`. Type-node keys (`semantic_type`,
//! `result_type`, literal `type`) are inputs, used exactly as given: how
//! builtin and anonymous type nodes are keyed is a separate ruling (OQ-7),
//! wired in by A4b.
//!
//! Canonical bytes: the typed preimage is converted to a `serde_json::Value`
//! and serialized. `serde_json`'s map is ordered by key (this crate does not
//! enable `preserve_order`), every preimage key is a fixed ASCII schema name,
//! so byte order equals RFC 8785's UTF-16 code-unit order; strings use
//! `serde_json`'s escaping, which matches RFC 8785 for the ASCII escapes it
//! emits. The numbers in a preimage are `recursion`'s size and ordinal, an
//! `operation.member` position and literal integers; each is refused when
//! RFC 8785 cannot render it exactly (outside the IEEE-754 safe range)
//! rather than hashed. The pinned-bytes tests fix the exact encoding.
//!
//! The body walk is bounded: a body nested deeper than
//! [`MAX_CHECKING_DEPTH`] terms is refused, whatever path built it.
//!
//! Conformance against QSpec's own published `operation_vectors` is the
//! opt-in `conformance` test below (`make conformance` with `QSPEC_DIR`
//! pointing at a quire-specification checkout). QSpec is not public yet, so
//! nothing of it is copied here; the test reads the vectors at run time.

use std::collections::BTreeSet;

use serde::Serialize;
use sha2::{Digest, Sha256};

use qsl_foundation::absence::AbsenceMode;
use quire_exact::{Identifier, RoundingMode, TextProfile};

use super::definition::DefinitionReference;
use super::member::Member;
use super::node::{NodeKey, NODE_KEY_DOMAIN};
use super::MAX_CHECKING_DEPTH;

/// FR-322's application-node preimage version.
pub(crate) const APPLICATION_NODE_VERSION: &str = "quire.application-node/v1";

/// The largest integer magnitude RFC 8785 renders exactly (2^53 - 1).
const JCS_SAFE_INTEGER: i128 = (1 << 53) - 1;

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

/// The preimage schema's closed `ApplicationNode.node_tag` vocabulary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(serde::Deserialize))]
#[serde(rename_all = "snake_case")]
pub(crate) enum NodeTag {
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
        /// The bound name (schema `Nonempty`; an empty name is refused).
        name: String,
        /// The bound term.
        value: Box<SemanticTerm>,
    },
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

/// The schema's non-null `OperationMode` (`{kind, value}`), carrying the
/// crate's own mode enums.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub(crate) enum OperationMode {
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

/// A node's place in its recursion group: the group's distinct member keys
/// in graph order and this node's ordinal among them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RecursionGroup<'a> {
    members: &'a [NodeKey],
    ordinal: usize,
}

/// Why a [`RecursionGroup`] cannot be formed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum RecursionGroupRefusal {
    /// `ordinal` is not a position in the member list.
    #[error("ordinal {ordinal} is outside a group of {size}")]
    OrdinalOutOfRange {
        /// The requested ordinal.
        ordinal: usize,
        /// The group size.
        size: usize,
    },
    /// A key appears twice, so a reference to it has no single ordinal.
    #[error("recursion group member {member} appears more than once")]
    DuplicateMember {
        /// The repeated key.
        member: NodeKey,
    },
}

impl<'a> RecursionGroup<'a> {
    /// The group `members` (graph order) with this node at `ordinal`.
    pub(crate) fn new(
        members: &'a [NodeKey],
        ordinal: usize,
    ) -> Result<Self, RecursionGroupRefusal> {
        if ordinal >= members.len() {
            return Err(RecursionGroupRefusal::OrdinalOutOfRange {
                ordinal,
                size: members.len(),
            });
        }
        let mut seen = BTreeSet::new();
        if let Some(member) = members.iter().find(|member| !seen.insert(**member)) {
            return Err(RecursionGroupRefusal::DuplicateMember { member: *member });
        }
        Ok(Self { members, ordinal })
    }
}

/// The inputs of one application-bearing node's FR-322 key.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ApplicationNode<'a> {
    /// The node's v2 `node_tag`.
    pub(crate) node_tag: NodeTag,
    /// The node's v2 `semantic_form`, spelled as on the wire (schema
    /// `Nonempty`; an empty form is refused).
    pub(crate) semantic_form: &'a str,
    /// The node's semantic type key.
    pub(crate) semantic_type: NodeKey,
    /// The node's `declaration.qualified_name`, when it carries one (schema
    /// `minItems: 1`; an empty name is refused).
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

/// Where a preimage number sits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IntegerSite {
    /// A literal term's integer value.
    Literal,
    /// An `operation.member` position.
    MemberPosition,
    /// The `recursion` group size.
    RecursionSize,
}

/// Why no FR-322 application key exists for a node.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum ApplicationKeyRefusal {
    /// The body contains no application term, so FR-322's application
    /// preimage does not key this node.
    #[error("the node body contains no application term")]
    NoApplication,
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
        value: i128,
    },
    /// The body nests deeper than [`MAX_CHECKING_DEPTH`] terms.
    #[error("the body nests deeper than {limit} terms")]
    TooDeep {
        /// The depth limit.
        limit: u64,
    },
    /// The typed preimage failed to convert to a JSON value. Unreachable for
    /// these types (every map key is a fixed string and no `Serialize` impl
    /// here errors), but `serde_json::to_value` is fallible and this module
    /// does not panic on an input path.
    #[error("the preimage failed to serialize: {reason}")]
    Serialize {
        /// `serde_json`'s own message.
        reason: String,
    },
}

/// Refuse `value` at `site` when RFC 8785 cannot render it exactly.
fn exact_integer(site: IntegerSite, value: i128) -> Result<(), ApplicationKeyRefusal> {
    if (-JCS_SAFE_INTEGER..=JCS_SAFE_INTEGER).contains(&value) {
        Ok(())
    } else {
        Err(ApplicationKeyRefusal::UnsafeInteger { site, value })
    }
}

/// The FR-322 `application_node_preimage` key of `node`.
pub(crate) fn application_node_key(
    node: &ApplicationNode<'_>,
) -> Result<ApplicationKey, ApplicationKeyRefusal> {
    if node.semantic_form.is_empty() {
        return Err(ApplicationKeyRefusal::EmptySemanticForm);
    }
    if node.declaration.is_some_and(<[Identifier]>::is_empty) {
        return Err(ApplicationKeyRefusal::EmptyQualifiedName);
    }
    if let Some(group) = node.recursion {
        let size = i128::try_from(group.members.len()).unwrap_or(i128::MAX);
        exact_integer(IntegerSite::RecursionSize, size)?;
    }
    let group = node.recursion.map_or(&[][..], |group| group.members);
    let walk = Walk { group };
    let (body, has_application) = walk.term(node.body, 1)?;
    if !has_application {
        return Err(ApplicationKeyRefusal::NoApplication);
    }
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
        body,
    };
    let canonical =
        serde_json::to_value(&preimage).map_err(|error| ApplicationKeyRefusal::Serialize {
            reason: error.to_string(),
        })?;
    let bytes = canonical.to_string().into_bytes();
    let key = NodeKey::from_digest(Sha256::digest(&bytes).into());
    Ok(ApplicationKey {
        preimage: bytes,
        key,
    })
}

#[derive(Serialize)]
struct Preimage<'a> {
    version: &'static str,
    node_tag: NodeTag,
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

/// One pass over a body: builds its preimage form, reports whether it
/// contains an application, and enforces the depth and number bounds.
struct Walk<'g> {
    /// The recursion group's member keys, in graph order.
    group: &'g [NodeKey],
}

impl Walk<'_> {
    /// `term` at nesting `depth` (the body root is depth 1) in preimage form,
    /// and whether it is or contains an application.
    fn term<'a>(
        &self,
        term: &'a SemanticTerm,
        depth: u64,
    ) -> Result<(PreimageTerm<'a>, bool), ApplicationKeyRefusal> {
        if depth > MAX_CHECKING_DEPTH {
            return Err(ApplicationKeyRefusal::TooDeep {
                limit: MAX_CHECKING_DEPTH,
            });
        }
        let terms = |terms: &'a [SemanticTerm]| {
            terms.iter().try_fold(
                (Vec::with_capacity(terms.len()), false),
                |(mut mapped, any), term| {
                    let (preimage, has) = self.term(term, depth + 1)?;
                    mapped.push(preimage);
                    Ok::<_, ApplicationKeyRefusal>((mapped, any || has))
                },
            )
        };
        Ok(match term {
            SemanticTerm::Literal {
                ty,
                value_kind,
                value,
            } => {
                if let Some(LiteralValue::Integer(integer)) = value {
                    exact_integer(IntegerSite::Literal, i128::from(*integer))?;
                }
                let preimage = PreimageTerm::Literal {
                    ty: *ty,
                    value_kind: *value_kind,
                    value,
                };
                (preimage, false)
            }
            SemanticTerm::Reference { target } => {
                let preimage = match self.group.iter().position(|member| *member == target.0) {
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
                    exact_integer(IntegerSite::MemberPosition, i128::from(*position))?;
                }
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
                    return Err(ApplicationKeyRefusal::EmptyBindingName);
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

#[cfg(test)]
mod tests;
