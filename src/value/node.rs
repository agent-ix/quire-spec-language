// SPDX-License-Identifier: AGPL-3.0-or-later
//! I04 nominal semantic-node identity shared by enum, dimension and unit nodes.
//!
//! A node key is the SHA-256 of the RFC 8785 JCS encoding of the node's
//! preimage in the `quire.checked-semantic-node/v1` domain. The strict reader
//! recomputes it from admitted content and refuses a mismatch with
//! `invalid_semantic_graph`.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use quire_exact::Integer;

// `NODE_KEY_DOMAIN` is `quire_exact`'s own canonical constant (QSL-131):
// byte-identical, and independent of `NodeKey` itself, which stays local
// (see this module's own [`NodeKey`] doc).
pub use quire_exact::NODE_KEY_DOMAIN;

/// An opaque `quire.checked-semantic-node/v1` node key.
///
/// `quire_exact::node::NodeKey` (QSL-131) is a stripped kernel copy of this
/// type: same 32-byte wrapper and `Display`/`Debug`, but its own module doc
/// states it retires hex parsing and hashing from the kernel by design
/// (`from_digest` is its one public constructor, wrapping an
/// already-computed digest; QSL's own hex parsing and SHA-256 hashing stay
/// here). This type's `from_hex`, `from_bytes` and `of` are exactly the
/// retired capability, still load-bearing here
/// (production callers in `value::model_query`/`value::member`, and every
/// `it` fixture that builds a `NodeKey` from a literal digest), so widening
/// `quire_exact::node::NodeKey` back out to cover them would undo that
/// design rather than cut a duplicate. Cutting this type over to
/// `quire_exact::node::NodeKey` needs its callers rerouted through a
/// `WireNodeId`-style lookup first; that is remaining work, not part of this
/// slice (Linear QSL-131).
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

    /// A key from its raw 32-byte digest, with no domain check: the caller
    /// already knows the bytes are a node key (for example, a same-domain
    /// identity bridged from another 32-byte digest type). Mirrors
    /// [`quire_exact::EffectiveId::from_digest`].
    pub(crate) fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub(crate) fn of(canonical: &[u8]) -> Self {
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
    /// An exact model-selected declaration.
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

/// Identity and node of a model owner.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(deny_unknown_fields)]
pub struct ModelSubject {
    /// The domain package identity.
    pub identity: String,
    /// The IR node identity.
    pub node: String,
}

impl NodeOwner {
    pub(crate) fn is_well_formed(&self) -> bool {
        match self {
            Self::Source(subject) | Self::Definition(subject) => {
                !subject.authority.is_empty() && !subject.identity.is_empty()
            }
            Self::Model(subject) => !subject.identity.is_empty() && !subject.node.is_empty(),
        }
    }

    pub(crate) fn canonical(&self) -> CanonicalOwner<'_> {
        let (kind, authority, identity, node) = match self {
            Self::Source(subject) => (
                "source",
                Some(subject.authority.as_str()),
                &subject.identity,
                None,
            ),
            Self::Definition(subject) => (
                "definition",
                Some(subject.authority.as_str()),
                &subject.identity,
                None,
            ),
            Self::Model(subject) => (
                "model",
                None,
                &subject.identity,
                Some(subject.node.as_str()),
            ),
        };
        CanonicalOwner {
            authority,
            identity,
            kind,
            node,
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

/// Why a nominal semantic node or node graph was refused.
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
    /// A dimension term has exponent zero.
    ZeroExponent,
    /// A dimension term names the same base dimension twice.
    DuplicateTerm,
    /// Dimension terms are not strictly ascending by node key.
    UnsortedTerms,
    /// A unit scale or offset is not a reduced rational.
    UnreducedRational,
    /// A unit scale is zero.
    ZeroScale,
    /// A targetless unit does not have scale one and offset zero.
    NonIdentityRoot,
    /// Two admitted nodes have the same key.
    DuplicateNode,
    /// A dimension term or unit names a dimension node that is not admitted.
    UnknownDimension,
    /// A dimension term names a derived (non-base) dimension.
    NonBaseDimensionTerm,
    /// A unit target is not an admitted unit.
    UnknownTarget,
    /// A unit target belongs to a different dimension node.
    CrossDimensionTarget,
    /// A dimension's unit graph has no targetless canonical root.
    MissingRoot,
    /// A dimension's unit graph has more than one targetless root.
    DuplicateRoot,
    /// Unit targets form a cycle.
    TargetCycle,
}

pub(crate) fn refuse(cause: SemanticGraphCause) -> InvalidSemanticGraph {
    InvalidSemanticGraph { cause }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeIdDocument {
    domain: String,
    digest: String,
}

impl NodeIdDocument {
    /// The referenced key when the domain and digest spelling are canonical.
    pub(crate) fn key(&self) -> Option<NodeKey> {
        NodeKey::from_hex(&self.digest).filter(|_| self.domain == NODE_KEY_DOMAIN)
    }
}

/// A schema `Rational`: canonical integer numerator and positive denominator
/// spellings.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RationalDocument {
    numerator: String,
    denominator: String,
}

impl RationalDocument {
    /// The exact spelled parts, or `None` for a non-canonical spelling.
    pub(crate) fn parts(&self) -> Option<(Integer, Integer)> {
        let numerator: Integer = self.numerator.parse().ok()?;
        let denominator: Integer = self.denominator.parse().ok()?;
        (!denominator.is_zero() && !denominator.is_negative()).then_some((numerator, denominator))
    }
}

// RFC 8785 JCS: every key below is ASCII, so declaring fields in ascending
// byte order is ascending UTF-16 order. `serde_json`'s compact string encoder
// escapes exactly `"`, `\`, `\b`, `\f`, `\n`, `\r`, `\t` and other C0 controls
// as lowercase `\u00xx`, which is the JCS string serialization.
#[derive(Serialize)]
pub(crate) struct CanonicalOwner<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    authority: Option<&'a str>,
    identity: &'a str,
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    node: Option<&'a str>,
}

#[derive(Serialize)]
pub(crate) struct CanonicalNodeId {
    digest: String,
    domain: &'static str,
}

impl From<NodeKey> for CanonicalNodeId {
    fn from(key: NodeKey) -> Self {
        Self {
            digest: key.to_string(),
            domain: NODE_KEY_DOMAIN,
        }
    }
}

#[derive(Serialize)]
pub(crate) struct CanonicalRational {
    denominator: String,
    numerator: String,
}

impl CanonicalRational {
    pub(crate) fn new(numerator: &Integer, denominator: &Integer) -> Self {
        Self {
            denominator: denominator.to_string(),
            numerator: numerator.to_string(),
        }
    }
}

pub(crate) fn is_identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// A `node-identity-preimage.schema.json` `$defs.Identifier`
/// (`^[A-Za-z_][A-Za-z0-9_]*$`): the identifier-shaped single name segment
/// every checked-member name/operator field and `check`'s own name-segment
/// use carry, so an invalid identifier is refused at construction rather
/// than reaching a schema-invalid wire shape (ADR-013 O-06). Lives here, in
/// this K-designated `node` module, rather than in `value::member` (not
/// K-designated) or as a `check`-lane-private newtype, because FR-068-AC-6
/// bounds `check`'s imports from `value` to the nine K-designated siblings
/// (`node` among them) plus a small declared-interim allow-list -- `member`
/// is in neither. Reuses `is_identifier`, the same character-class check
/// every other identifier-shaped field in this crate already uses ("one
/// fact, one place"), rather than a second copy of the schema's character
/// class.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Identifier(String);

/// A string that is not `^[A-Za-z_][A-Za-z0-9_]*$`.
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("an identifier is `^[A-Za-z_][A-Za-z0-9_]*$`")]
pub struct InvalidIdentifier;

