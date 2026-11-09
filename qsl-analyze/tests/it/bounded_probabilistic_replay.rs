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
probabilistic Min using ev on M over (m: M::Msg) under every scheduler on origin
{ probability >= 0.8 [ eventually[0,2] holds(m.delivered) ] }
probabilistic Max using ev on M over (m: M::Msg) under every scheduler on origin
{ probability <= 0.9 [ eventually[0,2] holds(m.delivered) ] }
probabilistic WorkMin using ev on M over (m: M::Msg) under Even on origin
{ probability >= 0.875 [ eventually[0,2] holds(m.delivered) ] }
probabilistic WorkMax using ev on M over (m: M::Msg) under Even on origin
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
    for (initial, comparison, threshold) in [(true, "<=", "0.5"), (false, ">=", "0.5")] {
        let claim = format!("probabilistic Zero using ev on M over (m: M::Msg) under every scheduler on origin {{ probability {comparison} {threshold} [ eventually[0,0] holds(m.delivered) ] }}");
        let mut fixture = model(&claim);
        fixture.initial["delivered"] = serde_json::json!({"boolean":initial});
        let world = World::new(&fixture);
        let witness = evidence(&world, "Zero");
        assert_eq!(witness.paths.len(), 1);
        assert!(witness.paths[0].steps.is_empty());
        assert!(witness.scheduler.is_empty());
        assert_eq!(mass(&world, &witness), ratio(1, 1));
    }
    let mut fixture = model("probabilistic Leaf using ev on M over (m: M::Msg) under every scheduler on origin { probability >= 0.5 [ eventually[0,2] holds(m.delivered) ] }");
    fixture
        .body
        .push_str("terminal Stop using v on M when { true };\n");
    fixture.body = fixture.body.replace("not self.delivered", "false");
    let world = World::new(&fixture);
    let witness = evidence(&world, "Leaf");
    assert!(witness.paths[0].steps.is_empty());
    assert_eq!(mass(&world, &witness), ratio(1, 1));
}

