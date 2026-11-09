// SPDX-License-Identifier: AGPL-3.0-or-later
//! The provider/analyze byte-provision and evidence boundary.

use ix_trace_rs::trace;
use qsl_analyze::analyze::{
    analyze, AnalyzeLimits, AnalyzeRefusal, AnalyzeRequest, ClaimItem, ClauseLimits,
    Counterexample, Engines, ItemForm, ItemOptions, NegotiatedItem, ProbabilisticItem,
    ProbabilisticLimits, SourceProvision, TemporalItem,
};
use qsl_analyze::exact_probabilistic::ExactProbLimits;
use qsl_analyze::provider;
use qsl_foundation::diagnostic::StageFailure;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_replay::{IncompleteCause, ModelCheckLimit, TerminalValue};
use qsl_route::request::ExtentClassification;
use qsl_route::routing::Disposition;
use qsl_route::{Candidate, EvidenceKind};
use qsl_semantics::check::Capability;
use quire_exact::Cancel;
use serde_json::json;

use crate::model_world::{self, Clause, World, INFINITE};
use crate::prob_world;

fn counter() -> World {
    World::new(
        model_world::counter(),
        crate::counter::CONTRACT,
        INFINITE,
        &[Clause {
            activation: "on each (q: M::Counter)",
            formula: "always holds(c.value < 2)",
        }],
        &[("c", json!({"value":{"integer":"0"}}))],
    )
}

fn temporal(world: &World) -> NegotiatedItem {
    let node = &world.package.graph().temporal_clauses()[0];
    let candidate = Candidate::new(BackendId::new("en-1"));
    NegotiatedItem {
        item: ClaimItem {
            request_index: RequestIndex::new(0),
            claim: OccurrenceKey::new(node.identity(), node.claim().clone()),
            kind: Capability::TemporalSatisfaction,
            extent: ExtentClassification::Unbounded {
                finite_bound_available: true,
            },
            backend: None,
            candidates: vec![candidate.clone()],
            options: ItemOptions {
                required: true,
                ..ItemOptions::default()
            },
            form: Some(ItemForm::Temporal(TemporalItem::default())),
        },
        disposition: Disposition::Supported(candidate),
        diagnostic: None,
    }
}

fn exact(world: &prob_world::World, limits: ExactProbLimits) -> NegotiatedItem {
    let candidate = Candidate::new(BackendId::new("en-5"));
    NegotiatedItem {
        item: ClaimItem {
            request_index: RequestIndex::new(0),
            claim: world
                .claim("Likely")
                .occurrence()
                .expect("the authored occurrence")
                .clone(),
            kind: Capability::ProbabilisticSatisfaction,
            extent: ExtentClassification::Unbounded {
                finite_bound_available: false,
            },
            backend: None,
            candidates: vec![candidate.clone()],
            options: ItemOptions::default(),
            form: Some(ItemForm::Probabilistic(ProbabilisticItem {
                evidence: EvidenceKind::Exact,
                limits: ProbabilisticLimits::Exact(limits),
                method: None,
                seed: None,
                sampler: None,
                max_witnesses: None,
                symmetry: Vec::new(),
                timed_sampling: None,
            })),
        },
        disposition: Disposition::Supported(candidate),
        diagnostic: None,
    }
}

#[trace("TC-763", "FR-281-AC-20", "TC-761", "FR-280-AC-10")]
#[test]
fn missing_source_refuses_analyze_and_declines_provider_items_before_any_engine() {
    let world = counter();
    let request = AnalyzeRequest {
        binding: world.binding(&["c"]),
        provision: SourceProvision::default(),
        items: vec![temporal(&world)],
    };
    let engines = Engines::qsl().expect("the manifests register");
    let limits = AnalyzeLimits::unbounded();
    let failure = analyze(&world.package, &request, &engines, &limits, &Cancel::new())
        .expect_err("the source is absent");
    let StageFailure::Refused(AnalyzeRefusal::Provision(refusal)) = failure else {
        panic!("wrong request refusal: {failure:?}")
    };
    assert_eq!(
        refusal.authority,
        world.package.graph().source().authority()
    );
    assert_eq!(refusal.identity, world.package.graph().source().identity());
    assert_eq!(refusal.catalog_code().code(), "missing_import");
    let served = provider::run(
        &world.package,
        &request.binding,
        &request.provision,
        &request.items,
        &engines,
        &limits,
        &Cancel::new(),
    );
    assert_eq!(served.records.len(), 1);
    assert!(matches!(
        served.records[0].value(),
        TerminalValue::Declined {
            cause: qsl_replay::ProofRefusalCause::IncompleteInput,
            ..
        }
    ));
    assert_eq!(served.records[0].diagnostic(), Some(refusal.catalog_code()));
    assert!(served.records[0].backend().is_none());
    assert!(served.counterexamples.is_empty());
}

