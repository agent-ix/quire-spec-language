// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-174 (TC-599): EN-1 stores one product state for each orbit of the copy
//! group of a hyper clause, and settles a request's reductions against
//! ADR-023 section 5's rows before it expands a state, on ADR-023 section
//! 8.1's vault (`crate::hyper_world`).
//!
//! The unreduced product of a clause is read through `World::explore`, which
//! builds `HyperProduct` with no copy group; the engine's own run (`World::run`)
//! applies the copy group whenever the clause has one.
//!
//! Not claimed here: the `Proved{basis: Reduced}` settlement of a proof under
//! copy-swap (FR-174-AC-2, second half), which waits for FR-163's certificate
//! checker and FR-182's settlement, neither built; the engine's outcome names
//! the reduction as `HoldsReduced{[CopySwap{..}]}`. The leaky product of
//! ADR-023 section 8.1 stores 7 states here, not the 9 of the criterion: the
//! unreduced product has the 10 states `crate::hyper` explains rather than the
//! 14 the criterion halves.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{Reductions, SymmetryDeclaration};
use qsl_eval::hyper::{HyperLetters, MatchReading, ParameterBinding, StepLabel, StepMatch};
use qsl_eval::simulation::model::ModelSimState;
use qsl_eval::temporal::{AtomReading, UndefinedCause};
use qsl_replay::{
    AppliedReduction, HyperCounterexample, HyperCounterexampleKind, HyperLocus, HyperOutcome,
    HyperReductionCause, ModelCheckLimits, ModelTraceSettlement, ReductionKind,
};
use qsl_semantics::check::{FormulaNodeId, HyperForm};
use quire_exact::{Cancel, Incomplete, Meter, Undefined};

use crate::hyper::{inputs, positions, refuted};
use crate::hyper_world::{
    key, low_of, non_interference, Seams, Vault, World, EQUAL_INPUTS, KEY, STARTS,
};

/// The classes `NonInterference` is swappable in.
fn swapped() -> AppliedReduction {
    AppliedReduction::CopySwap {
        classes: vec![vec!["a".to_owned(), "b".to_owned()]],
    }
}

fn run(world: &World) -> HyperOutcome {
    world
        .run_selecting(Reductions::default())
        .expect("not refused")
}

/// Over ADR-023 section 8.1's secure vault, `NonInterference`
/// stores 6 states against the unreduced 8 and holds, naming `CopySwap{[[a,
/// b]]}`: the 4 states whose two components are equal are fixed by the swap
/// and the 4 others pair up. Over the leaky vault it stores 7 against the
/// unreduced 10 and is refuted, as the unreduced product has its two
/// accepting components: the 4 fixed states and the 6 others in 3 pairs.
#[trace("TC-599")]
#[test]
fn the_swap_halves_the_pair_product_and_keeps_both_verdicts() {
    let secure = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let explored = secure.explore();
    assert_eq!(explored.graph.states().len(), 8);
    assert_eq!(explored.accepting_components(), 0);
    let HyperOutcome::HoldsReduced { reductions, stats } = run(&secure) else {
        panic!("expected the secure vault to hold under the swap");
    };
    assert_eq!(reductions, [swapped()]);
    assert_eq!(stats.product_states, 6);

    let leaky = World::new(
        Vault::Leaky,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let explored = leaky.explore();
    assert_eq!(explored.graph.states().len(), 10);
    assert_eq!(explored.accepting_components(), 2);
    let outcome = run(&leaky);
    assert_eq!(outcome.stats().product_states, 7);
    assert!(
        matches!(outcome, HyperOutcome::Violated { .. }),
        "{outcome:?}"
    );
}

/// A clause the swap changes gets no swap. The
/// second disjunct reads `a` alone, so exchanging `a` and `b` does not leave
/// the body equal in normal form, the copy group is empty, and the engine
/// stores the unreduced product and names no reduction.
#[trace("TC-599")]
#[test]
fn an_asymmetric_clause_is_stored_unreduced() {
    let clause = format!(
        "hyper Skewed using inf over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V {EQUAL_INPUTS} \
         {{ always (holds(v.l @ a = v.l @ b) or holds(v.l @ a = 0)) }}"
    );
    let world = World::new(Vault::Secure, false, &STARTS, &clause);
    assert!(world.clause.copy_group().classes.is_empty());
    let HyperOutcome::Holds { stats } = run(&world) else {
        panic!("expected the clause to hold with no reduction");
    };
    assert_eq!(stats.product_states, 8);
    assert_eq!(
        stats.product_states,
        u64::try_from(world.explore().graph.states().len()).expect("fits")
    );
}

/// A body whose automaton the swap permutes is stored reduced.
/// Both copies must eventually output 1; the negation is `always not` of
/// either, two operators the swap exchanges, so it permutes the automaton's
/// states by renaming the operator each records. Writing `A` for a copy at
/// `l = 0` and `B` at `l = 1`, the flipping vault from `A` and `B` gives the
/// unreduced states `(AA, ¬a)`, `(AA, ¬b)`, `(BB, dead)`, `(AA, dead)`,
/// `(AB, ¬a)`, `(BA, ¬b)`, `(AB, dead)` and `(BA, dead)`: 8. The swap fixes
/// `(AA, dead)` and `(BB, dead)` and pairs the others, `(AA, ¬a)` with
/// `(AA, ¬b)`, `(AB, ¬a)` with `(BA, ¬b)` and `(AB, dead)` with `(BA, dead)`:
/// 5. The item holds.
#[trace("TC-599")]
#[test]
fn a_swap_that_permutes_the_automaton_s_states_halves_the_product() {
    let world = World::new(
        TOGGLING,
        false,
        &[(0, 0), (0, 1)],
        &format!(
            "hyper Both using inf over behaviours (v: V::Vault) \
             forall trace a of V forall trace b of V {EQUAL_INPUTS} \
             {{ eventually holds(v.l @ a = 1) and eventually holds(v.l @ b = 1) }}"
        ),
    );
    assert_eq!(world.clause.copy_group().generators.len(), 1);
    assert_eq!(world.explore().graph.states().len(), 8);
    let HyperOutcome::HoldsReduced { reductions, stats } = run(&world) else {
        panic!("expected the item to hold under the swap");
    };
    assert_eq!(reductions, [swapped()]);
    assert_eq!(stats.product_states, 5);
}

/// A swap that moves one acceptance set of
/// the body's automaton to another does not keep acceptance on the quotient,
/// so the engine stores the product whole and names no reduction. Each copy
/// must always output 0; the negation is `eventually not` of either, two
/// acceptance sets the swap exchanges.
#[trace("TC-599")]
#[test]
fn a_swap_that_exchanges_acceptance_sets_is_not_applied() {
    let world = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &format!(
            "hyper Zeroes using inf over behaviours (v: V::Vault) \
             forall trace a of V forall trace b of V {EQUAL_INPUTS} \
             {{ always holds(v.l @ a = 0) and always holds(v.l @ b = 0) }}"
        ),
    );
    assert!(!world.clause.copy_group().classes.is_empty());
    let outcome = run(&world);
    assert_eq!(
        outcome.stats().product_states,
        u64::try_from(world.explore().graph.states().len()).expect("fits")
    );
    let counterexample = refuted(outcome);
    assert_lockstep_behaviours(&world, &counterexample);
}

