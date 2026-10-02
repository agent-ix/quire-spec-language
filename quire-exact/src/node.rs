// SPDX-License-Identifier: AGPL-3.0-or-later
//! O-04 checked semantic node identity (ADR-013 O-04).
//!
//! A `NodeKey` is an opaque 32-byte digest in domain
//! `quire.checked-semantic-node/v1`. Node ids are content-addressed over the
//! `node-identity-preimage.schema.json` preimage: equal keys mean
//! structurally identical nodes.
//!
//! Minting rule (ADR-011 §6.1, ADR-013 O-04, T-6): the kernel `NodeKey` has
//! one minting constructor, [`NodeKey::from_digest`], which takes an
//! already-computed digest. It does not hash: the preimage schema, RFC 8785
//! JCS canonicalization and the SHA-256 computation over that canonical form
//! all stay in QSL, which computes the digest and passes the finished 32
//! bytes in. Only QSL's `check` module calls it in production; this crate's
//! own tests call it freely to exercise the type.
//!
//! Decoding rule (ADR-011 T-12, "Decoding an admitted key"): a key `check`
//! already minted, read back out of an admitted package's bytes, is decoded
//! with [`NodeKey::decode_admitted`], not minted. Its caller runs only after
//! the admitted-package identity check has bound the key bytes, and a
//! decoded key carries that check's provenance, not a fresh `check` mint.
//!
//! A wire-read node id otherwise becomes a `NodeKey` only by lookup in a
//! checked package (ADR-013 O-04), never by parsing a digest string
//! directly: parsing a wire digest stays QSL's own concern
//! (`qsl_foundation::digest::WireNodeId`). Bridging another digest type's
//! bytes into a `NodeKey` has no canonical role either (ADR-013 O-05,
//! OBS-018).
//!
//! Like `from_digest`, `decode_admitted` accepts any 32 bytes: the guarantee
//! is the call-site allow-list plus the verified binding, not the type.
//! `arch-lint api-surface` holds both allow-lists
//! (`tools/arch-lint/api_surface.rs`): T12-B reports every
//! `from_digest` call outside `check`, and T12-F every `decode_admitted`
//! call in QSL, which has no decode site; a backend's own admitted-package
//! reader is the intended caller.

use alloc::string::String;
use core::fmt;

/// Digest domain of every checked semantic node key.
pub const NODE_KEY_DOMAIN: &str = "quire.checked-semantic-node/v1";

/// Whether `text` is `^[A-Za-z_][A-Za-z0-9_]*$`: the one definition of this
/// character class, which every identifier-shaped field and [`Identifier`]
/// validate against.
pub fn is_identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// A `node-identity-preimage.schema.json` `$defs.Identifier`
/// (`^[A-Za-z_][A-Za-z0-9_]*$`): one identifier-shaped name segment. An
/// invalid identifier is refused at construction rather than reaching a
/// schema-invalid wire shape (ADR-013 O-06).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Identifier(String);

/// A string that is not `^[A-Za-z_][A-Za-z0-9_]*$`.
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("an identifier is `^[A-Za-z_][A-Za-z0-9_]*$`")]
pub struct InvalidIdentifier;

impl Identifier {
    /// `value` as an identifier, or [`InvalidIdentifier`] if it is not
    /// `^[A-Za-z_][A-Za-z0-9_]*$`.
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidIdentifier> {
        let value = value.into();
        if is_identifier(&value) {
            Ok(Self(value))
        } else {
            Err(InvalidIdentifier)
        }
    }

    /// The identifier's own text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An opaque `quire.checked-semantic-node/v1` node key.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeKey([u8; 32]);

impl NodeKey {
    /// The minting constructor: wrap an already-computed
    /// `quire.checked-semantic-node/v1` digest. The caller (QSL `check`)
    /// computes the digest over the canonical preimage; this type performs
    /// no hashing and holds no preimage knowledge.
    pub fn from_digest(digest: [u8; 32]) -> Self {
        Self(digest)
    }

    /// Decode a key `check` minted and an admitted package carries
    /// (ADR-011 T-12, "Decoding an admitted key"). It hashes nothing and
    /// takes no preimage: it re-reads `digest`, the key's own bytes.
    ///
    /// Precondition: call it only on the bytes of a package that already
    /// passed the admitted-package identity check (ADR-011 §4's verified
    /// binding), so a forged package is refused before any of its keys is
    /// decoded. The decoded key carries that check's provenance; it never
    /// feeds a constructor that asserts `check` produced it. The guarantee
    /// is the call-site allow-list (`arch-lint api-surface` T12-F) plus the
    /// verified binding, not this type.
    pub fn decode_admitted(digest: [u8; 32]) -> Self {
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
    use alloc::string::ToString;

    use super::*;

    fn digest(byte: u8) -> [u8; 32] {
        let mut bytes = [0_u8; 32];
        bytes[31] = byte;
        bytes
    }

    /// (H-7/H-8, strengthened): equal digests mint interchangeable
    /// `NodeKey`s (ADR-013 O-04 normalized identity) -- not just `==`, but
    /// hashing equal (so either stands in for the other as a set/map key)
    /// and ordering equal to zero. Unequal digests mint keys that order by
    /// raw digest bytes, not merely compare unequal.
    #[test]
    fn tc_300_equal_digest_bytes_mint_interchangeable_node_keys() {
        use std::collections::HashSet;

        let a = NodeKey::from_digest(digest(1));
        let b = NodeKey::from_digest(digest(1));
        let c = NodeKey::from_digest(digest(2));
        assert_eq!(a, b);
        assert_eq!(a.as_bytes(), b.as_bytes());
        assert_eq!(a.cmp(&b), core::cmp::Ordering::Equal);
        assert_eq!(HashSet::from([a, b, c]).len(), 2);
        assert!(a < c);
    }

    /// ADR-011 T-12: a key decoded from an admitted package's bytes is the
    /// key `check` minted from the same digest -- equal, hashing equal and
    /// ordering equal -- so a lookup by the decoded key finds the minted one.
    #[test]
    fn a_decoded_admitted_key_is_the_key_check_minted() {
        use std::collections::HashSet;

        let minted = NodeKey::from_digest(digest(7));
        let decoded = NodeKey::decode_admitted(*minted.as_bytes());
        assert_eq!(decoded, minted);
        assert_eq!(decoded.cmp(&minted), core::cmp::Ordering::Equal);
        assert!(HashSet::from([minted]).contains(&decoded));
        assert_ne!(NodeKey::decode_admitted(digest(8)), minted);
    }

    /// `Display` renders exactly 64 lowercase hex digits, the wire
    /// spelling QSL's emitter writes for `NodeId{digest}` (ADR-013 O-04).
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

    /// A name that is not `^[A-Za-z_][A-Za-z0-9_]*$` is refused at
    /// `Identifier::new`.
    #[test]
    fn a_non_identifier_name_is_refused() {
        for invalid in ["not an id!", "", "1starts_with_digit", "has-a-dash"] {
            assert_eq!(
                Identifier::new(invalid),
                Err(InvalidIdentifier),
                "{invalid:?} must be refused"
            );
        }
    }

    /// The positive complement of the refusal above.
    #[test]
    fn ordinary_identifiers_are_accepted() {
        for valid in [
            "quantity",
            "source",
            "totalPrice",
            "add",
            "_leading_underscore",
        ] {
            assert!(Identifier::new(valid).is_ok(), "{valid:?} must be accepted");
        }
    }
}
