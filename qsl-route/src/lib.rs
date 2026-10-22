// SPDX-License-Identifier: AGPL-3.0-or-later
//! `qsl-route`: the ADR-011 §6.1 layer **R** crate (ADR-011 §7.3
//! X-9) -- the `#185` capability-backend registry and router (ADR-012 §6,
//! §7). Among the workspace crates it depends only on `qsl-semantics`
//! (layer 3) and `qsl-foundation` (layer F), and on the layer-SV leaf
//! `quire-semantic-value` for the [`deadlock`] item's property form; its one
//! other dependency is `thiserror`.
//!
//! `route` answers one question -- **which registered backends, if any,
//! advertise the capability kind a requested item needs** -- and nothing
//! else. It computes a candidate set from an explicitly-built [`Registry`]
//! value (ADR-012 §7.1), matching on the canonical
//! [`qsl_semantics::check::Capability`] kind alone (FR-075): mode
//! (`bounded`/`unbounded`) never filters candidacy, and no local
//! capability-kind type is defined here or anywhere else in this ticket's
//! scope (FR-075-AC-5). Fairness kinds do filter (FR-134): a backend is a
//! candidate for an item only when it advertises every fairness kind the
//! item's fairness set uses ([`Registry::candidates_for_fairness`]). So do
//! memory models (FR-226): an item over a subject with a weak `parallel` is
//! given only backends advertising its resolved models
//! ([`Registry::candidates_for_models`]).
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
//! ([`descriptor_from_manifest`], FR-057-AC-8): each advertised kind and
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

pub mod deadlock;
pub mod model_request;
pub mod request;
pub mod routing;

use qsl_foundation::bound::DomainKind;
pub use qsl_foundation::digest::ManifestDigest;
use qsl_foundation::{BackendId, MemoryModelName};
#[cfg(test)]
use qsl_foundation::{Code, EmptyBackendIdentity};
use qsl_semantics::check::Capability;
use quire_semantic_value::catalog::CatalogCode;
#[cfg(test)]
use quire_semantic_value::catalog::Category;
use quire_semantic_value::fairness::FairnessKind;
use std::collections::{BTreeMap, BTreeSet, HashSet};

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
    /// these two") is `None`; [`descriptor_from_manifest`] turns that into
    /// `invalid_capability`/`unknown-mode`.
    pub fn from_wire(label: &str) -> Option<Mode> {
        Mode::ALL
            .iter()
            .copied()
            .find(|mode| mode.to_wire() == label)
    }
}

/// A probabilistic evidence kind (FR-290 "Evidence kinds"): what a
/// `probabilistic-satisfaction` provider produces and what an item asks for.
/// The two are never substituted for each other (FR-075-AC-13).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EvidenceKind {
    /// A measured result with an error guarantee (EN-4).
    Statistical,
    /// An exact probability over the full finite subject.
    Exact,
}

impl EvidenceKind {
    /// Both FR-290 evidence kinds. The order carries no meaning; the slice
    /// exists so [`EvidenceKind::from_wire`] derives from
    /// [`EvidenceKind::to_wire`] rather than restating the two labels.
    pub const ALL: &'static [EvidenceKind] = &[EvidenceKind::Statistical, EvidenceKind::Exact];

    /// The FR-290 wire spelling: exactly `statistical` or `exact`.
    pub const fn to_wire(self) -> &'static str {
        match self {
            EvidenceKind::Statistical => "statistical",
            EvidenceKind::Exact => "exact",
        }
    }

    /// Read an evidence kind from its exact wire spelling, with no
    /// normalization or default.
    pub fn from_wire(label: &str) -> Option<EvidenceKind> {
        EvidenceKind::ALL
            .iter()
            .copied()
            .find(|kind| kind.to_wire() == label)
    }
}

/// The evidence kind a `probabilistic-satisfaction` item names, preserved as
/// the item carries it (FR-075 "Candidate computation matches probabilistic
/// evidence exactly"). An absent or unknown value yields no candidate and
/// stays on the [`CandidateSet`] for negotiation to refuse as
/// `absent-evidence-kind` or `unknown-evidence-kind`; it is never turned into
/// a supported candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ItemEvidence {
    /// The item names no evidence kind.
    Absent,
    /// The item names a label that is neither `statistical` nor `exact`;
    /// carries the exact received bytes.
    Unknown(String),
    /// The item names one of the two evidence kinds.
    Kind(EvidenceKind),
}

