// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-483 and FR-485 (TC-972, TC-974): the `operation-contract` check
//! arm over ADR-022 §7.1's `ConfigVersion` model (`versionNumber: Int[0, 3]`,
//! operation `attemptUpdate`), with the universe `{a}`.
//!
//! The arm evaluates the claim at every initial state and at every pre-state
//! of the declared types: `a` absent, or a `ConfigVersion` with each of the
//! four values.

use std::num::NonZeroU64;

use ix_trace_rs::trace;
use qsl_analyze::analyze::{analyze, contract_item, AnalyzeRecord, Engines, Settlement};
use qsl_analyze::contract_check::{manifest, ContractRequest};
use qsl_eval::simulation::model::ModelSystem;
use qsl_forms::StateClauseKind;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::RequestIndex;
use qsl_replay::contract_check::{
    select_contract, CaseVerdict, ContractCase, ContractObligation, ContractSelection,
    ContractTarget, ReadCounterexample,
};
use qsl_replay::{
    Certification, IncompleteCause, InconclusiveCause, ModelCheckLimit, OccurrenceKey, ProofBasis,
    ReportedInconclusiveCause, TerminalValue, UnavailabilityCause,
};
use quire_exact::{Cancel, Origin, Role};

use crate::counter::UNLIMITED;
use crate::graph_world::{config_version, config_world_over, World, CONFIG_CONTRACT};

/// The world with `extra` declarations and one initial state `a` at
/// `start`.
fn world(extra: &str, start: i64) -> World {
    config_world_over(extra, "", &["a"], &[vec![start]])
}

/// The claim occurrence of the state clause `name`.
fn clause(world: &World, name: &str) -> OccurrenceKey {
    let clause = world
        .package
        .graph()
        .state_clauses()
        .iter()
        .find(|clause| clause.name() == name)
        .unwrap_or_else(|| panic!("the clause {name} is checked"));
    OccurrenceKey::new(clause.identity().clone(), clause.claim().clone())
}

fn run(world: &World, claim: &OccurrenceKey, max_cases: Option<u64>) -> AnalyzeRecord {
    let request = select_contract(&world.package, claim).map(|selected| ContractRequest {
        package: &world.package,
        claim: selected,
        subject: world.subject(&["a"], UNLIMITED).simulation,
        abstract_subject: None,
        max_cases: max_cases.and_then(NonZeroU64::new),
    });
    analyze(
        &Engines::qsl().expect("the linked engines register"),
        vec![contract_item(RequestIndex::new(0), request)],
        &Cancel::new(),
    )
    .remove(0)
}

fn terminal(record: &AnalyzeRecord) -> &TerminalValue {
    record
        .terminal()
        .expect("the item settles a terminal record")
        .value()
}

fn assert_proved(record: &AnalyzeRecord) {
    let TerminalValue::Proved {
        basis,
        certification,
    } = terminal(record)
    else {
        panic!("expected proved: {:?}", terminal(record));
    };
    assert_eq!(*basis, ProofBasis::Exhaustive);
    assert_eq!(*certification, Certification::Uncertified);
}

fn refuted(record: &AnalyzeRecord) -> &qsl_replay::contract_check::ContractCounterexample {
    assert!(
        matches!(terminal(record), TerminalValue::Refuted),
        "expected refuted: {:?}",
        terminal(record)
    );
    let counterexample = record
        .contract_check
        .as_deref()
        .expect("a refuted item keeps its counterexample");
    assert!(!counterexample.bound, "the arm's own oracle is unbound");
    counterexample
}

fn version(state: &qsl_semantics::model::state::ModelState) -> i64 {
    let objects = state.populations.values().next().expect("one population");
    let object = objects.get("a").expect("a is present");
    let quire_exact::Value::Integer(value) = object.fields.values().next().expect("one field")
    else {
        panic!("an integer field");
    };
    value.to_string().parse().expect("a small value")
}