/// The body of three copies that is equal under every exchange of two, and
/// the `match` of the same inputs for every pair.
fn three_copies() -> String {
    let same = |x: &str, y: &str| {
        format!("{x}.step.op = {y}.step.op and {x}.step.Vault::step.i = {y}.step.Vault::step.i")
    };
    format!(
        "hyper Three using inf over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V forall trace c of V \
         match {{ {} and {} and {} }} \
         {{ always (holds(v.l @ a = v.l @ b) and holds(v.l @ b = v.l @ c) \
         and holds(v.l @ a = v.l @ c)) }}",
        same("a", "b"),
        same("b", "c"),
        same("a", "c"),
    )
}

/// Three copies swappable in one class have the symmetric group
/// of three as their copy group. The secure product stores one state for each
/// multiset of secrets and each shared output, 4 times 2, against the
/// unreduced 8 triples of secrets times 2; the leaky one is refuted by a
/// counterexample that is a real behaviour of each of the three copies.
#[trace("TC-599")]
#[test]
fn three_copies_store_one_state_for_each_multiset_of_secrets() {
    let secure = World::new(Vault::Secure, false, &STARTS, &three_copies());
    assert_eq!(
        secure.clause.copy_group().classes,
        [["a", "b", "c"].map(str::to_owned).to_vec()]
    );
    assert_eq!(secure.explore().graph.states().len(), 16);
    let HyperOutcome::HoldsReduced { reductions, stats } = run(&secure) else {
        panic!("expected the secure vault to hold under the swaps");
    };
    assert_eq!(stats.product_states, 8);
    assert_eq!(
        reductions,
        [AppliedReduction::CopySwap {
            classes: vec![["a", "b", "c"].map(str::to_owned).to_vec()]
        }]
    );

    let leaky = World::new(Vault::Leaky, false, &STARTS, &three_copies());
    let unreduced = leaky.explore().graph.states().len();
    let outcome = run(&leaky);
    let reduced = usize::try_from(outcome.stats().product_states).expect("fits");
    assert!(reduced * 3 <= unreduced * 2, "{reduced} of {unreduced}");
    let counterexample = refuted(outcome);
    let names: Vec<_> = counterexample
        .traces
        .iter()
        .map(|trace| trace.variable.as_str())
        .collect();
    assert_eq!(names, ["a", "b", "c"]);
    assert_lockstep_behaviours(&leaky, &counterexample);
}

/// Every trace of `counterexample` is a behaviour of the vault, step for step
/// and post-state for post-state, from the initial state it names, the traces
/// have one length, and the model reproduces it (FR-183).
fn assert_lockstep_behaviours(world: &World, counterexample: &HyperCounterexample) {
    let HyperCounterexampleKind::Lockstep { loop_entry } = counterexample.kind else {
        panic!(
            "expected a lockstep counterexample: {:?}",
            counterexample.kind
        );
    };
    let length = counterexample.traces[0].steps.len();
    assert!(loop_entry <= u64::try_from(length).expect("fits"));
    for (index, trace) in counterexample.traces.iter().enumerate() {
        assert_eq!(trace.steps.len(), length);
        assert_eq!(positions(world, counterexample, index).len(), length + 1);
        assert_eq!(inputs(counterexample, index).len(), length);
    }
    assert!(matches!(
        world.replay(counterexample),
        Ok(ModelTraceSettlement::Reproduced { .. })
    ));
}

