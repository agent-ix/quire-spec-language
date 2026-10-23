// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-145 (TC-550) and FR-144 (TC-549, TC-555): a refinement counterexample
//! replays by re-executing its prefix and rerunning the step check, and the
//! safety half settles as a terminal record from the outcome and the replay.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_refinement, RefinementCheckRequest};
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{operation_identity_text, ModelSimulationRequest, ModelSystem};
use qsl_eval::simulation::TransitionSystem;
use qsl_package::CheckedPackage;
use qsl_replay::{
    abstract_classes, open_systems, replay_refinement_trace, settle_refinement,
    settle_refinement_refusal, settle_refinement_safety, Certification, CounterexampleKind,
    Execution, IncompleteCause, InconclusiveCause, Lasso, LivenessHalf, ModelCheckLimit,
    ModelCheckLimits, ModelCheckOutcome, ModelCheckRecord, ModelStep, ModelTraceRefusal,
    ProofBasis, ReachedLimit, RefinementFailure, RefinementOutcome, RefinementRecord,
    RefinementTraceSettlement, ReplayRefusal, ResultLabel, RunStats, SafetyOutcome,
    SafetyUndecided, SettlementBasis, SimulationCertificateRequest, TemporalCounterexample,
    TerminalValue, UnavailabilityCause, UndecidedCause,
};
use qsl_semantics::check::CheckedRefinement;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::Cancel;
use quire_semantic_value::catalog::InternalFault;

use crate::refinement_aux::{
    halting_refinement, hidden_register_history, int, pair_refinement, probe_contracts,
    probe_refinement, register_history_ensured, Unit, World as AuxWorld, NEVER,
    UNLIMITED as AUX_UNLIMITED,
};
use crate::refinement_fixture::{
    cas_refines_counter, divergence_refinement, key, lost_update_refinement, mutated, spec_state,
    Variant, World, CAS, UNLIMITED,
};
use crate::ring_world::{
    ring_is_queue, RingWorld, Variant as RingVariant, UNLIMITED as RING_UNLIMITED, ZERO,
};

/// What a replay or a settlement needs of a world.
struct Subject<'w> {
    refinement: &'w CheckedRefinement,
    package: &'w CheckedPackage,
    concrete: ModelSimulationRequest<'w>,
    abstract_initial: DocumentRef,
}

fn cas(world: &World) -> Subject<'_> {
    Subject {
        refinement: world.refinement(),
        package: world.package(),
        concrete: world.concrete_request(UNLIMITED),
        abstract_initial: world.abstract_initial(),
    }
}

fn ring(world: &RingWorld) -> Subject<'_> {
    ring_with(world, RING_UNLIMITED)
}

fn ring_with(world: &RingWorld, meter: quire_exact::ScalarLimits) -> Subject<'_> {
    Subject {
        refinement: world.refinement(),
        package: world.package(),
        concrete: world.concrete_request(meter),
        abstract_initial: world.abstract_initial(),
    }
}

fn aux<'w>(world: &'w AuxWorld<'_>) -> Subject<'w> {
    Subject {
        refinement: world.refinement(),
        package: world.package(),
        concrete: world.concrete_request(AUX_UNLIMITED),
        abstract_initial: world.abstract_initial(),
    }
}

fn check(subject: &Subject<'_>) -> SafetyOutcome {
    outcome(subject).safety
}

fn outcome(subject: &Subject<'_>) -> RefinementOutcome {
    check_refinement(
        RefinementCheckRequest {
            refinement: subject.refinement,
            concrete: ModelSubject {
                simulation: subject.concrete.clone(),
                proof_bounds: Vec::new(),
            },
            abstract_package: subject.package,
            abstract_initial: vec![subject.abstract_initial.clone()],
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the refinement is checked")
}

fn counterexample(outcome: SafetyOutcome) -> TemporalCounterexample {
    let SafetyOutcome::Violated { counterexample, .. } = outcome else {
        panic!("the refinement is violated: {outcome:?}");
    };
    *counterexample
}

fn replay(
    subject: &Subject<'_>,
    payload: &TemporalCounterexample,
) -> Result<RefinementTraceSettlement, ReplayRefusal> {
    let systems = open_systems(
        subject.refinement,
        &subject.concrete,
        subject.package,
        vec![subject.abstract_initial.clone()],
    )
    .expect("the subjects open");
    replay_refinement_trace(subject.refinement, &systems, payload)
}

fn settle(subject: &Subject<'_>, outcome: &SafetyOutcome) -> ModelCheckRecord {
    settle_refinement_safety(
        outcome,
        &ModelCheckLimits::default(),
        &certified(subject),
        &Cancel::new(),
        |payload| replay(subject, payload),
    )
}

fn certified<'w>(subject: &Subject<'w>) -> SimulationCertificateRequest<'w> {
    SimulationCertificateRequest {
        refinement: subject.refinement,
        concrete: subject.concrete.clone(),
        abstract_package: subject.package,
        abstract_initial: vec![subject.abstract_initial.clone()],
    }
}

