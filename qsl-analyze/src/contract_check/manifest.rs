// SPDX-License-Identifier: AGPL-3.0-or-later
//! The operation-contract check arm's FR-331 provider manifest (QSpec FR-483
//! "Descriptor modes, domains and registration").
//!
//! The arm settles one (kind, mode) pair, `operation-contract` over a
//! `bounded` domain: it evaluates every case of the subject's finite
//! universes, so it advertises no `unbounded` mode. Its `domains` are the
//! boundable kinds it substitutes, `population` only (a population is bound
//! by the subject's universe of keys), and its `bounds` publish the default
//! `max_cases`.

use std::num::NonZeroU64;

use qsl_foundation::bound::DomainKind;
use qsl_route::{
    descriptor_from_manifest, BackendDescriptor, Candidate, ManifestDigest, ManifestMembers, Mode,
    ProviderOrigin, RegistrationRefusal,
};
use qsl_semantics::check::Capability;

/// The arm's backend identity, the identity its terminal records name.
pub const BACKEND_ID: &str = "contract-checker";

/// The (kind, mode) pairs the arm advertises: exactly one.
pub const ADVERTISED: [(Option<&str>, Option<&str>); 1] = [(
    Some(Capability::OperationContract.to_wire()),
    Some(Mode::Bounded.to_wire()),
)];

/// The boundable domain kinds the arm substitutes.
pub const DOMAINS: [&str; 1] = [DomainKind::Population.to_wire()];

/// The published default for `max_cases` (QSpec FR-483): the cases an item
/// without its own `check.limits.max_cases` may examine.
pub const DEFAULT_MAX_CASES: NonZeroU64 = NonZeroU64::new(65_536).unwrap();

/// The arm's routed descriptor, converted by FR-288's conversion, with its
/// published `max_cases` default.
///
/// # Errors
///
/// Only if the registry's admission rules refuse this module's own data,
/// which the tests of this module rule out.
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
            domains: Some(&DOMAINS),
            fairness_kinds: None,
            evidence_kinds: None,
            memory_models: None,
            reductions: None,
        },
    )
    .map(|descriptor| descriptor.with_max_cases(DEFAULT_MAX_CASES))
}
