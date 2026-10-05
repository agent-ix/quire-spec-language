// SPDX-License-Identifier: AGPL-3.0-or-later
//! `qsl-route`: the ADR-011 §6.1 layer **R** crate (ADR-011 §7.3
//! X-9) -- the `#185` capability-backend registry and router (ADR-012 §6,
//! §7). Among the workspace crates it depends only on `qsl-semantics`
//! (layer 3) and `qsl-foundation` (layer F); its one other dependency is
//! `thiserror`.
//!
//! `route` answers one question -- **which registered backends, if any,
//! advertise the capability kind a requested item needs** -- and nothing
//! else. It computes a candidate set from an explicitly-built [`Registry`]
//! value (ADR-012 §7.1), matching on the canonical
//! [`qsl_semantics::check::Capability`] kind alone (FR-075): mode
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
//! (ADR-012 §7.2, quire-contract-codegen#86). The routing step
//! ([`routing`], FR-057) reads those settled dispositions back as input
//! data and gives a target only to a `supported` item; it produces no
//! disposition of its own.
//!
//! A backend registers from the labels its FR-331 provider manifest states
//! ([`BackendDescriptor::admit`], FR-057-AC-8): each advertised kind and
//! mode is admitted under the same exact-label rules as a requested pair,
//! and an absent or unknown kind or an unknown mode refuses the whole
//! registration, keyed by backend identity. A candidate is the ADR-013
//! O-19 `backend` value: the backend identity alone ([`Candidate`]). The
//! digest of a backend's FR-331 provider manifest ([`ManifestDigest`],
//! domain `quire.tool-manifest.jcs/v1`) stays here, on the descriptor and
//! on the `duplicate-backend` refusal, as the key for one refusal per
//! distinct manifest (FR-290); it is no part of the `backend` wire member.
//!
//! The registry is an ordinary value (FR-075 "The registry is an ordinary
//! value, not ambient state"; FR-075-CON-1): it is built by the
//! orchestrating driver and passed as an argument to every consumer. This
//! module defines no `static`, `OnceLock` or `thread_local!`, and depends
//! on no plugin-discovery crate (`inventory`, `linkme`, `ctor`); FR-080
//! records the gates (`make route-lint`, `cargo deny check bans`) that hold
//! both of those.

pub mod request;
pub mod routing;

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt;

pub use qsl_foundation::digest::ManifestDigest;
use qsl_foundation::{CatalogCode, Category, Code};
use qsl_semantics::check::Capability;

/// A backend's declared identity, unique within one [`Registry`] (FR-290
/// "Candidate set and negotiation": "a backend identity is unique within a
/// registry"). Opaque, typed data: on the wire it is the
/// `backend` member, the identity string exactly as the FR-331 manifest
/// states it (ADR-012 §7.1, ADR-013 O-19), which this module receives
/// already formed and does not itself mint.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct BackendId(String);

impl BackendId {
    /// Build a `BackendId` from its wire identity string. This module
    /// applies no normalization: the identity is opaque data, compared only
    /// for exact equality.
    pub fn new(identity: impl Into<String>) -> Self {
        Self(identity.into())
    }

    /// Read a `BackendId` from the wire `backend` member (ADR-013 O-19,
    /// FR-075-AC-6): the identity is kept verbatim, and an empty identity
    /// refuses with [`EmptyBackendIdentity`].
    pub fn from_wire(identity: &str) -> Result<Self, EmptyBackendIdentity> {
        if identity.is_empty() {
            Err(EmptyBackendIdentity)
        } else {
            Ok(Self::new(identity))
        }
    }

    /// The identity's exact wire spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// [`BackendId::from_wire`]'s refusal: the `backend` member is empty.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("invalid_identifier: the backend identity is empty")]
pub struct EmptyBackendIdentity;

impl EmptyBackendIdentity {
    /// The catalog code of this refusal, the one
    /// `qsl_replay::ReplayRequestRefusal::EmptyBackendIdentity` carries:
    /// `invalid_identifier`.
    pub const fn code(&self) -> Code {
        Code::InvalidIdentifier
    }