fn reproduced(
    settlement: Result<RefinementTraceSettlement, ReplayRefusal>,
) -> (RefinementFailure, usize) {
    match settlement.expect("the trace replays") {
        RefinementTraceSettlement::Reproduced { failure, mapped } => (failure, mapped.len()),
        other => panic!("the failure reproduces: {other:?}"),
    }
}

/// TC-550 step 1: the lost-update counterexample replays,
/// recomputing the mapped values 0, 0, 0, 1, 1, to the recorded
/// `AbstractStepRejected{position: 4, ..}`.
#[trace("TC-550", "FR-145-AC-1")]
#[test]
fn the_lost_update_counterexample_replays_with_its_mapped_values() {
    let world = World::with(Variant::LOST_UPDATE, &lost_update_refinement());
    let subject = cas(&world);
    let payload = counterexample(check(&subject));
    let RefinementTraceSettlement::Reproduced { failure, mapped } =
        replay(&subject, &payload).expect("the trace replays")
    else {
        panic!("the failure reproduces");
    };
    assert_eq!(Some(failure), payload.refinement);
    let systems = open_systems(
        subject.refinement,
        &subject.concrete,
        subject.package,
        vec![subject.abstract_initial.clone()],
    )
    .expect("the subjects open");
    let keys: Vec<Vec<u8>> = mapped
        .iter()
        .map(|state| {
            systems
                .abstract_system
                .state_key_of(state)
                .expect("a key")
                .0
        })
        .collect();
    let expected: Vec<Vec<u8>> = [0, 0, 0, 1, 1]
        .iter()
        .map(|value| {
            systems
                .abstract_system
                .state_key_of(&spec_state(*value))
                .expect("a key")
                .0
        })
        .collect();
    assert_eq!(keys, expected);
}

/// TC-550 step 1): the `InitialNotAbstract` and
/// `StutterChanged` counterexamples each reproduce.
#[trace("TC-550", "FR-145-AC-1")]
#[test]
fn the_initial_and_stutter_counterexamples_replay() {
    let initial = World::with(
        Variant {
            concrete_value: 1,
            ..Variant::CAS
        },
        &cas_refines_counter(),
    );
    let subject = cas(&initial);
    let payload = counterexample(check(&subject));
    assert_eq!(
        reproduced(replay(&subject, &payload)).0,
        RefinementFailure::InitialNotAbstract { initial: 0 }
    );

    let stutter = World::new(&mutated(
        &cas_refines_counter(),
        "step Impl::Counter::commitA -> Spec::Counter::inc(self);",
        "step Impl::Counter::commitA -> stutter;",
    ));
    let subject = cas(&stutter);
    let payload = counterexample(check(&subject));
    assert_eq!(
        reproduced(replay(&subject, &payload)).0,
        RefinementFailure::StutterChanged { position: 2 }
    );
}

/// TC-550 step 3): the lost-update counterexample with its
/// recorded failure changed to position 3 settles `inconclusive`,
/// `ReplayParity`.
#[trace("TC-550")]
#[test]
fn a_failure_recorded_at_another_position_disagrees() {
    let world = World::with(Variant::LOST_UPDATE, &lost_update_refinement());
    let subject = cas(&world);
    let mut payload = counterexample(check(&subject));
    let Some(RefinementFailure::AbstractStepRejected {
        transition, cause, ..
    }) = payload.refinement.clone()
    else {
        panic!("an abstract step is rejected");
    };
    payload.refinement = Some(RefinementFailure::AbstractStepRejected {
        position: 3,
        transition,
        cause,
    });
    assert!(matches!(
        replay(&subject, &payload).expect("the trace replays"),
        RefinementTraceSettlement::Inconclusive(_)
    ));
}

/// TC-550 step 4): a counterexample whose initial index is out
/// of range, whose step is not enabled or whose post-state digest differs
/// refuses with no result, and replaying one counterexample twice gives
/// equal results.
#[trace("TC-550")]
#[test]
fn a_counterexample_that_does_not_re_execute_refuses() {
    let world = World::with(Variant::LOST_UPDATE, &lost_update_refinement());
    let subject = cas(&world);
    let payload = counterexample(check(&subject));
    assert_eq!(
        format!("{:?}", replay(&subject, &payload)),
        format!("{:?}", replay(&subject, &payload))
    );

    let mut moved = payload.clone();
    moved.initial_state = 1;
    assert!(matches!(
        replay(&subject, &moved),
        Err(ReplayRefusal::ModelTrace(refusal)) if matches!(*refusal, ModelTraceRefusal::InitialState { .. })
    ));
    let mut disabled = payload.clone();
    disabled.prefix[1].transition = disabled.prefix[0].transition.clone();
    assert!(matches!(
        replay(&subject, &disabled),
        Err(ReplayRefusal::ModelTrace(refusal)) if matches!(*refusal, ModelTraceRefusal::NotEnabled { .. })
    ));
    let mut altered = payload;
    altered.prefix[0].post_state = altered.prefix[1].post_state;
    assert!(matches!(
        replay(&subject, &altered),
        Err(ReplayRefusal::ModelTrace(refusal)) if matches!(*refusal, ModelTraceRefusal::PostState { .. })
    ));
}