impl Identifier {
    /// `value` as an identifier, or [`InvalidIdentifier`] if it is not
    /// `^[A-Za-z_][A-Za-z0-9_]*$`.
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidIdentifier> {
        let value = value.into();
        if is_identifier(&value) {
            Ok(Self(value))
        } else {
            Err(InvalidIdentifier)
        }
    }

    /// The identifier's own text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A schema `QualifiedName`: one or more identifiers.
pub(crate) fn is_qualified_name(segments: &[String]) -> bool {
    !segments.is_empty() && segments.iter().all(|segment| is_identifier(segment))
}

/// The node key of a canonical (JCS field-ordered) preimage.
pub(crate) fn node_key_of(value: &impl Serialize) -> Result<NodeKey, InvalidSemanticGraph> {
    serde_json::to_vec(value)
        .map(|bytes| NodeKey::of(&bytes))
        .map_err(|_| refuse(SemanticGraphCause::NonCanonicalPreimage))
}

/// Refuse terms that are zero, repeated or not strictly ascending, in that
/// order.
pub(crate) fn check_terms(terms: &[(NodeKey, Integer)]) -> Result<(), SemanticGraphCause> {
    if terms.iter().any(|(_, exponent)| exponent.is_zero()) {
        return Err(SemanticGraphCause::ZeroExponent);
    }
    let distinct: BTreeSet<_> = terms.iter().map(|(key, _)| key).collect();
    if distinct.len() != terms.len() {
        return Err(SemanticGraphCause::DuplicateTerm);
    }
    if !terms.is_sorted_by(|(left, _), (right, _)| left < right) {
        return Err(SemanticGraphCause::UnsortedTerms);
    }
    Ok(())
}

#[cfg(test)]
mod identifier_tests {
    use super::*;

    /// ADR-013 O-06/#260 review item 3: a name that is not
    /// `^[A-Za-z_][A-Za-z0-9_]*$` is refused at `Identifier::new`, before a
    /// `Member` carrying it can ever exist to be serialized as a
    /// schema-invalid `OperationMember`.
    #[test]
    fn a_non_identifier_name_is_refused() {
        for invalid in ["not an id!", "", "1starts_with_digit", "has-a-dash"] {
            assert_eq!(
                Identifier::new(invalid),
                Err(InvalidIdentifier),
                "{invalid:?} must be refused"
            );
        }
    }

    /// The positive complement of the refusal above: every name/operator
    /// `value::member`'s own schema-shape tests use is itself accepted.
    #[test]
    fn ordinary_identifiers_are_accepted() {
        for valid in [
            "quantity",
            "source",
            "totalPrice",
            "add",
            "_leading_underscore",
        ] {
            assert!(Identifier::new(valid).is_ok(), "{valid:?} must be accepted");
        }
    }
}
