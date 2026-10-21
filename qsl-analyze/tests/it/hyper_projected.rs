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
use qsl_analyze::model_check::{HyperRefusal, ModelCheckRefusal};
use qsl_eval::hyper::{HyperLetters, MatchReading, ParameterBinding, StepLabel, StepMatch};
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::Limits;
use qsl_eval::temporal::AtomReading;
use qsl_replay::{
    HyperCounterexample, HyperCounterexampleKind, HyperLocus, HyperOutcome, HyperStep,
    IncompleteCause, ModelCheckLimit, ModelCheckLimits, ModelTraceRefusal, ReplayRefusal, StepMark,
    UndecidedCause,
};
use qsl_semantics::check::{FormulaNodeId, HyperForm, OperationKey};
use qsl_semantics::model::key::DeclarationKey;
use quire_exact::{Cancel, Charge, ChargePoint, Identifier, Incomplete, Meter, Undefined};
use std::cell::RefCell;

use crate::counter::ZERO;

use crate::hyper::{input, inputs, positions, refuted, Exhausted, Refusing};
use crate::hyper_world::{
    busy_of, high, input_of, key as hyper_key, low, non_interference, Vault, World, EQUAL_INPUTS,
    KEY,
};
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
pub(crate) const IDLE: [(i64, i64, i64); 2] = [(0, 0, 0), (1, 0, 0)];

/// The secret-1 run caught mid-operation, `busy` already set: its first step
/// is a `mix`.
const MID_OPERATION: [(i64, i64, i64); 2] = [(0, 0, 0), (1, 0, 1)];

/// Section 13's clause with `align skip { V::Vault::mix }` over the body
/// `body` and the `match` conjuncts `conjuncts`.
fn aligned_over(body: &str, conjuncts: &[&str]) -> String {
    format!(
        "hyper NonInterference using inf over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V align skip {{ V::Vault::mix }} \
         match {{ {} }} {{ {body} }}",
        [&[AGREE], conjuncts].concat().join(" and ")
    )
}

/// ADR-023 section 13's lockstep clause.
fn lockstep() -> String {
    non_interference(&format!("match {{ {AGREE} }}"))
}

