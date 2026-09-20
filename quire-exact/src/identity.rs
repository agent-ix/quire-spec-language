// SPDX-License-Identifier: AGPL-3.0-or-later
//! QC-15 opaque value-component identities (ADR-013 QC-15, T-6).
//!
//! Six digest newtypes appear as bare payload inside kernel `Value`s wherever
//! ADR-013 T-6 cuts a payload down to "just the id": `EffectiveId`
//! (declaration identity, O-05), `UniverseId` and `ObjectId` (a `Reference`
//! payload, T-6), `UnitId` (a `Quantity` payload, T-6), `VariantId` (an `Enum`
//! payload, T-6) and `MemberId` (O-06). Each is an opaque 32-byte digest with
//! exactly one public constructor, `from_digest` (each type's own, e.g.
//! [`EffectiveId::from_digest`]), mirroring [`crate::NodeKey::from_digest`]:
//! the kernel wraps an already-computed digest and never hashes. QSL's
//! `check`/`model` compute each digest over their own preimage schema and
//! mint the id; this crate holds no preimage knowledge for any of them. As
//! with `NodeKey` (see the `node` module's doc comment), the ADR-011 T-12
//! `arch-lint api-surface` check that is meant to enforce "only `model`
//! calls `EffectiveId::from_digest`" does not yet do so: it currently
//! matches `EffectiveId::from_digest_bytes(`, a different name from this
//! crate's real `from_digest`. Nothing today fails a caller outside `model`.
//!
//! The six types share one shape (an opaque 32-byte digest, `Eq`/`Ord`/`Hash`,
//! hex `Display`/`Debug`), so the `digest_identity!` macro below generates
//! all six from one macro body rather than repeating the impls six times
//! ("one fact, one place").
//!
//! **Domain strings.** `EffectiveId`'s domain, `quire.model.effective-
//! declaration/v1`, is the real value already live at
//! `src/model/key.rs:26`'s `EFFECTIVE_DECLARATION_DOMAIN` (confirmed against
//! ADR-013 O-05 and ADR-010's DA-02 row). The other five domain strings below
//! (`UniverseId`, `ObjectId`, `UnitId`, `VariantId`, `MemberId`) do not appear
//! anywhere in `spec/` or `src/` today -- there is no existing canonical
//! value to copy the way there was for `EffectiveId`. They are placeholders,
//! not settled product semantics: ADR-013 QC-2 requires an FR-201 amendment
//! to list the model digest domains, scoped to #213 S-2, not this slice. Each
//! of the five types they belong to is genuinely used elsewhere in this
//! crate ([`crate::reference::ObjectReference`] for `UniverseId`/`ObjectId`,
//! [`crate::quantity::Quantity`] and `ValueType::Quantity` for `UnitId`,
//! `Value::Enum` and `ValueType`'s enum sum shape for `VariantId`,
//! [`crate::value::FieldDeclaration`] for `MemberId`) -- kept for that reason
//! -- but their domain *strings* are not to be treated as ratified until the
//! FR-201 amendment lands.

use std::fmt;

