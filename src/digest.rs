// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-001/004: exact byte integrity, independent of semantic canonicalization
//! ([`ByteDigest`]). ADR-013 O-18: one domain-labelled digest record for
//! every digest QSL mints or reads ([`DigestRecord`]).
use sha2::{Digest, Sha256};
use std::{fmt, str::FromStr};

/// SHA-256 value with canonical lowercase, algorithm-prefixed text encoding.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ByteDigest([u8; 32]);

impl ByteDigest {
    /// Hash precisely the supplied bytes, without parsing or normalization.
    pub fn of(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    /// The raw 32-byte digest. `pub(crate)`: outside this module, the
    /// `sha256:`-prefixed [`fmt::Display`] form is `ByteDigest`'s public
    /// spelling; this is an internal accessor for callers, such as
    /// `replay::identity`'s digest-match checks, that need the raw bytes
    /// rather than the prefixed text.
    pub(crate) fn as_bytes(&self) -> [u8; 32] {
        self.0
    }

    // Native input fields declare the algorithm through their enclosing format.
    // Other digest domains retain the algorithm-prefixed FromStr contract.
    pub(crate) fn from_hex(hex: &str) -> Result<Self, InvalidDigest> {
        if hex.len() != 64
            || !hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(InvalidDigest);
        }
        let mut bytes = [0; 32];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).map_err(|_| InvalidDigest)?;
        }
        Ok(Self(bytes))
    }
}

impl fmt::Display for ByteDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("sha256:")?;
        fmt::LowerHex::fmt(self, f)
    }
}

impl fmt::LowerHex for ByteDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Refusal of a noncanonical or malformed textual byte digest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidDigest;

impl fmt::Display for InvalidDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expected sha256: followed by 64 lowercase hexadecimal digits")
    }
}
impl std::error::Error for InvalidDigest {}

impl FromStr for ByteDigest {
    type Err = InvalidDigest;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let hex = text.strip_prefix("sha256:").ok_or(InvalidDigest)?;
        Self::from_hex(hex)
    }
}

// ---------------------------------------------------------------------------
// ADR-013 O-18: digest records.
//
// A fold candidate for this type is a wire `{domain, digest}` pair whose
// `domain` is a `DigestDomain` variant -- not a bare domain-label string
// with no accompanying digest field in the same object, not a wire shape
// that carries its own `algorithm` field (this type fixes SHA-256), and not
// a domain outside FR-201's 21-member vocabulary.
//
// One real caller landed in this slice: `value::member::Member::to_wire`'s
// `declaration_json` mints a `NodeKey`'s digest through
// `DigestRecord::mint(DigestDomain::CheckedSemanticNodeV1, ...)` (#260
// review item 5), in place of the bare `NODE_KEY_DOMAIN` constant plus raw
// hex it used before.
//
// A hand-built enumeration of every other site meeting the criterion above
// was attempted across three rounds of review on this one comment and was
// wrong every time (3 candidates named, then corrected to 4, then corrected
// to 7 -- one of the 7, `model::intake::admit`'s digest-domain check, was at
// one point ordered folded and then correctly un-ordered: nothing
// downstream of that check consumes a `DigestRecord`, so folding it would
// mint one and discard it. Its own real gap -- no adverse test distinguished
// an unrecognized domain from a valid FR-201 domain that is merely the
// wrong one -- is fixed regardless, at `refuses_a_valid_fr201_domain_that_
// is_not_sha256_jcs` in its own test module). A hand-built list is not a
// reliable artifact for a source comment to carry; the remaining real
// sites are tracked as O-18 debt on QSL-26 instead of enumerated here.
//
// A candidate blocked on a spec decision is still a fold target, not a
// disqualified one -- for example, `DomainPackageRef.digest` staying
// `[u8; 32]` rather than `DigestRecord` is blocked by ADR-013 O-01's own
// decided public shape (its `Field`/`Decision` table keeps `digest_domain`
// and `digest` as separate members), not by the shape criterion above.
// Recorded on QSL-26 for whoever can amend that decision; not this slice's
// to touch.
// ---------------------------------------------------------------------------

