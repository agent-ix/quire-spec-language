// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-177 (TC-602), FR-184 (TC-609): EN-1 checks a forall-exists safety
//! hyperproperty by witness sets, on ADR-023 section 8.2's `Opaque` over the
//! vault of section 8 (`crate::hyper_world`).
//!
//! Every expected verdict below is derived by hand from the vault's two
//! transition relations (leaky `l = i xor h`, secure `l = i`; `h` never
//! changes) and from the witness-set product's definition (ADR-023 HC-2).
//! The tests that read the product itself explore it through
//! `qsl-eval`'s `WitnessProduct` and the engine's own retained graph.
//!
//! Not claimed here: FR-183-AC-6 (the reader and replay of a counterexample
//! with an existential path) and the recomputation of `X` by replay, which are
//! FR-183's; FR-184-AC-3 (`max_relation_tuples` on FR-179's `Det`), which
//! needs the HP-1 engine; and the "marked as published defaults" half of
//! FR-184-AC-1, which is a member of FR-182's terminal record.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{HyperRefusal, ModelCheckRefusal};
use qsl_eval::hyper::{
    HyperLetters, MatchReading, ParameterBinding, Property, StepLabel, StepMatch,
};
use qsl_eval::simulation::model::ModelSimState;
use qsl_eval::temporal::{AtomReading, UndefinedCause};
use qsl_replay::{
    HyperCounterexampleKind, HyperLocus, HyperOutcome, HyperStep, IncompleteCause, ModelCheckLimit,
    ModelCheckLimits, ModelTraceRefusal, ReachedLimit, ReplayRefusal, UndecidedCause,
    DEFAULT_MAX_RELATION_TUPLES, DEFAULT_MAX_WITNESS_SET,
};
use qsl_semantics::check::FormulaNodeId;
use quire_exact::{Cancel, CancelCause, Charge, ChargePoint, Incomplete, Meter, Undefined};

use crate::counter::ZERO;
use crate::hyper::{inputs, positions, refuted};
use crate::hyper_world::{high, low, Vault, World, STARTS};

/// ADR-023 section 8.2's `Opaque`, with `conjuncts` added to its `match`.
fn opaque(conjuncts: &str) -> String {
    format!(
        "hyper Opaque using inf over behaviours (v: V::Vault) \
         forall trace a of V exists trace b of V \
         match {{ a.step.Vault::step.i = b.step.Vault::step.i{conjuncts} }} \
         {{ holds(v.h @ a != v.h @ b) and always holds(v.l @ a = v.l @ b) }}"
    )
}

/// FR-177-AC-3's clause: two existential variables and no universal one.
const TWO_EXISTENTIAL: &str = "hyper Hidden using inf over behaviours (v: V::Vault) \
     exists trace b of V exists trace c of V \
     match { b.step.Vault::step.i = c.step.Vault::step.i } \
     { holds(v.h @ b != v.h @ c) and always holds(v.l @ b = v.l @ c) }";

fn leaky() -> World {
    World::new(Vault::Leaky, false, &STARTS, &opaque(""))
}

fn secure() -> World {
    World::new(Vault::Secure, false, &STARTS, &opaque(""))
}

/// The default limits with `max_witness_set` set to `members`.
fn witness_budget(members: u64) -> ModelCheckLimits {
    ModelCheckLimits {
        max_witness_set: members,
        ..ModelCheckLimits::default()
    }
}

fn run(world: &World, limits: ModelCheckLimits) -> HyperOutcome {
    world
        .run(limits, &Cancel::new())
        .expect("the run is not refused")
}

/// The `(h, l)` of each existential member of the witness set of `state`.
fn members(state: &qsl_eval::hyper::HyperState) -> Vec<(i64, i64)> {
    let Property::Witnesses(set) = &state.property else {
        panic!("an HP-3 state holds a witness set");
    };
    set.members()
        .iter()
        .map(|member| (high(&member.states()[0]), low(&member.states()[0])))
        .collect()
}