/// Section 13's clause with `align skip { V::Vault::mix }` and the `match`
/// conjuncts `conjuncts`.
pub(crate) fn aligned(conjuncts: &[&str]) -> String {
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

/// The always-equal body of section 13's clause written as a disjunction whose
/// first disjunct is a plain state test.
const OR_BODY: &str = "holds(v.l @ a = 1) or always holds(v.l @ a = v.l @ b)";

/// QSL-923: every vault state starts with `l = 0` in both runs, so the state
/// test `l @ a = 1` is false at position 0 and the clause is section 13's:
/// it is held on its projection and refuted by a projected counterexample at
/// position 1 when the vault leaks. A translation that loses the `always`
/// disjunct has no behaviour in the body and refutes the held clause at once.
#[trace("TC-603")]
#[test]
fn a_projected_body_with_a_state_test_disjunct_keeps_the_other_disjunct() {
    let held = World::busy(Vault::Busy, &IDLE, &aligned_over(OR_BODY, &[]));
    assert!(matches!(run(&held), HyperOutcome::Holds { .. }));

    let leaky = World::busy(
        Vault::BusyLeaky,
        &MID_OPERATION,
        &aligned_over(OR_BODY, &[]),
    );
    let counterexample = refuted(run(&leaky));
    assert_eq!(counterexample.kind, HyperCounterexampleKind::Projected);
    assert_eq!(counterexample.trace_position, 1);
}

/// QSL-923: a body whose first disjunct is itself a disjunction of two state
/// tests. At position 0 both runs have `l = 0`, so that disjunct is false and
/// the clause is section 13's. The translation refused it, because the
/// negation of `l @ a = 1 or l @ b = 1` was never built.
#[trace("TC-603")]
#[test]
fn a_projected_body_with_a_nested_state_disjunction_is_decided() {
    let body = "(holds(v.l @ a = 1) or holds(v.l @ b = 1)) or \
                always holds(v.l @ a = v.l @ b)";
    let held = World::busy(Vault::Busy, &IDLE, &aligned_over(body, &[]));
    assert!(matches!(run(&held), HyperOutcome::Holds { .. }));

    let leaky = World::busy(Vault::BusyLeaky, &MID_OPERATION, &aligned_over(body, &[]));
    let counterexample = refuted(run(&leaky));
    assert_eq!(counterexample.kind, HyperCounterexampleKind::Projected);
    assert_eq!(counterexample.trace_position, 1);
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
/// step precedes it, so the traces have equal lengths.
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
#[trace("TC-603")]
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
#[trace("TC-603")]
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

/// FR-178-AC-6, the engine half: a `match` whose reading is incomplete or
/// refused at a joint visible step leaves the item `Undecided` with
/// `MatchUndetermined`, never `Holds`. S3 does not lower hyper `match` blocks
/// to checked nodes the clause evaluator can run yet, so the `StepMatch` seam
/// is replaced by a stub that charges a meter admitting no work (`Exhausted`)
/// or refuses (`Refusing`); no conjunct calls a function, and the evaluator's
/// own meter path is not run here.
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

/// A search horizon is built for an HP-6 item alone: a clause of another form
/// that carries one is refused, whichever form it is, rather than run
/// without it.
#[trace("TC-603")]
#[test]
fn a_horizon_on_a_lockstep_or_forall_exists_item_is_refused() {
    let opaque = "hyper Opaque using inf over behaviours (v: V::Vault) \
         forall trace a of V exists trace b of V \
         match { a.step.Vault::step.i = b.step.Vault::step.i } \
         { holds(v.h @ a != v.h @ b) and always holds(v.l @ a = v.l @ b) }";
    for (declaration, form) in [
        (non_interference(EQUAL_INPUTS), HyperForm::Universal),
        (opaque.to_owned(), HyperForm::ForallExistsSafety),
    ] {
        let mut world = World::new(Vault::Secure, false, &[(0, 0), (1, 0)], &declaration);
        world.max_depth = Some(3);
        assert!(
            matches!(
                world.run(ModelCheckLimits::default(), &Cancel::new()),
                Err(ModelCheckRefusal::Hyper(HyperRefusal::HorizonNotDecided {
                    form: Some(refused)
                })) if refused == form
            ),
            "{form:?}"
        );
    }
}

/// A counterexample of an `align skip` clause is not replayed yet: FR-183
/// checks the marks and the projected prefix of a `Projected` one, and does not
/// state the replay of an `Undefined` one, whose paths differ in length and
/// whose position counts visible steps. Until that is built the replay refuses
/// both as unsupported rather than reading them as lockstep ones.
#[trace("TC-603")]
#[test]
fn a_counterexample_of_an_align_skip_clause_is_refused_by_replay_until_fr_183() {
    let projected = World::busy(Vault::BusyLeaky, &IDLE, &aligned(&[]));
    let undefined = World::busy(Vault::Busy, &[(1, 0, 1)], &aligned(&[INVERSE]));
    for world in [&projected, &undefined] {
        let counterexample = refuted(run(world));
        assert!(counterexample.marks_match_kind());
        let Err(ReplayRefusal::ModelTrace(refusal)) = world.replay(&counterexample) else {
            panic!("expected the replay to refuse");
        };
        assert_eq!(*refusal, ModelTraceRefusal::HyperProjected);
    }
    assert!(matches!(
        refuted(run(&undefined)).kind,
        HyperCounterexampleKind::Undefined(_)
    ));
}

/// The marks belong to a `Projected` counterexample: one for every step of
/// every trace, and none on any other kind (QSpec FR-400). The reader refuses
/// a counterexample that breaks this as malformed, before it reads its kind.
#[trace("TC-603")]
#[test]
fn marks_on_the_wrong_kind_or_of_the_wrong_length_are_refused_as_malformed() {
    let world = World::busy(Vault::BusyLeaky, &IDLE, &aligned(&[]));
    let projected = refuted(run(&world));
    assert!(projected.marks_match_kind());
    let mut short = projected.clone();
    short.traces[0].marks.clear();
    let mut marked_lockstep = projected;
    marked_lockstep.kind = HyperCounterexampleKind::Lockstep { loop_entry: 0 };
    for counterexample in [short, marked_lockstep] {
        assert!(!counterexample.marks_match_kind());
        let Err(ReplayRefusal::ModelTrace(refusal)) = world.replay(&counterexample) else {
            panic!("expected the replay to refuse");
        };
        assert!(matches!(*refusal, ModelTraceRefusal::HyperShape { .. }));
    }
}

/// The reads the body's atoms get, kept in order, each as the states'
/// `(h, l, busy)` of every component. A read of a tuple the run already read
/// answers `again` when that is `Incomplete`.
struct Probe<'w> {
    inner: crate::hyper_world::Seams<'w>,
    incomplete_again: bool,
    log: RefCell<Vec<Vec<i64>>>,
}

impl Probe<'_> {
    fn reads(&self, tuple: &[i64]) -> usize {
        self.log
            .borrow()
            .iter()
            .filter(|read| read.as_slice() == tuple)
            .count()
    }
}