#[trace("QSpec-TC-974", "QSpec-FR-485-AC-2")]
#[trace("QSpec-TC-972", "QSpec-FR-483-AC-1")]
#[test]
fn an_invariant_that_holds_initially_and_is_preserved_is_proved() {
    // attemptUpdate moves 0 -> 1 -> 2 -> 0: every pre-state at most 2 stays
    // at most 2.
    let world = world(
        "invariant Small using v on Config::ConfigVersion at current { self.versionNumber <= 2 }\n",
        0,
    );
    assert_proved(&run(&world, &clause(&world, "Small"), None));
}

#[trace("QSpec-TC-974", "QSpec-FR-485-AC-2")]
#[trace("QSpec-TC-974", "QSpec-FR-485-AC-3")]
#[trace("QSpec-TC-972", "QSpec-FR-483-AC-1")]
#[test]
fn an_operation_that_breaks_an_invariant_from_a_satisfying_pre_state_is_refuted() {
    // The pre-state versionNumber = 1 satisfies the invariant and is not
    // reachable from the initial state 0 only through it: preservation is
    // over the declared types, not the reachable states.
    let world = world(
        "invariant Low using v on Config::ConfigVersion at current { self.versionNumber <= 1 }\n",
        0,
    );
    let record = run(&world, &clause(&world, "Low"), None);
    let counterexample = refuted(&record);
    assert_eq!(counterexample.obligation, ContractObligation::Preservation);
    assert_eq!(counterexample.violated, clause(&world, "Low"));
    let [execution] = counterexample.executions.as_slice() else {
        panic!("one execution");
    };
    assert_eq!(version(&execution.pre_state), 1);
    assert_eq!(
        version(execution.post_state.as_ref().expect("a post-state")),
        2
    );
    assert!(execution.transition.is_some());
    assert!(execution.operation.is_some(), "the operation's anchor");
}

#[trace("QSpec-TC-974", "QSpec-FR-485-AC-2")]
#[test]
fn an_initial_state_that_breaks_an_invariant_is_refuted_with_no_operation() {
    let world = world(
        "invariant Small using v on Config::ConfigVersion at current { self.versionNumber <= 2 }\n",
        3,
    );
    let record = run(&world, &clause(&world, "Small"), None);
    let counterexample = refuted(&record);
    assert_eq!(counterexample.obligation, ContractObligation::InitialState);
    let [execution] = counterexample.executions.as_slice() else {
        panic!("one execution");
    };
    assert!(execution.operation.is_none());
    assert!(execution.transition.is_none());
    assert_eq!(version(&execution.pre_state), 3);
}

#[trace("QSpec-TC-974", "QSpec-FR-485-AC-4")]
#[test]
fn an_unbound_postcondition_undefined_at_a_case_is_refuted_for_definedness() {
    // 2 div (2 - v) is undefined at the post-state v = 2.
    let world = world(
        "post Ratio using v on Config::ConfigVersion::attemptUpdate \
         { 2 div (2 - self.versionNumber) >= 0 }\n",
        0,
    );
    let record = run(&world, &clause(&world, "Ratio"), None);
    let counterexample = refuted(&record);
    assert_eq!(counterexample.obligation, ContractObligation::Definedness);
    assert!(counterexample.undefined.is_some());
    let [execution] = counterexample.executions.as_slice() else {
        panic!("one execution");
    };
    assert_eq!(
        version(execution.post_state.as_ref().expect("a post-state")),
        2
    );
}

#[trace("QSpec-TC-974", "QSpec-FR-485-AC-4")]
#[test]
fn an_unbound_precondition_undefined_at_a_pre_state_is_refuted_for_definedness() {
    let world = world(
        "pre Ratio using v on Config::ConfigVersion::attemptUpdate \
         { 2 div (2 - self.versionNumber) >= 0 }\n",
        0,
    );
    let record = run(&world, &clause(&world, "Ratio"), None);
    let counterexample = refuted(&record);
    assert_eq!(counterexample.obligation, ContractObligation::Definedness);
    let [execution] = counterexample.executions.as_slice() else {
        panic!("one execution");
    };
    assert_eq!(version(&execution.pre_state), 2);
    assert!(execution.post_state.is_none());
}

