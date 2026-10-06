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

use qsl_foundation::diagnostic::{LimitExceeded, LimitKind, LimitsField};
use qsl_foundation::digest::WireNodeId;
use quire_exact::{Integer, NodeKey, NODE_KEY_DOMAIN};
use quire_semantic_value::semantic_node::{
    IdentityRefusal, InvalidSemanticGraph, SemanticGraphCause, IDENTITY_LIMITS,
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
    ///
    /// # Errors
    ///
    /// As [`preimage_digest`].
    fn digest(&self) -> Result<[u8; 32], NominalRefusal>;
}

/// Why a nominal preimage was refused: an `invalid_semantic_graph` refusal,
/// or its identity encoding reached the `identity.input_bytes` limit.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum NominalRefusal {
    /// The preimage or its graph is invalid.
    #[error(transparent)]
    Graph(#[from] InvalidSemanticGraph),
    /// The preimage's canonical bytes would exceed `identity.input_bytes`
    /// (FR-259 Behavior 4): a stage limit, never a malformed value.
    #[error("stage_limit_exceeded: {0:?}")]
    Limit(LimitExceeded),
    /// A heap reservation for the preimage's canonical bytes failed
    /// (FR-259 Behavior 6): `resource_exhausted`/`allocation-failed`, no
    /// limit and no setting.
    #[error("resource_exhausted/allocation-failed: {requested} bytes")]
    Allocation {
        /// The size in bytes of the reservation that failed.
        requested: usize,
    },
}

/// The `identity.input_bytes` stage limit an identity encoding reached:
/// input bytes, its bound and the bytes it needed (FR-259 Behavior 4).
pub fn identity_limit(bound: u64, required: u64) -> LimitExceeded {
    LimitExceeded::new(LimitKind::InputBytes, bound, u128::from(required))
        .named(LimitsField::IdentityInputBytes)
}

/// A nominal preimage's encoding error: the byte error as the
/// `identity.input_bytes` limit, a failed reservation as allocation-failed,
/// and anything else as a non-canonical preimage.
fn nominal_refusal(error: quire_canonical::Error) -> NominalRefusal {
    match IdentityRefusal::from(error) {
        IdentityRefusal::InputBytes { bound, required } => {
            NominalRefusal::Limit(identity_limit(bound, required))
        }
        IdentityRefusal::Allocation { requested } => NominalRefusal::Allocation { requested },
        IdentityRefusal::NonCanonical => refuse(SemanticGraphCause::NonCanonicalPreimage).into(),
    }
}

/// The SHA-256 digest of `value`'s RFC 8785 bytes, encoded and hashed by
/// `quire-canonical` (ADR-013 §2, ADR-013:113: the one RFC 8785
/// implementation) under [`IDENTITY_LIMITS`]. QSL computes the digest here;
/// `check` alone wraps it into a `NodeKey`.
///
/// # Errors
///
/// [`NominalRefusal::Limit`] naming `identity.input_bytes` when the bytes
/// would exceed the limit, and a non-canonical preimage when `value` has no
/// RFC 8785 encoding.
pub(crate) fn preimage_digest(value: &impl Encode) -> Result<[u8; 32], NominalRefusal> {
    quire_canonical::sha256(value, IDENTITY_LIMITS)
        .map(|digest| *digest.as_bytes())
        .map_err(nominal_refusal)
}

/// `value`'s RFC 8785 bytes, the preimage [`preimage_digest`] hashes.
///
/// # Errors
///
/// As [`preimage_digest`].
pub(crate) fn preimage_bytes(value: &impl Encode) -> Result<Vec<u8>, NominalRefusal> {
    quire_canonical::to_vec(value, IDENTITY_LIMITS).map_err(nominal_refusal)
}

/// Whether `retained` is the node key `preimage` determines.
pub(crate) fn retains(
    retained: NodeKey,
    preimage: &impl NodeIdentityPreimage,
) -> Result<bool, NominalRefusal> {
    Ok(preimage.digest()? == *retained.as_bytes())
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_foundation::diagnostic::{CatalogCoded, Code};

    use super::*;

    /// FR-259 Behavior 4: a nominal preimage whose canonical bytes exceed
    /// `identity.input_bytes` stops with that stage limit (input bytes, the
    /// bound, the bytes needed and the setting), `stage_limit_exceeded`, which
    /// is incomplete work (exit 22), never a non-canonical preimage.
    #[trace("TC-728", "FR-259-AC-2")]
    #[test]
    fn a_nominal_preimage_over_the_identity_byte_limit_is_the_identity_limit() {
        let bound = IDENTITY_LIMITS.max_bytes();
        let over = "a".repeat(usize::try_from(bound).expect("16 MiB fits usize"));
        let Err(NominalRefusal::Limit(limit)) = preimage_digest(&over.as_str()) else {
            panic!("a preimage of more than {bound} bytes reaches the identity limit");
        };
        assert_eq!(limit.kind(), LimitKind::InputBytes);
        assert_eq!(limit.configured_bound(), bound);
        assert!(limit.actual() > u128::from(bound));
        assert_eq!(
            limit.limits_field().map(LimitsField::as_str),
            Some("identity.input_bytes")
        );
        assert_eq!(
            limit.catalog_code().code(),
            Code::StageLimitExceeded.as_str()
        );
        assert!(
            preimage_digest(&"a").is_ok(),
            "a short preimage has a digest"
        );
        assert_eq!(
            nominal_refusal(quire_canonical::Error::Allocation { requested: 4096 }),
            NominalRefusal::Allocation { requested: 4096 },
            "a failed reservation is allocation-failed, not non-canonical"
        );
    }
}