/// FR-145 (TC-550): a payload that names no refinement failure is bad input,
/// refused `invalid_runtime_input`/`invalid-value`, not a fault.
#[trace("TC-550")]
#[test]
fn a_payload_with_no_refinement_failure_is_refused() {
    let world = World::with(Variant::LOST_UPDATE, &lost_update_refinement());
    let subject = cas(&world);
    let mut payload = counterexample(check(&subject));
    payload.refinement = None;
    assert!(matches!(
        replay(&subject, &payload),
        Err(ReplayRefusal::ModelTrace(refusal)) if matches!(*refusal, ModelTraceRefusal::NoRefinementFailure)
    ));
}

/// TC-549 step 3 and FR-144-AC-6 (TC-555 step 3): a violated
/// half settles `refuted` through its replay, for a failed step and for an
/// undefined mapping row, carrying the counterexample.
#[trace("TC-549")]
#[trace("TC-555", "FR-144-AC-6")]
#[test]
fn a_violated_half_settles_refuted_once_it_replays() {
    let world = World::with(Variant::LOST_UPDATE, &lost_update_refinement());
    let subject = cas(&world);
    let record = settle(&subject, &check(&subject));
    assert_eq!(record.value, TerminalValue::Refuted);
    assert_eq!(record.basis, SettlementBasis::DecisiveCounterexample);
    assert!(record.counterexample.is_some());

    let only = RingWorld::variant(RingVariant {
        only_ring: true,
        ..RingVariant::default()
    });
    let subject = ring(&only);
    let outcome = check(&subject);
    let record = settle(&subject, &outcome);
    assert_eq!(record.value, TerminalValue::Refuted);
    let Some(counterexample) = record.counterexample else {
        panic!("a refutation carries its counterexample");
    };
    assert!(matches!(
        counterexample.kind,
        CounterexampleKind::UndefinedEvaluation(_)
    ));
}

/// FR-144-AC-6 (TC-555 step 3): an undefined step-row argument settles
/// `refuted`, `decisive-counterexample`, once FR-145's replay reproduces it.
#[trace("TC-555", "FR-144-AC-6")]
#[test]
fn an_undefined_step_row_argument_settles_refuted_once_it_replays() {
    let items = "items = seq(i in 0 .. self.size - 1: only(s in R::slots where s.ring = self and s.index + 0 = (self.head + i) mod 2: s.value));";
    let refinement = format!(
        "function g using v(i: Int[0, 1]): Int[0, 1] pure {{ if 1 mod i = 0 then 0 else 1 }}\n{}",
        ring_is_queue(items).replace("enq(self.ring, x)", "enq(self.ring, g(self.index))")
    );
    let world = RingWorld::with(RingVariant::default(), &refinement);
    let subject = ring(&world);
    let outcome = check(&subject);
    let record = settle(&subject, &outcome);
    assert_eq!(record.value, TerminalValue::Refuted);
    assert_eq!(record.basis, SettlementBasis::DecisiveCounterexample);
}

/// FR-144-AC-4 (TC-549 step 4): a counterexample whose recorded failure was
/// changed settles `inconclusive`, `ReplayParity`, never `refuted`; one with
/// an `initial` index its subject does not have settles `inconclusive`,
/// `ReplayRefused`.
#[trace("TC-549", "FR-144-AC-4")]
#[test]
fn a_counterexample_that_replay_does_not_reproduce_never_settles_refuted() {
    let world = World::with(Variant::LOST_UPDATE, &lost_update_refinement());
    let subject = cas(&world);
    let outcome = check(&subject);
    let SafetyOutcome::Violated {
        counterexample: payload,
        stats,
    } = outcome
    else {
        panic!("the refinement is violated");
    };
    let mut edited = *payload.clone();
    edited.refinement = Some(RefinementFailure::NoAbstractMatch { position: 4 });
    let record = settle(
        &subject,
        &SafetyOutcome::Violated {
            counterexample: Box::new(edited),
            stats,
        },
    );
    assert!(matches!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(_))
    ));
    assert_eq!(record.label, ResultLabel::Inconclusive);

    let mut moved = *payload;
    moved.initial_state = 1;
    let record = settle(
        &subject,
        &SafetyOutcome::Violated {
            counterexample: Box::new(moved),
            stats,
        },
    );
    assert!(matches!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(_))
    ));
}

/// FR-144-AC-6 (TC-555 step 3): an incomplete mapping row settles
/// `inconclusive`, `unsettled`, `Inconclusive(MappingUndetermined)`, never
/// `refuted`; a fault settles `failed`.
#[trace("TC-555", "FR-144-AC-6")]
#[trace("TC-555", "FR-141-AC-8")]
#[test]
fn an_incomplete_row_settles_inconclusive_and_a_fault_settles_failed() {
    let world = RingWorld::new();
    let starved = ring_with(&world, ZERO);
    let outcome = check(&starved);
    let record = settle(&starved, &outcome);
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::MappingUndetermined)
    );
    assert_eq!(record.basis, SettlementBasis::Unsettled);
    assert_eq!(record.label, ResultLabel::Inconclusive);

    let faulted = SafetyOutcome::Fault {
        fault: InternalFault::new("refinement-step", "mapping-evaluation"),
        stats: RunStats::default(),
    };
    let record = settle(&starved, &faulted);
    assert_eq!(record.value, TerminalValue::Failed);
    assert_eq!(record.label, ResultLabel::Failed);
}