/// The leaky refutation under copy-swap is an ordinary
/// `HyperCounterexample`: the traces are `a`'s and `b`'s in quantifier order,
/// each a behaviour of the vault from its own initial state, `a` from
/// `(0, 0)` and `b` from `(1, 0)`, with the outputs differing at joint
/// position 1, and it replays (FR-183). Nothing in it names a canonical
/// state.
#[trace("TC-599", "FR-174-AC-3")]
#[test]
fn the_leaky_refutation_under_the_swap_is_a_concrete_counterexample_that_replays() {
    let world = World::new(
        Vault::Leaky,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let counterexample = refuted(run(&world));
    let names: Vec<_> = counterexample
        .traces
        .iter()
        .map(|trace| (trace.variable.as_str(), trace.alias.as_str(), trace.initial))
        .collect();
    assert_eq!(names, [("a", "V", 0), ("b", "V", 1)]);
    assert_eq!(
        counterexample.kind,
        HyperCounterexampleKind::Lockstep { loop_entry: 1 }
    );
    assert_eq!(counterexample.trace_position, 1);
    let a = positions(&world, &counterexample, 0);
    let b = positions(&world, &counterexample, 1);
    assert_eq!(a, [(0, 0), (0, 0), (0, 0)]);
    assert_eq!(b, [(1, 0), (1, 1), (1, 1)]);
    assert_lockstep_behaviours(&world, &counterexample);
}

/// A path of the quotient that lands on a swapped state is
/// followed through the swap. With no `match` the secure vault pairs unequal
/// inputs, `(step(0), step(1))` and `(step(1), step(0))` reach swapped states
/// that the quotient stores once; the refutation it finds is still each
/// copy's own behaviour.
#[trace("TC-599", "FR-174-AC-3")]
#[test]
fn a_path_through_swapped_states_is_lifted_to_each_copy_s_own_behaviour() {
    let world = World::new(Vault::Secure, false, &STARTS, &non_interference(""));
    assert!(!world.clause.copy_group().classes.is_empty());
    let counterexample = refuted(run(&world));
    assert_lockstep_behaviours(&world, &counterexample);
    let a = positions(&world, &counterexample, 0);
    let b = positions(&world, &counterexample, 1);
    assert!(a.iter().zip(&b).any(|(a, b)| a.1 != b.1));
    // The first position at which the outputs differ is the one reported.
    let first = a.iter().zip(&b).position(|(a, b)| a.1 != b.1);
    assert_eq!(
        first,
        Some(usize::try_from(counterexample.trace_position).expect("fits"))
    );
}

/// A vault whose `step` flips `l`, whatever the input.
const TOGGLING: Vault = Vault::Custom(
    "post Step using v on V::Vault::step \
     { (pre(self.l) = 0 and self.l = 1) or (pre(self.l) = 1 and self.l = 0) }\n",
);

/// A loop of the quotient that closes on a swapped state is
/// repeated until the concrete tuple closes. From `a = (0, 0)` and `b = (0, 1)`
/// both copies flip `l` at each step, so the pair swaps its outputs: the
/// quotient stores the pair once, with one edge from it to itself that carries
/// the swap, and the concrete lasso goes round twice, a loop of two steps with
/// `a` at `l = 0, 1` and `b` at `l = 1, 0` (the order of the swap is two).
#[trace("TC-599", "FR-174-AC-3")]
#[test]
fn a_loop_closing_on_the_swapped_state_is_repeated_until_the_tuple_closes() {
    let world = World::new(
        TOGGLING,
        false,
        &[(0, 0), (0, 1)],
        &non_interference(EQUAL_INPUTS),
    );
    let counterexample = refuted(run(&world));
    assert_eq!(
        counterexample.kind,
        HyperCounterexampleKind::Lockstep { loop_entry: 0 }
    );
    assert_eq!(counterexample.trace_position, 0);
    let names: Vec<_> = counterexample
        .traces
        .iter()
        .map(|trace| (trace.variable.as_str(), trace.initial))
        .collect();
    assert_eq!(names, [("a", 0), ("b", 1)]);
    assert_eq!(
        positions(&world, &counterexample, 0),
        [(0, 0), (0, 1), (0, 0)]
    );
    assert_eq!(
        positions(&world, &counterexample, 1),
        [(0, 1), (0, 0), (0, 1)]
    );
    assert_lockstep_behaviours(&world, &counterexample);
}

/// A vault whose `step` sets `l` to the complement of its input.
const INVERTING: Vault = Vault::Custom(
    "post Step using v on V::Vault::step \
     { (i = 0 and self.l = 1) or (i = 1 and self.l = 0) }\n",
);

/// The world's letters, except that they have no value where the two outputs
/// differ: a reading that is the same for a state and its swap.
struct UndefinedWhereTheyDiffer<'w>(Seams<'w>);

impl HyperLetters for UndefinedWhereTheyDiffer<'_> {
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &[ParameterBinding],
        states: &[&ModelSimState],
        meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        if low_of(states[0], KEY) != low_of(states[1], KEY) {
            return Ok(AtomReading::Undefined(UndefinedCause::Kernel(
                Undefined::DivisionByZero,
            )));
        }
        self.0.atom(atom, instance, states, meter)
    }
}

/// The letters of a body that reads each copy's own output, with no value for
/// the atom of a copy at `l = 1` while the other is at `l = 0`: a reading that
/// moves with the copies when they are swapped.
struct UndefinedForTheCopyAhead<'w> {
    world: &'w World,
}

impl HyperLetters for UndefinedForTheCopyAhead<'_> {
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &[ParameterBinding],
        states: &[&ModelSimState],
        meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        let own = if self.world.atom_text(atom).contains("@ a") {
            0
        } else {
            1
        };
        if low_of(states[own], KEY) == 1 && low_of(states[1 - own], KEY) == 0 {
            return Ok(AtomReading::Undefined(UndefinedCause::Kernel(
                Undefined::DivisionByZero,
            )));
        }
        self.world.seams().atom(atom, instance, states, meter)
    }
}

