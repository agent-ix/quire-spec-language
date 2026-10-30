// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-1's FR-331 provider manifest (FR-280): the data this engine publishes
//! so the registry can route items to it. It is data of this module, not of
//! `qsl-route`, which holds the registry and FR-288's conversion and no
//! engine's manifest.
//!
//! EN-1 settles one (kind, mode) pair, `temporal-satisfaction` over a
//! `bounded` domain: the reachable invariants, bounded clauses, liveness and
//! deadlock-freedom items its explicit-state search decides over a finite
//! subject (ADR-018). It decides under weak and strong fairness. Its
//! `domains` are the boundable domains the subject's universes bound:
//! `population` and `integer`. Every label is spelled by the type that owns
//! it, so this module restates no wire string but the two fairness kinds
//! `qsl-route` reads by their literal spelling.

use qsl_foundation::bound::DomainKind;
use qsl_foundation::{MemoryModelName, ReductionKind};
use qsl_route::{
    descriptor_from_manifest, BackendDescriptor, Candidate, FairnessKindsMember, FairnessLabel,
    ManifestMembers, Mode, ProviderOrigin, RegistrationRefusal,
};
use qsl_semantics::check::Capability;

/// EN-1's backend identity, the identity its terminal records name.
pub const BACKEND_ID: &str = "en-1";

/// The (kind, mode) pairs EN-1 advertises: exactly one.
pub const ADVERTISED: [(Option<&str>, Option<&str>); 1] = [(
    Some(Capability::TemporalSatisfaction.to_wire()),
    Some(Mode::Bounded.to_wire()),
)];

/// The boundable domain kinds EN-1's subject universes bound.
pub const DOMAINS: [&str; 2] = [
    DomainKind::Population.to_wire(),
    DomainKind::Integer.to_wire(),
];

/// The fairness kinds EN-1 decides under (ADR-019).
pub const FAIRNESS_KINDS: [FairnessLabel<'static>; 2] =
    [FairnessLabel::Text("weak"), FairnessLabel::Text("strong")];

/// The memory models EN-1 decides (FR-226, QSpec FR-331-AC-30).
pub const MEMORY_MODELS: [MemoryModelName; 3] = [
    MemoryModelName::Sc,
    MemoryModelName::Tso,
    MemoryModelName::Ra,
];

/// The reductions EN-1 applies (FR-159 "Capability"), as QSpec FR-382's
/// `reductions` member spells them.
pub const REDUCTIONS: [&str; 3] = [
    ReductionKind::Symmetry.as_str(),
    ReductionKind::PartialOrder.as_str(),
    ReductionKind::StateConstraint.as_str(),
];

/// EN-1's routed descriptor, converted from [`ADVERTISED`], [`DOMAINS`],
/// [`FAIRNESS_KINDS`], [`MEMORY_MODELS`] and [`REDUCTIONS`] by FR-288's
/// conversion. `origin` is the host's fact (ADR-029 PV-1).
///
/// # Errors
///
/// Only if the registry's admission rules refuse this module's own data, which
/// the tests of this module rule out.
pub fn descriptor(origin: ProviderOrigin) -> Result<BackendDescriptor, RegistrationRefusal> {
    descriptor_from_manifest(
        Candidate::new(qsl_foundation::BackendId::new(BACKEND_ID)),
        origin,
        ADVERTISED,
        ManifestMembers {
            domains: Some(&DOMAINS),
            fairness_kinds: Some(FairnessKindsMember::List(&FAIRNESS_KINDS)),
            evidence_kinds: None,
            memory_models: Some(&MEMORY_MODELS),
            reductions: Some(&REDUCTIONS),
        },
    )
}