fn settle_item(subject: &Subject<'_>, outcome: &RefinementOutcome) -> RefinementRecord {
    settle_refinement(
        outcome,
        &ModelCheckLimits::default(),
        &certified(subject),
        &Cancel::new(),
        |payload| replay(subject, payload),
    )
}

/// The lasso the liveness half of `outcome` refutes with.
fn lasso(outcome: RefinementOutcome) -> TemporalCounterexample {
    let Some(LivenessHalf::Checked(ModelCheckOutcome::Violated { counterexample, .. })) =
        outcome.liveness
    else {
        panic!("the liveness half is violated: {:?}", outcome.liveness);
    };
    *counterexample
}

/// FR-145-AC-2 (TC-550 step 2): the divergence lasso reproduces
/// `Divergence{inc}`, the terminal-stutter lasso of `Halting` reproduces
/// `Divergence`, and the lasso of `Pair` reproduces `AbstractUnfair{flipB}`.
#[trace("TC-550", "FR-145-AC-2")]
#[test]
fn the_divergence_terminal_stutter_and_pair_lassos_replay() {
    let world = World::new(&divergence_refinement());
    let subject = cas(&world);
    let payload = lasso(outcome(&subject));
    let (failure, positions) = reproduced(replay(&subject, &payload));
    assert!(
        matches!(&failure, RefinementFailure::Divergence { constraint }
        if constraint.operation().name() == "inc")
    );
    assert_eq!(Some(failure), payload.refinement);
    assert_eq!(positions, payload.prefix.len() + 1);

    let unit = Unit::halting(1, "");
    let world = unit.world(&halting_refinement(true));
    let subject = aux(&world);
    let payload = lasso(outcome(&subject));
    assert_eq!(
        payload.lasso,
        Some(Lasso {
            entry: 1,
            terminal_stutter: true
        })
    );
    let (failure, positions) = reproduced(replay(&subject, &payload));
    assert!(matches!(failure, RefinementFailure::Divergence { .. }));
    assert_eq!(positions, 2);

    let unit = Unit::pair();
    let world = unit.world(&pair_refinement(&["flipA"]));
    let subject = aux(&world);
    let payload = lasso(outcome(&subject));
    let (failure, positions) = reproduced(replay(&subject, &payload));
    assert!(
        matches!(&failure, RefinementFailure::AbstractUnfair { constraint }
        if constraint.operation().name() == "flipB")
    );
    assert_eq!(Some(failure), payload.refinement);
    assert_eq!(positions, 3);
}

/// FR-145-AC-3 (TC-550 step 3): a lasso whose recorded failure is the other
/// liveness kind settles `inconclusive` (`ReplayParity` once settled), and
/// the divergence lasso replayed against `CasRefinesCounter`'s `F_C` is
/// refused as unfair, since `beginA` is enabled at its state and never taken.
#[trace("TC-550", "FR-145-AC-3")]
#[test]
fn a_lasso_of_another_kind_disagrees_and_an_unfair_lasso_is_refused() {
    let world = World::new(&divergence_refinement());
    let subject = cas(&world);
    let payload = lasso(outcome(&subject));
    let Some(RefinementFailure::Divergence { constraint }) = payload.refinement.clone() else {
        panic!("a divergence");
    };
    let mut unfair = payload.clone();
    unfair.refinement = Some(RefinementFailure::AbstractUnfair {
        constraint: constraint.clone(),
    });
    assert!(matches!(
        replay(&subject, &unfair),
        Ok(RefinementTraceSettlement::Inconclusive(_))
    ));
    let settled = settle_item(
        &subject,
        &RefinementOutcome {
            safety: check(&subject),
            liveness: Some(LivenessHalf::Checked(ModelCheckOutcome::Violated {
                counterexample: Box::new(unfair),
                stats: RunStats::default(),
            })),
        },
    );
    assert!(matches!(
        settled.record.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(_))
    ));
    assert_eq!(settled.liveness, Some(settled.record.value.clone()));

    let fair = World::new(&cas_refines_counter());
    let Err(ReplayRefusal::UnfairLasso { constraint }) = replay(&cas(&fair), &payload) else {
        panic!("the lasso is unfair under CasRefinesCounter's F_C");
    };
    assert_eq!(constraint.operation().name(), "beginA");

    let unit = Unit::pair();
    let world = unit.world(&pair_refinement(&["flipA"]));
    let subject = aux(&world);
    let mut divergent = lasso(outcome(&subject));
    let Some(RefinementFailure::AbstractUnfair { constraint }) = divergent.refinement.clone()
    else {
        panic!("an abstract unfairness");
    };
    divergent.refinement = Some(RefinementFailure::Divergence { constraint });
    assert!(matches!(
        replay(&subject, &divergent),
        Ok(RefinementTraceSettlement::Inconclusive(_))
    ));
}

