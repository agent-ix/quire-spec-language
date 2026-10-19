// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126's liveness phase over ADR-018 §6's ConfigVersion units and
//! FR-126-AC-4's `Health` units (`crate::units`), the subjects the spec's
//! acceptance criteria name.
//!
//! The example unit is the shipped unit of `examples/config-version` with
//! the post clause `Cycles` in place of `VersionUnchanged`, universe `{a,
//! b}`, both versions 0. The `holds` atoms are read through the test's own
//! letters. FR-126-AC-1's `Holds` row and AC-4's `Holds` row wait on FR-339's
//! component certificate, so AC-1 and AC-4 are not tagged; their `Violated`
//! rows are verified here.
//!
//! The spec expects AC-1's no-granularity lasso "bound to `c = b`". The two
//! instances are symmetric, the engine takes them in order, so it binds
//! `c = a` and loops on `b`; these tests assert the symmetric shape and leave
//! which instance to the planner's ruling on the cross-instance order.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{
    check_model, LivenessRefusal, ModelCheckRefusal, ModelCheckRequest,
};
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::TransitionSystem;
use qsl_replay::{
    IncompleteCause, ModelCheckItem, ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome,
    ReachedLimit, TemporalCounterexample,
};
use quire_exact::{Cancel, CancelCause, ScalarLimits};

use crate::counter::{document, UNLIMITED, ZERO};
use crate::liveness::{counter, is_zero};
use crate::units::{clause, config_object, health, shipped_unit_text, unit, Atoms, Domain, World};

const FORMULA: &str = "always eventually holds(c.versionNumber = 2)";
const WHOLE: &str = "fair weak Config::ConfigVersion::attemptUpdate;";
const EACH: &str = "fair weak each Config::ConfigVersion::attemptUpdate;";
const OVER: &str = "c: Config::ConfigVersion";

/// The example unit's post clause: the version advances modulo 3 (the unit
/// reaches versions 0 to 2 only, so the wrap is spelled out).
const CYCLES: &str = "post Cycles using v on Config::ConfigVersion::attemptUpdate { \
     (pre(self.versionNumber) < 2 and self.versionNumber = pre(self.versionNumber) + 1) \
     or (pre(self.versionNumber) = 2 and self.versionNumber = 0) }\n";

/// The ConfigVersion world: the example unit when `shipped` is false, the
/// shipped unit of `examples/config-version` when true, with `fairness`,
/// universe `{a, b}`.
fn config(shipped: bool, fairness: &str) -> World {
    let domain = Domain::config_version();
    let clause = clause(OVER, fairness, FORMULA);
    let text = if shipped {
        let header = "profile v = \"quire.model.complete/v1\";\n";
        let shipped = shipped_unit_text(&domain.digest_hex());
        assert!(shipped.contains(header), "the shipped unit's profile line");
        let shipped = shipped.replace(
            header,
            &format!("{header}profile t0 = \"quire.temporal.infinite-trace/v1\";\n"),
        );
        format!("{shipped}{clause}")
    } else {
        unit(&domain, "Config", &format!("{CYCLES}{clause}"))
    };
    World::new(
        domain,
        &text,
        &[("a", config_object(0)), ("b", config_object(0))],
    )
}

fn reaches_two(world: &World) -> Atoms<'_> {
    Atoms::new(
        world,
        vec![Box::new(move |state, object| {
            world.integer(state, object, "versionNumber") == 2
        })],
    )
}

type Outcome = Result<ModelCheckOutcome, ModelCheckRefusal>;

fn run(
    world: &World,
    letters: &Atoms<'_>,
    max_depth: Option<usize>,
    limits: ModelCheckLimits,
    cancel: &Cancel,
) -> Outcome {
    check_model(
        ModelCheckRequest {
            subject: world.subject(UNLIMITED),
            item: ModelCheckItem::Clause {
                clause: &world.clauses[0],
                letters,
            },
            max_depth,
            limits,
        },
        cancel,
    )
}

