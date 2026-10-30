// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-144 (TC-549): a refinement's requirement record, its routing, the
//! deadlock-freedom item its request carries and its obligation identity.

use ix_trace_rs::trace;
use qsl_analyze::model_check::manifest as en1;
use qsl_analyze::model_check::{check_refinement, RefinementCheckRequest, SubjectContext};
use qsl_eval::model_subject::ModelSubject;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_replay::{
    open_systems, replay_refinement_trace, settle_refinement_safety, ModelCheckLimits,
    RefinementFailure, SafetyOutcome, SimulationCertificateRequest, SubjectIdentity, TerminalValue,
};
use qsl_route::deadlock::{ModelRefinementItem, ObligationKind};
use qsl_route::model_request::ModelRequest;
use qsl_route::request::{items_from_requirements, ItemKind};
use qsl_route::{CandidateOutcome, ProviderOrigin, Registry};
use qsl_semantics::check::{Capability, RequirementRecord};
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::Cancel;
use quire_semantic_value::fairness::PropertyForm;

use crate::derived_request::derived_items;
use crate::refinement_fixture::{cas_refines_counter, mutated, World, UNLIMITED};

const ENSURE: &str = "ensure fair weak each Spec::Counter::inc;\n";

/// `CasRefinesCounter` with its `ensure` row removed.
fn without_ensure() -> String {
    mutated(&cas_refines_counter(), ENSURE, "")
}

/// The refinement's one record, by its occurrence.
fn record(world: &World) -> (&OccurrenceKey, &RequirementRecord) {
    let occurrence = world
        .refinement()
        .requirement_occurrence()
        .expect("the refinement has a record");
    let record = world
        .package()
        .graph()
        .requirements()
        .get(occurrence)
        .expect("the record is in the package");
    (occurrence, record)
}

fn subject(world: &World) -> ModelSubject<'_> {
    ModelSubject {
        simulation: world.concrete_request(UNLIMITED),
        proof_bounds: Vec::new(),
    }
}

/// FR-144-AC-1 (TC-549 step 1): `CasRefinesCounter` writes one record
/// (`temporal-satisfaction`, `Refinement{liveness: true}`) over its concrete
/// model subject, and with its `ensure` row removed `liveness: false`. No
/// other record is keyed at the refinement's node, and the record routes to
/// the explicit-state provider.
#[trace("TC-549", "FR-144-AC-1")]
#[test]
fn a_refinement_writes_one_temporal_record_routed_to_the_explicit_state_provider() {
    for (body, liveness) in [(cas_refines_counter(), true), (without_ensure(), false)] {
        let world = World::new(&body);
        let (occurrence, record) = record(&world);
        let requirements = record.requirements();
        assert_eq!(requirements.kind(), Capability::TemporalSatisfaction);
        assert!(requirements.model_subject());
        assert_eq!(
            requirements.property_form(),
            Some(PropertyForm::Refinement { liveness })
        );
        assert_eq!(
            world.refinement().property_form(),
            PropertyForm::Refinement { liveness }
        );
        let records = world.package().graph().requirements();
        let roles: Vec<&str> = records
            .keys()
            .filter(|key| key.node() == occurrence.node())
            .map(|key| key.origin().role().as_str())
            .collect();
        // The refinement record, and FR-146's per-step simulation records
        // (an `Initial` and one `Step` per row) at their `generated`
        // occurrences.
        assert_eq!(
            roles.iter().filter(|role| **role == "declaration").count(),
            1,
            "one refinement record at the refinement's node"
        );
        let generated = roles.iter().filter(|role| **role == "generated").count();
        assert_eq!(generated, world.refinement().steps().len() + 1);
        assert_eq!(generated, world.refinement().step_simulations().len());

        let mut registry = Registry::new();
        registry
            .register(en1::descriptor(ProviderOrigin::Linked).expect("EN-1 admits"))
            .expect("registered");
        let items = items_from_requirements(records, &registry, None);
        let item = items
            .iter()
            .find(|item| item.occurrence() == occurrence)
            .expect("the record has an item");
        let CandidateOutcome::Candidates(set) = item.candidates() else {
            panic!("a computed candidate set: {item:?}");
        };
        let candidates: Vec<&str> = set
            .candidates()
            .iter()
            .map(|candidate| candidate.id().as_str())
            .collect();
        assert_eq!(candidates, [en1::BACKEND_ID]);
    }
}