#[trace("QSpec-TC-974", "QSpec-FR-485-AC-4")]
#[test]
fn a_clause_defined_at_every_case_is_proved() {
    let world = world(
        "post Bounded using v on Config::ConfigVersion::attemptUpdate \
         { self.versionNumber >= 0 }\n",
        0,
    );
    assert_proved(&run(&world, &clause(&world, "Bounded"), None));
}

#[trace("QSpec-TC-972", "QSpec-FR-483-AC-2")]
#[trace("QSpec-TC-972", "QSpec-FR-483-AC-1")]
#[test]
fn a_clause_whose_premises_hold_at_no_case_is_vacuous() {
    // No ConfigVersion satisfies the context invariant, so the clause is
    // evaluated at no case.
    let world = world(
        "invariant Never using v on Config::ConfigVersion at current { self.versionNumber > 3 }\n\
         pre Any using v on Config::ConfigVersion::attemptUpdate { self.versionNumber >= 0 }\n",
        0,
    );
    let record = run(&world, &clause(&world, "Any"), None);
    assert_eq!(
        terminal(&record).inconclusive_cause(),
        Some(ReportedInconclusiveCause::KaniVacuousProof),
        "{:?}",
        terminal(&record)
    );
}

#[trace("QSpec-TC-972", "QSpec-FR-483-AC-4")]
#[trace("QSpec-TC-972", "QSpec-FR-483-AC-1")]
#[test]
fn the_case_budget_settles_incomplete_naming_max_cases() {
    let world = world(
        "invariant Small using v on Config::ConfigVersion at current { self.versionNumber <= 2 }\n",
        0,
    );
    let record = run(&world, &clause(&world, "Small"), Some(2));
    let TerminalValue::Incomplete(IncompleteCause::LimitReached { limit, value }) =
        terminal(&record)
    else {
        panic!("expected limit-reached: {:?}", terminal(&record));
    };
    assert_eq!(*limit, ModelCheckLimit::ContractCheckMaxCases);
    assert_eq!(
        (limit.as_str(), limit.setting()),
        ("max_cases", "contract_check.max_cases")
    );
    assert_eq!(*value, 2);
    let engines = Engines::qsl().unwrap();
    let descriptor = engines
        .registrations()
        .find(|d| d.id().as_str() == manifest::BACKEND_ID)
        .expect("the arm is registered");
    assert_eq!(descriptor.max_cases(), Some(manifest::DEFAULT_MAX_CASES));
}

#[trace("QSpec-TC-974", "QSpec-FR-485-AC-1")]
#[test]
fn a_claim_that_selects_no_form_is_declined_invalid_input() {
    let world = world("", 0);
    let claim = OccurrenceKey::new(
        crate::fixture_ids::node_key([7]),
        Origin::new(Role::new("claim"), 0),
    );
    assert_eq!(
        select_contract(&world.package, &claim),
        Err(ContractSelection::NoForm)
    );
    let record = run(&world, &claim, None);
    let TerminalValue::Declined { cause, .. } = terminal(&record) else {
        panic!("declined, not {:?}", terminal(&record));
    };
    assert_eq!(cause.as_str(), "invalid-input");
}

#[trace("QSpec-TC-974", "QSpec-FR-485-AC-6")]
#[test]
fn an_unbound_frame_item_is_unsupported_with_no_contract_level_reading() {
    let world = world("", 0);
    let graph = world.package.graph();
    let clause = graph
        .state_clauses()
        .iter()
        .find(|clause| clause.kind() == StateClauseKind::Postcondition)
        .expect("CONFIG_CONTRACT's postcondition");
    let operation = clause.operation().expect("a post names its operation");
    let anchor = graph
        .operation_anchor_of(operation.declaring, operation.declaration.name())
        .expect("the operation has an anchor");
    let claim = OccurrenceKey::new(anchor.clone(), Origin::new(Role::new("anchor"), 0));
    let record = run(&world, &claim, None);
    let TerminalValue::Unsupported(UnavailabilityCause::ContractCheck { form, reason }) =
        terminal(&record)
    else {
        panic!("expected unsupported: {:?}", terminal(&record));
    };
    assert_eq!(form.as_str(), "frame");
    assert_eq!(reason.as_str(), "no-contract-level-reading");
}

