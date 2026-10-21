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
//! The replay of a `WitnessExhausted` counterexample, which recomputes the
//! witness sets (FR-183, TC-608), and of an `Undefined` one with an existential
//! path (FR-183-AC-6) are tested here beside the runs that produce them.
//!
//! Not claimed here: FR-184-AC-3 (`max_relation_tuples` on FR-179's `Det`),
//! which needs the HP-1 engine.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{HyperRefusal, ModelCheckRefusal};
use qsl_eval::hyper::{
    HyperLetters, MatchReading, ParameterBinding, Property, StepLabel, StepMatch,
};
use qsl_eval::simulation::model::ModelSimState;
use qsl_eval::temporal::{AtomReading, UndefinedCause};
use qsl_replay::{
    HyperCounterexampleKind, HyperLocus, HyperOutcome, HyperStep, IncompleteCause, ModelCheckLimit,
    ModelCheckLimits, ModelTraceRefusal, ModelTraceSettlement, ModelTraceValue, ReachedLimit,
    ReplayRefusal, UndecidedCause, DEFAULT_MAX_RELATION_TUPLES, DEFAULT_MAX_WITNESS_SET,
};
use qsl_semantics::check::FormulaNodeId;
use quire_exact::{Cancel, CancelCause, Charge, ChargePoint, Incomplete, Meter, Undefined};

use crate::counter::ZERO;
use crate::hyper::{inputs, positions, refuted};
use crate::hyper_world::{high, low, opaque, Vault, World, STARTS};

/// The clause of FR-177's third criterion: two existential variables and no universal one.
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
pub(crate) fn members(state: &qsl_eval::hyper::HyperState) -> Vec<(i64, i64)> {
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
pub(crate) struct ExhaustedExistential<'w>(pub(crate) crate::hyper_world::Seams<'w>);

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
/// joint position, and the refused `match` ranks before that. The seam is a
/// stub whose `existential` charges a meter that admits no work: no S3
/// lowering of a hyper `match` conjunct exists to call a function yet, so this
/// is the engine half of the criterion.
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

/// FR-177 "Undefined and undetermined evaluation", the universal side: a
/// `μ_U` conjunct with no value on a joint step ends the item with an
/// `Undefined` counterexample while the witness set is not empty. The
/// existential variable follows the first member's partner to the pre-state,
/// one step short of the universal path, and the evaluation names the
/// universal variable alone.
#[trace("TC-602")]
#[test]
fn an_undefined_universal_match_with_a_witness_ends_the_item_with_the_partners_path() {
    let divided = " and (if present(a.step.Vault::step.i) then 1 / value(a.step.Vault::step.i) \
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
    assert_eq!(undefined.locus, HyperLocus::Match { conjunct: 0 });
    assert_eq!(undefined.position, 1);
    assert_eq!(counterexample.trace_position, 1);
    // Every variable is named with its state: `a` after the step, the partner
    // before it.
    assert_eq!(
        undefined.variables,
        [
            ("a".to_owned(), "V".to_owned()),
            ("b".to_owned(), "V".to_owned())
        ]
    );
    assert_eq!(undefined.state_keys.len(), 2);
    assert_eq!(counterexample.traces.len(), 2);
    assert_eq!(counterexample.traces[0].initial, 0);
    assert_eq!(inputs(&counterexample, 0), [0]);
    // The partner from (1, 0) has taken no step yet.
    assert_eq!(counterexample.traces[1].initial, 1);
    assert!(counterexample.traces[1].steps.is_empty());
}

/// A `match` whose universal conjunct has no value on any joint step.
struct UndefinedUniversal<'w>(crate::hyper_world::Seams<'w>);