fn decide(world: &World) -> Outcome {
    run(
        world,
        &reaches_two(world),
        None,
        ModelCheckLimits::default(),
        &Cancel::new(),
    )
}

fn violated(outcome: &Outcome) -> &TemporalCounterexample {
    match outcome {
        Ok(ModelCheckOutcome::Violated { counterexample, .. }) => counterexample,
        other => panic!("expected a violation, got {other:?}"),
    }
}

/// The model states a counterexample's steps reach (each step's post-state
/// is the successor of its transition whose digest it records).
fn positions(world: &World, counterexample: &TemporalCounterexample) -> Vec<ModelSimState> {
    let system = ModelSystem::new(world.unlimited().simulation).expect("the subject admits");
    let mut current = system.initial()[counterexample.initial_state].clone();
    let mut states = vec![current.clone()];
    for step in &counterexample.prefix {
        current = system
            .successors(&current)
            .expect("the state expands")
            .successors
            .into_iter()
            .find(|(transition, state)| {
                *transition == step.transition
                    && system.state_key(state).expect("encodes").1 == step.post_state
            })
            .map(|(_, state)| state)
            .expect("the recorded step is a successor");
        states.push(current.clone());
    }
    states
}

/// Which object a step's receiver is: the receiver text carries the object
/// id as hex (`a` is 61, `b` is 62).
fn receiver(step: &qsl_replay::ModelStep) -> char {
    let text = step.transition.receiver().expect("a receiver");
    if text.contains("\"identity\":\"61\"") {
        'a'
    } else if text.contains("\"identity\":\"62\"") {
        'b'
    } else {
        panic!("an unknown receiver: {text}")
    }
}

/// The counts of a run refused for want of the component certificate.
fn explored(outcome: &Outcome) -> qsl_replay::RunStats {
    match outcome {
        Err(ModelCheckRefusal::Liveness(LivenessRefusal::ComponentCertificateNotBuilt {
            stats,
        })) => *stats,
        other => panic!("expected the missing certificate: {other:?}"),
    }
}

/// ADR-018 §6 "Product": for `c = b` over the example unit the reachable
/// product has 15 states, 9 with the waiting automaton state and 6 with the
/// accepting one (`vb` in `{0, 1}`). `fair weak each` leaves no fair
/// accepting cycle, so the run is refused for want of the component
/// certificate (FR-339, not built) after exploring that product; both
/// instances together explore it twice. This asserts the interim refusal, so
/// it carries no AC tag.
#[test]
fn the_example_unit_has_a_fifteen_state_product_per_instance() {
    let world = config(false, EACH);
    assert_eq!(explored(&decide(&world)).product_states, 30);
}

/// FR-126-AC-1, the `Violated` row with no granularity (`Verifies` TC-521
/// step 1, not the `Holds` row, so AC-1 stays untagged): the weak constraint
/// on `attemptUpdate` as a whole is met by either receiver, so the lasso has
/// an empty stem and a loop of three steps of one receiver, which keeps the
/// bound object's version at 0 while the other cycles `0, 1, 2`.
#[trace("TC-521")]
#[test]
fn without_granularity_the_loop_is_three_steps_of_the_other_object() {
    let world = config(false, WHOLE);
    let outcome = decide(&world);
    let counterexample = violated(&outcome);
    let lasso = counterexample.lasso.expect("a lasso");
    assert_eq!((lasso.entry, lasso.terminal_stutter), (0, false));
    assert_eq!(counterexample.len(), 3);
    let looping = receiver(&counterexample.prefix[0]);
    assert!(counterexample
        .prefix
        .iter()
        .all(|step| receiver(step) == looping));
    let bound = counterexample
        .over_binding
        .as_ref()
        .expect("a bound object")
        .key
        .clone();
    assert_ne!(
        bound,
        looping.to_string(),
        "the loop moves the other object"
    );
    let states = positions(&world, counterexample);
    assert_eq!(states.len(), 4);
    let bound_versions: Vec<i64> = states
        .iter()
        .map(|state| world.integer(state, &bound, "versionNumber"))
        .collect();
    assert_eq!(bound_versions, [0, 0, 0, 0]);
    let moved: Vec<i64> = states
        .iter()
        .map(|state| world.integer(state, &looping.to_string(), "versionNumber"))
        .collect();
    assert_eq!(moved, [0, 1, 2, 0]);
}