/// The lasso over `RegisterHistory` with `ensure fair weak
/// Spec::Register::write` of FR-145-AC-4: the stem and the loop are both
/// `write(r, 1)`, so the loop returns to its entry concrete state with `last`
/// 0 at the entry and 1 at its end.
fn drifting_lasso(world: &AuxWorld<'_>) -> TemporalCounterexample {
    use qsl_eval::simulation::TransitionSystem;

    let system = world.concrete_system(AUX_UNLIMITED);
    let one = system
        .state_key_of(&world.concrete_state(&[("value", int(1))]))
        .expect("a key")
        .0;
    let mut state = system.initial_states()[0].clone();
    let mut prefix = Vec::new();
    for _ in 0..2 {
        let expansion = system.successors(&state).expect("the state expands");
        let (transition, next) = expansion
            .successors
            .into_iter()
            .find(|(_, next)| system.state_key(next).is_ok_and(|(key, _)| key == one))
            .expect("write(r, 1)");
        prefix.push(ModelStep {
            transition,
            post_state: system.state_key(&next).expect("a key").1,
        });
        state = next;
    }
    TemporalCounterexample {
        initial_state: 0,
        prefix,
        over_binding: None,
        fairness: Vec::new(),
        kind: CounterexampleKind::Formula,
        lasso: Some(Lasso {
            entry: 1,
            terminal_stutter: false,
        }),
        refinement: Some(RefinementFailure::AbstractUnfair {
            constraint: world.refinement().abstract_fairness()[0].clone(),
        }),
        blocked: Vec::new(),
    }
}

/// FR-145-AC-4 (TC-550 step 4): a lasso whose loop returns to its entry
/// concrete state with other history values refuses
/// `invalid_runtime_input`/`invalid-value` and settles no result, and
/// replaying one lasso twice gives equal results.
#[trace("TC-550", "FR-145-AC-4")]
#[test]
fn a_loop_that_does_not_close_in_the_product_is_refused() {
    let unit = Unit::register();
    let world = unit.world(&register_history_ensured());
    let subject = aux(&world);
    let payload = drifting_lasso(&world);
    let Err(ReplayRefusal::ModelTrace(refusal)) = replay(&subject, &payload) else {
        panic!("the loop does not close");
    };
    assert_eq!(*refusal, ModelTraceRefusal::LoopDrifts { entry: 1 });
    assert_eq!(refusal.code(), qsl_replay::Code::InvalidRuntimeInput);
    assert_eq!(refusal.cause(), "invalid-value");

    let world = World::new(&divergence_refinement());
    let subject = cas(&world);
    let payload = lasso(outcome(&subject));
    assert_eq!(
        format!("{:?}", replay(&subject, &payload)),
        format!("{:?}", replay(&subject, &payload))
    );
}

fn proved(certification: Certification) -> TerminalValue {
    TerminalValue::Proved {
        basis: ProofBasis::Exhaustive,
        certification,
    }
}

/// FR-144-AC-2 (TC-549 step 2) and FR-144-AC-7 (TC-556 step 5):
/// `CasRefinesCounter` settles `proved`, `closed-scope`, `Uncertified`, its
/// safety half `Certified` and its liveness half `Uncertified`; without its
/// `ensure` row it settles `Certified`; its divergence variant settles
/// `refuted`, `decisive-counterexample`, after its lasso replays, with its
/// liveness half `Refuted`.
#[trace("TC-549", "FR-144-AC-2")]
#[trace("TC-556", "FR-144-AC-7")]
#[test]
fn a_proved_liveness_half_is_uncertified_and_a_replayed_lasso_refutes() {
    let world = World::new(&cas_refines_counter());
    let subject = cas(&world);
    let settled = settle_item(&subject, &outcome(&subject));
    assert_eq!(settled.record.value, proved(Certification::Uncertified));
    assert_eq!(settled.record.label, ResultLabel::Proved);
    assert_eq!(settled.record.basis, SettlementBasis::ClosedScope);
    assert_eq!(settled.record.method, "explicit-state");
    assert_eq!(settled.safety, Some(proved(Certification::Certified)));
    assert_eq!(settled.liveness, Some(proved(Certification::Uncertified)));

    let safety_only = World::new(&mutated(
        &cas_refines_counter(),
        "ensure fair weak each Spec::Counter::inc;",
        "",
    ));
    let subject = cas(&safety_only);
    let settled = settle_item(&subject, &outcome(&subject));
    assert_eq!(settled.record.value, proved(Certification::Certified));
    assert_eq!(settled.safety, Some(proved(Certification::Certified)));
    assert_eq!(settled.liveness, None);

    let world = World::new(&divergence_refinement());
    let subject = cas(&world);
    let settled = settle_item(&subject, &outcome(&subject));
    assert_eq!(settled.record.value, TerminalValue::Refuted);
    assert_eq!(settled.record.label, ResultLabel::Refuted);
    assert_eq!(
        settled.record.basis,
        SettlementBasis::DecisiveCounterexample
    );
    assert!(settled.record.counterexample.is_some());
    assert_eq!(settled.safety, Some(proved(Certification::Certified)));
    assert_eq!(settled.liveness, Some(TerminalValue::Refuted));
}

