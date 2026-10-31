// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-182 V-3 (TC-607): `settle_hyper_smt` over an EN-3 k-induction outcome.

use ix_trace_rs::trace;
use qsl_eval::simulation::model::ModelSystem;
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::smt::{Measure, Refusal as SmtRefusal};
use qsl_replay::{
    settle_hyper_smt, BoundCovers, Certification, HyperProductIdentity, HyperQueryRequest,
    HyperSettleRefusal, HyperSmtOutcome, HyperSmtResult, IncompleteCause, InconclusiveCause,
    ModelCheckLimit, ModelCheckLimits, ProductMember, ProofBasis, ReachedLimit, SettlementBasis,
    SmtEncodingLimits, SmtMethod, TerminalValue,
};
use quire_exact::{CancelCause, Meter};
use quire_semantic_value::catalog::InternalFault;

use crate::vault_world::{non_interference, World, EQUAL_INPUTS, UNLIMITED};

fn instance(key: &str) -> Vec<(String, String)> {
    vec![("v".to_owned(), key.to_owned())]
}

fn reset_loop(world: &World, variable: &str, initial: usize) -> qsl_replay::HyperTrace {
    let subject = world.subject();
    let system = ModelSystem::new(subject.simulation.clone()).expect("the model admits");
    let state = system
        .initial()
        .into_iter()
        .nth(initial)
        .expect("initial index");
    let (key, _) = system.state_key(&state).expect("state key");
    let key = qsl_eval::simulation::StateKey::from_canonical_bytes(key);
    let expansion = system
        .successors(&state)
        .expect("the initial state expands");
    let (transition, _) = expansion
        .successors
        .into_iter()
        .find(|(transition, post)| {
            transition.operation().contains("Vault/reset")
                && system
                    .state_key(post)
                    .is_ok_and(|(bytes, _)| bytes == key.as_bytes())
        })
        .expect("reset has a self-loop");
    qsl_replay::HyperTrace {
        variable: variable.to_owned(),
        alias: "V".to_owned(),
        initial: u32::try_from(initial).expect("initial index fits"),
        steps: vec![qsl_replay::HyperStep::Model(qsl_replay::ModelStep {
            transition,
            post_state: key,
            delay: None,
        })],
        marks: Vec::new(),
    }
}

/// Settle `result` for the item at instance `item_key`, the outcome carrying
/// the identity of instance `outcome_key`.
fn settle(
    world: &World,
    item_key: &str,
    outcome_key: &str,
    result: HyperSmtResult,
) -> Result<qsl_replay::HyperRecord, HyperSettleRefusal> {
    settle_with(world, item_key, Some(outcome_key), result)
}

/// As [`settle`]; `None` hands over an outcome whose `product` is `Err`.
fn settle_with(
    world: &World,
    item_key: &str,
    outcome_key: Option<&str>,
    result: HyperSmtResult,
) -> Result<qsl_replay::HyperRecord, HyperSettleRefusal> {
    let requested = match &result {
        HyperSmtResult::LimitStopped { limit, .. } => vec![limit.limit],
        _ => Vec::new(),
    };
    let subjects = [world.subject()];
    let mut meter = Meter::new(UNLIMITED);
    let product = match outcome_key {
        Some(key) => {
            let named = instance(key);
            Ok(HyperProductIdentity::of(&HyperQueryRequest {
                clause: world.clause(),
                subjects: &subjects,
                instance: &named,
                meter: &mut meter,
                encoding: SmtEncodingLimits::default(),
            })
            .expect("the outcome's identity"))
        }
        None => Err(InternalFault::new("smt-encoding", "the identity faulted")),
    };
    let key = instance(item_key);
    let item = HyperQueryRequest {
        clause: world.clause(),
        subjects: &subjects,
        instance: &key,
        meter: &mut meter,
        encoding: SmtEncodingLimits::default(),
    };
    settle_hyper_smt(
        &item,
        &HyperSmtOutcome { result, product },
        None,
        qsl_replay::ReplayLimits::default(),
        &ModelCheckLimits::default(),
        &requested,
    )
}