/// The closed FR-201 canonical-identity-domain vocabulary, as amended by
/// QSpec#138 (QC-2, merged `f4128c45`): the sixteen domains FR-201 already
/// named, plus the three QSL-derived model-schema domains QC-2 adds
/// (`ModelEffectiveDeclarationV1`, `ModelEffectiveViewV1`,
/// `ModelObjectUniverseV1`). Exhaustive: a domain FR-201 does not list is not
/// a variant, and reading one refuses ([`InvalidDigestRecord::UnknownDomain`])
/// rather than silently admitting it.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DigestDomain {
    /// `ir-canonical`.
    IrCanonical,
    /// `ir-bound`.
    IrBound,
    /// `ir-semantic`.
    IrSemantic,
    /// `package-fingerprint`.
    PackageFingerprint,
    /// `lock-file-digest`.
    LockFileDigest,
    /// `raw-artifact-digest`.
    RawArtifactDigest,
    /// `sha256-jcs`.
    Sha256Jcs,
    /// `quire.package.semantic/v2`.
    PackageSemanticV2,
    /// `quire.source.bytes/v1`.
    SourceBytesV1,
    /// `quire.definition.bytes/v1`.
    DefinitionBytesV1,
    /// `quire.checked-semantic-node/v1`.
    CheckedSemanticNodeV1,
    /// `quire.contract-ir.semantic/v1`.
    ContractIrSemanticV1,
    /// `quire.diagnostic-catalog.bytes/v1`.
    DiagnosticCatalogBytesV1,
    /// `quire.generated-artifact.bytes/v1`.
    GeneratedArtifactBytesV1,
    /// `quire.tool-manifest.jcs/v1`.
    ToolManifestJcsV1,
    /// `quire.simulation.state-key/v1`.
    SimulationStateKeyV1,
    /// `typed-expression-identity`.
    TypedExpressionIdentity,
    /// `quire.verification.jcs`.
    VerificationJcs,
    /// `quire.model.effective-declaration/v1` (QC-2; O-05's `EffectiveId`).
    ModelEffectiveDeclarationV1,
    /// `quire.model.effective-view/v1` (QC-2).
    ModelEffectiveViewV1,
    /// `quire.model.object-universe/v1` (QC-2).
    ModelObjectUniverseV1,
}

impl DigestDomain {
    /// Every closed variant, in FR-201's own table order, for exhaustive
    /// round-trip tests.
    pub const ALL: [Self; 21] = [
        Self::IrCanonical,
        Self::IrBound,
        Self::IrSemantic,
        Self::PackageFingerprint,
        Self::LockFileDigest,
        Self::RawArtifactDigest,
        Self::Sha256Jcs,
        Self::PackageSemanticV2,
        Self::SourceBytesV1,
        Self::DefinitionBytesV1,
        Self::CheckedSemanticNodeV1,
        Self::ContractIrSemanticV1,
        Self::DiagnosticCatalogBytesV1,
        Self::GeneratedArtifactBytesV1,
        Self::ToolManifestJcsV1,
        Self::SimulationStateKeyV1,
        Self::TypedExpressionIdentity,
        Self::VerificationJcs,
        Self::ModelEffectiveDeclarationV1,
        Self::ModelEffectiveViewV1,
        Self::ModelObjectUniverseV1,
    ];

    /// The exact FR-201 label string. Never case-folded or aliased
    /// (FR-201's own "Key" rule: "a hyphenated and an underscored spelling
    /// are different labels").
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::IrCanonical => "ir-canonical",
            Self::IrBound => "ir-bound",
            Self::IrSemantic => "ir-semantic",
            Self::PackageFingerprint => "package-fingerprint",
            Self::LockFileDigest => "lock-file-digest",
            Self::RawArtifactDigest => "raw-artifact-digest",
            Self::Sha256Jcs => "sha256-jcs",
            Self::PackageSemanticV2 => "quire.package.semantic/v2",
            Self::SourceBytesV1 => "quire.source.bytes/v1",
            Self::DefinitionBytesV1 => "quire.definition.bytes/v1",
            Self::CheckedSemanticNodeV1 => "quire.checked-semantic-node/v1",
            Self::ContractIrSemanticV1 => "quire.contract-ir.semantic/v1",
            Self::DiagnosticCatalogBytesV1 => "quire.diagnostic-catalog.bytes/v1",
            Self::GeneratedArtifactBytesV1 => "quire.generated-artifact.bytes/v1",
            Self::ToolManifestJcsV1 => "quire.tool-manifest.jcs/v1",
            Self::SimulationStateKeyV1 => "quire.simulation.state-key/v1",
            Self::TypedExpressionIdentity => "typed-expression-identity",
            Self::VerificationJcs => "quire.verification.jcs",
            Self::ModelEffectiveDeclarationV1 => "quire.model.effective-declaration/v1",
            Self::ModelEffectiveViewV1 => "quire.model.effective-view/v1",
            Self::ModelObjectUniverseV1 => "quire.model.object-universe/v1",
        }
    }
}

