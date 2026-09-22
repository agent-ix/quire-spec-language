// SPDX-License-Identifier: AGPL-3.0-or-later
//! `PackageNodeKey` (ADR-013 T-3, FR-087, QSL-158 S-3a): the sole
//! cross-package node reference. An I2 reference into an imported package's
//! `ImportView` names a node by `{package: package_id, node: WireNodeId}`,
//! pinning the verified content and naming the node without constructing a
//! `NodeKey` from wire bytes (R-10, O-04): a `WireNodeId` becomes a
//! `NodeKey` only by lookup in an already-checked package, at E4 (the
//! dependency's own checked package, compiled from its digest-addressed
//! source) or at E9 (`replay`'s recompiled package) -- never by a
//! conversion function, and none is defined here.
//!
//! **Temporary home (QSL-158 S-3a).** ADR-013 T-3 assigns `PackageNodeKey`
//! to the new top-level `library` module, alongside `ImportView` and
//! `LibraryLock` (FR-087 Outputs). That module does not exist yet: FR-087's
//! `value::library`/`value::package_identity` relocation and `ImportView`'s
//! own construction are a separate, parallel slice of QSL-158 (owning
//! `ExportIdentity` -> `PackageNodeKey`'s call-site migration). This type is
//! defined here, in `package`, only so that slice has a stable, already-
//! compiling `PackageNodeKey` to build against without waiting on this
//! slice to land; it is a pure move into `library` with no shape change, not
//! a second, competing definition (there is exactly one `PackageNodeKey`,
//! here, until that move lands).
//!
//! `node`'s type, `WireNodeId`, is `crate::digest::WireNodeId` -- relocated
//! there by this same slice from its former, explicitly provisional home in
//! `replay::identity` (ADR-011 `:588` places a wire node id in the `F`
//! foundation layer, alongside `digest`, before `wire_format`; `replay`
//! (layer 6) now imports it from `digest` directly, with no re-export alias
//! left at the old `replay::WireNodeId` path).

use crate::digest::WireNodeId;
use crate::value::PackageId;

/// A cross-package node reference (ADR-013 T-3): pins the verified content
/// (`package`) and names a node inside it (`node`), without ever
/// constructing a `NodeKey` from wire bytes. Equality is declared: both
/// components compare lexically (ADR-013 §2), matching the derived
/// `PartialEq`/`Eq`/`PartialOrd`/`Ord` below -- no digest or structural
/// comparison over the referenced node's own content substitutes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PackageNodeKey {
    /// The verified package's own content-addressed identity.
    pub package: PackageId,
    /// The node's wire spelling inside that package, resolved to a
    /// `NodeKey` only by lookup in an already-checked package (E4/E9).
    pub node: WireNodeId,
}

impl PackageNodeKey {
    /// Build a reference from its two already-known components. Not
    /// checked typestate (R-10 governs `CheckedGraph`/`CheckedPackage`, not
    /// this plain data key): a `PackageNodeKey` names a node without
    /// claiming it resolves to one, the same way `WireNodeId::from_digest`
    /// carries no resolution claim either.
    pub fn new(package: PackageId, node: WireNodeId) -> Self {
        Self { package, node }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn package_id(byte: u8) -> PackageId {
        PackageId::of_preimage(&[byte; 32])
    }

    /// FR-087-AC-5: two `PackageNodeKey` values compare equal iff both
    /// components compare lexically equal -- the declared-equality claim
    /// itself, not only that the derive exists. A mutation that swapped the
    /// equality implementation for a structural comparison over the
    /// referenced node's content (rather than the two components
    /// themselves) would still pass a same-value-same-value check; this
    /// test additionally pins that changing either component alone breaks
    /// equality, which such a mutation could not do consistently for an
    /// opaque `WireNodeId`.
    #[trace("TC-245", "FR-087-AC-5")]
    #[test]
    fn equality_holds_iff_both_components_are_lexically_equal() {
        let a = PackageNodeKey::new(package_id(1), WireNodeId::from_digest([9; 32]));
        let same = PackageNodeKey::new(package_id(1), WireNodeId::from_digest([9; 32]));
        let different_package = PackageNodeKey::new(package_id(2), WireNodeId::from_digest([9; 32]));
        let different_node = PackageNodeKey::new(package_id(1), WireNodeId::from_digest([8; 32]));

        assert_eq!(a, same);
        assert_ne!(a, different_package);
        assert_ne!(a, different_node);
    }
}
