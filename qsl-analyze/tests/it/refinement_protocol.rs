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
    open_protocol_systems, replay_protocol_assumption_witness, replay_refinement_trace,
    settle_refinement_safety, settle_refinement_safety_with_assumption_replay, Certification,
    ClosureRefusal, ConcreteStep, ModelCheckLimits, ProofBasis, ProtocolAssumptionReplaySettlement,
    RefinementPosition, RefinementTraceSettlement, SafetyOutcome, SettlementBasis,
    SimulationCertificateRequest, SimulationRefusal, StepVerdict, TerminalValue,
};
use quire_exact::Cancel;

use crate::refinement_fixture::{cas_refines_counter, World, UNLIMITED};
use crate::refinement_fixture::{key, Variant, CAS, SPEC};

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
         role w on Spec::Counter; scheduling adversarial;\n\
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
            // The concrete model declares its completed value=2 state as
            // intended terminal, matching the protocol's successful finish
            // after both `inc` attempts.
            intended_terminal: true,
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
    assert!(
        matches!(&outcome.safety, SafetyOutcome::HoldsUncertified { .. }),
        "unexpected safety outcome: {:?}",
        outcome.safety
    );
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

/// FR-148-AC-7 (TC-553): a protocol with adversarial scheduling cannot
/// establish a concrete weak-fairness assumption that excludes a paired
/// `peek` loop while a different concrete operation remains continuously
/// enabled.
#[trace("TC-553", "FR-148-AC-7")]
#[test]
fn a_stronger_concrete_assumption_never_settles_proved() {
    let source = cas_twice(true, false)
        .replace(
            "protocol Twice",
            "pre PeekPre using v on Impl::Counter::peek { self.value = 2 }\nprotocol Twice",
        )
        .replace(
            "step Impl::Counter::peek -> stutter;",
            "step Impl::Counter::peek -> stutter;\nassume fair weak whole Impl::Counter::beginA;",
        );
    let world = World::with_internals(
        Variant {
            peek: true,
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
    let SafetyOutcome::Undecided {
        cause: qsl_replay::SafetyUndecided::ProtocolAssumptionNotEstablished(witness),
        ..
    } = &outcome.safety
    else {
        panic!(
            "a strengthened implementation assumption cannot be proved: {:?}",
            outcome.safety
        );
    };
    assert_eq!(
        witness.reason,
        qsl_replay::ProtocolAssumptionReason::Strengthened
    );
    assert!(witness.path.loop_entry.is_some());
    assert_eq!(witness.identities.len(), 1);
    let systems = open_protocol_systems(
        world.refinement(),
        &world.concrete_request(UNLIMITED),
        world.package(),
        vec![world.abstract_initial()],
        protocol_binding().over,
    )
    .expect("the assumption lasso replays over its admitted systems");
    assert_eq!(
        replay_protocol_assumption_witness(world.refinement(), &systems, &witness)
            .expect("typed lasso replay runs"),
        ProtocolAssumptionReplaySettlement::Reproduced
    );
    let mut stale = witness.as_ref().clone();
    stale.path.loop_entry = Some(usize::MAX);
    assert!(matches!(
        replay_protocol_assumption_witness(world.refinement(), &systems, &stale)
            .expect("a stale assumption lasso settles by parity"),
        ProtocolAssumptionReplaySettlement::Inconclusive(_)
    ));
}

/// FR-148-AC-6 (TC-553): the protocol's derived weak whole scheduler
/// constraint is checked over protocol transitions and implies the matching
/// concrete weak whole operation assumption on the paired product.
#[trace("TC-553", "FR-148-AC-6")]
#[test]
fn protocol_scheduler_fairness_can_imply_a_concrete_assumption() {
    let source = cas_twice(false, false)
        .replace(
            "role w on Spec::Counter; scheduling adversarial;",
            "role w on Spec::Counter;",
        )
        .replace(
            "step Impl::Counter::commitB -> Spec::Counter::inc(self);",
            "step Impl::Counter::commitB -> Spec::Counter::inc(self);\nassume fair weak whole Impl::Counter::commitA;",
        );
    let world = World::with_internals(
        Variant {
            peek: false,
            intended_terminal: true,
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
    assert!(
        matches!(outcome.safety, SafetyOutcome::HoldsUncertified { .. }),
        "the protocol scheduler fairness establishes the concrete assumption: {:?}",
        outcome.safety
    );
}

/// FR-148-AC-8 (TC-553): the scheduler's repeatedly taken event class does
/// not imply a second, distinct concrete operation assumption that remains
/// enabled and untaken on the same paired lasso.
fn ac8_two_class_source() -> String {
    let source = cas_twice(true, true)
        .replace(
            "run sequence Main {\n\
         attempt a1 by w on Spec::Counter::inc contracts [] as (r1: Spec::Counter) { true };\n\
         attempt a2 by w on Spec::Counter::inc contracts [] as (r2: Spec::Counter) { true };\n\
         }",
            "run repeat Loop by w visible () while { true } sequence Main { event Ack by w as (worked: Boolean) { worked = true }; }",
        )
        .replace(
            "role w on Spec::Counter; scheduling adversarial;",
            "role w on Spec::Counter;",
        )
        .replace(
            "object Spec::Counter from Impl::Counter { value = self.value; }",
            "object Spec::Counter from Impl::Counter { }",
        )
        .replace(
            "step Impl::Counter::commitA -> Spec::Counter::inc(self);",
            "step Impl::Counter::commitA -> node Spec::Twice::Ack(true);\n\
         assume fair weak whole Impl::Counter::commitA;",
        );
    source
        .replace(
            "step Impl::Counter::commitB -> Spec::Counter::inc(self);",
            "step Impl::Counter::commitB -> node Spec::Twice::Ack(true);",
        )
        .replace(
            "step Impl::Counter::peek -> stutter;",
            "step Impl::Counter::peek -> node Spec::Twice::Ack(true);\n\
         assume fair weak whole Impl::Counter::peek;",
        )
}

#[trace("TC-553", "FR-148-AC-8")]
#[test]
fn one_protocol_class_does_not_imply_a_second_concrete_operation_assumption() {
    let source = ac8_two_class_source();
    let world = World::with_internals(
        Variant {
            peek: true,
            lost_update: true,
            disable_begin: true,
            ..Variant::CAS
        },
        &source,
        serde_json::json!([{"refinement":"CasTwice","internal":["Spec::Twice::Done"]}]),
    );
    let mut snapshots = world.snapshot_provision().clone();
    let original = world.concrete_initial();
    let mut initial_snapshot: serde_json::Value = serde_json::from_slice(
        snapshots
            .get(&original)
            .expect("the original concrete snapshot is provisioned"),
    )
    .expect("the original concrete snapshot is JSON");
    initial_snapshot["populations"][0]["objects"][0]["fields"]["busyA"]["boolean"] =
        serde_json::json!(true);
    let bytes = initial_snapshot.to_string().into_bytes();
    let initial = qsl_semantics::model::observation::document_reference(&bytes)
        .expect("the modified snapshot names itself");
    snapshots.insert(initial.clone(), bytes);
    let mut concrete = world.concrete_request(UNLIMITED);
    concrete.snapshots = &snapshots;
    concrete.initial = vec![initial];
    let outcome = check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: concrete.clone(),
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
    let SafetyOutcome::Undecided {
        cause: qsl_replay::SafetyUndecided::ProtocolAssumptionNotEstablished(witness),
        ..
    } = &outcome.safety
    else {
        panic!(
            "the unmatched concrete assumption cannot be proved: {:?}",
            outcome.safety
        );
    };
    assert_eq!(
        witness.reason,
        qsl_replay::ProtocolAssumptionReason::Strengthened
    );
    let loop_entry = witness
        .path
        .loop_entry
        .expect("the witness has a paired lasso");
    assert!(
        witness.identities.iter().any(|identity| {
            identity.side == qsl_replay::AssumptionSide::Implementation
                && identity.origin == qsl_replay::AssumptionOrigin::Authored
                && matches!(identity.target, qsl_replay::AssumptionTarget::Model(_))
        }),
        "the unmet concrete operation keeps its checked model identity: {:?}",
        witness.identities
    );
    assert!(witness.identities.iter().any(|identity| {
        identity.side == qsl_replay::AssumptionSide::Specification
            && identity.origin == qsl_replay::AssumptionOrigin::Scheduler
            && matches!(
                &identity.target,
                qsl_replay::AssumptionTarget::Protocol(fairness)
                    if fairness.origin() == qsl_eval::simulation::protocol::FairnessOrigin::Scheduler
            )
            && matches!(identity.selected, Some(qsl_replay::AssumptionSelectedMember::Protocol(_)))
    }), "the protocol scheduler class remains typed and selected: {:?}", witness.identities);
    let peek = operation_identity_text(&key(crate::refinement_fixture::CAS, "Counter/peek"))
        .expect("the concrete peek identity is canonical");
    assert!(witness.path.steps[loop_entry..].iter().any(|step| step
        .concrete
        .as_ref()
        .is_some_and(|transition| transition.operation() == peek)));
    let commit = operation_identity_text(&key(crate::refinement_fixture::CAS, "Counter/commitA"))
        .expect("the unmet commit identity is canonical");
    assert!(!witness.path.steps[loop_entry..].iter().any(|step| step
        .concrete
        .as_ref()
        .is_some_and(|transition| transition.operation() == commit)));
    let systems = open_protocol_systems(
        world.refinement(),
        &concrete,
        world.package(),
        vec![world.abstract_initial()],
        protocol_binding().over,
    )
    .expect("the assumption lasso replays over its admitted systems");
    assert_eq!(
        replay_protocol_assumption_witness(world.refinement(), &systems, witness)
            .expect("typed lasso replay runs"),
        ProtocolAssumptionReplaySettlement::Reproduced
    );
}

/// FR-148-AC-8 (TC-553): an exploration budget that cuts off the protocol
/// product cannot be reported as a proved assumption relation.
#[trace("TC-553", "FR-148-AC-8")]
#[test]
fn a_protocol_product_limit_stops_assumption_decision_incomplete() {
    let world = World::with_internals(
        Variant {
            peek: true,
            lost_update: true,
            disable_begin: true,
            ..Variant::CAS
        },
        &ac8_two_class_source(),
        serde_json::json!([{"refinement":"CasTwice","internal":["Spec::Twice::Done"]}]),
    );
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_transitions = 1;
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
            limits,
        },
        &Cancel::new(),
    )
    .expect("the abstract protocol subject is admitted");
    assert!(
        matches!(
            outcome.safety,
            SafetyOutcome::Stopped {
                cause: qsl_replay::IncompleteCause::ResourceExhausted,
                limit: Some(qsl_replay::ReachedLimit {
                    limit: qsl_replay::ModelCheckLimit::MaxTransitions,
                    value: 1
                }),
                ..
            }
        ),
        "a product limit is incomplete, never proved: {:?}",
        outcome.safety
    );
}

/// FR-148-AC-10 (TC-553): a TSO flush fairness member participates in
/// the assumption relation and its paired lasso replays.
#[trace("TC-553", "FR-148-AC-10")]
#[test]
fn tso_flush_fairness_replays_against_an_unmet_inc_assumption() {
    let variant = Variant {
        lost_update: true,
        peek: false,
        disable_begin: true,
        tick: true,
        inc: true,
        ..Variant::CAS
    };
    let world_source = "refinement Base using inf abstract Spec concrete Impl {\n\
         population Spec::counters from Impl::counters;\n\
         object Spec::Counter from Impl::Counter { value = self.value; }\n\
         step Impl::Counter::tick -> Spec::Counter::inc(self);\n\
         step Impl::Counter::inc -> stutter;\n\
         }";
    let world = World::with(variant, world_source);
    let documents = crate::litmus::CellDocuments::new();
    let run = format!(
        "parallel Both memory tso {{ \
         branch writer repeat Tick by w visible () while {{ true }} sequence Writer {{ {} }} \
         branch reader sequence Reader {{ {} }} }} \
         join all [writer, reader];",
        crate::litmus::attempt("Tick", "w", "Cell::set", "relaxed", "IsX, StoreOne"),
        crate::litmus::attempt("Add", "r", "Cell::add", "relaxed", "AddReady, AddStays"),
    );
    let body = format!(
        "model Impl = \"{CAS}\";\n\
         pre IsX using v on M::Cell::set {{ self.id = 0 }}\n\
         post StoreOne using v on M::Cell::set {{ self.v = 1 }}\n\
         pre AddReady using v on M::Cell::add {{ self.id = 0 and self.v = 1 }}\n\
         post AddStays using v on M::Cell::add {{ self.v = pre(self.v) }}\n\
         pre TickReady using v on Impl::Counter::tick {{ self.value < 2 }}\n\
         post TickOne using v on Impl::Counter::tick {{ self.value = 1 }}\n\
         pre IncReady using v on Impl::Counter::inc {{ self.value = 1 }}\n\
         post IncStays using v on Impl::Counter::inc {{ self.value = pre(self.value) }}\n\
         {}\
         refinement Same using t0 abstract M::Twice concrete Impl {{\
         population M::cells from Impl::counters;\
         object M::Cell from Impl::Counter {{ id = self.id; }}\
         step Impl::Counter::tick -> M::Cell::set(self);\
         step Impl::Counter::inc -> stutter;\
         assume fair weak whole Impl::Counter::inc;\
         }}",
        crate::litmus::protocol("Twice", &run)
            .replace("using v", "using pp")
            .replace("over (k: M::Core)", "over (k: M::Cell)")
            .replace(
                "role c on M::Core;",
                "role r on M::Cell;\nscheduling adversarial;"
            ),
    );
    let mut package_provision = world.package_provision().clone();
    package_provision.extend(documents.packages.clone());
    let checked = crate::litmus::protocol_package_with_provisions(
        &body,
        package_provision,
        serde_json::json!([{"refinement":"Same","internal":[
            "M::Twice::Done",
            "M::Twice::Tick",
            "M::Twice::Add",
            "fork M::Twice::Both",
            "join M::Twice::Both",
            "memory flush"
        ]}]),
    );
    let refinement = checked
        .graph()
        .refinements()
        .first()
        .expect("the protocol refinement is checked");
    let mut snapshots = world.snapshot_provision().clone();
    snapshots.extend(documents.snapshots.clone());
    let mut package_provision = world.package_provision().clone();
    package_provision.extend(documents.packages.clone());
    let concrete_request = qsl_eval::simulation::model::ModelSimulationRequest {
        package: &checked,
        packages: &package_provision,
        normalization: qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        snapshots: &snapshots,
        initial: vec![world.concrete_initial()],
        universes: vec![qsl_eval::simulation::model::PopulationUniverse {
            population: key(CAS, "counters"),
            keys: world.object_keys().to_vec(),
        }],
        observation: qsl_semantics::model::observation::ObservationLimits::default(),
        expansion: qsl_eval::simulation::model::ExpansionLimits {
            max_candidates: 1_000_000,
        },
        meter: UNLIMITED,
    };
    let abstract_initial = vec![documents.initial.clone()];
    let abstract_over = qsl_eval::model_subject::ModelInstance {
        population: crate::litmus::key("cells"),
        key: "c".to_owned(),
    };
    let preopened = open_protocol_systems(
        refinement,
        &concrete_request,
        &checked,
        abstract_initial.clone(),
        abstract_over.clone(),
    )
    .expect("the abstract TSO protocol system opens");
    let concrete = ModelSubject {
        simulation: concrete_request,
        proof_bounds: Vec::new(),
    };
    let request = RefinementCheckRequest {
        refinement,
        concrete,
        abstract_package: &checked,
        abstract_initial: abstract_initial.clone(),
        abstract_protocol_binding: Some(qsl_analyze::model_check::AbstractProtocolBinding {
            over: abstract_over.clone(),
        }),
        max_depth: None,
        limits: ModelCheckLimits::default(),
    };
    let memory_fairness = preopened
        .abstract_protocol
        .as_ref()
        .expect("protocol system is present")
        .environment_fairness()
        .expect("environment fairness resolves");
    assert!(memory_fairness.iter().any(|fairness| {
        fairness.origin() == qsl_eval::simulation::protocol::FairnessOrigin::Memory
            && matches!(
                fairness.target(),
                qsl_eval::simulation::protocol::ProtocolTarget::MemoryStep { step: "flush", .. }
            )
    }));
    let outcome = check_refinement(request, &Cancel::new())
        .expect("the abstract TSO protocol subject is admitted");
    let SafetyOutcome::Undecided {
        cause: qsl_replay::SafetyUndecided::ProtocolAssumptionNotEstablished(witness),
        ..
    } = &outcome.safety
    else {
        panic!(
            "the TSO flush lasso must violate concrete inc fairness: {:?}",
            outcome.safety
        );
    };
    assert_eq!(
        witness.reason,
        qsl_replay::ProtocolAssumptionReason::Strengthened
    );
    let loop_entry = witness
        .path
        .loop_entry
        .expect("strengthening witness has a loop");
    let protocol_steps: Vec<_> = witness
        .path
        .steps
        .iter()
        .filter_map(|step| step.protocol.as_ref())
        .map(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok())
        .collect();
    assert!(
        witness
            .path
            .steps
            .iter()
            .skip(loop_entry)
            .filter_map(|step| step.protocol.as_ref())
            .filter_map(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok())
            .any(|step| {
                step["step"] == "memory"
                    && step["memory_step"]["step"] == "flush"
                    && step["memory_step"]["thread"]["steps"][0]["branch"] == "writer"
            }),
        "paired AC-10 loop must retain the writer flush; protocol transitions: {protocol_steps:?}"
    );
    assert!(witness.identities.iter().any(|identity| {
        identity.side == qsl_replay::AssumptionSide::Specification
            && identity.origin == qsl_replay::AssumptionOrigin::Memory
            && matches!(identity.target, qsl_replay::AssumptionTarget::Protocol(ref fairness)
                if matches!(fairness.target(), qsl_eval::simulation::protocol::ProtocolTarget::MemoryStep {
                    step: "flush", branch, ..
                } if branch == "writer"))
            && matches!(identity.selected, Some(qsl_replay::AssumptionSelectedMember::Protocol(
                qsl_eval::simulation::protocol::ProtocolClass::Thread(_)
            )))
    }), "expected selected memory fairness identity in {:?}; fairness {:?}; protocol steps {:?}", witness.identities, memory_fairness, protocol_steps);
    assert!(witness.identities.iter().any(|identity| {
        identity.side == qsl_replay::AssumptionSide::Implementation
            && matches!(identity.target, qsl_replay::AssumptionTarget::Model(ref fairness)
                if fairness.operation().name() == "inc")
    }));
    let replay = replay_protocol_assumption_witness(refinement, &preopened, witness)
        .expect("the TSO fairness witness replays");
    assert_eq!(replay, ProtocolAssumptionReplaySettlement::Reproduced);
}

/// FR-148-AC-12 (TC-553): an undecided precondition on an identified
/// concrete fairness class carries a finite paired prefix and remains an
/// inconclusive relation result rather than a proof or a failure.
#[trace("TC-553", "FR-148-AC-12")]
#[test]
fn an_undetermined_assumption_predicate_has_a_typed_finite_witness() {
    let source = cas_twice(false, false)
        .replace(
            "protocol Twice",
            "pre UnknownCommit using v on Impl::Counter::commitA { 6 mod (1 - self.value) >= 0 }\nprotocol Twice",
        )
        .replace(
            "step Impl::Counter::commitB -> Spec::Counter::inc(self);",
            "step Impl::Counter::commitB -> Spec::Counter::inc(self);\nassume fair weak whole Impl::Counter::commitA;",
        );
    let world = World::with_internals(
        Variant {
            peek: false,
            intended_terminal: true,
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
    let SafetyOutcome::Undecided {
        cause: qsl_replay::SafetyUndecided::ProtocolAssumptionNotEstablished(witness),
        ..
    } = &outcome.safety
    else {
        panic!(
            "an unknown fairness predicate cannot prove refinement: {:?}",
            outcome.safety
        );
    };
    assert_eq!(
        witness.reason,
        qsl_replay::ProtocolAssumptionReason::RelationUndetermined
    );
    assert!(witness.path.loop_entry.is_none());
    assert!(witness.undecided_locus.is_some());
    let systems = open_protocol_systems(
        world.refinement(),
        &world.concrete_request(UNLIMITED),
        world.package(),
        vec![world.abstract_initial()],
        protocol_binding().over,
    )
    .expect("the assumption witness replays over its admitted systems");
    assert_eq!(
        replay_protocol_assumption_witness(world.refinement(), &systems, witness)
            .expect("typed assumption witness replay runs"),
        ProtocolAssumptionReplaySettlement::Reproduced
    );
    let mut stale = witness.as_ref().clone();
    stale.path.protocol_initial = vec![0];
    assert!(matches!(
        replay_protocol_assumption_witness(world.refinement(), &systems, &stale)
            .expect("a stale assumption witness settles by parity"),
        ProtocolAssumptionReplaySettlement::Inconclusive(_)
    ));
    let mut stale_locus = witness.as_ref().clone();
    stale_locus.undecided_locus = Some("forged-contract-undetermined-locus".to_owned());
    assert!(matches!(
        replay_protocol_assumption_witness(world.refinement(), &systems, &stale_locus)
            .expect("a stale undecided locus settles by parity"),
        ProtocolAssumptionReplaySettlement::Inconclusive(_)
    ));
    let settled = settle_refinement_safety_with_assumption_replay(
        &outcome.safety,
        &ModelCheckLimits::default(),
        &SimulationCertificateRequest {
            refinement: world.refinement(),
            concrete: world.concrete_request(UNLIMITED),
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()],
        },
        &Cancel::new(),
        |_| unreachable!("an assumption witness is not a counterexample"),
        |witness| replay_protocol_assumption_witness(world.refinement(), &systems, witness),
    );
    assert!(
        !matches!(settled.value, TerminalValue::Proved { .. }),
        "an undecided assumption relation cannot settle proved"
    );
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

/// FR-148-AC-11 (TC-553): a stable concrete terminal state cannot refuse a
/// visible event enabled by every stable protocol candidate; replay checks
/// the relation at the endpoint after all incoming step rules pass.
#[trace("TC-553", "FR-148-AC-11")]
#[test]
fn a_terminal_refusal_of_a_protocol_event_replays() {
    let source = cas_twice(false, false).replace(
        "attempt a2 by w on Spec::Counter::inc contracts [] as (r2: Spec::Counter) { true };",
        "attempt a2 by w on Spec::Counter::inc contracts [] as (r2: Spec::Counter) { true };\n\
         event ack by w as (worked: Boolean) { true };",
    );
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
    let SafetyOutcome::Violated { counterexample, .. } = outcome.safety else {
        panic!(
            "terminal refusal violates the protocol relation: {:?}",
            outcome.safety
        );
    };
    assert!(
        matches!(
            counterexample.refinement,
            Some(qsl_replay::RefinementFailure::RefusalNotPermitted {
                position: 6,
                label: qsl_replay::ObservationLabel { ref step, .. },
            }) if step.contains("ack")
        ),
        "unexpected failure: {:?}; prefix {:?}",
        counterexample.refinement,
        counterexample.prefix
    );
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
        RefinementTraceSettlement::Reproduced {
            failure: qsl_replay::RefinementFailure::RefusalNotPermitted { .. },
            ..
        }
    ));
    let mut wrong_label = counterexample.as_ref().clone();
    if let Some(qsl_replay::RefinementFailure::RefusalNotPermitted { label, .. }) =
        wrong_label.refinement.as_mut()
    {
        label.step.push_str("/forged");
    }
    assert!(matches!(
        replay_refinement_trace(world.refinement(), &systems, &wrong_label)
            .expect("wrong refusal label replay runs"),
        RefinementTraceSettlement::Inconclusive(_)
    ));
    let mut wrong_position = counterexample.as_ref().clone();
    if let Some(qsl_replay::RefinementFailure::RefusalNotPermitted { position, .. }) =
        wrong_position.refinement.as_mut()
    {
        *position = position.saturating_add(1);
    }
    assert!(matches!(
        replay_refinement_trace(world.refinement(), &systems, &wrong_position)
            .expect("wrong refusal position replay runs"),
        RefinementTraceSettlement::Inconclusive(_)
    ));
}

/// FR-148-AC-11 (TC-553): a refusal is permitted if any stable protocol
/// candidate refuses the label, even when another stable candidate offers it.
#[trace("TC-553", "FR-148-AC-11")]
#[test]
fn a_refusal_is_permitted_when_one_stable_protocol_candidate_refuses() {
    let source = cas_twice(false, false)
        .replace(
            "protocol Twice using p over (c: Spec::Counter) on origin {",
            "protocol Twice using p over (c: Spec::Counter) on origin {",
        )
        .replace(
            "run sequence Main {\n\
         attempt a1 by w on Spec::Counter::inc contracts [] as (r1: Spec::Counter) { true };\n\
         attempt a2 by w on Spec::Counter::inc contracts [] as (r2: Spec::Counter) { true };\n\
         }\n\
         finish Done as (d: Boolean) { true };",
            "run sequence Main {\n\
         event Seed by w as (f: Boolean) { true };\n\
         choice Pick by w visible (f) { case offers when { f } sequence Offers { event ack by w as (worked: Boolean) { true }; } case refuses when { not f } sequence Refuses { } }\n\
         }\n\
         finish Done as (d: Boolean) { false };",
        );
    let world = World::with_internals(
        Variant {
            concrete_value: 0,
            disable_begin: true,
            peek: false,
            ..Variant::CAS
        },
        &source,
        serde_json::json!([{"refinement":"CasTwice","internal":["Spec::Twice::Seed", "Spec::Twice::Done"]}]),
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
    assert!(
        matches!(outcome.safety, SafetyOutcome::HoldsUncertified { .. }),
        "the refusing stable candidate permits the concrete refusal: {:?}",
        outcome.safety
    );
}

/// FR-148-AC-11 (TC-553): an incoming step-rule failure at the refusal
/// candidate state takes precedence over the endpoint refusal check.
#[trace("TC-553", "FR-148-AC-11")]
#[test]
fn an_incoming_step_rule_failure_precedes_refusal_at_the_same_state() {
    let source = cas_twice(false, false)
        .replace(
            "attempt a1 by w on Spec::Counter::inc contracts [] as (r1: Spec::Counter) { true };",
            "attempt a2 by w on Spec::Counter::inc contracts [] as (r2: Spec::Counter) { true };\n\
             event ack by w as (worked: Boolean) { true };",
        )
        .replace(
            "attempt a2 by w on Spec::Counter::inc contracts [] as (r2: Spec::Counter) { true };",
            "",
        )
        .replace(
            "step Impl::Counter::commitB -> Spec::Counter::inc(self);",
            "step Impl::Counter::commitB -> stutter;",
        );
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
    .expect("the protocol product admits");
    let SafetyOutcome::Violated { counterexample, .. } = outcome.safety else {
        panic!(
            "the invalid incoming step must fail before refusal: {:?}",
            outcome.safety
        );
    };
    assert!(
        matches!(
            counterexample.refinement,
            Some(qsl_replay::RefinementFailure::NoAbstractMatch { position: 2 })
        ),
        "step-rule failure must take precedence over refusal: {:?}",
        counterexample.refinement
    );
}

/// FR-148-AC-11 (TC-553): a terminal protocol state marked intended cannot
/// match a concrete deadlock whose model declares no intended terminal.
#[trace("TC-553", "FR-148-AC-11")]
#[test]
fn an_intended_protocol_terminal_mismatch_replays() {
    let source = cas_twice(false, true)
        .replace(
            "attempt a1 by w on Spec::Counter::inc contracts [] as (r1: Spec::Counter) { true };\n",
            "",
        )
        .replace(
            "attempt a2 by w on Spec::Counter::inc contracts [] as (r2: Spec::Counter) { true };\n",
            "",
        )
        .replace(
            "role w on Spec::Counter;",
            "role w on Spec::Counter;\nterminal any;",
        );
    let world = World::with_internals(
        Variant {
            concrete_value: 2,
            lost_update: true,
            disable_begin: true,
            peek: false,
            ..Variant::CAS
        },
        &source,
        serde_json::json!([{"refinement":"CasTwice","internal":["Spec::Twice::Done"]}]),
    );
    let initial = world.abstract_initial_two();
    let outcome = check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: vec![initial.clone()],
            abstract_protocol_binding: Some(protocol_binding()),
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the abstract protocol subject is admitted");
    let SafetyOutcome::Violated { counterexample, .. } = outcome.safety else {
        panic!(
            "terminal success mismatch violates the protocol relation: {:?}",
            outcome.safety
        );
    };
    assert!(
        matches!(
            counterexample.refinement,
            Some(qsl_replay::RefinementFailure::TerminalSuccessMismatch {
                position: 0,
                direction: qsl_replay::TerminalSuccessDirection::AbstractOnly,
            })
        ),
        "unexpected failure: {:?}; prefix {:?}",
        counterexample.refinement,
        counterexample.prefix
    );
    let systems = open_protocol_systems(
        world.refinement(),
        &world.concrete_request(UNLIMITED),
        world.package(),
        vec![initial],
        protocol_binding().over,
    )
    .expect("replay opens the same abstract protocol system");
    assert!(matches!(
        replay_refinement_trace(world.refinement(), &systems, &counterexample)
            .expect("counterexample replay runs"),
        RefinementTraceSettlement::Reproduced {
            failure: qsl_replay::RefinementFailure::TerminalSuccessMismatch { .. },
            ..
        }
    ));
    let mut wrong_position = counterexample.as_ref().clone();
    if let Some(qsl_replay::RefinementFailure::TerminalSuccessMismatch { position, .. }) =
        wrong_position.refinement.as_mut()
    {
        *position = 1;
    }
    assert!(matches!(
        replay_refinement_trace(world.refinement(), &systems, &wrong_position)
            .expect("wrong terminal position replay runs"),
        RefinementTraceSettlement::Inconclusive(_)
    ));
    let mut wrong_direction = counterexample.as_ref().clone();
    if let Some(qsl_replay::RefinementFailure::TerminalSuccessMismatch { direction, .. }) =
        wrong_direction.refinement.as_mut()
    {
        *direction = qsl_replay::TerminalSuccessDirection::ConcreteOnly;
    }
    assert!(matches!(
        replay_refinement_trace(world.refinement(), &systems, &wrong_direction)
            .expect("wrong terminal direction replay runs"),
        RefinementTraceSettlement::Inconclusive(_)
    ));
}

/// FR-148-AC-11 (TC-553): intended concrete terminal success cannot be paired
/// with a protocol terminal state whose checked terminal predicate is false.
#[trace("TC-553", "FR-148-AC-11")]
#[test]
fn a_concrete_terminal_success_mismatch_replays() {
    let source = cas_twice(false, true)
        .replace(
            "attempt a1 by w on Spec::Counter::inc contracts [] as (r1: Spec::Counter) { true };\n",
            "",
        )
        .replace(
            "attempt a2 by w on Spec::Counter::inc contracts [] as (r2: Spec::Counter) { true };\n",
            "",
        )
        .replace(
            "role w on Spec::Counter;",
            "role w on Spec::Counter;\nterminal when { c.value = 0 };",
        )
        .replace(
            "finish Done as (d: Boolean) { true };",
            "finish Done as (d: Boolean) { false };",
        );
    let world = World::with_internals(
        Variant {
            concrete_value: 2,
            lost_update: true,
            disable_begin: true,
            intended_terminal: true,
            peek: false,
            ..Variant::CAS
        },
        &source,
        serde_json::json!([{"refinement":"CasTwice","internal":[]}]),
    );
    let initial = world.abstract_initial_two();
    let outcome = check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: vec![initial.clone()],
            abstract_protocol_binding: Some(protocol_binding()),
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the abstract protocol subject is admitted");
    let SafetyOutcome::Violated { counterexample, .. } = outcome.safety else {
        panic!(
            "terminal success mismatch violates the protocol relation: {:?}",
            outcome.safety
        );
    };
    assert!(matches!(
        counterexample.refinement,
        Some(qsl_replay::RefinementFailure::TerminalSuccessMismatch {
            position: 0,
            direction: qsl_replay::TerminalSuccessDirection::ConcreteOnly,
        })
    ));
    let systems = open_protocol_systems(
        world.refinement(),
        &world.concrete_request(UNLIMITED),
        world.package(),
        vec![initial],
        protocol_binding().over,
    )
    .expect("replay opens the same abstract protocol system");
    assert!(matches!(
        replay_refinement_trace(world.refinement(), &systems, &counterexample)
            .expect("counterexample replay runs"),
        RefinementTraceSettlement::Reproduced {
            failure: qsl_replay::RefinementFailure::TerminalSuccessMismatch { .. },
            ..
        }
    ));
}

/// FR-148-AC-11 (TC-553): model-abstract refinements do not produce protocol
/// refusal or terminal-success failures.
#[trace("TC-553", "FR-148-AC-11")]
#[test]
fn model_abstract_refinements_exclude_protocol_refusal_and_terminal_failures() {
    let world = World::with(
        Variant {
            concrete_value: 1,
            ..Variant::CAS
        },
        &cas_refines_counter(),
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
            abstract_protocol_binding: None,
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the model-abstract refinement subject is admitted");
    assert!(matches!(
        outcome.safety,
        SafetyOutcome::Violated {
            counterexample,
            ..
        } if matches!(
            counterexample.refinement,
            Some(qsl_replay::RefinementFailure::InitialNotAbstract { .. })
        )
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
        protocol,
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
            protocol,
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
    let transitions: Vec<_> = match &taken {
        qsl_replay::Taken::Protocol(transitions) => transitions.iter().collect(),
        qsl_replay::Taken::ProtocolTraces(paths) => paths
            .iter()
            .flat_map(|path| path.transitions.iter())
            .collect(),
        other => panic!("any selects a visible protocol transition: {other:?}"),
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
    assert!(matches!(
        taken,
        qsl_replay::Taken::Protocol(_) | qsl_replay::Taken::ProtocolTraces(_)
    ));
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
