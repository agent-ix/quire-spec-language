// SPDX-License-Identifier: AGPL-3.0-or-later
//! QC-15/QC-21/QC-22 value-component identities (ADR-013 QC-15, QC-21,
//! QC-22, T-6).
//!
//! Seven identities appear as bare payload inside kernel `Value`s wherever
//! ADR-013 T-6 cuts a payload down to "just the id": `EffectiveId`
//! (declaration identity, O-05), `UniverseId` and `ObjectId` (a `Reference`
//! payload, T-6), `UnitId` (a `Quantity` payload, T-6), `VariantId` (an `Enum`
//! payload, T-6), `MemberId` (O-06) and `PopulationId` (a `Population`
//! payload, O-13 Population row, QC-21, QSL-172). The kernel wraps an
//! already-computed digest and never hashes: QSL computes each digest over
//! its own preimage schema and mints the id; this crate holds no preimage
//! knowledge for any of them.
//!
//! Six of the seven are single-domain opaque 32-byte digests with exactly
//! one public constructor, `from_digest` (each type's own, e.g.
//! [`EffectiveId::from_digest`]), mirroring [`crate::NodeKey::from_digest`].
//! They share one shape (`Eq`/`Ord`/`Hash`, hex `Display`/`Debug`), so the
//! `digest_identity!` macro below generates all six from one macro body
//! ("one fact, one place"). [`UnitId`] is the seventh and differs: the
//! ADR-013 OQ-B ruling makes it a domain-labelled digest record over the two
//! unit identities QSpec FR-142 defines, with one constructor per domain
//! (QC-22), so it is written out by hand below.
//!
//! The ADR-011 T-12 `arch-lint api-surface` check enforces "only `model`
//! calls `EffectiveId::from_digest`" (T12-C) and "only `model` calls
//! `PopulationId::from_digest`" (T12-D, ADR-013 QC-21) by scanning QSL's
//! tree for each call pattern. `NodeKey`'s own T12-B rule (see the `node`
//! module's doc comment) scans QSL's tree for `NodeKey::from_digest` the
//! same way. Which callers may mint the other identities is open (QC-22).
//!
//! **Domain strings.** `EffectiveId`'s domain, `quire.model.effective-
//! declaration/v1`, is the real value already live at
//! `src/model/key.rs:26`'s `EFFECTIVE_DECLARATION_DOMAIN` (confirmed against
//! ADR-013 O-05 and ADR-010's DA-02 row). `UnitId`'s two domains are QSpec
//! FR-142's: [`crate::NODE_KEY_DOMAIN`] for a declared unit's node key and
//! [`COMPOUND_UNIT_DOMAIN`] for a compound unit (ADR-013 OQ-B). The other
//! five domain strings below (`UniverseId`, `ObjectId`, `VariantId`,
//! `MemberId`, `PopulationId`) do not appear anywhere in `spec/` or `src/`
//! today -- there is no existing canonical value to copy the way there was
//! for `EffectiveId`. They are placeholders, not settled product semantics:
//! ADR-013 QC-2 requires an FR-201 amendment to list the model digest
//! domains, scoped to #213 S-2, not this slice. Each of the types they
//! belong to is genuinely used elsewhere in this crate
//! ([`crate::reference::ObjectReference`] for `UniverseId`/`ObjectId`,
//! `Value::Enum` and `ValueType`'s enum sum shape for `VariantId`,
//! [`crate::value::FieldDeclaration`] for `MemberId`, `Value::Population`
//! and `ValueType::Population` for `PopulationId`) -- kept for that reason
//! -- but their domain *strings* are not to be treated as ratified until the
//! FR-201 amendment lands.

use std::cmp::Ordering;
use std::fmt;

use crate::node::{NodeKey, NODE_KEY_DOMAIN};

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

/// Digest domain and preimage version of a compound unit's [`UnitId`]: QSpec
/// FR-142's evaluator-owned `quire.value.compound-unit/v1` value identity
/// (ADR-013 T-6, OQ-B ruling).
pub const COMPOUND_UNIT_DOMAIN: &str = "quire.value.compound-unit/v1";

/// Which of QSpec FR-142's two unit identities a [`UnitId`] carries (ADR-013
/// OQ-B ruling). No other domain is admitted (T-6).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UnitDomain {
    /// A declared unit: its node key, in [`NODE_KEY_DOMAIN`]
    /// (`quire.checked-semantic-node/v1` over the `quire.unit-node/v1`
    /// preimage).
    Declared,
    /// A compound unit: its [`COMPOUND_UNIT_DOMAIN`] digest.
    Compound,
}

impl UnitDomain {
    /// The digest-domain label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Declared => NODE_KEY_DOMAIN,
            Self::Compound => COMPOUND_UNIT_DOMAIN,
        }
    }
}

/// A `Quantity` payload's unit (ADR-013 T-6, OQ-B ruling): a domain-labelled
/// digest record with O-18's shape over the two unit identities QSpec FR-142
/// defines. A declared unit carries its node key as opaque bytes under the
/// [`NODE_KEY_DOMAIN`] label, and a compound unit carries its
/// [`COMPOUND_UNIT_DOMAIN`] digest. The kernel names no declaration type and
/// computes neither digest: QSL `semantic_value` mints every `UnitId`.
///
/// Equality and order are lexical on the domain label, then on the bytes
/// (R-04), so a declared unit and a compound unit are never equal whatever
/// their bytes.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct UnitId {
    domain: UnitDomain,
    digest: [u8; 32],
}

