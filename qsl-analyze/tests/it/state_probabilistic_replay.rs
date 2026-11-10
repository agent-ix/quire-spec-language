// SPDX-License-Identifier: AGPL-3.0-or-later
//! Direct infinite-trace state-event witnesses use independent native replay.
use crate::prob_world::{self, Operation, World};
use ix_trace_rs::trace;
use qsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
use qsl_eval::simulation::model::ModelSystem;
use qsl_eval::simulation::TransitionSystem;
use qsl_foundation::RequestIndex;
use qsl_replay::{
    replay_probabilistic_witness, ExactProbLimits, ModelCheckLimits, ProbabilisticCounterexample,
    ProbabilityWitnessError, ProbabilityWitnessPath, ProbabilityWitnessReplay, TerminalValue,
};
use quire_exact::{Cancel, Integer, Rational};

fn ratio(a: i64, b: i64) -> Rational {
    Rational::new(Integer::from(a), Integer::from(b)).unwrap()
}
fn model(event: &str, scheduler: &str, operator: &str, threshold: &str) -> prob_world::Model {
    let claims = format!("probabilistic Claim using inf on M over (m: M::Msg) under {scheduler} on origin {{ probability {operator} {threshold} [ {event} ] }}");
    let mut model = prob_world::link(&claims);
    model.body = model
        .body
        .replace("true: 1, false: 9", "true: 3, false: 1")
        .replace("true: 1, false: 4", "true: 1, false: 3")
        .replace("self.attempts < 2", "self.attempts = 0")
        .replace("reward cost on Msg::send_a = 1;", "")
        .replace("reward cost on Msg::send_b = 1;", "");
    model.operations.push(Operation {
        name: "recover",
        parameters: vec![],
        modifies: vec!["delivered", "attempts"],
        deletes: false,
    });
    model.body.push_str("\npre Ready_recover using v on M::Msg::recover { not self.delivered and self.attempts = 1 }\npost Recovered using v on M::Msg::recover { self.delivered and self.attempts = 2 }\n");
    // A resolved workload must also name the deterministic continuation.
    model.body = model.body.replace(
        "weight Msg::send_b = 1;",
        "weight Msg::send_b = 1; weight Msg::recover = 1;",
    );
    model
}
fn world(event: &str, scheduler: &str, operator: &str, threshold: &str) -> World {
    World::new(&model(event, scheduler, operator, threshold))
}
fn evidence(world: &World) -> ProbabilisticCounterexample {
    let mut records = analyze_items(
        &Engines::qsl().unwrap(),
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("Claim", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    );
    let record = records.remove(0);
    assert_eq!(
        record.terminal().unwrap().value(),
        &TerminalValue::Refuted,
        "{record:?}"
    );
    assert!(record.exact_probabilistic.is_none());
    record
        .probabilistic_counterexample
        .unwrap()
        .evidence()
        .clone()
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
fn mass(world: &World, witness: &ProbabilisticCounterexample) -> Rational {
    match replay(world, witness, &ExactProbLimits::default(), &Cancel::new()).unwrap() {
        ProbabilityWitnessReplay::Reproduced(token) => token.mass().clone(),
        ProbabilityWitnessReplay::Parity => panic!("direct state event must reproduce"),
    }
}
fn no_token(world: &World, witness: &ProbabilisticCounterexample) {
    assert!(!matches!(
        replay(world, witness, &ExactProbLimits::default(), &Cancel::new()),
        Ok(ProbabilityWitnessReplay::Reproduced(_))
    ));
}
const UNTIL: &str = "holds(m.attempts = 0) until holds(m.delivered)";
const ALWAYS: &str = "always holds(m.attempts = 0 or m.delivered)";

#[trace("TC-637", "FR-202-AC-17", "FR-202-AC-18")]
#[test]
fn direct_until_and_always_refute_both_extrema_and_resolved_workloads() {
    for event in [UNTIL, ALWAYS] {
        for (scheduler, operator, threshold, expected) in [
            ("every scheduler", ">=", "0.5", ratio(3, 4)),
            ("every scheduler", "<=", "0.5", ratio(3, 4)),
            ("Even", ">=", "0.6", ratio(1, 2)),
            ("Even", "<=", "0.4", ratio(1, 2)),
        ] {
            let world = world(event, scheduler, operator, threshold);
            let witness = evidence(&world);
            assert_eq!(mass(&world, &witness), expected);
            assert_eq!(mass(&world, &witness), expected);
            assert!(
                witness.paths.iter().all(|path| path.steps.len() == 1),
                "first verdict ends the path before recovery"
            );
            assert_eq!(witness.scheduler.is_empty(), scheduler == "Even");
        }
    }
}

#[trace("TC-637", "FR-202-AC-19")]
#[test]
fn initial_state_verdicts_use_empty_cylinders() {
    for (event, operator) in [
        ("holds(false) until holds(m.attempts = 0)", "<="),
        ("holds(false) until holds(m.delivered)", ">="),
        ("always holds(m.attempts > 0)", ">="),
    ] {
        let world = world(event, "every scheduler", operator, "0.5");
        let witness = evidence(&world);
        assert_eq!(witness.paths.len(), 1);
        assert!(witness.paths[0].steps.is_empty());
        assert_eq!(mass(&world, &witness), ratio(1, 1));
    }
}

#[trace("TC-637", "FR-202-AC-20", "FR-202-AC-21")]
#[test]
fn altered_semantics_prefixes_policy_and_duplicate_cylinders_have_no_token() {
    for event in [UNTIL, ALWAYS] {
        let world = world(event, "every scheduler", ">=", "0.5");
        let witness = evidence(&world);
        let mut changed = witness.clone();
        changed.paths.push(changed.paths[0].clone());
        no_token(&world, &changed);
        let mut changed = witness.clone();
        changed.paths = vec![ProbabilityWitnessPath { steps: vec![] }];
        no_token(&world, &changed);
        let mut changed = witness.clone();
        changed.paths[0].steps[0].post = witness.initial.clone();
        no_token(&world, &changed);
        let mut changed = witness.clone();
        changed.paths[0].steps[0].post_key = witness.initial_key.clone();
        no_token(&world, &changed);
        let mut changed = witness.clone();
        changed.paths[0].steps[0].draw = vec![quire_exact::Value::Integer(Integer::from(7_i64))];
        no_token(&world, &changed);
        let mut changed = witness.clone();
        changed.binding = None;
        no_token(&world, &changed);
        let mut changed = witness.clone();
        changed.paths[0].steps[0]
            .action
            .operation
            .push_str("-invalid");
        no_token(&world, &changed);
        let mut changed = witness.clone();
        changed.paths[0]
            .steps
            .push(witness.paths[0].steps[0].clone());
        no_token(&world, &changed);
        let mut changed = witness.clone();
        changed.scheduler.clear();
        no_token(&world, &changed);
    }
}

#[trace("TC-637", "FR-202-AC-22")]
#[test]
fn cancellation_and_caller_storage_arithmetic_limits_return_no_token() {
    for event in [UNTIL, ALWAYS] {
        let world = world(event, "every scheduler", ">=", "0.5");
        let witness = evidence(&world);
        let cancel = Cancel::new();
        cancel.cancel(quire_exact::CancelCause::Requested);
        assert!(replay(&world, &witness, &ExactProbLimits::default(), &cancel).is_err());
        let mut limits = ExactProbLimits::default();
        limits.document.storage_bytes = 1;
        assert!(replay(&world, &witness, &limits, &Cancel::new()).is_err());
        let limits = ExactProbLimits {
            max_rational_bits: 1,
            ..ExactProbLimits::default()
        };
        assert!(replay(&world, &witness, &limits, &Cancel::new()).is_err());
    }
}

fn empty(world: &World) -> ProbabilisticCounterexample {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let binding = qsl_replay::claim_binding(&system, world.claim("Claim")).unwrap();
    let reference = binding.as_ref().map(|binding| binding.reference.clone());
    let initial = system.initial().remove(0);
    let bytes = system.state_key(&initial).unwrap().0;
    ProbabilisticCounterexample {
        item: qsl_replay::probabilistic_claim_identity(world.claim("Claim")).unwrap(),
        binding: reference.clone(),
        initial_key: qsl_replay::ProbabilityProductKey::for_model_state(reference.as_ref(), &bytes)
            .unwrap(),
        initial,
        scheduler: vec![],
        paths: vec![ProbabilityWitnessPath { steps: vec![] }],
    }
}

#[trace("TC-637", "FR-202-AC-19", "FR-202-AC-20")]
#[test]
fn terminal_stutter_is_evaluated_without_fabricating_a_model_step() {
    for (event, operator, delivered) in [
        ("holds(true) until holds(m.delivered)", ">=", false),
        ("always holds(true)", "<=", false),
    ] {
        let mut model = model(event, "every scheduler", operator, "0.5");
        model.initial =
            serde_json::json!({"delivered": {"boolean": delivered}, "attempts": {"integer": "2"}});
        let world = World::new(&model);
        let witness = evidence(&world);
        assert!(witness.paths[0].steps.is_empty());
        assert_eq!(mass(&world, &witness), ratio(1, 1));
        assert_eq!(mass(&world, &empty(&world)), ratio(1, 1));
        let mut changed = witness;
        changed.item = qsl_replay::probabilistic_claim_identity(world.claim("Claim")).unwrap();
        changed.initial_key = qsl_replay::ProbabilityProductKey::from_canonical_bytes(vec![]);
        no_token(&world, &changed);
    }
}

#[trace("TC-637", "FR-202-AC-20")]
#[test]
fn equality_source_identity_and_opposite_verdicts_do_not_reproduce() {
    for event in [UNTIL, ALWAYS] {
        let original = world(event, "every scheduler", ">=", "0.5");
        let witness = evidence(&original);
        let equal = world(event, "every scheduler", ">=", "0.25");
        no_token(&equal, &witness);
        let mut rebound = witness.clone();
        rebound.item = qsl_replay::probabilistic_claim_identity(equal.claim("Claim")).unwrap();
        assert!(matches!(
            replay(
                &equal,
                &rebound,
                &ExactProbLimits::default(),
                &Cancel::new()
            ),
            Ok(ProbabilityWitnessReplay::Parity)
        ));
        let changed_event = if event == UNTIL {
            "holds(true) until holds(m.delivered or m.attempts = 1)"
        } else {
            "always holds(true)"
        };
        let changed_formula = world(changed_event, "every scheduler", ">=", "0.5");
        let mut wrong_formula = witness.clone();
        wrong_formula.item =
            qsl_replay::probabilistic_claim_identity(changed_formula.claim("Claim")).unwrap();
        no_token(&changed_formula, &wrong_formula);
        let opposite = world(event, "every scheduler", "<=", "0.5");
        rebound.item = qsl_replay::probabilistic_claim_identity(opposite.claim("Claim")).unwrap();
        no_token(&opposite, &rebound);
    }
}

#[trace("TC-637", "FR-202-AC-21")]
#[test]
fn original_cycles_are_rejected_even_beyond_initial_state_boundaries() {
    for event in ["holds(false) until holds(true)", "always holds(false)"] {
        let mut cyclic = model(event, "every scheduler", "<=", "0.5");
        cyclic.body = cyclic.body.replace(
            "self.attempts - pre(self.attempts) = 1",
            "self.attempts = pre(self.attempts)",
        );
        let world = World::new(&cyclic);
        let witness = empty(&world);
        assert!(matches!(
            replay(
                &world,
                &witness,
                &ExactProbLimits::default(),
                &Cancel::new()
            ),
            Err(ProbabilityWitnessError::Graph(
                qsl_replay::ExactProbabilityGraphError::Cycle
            ))
        ));
    }
}

#[trace("TC-637", "FR-202-AC-22")]
#[test]
fn cancellation_at_the_final_acceptance_poll_cannot_release_a_token() {
    use std::sync::{
        atomic::{AtomicU32, Ordering},
        Arc, OnceLock,
    };
    for event in [UNTIL, ALWAYS] {
        let world = world(event, "every scheduler", ">=", "0.5");
        let witness = evidence(&world);
        let live = Cancel::new();
        let _count = live.count_charges();
        assert!(matches!(
            replay(&world, &witness, &ExactProbLimits::default(), &live),
            Ok(ProbabilityWitnessReplay::Reproduced(_))
        ));
        let final_poll = live.charges();
        let slot = Arc::new(OnceLock::<Cancel>::new());
        let weak = Arc::downgrade(&slot);
        let count = Arc::new(AtomicU32::new(0));
        let observed = Arc::clone(&count);
        let cancel = Cancel::observing(move || {
            if observed.fetch_add(1, Ordering::Relaxed) + 1 == final_poll {
                weak.upgrade()
                    .unwrap()
                    .get()
                    .unwrap()
                    .cancel(quire_exact::CancelCause::Requested);
            }
        });
        slot.set(cancel.clone()).unwrap();
        assert!(matches!(
            replay(&world, &witness, &ExactProbLimits::default(), &cancel),
            Err(ProbabilityWitnessError::Graph(
                qsl_replay::ExactProbabilityGraphError::Cancelled(_)
            ))
        ));
        assert_eq!(count.load(Ordering::Relaxed), final_poll);
    }
}

#[trace("TC-637", "FR-202-AC-20", "FR-202-AC-21")]
#[test]
fn disjoint_actual_draws_to_one_post_state_count_once_each() {
    for event in [UNTIL, ALWAYS] {
        let mut model = model(event, "every scheduler", "<=", "0.5");
        model.body = model
            .body
            .replace("self.delivered = (not lost)", "self.delivered");
        let world = World::new(&model);
        let witness = evidence(&world);
        assert_eq!(
            witness.paths.len(),
            2,
            "merged producer edge expands to actual draw cylinders"
        );
        assert_eq!(
            witness.paths[0].steps[0].post_key,
            witness.paths[1].steps[0].post_key
        );
        assert!(!qsl_eval::simulation::model::same_values(
            &witness.paths[0].steps[0].draw,
            &witness.paths[1].steps[0].draw
        ));
        assert_eq!(mass(&world, &witness), ratio(1, 1));
        let mut repeated = witness.clone();
        repeated.paths.push(repeated.paths[0].clone());
        no_token(&world, &repeated);
        let mut changed = witness.clone();
        let key: serde_json::Value =
            serde_json::from_slice(witness.initial_key.as_bytes()).unwrap();
        assert!(!key[1].is_null());
        assert!(key[3].is_null() && key[4].is_null() && key[5].is_null());
        let mut missing_binding = key.clone();
        missing_binding[1] = serde_json::Value::Null;
        changed.initial_key = qsl_replay::ProbabilityProductKey::from_canonical_bytes(
            serde_json::to_vec(&missing_binding).unwrap(),
        );
        no_token(&world, &changed);
        let mut fake_monitor = key;
        fake_monitor[3] = serde_json::json!(["formula", 0]);
        changed.initial_key = qsl_replay::ProbabilityProductKey::from_canonical_bytes(
            serde_json::to_vec(&fake_monitor).unwrap(),
        );
        no_token(&world, &changed);
    }
}

#[trace("TC-637", "FR-202-AC-21")]
#[test]
fn choices_at_one_complete_key_and_workload_policies_must_be_consistent() {
    for event in [UNTIL, ALWAYS] {
        let world = world(event, "every scheduler", ">=", "0.5");
        let mut witness = evidence(&world);
        let system = ModelSystem::new(world.subject().simulation).unwrap();
        let step = &witness.paths[0].steps[0];
        let other = system
            .actions(&witness.initial)
            .unwrap()
            .unwrap()
            .into_iter()
            .find(|action| action.identity != step.action)
            .unwrap();
        let draw = other
            .draws
            .into_iter()
            .find(|draw| qsl_eval::simulation::model::same_values(&draw.random, &step.draw))
            .unwrap();
        let (transition, post) = draw.successors.into_iter().next().unwrap();
        let mut alternative = step.clone();
        alternative.action = other.identity;
        alternative.transition = transition;
        alternative.post = post;
        witness.paths.push(ProbabilityWitnessPath {
            steps: vec![alternative],
        });
        no_token(&world, &witness);
        let resolved = self::world(event, "Even", ">=", "0.6");
        let mut workload = evidence(&resolved);
        workload.scheduler = witness.scheduler;
        no_token(&resolved, &workload);
    }
}

#[trace("TC-637", "FR-202-AC-22")]
#[test]
fn adapter_budget_exhaustion_keeps_incomplete_without_evidence() {
    for event in [UNTIL, ALWAYS] {
        let world = world(event, "every scheduler", ">=", "0.5");
        for budget in [
            "paths",
            "states",
            "transitions",
            "work",
            "storage",
            "expansion",
            "scalar",
        ] {
            let mut request = world.request("Claim", ModelCheckLimits::default());
            match budget {
                "paths" => request.limits.max_witness_paths = 0,
                "states" => request.limits.model.limits.max_states = 0,
                "transitions" => request.limits.model.limits.max_transitions = 0,
                "work" => request.subject.simulation.meter.work_units = 0,
                "storage" => request.limits.document.storage_bytes = 0,
                "expansion" => request.subject.simulation.expansion.max_candidates = 0,
                "scalar" => request.limits.max_rational_bits = 1,
                _ => unreachable!(),
            }
            let record = analyze_items(
                &Engines::qsl().unwrap(),
                vec![exact_probabilistic_item(RequestIndex::new(0), request)],
                &Cancel::new(),
            )
            .remove(0);
            assert!(
                matches!(
                    record.terminal().unwrap().value(),
                    TerminalValue::Incomplete(qsl_replay::IncompleteCause::LimitReached { .. })
                ),
                "{budget}: {:?}",
                record.terminal().unwrap().value()
            );
            assert!(record.probabilistic_counterexample.is_none(), "{budget}");
            assert!(record.exact_probabilistic.is_none(), "{budget}");
        }
    }
}

#[trace("TC-637", "FR-202-AC-20")]
#[test]
fn actual_s3_over_parameter_and_single_admitted_instance_drive_replay() {
    for event in [UNTIL, ALWAYS] {
        let world = World::with_universe(
            &model(event, "every scheduler", ">=", "0.5"),
            &["m", "other"],
        );
        let mut request = world.request("Claim", ModelCheckLimits::default());
        request.subject.simulation.universes[0]
            .keys
            .retain(|key| key == "m");
        let system = ModelSystem::new(request.subject.simulation.clone()).unwrap();
        let record = analyze_items(
            &Engines::qsl().unwrap(),
            vec![exact_probabilistic_item(RequestIndex::new(0), request)],
            &Cancel::new(),
        )
        .remove(0);
        assert_eq!(record.terminal().unwrap().value(), &TerminalValue::Refuted);
        let witness = record
            .probabilistic_counterexample
            .as_ref()
            .unwrap()
            .evidence();
        let actual_parameter = world.claim("Claim").over.as_ref().unwrap().node().unwrap();
        assert_ne!(
            actual_parameter,
            witness.item.node(),
            "the actual checked over node differs from the claim node"
        );
        assert!(matches!(
            replay_probabilistic_witness(
                system.package(),
                &system,
                &witness.item,
                witness,
                &ExactProbLimits::default(),
                &Cancel::new()
            ),
            Ok(ProbabilityWitnessReplay::Reproduced(_))
        ));
        let unrestricted = ModelSystem::new(world.subject().simulation).unwrap();
        assert!(matches!(
            replay_probabilistic_witness(
                unrestricted.package(),
                &unrestricted,
                &witness.item,
                witness,
                &ExactProbLimits::default(),
                &Cancel::new()
            ),
            Err(ProbabilityWitnessError::Graph(
                qsl_replay::ExactProbabilityGraphError::UnsupportedBinding
            ))
        ));
    }
}

#[trace("TC-637", "FR-202-AC-20")]
#[test]
fn maximum_mass_equal_to_the_authored_threshold_yields_parity() {
    for event in [UNTIL, ALWAYS] {
        let original = world(event, "every scheduler", "<=", "0.5");
        let mut witness = evidence(&original);
        let equal = world(event, "every scheduler", "<=", "0.75");
        witness.item = qsl_replay::probabilistic_claim_identity(equal.claim("Claim")).unwrap();
        assert!(matches!(
            replay(
                &equal,
                &witness,
                &ExactProbLimits::default(),
                &Cancel::new()
            ),
            Ok(ProbabilityWitnessReplay::Parity)
        ));
    }
}

#[trace("TC-637", "FR-202-AC-22")]
#[test]
fn cancellation_inside_qualified_expansion_step_and_predicate_returns_no_token() {
    use qsl_replay::{
        replay_probabilistic_witness_observing, ExactProbabilityGraphError,
        ProbabilityWitnessSemanticPhase as Phase,
    };
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, OnceLock,
    };
    for event in [UNTIL, ALWAYS] {
        for scheduler in ["every scheduler", "Even"] {
            let mut fixture = model(event, scheduler, ">=", "0.5");
            // Workload rejection mass is 1/2, so use its strict interior bound.
            if scheduler == "Even" {
                fixture.body = fixture.body.replace(">= 0.5", ">= 0.6");
            }
            fixture.body.push_str(
                "\ninvariant Valid using v on M::Msg at current { self.attempts >= 0 }\n",
            );
            let world = World::new(&fixture);
            let witness = evidence(&world);
            let system = ModelSystem::new(world.subject().simulation).unwrap();
            for (entered, returned) in [
                (Phase::ExpansionEntered, Phase::ExpansionReturned),
                (Phase::PredicateEntered, Phase::PredicateReturned),
                (Phase::StepEntered, Phase::StepReturned),
            ] {
                let mut saw_entered = false;
                let mut saw_returned = false;
                let control = replay_probabilistic_witness_observing(
                    system.package(),
                    &system,
                    &witness.item,
                    &witness,
                    &ExactProbLimits::default(),
                    &Cancel::new(),
                    &mut |phase| {
                        saw_entered |= phase == entered;
                        saw_returned |= phase == returned;
                    },
                )
                .unwrap();
                assert!(matches!(control, ProbabilityWitnessReplay::Reproduced(_)));
                assert!(
                    saw_entered && saw_returned,
                    "uncancelled control traverses the same semantic call"
                );
                let armed = Arc::new(AtomicBool::new(false));
                let observing = Arc::clone(&armed);
                let slot = Arc::new(OnceLock::<Cancel>::new());
                let weak = Arc::downgrade(&slot);
                let cancel = Cancel::observing(move || {
                    if observing.load(Ordering::Relaxed) {
                        weak.upgrade()
                            .unwrap()
                            .get()
                            .unwrap()
                            .cancel(quire_exact::CancelCause::Deadline);
                    }
                });
                slot.set(cancel.clone()).unwrap();
                let mut completed = false;
                let result = replay_probabilistic_witness_observing(
                    system.package(),
                    &system,
                    &witness.item,
                    &witness,
                    &ExactProbLimits::default(),
                    &cancel,
                    &mut |phase| {
                        if phase == entered {
                            armed.store(true, Ordering::Relaxed);
                        }
                        completed |= phase == returned;
                    },
                );
                assert!(armed.load(Ordering::Relaxed));
                assert!(!completed, "{scheduler} {entered:?}: cancellation must trip inside the qualified call, before it returns uncancelled");
                assert!(matches!(
                    result,
                    Err(ProbabilityWitnessError::Graph(
                        ExactProbabilityGraphError::Cancelled(quire_exact::CancelCause::Deadline)
                    ))
                ));
            }
        }
    }
}
