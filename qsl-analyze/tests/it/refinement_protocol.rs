// SPDX-License-Identifier: AGPL-3.0-or-later
//! A refinement whose step rows select a concrete protocol (FR-136): S3
//! checks it and FR-148 decides a model refinement against an abstract
//! protocol subject, with protocol-state replay inputs kept explicit.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_refinement, ModelCheckRefusal, RefinementCheckRequest};
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::operation_identity_text;
use qsl_eval::simulation::TransitionSystem;
use qsl_replay::{
    check_protocol_initial, check_protocol_step, check_simulation_certificate,
    open_protocol_systems, replay_refinement_trace, settle_refinement_safety, Certification,
    ClosureRefusal, ConcreteStep, ModelCheckLimits, ProofBasis, RefinementPosition,
    RefinementTraceSettlement, SafetyOutcome, SettlementBasis, SimulationCertificateRequest,
    SimulationRefusal, StepVerdict, TerminalValue,
};
use quire_exact::Cancel;

use crate::refinement_fixture::{cas_refines_counter, World, UNLIMITED};
use crate::refinement_fixture::{key, Variant, SPEC};

fn cas_twice(peek: bool, lost_update: bool) -> String {
    let retries = if lost_update {
        ""
    } else {
        "step Impl::Counter::retryA -> stutter;\n\
         step Impl::Counter::retryB -> stutter;\n"
    };
    let peek_row = if peek {
        "step Impl::Counter::peek -> stutter;\n"
    } else {
        ""
    };
    format!(
        "profile p = \"quire.protocol.finite-global/v1\";\n\
         protocol Twice using p over (c: Spec::Counter) on origin {{\n\
         role w on Spec::Counter;\n\
         run sequence Main {{\n\
         attempt a1 by w on Spec::Counter::inc contracts [] as (r1: Spec::Counter) {{ true }};\n\
         attempt a2 by w on Spec::Counter::inc contracts [] as (r2: Spec::Counter) {{ true }};\n\
         }}\n\
         finish Done as (d: Boolean) {{ true }};\n\
         }}\n\
         refinement CasTwice using inf abstract Spec::Twice concrete Impl {{\n\
         population Spec::counters from Impl::counters;\n\
         object Spec::Counter from Impl::Counter {{ value = self.value; }}\n\
         step Impl::Counter::commitA -> Spec::Counter::inc(self);\n\
         step Impl::Counter::commitB -> Spec::Counter::inc(self);\n\
         step Impl::Counter::beginA -> stutter;\n\
         step Impl::Counter::beginB -> stutter;\n\
         {retries}{peek_row}}}"
    )
}

fn cas_once() -> String {
    cas_twice(false, false)
        .replace("protocol Twice", "protocol Once")
        .replace("abstract Spec::Twice", "abstract Spec::Once")
        .replace("refinement CasTwice", "refinement CasOnce")
        .replace(
            "attempt a2 by w on Spec::Counter::inc contracts [] as (r2: Spec::Counter) { true };",
            "",
        )
}

fn protocol_binding() -> qsl_analyze::model_check::AbstractProtocolBinding {
    qsl_analyze::model_check::AbstractProtocolBinding {
        over: qsl_eval::model_subject::ModelInstance {
            population: key(SPEC, "counters"),
            key: "c".to_owned(),
        },
    }
}

/// `Run`, a protocol with an empty `run` sequence (S4 writes no node for an
/// `attempt` yet, QSL-1126) over the compare-and-set counter, and the refinement of the counter by it.
fn protocol_refinement() -> String {
    "profile pp = \"quire.protocol.finite-global/v1\";\n\
     protocol Run using v over (k: Impl::Counter) on origin {\n\
     role w on Impl::Counter;\n\
     run sequence Main { }\n\
     finish Done as (d: Boolean) { true };\n\
     }\n\
     refinement CasRun using inf abstract Spec concrete Impl {\n\
     population Spec::counters from Impl::counters;\n\
     object Spec::Counter from Impl::Counter { value = self.value; }\n\
     step Impl::Counter::peek -> stutter;\n\
     step Impl::Run::Done -> stutter;\n\
     }"
    .to_owned()
}

