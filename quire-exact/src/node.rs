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
//! Two QSL call sites built on `from_digest` do not conform to the minting
//! rule above and are named debt (FR-060 T12-B's named-debt list), not this
//! design's sanctioned path:
//! `value::node::NodeIdDocument::key` parses a wire digest string and wraps
//! it in a `NodeKey` directly, and `value::model_query::to_object_reference`
//! bridges an `EffectiveId`'s bytes into a `NodeKey`.
//!
//! `arch-lint api-surface`'s T12-B rule (`tools/arch-lint/api_surface.rs`)
//! scans for the kernel constructor and reports every call site outside
//! T12-B's own allow-list (`tools/arch-lint/api_surface.rs`) -- including
//! the two named above. `arch-lint` is not part of `make ci` (Makefile), so
//! it is advisory, not gating, today.

use std::fmt;

/// Digest domain of every checked semantic node key.
pub const NODE_KEY_DOMAIN: &str = "quire.checked-semantic-node/v1";

/// Whether `text` is `^[A-Za-z_][A-Za-z0-9_]*$`: the one definition of this
/// character class every identifier-shaped field that carries an
/// [`Identifier`] validates against. QSL's own `value::node`, `value::member`
/// and `check` each used to define this same class independently; this is
/// the single copy all of them call now ("one fact, one place").
pub fn is_identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// A `node-identity-preimage.schema.json` `$defs.Identifier`
/// (`^[A-Za-z_][A-Za-z0-9_]*$`): the shared identifier-shaped single name
/// segment every checked-member name/operator field, `check`'s own
/// name-segment use and the layer-6 `replay` facade's `QualifiedName`
/// carry, so an invalid identifier is refused at construction rather than
/// reaching a schema-invalid wire shape (ADR-013 O-06) or a second copy of
/// this same check. `value::member` and `check` import this type directly
/// from this crate; no other crate defines or re-exports it.
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