#[trace("TC-637", "FR-202-AC-15")]
#[test]
fn insufficient_missing_and_equal_mass_are_parity_not_refutations() {
    let world = World::new(&model(CLAIMS));
    let mut witness = evidence(&world, "Max");
    witness.paths.truncate(1);
    assert!(matches!(
        replay(
            &world,
            &witness,
            &ExactProbLimits::default(),
            &Cancel::new()
        ),
        Ok(ProbabilityWitnessReplay::Parity) | Err(ProbabilityWitnessError::Refused)
    ));
    // Keep a consistent policy while removing every represented cylinder.
    witness.paths.clear();
    witness.scheduler.clear();
    assert!(matches!(
        replay(
            &world,
            &witness,
            &ExactProbLimits::default(),
            &Cancel::new()
        ),
        Ok(ProbabilityWitnessReplay::Parity)
    ));
    let min = evidence(&world, "Min");
    let equal_world = World::new(&model(&CLAIMS.replace(">= 0.8", ">= 0.75")));
    let mut equal = min;
    equal.item = qsl_replay::probabilistic_claim_identity(equal_world.claim("Min")).unwrap();
    // Keys include package and occurrence identity, so derive fresh keys by
    // replaying the same real semantic actions against the new checked item.
    rekey(&equal_world, "Min", &mut equal);
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

fn rekey(world: &World, name: &str, witness: &mut ProbabilisticCounterexample) {
    use qsl_eval::automaton::QualifiedBoundedMonitor;
    use qsl_semantics::check::{Event, ProbForm};
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let claim = world.claim(name);
    let binding = qsl_replay::claim_binding(&system, claim).unwrap();
    let reference = binding.as_ref().map(|binding| &binding.reference);
    let ProbForm::Probability {
        event: Event::Formula { formula, .. },
        ..
    } = &claim.form
    else {
        panic!("formula")
    };
    let context = qsl_replay::probabilistic_formula_monitor_context(claim, reference).unwrap();
    let mut monitor = QualifiedBoundedMonitor::new(formula.formula(), &context, 1000).unwrap();
    let predicate = formula.condition(monitor.atom().index()).unwrap();
    let read = |state| {
        matches!(
            system
                .state_predicates(state, &[(predicate, reference)])
                .unwrap()
                .first(),
            Some(qsl_eval::simulation::model::PredicateReading::True)
        )
    };
    let key = |state, control, monitor: &QualifiedBoundedMonitor| {
        ProbabilityProductKey::for_formula_state(
            reference,
            &system.state_key(state).unwrap().0,
            &monitor.key(control).unwrap(),
            ExactProbLimits::default().document,
            &Cancel::new(),
        )
        .unwrap()
    };
    witness.binding = reference.cloned();
    witness.scheduler.clear();
    let mut scheduler = std::collections::BTreeMap::new();
    for path in &mut witness.paths {
        let mut control = monitor.start(read(&witness.initial)).unwrap();
        let mut previous = key(&witness.initial, control, &monitor);
        witness.initial_key = previous.clone();
        for step in &mut path.steps {
            scheduler.insert(previous, step.action.clone());
            control = monitor.step(control, read(&step.post)).unwrap();
            step.post_key = key(&step.post, control, &monitor);
            previous = step.post_key.clone();
        }
    }
    witness.scheduler = scheduler.into_iter().collect();
}

#[trace("TC-637", "FR-202-AC-16")]
#[test]
fn cancellation_and_configured_native_limits_never_return_a_token() {
    use qsl_replay::{ExactProbabilityGraphError, ModelCheckLimit};
    use quire_exact::CancelCause;
    let world = World::new(&model(CLAIMS));
    let witness = evidence(&world, "Min");
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    assert!(matches!(
        replay(&world, &witness, &ExactProbLimits::default(), &cancel),
        Err(ProbabilityWitnessError::Graph(
            ExactProbabilityGraphError::Cancelled(CancelCause::Requested)
        ))
    ));
    let mut limits = ExactProbLimits::default();
    limits.model.limits.max_states = 0;
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
    let mut limits = ExactProbLimits::default();
    limits.document.storage_bytes = 0;
    assert!(matches!(
        replay(&world, &witness, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Limit {
            limit: ModelCheckLimit::CertificateMaxWorkingBytes,
            ..
        })
    ));
    let limits = ExactProbLimits {
        max_rational_bits: 1,
        ..ExactProbLimits::default()
    };
    assert!(matches!(
        replay(&world, &witness, &limits, &Cancel::new()),
        Err(ProbabilityWitnessError::Limit {
            limit: ModelCheckLimit::ExactProbMaxRationalBits,
            ..
        })
    ));
}

#[trace("TC-637", "FR-202-AC-16")]
#[test]
fn final_acceptance_cancellation_has_no_truth_or_evidence() {
    use quire_exact::CancelCause;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, OnceLock};
    let world = World::new(&model(CLAIMS));
    let witness = evidence(&world, "Min");
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
    let seen = Arc::clone(&count);
    let cancel = Cancel::observing(move || {
        if seen.fetch_add(1, Ordering::Relaxed) + 1 == final_poll {
            weak.upgrade()
                .unwrap()
                .get()
                .unwrap()
                .cancel(CancelCause::Requested);
        }
    });
    slot.set(cancel.clone()).unwrap();
    assert!(matches!(
        replay(&world, &witness, &ExactProbLimits::default(), &cancel),
        Err(ProbabilityWitnessError::Graph(
            qsl_replay::ExactProbabilityGraphError::Cancelled(CancelCause::Requested)
        ))
    ));
    assert_eq!(count.load(Ordering::Relaxed), final_poll);
}

#[trace("TC-637", "FR-202-AC-12", "FR-202-AC-15")]
#[test]
fn all_workload_first_hit_cylinders_have_exact_mass_fifty_five_over_sixty_four() {
    let world = World::new(&model(CLAIMS));
    let failures = evidence(&world, "WorkMin");
    assert_eq!(mass(&world, &failures), ratio(9, 64));
    assert_eq!(failures.paths.len(), 4);
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let mut accepting = failures.clone();
    accepting.item = qsl_replay::probabilistic_claim_identity(world.claim("WorkMax")).unwrap();
    accepting.paths.clear();
    for path in &failures.paths {
        for length in 0..path.steps.len() {
            let state = if length == 0 {
                &failures.initial
            } else {
                &path.steps[length - 1].post
            };
            let identity = &path.steps[length].action;
            let action = system
                .actions(state)
                .unwrap()
                .unwrap()
                .into_iter()
                .find(|action| &action.identity == identity)
                .unwrap();
            let draw = action
                .draws
                .into_iter()
                .find(|draw| matches!(draw.random.as_slice(), [Value::Boolean(false)]))
                .unwrap();
            let (transition, post) = draw.successors.into_iter().next().unwrap();
            let mut steps = path.steps[..length].to_vec();
            steps.push(qsl_replay::ProbabilityWitnessStep {
                action: identity.clone(),
                transition,
                draw: draw.random,
                post,
                post_key: failures.initial_key.clone(),
                elapsed: ratio(0, 1),
            });
            if !accepting.paths.iter().any(|previous| {
                previous.steps.len() == steps.len()
                    && previous.steps.iter().zip(&steps).all(|(a, b)| {
                        a.action == b.action
                            && a.transition == b.transition
                            && qsl_eval::simulation::model::same_values(&a.draw, &b.draw)
                    })
            }) {
                accepting
                    .paths
                    .push(qsl_replay::ProbabilityWitnessPath { steps });
            }
        }
    }
    rekey(&world, "WorkMax", &mut accepting);
    accepting.scheduler.clear();
    assert_eq!(accepting.paths.len(), 6);
    assert_eq!(mass(&world, &accepting), ratio(55, 64));
    let mut duplicated = accepting.clone();
    duplicated.paths.push(duplicated.paths[0].clone());
    assert!(matches!(
        replay(
            &world,
            &duplicated,
            &ExactProbLimits::default(),
            &Cancel::new()
        ),
        Ok(ProbabilityWitnessReplay::Parity)
    ));
}