/// FR-148-AC-2 (TC-553): an exhaustive protocol refinement settles as a
/// closed-scope but uncertified proof after exploring the concrete product.
#[trace("TC-553", "FR-148-AC-2")]
#[test]
fn a_twice_protocol_refinement_holds_uncertified() {
    let source = cas_twice(false, false);
    let world = World::with_internals(
        Variant {
            peek: false,
            ..Variant::CAS
        },
        &source,
        serde_json::json!([{"refinement":"CasTwice","internal":["Spec::Twice::Done"]}]),
    );
    let outcome = check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()],
            abstract_protocol_binding: Some(protocol_binding()),
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the abstract protocol subject is admitted");
    assert!(matches!(
        &outcome.safety,
        SafetyOutcome::HoldsUncertified { .. }
    ));
    let settled = settle_refinement_safety(
        &outcome.safety,
        &ModelCheckLimits::default(),
        &SimulationCertificateRequest {
            refinement: world.refinement(),
            concrete: world.concrete_request(UNLIMITED),
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()],
        },
        &Cancel::new(),
        |_| unreachable!("a holding result does not need replay"),
    );
    assert_eq!(
        settled.value,
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Uncertified,
        }
    );
    assert_eq!(settled.basis, SettlementBasis::ClosedScope);
}

fn assert_refuted_and_replayed(
    source: String,
    variant: Variant,
    refinement_name: &str,
    protocol_name: &str,
    expected_position: u64,
) {
    let world = World::with_internals(
        variant,
        &source,
        serde_json::json!([{"refinement":refinement_name,"internal":[format!("Spec::{protocol_name}::Done")]}]),
    );
    let outcome = check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()],
            abstract_protocol_binding: Some(protocol_binding()),
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the abstract protocol subject is admitted");
    let SafetyOutcome::Violated { counterexample, .. } = outcome.safety else {
        panic!("protocol refinement is refuted: {:?}", outcome.safety);
    };
    assert!(matches!(
        counterexample.refinement,
        Some(qsl_replay::RefinementFailure::NoAbstractMatch { position }) if position == expected_position
    ));
    let systems = open_protocol_systems(
        world.refinement(),
        &world.concrete_request(UNLIMITED),
        world.package(),
        vec![world.abstract_initial()],
        protocol_binding().over,
    )
    .expect("replay opens the same abstract protocol system");
    assert!(matches!(
        replay_refinement_trace(world.refinement(), &systems, &counterexample)
            .expect("counterexample replay runs"),
        RefinementTraceSettlement::Reproduced { .. }
    ));
}

/// FR-148-AC-3 (TC-553): a lost update fails at the commit that leaves the
/// mapped value unchanged, and replay recomputes the protocol state set.
#[trace("TC-553", "FR-148-AC-3")]
#[test]
fn lost_update_protocol_counterexample_replays() {
    assert_refuted_and_replayed(
        cas_twice(false, true),
        Variant {
            peek: false,
            ..Variant::LOST_UPDATE
        },
        "CasTwice",
        "Twice",
        4,
    );
}

/// FR-148-AC-4 (TC-553): the second commit has no successor in `Once` and
/// its protocol counterexample replays.
#[trace("TC-553", "FR-148-AC-4")]
#[test]
fn a_second_commit_against_once_replays() {
    assert_refuted_and_replayed(
        cas_once(),
        Variant {
            peek: false,
            ..Variant::CAS
        },
        "CasOnce",
        "Once",
        4,
    );
}

