// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-176 (TC-601): EN-1 checks a universal hyperproperty over the
//! self-composition product, on ADR-023 section 8.1's vault (`crate::hyper_world`).
//!
//! Every expected verdict below is derived by hand from the vault's two
//! transition relations (leaky `l = i xor h`, secure `l = i`; `h` never
//! changes) and from the product's definition (ADR-023 HC-1). Each
//! counterexample is also read back by an oracle written here that
//! re-executes its traces through the vault's `ModelSystem` and reads only the
//! model's successor relation.
//!
//! The leaky product has 10 reachable states here, not the 14 that
//! the first criterion of FR-176 and ADR-023 section 8.1 count. Those 14 assume an automaton
//! for the negated body whose `q0` moves to `q1` on a differing letter and
//! also stays; the automaton `qsl-eval` translates (ADR-018 SM-6, shared with
//! the liveness phase and the certificate checkers) guards that stay on the
//! letter, so it is deterministic here and an unequal-secret pair has three
//! product states, not five. The accepting components and the secure product's
//! 8 states agree with the ADR; that criterion is therefore not claimed below.
//!
//! The clauses here that are symmetric in their copies are stored as the
//! quotient by the swap (FR-174, `crate::hyper_swap`), so the counts the engine
//! reports are the quotient's; `World::explore` reads the unreduced product.

use ix_trace_rs::trace;
use qsl_eval::hyper::{HyperLetters, MatchReading, ParameterBinding, StepLabel, StepMatch};
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::temporal::{AtomReading, UndefinedCause};
use qsl_replay::{
    HyperCounterexample, HyperCounterexampleKind, HyperLocus, HyperOutcome, HyperStep,
    IncompleteCause, ModelCheckLimit, ModelCheckLimits, ReachedLimit, UndecidedCause,
};
use qsl_semantics::check::FormulaNodeId;
use quire_exact::{Cancel, CancelCause, Charge, ChargePoint, Incomplete, Meter, Undefined};

use crate::counter::ZERO;
use crate::hyper_world::{
    high, low, low_of, non_interference, Vault, World, EQUAL_INPUTS, KEY, STARTS,
};

/// The engine's outcome of `world` under the default limits.
fn run(world: &World) -> HyperOutcome {
    world
        .run(ModelCheckLimits::default(), &Cancel::new())
        .expect("the run is not refused")
}

/// The counterexample of a violated run.
pub(crate) fn refuted(outcome: HyperOutcome) -> HyperCounterexample {
    match outcome {
        HyperOutcome::Violated { counterexample, .. } => *counterexample,
        other => panic!("expected a violation, got {other:?}"),
    }
}

/// The inputs `step(i)` a trace's steps take, by `i`; the steps must all be
/// model steps of `step`.
pub(crate) fn inputs(counterexample: &HyperCounterexample, trace: usize) -> Vec<i64> {
    counterexample.traces[trace]
        .steps
        .iter()
        .map(|step| match step {
            HyperStep::Model(step) => {
                let [argument] = step.transition.arguments() else {
                    panic!("step takes one input");
                };
                argument
                    .split("\"value\":\"")
                    .nth(1)
                    .and_then(|rest| rest.split('"').next())
                    .expect("an integer argument")
                    .parse()
                    .expect("a small input")
            }
            HyperStep::TerminalStutter => panic!("the vault has no terminal state"),
        })
        .collect()
}

/// The `(h, l)` of each position of trace `trace`, read by re-executing its
/// steps through the vault from its initial state (the oracle).
pub(crate) fn positions(
    world: &World,
    counterexample: &HyperCounterexample,
    trace: usize,
) -> Vec<(i64, i64)> {
    let system = ModelSystem::new(world.subject().simulation).expect("the subject admits");
    let trace = &counterexample.traces[trace];
    let mut state = system.initial()[trace.initial as usize].clone();
    let mut seen = vec![(high(&state), low(&state))];
    for step in &trace.steps {
        let HyperStep::Model(step) = step else {
            panic!("the vault has no terminal state");
        };
        state = system
            .successors(&state)
            .expect("the vault expands")
            .successors
            .into_iter()
            .find(|(transition, next)| {
                *transition == step.transition
                    && system.state_key(next).expect("a key").1 == step.post_state
            })
            .unwrap_or_else(|| panic!("{step:?} is not a step of the vault"))
            .1;
        seen.push((high(&state), low(&state)));
    }
    seen
}