    /// Always `Category::Refusal` (ADR-013 O-16): the input is refused, not
    /// a fault.
    pub const fn category(&self) -> Category {
        Category::Refusal
    }
}

impl fmt::Display for BackendId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
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

impl Mode {
    /// Both FR-290 modes. The order carries no meaning; the slice exists so
    /// [`Mode::from_wire`] derives from [`Mode::to_wire`] rather than
    /// restating the two labels.
    pub const ALL: &'static [Mode] = &[Mode::Bounded, Mode::Unbounded];

    /// The FR-290 wire spelling: exactly `bounded` or `unbounded`.
    pub const fn to_wire(self) -> &'static str {
        match self {
            Mode::Bounded => "bounded",
            Mode::Unbounded => "unbounded",
        }
    }

    /// Read a mode from its exact FR-290 wire spelling, with no
    /// normalization or default. Any other label (FR-290 "a mode outside
    /// these two") is `None`; [`BackendDescriptor::admit`] turns that into
    /// `invalid_capability`/`unknown-mode`.
    pub fn from_wire(label: &str) -> Option<Mode> {
        Mode::ALL
            .iter()
            .copied()
            .find(|mode| mode.to_wire() == label)
    }
}

// `ManifestDigest` (ADR-013 O-19's `quire.tool-manifest.jcs/v1`-typed
// backend digest) and its refusal live in `qsl_foundation::digest`:
// `route` (layer R) and `replay` (layer 6) cannot depend on each other
// (ADR-011 §6.1), but both depend on `qsl_foundation` (layer F), so the one
// domain-first-checked type lives there and both readers share it instead of
// each carrying its own copy of the same "not just any FR-201 domain" check.

/// A candidate: the backend identity of a registered backend (FR-290
/// "Candidate set and negotiation": "A candidate is the backend identity of
/// a registered backend"), which is also the ADR-013 O-19 `backend` value.
///
/// Candidates order bytewise by identity, as FR-290 orders them.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Candidate {
    id: BackendId,
}

impl Candidate {
    /// The candidate for a backend identity.
    pub fn new(id: BackendId) -> Self {
        Self { id }
    }

    /// The backend identity.
    pub fn id(&self) -> &BackendId {
        &self.id
    }
}

/// Where a registered provider comes from (ADR-029 PV-1, FR-288). The host
/// knows this fact; the provider never states it about itself, so no
/// manifest member carries it and a plugin cannot claim to be linked.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProviderOrigin {
    /// A compile-time `Provider` linked into the binary.
    Linked,
    /// A plugin process that gave its manifest in a `hello` frame.
    Process,
}

/// One backend's registration (FR-075 Inputs): its identity, its
/// [`ProviderOrigin`], the digest of its own FR-331 provider manifest, and
/// the `(capability kind, mode)` pairs it advertises. Two descriptors of one
/// identity that differ in origin differ in a member, so they conflict like
/// any other differing pair (FR-288-AC-6). The manifest digest only tells two registrations of one
/// identity apart, so a conflict is refused once per distinct manifest
/// (FR-290); it is never part of the candidate.
///
/// `BackendDescriptor`, the candidate set and `Capability` cross repository
/// boundaries as QSpec data, never through a shared Rust crate
/// (ADR-013 T-7).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendDescriptor {
    backend: Candidate,
    origin: ProviderOrigin,
    manifest_digest: ManifestDigest,
    advertises: HashSet<(Capability, Mode)>,
}

impl BackendDescriptor {
    /// Build a descriptor from already-typed FR-331 provider-manifest
    /// fields. A backend supplies one descriptor per registration, so its
    /// identity carries exactly one manifest digest at a time (FR-075
    /// Inputs).
    pub fn new(
        backend: Candidate,
        origin: ProviderOrigin,
        manifest_digest: ManifestDigest,
        advertises: impl IntoIterator<Item = (Capability, Mode)>,
    ) -> Self {
        Self {
            backend,
            origin,
            manifest_digest,
            advertises: advertises.into_iter().collect(),
        }
    }