impl ItemEvidence {
    /// Read an item's evidence label, `None` when the item names none.
    pub fn from_label(label: Option<&str>) -> Self {
        match label {
            None => ItemEvidence::Absent,
            Some(label) => EvidenceKind::from_wire(label).map_or_else(
                || ItemEvidence::Unknown(label.to_owned()),
                ItemEvidence::Kind,
            ),
        }
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
/// the `(capability kind, mode)` pairs it advertises, and the boundable
/// domain kinds its manifest accepts a finite bound for (`domains`). Two descriptors of one
/// identity that differ in origin differ in a member, so they conflict like
/// any other differing pair (FR-288-AC-6). The manifest digest only tells
/// two registrations of one identity apart, so a conflict is refused once
/// per distinct manifest (FR-290); it is never part of the candidate.
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
    domains: Option<BTreeSet<DomainKind>>,
    fairness_kinds: BTreeSet<FairnessKind>,
    memory_models: BTreeSet<MemoryModelName>,
    evidence_kinds: BTreeSet<EvidenceKind>,
}

/// Convert one already-read FR-331 provider manifest into its routed
/// descriptor. The driver supplies the manifest fields as data; this is the
/// sole manifest conversion entry point shared by linked and process
/// providers (FR-288, C-28).
///
/// `fairness_kinds` is the FR-331 manifest's `fairness_kinds` member exactly
/// as supplied, `None` when the manifest has none (FR-134 "Manifest
/// `fairness_kinds` admission"). It is judged after every advertised pair and
/// `domains`, and only for a backend that advertises `temporal-satisfaction`:
/// a member of any other backend is ignored. For a temporal backend a
/// defective member refuses `invalid-fairness-kinds` retaining the
/// [`FairnessKindsDefect`]: `absent`, `not-a-list`, `unknown-label` (the first
/// in member order) or `duplicate-label`. An empty list is admitted and
/// advertises none.
///
/// `evidence_kinds` is the manifest's evidence-kind labels, `None` when it
/// has none. It is judged last (after every advertised pair, `domains` and
/// `fairness_kinds`), and only for a backend that advertises
/// `probabilistic-satisfaction`: a member of any other backend is ignored.
/// For a probabilistic backend an absent or empty set, or a label that is
/// not byte-equal to `statistical` or `exact`, refuses
/// `unknown-evidence-kind` (FR-075-AC-11); a repeated label is one member.
pub fn descriptor_from_manifest<'a>(
    backend: Candidate,
    origin: ProviderOrigin,
    manifest_digest: ManifestDigest,
    advertised: impl IntoIterator<Item = (Option<&'a str>, Option<&'a str>)>,
    domains: Option<&[&str]>,
    fairness_kinds: Option<FairnessKindsMember<'_>>,
    evidence_kinds: Option<&[&str]>,
) -> Result<BackendDescriptor, RegistrationRefusal> {
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
    let advertises_bounded = advertises
        .iter()
        .any(|&(_, mode)| matches!(mode, Mode::Bounded));
    let domains = admit_domains(domains, advertises_bounded)
        .map_err(|defect| refuse(RegistrationCause::InvalidDomains(defect)))?;
    let advertises_temporal = advertises
        .iter()
        .any(|&(kind, _)| matches!(kind, Capability::TemporalSatisfaction));
    let fairness_kinds = if advertises_temporal {
        admit_fairness_kinds(fairness_kinds).map_err(refuse)?
    } else {
        BTreeSet::new()
    };
    let evidence_kinds = if advertises_probabilistic(&advertises) {
        admit_evidence_kinds(evidence_kinds)
            .map_err(|defect| refuse(RegistrationCause::UnknownEvidenceKind(defect)))?
    } else {
        BTreeSet::new()
    };
    Ok(BackendDescriptor {
        backend,
        origin,
        manifest_digest,
        advertises,
        domains,
        fairness_kinds,
        evidence_kinds,
        // Remaining work: QSpec FR-331-AC-30's `memory_models` member is not
        // read here yet, so a manifest descriptor advertises `sc` only, as a
        // descriptor without the member does (FR-226).
        memory_models: BTreeSet::new(),
    })
}

