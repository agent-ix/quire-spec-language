// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-142, FR-144 and FR-145 (TC-547, TC-549, TC-550, TC-554): the safety
//! half of a refinement with a history field or a hidden abstract field, its
//! counterexamples' replay and its settlement, over `RegisterHistory` and
//! `Coin`.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_refinement, RefinementCheckRequest};
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::operation_identity_text;
use qsl_foundation::diagnostic::{UndefinedReason, UndefinedRecord};
use qsl_replay::{
    open_systems, replay_refinement_trace, settle_refinement_safety, CounterexampleKind,
    InconclusiveCause, MappingRowRef, MappingSite, ModelCheckLimits, ModelCheckRecord, ObjectKey,
    RefinementFailure, RefinementOutcome, RefinementTraceSettlement, ResultLabel, SafetyOutcome,
    SafetyUndecided, SettlementBasis, StepRejection, TemporalCounterexample, TerminalValue,
    UndefinedLocus, Undetermined,
};
use quire_exact::{Cancel, Undefined};

use crate::refinement_aux::{
    coin_refines, int, register_history, Unit, World, INV, LAST, UNLIMITED, WRITES,
};

fn run(world: &World<'_>) -> SafetyOutcome {
    let RefinementOutcome { safety } = check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()],
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the refinement is checked");
    safety
}

fn violated(outcome: SafetyOutcome) -> TemporalCounterexample {
    let SafetyOutcome::Violated { counterexample, .. } = outcome else {
        panic!("the refinement is violated: {outcome:?}");
    };
    *counterexample
}

fn replay(
    world: &World<'_>,
    payload: &TemporalCounterexample,
) -> Result<RefinementTraceSettlement, qsl_replay::ReplayRefusal> {
    let systems = open_systems(
        world.refinement(),
        &world.concrete_request(UNLIMITED),
        world.package(),
        vec![world.abstract_initial()],
    )
    .expect("the subjects open");
    replay_refinement_trace(world.refinement(), &systems, payload)
}

fn settle(world: &World<'_>, outcome: &SafetyOutcome) -> ModelCheckRecord {
    settle_refinement_safety(outcome, &ModelCheckLimits::default(), |payload| {
        replay(world, payload)
    })
}

fn reproduced(world: &World<'_>, payload: &TemporalCounterexample) -> RefinementFailure {
    match replay(world, payload).expect("the trace replays") {
        RefinementTraceSettlement::Reproduced { failure, .. } => failure,
        other => panic!("the failure reproduces: {other:?}"),
    }
}

fn history_row(unit: &Unit, field: &str) -> MappingRowRef {
    MappingRowRef::History {
        object_type: unit.concrete_key("Register"),
        field: field.to_owned(),
        operation: unit.concrete_key("Register/write"),
    }
}

fn register_key(unit: &Unit) -> ObjectKey {
    ObjectKey {
        population: unit.concrete_key("regs"),
        key: "r".to_owned(),
    }
}

/// The text of the concrete operation `name` of `Coin`.
fn coin_operation(unit: &Unit, name: &str) -> String {
    operation_identity_text(&unit.concrete_key(&format!("Coin/{name}"))).expect("a text")
}

/// The refinement of `Register` whose `last` row reads the post-state value,
/// which is not the value the abstract `prev` needs.
fn broken_last() -> String {
    register_history(
        "history Impl::Register.last : Int[0, 1] = 0 { on Impl::Register::write : self.value; }",
    )
}

/// FR-142-AC-4 (TC-547 step 4): `RegisterHistory` holds over 4 product
/// states, the (value, last) pairs, which a product keyed by the concrete
/// state alone would count as 2.
#[trace("TC-547", "FR-142-AC-4")]
#[test]
fn a_register_with_a_history_field_holds_over_four_product_states() {
    let unit = Unit::register();
    let SafetyOutcome::Holds { stats } = run(&unit.world(&register_history(LAST))) else {
        panic!("the refinement holds");
    };
    assert_eq!(stats.product_states, 4);
}