/// The deadlock-freedom items of a request holding `CasRefinesCounter` over
/// its concrete subject, in a unit that also declares `terminal`.
fn derived(terminal: &str) -> (SubjectIdentity, Vec<SubjectIdentity>) {
    let world = World::new(&format!("{terminal}{}", cas_refines_counter()));
    let subject = subject(&world);
    let identity = SubjectIdentity::of(&subject).expect("the package emits");
    let initial = [world.abstract_initial()];
    let item = ModelRefinementItem {
        refinement: world.refinement(),
        subject: &identity,
        abstract_initial: &initial,
        package: world.package().graph(),
    };
    let derived = derived_items(&[], &[item], &[], &[])
        .into_iter()
        .filter(|(item, _)| item.item_kind() == Some(ItemKind::DeadlockFreedom))
        .map(|(_, obligation)| obligation.subject)
        .collect();
    (identity, derived)
}

/// FR-144-AC-1 (TC-549 step 1): the request carries the concrete subject's
/// deadlock-freedom item and none for the abstract subject; `terminal any`
/// on the concrete model opts it out, and on the abstract model it does not.
#[trace("TC-549", "FR-144-AC-1")]
#[test]
fn a_refinement_brings_only_its_concrete_subjects_deadlock_item() {
    for terminal in ["", "terminal Halt using v on Spec any;\n"] {
        let (concrete, derived) = derived(terminal);
        assert_eq!(derived, [concrete], "{terminal:?}");
    }
    let (_, derived) = derived("terminal Halt using v on Impl any;\n");
    assert!(derived.is_empty(), "{derived:?}");
}

/// FR-124 with FR-144 (TC-549 step 1, SR-1751002 FND-002): a temporal item
/// and a refinement item over the one concrete subject derive one
/// deadlock-freedom item, alone or together, while the abstract model
/// declares `terminal any`; the concrete model's `terminal any` removes it
/// in every case.
#[trace("TC-549", "FR-144-AC-1")]
#[test]
fn a_temporal_and_a_refinement_item_share_the_concrete_deadlock_item() {
    use qsl_route::deadlock::ModelTemporalItem;
    for (terminal, expected) in [
        ("terminal Halt using v on Spec any;\n", 1),
        ("terminal Halt using v on Impl any;\n", 0),
    ] {
        let world = World::new(&format!("{terminal}{}", cas_refines_counter()));
        let subject = subject(&world);
        let identity = SubjectIdentity::of(&subject).expect("the package emits");
        let clause = OccurrenceKey::new(
            qsl_foundation::digest::WireNodeId::from_digest([3; 32]),
            quire_exact::Origin::new(quire_exact::Role::new("temporal"), 0),
        );
        let graph = world.package().graph();
        let temporal = [ModelTemporalItem {
            occurrence: &clause,
            subject: &identity,
            package: graph,
        }];
        let initial = [world.abstract_initial()];
        let refinement = [ModelRefinementItem {
            refinement: world.refinement(),
            subject: &identity,
            abstract_initial: &initial,
            package: graph,
        }];
        // The concrete subject reads the one model `Impl`'s selection.
        let over_impl = [world
            .refinement()
            .concrete_model_selection()
            .identity
            .as_str()];
        for (name, derived) in [
            ("temporal", derived_items(&temporal, &[], &[], &over_impl)),
            (
                "refinement",
                derived_items(&[], &refinement, &[], &over_impl),
            ),
            (
                "both",
                derived_items(&temporal, &refinement, &[], &over_impl),
            ),
        ] {
            assert_eq!(derived.len(), expected, "{terminal:?} {name}");
            assert!(
                derived.iter().all(|(item, obligation)| {
                    obligation.subject == identity
                        && item.item_kind() == Some(ItemKind::DeadlockFreedom)
                }),
                "{terminal:?} {name}"
            );
        }
    }
}

