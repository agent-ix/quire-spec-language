// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-182 (TC-607, TC-609): the settlement of a hyper run's outcome as a
//! terminal record, on the vault of `crate::hyper_world`.
//!
//! The outcomes are the engine's own, settled through the replay that gates
//! each refutation, or built by hand where a row has no engine run.
//!
//! Not claimed here: FR-182-AC-1 to AC-3 and AC-7, whose `proved` rows need
//! FR-163's product-closure certificate and checker, the SMT `k`-induction
//! row, the HP-1 rows and the HP-4 negotiation rows; AC-4 to AC-6, which are
//! HP-1's; AC-9, whose `NoInstance` settles from an engine run over an empty
//! universe the vault world does not model; and the "published defaults"
//! marking of a `proved` record in FR-184-AC-1.

use ix_trace_rs::trace;
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::diagnostic::Category;
use qsl_replay::{
    settle_hyper_check, BoundCovers, Code, DisagreementCause, Execution, HyperCounterexample,
    HyperCounterexampleKind, HyperOutcome, HyperRecord, IncompleteCause, InconclusiveCause,
    InternalFault, LimitOrigin, ModelCheckLimit, ModelCheckLimits, ModelTraceSettlement,
    ReachedLimit, ReplayRefusal, ResultLabel, RunStats, SettlementBasis, TerminalValue, Truth,
    UndecidedCause, EXPLICIT_STATE,
};
use quire_exact::{Cancel, Undefined};

use crate::hyper::refuted;
use crate::hyper_projected::{aligned, IDLE, MID_OPERATION};
use crate::hyper_witness::ExhaustedExistential;
use crate::hyper_world::{non_interference, opaque, Vault, World, EQUAL_INPUTS, STARTS};

/// `outcome` settled with `world`'s own replay under the default limits.
fn settle(world: &World, outcome: &HyperOutcome) -> HyperRecord {
    settle_under(world, outcome, &ModelCheckLimits::default(), &[])
}

fn settle_under(
    world: &World,
    outcome: &HyperOutcome,
    limits: &ModelCheckLimits,
    requested: &[ModelCheckLimit],
) -> HyperRecord {
    settle_hyper_check(outcome, limits, requested, |counterexample| {
        world.replay_under(counterexample, limits.max_witness_set)
    })
    .expect("the outcome settles")
}

fn run(world: &World, limits: ModelCheckLimits) -> HyperOutcome {
    world
        .run(limits, &Cancel::new())
        .expect("the run is not refused")
}

fn leaky_lockstep() -> World {
    World::new(
        Vault::Leaky,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    )
}

fn leaky_opaque() -> World {
    World::new(Vault::Leaky, false, &STARTS, &opaque(""))
}

/// The columns of a record that make up its row of FR-182's table.
fn row(record: &HyperRecord) -> (ResultLabel, SettlementBasis, Execution, Truth, Category) {
    (
        record.label,
        record.basis,
        record.execution,
        record.truth,
        record.value.category(),
    )
}

/// ADR-023 section 8.1's leaky refutation and section 8.2's leaky `Opaque`
/// refutation each settle `refuted`, `decisive-counterexample`, category
/// violation, only because their counterexample replays, and the record
/// carries it. The record names the method and no horizon.
#[trace("TC-607")]
#[test]
fn a_counterexample_that_replays_settles_refuted_and_is_carried() {
    for world in [leaky_lockstep(), leaky_opaque()] {
        let outcome = run(&world, ModelCheckLimits::default());
        let counterexample = refuted(outcome.clone());
        let record = settle(&world, &outcome);
        assert_eq!(record.value, TerminalValue::Refuted);
        assert_eq!(
            row(&record),
            (
                ResultLabel::Refuted,
                SettlementBasis::DecisiveCounterexample,
                Execution::Completed,
                Truth::Violated,
                Category::Violation
            )
        );
        assert_eq!(record.counterexample, Some(counterexample));
        assert_eq!(record.method, EXPLICIT_STATE);
        assert_eq!(
            (record.depth, record.covers, record.limit_reached),
            (None, None, None)
        );
    }
}