/// One element of a manifest's `fairness_kinds` list as the driver read it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FairnessLabel<'a> {
    /// A string element, its exact bytes.
    Text(&'a str),
    /// An element that is not a string, rendered as the manifest wrote it
    /// (such as `1` or `null`); it is never a fairness kind.
    Other(&'a str),
}

/// A manifest's present `fairness_kinds` member, exactly as supplied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FairnessKindsMember<'a> {
    /// A list, in member order.
    List(&'a [FairnessLabel<'a>]),
    /// A value that is not a list, rendered as the manifest wrote it (such
    /// as `null`, `"weak"` or `{}`).
    NotAList(&'a str),
}

/// Admit a temporal backend's `fairness_kinds` member (FR-134-AC-3 to
/// AC-5): the first unknown element wins over any repeat, and a repeat is
/// never collapsed.
fn admit_fairness_kinds(
    member: Option<FairnessKindsMember<'_>>,
) -> Result<BTreeSet<FairnessKind>, RegistrationCause> {
    let labels = match member {
        None => {
            return Err(RegistrationCause::InvalidFairnessKinds(
                FairnessKindsDefect::Absent,
            ))
        }
        Some(FairnessKindsMember::NotAList(value)) => {
            return Err(RegistrationCause::InvalidFairnessKinds(
                FairnessKindsDefect::NotAList(value.to_owned()),
            ))
        }
        Some(FairnessKindsMember::List(labels)) => labels,
    };
    let kinds = labels
        .iter()
        .map(|label| match *label {
            FairnessLabel::Text("weak") => Ok((FairnessKind::Weak, "weak")),
            FairnessLabel::Text("strong") => Ok((FairnessKind::Strong, "strong")),
            FairnessLabel::Text(other) | FairnessLabel::Other(other) => {
                Err(RegistrationCause::InvalidFairnessKinds(
                    FairnessKindsDefect::UnknownLabel(other.to_owned()),
                ))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut seen = BTreeSet::new();
    for (kind, label) in &kinds {
        if !seen.insert(*kind) {
            return Err(RegistrationCause::InvalidFairnessKinds(
                FairnessKindsDefect::DuplicateLabel((*label).to_owned()),
            ));
        }
    }
    Ok(seen)
}

impl BackendDescriptor {
    /// Build a descriptor from already-typed FR-331 provider-manifest
    /// fields. A backend supplies one descriptor per registration, so its
    /// identity carries exactly one manifest digest at a time (FR-075
    /// Inputs). `domains` is the manifest's `domains`, `None` when it has
    /// none.
    ///
    /// The same `domains` rules as [`descriptor_from_manifest`] apply: an
    /// absent set while a `bounded` pair is advertised, an empty set, or a
    /// kind that is not boundable refuses `invalid_capability`/
    /// `invalid-domains`, keyed by `backend` and `manifest_digest`. (A set
    /// holds no repeat.)
    ///
    /// `evidence_kinds` is required and nonempty exactly when a
    /// `probabilistic-satisfaction` pair is advertised, else
    /// `unknown-evidence-kind` (FR-075-AC-11); it is ignored for any other
    /// backend (FR-075-AC-12). Judged after `domains`.
    pub fn new(
        backend: Candidate,
        origin: ProviderOrigin,
        manifest_digest: ManifestDigest,
        advertises: impl IntoIterator<Item = (Capability, Mode)>,
        domains: Option<BTreeSet<DomainKind>>,
        evidence_kinds: Option<BTreeSet<EvidenceKind>>,
    ) -> Result<Self, RegistrationRefusal> {
        let advertises: HashSet<_> = advertises.into_iter().collect();
        let advertises_bounded = advertises
            .iter()
            .any(|&(_, mode)| matches!(mode, Mode::Bounded));
        check_domains(domains.as_ref(), advertises_bounded).map_err(|defect| {
            RegistrationRefusal {
                backend: backend.clone(),
                manifest_digest,
                cause: RegistrationCause::InvalidDomains(defect),
            }
        })?;
        let evidence_kinds = if advertises_probabilistic(&advertises) {
            check_evidence_kinds(evidence_kinds.as_ref()).map_err(|defect| {
                RegistrationRefusal {
                    backend: backend.clone(),
                    manifest_digest,
                    cause: RegistrationCause::UnknownEvidenceKind(defect),
                }
            })?;
            evidence_kinds.unwrap_or_default()
        } else {
            BTreeSet::new()
        };
        Ok(Self {
            backend,
            origin,
            manifest_digest,
            advertises,
            domains,
            fairness_kinds: BTreeSet::new(),
            memory_models: BTreeSet::new(),
            evidence_kinds,
        })
    }

    /// Advertise the fairness kinds this backend decides (FR-134, QSpec
    /// FR-368 `fairness_kinds`). A descriptor built without this call
    /// advertises none, so it is no candidate for an item whose fairness
    /// set uses a kind.
    #[must_use]
    pub fn with_fairness_kinds(mut self, kinds: impl IntoIterator<Item = FairnessKind>) -> Self {
        self.fairness_kinds = kinds.into_iter().collect();
        self
    }

    /// The fairness kinds this backend advertises (FR-134).
    pub fn fairness_kinds(&self) -> &BTreeSet<FairnessKind> {
        &self.fairness_kinds
    }

    /// Advertise the memory models this backend decides (FR-226, ADR-025
    /// MX-7, QSpec FR-439), by the names of the `memory` clause. A
    /// descriptor built without this call advertises `sc` only (QSpec
    /// FR-331), so it is no candidate for an item resolved to `tso` or `ra`.
    #[must_use]
    pub fn with_memory_models(mut self, models: impl IntoIterator<Item = MemoryModelName>) -> Self {
        self.memory_models = models.into_iter().collect();
        self
    }

    /// The memory models this backend advertises (FR-226).
    pub fn memory_models(&self) -> &BTreeSet<MemoryModelName> {
        &self.memory_models
    }

    /// Whether this backend decides an item whose subject resolves its
    /// outermost `parallel` controls to `resolved` (FR-226): it advertises
    /// every resolved model, and a descriptor with no `memory_models`
    /// advertises `sc` only (QSpec FR-331). An item over a subject with no
    /// `parallel` resolves no model and is decided by every backend.
    fn decides_models(&self, resolved: &BTreeSet<MemoryModelName>) -> bool {
        if self.memory_models.is_empty() {
            resolved.iter().all(|model| !model.is_weak())
        } else {
            resolved.is_subset(&self.memory_models)
        }
    }

    /// The evidence kinds this backend produces for
    /// `probabilistic-satisfaction` (FR-075-AC-10); empty for a backend that
    /// does not advertise that kind.
    pub fn evidence_kinds(&self) -> &BTreeSet<EvidenceKind> {
        &self.evidence_kinds
    }

    /// Whether this backend is a candidate for an item of `kind` naming
    /// `evidence`: for `probabilistic-satisfaction` its evidence-kind set
    /// must contain the item's exact evidence kind (neither kind substitutes
    /// for the other), and an absent or unknown item evidence matches no
    /// backend (FR-075-AC-13); any other kind ignores evidence.
    fn serves_evidence(&self, kind: Capability, evidence: &ItemEvidence) -> bool {
        if !matches!(kind, Capability::ProbabilisticSatisfaction) {
            return true;
        }
        match evidence {
            ItemEvidence::Kind(wanted) => self.evidence_kinds.contains(wanted),
            ItemEvidence::Absent | ItemEvidence::Unknown(_) => false,
        }
    }

    /// The boundable domain kinds the manifest accepts a finite bound for
    /// (FR-331 `domains`), or `None` when it stated none, which only a
    /// descriptor advertising no `bounded` pair can hold: both
    /// constructors refuse the rest.
    pub fn domains(&self) -> Option<&BTreeSet<DomainKind>> {
        self.domains.as_ref()
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
    /// hand over [`Capability`]'s twelve members, with no wildcard arm, so a
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
                Capability::ProbabilisticSatisfaction => {
                    matches!(b, Capability::ProbabilisticSatisfaction)
                }
                Capability::Schedulability => matches!(b, Capability::Schedulability),
            }
        }
        self.advertises.iter().any(|(k, _)| same_kind(*k, kind))
    }
}

/// Whether the advertised pairs include `probabilistic-satisfaction`.
fn advertises_probabilistic(advertises: &HashSet<(Capability, Mode)>) -> bool {
    advertises
        .iter()
        .any(|&(kind, _)| matches!(kind, Capability::ProbabilisticSatisfaction))
}

/// Admit a probabilistic backend's evidence-kind labels (FR-075-AC-11): the
/// first label that is neither `statistical` nor `exact` wins, then
/// [`check_evidence_kinds`].
fn admit_evidence_kinds(
    labels: Option<&[&str]>,
) -> Result<BTreeSet<EvidenceKind>, EvidenceKindsDefect> {
    let Some(labels) = labels else {
        return Err(EvidenceKindsDefect::Absent);
    };
    let mut parsed = BTreeSet::new();
    for &label in labels {
        let kind = EvidenceKind::from_wire(label)
            .ok_or_else(|| EvidenceKindsDefect::Unknown(label.to_owned()))?;
        parsed.insert(kind);
    }
    check_evidence_kinds(Some(&parsed))?;
    Ok(parsed)
}

/// The FR-075 evidence-kind rule over typed kinds: present and nonempty.
fn check_evidence_kinds(
    evidence_kinds: Option<&BTreeSet<EvidenceKind>>,
) -> Result<(), EvidenceKindsDefect> {
    match evidence_kinds {
        None => Err(EvidenceKindsDefect::Absent),
        Some(kinds) if kinds.is_empty() => Err(EvidenceKindsDefect::Empty),
        Some(_) => Ok(()),
    }
}

/// Admit a manifest's `domains` labels (FR-290 "Advertised mode"): parse
/// them, then apply [`check_domains`].
fn admit_domains(
    labels: Option<&[&str]>,
    advertises_bounded: bool,
) -> Result<Option<BTreeSet<DomainKind>>, DomainsDefect> {
    let Some(labels) = labels else {
        check_domains(None, advertises_bounded)?;
        return Ok(None);
    };
    let mut parsed = BTreeSet::new();
    for &label in labels {
        let kind = DomainKind::from_wire(label)
            .ok_or_else(|| DomainsDefect::NotBoundable(label.to_owned()))?;
        if !parsed.insert(kind) {
            return Err(DomainsDefect::Repeated(kind));
        }
    }
    check_domains(Some(&parsed), advertises_bounded)?;
    Ok(Some(parsed))
}

/// The FR-290 `domains` rules over typed kinds: absent only without a
/// `bounded` pair, never empty, every kind boundable.
fn check_domains(
    domains: Option<&BTreeSet<DomainKind>>,
    advertises_bounded: bool,
) -> Result<(), DomainsDefect> {
    let Some(domains) = domains else {
        return if advertises_bounded {
            Err(DomainsDefect::Absent)
        } else {
            Ok(())
        };
    };
    if domains.is_empty() {
        return Err(DomainsDefect::Empty);
    }
    match domains.iter().find(|kind| kind.finite_kind().is_none()) {
        Some(kind) => Err(DomainsDefect::NotBoundable(kind.to_wire().to_owned())),
        None => Ok(()),
    }
}

/// What is wrong with a manifest's `domains` (FR-290 "Advertised mode"):
/// the detail of [`RegistrationCause::InvalidDomains`].
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DomainsDefect {
    /// No `domains`, though a `bounded` pair is advertised.
    Absent,
    /// `domains` is present and empty.
    Empty,
    /// A label that is not `collection`, `population`, `integer` or
    /// `recursive`; carries the exact received bytes.
    NotBoundable(String),
    /// A kind listed twice.
    Repeated(DomainKind),
}

/// What is wrong with a probabilistic backend's evidence kinds (FR-075-AC-11):
/// the detail of [`RegistrationCause::UnknownEvidenceKind`].
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EvidenceKindsDefect {
    /// No evidence-kind set, though `probabilistic-satisfaction` is
    /// advertised.
    Absent,
    /// The evidence-kind set is present and empty.
    Empty,
    /// A label that is neither `statistical` nor `exact`; carries the first
    /// such label in member order, exactly as received.
    Unknown(String),
}

/// What is wrong with a manifest's `fairness_kinds` (QSpec FR-272): the
/// detail of [`RegistrationCause::InvalidFairnessKinds`].
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FairnessKindsDefect {
    /// A temporal backend's manifest has no `fairness_kinds` member.
    Absent,
    /// The member is not a list; carries it as supplied.
    NotAList(String),
    /// An element is not byte-equal to `weak` or `strong`; carries the first
    /// such element in member order.
    UnknownLabel(String),
    /// A label repeats; carries the first element that repeats an earlier
    /// one.
    DuplicateLabel(String),
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
    /// An advertised capability kind is not byte-equal to an FR-290 label;
    /// carries the exact received bytes. A fairness label is never this
    /// cause (see [`Self::InvalidFairnessKinds`]).
    UnknownKind(String),
    /// An advertised mode is absent, or neither `bounded` nor `unbounded`;
    /// carries the exact received bytes, `None` when there were none.
    UnknownMode(Option<String>),
    /// The manifest's `domains` is defective (FR-290 "Advertised mode").
    InvalidDomains(DomainsDefect),
    /// A `temporal-satisfaction` backend's `fairness_kinds` is defective
    /// (FR-134-AC-3 to AC-5; QSpec FR-368-AC-8).
    InvalidFairnessKinds(FairnessKindsDefect),
    /// A `probabilistic-satisfaction` backend's evidence kinds are defective
    /// (FR-075-AC-11).
    UnknownEvidenceKind(EvidenceKindsDefect),
}

impl RegistrationCause {
    /// The catalog cause under `invalid_capability`.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::DuplicateBackend => "duplicate-backend",
            Self::AbsentKind => "absent-kind",
            Self::UnknownKind(_) => "unknown-kind",
            Self::UnknownMode(_) => "unknown-mode",
            Self::InvalidDomains(_) => "invalid-domains",
            Self::InvalidFairnessKinds(_) => "invalid-fairness-kinds",
            Self::UnknownEvidenceKind(_) => "unknown-evidence-kind",
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
    fairness: BTreeMap<Candidate, BTreeSet<FairnessKind>>,
    missing_fairness: BTreeSet<FairnessKind>,
    evidence: Option<ItemEvidence>,
}

impl CandidateSet {
    /// The empty set of an item of `kind` that is routed to no engine
    /// whatever is registered (an item the request writer disposes
    /// `Unsupported`, FR-234).
    pub(crate) fn none(kind: Capability, named: Option<&BackendId>) -> Self {
        Self {
            kind,
            requested_backend: named.cloned(),
            candidates: Vec::new(),
            fairness: BTreeMap::new(),
            missing_fairness: BTreeSet::new(),
            evidence: None,
        }
    }

    /// The evidence value a `probabilistic-satisfaction` item named,
    /// preserved unchanged for negotiation (FR-075 "Candidate computation
    /// matches probabilistic evidence exactly"); `None` for an item of any
    /// other kind.
    pub fn evidence(&self) -> Option<&ItemEvidence> {
        self.evidence.as_ref()
    }

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

    /// The fairness kinds `candidate` advertises, carried unchanged from its
    /// registration (FR-134). `None` when `candidate` is not in this set.
    pub fn fairness_kinds(&self, candidate: &Candidate) -> Option<&BTreeSet<FairnessKind>> {
        self.fairness.get(candidate)
    }

    /// The fairness kinds that emptied this set (FR-134-AC-2): the requested
    /// kinds that a backend advertising the capability kind in scope lacks.
    /// Empty when the set is not empty, when the item uses no fairness kind,
    /// or when no backend advertises the capability kind at all (backend
    /// absence, FR-076).
    pub fn missing_fairness_kinds(&self) -> &BTreeSet<FairnessKind> {
        &self.missing_fairness
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
    ///   manifest digest, origin, advertised pairs, `domains`, fairness kinds,
    ///   memory models and evidence kinds), the repeat is one registration and
    ///   is not refused (FR-290-AC-9).
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
    /// values, and [`descriptor_from_manifest`] refuses such labels before a
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
        self.candidates_for_fairness(kind, named, &BTreeSet::new())
    }

    /// Compute `kind`'s candidate set for an item whose fairness set uses
    /// the kinds `required` (FR-134, ADR-019 DS-2): as [`Self::candidates`],
    /// leaving out every backend that does not advertise each kind in
    /// `required`. Each candidate carries its registration's fairness kinds
    /// unchanged. When the filter empties the set,
    /// [`CandidateSet::missing_fairness_kinds`] names the kinds that did it.
    pub fn candidates_for_fairness(
        &self,
        kind: Capability,
        named: Option<&BackendId>,
        required: &BTreeSet<FairnessKind>,
    ) -> CandidateOutcome {
        self.candidates_for_models(kind, named, required, &BTreeSet::new())
    }

    /// Compute `kind`'s candidate set for an item whose fairness set uses
    /// the kinds `required` and whose subject resolves its outermost
    /// `parallel` controls to the models `resolved` (FR-226, ADR-025 MX-7):
    /// as [`Self::candidates_for_fairness`], leaving out every backend that
    /// does not advertise each resolved model (a backend advertising no
    /// models advertises `sc` only). The
    /// models are never weakened to fit a backend; when none advertises
    /// them the set is empty, an ordinary set (FR-075).
    pub fn candidates_for_models(
        &self,
        kind: Capability,
        named: Option<&BackendId>,
        required: &BTreeSet<FairnessKind>,
        resolved: &BTreeSet<MemoryModelName>,
    ) -> CandidateOutcome {
        self.compute(kind, &ItemEvidence::Absent, named, required, resolved)
    }

    /// Compute the candidate set of a `probabilistic-satisfaction` item
    /// naming `evidence` (FR-075-AC-13): a backend is a candidate only when
    /// its evidence-kind set contains the item's exact evidence kind, so
    /// `statistical` never stands for `exact` or the reverse. An absent or
    /// unknown `evidence` gives no candidate and is preserved on the set
    /// ([`CandidateSet::evidence`]). This entry applies neither the fairness
    /// nor the memory-model filter, which a probabilistic item has none of: a
    /// `temporal-satisfaction` item goes through
    /// [`Self::candidates_for_models`], and any other `kind` here matches by
    /// kind alone.
    pub fn candidates_for_evidence(
        &self,
        kind: Capability,
        evidence: &ItemEvidence,
        named: Option<&BackendId>,
    ) -> CandidateOutcome {
        self.compute(kind, evidence, named, &BTreeSet::new(), &BTreeSet::new())
    }

    fn compute(
        &self,
        kind: Capability,
        evidence: &ItemEvidence,
        named: Option<&BackendId>,
        required: &BTreeSet<FairnessKind>,
        resolved: &BTreeSet<MemoryModelName>,
    ) -> CandidateOutcome {
        let in_scope: Vec<&BackendDescriptor> = match named {
            Some(id) => match self.held.get(id) {
                None => return CandidateOutcome::UnknownBackend(id.clone()),
                Some(descriptor) => vec![descriptor],
            },
            None => self.held.values().collect(),
        };
        let capable: Vec<&BackendDescriptor> = in_scope
            .into_iter()
            .filter(|descriptor| {
                descriptor.advertises_kind(kind)
                    && descriptor.serves_evidence(kind, evidence)
                    && descriptor.decides_models(resolved)
            })
            .collect();
        let (kept, left_out): (Vec<&BackendDescriptor>, Vec<&BackendDescriptor>) = capable
            .into_iter()
            .partition(|descriptor| required.is_subset(&descriptor.fairness_kinds));
        let missing_fairness = if kept.is_empty() {
            left_out
                .iter()
                .flat_map(|descriptor| required.difference(&descriptor.fairness_kinds).copied())
                .collect()
        } else {
            BTreeSet::new()
        };
        let mut candidates: Vec<Candidate> = kept
            .iter()
            .map(|descriptor| descriptor.backend.clone())
            .collect();
        candidates.sort();
        let fairness = kept
            .iter()
            .map(|descriptor| {
                (
                    descriptor.backend.clone(),
                    descriptor.fairness_kinds.clone(),
                )
            })
            .collect();
        CandidateOutcome::Candidates(CandidateSet {
            kind,
            requested_backend: named.cloned(),
            candidates,
            fairness,
            missing_fairness,
            evidence: matches!(kind, Capability::ProbabilisticSatisfaction)
                .then(|| evidence.clone()),
        })
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
            Some(BTreeSet::from([DomainKind::Collection])),
            None,
        )
        .expect("a well-formed descriptor")
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
