// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-357, FR-358, FR-479: the obligation identity of a parity claim, the
//! structural triple of the claim node, its owning model and the occurrence
//! ordinal. There is no preimage digest: the operands and their domains are
//! carried by the claim, not folded into its identity.
//!
//! An operator-parity claim and a composite equality claim share the input
//! shape: the application node and its occurrence key, the obligation kind
//! and one argument per operand position, each an operand identity and a
//! domain (a range for a scalar, harness bounds for a composite).

use qsl_foundation::bound::ProofBound;
use quire_exact::{IntegerInterval, NodeKey, Origin};

use crate::identity::ObligationIdentity;
use crate::scalar::OperandIdentity;

/// Why an identity could not be minted. The claim refuses; no identity is
/// ever produced from an error.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum IdentityEncodeError {
    /// An occurrence ordinal beyond the identity's `u32` ordinal.
    #[error("the integer {magnitude} is beyond the identity's ordinal range")]
    IntegerBeyondEncoder {
        /// The refused integer.
        magnitude: i128,
    },
    /// Two harness bounds of one operand name the same domain key.
    #[error("two harness bounds name the domain key {key}")]
    DuplicateKey {
        /// The repeated key, in its display form.
        key: String,
    },
    /// A value-parity argument's `position` is not its index in the declared
    /// parameter list (FR-484): the arguments are never sorted, so the list
    /// and the positions must already agree.
    #[error("the value-parity argument at index {index} claims position {position}")]
    PositionOrder {
        /// The argument's index in the list.
        index: usize,
        /// The position the argument claims.
        position: usize,
    },
}

fn ordinal_of(origin: &Origin) -> Result<u32, IdentityEncodeError> {
    u32::try_from(origin.ordinal()).map_err(|_| IdentityEncodeError::IntegerBeyondEncoder {
        magnitude: i128::from(origin.ordinal()),
    })
}

/// A composite operand's harness bounds: one entry per domain key, ascending
/// by `DomainKey`'s structural order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundEntries(Vec<ProofBound>);

impl BoundEntries {
    /// The bounds in ascending key order, refusing a repeated key.
    pub fn new(mut bounds: Vec<ProofBound>) -> Result<Self, IdentityEncodeError> {
        bounds.sort_by(|left, right| left.domain().cmp(right.domain()));
        if let Some(pair) = bounds
            .windows(2)
            .find(|pair| pair[0].domain() == pair[1].domain())
        {
            return Err(IdentityEncodeError::DuplicateKey {
                key: pair[0].domain().to_string(),
            });
        }
        Ok(Self(bounds))
    }

    /// The bounds, in ascending key order.
    pub fn entries(&self) -> &[ProofBound] {
        &self.0
    }
}

/// What an operand ranges over (ADR-013 O-09's `domain`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Domain {
    /// A scalar operand's inclusive integer range; an inline literal's is
    /// the singleton `(value, value)`.
    Range(IntegerInterval),
    /// A composite operand's harness bounds.
    Bounds(BoundEntries),
}

/// One argument of the preimage: an operand's identity and domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParityArgument {
    /// How the operand is named.
    pub identity: OperandIdentity,
    /// What the operand ranges over.
    pub domain: Domain,
}

/// ADR-013 O-09's parity preimage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParityPreimage {
    /// The application node.
    pub node: NodeKey,
    /// The application node's occurrence key.
    pub occurrence: Origin,
    /// The CG `ObligationKind` wire string.
    pub obligation_kind: String,
    /// One argument per operand position, in operand order.
    pub arguments: Vec<ParityArgument>,
}

/// The identity of a parity claim: its application node, no owning model and
/// the occurrence's ordinal. A refusal is an error, never an identity.
pub fn parity_obligation(
    preimage: &ParityPreimage,
) -> Result<ObligationIdentity, IdentityEncodeError> {
    Ok(ObligationIdentity::new(
        preimage.node.clone(),
        None,
        ordinal_of(&preimage.occurrence)?,
    ))
}