impl HyperLetters for Probe<'_> {
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &[ParameterBinding],
        states: &[&ModelSimState],
        meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        let tuple: Vec<i64> = states
            .iter()
            .flat_map(|state| [high(state), low(state), busy_of(state)])
            .collect();
        let again = self.log.borrow().contains(&tuple);
        self.log.borrow_mut().push(tuple);
        if again && self.incomplete_again {
            return Err(Meter::new(ZERO)
                .charge(Charge::new(ChargePoint::GraphExpand))
                .expect_err("a meter that admits no work refuses a charge"));
        }
        self.inner.atom(atom, instance, states, meter)
    }
}

fn probed(
    world: &World,
    incomplete_again: bool,
) -> (Result<HyperOutcome, ModelCheckRefusal>, Probe<'_>) {
    let probe = Probe {
        inner: world.seams(),
        incomplete_again,
        log: RefCell::new(Vec::new()),
    };
    let outcome = world.run_through(
        &probe,
        &world.seams(),
        ModelCheckLimits::default(),
        &Cancel::new(),
    );
    (outcome, probe)
}

/// The engine reports a rejecting monitor state only after the three-valued
/// evaluator reads the joint projected prefix to it as a bad prefix. From
/// mid-operation starts the path to the rejecting state takes a skip move:
/// the evaluator reads projected positions 0 and 1, the state each visible
/// move entered, and never the state the skip move left in between. The
/// rejecting tuple is read twice, once by the product and once by the
/// evaluator, and the tuple after the skip move is read by neither.
#[trace("TC-603")]
#[test]
fn a_rejecting_state_is_confirmed_over_the_projected_positions_only() {
    let world = World::busy(Vault::BusyLeaky, &MID_OPERATION, &aligned(&[]));
    let (outcome, probe) = probed(&world, false);
    let counterexample = refuted(outcome.expect("the run is not refused"));
    assert_eq!(counterexample.kind, HyperCounterexampleKind::Projected);
    // a: (h 0, l 0, busy 0), b: (h 1, l 1, busy 1) after its `mix` and `step(0)`.
    assert_eq!(probe.reads(&[0, 0, 0, 1, 1, 1]), 2);
    // a: (0, 0, 0), b: (1, 0, 0), the tuple after the skip move.
    assert_eq!(probe.reads(&[0, 0, 0, 1, 0, 0]), 0);
    // The initial tuple is read by the product and again by the evaluator.
    assert_eq!(probe.reads(&[0, 0, 0, 1, 0, 1]), 2);
}

