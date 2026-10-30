// SPDX-License-Identifier: AGPL-3.0-or-later
//! The value-validity check arm's FR-331 provider manifest (QSpec FR-483
//! "Descriptor modes, domains and registration").
//!
//! The arm settles one (kind, mode) pair, `value-validity` over a `bounded`
//! domain: it evaluates every assignment of a finite domain, so it advertises
//! no `unbounded` mode. Its `domains` are the boundable kinds it substitutes,
//! `integer` only, and its `bounds` publish the default `max_cases`.

use std::num::NonZeroU64;

use qsl_foundation::bound::DomainKind;
use qsl_route::{
    descriptor_from_manifest_with_max_cases, BackendDescriptor, Candidate, ManifestMembers,
    MaxCasesMember, Mode, ProviderOrigin, RegistrationRefusal,
};
use qsl_semantics::check::Capability;

/// The arm's backend identity, the identity its terminal records name.
pub const BACKEND_ID: &str = "value-checker";

/// The (kind, mode) pairs the arm advertises: exactly one.
pub const ADVERTISED: [(Option<&str>, Option<&str>); 1] = [(
    Some(Capability::ValueValidity.to_wire()),
    Some(Mode::Bounded.to_wire()),
)];

/// The boundable domain kinds the arm substitutes.
pub const DOMAINS: [&str; 1] = [DomainKind::Integer.to_wire()];

/// The published default for `max_cases` (QSpec FR-483): the cases an item
/// without its own `check.limits.max_cases` may examine.
pub const DEFAULT_MAX_CASES: NonZeroU64 = NonZeroU64::new(1_048_576).unwrap();

/// The arm's routed descriptor with its published `max_cases` default,
/// converted by FR-288's conversion for a check arm, which refuses a
/// descriptor without a positive default (QSpec FR-483).
///
/// # Errors
///
/// Only if the registry's admission rules refuse this module's own data,
/// which the tests of this module rule out.
pub fn descriptor(origin: ProviderOrigin) -> Result<BackendDescriptor, RegistrationRefusal> {
    descriptor_with(origin, DEFAULT_MAX_CASES)
}

/// As [`descriptor`], publishing `max_cases` as the default.
///
/// # Errors
///
/// As [`descriptor`].
pub fn descriptor_with(
    origin: ProviderOrigin,
    max_cases: NonZeroU64,
) -> Result<BackendDescriptor, RegistrationRefusal> {
    descriptor_from_manifest_with_max_cases(
        Candidate::new(qsl_foundation::BackendId::new(BACKEND_ID)),
        origin,
        ADVERTISED,
        ManifestMembers {
            domains: Some(&DOMAINS),
            fairness_kinds: None,
            evidence_kinds: None,
            memory_models: None,
            reductions: None,
        },
        MaxCasesMember::Count(max_cases.get()),
    )
}