impl StepMatch for UndefinedUniversal<'_> {
    fn universal(
        &self,
        _instance: &[ParameterBinding],
        _steps: &[StepLabel<'_>],
        _meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        Ok(MatchReading::Undefined {
            conjunct: 0,
            cause: UndefinedCause::Kernel(Undefined::DivisionByZero),
        })
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

/// The same evaluation where the witness set is already empty: FR-177 and
/// QSpec FR-400 give the counterexample no existential path, so the engine
/// refuses to state one rather than invent it. From the one initial state the
/// two variables share the secret, `X_0` is empty and every universal step is
/// undefined, so no cycle and no earlier finding exists.
#[trace("TC-602")]
#[test]
fn an_undefined_universal_match_with_no_witness_is_refused_not_guessed() {
    let world = World::new(
        Vault::Secure,
        false,
        &STARTS[..1],
        &opaque(" and a.step.op != stutter"),
    );
    let refusal = world
        .run_through(
            &world.seams(),
            &UndefinedUniversal(world.seams()),
            ModelCheckLimits::default(),
            &Cancel::new(),
        )
        .expect_err("no existential path to state");
    assert!(
        matches!(
            refusal,
            ModelCheckRefusal::Hyper(HyperRefusal::NoPartnerPath)
        ),
        "{refusal:?}"
    );
}

/// μ_U undefined on `step(1)` from the second breadth-first layer on (the
/// fifth `μ_U` evaluation): the empty witness set at joint position 1 comes
/// strictly first, with `step(0)` at (0, 0) as its fair loop.
struct LateUndefined<'w> {
    seams: crate::hyper_world::Seams<'w>,
    calls: std::cell::Cell<usize>,
}

impl StepMatch for LateUndefined<'_> {
    fn universal(
        &self,
        instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        let calls = self.calls.get() + 1;
        self.calls.set(calls);
        let one = matches!(steps[0], StepLabel::Operation(step)
            if step.arguments().first().is_some_and(|argument| argument.contains("\"value\":\"1\"")));
        if calls >= 5 && one {
            return Ok(MatchReading::Undefined {
                conjunct: 0,
                cause: UndefinedCause::Kernel(Undefined::DivisionByZero),
            });
        }
        self.seams.universal(instance, steps, meter)
    }

    fn existential(
        &self,
        instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        self.seams.existential(instance, steps, meter)
    }
}