#[trace("TC-637", "FR-202-AC-16")]
#[test]
fn adapter_exhaustion_preserves_incomplete_without_accepted_evidence() {
    use qsl_replay::IncompleteCause;
    let world = World::new(&model(CLAIMS));
    for budget in [
        "paths",
        "states",
        "transitions",
        "arithmetic",
        "work",
        "storage",
        "expansion",
    ] {
        let mut request = world.request("Min", ModelCheckLimits::default());
        match budget {
            "paths" => request.limits.max_witness_paths = 0,
            "states" => request.limits.model.limits.max_states = 0,
            "transitions" => request.limits.model.limits.max_transitions = 0,
            "arithmetic" => request.limits.max_rational_bits = 1,
            "work" => request.subject.simulation.meter.work_units = 0,
            "storage" => request.limits.document.storage_bytes = 0,
            "expansion" => request.subject.simulation.expansion.max_candidates = 0,
            _ => unreachable!(),
        }
        let record = analyze_items(
            &Engines::qsl().unwrap(),
            vec![exact_probabilistic_item(RequestIndex::new(0), request)],
            &Cancel::new(),
        )
        .remove(0);
        assert!(record.terminal().is_some(), "{budget}: {record:?}");
        assert!(
            matches!(
                record.terminal().unwrap().value(),
                TerminalValue::Incomplete(IncompleteCause::LimitReached { .. })
            ),
            "{budget}: {:?}",
            record.terminal().unwrap().value()
        );
        assert!(record.probabilistic_counterexample.is_none(), "{budget}");
        assert!(record.exact_probabilistic.is_none(), "{budget}");
    }
}

#[trace("TC-637", "FR-202-AC-14", "FR-202-AC-15")]
#[test]
fn a_decided_initial_path_cannot_continue_and_policy_keys_keep_horizons_distinct() {
    let world = World::new(&model(CLAIMS));
    let failure = evidence(&world, "Min");
    let zero_source = "probabilistic Zero using ev on M over (m: M::Msg) under every scheduler on origin { probability <= 0.5 [ eventually[0,0] holds(m.delivered) ] }";
    let mut fixture = model(zero_source);
    fixture.initial["delivered"] = serde_json::json!({"boolean":true});
    let zero = World::new(&fixture);
    let mut extended = evidence(&zero, "Zero");
    extended.paths[0]
        .steps
        .push(failure.paths[0].steps[0].clone());
    assert!(matches!(
        replay(
            &zero,
            &extended,
            &ExactProbLimits::default(),
            &Cancel::new()
        ),
        Ok(ProbabilityWitnessReplay::Parity)
    ));
    let source = CLAIMS.replace(">= 0.8", ">= 0.9");
    let varying = World::new(&model(&source));
    let mut witness = evidence(&varying, "Min");
    let system = ModelSystem::new(varying.subject().simulation).unwrap();
    let state = &witness.paths[0].steps[0].post;
    let action = system
        .actions(state)
        .unwrap()
        .unwrap()
        .into_iter()
        .find(|action| action.identity != witness.paths[0].steps[1].action)
        .unwrap();
    let draw = action
        .draws
        .into_iter()
        .find(|draw| matches!(draw.random.as_slice(), [Value::Boolean(true)]))
        .unwrap();
    let (transition, post) = draw.successors.into_iter().next().unwrap();
    let step = &mut witness.paths[0].steps[1];
    step.action = action.identity;
    step.draw = draw.random;
    step.transition = transition;
    step.post = post;
    rekey(&varying, "Min", &mut witness);
    assert_eq!(witness.scheduler.len(), 2);
    assert_ne!(witness.scheduler[0].1, witness.scheduler[1].1);
    assert_eq!(mass(&varying, &witness), ratio(1, 8));
}
