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
//! bytes in. Only QSL's `check` module calls this constructor in production
//! (the ADR-011 T-12 API-surface check enforces that no other crate that
//! depends on `quire-exact` calls it either); this crate's own tests call it
//! freely to exercise the type.
//!
//! `NodeKey::from_bytes` (the QSL `value/node.rs:49` internal escape hatch)
//! and the public `NodeKey::from_hex` (`value/node.rs:25`) are both retired by
//! this cut: hex parsing is a wire-string concern that stays with QSL's
//! `WireNodeId`, which becomes a `NodeKey` only by lookup in a checked
//! package (ADR-013 O-04), never by parsing a digest string directly.

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

    /// TC-300: equal digests mint equal, interchangeable `NodeKey`s
    /// (ADR-013 O-04 normalized identity), and unequal digests mint distinct
    /// keys.
    #[trace("TC-300")]
    #[test]
    fn tc_300_equal_digest_bytes_mint_equal_node_keys() {
        let a = NodeKey::from_digest(digest(1));
        let b = NodeKey::from_digest(digest(1));
        let c = NodeKey::from_digest(digest(2));
        assert_eq!(a, b);
        assert_eq!(a.as_bytes(), b.as_bytes());
        assert_ne!(a, c);
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