/// FR-177-AC-1: `Opaque` over the leaky vault. From `a = (0, 0)`, `X_0` is
/// `{((1, 0), g)}` and `X_1` is empty; the product has 6 reachable states,
/// the 2 initial ones with a singleton `X` and the 4 universal states with
/// `X` empty; the safety automaton has the two states `init` and `g`. The
/// item returns `Violated` with `WitnessExhausted{position: 1}` whose one
/// trace is `a`'s lasso `(0, 0) -step(0)-> (0, 0)` with loop entry 0.
#[trace("TC-602", "FR-177-AC-1")]
#[test]
fn the_leaky_vault_exhausts_the_witness_set_at_position_one() {
    let world = leaky();
    let explored = world.explore_witness();
    let states = explored.graph.states();
    assert_eq!(states.len(), 6);
    assert_eq!(explored.automaton_states, 2);
    let initial: Vec<_> = explored
        .graph
        .initial()
        .iter()
        .map(|id| {
            let state = &explored.graph.state(*id).expect("retained").state;
            (
                (high(&state.components[0]), low(&state.components[0])),
                members(state),
            )
        })
        .collect();
    // `X_0` of `a = (0, 0)` holds the partner with the other secret, and of
    // `a = (1, 0)` the symmetric one.
    assert_eq!(initial, [((0, 0), vec![(1, 0)]), ((1, 0), vec![(0, 0)])]);
    let empty = states
        .iter()
        .filter(|state| members(&state.state).is_empty())
        .count();
    assert_eq!(
        empty, 4,
        "after one step every member has left the automaton"
    );
    assert!(states
        .iter()
        .all(|state| state.depth == 0 || members(&state.state).is_empty()));

    let outcome = run(&world, ModelCheckLimits::default());
    let HyperOutcome::Violated { stats, .. } = &outcome else {
        panic!("expected a violation, got {outcome:?}");
    };
    assert_eq!(stats.product_states, 6);
    let counterexample = refuted(outcome);
    assert_eq!(
        counterexample.kind,
        HyperCounterexampleKind::WitnessExhausted {
            loop_entry: 0,
            position: 1
        }
    );
    assert_eq!(counterexample.trace_position, 1);
    assert_eq!(counterexample.traces.len(), 1, "the universal trace alone");
    let trace = &counterexample.traces[0];
    assert_eq!((trace.variable.as_str(), trace.alias.as_str()), ("a", "V"));
    assert_eq!(trace.initial, 0, "from (0, 0)");
    assert_eq!(inputs(&counterexample, 0), [0]);
    assert_eq!(positions(&world, &counterexample, 0), [(0, 0), (0, 0)]);
}

/// FR-177-AC-2: `Opaque` over the secure vault. Every reachable universal
/// state `(h, l)` has `X = {((1 - h, l), g)}`, the product has 4 states, and
/// the item returns `Holds`.
#[trace("TC-602", "FR-177-AC-2")]
#[test]
fn the_secure_vault_keeps_a_singleton_witness_set_over_four_states() {
    let world = secure();
    let explored = world.explore_witness();
    let states = explored.graph.states();
    assert_eq!(states.len(), 4);
    let mut seen = Vec::new();
    for state in states {
        let universal = (
            high(&state.state.components[0]),
            low(&state.state.components[0]),
        );
        assert_eq!(
            members(&state.state),
            [(1 - universal.0, universal.1)],
            "X of {universal:?}"
        );
        seen.push(universal);
    }
    seen.sort_unstable();
    assert_eq!(seen, [(0, 0), (0, 1), (1, 0), (1, 1)]);

    let outcome = run(&world, ModelCheckLimits::default());
    let HyperOutcome::Holds { stats } = outcome else {
        panic!("expected the secure vault to hold, got {outcome:?}");
    };
    assert_eq!(stats.product_states, 4);
}