    /// Build a descriptor from the advertised `(kind, mode)` labels exactly
    /// as the backend's provider manifest states them (FR-057 "Stage
    /// ownership", FR-290 "Advertised mode", ADR-013 C-28).
    ///
    /// Each kind is admitted under the same rule as a requested pair: exact
    /// byte equality with one FR-290 label, no normalization or default.
    /// Any failing pair refuses the whole registration, keyed by
    /// `backend` and `manifest_digest`:
    ///
    /// - kind `None` (absent or `null`): `invalid_capability`/`absent-kind`;
    /// - kind not an FR-290 label: `invalid_capability`/`unknown-kind`,
    ///   carrying the received bytes;
    /// - mode `None`, or other than `bounded`/`unbounded`:
    ///   `invalid_capability`/`unknown-mode`, carrying the received bytes
    ///   when there are any.
    ///
    /// **First-failure rule.** A refusal names one cause (FR-290: "refuse
    /// that registration with `invalid_capability` (`absent-kind`,
    /// `unknown-kind` or `unknown-mode`)"). When more than one pair fails,
    /// the cause reported is the first failure in the order the pairs are
    /// given, and within one pair the kind is checked before the mode, so a
    /// pair whose kind and mode are both bad reports its kind. FR-290 treats
    /// the advertised pairs as a set and does not rank causes; this order
    /// is this function's own, stated here so it is deterministic for one
    /// manifest. Whether the refusal is reported at all never depends on
    /// order.
    ///
    /// A refused descriptor never exists, so it contributes nothing to any
    /// registry.
    pub fn admit<'a>(
        backend: Candidate,
        origin: ProviderOrigin,
        manifest_digest: ManifestDigest,
        advertised: impl IntoIterator<Item = (Option<&'a str>, Option<&'a str>)>,
    ) -> Result<Self, RegistrationRefusal> {
        let refuse = |cause| RegistrationRefusal {
            backend: backend.clone(),
            manifest_digest,
            cause,
        };
        let mut advertises = HashSet::new();
        for (kind, mode) in advertised {
            let kind = kind.ok_or_else(|| refuse(RegistrationCause::AbsentKind))?;
            let kind = Capability::from_wire(kind).map_err(|unknown| {
                refuse(RegistrationCause::UnknownKind(
                    unknown.received().to_owned(),
                ))
            })?;
            let mode = mode
                .and_then(Mode::from_wire)
                .ok_or_else(|| refuse(RegistrationCause::UnknownMode(mode.map(str::to_owned))))?;
            advertises.insert((kind, mode));
        }
        Ok(Self {
            backend,
            origin,
            manifest_digest,
            advertises,
        })
    }

    /// This descriptor's backend identity.
    pub fn id(&self) -> &BackendId {
        &self.backend.id
    }

    /// This backend as a candidate: its identity alone.
    pub fn candidate(&self) -> &Candidate {
        &self.backend
    }

    /// Where this provider comes from: linked at compile time, or a plugin
    /// process (ADR-029 PV-1, PV-4).
    pub fn origin(&self) -> ProviderOrigin {
        self.origin
    }

    /// The digest of this backend's own FR-331 provider manifest.
    pub fn manifest_digest(&self) -> ManifestDigest {
        self.manifest_digest
    }

    /// Every advertised `(kind, mode)` pair, in no particular order: the
    /// data the FR-331 `manifest` restates per registered backend (FR-057
    /// "Stage ownership"). Mode is carried for `negotiate_*`; candidate
    /// computation never reads it (FR-075).
    pub fn advertises(&self) -> impl Iterator<Item = (Capability, Mode)> + '_ {
        self.advertises.iter().copied()
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
    /// `RUSTFLAGS=--cfg seam_probe --cfg seam_probe_downstream`, which adds a
    /// `#[cfg(seam_probe)]` probe variant to `Capability`
    /// (`qsl-semantics/src/check/capability.rs`), and confirms this match
    /// fails to compile with `E0004` (non-exhaustive) against it.
    fn advertises_kind(&self, kind: Capability) -> bool {
        /// FR-063-AC-7: `#[deny(...)]` closes the `_ => unsupported(...)`
        /// escape hatch the seam probe alone cannot see.
        #[deny(clippy::wildcard_enum_match_arm)]
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

/// Why a registration is refused: the FR-290 registration causes, all
/// under code `invalid_capability` (FR-057-AC-8).
///
/// `Ord` exists only so [`RegistrationRefusal`] can order refusals under
/// one backend deterministically; it ranks no cause above another.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RegistrationCause {
    /// A registration of an identity whose admitted registrations are not
    /// all equal (FR-290-AC-10; FR-075-AC-4).
    DuplicateBackend,
    /// An advertised pair names no kind.
    AbsentKind,
    /// An advertised kind is not byte-equal to an FR-290 label; carries the
    /// exact received bytes.
    UnknownKind(String),
    /// An advertised mode is absent, or neither `bounded` nor `unbounded`;
    /// carries the exact received bytes, `None` when there were none.
    UnknownMode(Option<String>),
}

