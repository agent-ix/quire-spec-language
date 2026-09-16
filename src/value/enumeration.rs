// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-141 declaration-qualified enumerations and their I04 nominal node
//! identities.
//!
//! A declaration node key is the SHA-256 of the RFC 8785 JCS encoding of its
//! `quire.enum-declaration-node/v1` preimage; a member node key hashes
//! `quire.enum-member-node/v1`. The strict reader recomputes both from the
//! admitted content and refuses a retained key that no longer matches with
//! `invalid_semantic_graph`. Display text and source loci are not part of
//! either preimage.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::accounting::{Charge, ChargePoint, LimitKind, Meter};
use super::comparison::{ComparisonOperator, IllTyped, IllTypedCause};
use super::outcome::{Outcome, Stop};

/// Digest domain of every checked semantic node key.
pub const NODE_KEY_DOMAIN: &str = "quire.checked-semantic-node/v1";

const DECLARATION_VERSION: &str = "quire.enum-declaration-node/v1";
const MEMBER_VERSION: &str = "quire.enum-member-node/v1";

/// An opaque `quire.checked-semantic-node/v1` node key.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeKey([u8; 32]);

impl NodeKey {
    /// Parse 64 lowercase hexadecimal digits.
    pub fn from_hex(digest: &str) -> Option<Self> {
        let (pairs, []) = digest.as_bytes().as_chunks::<2>() else {
            return None;
        };
        if pairs.len() != 32 {
            return None;
        }
        let mut key = [0_u8; 32];
        for (slot, [high, low]) in key.iter_mut().zip(pairs) {
            *slot = (lower_hex(*high)? << 4) | lower_hex(*low)?;
        }
        Some(Self(key))
    }

    /// The raw digest.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    fn of(canonical: &[u8]) -> Self {
        Self(Sha256::digest(canonical).into())
    }
}

fn lower_hex(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        _ => None,
    }
}

impl fmt::Display for NodeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|byte| write!(f, "{byte:02x}"))
    }
}

impl fmt::Debug for NodeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeKey({self})")
    }
}

/// The stable subject projection of the exact admitted owner of a nominal
/// declaration.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NodeOwner {
    /// An exact admitted source.
    Source(OwnerSubject),
    /// An exact DefinitionRef.
    Definition(OwnerSubject),
    /// An exact ModelRef export.
    Model(ModelSubject),
}

/// Authority and identity of a source or definition owner.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(deny_unknown_fields)]
pub struct OwnerSubject {
    /// Nonempty authority.
    pub authority: String,
    /// Nonempty identity.
    pub identity: String,
}

/// Authority, identity and export of a model owner.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(deny_unknown_fields)]
pub struct ModelSubject {
    /// Nonempty authority.
    pub authority: String,
    /// Nonempty identity.
    pub identity: String,
    /// Nonempty export.
    pub export: String,
}

impl NodeOwner {
    fn is_well_formed(&self) -> bool {
        match self {
            Self::Source(subject) | Self::Definition(subject) => {
                !subject.authority.is_empty() && !subject.identity.is_empty()
            }
            Self::Model(subject) => {
                !subject.authority.is_empty()
                    && !subject.identity.is_empty()
                    && !subject.export.is_empty()
            }
        }
    }

    fn canonical(&self) -> CanonicalOwner<'_> {
        let (kind, authority, identity, export) = match self {
            Self::Source(subject) => ("source", &subject.authority, &subject.identity, None),
            Self::Definition(subject) => {
                ("definition", &subject.authority, &subject.identity, None)
            }
            Self::Model(subject) => (
                "model",
                &subject.authority,
                &subject.identity,
                Some(subject.export.as_str()),
            ),
        };
        CanonicalOwner {
            authority,
            export,
            identity,
            kind,
        }
    }
}

/// The owners the package's lock selection admits; a declaration owner must
/// join this set.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OwnerSelection(BTreeSet<NodeOwner>);

impl OwnerSelection {
    /// Select exactly `owners`.
    pub fn new(owners: impl IntoIterator<Item = NodeOwner>) -> Self {
        Self(owners.into_iter().collect())
    }

    /// Whether `owner` joins the selection.
    pub fn contains(&self, owner: &NodeOwner) -> bool {
        self.0.contains(owner)
    }
}

/// The strict reader's `refused { code: invalid_semantic_graph }`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("invalid_semantic_graph: {cause:?}")]
pub struct InvalidSemanticGraph {
    /// The typed reason.
    pub cause: SemanticGraphCause,
}

impl InvalidSemanticGraph {
    /// Stable refusal code.
    pub const CODE: &'static str = "invalid_semantic_graph";
}