/// ADR-023 section 8.1: `NonInterference` over the leaky vault is refuted by a
/// `Lockstep` counterexample of the section's shape, `a` from `(0, 0)` and
/// `b` from `(1, 0)`, joint steps `(step(0), step(0))`, loop entry 1, whose
/// outputs differ at joint position 1; the unreduced product holds two
/// accepting components, one for each unequal pair of secrets, each reached
/// through the differing outputs. The clause is symmetric in `a` and `b`, so
/// the engine stores the quotient by the swap (FR-174): the same verdict over
/// fewer states.
#[trace("TC-601")]
#[test]
fn the_leaky_vault_is_refuted_by_the_lockstep_counterexample_of_section_8_1() {
    let world = World::new(
        Vault::Leaky,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let outcome = run(&world);
    let HyperOutcome::Violated { stats, .. } = &outcome else {
        panic!("expected a violation, got {outcome:?}");
    };
    // Unreduced: 4 equal-secret states and 3 for each of the 2 unequal pairs.
    assert_eq!(world.explore().graph.states().len(), 10);
    assert_eq!(world.explore().accepting_components(), 2);
    // The swap fixes the 4 states whose components are equal and pairs the 6
    // others.
    assert_eq!(stats.product_states, 7);
    let counterexample = refuted(outcome);
    assert_eq!(
        counterexample.kind,
        HyperCounterexampleKind::Lockstep { loop_entry: 1 }
    );
    assert_eq!(counterexample.instance, [("v".to_owned(), KEY.to_owned())]);
    assert_eq!(counterexample.trace_position, 1);
    let names: Vec<_> = counterexample
        .traces
        .iter()
        .map(|trace| (trace.variable.as_str(), trace.alias.as_str(), trace.initial))
        .collect();
    assert_eq!(names, [("a", "V", 0), ("b", "V", 1)]);
    // Stem one step and a loop of one: two steps, each `step(0)` on both.
    assert_eq!(inputs(&counterexample, 0), [0, 0]);
    assert_eq!(inputs(&counterexample, 1), [0, 0]);
    // The oracle: `a` keeps `l = 0`, `b` takes `l = 1` and keeps it, and the
    // loop closes on the state at position 1; the first difference is joint
    // position 1.
    let a = positions(&world, &counterexample, 0);
    let b = positions(&world, &counterexample, 1);
    assert_eq!(a, [(0, 0), (0, 0), (0, 0)]);
    assert_eq!(b, [(1, 0), (1, 1), (1, 1)]);
    let first_difference = a.iter().zip(&b).position(|(a, b)| a.1 != b.1);
    assert_eq!(first_difference, Some(1));
    assert_eq!(a[1], a[2], "the loop closes at its entry");
    assert_eq!(b[1], b[2], "the loop closes at its entry");
}

/// ADR-023 section 8.1: over the secure vault every joint step gives
/// `l_a = l_b = i`, the product has 8 states (the 4 pairs of secrets times
/// the equal `l` pairs `(0, 0)` and `(1, 1)`, all in the one automaton
/// state), and the item holds; the engine stores the 6 states of the quotient
/// by the swap of `a` and `b` (FR-174).
#[trace("TC-601")]
#[test]
fn the_secure_vault_holds_over_eight_product_states() {
    let world = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let HyperOutcome::HoldsReduced { stats, .. } = run(&world) else {
        panic!("expected the secure vault to hold");
    };
    assert_eq!(world.explore().graph.states().len(), 8);
    assert_eq!(stats.product_states, 6);
    assert_eq!(world.explore().accepting_components(), 0);
}

/// Criterion 2 of FR-176, not claimed: `NonInterference` with its `match`
/// block removed is refuted over the secure vault, which pairs `step(0)` with `step(1)`; and a `match`
/// that no pair of runs satisfies, `b` stuttering where the vault has no
/// terminal state, is `Undecided(VacuousMatch)`, never `Holds`. The AC writes
/// the first conjunct `a.step.op = V::Vault::step`, which S3 refuses as
/// `MatchReadsState` (FR-172 has no operation-constant operand yet), so the
/// vacuous clause uses `a.step.op != stutter`, which is equivalent over this
/// vault; the criterion is not claimed.
#[trace("TC-601")]
#[test]
fn without_match_the_secure_vault_is_refuted_and_an_unsatisfiable_match_is_vacuous() {
    let unmatched = World::new(Vault::Secure, false, &STARTS, &non_interference(""));
    let counterexample = refuted(run(&unmatched));
    assert!(matches!(
        counterexample.kind,
        HyperCounterexampleKind::Lockstep { .. }
    ));
    // The inputs differ at some joint step: the oracle's `l` differ.
    let a = positions(&unmatched, &counterexample, 0);
    let b = positions(&unmatched, &counterexample, 1);
    assert!(a.iter().zip(&b).any(|(a, b)| a.1 != b.1));

    let vacuous = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &non_interference("match { a.step.op != stutter and b.step.op = stutter }"),
    );
    let HyperOutcome::Undecided { cause, .. } = run(&vacuous) else {
        panic!("expected the vacuous match to decide nothing");
    };
    assert_eq!(cause, UndecidedCause::VacuousMatch);
}

