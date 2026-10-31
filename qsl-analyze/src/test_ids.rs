// SPDX-License-Identifier: AGPL-3.0-or-later
//! Structural fixture identities for `qsl-analyze`'s tests.

#![allow(dead_code)]

use quire_exact::{
    EffectiveId, NodeKey, OriginalDeclarationKey, PathSegment, UniverseId, VariantId,
};

/// A structural fixture node identity named by `seed`.
pub(crate) fn node_key(seed: impl AsRef<[u8]>) -> NodeKey {
    NodeKey::new(
        "fixture",
        vec![PathSegment::Text(format!("{:?}", seed.as_ref()))],
        0,
    )
    .expect("a fixture node identity is well formed")
}

/// A structural fixture effective declaration identity named by `seed`.
pub(crate) fn effective_id(seed: impl AsRef<[u8]>) -> EffectiveId {
    EffectiveId::new(
        None,
        OriginalDeclarationKey::new("fixture", format!("{:?}", seed.as_ref())),
    )
}

/// A structural fixture universe identity named by `seed`.
pub(crate) fn universe_id(seed: impl AsRef<[u8]>) -> UniverseId {
    UniverseId::new(node_key(seed))
}

/// A structural fixture variant identity of `seed`'s node.
pub(crate) fn variant_id(seed: impl AsRef<[u8]>) -> VariantId {
    VariantId::new(node_key(seed), "case").expect("a fixture variant case is non-empty")
}
