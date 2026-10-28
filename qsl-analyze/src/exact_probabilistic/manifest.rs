// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-5's FR-331 provider manifest (FR-280): exact bounded and unbounded
//! probabilistic satisfaction, registered separately from EN-4's statistical
//! evidence.

use qsl_foundation::bound::DomainKind;
use qsl_route::{
    descriptor_from_manifest, BackendDescriptor, Candidate, EvidenceKind, ManifestDigest,
    ManifestMembers, Mode, ProviderOrigin, RegistrationRefusal,
};
use qsl_semantics::check::Capability;

/// EN-5's backend identity.
pub const BACKEND_ID: &str = "en-5";

/// The probabilistic modes EN-5 handles with exact evidence.
pub const ADVERTISED: [(Option<&str>, Option<&str>); 2] = [
    (
        Some(Capability::ProbabilisticSatisfaction.to_wire()),
        Some(Mode::Bounded.to_wire()),
    ),
    (
        Some(Capability::ProbabilisticSatisfaction.to_wire()),
        Some(Mode::Unbounded.to_wire()),
    ),
];

/// The finite domains EN-5 can enumerate for a bounded claim's `over` set.
pub const DOMAINS: [&str; 2] = [
    DomainKind::Population.to_wire(),
    DomainKind::Integer.to_wire(),
];

/// The evidence EN-5 produces.
pub const EVIDENCE_KINDS: [&str; 1] = [EvidenceKind::Exact.to_wire()];

/// EN-5's linked provider descriptor.
///
/// # Errors
///
/// Returns only when EN-5's own manifest violates FR-288 registration rules.
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
            evidence_kinds: Some(&EVIDENCE_KINDS),
            memory_models: None,
            reductions: None,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn en5_advertises_exact_probabilistic_work_in_both_modes() {
        let descriptor = descriptor(ProviderOrigin::Linked, ManifestDigest::from_digest([0; 32]))
            .expect("EN-5's manifest is admitted");
        assert_eq!(descriptor.id().as_str(), BACKEND_ID);
        let advertised = descriptor.advertises().collect::<Vec<_>>();
        assert_eq!(advertised.len(), 2);
        assert!(advertised.contains(&(Capability::ProbabilisticSatisfaction, Mode::Bounded)));
        assert!(advertised.contains(&(Capability::ProbabilisticSatisfaction, Mode::Unbounded)));
        assert!(descriptor.evidence_kinds().contains(&EvidenceKind::Exact));
    }
}