/// FR-176-AC-3, first sentence: the secure vault with a `reset` operation and
/// the one-variable clause `always eventually holds(v.l @ a = 0)`. Under
/// `fair weak V::Vault::reset;` on `a` it holds, since `reset` is enabled
/// at every state and must be taken infinitely often; with the empty fairness
/// set it is refuted by a loop of `step(1)` steps, on which `l` stays 1.
#[trace("TC-601", "FR-176-AC-3")]
#[test]
fn a_variable_s_own_fairness_set_decides_a_liveness_clause() {
    let clause = |fairness: &str| {
        format!(
            "hyper Returns using inf over behaviours (v: V::Vault) \
             forall trace a of V {fairness} {{ always eventually holds(v.l @ a = 0) }}"
        )
    };
    let fair = World::new(
        Vault::Secure,
        true,
        &STARTS[..1],
        &clause("fair weak V::Vault::reset;"),
    );
    assert!(matches!(run(&fair), HyperOutcome::Holds { .. }));

    let unfair = World::new(Vault::Secure, true, &STARTS[..1], &clause(""));
    let counterexample = refuted(run(&unfair));
    let HyperCounterexampleKind::Lockstep { loop_entry } = counterexample.kind else {
        panic!("expected a lockstep counterexample");
    };
    let loop_entry = usize::try_from(loop_entry).expect("a small position");
    let a = positions(&unfair, &counterexample, 0);
    // Every step of the loop is `step(1)`, and `l` is 1 throughout it.
    let steps = &counterexample.traces[0].steps;
    assert!(steps.len() > loop_entry, "the loop has a step");
    for step in &steps[loop_entry..] {
        let HyperStep::Model(step) = step else {
            panic!("the vault has no terminal state");
        };
        assert!(step.transition.operation().contains("/Vault/step\""));
        assert!(step.transition.arguments()[0].contains("\"value\":\"1\""));
    }
    assert!(a[loop_entry..].iter().all(|(_, l)| *l == 1));
    assert_eq!(a[loop_entry], *a.last().expect("a position"));
}

/// ADR-023 HC-4 (FR-176 "Phases"): a constraint of variable `k` is read on
/// component `k` alone. The body speaks of `a` only, so the same weak
/// `reset` constraint holds the item when it is `a`'s and does nothing when it
/// is `b`'s: `a` may loop on `step(1)` while `b` takes `reset`.
#[trace("TC-601")]
#[test]
fn a_fairness_constraint_is_read_on_its_own_variable_only() {
    let clause = |on_a: &str, on_b: &str| {
        format!(
            "hyper Returns using inf over behaviours (v: V::Vault) \
             forall trace a of V {on_a} forall trace b of V {on_b} \
             {{ always eventually holds(v.l @ a = 0) }}"
        )
    };
    let reset = "fair weak V::Vault::reset;";
    let on_a = World::new(Vault::Secure, true, &STARTS[..1], &clause(reset, ""));
    assert!(matches!(run(&on_a), HyperOutcome::Holds { .. }));

    let on_b = World::new(Vault::Secure, true, &STARTS[..1], &clause("", reset));
    let counterexample = refuted(run(&on_b));
    let HyperCounterexampleKind::Lockstep { loop_entry } = counterexample.kind else {
        panic!("expected a lockstep counterexample");
    };
    let loop_entry = usize::try_from(loop_entry).expect("a small position");
    let operations = |trace: usize| -> Vec<bool> {
        counterexample.traces[trace].steps[loop_entry..]
            .iter()
            .map(|step| match step {
                HyperStep::Model(step) => step.transition.operation().contains("/Vault/reset\""),
                HyperStep::TerminalStutter => panic!("the vault has no terminal state"),
            })
            .collect()
    };
    // `b` is fair: it takes `reset` in its loop. `a` is not: it never does.
    assert!(operations(1).contains(&true), "b's loop takes reset");
    assert!(!operations(0).contains(&true), "a's loop never takes reset");
}

/// FR-176-AC-3, last sentence: in a two-variable clause over a subject where
/// `a` can reach a terminal state, `μ` `a.step.op = b.step.op` pairs `a`'s
/// stutter step only with `b`'s stutter step. The `Terminal` vault's `step`
/// needs `l = 0`, so `step(1)` ends in a terminal state.
#[trace("TC-601", "FR-176-AC-3")]
#[test]
fn a_stutter_step_is_paired_only_with_a_stutter_step() {
    let world = World::new(
        Vault::Terminal,
        false,
        &STARTS[..1],
        "hyper Same using inf over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V match { a.step.op = b.step.op } \
         { always holds(v.l @ a = v.l @ b) }",
    );
    let explored = world.explore();
    let mut stutters = 0;
    for edge in explored.graph.edges() {
        use qsl_eval::hyper::ComponentStep::Stutter;
        let [a, b] = &edge.transition.0[..] else {
            panic!("a pair step");
        };
        assert_eq!(
            matches!(a, Stutter),
            matches!(b, Stutter),
            "a stutter step is paired with a stutter step only: {edge:?}"
        );
        stutters += usize::from(matches!(a, Stutter));
    }
    assert!(stutters > 0, "a terminal pair stutters");
    // The matched product has no joint step that mixes the two: it is the
    // diagonal of the vault's graph, so both always read the same `l`.
    assert!(matches!(run(&world), HyperOutcome::HoldsReduced { .. }));
}

