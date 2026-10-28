// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-7's FR-331 provider manifest (FR-280): the data this engine publishes so
//! the registry can route items to it (QSL FR-246: "route it by negotiation
//! (FR-075), where EN-7 registers a provider manifest for that kind"). It is
//! data of this module, not of `qsl-route`, which holds the registry and
//! FR-288's conversion and no engine's manifest.
//!
//! EN-7 settles one (kind, mode) pair, `schedulability` in the `unbounded`
//! mode: a closed form decides a task set's claim in exact rationals and has
//! no bound to ask for, so it takes a bounded claim too (ADR-012 section 1.1).
//! The registry's `bounded` mode would oblige a boundable-domain set
//! (FR-290), which a task set has none of. It declares no domain, fairness
//! kind, evidence kind or memory model.

use qsl_route::{
    descriptor_from_manifest, BackendDescriptor, Candidate, ManifestDigest, ManifestMembers, Mode,
    ProviderOrigin, RegistrationRefusal,
};
use qsl_semantics::check::Capability;

/// EN-7's backend identity, the identity its terminal records name.
pub const BACKEND_ID: &str = "en-7";

/// The (kind, mode) pairs EN-7 advertises: exactly one.
pub const ADVERTISED: [(Option<&str>, Option<&str>); 1] = [(
    Some(Capability::Schedulability.to_wire()),
    Some(Mode::Unbounded.to_wire()),
)];

/// EN-7's routed descriptor, converted from [`ADVERTISED`] by FR-288's
/// conversion. `origin` and `manifest_digest` are the host's facts
/// (ADR-029 PV-1).
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
        ManifestMembers {
            domains: None,
            fairness_kinds: None,
            evidence_kinds: None,
            memory_models: None,
            reductions: None,
        },
    )
}