/// FR-148-AC-5 (TC-553): a mapped concrete `peek` loop diverges while every
/// protocol candidate at the initial product state enables `Twice::a1`.
#[trace("TC-553", "FR-148-AC-5")]
#[test]
fn a_stuttering_peek_loop_is_protocol_observable_divergence() {
    let world = World::with_internals(
        Variant {
            peek: true,
            ..Variant::CAS
        },
        &cas_twice(true, false),
        serde_json::json!([{"refinement":"CasTwice","internal":["Spec::Twice::Done"]}]),
    );
    let outcome = check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()],
            abstract_protocol_binding: Some(protocol_binding()),
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the abstract protocol subject is admitted");
    let SafetyOutcome::Violated { counterexample, .. } = outcome.safety else {
        panic!(
            "observable protocol divergence refutes the refinement: {:?}",
            outcome.safety
        );
    };
    assert_eq!(
        counterexample
            .prefix
            .iter()
            .map(|step| step.transition.operation())
            .collect::<Vec<_>>(),
        [operation_identity_text(&key(crate::refinement_fixture::CAS, "Counter/peek")).unwrap()]
    );
    assert_eq!(
        counterexample.lasso,
        Some(qsl_replay::Lasso {
            entry: 0,
            terminal_stutter: false
        })
    );
    assert!(matches!(
        counterexample.refinement,
        Some(qsl_replay::RefinementFailure::Divergence {
            constraint: None,
            protocol_class: Some(ref class),
        }) if class == "Twice::a1"
    ));
    let systems = open_protocol_systems(
        world.refinement(),
        &world.concrete_request(UNLIMITED),
        world.package(),
        vec![world.abstract_initial()],
        protocol_binding().over,
    )
    .expect("replay opens the same abstract protocol system");
    assert!(matches!(
        replay_refinement_trace(world.refinement(), &systems, &counterexample)
            .expect("protocol divergence replay runs"),
        RefinementTraceSettlement::Reproduced { .. }
    ));
}

/// FR-148-AC-1 (TC-553): roots and visible operation steps track protocol
/// control together with the mapped model state; `Done` closes internally.
#[trace("TC-553", "FR-148-AC-1")]
#[test]
fn check_initial_and_steps_retain_protocol_control_candidates() {
    let source = cas_twice(false, false);
    let world = World::with_internals(
        Variant {
            peek: false,
            ..Variant::CAS
        },
        &source,
        serde_json::json!([{"refinement":"CasTwice","internal":["Spec::Twice::Done"]}]),
    );
    let systems = open_protocol_systems(
        world.refinement(),
        &world.concrete_request(UNLIMITED),
        world.package(),
        vec![world.abstract_initial()],
        protocol_binding().over,
    )
    .expect("the admitted abstract protocol subject opens");
    let protocol = systems.abstract_protocol.as_ref().unwrap();
    let concrete_initial = systems.concrete.initial_states()[0].clone();
    let initial = check_protocol_initial(
        world.refinement(),
        &systems.concrete,
        &systems.abstract_system,
        &protocol,
        0,
        concrete_initial.state(),
    );
    let StepVerdict::Passes {
        post: mut position, ..
    } = initial.verdict
    else {
        panic!(
            "the initial state maps to the protocol root: {:?}",
            initial.verdict
        );
    };
    assert_eq!(position.abstract_protocol_states.len(), 1);
    assert_eq!(counter_value(&position.abstract_states[0]), 0);
    assert_eq!(
        protocol
            .successors(&position.abstract_protocol_states[0])
            .unwrap()
            .successors[0]
            .0
            .node(),
        Some("Main/a1")
    );

    let apply = |position: &RefinementPosition,
                 concrete_state: &qsl_eval::simulation::model::ModelSimState,
                 name: &str,
                 step_index: u64| {
        let transition = operation_identity_text(&key(
            crate::refinement_fixture::CAS,
            &format!("Counter/{name}"),
        ))
        .expect("the concrete operation identity");
        let expansion = systems.concrete.successors(concrete_state).unwrap();
        let (transition, post_state) = expansion
            .successors
            .into_iter()
            .find(|(identity, _)| identity.operation() == transition)
            .unwrap_or_else(|| panic!("the concrete step {name} is enabled"));
        let result = check_protocol_step(
            world.refinement(),
            &systems.concrete,
            &systems.abstract_system,
            &protocol,
            position,
            &ConcreteStep {
                position: step_index,
                transition: Some(&transition),
                post: &post_state,
            },
        );
        let StepVerdict::Passes { post, .. } = result.verdict else {
            panic!(
                "{name} passes against the abstract protocol: {:?}",
                result.verdict
            );
        };
        (post, post_state)
    };
    let (next, concrete_state) = apply(&position, &concrete_initial, "beginA", 1);
    position = next;
    let (next, concrete_state) = apply(&position, &concrete_state, "commitA", 2);
    position = next;
    assert_eq!(counter_value(&position.abstract_states[0]), 1);
    assert_eq!(position.abstract_protocol_states.len(), 1);
    assert_eq!(
        protocol
            .successors(&position.abstract_protocol_states[0])
            .unwrap()
            .successors[0]
            .0
            .node(),
        Some("Main/a2")
    );
    let (next, concrete_state) = apply(&position, &concrete_state, "beginB", 3);
    position = next;
    let (next, _concrete_state) = apply(&position, &concrete_state, "commitB", 4);
    position = next;
    assert_eq!(counter_value(&position.abstract_states[0]), 2);
    assert!(position
        .abstract_protocol_states
        .iter()
        .any(|state| protocol.successors(state).unwrap().successors.is_empty()));
}