/// FR-177-AC-3: the `exists trace b of V exists trace c of V` clause (no
/// universal variable) holds over the secure vault and is violated over the
/// leaky one; with no universal variable the one universal tuple is the
/// empty tuple and every joint step is an existential one.
#[trace("TC-602", "FR-177-AC-3")]
#[test]
fn a_clause_with_no_universal_variable_holds_exactly_when_the_witness_set_never_empties() {
    let secure = World::new(Vault::Secure, false, &STARTS, TWO_EXISTENTIAL);
    let explored = secure.explore_witness();
    // `X_0` holds the two ordered pairs of distinct secrets with `l = 0`;
    // the one joint step, an existential one, takes both inputs, so `X_1`
    // holds those pairs at `l = 0` and at `l = 1` and carries them over.
    let sizes: Vec<_> = explored
        .graph
        .states()
        .iter()
        .map(|state| match &state.state.property {
            Property::Witnesses(set) => set.members().len(),
            other => panic!("an HP-3 state holds a witness set, not {other:?}"),
        })
        .collect();
    assert_eq!(sizes, [2, 4]);
    assert_eq!(
        run(&secure, ModelCheckLimits::default()),
        HyperOutcome::Holds {
            stats: qsl_replay::RunStats {
                product_states: 2,
                product_edges: 2,
                automaton_states: 2,
                depth: 1
            }
        }
    );

    let leaky = World::new(Vault::Leaky, false, &STARTS, TWO_EXISTENTIAL);
    let counterexample = refuted(run(&leaky, ModelCheckLimits::default()));
    assert_eq!(
        counterexample.kind,
        HyperCounterexampleKind::WitnessExhausted {
            loop_entry: 0,
            position: 1
        }
    );
    assert!(counterexample.traces.is_empty(), "no universal variable");
}

/// FR-177-AC-4: `Opaque` over the leaky vault with `max_witness_set` 0
/// returns `Stopped(ResourceExhausted, MaxWitnessSet)` naming the limit and
/// its value, and the count the set would have reached; with the default it
/// returns AC-1's outcome.
#[trace("TC-602", "FR-177-AC-4")]
#[test]
fn a_witness_set_over_its_budget_stops_the_run() {
    let world = leaky();
    let none = witness_budget(0);
    assert_eq!(
        run(&world, none),
        HyperOutcome::Stopped {
            cause: IncompleteCause::ResourceExhausted,
            limit: Some(ReachedLimit {
                limit: ModelCheckLimit::MaxWitnessSet,
                value: 0
            }),
            count: Some(1),
            stats: qsl_replay::RunStats::default(),
        }
    );
    let default = refuted(run(&world, ModelCheckLimits::default()));
    assert_eq!(
        default.kind,
        HyperCounterexampleKind::WitnessExhausted {
            loop_entry: 0,
            position: 1
        }
    );
}

