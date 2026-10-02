// SPDX-License-Identifier: AGPL-3.0-or-later
//! The I04 nominal semantic-node refusal vocabulary (ADR-011 §6.1 layer SV):
//! the strict reader's `invalid_semantic_graph` refusal, its typed causes,
//! and the structural term check every dimension and compound unit applies.
//!
//! The preimage side (owner projection, RFC 8785 encoding and the SHA-256
//! node-key digest) stays in `qsl-semantics`' `value::semantic_node`; this
//! module holds only what a backend that is handed already-admitted node
//! keys raises as well.

use alloc::collections::BTreeSet;
use alloc::string::{String, ToString};
use core::fmt;

use quire_canonical::{FixedShape, Limits};
use serde::Serialize;

use quire_exact::{Integer, NodeKey, NODE_KEY_DOMAIN};

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

/// Refuse terms that are zero, repeated or not strictly ascending, in that
/// order.
pub fn check_terms<K: Ord>(terms: &[(K, Integer)]) -> Result<(), SemanticGraphCause> {
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

/// The limits every QSL identity preimage encodes under.
///
/// ADR-013 §2 (ADR-013:113, "One RFC 8785 JCS implementation produces every
/// RFC 8785 encoding"): every QSL normalized identity -- checked node keys,
/// nominal and unit preimages, `EffectiveId`, `UniverseId`, `PopulationId`,
/// `package_id` and the domain-package `sha256-jcs` digest -- is encoded by
/// the `quire-canonical` crate, called directly at each identity site. This
/// constant is no encoder: it fixes only the limits those calls share, so the
/// bound is stated once. It lives here, in the lowest identity-preimage
/// module, so every QSL layer from SV up may name it (FR-068-AC-6), and this
/// crate's compound-unit id encodes under it.
///
/// The encoder bounds bytes only; depth is not a limit (ADR-030 D-4.4). The
/// byte ceiling is `u64::MAX`, i.e. none of its own: every preimage is
/// built from values an earlier stage already bounded (intake's
/// `MAX_INPUT_BYTES`, the check stage's limits, a package reader's
/// `artifact_bytes`), and a caller with a tighter byte budget of its own
/// passes its own [`Limits`] instead (the v2 reader does).
pub const IDENTITY_LIMITS: Limits = Limits::new(u64::MAX);

/// The `node-identity-preimage.schema.json` node-id member every node-key
/// and compound-unit preimage embeds: a node key's digest as 64 lowercase
/// hexadecimal digits under the `quire.checked-semantic-node/v1` domain. The
/// one definition of that JCS shape, so node keys and compound-unit ids
/// encode a node id the same way.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, FixedShape)]
pub struct CanonicalNodeId {
    // RFC 8785 JCS: `quire-canonical` orders members itself, so field
    // declaration order carries no meaning.
    digest: String,
    domain: &'static str,
}

impl From<[u8; 32]> for CanonicalNodeId {
    /// The node id of these 32 digest bytes, as read or as retained.
    fn from(digest: [u8; 32]) -> Self {
        Self {
            digest: DigestHex(digest).to_string(),
            domain: NODE_KEY_DOMAIN,
        }
    }
}

impl From<NodeKey> for CanonicalNodeId {
    fn from(key: NodeKey) -> Self {
        Self::from(*key.as_bytes())
    }
}

/// 32 digest bytes as 64 lowercase hexadecimal digits, the spelling
/// `NodeKey`'s `Display` uses.
struct DigestHex([u8; 32]);

impl fmt::Display for DigestHex {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0
            .iter()
            .try_for_each(|byte| write!(formatter, "{byte:02x}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn term(key: u8, exponent: i64) -> (u8, Integer) {
        (key, Integer::from(exponent))
    }

    #[test]
    fn check_terms_refuses_zero_then_repeated_then_unsorted() {
        assert_eq!(check_terms::<u8>(&[]), Ok(()));
        assert_eq!(check_terms(&[term(1, 2), term(2, -1)]), Ok(()));
        assert_eq!(
            check_terms(&[term(2, 1), term(1, 0), term(1, 1)]),
            Err(SemanticGraphCause::ZeroExponent)
        );
        assert_eq!(
            check_terms(&[term(2, 1), term(1, 1), term(1, 1)]),
            Err(SemanticGraphCause::DuplicateTerm)
        );
        assert_eq!(
            check_terms(&[term(2, 1), term(1, 1)]),
            Err(SemanticGraphCause::UnsortedTerms)
        );
    }
}
