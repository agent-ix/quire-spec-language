// SPDX-License-Identifier: AGPL-3.0-or-later
//! Provenance and identity carriers the four #231 envelopes need (ADR-013
//! O-07, O-09, O-11, O-19), scoped to the layer-6 `replay` module.
//!
//! Where each type's canonical home is:
//!
//! - [`QualifiedName`] (O-11) is #213 S-3's. PR #262 (#214, open, unmerged)
//!   independently adds an equivalent `QualifiedName` to
//!   `value::expression`, which this change must not touch (M-5 is
//!   splitting that module concurrently) and cannot depend on before it
//!   merges. This module's copy is #231's own, scoped to `replay` only.
//! - O-07's occurrence key and O-12's source region have their canonical
//!   home in `qsl_foundation::source::provenance` (#213 S-4);
//!   this crate carries them from there.
//! - [`ObligationIdentity`] (O-09) is canonical here (ADR-013 OQ-H): CG
//!   names this type directly; IR names no QSL type. CG computes the digest
//!   of a clause, application or function contract (AD-016 arrow 5) and QSL
//!   carries it; QSL itself mints the digest of a parity claim, a
//!   process-provider item and a function-value parity claim (FR-484).
//!
//! [`QualifiedName`] moves to its ADR-013-assigned home once that ticket
//! lands; until then it exists here, once, so the four envelope types have a
//! typed (never string-keyed, never bare-`&str`) vocabulary to carry.
use std::fmt;
use quire_exact::NodeKey;

use qsl_foundation::bound::{DomainKey, FiniteBound, ProofBound};
use quire_exact::Identifier;

/// ADR-013 O-09: the digest identifying one obligation. For a clause, an
/// application or a function contract CG mints it, and QSL treats it as
/// opaque and carries it unchanged; the preimage is the RFC 8785 encoding, through ADR-013 §2's one encoder, of
/// the clause node id, the clause occurrence key, the obligation kind and the
/// arguments (each a parameter node id and its declared domain); the source
/// span is excluded (QC-14). The occurrence key is part of the preimage, so
/// two occurrences of one clause never collide. QSL mints the identity of a
/// parity claim, a process-provider item and a function-value parity claim
/// (FR-484, [`crate::value_parity_obligation`], which `call_site` returns in
/// `FunctionSite::value_parity`); this type only wraps and compares the
/// digest. Its digest domain is not in the closed FR-201 set (QC-4), so it
/// is not a `qsl_foundation::digest::DigestRecord`.
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
/// revision namespace, revision)` -- FR-001's four source labels. Named once
/// here and reused by every envelope's wire struct
/// (`replay::request::ReplayRequestWire`) that carries a list of these, and
/// read and written only by [`RawSourceRef::from_wire`] and
/// [`RawSourceRef::to_wire`].
pub type SourceRefWire = (String, String, String, String);

/// One entry of a replay request's `sources` list: FR-001's four labels
/// (authority, identity, revision namespace, revision). The replay executor
/// recompiles a source under exactly these labels (ADR-013 §7 S-4b: "The
/// replay executor's recompilation under the reference's labels (FR-001) is
/// TK-01's"). It is not the O-07
/// `qsl_foundation::source::provenance::RawSourceRef`, which names a source
/// by authority and identity alone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawSourceRef {
    authority: String,
    identity: String,
    revision_namespace: String,
    revision: String,
}

impl RawSourceRef {
    /// Name a source document by its four labels.
    pub fn new(
        authority: String,
        identity: String,
        revision_namespace: String,
        revision: String,
    ) -> Self {
        Self {
            authority,
            identity,
            revision_namespace,
            revision,
        }
    }

    /// Read one wire entry. The labels are carried as given, and a blank one
    /// refuses where the source is admitted.
    pub fn from_wire(wire: SourceRefWire) -> Self {
        let (authority, identity, revision_namespace, revision) = wire;
        Self::new(authority, identity, revision_namespace, revision)
    }

    /// This reference as one wire entry, the inverse of [`Self::from_wire`].
    pub fn to_wire(&self) -> SourceRefWire {
        (
            self.authority.clone(),
            self.identity.clone(),
            self.revision_namespace.clone(),
            self.revision.clone(),
        )
    }

    /// A wire entry's variable-length bytes: its four labels, as the #231
    /// readers measure them against the reader bound.
    pub(crate) fn wire_len(wire: &SourceRefWire) -> usize {
        let (authority, identity, revision_namespace, revision) = wire;
        authority.len() + identity.len() + revision_namespace.len() + revision.len()
    }

    /// The source's authority (e.g. a registry or repository host).
    pub fn authority(&self) -> &str {
        &self.authority
    }

    /// The source's identity within that authority.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// The revision system the revision belongs to, such as `git`.
    pub fn revision_namespace(&self) -> &str {
        &self.revision_namespace
    }

    /// The selected revision of that identity, within its namespace.
    pub fn revision(&self) -> &str {
        &self.revision
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
/// its full [`DomainKey`] (a node and the path into its type, or a
/// population), so two bounds on one parameter at different paths, or on a
/// field and a population, stay distinct. It is a
/// typed `FiniteBound`, so an empty or inverted domain cannot be carried, and
/// no accounting limit, stage limit or profile ceiling can stand in for it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclaredDomain(ProofBound);

impl DeclaredDomain {
    /// The declared domain `bound`. Every input type is re-exported at this
    /// crate's root, so a client builds one through `qsl_replay` alone:
    ///
    /// ```
    /// use qsl_replay::{
    ///     DeclaredDomain, DomainKey, DomainKind, FiniteBound, Integer, ProofBound, NodeKey,
    /// };
    ///
    /// let parameter = crate::fixture_node(7);
    /// let bound = FiniteBound::integer_range(Integer::from(0_i64), Integer::from(9_i64))?;
    /// let domain = DomainKey::Node { node: parameter, path: Vec::new() };
    /// let declared = DeclaredDomain::new(ProofBound::new(domain, Some(DomainKind::Integer), bound)?);
    /// assert_eq!(declared.domain(), &DomainKey::Node { node: parameter, path: Vec::new() });
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(bound: ProofBound) -> Self {
        Self(bound)
    }

    /// The domain key: what the domain is a domain of.
    pub fn domain(&self) -> &DomainKey {
        self.0.domain()
    }

    /// The declared finite domain.
    pub fn bound(&self) -> &FiniteBound {
        self.0.bound()
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
