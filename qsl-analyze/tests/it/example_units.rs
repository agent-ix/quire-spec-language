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
//! Both instances give a lasso of length 3 under AC-1's no-granularity claim,
//! and a tie goes to the earlier binding, so the counterexample is bound to
//! `c = a` and loops on `b`. AC-10 to AC-15 (the cross-binding ranking) are
//! verified here with letters that can read an atom undefined.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{
    check_model, LivenessRefusal, ModelCheckRefusal, ModelCheckRequest,
};
use qsl_eval::model_subject::{ModelInstance, ModelLetters};
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::temporal::{AtomReading, UndefinedCause};
use qsl_replay::{
    CounterexampleKind, IncompleteCause, ModelCheckItem, ModelCheckLimit, ModelCheckLimits,
    ModelCheckOutcome, ReachedLimit, TemporalCounterexample, UndefinedLocus,
};
use qsl_semantics::check::FormulaNodeId;
use quire_exact::{Cancel, CancelCause, Incomplete, Meter, ScalarLimits};

use crate::counter::{document, UNLIMITED, ZERO};
use crate::liveness::{counter, is_zero};
use crate::units::{clause, config_object, health, shipped_unit_text, unit, Atoms, Domain, World};

pub(crate) const FORMULA: &str = "always eventually holds(c.versionNumber = 2)";
pub(crate) const WHOLE: &str = "fair weak Config::ConfigVersion::attemptUpdate;";
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
    config_over(shipped, fairness, FORMULA, (0, 0))
}

/// [`config`] with the claim `formula` and the initial versions of `a` and
/// `b`.
pub(crate) fn config_over(
    shipped: bool,
    fairness: &str,
    formula: &str,
    versions: (i64, i64),
) -> World {
    let domain = Domain::config_version();
    let clause = clause(OVER, fairness, formula);
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
        &[
            ("a", config_object(versions.0)),
            ("b", config_object(versions.1)),
        ],
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
    letters: &dyn ModelLetters,
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
            reductions: Default::default(),
        },
        cancel,
    )
}

pub(crate) fn decide(world: &World) -> Outcome {
    run(
        world,
        &reaches_two(world),
        None,
        ModelCheckLimits::default(),
        &Cancel::new(),
    )
}

pub(crate) fn violated(outcome: &Outcome) -> &TemporalCounterexample {
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
pub(crate) fn receiver(step: &qsl_replay::ModelStep) -> char {
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
/// bound object's version at 0 while the other cycles `0, 1, 2`. Both
/// bindings give length 3, so it is bound to `a`, the first.
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
    assert_eq!(bound, "a", "both bindings give length 3: the earlier wins");
    assert_eq!(looping, 'b', "the loop moves the other object");
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

/// The example world's letters, each atom of the clause (in clause-node
/// order) a function of the bound object's version: `Some(truth)`, or `None`
/// for an undefined division by zero.
pub(crate) struct Versions<'w> {
    world: &'w World,
    atoms: Vec<FormulaNodeId>,
    reads: Vec<fn(i64) -> Option<bool>>,
}

impl<'w> Versions<'w> {
    pub(crate) fn new(world: &'w World, reads: Vec<fn(i64) -> Option<bool>>) -> Self {
        let atoms = qsl_eval::automaton::translate(&world.clauses[0], 1 << 20)
            .expect("the clause translates")
            .atoms()
            .to_vec();
        assert_eq!(atoms.len(), reads.len(), "one read per atom");
        Self {
            world,
            atoms,
            reads,
        }
    }
}

impl ModelLetters for Versions<'_> {
    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        let index = self
            .atoms
            .iter()
            .position(|known| *known == atom)
            .expect("the clause's own atom");
        let version = self.world.integer(state, &instance.key, "versionNumber");
        Ok(match (self.reads[index])(version) {
            Some(true) => AtomReading::True,
            Some(false) => AtomReading::False,
            None => AtomReading::Undefined(UndefinedCause::Kernel(
                quire_exact::Undefined::DivisionByZero,
            )),
        })
    }
}

fn is_zero_version(version: i64) -> Option<bool> {
    Some(version == 0)
}

/// `6 mod (2 - version) >= 0`: undefined at version 2.
fn mod_is_defined(version: i64) -> Option<bool> {
    (version != 2).then_some(true)
}

/// The key of the object a counterexample is bound to.
fn bound(counterexample: &TemporalCounterexample) -> &str {
    &counterexample
        .over_binding
        .as_ref()
        .expect("a bound object")
        .key
}

