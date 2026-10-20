// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-187 (TC-622): `actions`, `weighted_steps`, `NotMarkov`, the negative
//! reward refusal and the support graph, over the fixtures of
//! [`crate::simulation::sampled::world`].

use std::collections::{BTreeSet, VecDeque};

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::model::observation::AdmissionFailure;
use quire_exact::{Integer, Rational, Value};

use super::{NotMarkov, TransitionId, Unsteppable, WeightedStep};
use crate::simulation::model::ModelSimState;
use crate::simulation::sampled::world::{self, field, initial, World};
use crate::simulation::TransitionSystem;

/// The identity text of `system`'s operation `name`.
fn operation_text(system: &super::ModelSystem<'_>, name: &str) -> String {
    system
        .operations()
        .into_iter()
        .find(|(_, operation, _)| *operation == name)
        .map(|(_, _, text)| text.to_owned())
        .unwrap_or_else(|| panic!("no operation {name}"))
}

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

fn int(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

fn integer_of(value: &Value) -> i64 {
    match value {
        Value::Integer(integer) => integer.to_string().parse().expect("small"),
        other => panic!("not an integer: {other:?}"),
    }
}

fn boolean_of(value: &Value) -> bool {
    match value {
        Value::Boolean(flag) => *flag,
        other => panic!("not a Boolean: {other:?}"),
    }
}

fn steps(
    system: &super::ModelSystem<'_>,
    state: &ModelSimState,
    world: &World,
    workload: &str,
) -> Vec<WeightedStep> {
    system
        .weighted_steps(state, world.workload(workload))
        .expect("the state expands")
        .expect("the state is Markov")
}

fn sum(steps: &[WeightedStep]) -> Rational {
    steps
        .iter()
        .fold(Rational::from_integer(Integer::zero()), |total, step| {
            total.add(&step.probability)
        })
}

/// The one reward value of a step.
fn duration(step: &WeightedStep) -> i64 {
    let [(name, value)] = &step.rewards[..] else {
        panic!("one reward: {:?}", step.rewards);
    };
    assert_eq!(name.as_str(), "duration");
    integer_of(value)
}

/// The transition's argument texts.
fn arguments(step: &WeightedStep) -> Vec<String> {
    match &step.transition {
        TransitionId::Operation(transition) => transition.arguments().to_vec(),
        TransitionId::Stutter => panic!("a stutter step"),
    }
}

/// FR-187-AC-1 (TC-622 step 1): `Service` at `Busy, attempts = 0` under
/// `Steady` has six `attempt` steps whose probabilities are the products of
/// `d`'s and `outcome`'s, summing to 1, with `duration = d`; at `Idle` the
/// only step is `request`, probability 1, `duration` 0. Untagged
/// `FR-187-AC-1`: the fixture's `phase`, `d` and `outcome` stand in
/// integers and Booleans for ADR-024's enum and quantity types, which the
/// simulator cannot yet encode (see the world's module doc).
#[trace("TC-622")]
#[test]
fn service_steps_carry_the_products_of_their_draws() {
    let world = World::new(&world::service(""));
    let system = world.system();
    let idle = initial(&system);
    let [request] = <[_; 1]>::try_from(steps(&system, &idle, &world, "Steady")).expect("one step");
    assert_eq!(request.probability, ratio(1, 1));
    assert_eq!(duration(&request), 0);
    assert_eq!(integer_of(&field(&request.post, "phase")), 1);
    assert_eq!(integer_of(&field(&request.post, "attempts")), 0);

    let busy = steps(&system, &request.post, &world, "Steady");
    assert_eq!(busy.len(), 6);
    assert_eq!(sum(&busy), ratio(1, 1));
    let expected = [
        (1, true, ratio(441, 500)),
        (1, false, ratio(9, 500)),
        (3, true, ratio(343, 5000)),
        (3, false, ratio(7, 5000)),
        (12, true, ratio(147, 5000)),
        (12, false, ratio(3, 5000)),
    ];
    for (d, ok, probability) in expected {
        let texts = [
            crate::simulation::model::frames::text_of(&int(d)).expect("text"),
            crate::simulation::model::frames::text_of(&Value::Boolean(ok)).expect("text"),
        ];
        let step = busy
            .iter()
            .find(|step| arguments(step) == texts)
            .unwrap_or_else(|| panic!("a step with d = {d}, ok = {ok}"));
        assert_eq!(step.probability, probability, "d = {d}, ok = {ok}");
        assert_eq!(duration(step), d);
        let phase = integer_of(&field(&step.post, "phase"));
        assert_eq!(phase, if ok { 2 } else { 1 });
    }
}

/// FR-187-AC-2 (TC-622 step 2): `Link` under `Even` at its initial state
/// gives `send_a` delivered `9/20`, lost `1/20`, `send_b` delivered `2/5`,
/// lost `1/10`; `actions` gives two actions in canonical order whose draws
/// each sum to 1; at a delivered state the only step is the stutter step.
#[trace("TC-622", "FR-187-AC-2")]
#[test]
fn link_steps_weigh_operations_then_draws() {
    let world = World::new(&world::link());
    let system = world.system();
    let start = initial(&system);
    let weighted = steps(&system, &start, &world, "Even");
    assert_eq!(weighted.len(), 4);
    assert_eq!(sum(&weighted), ratio(1, 1));
    let probability = |operation: &str, lost: bool| {
        weighted
            .iter()
            .find(|step| {
                let TransitionId::Operation(transition) = &step.transition else {
                    return false;
                };
                transition.operation() == operation_text(&system, operation).as_str()
                    && boolean_of(&field(&step.post, "delivered")) != lost
            })
            .map(|step| step.probability.clone())
            .unwrap_or_else(|| panic!("{operation} lost = {lost}"))
    };
    assert_eq!(probability("send_a", false), ratio(9, 20));
    assert_eq!(probability("send_a", true), ratio(1, 20));
    assert_eq!(probability("send_b", false), ratio(2, 5));
    assert_eq!(probability("send_b", true), ratio(1, 10));

    let actions = system
        .actions(&start)
        .expect("the state expands")
        .expect("decided");
    assert_eq!(actions.len(), 2);
    let operations: Vec<String> = actions
        .iter()
        .map(|action| action.identity.operation.clone())
        .collect();
    let mut ordered = vec![
        operation_text(&system, "send_a"),
        operation_text(&system, "send_b"),
    ];
    ordered.sort_by_key(|text| {
        super::identity_bytes(text, &actions[0].identity.receiver, &[]).expect("bytes")
    });
    assert_eq!(operations, ordered);
    for action in &actions {
        assert!(action.identity.arguments.is_empty());
        assert_eq!(action.draws.len(), 2);
        let total = action
            .draws
            .iter()
            .fold(Rational::from_integer(Integer::zero()), |total, draw| {
                total.add(&draw.probability)
            });
        assert_eq!(total, ratio(1, 1));
        // Canonical order of the drawn vectors: `false` before `true`.
        let drawn: Vec<bool> = action
            .draws
            .iter()
            .map(|draw| boolean_of(&draw.random[0]))
            .collect();
        assert_eq!(drawn, [false, true]);
    }

    let delivered = weighted
        .iter()
        .find(|step| boolean_of(&field(&step.post, "delivered")))
        .expect("a delivering step");
    let [stutter] =
        <[_; 1]>::try_from(steps(&system, &delivered.post, &world, "Even")).expect("one step");
    assert_eq!(stutter.transition, TransitionId::Stutter);
    assert_eq!(stutter.probability, ratio(1, 1));
    assert!(stutter.rewards.is_empty());
}

/// FR-187-AC-3 (TC-622 step 3): a `Health` whose `tick` leaves `spare`
/// unconstrained is not Markov under `Steady`: `weighted_steps` names the
/// state key, `tick`, the first drawn `fault` (`false`) and 2 post-states,
/// and `actions` gives one action whose draws each carry two successors. A
/// `Health` with no post-state for `fault = true` gives `NotMarkov` with 0
/// post-states, and its action's `true` draw carries none.
#[trace("TC-622", "FR-187-AC-3")]
#[test]
fn health_variants_are_not_markov() {
    let world = World::new(&world::loose_health());
    let system = world.system();
    let start = initial(&system);
    let (_, digest) = system.state_key(&start).expect("a key");
    let not_markov = not_markov_of(
        system
            .weighted_steps(&start, world.workload("Steady"))
            .expect("the state expands"),
        "two post-states per draw",
    );
    assert_eq!(not_markov.state, digest);
    assert_eq!(
        not_markov.transition.operation,
        operation_text(&system, "tick")
    );
    assert!(not_markov.transition.arguments.is_empty());
    assert!(super::same_values(
        &not_markov.random,
        &[Value::Boolean(false)]
    ));
    assert_eq!(not_markov.post_states, 2);
    let [action] = <[_; 1]>::try_from(system.actions(&start).expect("expands").expect("decided"))
        .expect("one");
    assert_eq!(action.draws.len(), 2);
    assert!(action.draws.iter().all(|draw| draw.successors.len() == 2));

    let world = World::new(&world::blocked_health());
    let system = world.system();
    let start = initial(&system);
    let not_markov = not_markov_of(
        system
            .weighted_steps(&start, world.workload("Steady"))
            .expect("the state expands"),
        "no post-state for fault = true",
    );
    assert!(super::same_values(
        &not_markov.random,
        &[Value::Boolean(true)]
    ));
    assert_eq!(not_markov.post_states, 0);
    let [action] = <[_; 1]>::try_from(system.actions(&start).expect("expands").expect("decided"))
        .expect("one");
    let counts: Vec<(bool, usize)> = action
        .draws
        .iter()
        .map(|draw| (boolean_of(&draw.random[0]), draw.successors.len()))
        .collect();
    assert_eq!(counts, [(false, 1), (true, 0)]);
}

/// FR-187-AC-4 (TC-622 step 4): `refund = n - 3` over `n: Int[0, 5]`
/// refuses admission `invalid_probabilistic_model`/`negative-reward`,
/// naming the operation, the reward and `n = 0`.
#[trace("TC-622", "FR-187-AC-4")]
#[test]
fn a_negative_reward_refuses_admission() {
    let model = world::refund();
    let world = World::new(&model);
    let Err(AdmissionFailure::Refused(record)) = world.admit() else {
        panic!("admission refuses");
    };
    assert_eq!(
        (record.code, record.cause),
        (Code::InvalidProbabilisticModel, "negative-reward")
    );
    assert_eq!(
        record.fields.get("reward").map(String::as_str),
        Some("refund")
    );
    assert_eq!(
        record.fields.get("operation"),
        Some(&model.ident("Account/pay"))
    );
    assert_eq!(
        record.fields.get("arguments").map(String::as_str),
        Some("n = 0")
    );
}

/// FR-187-AC-4 (TC-622 step 4): over every state of `Service`, the
/// transitions `weighted_steps` gives positive probability under `Steady`
/// are exactly FR-120's successors (ADR-024 PM-6). Unlike FR-187-AC-1, this
/// clause does not depend on `Service`'s value types: PM-6 compares
/// transition sets, which the integer and Boolean stand-ins of the module
/// doc of [`world`] give the same shape as the enum and quantity ones.
#[trace("TC-622", "FR-187-AC-4")]
#[test]
fn the_support_graph_is_the_state_graph() {
    let world = World::new(&world::service(""));
    let system = world.system();
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::from([initial(&system)]);
    while let Some(state) = queue.pop_front() {
        let (key, _) = system.state_key(&state).expect("a key");
        if !seen.insert(key) {
            continue;
        }
        let successors: BTreeSet<(Vec<u8>, Vec<u8>)> = system
            .successors(&state)
            .expect("expands")
            .successors
            .iter()
            .map(|(transition, post)| {
                (
                    system.transition_key(transition).expect("bytes"),
                    system.state_key(post).expect("key").0,
                )
            })
            .collect();
        let weighted = steps(&system, &state, &world, "Steady");
        let positive: BTreeSet<(Vec<u8>, Vec<u8>)> = weighted
            .iter()
            .filter(|step| !step.probability.is_zero())
            .map(|step| {
                let TransitionId::Operation(transition) = &step.transition else {
                    panic!("Service has no terminal state");
                };
                (
                    system.transition_key(transition).expect("bytes"),
                    system.state_key(&step.post).expect("key").0,
                )
            })
            .collect();
        assert_eq!(positive, successors);
        assert_eq!(positive.len(), weighted.len());
        queue.extend(weighted.into_iter().map(|step| step.post));
    }
    // Idle at 0, 1 and 2 attempts (`reset` keeps them), Busy at 0 and 1,
    // and Done at 1 and 2.
    assert_eq!(seen.len(), 7);
}

/// FR-185-AC-7 (TC-620 step 7): `Service`, with `d` typed `Millis = Int[1,
/// 12]`, checks at S3 with its record intact and refuses at S4
/// `unsupported_construct`/`not-yet-implemented`, naming its first random
/// parameter in source order, `random-parameter` and the selected profile.
#[trace("TC-620", "FR-185-AC-7")]
#[test]
fn service_checks_and_refuses_emission_naming_its_first_random_parameter() {
    let world = World::new(&world::service(""));
    let model = world.package.graph().probabilistic_model();
    assert_eq!(model.random_parameters().len(), 2);
    assert_eq!(model.rewards().len(), 1);
    assert_eq!(model.workloads().len(), 1);
    let refusal = qsl_package::emit_checked(&world.package).expect_err("S4 refuses");
    assert_eq!(
        refusal,
        qsl_package::EmitRefusal::ProbabilisticModelNotEmitted {
            declaration: "attempt(d)".to_owned(),
            feature: qsl_semantics::check::ProbabilisticFeature::RandomParameter,
            profile: Some("quire.model.complete/v1"),
        }
    );
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::UnsupportedConstruct, Some("not-yet-implemented"))
    );
}

