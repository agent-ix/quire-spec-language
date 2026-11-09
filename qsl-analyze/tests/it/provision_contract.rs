// SPDX-License-Identifier: AGPL-3.0-or-later
//! The provider/analyze byte-provision and evidence boundary.
//!
//! These are the in-process portions of the tagged criteria. Candidate
//! fixtures come from the real engine registry; their dispositions are
//! supplied by this caller. Genuine CG negotiation and the complete FR-331
//! wire projection remain integration obligations, not coverage claimed here.

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
    let candidate = registered_candidate(Capability::TemporalSatisfaction, None, "en-1");
    NegotiatedItem {
        item: ClaimItem {
            request_index: RequestIndex::new(0),
            claim: OccurrenceKey::new(node.identity(), node.claim().clone()),
            kind: Capability::TemporalSatisfaction,
            extent: ExtentClassification::Unbounded {
                finite_bound_available: true,
            },
            backend: Some(candidate.id().clone()),
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
    let candidate = registered_candidate(
        Capability::ProbabilisticSatisfaction,
        Some(EvidenceKind::Exact),
        "en-5",
    );
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
            backend: Some(candidate.id().clone()),
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
    let world = prob_world::World::new(&prob_world::service(""));
    qsl_package::emit_checked(&world.package, qsl_package::EmitLimits::fixture())
        .expect_err("probabilistic model refuses S4 emission");
    let request = AnalyzeRequest {
        binding: world.binding(),
        provision: SourceProvision::default(),
        items: Vec::new(),
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

fn plain(claim: OccurrenceKey, kind: Capability, backend: &str) -> NegotiatedItem {
    let candidate = registered_candidate(kind, None, backend);
    NegotiatedItem {
        item: ClaimItem {
            request_index: RequestIndex::new(0),
            claim,
            kind,
            extent: ExtentClassification::Bounded,
            backend: Some(candidate.id().clone()),
            candidates: vec![candidate.clone()],
            options: ItemOptions::default(),
            form: None,
        },
        disposition: Disposition::Supported(candidate),
        diagnostic: None,
    }
}

/// Generalized items retain the existing value arm's native replay token;
/// they do not reinterpret a value claim as a probabilistic request.
#[trace("TC-761", "FR-280-AC-2")]
#[test]
fn a_value_claim_returns_its_native_replayed_assignment() {
    let declaration = "function Goal using v(): Boolean pure { false }";
    let package = crate::value_validity::package_of(declaration);
    let source = format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n{declaration}\n");
    let packages = qsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("empty package input");
    let snapshots = qsl_semantics::model::observation::DocumentProvision::default();
    let binding = qsl_analyze::analyze::SubjectBinding {
        packages: &packages,
        snapshots: &snapshots,
        initial: Vec::new(),
        universes: Vec::new(),
    };
    let claim = OccurrenceKey::declaration(
        package
            .graph()
            .function_identity("Goal")
            .expect("the function identity"),
    );
    let provision = SourceProvision::single(package.graph().source(), source.as_bytes());
    let result = provider::run(
        &package,
        &binding,
        &provision,
        &[plain(
            claim,
            Capability::ValueValidity,
            qsl_analyze::value_check::manifest::BACKEND_ID,
        )],
        &Engines::qsl().expect("the manifests register"),
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    );
    assert_eq!(result.records[0].value(), &TerminalValue::Refuted);
    assert_eq!(
        result.records[0].backend().map(BackendId::as_str),
        Some(qsl_analyze::value_check::manifest::BACKEND_ID)
    );
    assert_eq!(result.counterexamples.len(), 1);
    let Counterexample::Assignments(assignment) = &result.counterexamples[0].counterexample else {
        panic!("native value assignment")
    };
    assert!(assignment.parameters.is_empty());
    assert!(assignment.reproduced.assignment().is_empty());
}

#[trace("TC-761", "FR-280-AC-2")]
#[test]
fn a_contract_claim_reaches_the_existing_contract_arm() {
    let world = counter();
    let clause = world
        .package
        .graph()
        .state_clause("CanInc")
        .expect("the precondition is checked");
    let claim = OccurrenceKey::new(clause.identity(), clause.claim().clone());
    let result = provider::run(
        &world.package,
        &world.binding(&["c"]),
        &world.provision,
        &[plain(
            claim,
            Capability::OperationContract,
            qsl_analyze::contract_check::manifest::BACKEND_ID,
        )],
        &Engines::qsl().expect("the manifests register"),
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    );
    assert!(
        matches!(result.records[0].value(), TerminalValue::Proved { .. }),
        "{:?}",
        result.records[0]
    );
    assert!(result.counterexamples.is_empty());
    assert_eq!(
        result.records[0].backend().map(BackendId::as_str),
        Some(qsl_analyze::contract_check::manifest::BACKEND_ID)
    );
}

/// Omitted optional sampling settings use the engine's published defaults,
/// rather than turning a valid statistical item into an unsupported one.
#[trace("TC-761", "FR-280-AC-5")]
#[test]
fn omitted_optional_statistical_settings_equal_the_explicit_defaults() {
    let claim = "probabilistic Two using ev on M over (s: M::Server) under Steady on origin with alpha 0.01, beta 0.01, indifference 0.01 { quantile 0.95 of (accumulate duration from holds(s.phase = 1) until holds(s.phase = 2) within 10) <= 2 }";
    let world = prob_world::World::new(&prob_world::service(claim));
    let mut explicit = crate::analyze::routed_service_item(&world);
    let Some(ItemForm::Probabilistic(form)) = &mut explicit.item.form else {
        panic!("statistical form")
    };
    form.max_witnesses = Some(qsl_analyze::statistical::DEFAULT_MAX_WITNESSES);
    form.timed_sampling = Some(qsl_replay::statistical::TimedSampling::default());
    let mut omitted = explicit.clone();
    let Some(ItemForm::Probabilistic(form)) = &mut omitted.item.form else {
        panic!("statistical form")
    };
    form.max_witnesses = None;
    form.timed_sampling = None;
    let limits = AnalyzeLimits {
        accounting: prob_world::UNLIMITED,
        ..AnalyzeLimits::unbounded()
    };
    let engines = Engines::qsl().expect("the manifests register");
    let serve = |item| {
        provider::run(
            &world.package,
            &world.binding(),
            &world.provision,
            &[item],
            &engines,
            &limits,
            &Cancel::new(),
        )
    };
    let expected = serve(explicit);
    assert!(matches!(
        expected.records[0].value(),
        TerminalValue::Measured(_)
    ));
    assert_eq!(serve(omitted).records, expected.records);
}

fn registered_candidate(
    kind: Capability,
    evidence: Option<EvidenceKind>,
    backend: &str,
) -> Candidate {
    let engines = Engines::qsl().expect("the production manifests register");
    let mut registry = qsl_route::Registry::new();
    for descriptor in engines.registrations() {
        registry
            .register(descriptor.clone())
            .expect("a unique backend");
    }
    let evidence = evidence
        .map(qsl_route::ItemEvidence::Kind)
        .unwrap_or(qsl_route::ItemEvidence::Absent);
    let requested = BackendId::new(backend);
    let qsl_route::CandidateOutcome::Candidates(candidates) =
        registry.candidates_for_evidence(kind, &evidence, Some(&requested))
    else {
        panic!("the named backend is registered")
    };
    let [candidate] = candidates.candidates() else {
        panic!("the registered named backend advertises this kind/evidence")
    };
    candidate.clone()
}