/// FR-177 "Undefined and undetermined evaluation": the first finding in
/// canonical order wins. A universal `μ_U` with no value at joint position 2
/// (whose counterexample FR-177 could not state, or would state later) does
/// not replace the empty witness set at position 1.
#[trace("TC-602")]
#[test]
fn a_later_undefined_evaluation_does_not_replace_an_earlier_empty_witness_set() {
    let world = World::new(
        Vault::Leaky,
        false,
        &STARTS,
        &opaque(" and a.step.op != stutter"),
    );
    let matcher = LateUndefined {
        seams: world.seams(),
        calls: std::cell::Cell::new(0),
    };
    let outcome = world
        .run_through(
            &world.seams(),
            &matcher,
            ModelCheckLimits::default(),
            &Cancel::new(),
        )
        .expect("the run is not refused");
    let counterexample = refuted(outcome);
    assert_eq!(
        counterexample.kind,
        HyperCounterexampleKind::WitnessExhausted {
            loop_entry: 0,
            position: 1
        }
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
#[trace("TC-602")]
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
#[trace("TC-602")]
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
#[trace("TC-602")]
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
#[trace("TC-602")]
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
#[trace("TC-602")]
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

/// The same request run twice, as FR-176 states for HP-2, gives equal
/// gives equal outcomes.
#[trace("TC-602")]
#[test]
fn the_leaky_request_run_twice_gives_equal_outcomes() {
    let world = leaky();
    assert_eq!(
        run(&world, ModelCheckLimits::default()),
        run(&world, ModelCheckLimits::default())
    );
}

/// First half of FR-184's first criterion: a request that sets neither member runs
/// under 65,536 and 16,777,216, whatever the verdict: a proof, a refutation
/// and a stop. The terminal record that reports them marked as published
/// defaults is FR-182's and is not built, so the criterion is not tagged.
#[trace("TC-609")]
#[test]
fn a_request_that_sets_neither_budget_runs_under_the_published_defaults() {
    let defaults = ModelCheckLimits::default();
    assert_eq!(defaults.max_witness_set, 65_536);
    assert_eq!(defaults.max_relation_tuples, 16_777_216);
    assert_eq!(DEFAULT_MAX_WITNESS_SET, 65_536);
    assert_eq!(DEFAULT_MAX_RELATION_TUPLES, 16_777_216);
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
#[trace("TC-609")]
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

/// The leaky `step`, enabled only while `l = 0`, and a `reset` that sets `l`
/// to 1 (and enabled only while `l = 0`): from `(0, 0)` a step can reach the
/// empty witness set in a state with
/// a `step(0)` loop on which `reset` is enabled and never taken, and both
/// moves lead on to the state `l = 1`, where neither is enabled.
const RESETTING: Vault = Vault::Custom(
    "pre Fresh using v on V::Vault::step { self.l = 0 }\n\
     post Step using v on V::Vault::step { (i = self.h and self.l = 0) or (i != self.h and self.l = 1) }\n\
     pre Ready using v on V::Vault::reset { self.l = 0 }\n\
     post Reset using v on V::Vault::reset { self.l = 1 }\n",
);

/// FR-177 "Violation": the counterexample is the lasso through the first
/// state whose witness set is empty, reaching its fair cycle by a connecting
/// path when the state is not on one. With weak fairness on `reset`, the
/// empty state after `step(0)` has only the unfair loop `step(0)`, so the
/// lasso goes on to the state `l = 1`, where `reset` is disabled and the
/// terminal stutter is fair; `position`
/// stays the joint position at which the set first empties, not the length of
/// the stem.
#[trace("TC-602")]
#[test]
fn the_lasso_reaches_its_fair_cycle_by_a_connecting_path() {
    let clause = "hyper Opaque using inf over behaviours (v: V::Vault) \
         forall trace a of V fair weak V::Vault::reset; exists trace b of V \
         match { a.step.Vault::step.i = b.step.Vault::step.i } \
         { holds(v.h @ a != v.h @ b) and always holds(v.l @ a = v.l @ b) }";
    let world = World::new(RESETTING, true, &STARTS, clause);
    let counterexample = refuted(run(&world, ModelCheckLimits::default()));
    assert_eq!(
        counterexample.kind,
        HyperCounterexampleKind::WitnessExhausted {
            loop_entry: 2,
            position: 1
        }
    );
    assert_eq!(counterexample.trace_position, 1);
    // The stem is `step(0)` into the empty set and one connecting step to
    // `l = 1`, where nothing is enabled; the loop is the terminal stutter.
    let steps = &counterexample.traces[0].steps;
    assert_eq!(steps.len(), 3);
    assert!(matches!(steps[0], HyperStep::Model(_)));
    assert!(matches!(steps[1], HyperStep::Model(_)));
    assert_eq!(steps[2], HyperStep::TerminalStutter);
}

/// Two instances of the clause, over two initial states that each hold a
/// vault `v` and a vault `w` with the same secret.
fn two_instances(vault: Vault, limits: &[(i64, i64)], conjuncts: &str) -> World {
    World::with_snapshots(
        vault,
        limits
            .iter()
            .map(|start| vec![("v", *start), ("w", *start)])
            .collect(),
        &["v", "w"],
        &opaque(conjuncts),
    )
}

/// FR-126 "Bindings and ranking" read for an HP-3 clause: a later instance
/// wins only with a strictly earlier finding. Both instances end at the
/// undefined `μ_E` at joint position 1, so the first instance's counterexample
/// is the one returned.
#[trace("TC-602")]
#[test]
fn a_later_instance_wins_only_with_a_strictly_earlier_finding() {
    let divided = " and (if present(b.step.Vault::step.i) then 1 / value(b.step.Vault::step.i) \
                   else 1) = 1";
    let world = two_instances(Vault::Secure, &[(0, 0), (1, 0)], divided);
    let counterexample = refuted(run(&world, ModelCheckLimits::default()));
    assert!(matches!(
        counterexample.kind,
        HyperCounterexampleKind::Undefined(_)
    ));
    assert_eq!(counterexample.instance, [("v".to_owned(), "v".to_owned())]);
}

/// The same ranking bounds a later instance: with one initial state the two
/// variables share the secret, so instance `v` is violated at joint position
/// 0, and `max_states` that instance `w` would run out of does not turn that
/// violation into a stop.
#[trace("TC-602")]
#[test]
fn a_violation_at_position_zero_bounds_the_later_instances() {
    let world = two_instances(Vault::Secure, &[(0, 0)], "");
    let full = run(&world, ModelCheckLimits::default());
    let counterexample = refuted(full);
    assert_eq!(
        counterexample.kind,
        HyperCounterexampleKind::WitnessExhausted {
            loop_entry: 0,
            position: 0
        }
    );
    assert_eq!(counterexample.instance, [("v".to_owned(), "v".to_owned())]);
    // Instance `w` is explored to the horizon that expands its initial states
    // (where a position-0 finding of its own would be made), which needs 7
    // states in all; one fewer stops it.
    for max_states in 7..=9 {
        let mut limits = ModelCheckLimits::default();
        limits.limits.max_states = max_states;
        let outcome = run(&world, limits);
        assert!(
            matches!(outcome, HyperOutcome::Violated { .. }),
            "max_states {max_states}: {outcome:?}"
        );
    }
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 6;
    assert!(matches!(run(&world, limits), HyperOutcome::Stopped { .. }));
}

/// The same bound under a search horizon: instance `w` is explored through the
/// depth that expands its initial states and no deeper, whatever the horizon,
/// so a horizon of 5 leaves the 7 to 9 states that suffice without one enough,
/// and 6 still stop it.
#[trace("TC-602")]
#[test]
fn a_horizon_deeper_than_the_ranking_depth_does_not_widen_a_later_instance() {
    let mut world = two_instances(Vault::Secure, &[(0, 0)], "");
    world.max_depth = Some(5);
    for max_states in 7..=9 {
        let mut limits = ModelCheckLimits::default();
        limits.limits.max_states = max_states;
        let outcome = run(&world, limits);
        assert!(
            matches!(outcome, HyperOutcome::Violated { .. }),
            "max_states {max_states}: {outcome:?}"
        );
    }
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 6;
    assert!(matches!(run(&world, limits), HyperOutcome::Stopped { .. }));
}

/// A clause with two universal variables: the counterexample holds a trace for
/// each of them and none for the existential one, and the universal tuple is
/// two components of every product state.
#[trace("TC-602")]
#[test]
fn two_universal_variables_each_have_a_trace_in_the_counterexample() {
    let clause = "hyper Opaque using inf over behaviours (v: V::Vault) \
         forall trace a of V forall trace c of V exists trace b of V \
         match { a.step.Vault::step.i = b.step.Vault::step.i } \
         { holds(v.h @ a != v.h @ b) and always holds(v.l @ a = v.l @ b) }";
    let world = World::new(Vault::Leaky, false, &STARTS, clause);
    let counterexample = refuted(run(&world, ModelCheckLimits::default()));
    assert_eq!(
        counterexample.kind,
        HyperCounterexampleKind::WitnessExhausted {
            loop_entry: 0,
            position: 1
        }
    );
    let variables: Vec<_> = counterexample
        .traces
        .iter()
        .map(|trace| trace.variable.as_str())
        .collect();
    assert_eq!(variables, ["a", "c"]);
    assert_eq!(counterexample.traces[0].steps.len(), 1);
    assert_eq!(counterexample.traces[1].steps.len(), 1);
    let explored = world.explore_witness();
    assert!(explored
        .graph
        .states()
        .iter()
        .all(|state| state.state.components.len() == 2));
}

/// `max_automaton_states` bounds the safety automaton of the witness product:
/// the two states `Opaque` needs do not fit in one.
#[trace("TC-602")]
#[test]
fn the_safety_automaton_stops_at_max_automaton_states() {
    let limits = ModelCheckLimits {
        max_automaton_states: 1,
        ..ModelCheckLimits::default()
    };
    let HyperOutcome::Stopped { limit, count, .. } = run(&leaky(), limits) else {
        panic!("expected the automaton to stop the run");
    };
    assert_eq!(
        limit,
        Some(ReachedLimit {
            limit: ModelCheckLimit::MaxAutomatonStates,
            value: 1
        })
    );
    assert_eq!(count, None);
}

/// The leaky `step`, with a precondition that has no value in the state
/// `(0, 1)`, which `a` never reaches under the `match` below but a partner
/// does.
pub(crate) const GUARDED: Vault = Vault::Custom(
    "pre Guard using v on V::Vault::step { 1 div (1 - self.l + self.h) >= 0 }\n\
     post Step using v on V::Vault::step { (i = self.h and self.l = 0) or (i != self.h and self.l = 1) }\n",
);

/// `Opaque` over [`GUARDED`] with a `match` that confines `a` to `step(0)`.
pub(crate) const GUARDED_CLAUSE: &str = "hyper Opaque using inf over behaviours (v: V::Vault) \
     forall trace a of V exists trace b of V \
     match { (if present(a.step.Vault::step.i) then value(a.step.Vault::step.i) else 0) = 0 \
     and b.step.op != stutter } \
     { holds(v.h @ a != v.h @ b) and always holds(v.l @ a = v.l @ b) }";

/// FR-177 states no verdict for an existential member whose contract has no
/// value (the spec follow-up), so the engine does not claim one: the
/// successor relation of the witness set is not exactly known and the item is
/// `Undecided(UndecidedSuccessor)`, never `Holds`. `a` takes only `step(0)`, so
/// it never reaches `(0, 1)`; its partner from `(0, 0)` takes `step(1)` there.
#[trace("TC-602")]
#[test]
fn an_existential_member_with_an_undefined_contract_leaves_the_item_undecided() {
    let world = World::new(GUARDED, false, &STARTS, GUARDED_CLAUSE);
    let outcome = run(&world, ModelCheckLimits::default());
    let HyperOutcome::Undecided { cause, .. } = outcome else {
        panic!("expected no decision, got {outcome:?}");
    };
    assert_eq!(cause, UndecidedCause::UndecidedSuccessor);
}

/// μ_U refused on the fifth evaluation, the first step of the state `a = (1, 1)`
/// whose member `(0, 1)` has the contract with no value, and held on the sixth,
/// the second step of that state, whatever the clause says: the state's
/// findings are then the refused `match` before the member's contract.
struct RefuseThenHold<'w> {
    seams: crate::hyper_world::Seams<'w>,
    calls: std::cell::Cell<usize>,
}

impl StepMatch for RefuseThenHold<'_> {
    fn universal(
        &self,
        instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        let calls = self.calls.get() + 1;
        self.calls.set(calls);
        match calls {
            5 => Ok(MatchReading::Refused),
            6 => Ok(MatchReading::Holds),
            _ => self.seams.universal(instance, steps, meter),
        }
    }

    fn existential(
        &self,
        instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        self.seams.existential(instance, steps, meter)
    }
}

/// A state's refused findings rank by the joint position they are at, not by
/// their order in its list: the member's contract at joint position 1 comes
/// before a refused `μ_U` at position 2 that was found first, so the cause is
/// `UndecidedSuccessor`, not `MatchUndetermined`.
#[trace("TC-602")]
#[test]
fn a_state_keeps_its_earliest_refused_finding() {
    let clause = "hyper Opaque using inf over behaviours (v: V::Vault) \
         forall trace a of V exists trace b of V \
         match { (if present(a.step.Vault::step.i) then value(a.step.Vault::step.i) else 0) = 0 \
         and b.step.op != stutter } \
         { holds(v.h @ a != v.h @ b) and always holds(v.l @ a = v.l @ b) }";
    let world = World::new(GUARDED, false, &STARTS, clause);
    let matcher = RefuseThenHold {
        seams: world.seams(),
        calls: std::cell::Cell::new(0),
    };
    let outcome = world
        .run_through(
            &world.seams(),
            &matcher,
            ModelCheckLimits::default(),
            &Cancel::new(),
        )
        .expect("the run is not refused");
    let HyperOutcome::Undecided { cause, .. } = outcome else {
        panic!("expected no decision, got {outcome:?}");
    };
    assert_eq!(cause, UndecidedCause::UndecidedSuccessor);
}

/// With two universal variables an undefined `μ_U` names all three variables,
/// each with its state, in the order of the three traces (QSpec FR-400's
/// reader rule): `a` and `c` after their step, the partner before it.
#[trace("TC-602")]
#[test]
fn an_undefined_universal_match_names_every_variable_with_two_universals() {
    let clause = "hyper Opaque using inf over behaviours (v: V::Vault) \
         forall trace a of V forall trace c of V exists trace b of V \
         match { (if present(a.step.Vault::step.i) then 1 / value(a.step.Vault::step.i) else 1) = 1 \
         and a.step.Vault::step.i = b.step.Vault::step.i } \
         { holds(v.h @ a != v.h @ b) and always holds(v.l @ a = v.l @ b) }";
    let world = World::new(Vault::Secure, false, &STARTS, clause);
    let counterexample = refuted(run(&world, ModelCheckLimits::default()));
    let HyperCounterexampleKind::Undefined(undefined) = &counterexample.kind else {
        panic!("expected an undefined-evaluation counterexample");
    };
    assert_eq!(undefined.locus, HyperLocus::Match { conjunct: 0 });
    let traced: Vec<_> = counterexample
        .traces
        .iter()
        .map(|trace| (trace.variable.as_str(), trace.steps.len()))
        .collect();
    assert_eq!(traced, [("a", 1), ("c", 1), ("b", 0)]);
    let named: Vec<_> = undefined
        .variables
        .iter()
        .map(|(variable, _)| variable.as_str())
        .collect();
    assert_eq!(named, ["a", "c", "b"]);
    assert_eq!(undefined.state_keys.len(), 3);
}

/// Reads the vault world's atoms and `match`, except for instance `v`: its
/// atoms are all false, so no partner survives position 0, and its `μ_U` has
/// no value on any step (`undefined_at_w`: for instance `w` instead, its
/// atoms have no value at position 0).
struct PerInstance<'w> {
    seams: crate::hyper_world::Seams<'w>,
    undefined_at_w: bool,
}

