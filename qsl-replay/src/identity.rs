// SPDX-License-Identifier: AGPL-3.0-or-later
//! Provenance and identity carriers the four #231 envelopes need (ADR-013
//! O-07, O-09, O-11, O-19), scoped to the layer-6 `replay` module.
//!
//! **Provisional, not canonical.** ADR-013 assigns the long-term home of
//! several of these types to tickets that have not landed on `origin/main`
//! as of this change:
//!
//! - [`QualifiedName`] (O-11) is #213 S-3's. PR #262 (#214, open, unmerged)
//!   independently adds an equivalent `QualifiedName` to
//!   `value::expression`, which this change must not touch (M-5 is
//!   splitting that module concurrently) and cannot depend on before it
//!   merges. This module's copy is #231's own, scoped to `replay` only.
//! - O-07's occurrence key and O-12's source region have their canonical
//!   home in `qsl_foundation::source::provenance` (#213 S-4, QSL-159);
//!   this crate carries them from there.
//! - [`ObligationIdentity`] (O-09) is CG-computed (AD-016 arrow 5, no QSL
//!   ticket); QSL only ever carries the digest CG mints, never hashes one
//!   itself, mirroring `quire_exact`'s own opaque digest identities.
//!
//! Each of these should be consolidated into its ADR-013-assigned canonical
//! home once that ticket lands; until then they exist here, once, so the
//! four envelope types have a typed (never string-keyed, never bare-`&str`)
//! vocabulary to carry.
use std::fmt;

use qsl_foundation::bound::{DomainKey, FiniteBound, ProofBound};
use qsl_foundation::digest::{DigestRecord, ManifestDigest, WireNodeId};
use quire_exact::Identifier;

/// ADR-013 O-09: the CG-computed digest identifying one Kani obligation --
/// the digest over every `KaniObligationIdentity` member except
/// `source_span` (QC-14). QSL never mints this digest; CG does (AD-016
/// arrow 5), so this type only wraps and compares it, performing no hashing
/// and holding no preimage knowledge -- the same shape as
/// `quire_exact`'s QC-15 opaque identities. Its digest domain is not in the
/// closed FR-201 set (QC-4), so it is not a `qsl_foundation::digest::DigestRecord`.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObligationIdentity([u8; 32]);

impl ObligationIdentity {
    /// Wrap an already-computed obligation-identity digest (CG's, not
    /// QSL's own hash).
    pub fn from_digest(digest: [u8; 32]) -> Self {
        Self(digest)
    }

    /// The raw digest bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for ObligationIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|byte| write!(f, "{byte:02x}"))
    }
}

impl fmt::Debug for ObligationIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ObligationIdentity({self})")
    }
}

/// ADR-013 O-11: a qualified name -- a non-empty sequence of identifiers --
/// naming the function a replay selects (OQ-5 ruling). Deliberately carries
/// no `From<&str>`, `From<String>` or `FromStr` impl anywhere on this type:
/// FR-071-AC-3 requires that no public constructor or decoder accept a bare
/// string in the function-selection position, and that no implicit
/// string-to-`QualifiedName` conversion exist.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct QualifiedName(Vec<Identifier>);

/// [`QualifiedName::new`] was given zero segments.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("a qualified name needs at least one identifier segment")]
pub struct EmptyQualifiedName;

impl QualifiedName {
    /// A qualified name from its typed identifier segments. Refuses an
    /// empty sequence; a qualified name is never zero segments.
    pub fn new(segments: Vec<Identifier>) -> Result<Self, EmptyQualifiedName> {
        if segments.is_empty() {
            Err(EmptyQualifiedName)
        } else {
            Ok(Self(segments))
        }
    }

    /// The name's identifier segments, in declared order.
    pub fn segments(&self) -> &[Identifier] {
        &self.0
    }
}

impl fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut segments = self.0.iter();
        if let Some(first) = segments.next() {
            write!(f, "{}", first.as_str())?;
        }
        for segment in segments {
            write!(f, ".{}", segment.as_str())?;
        }
        Ok(())
    }
}

/// The wire shape a [`RawSourceRef`] decodes from: `(authority, identity,
/// revision, digest domain, digest hex)`. Named once here and reused by
/// every envelope's wire struct (`replay::witness::WitnessPacket`,
/// `replay::request::ReplayRequestWire`) that carries a list of these, both
/// to avoid clippy's `type_complexity` lint on the bare nested tuple and so
/// the shape is stated in one place rather than repeated per call site.
pub type SourceDigestWire = (String, String, String, Option<String>, String);

/// One entry of an envelope's `source_digests` list (QSpec FR-323: "the
/// `RawSourceRef` digest of every source and definition document a replay
/// recompiles") -- authority, identity, revision and digest. It admits any
/// FR-201 domain and a one-string revision, so it is not the O-07
/// `qsl_foundation::source::provenance::RawSourceRef`, whose digest is
/// `quire.source.bytes/v1` only and whose revision is QSpec's
/// `{namespace, value}`. FR-323 calls a definition document's digest a
/// `RawSourceRef` digest while QSpec's `RawSourceRef` schema admits only
/// `quire.source.bytes/v1`; the two types stay apart until QSpec settles
/// that.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawSourceRef {
    authority: String,
    identity: String,
    revision: String,
    digest: DigestRecord,
}

