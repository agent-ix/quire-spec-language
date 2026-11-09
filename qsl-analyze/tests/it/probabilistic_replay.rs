// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed checked-subject replay and EN-5 refutation integration.
use crate::prob_world::{self, World};
use ix_trace_rs::trace;
use qsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
use qsl_eval::simulation::model::ModelSystem;
use qsl_foundation::RequestIndex;
use qsl_replay::{
    replay_probabilistic_witness, ExactProbLimits, ExactProbabilityGraphError, IncompleteCause,
    ModelCheckLimit, ModelCheckLimits, ProbabilisticCounterexample, ProbabilityWitnessError,
    ProbabilityWitnessReplay, TerminalValue,
};
use quire_exact::{Cancel, CancelCause, Integer, Rational, Value};

const CLAIMS: &str = "
probabilistic TooMuch using inf on M over (m: M::Msg) under every scheduler on origin
{ probability <= 0.5 [ eventually holds(m.delivered) ] }
probabilistic TooLittle using inf on M over (m: M::Msg) under every scheduler on origin
{ probability >= 0.97 [ eventually holds(m.delivered) ] }
";
fn ratio(a: i64, b: i64) -> Rational {
    Rational::new(Integer::from(a), Integer::from(b)).unwrap()
}
fn evidence(world: &World, claim: &str) -> ProbabilisticCounterexample {
    let engines = Engines::qsl().unwrap();
    let record = analyze_items(
        &engines,
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request(claim, ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    )
    .remove(0);
    assert_eq!(record.terminal().unwrap().value(), &TerminalValue::Refuted);
    assert!(record.exact_probabilistic.is_none());
    record
        .probabilistic_counterexample
        .expect("reproduced evidence accompanies refuted")
        .evidence().clone()
}
fn replay(
    world: &World,
    witness: &ProbabilisticCounterexample,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    replay_probabilistic_witness(
        system.package(),
        &system,
        &witness.item,
        witness,
        limits,
        cancel,
    )
}
fn mass(result: Result<ProbabilityWitnessReplay, ProbabilityWitnessError>) -> Rational {
    let ProbabilityWitnessReplay::Reproduced(token) = result.expect("replay completes") else {
        panic!("must reproduce")
    };
    token.mass().clone()
}
#[trace("TC-637", "FR-202-AC-9")]
#[test]
fn real_link_refutations_replay_target_hits_and_terminal_nonreach() {
    let world = World::new(&prob_world::link(CLAIMS));
    for (claim, expected) in [("TooMuch", ratio(9, 10)), ("TooLittle", ratio(1, 25))] {
        let witness = evidence(&world, claim);
        let limits = ExactProbLimits::default();
        assert_eq!(
            mass(replay(&world, &witness, &limits, &Cancel::new())),
            expected
        );
        assert_eq!(
            mass(replay(&world, &witness, &limits, &Cancel::new())),
            expected
        );
    }
}
#[trace("TC-637", "FR-202-AC-10")]
#[test]
fn semantic_mutations_cannot_obtain_a_reproduced_token() {
    let world = World::new(&prob_world::link(CLAIMS));
    let witness = evidence(&world, "TooMuch");
    let limits = ExactProbLimits::default();
    let mut changed = witness.clone();
    changed.paths[0].steps[0]
        .action
        .operation
        .push_str("-disabled");
    assert!(matches!(
        replay(&world, &changed, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Refused)
    ));
    let mut changed = witness.clone();
    changed.paths[0].steps[0].draw = vec![Value::Integer(Integer::from(7_i64))];
    assert!(matches!(
        replay(&world, &changed, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Refused)
    ));
    let mut changed = witness.clone();
    changed.paths[0].steps[0].post_key = witness.initial_key.clone();
    assert!(matches!(
        replay(&world, &changed, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Refused)
    ));
    let mut changed = witness.clone();
    changed.paths[0].steps[0].transition = quire_semantic_value::model_transition::ModelTransition::new(
        "\"disabled\"".to_owned(),None,vec![],quire_canonical::Limits::new(1024)
    ).expect("canonical syntax with a disabled semantic identity");
    assert!(matches!(replay(&world,&changed,&limits,&Cancel::new()),Err(ProbabilityWitnessError::Refused)));
    let mut changed = witness.clone();
    changed.paths[0].steps[0].post = witness.initial.clone();
    assert!(matches!(
        replay(&world, &changed, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Refused)
    ));
    let mut changed = witness.clone();
    changed.binding = None;
    assert!(matches!(
        replay(&world, &changed, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Refused)
    ));
    let mut changed = witness.clone();
    changed.scheduler[0].1.operation.push_str("-disabled");
    assert!(matches!(
        replay(&world, &changed, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Refused)
    ));
}
#[trace("TC-637", "FR-202-AC-10")]
#[test]
fn duplicate_paths_live_prefixes_and_threshold_equality_have_no_token() {
    let world = World::new(&prob_world::link(CLAIMS));
    let witness = evidence(&world, "TooMuch");
    let mut changed = witness.clone();
    changed.paths[0].steps.push(witness.paths[0].steps[0].clone());
    assert!(matches!(replay(&world,&changed,&ExactProbLimits::default(),&Cancel::new()),Ok(ProbabilityWitnessReplay::Parity)),"a step after the first target cannot extend an accepted path");
    let mut changed = witness.clone();
    changed.paths.push(changed.paths[0].clone());
    assert!(matches!(
        replay(
            &world,
            &changed,
            &ExactProbLimits::default(),
            &Cancel::new()
        ),
        Ok(ProbabilityWitnessReplay::Parity)
    ));
    let nonreach = evidence(&world, "TooLittle");
    let mut changed = nonreach.clone();
    changed.paths[0].steps.pop();
    assert!(matches!(
        replay(
            &world,
            &changed,
            &ExactProbLimits::default(),
            &Cancel::new()
        ),
        Ok(ProbabilityWitnessReplay::Parity)
    ));
    let equal_world = World::new(&prob_world::link(&CLAIMS.replace("<= 0.5", "<= 0.9")));
    let mut equal = witness;
    // Use the equal-threshold claim's exact checked identity and scheduler binding.
    equal.item = qsl_replay::ObligationIdentity::new(
        equal_world
            .claim("TooMuch")
            .occurrence()
            .unwrap()
            .node()
            .clone(),
        None,
        0,
    );
    assert!(matches!(
        replay(
            &equal_world,
            &equal,
            &ExactProbLimits::default(),
            &Cancel::new()
        ),
        Ok(ProbabilityWitnessReplay::Parity)
    ));
}
#[trace("TC-637", "FR-202-AC-11")]
#[test]
fn witness_limits_and_final_acceptance_cancellation_never_yield_tokens() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, OnceLock};
    let world = World::new(&prob_world::link(CLAIMS));
    let witness = evidence(&world, "TooMuch");
    let limited = ExactProbLimits {
        max_witness_paths: 0,
        ..ExactProbLimits::default()
    };
    assert!(matches!(
        replay(&world, &witness, &limited, &Cancel::new()),
        Err(ProbabilityWitnessError::Limit {
            limit: ModelCheckLimit::ExactProbMaxWitnessPaths,
            value: 0
        })
    ));
    let live = Cancel::new();
    let _count = live.count_charges();
    assert_eq!(
        mass(replay(&world, &witness, &ExactProbLimits::default(), &live)),
        ratio(9, 10)
    );
    let final_poll = live.charges();
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let slot = Arc::new(OnceLock::<Cancel>::new());
        let weak = Arc::downgrade(&slot);
        let count = Arc::new(AtomicU32::new(0));
        let observed = Arc::clone(&count);
        let cancel = Cancel::observing(move || {
            if observed.fetch_add(1, Ordering::Relaxed) + 1 == final_poll {
                weak.upgrade().unwrap().get().unwrap().cancel(cause);
            }
        });
        slot.set(cancel.clone()).unwrap();
        assert!(
            matches!(replay(&world,&witness,&ExactProbLimits::default(),&cancel), Err(ProbabilityWitnessError::Graph(ExactProbabilityGraphError::Cancelled(found))) if found == cause)
        );
        assert_eq!(count.load(Ordering::Relaxed), final_poll);
        let cancel = Cancel::new();
        cancel.cancel(cause);
        let engines = Engines::qsl().unwrap();
        let record = analyze_items(
            &engines,
            vec![exact_probabilistic_item(
                RequestIndex::new(0),
                world.request("TooMuch", ModelCheckLimits::default()),
            )],
            &cancel,
        )
        .remove(0);
        assert_eq!(
            record.terminal().unwrap().value(),
            &TerminalValue::Incomplete(IncompleteCause::Cancelled { source: cause })
        );
        assert!(record.probabilistic_counterexample.is_none());
    }
}