/// Why an enum node was refused.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SemanticGraphCause {
    /// The preimage does not satisfy `node-identity-preimage.schema.json`.
    NonCanonicalPreimage,
    /// An unordered declaration's members are not sorted by case name.
    UnsortedUnorderedMembers,
    /// The owner does not join the lock selection.
    OwnerNotSelected,
    /// The retained key is not the digest of the admitted content.
    StaleKey,
    /// A member node references a different declaration node.
    ForeignDeclaration,
    /// A member node's case is not a member of its declaration.
    UndeclaredCase,
}

fn refuse(cause: SemanticGraphCause) -> InvalidSemanticGraph {
    InvalidSemanticGraph { cause }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeclarationDocument {
    version: String,
    owner: NodeOwner,
    qualified_declaration: Vec<String>,
    ordered: bool,
    members: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeIdDocument {
    domain: String,
    digest: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MemberDocument {
    version: String,
    declaration_node_id: NodeIdDocument,
    case: String,
}

// RFC 8785 JCS: every key below is ASCII, so declaring fields in ascending
// byte order is ascending UTF-16 order. `serde_json`'s compact string encoder
// escapes exactly `"`, `\`, `\b`, `\f`, `\n`, `\r`, `\t` and other C0 controls
// as lowercase `\u00xx`, which is the JCS string serialization.
#[derive(Serialize)]
struct CanonicalOwner<'a> {
    authority: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    export: Option<&'a str>,
    identity: &'a str,
    kind: &'static str,
}

#[derive(Serialize)]
struct CanonicalDeclaration<'a> {
    members: &'a [String],
    ordered: bool,
    owner: CanonicalOwner<'a>,
    qualified_declaration: &'a [String],
    version: &'static str,
}

#[derive(Serialize)]
struct CanonicalNodeId {
    digest: String,
    domain: &'static str,
}

#[derive(Serialize)]
struct CanonicalMember<'a> {
    case: &'a str,
    declaration_node_id: CanonicalNodeId,
    version: &'static str,
}

fn is_identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn canonical_json(value: &impl Serialize) -> Result<Vec<u8>, InvalidSemanticGraph> {
    serde_json::to_vec(value).map_err(|_| refuse(SemanticGraphCause::NonCanonicalPreimage))
}

/// A `quire.enum-declaration-node/v1` preimage that satisfies the preimage
/// schema.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumDeclarationPreimage {
    owner: NodeOwner,
    qualified_declaration: Vec<String>,
    ordered: bool,
    members: Vec<String>,
}

impl EnumDeclarationPreimage {
    /// Read a preimage object; any schema violation is non-canonical.
    pub fn from_json(value: serde_json::Value) -> Result<Self, InvalidSemanticGraph> {
        let document: DeclarationDocument = serde_json::from_value(value)
            .map_err(|_| refuse(SemanticGraphCause::NonCanonicalPreimage))?;
        let distinct: BTreeSet<_> = document.members.iter().collect();
        let well_formed = document.version == DECLARATION_VERSION
            && document.owner.is_well_formed()
            && !document.qualified_declaration.is_empty()
            && document
                .qualified_declaration
                .iter()
                .all(|name| is_identifier(name))
            && !document.members.is_empty()
            && distinct.len() == document.members.len()
            && document.members.iter().all(|case| is_identifier(case));
        if !well_formed {
            return Err(refuse(SemanticGraphCause::NonCanonicalPreimage));
        }
        Ok(Self {
            owner: document.owner,
            qualified_declaration: document.qualified_declaration,
            ordered: document.ordered,
            members: document.members,
        })
    }

    /// The owner projection.
    pub fn owner(&self) -> &NodeOwner {
        &self.owner
    }

    /// Qualified declaration name segments.
    pub fn qualified_declaration(&self) -> &[String] {
        &self.qualified_declaration
    }

    /// Whether the declaration selects ordered semantics.
    pub fn is_ordered(&self) -> bool {
        self.ordered
    }

    /// Declaration-ordered (or case-sorted, if unordered) member cases.
    pub fn members(&self) -> &[String] {
        &self.members
    }

    /// The node key this content determines.
    pub fn node_key(&self) -> Result<NodeKey, InvalidSemanticGraph> {
        canonical_json(&CanonicalDeclaration {
            members: &self.members,
            ordered: self.ordered,
            owner: self.owner.canonical(),
            qualified_declaration: &self.qualified_declaration,
            version: DECLARATION_VERSION,
        })
        .map(|bytes| NodeKey::of(&bytes))
    }
}

/// A `quire.enum-member-node/v1` preimage that satisfies the preimage schema.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumMemberPreimage {
    declaration: NodeKey,
    case: String,
}

impl EnumMemberPreimage {
    /// Read a preimage object; any schema violation is non-canonical.
    pub fn from_json(value: serde_json::Value) -> Result<Self, InvalidSemanticGraph> {
        let document: MemberDocument = serde_json::from_value(value)
            .map_err(|_| refuse(SemanticGraphCause::NonCanonicalPreimage))?;
        let declaration = NodeKey::from_hex(&document.declaration_node_id.digest)
            .filter(|_| {
                document.version == MEMBER_VERSION
                    && document.declaration_node_id.domain == NODE_KEY_DOMAIN
                    && is_identifier(&document.case)
            })
            .ok_or(refuse(SemanticGraphCause::NonCanonicalPreimage))?;
        Ok(Self {
            declaration,
            case: document.case,
        })
    }

