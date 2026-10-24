// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-705 (FR-250-AC-1): a checked hybrid claim's requirement record gives
//! its request item the `hybrid-dynamics` requirement, which routes it to
//! the registered hybrid provider and never to EN-6.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_foundation::bound::DomainKind;
use qsl_foundation::digest::ByteDigest;
use qsl_foundation::{BackendId, SourceIdentity};
use qsl_replay::{settle_hybrid_candidates, TerminalValue, UnavailabilityCause};
use qsl_route::request::items_from_requirements;
use qsl_route::{
    BackendDescriptor, Candidate, CandidateOutcome, DynamicsKind, ManifestDigest, Mode,
    ProviderOrigin, Registry, SolverBounds,
};
use qsl_semantics::check::{Capability, CheckedGraph, PackageDeclarations};
use quire_semantic_value::checking::CheckingLimits;

/// FR-447's `Braking` and `SlowEnough`, and `Reaches`, a temporal claim over
/// a model that is not hybrid. `{}` is the model's digest.
const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile p = \"quire.temporal.infinite-trace/v1\";\n\
     model Config = \"example/config-version\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
     hybrid Braking using p {\n\
       mode Moving invariant { v >= 0 and v <= 20 and a >= 1 and a <= 2 }\n\
         flow { v' = -a; a' = 0; t' = 1; }\n\
         transition to Stopped when { v = 0 } reset { v = 0 };\n\
       mode Stopped invariant { v = 0 }\n\
         flow { v' = 0; a' = 0; t' = 1; }\n\
     }\n\
     temporal SlowEnough using p over (h: Braking) clock \"model-time\" \
     on origin { always [0, 10] holds(h.v < 30) }\n\
     temporal Reaches using p over (c: Config::ConfigVersion) clock \"model-steps\" \
     on origin { eventually holds(c.versionNumber = 2) }\n";

fn graph() -> CheckedGraph {
    let document = crate::support::config_version_spine::spine::DOMAIN_PACKAGE.as_bytes();
    let packages = qsl_semantics::model::intake::package_input([document]);
    let digest = packages.keys().next().expect("one package");
    let text = SOURCE.replace("{}", &qsl_semantics::model::key::hex(digest));
    let parsed = qsl_cst::parse(
        SourceIdentity::new("agent-ix", "test:hybrid", "fixture", "fixture:1"),
        "program.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let raw = parsed.source().reference().clone();
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = qsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the model is admitted");
    PackageDeclarations::assemble(raw, unit, models, Vec::new())
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the unit checks")
}

fn discrete() -> BackendDescriptor {
    BackendDescriptor::new(
        Candidate::new(BackendId::new("qsl-model-check")),
        ProviderOrigin::Linked,
        ManifestDigest::from_digest(ByteDigest::of(b"discrete").as_bytes()),
        [(Capability::TemporalSatisfaction, Mode::Bounded)],
        Some(BTreeSet::from([DomainKind::Collection])),
        None,
    )
    .expect("a well-formed descriptor")
}

fn hybrid() -> BackendDescriptor {
    BackendDescriptor::new(
        Candidate::new(BackendId::new("hybrid-fixture")),
        ProviderOrigin::Linked,
        ManifestDigest::from_digest(ByteDigest::of(b"hybrid").as_bytes()),
        [(Capability::TemporalSatisfaction, Mode::Bounded)],
        Some(BTreeSet::from([DomainKind::Collection])),
        None,
    )
    .expect("a well-formed descriptor")
    .with_dynamics(
        Some(BTreeSet::from([DynamicsKind::Hybrid])),
        Some(SolverBounds {
            max_solver_steps: Some(1_000_000),
            max_solver_time: Some(60),
        }),
    )
    .expect("the fixture publishes both solver bounds")
}

/// Each item's candidates, for the item labelled `hybrid-dynamics` and for
/// the one that is not, in request order.
fn routed(descriptors: impl IntoIterator<Item = BackendDescriptor>) -> Vec<(bool, Vec<String>)> {
    let mut registry = Registry::new();
    for descriptor in descriptors {
        registry.register(descriptor).expect("distinct identities");
    }
    let graph = graph();
    let items = items_from_requirements(graph.requirements(), &registry, None);
    assert_eq!(items.len(), 2, "one item for each temporal claim");
    let mut routed: Vec<(bool, Vec<String>)> = items
        .iter()
        .map(|item| {
            assert_eq!(item.kind(), Capability::TemporalSatisfaction);
            let CandidateOutcome::Candidates(set) = item.candidates() else {
                panic!("a computed candidate set");
            };
            (
                item.requirements() == ["hybrid-dynamics"],
                set.candidates()
                    .iter()
                    .map(|candidate| candidate.id().as_str().to_owned())
                    .collect(),
            )
        })
        .collect();
    routed.sort();
    routed
}

/// `SlowEnough`'s item carries `hybrid-dynamics` and routes to the hybrid
/// provider and not to EN-6 beside it, in either registration order, while
/// `Reaches`, over a model that is not hybrid, carries no label and routes to
/// EN-6 only. With no hybrid provider registered the hybrid item's candidate
/// set is empty and settles `unsupported`, `unsupported-requested-capability`.
#[trace("TC-705", "FR-250-AC-1")]
#[test]
fn a_checked_hybrid_claim_routes_to_the_hybrid_provider_only() {
    let both = vec![
        (false, vec!["qsl-model-check".to_owned()]),
        (true, vec!["hybrid-fixture".to_owned()]),
    ];
    for descriptors in [[discrete(), hybrid()], [hybrid(), discrete()]] {
        assert_eq!(routed(descriptors), both);
    }
    let alone = routed([discrete()]);
    assert_eq!(
        alone,
        vec![
            (false, vec!["qsl-model-check".to_owned()]),
            (true, Vec::new())
        ]
    );
    let label = |labelled: bool| -> &'static [&'static str] {
        if labelled {
            &["hybrid-dynamics"]
        } else {
            &[]
        }
    };
    let settled: Vec<_> = alone
        .iter()
        .map(|(labelled, candidates)| settle_hybrid_candidates(label(*labelled), candidates.len()))
        .collect();
    // Only the hybrid item is settled; `Reaches` has a candidate, and an
    // unlabelled item with none would still not be a hybrid item.
    assert_eq!(
        settled,
        vec![
            None,
            Some(TerminalValue::Unsupported(
                UnavailabilityCause::HybridProviderAbsent
            ))
        ]
    );
    assert_eq!(settle_hybrid_candidates(&[], 0), None);
    assert_eq!(
        UnavailabilityCause::HybridProviderAbsent.as_str(),
        "unsupported-requested-capability"
    );
    assert_eq!(settle_hybrid_candidates(&["hybrid-dynamics"], 1), None);
}