impl HyperLetters for PerInstance<'_> {
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &[ParameterBinding],
        states: &[&ModelSimState],
        meter: &mut Meter,
    ) -> Result<AtomReading, Incomplete> {
        match (instance[0].instance.key.as_str(), self.undefined_at_w) {
            ("v", false) => Ok(AtomReading::False),
            ("w", true) => Ok(AtomReading::Undefined(UndefinedCause::Kernel(
                Undefined::DivisionByZero,
            ))),
            _ => self.seams.atom(atom, instance, states, meter),
        }
    }
}

impl StepMatch for PerInstance<'_> {
    fn universal(
        &self,
        instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        if instance[0].instance.key == "v" && !self.undefined_at_w {
            return Ok(MatchReading::Undefined {
                conjunct: 0,
                cause: UndefinedCause::Kernel(Undefined::DivisionByZero),
            });
        }
        self.seams.universal(instance, steps, meter)
    }

    fn existential(
        &self,
        instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<MatchReading, Incomplete> {
        self.seams.existential(instance, steps, meter)
    }
}

/// A position-0 finding of one instance is beaten by a later instance's
/// position-0 finding of an earlier rank: the first instance's empty witness
/// set (the vaults share their secret in the one initial state), the second's
/// atom with no value.
#[trace("TC-602")]
#[test]
fn a_later_instance_with_an_undefined_evaluation_at_position_zero_beats_an_empty_witness_set() {
    let world = two_instances(Vault::Secure, &[(0, 0)], "");
    let both = PerInstance {
        seams: world.seams(),
        undefined_at_w: true,
    };
    let counterexample = refuted(
        world
            .run_through(&both, &both, ModelCheckLimits::default(), &Cancel::new())
            .expect("the run is not refused"),
    );
    assert!(matches!(
        counterexample.kind,
        HyperCounterexampleKind::Undefined(_)
    ));
    assert_eq!(counterexample.instance, [("v".to_owned(), "w".to_owned())]);
    assert_eq!(counterexample.trace_position, 0);
}