#[trace("TC-637", "FR-202-AC-9")]
#[test]
fn workload_replay_preserves_weight_shares_and_emits_no_scheduler() {
    let world = World::new(&prob_world::link(
        &CLAIMS.replace("under every scheduler", "under Even").replace(">= 0.97", ">= 0.99"),
    ));
    use qsl_analyze::exact_probabilistic::{
        build_product, decide_reachability, produce_witness, ProbBuild, UnboundedObjective,
        WitnessScope,
    };
    let request = world.request("TooMuch", ModelCheckLimits::default());
    let limits = request.limits;
    let ProbBuild::Built(product) = build_product(request, &Cancel::new()).unwrap() else {
        panic!("defined product");
    };
    let target = product.labels.iter().map(|row| row[0]).collect::<Vec<_>>();
    let solved = decide_reachability(&product, &target, UnboundedObjective::Min, &limits)
        .expect("workload solver");
    assert_eq!(solved.values[product.initial[0].2.index()], qsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(391,400)));
    assert!(produce_witness(
        &product,
        &solved.policy,
        WitnessScope::Workload,
        product.initial[0].2,
        &target,
        &ratio(1, 2),
        &limits
    )
    .expect("workload witness producer")
    .is_some());
    let witness = evidence(&world, "TooMuch");
    assert!(witness.scheduler.is_empty());
    assert_eq!(
        mass(replay(
            &world,
            &witness,
            &ExactProbLimits::default(),
            &Cancel::new()
        )),
        ratio(17, 20)
    );
    let nonreach = evidence(&world,"TooLittle");
    assert!(nonreach.scheduler.is_empty());
    assert_eq!(mass(replay(&world,&nonreach,&ExactProbLimits::default(),&Cancel::new())),ratio(3,200));
}

