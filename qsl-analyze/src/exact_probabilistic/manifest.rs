// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-5's FR-331 provider manifest (FR-280, ADR-028 SP-3): the data this
//! engine publishes so the registry can route items to it. It is data of
//! this module, not of `qsl-route`, which holds the registry and FR-288's
//! conversion and no engine's manifest.
//!
//! EN-5 settles one (kind, mode) pair, `probabilistic-satisfaction` over a
//! `unbounded` extent, with the evidence kind `exact` and no other: EN-4 holds
//! `statistical` for the same kind, and one claim requested with either kind
//! is two items (ADR-028 SP-3). Its `domains` are the boundable domains its
//! product explores: `population` and `integer`.

use qsl_foundation::bound::DomainKind;
use qsl_route::{
    descriptor_from_manifest, BackendDescriptor, Candidate, EvidenceKind, ManifestMembers, Mode,
    ProviderOrigin, RegistrationRefusal,
};
use qsl_semantics::check::Capability;

/// EN-5's backend identity, the identity its terminal records name.
pub const BACKEND_ID: &str = "en-5";

/// The (kind, mode) pairs EN-5 advertises: exactly one.
pub const ADVERTISED: [(Option<&str>, Option<&str>); 1] = [(
    Some(Capability::ProbabilisticSatisfaction.to_wire()),
    Some(Mode::Unbounded.to_wire()),
)];

/// The boundable domain kinds EN-5's product explores.
pub const DOMAINS: [&str; 2] = [
    DomainKind::Population.to_wire(),
    DomainKind::Integer.to_wire(),
];

/// The evidence kinds EN-5 settles: `exact` only.
pub const EVIDENCE_KINDS: [&str; 1] = [EvidenceKind::Exact.to_wire()];

/// EN-5's routed descriptor, converted from [`ADVERTISED`], [`DOMAINS`] and
/// [`EVIDENCE_KINDS`] by FR-288's conversion. `origin` is the host's fact
/// (ADR-029 PV-1).
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
            fairness_kinds: None,
            evidence_kinds: Some(&EVIDENCE_KINDS),
            memory_models: None,
            reductions: None,
        },
    )
}