#[trace("TC-762", "FR-281-AC-2", "TC-761", "FR-280-AC-9")]
#[test]
fn a_refuted_clause_returns_the_reproduced_counterexample_through_both_entries() {
    let world = counter();
    let provision = SourceProvision::single(world.package.graph().source(), &world.unit);
    let request = AnalyzeRequest {
        binding: world.binding(&["c"]),
        provision,
        items: vec![temporal(&world)],
    };
    let limits = AnalyzeLimits::unbounded();
    let engines = Engines::qsl().expect("the manifests register");
    let outcome = analyze(&world.package, &request, &engines, &limits, &Cancel::new())
        .expect("the request admits")
        .into_value();
    let served = provider::run(
        &world.package,
        &request.binding,
        &request.provision,
        &request.items,
        &engines,
        &limits,
        &Cancel::new(),
    );
    assert_eq!(outcome.records, served.records);
    assert_eq!(outcome.records[0].value(), &TerminalValue::Refuted);
    assert_eq!(outcome.counterexamples.len(), 1);
    assert_eq!(served.counterexamples.len(), 1);
    let Counterexample::Temporal(payload) = &outcome.counterexamples[0].counterexample else {
        panic!("a temporal counterexample")
    };
    let Counterexample::Temporal(served_payload) = &served.counterexamples[0].counterexample else {
        panic!("a temporal counterexample")
    };
    assert_eq!(payload, served_payload);
    assert_eq!(payload.kind, qsl_replay::CounterexampleKind::Formula);
    assert_eq!(outcome.counterexamples[0].backend.as_str(), "en-1");
    let node = &world.package.graph().temporal_clauses()[0];
    let subject = qsl_analyze::analyze::ClauseSubject::new(
        &world.package,
        world.binding(&["c"]),
        limits,
        &request.provision,
        &OccurrenceKey::new(node.identity(), node.claim().clone()),
        ClauseLimits::default(),
    )
    .expect("the clause is present");
    assert!(matches!(
        subject
            .replay_counterexample(payload)
            .expect("the replay admits")
            .settlement,
        qsl_replay::ModelTraceSettlement::Reproduced {
            value: qsl_replay::ModelTraceValue::Truth(false),
            ..
        }
    ));
}

const LIKELY: &str = "probabilistic Likely using inf on M over (m: M::Msg) under every scheduler on origin { probability >= 0.95 [ eventually holds(m.delivered) ] }";

#[trace("TC-762", "FR-281-AC-1", "TC-761", "FR-280-AC-2", "FR-280-AC-8")]
#[test]
fn the_unbounded_exact_member_retains_the_checker_owned_certificate() {
    let world = prob_world::World::new(&prob_world::link(LIKELY));
    let request = AnalyzeRequest {
        binding: world.binding(),
        provision: world.provision.clone(),
        items: vec![exact(&world, ExactProbLimits::default())],
    };
    let engines = Engines::qsl().expect("the manifests register");
    let limits = AnalyzeLimits::unbounded();
    let outcome = analyze(&world.package, &request, &engines, &limits, &Cancel::new())
        .expect("the request admits")
        .into_value();
    let served = provider::run(
        &world.package,
        &request.binding,
        &request.provision,
        &request.items,
        &engines,
        &limits,
        &Cancel::new(),
    );
    assert_eq!(outcome.records, served.records);
    assert!(matches!(
        outcome.records[0].value(),
        TerminalValue::Proved { .. }
    ));
    let certificate = outcome.records[0]
        .exact_certificate()
        .expect("the accepted certificate is carried");
    assert!(!certificate.values.is_empty());
    assert!(outcome.counterexamples.is_empty());
    assert_eq!(
        outcome.records[0].backend().map(BackendId::as_str),
        Some("en-5")
    );
}

#[trace("TC-763", "FR-281-AC-6")]
#[test]
fn the_one_state_exact_budget_returns_no_counterexample_or_certificate() {
    let world = prob_world::World::new(&prob_world::link(LIKELY));
    let mut exact_limits = ExactProbLimits::default();
    exact_limits.model.limits.max_states = 1;
    let request = AnalyzeRequest {
        binding: world.binding(),
        provision: world.provision.clone(),
        items: vec![exact(&world, exact_limits)],
    };
    let outcome = analyze(
        &world.package,
        &request,
        &Engines::qsl().expect("the manifests register"),
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect("the request admits")
    .into_value();
    assert_eq!(
        outcome.records[0].value(),
        &TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::ExactProbMaxStates,
            value: 1
        })
    );
    assert!(outcome.records[0].exact_certificate().is_none());
    assert!(outcome.counterexamples.is_empty());
}

/// The provision's labels do not grant authority to different source bytes.
/// A counterexample that the supplied unit cannot reproduce is not exported.
#[trace("TC-763", "FR-281-AC-20")]
#[test]
fn changed_source_bytes_never_promote_an_unreproduced_candidate_to_refuted() {
    let world = counter();
    let changed = String::from_utf8(world.unit.clone())
        .expect("fixture UTF-8")
        .replace("c.value < 2", "c.value < 5");
    assert_ne!(
        changed.as_bytes(),
        world.unit.as_slice(),
        "the source mutation changes the bound"
    );
    let request = AnalyzeRequest {
        binding: world.binding(&["c"]),
        provision: SourceProvision::single(world.package.graph().source(), changed.as_bytes()),
        items: vec![temporal(&world)],
    };
    let outcome = analyze(
        &world.package,
        &request,
        &Engines::qsl().expect("the manifests register"),
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect("the labels, binding and source provision admit")
    .into_value();
    assert!(
        matches!(outcome.records[0].value(), TerminalValue::Inconclusive(_)),
        "{:?}",
        outcome.records[0]
    );
    assert!(outcome.counterexamples.is_empty());
}

#[trace("TC-763", "FR-281-AC-20")]
#[test]
fn source_completeness_is_checked_even_when_the_package_cannot_emit() {
    let world = prob_world::World::new(&prob_world::link(LIKELY));
    qsl_package::emit_checked(&world.package, qsl_package::EmitLimits::fixture())
        .expect_err("probabilistic model refuses S4 emission");
    let request = AnalyzeRequest {
        binding: world.binding(),
        provision: SourceProvision::default(),
        items: vec![exact(&world, ExactProbLimits::default())],
    };
    let failure = analyze(
        &world.package,
        &request,
        &Engines::qsl().expect("the manifests register"),
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect_err("missing source refuses even this package");
    assert!(matches!(
        failure,
        StageFailure::Refused(AnalyzeRefusal::Provision(_))
    ));
}