impl RegistrationCause {
    /// The catalog cause under `invalid_capability`.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::DuplicateBackend => "duplicate-backend",
            Self::AbsentKind => "absent-kind",
            Self::UnknownKind(_) => "unknown-kind",
            Self::UnknownMode(_) => "unknown-mode",
        }
    }
}

/// A registration refused outright, keyed by backend identity (FR-290
/// "Advertised mode", "Candidate set and negotiation"; FR-057-AC-8).
/// Structurally distinct from [`CandidateOutcome`] (FR-076-AC-3, TC-198) --
/// a caller cannot mistake a registration refusal for an empty candidate
/// set or an unknown-backend marker by pattern-matching alone.
///
/// Carries the refused registration's manifest digest beside its
/// [`Candidate`], so refusals order as FR-290 reports them: bytewise by
/// backend identity, then by manifest digest (`Ord` is derived over
/// `(backend, manifest_digest, cause)` in that field order).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, thiserror::Error)]
#[error("invalid_capability/{}: registration of {} refused", cause.as_str(), backend.id)]
pub struct RegistrationRefusal {
    backend: Candidate,
    manifest_digest: ManifestDigest,
    cause: RegistrationCause,
}

impl RegistrationRefusal {
    /// The identity whose registration was refused.
    pub fn identity(&self) -> &BackendId {
        &self.backend.id
    }

    /// The refused registration's candidate.
    pub fn backend(&self) -> &Candidate {
        &self.backend
    }

    /// The digest of the refused registration's manifest.
    pub fn manifest_digest(&self) -> ManifestDigest {
        self.manifest_digest
    }

    /// Why it was refused.
    pub fn cause(&self) -> &RegistrationCause {
        &self.cause
    }

