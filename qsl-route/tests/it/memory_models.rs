// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-226 (TC-671 step 3): the `route` candidate step gives an item over a
//! subject with a weak `parallel` only backends advertising its resolved
//! model, and never weakens the model to fit a backend.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_foundation::BackendId;
use qsl_foundation::MemoryModelName::{self, Ra, Sc, Tso};
use qsl_route::{
    BackendDescriptor, Candidate, CandidateOutcome, ManifestDigest, Mode, ProviderOrigin, Registry,
};
use qsl_semantics::check::Capability;
use quire_semantic_value::fairness::FairnessKind::Weak;

fn candidate(id: &str) -> Candidate {
    Candidate::new(BackendId::new(id))
}

/// A temporal backend advertising weak fairness and `models`.
fn temporal(id: &str, seed: u8, models: &[MemoryModelName]) -> BackendDescriptor {
    BackendDescriptor::new(
        candidate(id),
        ProviderOrigin::Linked,
        ManifestDigest::from_digest([seed; 32]),
        [(Capability::TemporalSatisfaction, Mode::Unbounded)],
        None,
    )
    .expect("a well-formed descriptor")
    .with_fairness_kinds([Weak])
    .with_memory_models(models.iter().copied())
}

fn registry(backends: impl IntoIterator<Item = BackendDescriptor>) -> Registry {
    let mut registry = Registry::new();
    for backend in backends {
        registry.register(backend).expect("registration admitted");
    }
    registry
}

/// The candidates of a temporal item over `SB`, whose one `parallel`
/// resolves to `model`.
fn candidates(registry: &Registry, model: MemoryModelName) -> Vec<Candidate> {
    match registry.candidates_for_models(
        Capability::TemporalSatisfaction,
        None,
        &BTreeSet::from([Weak]),
        &BTreeSet::from([model]),
    ) {
        CandidateOutcome::Candidates(set) => set.candidates().to_vec(),
        CandidateOutcome::UnknownBackend(id) => panic!("unknown backend {id}"),
    }
}

/// FR-226-AC-3's candidate step: with EN-1 (`sc`, `tso`, `ra`) and a test
/// candidate advertising only `sc`, the `tso` item over `SB` has `{EN-1}`;
/// with the test candidate alone it has none, and the `sc` request has
/// `{test candidate}`. A backend advertising no model still decides an
/// item whose subject resolves only to `sc`.
#[trace("TC-671")]
#[test]
fn a_weak_item_is_given_only_backends_advertising_its_resolved_model() {
    let en1 = || temporal("EN-1", 1, &[Sc, Tso, Ra]);
    let sc_only = || temporal("sc-only", 2, &[Sc]);

    let both = registry([en1(), sc_only()]);
    assert_eq!(candidates(&both, Tso), [candidate("EN-1")]);
    assert_eq!(candidates(&both, Ra), [candidate("EN-1")]);
    assert_eq!(
        candidates(&both, Sc),
        [candidate("EN-1"), candidate("sc-only")]
    );

    let alone = registry([sc_only()]);
    assert_eq!(candidates(&alone, Tso), []);
    assert_eq!(candidates(&alone, Sc), [candidate("sc-only")]);

    let unadvertised = registry([temporal("plain", 3, &[])]);
    assert_eq!(candidates(&unadvertised, Sc), [candidate("plain")]);
    assert_eq!(candidates(&unadvertised, Tso), []);
}