/// An evaluator that runs out of its meter while it confirms a rejecting state
/// has no verdict: the item stops with `EvaluationMeter`, never `Violated`.
#[trace("TC-603")]
#[test]
fn an_evaluator_that_runs_out_while_confirming_stops_the_item() {
    let world = World::busy(Vault::BusyLeaky, &IDLE, &aligned(&[]));
    let (outcome, _) = probed(&world, true);
    let outcome = outcome.expect("the run is not refused");
    let HyperOutcome::Stopped { cause, limit, .. } = outcome else {
        panic!("expected the item to stop, got {outcome:?}");
    };
    assert_eq!(cause, IncompleteCause::ResourceExhausted);
    assert_eq!(
        limit.map(|reached| reached.limit),
        Some(ModelCheckLimit::EvaluationMeter)
    );
}

/// The first product state in breadth-first order wins between an undefined
/// `μ_U` and a rejecting monitor state (ADR-023 HV-8). Over idle starts with
/// the body `always holds(v.l @ a = 0)`, `step(1)` rejects and `step(0)` is
/// the visible move on which AC-5's conjunct has no value; `step(0)` is the
/// canonically first move, so the undefined state is discovered first.
#[trace("TC-603")]
#[test]
fn an_undefined_match_discovered_before_a_rejection_wins() {
    let clause = aligned_over("always holds(v.l @ a = 0)", &[INVERSE]);
    let world = World::busy(Vault::Busy, &IDLE, &clause);
    let counterexample = refuted(run(&world));
    let HyperCounterexampleKind::Undefined(undefined) = &counterexample.kind else {
        panic!(
            "expected the undefined state first: {:?}",
            counterexample.kind
        );
    };
    assert_eq!(undefined.locus, HyperLocus::Match { conjunct: 1 });
}

/// A `μ_U` that has no value where `a`'s input is 1.
struct UndefinedOnOne<'w>(crate::hyper_world::Seams<'w>);

impl StepMatch for UndefinedOnOne<'_> {
    fn universal(
        &self,
        instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        Ok(match self.0.universal(instance, steps, meter)? {
            MatchReading::Holds if input_of(&steps[0]) == Some(1) => MatchReading::Undefined {
                conjunct: 1,
                cause: UndefinedCause::Kernel(Undefined::DivisionByZero),
            },
            other => other,
        })
    }

    fn existential(
        &self,
        _instance: &[ParameterBinding],
        _steps: &[StepLabel<'_>],
        _meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        panic!("an HP-6 clause has no existential conjunct")
    }
}

/// The other order: over idle starts with `l = 1` and the body
/// `always holds(v.l @ a = 1)`, `step(0)` rejects and `step(1)` is the move
/// with no value for `mu_U`; `step(0)` is first, so the rejecting state wins.
#[trace("TC-603")]
#[test]
fn a_rejection_discovered_before_an_undefined_match_wins() {
    let clause = aligned_over("always holds(v.l @ a = 1)", &[]);
    let world = World::busy(Vault::Busy, &[(0, 1, 0), (1, 1, 0)], &clause);
    let matcher = UndefinedOnOne(world.seams());
    let outcome = world
        .run_through(
            &world.seams(),
            &matcher,
            ModelCheckLimits::default(),
            &Cancel::new(),
        )
        .expect("the run is not refused");
    assert_eq!(refuted(outcome).kind, HyperCounterexampleKind::Projected);
}

/// A body already false at projected position 0 is rejected before any move:
/// the traces have no steps and `trace_position` is 0.
#[trace("TC-603")]
#[test]
fn a_body_false_at_the_initial_position_is_refuted_with_empty_traces() {
    let world = World::busy(Vault::Busy, &[(0, 0, 0), (1, 1, 0)], &aligned(&[]));
    let counterexample = refuted(run(&world));
    assert_eq!(counterexample.kind, HyperCounterexampleKind::Projected);
    assert_eq!(counterexample.trace_position, 0);
    for trace in &counterexample.traces {
        assert!(trace.steps.is_empty() && trace.marks.is_empty());
    }
    let mut starts: Vec<_> = counterexample.traces.iter().map(|t| t.initial).collect();
    starts.sort_unstable();
    assert_eq!(starts, [0, 1]);
}