impl UnitId {
    /// The declared-unit arm: `key` is the unit's node key. The caller
    /// admits only a key minted over a `quire.unit-node/v1` preimage
    /// (ADR-013 C-30); this type cannot tell one node key from another.
    pub fn declared(key: NodeKey) -> Self {
        Self {
            domain: UnitDomain::Declared,
            digest: *key.as_bytes(),
        }
    }

    /// The compound-unit arm: wrap an already-computed
    /// `quire.value.compound-unit/v1` digest.
    pub fn compound(digest: [u8; 32]) -> Self {
        Self {
            domain: UnitDomain::Compound,
            digest,
        }
    }

    /// The domain this identity is labelled with.
    pub fn domain(&self) -> UnitDomain {
        self.domain
    }

    /// The raw digest bytes, meaningful only under [`Self::domain`].
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.digest
    }
}

impl Ord for UnitId {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.domain.label(), &self.digest).cmp(&(other.domain.label(), &other.digest))
    }
}

impl PartialOrd for UnitId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The digest bytes as 64 lowercase hex digits; the domain is not rendered
/// (see [`UnitId::domain`]).
impl fmt::Display for UnitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.digest
            .iter()
            .try_for_each(|byte| write!(f, "{byte:02x}"))
    }
}

impl fmt::Debug for UnitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UnitId({}, {self})", self.domain.label())
    }
}

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

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    fn digest(byte: u8) -> [u8; 32] {
        let mut bytes = [0_u8; 32];
        bytes[31] = byte;
        bytes
    }

    /// TC-302 (H-7/H-8, strengthened): each of the six single-domain digest
    /// identities (QC-15's, plus QC-21's `PopulationId`; `UnitId`'s own test
    /// is below) treats equal digest bytes
    /// as interchangeable ids -- not just `==`, but hashing equal (either
    /// stands in for the other as a set/map key) -- and distinct digests as
    /// distinct, differently ordered ids (ADR-013 QC-15, QC-21).
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
        check!(VariantId);
        check!(MemberId);
        check!(PopulationId);
    }

    /// TC-303 (H-7/H-8, strengthened): each identity's domain constant is
    /// distinct, so no two of the identities (QC-15's, QC-21's
    /// `PopulationId` and QC-22's compound-unit arm of `UnitId`) can be
    /// confused by domain string (ADR-013 QC-15,
    /// QC-21). Also pins `EFFECTIVE_ID_DOMAIN` against the literal
    /// this module's own doc comment claims to match (`src/model/key.rs:26`'s
    /// `EFFECTIVE_DECLARATION_DOMAIN`, per H-2).
    ///
    /// **L-2, doc correction: this is a hard-coded literal, not an
    /// import-based check.** A leaf crate at ADR-011's module-DAG layer `K`
    /// cannot depend on QSL's `src/model`, so this test cannot actually read
    /// `EFFECTIVE_DECLARATION_DOMAIN` and compare against it live -- it
    /// compares `EFFECTIVE_ID_DOMAIN` to a second copy of the same string
    /// typed here. A future edit to `src/model/key.rs:26` alone, with this
    /// literal left unchanged, leaves this test green even though the two
    /// values have now drifted; only an edit to *this* crate's own
    /// `EFFECTIVE_ID_DOMAIN` would be caught. The test is right to exist at
    /// this crate's boundary -- there is no other way to pin the value here
    /// -- the previous wording overstated what it actually catches.
    #[trace("TC-303")]
    #[test]
    fn tc_303_domain_constants_are_pairwise_distinct() {
        assert_eq!(EFFECTIVE_ID_DOMAIN, "quire.model.effective-declaration/v1");
        let domains = [
            EFFECTIVE_ID_DOMAIN,
            UNIVERSE_ID_DOMAIN,
            OBJECT_ID_DOMAIN,
            COMPOUND_UNIT_DOMAIN,
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

    /// TC-411 (kernel half): a `UnitId` is labelled with one of its two
    /// domains; equality and order are lexical on the label, then the bytes,
    /// so a declared and a compound unit over identical bytes are unequal,
    /// and every declared unit orders before every compound unit
    /// (`quire.checked-semantic-node/v1` < `quire.value.compound-unit/v1`).
    #[trace("FR-088-AC-12", "TC-411")]
    #[test]
    fn tc_411_unit_id_is_a_two_domain_record_compared_on_label_then_bytes() {
        let declared = UnitId::declared(NodeKey::from_digest(digest(2)));
        let compound = UnitId::compound(digest(2));
        assert_eq!(declared.as_bytes(), compound.as_bytes());
        assert_ne!(declared, compound);
        assert_eq!(
            std::collections::HashSet::from([declared, compound]).len(),
            2
        );
        assert_eq!(declared.domain(), UnitDomain::Declared);
        assert_eq!(declared.domain().label(), NODE_KEY_DOMAIN);
        assert_eq!(compound.domain(), UnitDomain::Compound);
        assert_eq!(compound.domain().label(), COMPOUND_UNIT_DOMAIN);
        assert!(declared < compound);
        assert!(UnitId::declared(NodeKey::from_digest(digest(9))) < UnitId::compound(digest(1)));
        assert!(UnitId::compound(digest(1)) < UnitId::compound(digest(2)));
        assert_eq!(declared, UnitId::declared(NodeKey::from_digest(digest(2))));
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