/// The target of `name` over `world`'s system, and the counterexample the
/// arm finds for it.
fn target_and_violation(
    world: &World,
    name: &str,
) -> (
    qsl_replay::contract_check::ContractClaim,
    qsl_replay::contract_check::ContractCounterexample,
) {
    let selected = select_contract(&world.package, &clause(world, name)).expect("selects");
    let record = run(world, &clause(world, name), None);
    (selected, refuted(&record).clone())
}

#[trace("QSpec-TC-972", "QSpec-FR-483-AC-3")]
#[trace("QSpec-TC-974", "QSpec-FR-485-AC-10")]
#[test]
fn a_counterexample_the_replay_does_not_reproduce_is_not_a_refutation() {
    let world = world(
        "invariant Low using v on Config::ConfigVersion at current { self.versionNumber <= 1 }\n",
        0,
    );
    let (selected, counterexample) = target_and_violation(&world, "Low");
    let system = ModelSystem::new(world.subject(&["a"], UNLIMITED).simulation).expect("admits");
    let target = ContractTarget::new(selected, &world.package, &system, None)
        .expect("no fault")
        .expect("decided");
    assert!(target.replay(&counterexample).unwrap().is_some());
    // The recorded post-state is replaced by one the operation does not
    // reach: the replay does not reproduce it.
    let mut tampered = counterexample.clone();
    tampered.executions[0].post_state = Some(tampered.executions[0].pre_state.clone());
    assert!(target.replay(&tampered).unwrap().is_none());
    // A case that holds is no violation.
    let holds = system
        .declared_states()
        .expect("enumerable")
        .map(|state| state.expect("enumerates"))
        .find(|state| {
            state
                .state()
                .populations
                .values()
                .all(|objects| objects.is_empty())
        })
        .expect("the empty pre-state");
    assert!(matches!(
        target.evaluate(&ContractCase::Declared(holds)).unwrap(),
        CaseVerdict::Inapplicable | CaseVerdict::Holds
    ));
}

#[trace("QSpec-TC-974", "QSpec-FR-485-AC-13")]
#[trace("QSpec-TC-972", "QSpec-FR-483-AC-3")]
#[test]
fn a_reader_with_no_implementation_engine_refuses_a_bound_counterexample() {
    let world = world(
        "invariant Low using v on Config::ConfigVersion at current { self.versionNumber <= 1 }\n",
        0,
    );
    let (selected, counterexample) = target_and_violation(&world, "Low");
    let system = ModelSystem::new(world.subject(&["a"], UNLIMITED).simulation).expect("admits");
    let target = ContractTarget::new(selected, &world.package, &system, None)
        .expect("no fault")
        .expect("decided");
    let mut bound = counterexample.clone();
    bound.bound = true;
    assert!(matches!(
        target.read(&bound).unwrap(),
        ReadCounterexample::ReplayRefused
    ));
    let refused = target.read(&bound).unwrap().inconclusive_cause();
    assert!(
        matches!(refused, Some(InconclusiveCause::ReplayRefused(_))),
        "{refused:?}"
    );
    let unbound = target.read(&counterexample).unwrap();
    assert!(matches!(unbound, ReadCounterexample::Reproduced(_)));
    assert!(unbound.inconclusive_cause().is_none(), "it settles refuted");
}

// ---- simulation: ADR-020 §8's `CasRefinesCounter` ----

mod simulation {
    use super::*;
    use crate::refinement_fixture::{
        cas_refines_counter, lost_update_refinement, Variant, World as Cas,
    };
    use qsl_analyze::contract_check::AbstractSubject;
    use quire_semantic_value::step_simulation::StepObligation;

    /// The occurrence of the step-simulation item whose obligation is the
    /// initial rule (`None`) or the step row of the concrete operation
    /// `operation`.
    fn item(world: &Cas, operation: Option<&str>) -> OccurrenceKey {
        let refinement = world.refinement();
        refinement
            .step_simulations()
            .iter()
            .find(|entry| match (entry.record().obligation, operation) {
                (StepObligation::Initial, None) => true,
                (StepObligation::Step { row }, Some(name)) => {
                    (0..refinement.steps().len()).any(|source| {
                        refinement.step_row_ref(source) == Some(row)
                            && refinement.steps()[source]
                                .concrete_operation()
                                .is_some_and(|(_, operation)| operation == name)
                    })
                }
                _ => false,
            })
            .map(|entry| entry.occurrence().clone())
            .expect("the item exists")
    }