/// The position of an undefined-evaluation counterexample.
fn undefined_at(counterexample: &TemporalCounterexample) -> (u64, UndefinedLocus) {
    let CounterexampleKind::UndefinedEvaluation(found) = &counterexample.kind else {
        panic!(
            "expected an undefined evaluation: {:?}",
            counterexample.kind
        );
    };
    (found.site.position, found.site.locus.clone())
}

fn with_limits(world: &World, letters: &dyn ModelLetters, limits: ModelCheckLimits) -> Outcome {
    run(world, letters, None, limits, &Cancel::new())
}

/// FR-126-AC-10: with `a` at version 2 and `b` at 0, a weak constraint with
/// no granularity gives `b` the empty stem and the loop `(2,0) -upd(a)->
/// (0,0) -upd(a)-> (1,0) -upd(a)-> (2,0)`, length 3, though `a` is the first
/// binding: `a`'s counterexample has a stem of one transition and a loop of
/// three, length 4. The shortest lasso across the bindings wins.
#[trace("TC-521", "FR-126-AC-10")]
#[test]
fn the_shortest_lasso_across_bindings_wins_not_the_first_binding() {
    let world = config_over(false, WHOLE, FORMULA, (2, 0));
    let outcome = decide(&world);
    let counterexample = violated(&outcome);
    assert_eq!(bound(counterexample), "b");
    assert_eq!(counterexample.len(), 3);
    let lasso = counterexample.lasso.expect("a lasso");
    assert_eq!((lasso.entry, lasso.terminal_stutter), (0, false));
    assert!(counterexample
        .prefix
        .iter()
        .all(|step| receiver(step) == 'a'));
    let versions: Vec<(i64, i64)> = positions(&world, counterexample)
        .iter()
        .map(|state| {
            (
                world.integer(state, "a", "versionNumber"),
                world.integer(state, "b", "versionNumber"),
            )
        })
        .collect();
    assert_eq!(versions, [(2, 0), (0, 0), (1, 0), (2, 0)]);
}

/// FR-126-AC-11, the safety engine's ranking by depth, then binding: `always
/// holds(6 mod (2 - c.versionNumber) >= 0)` over both versions 0 is violated
/// bound to `a` by `(0,0) -upd(a)-> (1,0) -upd(a)-> (2,0)` undefined at
/// position 2 (both bindings are undefined at depth 2 and `a` is the
/// earlier); with `a` at 0 and `b` at 1 it is bound to `b` by `(0,1)
/// -upd(b)-> (0,2)`, undefined at position 1.
#[trace("TC-521", "FR-126-AC-11")]
#[test]
fn a_safety_claim_is_bound_to_the_binding_undefined_at_the_shallowest_depth() {
    let formula = "always holds(6 mod (2 - c.versionNumber) >= 0)";
    let both = config_over(false, EACH, formula, (0, 0));
    let outcome = with_limits(
        &both,
        &Versions::new(&both, vec![mod_is_defined]),
        ModelCheckLimits::default(),
    );
    let counterexample = violated(&outcome);
    assert_eq!(bound(counterexample), "a");
    assert_eq!(
        counterexample
            .prefix
            .iter()
            .map(receiver)
            .collect::<Vec<_>>(),
        ['a', 'a']
    );
    assert_eq!(undefined_at(counterexample).0, 2);

    let apart = config_over(false, EACH, formula, (0, 1));
    let outcome = with_limits(
        &apart,
        &Versions::new(&apart, vec![mod_is_defined]),
        ModelCheckLimits::default(),
    );
    let counterexample = violated(&outcome);
    assert_eq!(bound(counterexample), "b");
    assert_eq!(
        counterexample
            .prefix
            .iter()
            .map(receiver)
            .collect::<Vec<_>>(),
        ['b']
    );
    assert_eq!(undefined_at(counterexample).0, 1);
}

const DEAD_AT_ZERO: &str =
    "eventually (holds(c.versionNumber = 0) and holds(6 mod (2 - c.versionNumber) >= 0))";

fn dead_at_zero(world: &World) -> Versions<'_> {
    Versions::new(world, vec![is_zero_version, mod_is_defined])
}