impl DigestDomain {
    /// Whether this domain digests exactly the raw bytes it addresses, with
    /// no RFC 8785/JCS canonicalization or structural preimage in between.
    /// A byte-provision entry (ADR-013 O-26/QC-1: "every source, definition
    /// and domain-package input") is admitted only under one of these
    /// domains: the byte-provision integrity check (FR-071-AC-6) is exactly
    /// "the entry's raw bytes hash to its own declared digest", which is
    /// only ever true for a domain defined that way. A `sha256-jcs` or
    /// `.../jcs/v1` domain digests a canonicalized form this repo has no
    /// canonicalizer for; a `quire.package.semantic/v2` or
    /// `quire.checked-semantic-node/v1` domain digests a structural
    /// preimage, not arbitrary bytes. Admitting one of those here would
    /// make every entry under it fail the raw-bytes check with a spurious
    /// `byte-digest-mismatch`, naming a staleness that does not exist for a
    /// domain the entry was never eligible to declare in the first place.
    pub(crate) fn is_raw_byte_addressed(&self) -> bool {
        matches!(
            self,
            Self::SourceBytesV1
                | Self::DefinitionBytesV1
                | Self::RawArtifactDigest
                | Self::DiagnosticCatalogBytesV1
                | Self::GeneratedArtifactBytesV1
        )
    }
}

impl fmt::Display for DigestDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DigestDomain {
    type Err = UnknownDigestDomain;

    /// Exact lexical match against FR-201's label vocabulary (FR-201's own
    /// "Key" rule): no case folding, no alias, and an unrecognized label
    /// refuses rather than being treated as absent or defaulted
    /// (FR-201-AC-3 is about a wire member that is missing outright, kept
    /// distinct at [`DigestRecord::from_wire`]'s `Option` parameter, not
    /// conflated with an unknown label here).
    fn from_str(label: &str) -> Result<Self, Self::Err> {
        DigestDomain::ALL
            .into_iter()
            .find(|domain| domain.as_str() == label)
            .ok_or_else(|| UnknownDigestDomain(label.to_owned()))
    }
}

/// [`DigestDomain::from_str`] read a label FR-201 does not define.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnknownDigestDomain(String);

impl fmt::Display for UnknownDigestDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} is not an FR-201 digest domain", self.0)
    }
}
impl std::error::Error for UnknownDigestDomain {}

/// One domain-labelled digest (ADR-013 O-18): a [`DigestDomain`] and its
/// 32-byte SHA-256 digest. The algorithm is a fixed invariant, not a stored
/// field or a wire member -- every FR-201 domain is SHA-256 -- so there is
/// no second algorithm value this type could ever hold.
///
/// Equality and ordering are `derive`d over `(domain, bytes)` in that field
/// order, giving FR-201's own comparison rule directly: two records compare
/// by domain first, so equal bytes under different domains are unequal
/// (FR-201-AC-2), with no cross-domain conversion possible because nothing
/// here strips or substitutes the domain.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DigestRecord {
    domain: DigestDomain,
    bytes: [u8; 32],
}

impl DigestRecord {
    /// Wrap an already-computed digest under `domain`, with no re-hash:
    /// the caller's own minting function computed `bytes` over that
    /// domain's own preimage. Mirrors the kernel identity newtypes'
    /// `from_digest` (`quire-exact::identity`), which this record generalizes
    /// to every FR-201 domain rather than one opaque type per domain.
    pub fn mint(domain: DigestDomain, bytes: [u8; 32]) -> Self {
        Self { domain, bytes }
    }

    /// The record's domain.
    pub fn domain(&self) -> DigestDomain {
        self.domain
    }

