// SPDX-License-Identifier: AGPL-3.0-or-later
//! QC-15/QC-21 opaque value-component identities (ADR-013 QC-15, QC-21, T-6).
//!
//! Six digest newtypes and one authored-string newtype appear as bare
//! payload inside kernel `Value`s wherever ADR-013 T-6 cuts a payload down to
//! "just the id": `EffectiveId` (declaration identity, O-05), `UniverseId`
//! and `ObjectId` (a `Reference` payload, T-6), `UnitId` (a `Quantity`
//! payload, T-6), `VariantId` (an `Enum` payload, T-6), `MemberId` (O-06) and
//! `PopulationId` (a `Population` payload, O-13 Population row, QC-21,
//! QSL-172). Every one but `ObjectId` is an opaque 32-byte digest with
//! exactly one public constructor, `from_digest` (each type's own, e.g.
//! [`EffectiveId::from_digest`]), mirroring [`crate::NodeKey::from_digest`]:
//! the kernel wraps an already-computed digest and never hashes. QSL's
//! `check`/`model` compute each digest over their own preimage schema and
//! mint the id; this crate holds no preimage knowledge for any of them.
//! [`ObjectId`] is different in kind, not degree (ADR-013 §8 OQ-C ruling):
//! QSpec FR-204 states a declared object identity is "authored, not
//! digested", so `ObjectId` wraps the object's own exact UTF-8 bytes
//! directly, with one public constructor, `new`, that takes the string
//! itself and refuses only when it is empty (QSpec FR-035) -- never a digest,
//! never a hash. The ADR-011 T-12 `arch-lint api-surface` check enforces
//! "only `model` calls `EffectiveId::from_digest`" (T12-C) and "only `model`
//! calls `PopulationId::from_digest`" (T12-D, ADR-013 QC-21) by scanning
//! QSL's tree for each call pattern (#213 S-2 fixed T12-C's pattern to match
//! this crate's real `from_digest`, in place of the pre-migration
//! `from_digest_bytes(` name); which callers it allows for `UniverseId`,
//! `ObjectId`, `UnitId`, `VariantId` and `MemberId` stays open (ADR-013
//! QC-22).
//! `NodeKey`'s own T12-B rule (see the `node` module's doc comment) scans
//! QSL's tree for `NodeKey::from_digest` the same way.
//!
//! The six digest types share one shape (an opaque 32-byte digest,
//! `Eq`/`Ord`/`Hash`, hex `Display`/`Debug`), so the `digest_identity!` macro
//! below generates all six from one macro body rather than repeating the
//! impls six times ("one fact, one place"). `ObjectId` has its own shape
//! (an opaque non-empty UTF-8 string) and its own impls, immediately after
//! the macro-generated types.
//!
//! **Domain strings.** `EffectiveId`'s domain, `quire.model.effective-
//! declaration/v1`, is the real value already live at
//! `src/model/key.rs:26`'s `EFFECTIVE_DECLARATION_DOMAIN` (confirmed against
//! ADR-013 O-05 and ADR-010's DA-02 row). `UniverseId`'s domain,
//! `quire.model.object-universe/v1`, is likewise the real value already live
//! at `src/model/key.rs:42-43`'s `OBJECT_UNIVERSE_DOMAIN` (ADR-013 §8 OQ-C
//! ruling, confirmed against QSpec `model-complete.md`'s "Object universe"
//! and FR-201's `quire.model.object-universe/v1` row). `ObjectId` carries no
//! digest domain at all: OQ-C settles it as the authored UTF-8 object
//! identity (QSpec FR-204, "authored, not digested"; non-empty per QSpec
//! FR-035), never hashed, so [`ObjectId`] is not one of this module's digest
//! newtypes -- see its own definition below. The four remaining domain
//! strings (`UnitId`, `VariantId`, `MemberId`, `PopulationId`) do not appear
//! anywhere in `spec/` or `src/` today -- there is no existing canonical
//! value to copy the way there was for `EffectiveId` and `UniverseId`. They
//! are placeholders, not settled product semantics: ADR-013 QC-2 requires an
//! FR-201 amendment to list the model digest domains, scoped to #213 S-2,
//! not this slice. Each of the four types they belong to is genuinely used
//! elsewhere in this crate ([`crate::quantity::Quantity`] and
//! `ValueType::Quantity` for `UnitId`, `Value::Enum` and `ValueType`'s enum
//! sum shape for `VariantId`, [`crate::value::FieldDeclaration`] for
//! `MemberId`, `Value::Population` and `ValueType::Population` for
//! `PopulationId`) -- kept for that reason -- but their domain *strings* are
//! not to be treated as ratified until the FR-201 amendment lands.

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
    /// An opaque `quire.model.object-universe/v1` identity: the universe
    /// half of a `Reference` payload's `(UniverseId, ObjectId)` pair (ADR-013
    /// T-6, §8 OQ-C and OQ-E rulings). One per connected component of the
    /// object-type supertype graph (OQ-E). Only QSL `model` computes this
    /// digest, over the `ObjectUniverse` preimage
    /// (`quire.model.object-universe/v1`) `model-complete.md` defines.
    UniverseId,
    UNIVERSE_ID_DOMAIN,
    "quire.model.object-universe/v1"
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

