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
//! (ADR-013 O-04, R-10), never by wrapping the parsed digest; for units that
//! lookup is `quire_semantic_value::unit::UnitGraph`'s.

use std::collections::BTreeSet;

use quire_canonical::{Encode, FixedShape};
use serde::{Deserialize, Serialize};

use qsl_foundation::digest::WireNodeId;
use quire_exact::{Integer, NodeKey, NODE_KEY_DOMAIN};
use quire_semantic_value::semantic_node::{
    InvalidSemanticGraph, SemanticGraphCause, IDENTITY_LIMITS,
};

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
    /// admitted keys by lookup (ADR-013 O-04).
    pub(crate) fn wire_id(&self) -> Option<WireNodeId> {
        WireNodeId::from_hex(&self.digest).filter(|_| self.domain == NODE_KEY_DOMAIN)
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

// RFC 8785 JCS: `preimage_digest` encodes these through `quire-canonical`,
// which orders members itself, so field declaration order carries no meaning.
#[derive(Serialize, FixedShape)]
pub(crate) struct CanonicalOwner<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    authority: Option<&'a str>,
    identity: &'a str,
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    node: Option<&'a str>,
}

#[derive(Serialize, FixedShape)]
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

/// The SHA-256 digest of `value`'s RFC 8785 bytes, encoded and hashed by
/// `quire-canonical` (ADR-013 §2, ADR-013:113: the one RFC 8785
/// implementation). A value with no RFC 8785 encoding refuses as
/// non-canonical. QSL computes the digest here; `check` alone wraps it into a
/// `NodeKey`.
pub(crate) fn preimage_digest(value: &impl Encode) -> Result<[u8; 32], InvalidSemanticGraph> {
    quire_canonical::sha256(value, IDENTITY_LIMITS)
        .map(|digest| *digest.as_bytes())
        .map_err(|_| refuse(SemanticGraphCause::NonCanonicalPreimage))
}

/// `value`'s RFC 8785 bytes, the preimage [`preimage_digest`] hashes. A
/// value with no RFC 8785 encoding refuses as non-canonical.
pub(crate) fn preimage_bytes(value: &impl Encode) -> Result<Vec<u8>, InvalidSemanticGraph> {
    quire_canonical::to_vec(value, IDENTITY_LIMITS)
        .map_err(|_| refuse(SemanticGraphCause::NonCanonicalPreimage))
}

/// Whether `retained` is the node key `preimage` determines.
pub(crate) fn retains(
    retained: NodeKey,
    preimage: &impl NodeIdentityPreimage,
) -> Result<bool, InvalidSemanticGraph> {
    Ok(preimage.digest()? == *retained.as_bytes())
}