/// FR-176-AC-4, first sentence: a subject that alone exceeds `max_states`
/// stops the item `Stopped(ResourceExhausted, MaxStates)` before the product
/// is built: the vault has 4 reachable states and the product none counted.
#[trace("TC-601", "FR-176-AC-4")]
#[test]
fn a_subject_that_alone_exceeds_max_states_stops_before_the_product_is_built() {
    let world = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 3;
    let outcome = world
        .run(limits, &Cancel::new())
        .expect("the run is not refused");
    assert_eq!(
        outcome,
        HyperOutcome::Stopped {
            cause: IncompleteCause::ResourceExhausted,
            limit: Some(ReachedLimit {
                limit: ModelCheckLimit::MaxStates,
                value: 3
            }),
            count: None,
            stats: qsl_replay::RunStats::default(),
        }
    );
}

/// FR-176-AC-4, second sentence: `NonInterference` over the secure vault with
/// the added `match` conjunct `(if present(a.step.Vault::step.i) then 1 /
/// value(a.step.Vault::step.i) else 1) = 1`, which has no value on a joint
/// step whose `a` step is `step(0)`, is refuted by an `Undefined`
/// counterexample that ends at the first such joint step in canonical order,
/// with a path for `a` and one for `b` and the cause `division-by-zero`.
#[trace("TC-601", "FR-176-AC-4")]
#[test]
fn an_undefined_match_ends_the_item_at_the_first_such_joint_step() {
    let divided = "match { a.step.op = b.step.op and \
         a.step.Vault::step.i = b.step.Vault::step.i and \
         (if present(a.step.Vault::step.i) then 1 / value(a.step.Vault::step.i) else 1) = 1 }";
    let world = World::new(Vault::Secure, false, &STARTS, &non_interference(divided));
    let counterexample = refuted(run(&world));
    let HyperCounterexampleKind::Undefined(undefined) = &counterexample.kind else {
        panic!("expected an undefined-evaluation counterexample");
    };
    assert_eq!(
        undefined.cause,
        UndefinedCause::Kernel(Undefined::DivisionByZero)
    );
    assert_eq!(undefined.locus, HyperLocus::Match { conjunct: 2 });
    // One path per trace variable, each the single step into joint position 1,
    // `step(0)` on both: the first joint step out of the first initial pair.
    assert_eq!(counterexample.trace_position, 1);
    assert_eq!(undefined.position, 1);
    assert_eq!(counterexample.traces.len(), 2);
    assert_eq!(inputs(&counterexample, 0), [0]);
    assert_eq!(inputs(&counterexample, 1), [0]);
    let variables: Vec<_> = undefined
        .variables
        .iter()
        .map(|(variable, alias)| (variable.as_str(), alias.as_str()))
        .collect();
    assert_eq!(variables, [("a", "V"), ("b", "V")]);
    // The named states are the two post-states, and the paths end in them.
    assert_eq!(undefined.state_keys.len(), 2);
    for (index, expected) in undefined.state_keys.iter().enumerate() {
        let HyperStep::Model(step) = &counterexample.traces[index].steps[0] else {
            panic!("a model step");
        };
        assert_eq!(&step.post_state, expected);
    }
    // The first initial pair in canonical order is at depth 0, so no deeper
    // finding precedes it.
    assert_eq!(counterexample.steps(), 1);
}

