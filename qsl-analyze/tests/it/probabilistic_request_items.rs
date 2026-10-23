// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1163 (FR-195-AC-6, FR-075-AC-18): S3 records each probabilistic
//! claim's evidence kind on its requirement record, and FR-075's request
//! builder writes items that name it, the one kind a request selects, or
//! each kind once when it selects both.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_analyze::statistical::manifest;
use qsl_foundation::bound::{DomainKey, DomainKind};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::BackendId;
use qsl_route::request::{items_with_evidence, EvidenceSelection, RequirementItem};
use qsl_route::{
    descriptor_from_manifest, Candidate, CandidateOutcome, EvidenceKind, ManifestDigest, Mode,
    ProviderOrigin, Registry,
};
use qsl_semantics::check::{Capability, RequirementRecord};
use qsl_semantics::family::{ClaimEvidence, ClaimExtent, Requirements};
use quire_exact::{Origin, Role};

use crate::prob_world::{self, World};

const CLAIMS: &str = "probabilistic P95 using ev on M over (s: M::Server) under Steady on origin \
     with alpha 0.01, beta 0.01, indifference 0.01 \
     { quantile 0.95 of (accumulate duration from holds(s.phase = 1) \
     until holds(s.phase = 2) within 10) <= 2 }\n\
     probabilistic Every using ev on M over (s: M::Server) under every scheduler \
     on origin with alpha 0.01, beta 0.01, indifference 0.01 \
     { probability >= 0.5 [ eventually[0,3] holds(s.phase = 2) ] }\n\
     probabilistic Bare using ev on M over (s: M::Server) under Steady on origin \
     { probability >= 0.5 [ eventually[0,3] holds(s.phase = 2) ] }\n\
     probabilistic Expected using inf on M over (s: M::Server) under Steady on origin \
     { expected accumulate duration until holds(s.phase = 2) <= 5 }";

fn node(fill: u8) -> WireNodeId {
    WireNodeId::from_digest([fill; 32])
}

fn key(fill: u8) -> OccurrenceKey {
    OccurrenceKey::new(node(fill), Origin::new(Role::new("claim"), 0))
}

/// The record S3 writes for the claim `name` of `world`, keyed by `fill`.
fn record(world: &World, name: &str, fill: u8) -> RequirementRecord {
    RequirementRecord::model_claim(world.claim(name).requirements(node(fill)), node(99))
}

fn evidence_of(record: &RequirementRecord) -> Option<ClaimEvidence> {
    record.requirements().evidence()
}

/// Checks FR-195-AC-6 (TC-1163 step 1): each probabilistic claim has a
/// `probabilistic-satisfaction` record, unbounded over a domain naming the
/// claim, with a `Boolean` result bound, no conversion context and an empty
/// path condition; `P95` records `statistical`, and a claim under every
/// scheduler, one with no confidence parameters and an expected-reward form
/// record `exact`.
#[trace("TC-1163", "FR-195-AC-6")]
#[test]
fn each_probabilistic_claim_records_its_evidence_kind() {
    let world = World::new(&prob_world::service(CLAIMS));
    for (name, evidence) in [
        ("P95", ClaimEvidence::Statistical),
        ("Every", ClaimEvidence::Exact),
        ("Bare", ClaimEvidence::Exact),
        ("Expected", ClaimEvidence::Exact),
    ] {
        let record = record(&world, name, 1);
        let requirements = record.requirements();
        assert_eq!(
            requirements.kind(),
            Capability::ProbabilisticSatisfaction,
            "{name}"
        );
        assert_eq!(evidence_of(&record), Some(evidence), "{name}");
        assert_eq!(
            requirements.extent(),
            &ClaimExtent::from_domains(BTreeMap::from([(
                DomainKey::Node {
                    node: node(1),
                    path: Vec::new()
                },
                DomainKind::InfiniteTrace
            )])),
            "{name}"
        );
        assert_eq!(record.result_bound().node(), node(99), "{name}");
        assert!(record.conversion_context().is_none(), "{name}");
        assert!(record.path_condition().is_empty(), "{name}");
    }
}