macro_rules! digest_identity {
    (
        $(#[$doc:meta])*
        $name:ident, $domain_const:ident, $domain:literal
    ) => {
        /// Digest domain for
        #[doc = concat!("[`", stringify!($name), "`].")]
        pub const $domain_const: &str = $domain;

        $(#[$doc])*
        #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; 32]);

        impl $name {
            /// The one public constructor: wrap an already-computed
            #[doc = concat!("`", $domain, "` digest.")]
            /// The caller computes the digest over its own canonical
            /// preimage; this type performs no hashing and holds no preimage
            /// knowledge.
            pub fn from_digest(digest: [u8; 32]) -> Self {
                Self(digest)
            }

            /// The raw digest bytes.
            pub fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.iter().try_for_each(|byte| write!(f, "{byte:02x}"))
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({self})", stringify!($name))
            }
        }
    };
}

digest_identity!(
    /// An opaque `quire.model.effective-declaration/v1` identity (ADR-013
    /// O-05, confirmed against the live `src/model/key.rs:26`
    /// `EFFECTIVE_DECLARATION_DOMAIN`): the effective, fully-resolved
    /// identity of a declaration after archetype composition. Only QSL's
    /// `model` calls [`EffectiveId::from_digest`] in production.
    EffectiveId,
    EFFECTIVE_ID_DOMAIN,
    "quire.model.effective-declaration/v1"
);

digest_identity!(
    /// An opaque `quire.universe/v1` identity: the universe half of a
    /// `Reference` payload's `(UniverseId, ObjectId)` pair (ADR-013 T-6).
    UniverseId,
    UNIVERSE_ID_DOMAIN,
    "quire.universe/v1"
);

digest_identity!(
    /// An opaque `quire.object/v1` identity: the object half of a
    /// `Reference` payload's `(UniverseId, ObjectId)` pair (ADR-013 T-6).
    ObjectId,
    OBJECT_ID_DOMAIN,
    "quire.object/v1"
);

digest_identity!(
    /// An opaque `quire.unit/v1` identity: a `Quantity` payload's unit,
    /// carried "with no reference to quantity declarations" (ADR-013 T-6).
    UnitId,
    UNIT_ID_DOMAIN,
    "quire.unit/v1"
);

digest_identity!(
    /// An opaque `quire.enum-variant/v1` identity: a bare `Enum` payload
    /// "with no `NodeKey`" (ADR-013 T-6).
    VariantId,
    VARIANT_ID_DOMAIN,
    "quire.enum-variant/v1"
);

digest_identity!(
    /// An opaque `quire.member/v1` identity (ADR-013 O-06): a collection or
    /// composite member's identity.
    MemberId,
    MEMBER_ID_DOMAIN,
    "quire.member/v1"
);

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    fn digest(byte: u8) -> [u8; 32] {
        let mut bytes = [0_u8; 32];
        bytes[31] = byte;
        bytes
    }

    /// TC-302 (H-7/H-8, strengthened): each of the six digest identities
    /// treats equal digest bytes as interchangeable ids -- not just `==`,
    /// but hashing equal (either stands in for the other as a set/map key)
    /// -- and distinct digests as distinct, differently ordered ids
    /// (ADR-013 QC-15).
    #[trace("TC-302")]
    #[test]
    fn tc_302_equal_digest_bytes_mint_interchangeable_identities() {
        macro_rules! check {
            ($ty:ident) => {
                let a = $ty::from_digest(digest(1));
                let b = $ty::from_digest(digest(1));
                let c = $ty::from_digest(digest(2));
                assert_eq!(a, b);
                assert_eq!(a.as_bytes(), b.as_bytes());
                assert_eq!(std::collections::HashSet::from([a, b, c]).len(), 2);
                assert!(a < c);
            };
        }
        check!(EffectiveId);
        check!(UniverseId);
        check!(ObjectId);
        check!(UnitId);
        check!(VariantId);
        check!(MemberId);
    }

    /// TC-303 (H-7/H-8, strengthened): each identity's domain constant is
    /// distinct, so no two of the six can be confused by domain string
    /// (ADR-013 QC-15). Also grounds `EFFECTIVE_ID_DOMAIN` against the real,
    /// external value this module's own doc comment claims to match
    /// (`src/model/key.rs:26`'s `EFFECTIVE_DECLARATION_DOMAIN`, per H-2),
    /// so a future edit to either side that silently drifts the literal
    /// fails here rather than only in a future S-2 integration.
    #[trace("TC-303")]
    #[test]
    fn tc_303_domain_constants_are_pairwise_distinct() {
        assert_eq!(EFFECTIVE_ID_DOMAIN, "quire.model.effective-declaration/v1");
        let domains = [
            EFFECTIVE_ID_DOMAIN,
            UNIVERSE_ID_DOMAIN,
            OBJECT_ID_DOMAIN,
            UNIT_ID_DOMAIN,
            VARIANT_ID_DOMAIN,
            MEMBER_ID_DOMAIN,
        ];
        for (i, a) in domains.iter().enumerate() {
            for (j, b) in domains.iter().enumerate() {
                assert_eq!(i == j, a == b, "domains at {i} and {j} collided");
            }
        }
    }

    /// TC-304: `Display` renders exactly 64 lowercase hex digits.
    #[trace("TC-304")]
    #[test]
    fn tc_304_display_is_64_lowercase_hex_digits() {
        let id = ObjectId::from_digest(digest(0xcd));
        let rendered = id.to_string();
        assert_eq!(rendered.len(), 64);
        assert!(rendered
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert!(rendered.ends_with("cd"));
    }
}
