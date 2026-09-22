// SPDX-License-Identifier: AGPL-3.0-or-later
//! O-04 checked semantic node identity (ADR-013 O-04).
//!
//! A `NodeKey` is an opaque 32-byte digest in domain
//! `quire.checked-semantic-node/v1`. Node ids are content-addressed over the
//! `node-identity-preimage.schema.json` preimage: equal keys mean
//! structurally identical nodes.
//!
//! Minting rule (ADR-011 §6.1, ADR-013 O-04, T-6): the kernel `NodeKey` has
//! exactly one public constructor, [`NodeKey::from_digest`], which takes an
//! already-computed digest. It does not hash: the preimage schema, RFC 8785
//! JCS canonicalization and the SHA-256 computation over that canonical form
//! all stay in QSL, which computes the digest and passes the finished 32
//! bytes in. Only QSL's `check` module is meant to call this constructor in
//! production; this crate's own tests call it freely to exercise the type.
//!
//! A wire-read node id becomes a `NodeKey` only by lookup in a checked
//! package (ADR-013 O-04), never by parsing a digest string directly:
//! parsing a wire digest stays QSL's own concern
//! (`qsl_foundation::digest::WireNodeId`), not this constructor's. Bridging
//! another digest type's bytes into a `NodeKey` has no canonical role
//! either (ADR-013 O-05, OBS-018).
//!
//! `NodeKey::from_bytes` and the public `NodeKey::from_hex` are retired:
//! every QSL call site is rerouted onto `from_digest`. Two of QSL's
//! rerouted sites do not conform to the rule above and are known, tracked
//! debt (ADR-011 FB-13/SR-508; ADR-013 OBS-018), not this design's
//! sanctioned path: `value::node::NodeIdDocument::key` parses a wire
//! digest string and wraps it in a `NodeKey` directly, and
//! `value::model_query::to_object_reference` bridges an `EffectiveId`'s
//! bytes into a `NodeKey`.
//!
//! `arch-lint api-surface`'s T12-B rule (`tools/arch-lint/api_surface.rs`,
//! ADR-011 T-12) scans for `NodeKey::from_digest(` and the crate-internal
//! `node_key_of` helper QSL mints through, and reports every call site
//! outside `check`'s own submodules -- including the two named above.
//! `arch-lint` is not part of `make ci` (Makefile), so it is advisory, not
//! gating, today.

use std::fmt;

/// Digest domain of every checked semantic node key.
pub const NODE_KEY_DOMAIN: &str = "quire.checked-semantic-node/v1";

/// An opaque `quire.checked-semantic-node/v1` node key.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeKey([u8; 32]);

impl NodeKey {
    /// The one public constructor: wrap an already-computed
    /// `quire.checked-semantic-node/v1` digest. The caller (QSL `check`)
    /// computes the digest over the canonical preimage; this type performs
    /// no hashing and holds no preimage knowledge.
    pub fn from_digest(digest: [u8; 32]) -> Self {
        Self(digest)
    }

    /// The raw digest bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for NodeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|byte| write!(f, "{byte:02x}"))
    }
}

impl fmt::Debug for NodeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeKey({self})")
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    fn digest(byte: u8) -> [u8; 32] {
        let mut bytes = [0_u8; 32];
        bytes[31] = byte;
        bytes
    }

    /// TC-300 (H-7/H-8, strengthened): equal digests mint interchangeable
    /// `NodeKey`s (ADR-013 O-04 normalized identity) -- not just `==`, but
    /// hashing equal (so either stands in for the other as a set/map key)
    /// and ordering equal to zero. Unequal digests mint keys that order by
    /// raw digest bytes, not merely compare unequal.
    #[trace("TC-300")]
    #[test]
    fn tc_300_equal_digest_bytes_mint_interchangeable_node_keys() {
        use std::collections::HashSet;

        let a = NodeKey::from_digest(digest(1));
        let b = NodeKey::from_digest(digest(1));
        let c = NodeKey::from_digest(digest(2));
        assert_eq!(a, b);
        assert_eq!(a.as_bytes(), b.as_bytes());
        assert_eq!(a.cmp(&b), std::cmp::Ordering::Equal);
        assert_eq!(HashSet::from([a, b, c]).len(), 2);
        assert!(a < c);
    }

    /// TC-301: `Display` renders exactly 64 lowercase hex digits, the wire
    /// spelling QSL's emitter writes for `NodeId{digest}` (ADR-013 O-04).
    #[trace("TC-301")]
    #[test]
    fn tc_301_display_is_64_lowercase_hex_digits() {
        let key = NodeKey::from_digest(digest(0xab));
        let rendered = key.to_string();
        assert_eq!(rendered.len(), 64);
        assert!(rendered
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert!(rendered.ends_with("ab"));
    }
}