/// FR-144-AC-3 (TC-549 step 3): the hidden-field refinement with its
/// `ensure` row settles `unsupported`, `unavailable`,
/// `Unsupported(unsupported-requested-capability)`, with the safety half
/// `Proved{Exhaustive, Certified}` on the record.
#[trace("TC-549", "FR-144-AC-3")]
#[test]
fn liveness_through_a_hidden_field_settles_unsupported_beside_a_proved_safety_half() {
    let unit = Unit::register();
    let world = unit.world(&hidden_register_history());
    let subject = aux(&world);
    let settled = settle_item(&subject, &outcome(&subject));
    let cause = TerminalValue::Unsupported(UnavailabilityCause::HiddenFieldLiveness);
    assert_eq!(settled.record.value, cause);
    assert_eq!(settled.record.label, ResultLabel::Unsupported);
    assert_eq!(settled.record.basis, SettlementBasis::Unavailable);
    assert_eq!(settled.safety, Some(proved(Certification::Certified)));
    assert_eq!(settled.liveness, Some(cause));
    let TerminalValue::Unsupported(cause) = &settled.record.value else {
        panic!("unsupported");
    };
    assert_eq!(cause.as_str(), "unsupported-requested-capability");
}

/// The steps of `operations`, each the first successor of the state before
/// it whose transition is of that operation's text.
fn walk(system: &ModelSystem<'_>, operations: &[String]) -> Vec<ModelStep> {
    let mut state = system.initial_states()[0].clone();
    let mut steps = Vec::new();
    for operation in operations {
        let (transition, next) = system
            .successors(&state)
            .expect("the state expands")
            .successors
            .into_iter()
            .find(|(transition, _)| transition.operation() == operation)
            .unwrap_or_else(|| panic!("{operation} is enabled"));
        steps.push(ModelStep {
            transition,
            post_state: system.state_key(&next).expect("a key").1,
        });
        state = next;
    }
    steps
}

/// A lasso over `steps` that re-enters at `entry`, recording `failure`.
fn handmade(
    steps: Vec<ModelStep>,
    entry: usize,
    terminal_stutter: bool,
    failure: RefinementFailure,
) -> TemporalCounterexample {
    TemporalCounterexample {
        initial_state: 0,
        prefix: steps,
        over_binding: None,
        fairness: Vec::new(),
        kind: CounterexampleKind::Formula,
        lasso: Some(Lasso {
            entry,
            terminal_stutter,
        }),
        refinement: Some(failure),
        blocked: Vec::new(),
    }
}

fn pair_operations(unit: &Unit, names: &[&str]) -> Vec<String> {
    names
        .iter()
        .map(|name| {
            operation_identity_text(&unit.concrete_key(&format!("Pair/{name}"))).expect("a text")
        })
        .collect()
}

/// Replay of the lasso `loop_of` over `unit`'s refinement `rows`, recorded
/// as `failure` for the `F_A` constraint at `constraint`.
fn replay_probe(
    unit: &Unit,
    rows: &[&str],
    loop_of: &[&str],
    constraint: usize,
    failure: fn(quire_semantic_value::fairness::FairnessConstraint) -> RefinementFailure,
) -> Result<RefinementTraceSettlement, ReplayRefusal> {
    let world = unit.world(&probe_refinement(rows));
    let subject = aux(&world);
    let system = world.concrete_system(AUX_UNLIMITED);
    let steps = walk(&system, &pair_operations(unit, loop_of));
    let constraint = world.refinement().abstract_fairness()[constraint].clone();
    replay(&subject, &handmade(steps, 0, false, failure(constraint)))
}

fn unfair(constraint: quire_semantic_value::fairness::FairnessConstraint) -> RefinementFailure {
    RefinementFailure::AbstractUnfair { constraint }
}

fn divergence(constraint: quire_semantic_value::fairness::FairnessConstraint) -> RefinementFailure {
    RefinementFailure::Divergence { constraint }
}

fn agrees(settlement: Result<RefinementTraceSettlement, ReplayRefusal>) -> bool {
    match settlement.expect("the lasso replays") {
        RefinementTraceSettlement::Reproduced { .. } => true,
        RefinementTraceSettlement::Inconclusive(_) => false,
        other @ RefinementTraceSettlement::Stopped(_) => panic!("{other:?}"),
    }
}

const FLIP_B: &str = "ensure fair weak whole Spec::Pair::flipB;";
const FLIP_A: &str = "ensure fair weak whole Spec::Pair::flipA;";

