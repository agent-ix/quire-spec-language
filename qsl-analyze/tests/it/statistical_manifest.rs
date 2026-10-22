// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-4's published provider manifest (FR-280): converted by FR-288's
//! conversion into a descriptor that advertises exactly
//! (`probabilistic-satisfaction`, `bounded`), the evidence kind
//! `statistical` and the domains `population` and `integer`, and that the
//! registry routes a `statistical` item to and an `exact` item away from.

use ix_trace_rs::trace;
use qsl_analyze::statistical::manifest::{self, BACKEND_ID};
use qsl_foundation::bound::DomainKind;
use qsl_foundation::BackendId;
use qsl_route::{
    descriptor_from_manifest, Candidate, CandidateOutcome, EvidenceKind, ItemEvidence,
    ManifestDigest, Mode, ProviderOrigin, Registry,
};
use qsl_semantics::check::Capability;
use std::collections::BTreeSet;

fn en4() -> qsl_route::BackendDescriptor {
    manifest::descriptor(ProviderOrigin::Linked, ManifestDigest::from_digest([4; 32]))
        .expect("EN-4's own manifest is admitted")
}

fn exact_backend() -> qsl_route::BackendDescriptor {
    descriptor_from_manifest(
        Candidate::new(BackendId::new("exact-engine")),
        ProviderOrigin::Linked,
        ManifestDigest::from_digest([5; 32]),
        [(
            Some(Capability::ProbabilisticSatisfaction.to_wire()),
            Some(Mode::Bounded.to_wire()),
        )],
        Some(&[DomainKind::Population.to_wire()]),
        None,
        Some(&[EvidenceKind::Exact.to_wire()]),
    )
    .expect("a well-formed exact backend")
}

fn candidates(registry: &Registry, evidence: EvidenceKind) -> Vec<String> {
    let CandidateOutcome::Candidates(set) = registry.candidates_for_evidence(
        Capability::ProbabilisticSatisfaction,
        &ItemEvidence::Kind(evidence),
        None,
    ) else {
        panic!("expected a computed candidate set");
    };
    set.candidates()
        .iter()
        .map(|candidate| candidate.id().as_str().to_owned())
        .collect()
}

#[test]
#[trace("TC-761", "FR-280-AC-4")]
fn en4_advertises_exactly_the_statistical_probabilistic_bounded_pair() {
    let descriptor = en4();
    assert_eq!(descriptor.id().as_str(), BACKEND_ID);
    assert_eq!(
        descriptor.advertises().collect::<Vec<_>>(),
        [(Capability::ProbabilisticSatisfaction, Mode::Bounded)]
    );
    assert_eq!(
        descriptor.evidence_kinds(),
        &BTreeSet::from([EvidenceKind::Statistical])
    );
    assert_eq!(
        descriptor.domains(),
        Some(&BTreeSet::from([
            DomainKind::Population,
            DomainKind::Integer
        ]))
    );
}

#[test]
#[trace("TC-761", "FR-280-AC-4")]
fn a_registry_routes_by_evidence_kind_between_en4_and_an_exact_backend() {
    let mut registry = Registry::new();
    registry.register(en4()).unwrap();
    registry.register(exact_backend()).unwrap();
    assert_eq!(
        candidates(&registry, EvidenceKind::Statistical),
        [BACKEND_ID]
    );
    assert_eq!(candidates(&registry, EvidenceKind::Exact), ["exact-engine"]);
}

/// The origin is the host's fact: it changes the descriptor's origin and
/// nothing EN-4 publishes.
#[test]
#[trace("TC-761", "FR-280-AC-4")]
fn the_published_manifest_is_independent_of_the_provider_origin() {
    let digest = ManifestDigest::from_digest([4; 32]);
    let linked = manifest::descriptor(ProviderOrigin::Linked, digest).unwrap();
    let process = manifest::descriptor(ProviderOrigin::Process, digest).unwrap();
    assert_eq!(linked.evidence_kinds(), process.evidence_kinds());
    assert_eq!(linked.domains(), process.domains());
    assert_ne!(linked, process);
}