/// FR-176-AC-4, last sentence: running the leaky request twice gives
/// byte-equal counterexamples. A `HyperCounterexample` has no wire encoding
/// yet (QSpec FR-400's writer is not built), so equality is `PartialEq` over
/// the payload and over its `Debug` rendering, which prints every member.
#[trace("TC-601", "FR-176-AC-4")]
#[test]
fn the_leaky_request_run_twice_gives_equal_counterexamples() {
    let world = World::new(
        Vault::Leaky,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let first = refuted(run(&world));
    let second = refuted(run(&world));
    assert_eq!(first, second);
    assert_eq!(format!("{first:?}"), format!("{second:?}"));
}

/// ADR-023 HM-7: a subject with no initial state decides nothing
/// (`NoInitialState`), and a clause with an object parameter whose universe
/// is empty has no instance (`NoInstance`), before any state is explored.
#[trace("TC-601")]
#[test]
fn a_subject_without_an_initial_state_or_an_instance_decides_nothing() {
    let empty_start = World::new(Vault::Secure, false, &[], &non_interference(EQUAL_INPUTS));
    let HyperOutcome::Undecided { cause, stats } = run(&empty_start) else {
        panic!("expected no decision");
    };
    assert_eq!(cause, UndecidedCause::NoInitialState);
    assert_eq!(stats.product_states, 0);

    let mut empty_universe = World::started(
        Vault::Secure,
        false,
        &[None],
        &non_interference(EQUAL_INPUTS),
    );
    empty_universe.universe = Vec::new();
    let HyperOutcome::Undecided { cause, stats } = run(&empty_universe) else {
        panic!("expected no decision");
    };
    assert_eq!(cause, UndecidedCause::NoInstance);
    assert_eq!(stats.product_states, 0);
}

/// A `match` that refuses: the reading of every joint step is `Refused`.
struct Refusing;

impl StepMatch for Refusing {
    fn universal(
        &self,
        _instance: &[ParameterBinding],
        _steps: &[StepLabel<'_>],
        _meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        Ok(MatchReading::Refused)
    }

    fn existential(
        &self,
        _instance: &[ParameterBinding],
        _steps: &[StepLabel<'_>],
        _meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        panic!("an HP-2 clause has no existential conjunct")
    }
}

/// A `match` whose evaluation exhausts its meter: it charges a meter that
/// admits no work.
struct Exhausted;

impl StepMatch for Exhausted {
    fn universal(
        &self,
        _instance: &[ParameterBinding],
        _steps: &[StepLabel<'_>],
        _meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        Meter::new(ZERO).charge(Charge::new(ChargePoint::GraphExpand))?;
        Ok(MatchReading::Holds)
    }

    fn existential(
        &self,
        _instance: &[ParameterBinding],
        _steps: &[StepLabel<'_>],
        _meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        panic!("an HP-2 clause has no existential conjunct")
    }
}

/// FR-176 "Product": a `μ_U` that is refused or incomplete at a reachable
/// joint step is `Undecided(MatchUndetermined)`, not a verdict.
#[trace("TC-601")]
#[test]
fn a_match_refused_or_incomplete_at_a_joint_step_is_undetermined() {
    let world = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    for matcher in [&Refusing as &dyn StepMatch, &Exhausted] {
        let outcome = world
            .run_through(
                &world.seams(),
                matcher,
                ModelCheckLimits::default(),
                &Cancel::new(),
            )
            .expect("the run is not refused");
        let HyperOutcome::Undecided { cause, .. } = outcome else {
            panic!("expected no decision, got {outcome:?}");
        };
        assert_eq!(cause, UndecidedCause::MatchUndetermined);
    }
}

/// A letter that has no value where `b`'s output is 1.
struct UndefinedWhereBHigh<'w>(crate::hyper_world::Seams<'w>);

impl HyperLetters for UndefinedWhereBHigh<'_> {
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &[ParameterBinding],
        states: &[&ModelSimState],
        meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        if low(states[1]) == 1 {
            return Ok(AtomReading::Undefined(UndefinedCause::Kernel(
                Undefined::DivisionByZero,
            )));
        }
        self.0.atom(atom, instance, states, meter)
    }
}

/// ADR-018 UE-4 and ADR-023 HV-8: a body atom with no value at a reachable
/// joint position ends the item with an `Undefined` counterexample whose
/// paths end at that position, naming both states and the atom.
#[trace("TC-601")]
#[test]
fn a_letter_with_no_value_ends_the_item_at_its_joint_position() {
    let world = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let letters = UndefinedWhereBHigh(world.seams());
    let outcome = world
        .run_through(
            &letters,
            &world.seams(),
            ModelCheckLimits::default(),
            &Cancel::new(),
        )
        .expect("the run is not refused");
    let counterexample = refuted(outcome);
    let HyperCounterexampleKind::Undefined(undefined) = &counterexample.kind else {
        panic!("expected an undefined-evaluation counterexample");
    };
    assert!(matches!(undefined.locus, HyperLocus::Atom(_)));
    assert_eq!(undefined.position, 1);
    assert_eq!(counterexample.trace_position, 1);
    // The paths end where `b`'s output is 1: the oracle reads it.
    let b = positions(&world, &counterexample, 1);
    assert_eq!(b.len(), 2);
    assert_eq!(b[1].1, 1);
    assert_eq!(undefined.state_keys.len(), 2);
}

/// FR-176 "Limits": `max_states` counts product states, `max_automaton_states`
/// automaton states, and a cancelled run is `Stopped(Cancelled)`.
#[trace("TC-601")]
#[test]
fn the_products_limits_and_cancellation_stop_the_run() {
    let world = World::new(
        Vault::Leaky,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    // The vault has 4 states; its product 10: a budget of 5 passes the
    // subject and stops the product.
    let mut states = ModelCheckLimits::default();
    states.limits.max_states = 5;
    let HyperOutcome::Stopped {
        cause,
        limit,
        stats,
        ..
    } = world
        .run(states, &Cancel::new())
        .expect("the run is not refused")
    else {
        panic!("expected the product to stop");
    };
    assert_eq!(cause, IncompleteCause::ResourceExhausted);
    assert_eq!(
        limit,
        Some(ReachedLimit {
            limit: ModelCheckLimit::MaxStates,
            value: 5
        })
    );
    assert_eq!(stats.product_states, 5);

    let automaton = ModelCheckLimits {
        max_automaton_states: 1,
        ..ModelCheckLimits::default()
    };
    let HyperOutcome::Stopped { limit, .. } = world
        .run(automaton, &Cancel::new())
        .expect("the run is not refused")
    else {
        panic!("expected the automaton to stop");
    };
    assert_eq!(
        limit,
        Some(ReachedLimit {
            limit: ModelCheckLimit::MaxAutomatonStates,
            value: 1
        })
    );

    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let HyperOutcome::Stopped { cause, limit, .. } = world
        .run(ModelCheckLimits::default(), &cancel)
        .expect("the run is not refused")
    else {
        panic!("expected the run to be cancelled");
    };
    assert_eq!(
        cause,
        IncompleteCause::Cancelled {
            source: CancelCause::Requested
        }
    );
    assert_eq!(limit, None);
}

/// The clause over one object parameter `v` and one variable `a`, whose body
/// is `body`, with `fairness` after the quantifier.
fn single(fairness: &str, body: &str) -> String {
    format!(
        "hyper H using inf over behaviours (v: V::Vault) \
         forall trace a of V {fairness} {{ {body} }}"
    )
}

/// SR-13450 FND-001: a body whose negation has no acceptance set (`eventually
/// p`, a bare state atom) is decided. From `(0, 0)` over the secure vault,
/// `eventually holds(v.l @ a = 0)` and `holds(v.l @ a = 0)` hold at position 0;
/// `eventually holds(v.l @ a = 1)` fails on the behaviour that always takes
/// `step(0)`, and `holds(v.l @ a = 1)` fails at position 0. Every
/// counterexample replays.
#[trace("TC-601")]
#[test]
fn a_co_safety_body_is_decided_and_its_counterexample_replays() {
    for body in ["eventually holds(v.l @ a = 0)", "holds(v.l @ a = 0)"] {
        let world = World::new(Vault::Secure, false, &STARTS[..1], &single("", body));
        assert!(
            matches!(run(&world), HyperOutcome::Holds { .. }),
            "{body} holds"
        );
    }
    for (body, position) in [
        ("eventually holds(v.l @ a = 1)", None),
        ("holds(v.l @ a = 1)", Some(0)),
    ] {
        let world = World::new(Vault::Secure, false, &STARTS[..1], &single("", body));
        let counterexample = refuted(run(&world));
        assert!(
            matches!(
                counterexample.kind,
                HyperCounterexampleKind::Lockstep { .. }
            ),
            "{body}"
        );
        if let Some(position) = position {
            assert_eq!(counterexample.trace_position, position, "{body}");
        }
        assert!(
            matches!(
                world.replay(&counterexample),
                Ok(qsl_replay::ModelTraceSettlement::Reproduced { .. })
            ),
            "{body} replays"
        );
    }
}

/// The lassos of several instances (second phase): the instance that has a
/// violation is the answer when the others hold, and at equal length the
/// earlier instance wins. These violations are lassos, not first-phase
/// findings, so they exercise the second phase's ranking only.
#[trace("TC-601")]
#[test]
fn second_phase_lassos_across_instances_rank_by_length_then_instance_order() {
    let at_start = single("", "holds(v.l @ a = 0)");
    let key_of = |world: &World| refuted(run(world)).instance;
    let second = World::with_vaults(Vault::Secure, &[("v0", (0, 0)), ("v1", (0, 1))], &at_start);
    assert_eq!(key_of(&second), [("v".to_owned(), "v1".to_owned())]);
    let tie = World::with_vaults(Vault::Secure, &[("v0", (0, 1)), ("v1", (0, 1))], &at_start);
    assert_eq!(key_of(&tie), [("v".to_owned(), "v0".to_owned())]);
}

/// A letter that has no value for the instance `key` where that vault's `l`
/// (read on trace `a`) is 1; every other reading is the world's.
struct UndefinedFor<'w> {
    seams: crate::hyper_world::Seams<'w>,
    key: &'static str,
}

impl HyperLetters for UndefinedFor<'_> {
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &[ParameterBinding],
        states: &[&ModelSimState],
        meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        if instance[0].instance.key == self.key && low_of(states[0], self.key) == 1 {
            return Ok(AtomReading::Undefined(UndefinedCause::Kernel(
                Undefined::DivisionByZero,
            )));
        }
        self.seams.atom(atom, instance, states, meter)
    }
}