fn exact_backend() -> qsl_route::BackendDescriptor {
    descriptor_from_manifest(
        Candidate::new(BackendId::new("en-5")),
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

fn registry() -> Registry {
    let mut registry = Registry::new();
    registry
        .register(
            manifest::descriptor(ProviderOrigin::Linked, ManifestDigest::from_digest([4; 32]))
                .expect("EN-4's manifest is admitted"),
        )
        .expect("EN-4 registers");
    registry.register(exact_backend()).expect("EN-5 registers");
    registry
}

fn candidate_ids(item: &RequirementItem) -> Vec<String> {
    match item.candidates() {
        CandidateOutcome::Candidates(set) => set
            .candidates()
            .iter()
            .map(|candidate| candidate.id().as_str().to_owned())
            .collect(),
        CandidateOutcome::UnknownBackend(id) => panic!("unknown backend {id}"),
    }
}

/// (index, evidence, candidates, occurrence) of each item.
fn summary(
    items: &[RequirementItem],
) -> Vec<(usize, Option<EvidenceKind>, Vec<String>, OccurrenceKey)> {
    items
        .iter()
        .map(|item| {
            (
                item.index().get(),
                item.evidence(),
                candidate_ids(item),
                item.occurrence().clone(),
            )
        })
        .collect()
}

/// Checks FR-075-AC-18 (TC-1163 step 3): over `P95`, `FairTerminates`-like
/// `Every` and a `value-validity` record, in key order, with EN-4 `{statistical}`
/// and EN-5 `{exact}` registered: no selection names each record's kind;
/// `exact` makes both probabilistic items `exact` (EN-5 only); both kinds
/// append the other-kind item of each probabilistic record, and the
/// `value-validity` item names no kind under every selection.
#[trace("TC-1163", "FR-075-AC-18")]
#[test]
fn the_request_builder_names_the_evidence_kind_of_each_probabilistic_item() {
    let world = World::new(&prob_world::service(CLAIMS));
    let records = BTreeMap::from([
        (key(1), record(&world, "P95", 1)),
        (key(2), record(&world, "Every", 2)),
        (
            key(3),
            RequirementRecord::model_claim(
                Requirements::new(Capability::ValueValidity, ClaimExtent::Bounded),
                node(99),
            ),
        ),
    ]);
    let registry = registry();
    let en4 = vec!["en-4".to_owned()];
    let en5 = vec!["en-5".to_owned()];
    let stat = Some(EvidenceKind::Statistical);
    let exact = Some(EvidenceKind::Exact);
    let build = |selection| summary(&items_with_evidence(&records, &registry, None, selection));

    let record_kinds = build(EvidenceSelection::Record);
    assert_eq!(record_kinds.len(), 3);
    assert_eq!(
        record_kinds
            .iter()
            .map(|i| (i.0, i.1, i.2.clone()))
            .take(2)
            .collect::<Vec<_>>(),
        [(0, stat, en4.clone()), (1, exact, en5.clone())]
    );
    assert_eq!(record_kinds[2].0, 2);
    assert_eq!(record_kinds[2].1, None);

    let only_exact = build(EvidenceSelection::One(EvidenceKind::Exact));
    assert_eq!(only_exact.len(), 3);
    assert_eq!(
        only_exact
            .iter()
            .map(|i| (i.0, i.1, i.2.clone()))
            .take(2)
            .collect::<Vec<_>>(),
        [(0, exact, en5.clone()), (1, exact, en5.clone())]
    );
    assert_eq!(only_exact[2].1, None);

    let both = build(EvidenceSelection::Both);
    assert_eq!(both.len(), 5);
    assert_eq!(both[..3], record_kinds[..]);
    assert_eq!((both[3].0, both[3].1, both[3].2.clone()), (3, exact, en5));
    assert_eq!((both[4].0, both[4].1, both[4].2.clone()), (4, stat, en4));
    assert_eq!(both[3].3, key(1));
    assert_eq!(both[4].3, key(2));
}
