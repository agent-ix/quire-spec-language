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
use qsl_route::{
    descriptor_from_manifest, BackendDescriptor, Candidate, FairnessKindsMember, FairnessLabel,
    ManifestDigest, Mode, ProviderOrigin, RegistrationRefusal,
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

/// EN-1's routed descriptor, converted from [`ADVERTISED`], [`DOMAINS`] and
/// [`FAIRNESS_KINDS`] by FR-288's conversion. `origin` and `manifest_digest`
/// are the host's facts (ADR-029 PV-1).
///
/// # Errors
///
/// Only if the registry's admission rules refuse this module's own data, which
/// the tests of this module rule out.
pub fn descriptor(
    origin: ProviderOrigin,
    manifest_digest: ManifestDigest,
) -> Result<BackendDescriptor, RegistrationRefusal> {
    descriptor_from_manifest(
        Candidate::new(qsl_foundation::BackendId::new(BACKEND_ID)),
        origin,
        manifest_digest,
        ADVERTISED,
        Some(&DOMAINS),
        Some(FairnessKindsMember::List(&FAIRNESS_KINDS)),
        None,
    )
}