/// A `match` with no value on every joint step of the instance `key`.
struct UndefinedMatchFor<'w> {
    seams: crate::hyper_world::Seams<'w>,
    key: &'static str,
}

impl StepMatch for UndefinedMatchFor<'_> {
    fn universal(
        &self,
        instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        if instance[0].instance.key == self.key {
            return Ok(MatchReading::Undefined {
                conjunct: 0,
                cause: UndefinedCause::Kernel(Undefined::DivisionByZero),
            });
        }
        self.seams.universal(instance, steps, meter)
    }

    fn existential(
        &self,
        _instance: &[ParameterBinding],
        _steps: &[StepLabel<'_>],
        _meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        panic!("an HP-2 clause has no existential conjunct")
    }
}

/// FR-126 "Bindings and ranking" (FR-176 reuses it), first-phase findings: a
/// finding at depth 0 in the first instance ends the run before a later
/// instance is explored. The first instance's only state has an undefined
/// letter, so it costs one product state; the second would need more than the
/// 3 left under `max_states` 4 (the vault's 4 reachable states pass the
/// pre-check), and without the break the run would stop there.
#[trace("TC-601")]
#[test]
fn a_finding_at_depth_zero_ends_the_later_instances() {
    let clause = single("", "always holds(v.l @ a = 0)");
    let world = World::with_vaults(Vault::Secure, &[("v0", (0, 1)), ("v1", (0, 0))], &clause);
    let letters = UndefinedFor {
        seams: world.seams(),
        key: "v0",
    };
    let mut tight = ModelCheckLimits::default();
    tight.limits.max_states = 4;
    let outcome = world
        .run_through(&letters, &world.seams(), tight, &Cancel::new())
        .expect("not refused");
    let HyperOutcome::Violated {
        counterexample,
        stats,
    } = outcome
    else {
        panic!("the depth-0 finding stands, got {outcome:?}");
    };
    assert_eq!(counterexample.instance, [("v".to_owned(), "v0".to_owned())]);
    assert_eq!(counterexample.trace_position, 0);
    assert!(matches!(
        counterexample.kind,
        HyperCounterexampleKind::Undefined(ref undefined)
            if matches!(undefined.locus, HyperLocus::Atom(_))
    ));
    assert_eq!(
        stats.product_states, 1,
        "only the first instance is explored"
    );
}