    fn run(world: &Cas, claim: &OccurrenceKey) -> AnalyzeRecord {
        let request = select_contract(world.package(), claim).map(|selected| ContractRequest {
            package: world.package(),
            claim: selected,
            subject: world.concrete_request(UNLIMITED),
            abstract_subject: Some(AbstractSubject {
                package: world.package(),
                initial: vec![world.abstract_initial()],
            }),
            max_cases: None,
        });
        analyze(
            &Engines::qsl().expect("the linked engines register"),
            vec![contract_item(RequestIndex::new(0), request)],
            &Cancel::new(),
        )
        .remove(0)
    }

    #[trace("QSpec-TC-974", "QSpec-FR-485-AC-9")]
    #[test]
    fn the_initial_rule_is_ordinal_zero_and_holds_when_initial_states_map() {
        let world = Cas::new(&cas_refines_counter());
        let claim = item(&world, None);
        assert_eq!(claim.origin().ordinal(), 0);
        assert_proved(&run(&world, &claim));
    }

    #[trace("QSpec-TC-974", "QSpec-FR-485-AC-9")]
    #[test]
    fn an_initial_state_that_maps_to_no_abstract_initial_state_refutes_the_initial_rule() {
        let world = Cas::with(
            Variant {
                concrete_value: 1,
                ..Variant::CAS
            },
            &cas_refines_counter(),
        );
        let record = run(&world, &item(&world, None));
        let counterexample = refuted(&record);
        assert_eq!(
            counterexample.obligation,
            ContractObligation::SimulationInitial
        );
        assert!(counterexample.executions[0].operation.is_none());
    }

    #[trace("QSpec-TC-974", "QSpec-FR-485-AC-9")]
    #[test]
    fn a_compare_and_set_commit_simulates_inc_from_every_declared_pre_state() {
        let world = Cas::new(&cas_refines_counter());
        let claim = item(&world, Some("commitA"));
        assert!(claim.origin().ordinal() >= 1, "step row k is ordinal k + 1");
        assert_proved(&run(&world, &claim));
        assert_proved(&run(&world, &item(&world, Some("beginA"))));
    }

    #[trace("QSpec-TC-974", "QSpec-FR-485-AC-9")]
    #[test]
    fn a_lost_update_commit_is_refuted_never_inconclusive() {
        let world = Cas::with(Variant::LOST_UPDATE, &lost_update_refinement());
        let record = run(&world, &item(&world, Some("commitA")));
        let counterexample = refuted(&record);
        assert_eq!(
            counterexample.obligation,
            ContractObligation::SimulationStep
        );
        let [execution] = counterexample.executions.as_slice() else {
            panic!("one execution");
        };
        assert!(execution.transition.is_some());
        assert!(execution.post_state.is_some());
    }
}

// ---- implementation bindings (FR-353) ----

/// FR-304-AC-1's relation over the ConfigVersion package, binding
/// `attemptUpdate`; with `typed`, also `ConfigVersion` and its field.
fn relation(typed: bool) -> String {
    let typed = if typed {
        "type Config::ConfigVersion to \"store::Version\" { versionNumber to \"number\"; };\n"
    } else {
        ""
    };
    format!(
        "abstraction A using v {{\n{typed}\
         population Config::config_history to \"store::history\";\n\
         operation Config::ConfigVersion::attemptUpdate to \"store::update\" receiver \"self\" ();\n\
         }}\n"
    )
}

const BOUNDED_POST: &str = "post Bounded using v on Config::ConfigVersion::attemptUpdate \
     { self.versionNumber >= 0 }\n";

