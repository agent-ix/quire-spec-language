// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-178 (TC-603): EN-1 checks a projection-aligned universal hyperproperty
//! (HP-6) over the projected product, on ADR-023 section 13's vault
//! (`crate::hyper_world`): `step(i)` requires `busy = 0`, sets `l = i` and
//! sets `busy` to `h`, and `mix` requires `busy = 1` and clears it, so a run
//! with secret 1 takes a `mix` after every `step` and a run with secret 0
//! never does.
//!
//! Every expected verdict below is derived by hand from the vault's
//! transition relation and the projected product's definition (ADR-023 PA-4).
//! Each counterexample is also read back by an oracle written here that
//! re-executes its traces through the vault's `ModelSystem`.

use ix_trace_rs::trace;
use qsl_replay::{
    HyperCounterexample, HyperCounterexampleKind, HyperLocus, HyperOutcome, HyperStep,
    ModelCheckLimits, ModelTraceRefusal, ReplayRefusal, StepMark, UndecidedCause,
};
use quire_exact::{Cancel, Undefined};

use crate::hyper::{input, inputs, positions, refuted, Exhausted, Refusing};
use crate::hyper_world::{non_interference, Vault, World, EQUAL_INPUTS, KEY};
use qsl_eval::temporal::UndefinedCause;

/// `μ` matching the arguments of two `step` steps and nothing else: a `step`
/// is matched against a `step` of equal input, and against any step that is
/// not one.
const AGREE: &str = "(if present(a.step.Vault::step.i) and present(b.step.Vault::step.i) \
     then value(a.step.Vault::step.i) = value(b.step.Vault::step.i) else true)";

/// The conjunct of FR-178-AC-5, which has no value where `a`'s step is
/// `step(0)`.
const INVERSE: &str = "(if present(a.step.Vault::step.i) \
     then 1 / value(a.step.Vault::step.i) else 1) = 1";

/// The vault's two idle initial states `(h, l, busy)`.
const IDLE: [(i64, i64, i64); 2] = [(0, 0, 0), (1, 0, 0)];

/// The secret-1 run caught mid-operation, `busy` already set: its first step
/// is a `mix`.
const MID_OPERATION: [(i64, i64, i64); 2] = [(0, 0, 0), (1, 0, 1)];

/// ADR-023 section 13's lockstep clause.
fn lockstep() -> String {
    non_interference(&format!("match {{ {AGREE} }}"))
}

/// Section 13's clause with `align skip { V::Vault::mix }` and the `match`
/// conjuncts `conjuncts`.
fn aligned(conjuncts: &[&str]) -> String {
    format!(
        "hyper NonInterference using inf over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V align skip {{ V::Vault::mix }} \
         match {{ {} }} {{ always holds(v.l @ a = v.l @ b) }}",
        [&[AGREE], conjuncts].concat().join(" and ")
    )
}

fn run(world: &World) -> HyperOutcome {
    world
        .run(ModelCheckLimits::default(), &Cancel::new())
        .expect("the run is not refused")
}

/// Whether the model step `step` is a `mix`.
fn is_mix(step: &HyperStep) -> bool {
    match step {
        HyperStep::Model(step) => step.transition.operation().contains("/Vault/mix\""),
        HyperStep::TerminalStutter => panic!("the vault has no terminal state"),
    }
}

/// The number of visible steps of trace `trace`.
fn visible(counterexample: &HyperCounterexample, trace: usize) -> usize {
    counterexample.traces[trace]
        .marks
        .iter()
        .filter(|mark| **mark == StepMark::Visible)
        .count()
}