/// FR-182-AC-7: an `Inductive{depth: 1}` outcome with an empty `μ_U` settles
/// `proved`, `decisive-witness`, `Uncertified`, method `k-induction`.
#[trace("TC-607", "FR-182-AC-7")]
#[test]
fn an_inductive_outcome_with_an_empty_match_settles_proved_uncertified() {
    let world = World::new(&["v", "w"], false, &non_interference("", ""));
    let record = settle(&world, "v", "v", HyperSmtResult::Inductive { depth: 1 })
        .expect("the outcome answers the item");
    assert_eq!(
        record.value,
        TerminalValue::Proved {
            basis: ProofBasis::Inductive { depth: 1 },
            certification: Certification::Uncertified,
        }
    );
    assert_eq!(record.basis, SettlementBasis::DecisiveWitness);
    assert_eq!(record.method, "k-induction");
}

/// An `Inductive` outcome whose clause has a non-empty `μ_U` and no accepted
/// witness settles `inconclusive`, `VacuousMatch`, never `proved`.
#[trace("TC-607", "FR-182-AC-1")]
#[test]
fn an_inductive_outcome_with_a_match_and_no_witness_is_vacuous() {
    let world = World::new(&["v", "w"], false, &non_interference(EQUAL_INPUTS, ""));
    let record = settle(&world, "v", "v", HyperSmtResult::Inductive { depth: 1 })
        .expect("the outcome answers the item");
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::VacuousMatch)
    );
}

/// An outcome carrying another instance's identity, or no identity, is no
/// evidence for the item and is refused.
#[test]
fn an_outcome_for_another_product_is_refused() {
    let world = World::new(&["v", "w"], false, &non_interference("", ""));
    let result = HyperSmtResult::Inductive { depth: 1 };
    assert_eq!(
        settle(&world, "v", "w", result.clone()).unwrap_err(),
        HyperSettleRefusal::ProductMismatch {
            member: ProductMember::Instance
        }
    );
    assert_eq!(
        settle_with(&world, "v", None, result).unwrap_err(),
        HyperSettleRefusal::ProductMismatch {
            member: ProductMember::Clause
        }
    );
}

/// A subject with an empty initial list settles
/// `inconclusive`, `NoInitialState`, never `proved`.
#[trace("TC-607", "FR-182-AC-1")]
#[test]
fn an_empty_initial_list_settles_no_initial_state() {
    let world = World::new(&["v"], false, &non_interference("", "")).without_initial_states();
    let record = settle(&world, "v", "v", HyperSmtResult::Inductive { depth: 1 })
        .expect("the outcome answers the item");
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::NoInitialState)
    );
}

/// `InductionNotClosed{depth}` settles `inconclusive`, never `proved`.
#[trace("TC-607", "FR-182-AC-1")]
#[test]
fn an_unclosed_induction_is_inconclusive() {
    let world = World::new(&["v"], false, &non_interference("", ""));
    let record = settle(
        &world,
        "v",
        "v",
        HyperSmtResult::InductionNotClosed { depth: 1 },
    )
    .expect("the outcome answers the item");
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::InductionNotClosed { depth: 1 })
    );
}

/// FR-182-AC-12: an EN-2 bound states its method, depth and HP-2 reading.
#[trace("TC-1190", "FR-182-AC-12")]
#[test]
fn bounded_unrolling_records_the_joint_step_bound() {
    let world = World::new(&["v"], false, &non_interference("", ""));
    let record = settle(&world, "v", "v", HyperSmtResult::BoundReached { depth: 2 })
        .expect("the outcome answers the item");
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::BoundReached { depth: 2 })
    );
    assert_eq!(record.method, "smt-unrolling");
    assert_eq!(record.depth, Some(2));
    assert_eq!(record.covers, Some(BoundCovers::JointSteps));
}