    /// The raw 32-byte digest.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    /// The unprefixed 64-lowercase-hex digest, FR-322's own digest-member
    /// spelling (domain travels as that member's sibling field, never
    /// concatenated into this string).
    pub fn hex(&self) -> String {
        self.bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    /// Read a wire digest member: an explicit domain label and an exact
    /// 64-lowercase-hex digest (C-16). `domain` is `None` for a wire member
    /// that supplies no domain at all (FR-201-AC-3's absent domain, kept
    /// distinct from an unrecognized label, which is
    /// [`InvalidDigestRecord::UnknownDomain`]).
    pub fn from_wire(domain: Option<&str>, digest_hex: &str) -> Result<Self, InvalidDigestRecord> {
        let Some(domain) = domain else {
            return Err(InvalidDigestRecord::AbsentDomain);
        };
        let domain = domain
            .parse::<DigestDomain>()
            .map_err(InvalidDigestRecord::UnknownDomain)?;
        if digest_hex.len() != 64 {
            return Err(InvalidDigestRecord::WrongLength(digest_hex.len()));
        }
        let (pairs, []) = digest_hex.as_bytes().as_chunks::<2>() else {
            return Err(InvalidDigestRecord::NotLowerHex);
        };
        let mut bytes = [0_u8; 32];
        for (slot, &[high, low]) in bytes.iter_mut().zip(pairs) {
            let (Some(high), Some(low)) = (lower_hex_nibble(high), lower_hex_nibble(low)) else {
                return Err(InvalidDigestRecord::NotLowerHex);
            };
            *slot = (high << 4) | low;
        }
        Ok(Self { domain, bytes })
    }
}

/// A single lowercase-hex ASCII digit's value, or `None` for anything else
/// (including an uppercase hex digit -- FR-201's exact-match rule admits no
/// case folding). Mirrors `crate::value::node`'s own `lower_hex`.
fn lower_hex_nibble(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        _ => None,
    }
}

impl fmt::Debug for DigestRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DigestRecord({}, {})", self.domain, self.hex())
    }
}

/// [`DigestRecord::from_wire`]'s refusal (ADR-013 O-18 C-16): the wire
/// member is malformed, or the domain refuses outright (FR-201-AC-3).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum InvalidDigestRecord {
    /// The wire member supplied no domain at all (FR-201-AC-3): "a digest
    /// supplied without a selected domain has an absent domain... it is
    /// never defaulted to `raw-artifact-digest` or to any other member".
    #[error("digest member names no domain; FR-201 admits no default")]
    AbsentDomain,
    /// The wire member named a domain label FR-201 does not define.
    #[error("{0}")]
    UnknownDomain(#[source] UnknownDigestDomain),
    /// The digest string is not exactly 64 characters (a prefixed form such
    /// as `ByteDigest`'s own `sha256:`-prefixed spelling is one instance of
    /// this: its 71-character text never parses as FR-322's unprefixed
    /// digest member).
    #[error("digest is {0} characters, not exactly 64")]
    WrongLength(usize),
    /// The digest string contains a character outside `[0-9a-f]` -- an
    /// uppercase-hex digest is exactly this case: FR-201's exact-match rule
    /// (no case folding) means `"AB..."` is not the same label-and-bytes
    /// pair as `"ab..."`.
    #[error("digest is not exactly 64 lowercase hexadecimal digits")]
    NotLowerHex,
}

impl InvalidDigestRecord {
    /// Whether this refusal is genuinely about the digest's *domain*
    /// (absent, or naming a label FR-201 does not define) as opposed to the
    /// digest string's own encoding (wrong length, not lowercase hex). A
    /// caller rendering a catalog cause for this refusal must not spell a
    /// malformed-encoding case as `digest-domain-mismatch`: the domain named
    /// no problem at all in that case.
    pub(crate) fn is_domain_mismatch(&self) -> bool {
        matches!(self, Self::AbsentDomain | Self::UnknownDomain(_))
    }
}

/// A node id exactly as it travels on the wire (a v2 node key's 64
/// lowercase-hex digest), before a checked-package lookup resolves it to a
/// `quire_exact::NodeKey` (ADR-013 O-04). Relocated here (QSL-158 S-3a) from
/// `replay::identity`, whose own module doc named this as a provisional,
/// not-yet-canonical home: ADR-011 `:588` places the wire node id in the `F`
/// foundation layer, alongside `digest` and before `wire_format`, not in the
/// layer-6 `replay` facade -- `replay` (layer 6) importing `digest` (layer
/// F) is the permitted direction; `package` (layer 4) or `library`
/// (layer 3) importing `replay` would not be, which is exactly why
/// `package::PackageNodeKey` (ADR-013 T-3) needs this type here rather than
/// in `replay`. A wire-read node id stays a `WireNodeId`, never a
/// `NodeKey`, until a lookup in an already-checked package resolves it, at
/// E4 (the dependency's own checked package) or E9 (`replay`'s recompiled
/// package) -- never by a conversion function, and none is defined here.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WireNodeId([u8; 32]);

