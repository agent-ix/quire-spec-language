// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-145 (TC-550) and FR-144 (TC-549, TC-555): a refinement counterexample
//! replays by re-executing its prefix and rerunning the step check, and the
//! safety half settles as a terminal record from the outcome and the replay.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_refinement, RefinementCheckRequest};
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::ModelSimulationRequest;
use qsl_foundation::InternalFault;
use qsl_package::CheckedPackage;
use qsl_replay::{
    open_systems, replay_refinement_trace, settle_refinement_safety, Certification,
    CounterexampleKind, InconclusiveCause, ModelCheckLimits, ModelCheckRecord, ModelTraceRefusal,
    ProofBasis, RefinementFailure, RefinementOutcome, RefinementTraceSettlement, ReplayRefusal,
    ResultLabel, RunStats, SafetyOutcome, SettlementBasis, TemporalCounterexample, TerminalValue,
};
use qsl_semantics::check::CheckedRefinement;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::Cancel;

use crate::refinement_fixture::{
    cas_refines_counter, lost_update_refinement, mutated, spec_state, Variant, World, UNLIMITED,
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

fn check(subject: &Subject<'_>) -> SafetyOutcome {
    let RefinementOutcome { safety } = check_refinement(
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
    .expect("the refinement is checked");
    safety
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
    settle_refinement_safety(outcome, &ModelCheckLimits::default(), |payload| {
        replay(subject, payload)
    })
}

fn reproduced(
    settlement: Result<RefinementTraceSettlement, ReplayRefusal>,
) -> (RefinementFailure, usize) {
    match settlement.expect("the trace replays") {
        RefinementTraceSettlement::Reproduced { failure, mapped } => (failure, mapped.len()),
        other => panic!("the failure reproduces: {other:?}"),
    }
}

/// FR-145-AC-1 (TC-550 step 1): the lost-update counterexample replays,
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

/// FR-145-AC-1 (TC-550 step 1): the `InitialNotAbstract` and
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

/// FR-145-AC-3 (TC-550 step 3): the lost-update counterexample with its
/// recorded failure changed to position 3 settles `inconclusive`,
/// `ReplayParity`.
#[trace("TC-550", "FR-145-AC-3")]
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

/// FR-145-AC-4 (TC-550 step 4): a counterexample whose initial index is out
/// of range, whose step is not enabled or whose post-state digest differs
/// refuses with no result, and replaying one counterexample twice gives
/// equal results.
#[trace("TC-550", "FR-145-AC-4")]
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

/// FR-144-AC-2 (TC-549 step 2), safety half: `CasRefinesCounter`'s safety
/// half holds and settles `proved`, `closed-scope`, exhaustive. It is labelled
/// `Uncertified` until the simulation certificate (FR-149) can certify it.
#[trace("TC-549", "FR-144-AC-2")]
#[test]
fn a_safety_half_that_holds_settles_proved_uncertified() {
    let world = World::new(&cas_refines_counter());
    let subject = cas(&world);
    let outcome = check(&subject);
    let record = settle(&subject, &outcome);
    assert_eq!(
        record.value,
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Uncertified,
        }
    );
    assert_eq!(record.label, ResultLabel::Proved);
    assert_eq!(record.basis, SettlementBasis::ClosedScope);
    assert_eq!(record.method, "explicit-state");
}

/// FR-144-AC-3 (TC-549 step 3) and FR-144-AC-6 (TC-555 step 3): a violated
/// half settles `refuted` through its replay, for a failed step and for an
/// undefined mapping row, carrying the counterexample.
#[trace("TC-549", "FR-144-AC-3")]
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