/// A refutation settles only through its replay: a replay that disagrees
/// settles `inconclusive` by `ReplayParity` and carries no counterexample; one
/// that a limit stopped settles `incomplete` naming it; a refused one settles
/// `inconclusive` by `ReplayRefused` with its code, and a fault `failed`, each
/// with no counterexample and no half value.
#[trace("TC-607")]
#[test]
fn a_refutation_that_does_not_replay_settles_with_no_counterexample() {
    let world = leaky_opaque();
    let outcome = run(&world, ModelCheckLimits::default());

    // The secure vault does not empty the set: the replay disagrees.
    let secure = World::new(Vault::Secure, false, &STARTS, &opaque(""));
    let parity = settle(&secure, &outcome);
    assert!(matches!(
        parity.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(
            DisagreementCause::Verdicts { .. }
        ))
    ));
    assert_eq!(
        row(&parity),
        (
            ResultLabel::Inconclusive,
            SettlementBasis::Unsettled,
            Execution::Completed,
            Truth::Pending,
            Category::Inconclusive
        )
    );
    assert_eq!(parity.counterexample, None);

    let tight = ModelCheckLimits {
        max_witness_set: 0,
        ..ModelCheckLimits::default()
    };
    let stopped = settle_under(&world, &outcome, &tight, &[ModelCheckLimit::MaxWitnessSet]);
    let reached = ReachedLimit {
        limit: ModelCheckLimit::MaxWitnessSet,
        value: 0,
    };
    assert_eq!(
        stopped.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxWitnessSet,
            value: 0
        })
    );
    assert_eq!(stopped.limit_reached, Some(reached));
    assert_eq!(stopped.execution, Execution::ResourceIncomplete);
    assert_eq!(stopped.counterexample, None);

    // A counterexample of one trace more than the clause has is refused.
    let mut extra = refuted(outcome.clone());
    extra.traces.push(extra.traces[0].clone());
    let refused = settle_hyper_check(
        &HyperOutcome::Violated {
            counterexample: Box::new(extra),
            stats: RunStats::default(),
        },
        &ModelCheckLimits::default(),
        &[],
        |counterexample| world.replay(counterexample),
    )
    .expect("settles");
    assert_eq!(
        refused.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(Code::InvalidRuntimeInput))
    );
    assert_eq!(refused.counterexample, None);

    let failed = settle_hyper_check(
        &outcome,
        &ModelCheckLimits::default(),
        &[],
        |_: &HyperCounterexample| -> Result<ModelTraceSettlement, ReplayRefusal> {
            Err(ReplayRefusal::Fault(InternalFault::new(
                "test",
                "replay-faults",
            )))
        },
    )
    .expect("settles");
    assert_eq!(failed.value, TerminalValue::Failed);
    assert_eq!(failed.label, ResultLabel::Failed);
    assert_eq!(failed.value.category(), Category::InternalFailure);
    assert_eq!(failed.counterexample, None);
}

/// FR-177-AC-5's outcome settles `refuted`, `decisive-counterexample`, with
/// the `Undefined` counterexample that names `a` and `b` and the
/// division-by-zero cause, after its replay reproduces it; FR-177-AC-6's and
/// FR-178-AC-6's outcomes settle `inconclusive`, `MatchUndetermined`.
#[trace("TC-607", "FR-182-AC-8")]
#[test]
fn an_undefined_tuple_refutes_and_an_undetermined_match_is_inconclusive() {
    let divided = " and (if present(b.step.Vault::step.i) then 1 / value(b.step.Vault::step.i) \
                   else 1) = 1";
    let world = World::new(Vault::Secure, false, &STARTS, &opaque(divided));
    let record = settle(&world, &run(&world, ModelCheckLimits::default()));
    assert_eq!(record.value, TerminalValue::Refuted);
    assert_eq!(
        row(&record),
        (
            ResultLabel::Refuted,
            SettlementBasis::DecisiveCounterexample,
            Execution::Completed,
            Truth::Violated,
            Category::Violation
        )
    );
    let Some(HyperCounterexample {
        kind: HyperCounterexampleKind::Undefined(undefined),
        ..
    }) = &record.counterexample
    else {
        panic!("the record carries the undefined-evaluation counterexample");
    };
    assert_eq!(
        undefined.cause,
        UndefinedCause::Kernel(Undefined::DivisionByZero)
    );
    let named: Vec<&str> = undefined
        .variables
        .iter()
        .map(|(variable, _)| variable.as_str())
        .collect();
    assert_eq!(named, ["a", "b"]);

    let secure = World::new(Vault::Secure, false, &STARTS, &opaque(""));
    let existential = secure
        .run_through(
            &secure.seams(),
            &ExhaustedExistential(secure.seams()),
            ModelCheckLimits::default(),
            &Cancel::new(),
        )
        .expect("the run is not refused");
    let projected = World::busy(Vault::Busy, &IDLE, &aligned(&[]));
    let projected_outcome = projected
        .run_through(
            &projected.seams(),
            &crate::hyper::Exhausted,
            ModelCheckLimits::default(),
            &Cancel::new(),
        )
        .expect("the run is not refused");
    for (world, outcome) in [(&secure, existential), (&projected, projected_outcome)] {
        let record = settle(world, &outcome);
        assert_eq!(
            record.value,
            TerminalValue::Inconclusive(InconclusiveCause::MatchUndetermined)
        );
        assert_eq!(
            row(&record),
            (
                ResultLabel::Inconclusive,
                SettlementBasis::Unsettled,
                Execution::Completed,
                Truth::Pending,
                Category::Inconclusive
            )
        );
        assert_eq!(record.counterexample, None);
    }
}