impl WireNodeId {
    /// Wrap an already-known wire node-id digest. Unlike
    /// `quire_exact::NodeKey::from_digest`, this constructor carries no
    /// "only `check` calls this" restriction: a `WireNodeId` is exactly the
    /// unchecked wire spelling, never a claim that the id resolves to a
    /// real node.
    pub fn from_digest(digest: [u8; 32]) -> Self {
        Self(digest)
    }

    /// The raw digest bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for WireNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|byte| write!(f, "{byte:02x}"))
    }
}

impl fmt::Debug for WireNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WireNodeId({self})")
    }
}

#[cfg(test)]
mod digest_record_tests {
    use super::*;

    fn bytes(fill: u8) -> [u8; 32] {
        [fill; 32]
    }

    /// (#213 S-2, O-18) every FR-201 domain round-trips through
    /// `from_wire`/`hex`/`domain` with no loss. The label table below is
    /// hand-written against FR-201's own text, independent of
    /// `DigestDomain::as_str()`'s own match arms: a wrong label there (e.g.
    /// `ModelEffectiveDeclarationV1` silently becoming `.../v2`) fails the
    /// first assertion in the loop rather than round-tripping only against
    /// itself (#260 review item 4). Both `[(...); 21]` and `DigestDomain::
    /// ALL: [Self; 21]` are fixed-length arrays, so a 21st-to-22nd-entry
    /// mismatch between them is a compile error, not something this test
    /// could observe at runtime -- the per-entry label and round-trip
    /// assertions below are this test's real teeth (#260 review round 2).
    #[test]
    fn every_fr201_domain_round_trips_through_the_wire() {
        let labels: [(DigestDomain, &str); 21] = [
            (DigestDomain::IrCanonical, "ir-canonical"),
            (DigestDomain::IrBound, "ir-bound"),
            (DigestDomain::IrSemantic, "ir-semantic"),
            (DigestDomain::PackageFingerprint, "package-fingerprint"),
            (DigestDomain::LockFileDigest, "lock-file-digest"),
            (DigestDomain::RawArtifactDigest, "raw-artifact-digest"),
            (DigestDomain::Sha256Jcs, "sha256-jcs"),
            (DigestDomain::PackageSemanticV2, "quire.package.semantic/v2"),
            (DigestDomain::SourceBytesV1, "quire.source.bytes/v1"),
            (DigestDomain::DefinitionBytesV1, "quire.definition.bytes/v1"),
            (
                DigestDomain::CheckedSemanticNodeV1,
                "quire.checked-semantic-node/v1",
            ),
            (
                DigestDomain::ContractIrSemanticV1,
                "quire.contract-ir.semantic/v1",
            ),
            (
                DigestDomain::DiagnosticCatalogBytesV1,
                "quire.diagnostic-catalog.bytes/v1",
            ),
            (
                DigestDomain::GeneratedArtifactBytesV1,
                "quire.generated-artifact.bytes/v1",
            ),
            (
                DigestDomain::ToolManifestJcsV1,
                "quire.tool-manifest.jcs/v1",
            ),
            (
                DigestDomain::SimulationStateKeyV1,
                "quire.simulation.state-key/v1",
            ),
            (
                DigestDomain::TypedExpressionIdentity,
                "typed-expression-identity",
            ),
            (DigestDomain::VerificationJcs, "quire.verification.jcs"),
            (
                DigestDomain::ModelEffectiveDeclarationV1,
                "quire.model.effective-declaration/v1",
            ),
            (
                DigestDomain::ModelEffectiveViewV1,
                "quire.model.effective-view/v1",
            ),
            (
                DigestDomain::ModelObjectUniverseV1,
                "quire.model.object-universe/v1",
            ),
        ];
        assert_eq!(DigestDomain::ALL.len(), 21);
        assert_eq!(labels.len(), 21);
        for (domain, expected_label) in labels {
            assert_eq!(domain.as_str(), expected_label, "{domain:?}'s FR-201 label");
            let record = DigestRecord::mint(domain, bytes(0xab));
            let round_tripped = DigestRecord::from_wire(Some(expected_label), &record.hex())
                .unwrap_or_else(|err| panic!("{domain} round trip: {err}"));
            assert_eq!(round_tripped, record);
            assert_eq!(round_tripped.domain(), domain);
        }
    }

