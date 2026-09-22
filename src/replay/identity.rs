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
//! - [`OccurrenceKey`] (O-07) has no landed canonical type either; only the
//!   *checked* domain sibling (`quire_exact::Location`) exists.
//!   `OccurrenceKey` reuses the kernel's own `Role`/`Origin` pair for its
//!   role/ordinal half, since that part carries no checked-only
//!   restriction; its node half is `qsl_foundation::digest::WireNodeId` (O-04,
//!   relocated there by QSL-158 S-3a per ADR-011 `:588`'s `F` foundation
//!   layer placement -- imported directly from `digest` below, not
//!   re-exported back out under this module's own path, so `replay`'s own
//!   layer-6-depends-on-F edge stays the only edge to it).
//! - [`ObligationIdentity`] (O-09) is CG-computed (AD-016 arrow 5, no QSL
//!   ticket); QSL only ever carries the digest CG mints, never hashes one
//!   itself, mirroring `quire_exact`'s own opaque digest identities.
//!
//! Each of these should be consolidated into its ADR-013-assigned canonical
//! home once that ticket lands; until then they exist here, once, so the
//! four envelope types have a typed (never string-keyed, never bare-`&str`)
//! vocabulary to carry.
use std::fmt;

use quire_exact::Origin;

use crate::value::Identifier;
use qsl_foundation::digest::{DigestRecord, WireNodeId};

/// ADR-013 O-07: an occurrence key -- `(node id, role, ordinal)` -- keeping
/// two source occurrences of a structurally identical node apart. Reuses
/// the kernel's own `quire_exact::Origin` (`role`, `ordinal`) for the
/// role/ordinal half; only the node half is wire-level here (see
/// [`WireNodeId`]).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct OccurrenceKey {
    node: WireNodeId,
    origin: Origin,
}

impl OccurrenceKey {
    /// Name an occurrence by wire node id, role and ordinal.
    pub fn new(node: WireNodeId, origin: Origin) -> Self {
        Self { node, origin }
    }

    /// The occurrence's wire node id.
    pub fn node(&self) -> WireNodeId {
        self.node
    }

    /// The occurrence's role and ordinal.
    pub fn origin(&self) -> &Origin {
        &self.origin
    }
}

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

/// ADR-013 O-07: names a source document a proved package or one of its
/// dependencies was generated from -- authority, identity, revision, and
/// its `quire.source.bytes/v1` digest (`crate::digest`).
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
/// that manifest (domain `quire.tool-manifest.jcs/v1`), which also pins the
/// tool. Two backend identities are equal iff both fields are equal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Backend {
    identity: String,
    manifest_digest: DigestRecord,
}

impl Backend {
    /// Name a backend by its provider identity and manifest digest.
    pub fn new(identity: String, manifest_digest: DigestRecord) -> Self {
        Self {
            identity,
            manifest_digest,
        }
    }

    /// The provider identity string, exactly as the FR-331 manifest states it.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// The manifest's digest (domain `quire.tool-manifest.jcs/v1`), which
    /// also pins the tool.
    pub fn manifest_digest(&self) -> DigestRecord {
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

/// One obligation argument's declared per-argument domain (ADR-013 O-09):
/// the parameter's wire node id and its declared-domain descriptor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclaredDomain {
    parameter: WireNodeId,
    domain: String,
}

impl DeclaredDomain {
    /// Name a declared domain by parameter and domain descriptor.
    pub fn new(parameter: WireNodeId, domain: String) -> Self {
        Self { parameter, domain }
    }

    /// The parameter this domain is declared for.
    pub fn parameter(&self) -> WireNodeId {
        self.parameter
    }

    /// The declared-domain descriptor.
    pub fn domain(&self) -> &str {
        &self.domain
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
    fn occurrence_key_same_node_distinct_ordinal_are_unequal() {
        let node = WireNodeId::from_digest(digest(1));
        let a = OccurrenceKey::new(node, Origin::new("reference".into(), 0));
        let b = OccurrenceKey::new(node, Origin::new("reference".into(), 1));
        assert_ne!(a, b);
        assert_eq!(a.node(), b.node());
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
