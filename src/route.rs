// SPDX-License-Identifier: AGPL-3.0-or-later
//! Layer R: the `#185` capability-backend registry and router (ADR-011
//! §6.1 layer R; ADR-012 §6, §7).
//!
//! `route` answers one question -- **which registered backends, if any,
//! advertise the capability kind a requested item needs** -- and nothing
//! else. It computes a candidate set from an explicitly-built [`Registry`]
//! value (ADR-012 §7.1), matching on the canonical
//! [`crate::check::Capability`] kind alone (FR-075): mode
//! (`bounded`/`unbounded`) never filters candidacy, and no local
//! capability-kind type is defined here or anywhere else in this ticket's
//! scope (FR-075-AC-5).
//!
//! `route` negotiates nothing and settles no disposition. An item whose
//! kind no registered backend advertises reports an ordinary, well-formed
//! empty candidate set -- never a refusal, a panic, a hold, or a deferred/
//! pending outcome (FR-076). Every disposition -- `supported`,
//! `requires-bound`, `unsupported` (warned) and `invalid-request` -- is
//! settled downstream by `quire-contract-codegen`'s `negotiate_*`
//! (ADR-012 §7.2, quire-contract-codegen#86); this module's return types
//! carry no such variant.
//!
//! The registry is an ordinary value (FR-075 "The registry is an ordinary
//! value, not ambient state"; FR-075-CON-1): it is built by the
//! orchestrating driver and passed as an argument to every consumer. This
//! module defines no `static`, `OnceLock` or `thread_local!`, and depends
//! on no plugin-discovery crate (`inventory`, `linkme`, `ctor`); FR-080
//! records the gates (`make route-lint`, `cargo deny check bans`) that hold
//! both of those.

use std::collections::{BTreeMap, HashSet};
use std::fmt;

use crate::check::Capability;
use qsl_foundation::digest::ByteDigest;
use qsl_foundation::CatalogCode;

/// A backend's declared identity, unique within one [`Registry`] (FR-290
/// "Candidate set and negotiation": "a backend identity is unique within a
/// registry"). Opaque, typed data: on the wire it is the
/// `backend{identity, manifest_digest}` member's `identity` field
/// (ADR-012 §7.1, ADR-013 O-19), which this module receives already
/// formed and does not itself mint.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct BackendId(String);

impl BackendId {
    /// Build a `BackendId` from its wire identity string. This module
    /// applies no normalization: the identity is opaque data, compared only
    /// for exact equality.
    pub fn new(identity: impl Into<String>) -> Self {
        Self(identity.into())
    }

    /// The identity's exact wire spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BackendId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A backend's pinned tool identity (ADR-012 §7.1).
///
/// This module carries `ToolIdentity` through registration and candidate
/// output unread and uninterpreted (FR-075 Inputs): the probe that checks a
/// routed backend's tool against this pin runs after routing, at run time
/// (ADR-012 §7.4, FR-290 "Tool absence"), outside `route`'s scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolIdentity(String);

impl ToolIdentity {
    /// Build a `ToolIdentity` from its pinned identity string. Opaque data;
    /// this module neither parses nor interprets it.
    pub fn new(identity: impl Into<String>) -> Self {
        Self(identity.into())
    }

    /// The pinned identity's exact spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether a backend advertises a capability kind over a bounded domain
/// only, or over an unbounded domain too (ADR-012 §1.1).
///
/// `route` never compares `Mode` against a requested item's extent: that
/// comparison is `negotiate_*`'s work, after candidates are computed
/// (FR-075 "Candidate computation matches on capability kind alone"; an
/// advertised mode never excludes a registrant from a candidate set).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Mode {
    /// The backend admits only a finite, bounded domain.
    Bounded,
    /// The backend admits an unbounded domain (and, per ADR-012 §1.1, a
    /// bounded one too).
    Unbounded,
}

/// One backend's registration (FR-075 Inputs): its identity, the digest of
/// its own FR-331 provider manifest, its pinned tool, and the
/// `(capability kind, mode)` pairs it advertises.
///
/// `BackendDescriptor`, the candidate set and `Capability` cross repository
/// boundaries as QSpec data, never through a shared Rust crate
/// (ADR-013 T-7).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendDescriptor {
    id: BackendId,
    manifest_digest: ByteDigest,
    tool: ToolIdentity,
    advertises: HashSet<(Capability, Mode)>,
}