/// FR-126-AC-12: over both versions 0, `eventually (holds(c.versionNumber =
/// 0) and holds(6 mod (2 - c.versionNumber) >= 0))` negates to an automaton
/// with no move at position 0, so the model-only pass finds the undefined
/// letter at position 2: bound to `a`, prefix `(0,0) -upd(a)-> (1,0)
/// -upd(a)-> (2,0)`, and no product state for the dead positions. With
/// `max_states` 1 the pass's second state exhausts the budget.
#[trace("TC-1085", "FR-126-AC-12")]
#[test]
fn the_model_only_pass_finds_the_undefined_letter_a_dead_automaton_leaves() {
    let world = config_over(false, WHOLE, DEAD_AT_ZERO, (0, 0));
    let letters = dead_at_zero(&world);
    let outcome = with_limits(&world, &letters, ModelCheckLimits::default());
    let Ok(ModelCheckOutcome::Violated {
        counterexample,
        stats,
    }) = &outcome
    else {
        panic!("expected a violation, got {outcome:?}");
    };
    assert_eq!(bound(counterexample), "a");
    assert_eq!(undefined_at(counterexample).0, 2);
    assert_eq!(
        counterexample
            .prefix
            .iter()
            .map(receiver)
            .collect::<Vec<_>>(),
        ['a', 'a']
    );
    assert_eq!(
        stats.product_states, 0,
        "no product state for a dead position"
    );

    let mut states = ModelCheckLimits::default();
    states.limits.max_states = 1;
    let outcome = with_limits(&world, &letters, states);
    assert!(
        matches!(
            outcome,
            Ok(ModelCheckOutcome::Stopped {
                cause: IncompleteCause::ResourceExhausted,
                limit: Some(ReachedLimit {
                    limit: ModelCheckLimit::MaxStates,
                    value: 1
                }),
                ..
            })
        ),
        "{outcome:?}"
    );
}

/// FR-126-AC-14: with `a` at 0 and `b` at 1, the AC-12 claim is bound to
/// `b`: `(0,1) -upd(b)-> (0,2)`, undefined at position 1, since `b`'s
/// product reaches the undefined letter at depth 1 and `a`'s model-only pass
/// at depth 2.
#[trace("TC-1085", "FR-126-AC-14")]
#[test]
fn a_later_binding_wins_with_a_strictly_shallower_finding() {
    let world = config_over(false, WHOLE, DEAD_AT_ZERO, (0, 1));
    let outcome = with_limits(&world, &dead_at_zero(&world), ModelCheckLimits::default());
    let counterexample = violated(&outcome);
    assert_eq!(bound(counterexample), "b");
    assert_eq!(
        counterexample
            .prefix
            .iter()
            .map(receiver)
            .collect::<Vec<_>>(),
        ['b']
    );
    assert_eq!(undefined_at(counterexample).0, 1);
}

/// FR-126-AC-15: with `a` at 0 and `b` at 1, `a`'s first phase ends at depth
/// 2, so `b` is explored through depth 1 alone: with `max_states` 6 `b` has
/// begun (it has a product state) and stops before it completes depth 1, and
/// that `Stopped` is returned; 1,000 returns AC-14's finding for `b`, the
/// prefix `(0,1) -upd(b)-> (0,2)` at position 1. `a`'s own pass stops the run
/// only below 5 states. With both versions 0, `b` finds nothing shallower than
/// depth 2 and the tie goes to `a`.
#[trace("TC-1085", "FR-126-AC-15")]
#[test]
fn a_later_binding_is_explored_through_one_less_than_the_finding() {
    let world = config_over(false, WHOLE, DEAD_AT_ZERO, (0, 1));
    let letters = dead_at_zero(&world);
    let mut limits = ModelCheckLimits::default();
    let stopped_in = |limits: ModelCheckLimits| match with_limits(&world, &letters, limits) {
        Ok(ModelCheckOutcome::Stopped {
            cause: IncompleteCause::ResourceExhausted,
            limit:
                Some(ReachedLimit {
                    limit: ModelCheckLimit::MaxStates,
                    value,
                }),
            stats,
        }) => (value, stats.product_states),
        other => panic!("expected a stop on max_states, got {other:?}"),
    };
    limits.limits.max_states = 4;
    assert_eq!(
        stopped_in(limits),
        (4, 0),
        "`a`'s pass stops: `b` never starts"
    );
    limits.limits.max_states = 6;
    let (value, b_states) = stopped_in(limits);
    assert_eq!(value, 6);
    assert!(b_states > 0, "`b` began and stopped before depth 1");
    limits.limits.max_states = 1_000;
    let outcome = with_limits(&world, &letters, limits);
    let counterexample = violated(&outcome);
    assert_eq!(bound(counterexample), "b");
    assert_eq!(
        counterexample
            .prefix
            .iter()
            .map(receiver)
            .collect::<Vec<_>>(),
        ['b']
    );
    assert_eq!(undefined_at(counterexample).0, 1);

    let tied = config_over(false, WHOLE, DEAD_AT_ZERO, (0, 0));
    let outcome = with_limits(&tied, &dead_at_zero(&tied), limits);
    let counterexample = violated(&outcome);
    assert_eq!(bound(counterexample), "a");
    assert_eq!(undefined_at(counterexample).0, 2);
}

