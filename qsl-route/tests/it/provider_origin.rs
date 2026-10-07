// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-288-AC-5 and FR-288-AC-6: a descriptor carries the provider's origin,
//! and two registrations of one identity that differ only in origin
//! conflict.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_foundation::bound::DomainKind;
use qsl_route::{
    BackendDescriptor, BackendId, Candidate, CandidateOutcome, ManifestDigest, Mode,
    ProviderOrigin, RegistrationCause, Registry,
};
use qsl_semantics::check::Capability;

fn manifest_digest() -> ManifestDigest {
    ManifestDigest::from_digest([7; 32])
}

fn kani(origin: ProviderOrigin) -> BackendDescriptor {
    BackendDescriptor::new(
        Candidate::new(BackendId::new("kani")),
        origin,
        manifest_digest(),
        [(Capability::OperationContract, Mode::Bounded)],
        Some(BTreeSet::from([DomainKind::Collection])),
    )
    .expect("a well-formed descriptor")
}

/// FR-288-AC-5: the origin the host supplies is the one the descriptor
/// holds, through both constructors, and the identity is unchanged by it.
#[test]
#[trace("TC-772", "FR-288-AC-5")]
fn descriptor_holds_the_origin_it_was_built_with() {
    let linked = kani(ProviderOrigin::Linked);
    let process = kani(ProviderOrigin::Process);
    assert_eq!(linked.origin(), ProviderOrigin::Linked);
    assert_eq!(process.origin(), ProviderOrigin::Process);
    assert_eq!(linked.id(), process.id());
    assert_ne!(linked, process);

    let admitted = BackendDescriptor::admit(
        Candidate::new(BackendId::new("plug")),
        ProviderOrigin::Process,
        manifest_digest(),
        [(Some("value-validity"), Some("bounded"))],
        Some(&["collection"]),
    )
    .expect("a well-formed manifest is admitted");
    assert_eq!(admitted.origin(), ProviderOrigin::Process);
}

/// FR-288-AC-6: Linked and Process registrations of one identity conflict
/// in either order; both are withdrawn and the one manifest digest is
/// recorded.
#[test]
#[trace("TC-772", "FR-288-AC-6")]
fn origin_differing_registrations_conflict_in_either_order() {
    for linked_first in [true, false] {
        let (first, second) = if linked_first {
            (kani(ProviderOrigin::Linked), kani(ProviderOrigin::Process))
        } else {
            (kani(ProviderOrigin::Process), kani(ProviderOrigin::Linked))
        };
        let mut registry = Registry::new();
        registry.register(first).expect("first registration holds");
        let refusals = registry
            .register(second)
            .expect_err("a differing origin conflicts");
        assert_eq!(refusals.len(), 1);
        assert_eq!(refusals[0].cause(), &RegistrationCause::DuplicateBackend);
        assert_eq!(refusals[0].manifest_digest(), manifest_digest());
        assert_eq!(registry.descriptors().count(), 0);
        assert!(matches!(
            registry.candidates(Capability::OperationContract, Some(&BackendId::new("kani"))),
            CandidateOutcome::UnknownBackend(_)
        ));
    }
}

/// FR-288-AC-6: a byte-identical registration, origin included, stays
/// idempotent.
#[test]
#[trace("TC-772", "FR-288-AC-6")]
fn identical_origin_registration_is_idempotent() {
    for origin in [ProviderOrigin::Linked, ProviderOrigin::Process] {
        let mut registry = Registry::new();
        registry.register(kani(origin)).expect("first registration");
        registry
            .register(kani(origin))
            .expect("an identical repeat");
        assert_eq!(registry.descriptors().count(), 1);
        assert_eq!(registry.refusals().count(), 0);
    }
}
