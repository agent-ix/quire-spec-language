// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-4's FR-331 provider manifest (FR-280): the data this engine publishes
//! so the registry can route items to it. It is data of this module, not of
//! `qsl-route`, which holds the registry and FR-288's conversion and no
//! engine's manifest.
//!
//! EN-4 settles one (kind, mode) pair, `probabilistic-satisfaction` over a
//! `bounded` domain, with the evidence kind `statistical` and no other
//! (ADR-024 SV-9). Its `domains` are the boundable domains its sampler draws
//! over: `population` and `integer`. Every label is spelled by the type that
//! owns it, so this module restates no wire string.
//!
//! The driver reads the manifest and supplies the host-side facts the
//! manifest does not carry, the [`ProviderOrigin`] and the manifest digest;
//! [`descriptor`] converts the rest through FR-288's one conversion.

use qsl_foundation::bound::DomainKind;
use qsl_route::{
    descriptor_from_manifest, BackendDescriptor, Candidate, EvidenceKind, ManifestDigest, Mode,
    ProviderOrigin, RegistrationRefusal,
};
use qsl_semantics::check::Capability;

/// EN-4's backend identity, the identity its terminal records name.
pub const BACKEND_ID: &str = "en-4";

/// The (kind, mode) pairs EN-4 advertises: exactly one.
pub const ADVERTISED: [(Option<&str>, Option<&str>); 1] = [(
    Some(Capability::ProbabilisticSatisfaction.to_wire()),
    Some(Mode::Bounded.to_wire()),
)];

/// The boundable domain kinds EN-4's sampler draws over.
pub const DOMAINS: [&str; 2] = [
    DomainKind::Population.to_wire(),
    DomainKind::Integer.to_wire(),
];

/// The evidence kinds EN-4 settles: `statistical` only.
pub const EVIDENCE_KINDS: [&str; 1] = [EvidenceKind::Statistical.to_wire()];

/// EN-4's routed descriptor, converted from [`ADVERTISED`], [`DOMAINS`] and
/// [`EVIDENCE_KINDS`] by FR-288's conversion. `origin` and `manifest_digest`
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
        None,
        Some(&EVIDENCE_KINDS),
    )
}
