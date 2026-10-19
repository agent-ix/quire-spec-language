// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-134: the registry carries each backend's fairness kinds to its
//! candidates and leaves out a backend lacking a kind the item uses.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_foundation::BackendId;
use qsl_route::{
    BackendDescriptor, Candidate, CandidateOutcome, CandidateSet, ManifestDigest, Mode,
    ProviderOrigin, Registry,
};
use qsl_semantics::check::Capability;
use quire_semantic_value::fairness::FairnessKind;
use quire_semantic_value::fairness::FairnessKind::{Strong, Weak};

fn candidate(id: &str) -> Candidate {
    Candidate::new(BackendId::new(id))
}

/// A temporal backend advertising (`temporal-satisfaction`, `unbounded`)
/// and the given fairness kinds.
fn temporal(id: &str, seed: u8, kinds: &[FairnessKind]) -> BackendDescriptor {
    BackendDescriptor::new(
        candidate(id),
        ProviderOrigin::Linked,
        ManifestDigest::from_digest([seed; 32]),
        [(Capability::TemporalSatisfaction, Mode::Unbounded)],
        None,
    )
    .expect("a well-formed descriptor")
    .with_fairness_kinds(kinds.iter().copied())
}

fn registry(backends: impl IntoIterator<Item = BackendDescriptor>) -> Registry {
    let mut registry = Registry::new();
    for backend in backends {
        registry.register(backend).expect("registration admitted");
    }
    registry
}

fn set_for(registry: &Registry, required: &[FairnessKind]) -> CandidateSet {
    let required: BTreeSet<_> = required.iter().copied().collect();
    match registry.candidates_for_fairness(Capability::TemporalSatisfaction, None, &required) {
        CandidateOutcome::Candidates(set) => set,
        CandidateOutcome::UnknownBackend(id) => panic!("unknown backend {id}"),
    }
}

/// FR-134-AC-1: a weak constraint keeps both backends, each with its own
/// kinds; a strong constraint keeps only the backend advertising `Strong`.
#[test]
#[trace("TC-535", "FR-134-AC-1")]
fn a_backend_without_the_requested_kind_is_not_a_candidate() {
    let registry = registry([
        temporal("EN-1", 1, &[Weak, Strong]),
        temporal("weak-only", 2, &[Weak]),
    ]);

    let weak = set_for(&registry, &[Weak]);
    assert_eq!(
        weak.candidates(),
        [candidate("EN-1"), candidate("weak-only")]
    );
    assert_eq!(
        weak.fairness_kinds(&candidate("EN-1")),
        Some(&BTreeSet::from([Weak, Strong]))
    );
    assert_eq!(
        weak.fairness_kinds(&candidate("weak-only")),
        Some(&BTreeSet::from([Weak]))
    );

    let strong = set_for(&registry, &[Strong]);
    assert_eq!(strong.candidates(), [candidate("EN-1")]);
    assert_eq!(strong.fairness_kinds(&candidate("weak-only")), None);
}

/// FR-134-AC-1 (filter is by every kind the item uses): an item using both
/// kinds needs a backend advertising both.
#[test]
#[trace("TC-535", "FR-134-AC-1")]
fn every_kind_in_the_fairness_set_must_be_advertised() {
    let registry = registry([
        temporal("strong-only", 1, &[Strong]),
        temporal("both", 2, &[Weak, Strong]),
    ]);
    assert_eq!(
        set_for(&registry, &[Weak, Strong]).candidates(),
        [candidate("both")]
    );
}

/// FR-134 Behavior: an item that uses no fairness kind is filtered by no
/// kind, so a backend advertising none stays a candidate.
#[test]
fn an_item_without_fairness_keeps_every_capable_backend() {
    let registry = registry([
        temporal("plain", 1, &[]),
        temporal("EN-1", 2, &[Weak, Strong]),
    ]);
    let set = set_for(&registry, &[]);
    assert_eq!(set.candidates(), [candidate("EN-1"), candidate("plain")]);
    assert!(set.missing_fairness_kinds().is_empty());
}

/// FR-134-AC-2 (registry half): with only the weak backend, a strong item
/// has an empty set that names the missing `Strong` kind. Settling it
/// `unsupported` is `negotiate_*`'s work, not the registry's.
#[test]
#[trace("TC-535", "FR-134-AC-2")]
fn an_emptied_set_names_the_missing_fairness_kind() {
    let registry = registry([temporal("weak-only", 1, &[Weak])]);
    let set = set_for(&registry, &[Strong]);
    assert!(set.is_empty());
    assert_eq!(set.kind(), Capability::TemporalSatisfaction);
    assert_eq!(set.missing_fairness_kinds(), &BTreeSet::from([Strong]));
}

/// FR-076: an empty set from backend absence names no fairness kind, so it
/// is told apart from one the filter emptied.
#[test]
fn backend_absence_names_no_missing_fairness_kind() {
    let set = set_for(&Registry::new(), &[Strong]);
    assert!(set.is_empty());
    assert!(set.missing_fairness_kinds().is_empty());
}

/// Two registrations of one identity that differ only in fairness kinds
/// conflict like any other differing member.
#[test]
fn registrations_differing_in_fairness_kinds_conflict() {
    let mut registry = Registry::new();
    registry
        .register(temporal("EN-1", 1, &[Weak]))
        .expect("first admitted");
    assert!(registry
        .register(temporal("EN-1", 1, &[Weak, Strong]))
        .is_err());
    assert_eq!(registry.backends().count(), 0);
}