/// The world with `body` and an abstraction relation, under the model
/// profile that admits it.
fn bound_world(body: &str) -> World {
    World::with_profiles(
        config_version(),
        "profile m = \"quire.model.complete/v1\";\n",
        &format!("{CONFIG_CONTRACT}{body}"),
        &[&[("a", &[("versionNumber", 0)])]],
    )
}

#[trace("QSpec-TC-974", "QSpec-FR-485-AC-5")]
#[trace("QSpec-TC-972", "QSpec-FR-483-AC-1")]
#[test]
fn a_bound_clause_is_unsupported_with_no_implementation_engine() {
    let world = bound_world(&format!("{BOUNDED_POST}{}", relation(true)));
    let claim = clause(&world, "Bounded");
    assert!(select_contract(&world.package, &claim)
        .expect("selects")
        .bound());
    let record = run(&world, &claim, None);
    let TerminalValue::Unsupported(UnavailabilityCause::ContractCheck { form, reason }) =
        terminal(&record)
    else {
        panic!("expected unsupported: {:?}", terminal(&record));
    };
    assert_eq!(form.as_str(), "clause");
    assert_eq!(reason.as_str(), "no-implementation-engine");
    // FR-483-AC-1: an arm outcome `unsupported` is a result on a `supported`
    // disposition, not a disposition.
    assert!(matches!(record.settlement, Settlement::Terminal(_)));
    assert!(record.contract_check.is_none());
}

/// SR FND-001 (QSpec FR-485 "The oracle"): an invariant whose context has an
/// operation with an implementation binding settles `unsupported`
/// `no-implementation-engine` and records no counterexample. The invariant
/// is one the contract relation breaks (`attemptUpdate` takes 1 to 2), and
/// the same invariant with no binding is refuted, so the arm ran no check.
#[trace("QSpec-TC-974", "QSpec-FR-485-AC-5")]
#[trace("QSpec-TC-972", "QSpec-FR-483-AC-1")]
#[test]
fn an_invariant_over_a_bound_operation_is_unsupported_and_runs_no_check() {
    const LOW: &str =
        "invariant Low using v on Config::ConfigVersion at current { self.versionNumber <= 1 }\n";
    let unbound = world(LOW, 0);
    assert!(matches!(
        terminal(&run(&unbound, &clause(&unbound, "Low"), None)),
        TerminalValue::Refuted
    ));
    let bound = bound_world(&format!("{LOW}{}", relation(true)));
    let claim = clause(&bound, "Low");
    let selected = select_contract(&bound.package, &claim).expect("selects");
    assert_eq!(selected.form().as_str(), "invariant");
    let record = run(&bound, &claim, None);
    let TerminalValue::Unsupported(UnavailabilityCause::ContractCheck { form, reason }) =
        terminal(&record)
    else {
        panic!("expected unsupported: {:?}", terminal(&record));
    };
    assert_eq!(form.as_str(), "invariant");
    assert_eq!(reason.as_str(), "no-implementation-engine");
    assert!(matches!(record.settlement, Settlement::Terminal(_)));
    assert!(record.contract_check.is_none(), "no counterexample");
}