/// The atom an undefined evaluation names is the concrete
/// tuple's. After `(step(0), step(1))`, `a` is at `l = 1` while `b` is at
/// `l = 0`, so the atom of `a` has no value; the quotient stores the swapped
/// pair, where it is the atom of the other variable that has none.
#[trace("TC-599", "FR-174-AC-3")]
#[test]
fn an_undefined_atom_names_the_atom_of_the_concrete_tuple() {
    let world = World::new(
        INVERTING,
        false,
        &[(0, 0)],
        "hyper Zeroes using inf over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V \
         { always (holds(v.l @ a = 0) and holds(v.l @ b = 0)) }",
    );
    assert_eq!(world.clause.copy_group().generators.len(), 1);
    let letters = UndefinedForTheCopyAhead { world: &world };
    let outcome = world
        .run_through(
            &letters,
            &world.seams(),
            ModelCheckLimits::default(),
            &Cancel::new(),
        )
        .expect("not refused");
    let counterexample = refuted(outcome);
    let HyperCounterexampleKind::Undefined(undefined) = &counterexample.kind else {
        panic!("expected an undefined-evaluation counterexample");
    };
    let HyperLocus::Atom(atom) = undefined.locus else {
        panic!("expected an atom: {:?}", undefined.locus);
    };
    assert_eq!(world.atom_text(atom), "holds(v.l @ a = 0)");
    assert_eq!(undefined.position, 1);
    assert_eq!(inputs(&counterexample, 0), [0]);
    assert_eq!(inputs(&counterexample, 1), [1]);
    assert_eq!(positions(&world, &counterexample, 0), [(0, 0), (0, 1)]);
    assert_eq!(positions(&world, &counterexample, 1), [(0, 0), (0, 0)]);
}

/// With three copies the stored tuple can be the concrete one
/// rotated, and the atom named is still the concrete tuple's. After
/// `(step(0), step(1), step(1))`, `a` is at `l = 1` while `b` and `c` are at
/// `l = 0`; the quotient stores the tuple sorted by output, with the copy at
/// `l = 1` last.
#[trace("TC-599", "FR-174-AC-3")]
#[test]
fn an_undefined_atom_of_three_copies_names_the_atom_of_the_concrete_tuple() {
    struct Alone<'w>(&'w World);
    impl HyperLetters for Alone<'_> {
        fn atom(
            &self,
            atom: FormulaNodeId,
            instance: &[ParameterBinding],
            states: &[&ModelSimState],
            meter: &mut Meter,
        ) -> Result<AtomReading, Incomplete> {
            let own = ["@ a", "@ b", "@ c"]
                .iter()
                .position(|variable| self.0.atom_text(atom).contains(variable))
                .expect("an atom reads a copy");
            let ones = states
                .iter()
                .filter(|state| low_of(state, KEY) == 1)
                .count();
            if low_of(states[own], KEY) == 1 && ones == 1 {
                return Ok(AtomReading::Undefined(UndefinedCause::Kernel(
                    Undefined::DivisionByZero,
                )));
            }
            self.0.seams().atom(atom, instance, states, meter)
        }
    }
    let world = World::new(
        INVERTING,
        false,
        &[(0, 0)],
        "hyper Zeroes using inf over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V forall trace c of V \
         { always (holds(v.l @ a = 0) and holds(v.l @ b = 0) and holds(v.l @ c = 0)) }",
    );
    assert_eq!(world.clause.copy_group().generators.len(), 2);
    let outcome = world
        .run_through(
            &Alone(&world),
            &world.seams(),
            ModelCheckLimits::default(),
            &Cancel::new(),
        )
        .expect("not refused");
    let counterexample = refuted(outcome);
    let HyperCounterexampleKind::Undefined(undefined) = &counterexample.kind else {
        panic!("expected an undefined-evaluation counterexample");
    };
    let HyperLocus::Atom(atom) = undefined.locus else {
        panic!("expected an atom: {:?}", undefined.locus);
    };
    assert_eq!(world.atom_text(atom), "holds(v.l @ a = 0)");
    assert_eq!(inputs(&counterexample, 0), [0]);
    assert_eq!(inputs(&counterexample, 1), [1]);
    assert_eq!(inputs(&counterexample, 2), [1]);
    assert_eq!(positions(&world, &counterexample, 0), [(0, 0), (0, 1)]);
    assert_eq!(positions(&world, &counterexample, 2), [(0, 0), (0, 0)]);
}

/// An undefined evaluation found at a stored state is reported
/// for the concrete tuple. With no `match`, `(step(0), step(1))` leaves `a` at
/// `l = 1` and `b` at `l = 0`, which the quotient stores as the swapped pair
/// `(l = 0, l = 1)`; the counterexample names the copies as they stepped.
#[trace("TC-599", "FR-174-AC-3")]
#[test]
fn an_undefined_letter_is_reported_for_the_copies_as_they_stepped() {
    let world = World::new(INVERTING, false, &[(0, 0)], &non_interference(""));
    assert!(!world.clause.copy_group().classes.is_empty());
    let letters = UndefinedWhereTheyDiffer(world.seams());
    let outcome = world
        .run_through(
            &letters,
            &world.seams(),
            ModelCheckLimits::default(),
            &Cancel::new(),
        )
        .expect("not refused");
    let counterexample = refuted(outcome);
    let HyperCounterexampleKind::Undefined(undefined) = &counterexample.kind else {
        panic!("expected an undefined-evaluation counterexample");
    };
    assert!(matches!(undefined.locus, HyperLocus::Atom(_)));
    assert_eq!(undefined.position, 1);
    assert_eq!(inputs(&counterexample, 0), [0]);
    assert_eq!(inputs(&counterexample, 1), [1]);
    assert_eq!(positions(&world, &counterexample, 0), [(0, 0), (0, 1)]);
    assert_eq!(positions(&world, &counterexample, 1), [(0, 0), (0, 0)]);
    // The states the evaluation names are the concrete ones, `a`'s first.
    let reached: Vec<_> = counterexample
        .traces
        .iter()
        .map(|trace| match trace.steps.last() {
            Some(qsl_replay::HyperStep::Model(step)) => step.post_state,
            other => panic!("a model step, not {other:?}"),
        })
        .collect();
    assert_eq!(undefined.state_keys, reached);
}