    /// The referenced declaration node key.
    pub fn declaration(&self) -> NodeKey {
        self.declaration
    }

    /// The member case name.
    pub fn case(&self) -> &str {
        &self.case
    }

    /// The node key this content determines.
    pub fn node_key(&self) -> Result<NodeKey, InvalidSemanticGraph> {
        canonical_json(&CanonicalMember {
            case: &self.case,
            declaration_node_id: CanonicalNodeId {
                digest: self.declaration.to_string(),
                domain: NODE_KEY_DOMAIN,
            },
            version: MEMBER_VERSION,
        })
        .map(|bytes| NodeKey::of(&bytes))
    }
}

/// An admitted enum declaration node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumDeclaration {
    key: NodeKey,
    preimage: EnumDeclarationPreimage,
}

impl EnumDeclaration {
    /// Admit a declaration node whose graph retains `key`.
    ///
    /// Order: semantic well-formedness, owner join, then key recomputation.
    pub fn admit(
        preimage: EnumDeclarationPreimage,
        key: NodeKey,
        owners: &OwnerSelection,
    ) -> Result<Self, InvalidSemanticGraph> {
        if !preimage.ordered && !preimage.members.is_sorted() {
            return Err(refuse(SemanticGraphCause::UnsortedUnorderedMembers));
        }
        if !owners.contains(&preimage.owner) {
            return Err(refuse(SemanticGraphCause::OwnerNotSelected));
        }
        if preimage.node_key()? != key {
            return Err(refuse(SemanticGraphCause::StaleKey));
        }
        Ok(Self { key, preimage })
    }

    /// The declaration node key.
    pub fn key(&self) -> NodeKey {
        self.key
    }

    /// The admitted content.
    pub fn preimage(&self) -> &EnumDeclarationPreimage {
        &self.preimage
    }

    /// Admit a member node of this declaration whose graph retains `key`.
    pub fn admit_member(
        &self,
        preimage: &EnumMemberPreimage,
        key: NodeKey,
    ) -> Result<EnumValue, InvalidSemanticGraph> {
        if preimage.declaration != self.key {
            return Err(refuse(SemanticGraphCause::ForeignDeclaration));
        }
        let position = self
            .preimage
            .members
            .iter()
            .position(|case| *case == preimage.case)
            .ok_or(refuse(SemanticGraphCause::UndeclaredCase))?;
        if preimage.node_key()? != key {
            return Err(refuse(SemanticGraphCause::StaleKey));
        }
        Ok(EnumValue {
            declaration: self.key,
            member: key,
            ordered: self.preimage.ordered,
            position,
        })
    }
}

/// An enumeration value: (declaration identity, member identity).
#[derive(Clone, Copy, Debug)]
pub struct EnumValue {
    declaration: NodeKey,
    member: NodeKey,
    ordered: bool,
    position: usize,
}

impl EnumValue {
    /// Declaration node key.
    pub fn declaration(&self) -> NodeKey {
        self.declaration
    }

    /// Member node key.
    pub fn member(&self) -> NodeKey {
        self.member
    }
}

/// Compare two enum values. Members of different declarations, and ordering
/// requests over an unordered declaration, are ill-typed and consume nothing.
// SPEC-GAP(8): FR-141-AC-5 says an unordered ordering request "refuses" without
// naming the code; it is reported as the type-checking `ill_typed`.
pub fn compare_enum(
    operator: ComparisonOperator,
    left: &EnumValue,
    right: &EnumValue,
    meter: &mut Meter,
) -> Result<Outcome<bool>, IllTyped> {
    if left.declaration != right.declaration {
        return Err(IllTyped {
            cause: IllTypedCause::DistinctEnumDeclarations,
        });
    }
    if operator.is_ordering() && !left.ordered {
        return Err(IllTyped {
            cause: IllTypedCause::UnorderedEnumOrdering,
        });
    }
    Ok(Outcome::from_stop(compare(operator, left, right, meter)))
}

fn compare(
    operator: ComparisonOperator,
    left: &EnumValue,
    right: &EnumValue,
    meter: &mut Meter,
) -> Result<bool, Stop> {
    for read in 1..=2 {
        meter.charge(
            Charge::new(ChargePoint::EnumIdentityRead).size(LimitKind::ValueOccurrences, read),
        )?;
    }
    let ordering = if operator.is_ordering() {
        left.position.cmp(&right.position)
    } else {
        left.member.cmp(&right.member)
    };
    meter.charge(Charge::new(ChargePoint::EnumResultRetain).results(1))?;
    Ok(operator.holds(ordering))
}
