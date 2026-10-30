// SPDX-License-Identifier: AGPL-3.0-or-later
//! Structural identity fixtures for the spine tests: distinct, well-formed
//! ids for tests that need an id value and no particular declaration.

use quire_exact::{EffectiveId, NodeKey, OriginalDeclarationKey, PathSegment, PopulationId};
use quire_exact::{UniverseId, VariantId};

/// A distinct fixture node under the package `fixture`.
pub(crate) fn node(seed: u8) -> NodeKey {
    NodeKey::new("fixture", vec![PathSegment::Text(format!("n{seed}"))], 0)
        .expect("a fixture node key is well formed")
}

/// A distinct fixture universe.
pub(crate) fn universe(seed: u8) -> UniverseId {
    UniverseId::new(node(seed))
}

/// A distinct fixture effective type.
pub(crate) fn effective(seed: u8) -> EffectiveId {
    EffectiveId::new(
        None,
        OriginalDeclarationKey::new("fixture".to_owned(), format!("e{seed}")),
    )
}

/// A distinct fixture variant of a fixture declaration.
pub(crate) fn variant(seed: u8) -> VariantId {
    VariantId::new(node(0), format!("Case{seed}")).expect("a fixture case name is not empty")
}

/// A distinct fixture population.
pub(crate) fn population(seed: u8) -> PopulationId {
    PopulationId::new(node(seed))
}