    /// C-16 adverse case: uppercase hex refuses rather than being
    /// case-folded to a match.
    #[test]
    fn refuses_uppercase_hex() {
        let record = DigestRecord::mint(DigestDomain::Sha256Jcs, bytes(0xab));
        let uppercase = record.hex().to_uppercase();
        assert_eq!(
            DigestRecord::from_wire(Some(DigestDomain::Sha256Jcs.as_str()), &uppercase),
            Err(InvalidDigestRecord::NotLowerHex)
        );
    }

    /// C-16 adverse case: a digest of the wrong length (short or long)
    /// refuses rather than being padded or truncated.
    #[test]
    fn refuses_the_wrong_length() {
        let short = "ab".repeat(31);
        assert_eq!(
            DigestRecord::from_wire(Some(DigestDomain::Sha256Jcs.as_str()), &short),
            Err(InvalidDigestRecord::WrongLength(62))
        );
        let long = "ab".repeat(33);
        assert_eq!(
            DigestRecord::from_wire(Some(DigestDomain::Sha256Jcs.as_str()), &long),
            Err(InvalidDigestRecord::WrongLength(66))
        );
    }

    /// C-16 adverse case: `ByteDigest`'s own `sha256:`-prefixed spelling
    /// (FR-001/004) is not FR-322's unprefixed digest-member spelling, and
    /// refuses here on length alone -- the two conventions never merge.
    #[test]
    fn refuses_a_prefixed_form() {
        let prefixed = ByteDigest::of(b"probe").to_string();
        assert_eq!(prefixed.len(), 71, "sanity: \"sha256:\" plus 64 hex digits");
        assert_eq!(
            DigestRecord::from_wire(Some(DigestDomain::RawArtifactDigest.as_str()), &prefixed),
            Err(InvalidDigestRecord::WrongLength(71))
        );
    }

    /// C-16 adverse case, FR-201-AC-3: a digest wire member that names no
    /// domain at all refuses, and is never defaulted to
    /// `raw-artifact-digest` or any other domain.
    #[test]
    fn refuses_an_absent_domain() {
        assert_eq!(
            DigestRecord::from_wire(None, &"ab".repeat(32)),
            Err(InvalidDigestRecord::AbsentDomain)
        );
    }

    /// A domain label FR-201 does not define refuses distinctly from an
    /// absent domain.
    #[test]
    fn refuses_an_unknown_domain_label() {
        let err = DigestRecord::from_wire(Some("quire.not-a-real-domain/v1"), &"ab".repeat(32))
            .expect_err("an unrecognized label never admits");
        assert!(matches!(err, InvalidDigestRecord::UnknownDomain(_)));
    }

    /// C-16 adverse case, FR-201-AC-2: equal digest bytes under two
    /// different domains are unequal records -- domain is part of
    /// identity, not decoration, and no conversion collapses the two.
    #[test]
    fn a_cross_domain_digest_is_unequal_despite_equal_bytes() {
        let same_bytes = bytes(0x42);
        let node = DigestRecord::mint(DigestDomain::CheckedSemanticNodeV1, same_bytes);
        let effective = DigestRecord::mint(DigestDomain::ModelEffectiveDeclarationV1, same_bytes);
        assert_ne!(node, effective);
        assert_eq!(
            node.as_bytes(),
            effective.as_bytes(),
            "sanity: bytes really do match"
        );
    }

    /// `DigestDomain::from_str`'s exact-match rule: no case folding and no
    /// hyphen/underscore aliasing (FR-201's own "Key" rule).
    #[test]
    fn domain_parsing_is_exact_no_case_folding_or_aliasing() {
        assert!("SHA256-JCS".parse::<DigestDomain>().is_err());
        assert!("sha256_jcs".parse::<DigestDomain>().is_err());
        assert_eq!(
            "sha256-jcs".parse::<DigestDomain>(),
            Ok(DigestDomain::Sha256Jcs)
        );
    }
}