impl BackendDescriptor {
    /// Build a descriptor from its FR-331 provider-manifest fields. A
    /// backend supplies one descriptor per registration, so `id` carries
    /// exactly one `manifest_digest` and one `tool` at a time (FR-075
    /// Inputs).
    pub fn new(
        id: BackendId,
        manifest_digest: ByteDigest,
        tool: ToolIdentity,
        advertises: impl IntoIterator<Item = (Capability, Mode)>,
    ) -> Self {
        Self {
            id,
            manifest_digest,
            tool,
            advertises: advertises.into_iter().collect(),
        }
    }

    /// This descriptor's backend identity.
    pub fn id(&self) -> &BackendId {
        &self.id
    }

    /// The digest of this backend's own FR-331 provider manifest.
    pub fn manifest_digest(&self) -> ByteDigest {
        self.manifest_digest
    }

    /// This backend's pinned tool identity, carried unread (see the type's
    /// own doc).
    pub fn tool(&self) -> &ToolIdentity {
        &self.tool
    }

    /// Whether this descriptor advertises `kind`, in any mode -- mode
    /// never filters candidacy (FR-075).
    ///
    /// ADR-012 §5.1 S7's "registry advertisement check" seam: exhaustive by
    /// hand over [`Capability`]'s ten members, with no wildcard arm, so a
    /// variant added to `Capability` fails this match to compile
    /// (FR-080-AC-5) instead of silently comparing unequal. Checked in as an
    /// S7 seam-probe location (`xtask seam_probe::checked_in_locations`,
    /// `same_kind`): `cargo xtask seam-probe` builds this crate under
    /// `RUSTFLAGS=--cfg seam_probe`, which adds a `#[cfg(seam_probe)]` probe
    /// variant to `Capability` (`src/check/capability.rs`), and confirms
    /// this match fails to compile with `E0004` (non-exhaustive) against it.
    fn advertises_kind(&self, kind: Capability) -> bool {
        fn same_kind(a: Capability, b: Capability) -> bool {
            match a {
                Capability::ValueValidity => matches!(b, Capability::ValueValidity),
                Capability::OperationContract => matches!(b, Capability::OperationContract),
                Capability::FiniteReplay => matches!(b, Capability::FiniteReplay),
                Capability::TemporalSatisfaction => matches!(b, Capability::TemporalSatisfaction),
                Capability::GlobalConformance => matches!(b, Capability::GlobalConformance),
                Capability::Monitorability => matches!(b, Capability::Monitorability),
                Capability::LocalProjection => matches!(b, Capability::LocalProjection),
                Capability::Refinement => matches!(b, Capability::Refinement),
                Capability::Realizability => matches!(b, Capability::Realizability),
                Capability::Composition => matches!(b, Capability::Composition),
            }
        }
        self.advertises.iter().any(|(k, _)| same_kind(*k, kind))
    }
}

/// A registration the [`Registry`] refuses outright (ADR-012 §5.2 row 1;
/// FR-075-AC-4): a repeated `BackendId`. Structurally distinct from
/// [`CandidateOutcome`] (FR-076-AC-3, TC-198) -- a caller cannot mistake a
/// registration refusal for an empty candidate set or an unknown-backend
/// marker by pattern-matching alone.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("invalid_capability/duplicate-backend: {identity} is already registered")]
pub struct RegistrationRefusal {
    identity: BackendId,
}

impl RegistrationRefusal {
    /// The identity that was already registered.
    pub fn identity(&self) -> &BackendId {
        &self.identity
    }

    /// Always `invalid_capability`/`duplicate-backend` (FR-075-AC-4).
    pub fn catalog_code(&self) -> CatalogCode {
        CatalogCode::new("invalid_capability", "duplicate-backend")
    }
}

/// A requested item's candidate set (FR-075 Outputs).
///
/// Retains the data an `unsupported` warning needs even when the set is
/// empty (FR-076-AC-2): the requested capability kind and, when the
/// request named one, that backend's identity. Both are exposed by this
/// type's own accessors -- a caller never has to re-derive them from data
/// held before the call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateSet {
    kind: Capability,
    requested_backend: Option<BackendId>,
    candidates: Vec<(BackendId, ByteDigest)>,
}

impl CandidateSet {
    /// The requested item's capability kind.
    pub fn kind(&self) -> Capability {
        self.kind
    }

    /// The `BackendId` the request named, if any.
    pub fn requested_backend(&self) -> Option<&BackendId> {
        self.requested_backend.as_ref()
    }