/// QSpec FR-485-AC-12: an `operation-contract` item over an operation whose
/// context population has no maximum is `unbounded` with one `population`
/// domain, the arm a candidate in its `bounded` mode; with a cardinality
/// bound on that population the item is `bounded`.
#[trace("QSpec-TC-974", "QSpec-FR-485-AC-12")]
#[test]
fn a_clause_item_over_an_unbounded_population_is_unbounded_with_one_population_domain() {
    use qsl_foundation::bound::DomainKind;
    use qsl_route::request::{items_from_requirements, ExtentClassification};
    use qsl_semantics::family::ClaimExtent;

    const POST: &str = "post Bounded using v on Config::ConfigVersion::attemptUpdate \
         { self.versionNumber >= 0 }\n";
    let engines = Engines::qsl().expect("the linked engines register");
    let mut registry = qsl_route::Registry::new();
    for descriptor in engines.registrations() {
        registry.register(descriptor.clone()).expect("registers");
    }
    let item_of = |world: &World| {
        let claim = clause(world, "Bounded");
        let graph = world.package.graph();
        let record = graph.requirements().get(&claim).expect("its record");
        assert_eq!(
            record.requirements().kind(),
            qsl_semantics::check::Capability::OperationContract
        );
        let extent = record.requirements().extent().clone();
        let item = items_from_requirements(graph.requirements(), &registry, None)
            .into_iter()
            .find(|item| item.occurrence() == &claim)
            .expect("the clause's item");
        (extent, item)
    };

    let (extent, item) = item_of(&world(POST, 0));
    let ClaimExtent::Unbounded(domains) = extent else {
        panic!("unbounded: {extent:?}");
    };
    let kinds: Vec<DomainKind> = domains.iter().map(|(_, kind)| kind).collect();
    assert_eq!(kinds, [DomainKind::Population]);
    assert_eq!(
        item.extent(),
        ExtentClassification::Unbounded {
            finite_bound_available: true
        }
    );

    let bounded = world(
        &format!("{POST}bound HistoryBound on Config::config_history [2]\n"),
        0,
    );
    let (extent, item) = item_of(&bounded);
    assert_eq!(extent, ClaimExtent::Bounded);
    assert_eq!(item.extent(), ExtentClassification::Bounded);
}

#[trace("QSpec-TC-974", "QSpec-FR-485-AC-7")]
#[test]
fn a_bound_clause_reading_an_unbound_element_is_declined_incomplete_input() {
    let world = bound_world(&format!("{BOUNDED_POST}{}", relation(false)));
    let claim = clause(&world, "Bounded");
    assert!(matches!(
        select_contract(&world.package, &claim),
        Err(ContractSelection::Unbound(_))
    ));
    let record = run(&world, &claim, None);
    let TerminalValue::Declined { cause, .. } = terminal(&record) else {
        panic!("declined, not {:?}", terminal(&record));
    };
    assert_eq!(cause.as_str(), "incomplete-input");
}

// ---- a refinement with no per-step reading (QSpec FR-485-AC-14) ----

/// Run the simulation items of `refinement` at ordinal 0 (OC-24, the
/// initial rule) and 1 (OC-23, step row 0) and assert each settles
/// `unsupported` `no-contract-level-reading` with the FR-483 diagnostic,
/// runs no check and records no counterexample.
fn assert_no_per_step_reading(
    package: &qsl_package::CheckedPackage,
    refinement: &qsl_semantics::check::CheckedRefinement,
    subject: qsl_eval::simulation::model::ModelSimulationRequest<'_>,
    abstract_initial: qsl_semantics::model::observation::DocumentRef,
) {
    use qsl_analyze::contract_check::AbstractSubject;

    assert!(!refinement.proof_eligible());
    assert!(refinement.step_simulations().is_empty());
    let node = refinement.v2_emission().expect("emits").identity;
    for ordinal in [0, 1] {
        let claim = OccurrenceKey::new(node.clone(), Origin::new(Role::new("generated"), ordinal));
        let request = select_contract(package, &claim).map(|selected| ContractRequest {
            package,
            claim: selected,
            subject: subject.clone(),
            abstract_subject: Some(AbstractSubject {
                package,
                initial: vec![abstract_initial.clone()],
            }),
            // A budget of one case: a check that ran would reach it.
            max_cases: NonZeroU64::new(1),
        });
        let record = analyze(
            &Engines::qsl().expect("the linked engines register"),
            vec![contract_item(RequestIndex::new(0), request)],
            &Cancel::new(),
        )
        .remove(0);
        let TerminalValue::Unsupported(cause) = terminal(&record) else {
            panic!(
                "ordinal {ordinal}: expected unsupported: {:?}",
                terminal(&record)
            );
        };
        assert_eq!(cause.code(), Some("unsupported_projection"));
        assert_eq!(cause.as_str(), "unsupported-requested-capability");
        let UnavailabilityCause::ContractCheck { form, reason } = cause else {
            panic!("the arm's own cause: {cause:?}");
        };
        assert_eq!(form.as_str(), "simulation");
        assert_eq!(reason.as_str(), "no-contract-level-reading");
        assert!(record.contract_check.is_none(), "no counterexample");
    }
}