/// FR-178-AC-1: with `μ` matching the arguments of two `step` steps, the
/// lockstep clause pairs a secret-1 run's `mix` with a secret-0 run's `step`
/// and is refuted by a lockstep counterexample, although every visible `step`
/// sets `l = i` in both runs; with `align skip { V::Vault::mix }` the runs
/// align on their `step`s and the item holds exhaustively. The projected
/// product has 18 states: for each pair of secrets, the two outputs `l`
/// both traces share, times the two `busy` values of a secret-1 trace and the
/// one of a secret-0 trace.
#[trace("TC-603", "FR-178-AC-1")]
#[test]
fn the_vault_is_refuted_in_lockstep_and_held_on_its_projection() {
    let lockstep = World::busy(Vault::Busy, &IDLE, &lockstep());
    let counterexample = refuted(run(&lockstep));
    assert!(matches!(
        counterexample.kind,
        HyperCounterexampleKind::Lockstep { .. }
    ));

    let aligned = World::busy(Vault::Busy, &IDLE, &aligned(&[]));
    let HyperOutcome::Holds { stats } = run(&aligned) else {
        panic!("expected the projection to hold");
    };
    assert_eq!(stats.product_states, 18);
}

/// FR-178-AC-2: the leaky `l = (i + h) mod 2` over a secret-1 run that starts
/// mid-operation. The first path to a rejecting monitor state takes that
/// run's `mix` as a skip move, then both runs' `step(0)` together: the
/// traces have one visible step each and two and one steps in all, there is
/// no loop entry, every step is marked, and `trace_position` is the projected
/// position 1. The oracle re-executes the traces and finds the outputs apart.
///
/// The section 13 vault started idle is refuted by one visible step each, so
/// its traces have equal total lengths; the criterion's unequal lengths need
/// a run that begins with a `mix`.
#[trace("TC-603", "FR-178-AC-2")]
#[test]
fn a_leaky_projection_is_refuted_by_a_projected_counterexample() {
    let world = World::busy(Vault::BusyLeaky, &MID_OPERATION, &aligned(&[]));
    let counterexample = refuted(run(&world));
    assert_eq!(counterexample.kind, HyperCounterexampleKind::Projected);
    assert_eq!(counterexample.instance, [("v".to_owned(), KEY.to_owned())]);
    assert_eq!(counterexample.trace_position, 1);
    let names: Vec<_> = counterexample
        .traces
        .iter()
        .map(|trace| (trace.variable.as_str(), trace.alias.as_str()))
        .collect();
    assert_eq!(names, [("a", "V"), ("b", "V")]);

    // The secret-0 trace (initial state 0) takes one visible `step`; the
    // secret-1 trace (initial state 1) takes a skipped `mix` and then its
    // visible `step`.
    let lengths: Vec<_> = counterexample
        .traces
        .iter()
        .map(|trace| (trace.initial, trace.steps.len()))
        .collect();
    assert!(
        lengths == [(0, 1), (1, 2)] || lengths == [(1, 2), (0, 1)],
        "{lengths:?}"
    );
    assert_eq!(visible(&counterexample, 0), 1);
    assert_eq!(visible(&counterexample, 1), 1);
    for (index, trace) in counterexample.traces.iter().enumerate() {
        assert_eq!(trace.marks.len(), trace.steps.len());
        // A skipped mark names a `mix` and a visible mark a `step`.
        for (step, mark) in trace.steps.iter().zip(&trace.marks) {
            assert_eq!(*mark == StepMark::Skipped, is_mix(step), "{step:?}");
        }
        assert_eq!(trace.marks.last(), Some(&StepMark::Visible));
        // The visible step takes the canonically first equal input.
        let visible_inputs: Vec<_> = trace
            .steps
            .iter()
            .filter(|step| !is_mix(step))
            .map(input)
            .collect();
        assert_eq!(visible_inputs, [0], "trace {index}");
    }
    // The oracle: both traces end after their visible step, with outputs
    // `i` and `(i + 1) mod 2` for the secrets 0 and 1.
    let ends: Vec<_> = (0..2)
        .map(|trace| {
            positions(&world, &counterexample, trace)
                .pop()
                .expect("a position")
        })
        .collect();
    let (secret_0, secret_1) = if counterexample.traces[0].initial == 0 {
        (ends[0], ends[1])
    } else {
        (ends[1], ends[0])
    };
    assert_eq!((secret_0.0, secret_0.1), (0, 0));
    assert_eq!((secret_1.0, secret_1.1), (1, 1));
}