    /// Every candidate, ordered bytewise by identity then by manifest
    /// digest (FR-290 "Candidate set and negotiation"), independent of
    /// registration order (FR-075-AC-2). Empty when no registrant
    /// advertises `kind` (FR-076): an ordinary, well-formed value, never a
    /// refusal or a hold.
    pub fn candidates(&self) -> &[(BackendId, ByteDigest)] {
        &self.candidates
    }

    /// Whether this set has no candidates (backend absence, FR-076).
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }
}

/// A candidate-set computation's result (FR-075 Outputs): either a computed
/// set (possibly empty; FR-076) or the distinct unknown-backend marker
/// (FR-075-AC-3).
///
/// Neither variant is a refusal, a panic, or a hold: this is ordinary data
/// about what the registry currently holds, computed once per call and
/// never deferred (FR-076-AC-3, TC-198).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CandidateOutcome {
    /// The computed candidate set, ordinary data even when empty.
    Candidates(CandidateSet),
    /// The request named a `BackendId` the registry does not hold. Never
    /// reported as, or confused with, an empty candidate set
    /// (FR-075-AC-3).
    UnknownBackend(BackendId),
}

/// The `#185` registry: an ordinary value built by the orchestrating driver
/// and passed as an argument to every consumer (ADR-012 §7.1). No consumer
/// reads registrations through a global, a `static`, a `OnceLock`, a
/// `thread_local!`, or a plugin-discovery mechanism such as `inventory`,
/// `linkme` or `ctor` (FR-075-CON-1).
///
/// Two registries built from the same set of descriptors, added in any
/// order, are equal, and compute identical candidate sets, in identical
/// order, for every item (FR-075-AC-2). This holds because storage is a
/// `BTreeMap` keyed by `BackendId` (whose iteration and equality are
/// already order-independent) and each descriptor's `advertises` set is a
/// `HashSet` (also order-independent); candidate output is explicitly
/// sorted (see [`Registry::candidates`]).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Registry(BTreeMap<BackendId, BackendDescriptor>);

impl Registry {
    /// An empty registry.
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Register one backend.
    ///
    /// Refuses a repeated `BackendId` with
    /// `invalid_capability`/`duplicate-backend`, naming the identity, and
    /// leaves the existing registration unchanged and in effect
    /// (FR-075-AC-4). A capability kind outside the FR-290 vocabulary, or a
    /// mode other than `bounded`/`unbounded`, cannot reach this function at
    /// all: both are excluded by the [`Capability`] and [`Mode`] types
    /// themselves, so this registry adds no rule for them (ADR-012 §5.2's
    /// own note on that row: "this record adds nothing").
    pub fn register(&mut self, descriptor: BackendDescriptor) -> Result<(), RegistrationRefusal> {
        if self.0.contains_key(&descriptor.id) {
            return Err(RegistrationRefusal {
                identity: descriptor.id,
            });
        }
        self.0.insert(descriptor.id.clone(), descriptor);
        Ok(())
    }

    /// Every registered backend's identity, in registry (sorted) order.
    /// Not part of candidate computation's own contract; provided for
    /// diagnostics and tests.
    pub fn backends(&self) -> impl Iterator<Item = &BackendId> {
        self.0.keys()
    }