/// Two instances rank as FR-126 ranks bindings. With the vaults `k1` and `k2`
/// in every state, the initial states differ in `l` for `k1` alone: `k1` is
/// refuted at depth 0 and `k2` is not explored at all, so the run counts the
/// product states of `k1` alone; with the order reversed, a later instance
/// with a shallower finding wins over an earlier one that holds.
#[trace("TC-603")]
#[test]
fn instances_are_ranked_and_a_finding_at_depth_zero_ends_the_search() {
    let both = |first: (i64, i64), second: (i64, i64), universe: &[&str]| {
        World::with_snapshots(
            Vault::Busy,
            vec![
                vec![("k1", (0, 0)), ("k2", (0, 0))],
                vec![("k1", (1, first.1)), ("k2", (1, second.1))],
            ],
            universe,
            &aligned(&[]),
        )
    };
    let refuted_instance = |world: &World| match run(world) {
        HyperOutcome::Violated {
            counterexample,
            stats,
        } => (counterexample.instance, stats.product_states),
        other => panic!("expected a violation, got {other:?}"),
    };
    let (instance, states) = refuted_instance(&both((1, 1), (1, 0), &["k1", "k2"]));
    assert_eq!(instance, [("v".to_owned(), "k1".to_owned())]);
    // `k2` is not explored: the 174 product states are `k1`'s alone, where
    // reading `k2`'s four initial tuples would count 178, and a budget of
    // exactly 174 still refutes.
    assert_eq!(states, 174);
    let world = both((1, 1), (1, 0), &["k1", "k2"]);
    let budget = ModelCheckLimits {
        limits: Limits {
            max_states: 174,
            max_transitions: usize::MAX,
        },
        ..ModelCheckLimits::default()
    };
    let outcome = world
        .run(budget, &Cancel::new())
        .expect("the run is not refused");
    assert_eq!(
        refuted(outcome).instance,
        [("v".to_owned(), "k1".to_owned())]
    );

    let (instance, _) = refuted_instance(&both((1, 0), (1, 1), &["k1", "k2"]));
    assert_eq!(instance, [("v".to_owned(), "k2".to_owned())]);
}

/// `align skip` names an operation of the subject's model by its declaring
/// type and name (ADR-023 PA-1): the operation of another model, or another
/// type, or another name, has no operation text in this subject, so none of
/// its steps is skipped.
#[test]
fn an_align_skip_key_selects_the_operation_of_its_own_declaring_type() {
    let world = World::busy(Vault::Busy, &IDLE, &aligned(&[]));
    let system = ModelSystem::new(world.subject().simulation).expect("the subject admits");
    let key = |declaring: DeclarationKey, operation: &str| OperationKey {
        declaring,
        operation: Identifier::new(operation).expect("an identifier"),
    };
    let mix = system
        .operation_text_of(&key(hyper_key("Vault"), "mix"))
        .expect("the vault declares mix");
    let step = system
        .operation_text_of(&key(hyper_key("Vault"), "step"))
        .expect("the vault declares step");
    assert_ne!(mix, step);
    assert!(mix.contains("/Vault/mix\""));
    // The same name on another type of the package, a name the type does not
    // declare, and a type of another model.
    assert_eq!(
        system.operation_text_of(&key(hyper_key("Bit"), "mix")),
        None
    );
    assert_eq!(
        system.operation_text_of(&key(hyper_key("Vault"), "reset")),
        None
    );
    let other_model = DeclarationKey {
        package: "test/other".to_owned(),
        node: hyper_key("Vault").node,
    };
    assert_eq!(system.operation_text_of(&key(other_model, "mix")), None);
}