/// The [`NotMarkov`] of a `weighted_steps` result that is not steppable.
fn not_markov_of(result: Result<Vec<WeightedStep>, Unsteppable>, why: &str) -> NotMarkov {
    match result {
        Err(Unsteppable::NotMarkov(not_markov)) => not_markov,
        other => panic!("NotMarkov ({why}): {other:?}"),
    }
}

/// FR-187 Behavior: an enabled scheduled identity with no post-state for
/// any draw makes `weighted_steps` return `NotMarkov` with 0 post-states
/// for the first draw (`x = false`), not the stutter step of a terminal
/// state; `actions` leaves the identity out.
#[trace("TC-622")]
#[test]
fn an_identity_with_no_post_state_for_any_draw_is_not_markov() {
    let world = World::new(&world::gate("", "self.up = x and self.up = (not x)", ""));
    let system = world.system();
    let start = initial(&system);
    let (_, digest) = system.state_key(&start).expect("a key");
    let not_markov = not_markov_of(
        system
            .weighted_steps(&start, world.workload("Unit"))
            .expect("the state expands"),
        "no post-state for any draw",
    );
    assert_eq!(not_markov.state, digest);
    assert_eq!(
        not_markov.transition.operation,
        operation_text(&system, "step")
    );
    assert!(super::same_values(
        &not_markov.random,
        &[Value::Boolean(false)]
    ));
    assert_eq!(not_markov.post_states, 0);
    assert!(system
        .actions(&start)
        .expect("expands")
        .expect("decided")
        .is_empty());
}

/// FR-187 Behavior: the stutter step of a terminal state carries every
/// reward of the model at 0, here `cost` on a state where `step` is not
/// enabled.
#[trace("TC-622")]
#[test]
fn the_stutter_step_carries_every_reward_at_zero() {
    let world = World::new(&world::gate(
        "self.up",
        "self.up = x",
        "reward cost on Gate::step = 1;\n",
    ));
    let system = world.system();
    let start = initial(&system);
    let [stutter] = <[_; 1]>::try_from(steps(&system, &start, &world, "Unit")).expect("one");
    assert_eq!(stutter.transition, TransitionId::Stutter);
    assert_eq!(stutter.probability, ratio(1, 1));
    let [(name, value)] = &stutter.rewards[..] else {
        panic!("one reward: {:?}", stutter.rewards);
    };
    assert_eq!(name.as_str(), "cost");
    assert_eq!(integer_of(value), 0);
}