/// The causes of a run that decided nothing settle `inconclusive`,
/// `unsettled`, completed and pending, each by its own cause, and none carries
/// a counterexample.
#[trace("TC-607")]
#[test]
fn a_run_that_decided_nothing_settles_by_its_cause() {
    let world = leaky_lockstep();
    for (cause, expected) in [
        (
            UndecidedCause::VacuousMatch,
            InconclusiveCause::VacuousMatch,
        ),
        (UndecidedCause::NoInstance, InconclusiveCause::NoInstance),
        (
            UndecidedCause::NoInitialState,
            InconclusiveCause::NoInitialState,
        ),
        (
            UndecidedCause::UndecidedSuccessor,
            InconclusiveCause::UndecidedSuccessor,
        ),
    ] {
        let record = settle(
            &world,
            &HyperOutcome::Undecided {
                cause,
                stats: RunStats::default(),
            },
        );
        assert_eq!(record.value, TerminalValue::Inconclusive(expected));
        assert_eq!(
            (record.label, record.basis, record.execution, record.truth),
            (
                ResultLabel::Inconclusive,
                SettlementBasis::Unsettled,
                Execution::Completed,
                Truth::Pending
            )
        );
        assert_eq!(record.counterexample, None);
    }
}

/// `Opaque` over the leaky vault with `max_witness_set` 0 stops the run, and
/// the record is `failed` with execution `resource-incomplete`, naming the
/// limit, its value and the count one past it, and carrying no counterexample.
/// A cancelled run keeps its cancellation and names no limit.
#[trace("TC-607")]
#[test]
fn a_stopped_run_names_its_limit_its_value_and_its_count() {
    let world = leaky_opaque();
    let limits = ModelCheckLimits {
        max_witness_set: 0,
        ..ModelCheckLimits::default()
    };
    let record = settle_under(
        &world,
        &run(&world, limits),
        &limits,
        &[ModelCheckLimit::MaxWitnessSet],
    );
    assert_eq!(
        record.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxWitnessSet,
            value: 0
        })
    );
    assert_eq!(
        row(&record),
        (
            ResultLabel::Failed,
            SettlementBasis::Unavailable,
            Execution::ResourceIncomplete,
            Truth::Unavailable,
            Category::Incomplete
        )
    );
    assert_eq!(record.count, Some(1));
    assert_eq!(
        record.limit_reached,
        Some(ReachedLimit {
            limit: ModelCheckLimit::MaxWitnessSet,
            value: 0
        })
    );
    assert_eq!(record.counterexample, None);

    let cancel = Cancel::new();
    cancel.cancel(quire_exact::CancelCause::Requested);
    let cancelled = settle(
        &world,
        &world
            .run(ModelCheckLimits::default(), &cancel)
            .expect("the run is not refused"),
    );
    assert!(matches!(
        cancelled.value,
        TerminalValue::Incomplete(IncompleteCause::Cancelled { .. })
    ));
    assert_eq!((cancelled.limit_reached, cancelled.count), (None, None));
}