/// The AC-15 world's letters, with `on_b` run when the clause is read for
/// `b`: the later binding's exploration is interfered with.
struct Interfering<'w> {
    inner: &'w Versions<'w>,
    on_b: Box<dyn Fn() -> Option<Incomplete> + 'w>,
}

impl ModelLetters for Interfering<'_> {
    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        if instance.key == "b" {
            if let Some(stop) = (self.on_b)() {
                return Err(stop);
            }
        }
        self.inner.atom(atom, instance, state, meter)
    }
}

/// FR-126 "Bindings and ranking": a later binding that stops on any cause
/// before it completes depth `d - 1` answers with its own `Stopped`, never
/// the earlier binding's finding. `max_transitions` and the evaluation meter
/// and cancellation each stop `b` in AC-15's world (`a` at 0, `b` at 1).
#[trace("TC-1085")]
#[test]
fn a_later_binding_that_stops_on_any_cause_answers_with_its_stop() {
    let world = config_over(false, WHOLE, DEAD_AT_ZERO, (0, 1));
    let inner = dead_at_zero(&world);

    let mut limits = ModelCheckLimits::default();
    limits.limits.max_transitions = 4;
    let outcome = with_limits(&world, &inner, limits);
    assert!(
        matches!(
            outcome,
            Ok(ModelCheckOutcome::Stopped {
                limit: Some(ReachedLimit {
                    limit: ModelCheckLimit::MaxTransitions,
                    value: 4
                }),
                ..
            })
        ),
        "{outcome:?}"
    );
    limits.limits.max_transitions = 5;
    assert_eq!(bound(violated(&with_limits(&world, &inner, limits))), "b");

    let metered = Interfering {
        inner: &inner,
        on_b: Box::new(|| {
            let mut spent = Meter::new(ZERO);
            spent
                .charge(quire_exact::Charge::new(
                    quire_exact::ChargePoint::GraphExpand,
                ))
                .err()
        }),
    };
    let outcome = with_limits(&world, &metered, ModelCheckLimits::default());
    assert!(
        matches!(
            outcome,
            Ok(ModelCheckOutcome::Stopped {
                limit: Some(ReachedLimit {
                    limit: ModelCheckLimit::EvaluationMeter,
                    ..
                }),
                ..
            })
        ),
        "{outcome:?}"
    );

    let cancel = Cancel::new();
    let cancelling = Interfering {
        inner: &inner,
        on_b: Box::new(|| {
            cancel.cancel(CancelCause::Requested);
            None
        }),
    };
    let outcome = run(
        &world,
        &cancelling,
        None,
        ModelCheckLimits::default(),
        &cancel,
    );
    assert!(
        matches!(
            outcome,
            Ok(ModelCheckOutcome::Stopped {
                cause: IncompleteCause::Cancelled {
                    source: CancelCause::Requested
                },
                limit: None,
                ..
            })
        ),
        "{outcome:?}"
    );
}