/// FR-484: one declared parameter of a function-value parity claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueParityArgument {
    /// The parameter's `value`/`parameter` node id, the id
    /// `FunctionSite.parameters` pairs with the parameter.
    pub parameter: NodeKey,
    /// The parameter's zero-based position in the declared parameter list.
    pub position: usize,
    /// The node id of the parameter node's `semantic_type`, its declared
    /// type.
    pub semantic_type: NodeKey,
}

/// FR-484's preimage of a function-value parity claim. The occurrence key
/// (the function's one `declaration` occurrence) and the `subject_kind` are
/// fixed by the form and are not fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueParityPreimage {
    /// The function's checked `function` node id.
    pub node: NodeKey,
    /// One entry per declared parameter, in declared parameter order.
    pub arguments: Vec<ValueParityArgument>,
}

/// FR-484: the identity of a function-value parity claim: the function node,
/// no owning model and ordinal 0 (the function's one `declaration`
/// occurrence). An argument whose `position` is not its index refuses
/// [`IdentityEncodeError::PositionOrder`].
pub fn value_parity_obligation(
    preimage: &ValueParityPreimage,
) -> Result<ObligationIdentity, IdentityEncodeError> {
    if let Some((index, argument)) = preimage
        .arguments
        .iter()
        .enumerate()
        .find(|(index, argument)| argument.position != *index)
    {
        return Err(IdentityEncodeError::PositionOrder {
            index,
            position: argument.position,
        });
    }
    Ok(ObligationIdentity::new(preimage.node.clone(), None, 0))
}

/// FR-290: the obligation identity of a process-provider item: its
/// occurrence's node, no owning model and the occurrence's ordinal. A
/// process item has no CG obligation (ADR-029 PV-4 ruling 3), so QSL mints
/// it. The harness bounds are carried by the item, not by its identity.
pub(crate) fn process_provider_obligation(
    node: &NodeKey,
    occurrence: &Origin,
    _bounds: &BoundEntries,
) -> Result<ObligationIdentity, IdentityEncodeError> {
    Ok(ObligationIdentity::new(
        node.clone(),
        None,
        ordinal_of(occurrence)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use qsl_foundation::bound::{DomainKey, DomainKind, FiniteBound};

    fn node() -> NodeKey {
        crate::fixture_node(0x11)
    }

    fn key(path: Vec<u32>) -> DomainKey {
        DomainKey::Node { node: node(), path }
    }

    /// Harness bounds order by `DomainKey`'s structural order, and a repeated
    /// key refuses.
    #[trace("TC-904", "FR-357-AC-19")]
    #[test]
    fn bound_entries_order_structurally_and_refuse_a_repeat() {
        let bound = |path: Vec<u32>| {
            ProofBound::new(
                key(path),
                Some(DomainKind::Collection),
                FiniteBound::cardinality(1),
            )
            .unwrap()
        };
        let entries =
            BoundEntries::new(vec![bound(vec![2]), bound(vec![10]), bound(vec![0])]).unwrap();
        let order: Vec<_> = entries
            .entries()
            .iter()
            .map(|entry| entry.domain().clone())
            .collect();
        assert_eq!(order, [key(vec![0]), key(vec![2]), key(vec![10])]);
        assert!(matches!(
            BoundEntries::new(vec![bound(vec![1]), bound(vec![1])]),
            Err(IdentityEncodeError::DuplicateKey { .. })
        ));
    }

    /// A parity claim's identity is its node, no model and its ordinal.
    #[trace("TC-904", "FR-357-AC-19")]
    #[test]
    fn a_parity_identity_is_the_structural_triple() {
        let preimage = ParityPreimage {
            node: node(),
            occurrence: Origin::new(quire_exact::Role::new("expression"), 3),
            obligation_kind: "operator_parity".to_owned(),
            arguments: Vec::new(),
        };
        assert_eq!(
            parity_obligation(&preimage).unwrap(),
            ObligationIdentity::new(node(), None, 3)
        );
    }
}
