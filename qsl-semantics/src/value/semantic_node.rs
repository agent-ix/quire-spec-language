// SPDX-License-Identifier: AGPL-3.0-or-later
//! I04 nominal semantic-node preimages shared by enum, dimension and unit
//! nodes (ADR-011 §6.2: layer 3 `semantic_value`).
//!
//! A node key is the SHA-256 of the RFC 8785 JCS encoding of the node's
//! preimage in the `quire.checked-semantic-node/v1` domain. This module owns
//! the preimage side: the owner projection, the canonical JCS field order and
//! the digest ([`NodeIdentityPreimage::digest`]). It never constructs a
//! `NodeKey`: only `check` wraps a preimage digest into the kernel `NodeKey`
//! (ADR-011 §6.1, ADR-013 O-04). The strict reader
//! here compares a retained key's bytes with the recomputed digest and
//! refuses a mismatch with `invalid_semantic_graph`.
//!
//! A node id read from a preimage document is a [`WireNodeId`]. It resolves
//! to a `NodeKey` only by lookup among the admitted nodes' retained keys
//! (ADR-013 O-04, R-10), never by wrapping the parsed digest.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use quire_canonical::Limits;

use qsl_foundation::digest::WireNodeId;
use quire_exact::{Integer, NodeKey, NODE_KEY_DOMAIN};

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
    /// The referenced wire node id when the domain and digest spelling are
    /// canonical. It stays a [`WireNodeId`]: the caller resolves it against
    /// admitted keys with [`resolve`] (ADR-013 O-04).
    pub(crate) fn wire_id(&self) -> Option<WireNodeId> {
        WireNodeId::from_hex(&self.digest).filter(|_| self.domain == NODE_KEY_DOMAIN)
    }
}

/// The index that resolves a wire node id to one of `keys` by lookup.
pub(crate) fn wire_index(keys: impl IntoIterator<Item = NodeKey>) -> BTreeMap<WireNodeId, NodeKey> {
    keys.into_iter()
        .map(|key| (WireNodeId::from_digest(*key.as_bytes()), key))
        .collect()
}

/// The admitted key `id` names, or `None` when no admitted node has it.
pub(crate) fn resolve(index: &BTreeMap<WireNodeId, NodeKey>, id: WireNodeId) -> Option<NodeKey> {
    index.get(&id).copied()
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

// RFC 8785 JCS: `preimage_digest` encodes these through `quire-canonical`,
// which orders members itself, so field declaration order carries no meaning.
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

impl From<WireNodeId> for CanonicalNodeId {
    fn from(id: WireNodeId) -> Self {
        Self {
            digest: id.to_string(),
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

/// A schema `QualifiedName`: one or more identifiers.
pub(crate) fn is_qualified_name(segments: &[String]) -> bool {
    !segments.is_empty()
        && segments
            .iter()
            .all(|segment| quire_exact::is_identifier(segment))
}

/// A `node-identity-preimage.schema.json` preimage whose SHA-256 digest is its
/// node key's bytes.
pub trait NodeIdentityPreimage {
    /// The SHA-256 digest of this preimage's RFC 8785 JCS encoding.
    fn digest(&self) -> Result<[u8; 32], InvalidSemanticGraph>;
}

/// The limits every QSL identity preimage encodes under.
///
/// ADR-013 §2 (ADR-013:113, "One RFC 8785 JCS implementation produces every
/// RFC 8785 encoding"): every QSL normalized identity -- checked node keys,
/// nominal and unit preimages, `EffectiveId`, `UniverseId`, `PopulationId`,
/// `package_id` and the domain-package `sha256-jcs` digest -- is encoded by
/// the `quire-canonical` crate, called directly at each identity site. This
/// constant is no encoder: it fixes only the limits those calls share, so the
/// bound is stated once. It lives here, in the lowest identity-preimage
/// module, so every layer-3 module may name it (FR-068-AC-6).
///
/// Depth is [`Limits::MAX_DEPTH`], the encoder's own ceiling. It is above the
/// deepest preimage QSL builds: a typed preimage nests a fixed schema depth,
/// a checked node body is already bounded by `check::MAX_CHECKING_DEPTH`,
/// and an intake document by the intake reader's `MAX_DEPTH` (200).
///
/// The byte ceiling is `u64::MAX`, i.e. none of its own: every preimage is
/// built from values an earlier stage already bounded (intake's
/// `MAX_INPUT_BYTES`, the check stage's limits, a package reader's
/// `artifact_bytes`), and a caller with a tighter byte budget of its own
/// passes its own [`Limits`] instead (the v2 reader does).
pub const IDENTITY_LIMITS: Limits = match Limits::new(u64::MAX, Limits::MAX_DEPTH) {
    Ok(limits) => limits,
    // `Limits::MAX_DEPTH` is by definition within `Limits::MAX_DEPTH`; this
    // arm is evaluated at compile time and is unreachable.
    Err(_) => panic!("Limits::MAX_DEPTH is within Limits::MAX_DEPTH"),
};

/// The SHA-256 digest of `value`'s RFC 8785 bytes, encoded and hashed by
/// `quire-canonical` (ADR-013 §2, ADR-013:113: the one RFC 8785
/// implementation). A value with no RFC 8785 encoding refuses as
/// non-canonical. QSL computes the digest here; `check` alone wraps it into a
/// `NodeKey`.
pub(crate) fn preimage_digest(value: &impl Serialize) -> Result<[u8; 32], InvalidSemanticGraph> {
    quire_canonical::sha256(value, IDENTITY_LIMITS)
        .map(|digest| *digest.as_bytes())
        .map_err(|_| refuse(SemanticGraphCause::NonCanonicalPreimage))
}

/// Whether `retained` is the node key `preimage` determines.
pub(crate) fn retains(
    retained: NodeKey,
    preimage: &impl NodeIdentityPreimage,
) -> Result<bool, InvalidSemanticGraph> {
    Ok(preimage.digest()? == *retained.as_bytes())
}

/// Refuse terms that are zero, repeated or not strictly ascending, in that
/// order.
pub(crate) fn check_terms<K: Ord>(terms: &[(K, Integer)]) -> Result<(), SemanticGraphCause> {
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