digest_identity!(
    /// An opaque `quire.population/v1` identity (ADR-013 O-13 Population
    /// row, QC-21, QSL-172): a `Population` payload's admission identity.
    /// Only QSL `model` calls [`PopulationId::from_digest`] in production,
    /// minting it from the admitted binding's own preimage (domain package,
    /// `population_key`, and the closed three-state admission-role
    /// discriminator `Direct`/`Pre`/`Post`) at admission time -- this crate
    /// holds none of that preimage knowledge, only the resulting digest.
    /// `PopulationBinding` itself -- admission, membership, and the
    /// `allInstances`/`lookup` closure state -- stays a QSL `model` type,
    /// never a kernel one (FR-089).
    PopulationId,
    POPULATION_ID_DOMAIN,
    "quire.population/v1"
);

/// An empty [`ObjectId`]: QSpec FR-035 requires a non-empty authored object
/// identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("an object identity has at least one authored UTF-8 byte")]
pub struct EmptyObjectIdentity;

/// The object half of a `Reference` payload's `(UniverseId, ObjectId)` pair
/// (ADR-013 T-6, §8 OQ-C ruling): the object's own authored identity,
/// carried as its exact UTF-8 bytes. Unlike every type the `digest_identity!`
/// macro above generates, `ObjectId` is never a digest and has no digest
/// domain (QSpec FR-204: "authored, not digested") -- `model` never hashes
/// it, so there is no digest for this type to wrap.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObjectId(Box<str>);

impl ObjectId {
    /// The one public constructor: `identity`'s own authored UTF-8 bytes,
    /// exactly as supplied. Refuses only when `identity` is empty (QSpec
    /// FR-035); this crate performs no other validation of the authored
    /// identity.
    pub fn new(identity: impl Into<String>) -> Result<Self, EmptyObjectIdentity> {
        let identity = identity.into();
        if identity.is_empty() {
            return Err(EmptyObjectIdentity);
        }
        Ok(Self(identity.into_boxed_str()))
    }

    /// The identity's own exact UTF-8 bytes.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
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