/// FR-177-AC-5: `Opaque` over the secure vault with the added `match`
/// conjunct `(if present(b.step.Vault::step.i) then 1 / value(b.step.Vault::step.i)
/// else 1) = 1` on the existential variable `b`, which has no value on a joint
/// step whose `b` step is `step(0)`: the item returns `Violated` with an
/// `Undefined` counterexample whose traces hold a path for `a` and for `b`,
/// ending at the first such joint step in canonical order, with cause
/// `division-by-zero`. It ties with the empty witness set that dropping the
/// undefined step leaves, at joint position 1, and the undefined evaluation
/// ranks first.
#[trace("TC-602", "FR-177-AC-5")]
#[test]
fn an_undefined_existential_match_ends_the_item_with_a_path_for_b() {
    let divided = " and (if present(b.step.Vault::step.i) then 1 / value(b.step.Vault::step.i) \
                   else 1) = 1";
    let world = World::new(Vault::Secure, false, &STARTS, &opaque(divided));
    let counterexample = refuted(run(&world, ModelCheckLimits::default()));
    let HyperCounterexampleKind::Undefined(undefined) = &counterexample.kind else {
        panic!("expected an undefined-evaluation counterexample");
    };
    assert_eq!(
        undefined.cause,
        UndefinedCause::Kernel(Undefined::DivisionByZero)
    );
    assert_eq!(
        undefined.locus,
        HyperLocus::ExistentialMatch { conjunct: 1 }
    );
    assert_eq!(undefined.position, 1);
    assert_eq!(counterexample.trace_position, 1);
    assert_eq!(undefined.transition, None);
    let variables: Vec<_> = undefined
        .variables
        .iter()
        .map(|(variable, alias)| (variable.as_str(), alias.as_str()))
        .collect();
    assert_eq!(variables, [("a", "V"), ("b", "V")]);
    // A path for `a` and one for `b`, each the single step into position 1:
    // `a` from (0, 0) and its partner from (1, 0), both taking `step(0)`.
    assert_eq!(counterexample.traces.len(), 2);
    assert_eq!(counterexample.traces[0].initial, 0);
    assert_eq!(counterexample.traces[1].initial, 1);
    assert_eq!(inputs(&counterexample, 0), [0]);
    assert_eq!(inputs(&counterexample, 1), [0]);
    assert_eq!(positions(&world, &counterexample, 0), [(0, 0), (0, 0)]);
    assert_eq!(positions(&world, &counterexample, 1), [(1, 0), (1, 0)]);
    // The named states are the two post-states, and the paths end in them.
    assert_eq!(undefined.state_keys.len(), 2);
    for (index, expected) in undefined.state_keys.iter().enumerate() {
        let HyperStep::Model(step) = &counterexample.traces[index].steps[0] else {
            panic!("a model step");
        };
        assert_eq!(&step.post_state, expected);
    }
}

/// A `match` whose existential conjuncts exhaust their meter, reading
/// everything else as the vault world does.
struct ExhaustedExistential<'w>(crate::hyper_world::Seams<'w>);

impl StepMatch for ExhaustedExistential<'_> {
    fn universal(
        &self,
        instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        self.0.universal(instance, steps, meter)
    }

    fn existential(
        &self,
        _instance: &[ParameterBinding],
        _steps: &[StepLabel<'_>],
        _meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        Meter::new(ZERO).charge(Charge::new(ChargePoint::GraphExpand))?;
        Ok(MatchReading::Holds)
    }
}

/// FR-177-AC-6: a `match` conjunct whose evaluation exhausts its meter at the
/// first joint step (incomplete) returns `Undecided(MatchUndetermined)` and
/// never `Holds`. The members it leaves out would empty the set at the same
/// joint position, and the refused `match` ranks before that.
#[trace("TC-602", "FR-177-AC-6")]
#[test]
fn an_incomplete_existential_match_leaves_the_item_undecided() {
    let world = secure();
    let seams = world.seams();
    let outcome = world
        .run_through(
            &seams,
            &ExhaustedExistential(world.seams()),
            ModelCheckLimits::default(),
            &Cancel::new(),
        )
        .expect("the run is not refused");
    let HyperOutcome::Undecided { cause, .. } = outcome else {
        panic!("expected no decision, got {outcome:?}");
    };
    assert_eq!(cause, UndecidedCause::MatchUndetermined);
}

/// A refused `match`, or one without a value, on the universal side: an HP-3
/// clause gives its counterexample no path for the existential variables
/// there, so the engine refuses to state one.
#[trace("TC-602", "FR-177-AC-5")]
#[test]
fn an_undefined_universal_match_of_an_hp3_clause_is_refused_not_guessed() {
    let divided = " and (if present(a.step.Vault::step.i) then 1 / value(a.step.Vault::step.i) \
                   else 1) = 1";
    let world = World::new(Vault::Secure, false, &STARTS, &opaque(divided));
    let refusal = world
        .run(ModelCheckLimits::default(), &Cancel::new())
        .expect_err("no existential path to state");
    assert!(
        matches!(
            refusal,
            ModelCheckRefusal::Hyper(HyperRefusal::NoPartnerPath)
        ),
        "{refusal:?}"
    );
}