/// FR-126-AC-1, the `Violated` row over the shipped unit (`Verifies` TC-521
/// step 1): every transition is a self-loop, so under `fair weak each` the
/// loop must take both `upd(a)` and `upd(b)`: `(0,0) -upd(a)-> (0,0)
/// -upd(b)-> (0,0)`, with an empty stem.
#[trace("TC-521")]
#[test]
fn over_the_shipped_unit_each_needs_both_updates_in_the_loop() {
    let world = config(true, EACH);
    let outcome = decide(&world);
    let counterexample = violated(&outcome);
    let lasso = counterexample.lasso.expect("a lasso");
    assert_eq!((lasso.entry, lasso.terminal_stutter), (0, false));
    let receivers: Vec<char> = counterexample.prefix.iter().map(receiver).collect();
    assert_eq!(receivers, ['a', 'b']);
    for state in positions(&world, counterexample) {
        assert_eq!(world.integer(&state, "a", "versionNumber"), 0);
        assert_eq!(world.integer(&state, "b", "versionNumber"), 0);
    }
}

/// FR-126-AC-5: over the example subject and the TP-4 claim with no
/// granularity, `max_depth` 2 returns `BoundReached{depth: 2}` (the length-3
/// loop needs an edge out of depth 2); `max_depth` 3 returns `Violated` with
/// the AC-1 loop, found by the second phase over the retained graph although
/// depth 3 left a frontier; `max_states` 2 and `max_transitions` 3 stop the
/// run naming the limit; an already-cancelled handle returns `Stopped{
/// Cancelled, None}`; and over the `Counter` subject an evaluation meter
/// budget of zero stops the run at the `inc` precondition.
#[trace("TC-521", "FR-126-AC-5")]
#[test]
fn the_horizon_and_every_limit_stop_the_example_run_naming_what_stopped_it() {
    let world = config(false, WHOLE);
    let letters = reaches_two(&world);
    let defaults = ModelCheckLimits::default();
    let with = |max_depth, limits: ModelCheckLimits, cancel: &Cancel| {
        run(&world, &letters, max_depth, limits, cancel).expect("checked")
    };
    let live = Cancel::new();

    assert!(matches!(
        with(Some(2), defaults, &live),
        ModelCheckOutcome::BoundReached { depth: 2, .. }
    ));
    let found = Ok(with(Some(3), defaults, &live));
    let counterexample = violated(&found);
    assert_eq!(counterexample.len(), 3);
    assert_eq!(counterexample.lasso.map(|lasso| lasso.entry), Some(0));

    let stopped = |outcome: ModelCheckOutcome| match outcome {
        ModelCheckOutcome::Stopped { cause, limit, .. } => (cause, limit),
        other => panic!("expected a stopped run, got {other:?}"),
    };
    let reached = |limit, value| {
        (
            IncompleteCause::ResourceExhausted,
            Some(ReachedLimit { limit, value }),
        )
    };
    let mut states = defaults;
    states.limits.max_states = 2;
    assert_eq!(
        stopped(with(None, states, &live)),
        reached(ModelCheckLimit::MaxStates, 2)
    );
    let mut transitions = defaults;
    transitions.limits.max_transitions = 3;
    assert_eq!(
        stopped(with(None, transitions, &live)),
        reached(ModelCheckLimit::MaxTransitions, 3)
    );
    let cancelled = Cancel::new();
    cancelled.cancel(CancelCause::Requested);
    assert_eq!(
        stopped(with(None, defaults, &cancelled)),
        (
            IncompleteCause::Cancelled {
                source: CancelCause::Requested
            },
            None
        )
    );

    let meter: ScalarLimits = ZERO;
    let metered = counter(document(false), "", "always eventually holds(c.value = 0)")
        .run(is_zero, None, defaults, meter, &Cancel::new())
        .expect("checked");
    assert_eq!(
        stopped(metered),
        reached(ModelCheckLimit::EvaluationMeter, 0)
    );
}

