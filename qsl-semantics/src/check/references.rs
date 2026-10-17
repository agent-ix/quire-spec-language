// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-017 AR-4 "Referenced elements": the model elements a requirement
//! record's checked claim depends on, recorded where the record is keyed so
//! the layer-4 export reads them instead of re-walking a body.
//!
//! An item references the static object type of every model member it reads
//! (even when a supertype declares the member), the population of each of
//! its extent domains, and, for a frame, precondition or postcondition, the
//! operation it is about. This module holds those references in the checked
//! form `check` has at keying time (effective identities and population
//! domains); [`CheckedGraph::item_elements`](super::CheckedGraph::item_elements)
//! resolves them to declaration keys.

use std::collections::BTreeSet;

use quire_exact::{EffectiveId, ValueType};

use super::ir::{CheckedNode, NodeKind};
use super::state_clause::PopulationDomain;

/// What one requirement record references (ADR-017 AR-4).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModelReferences {
    pub(crate) objects: BTreeSet<EffectiveId>,
    pub(crate) populations: BTreeSet<PopulationDomain>,
    /// The declaring object type and name of the operation a frame,
    /// precondition or postcondition record is about.
    pub(crate) operation: Option<(EffectiveId, String)>,
}

impl ModelReferences {
    /// The references of a record that reads `objects`, ranges over
    /// `populations` and is about `operation`.
    pub(crate) fn new(
        objects: BTreeSet<EffectiveId>,
        populations: impl IntoIterator<Item = PopulationDomain>,
        operation: Option<(EffectiveId, String)>,
    ) -> Self {
        Self {
            objects,
            populations: populations.into_iter().collect(),
            operation,
        }
    }
}

/// The static object type of the reference `node` reads a model member
/// through, or `None` when `node` reads no model member.
#[deny(clippy::wildcard_enum_match_arm)]
pub(crate) fn read_object(node: CheckedNode<'_>) -> Option<EffectiveId> {
    let through = match node.kind() {
        NodeKind::Attribute { reference, .. } | NodeKind::Navigate { reference, .. } => *reference,
        NodeKind::Reaches { source, .. } | NodeKind::ReachesEnd { source, .. } => *source,
        NodeKind::Arithmetic(..)
        | NodeKind::IntegerDivision { .. }
        | NodeKind::Negate(_)
        | NodeKind::Divide { .. }
        | NodeKind::Rational { .. }
        | NodeKind::RationalNegate(..)
        | NodeKind::Decimal { .. }
        | NodeKind::DecimalNegate(..)
        | NodeKind::Ieee(..)
        | NodeKind::Quantity(..)
        | NodeKind::ConvertDecimal(..)
        | NodeKind::Order(..)
        | NodeKind::IeeeToRational(..)
        | NodeKind::ConvertScalar(..)
        | NodeKind::Equality(..)
        | NodeKind::Literal(_)
        | NodeKind::Coerce(..)
        | NodeKind::Local(_)
        | NodeKind::Let { .. }
        | NodeKind::If { .. }
        | NodeKind::Connective(..)
        | NodeKind::Not(_)
        | NodeKind::Field { .. }
        | NodeKind::Present(_)
        | NodeKind::Value(_)
        | NodeKind::Call { .. }
        | NodeKind::ImportedCall { .. }
        | NodeKind::Tuple { .. }
        | NodeKind::UnionValue { .. }
        | NodeKind::Record { .. }
        | NodeKind::Collection { .. }
        | NodeKind::ConvertCollection { .. }
        | NodeKind::Query { .. }
        | NodeKind::Flatten(_)
        | NodeKind::Fold { .. }
        | NodeKind::Size(_)
        | NodeKind::Contains(..)
        | NodeKind::AllInstances { .. }
        | NodeKind::Lookup { .. }
        | NodeKind::Dispatch { .. }
        | NodeKind::Pre(_)
        | NodeKind::ProtocolReference { .. }
        | NodeKind::Case { .. } => return None,
        #[cfg(seam_probe)]
        NodeKind::__SeamProbe => unreachable!("never constructed outside the probe build"),
    };
    match node.at(through).value_type() {
        ValueType::Reference(object) => Some(*object),
        ValueType::Boolean
        | ValueType::Integer
        | ValueType::Int(_)
        | ValueType::Rational(_)
        | ValueType::Decimal(_)
        | ValueType::Float(_)
        | ValueType::Quantity(_)
        | ValueType::Text(_)
        | ValueType::String
        | ValueType::Uuid
        | ValueType::Timestamp
        | ValueType::Enum(_)
        | ValueType::Option(_)
        | ValueType::Composite(_)
        | ValueType::Collection(_)
        | ValueType::Population(_) => None,
    }
}

/// The static object types every model member read under `root` goes
/// through.
pub(crate) fn body_objects(root: CheckedNode<'_>) -> BTreeSet<EffectiveId> {
    root.descendants()
        .into_iter()
        .filter_map(read_object)
        .collect()
}