/// The undefined evaluation FR-177 cannot state ranks as the undefined
/// evaluation it is: at one joint position it comes before another instance's
/// empty witness set. Instance `v` has an empty witness set at position 0 and
/// no value for `μ_U`, so the refusal is at position 1, where instance `w`'s
/// witness set empties.
#[trace("TC-602")]
#[test]
fn an_unstated_undefined_evaluation_ranks_before_an_empty_witness_set_at_its_position() {
    let world = two_instances(Vault::Leaky, &[(0, 0), (1, 0)], " and a.step.op != stutter");
    let both = PerInstance {
        seams: world.seams(),
        undefined_at_w: false,
    };
    let refusal = world
        .run_through(&both, &both, ModelCheckLimits::default(), &Cancel::new())
        .expect_err("the undefined evaluation comes first");
    assert!(
        matches!(
            refusal,
            ModelCheckRefusal::Hyper(HyperRefusal::NoPartnerPath)
        ),
        "{refusal:?}"
    );
}

/// The model refusal of a refused replay.
fn refusal(result: Result<ModelTraceSettlement, ReplayRefusal>) -> ModelTraceRefusal {
    match result {
        Err(ReplayRefusal::ModelTrace(refusal)) => *refusal,
        other => panic!("expected a model-trace refusal, got {other:?}"),
    }
}

