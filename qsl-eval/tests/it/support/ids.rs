// SPDX-License-Identifier: AGPL-3.0-or-later
//! Structural fixture identities for `qsl-eval`'s integration tests.
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

/// A node identity as its wire JSON.
pub(crate) fn node_json(key: &NodeKey) -> serde_json::Value {
    qsl_foundation::node_wire::node_key_to_json(key).expect("a fixture node key encodes")
}

/// An effective declaration identity as FR-201's recursive record.
pub(crate) fn effective_json(id: &EffectiveId) -> serde_json::Value {
    id.owner_chain()
        .iter()
        .fold(serde_json::Value::Null, |owner, original| {
            serde_json::json!({"original": {"node": original.node(), "package": original.package()},
                "owner_effective_type": owner})
        })
}