/// The clause over the pair `a`, `b` with equal inputs, as `NonInterference`.
const PAIR: &str = "hyper H using inf over behaviours (v: V::Vault) \
     forall trace a of V forall trace b of V \
     match { a.step.op = b.step.op and a.step.Vault::step.i = b.step.Vault::step.i } \
     { always holds(v.l @ a = v.l @ b) }";

/// The same ranking at depth 1. The first instance finds an undefined letter
/// at depth 1 (after `step(1)` on `v0`). A later instance is explored only
/// through depth 1, expanding its initial states and no deeper: the counts of
/// the whole run, set as the limits, still let it finish, and an uncapped
/// second instance, a product of dozens of states, would exceed them and stop
/// the run.
#[trace("TC-601")]
#[test]
fn a_finding_at_depth_one_caps_the_later_instances() {
    let world = World::with_vaults(Vault::Secure, &[("v0", (0, 0)), ("v1", (0, 0))], PAIR);
    let letters = UndefinedFor {
        seams: world.seams(),
        key: "v0",
    };
    let HyperOutcome::Violated {
        counterexample,
        stats,
    } = world
        .run_through(
            &letters,
            &world.seams(),
            ModelCheckLimits::default(),
            &Cancel::new(),
        )
        .expect("not refused")
    else {
        panic!("expected a violation");
    };
    assert_eq!(counterexample.instance, [("v".to_owned(), "v0".to_owned())]);
    assert_eq!(counterexample.trace_position, 1);
    // The counts measured from the capped run: `v0` explored in full, `v1` only
    // through the expansion of its initial states. They are the budget the
    // run is held to; any state the capped `v1` explored past its initial states would exceed the 18.
    assert_eq!((stats.product_states, stats.product_edges), (18, 80));
    let mut exact = ModelCheckLimits::default();
    exact.limits.max_states = 18;
    exact.limits.max_transitions = 80;
    let outcome = world
        .run_through(&letters, &world.seams(), exact, &Cancel::new())
        .expect("not refused");
    let HyperOutcome::Violated {
        counterexample: again,
        ..
    } = outcome
    else {
        panic!("the capped second instance fits the budget: {outcome:?}");
    };
    assert_eq!(*again, *counterexample);
}

/// A `match` finding is at the depth of the joint position it ends at, one
/// deeper than the state it is found at. The first instance has an undefined
/// letter at depth 1; the second has an undefined `match` on its first joint
/// step out of a depth-0 state, depth 1 too. A tie goes to the earlier
/// instance; ranking the `match` at the state's depth (0) would let the
/// second win.
#[trace("TC-601")]
#[test]
fn a_match_finding_ranks_at_the_joint_position_it_ends_at() {
    let world = World::with_vaults(Vault::Secure, &[("v0", (0, 0)), ("v1", (0, 0))], PAIR);
    let letters = UndefinedFor {
        seams: world.seams(),
        key: "v0",
    };
    let matcher = UndefinedMatchFor {
        seams: world.seams(),
        key: "v1",
    };
    let outcome = world
        .run_through(
            &letters,
            &matcher,
            ModelCheckLimits::default(),
            &Cancel::new(),
        )
        .expect("not refused");
    let counterexample = refuted(outcome);
    assert_eq!(counterexample.instance, [("v".to_owned(), "v0".to_owned())]);
    assert!(matches!(
        counterexample.kind,
        HyperCounterexampleKind::Undefined(ref undefined)
            if matches!(undefined.locus, HyperLocus::Atom(_))
    ));
    // Reverse the roles: the `match` finding is now the earlier instance's
    // and the letter finding the later one's, so the earlier `match` wins.
    let reversed = UndefinedMatchFor {
        seams: world.seams(),
        key: "v0",
    };
    let later_letters = UndefinedFor {
        seams: world.seams(),
        key: "v1",
    };
    let counterexample = refuted(
        world
            .run_through(
                &later_letters,
                &reversed,
                ModelCheckLimits::default(),
                &Cancel::new(),
            )
            .expect("not refused"),
    );
    assert_eq!(counterexample.instance, [("v".to_owned(), "v0".to_owned())]);
    assert!(matches!(
        counterexample.kind,
        HyperCounterexampleKind::Undefined(ref undefined)
            if matches!(undefined.locus, HyperLocus::Match { .. })
    ));
}