/// From idle starts the leak is one visible move away: the first rejecting
/// monitor state is reached by one visible step of each trace and no skipped
/// step precedes it, so the traces have equal lengths. The engine confirms
/// every rejecting state through the three-valued evaluator before it reports
/// it (`qsl-eval`'s `hyper_tuple` tests read the evaluator itself).
#[trace("TC-603")]
#[test]
fn from_idle_starts_the_leak_is_one_visible_step_away() {
    let world = World::busy(Vault::BusyLeaky, &IDLE, &aligned(&[]));
    let counterexample = refuted(run(&world));
    assert_eq!(counterexample.kind, HyperCounterexampleKind::Projected);
    assert_eq!(counterexample.trace_position, 1);
    for trace in &counterexample.traces {
        assert_eq!(trace.steps.len(), 1);
        assert_eq!(trace.marks, [StepMark::Visible]);
    }
}

/// FR-178-AC-4: with `max_depth` 1 the search of AC-2's request ends before
/// the rejecting state at two moves, a skip move counting as one, and settles
/// `BoundReached{depth: 1}`; with `max_depth` 2 it finds the counterexample.
/// Running AC-2's request twice gives equal counterexamples.
#[trace("TC-603", "FR-178-AC-4")]
#[test]
fn the_horizon_counts_a_skip_move_and_the_run_is_deterministic() {
    let mut world = World::busy(Vault::BusyLeaky, &MID_OPERATION, &aligned(&[]));
    world.max_depth = Some(1);
    let HyperOutcome::BoundReached { depth, stats } = run(&world) else {
        panic!("expected the horizon to be reached");
    };
    assert_eq!(depth, 1);
    // The initial pairs and the states one move away.
    assert!(stats.depth <= 1, "{stats:?}");

    world.max_depth = Some(2);
    let within = refuted(run(&world));
    world.max_depth = None;
    let first = refuted(run(&world));
    let second = refuted(run(&world));
    assert_eq!(within, first);
    assert_eq!(first, second);
    assert_eq!(format!("{first:?}"), format!("{second:?}"));
}

/// A counterexample at exactly the horizon is found: from idle starts the
/// leak is one visible move away, so `max_depth` 1 refutes and `max_depth` 0,
/// which expands nothing, reaches its horizon.
#[trace("TC-603", "FR-178-AC-4")]
#[test]
fn a_counterexample_at_the_horizon_is_found_and_one_beyond_it_is_not() {
    let mut world = World::busy(Vault::BusyLeaky, &IDLE, &aligned(&[]));
    world.max_depth = Some(1);
    let at = refuted(run(&world));
    assert_eq!(at.kind, HyperCounterexampleKind::Projected);
    world.max_depth = Some(0);
    let HyperOutcome::BoundReached { depth, .. } = run(&world) else {
        panic!("expected the horizon to be reached");
    };
    assert_eq!(depth, 0);
}

/// A run that finishes before its horizon holds, not `BoundReached`.
#[trace("TC-603", "FR-178-AC-4")]
#[test]
fn a_run_that_completes_inside_its_horizon_holds() {
    let mut world = World::busy(Vault::Busy, &IDLE, &aligned(&[]));
    world.max_depth = Some(64);
    assert!(matches!(run(&world), HyperOutcome::Holds { .. }));
    world.max_depth = Some(1);
    let HyperOutcome::BoundReached { depth, .. } = run(&world) else {
        panic!("expected the horizon to be reached");
    };
    assert_eq!(depth, 1);
}

/// FR-178-AC-5: AC-1's clause with a `match` conjunct that has no value where
/// `a`'s step is `step(0)` is refuted by an `Undefined` counterexample, one
/// unmarked path per variable ending at the first such joint visible step in
/// breadth-first order, cause `division-by-zero`.
#[trace("TC-603", "FR-178-AC-5")]
#[test]
fn an_undefined_match_ends_the_paths_at_the_first_such_visible_step() {
    let world = World::busy(Vault::Busy, &IDLE, &aligned(&[INVERSE]));
    let counterexample = refuted(run(&world));
    let HyperCounterexampleKind::Undefined(undefined) = &counterexample.kind else {
        panic!(
            "expected an undefined evaluation: {:?}",
            counterexample.kind
        );
    };
    assert_eq!(
        undefined.cause,
        UndefinedCause::Kernel(Undefined::DivisionByZero)
    );
    assert_eq!(undefined.locus, HyperLocus::Match { conjunct: 1 });
    assert_eq!(undefined.position, 1);
    assert_eq!(undefined.transition, None);
    assert_eq!(undefined.state_keys.len(), 2);
    assert_eq!(counterexample.trace_position, 1);
    assert_eq!(counterexample.traces.len(), 2);
    for (index, trace) in counterexample.traces.iter().enumerate() {
        assert_eq!(trace.steps.len(), 1);
        assert!(trace.marks.is_empty(), "an undefined path is unmarked");
        assert_eq!(inputs(&counterexample, index), [0]);
    }
}

