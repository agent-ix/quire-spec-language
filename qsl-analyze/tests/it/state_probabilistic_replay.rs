// SPDX-License-Identifier: AGPL-3.0-or-later
//! Direct infinite-trace state-event witnesses use independent native replay.
use crate::prob_world::{self, Operation, World};
use ix_trace_rs::trace;
use qsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
use qsl_eval::simulation::model::ModelSystem;
use qsl_foundation::RequestIndex;
use qsl_replay::{
    replay_probabilistic_witness, ExactProbLimits, ModelCheckLimits, ProbabilisticCounterexample,
    ProbabilityWitnessError, ProbabilityWitnessPath, ProbabilityWitnessReplay, TerminalValue,
};
use quire_exact::{Cancel, Integer, Rational};

fn ratio(a: i64, b: i64) -> Rational {
    Rational::new(Integer::from(a), Integer::from(b)).unwrap()
}
fn world(event: &str, scheduler: &str, operator: &str, threshold: &str) -> World {
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
    World::new(&model)
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