/// FR-148 (TC-553): `any` admits the union of internal stutter and visible
/// protocol matches; this `commitA` takes the visible `a1` branch.
#[trace("TC-553", "FR-148-AC-1")]
#[test]
fn an_any_row_can_take_a_visible_protocol_transition() {
    let source = cas_twice(false, false).replace(
        "step Impl::Counter::commitA -> Spec::Counter::inc(self);",
        "step Impl::Counter::commitA -> any;",
    );
    let world = World::with_internals(
        Variant {
            peek: false,
            ..Variant::CAS
        },
        &source,
        serde_json::json!([{"refinement":"CasTwice","internal":["Spec::Twice::Done"]}]),
    );
    let systems = open_protocol_systems(
        world.refinement(),
        &world.concrete_request(UNLIMITED),
        world.package(),
        vec![world.abstract_initial()],
        protocol_binding().over,
    )
    .expect("the admitted abstract protocol subject opens");
    let protocol = systems.abstract_protocol.as_ref().unwrap();
    let initial_state = systems.concrete.initial_states()[0].clone();
    let initial = check_protocol_initial(
        world.refinement(),
        &systems.concrete,
        &systems.abstract_system,
        protocol,
        0,
        initial_state.state(),
    );
    let StepVerdict::Passes { post: position, .. } = initial.verdict else {
        panic!("the root maps: {:?}", initial.verdict);
    };
    let apply = |position: &RefinementPosition,
                 concrete_state: &qsl_eval::simulation::model::ModelSimState,
                 name: &str,
                 step_index: u64| {
        let operation = operation_identity_text(&key(
            crate::refinement_fixture::CAS,
            &format!("Counter/{name}"),
        ))
        .expect("the concrete operation identity");
        let (transition, post_state) = systems
            .concrete
            .successors(concrete_state)
            .unwrap()
            .successors
            .into_iter()
            .find(|(step, _)| step.operation() == operation)
            .unwrap_or_else(|| panic!("{name} is enabled"));
        let result = check_protocol_step(
            world.refinement(),
            &systems.concrete,
            &systems.abstract_system,
            protocol,
            position,
            &ConcreteStep {
                position: step_index,
                transition: Some(&transition),
                post: &post_state,
            },
        );
        let StepVerdict::Passes { post, taken } = result.verdict else {
            panic!("{name} passes: {:?}", result.verdict);
        };
        (post, post_state, taken)
    };
    let (position, concrete_state, _) = apply(&position, &initial_state, "beginA", 1);
    let (post, _concrete_state, taken) = apply(&position, &concrete_state, "commitA", 2);
    let qsl_replay::Taken::Protocol(transitions) = taken else {
        panic!("any selects a visible protocol transition: {taken:?}");
    };
    assert!(transitions
        .iter()
        .any(|transition| transition.node() == Some("Main/a1")));
    assert_eq!(counter_value(&post.abstract_states[0]), 1);
}

