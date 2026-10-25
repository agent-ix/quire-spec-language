// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-226 (TC-671 step 3): the `route` candidate step gives an item over a
//! subject with a weak `parallel` only backends advertising its resolved
//! model, and never weakens the model to fit a backend.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_foundation::BackendId;
use qsl_foundation::MemoryModelName::{self, Ra, Sc, Tso};
use qsl_route::{
    descriptor_from_manifest, BackendDescriptor, Candidate, CandidateOutcome, FairnessKindsMember,
    FairnessLabel, ManifestDigest, ManifestMembers, Mode, ProviderOrigin, Registry,
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
/// `{test candidate}`. A backend whose descriptor carries no memory models
/// advertises `sc` only: it decides the `sc` request and no `tso` or `ra`
/// one.
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
    assert_eq!(candidates(&unadvertised, Ra), []);
    let tso_only = registry([temporal("tso-only", 4, &[Tso])]);
    assert_eq!(candidates(&tso_only, Sc), []);
}

/// FR-226-AC-2 (TC-671 step 2) at the manifest conversion: a temporal
/// manifest carrying `memory_models` advertises exactly them, and one
/// without the member advertises `sc` only, so it is no candidate for a
/// `tso` or `ra` item.
#[trace("TC-671", "FR-226-AC-2")]
#[test]
fn a_manifest_memory_models_member_is_read_and_its_absence_advertises_sc_only() {
    let converted = |id: &str, seed: u8, models: Option<&[MemoryModelName]>| {
        descriptor_from_manifest(
            candidate(id),
            ProviderOrigin::Linked,
            ManifestDigest::from_digest([seed; 32]),
            [(Some("temporal-satisfaction"), Some("unbounded"))],
            ManifestMembers {
                domains: None,
                fairness_kinds: Some(FairnessKindsMember::List(&[FairnessLabel::Text("weak")])),
                evidence_kinds: None,
                memory_models: models,
            },
        )
        .expect("a well-formed manifest")
    };
    let with = converted("with", 1, Some(&[Sc, Tso, Ra]));
    assert_eq!(with.memory_models(), &BTreeSet::from([Sc, Tso, Ra]));
    let without = converted("without", 2, None);
    assert!(without.memory_models().is_empty());

    let routed = registry([with, without]);
    assert_eq!(
        candidates(&routed, Sc),
        [candidate("with"), candidate("without")]
    );
    assert_eq!(candidates(&routed, Tso), [candidate("with")]);
    assert_eq!(candidates(&routed, Ra), [candidate("with")]);
}