/// A vault whose `step` sets `l` to the complement of its input and is
/// enabled, with a value, only where `l` is 1: at `l = 0` its precondition
/// divides by zero.
const DIVIDING: Vault = Vault::Custom(
    "pre Positive using v on V::Vault::step { (1 div self.l) = 1 }\n\
     post Step using v on V::Vault::step { (i = 0 and self.l = 1) or (i = 1 and self.l = 0) }\n",
);

/// An undefined contract found at one component of a stored state
/// names the variable of that component in the concrete tuple. `(step(0),
/// step(1))` leaves `a` at `l = 1` and `b` at `l = 0`, whose precondition has no
/// value; the quotient stores the pair swapped, with the component at `l = 0`
/// first, and the counterexample names `b`.
#[trace("TC-599", "FR-174-AC-3")]
#[test]
fn an_undefined_contract_names_the_variable_of_the_concrete_tuple() {
    let world = World::new(DIVIDING, false, &[(0, 1)], &non_interference(""));
    assert!(!world.clause.copy_group().classes.is_empty());
    let counterexample = refuted(run(&world));
    let HyperCounterexampleKind::Undefined(undefined) = &counterexample.kind else {
        panic!("expected an undefined-evaluation counterexample");
    };
    assert!(
        matches!(undefined.locus, HyperLocus::Contract(_)),
        "{:?}",
        undefined.locus
    );
    assert_eq!(undefined.position, 1);
    assert_eq!(undefined.variables, [("b".to_owned(), "V".to_owned())]);
    assert_eq!(inputs(&counterexample, 0), [0]);
    assert_eq!(inputs(&counterexample, 1), [1]);
    assert_eq!(positions(&world, &counterexample, 0), [(0, 1), (0, 1)]);
    assert_eq!(positions(&world, &counterexample, 1), [(0, 1), (0, 0)]);
    let Some(qsl_replay::HyperStep::Model(step)) = counterexample.traces[1].steps.last() else {
        panic!("a model step");
    };
    assert_eq!(undefined.state_keys, [step.post_state]);
}

/// Partial-order reduction settles `NonInterference`
/// `ReductionNotPreserving` before any state is expanded, for every hyper form
/// (ADR-023 PH-2); so does a state constraint for `Opaque` (PH-6) and key
/// symmetry for `SavesPower`'s two subjects (PH-3).
#[trace("TC-599", "FR-174-AC-4")]
#[test]
fn a_reduction_whose_row_reads_no_settles_before_any_expansion() {
    let not_preserving = |outcome: Result<HyperOutcome, _>| match outcome.expect("not refused") {
        HyperOutcome::Reduction {
            cause: HyperReductionCause::NotPreserving { reduction, form },
            stats,
        } => {
            assert_eq!(stats, qsl_replay::RunStats::default());
            (reduction, form)
        }
        other => panic!("expected ReductionNotPreserving, got {other:?}"),
    };

    let interference = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let partial_order = Reductions {
        partial_order: true,
        ..Reductions::default()
    };
    assert_eq!(
        not_preserving(interference.run_selecting(partial_order)),
        (ReductionKind::PartialOrder, HyperForm::Universal)
    );

    let opaque = World::new(
        Vault::Secure,
        false,
        &STARTS,
        "hyper Opaque using inf over behaviours (v: V::Vault) \
         forall trace a of V exists trace b of V \
         match { a.step.Vault::step.i = b.step.Vault::step.i } \
         { holds(v.h @ a != v.h @ b) and always holds(v.l @ a = v.l @ b) }",
    );
    let constraint = Reductions {
        constraint: Some("Bounded".to_owned()),
        ..Reductions::default()
    };
    assert_eq!(
        not_preserving(opaque.run_selecting(constraint)),
        (
            ReductionKind::StateConstraint,
            HyperForm::ForallExistsSafety
        )
    );

    let power = World::with_second_model(
        Vault::Secure,
        &STARTS,
        "hyper SavesPower using inf over behaviours (d: V::Vault, e: N::Vault) \
         forall trace s of V forall trace n of N \
         match { s.step.Vault::step.i = n.step.Vault::step.i } \
         { always holds(d.l @ s = e.l @ n) }",
    );
    assert_eq!(power.clause.aliases().len(), 2);
    assert_eq!(
        not_preserving(power.run_selecting(symmetry())),
        (ReductionKind::Symmetry, HyperForm::Universal)
    );
}

/// A key symmetry declaration over the vaults.
fn symmetry() -> Reductions {
    Reductions {
        symmetry: vec![SymmetryDeclaration {
            population: key("vaults"),
            classes: vec![vec!["v".to_owned(), "w".to_owned()]],
        }],
        ..Reductions::default()
    }
}

/// A reduction the table admits is not
/// refused as non-preserving. This engine does not offer key symmetry or a
/// state constraint for a hyper item, so the item settles `unsupported`
/// naming the first selected reduction, still before any state is expanded.
#[trace("TC-599")]
#[test]
fn a_reduction_whose_row_reads_yes_is_not_refused_as_non_preserving() {
    let interference = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    for (reductions, expected) in [
        (symmetry(), ReductionKind::Symmetry),
        (
            Reductions {
                constraint: Some("Bounded".to_owned()),
                ..Reductions::default()
            },
            ReductionKind::StateConstraint,
        ),
    ] {
        let outcome = interference.run_selecting(reductions).expect("not refused");
        let HyperOutcome::Reduction { cause, stats } = outcome else {
            panic!("expected a reduction cause, got {outcome:?}");
        };
        assert_eq!(
            cause,
            HyperReductionCause::Unsupported {
                reduction: expected
            }
        );
        assert_eq!(stats, qsl_replay::RunStats::default());
    }
    // No selection leaves the item to the engine.
    assert!(matches!(
        interference.run(Default::default(), &Cancel::new()),
        Ok(HyperOutcome::HoldsReduced { .. })
    ));
}