/// Reads the vault world's atoms, except that every atom has no value where
/// component `b` is in a state with `l = 1` (`at_start`: with `h = 1`).
struct UndefinedWhereB<'w> {
    seams: crate::hyper_world::Seams<'w>,
    at_start: bool,
}

impl HyperLetters for UndefinedWhereB<'_> {
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &[ParameterBinding],
        states: &[&ModelSimState],
        meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        let partner = states[1];
        let undefined = if self.at_start {
            high(partner) == 1
        } else {
            low(partner) == 1
        };
        if undefined {
            return Ok(AtomReading::Undefined(UndefinedCause::Kernel(
                Undefined::DivisionByZero,
            )));
        }
        self.seams.atom(atom, instance, states, meter)
    }
}

/// FR-177 "Undefined and undetermined evaluation": a letter with an atom that
/// has no value at a joint post-position ends the item there, with one path
/// per variable. Over the secure vault with every atom undefined where `b` has
/// `l = 1`, the first such position in canonical order is the one after
/// `step(1)` from `a = (0, 0)`, whose partner also took `step(1)`.
#[trace("TC-602", "FR-177-AC-5")]
#[test]
fn a_letter_with_no_value_ends_the_item_with_a_path_for_the_partner() {
    let world = secure();
    let letters = UndefinedWhereB {
        seams: world.seams(),
        at_start: false,
    };
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
    assert_eq!(
        undefined.cause,
        UndefinedCause::Kernel(Undefined::DivisionByZero)
    );
    assert_eq!(counterexample.traces.len(), 2);
    assert_eq!(inputs(&counterexample, 0), [1]);
    assert_eq!(inputs(&counterexample, 1), [1]);
    assert_eq!(counterexample.traces[0].initial, 0);
    assert_eq!(counterexample.traces[1].initial, 1);
    assert_eq!(undefined.state_keys.len(), 2);
    for (index, expected) in undefined.state_keys.iter().enumerate() {
        let HyperStep::Model(step) = &counterexample.traces[index].steps[0] else {
            panic!("a model step");
        };
        assert_eq!(&step.post_state, expected);
    }
}

/// An atom with no value at joint position 0 ends the item there: no step on
/// either path, the partner being the initial state the atom was read at.
#[trace("TC-602", "FR-177-AC-5")]
#[test]
fn a_letter_with_no_value_at_position_zero_ends_the_item_with_empty_paths() {
    let world = secure();
    let letters = UndefinedWhereB {
        seams: world.seams(),
        at_start: true,
    };
    let counterexample = refuted(
        world
            .run_through(
                &letters,
                &world.seams(),
                ModelCheckLimits::default(),
                &Cancel::new(),
            )
            .expect("the run is not refused"),
    );
    let HyperCounterexampleKind::Undefined(undefined) = &counterexample.kind else {
        panic!("expected an undefined-evaluation counterexample");
    };
    assert_eq!(undefined.position, 0);
    assert_eq!(counterexample.trace_position, 0);
    assert_eq!(counterexample.traces.len(), 2);
    assert!(counterexample
        .traces
        .iter()
        .all(|trace| trace.steps.is_empty()));
    assert_eq!(counterexample.traces[0].initial, 0);
    assert_eq!(
        counterexample.traces[1].initial, 1,
        "the partner from (1, 0)"
    );
}

