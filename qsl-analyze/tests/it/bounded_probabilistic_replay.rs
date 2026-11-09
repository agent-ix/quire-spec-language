// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-202 native finite-horizon producer and independent replay controls.
use crate::prob_world::{Model, Operation, World, BOOLEAN};
use ix_trace_rs::trace;
use qsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
use qsl_eval::simulation::model::ModelSystem;
use qsl_foundation::RequestIndex;
use qsl_replay::{
    replay_probabilistic_witness, ExactProbLimits, ModelCheckLimits, ProbabilisticCounterexample,
    ProbabilityProductKey, ProbabilityWitnessError, ProbabilityWitnessReplay, TerminalValue,
};
use quire_exact::{Cancel, Integer, Rational, Value};

const CLAIMS: &str = "
probabilistic Min using inf on M over (m: M::Msg) under every scheduler on origin
{ probability >= 0.8 [ eventually[0,2] holds(m.delivered) ] }
probabilistic Max using inf on M over (m: M::Msg) under every scheduler on origin
{ probability <= 0.9 [ eventually[0,2] holds(m.delivered) ] }
probabilistic WorkMin using inf on M over (m: M::Msg) under workload Even on origin
{ probability >= 0.875 [ eventually[0,2] holds(m.delivered) ] }
probabilistic WorkMax using inf on M over (m: M::Msg) under workload Even on origin
{ probability <= 0.8 [ eventually[0,2] holds(m.delivered) ] }
";
fn model(claims: &str) -> Model {
    let operation = |name| Operation {
        name,
        parameters: vec![("lost", BOOLEAN.to_owned())],
        modifies: vec!["delivered"],
        deletes: false,
    };
    Model {
        package: "test/bounded-replay",
        object: "Msg",
        population: "msgs",
        key: "m",
        integers: vec![],
        fields: vec![("delivered", BOOLEAN.to_owned())],
        operations: vec![operation("slow"), operation("fast")],
        body: format!(
            "\
pre Ready_slow using v on M::Msg::slow {{ not self.delivered }}
pre Ready_fast using v on M::Msg::fast {{ not self.delivered }}
post Sent_slow using v on M::Msg::slow {{ self.delivered = (not lost) }}
post Sent_fast using v on M::Msg::fast {{ self.delivered = (not lost) }}
random Msg::slow(lost) ~ {{ true: 1, false: 1 }};
random Msg::fast(lost) ~ {{ true: 1, false: 3 }};
workload Even on M {{ weight Msg::slow = 1; weight Msg::fast = 1; }}
{claims}"
        ),
        initial: serde_json::json!({"delivered": {"boolean": false}}),
    }
}
fn ratio(a: i64, b: i64) -> Rational {
    Rational::new(Integer::from(a), Integer::from(b)).unwrap()
}
fn evidence(world: &World, claim: &str) -> ProbabilisticCounterexample {
    let records = analyze_items(
        &Engines::qsl().unwrap(),
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request(claim, ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    );
    let record = records.first().unwrap();
    assert_eq!(record.terminal().unwrap().value(), &TerminalValue::Refuted);
    assert!(record.exact_probabilistic.is_none());
    record
        .probabilistic_counterexample
        .as_ref()
        .expect("accepted replay retains path evidence")
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
        ProbabilityWitnessReplay::Parity => panic!("expected reproduced bounded refutation"),
    }
}
fn no_token(world: &World, witness: &ProbabilisticCounterexample) {
    assert!(!matches!(
        replay(world, witness, &ExactProbLimits::default(), &Cancel::new()),
        Ok(ProbabilityWitnessReplay::Reproduced(_))
    ));
}
#[trace("TC-637", "FR-202-AC-12", "FR-202-AC-13")]
#[test]
fn cyclic_model_has_finite_debt_products_and_replays_both_bounds_and_workload() {
    let world = World::new(&model(CLAIMS));
    for (claim, expected) in [
        ("Min", ratio(1, 4)),
        ("Max", ratio(15, 16)),
        ("WorkMin", ratio(9, 64)),
    ] {
        let witness = evidence(&world, claim);
        assert_eq!(mass(&world, &witness), expected);
        assert_eq!(mass(&world, &witness), expected);
        if claim == "Min" {
            let path = &witness.paths[0];
            assert_eq!(path.steps.len(), 2);
            let system = ModelSystem::new(world.subject().simulation).unwrap();
            use qsl_eval::simulation::TransitionSystem;
            assert_eq!(
                system.state_key(&witness.initial).unwrap().0,
                system.state_key(&path.steps[0].post).unwrap().0
            );
            assert_ne!(witness.initial_key, path.steps[0].post_key);
            assert_eq!(witness.scheduler.len(), 2);
        }
    }
    let witness = evidence(&world, "WorkMax");
    assert!(witness.scheduler.is_empty());
    assert!(mass(&world, &witness) > ratio(4, 5));
}
#[trace("TC-637", "FR-202-AC-13", "FR-202-AC-15")]
#[test]
fn full_key_source_and_semantic_cylinder_mutations_never_reproduce() {
    let world = World::new(&model(CLAIMS));
    let original = evidence(&world, "Min");
    let mut changed = original.clone();
    changed.paths[0].steps[0].post_key = original.initial_key.clone();
    no_token(&world, &changed);
    let mut changed = original.clone();
    changed.initial_key = ProbabilityProductKey::from_canonical_bytes(b"null".to_vec());
    no_token(&world, &changed);
    let mut changed = original.clone();
    changed.binding = None;
    no_token(&world, &changed);
    let mut changed = original.clone();
    changed.paths[0].steps[0].draw = vec![Value::Boolean(false)];
    no_token(&world, &changed);
    let mut changed = original.clone();
    changed.paths[0].steps[0]
        .action
        .operation
        .push_str("-disabled");
    no_token(&world, &changed);
    let mut changed = original.clone();
    changed.paths.push(changed.paths[0].clone());
    no_token(&world, &changed);
    let mut changed = original.clone();
    changed.paths[0].steps.pop();
    no_token(&world, &changed);
    for source in [
        CLAIMS.replace(">= 0.8", ">= 0.7"),
        CLAIMS.replace("[0,2]", "[0,3]"),
        CLAIMS.replace("probabilistic Min ", "probabilistic Renamed "),
    ] {
        let other = World::new(&model(&source));
        no_token(&other, &original);
    }
}
#[trace("TC-637", "FR-202-AC-14")]
#[test]
fn zero_horizon_and_terminal_false_extension_need_no_model_steps() {
    for (initial, comparison, threshold) in [(true, "<=", "0"), (false, ">=", "1")] {
        let claim = format!("probabilistic Zero using inf on M over (m: M::Msg) under every scheduler on origin {{ probability {comparison} {threshold} [ eventually[0,0] holds(m.delivered) ] }}");
        let mut fixture = model(&claim);
        fixture.initial["delivered"] = serde_json::json!({"boolean":initial});
        let world = World::new(&fixture);
        let witness = evidence(&world, "Zero");
        assert_eq!(witness.paths.len(), 1);
        assert!(witness.paths[0].steps.is_empty());
        assert!(witness.scheduler.is_empty());
        assert_eq!(mass(&world, &witness), ratio(1, 1));
    }
    let mut fixture = model("probabilistic Leaf using inf on M over (m: M::Msg) under every scheduler on origin { probability >= 0.5 [ eventually[0,2] holds(m.delivered) ] }");
    fixture
        .body
        .push_str("terminal Stop using v on M when { true };\n");
    let world = World::new(&fixture);
    let witness = evidence(&world, "Leaf");
    assert!(witness.paths[0].steps.is_empty());
    assert_eq!(mass(&world, &witness), ratio(1, 1));
}