/// QSpec FR-485-AC-14 (OC-23, OC-24): FR-141-AC-4's `Coin` with `side`
/// hidden. (A refinement with a history field has no V2 refinement node, so
/// no claim can name one of its rules.)
#[trace("QSpec-TC-974", "QSpec-FR-485-AC-14")]
#[test]
fn a_rule_of_a_refinement_with_a_hidden_field_has_no_contract_level_reading() {
    use crate::refinement_aux::{coin_refines, Unit};
    let unit = Unit::coin(1);
    let world = unit.world(&coin_refines(""));
    assert_no_per_step_reading(
        world.package(),
        world.refinement(),
        world.concrete_request(UNLIMITED),
        world.abstract_initial(),
    );
}

/// QSpec FR-485-AC-14 (OC-23, OC-24): the lost-update refinement with its
/// commit rows `-> any`.
#[trace("QSpec-TC-974", "QSpec-FR-485-AC-14")]
#[test]
fn a_rule_of_a_refinement_with_an_any_row_has_no_contract_level_reading() {
    use crate::refinement_fixture::{lost_update_refinement, Variant, World as Cas};
    let any = lost_update_refinement()
        .replace(
            "step Impl::Counter::commitA -> Spec::Counter::inc(self);",
            "step Impl::Counter::commitA -> any;",
        )
        .replace(
            "step Impl::Counter::commitB -> Spec::Counter::inc(self);",
            "step Impl::Counter::commitB -> any;",
        );
    let world = Cas::with(Variant::LOST_UPDATE, &any);
    assert_no_per_step_reading(
        world.package(),
        world.refinement(),
        world.concrete_request(UNLIMITED),
        world.abstract_initial(),
    );
}

/// QSpec FR-485 "Forms" over a QSL-emitted step relation (FR-180): with
/// `attemptUpdate` bound by the abstraction relation, QSL writes the
/// relation's operation-contract code claim beside its model claim. The
/// code claim selects the `step-relation` form and, the arm holding no
/// implementation engine, settles `unsupported` `no-implementation-engine`;
/// the model claim, a `temporal-satisfaction` item, selects no form.
#[trace("QSpec-TC-974", "QSpec-FR-485-AC-8")]
#[test]
fn a_bound_step_relation_is_unsupported_with_no_implementation_engine() {
    let world = World::with_profiles(
        config_version(),
        "profile m = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.infinite-trace/v1\";\n",
        &format!(
            "{CONFIG_CONTRACT}\
             relation Det using t over (x: Config::ConfigVersion::attemptUpdate, \
             y: Config::ConfigVersion::attemptUpdate) \
             {{ pre(x.self.versionNumber) = pre(y.self.versionNumber) \
             implies x.self.versionNumber = y.self.versionNumber }}\n{}",
            relation(true)
        ),
        &[&[("a", &[("versionNumber", 0)])]],
    );
    let graph = world.package.graph();
    let [relation] = graph.step_relations() else {
        panic!("one step relation");
    };
    let node = relation.identity().expect("assembled").clone();
    let code: Vec<&OccurrenceKey> = graph
        .requirements()
        .iter()
        .filter(|(occurrence, record)| {
            occurrence.node() == node
                && record.requirements().kind()
                    == qsl_semantics::check::Capability::OperationContract
        })
        .map(|(occurrence, _)| occurrence)
        .collect();
    let [code] = code[..] else {
        panic!("one code claim: {code:?}");
    };
    let selected = select_contract(&world.package, code).expect("selects");
    assert_eq!(selected.form().as_str(), "step-relation");
    assert!(selected.bound());
    let record = run(&world, code, None);
    let TerminalValue::Unsupported(UnavailabilityCause::ContractCheck { form, reason }) =
        terminal(&record)
    else {
        panic!("expected unsupported: {:?}", terminal(&record));
    };
    assert_eq!(form.as_str(), "step-relation");
    assert_eq!(reason.as_str(), "no-implementation-engine");

    let model = relation.occurrence().expect("the model claim");
    assert_eq!(
        select_contract(&world.package, model),
        Err(ContractSelection::NoForm)
    );
}