/// FR-142-AC-4 (TC-547 step 4): the update `on write: self.value` gives the
/// post-state value where the abstract `prev` needs the pre-state's, so the
/// first `write(r, 1)` is rejected at its postcondition at position 1.
#[trace("TC-547", "FR-142-AC-4")]
#[test]
fn a_wrong_history_update_is_refuted_at_the_first_write() {
    let unit = Unit::register();
    let world = unit.world(&broken_last());
    let counterexample = violated(run(&world));
    assert_eq!(counterexample.prefix.len(), 1);
    let Some(RefinementFailure::AbstractStepRejected {
        position,
        transition,
        cause,
    }) = counterexample.refinement
    else {
        panic!("an abstract step is rejected");
    };
    assert_eq!((position, cause), (1, StepRejection::Postcondition));
    let abstracted = world.abstract_system(UNLIMITED);
    let written = abstracted
        .expand_state(&world.abstract_state(&[("value", int(0)), ("prev", int(0))]))
        .expect("the state expands")
        .successors
        .into_iter()
        .find(|(_, reached)| {
            abstracted.state_key_of(reached.state()).expect("a key").0
                == abstracted
                    .state_key_of(&world.abstract_state(&[("value", int(1)), ("prev", int(0))]))
                    .expect("a key")
                    .0
        })
        .expect("write(r, 1) reaches value 1")
        .0;
    assert_eq!(transition, written);
}

/// FR-142-AC-4 (TC-547 step 4) and FR-142-AC-8 (TC-554 step 3): a history
/// field `writes` that no row reads, whose second update leaves its type,
/// leaves the safety half `Undecided(MappingUndetermined)` naming `writes`.
#[trace("TC-547", "FR-142-AC-4")]
#[trace("TC-554", "FR-142-AC-8")]
#[test]
fn an_update_that_leaves_its_type_leaves_the_half_undecided() {
    let unit = Unit::register();
    let world = unit.world(&register_history(&format!("{LAST}\n{WRITES}")));
    let SafetyOutcome::Undecided {
        cause: SafetyUndecided::Undetermined(Undetermined::Mapping(undetermined)),
        ..
    } = run(&world)
    else {
        panic!("the half is undecided at the mapping");
    };
    assert_eq!(undetermined.row, history_row(&unit, "writes"));
    assert_eq!(undetermined.object, Some(register_key(&unit)));
}

/// FR-142-AC-8 (TC-554 step 3): the `inv` update `1 mod x + 1` is undefined
/// at `write(r, 0)`, so the half is refuted there: the prefix is
/// `write(r, 0)`, the failure is `Undefined{position: 1, row: inv}` and the
/// counterexample's kind names the update, the object and the cause.
#[trace("TC-554", "FR-142-AC-8")]
#[test]
fn an_undefined_history_update_refutes_at_its_step() {
    let unit = Unit::register();
    let world = unit.world(&register_history(&format!("{LAST}\n{INV}")));
    let counterexample = violated(run(&world));
    assert_eq!(counterexample.prefix.len(), 1);
    assert_eq!(
        counterexample.refinement,
        Some(RefinementFailure::Undefined {
            position: 1,
            row: history_row(&unit, "inv"),
        })
    );
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("an undefined evaluation: {:?}", counterexample.kind);
    };
    assert_eq!(evaluation.site.position, 1);
    assert_eq!(
        evaluation.site.locus,
        UndefinedLocus::Mapping(Box::new(MappingSite {
            row: history_row(&unit, "inv"),
            object: Some(register_key(&unit)),
        }))
    );
    assert_eq!(
        evaluation.cause,
        qsl_eval::temporal::UndefinedCause::Kernel(Undefined::DivisionByZero)
    );
}