/// The `Undefined` counterexample of an HP-3 clause (FR-177-AC-5), whose
/// traces include the existential variable's path to the state where `μ_E`
/// has no value, replays
/// to the `UndefinedEvaluation` it carries; with the existential trace removed
/// the reader refuses it as `invalid_runtime_input`/`invalid-value` before any
/// replay.
#[trace("TC-602", "FR-183-AC-6")]
#[test]
fn an_undefined_existential_match_counterexample_replays_and_needs_its_existential_path() {
    let divided = " and (if present(b.step.Vault::step.i) then 1 / value(b.step.Vault::step.i) \
                   else 1) = 1";
    let world = World::new(Vault::Secure, false, &STARTS, &opaque(divided));
    let counterexample = refuted(run(&world, ModelCheckLimits::default()));
    let HyperCounterexampleKind::Undefined(claimed) = counterexample.kind.clone() else {
        panic!("expected an undefined-evaluation counterexample");
    };
    assert_eq!(
        world.replay(&counterexample).expect("the replay settles"),
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::HyperUndefined(claimed),
            trace_position: Some(1),
        }
    );

    let mut universal_only = counterexample;
    universal_only.traces.truncate(1);
    let refused = refusal(world.replay(&universal_only));
    assert!(matches!(refused, ModelTraceRefusal::HyperShape { .. }));
    assert_eq!(
        (refused.code(), refused.cause()),
        (
            qsl_foundation::diagnostic::Code::InvalidRuntimeInput,
            "invalid-value"
        )
    );
}