/// FR-178-AC-5, at the projected position: when both runs start mid-operation
/// the first such step follows two skip moves, one for each run's `mix`. The
/// paths are unmarked, each holds its `mix` and the visible `step(0)`, and the
/// evaluation is at the projected position 1, the number of visible steps,
/// not at the three moves that reach it.
#[trace("TC-603", "FR-178-AC-5")]
#[test]
fn an_undefined_match_after_skip_moves_is_at_the_projected_position() {
    let world = World::busy(Vault::Busy, &[(1, 0, 1)], &aligned(&[INVERSE]));
    let counterexample = refuted(run(&world));
    let HyperCounterexampleKind::Undefined(undefined) = &counterexample.kind else {
        panic!(
            "expected an undefined evaluation: {:?}",
            counterexample.kind
        );
    };
    assert_eq!(undefined.position, 1);
    assert_eq!(counterexample.trace_position, 1);
    for trace in &counterexample.traces {
        assert!(trace.marks.is_empty(), "an undefined path is unmarked");
        let kinds: Vec<_> = trace.steps.iter().map(is_mix).collect();
        assert_eq!(kinds, [true, false]);
    }
    assert_eq!(inputs_after_mix(&counterexample, 0), 0);
    assert_eq!(inputs_after_mix(&counterexample, 1), 0);
}

/// The input of trace `trace`'s `step`, the one after its `mix`.
fn inputs_after_mix(counterexample: &HyperCounterexample, trace: usize) -> i64 {
    input(&counterexample.traces[trace].steps[1])
}

/// FR-178-AC-6: a `match` conjunct whose evaluation exhausts its meter, or is
/// refused, at a joint visible step leaves the item `Undecided` with
/// `MatchUndetermined`, never `Holds`.
#[trace("TC-603", "FR-178-AC-6")]
#[test]
fn an_incomplete_or_refused_match_is_undetermined_and_never_holds() {
    let world = World::busy(Vault::Busy, &IDLE, &aligned(&[]));
    for matcher in [&Exhausted as &dyn qsl_eval::hyper::StepMatch, &Refusing] {
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

/// A search horizon is built for an HP-6 item alone: a lockstep clause that
/// carries one is refused rather than run without it.
#[test]
fn a_lockstep_item_with_a_horizon_is_refused() {
    let mut world = World::new(
        Vault::Secure,
        false,
        &[(0, 0), (1, 0)],
        &non_interference(EQUAL_INPUTS),
    );
    world.max_depth = Some(3);
    let Err(refusal) = world.run(ModelCheckLimits::default(), &Cancel::new()) else {
        panic!("expected the horizon to be refused");
    };
    assert!(
        refusal.to_string().contains("HP-6"),
        "unexpected refusal: {refusal}"
    );
}

/// A `Projected` counterexample is not replayed yet: FR-183 checks its marks
/// and its projected prefix, and until that is built the replay refuses it as
/// unsupported rather than reading it as a lockstep one.
#[trace("TC-603")]
#[test]
fn a_projected_counterexample_is_refused_by_replay_until_fr_183() {
    let world = World::busy(Vault::BusyLeaky, &IDLE, &aligned(&[]));
    let counterexample = refuted(run(&world));
    let Err(ReplayRefusal::ModelTrace(refusal)) = world.replay(&counterexample) else {
        panic!("expected the replay to refuse");
    };
    assert_eq!(*refusal, ModelTraceRefusal::HyperProjected);
    assert_eq!(
        refusal.to_string(),
        "unsupported_construct/not-yet-implemented: replay of a projected counterexample"
    );
}