/// FR-144-AC-5 (TC-549 step 5): two `CasRefinesCounter` requests that
/// differ only in the abstract subject's initial snapshots (`value` 0 and
/// `value` 1) have different obligation identities, and the second settles
/// `refuted` with `InitialNotAbstract`.
#[trace("TC-549", "FR-144-AC-5")]
#[trace("QSpec-TC-431", "QSpec-FR-368-AC-19")]
#[trace("QSpec-TC-431", "QSpec-FR-377-AC-17")]
#[test]
fn the_abstract_initial_states_are_part_of_the_obligation() {
    let world = World::new(&cas_refines_counter());
    let subject = subject(&world);
    let context = SubjectContext::new(&subject).expect("the package emits");
    let (zero, one) = (world.abstract_initial(), world.abstract_initial_one());
    assert_ne!(zero, one);

    let mut request = ModelRequest::new();
    let author = |request: &mut ModelRequest<_>, initial: &DocumentRef| {
        context
            .author_refinement(
                request,
                world.refinement(),
                world.package(),
                std::slice::from_ref(initial),
            )
            .expect("the record is written")
    };
    let first = author(&mut request, &zero);
    let again = author(&mut request, &zero);
    let second = author(&mut request, &one);
    assert_eq!(first, again, "one obligation has one index");
    assert_ne!(first, second);
    let [a, b] = [first, second].map(|index| request.obligation(index).expect("written").clone());
    assert_eq!(a.subject, b.subject);
    assert_ne!(a, b);
    let ObligationKind::Refinement {
        occurrence,
        abstract_initial,
    } = &b.kind
    else {
        panic!("a refinement obligation: {b:?}");
    };
    assert_eq!(
        Some(occurrence),
        world.refinement().requirement_occurrence()
    );
    assert_eq!(abstract_initial.iter().collect::<Vec<_>>(), [&one]);
    let item = &request.items()[second.get()];
    assert_eq!(
        item.property_form(),
        Some(PropertyForm::Refinement { liveness: true })
    );
    // QSpec FR-368-AC-19, FR-377-AC-17: the refinement item carries its
    // abstract subject, the abstract package's id and its initial states.
    assert_eq!(item.item_kind(), Some(ItemKind::Refinement));
    let abstract_subject = item.abstract_subject().expect("an abstract subject");
    assert_eq!(
        abstract_subject.package_id,
        qsl_package::emit_checked(world.package())
            .expect("the package emits")
            .package()
            .package_id()
    );
    assert_eq!(abstract_subject.initial_states, std::slice::from_ref(&one));

    assert_eq!(settle(&world, &one), TerminalValue::Refuted);
}

/// The safety half of `world`'s refinement against `initial`, which must
/// fail `InitialNotAbstract`, settled through its replay.
fn settle(world: &World, initial: &DocumentRef) -> TerminalValue {
    let concrete = world.concrete_request(UNLIMITED);
    let outcome = check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: subject(world),
            abstract_package: world.package(),
            abstract_initial: vec![initial.clone()],
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the refinement is checked");
    let SafetyOutcome::Violated { counterexample, .. } = &outcome.safety else {
        panic!("the safety half is violated: {:?}", outcome.safety);
    };
    assert_eq!(
        counterexample.refinement,
        Some(RefinementFailure::InitialNotAbstract { initial: 0 })
    );
    let systems = open_systems(
        world.refinement(),
        &concrete,
        world.package(),
        vec![initial.clone()],
    )
    .expect("the subjects open");
    settle_refinement_safety(
        &outcome.safety,
        &ModelCheckLimits::default(),
        &SimulationCertificateRequest {
            refinement: world.refinement(),
            concrete: concrete.clone(),
            abstract_package: world.package(),
            abstract_initial: vec![initial.clone()],
        },
        &Cancel::new(),
        |payload| replay_refinement_trace(world.refinement(), &systems, payload),
    )
    .value
}

/// A stand-in for CG's SMT backend (ADR-018 EN-2, EN-3), registered from a
/// manifest advertising `advertises`, with the fairness kinds a refinement
/// record uses.
fn smt_backend(advertises: &[(Capability, qsl_route::Mode)]) -> qsl_route::BackendDescriptor {
    use qsl_foundation::bound::DomainKind;
    qsl_route::BackendDescriptor::new(
        qsl_route::Candidate::new(qsl_foundation::BackendId::new("smt")),
        ProviderOrigin::Process,
        advertises.iter().copied(),
        Some(
            [DomainKind::Population, DomainKind::Integer]
                .into_iter()
                .collect(),
        ),
        None,
    )
    .expect("the SMT stand-in admits")
    .with_fairness_kinds([
        quire_semantic_value::fairness::FairnessKind::Weak,
        quire_semantic_value::fairness::FairnessKind::Strong,
    ])
}

/// The registry of EN-1 and, when given, the SMT stand-in.
fn registry(smt: Option<qsl_route::BackendDescriptor>) -> Registry {
    let mut registry = Registry::new();
    registry
        .register(en1::descriptor(ProviderOrigin::Linked).expect("EN-1 admits"))
        .expect("registered");
    if let Some(smt) = smt {
        registry.register(smt).expect("registered");
    }
    registry
}