/// A search horizon on an HP-3 item (ADR-023 HV-3): the leaky vault's
/// counterexample is the lasso at the empty set, a self-loop at product depth
/// 1, so a horizon of 1 creates that state and expands nothing past it and
/// returns `BoundReached`, while a horizon of 2 finds the counterexample an
/// unbounded run finds. A horizon of 0 expands nothing at all.
#[trace("TC-602")]
#[test]
fn a_horizon_cuts_the_witness_set_search_before_its_lasso() {
    let mut world = leaky();
    let unbounded = refuted(run(&world, ModelCheckLimits::default()));
    for depth in [0, 1] {
        world.max_depth = Some(depth);
        let HyperOutcome::BoundReached {
            depth: reached,
            covers,
            ..
        } = run(&world, ModelCheckLimits::default())
        else {
            panic!("expected the horizon to be reached at {depth}");
        };
        assert_eq!(
            (reached, covers),
            (depth as u64, qsl_replay::BoundCovers::ProductDepth)
        );
    }
    world.max_depth = Some(2);
    assert_eq!(refuted(run(&world, ModelCheckLimits::default())), unbounded);
}

/// A run that completes inside its horizon holds, as it does with none; a
/// horizon shorter than the product reaches `BoundReached` instead of `Holds`.
#[trace("TC-602")]
#[test]
fn a_witness_set_run_that_completes_inside_its_horizon_holds() {
    let mut world = secure();
    world.max_depth = Some(64);
    assert!(matches!(
        run(&world, ModelCheckLimits::default()),
        HyperOutcome::Holds { .. }
    ));
    world.max_depth = Some(1);
    assert!(matches!(
        run(&world, ModelCheckLimits::default()),
        HyperOutcome::BoundReached { depth: 1, .. }
    ));
}