/// The engine's run of `world` stores the product whole: it names no
/// reduction and stores as many states as the unreduced product has.
fn assert_stored_whole(world: &World, outcome: &HyperOutcome) {
    assert!(!world.clause.copy_group().classes.is_empty());
    assert!(
        !matches!(outcome, HyperOutcome::HoldsReduced { .. }),
        "{outcome:?}"
    );
    assert_eq!(
        outcome.stats().product_states,
        u64::try_from(world.explore().graph.states().len()).expect("fits")
    );
}

/// The vault whose `reset` is enabled at `l = 1` and keeps it there.
const RESET_KEEPS: &str = "pre R using v on V::Vault::reset { self.l = 1 }\n\
     post Reset using v on V::Vault::reset { self.l = 1 }\n";

/// The vault whose `reset` is enabled at `l = 0` and sets it to 1.
const RESET_RAISES: &str = "pre R using v on V::Vault::reset { self.l = 0 }\n\
     post Reset using v on V::Vault::reset { self.l = 1 }\n";

/// A swap of copies that carry fairness is not applied: a fairness
/// constraint is read on the slot of the stored state, and the quotient moves
/// a concrete copy between slots wherever an edge swaps them. Every `step`
/// flips `l`, so from `a = (0, 0)` and `b = (0, 1)` the copies alternate their
/// outputs and each one's `reset` is enabled at every other step only, which
/// weak fairness allows. The pair differs forever, so the item is refuted, and
/// the counterexample is fair. Reduced, the copy at `l = 1` sits in one slot
/// and its `reset` is enabled and never taken there, which rejects the cycle
/// and would prove the item.
#[trace("TC-599")]
#[test]
fn copies_alternating_their_outputs_under_fairness_are_stored_whole() {
    let contracts: &'static str = Box::leak(
        format!(
            "post Step using v on V::Vault::step \
             {{ (pre(self.l) = 0 and self.l = 1) or (pre(self.l) = 1 and self.l = 0) }}\n{RESET_KEEPS}"
        )
        .into_boxed_str(),
    );
    let world = World::new(
        Vault::Custom(contracts),
        true,
        &[(0, 0), (0, 1)],
        "hyper P using inf over behaviours (v: V::Vault) \
         forall trace a of V fair weak V::Vault::reset; \
         forall trace b of V fair weak V::Vault::reset; \
         match { a.step.op = b.step.op } { always holds(v.l @ a = v.l @ b) }",
    );
    let outcome = run(&world);
    assert_stored_whole(&world, &outcome);
    assert_eq!(outcome.stats().product_states, 4);
    let counterexample = refuted(outcome);
    assert_eq!(positions(&world, &counterexample, 0)[..2], [(0, 0), (0, 1)]);
    assert_lockstep_behaviours(&world, &counterexample);
}

/// A swap of copies that carry fairness is not applied, under a body that
/// asks the outputs to agree infinitely often: with the leaky `step` and a
/// `reset` enabled at `l = 0`, the item is refuted from `(1, 1)`.
#[trace("TC-599")]
#[test]
fn a_liveness_body_under_fairness_is_refuted_as_the_unreduced_check_refutes_it() {
    let contracts: &'static str = Box::leak(
        format!(
            "post Step using v on V::Vault::step \
             {{ (i = self.h and self.l = 0) or (i != self.h and self.l = 1) }}\n{RESET_RAISES}"
        )
        .into_boxed_str(),
    );
    let world = World::new(
        Vault::Custom(contracts),
        true,
        &[(1, 1)],
        "hyper P using inf over behaviours (v: V::Vault) \
         forall trace a of V fair weak V::Vault::reset; \
         forall trace b of V fair weak V::Vault::reset; \
         match { a.step.op = b.step.op } { always eventually holds(v.l @ a = v.l @ b) }",
    );
    let outcome = run(&world);
    assert_stored_whole(&world, &outcome);
    assert_eq!(outcome.stats().product_states, 10);
    assert_lockstep_behaviours(&world, &refuted(outcome));
}

/// A swap of three copies that carry fairness is not applied. With `step`
/// setting `l` to its input and `reset` raising `l`, every copy eventually
/// outputs 1 on every fair behaviour, so the item holds over the 20 states of
/// the whole product; the quotient found a behaviour that replay refused as
/// unfair.
#[trace("TC-599")]
#[test]
fn three_copies_under_fairness_hold_as_the_unreduced_check_holds_them() {
    let contracts: &'static str = Box::leak(
        format!("post Step using v on V::Vault::step {{ self.l = i }}\n{RESET_RAISES}")
            .into_boxed_str(),
    );
    let world = World::new(
        Vault::Custom(contracts),
        true,
        &[(1, 0)],
        "hyper P using inf over behaviours (v: V::Vault) \
         forall trace a of V fair weak V::Vault::reset; \
         forall trace b of V fair weak V::Vault::reset; \
         forall trace c of V fair weak V::Vault::reset; \
         { eventually holds(v.l @ a = 1) and eventually holds(v.l @ b = 1) \
           and eventually holds(v.l @ c = 1) }",
    );
    let outcome = run(&world);
    assert!(matches!(outcome, HyperOutcome::Holds { .. }), "{outcome:?}");
    assert_stored_whole(&world, &outcome);
    assert_eq!(outcome.stats().product_states, 20);
}