/// A search horizon settles `inconclusive`, `BoundReached{depth}`, completed
/// and pending, and the record states what the bound covers: lasso joint steps
/// for an HP-2 and an HP-3 item, moves for an HP-6 item.
#[trace("TC-607")]
#[test]
fn a_horizon_settles_inconclusive_and_the_record_states_what_it_covers() {
    let mut lockstep = leaky_lockstep();
    lockstep.max_depth = Some(1);
    let mut witness = leaky_opaque();
    witness.max_depth = Some(1);
    let mut projected = World::busy(Vault::BusyLeaky, &MID_OPERATION, &aligned(&[]));
    projected.max_depth = Some(1);
    for (world, covers) in [
        (&lockstep, BoundCovers::JointSteps),
        (&witness, BoundCovers::JointSteps),
        (&projected, BoundCovers::Moves),
    ] {
        let record = settle(world, &run(world, ModelCheckLimits::default()));
        assert_eq!(
            record.value,
            TerminalValue::Inconclusive(InconclusiveCause::BoundReached { depth: 1 })
        );
        assert_eq!(
            row(&record),
            (
                ResultLabel::Inconclusive,
                SettlementBasis::Unsettled,
                Execution::Completed,
                Truth::Pending,
                Category::Inconclusive
            )
        );
        assert_eq!((record.depth, record.covers), (Some(1), Some(covers)));
    }
}

/// Every record, a refuted one and a stopped one alike, states the value of
/// each model-check limit it used, FR-126's and FR-184's, marked as the
/// request's when the request set it and as the published default when not.
#[trace("TC-609")]
#[test]
fn a_record_states_each_limit_with_where_its_value_came_from() {
    let world = leaky_opaque();
    let defaults = ModelCheckLimits::default();
    let refuted_record = settle(&world, &run(&world, defaults));
    let tight = ModelCheckLimits {
        max_witness_set: 0,
        ..defaults
    };
    let stopped_record = settle_under(
        &world,
        &run(&world, tight),
        &tight,
        &[ModelCheckLimit::MaxWitnessSet],
    );
    let seen = |record: &HyperRecord| {
        record
            .limits
            .iter()
            .map(|used| (used.limit, used.value, used.origin))
            .collect::<Vec<_>>()
    };
    let expected = |witness: u64, origin: LimitOrigin| {
        vec![
            (
                ModelCheckLimit::MaxStates,
                defaults.limits.max_states as u64,
                LimitOrigin::PublishedDefault,
            ),
            (
                ModelCheckLimit::MaxTransitions,
                defaults.limits.max_transitions as u64,
                LimitOrigin::PublishedDefault,
            ),
            (
                ModelCheckLimit::MaxAutomatonStates,
                defaults.max_automaton_states,
                LimitOrigin::PublishedDefault,
            ),
            (ModelCheckLimit::MaxWitnessSet, witness, origin),
            (
                ModelCheckLimit::MaxRelationTuples,
                defaults.max_relation_tuples,
                LimitOrigin::PublishedDefault,
            ),
        ]
    };
    assert_eq!(
        seen(&refuted_record),
        expected(defaults.max_witness_set, LimitOrigin::PublishedDefault)
    );
    assert_eq!(seen(&stopped_record), expected(0, LimitOrigin::Requested));
}

/// A `Holds` outcome settles no record: it would settle `proved` only after
/// FR-163's product-closure certificate checker accepts its certificate, and
/// the certificate and the checker are not built.
#[trace("TC-607")]
#[test]
fn a_holds_outcome_settles_no_record_until_the_product_closure_checker_exists() {
    let secure = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let outcome = run(&secure, ModelCheckLimits::default());
    assert!(matches!(outcome, HyperOutcome::Holds { .. }));
    let result = settle_hyper_check(&outcome, &ModelCheckLimits::default(), &[], |_| {
        panic!("a holds outcome replays nothing")
    });
    assert_eq!(
        result,
        Err(qsl_replay::HyperSettleRefusal::ProductClosureCheckerNotBuilt)
    );
}