    /// TC-302 (H-7/H-8, strengthened): each of the six digest identities
    /// (QC-15's five other than `ObjectId`, plus QC-21's `PopulationId`)
    /// treats equal digest bytes as interchangeable ids -- not just `==`,
    /// but hashing equal (either stands in for the other as a set/map key)
    /// -- and distinct digests as distinct, differently ordered ids
    /// (ADR-013 QC-15, QC-21). `ObjectId` is not a digest (ADR-013 §8 OQ-C
    /// ruling) and has its own equivalent coverage below
    /// (`tc_302b_equal_object_identity_bytes_mint_interchangeable_identities`).
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
        check!(UnitId);
        check!(VariantId);
        check!(MemberId);
        check!(PopulationId);
    }

    /// TC-302 (H-7/H-8) equivalent for `ObjectId`: equal authored UTF-8
    /// bytes mint interchangeable ids -- `==`, equal hashing, and distinct
    /// strings compare unequal and order lexically over their bytes (ADR-013
    /// §8 OQ-C ruling: `ObjectId` carries its bytes directly, never a
    /// digest, so there is no digest to equate here).
    #[trace("TC-302")]
    #[test]
    fn tc_302b_equal_object_identity_bytes_mint_interchangeable_identities() {
        let a = ObjectId::new("o1").unwrap();
        let b = ObjectId::new("o1").unwrap();
        let c = ObjectId::new("o2").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.as_str(), b.as_str());
        assert_eq!(std::collections::HashSet::from([a.clone(), b, c.clone()]).len(), 2);
        assert!(a < c);
    }

    /// An empty authored object identity refuses (QSpec FR-035): this is the
    /// one validation `ObjectId::new` performs, so it is worth its own
    /// mutation-provable assertion (removing the `is_empty` check would let
    /// this construct `Ok`).
    #[trace("TC-302")]
    #[test]
    fn tc_302c_empty_object_identity_refuses() {
        assert_eq!(ObjectId::new("").unwrap_err(), EmptyObjectIdentity);
        assert!(ObjectId::new("o1").is_ok());
    }

    /// TC-303 (H-7/H-8, strengthened): each identity's domain constant is
    /// distinct, so no two of the six digest identities (QC-15's five other
    /// than `ObjectId`, plus QC-21's `PopulationId`) can be confused by
    /// domain string (ADR-013 QC-15, QC-21). Also pins `EFFECTIVE_ID_DOMAIN`
    /// and `UNIVERSE_ID_DOMAIN` against the literals this module's own doc
    /// comment claims to match (`src/model/key.rs:26,43`'s
    /// `EFFECTIVE_DECLARATION_DOMAIN`/`OBJECT_UNIVERSE_DOMAIN`, per H-2).
    /// `ObjectId` has no digest domain to include (ADR-013 §8 OQ-C ruling).
    ///
    /// **L-2, doc correction: this is a hard-coded literal, not an
    /// import-based check.** A leaf crate at ADR-011's module-DAG layer `K`
    /// cannot depend on QSL's `src/model`, so this test cannot actually read
    /// `EFFECTIVE_DECLARATION_DOMAIN`/`OBJECT_UNIVERSE_DOMAIN` and compare
    /// against them live -- it compares `EFFECTIVE_ID_DOMAIN`/
    /// `UNIVERSE_ID_DOMAIN` to a second copy of the same strings typed here.
    /// A future edit to `src/model/key.rs` alone, with these literals left
    /// unchanged, leaves this test green even though the values have now
    /// drifted; only an edit to *this* crate's own domain constants would be
    /// caught. The test is right to exist at this crate's boundary -- there
    /// is no other way to pin the value here -- the previous wording
    /// overstated what it actually catches.
    #[trace("TC-303")]
    #[test]
    fn tc_303_domain_constants_are_pairwise_distinct() {
        assert_eq!(EFFECTIVE_ID_DOMAIN, "quire.model.effective-declaration/v1");
        assert_eq!(UNIVERSE_ID_DOMAIN, "quire.model.object-universe/v1");
        let domains = [
            EFFECTIVE_ID_DOMAIN,
            UNIVERSE_ID_DOMAIN,
            UNIT_ID_DOMAIN,
            VARIANT_ID_DOMAIN,
            MEMBER_ID_DOMAIN,
            POPULATION_ID_DOMAIN,
        ];
        for (i, a) in domains.iter().enumerate() {
            for (j, b) in domains.iter().enumerate() {
                assert_eq!(i == j, a == b, "domains at {i} and {j} collided");
            }
        }
    }

    /// TC-304: a digest identity's `Display` renders exactly 64 lowercase
    /// hex digits.
    #[trace("TC-304")]
    #[test]
    fn tc_304_display_is_64_lowercase_hex_digits() {
        let id = UniverseId::from_digest(digest(0xcd));
        let rendered = id.to_string();
        assert_eq!(rendered.len(), 64);
        assert!(rendered
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert!(rendered.ends_with("cd"));
    }

    /// TC-304 equivalent for `ObjectId`: `Display` renders the authored
    /// bytes exactly, never hex -- the OQ-C ruling's "not a digest" holds
    /// all the way through formatting.
    #[trace("TC-304")]
    #[test]
    fn tc_304b_object_id_display_is_the_authored_bytes_exactly() {
        let id = ObjectId::new("zz9-plural-z-alpha").unwrap();
        assert_eq!(id.to_string(), "zz9-plural-z-alpha");
        assert_eq!(id.as_str(), "zz9-plural-z-alpha");
    }
}