/// FR-126-AC-7's determinism, over AC-1's two requests (the `Holds` request
/// still ends in the certificate refusal, so equal means the same refusal
/// with the same counts): two runs of each give equal outcomes and
/// byte-equal counterexamples.
#[trace("TC-521")]
#[test]
fn the_example_requests_give_the_same_answer_twice() {
    let each = config(false, EACH);
    let whole = config(false, WHOLE);
    let refusal = |world: &World| format!("{:?}", decide(world));
    assert_eq!(refusal(&each), refusal(&each));
    let first = decide(&whole);
    let second = decide(&whole);
    assert_eq!(violated(&first), violated(&second));
}

/// FR-126-AC-4's `Restless` row: over `Health` (object `s`; `healthy: Bool`,
/// `failures: Int[0, 1]`; `fail` with precondition `self.failures = 0`
/// setting `healthy` false and `failures` 1; `recover` with precondition
/// `not self.healthy` setting `healthy` true; `terminal any`; `healthy` true
/// and `failures` 0 initially) the recovery-stability formula holds in the
/// spec's base variant, and in the `Restless` variant, where `fail` has no
/// precondition and can repeat, it is violated by a lasso on which `healthy`
/// never holds at three consecutive positions. Only the `Restless` row is
/// verified, since a `Holds` waits on FR-339's component certificate.
#[trace("TC-521")]
#[test]
fn a_restless_system_has_a_lasso_that_never_recovers_for_three_positions() {
    const FORMULA: &str =
        "always (holds(not s.healthy) implies eventually always[0,2] holds(s.healthy))";
    let contract = |fail_pre: &str| {
        format!(
            "{fail_pre}\
             post Fail using v on H::Health::fail {{ not self.healthy and self.failures = 1 }}\n\
             pre CanRecover using v on H::Health::recover {{ not self.healthy }}\n\
             post Recover using v on H::Health::recover {{ self.healthy }}\n\
             terminal Halt using v on H any;\n"
        )
    };
    let restless = contract("");
    let domain = health();
    let text = unit(
        &domain,
        "H",
        &format!("{restless}{}", clause("s: H::Health", "", FORMULA)),
    );
    let world = World::new(
        domain,
        &text,
        &[(
            "s",
            serde_json::json!({"healthy": {"boolean": true},
                "failures": {"integer": "0"}}),
        )],
    );
    let letters = Atoms::new(
        &world,
        vec![
            Box::new(|state, object| !world.boolean(state, object, "healthy")),
            Box::new(|state, object| world.boolean(state, object, "healthy")),
        ],
    );
    let outcome = run(
        &world,
        &letters,
        None,
        ModelCheckLimits::default(),
        &Cancel::new(),
    );
    let counterexample = violated(&outcome);
    let lasso = counterexample.lasso.expect("a lasso");
    assert!(!lasso.terminal_stutter);
    let states = positions(&world, counterexample);
    let healthy: Vec<bool> = states
        .iter()
        .map(|state| world.boolean(state, "s", "healthy"))
        .collect();
    // Unroll the loop (it re-enters at `entry`) over enough positions.
    let loop_part = &healthy[lasso.entry..healthy.len() - 1];
    let mut unrolled: Vec<bool> = healthy[..lasso.entry].to_vec();
    for _ in 0..4 {
        unrolled.extend_from_slice(loop_part);
    }
    assert!(
        unrolled
            .windows(3)
            .all(|window| window.iter().any(|healthy| !healthy)),
        "healthy never holds at three consecutive positions: {unrolled:?}"
    );
    assert!(
        unrolled.iter().any(|healthy| !healthy),
        "the claim's antecedent holds on the lasso"
    );
}