#[trace("TC-637", "FR-202-AC-10")]
#[test]
fn claim_and_package_mismatches_are_refused() {
    let world = World::new(&prob_world::link(CLAIMS));
    let witness = evidence(&world, "TooMuch");
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let other = qsl_replay::ObligationIdentity::new(
        world
            .claim("TooLittle")
            .occurrence()
            .unwrap()
            .node()
            .clone(),
        None,
        0,
    );
    assert!(matches!(
        replay_probabilistic_witness(
            system.package(),
            &system,
            &other,
            &witness,
            &ExactProbLimits::default(),
            &Cancel::new()
        ),
        Err(ProbabilityWitnessError::Refused)
    ));
    let other_world = World::new(&prob_world::link(&CLAIMS.replace("<= 0.5", "<= 0.4")));
    let other_system = ModelSystem::new(other_world.subject().simulation).unwrap();
    assert!(matches!(
        replay_probabilistic_witness(
            other_system.package(),
            &system,
            &witness.item,
            &witness,
            &ExactProbLimits::default(),
            &Cancel::new()
        ),
        Err(ProbabilityWitnessError::Refused)
    ));
    let ProbabilityWitnessReplay::Reproduced(token) = replay(
        &world,
        &witness,
        &ExactProbLimits::default(),
        &Cancel::new(),
    )
    .unwrap() else {
        panic!("positive control");
    };
    assert!(qsl_replay::TerminalRecord::refuted_by_probability_witness(
        RequestIndex::new(0),
        qsl_foundation::BackendId::new("en-5"),
        &other,
        &token
    )
    .is_none());
}

#[trace("TC-637", "FR-202-AC-9")]
#[test]
fn two_thirds_first_hits_and_nonreach_leaves_cross_one_half() {
    let mut model = prob_world::link(&CLAIMS.replace(">= 0.97", ">= 0.5"));
    model.body = model
        .body
        .replace("< 2", "< 1")
        .replace("attempts = 2", "attempts = 1")
        .replace("true: 1, false: 9", "true: 1, false: 2")
        .replace("true: 1, false: 4", "true: 2, false: 1");
    let world = World::new(&model);
    for claim in ["TooMuch", "TooLittle"] {
        let witness = evidence(&world, claim);
        assert_eq!(
            mass(replay(
                &world,
                &witness,
                &ExactProbLimits::default(),
                &Cancel::new()
            )),
            ratio(2, 3)
        );
    }
}

#[trace("TC-637", "FR-202-AC-10", "FR-203-AC-3")]
#[test]
fn replay_parity_and_refusal_settle_inconclusive_without_evidence() {
    use qsl_analyze::analyze::exact_probabilistic_item_witness_tampered;
    let world = World::new(&prob_world::link(CLAIMS));
    let engines = Engines::qsl().unwrap();
    for duplicate in [true, false] {
        let item = exact_probabilistic_item_witness_tampered(
            RequestIndex::new(0),
            world.request("TooMuch", ModelCheckLimits::default()),
            move |witness| {
                if duplicate {
                    witness.paths.push(witness.paths[0].clone());
                } else {
                    witness.paths[0].steps[0]
                        .action
                        .operation
                        .push_str("-disabled");
                }
            },
        );
        let record = analyze_items(&engines, vec![item], &Cancel::new()).remove(0);
        if duplicate {
            assert!(matches!(
                record.terminal().unwrap().value(),
                TerminalValue::Inconclusive(qsl_replay::InconclusiveCause::ReplayParity(_))
            ));
        } else {
            assert!(matches!(
                record.terminal().unwrap().value(),
                TerminalValue::Inconclusive(qsl_replay::InconclusiveCause::ReplayRefused(
                    qsl_foundation::diagnostic::Code::InvalidRuntimeInput
                ))
            ));
        }
        assert!(record.probabilistic_counterexample.is_none());
        assert!(record.exact_probabilistic.is_none());
    }
}