/// FR-142-AC-5 (TC-547 step 5): `Coin` with `side` hidden holds, since a
/// choice of `side` at `toss` that `reveal` later shows stays among the
/// candidates; with the row `side = self.face` it is refuted at the `reveal`
/// to `face = 1`, where `show` would have to change `side`.
#[trace("TC-547", "FR-142-AC-5")]
#[test]
fn a_hidden_side_holds_and_a_mapped_side_is_refuted() {
    let unit = Unit::coin(1);
    let SafetyOutcome::Holds { .. } = run(&unit.world(&coin_refines(""))) else {
        panic!("the refinement holds with side hidden");
    };
    let world = unit.world(&coin_refines("side = self.face;"));
    let counterexample = violated(run(&world));
    let operations: Vec<&str> = counterexample
        .prefix
        .iter()
        .map(|step| step.transition.operation())
        .collect();
    assert_eq!(
        operations,
        [
            coin_operation(&unit, "toss"),
            coin_operation(&unit, "reveal")
        ]
    );
    let Some(RefinementFailure::AbstractStepRejected {
        position,
        transition,
        cause: StepRejection::Frame { .. },
    }) = counterexample.refinement
    else {
        panic!(
            "the frame rejects the step: {:?}",
            counterexample.refinement
        );
    };
    assert_eq!(position, 2);
    assert_eq!(
        transition.operation(),
        operation_identity_text(&unit.abstract_key("Coin/show")).expect("a text")
    );
}

/// FR-145-AC-1 (TC-550 step 1): the counterexamples of a wrong history
/// update and of the `Coin` frame failure each reproduce.
#[trace("TC-550", "FR-145-AC-1")]
#[test]
fn a_history_and_a_hidden_field_counterexample_replay() {
    let unit = Unit::register();
    let world = unit.world(&broken_last());
    let payload = violated(run(&world));
    assert_eq!(reproduced(&world, &payload), payload.refinement.unwrap());

    let unit = Unit::coin(1);
    let world = unit.world(&coin_refines("side = self.face;"));
    let payload = violated(run(&world));
    assert_eq!(reproduced(&world, &payload), payload.refinement.unwrap());
}

/// FR-145-AC-5 (TC-554 step 4): the `inv` counterexample replays to
/// `reproduced-with-evaluated-witness`; the same payload with its cause
/// changed to `precondition-false` settles `inconclusive`, `ReplayParity`.
#[trace("TC-554", "FR-145-AC-5")]
#[test]
fn an_undefined_update_replays_and_a_changed_cause_disagrees() {
    let unit = Unit::register();
    let world = unit.world(&register_history(&format!("{LAST}\n{INV}")));
    let payload = violated(run(&world));
    assert_eq!(
        reproduced(&world, &payload),
        payload.refinement.clone().unwrap()
    );

    let mut changed = payload;
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &mut changed.kind else {
        panic!("an undefined evaluation");
    };
    evaluation.cause = qsl_eval::temporal::UndefinedCause::Family(UndefinedRecord {
        reason: UndefinedReason::PreconditionFalse,
        fields: std::collections::BTreeMap::new(),
    });
    assert!(matches!(
        replay(&world, &changed).expect("the trace replays"),
        RefinementTraceSettlement::Inconclusive(_)
    ));
}

/// FR-144-AC-3 (TC-549 step 3): the `writes` variant settles
/// `inconclusive`, `unsettled`, `Inconclusive(MappingUndetermined)`; the
/// `inv` variant settles `refuted`, `decisive-counterexample`, after FR-145's
/// replay reproduces it.
#[trace("TC-549")]
#[test]
fn history_failures_settle_as_their_kind_says() {
    let unit = Unit::register();
    let world = unit.world(&register_history(&format!("{LAST}\n{WRITES}")));
    let record = settle(&world, &run(&world));
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::MappingUndetermined)
    );
    assert_eq!(record.basis, SettlementBasis::Unsettled);
    assert_eq!(record.label, ResultLabel::Inconclusive);

    let world = unit.world(&register_history(&format!("{LAST}\n{INV}")));
    let record = settle(&world, &run(&world));
    assert_eq!(record.value, TerminalValue::Refuted);
    assert_eq!(record.basis, SettlementBasis::DecisiveCounterexample);
    assert!(matches!(
        record.counterexample.map(|payload| payload.kind),
        Some(CounterexampleKind::UndefinedEvaluation(_))
    ));
}
