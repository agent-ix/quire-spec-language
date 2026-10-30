// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-6's FR-331 provider manifest (FR-280): the data the zone engine
//! publishes so the registry can route items to it, converted by FR-288's
//! conversion as EN-1's is ([`crate::model_check::manifest`]).
//!
//! EN-6 settles `temporal-satisfaction` over a `bounded` domain: the timed
//! claims, deadlock-freedom and time-lock-freedom items it decides on zones
//! over a subject whose universes bound its populations (FR-239). It
//! decides under weak and strong fairness read over time (FR-240), and
//! applies symmetry and no other reduction (QSpec FR-430).

use qsl_foundation::bound::DomainKind;
use qsl_foundation::ReductionKind;
use qsl_route::{
    descriptor_from_manifest, BackendDescriptor, Candidate, FairnessKindsMember, FairnessLabel,
    ManifestMembers, Mode, ProviderOrigin, RegistrationRefusal,
};
use qsl_semantics::check::Capability;

/// EN-6's backend identity, the identity its terminal records name.
pub const BACKEND_ID: &str = "en-6";

/// The (kind, mode) pairs EN-6 advertises: exactly one.
pub const ADVERTISED: [(Option<&str>, Option<&str>); 1] = [(
    Some(Capability::TemporalSatisfaction.to_wire()),
    Some(Mode::Bounded.to_wire()),
)];

/// The boundable domain kinds EN-6's subject universes bound.
pub const DOMAINS: [&str; 2] = [
    DomainKind::Population.to_wire(),
    DomainKind::Integer.to_wire(),
];

/// The fairness kinds EN-6 decides under, read over time (FR-240).
pub const FAIRNESS_KINDS: [FairnessLabel<'static>; 2] =
    [FairnessLabel::Text("weak"), FairnessLabel::Text("strong")];

/// The reductions EN-6 applies: symmetry only (QSpec FR-430).
pub const REDUCTIONS: [&str; 1] = [ReductionKind::Symmetry.as_str()];

/// EN-6's routed descriptor, converted from [`ADVERTISED`], [`DOMAINS`],
/// [`FAIRNESS_KINDS`] and [`REDUCTIONS`] by FR-288's conversion, and
/// marked as deciding a `time dense` subject, so an item over one routes to
/// EN-6 alone (QSL FR-243, FR-430). `origin` is the host's fact (ADR-029 PV-1).
///
/// # Errors
///
/// Only if the registry's admission rules refuse this module's own data,
/// which the tests of this module rule out.
pub fn descriptor(origin: ProviderOrigin) -> Result<BackendDescriptor, RegistrationRefusal> {
    descriptor_from_manifest(
        Candidate::new(qsl_foundation::BackendId::new(BACKEND_ID)),
        origin,
        ADVERTISED,
        ManifestMembers {
            domains: Some(&DOMAINS),
            fairness_kinds: Some(FairnessKindsMember::List(&FAIRNESS_KINDS)),
            evidence_kinds: None,
            memory_models: None,
            reductions: Some(&REDUCTIONS),
        },
    )
    .map(BackendDescriptor::with_timed)
}