/// The candidate ids of `outcome`.
fn candidate_ids(outcome: &CandidateOutcome) -> Vec<String> {
    let CandidateOutcome::Candidates(set) = outcome else {
        panic!("a computed candidate set: {outcome:?}");
    };
    set.candidates()
        .iter()
        .map(|candidate| candidate.id().as_str().to_owned())
        .collect()
}

/// The request items of `world`'s FR-146 per-step simulation records.
fn per_step_items(world: &World, registry: &Registry) -> Vec<qsl_route::request::RequirementItem> {
    let records = world.package().graph().requirements();
    let generated: Vec<&OccurrenceKey> = world
        .refinement()
        .step_simulations()
        .iter()
        .map(|entry| entry.occurrence())
        .collect();
    let items: Vec<_> = items_from_requirements(records, registry, None)
        .into_iter()
        .filter(|item| generated.contains(&item.occurrence()))
        .collect();
    assert_eq!(items.len(), generated.len());
    items
}

/// FR-146 "Routing and settlement", FR-146-AC-10 (TC-551): each per-step
/// simulation record is an `operation-contract` item whose candidates are
/// chosen by kind alone, so under an SMT backend advertising
/// `operation-contract` only in bounded mode it routes there with an
/// unbounded extent every domain of which is boundable, the inputs from
/// which negotiation settles it `requires-bound`; with the population
/// bounded the extent is bounded.
#[trace("TC-551", "FR-146-AC-10")]
#[test]
fn per_step_records_route_by_kind_to_an_operation_contract_backend() {
    use qsl_route::request::ExtentClassification;
    let registry = registry(Some(smt_backend(&[(
        Capability::OperationContract,
        qsl_route::Mode::Bounded,
    )])));
    let world = World::new(&cas_refines_counter());
    let items = per_step_items(&world, &registry);
    assert_eq!(items.len(), 8);
    for item in &items {
        assert_eq!(item.kind(), Capability::OperationContract);
        assert_eq!(candidate_ids(item.candidates()), ["smt"]);
        assert_eq!(
            item.extent(),
            ExtentClassification::Unbounded {
                finite_bound_available: true
            }
        );
    }

    let bounded = World::new(&format!(
        "{}\nbound CountersBound on Impl::counters [2]\n",
        cas_refines_counter()
    ));
    for item in per_step_items(&bounded, &registry) {
        assert_eq!(item.extent(), ExtentClassification::Bounded);
    }
}

/// FR-146-AC-4's `unsupported` half (TC-551): with no backend advertising
/// `operation-contract`, each per-step simulation record's candidate set is
/// empty, which negotiation settles V-8 `unsupported`; the refinement
/// record still routes to EN-1.
#[trace("TC-551", "FR-146-AC-4")]
#[test]
fn per_step_records_have_no_candidate_without_an_smt_backend() {
    let registry = registry(None);
    let world = World::new(&cas_refines_counter());
    for item in per_step_items(&world, &registry) {
        assert!(candidate_ids(item.candidates()).is_empty(), "{item:?}");
    }
    let (occurrence, _) = record(&world);
    let item = items_from_requirements(world.package().graph().requirements(), &registry, None)
        .into_iter()
        .find(|item| item.occurrence() == occurrence)
        .expect("the refinement record has an item");
    assert_eq!(candidate_ids(item.candidates()), [en1::BACKEND_ID]);
}

/// FR-146 "Routing and settlement" (ADR-020 RE-3, TC-551): the refinement
/// record is a candidate for SMT unrolling as well as for EN-1, so an SMT
/// backend advertising `temporal-satisfaction` and the record's fairness
/// kinds is among its candidates.
#[trace("TC-551")]
#[test]
fn the_refinement_record_is_also_an_smt_unrolling_candidate() {
    let registry = registry(Some(smt_backend(&[(
        Capability::TemporalSatisfaction,
        qsl_route::Mode::Bounded,
    )])));
    let world = World::new(&cas_refines_counter());
    let (occurrence, _) = record(&world);
    let item = items_from_requirements(world.package().graph().requirements(), &registry, None)
        .into_iter()
        .find(|item| item.occurrence() == occurrence)
        .expect("the refinement record has an item");
    let mut ids = candidate_ids(item.candidates());
    ids.sort();
    let mut expected = vec![en1::BACKEND_ID.to_owned(), "smt".to_owned()];
    expected.sort();
    assert_eq!(ids, expected);
}