    /// Compute `kind`'s candidate set (FR-075), naming `named` when the
    /// caller requests one specific backend.
    ///
    /// - `named` absent: the set is every registered backend that
    ///   advertises `kind`, sorted by `(identity, manifest digest)`.
    /// - `named` present and registered: the set is that one backend when
    ///   it advertises `kind`, and empty otherwise.
    /// - `named` present and unregistered: [`CandidateOutcome::UnknownBackend`],
    ///   never reported as an empty set (FR-075-AC-3).
    pub fn candidates(&self, kind: Capability, named: Option<&BackendId>) -> CandidateOutcome {
        match named {
            Some(id) => match self.0.get(id) {
                None => CandidateOutcome::UnknownBackend(id.clone()),
                Some(descriptor) => {
                    let candidates = if descriptor.advertises_kind(kind) {
                        vec![(descriptor.id.clone(), descriptor.manifest_digest)]
                    } else {
                        Vec::new()
                    };
                    CandidateOutcome::Candidates(CandidateSet {
                        kind,
                        requested_backend: Some(id.clone()),
                        candidates,
                    })
                }
            },
            None => {
                let mut candidates: Vec<(BackendId, ByteDigest)> = self
                    .0
                    .values()
                    .filter(|descriptor| descriptor.advertises_kind(kind))
                    .map(|descriptor| (descriptor.id.clone(), descriptor.manifest_digest))
                    .collect();
                candidates.sort();
                CandidateOutcome::Candidates(CandidateSet {
                    kind,
                    requested_backend: None,
                    candidates,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    fn digest(byte: u8) -> ByteDigest {
        ByteDigest::of(&[byte])
    }

    fn descriptor(
        id: &str,
        digest_byte: u8,
        advertises: impl IntoIterator<Item = (Capability, Mode)>,
    ) -> BackendDescriptor {
        BackendDescriptor::new(
            BackendId::new(id),
            digest(digest_byte),
            ToolIdentity::new("tool"),
            advertises,
        )
    }

    /// FR-075-AC-5 (TC-193 step 6, narrowed to what a unit test -- not a
    /// source scan -- can assert): candidate matching goes through
    /// `Capability` alone, and two structurally different `Capability`
    /// values are never treated as the same kind.
    #[test]
    #[trace("TC-193", "FR-075-AC-5")]
    fn advertises_kind_matches_only_the_exact_capability() {
        let backend = descriptor("A", 1, [(Capability::ValueValidity, Mode::Bounded)]);
        assert!(backend.advertises_kind(Capability::ValueValidity));
        assert!(!backend.advertises_kind(Capability::OperationContract));
    }

    /// ADR-012 §5.2 row: duplicate `BackendId` (FR-075-AC-4).
    #[test]
    fn duplicate_backend_id_is_refused_and_the_original_stands() {
        let mut registry = Registry::new();
        registry
            .register(descriptor(
                "A",
                1,
                [(Capability::ValueValidity, Mode::Bounded)],
            ))
            .expect("first registration succeeds");
        let refusal = registry
            .register(descriptor(
                "A",
                2,
                [(Capability::OperationContract, Mode::Bounded)],
            ))
            .expect_err("duplicate BackendId must be refused");
        assert_eq!(refusal.identity().as_str(), "A");
        assert_eq!(
            refusal.catalog_code().to_string(),
            "invalid_capability/duplicate-backend"
        );
        // The original registration's advertised kinds are unaffected.
        let CandidateOutcome::Candidates(set) =
            registry.candidates(Capability::ValueValidity, None)
        else {
            panic!("expected a computed candidate set");
        };
        assert_eq!(set.candidates().len(), 1);
    }

    /// ADR-012 §5.2 row: an unregistered named `BackendId`.
    #[test]
    fn unregistered_named_backend_yields_the_unknown_backend_marker() {
        let registry = Registry::new();
        let outcome = registry.candidates(Capability::ValueValidity, Some(&BackendId::new("Z")));
        assert_eq!(
            outcome,
            CandidateOutcome::UnknownBackend(BackendId::new("Z"))
        );
    }

    /// ADR-012 §5.2 row: a capability kind no registrant advertises
    /// (FR-076).
    #[test]
    fn unadvertised_kind_yields_an_ordinary_empty_set() {
        let mut registry = Registry::new();
        registry
            .register(descriptor(
                "A",
                1,
                [(Capability::ValueValidity, Mode::Bounded)],
            ))
            .unwrap();
        let CandidateOutcome::Candidates(set) =
            registry.candidates(Capability::TemporalSatisfaction, None)
        else {
            panic!("expected a computed candidate set, not an unknown-backend marker");
        };
        assert!(set.is_empty());
        assert_eq!(set.kind(), Capability::TemporalSatisfaction);
    }

    /// ADR-012 §5.2 row: more than one registrant matches and the request
    /// names none (routing ambiguity is `negotiate_*`'s to settle; the
    /// registry just reports every match, in order).
    #[test]
    fn more_than_one_match_with_no_named_backend_returns_every_candidate_in_order() {
        let mut registry = Registry::new();
        registry
            .register(descriptor(
                "B",
                2,
                [(Capability::ValueValidity, Mode::Bounded)],
            ))
            .unwrap();
        registry
            .register(descriptor(
                "A",
                1,
                [(Capability::ValueValidity, Mode::Bounded)],
            ))
            .unwrap();
        let CandidateOutcome::Candidates(set) =
            registry.candidates(Capability::ValueValidity, None)
        else {
            panic!("expected a computed candidate set");
        };
        let ids: Vec<&str> = set.candidates().iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(
            ids,
            ["A", "B"],
            "candidates are sorted bytewise by identity"
        );
    }
}