#[trace("TC-637", "FR-202-AC-11")]
#[test]
fn graph_arithmetic_limits_and_cyclic_models_cannot_reproduce() {
    let world = World::new(&prob_world::link(CLAIMS));
    let witness = evidence(&world, "TooMuch");
    let mut limits = ExactProbLimits::default();
    limits.model.limits = qsl_eval::simulation::Limits::default().with_max_states(0);
    assert!(matches!(
        replay(&world, &witness, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Graph(
            ExactProbabilityGraphError::LimitReached
        ))
    ));
    let mut limits = ExactProbLimits::default();
    limits.model.limits.max_transitions = 0;
    assert!(matches!(
        replay(&world, &witness, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Graph(
            ExactProbabilityGraphError::EdgeLimitReached
        ))
    ));
    let limits = ExactProbLimits {
        max_rational_bits: 1,
        ..ExactProbLimits::default()
    };
    assert!(matches!(
        replay(&world, &witness, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Limit {
            limit: ModelCheckLimit::ExactProbMaxRationalBits,
            value: 1
        })
    ));
    let mut model = prob_world::link(CLAIMS);
    model.body = model.body.replace(
        "self.attempts - pre(self.attempts) = 1",
        "self.attempts - pre(self.attempts) = 0",
    );
    let cyclic = World::new(&model);
    let mut changed = witness;
    changed.item = qsl_replay::ObligationIdentity::new(
        cyclic.claim("TooMuch").occurrence().unwrap().node().clone(),
        None,
        0,
    );
    assert!(matches!(
        replay(
            &cyclic,
            &changed,
            &ExactProbLimits::default(),
            &Cancel::new()
        ),
        Err(ProbabilityWitnessError::Graph(
            ExactProbabilityGraphError::Cycle
        ))
    ));
}

#[trace("TC-637", "FR-202-AC-11")]
#[test]
fn cancellation_at_the_en5_replay_acceptance_boundary_has_no_evidence() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, OnceLock};
    let world = World::new(&prob_world::link(CLAIMS));
    let engines = Engines::qsl().unwrap();
    let run = |cancel: &Cancel| {
        analyze_items(
            &engines,
            vec![exact_probabilistic_item(
                RequestIndex::new(0),
                world.request("TooMuch", ModelCheckLimits::default()),
            )],
            cancel,
        )
        .remove(0)
    };
    let live = Cancel::new();
    let _count = live.count_charges();
    assert_eq!(
        run(&live).terminal().unwrap().value(),
        &TerminalValue::Refuted
    );
    let final_poll = live.charges();
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let slot = Arc::new(OnceLock::<Cancel>::new());
        let weak = Arc::downgrade(&slot);
        let seen = Arc::new(AtomicU32::new(0));
        let count = Arc::clone(&seen);
        let cancel = Cancel::observing(move || {
            if count.fetch_add(1, Ordering::Relaxed) + 1 == final_poll {
                weak.upgrade().unwrap().get().unwrap().cancel(cause);
            }
        });
        slot.set(cancel.clone()).unwrap();
        let record = run(&cancel);
        assert_eq!(
            record.terminal().unwrap().value(),
            &TerminalValue::Incomplete(IncompleteCause::Cancelled { source: cause })
        );
        assert!(record.probabilistic_counterexample.is_none());
        assert!(record.exact_probabilistic.is_none());
        assert_eq!(seen.load(Ordering::Relaxed), final_poll);
    }
}

#[trace("TC-637", "FR-202-AC-9", "FR-202-AC-11")]
#[test]
fn merged_draws_expand_into_actual_vectors_with_a_witness_storage_limit() {
    let mut model = prob_world::link(CLAIMS);
    model.body = model
        .body
        .replace("self.delivered = (not lost)", "self.delivered = true");
    let world = World::new(&model);
    let witness = evidence(&world, "TooMuch");
    assert_eq!(witness.paths.len(), 2);
    assert_eq!(
        mass(replay(
            &world,
            &witness,
            &ExactProbLimits::default(),
            &Cancel::new()
        )),
        ratio(1, 1)
    );
    assert!(!qsl_eval::simulation::model::same_values(
        &witness.paths[0].steps[0].draw,
        &witness.paths[1].steps[0].draw
    ));
    let engines = Engines::qsl().unwrap();
    let mut request = world.request("TooMuch", ModelCheckLimits::default());
    request.limits.max_witness_paths = 1;
    let record = analyze_items(
        &engines,
        vec![exact_probabilistic_item(RequestIndex::new(0), request)],
        &Cancel::new(),
    )
    .remove(0);
    assert_eq!(
        record.terminal().unwrap().value(),
        &TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::ExactProbMaxWitnessPaths,
            value: 1
        })
    );
    assert!(record.probabilistic_counterexample.is_none());
}