/// FR-177 "Violation": the fairness filter is scoped to the universal
/// components. With `reset` always enabled, a weak fairness constraint on it
/// for `a` makes `a`'s loop take `reset`, so the lasso of AC-1 is no longer
/// fair and the counterexample is one whose loop does.
#[trace("TC-602", "FR-177-AC-1")]
#[test]
fn a_universal_variables_fairness_set_shapes_the_lasso() {
    let clause = "hyper Opaque using inf over behaviours (v: V::Vault) \
         forall trace a of V fair weak V::Vault::reset; exists trace b of V \
         match { a.step.Vault::step.i = b.step.Vault::step.i } \
         { holds(v.h @ a != v.h @ b) and always holds(v.l @ a = v.l @ b) }";
    let world = World::new(Vault::Leaky, true, &STARTS, clause);
    let counterexample = refuted(run(&world, ModelCheckLimits::default()));
    // The stem `step(0)` into the empty set, then the loop `reset`, which is
    // not the stem's last step, so the entry stays at 1.
    assert_eq!(
        counterexample.kind,
        HyperCounterexampleKind::WitnessExhausted {
            loop_entry: 1,
            position: 1
        }
    );
    let operations: Vec<bool> = counterexample.traces[0]
        .steps
        .iter()
        .map(|step| match step {
            HyperStep::Model(step) => step.transition.operation().contains("/Vault/reset\""),
            HyperStep::TerminalStutter => panic!("the vault has no terminal state"),
        })
        .collect();
    assert_eq!(operations, [false, true]);
    assert_eq!(
        positions(&world, &counterexample, 0),
        [(0, 0), (0, 0), (0, 0)]
    );
}

/// ADR-023 HM-7 as FR-176 states it, for an HP-3 clause: a non-empty `μ_U`
/// that no universal behaviour satisfies is vacuous, never `Holds`.
#[trace("TC-602", "FR-177-AC-2")]
#[test]
fn an_unsatisfiable_universal_match_is_vacuous() {
    let clause = "hyper Opaque using inf over behaviours (v: V::Vault) \
         forall trace a of V exists trace b of V \
         match { a.step.op = stutter and a.step.Vault::step.i = b.step.Vault::step.i } \
         { holds(v.h @ a != v.h @ b) and always holds(v.l @ a = v.l @ b) }";
    let world = World::new(Vault::Secure, false, &STARTS, clause);
    let HyperOutcome::Undecided { cause, .. } = run(&world, ModelCheckLimits::default()) else {
        panic!("expected no decision");
    };
    assert_eq!(cause, UndecidedCause::VacuousMatch);
}

/// A cancelled run stops with its source and no limit.
#[trace("TC-602", "FR-177-AC-4")]
#[test]
fn a_cancelled_run_stops_with_its_source() {
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let outcome = leaky()
        .run(ModelCheckLimits::default(), &cancel)
        .expect("the run is not refused");
    let HyperOutcome::Stopped {
        cause,
        limit,
        count,
        ..
    } = outcome
    else {
        panic!("expected the run to be cancelled, got {outcome:?}");
    };
    assert_eq!(
        cause,
        IncompleteCause::Cancelled {
            source: CancelCause::Requested
        }
    );
    assert_eq!((limit, count), (None, None));
}

/// FR-177-AC-1, last sentence of FR-176-AC-4 applied: the same request twice
/// gives equal outcomes.
#[trace("TC-602", "FR-177-AC-1")]
#[test]
fn the_leaky_request_run_twice_gives_equal_outcomes() {
    let world = leaky();
    assert_eq!(
        run(&world, ModelCheckLimits::default()),
        run(&world, ModelCheckLimits::default())
    );
}

/// FR-184-AC-1: a request that sets neither member runs under 65,536 and
/// 16,777,216, whatever the verdict: a proof, a refutation and a stop. The
/// terminal record's marking of them as published defaults is FR-182's.
#[trace("TC-609", "FR-184-AC-1")]
#[test]
fn a_request_that_sets_neither_budget_runs_under_the_published_defaults() {
    let defaults = ModelCheckLimits::default();
    assert_eq!(defaults.max_witness_set, 65_536);
    assert_eq!(defaults.max_relation_tuples, 16_777_216);
    assert_eq!(DEFAULT_MAX_WITNESS_SET, 65_536);
    assert_eq!(DEFAULT_MAX_RELATION_TUPLES, 16_777_216);
    assert_eq!(
        defaults.hyper_used(),
        [
            (ModelCheckLimit::MaxWitnessSet, 65_536),
            (ModelCheckLimit::MaxRelationTuples, 16_777_216)
        ]
    );
    assert_eq!(ModelCheckLimit::MaxWitnessSet.setting(), "max_witness_set");
    assert_eq!(
        ModelCheckLimit::MaxRelationTuples.setting(),
        "max_relation_tuples"
    );
    assert!(matches!(
        run(&secure(), defaults),
        HyperOutcome::Holds { .. }
    ));
    assert!(matches!(
        run(&leaky(), defaults),
        HyperOutcome::Violated { .. }
    ));
    let mut tight = defaults;
    tight.limits.max_states = 3;
    assert!(matches!(
        run(&secure(), tight),
        HyperOutcome::Stopped { .. }
    ));
}