impl RawSourceRef {
    /// Name a source document by authority, identity, revision and digest.
    pub fn new(
        authority: String,
        identity: String,
        revision: String,
        digest: DigestRecord,
    ) -> Self {
        Self {
            authority,
            identity,
            revision,
            digest,
        }
    }

    /// The source's authority (e.g. a registry or repository host).
    pub fn authority(&self) -> &str {
        &self.authority
    }

    /// The source's identity within that authority.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// The selected revision of that identity.
    pub fn revision(&self) -> &str {
        &self.revision
    }

    /// The `quire.source.bytes/v1` digest of the source's bytes.
    pub fn digest(&self) -> DigestRecord {
        self.digest
    }
}

/// ADR-013 O-19: the `backend` member every proof-result envelope, witness
/// envelope and replay request carries unchanged (QC-8) -- the provider
/// identity exactly as the FR-331 manifest states it, plus the digest of
/// that manifest (domain `quire.tool-manifest.jcs/v1`, [`ManifestDigest`]),
/// which also pins the tool. Two backend identities are equal iff both
/// fields are equal.
///
/// QSL-227: the manifest digest is typed [`ManifestDigest`] (shared with
/// `qsl_route::Candidate`, moved to `qsl_foundation` for exactly this
/// reason), not a domain-agnostic `DigestRecord` -- a `Backend` cannot be
/// built at all with a digest in any other FR-201 domain. `Backend` keeps
/// its own struct, distinct from `Candidate`, because its `identity` field
/// is a plain wire-carried `String` (this envelope only round-trips it,
/// FR-070-AC-3/FR-071-AC-1) where `Candidate`'s `BackendId` additionally
/// serves as an ordered, hashable `Registry` key (FR-290) -- a role this
/// layer-6 facade has no registry to key. Layer 6 (`replay`) cannot depend
/// on layer R (`route`) either way (ADR-011 §6.1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Backend {
    identity: String,
    manifest_digest: ManifestDigest,
}

impl Backend {
    /// Name a backend by its provider identity and manifest digest.
    pub fn new(identity: String, manifest_digest: ManifestDigest) -> Self {
        Self {
            identity,
            manifest_digest,
        }
    }

    /// The provider identity string, exactly as the FR-331 manifest states it.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// The manifest's digest (always domain `quire.tool-manifest.jcs/v1`),
    /// which also pins the tool.
    pub fn manifest_digest(&self) -> ManifestDigest {
        self.manifest_digest
    }
}

/// One `v2` lock semantic-profile selection: a profile identifier and its
/// selected value, exactly as the lock records it. Opaque to #231 -- no
/// consumer here inspects an individual selection; they only round-trip
/// exactly (FR-070-AC-3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileSelection {
    profile: String,
    value: String,
}

impl ProfileSelection {
    /// Name a profile selection by its profile identifier and selected value.
    pub fn new(profile: String, value: String) -> Self {
        Self { profile, value }
    }

    /// The profile identifier.
    pub fn profile(&self) -> &str {
        &self.profile
    }

    /// The selected value.
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// One declared finite domain of the proving run (ADR-013 O-09; ADR-014
/// B-4, §11): the [`ProofBound`] the bounded request substituted, keyed by
/// its full [`DomainKey`] (the parameter node, then the path into its type),
/// so two bounds on one parameter at different paths stay distinct. It is a
/// typed `FiniteBound`, so an empty or inverted domain cannot be carried, and
/// no accounting limit, stage limit or profile ceiling can stand in for it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclaredDomain(ProofBound);

impl DeclaredDomain {
    /// The declared domain `bound`.
    pub fn new(bound: ProofBound) -> Self {
        Self(bound)
    }

    /// The parameter node this domain is declared in.
    pub fn parameter(&self) -> WireNodeId {
        self.0.domain.node()
    }

    /// The domain key: the parameter node and the path into its type.
    pub fn domain(&self) -> &DomainKey {
        &self.0.domain
    }

    /// The declared finite domain.
    pub fn bound(&self) -> &FiniteBound {
        &self.0.bound
    }

    /// The whole proof bound.
    pub fn proof_bound(&self) -> &ProofBound {
        &self.0
    }
}

/// A resolved position within a family's own trace (ADR-013 O-25: "the
/// trace position where the family has one"). Opaque to #231: only the
/// owning family evaluator interprets it; the common envelope only stores
/// and round-trips it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TracePosition(String);

impl TracePosition {
    /// Wrap an already-resolved trace-position descriptor.
    pub fn new(descriptor: String) -> Self {
        Self(descriptor)
    }

    /// The trace position's descriptor.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> [u8; 32] {
        let mut bytes = [0_u8; 32];
        bytes[31] = byte;
        bytes
    }

    #[test]
    fn qualified_name_refuses_zero_segments() {
        assert_eq!(QualifiedName::new(Vec::new()), Err(EmptyQualifiedName));
    }

    #[test]
    fn qualified_name_displays_dot_joined_segments() {
        let name = QualifiedName::new(vec![
            Identifier::new("a").unwrap(),
            Identifier::new("b").unwrap(),
        ])
        .unwrap();
        assert_eq!(name.to_string(), "a.b");
        assert_eq!(name.segments().len(), 2);
    }

    #[test]
    fn obligation_identity_equality_is_digest_bytes() {
        let a = ObligationIdentity::from_digest(digest(7));
        let b = ObligationIdentity::from_digest(digest(7));
        let c = ObligationIdentity::from_digest(digest(8));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
