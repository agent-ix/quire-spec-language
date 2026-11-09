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
        .replace("self.attempts < 2", "self.attempts = 0");
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
            .push(changed.paths[0].steps[0].clone());
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
        let mut limits = ExactProbLimits::default();
        limits.max_rational_bits = 1;
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