/// FR-145-AC-3 (TC-550 step 3): replay reproduces a lasso only when the
/// recorded constraint is enabled at the mapped state of every loop state
/// and taken by no loop step. A loop that leaves it disabled at one state,
/// a loop that takes it, and a lasso that records another constraint than the
/// one the loop violates are each `inconclusive`.
#[trace("TC-550", "FR-145-AC-3")]
#[test]
fn a_loop_reproduces_only_when_the_recorded_constraint_is_owed_and_never_taken() {
    // flipB is enabled abstractly at `a = 0` only, and the loop flips `a`.
    let disabled = Unit::probe_pair(
        probe_contracts(["", NEVER, NEVER, NEVER], ["", "self.a = 0", ""]),
        &[(0, 0)],
    );
    assert!(!agrees(replay_probe(
        &disabled,
        &[FLIP_B],
        &["flipA", "flipA"],
        0,
        unfair
    )));

    // The loop `flipB, flipB` takes flipB at every step.
    let taken = Unit::probe_pair(
        probe_contracts(["", "", NEVER, NEVER], ["", "", ""]),
        &[(0, 0)],
    );
    assert!(!agrees(replay_probe(
        &taken,
        &[FLIP_B],
        &["flipB", "flipB"],
        0,
        unfair
    )));

    // The loop `flipA, flipA` takes flipA and not flipB.
    let flipping = Unit::probe_pair(
        probe_contracts(["", "", NEVER, NEVER], ["", "", ""]),
        &[(0, 0)],
    );
    let rows = [FLIP_A, FLIP_B];
    assert!(!agrees(replay_probe(
        &flipping,
        &rows,
        &["flipA", "flipA"],
        0,
        unfair
    )));
    assert!(agrees(replay_probe(
        &flipping,
        &rows,
        &["flipA", "flipA"],
        1,
        unfair
    )));
}

/// FR-145-AC-3 (TC-550 step 3): a loop is `Divergence` when every step is a
/// `stutter` and `AbstractUnfair` as soon as one takes an abstract step.
#[trace("TC-550", "FR-145-AC-3")]
#[test]
fn a_loop_is_a_divergence_only_when_every_step_stutters() {
    let unit = Unit::probe_pair(
        probe_contracts(["", NEVER, NEVER, ""], ["", "", ""]),
        &[(0, 0)],
    );
    let rows = [FLIP_B];
    let mixed = ["peek", "flipA", "flipA"];
    assert!(agrees(replay_probe(&unit, &rows, &mixed, 0, unfair)));
    assert!(!agrees(replay_probe(&unit, &rows, &mixed, 0, divergence)));
    let stuttering = ["peek"];
    assert!(agrees(replay_probe(
        &unit,
        &rows,
        &stuttering,
        0,
        divergence
    )));
    assert!(!agrees(replay_probe(&unit, &rows, &stuttering, 0, unfair)));
}

/// FR-145-AC-3 (TC-550 step 3): the terminal stutter is a step of the loop
/// that takes no class. At `value` 2 of `Halting` the abstract `inc` is
/// disabled, so a terminal-stutter lasso recorded there is `inconclusive`
/// as `Divergence` and as `AbstractUnfair`; at `value` 1 it is enabled.
#[trace("TC-550", "FR-145-AC-3")]
#[test]
fn a_terminal_stutter_loop_is_read_at_its_mapped_state() {
    let replay_halt = |limit: u32, incs: usize, failure: fn(_) -> RefinementFailure| {
        let unit = Unit::halting(limit, "");
        let world = unit.world(&halting_refinement(true));
        let subject = aux(&world);
        let system = world.concrete_system(AUX_UNLIMITED);
        let inc = operation_identity_text(&unit.concrete_key("Counter/inc")).expect("a text");
        let steps = walk(&system, &vec![inc; incs]);
        let constraint = world.refinement().abstract_fairness()[0].clone();
        replay(&subject, &handmade(steps, incs, true, failure(constraint)))
    };
    assert!(!agrees(replay_halt(2, 2, divergence)));
    assert!(!agrees(replay_halt(2, 2, unfair)));
    assert!(agrees(replay_halt(1, 1, divergence)));
    assert!(!agrees(replay_halt(1, 1, unfair)));
}

/// FR-145 "Lasso" (TC-550): the loop must close in the concrete model. The
/// loop `beginA` leaves the compare-and-set counter in a state with the same
/// mapped value and history but another concrete state, which refuses
/// `LoopOpen` whatever the product says.
#[trace("TC-550", "FR-145-AC-4")]
#[test]
fn a_loop_open_in_the_concrete_model_is_refused() {
    let world = World::new(&cas_refines_counter());
    let subject = cas(&world);
    let system = world.concrete_system(UNLIMITED);
    let begin = operation_identity_text(&key(CAS, "Counter/beginA")).expect("a text");
    let steps = walk(&system, &[begin]);
    let constraint = world.refinement().abstract_fairness()[0].clone();
    let payload = handmade(steps, 0, false, divergence(constraint));
    let Err(ReplayRefusal::ModelTrace(refusal)) = replay(&subject, &payload) else {
        panic!("the loop does not close");
    };
    assert_eq!(*refusal, ModelTraceRefusal::LoopOpen { entry: 0, steps: 1 });
}