/// FR-182-AC-12: an undecided solver result is not proof evidence.
#[trace("TC-1190", "FR-182-AC-12")]
#[test]
fn solver_unknown_timeout_cancellation_and_fault_keep_their_distinct_rows() {
    let world = World::new(&["v"], false, &non_interference("", ""));
    let unknown = settle(
        &world,
        "v",
        "v",
        HyperSmtResult::Unknown {
            depth: 1,
            method: SmtMethod::KInduction,
        },
    )
    .unwrap();
    assert_eq!(
        unknown.value,
        TerminalValue::Inconclusive(InconclusiveCause::InductionNotClosed { depth: 1 })
    );
    assert_eq!(unknown.method, "k-induction");

    let timed_out = settle(
        &world,
        "v",
        "v",
        HyperSmtResult::TimedOut {
            method: SmtMethod::SmtUnrolling,
        },
    )
    .unwrap();
    assert_eq!(
        timed_out.value,
        TerminalValue::Incomplete(IncompleteCause::TimedOut)
    );
    assert_eq!(timed_out.method, "smt-unrolling");

    let cancelled = settle(
        &world,
        "v",
        "v",
        HyperSmtResult::Cancelled {
            source: CancelCause::Requested,
            method: SmtMethod::KInduction,
        },
    )
    .unwrap();
    assert_eq!(
        cancelled.value,
        TerminalValue::Incomplete(IncompleteCause::Cancelled {
            source: CancelCause::Requested
        })
    );

    let fault = settle(
        &world,
        "v",
        "v",
        HyperSmtResult::BackendFault {
            method: SmtMethod::KInduction,
        },
    )
    .unwrap();
    assert_eq!(fault.value, TerminalValue::Failed);
    assert_eq!(fault.execution, qsl_replay::Execution::Failed);
}

/// FR-182-AC-12 and AC-14: only solver step/time limits bound an SMT run.
#[trace("TC-1190", "FR-182-AC-12", "TC-1250", "FR-182-AC-14")]
#[test]
fn smt_limit_stops_are_named_and_non_smt_limits_refuse() {
    let world = World::new(&["v"], false, &non_interference("", ""));
    let record = settle(
        &world,
        "v",
        "v",
        HyperSmtResult::LimitStopped {
            limit: ReachedLimit {
                limit: ModelCheckLimit::SmtMaxSolverSteps,
                value: 1000,
            },
            method: SmtMethod::KInduction,
        },
    )
    .unwrap();
    assert_eq!(
        record.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::SmtMaxSolverSteps,
            value: 1000,
        })
    );
    assert_eq!(
        record.limit_reached,
        Some(ReachedLimit {
            limit: ModelCheckLimit::SmtMaxSolverSteps,
            value: 1000,
        })
    );

    let time_record = settle(
        &world,
        "v",
        "v",
        HyperSmtResult::LimitStopped {
            limit: ReachedLimit {
                limit: ModelCheckLimit::SmtMaxSolverTime,
                value: 5,
            },
            method: SmtMethod::SmtUnrolling,
        },
    )
    .unwrap();
    assert_eq!(time_record.method, "smt-unrolling");
    assert_eq!(
        time_record.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::SmtMaxSolverTime,
            value: 5,
        })
    );

    assert_eq!(
        settle(
            &world,
            "v",
            "v",
            HyperSmtResult::LimitStopped {
                limit: ReachedLimit {
                    limit: ModelCheckLimit::MaxStates,
                    value: 5
                },
                method: SmtMethod::KInduction,
            }
        )
        .unwrap_err(),
        HyperSettleRefusal::LimitNotApplicable {
            limit: ModelCheckLimit::MaxStates
        },
    );
    assert_eq!(
        settle(
            &world,
            "v",
            "v",
            HyperSmtResult::LimitStopped {
                limit: ReachedLimit {
                    limit: ModelCheckLimit::SmtEncodingMaxConstants,
                    value: 7,
                },
                method: SmtMethod::KInduction,
            },
        )
        .unwrap_err(),
        HyperSettleRefusal::LimitNotApplicable {
            limit: ModelCheckLimit::SmtEncodingMaxConstants
        },
    );
}