/// FR-126-AC-1's claim with an atom undefined at version 2
/// (`always eventually holds(6 mod (2 - c.versionNumber) >= 0)`): a binding
/// whose exploration stops while it holds a finding does not hide a shallower
/// one of a later binding. With `a` at 0 and `b` at 1 or 2, a budget too small
/// to finish `b` stops the run, and once it suffices the answer is `b`'s
/// shallower finding, never `a`'s deeper one.
#[trace("TC-1085")]
#[test]
fn a_binding_that_stops_with_a_finding_does_not_hide_a_shallower_one() {
    const CLAIM: &str = "always eventually holds(6 mod (2 - c.versionNumber) >= 0)";
    for (versions, enough, position) in [((0, 1), 8, 1), ((0, 2), 6, 0)] {
        let world = config_over(false, WHOLE, CLAIM, versions);
        let letters = Versions::new(&world, vec![mod_is_defined]);
        for max_states in 1..=enough {
            let mut limits = ModelCheckLimits::default();
            limits.limits.max_states = max_states;
            let outcome = with_limits(&world, &letters, limits);
            if max_states < enough {
                assert!(
                    matches!(outcome, Ok(ModelCheckOutcome::Stopped { .. })),
                    "{versions:?} max_states {max_states}: {outcome:?}"
                );
            } else {
                let counterexample = violated(&outcome);
                assert_eq!(bound(counterexample), "b", "{versions:?}");
                assert_eq!(undefined_at(counterexample).0, position, "{versions:?}");
            }
        }
    }
    let world = config_over(false, WHOLE, CLAIM, (0, 1));
    let letters = Versions::new(&world, vec![mod_is_defined]);
    for max_transitions in 1..=6 {
        let mut limits = ModelCheckLimits::default();
        limits.limits.max_transitions = max_transitions;
        let outcome = with_limits(&world, &letters, limits);
        assert_eq!(
            matches!(outcome, Ok(ModelCheckOutcome::Stopped { .. })),
            max_transitions < 6,
            "max_transitions {max_transitions}: {outcome:?}"
        );
    }
}

/// Letters over the joint state of the two objects: `read(key, atom, va, vb)`.
struct Joint<'w> {
    world: &'w World,
    atoms: Vec<FormulaNodeId>,
    read: fn(&str, usize, i64, i64) -> Option<bool>,
    /// Run when `b`'s second atom is read at the joint state (1, 2).
    on_trigger: Box<dyn Fn() + 'w>,
}

impl ModelLetters for Joint<'_> {
    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        let index = self
            .atoms
            .iter()
            .position(|known| *known == atom)
            .expect("atom");
        let va = self.world.integer(state, "a", "versionNumber");
        let vb = self.world.integer(state, "b", "versionNumber");
        if instance.key == "b" && (va, vb) == (1, 2) && index == 1 {
            (self.on_trigger)();
        }
        Ok(match (self.read)(&instance.key, index, va, vb) {
            Some(true) => AtomReading::True,
            Some(false) => AtomReading::False,
            None => AtomReading::Undefined(UndefinedCause::Kernel(
                quire_exact::Undefined::DivisionByZero,
            )),
        })
    }
}

fn hidden_pass(key: &str, atom: usize, va: i64, vb: i64) -> Option<bool> {
    if key == "a" {
        return Some(atom == 1);
    }
    match (atom, va, vb) {
        (0, 1, 0) | (0, 0, 2) => Some(true),
        (0, _, _) => Some(false),
        (1, 2, 0) | (1, 2, 1) => None,
        (1, _, _) => Some(true),
        _ => unreachable!(),
    }
}

/// A binding that stops while holding a product finding at depth `d` has not
/// run its model-only pass, so a shallower finding from a dead root may be
/// hidden. Unlimited, `b` is undefined at position 2 (found by the pass);
/// cancelling after `b`'s depth-3 product finding exists must answer
/// `Stopped{Cancelled}`, not that depth-3 finding.
#[trace("TC-1085")]
#[test]
fn a_stopped_binding_does_not_return_a_finding_its_own_pass_may_beat() {
    let world = config_over(false, WHOLE, DEAD_AT_ZERO, (0, 0));
    let atoms = qsl_eval::automaton::translate(&world.clauses[0], 1 << 20)
        .expect("translates")
        .atoms()
        .to_vec();
    let plain = Joint {
        world: &world,
        atoms: atoms.clone(),
        read: hidden_pass,
        on_trigger: Box::new(|| {}),
    };
    let outcome = with_limits(&world, &plain, ModelCheckLimits::default());
    let counterexample = violated(&outcome);
    assert_eq!(bound(counterexample), "b");
    assert_eq!(undefined_at(counterexample).0, 2);

    let cancel = Cancel::new();
    let cancelling = Joint {
        world: &world,
        atoms,
        read: hidden_pass,
        on_trigger: Box::new(|| cancel.cancel(CancelCause::Requested)),
    };
    let outcome = run(
        &world,
        &cancelling,
        None,
        ModelCheckLimits::default(),
        &cancel,
    );
    assert!(
        matches!(
            outcome,
            Ok(ModelCheckOutcome::Stopped {
                cause: IncompleteCause::Cancelled { .. },
                ..
            })
        ),
        "{outcome:?}"
    );
}
