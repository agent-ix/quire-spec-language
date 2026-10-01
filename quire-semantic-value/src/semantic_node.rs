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

use quire_exact::Integer;

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