/// FR-182-AC-12 and AC-14: encoding refusals distinguish uncertainty,
/// unsupported fragments, configured size limits and query-kind refusals.
#[trace("TC-1190", "FR-182-AC-12", "TC-1250", "FR-182-AC-14")]
#[test]
fn encoding_refusals_keep_their_distinct_rows() {
    let world = World::new(&["v"], false, &non_interference("", ""));
    let match_refusal = settle(
        &world,
        "v",
        "v",
        HyperSmtResult::EncodingRefused {
            refusal: SmtRefusal::MatchConjunct {
                clause: "NonInterference".to_owned(),
                conjunct: 0,
                site: "hyper NonInterference match 0".to_owned(),
                tuple: Vec::new(),
            }
            .into(),
        },
    )
    .unwrap();
    assert_eq!(
        match_refusal.value,
        TerminalValue::Inconclusive(InconclusiveCause::MatchUndetermined)
    );

    let too_large = settle(
        &world,
        "v",
        "v",
        HyperSmtResult::EncodingRefused {
            refusal: SmtRefusal::TooLarge {
                measure: Measure::Constants,
                value: 8,
                limit: 7,
                setting: "smt_encoding.max_constants",
            }
            .into(),
        },
    )
    .unwrap();
    assert_eq!(
        too_large.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::SmtEncodingMaxConstants,
            value: 7,
        })
    );

    assert_eq!(
        settle(
            &world,
            "v",
            "v",
            HyperSmtResult::EncodingRefused {
                refusal: SmtRefusal::QueryKind {
                    kind: "unrolling",
                    form: "HP-2"
                }
                .into(),
            }
        )
        .unwrap_err(),
        HyperSettleRefusal::QueryKind { kind: "unrolling" },
    );

    let unsupported = settle(
        &world,
        "v",
        "v",
        HyperSmtResult::EncodingRefused {
            refusal: SmtRefusal::HyperBody {
                clause: "NonInterference".to_owned(),
            }
            .into(),
        },
    )
    .unwrap();
    assert_eq!(
        unsupported.value,
        TerminalValue::Unsupported(qsl_replay::UnavailabilityCause::UnsupportedSmtFragment)
    );
}

/// FR-182-AC-12: a counterexample for a different object instance is rejected
/// before any replay can settle it.
#[trace("TC-1190", "FR-182-AC-12")]
#[test]
fn smt_counterexample_instance_must_match_the_outcome_identity() {
    let world = World::new(&["v", "w"], false, &non_interference("", ""));
    let counterexample = qsl_replay::HyperCounterexample {
        kind: qsl_replay::HyperCounterexampleKind::Lockstep { loop_entry: 0 },
        instance: instance("w"),
        traces: Vec::new(),
        trace_position: 0,
    };
    assert_eq!(
        settle(
            &world,
            "v",
            "v",
            HyperSmtResult::Violated {
                counterexample: Box::new(counterexample),
                method: SmtMethod::KInduction,
            }
        )
        .unwrap_err(),
        HyperSettleRefusal::ProductMismatch {
            member: ProductMember::Instance
        },
    );
}

/// FR-182-AC-10: the supplied witness is replayed against this clause and
/// instance, and an accepted matched fair tuple makes induction decisive.
#[trace("TC-607", "FR-182-AC-10")]
#[test]
fn an_inductive_outcome_checks_the_match_witness_itself() {
    let world = World::new(&["v"], false, &non_interference(EQUAL_INPUTS, ""));
    let witness = qsl_replay::MatchWitness {
        instance: instance("v"),
        traces: vec![reset_loop(&world, "a", 0), reset_loop(&world, "b", 0)],
        loop_entry: 0,
    };
    let accepted = settle_with_witness(&world, &witness);
    assert!(matches!(accepted.value, TerminalValue::Proved { .. }));
    assert_eq!(accepted.basis, SettlementBasis::DecisiveWitness);
}

fn settle_with_witness(
    world: &World,
    witness: &qsl_replay::MatchWitness,
) -> qsl_replay::HyperRecord {
    settle_with_witness_limits(world, witness, qsl_replay::ReplayLimits::default())
}