/// FR-184-AC-2: ADR-023 section 8.2's leaky `Opaque` with `max_witness_set` 0
/// stops with value 0, count 1 and no counterexample; with `max_witness_set` 1
/// it completes with FR-177-AC-1's outcome.
#[trace("TC-609", "FR-184-AC-2")]
#[test]
fn max_witness_set_stops_the_leaky_run_at_zero_and_completes_it_at_one() {
    let world = leaky();
    let zero = witness_budget(0);
    let HyperOutcome::Stopped {
        cause,
        limit,
        count,
        ..
    } = run(&world, zero)
    else {
        panic!("expected the run to stop");
    };
    assert_eq!(cause, IncompleteCause::ResourceExhausted);
    assert_eq!(
        limit,
        Some(ReachedLimit {
            limit: ModelCheckLimit::MaxWitnessSet,
            value: 0
        })
    );
    assert_eq!(count, Some(1));

    let one = witness_budget(1);
    assert_eq!(run(&world, one), run(&world, ModelCheckLimits::default()));
    assert!(matches!(run(&world, one), HyperOutcome::Violated { .. }));
}

/// FR-184 "Behavior": the budget counts the members of one set and stops when
/// a set would hold more. `X_0` of FR-177-AC-3's clause holds two members
/// (each ordered pair of distinct secrets) and `X_1` four: a budget of 1 stops
/// at count 2 in `X_0`, a budget of 2 at count 3 in `X_1`, and a budget of 4
/// completes. The run never continues past a reached budget.
#[trace("TC-609", "FR-184-AC-2")]
#[test]
fn the_budget_counts_the_members_of_one_set() {
    let world = World::new(Vault::Secure, false, &STARTS, TWO_EXISTENTIAL);
    let one = witness_budget(1);
    let HyperOutcome::Stopped { limit, count, .. } = run(&world, one) else {
        panic!("expected the run to stop");
    };
    assert_eq!(
        limit,
        Some(ReachedLimit {
            limit: ModelCheckLimit::MaxWitnessSet,
            value: 1
        })
    );
    assert_eq!(count, Some(2));

    let two = witness_budget(2);
    let HyperOutcome::Stopped { limit, count, .. } = run(&world, two) else {
        panic!("expected the run to stop");
    };
    assert_eq!(
        limit,
        Some(ReachedLimit {
            limit: ModelCheckLimit::MaxWitnessSet,
            value: 2
        })
    );
    assert_eq!(count, Some(3));

    let four = witness_budget(4);
    assert!(matches!(run(&world, four), HyperOutcome::Holds { .. }));
}

/// A `WitnessExhausted` counterexample is not replayed yet: FR-183 recomputes
/// the witness sets along its universal traces, and until that is built the
/// replay refuses it as unsupported rather than reading it as a lockstep one.
#[trace("TC-602", "FR-177-AC-1")]
#[test]
fn a_witness_exhausted_counterexample_is_refused_by_replay_until_fr_183() {
    let world = leaky();
    let counterexample = refuted(run(&world, ModelCheckLimits::default()));
    let Err(ReplayRefusal::ModelTrace(refusal)) = world.replay(&counterexample) else {
        panic!("expected the replay to refuse");
    };
    assert_eq!(*refusal, ModelTraceRefusal::HyperWitnessExhausted);
    assert_eq!(
        refusal.to_string(),
        "unsupported_construct/not-yet-implemented: replay of a witness-exhausted counterexample"
    );
}