/// The clause over `a` and `b` whose `match` no pair of runs can satisfy, so
/// every instance is vacuous: no joint step has `b` stuttering.
#[trace("TC-601")]
#[test]
fn a_clause_every_instance_of_which_is_vacuous_is_vacuous() {
    let clause = "hyper H using inf over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V \
         match { a.step.op != stutter and b.step.op = stutter } \
         { always holds(v.l @ a = v.l @ b) }";
    let world = World::with_vaults(Vault::Secure, &[("v0", (0, 0)), ("v1", (1, 0))], clause);
    let HyperOutcome::Undecided { cause, .. } = run(&world) else {
        panic!("expected no decision");
    };
    assert_eq!(cause, UndecidedCause::VacuousMatch);
}

/// FR-176 "Limits": `max_transitions` counts the product's edges. The leaky
/// vault has 8 edges and the product 20, so a budget of 10 passes the subject
/// and stops the product.
#[trace("TC-601")]
#[test]
fn the_products_edges_are_counted_against_max_transitions() {
    let world = World::new(
        Vault::Leaky,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_transitions = 10;
    let HyperOutcome::Stopped { cause, limit, .. } =
        world.run(limits, &Cancel::new()).expect("not refused")
    else {
        panic!("expected the product to stop");
    };
    assert_eq!(cause, IncompleteCause::ResourceExhausted);
    assert_eq!(
        limit,
        Some(ReachedLimit {
            limit: ModelCheckLimit::MaxTransitions,
            value: 10
        })
    );
}

/// ADR-019 in the component-scoped filter: over the secure vault with `reset`
/// and `always eventually holds(v.l @ a = 0)`, `step(0)` and `step(1)` are
/// two identities of one operation, both always enabled. A loop of `step(1)`
/// takes the operation and satisfies a constraint on the operation as a
/// whole, weak or strong, so those refute the clause; a constraint on each
/// identity forces `step(0)` too, which reaches `l = 0`, so those hold.
#[trace("TC-601")]
#[test]
fn each_and_strong_constraints_are_read_per_identity_in_the_scoped_filter() {
    let body = "always eventually holds(v.l @ a = 0)";
    for (fairness, holds) in [
        ("fair weak V::Vault::step;", false),
        ("fair weak each V::Vault::step;", true),
        ("fair strong V::Vault::step;", false),
        ("fair strong each V::Vault::step;", true),
    ] {
        let world = World::new(Vault::Secure, true, &STARTS[..1], &single(fairness, body));
        match (run(&world), holds) {
            (HyperOutcome::Holds { .. }, true) => {}
            (HyperOutcome::Violated { counterexample, .. }, false) => assert!(
                matches!(
                    world.replay(&counterexample),
                    Ok(qsl_replay::ModelTraceSettlement::Reproduced { .. })
                ),
                "{fairness}"
            ),
            (other, _) => panic!("{fairness}: unexpected {other:?}"),
        }
    }
}

/// ADR-023 HM-5 and HM-7: a vacuity verdict is decided by the stutter loop
/// where a terminal state is the only place the cycle can be. `match {
/// a.step.op = stutter and b.step.op = stutter }` over the `Terminal` vault
/// is vacuous from `(0, 0)`, where `step` is enabled and no joint step is
/// stutter, and holds from `(0, 1)`, a terminal state, whose stutter loop is
/// the fair matched cycle.
#[trace("TC-601")]
#[test]
fn a_stutter_loop_at_a_terminal_state_is_a_matched_fair_cycle() {
    let clause = "hyper H using inf over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V \
         match { a.step.op = stutter and b.step.op = stutter } \
         { always holds(v.l @ a = v.l @ b) }";
    let enabled = World::new(Vault::Terminal, false, &[(0, 0)], clause);
    let HyperOutcome::Undecided { cause, .. } = run(&enabled) else {
        panic!("expected no decision");
    };
    assert_eq!(cause, UndecidedCause::VacuousMatch);
    let terminal = World::new(Vault::Terminal, false, &[(0, 1)], clause);
    assert!(matches!(run(&terminal), HyperOutcome::HoldsReduced { .. }));
}