/// FR-145 "Lasso" (TC-550): replay reads an `F_A` constraint as weak, so a
/// lasso that records a strong one is never reproduced.
#[trace("TC-550")]
#[test]
fn a_lasso_of_a_strong_constraint_is_not_reproduced() {
    let unit = Unit::probe_pair(
        probe_contracts(["", NEVER, NEVER, ""], ["", "", ""]),
        &[(0, 0)],
    );
    let strong = "ensure fair strong whole Spec::Pair::flipB;";
    assert!(!agrees(replay_probe(
        &unit,
        &[strong],
        &["peek"],
        0,
        divergence
    )));
}

/// FR-144 Settlement (TC-549): a liveness half that a limit stops settles
/// `Incomplete` naming the limit, never `failed`, and one that cannot decide
/// settles `inconclusive` with `UndecidedSuccessor`.
#[trace("TC-549")]
#[test]
fn a_stopped_or_undecided_liveness_half_settles_by_its_cause() {
    let world = World::new(&cas_refines_counter());
    let subject = cas(&world);
    let checked = outcome(&subject);
    let with = |half| RefinementOutcome {
        safety: checked.safety.clone(),
        liveness: Some(LivenessHalf::Checked(half)),
    };
    let reached = ReachedLimit {
        limit: ModelCheckLimit::MaxTransitions,
        value: 7,
    };
    let settled = settle_item(
        &subject,
        &with(ModelCheckOutcome::Stopped {
            cause: IncompleteCause::ResourceExhausted,
            limit: Some(reached),
            stats: RunStats::default(),
        }),
    );
    assert_eq!(
        settled.record.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxTransitions,
            value: 7
        })
    );
    assert_eq!(settled.record.label, ResultLabel::Failed);
    assert_eq!(settled.record.execution, Execution::ResourceIncomplete);
    assert_eq!(settled.record.limit_reached, Some(reached));
    assert_eq!(settled.liveness, Some(settled.record.value.clone()));

    let settled = settle_item(
        &subject,
        &with(ModelCheckOutcome::Undecided {
            cause: UndecidedCause::UndecidedSuccessor,
            stats: RunStats::default(),
        }),
    );
    assert_eq!(
        settled.record.value,
        TerminalValue::Inconclusive(InconclusiveCause::UndecidedSuccessor)
    );
    assert_eq!(settled.safety, Some(proved(Certification::Certified)));
}

/// FR-144 Settlement (TC-549): the item's value is the safety half's unless
/// that is proved, even when the refinement's liveness half is
/// `Unsupported`; the half is still recorded beside it.
#[trace("TC-549")]
#[test]
fn an_unsupported_liveness_half_does_not_replace_a_safety_half_that_is_not_proved() {
    let world = World::new(&cas_refines_counter());
    let subject = cas(&world);
    let settled = settle_item(
        &subject,
        &RefinementOutcome {
            safety: SafetyOutcome::Undecided {
                cause: SafetyUndecided::NoInitialState,
                stats: RunStats::default(),
            },
            liveness: Some(LivenessHalf::Unsupported(
                UnavailabilityCause::HiddenFieldLiveness,
            )),
        },
    );
    assert_eq!(
        settled.record.value,
        TerminalValue::Inconclusive(InconclusiveCause::NoInitialState)
    );
    assert_eq!(settled.safety, Some(settled.record.value.clone()));
    assert_eq!(
        settled.liveness,
        Some(TerminalValue::Unsupported(
            UnavailabilityCause::HiddenFieldLiveness
        ))
    );
}

/// FR-144 Settlement (TC-549): a refusal `check_refinement` returned for a
/// second-phase fault settles the item `failed` with that fault and no half
/// value. The fault is the one `abstract_classes` returns for a system that
/// does not declare `F_A`'s operation; that `check_refinement` returns such a
/// fault as a refusal is covered by the unit test in `refine.rs`.
#[trace("TC-549")]
#[test]
fn a_second_phase_fault_settles_the_item_failed_with_no_half_value() {
    let world = World::new(&cas_refines_counter());
    let swapped = world.concrete_system(UNLIMITED);
    let Err(fault) = abstract_classes(&swapped, world.refinement().abstract_fairness(), &[]) else {
        panic!("the swapped system declares no operation of F_A");
    };
    assert_eq!(
        fault,
        InternalFault::new(
            "refinement-liveness",
            "an-abstract-fairness-names-a-declared-operation"
        )
    );

    let settled = settle_refinement_refusal(&fault, &ModelCheckLimits::default());
    assert_eq!(settled.record.value, TerminalValue::Failed);
    assert_eq!(settled.record.label, ResultLabel::Failed);
    assert_eq!(settled.record.execution, Execution::Failed);
    assert_eq!(settled.safety, None);
    assert_eq!(settled.liveness, None);
    assert_eq!(settled.fault, Some(fault));
}