/// Only the classes without fairness are swapped. `a` and `b` carry the same
/// fairness set and `c` and `d` none, so the copy group has two classes and
/// the engine reduces by the second alone: the outcome names `[[c, d]]`. The
/// whole product has the 16 tuples of four secrets and the 4 pairs of shared
/// outputs, 64 states; the swap of `c` and `d` takes the 4 pairs of their
/// secrets to 3, so 48.
#[trace("TC-599")]
#[test]
fn only_the_classes_without_fairness_are_swapped() {
    let world = World::new(
        Vault::Secure,
        false,
        &[(0, 0), (1, 0)],
        "hyper P using inf over behaviours (v: V::Vault) \
         forall trace a of V fair weak V::Vault::step; \
         forall trace b of V fair weak V::Vault::step; \
         forall trace c of V forall trace d of V \
         match { a.step.op = b.step.op and a.step.Vault::step.i = b.step.Vault::step.i \
                 and c.step.op = d.step.op and c.step.Vault::step.i = d.step.Vault::step.i } \
         { always (holds(v.l @ a = v.l @ b) and holds(v.l @ c = v.l @ d)) }",
    );
    assert_eq!(
        world.clause.copy_group().classes,
        [["a", "b"], ["c", "d"]].map(|class| class.map(str::to_owned).to_vec())
    );
    let unreduced = world.explore().graph.states().len();
    let HyperOutcome::HoldsReduced { reductions, stats } = run(&world) else {
        panic!("expected the item to hold under the swap of c and d");
    };
    assert_eq!(
        reductions,
        [AppliedReduction::CopySwap {
            classes: vec![vec!["c".to_owned(), "d".to_owned()]]
        }]
    );
    assert_eq!((unreduced, stats.product_states), (64, 48));
}

/// A vault whose `step` flips `l` on input 0 and keeps it on input 1.
const FLIPPING_ON_ZERO: Vault = Vault::Custom(
    "post Step using v on V::Vault::step \
     { (i = 0 and self.l != pre(self.l)) or (i = 1 and self.l = pre(self.l)) }\n",
);

/// A step taken from a swapped stored state is reported in the concrete
/// tuple's order. With the inputs required to differ, `(step(0), step(1))`
/// leaves `a` at `l = 1` and `b` at `l = 0`, which the quotient stores
/// swapped; the next joint step the refutation takes, to the state where both
/// are at 1, is stored as `(step(0), step(1))` too, and is `a` taking
/// `step(1)` and `b` taking `step(0)`.
#[trace("TC-599", "FR-174-AC-3")]
#[test]
fn a_step_out_of_a_swapped_state_is_reported_for_the_copy_that_took_it() {
    let world = World::new(
        FLIPPING_ON_ZERO,
        false,
        &[(0, 0)],
        "hyper P using inf over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V \
         match { a.step.Vault::step.i != b.step.Vault::step.i } \
         { always (holds(v.l @ a = 0) or holds(v.l @ b = 0)) }",
    );
    assert!(!world.clause.copy_group().classes.is_empty());
    let counterexample = refuted(run(&world));
    assert_eq!(inputs(&counterexample, 0)[..2], [0, 1]);
    assert_eq!(inputs(&counterexample, 1)[..2], [1, 0]);
    assert_eq!(
        positions(&world, &counterexample, 0)[..3],
        [(0, 0), (0, 1), (0, 1)]
    );
    assert_eq!(
        positions(&world, &counterexample, 1)[..3],
        [(0, 0), (0, 0), (0, 1)]
    );
    assert_lockstep_behaviours(&world, &counterexample);
}

/// A `match` with no value on a joint step of which exactly one component is a
/// `reset`.
struct UndefinedForMixedSteps<'w>(Seams<'w>);

impl StepMatch for UndefinedForMixedSteps<'_> {
    fn universal(
        &self,
        instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        let resets = steps
            .iter()
            .filter(|step| {
                matches!(step, StepLabel::Operation(t) if t.operation().contains("/Vault/reset\""))
            })
            .count();
        if resets == 1 {
            return Ok(MatchReading::Undefined {
                conjunct: 0,
                cause: UndefinedCause::Kernel(Undefined::DivisionByZero),
            });
        }
        self.0.universal(instance, steps, meter)
    }

    fn existential(
        &self,
        instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        self.0.existential(instance, steps, meter)
    }
}

/// An undefined `match` found on a joint step out of a swapped stored state
/// names the step and the post-states in the concrete tuple's order. `reset`
/// is enabled where `l = 1`; after `(step(0), step(1))` only `b` is at 1, so
/// the quotient stores the pair swapped and the joint steps with a `reset`
/// have it in the first slot. The counterexample says `a` takes the `reset`.
#[trace("TC-599", "FR-174-AC-3")]
#[test]
fn an_undefined_match_at_a_swapped_state_names_the_copy_that_took_the_step() {
    let contracts: &'static str = Box::leak(
        format!(
            "post Step using v on V::Vault::step \
             {{ (i = 0 and self.l != pre(self.l)) or (i = 1 and self.l = pre(self.l)) }}\n\
             {RESET_KEEPS}"
        )
        .into_boxed_str(),
    );
    let world = World::new(
        Vault::Custom(contracts),
        true,
        &[(0, 0)],
        "hyper P using inf over behaviours (v: V::Vault) \
         forall trace a of V forall trace b of V \
         match { a.step.Vault::step.i != b.step.Vault::step.i } \
         { always holds(v.l @ a = v.l @ b) }",
    );
    assert!(!world.clause.copy_group().classes.is_empty());
    let matcher = UndefinedForMixedSteps(world.seams());
    let outcome = world
        .run_through(
            &world.seams(),
            &matcher,
            ModelCheckLimits::default(),
            &Cancel::new(),
        )
        .expect("not refused");
    let counterexample = refuted(outcome);
    let HyperCounterexampleKind::Undefined(undefined) = &counterexample.kind else {
        panic!("expected an undefined-evaluation counterexample");
    };
    assert!(matches!(undefined.locus, HyperLocus::Match { .. }));
    assert_eq!(undefined.position, 2);
    let last = |trace: usize| match counterexample.traces[trace].steps.last() {
        Some(qsl_replay::HyperStep::Model(step)) => step.clone(),
        other => panic!("a model step, not {other:?}"),
    };
    assert!(last(0).transition.operation().contains("/Vault/reset\""));
    assert!(last(1).transition.operation().contains("/Vault/step\""));
    assert_eq!(
        undefined.state_keys,
        [last(0).post_state, last(1).post_state]
    );
    // `a` is at `l = 1` before the joint step that has no value.
    assert_eq!(positions(&world, &counterexample, 0)[1], (0, 1));
    assert_eq!(positions(&world, &counterexample, 1)[1], (0, 0));
}