    /// `invalid_capability` with this refusal's cause.
    pub fn catalog_code(&self) -> CatalogCode {
        CatalogCode::new("invalid_capability", self.cause.as_str())
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
    candidates: Vec<Candidate>,
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

    /// Every candidate, ordered bytewise by identity (FR-290 "Candidate set and negotiation"), independent of
    /// registration order (FR-075-AC-2). Empty when no registrant
    /// advertises `kind` (FR-076): an ordinary, well-formed value, never a
    /// refusal or a hold.
    pub fn candidates(&self) -> &[Candidate] {
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
/// order, for every item, including a set with an identical-repeat or a
/// conflicting registration of one identity (FR-075-AC-2, FR-290-AC-9/AC-10).
/// This holds because `held` is a `BTreeMap` keyed by `BackendId` (whose
/// iteration and equality are already order-independent), `conflicts` is a
/// `BTreeMap` of `BTreeSet`s (a union, so also order-independent), and each
/// descriptor's `advertises` set is a `HashSet` (also order-independent);
/// candidate output is explicitly sorted (see [`Registry::candidates`]).
///
/// `conflicts` records, per conflicted `BackendId`, every distinct manifest
/// digest an admitted registration of it has carried (FR-290 "Candidate set
/// and negotiation"). Once an identity appears here it is permanently
/// unregistered: `held` never holds that identity again, so `candidates`
/// reports it as [`CandidateOutcome::UnknownBackend`] for the rest of the
/// registry's life, independent of any later registration of it (FR-290:
/// "A registration of an identity that conflicts is refused whenever it
/// arrives").
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Registry {
    held: BTreeMap<BackendId, BackendDescriptor>,
    conflicts: BTreeMap<BackendId, BTreeSet<ManifestDigest>>,
}

impl Registry {
    /// An empty registry.
    pub fn new() -> Self {
        Self {
            held: BTreeMap::new(),
            conflicts: BTreeMap::new(),
        }
    }

    /// Register one backend (FR-290 "Candidate set and negotiation").
    ///
    /// - When `descriptor`'s identity is already conflicted, this
    ///   registration is refused on arrival: its manifest digest is added to
    ///   the identity's recorded conflict digests, and `held` is left
    ///   untouched (it already holds nothing for this identity).
    /// - When the identity is held with an equal descriptor (same identity,
    ///   manifest digest and advertised pairs), the repeat is one
    ///   registration and is not refused (FR-290-AC-9).
    /// - When the identity is held with a descriptor that differs in any
    ///   member, the two conflict: the held registration is withdrawn, both
    ///   registrations' manifest digests are recorded, and every refusal
    ///   this call produces is returned (FR-290-AC-10) -- one per distinct
    ///   digest, since a same-digest conflict (differing only in some other
    ///   member) has one refusal to report, not two.
    /// - Otherwise the descriptor is newly held.
    ///
    /// The `Err` vector, when returned, is never empty: it holds at least
    /// the refusal for `descriptor`'s own arrival. Its entries can repeat a
    /// key (`identity`, digest) [`Registry::refusals`] already reports from
    /// an earlier call -- one call's refusals are what that call caused or
    /// observed, not a diff against the registry's prior state.
    ///
    /// A capability kind outside the FR-290 vocabulary, or a mode other than
    /// `bounded`/`unbounded`, cannot reach this function at all: a
    /// [`BackendDescriptor`] holds only typed [`Capability`] and [`Mode`]
    /// values, and [`BackendDescriptor::admit`] refuses such labels before a
    /// descriptor exists.
    pub fn register(
        &mut self,
        descriptor: BackendDescriptor,
    ) -> Result<(), Vec<RegistrationRefusal>> {
        let id = descriptor.id().clone();

        if let Some(digests) = self.conflicts.get_mut(&id) {
            let digest = descriptor.manifest_digest();
            digests.insert(digest);
            return Err(vec![RegistrationRefusal {
                backend: Candidate::new(id),
                manifest_digest: digest,
                cause: RegistrationCause::DuplicateBackend,
            }]);
        }

        match self.held.get(&id) {
            None => {
                self.held.insert(id, descriptor);
                Ok(())
            }
            Some(existing) if *existing == descriptor => Ok(()),
            Some(existing) => {
                let mut digests = BTreeSet::new();
                digests.insert(existing.manifest_digest());
                digests.insert(descriptor.manifest_digest());
                // Ascending because `digests` is a `BTreeSet`, so this is
                // already in `Registry::refusals`' (identity, digest) order
                // -- no separate sort needed.
                let refusals: Vec<RegistrationRefusal> = digests
                    .iter()
                    .map(|&digest| RegistrationRefusal {
                        backend: Candidate::new(id.clone()),
                        manifest_digest: digest,
                        cause: RegistrationCause::DuplicateBackend,
                    })
                    .collect();
                self.held.remove(&id);
                self.conflicts.insert(id, digests);
                Err(refusals)
            }
        }
    }

    /// Every `duplicate-backend` refusal the registry currently holds, one
    /// per distinct manifest digest recorded against a conflicted identity,
    /// ordered bytewise by `(identity, manifest digest)` (FR-290 "Candidate
    /// set and negotiation", FR-290-AC-10). Stable for as long as the
    /// conflict stands, independent of registration order or of how many
    /// further registrations of the identity have since arrived.
    pub fn refusals(&self) -> impl Iterator<Item = RegistrationRefusal> + '_ {
        self.conflicts.iter().flat_map(|(id, digests)| {
            digests.iter().map(move |&digest| RegistrationRefusal {
                backend: Candidate::new(id.clone()),
                manifest_digest: digest,
                cause: RegistrationCause::DuplicateBackend,
            })
        })
    }

    /// Every registered descriptor, in registry (identity-sorted) order:
    /// the registry snapshot the FR-331 `manifest` restates, one descriptor
    /// per registered backend (FR-057 "Stage ownership"). Never includes a
    /// conflicted identity (FR-290-AC-10).
    pub fn descriptors(&self) -> impl Iterator<Item = &BackendDescriptor> {
        self.held.values()
    }

    /// Every registered backend's identity, in registry (sorted) order.
    /// Not part of candidate computation's own contract; provided for
    /// diagnostics and tests. Never includes a conflicted identity.
    pub fn backends(&self) -> impl Iterator<Item = &BackendId> {
        self.held.keys()
    }

    /// Compute `kind`'s candidate set (FR-075), naming `named` when the
    /// caller requests one specific backend.
    ///
    /// - `named` absent: the set is every registered backend that
    ///   advertises `kind`, sorted by identity.
    /// - `named` present and registered: the set is that one backend when
    ///   it advertises `kind`, and empty otherwise.
    /// - `named` present and unregistered, including a conflicted identity:
    ///   [`CandidateOutcome::UnknownBackend`], never reported as an empty
    ///   set (FR-075-AC-3, FR-290-AC-10).
    pub fn candidates(&self, kind: Capability, named: Option<&BackendId>) -> CandidateOutcome {
        match named {
            Some(id) => match self.held.get(id) {
                None => CandidateOutcome::UnknownBackend(id.clone()),
                Some(descriptor) => {
                    let candidates = if descriptor.advertises_kind(kind) {
                        vec![descriptor.backend.clone()]
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
                let mut candidates: Vec<Candidate> = self
                    .held
                    .values()
                    .filter(|descriptor| descriptor.advertises_kind(kind))
                    .map(|descriptor| descriptor.backend.clone())
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

    fn candidate(id: &str) -> Candidate {
        Candidate::new(BackendId::new(id))
    }

    fn descriptor(
        id: &str,
        digest_byte: u8,
        advertises: impl IntoIterator<Item = (Capability, Mode)>,
    ) -> BackendDescriptor {
        BackendDescriptor::new(
            candidate(id),
            ProviderOrigin::Linked,
            ManifestDigest::from_digest([digest_byte; 32]),
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

    /// FR-075-AC-7 (quire-specification TC-282 DB-01, FR-290-AC-9):
    /// repeating an identical registration -- same id, manifest digest and
    /// advertised pairs -- is one registration and is never refused.
    #[test]
    #[trace("TC-448", "FR-075-AC-7")]
    fn identical_repeat_registration_is_idempotent_and_not_refused() {
        let a1 = descriptor("A", 1, [(Capability::ValueValidity, Mode::Bounded)]);
        let a2 = descriptor("A", 1, [(Capability::ValueValidity, Mode::Bounded)]);
        let b = descriptor("B", 2, [(Capability::OperationContract, Mode::Bounded)]);

        // B, the unrelated identity, arrives before both A repeats, between
        // them, and after both -- the result must not depend on where.
        for b_position in 0..3 {
            let mut registrations = vec![a1.clone(), a2.clone()];
            registrations.insert(b_position, b.clone());

            let mut registry = Registry::new();
            for (i, r) in registrations.into_iter().enumerate() {
                registry
                    .register(r)
                    .unwrap_or_else(|_| panic!("registration {i} must not be refused"));
            }

            let CandidateOutcome::Candidates(set) =
                registry.candidates(Capability::ValueValidity, None)
            else {
                panic!("expected a computed candidate set; b_position={b_position}");
            };
            assert_eq!(set.candidates().len(), 1, "b_position={b_position}");
            assert_eq!(
                registry
                    .backends()
                    .map(BackendId::as_str)
                    .collect::<Vec<_>>(),
                ["A", "B"],
                "b_position={b_position}"
            );
            assert_eq!(registry.refusals().count(), 0, "b_position={b_position}");
        }
    }

    /// FR-075-AC-4 (quire-specification TC-282 DB-03, FR-290-AC-10): two
    /// unequal descriptors under one `BackendId` conflict. Every
    /// registration of that identity is refused, the held registration is
    /// withdrawn, and the identity becomes unregistered -- under either
    /// arrival order.
    #[test]
    #[trace("TC-196", "FR-075-AC-4")]
    fn conflicting_descriptors_refuse_both_and_withdraw_the_held_registration() {
        for reversed in [false, true] {
            let mut registry = Registry::new();
            let a1 = descriptor("A", 1, [(Capability::ValueValidity, Mode::Bounded)]);
            let a2 = descriptor("A", 2, [(Capability::OperationContract, Mode::Bounded)]);
            let (first, second) = if reversed { (a2, a1) } else { (a1, a2) };

            registry
                .register(first)
                .expect("the first registration of a fresh identity succeeds");
            let refusals = registry
                .register(second)
                .expect_err("a conflicting descriptor under one identity refuses both");

            assert_eq!(refusals.len(), 2, "reversed={reversed}");
            for refusal in &refusals {
                assert_eq!(refusal.identity().as_str(), "A", "reversed={reversed}");
                assert_eq!(
                    refusal.catalog_code().to_string(),
                    "invalid_capability/duplicate-backend",
                    "reversed={reversed}"
                );
            }

            assert!(
                registry.descriptors().next().is_none(),
                "the held registration is withdrawn; reversed={reversed}"
            );
            assert_eq!(
                registry.candidates(Capability::ValueValidity, Some(&BackendId::new("A"))),
                CandidateOutcome::UnknownBackend(BackendId::new("A")),
                "reversed={reversed}"
            );
            assert_eq!(registry.refusals().count(), 2, "reversed={reversed}");
        }
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
        let ids: Vec<&str> = set
            .candidates()
            .iter()
            .map(|candidate| candidate.id().as_str())
            .collect();
        assert_eq!(
            ids,
            ["A", "B"],
            "candidates are sorted bytewise by identity"
        );
    }

    /// ADR-013 O-19: the `backend` member is the identity string alone,
    /// kept verbatim, and two candidates are equal iff their identities are.
    #[test]
    #[trace("TC-433", "FR-075-AC-6")]
    fn backend_member_is_the_identity_string_alone() {
        let original = candidate(" Kani/1 ");
        assert_eq!(
            original.id().as_str(),
            " Kani/1 ",
            "identity is not normalized"
        );
        assert_eq!(original, Candidate::new(BackendId::new(" Kani/1 ")));
        assert_ne!(original, candidate("Kani/1"));
        assert_eq!(
            BackendId::from_wire(" Kani/1 ").unwrap().as_str(),
            " Kani/1 "
        );
    }

    /// FR-075-AC-6 (TC-433 step 2): an empty `backend` member refuses.
    #[test]
    #[trace("TC-433", "FR-075-AC-6")]
    fn an_empty_backend_member_refuses() {
        assert_eq!(BackendId::from_wire(""), Err(EmptyBackendIdentity));
    }

    /// FR-075-AC-6 (TC-433 step 2): the empty-identity refusal carries code
    /// `invalid_identifier` and category `refusal` (exit 20), so a caller
    /// forwards it without remapping.
    #[test]
    #[trace("TC-433", "FR-075-AC-6")]
    fn an_empty_backend_member_is_an_invalid_identifier_refusal() {
        let refusal = BackendId::from_wire("").unwrap_err();
        assert_eq!(refusal.code(), Code::InvalidIdentifier);
        assert_eq!(refusal.category(), Category::Refusal);
        assert_eq!(refusal.category().exit_code(), 20);
    }

    /// FR-290 "Advertised mode": exactly `bounded` and `unbounded`, read
    /// with no normalization.
    #[test]
    fn mode_reads_only_its_two_exact_labels() {
        assert_eq!(Mode::from_wire("bounded"), Some(Mode::Bounded));
        assert_eq!(Mode::from_wire("unbounded"), Some(Mode::Unbounded));
        for label in ["finite", "Bounded", " bounded", ""] {
            assert_eq!(Mode::from_wire(label), None, "{label:?}");
        }
    }
}