fn settle_with_witness_limits(
    world: &World,
    witness: &qsl_replay::MatchWitness,
    replay_limits: qsl_replay::ReplayLimits,
) -> qsl_replay::HyperRecord {
    let subjects = [world.subject()];
    let mut meter = Meter::new(UNLIMITED);
    let key = instance("v");
    let item = HyperQueryRequest {
        clause: world.clause(),
        subjects: &subjects,
        instance: &key,
        meter: &mut meter,
        encoding: SmtEncodingLimits::default(),
    };
    let product = HyperProductIdentity::of(&item).expect("the outcome identity");
    settle_hyper_smt(
        &item,
        &HyperSmtOutcome {
            result: HyperSmtResult::Inductive { depth: 1 },
            product: Ok(product),
        },
        Some(witness),
        replay_limits,
        &ModelCheckLimits::default(),
        &[],
    )
    .expect("the outcome answers the item")
}

/// A refused, mismatched or over-limit witness is not proof evidence.
#[trace("TC-607", "FR-182-AC-10")]
#[test]
fn rejected_and_over_limit_match_witnesses_remain_vacuous() {
    let world = World::new(&["v"], false, &non_interference(EQUAL_INPUTS, ""));
    let witness = qsl_replay::MatchWitness {
        instance: instance("v"),
        traces: vec![reset_loop(&world, "a", 0), reset_loop(&world, "b", 0)],
        loop_entry: 0,
    };
    let mut other_instance = witness.clone();
    other_instance.instance = instance("w");
    assert_eq!(
        settle_with_witness(&world, &other_instance).value,
        TerminalValue::Inconclusive(InconclusiveCause::VacuousMatch),
    );
    assert_eq!(
        settle_with_witness_limits(
            &world,
            &witness,
            qsl_replay::ReplayLimits::default().with_input_bytes(0),
        )
        .value,
        TerminalValue::Inconclusive(InconclusiveCause::VacuousMatch),
    );
}

/// FR-182-AC-12: a lockstep counterexample that replay reproduces is the only
/// route from a solver violation to `refuted`.
#[trace("TC-1190", "FR-182-AC-12")]
#[test]
fn a_reproduced_smt_counterexample_settles_refuted() {
    let world = World::new(
        &["v"],
        false,
        &crate::vault_world::with_body("", "", "always holds(v.h @ a = v.h @ b)"),
    );
    let counterexample = qsl_replay::HyperCounterexample {
        kind: qsl_replay::HyperCounterexampleKind::Lockstep { loop_entry: 0 },
        instance: instance("v"),
        traces: vec![reset_loop(&world, "a", 0), reset_loop(&world, "b", 1)],
        trace_position: 0,
    };
    let record = settle(
        &world,
        "v",
        "v",
        HyperSmtResult::Violated {
            counterexample: Box::new(counterexample),
            method: SmtMethod::SmtUnrolling,
        },
    )
    .expect("the outcome answers the item");
    assert_eq!(record.value, TerminalValue::Refuted);
    assert_eq!(record.basis, SettlementBasis::DecisiveCounterexample);
    assert_eq!(record.method, "smt-unrolling");
    assert!(record.counterexample.is_some());
}

/// A counterexample whose replay does not reproduce remains inconclusive.
#[trace("TC-1190", "FR-182-AC-12")]
#[test]
fn an_smt_counterexample_with_a_wrong_post_state_key_is_replay_refused() {
    let world = World::new(
        &["v"],
        false,
        &crate::vault_world::with_body("", "", "always holds(v.h @ a = v.h @ b)"),
    );
    let mut a = reset_loop(&world, "a", 0);
    let qsl_replay::HyperStep::Model(step) = &mut a.steps[0] else {
        unreachable!("reset_loop makes model steps")
    };
    let mut wrong_key = step.post_state.as_bytes().to_vec();
    wrong_key[0] ^= 1;
    step.post_state = qsl_eval::simulation::StateKey::from_canonical_bytes(wrong_key);
    let counterexample = qsl_replay::HyperCounterexample {
        kind: qsl_replay::HyperCounterexampleKind::Lockstep { loop_entry: 0 },
        instance: instance("v"),
        traces: vec![a, reset_loop(&world, "b", 1)],
        trace_position: 0,
    };
    let record = settle(
        &world,
        "v",
        "v",
        HyperSmtResult::Violated {
            counterexample: Box::new(counterexample),
            method: SmtMethod::KInduction,
        },
    )
    .expect("a refused replay settles inconclusive");
    assert!(matches!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(_))
    ));
    assert!(record.counterexample.is_none());
}