/// A body that remembers: a past operator keeps its memory in the automaton's
/// state, and the swap renames the operator each memory belongs to. Over the
/// flipping vault from `A = (0, 0)` and `B = (0, 1)`, whether either copy
/// has output 1 so far is always true after position 1, so the item holds.
#[trace("TC-599")]
#[test]
fn the_swap_renames_the_memory_of_past_operators() {
    let world = World::new(
        TOGGLING,
        false,
        &[(0, 0), (0, 1)],
        &format!(
            "hyper Past using inf over behaviours (v: V::Vault) \
             forall trace a of V forall trace b of V {EQUAL_INPUTS} \
             {{ eventually always (once holds(v.l @ a = 1) and once holds(v.l @ b = 1)) }}"
        ),
    );
    assert_eq!(world.clause.copy_group().generators.len(), 1);
    let unreduced = world.explore().graph.states().len();
    let HyperOutcome::HoldsReduced { reductions, stats } = run(&world) else {
        panic!("expected the item to hold under the swap");
    };
    assert_eq!(reductions, [swapped()]);
    assert_eq!(
        (unreduced, stats.product_states),
        (PAST_UNREDUCED, PAST_REDUCED)
    );
}

/// A body with an interval: the counter of the bounded `eventually` is part of
/// the automaton's state, and the swap renames the operator each counter
/// belongs to. Both copies must output 1 within two steps, which the flipping
/// vault does.
#[trace("TC-599")]
#[test]
fn the_swap_renames_the_counters_of_interval_operators() {
    let world = World::new(
        TOGGLING,
        false,
        &[(0, 0), (0, 1)],
        &format!(
            "hyper Within using inf over behaviours (v: V::Vault) \
             forall trace a of V forall trace b of V {EQUAL_INPUTS} \
             {{ eventually[0,2] holds(v.l @ a = 1) and eventually[0,2] holds(v.l @ b = 1) }}"
        ),
    );
    assert_eq!(world.clause.copy_group().generators.len(), 1);
    let unreduced = world.explore().graph.states().len();
    let HyperOutcome::HoldsReduced { reductions, stats } = run(&world) else {
        panic!("expected the item to hold under the swap");
    };
    assert_eq!(reductions, [swapped()]);
    assert_eq!(
        (unreduced, stats.product_states),
        (INTERVAL_UNREDUCED, INTERVAL_REDUCED)
    );
}

/// The states of the whole and the quotient product of the past-operator body,
/// as measured: the swap fixes the states whose copies are equal and pairs
/// the others.
const PAST_UNREDUCED: usize = 7;
const PAST_REDUCED: u64 = 5;
/// The same for the interval body.
const INTERVAL_UNREDUCED: usize = 8;
const INTERVAL_REDUCED: u64 = 5;

/// Key symmetry for `NonInterference` with a `fair each` constraint settles
/// `ReductionNotPreserving` (ADR-023 PH-3), while the same selection with
/// `whole` fairness reads "yes" (PH-2), and key symmetry for `Opaque` reads
/// "yes" too (PH-6): both settle `unsupported`.
#[trace("TC-599")]
#[test]
fn the_key_symmetry_rows_read_the_fairness_granularity_and_the_form() {
    let each = World::new(
        Vault::Secure,
        false,
        &STARTS,
        "hyper P using inf over behaviours (v: V::Vault) \
         forall trace a of V fair weak each V::Vault::step; \
         forall trace b of V fair weak each V::Vault::step; \
         match { a.step.op = b.step.op } { always holds(v.l @ a = v.l @ b) }",
    );
    assert_eq!(
        each.run_selecting(symmetry()).expect("not refused"),
        HyperOutcome::Reduction {
            cause: HyperReductionCause::NotPreserving {
                reduction: ReductionKind::Symmetry,
                form: HyperForm::Universal
            },
            stats: qsl_replay::RunStats::default()
        }
    );
    let whole = World::new(
        Vault::Secure,
        false,
        &STARTS,
        "hyper P using inf over behaviours (v: V::Vault) \
         forall trace a of V fair weak V::Vault::step; \
         forall trace b of V fair weak V::Vault::step; \
         match { a.step.op = b.step.op } { always holds(v.l @ a = v.l @ b) }",
    );
    let opaque = World::new(
        Vault::Secure,
        false,
        &STARTS,
        "hyper Opaque using inf over behaviours (v: V::Vault) \
         forall trace a of V exists trace b of V \
         match { a.step.Vault::step.i = b.step.Vault::step.i } \
         { holds(v.h @ a != v.h @ b) and always holds(v.l @ a = v.l @ b) }",
    );
    for world in [&whole, &opaque] {
        assert_eq!(
            world.run_selecting(symmetry()).expect("not refused"),
            HyperOutcome::Reduction {
                cause: HyperReductionCause::Unsupported {
                    reduction: ReductionKind::Symmetry
                },
                stats: qsl_replay::RunStats::default()
            }
        );
    }
}
