// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-057-AC-15: the one negotiation cause type and the settlement a
//! candidate outcome gives.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_foundation::bound::DomainKind;
use qsl_foundation::BackendId;
use qsl_route::{
    settle_candidates, BackendDescriptor, Candidate, EvidenceKind, Mode, NegotiationCause,
    NegotiationOutcome, NegotiationSettlement, ProviderOrigin, Registry,
};
use qsl_semantics::check::Capability;
use quire_semantic_value::catalog::CatalogCode;

const KIND: Capability = Capability::OperationContract;

fn backend(id: &str, kind: Capability) -> BackendDescriptor {
    BackendDescriptor::new(
        Candidate::new(BackendId::new(id)),
        ProviderOrigin::Linked,
        [(kind, Mode::Bounded)],
        Some(BTreeSet::from([DomainKind::Collection])),
        Some(BTreeSet::from([EvidenceKind::Statistical])),
    )
    .expect("a well-formed descriptor")
}

fn id(name: &str) -> BackendId {
    BackendId::new(name)
}

#[test]
#[trace("TC-155", "FR-057-AC-15")]
fn outcomes_spell_their_codes_and_causes_carry_payloads() {
    assert_eq!(
        NegotiationOutcome::UnknownBackend.code(),
        CatalogCode::new("invalid_capability", "unknown-backend")
    );
    assert_eq!(
        NegotiationOutcome::AmbiguousBackend.code(),
        CatalogCode::new("invalid_capability", "ambiguous-backend")
    );
    assert_eq!(
        NegotiationOutcome::UnsupportedRequestedCapability.code(),
        CatalogCode::new("unsupported_projection", "unsupported-requested-capability")
    );

    let causes = [
        (
            NegotiationCause::UnknownBackend(id("z")),
            NegotiationOutcome::UnknownBackend,
        ),
        (
            NegotiationCause::AmbiguousBackend(vec![id("a"), id("b")]),
            NegotiationOutcome::AmbiguousBackend,
        ),
        (
            NegotiationCause::UnsupportedRequestedCapability(KIND, Some(id("a"))),
            NegotiationOutcome::UnsupportedRequestedCapability,
        ),
    ];
    for (cause, outcome) in causes {
        assert_eq!(cause.outcome(), outcome);
        assert_eq!(cause.code(), outcome.code());
    }
}

#[test]
#[trace("TC-155", "FR-057-AC-15")]
fn settle_candidates_gives_the_table_row_for_each_outcome() {
    let mut registry = Registry::new();
    registry.register(backend("a", KIND)).unwrap();
    registry.register(backend("b", KIND)).unwrap();
    registry
        .register(backend("c", Capability::GlobalConformance))
        .unwrap();

    let settle = |named: Option<&BackendId>| {
        settle_candidates(&registry.candidates(KIND, named), KIND, named)
    };

    assert_eq!(
        settle(None),
        NegotiationSettlement::Cause(NegotiationCause::AmbiguousBackend(vec![id("a"), id("b")]))
    );
    assert_eq!(
        settle(Some(&id("a"))),
        NegotiationSettlement::Single(id("a"))
    );
    assert_eq!(
        settle(Some(&id("c"))),
        NegotiationSettlement::Cause(NegotiationCause::UnsupportedRequestedCapability(
            KIND,
            Some(id("c"))
        ))
    );
    assert_eq!(
        settle(Some(&id("z"))),
        NegotiationSettlement::Cause(NegotiationCause::UnknownBackend(id("z")))
    );

    // Two candidates with a named backend: no cause.
    let two = registry.candidates(KIND, None);
    assert_eq!(
        settle_candidates(&two, KIND, Some(&id("a"))),
        NegotiationSettlement::Inconsistent
    );
}