/// FR-147/FR-148 (TC-553): a node row compares its expression with the
/// attempt's typed receiver binder before accepting the protocol node.
#[trace("TC-553", "FR-148-AC-1")]
#[test]
fn a_node_row_matches_the_attempt_identity_and_binder() {
    let source =
        cas_twice(false, false).replace("Spec::Counter::inc(self)", "node Spec::Twice::a1(self)");
    let world = World::with_internals(
        Variant {
            peek: false,
            ..Variant::CAS
        },
        &source,
        serde_json::json!([{"refinement":"CasTwice","internal":["Spec::Twice::Done"]}]),
    );
    let systems = open_protocol_systems(
        world.refinement(),
        &world.concrete_request(UNLIMITED),
        world.package(),
        vec![world.abstract_initial()],
        protocol_binding().over,
    )
    .expect("the admitted abstract protocol subject opens");
    let protocol = systems.abstract_protocol.as_ref().unwrap();
    let initial_state = systems.concrete.initial_states()[0].clone();
    let initial = check_protocol_initial(
        world.refinement(),
        &systems.concrete,
        &systems.abstract_system,
        protocol,
        0,
        initial_state.state(),
    );
    let StepVerdict::Passes { post: position, .. } = initial.verdict else {
        panic!("the root maps: {:?}", initial.verdict);
    };
    let apply = |position: &RefinementPosition,
                 concrete_state: &qsl_eval::simulation::model::ModelSimState,
                 name: &str,
                 step_index: u64| {
        let operation = operation_identity_text(&key(
            crate::refinement_fixture::CAS,
            &format!("Counter/{name}"),
        ))
        .expect("the concrete operation identity");
        let (transition, post_state) = systems
            .concrete
            .successors(concrete_state)
            .unwrap()
            .successors
            .into_iter()
            .find(|(step, _)| step.operation() == operation)
            .unwrap_or_else(|| panic!("{name} is enabled"));
        let result = check_protocol_step(
            world.refinement(),
            &systems.concrete,
            &systems.abstract_system,
            protocol,
            position,
            &ConcreteStep {
                position: step_index,
                transition: Some(&transition),
                post: &post_state,
            },
        );
        let StepVerdict::Passes { post, taken } = result.verdict else {
            panic!("{name} passes the protocol node row: {:?}", result.verdict);
        };
        (post, post_state, taken)
    };
    let (position, concrete_state, _) = apply(&position, &initial_state, "beginA", 1);
    let (post, _, taken) = apply(&position, &concrete_state, "commitA", 2);
    assert!(matches!(taken, qsl_replay::Taken::Protocol(_)));
    assert_eq!(counter_value(&post.abstract_states[0]), 1);
}

fn counter_value(state: &qsl_semantics::model::state::ModelState) -> i64 {
    let value = state
        .populations
        .values()
        .flat_map(|objects| objects.values())
        .next()
        .and_then(|object| object.fields.get(&key(SPEC, "Counter/value")))
        .expect("the abstract counter value is present");
    match value {
        quire_exact::Value::Integer(value) => value.to_string().parse().expect("small integer"),
        other => panic!("integer counter value, got {other:?}"),
    }
}

/// The engine meets a refinement over a protocol and refuses it, naming the
/// cause, before it explores a state.
#[trace("TC-547")]
#[test]
fn the_engine_refuses_a_refinement_over_a_protocol() {
    let world = World::new(&protocol_refinement());
    assert!(world.refinement().concrete_protocol().is_some());
    let refused = check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()],
            abstract_protocol_binding: None,
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    );
    assert!(
        matches!(refused, Err(ModelCheckRefusal::ProtocolRefinement)),
        "{refused:?}"
    );
}

/// The certificate checker refuses a request over a protocol too, naming the
/// cause: the subjects it explores are concrete models.
#[trace("TC-556")]
#[test]
fn the_certificate_checker_refuses_a_refinement_over_a_protocol() {
    let model = World::new(&cas_refines_counter());
    let SafetyOutcome::Holds { certificate, .. } = check_refinement(
        RefinementCheckRequest {
            refinement: model.refinement(),
            concrete: ModelSubject {
                simulation: model.concrete_request(UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: model.package(),
            abstract_initial: vec![model.abstract_initial()],
            abstract_protocol_binding: None,
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the model refinement is checked")
    .safety
    else {
        panic!("the model refinement holds");
    };
    let mut certificate = *certificate;
    certificate.refinement = "CasRun".to_owned();

    let world = World::new(&protocol_refinement());
    let refused = check_simulation_certificate(
        &SimulationCertificateRequest {
            refinement: world.refinement(),
            concrete: world.concrete_request(UNLIMITED),
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()],
        },
        &certificate,
        &ModelCheckLimits::default(),
        &Cancel::new(),
    );
    assert!(
        matches!(
            refused,
            Err(SimulationRefusal::Subject(
                ClosureRefusal::ProtocolRefinement
            ))
        ),
        "{refused:?}"
    );
}
