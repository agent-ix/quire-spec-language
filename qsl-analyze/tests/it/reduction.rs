// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-021 (FR-151, FR-152, FR-158 to FR-160, FR-162): symmetry reduction
//! and state constraints on EN-1's safety items, over the subjects of
//! `crate::reduction_world`.
//!
//! ADR-021 §7.1's ConfigVersion subject is written here as three `Counter`
//! objects whose `bump` advances `v` modulo 3 (`CYCLE`); §7.3's as three
//! counters over `Int[0, 3]` that `bump` increments below 3 (`COUNT`). Every
//! counterexample is read back by re-executing its steps through
//! `ModelSystem`. Liveness items under a reduction, partial-order reduction
//! and the obligation identity are not covered here.

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::trace;
use qsl_analyze::model_check::{
    admit, generators, instance_groups, not_preserving, preserves, unoffered, ModelCheckRefusal,
    Reductions, SymmetryDeclaration, SymmetryRefusal, TableForm,
};
use qsl_eval::simulation::model::{explore_model, ModelSimState, ModelSystem};
use qsl_eval::simulation::{
    ConstraintValue, Expansion, ExpansionStop, Limits, OfferedReductions, Outcome, Permutation,
    SymmetryClass, SymmetryGroup, TransitionSystem,
};
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::diagnostic::Category;
use qsl_replay::{
    settle_model_check, AppliedReduction, CertificateRequest, Certification, CounterexampleKind,
    DigestDomain, DigestRecord, IncompleteCause, InconclusiveCause, ModelCheckItem,
    ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome, ProofBasis, ReachedLimit, ReductionCause,
    ReductionKind, ResultLabel, RunStats, SettlementBasis, SymmetryBrokenCause, TerminalValue,
    UndefinedLocus,
};
use quire_exact::Cancel;
use quire_semantic_value::fairness::PropertyForm;

use crate::reduction_world::{
    atom, counters, linked, spares, start, value, values, Atom, Clause, Shape, World, BOUNDED,
    INFINITE,
};

/// ADR-021 §7.1's update, modulo 3.
const CYCLE: &str = "post Cycle using v on M::Counter::bump { \
     (pre(self.v) < 2 and self.v - pre(self.v) = 1) or (pre(self.v) = 2 and self.v = 0) }\n";

/// ADR-021 §7.3's increment below 3, with its constraints: `Low` (`v <= 1`),
/// `Wide` (`v <= 3`), `Started` (`v >= 1`) and `Odd`, undefined at `v = 2`.
const COUNT: &str = "pre Below using v on M::Counter::bump { self.v < 3 }\n\
     post Up using v on M::Counter::bump { self.v - pre(self.v) = 1 }\n\
     invariant Low using v on M::Counter at current { self.v <= 1 }\n\
     invariant Wide using v on M::Counter at current { self.v <= 3 }\n\
     invariant Started using v on M::Counter at current { self.v >= 1 }\n\
     invariant Odd using v on M::Counter at current { 6 mod (2 - self.v) >= 0 }\n";

const KEYS: [&str; 3] = ["a", "b", "c"];

fn shape(symmetric: bool, max: u64) -> Shape {
    Shape {
        symmetric,
        max,
        link: false,
    }
}

/// §7.1's subject, annotated or not, from `starts`, with `clauses`.
fn cycle(symmetric: bool, starts: &[[i64; 3]], clauses: &[Clause<'_>]) -> World {
    let starts: Vec<_> = starts.iter().map(|values| start(*values)).collect();
    World::new(shape(symmetric, 2), CYCLE, clauses, &starts, &KEYS)
}

/// §7.3's subject, all counters at 0, with `clauses`.
fn count(clauses: &[Clause<'_>]) -> World {
    World::new(shape(false, 3), COUNT, clauses, &[start([0, 0, 0])], &KEYS)
}

fn infinite(formula: &str) -> Clause<'_> {
    Clause {
        profile: INFINITE,
        formula,
    }
}

fn keys(keys: &[&str]) -> Vec<String> {
    keys.iter().map(|key| (*key).to_owned()).collect()
}

fn class(
    population: &qsl_semantics::model::key::DeclarationKey,
    members: &[&str],
) -> SymmetryClass {
    SymmetryClass {
        population: population.clone(),
        keys: keys(members),
    }
}

/// The declaration of `classes` over `counters`.
fn symmetry(classes: &[&[&str]]) -> Reductions {
    Reductions {
        symmetry: vec![SymmetryDeclaration {
            population: counters(),
            classes: classes.iter().map(|class| keys(class)).collect(),
        }],
        ..Reductions::default()
    }
}

fn constraint(name: &str) -> Reductions {
    Reductions {
        constraint: Some(name.to_owned()),
        ..Reductions::default()
    }
}

fn partial_order() -> Reductions {
    Reductions {
        partial_order: true,
        ..Reductions::default()
    }
}

/// The permutation swapping `left` and `right` of `counters`.
fn swap(left: &str, right: &str) -> Permutation {
    Permutation::of(
        &counters(),
        [
            (left.to_owned(), right.to_owned()),
            (right.to_owned(), left.to_owned()),
        ],
    )
}

fn run(
    world: &World,
    clause: Option<usize>,
    atoms: Vec<Atom>,
    reductions: Reductions,
) -> ModelCheckOutcome {
    world
        .run(clause, atoms, reductions, None, ModelCheckLimits::default())
        .expect("checked")
}

fn settled(outcome: &ModelCheckOutcome) -> ReductionCause {
    match outcome {
        ModelCheckOutcome::Reduction { cause, stats } => {
            assert_eq!(*stats, RunStats::default(), "settled before any expansion");
            cause.clone()
        }
        other => panic!("expected a reduction cause, got {other:?}"),
    }
}

fn violation(outcome: &ModelCheckOutcome) -> &qsl_replay::TemporalCounterexample {
    match outcome {
        ModelCheckOutcome::Violated { counterexample, .. } => counterexample,
        other => panic!("expected a violation, got {other:?}"),
    }
}

/// The values of `a`, `b` and `c` at each position of the counterexample,
/// read by re-executing it.
fn path(world: &World, outcome: &ModelCheckOutcome) -> Vec<(i64, i64, i64)> {
    world
        .positions(violation(outcome))
        .iter()
        .map(values)
        .collect()
}

/// The receiver key of each step of the counterexample.
fn receivers(world: &World, outcome: &ModelCheckOutcome) -> Vec<String> {
    let states = world.positions(violation(outcome));
    states
        .windows(2)
        .map(|pair| {
            KEYS.into_iter()
                .find(|key| value(&pair[0], key) != value(&pair[1], key))
                .expect("a step changes its receiver")
                .to_owned()
        })
        .collect()
}

/// FR-151-AC-1: over the annotated §7.1 subject, `[[a, b, c]]` admits with
/// generators `(a b)` and `(a b c)`, and the claim's `over` parameter gives
/// one instance, bound to `a`, orbit `[a, b, c]`, under the class `[b, c]`.
#[trace("TC-567", "FR-151-AC-1")]
#[test]
fn a_full_class_admits_with_its_generators_and_one_instance() {
    let world = cycle(true, &[[0, 0, 0]], &[infinite("always holds(x.v <= 2)")]);
    let system = world.system();
    let declared = symmetry(&[&["c", "a", "b"]]);
    let group = admit(&system, &declared.symmetry).expect("the declaration admits");
    assert_eq!(group.classes, [class(&counters(), &["a", "b", "c"])]);
    let cycle = Permutation::of(
        &counters(),
        [("a", "b"), ("b", "c"), ("c", "a")].map(|(from, to)| (from.to_owned(), to.to_owned())),
    );
    assert_eq!(
        generators(&group),
        [(counters(), swap("a", "b")), (counters(), cycle)]
    );
    let bindings = system
        .instances(&world.clauses[0].over().1)
        .expect("the clause has instances");
    let instances = instance_groups(Some(&bindings), &group);
    assert_eq!(instances.len(), 1);
    let instance = &instances[0];
    assert_eq!(
        instance
            .representative
            .as_ref()
            .map(|binding| binding.key.as_str()),
        Some("a")
    );
    assert_eq!(instance.orbit, keys(&["a", "b", "c"]));
    assert_eq!(instance.group.classes, [class(&counters(), &["b", "c"])]);
    assert!(matches!(
        run(&world, Some(0), vec![atom(|v| Some(v <= 2))], declared),
        ModelCheckOutcome::HoldsReduced { .. }
    ));
}

/// FR-151-AC-2: each malformed declaration refuses `invalid_runtime_input`/
/// `invalid-value` before any state is expanded, naming the unannotated
/// population, the key `d` outside the universe, the key `b` in two
/// classes, and a population with no universe.
#[trace("TC-567", "FR-151-AC-2")]
#[test]
fn malformed_declarations_refuse_before_any_expansion() {
    let refusal = |world: &World, reductions: Reductions| match world.run(
        None,
        Vec::new(),
        reductions,
        None,
        ModelCheckLimits::default(),
    ) {
        Err(ModelCheckRefusal::Symmetry(refusal)) => refusal,
        other => panic!("expected a symmetry refusal, got {other:?}"),
    };
    let plain = cycle(false, &[[0, 0, 0]], &[]);
    let annotated = cycle(true, &[[0, 0, 0]], &[]);
    let refused = refusal(&plain, symmetry(&[&["a", "b", "c"]]));
    assert_eq!(
        refused,
        SymmetryRefusal::NotSymmetric {
            declaration: 0,
            population: counters()
        }
    );
    assert_eq!(
        refused.code(),
        qsl_foundation::diagnostic::Code::InvalidRuntimeInput
    );
    assert_eq!(refused.cause(), "invalid-value");
    assert_eq!(
        refusal(&annotated, symmetry(&[&["a", "b", "d"]])),
        SymmetryRefusal::UnknownKey {
            declaration: 0,
            key: "d".to_owned()
        }
    );
    assert_eq!(
        refusal(&annotated, symmetry(&[&["a", "b"], &["b", "c"]])),
        SymmetryRefusal::RepeatedKey {
            declaration: 0,
            key: "b".to_owned()
        }
    );
    let spare = Reductions {
        symmetry: vec![SymmetryDeclaration {
            population: spares(),
            classes: vec![keys(&["s", "t"])],
        }],
        ..Reductions::default()
    };
    assert_eq!(
        refusal(&annotated, spare),
        SymmetryRefusal::NoUniverse {
            declaration: 0,
            population: spares()
        }
    );
}

/// FR-151-AC-3: with `a` at 1 and `b`, `c` at 0, `[[a, b, c]]` settles
/// `SymmetryBroken{counters, InitialStatesNotClosed{0, (a b)}}` with no
/// state expanded; `[[a], [b, c]]` admits, fixing `a`, and the claim gives
/// two instances: `a` under `{id, (b c)}` and `b`, orbit `[b, c]`, under the
/// trivial group.
#[trace("TC-568", "FR-151-AC-3")]
#[test]
fn an_unclosed_initial_state_breaks_symmetry_and_a_finer_class_admits() {
    let world = cycle(true, &[[1, 0, 0]], &[infinite("always holds(x.v <= 2)")]);
    let holds = || vec![atom(|v| Some(v <= 2))];
    assert_eq!(
        settled(&run(
            &world,
            Some(0),
            holds(),
            symmetry(&[&["a", "b", "c"]])
        )),
        ReductionCause::SymmetryBroken {
            population: counters(),
            cause: SymmetryBrokenCause::InitialStatesNotClosed {
                initial: 0,
                generator: swap("a", "b"),
            },
        }
    );
    let finer = symmetry(&[&["a"], &["b", "c"]]);
    let system = world.system();
    let group = admit(&system, &finer.symmetry).expect("the declaration admits");
    assert_eq!(group.classes, [class(&counters(), &["b", "c"])]);
    let bindings = system
        .instances(&world.clauses[0].over().1)
        .expect("the clause has instances");
    let instances: Vec<_> = instance_groups(Some(&bindings), &group)
        .into_iter()
        .map(|instance| {
            (
                instance.representative.map(|binding| binding.key),
                instance.orbit,
                instance.group.classes,
            )
        })
        .collect();
    assert_eq!(
        instances,
        [
            (
                Some("a".to_owned()),
                keys(&["a"]),
                vec![class(&counters(), &["b", "c"])]
            ),
            (Some("b".to_owned()), keys(&["b", "c"]), Vec::new()),
        ]
    );
    assert!(matches!(
        run(&world, Some(0), holds(), finer),
        ModelCheckOutcome::HoldsReduced { .. }
    ));
}

/// FR-151-AC-4: the initial states `(0,0,0)` and `(1,0,0)` are not closed
/// under `[[a, b, c]]`, since `(a b)` maps the second to `(0,1,0)`; adding
/// `(0,1,0)` and `(0,0,1)` closes the set and the declaration admits.
#[trace("TC-568", "FR-151-AC-4")]
#[test]
fn a_set_of_initial_states_admits_once_it_is_closed() {
    let open = cycle(true, &[[0, 0, 0], [1, 0, 0]], &[]);
    assert_eq!(
        settled(&run(&open, None, Vec::new(), symmetry(&[&["a", "b", "c"]]))),
        ReductionCause::SymmetryBroken {
            population: counters(),
            cause: SymmetryBrokenCause::InitialStatesNotClosed {
                initial: 1,
                generator: swap("a", "b"),
            },
        }
    );
    let closed = cycle(true, &[[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]], &[]);
    assert!(matches!(
        run(&closed, None, Vec::new(), symmetry(&[&["a", "b", "c"]])),
        ModelCheckOutcome::HoldsReduced { .. }
    ));
}

fn group(classes: &[&[&str]]) -> SymmetryGroup {
    SymmetryGroup {
        classes: classes
            .iter()
            .map(|members| class(&counters(), members))
            .collect(),
    }
}

/// FR-152-AC-1: under the class `[b, c]`, `(0,1,0)` canonicalises to
/// `(0,0,1)` with `(b c)` and `(0,0,1)` to itself with the identity; under
/// `[a, b, c]`, `(2,0,1)` canonicalises to `(0,1,2)`.
#[trace("TC-569", "FR-152-AC-1")]
#[test]
fn the_sort_canonicaliser_orders_each_class_by_its_objects() {
    let world = cycle(true, &[[0, 1, 0], [0, 0, 1], [2, 0, 1]], &[]);
    let system = world.system();
    let initial = system.initial();
    let stabiliser = group(&[&["b", "c"]]);
    let (canonical, permutation) = system
        .canonicalise(&initial[0], &stabiliser)
        .expect("canonicalised");
    assert_eq!(values(&canonical), (0, 0, 1));
    assert_eq!(permutation, swap("b", "c"));
    let (canonical, permutation) = system
        .canonicalise(&initial[1], &stabiliser)
        .expect("canonicalised");
    assert_eq!(values(&canonical), (0, 0, 1));
    assert!(permutation.is_identity());
    let (canonical, _) = system
        .canonicalise(&initial[2], &group(&[&["a", "b", "c"]]))
        .expect("canonicalised");
    assert_eq!(values(&canonical), (0, 1, 2));
}

/// Every state of the §7.1 subject, all 27 value triples.
fn every_state(system: &ModelSystem<'_>) -> Vec<ModelSimState> {
    let mut seen = BTreeMap::new();
    let mut frontier = system.initial();
    while let Some(state) = frontier.pop() {
        let key = system.state_key(&state).expect("encodes").0;
        if seen.contains_key(&key) {
            continue;
        }
        let expansion = system.successors(&state).expect("expands");
        frontier.extend(expansion.successors.into_iter().map(|(_, next)| next));
        seen.insert(key, state);
    }
    seen.into_values().collect()
}

/// FR-152-AC-2: the §7.1 subject under `[[a, b, c]]` stores 10 model
/// states, one per multiset of three versions, and under the stabiliser of
/// `a` 18. Of the 27 reachable states, every one canonicalises to a form
/// that is the image of the state under the returned permutation and is its
/// own canonical form, and the forms number 10 and 18.
#[trace("TC-569", "FR-152-AC-2")]
#[test]
fn canonical_forms_coalesce_the_orbits_of_reachable_states() {
    let world = cycle(true, &[[0, 0, 0]], &[infinite("always holds(x.v <= 2)")]);
    let system = world.system();
    let states = every_state(&system);
    assert_eq!(states.len(), 27);
    for (classes, orbits) in [
        (group(&[&["a", "b", "c"]]), 10),
        (group(&[&["b", "c"]]), 18),
    ] {
        let mut forms = BTreeSet::new();
        for state in &states {
            let (canonical, permutation) = system.canonicalise(state, &classes).expect("canonical");
            let key = system.state_key(&canonical).expect("encodes").0;
            let image = system.permute(state, &permutation);
            assert_eq!(system.state_key(&image).expect("encodes").0, key);
            let (again, identity) = system
                .canonicalise(&canonical, &classes)
                .expect("canonical");
            assert_eq!(system.state_key(&again).expect("encodes").0, key);
            assert!(identity.is_identity());
            forms.insert(key);
        }
        assert_eq!(forms.len(), orbits);
    }
    let full = run(&world, None, Vec::new(), symmetry(&[&["a", "b", "c"]]));
    assert_eq!(full.stats().product_states, 10);
    let stabiliser = run(
        &world,
        Some(0),
        vec![atom(|v| Some(v <= 2))],
        symmetry(&[&["a", "b", "c"]]),
    );
    assert_eq!(stabiliser.stats().product_states, 18);
}

/// FR-152-AC-3: over `v: Int[0, 1]` and `link: Counter?`, the states `{a:
/// (0, ->b), b: (0, ->a), c: (0, none)}` and `{a: (0, none), b: (0, ->c), c:
/// (0, ->b)}` lie in one orbit; each canonicalises to a member of it, the
/// image of the state under its permutation. Over the 24 states with one
/// linked pair, each orbit has at least one canonical form and at most two.
#[trace("TC-569", "FR-152-AC-3")]
#[test]
fn objects_with_equal_sort_keys_canonicalise_into_their_orbit() {
    let shape = Shape {
        symmetric: true,
        max: 1,
        link: true,
    };
    // Every state of one linked pair and one unlinked counter.
    let mut starts = Vec::new();
    for unlinked in KEYS {
        let pair: Vec<&str> = KEYS.into_iter().filter(|key| *key != unlinked).collect();
        for bits in 0..8_i64 {
            let objects: Vec<(&str, serde_json::Value)> = KEYS
                .into_iter()
                .enumerate()
                .map(|(index, key)| {
                    let bit = (bits >> index) & 1;
                    let target = if key == unlinked {
                        None
                    } else {
                        pair.iter().copied().find(|other| *other != key)
                    };
                    (key, linked(bit, target))
                })
                .collect();
            starts.push(objects);
        }
    }
    let world = World::new(shape, "", &[], &starts, &KEYS);
    let system = world.system();
    let states = system.initial();
    let full = group(&[&["a", "b", "c"]]);
    let key = |state: &ModelSimState| system.state_key(state).expect("encodes").0;
    // The starts run over the unlinked key, then the eight value triples.
    let first = &states[16]; // c unlinked, all at 0: a->b, b->a.
    let second = &states[0]; // a unlinked, all at 0: b->c, c->b.
    let orbit_of = |state: &ModelSimState| -> BTreeSet<Vec<u8>> {
        let mut permutations = vec![Permutation::identity()];
        for _ in 0..3 {
            let mut next = permutations.clone();
            for (_, generator) in generators(&full) {
                for permutation in &permutations {
                    next.push(generator.compose(permutation));
                }
            }
            permutations = next;
        }
        permutations
            .iter()
            .map(|permutation| key(&system.permute(state, permutation)))
            .collect()
    };
    let orbit = orbit_of(first);
    assert!(
        orbit.contains(&key(second)),
        "the two states share an orbit"
    );
    for state in [first, second] {
        let (canonical, permutation) = system.canonicalise(state, &full).expect("canonical");
        assert!(orbit.contains(&key(&canonical)));
        assert_eq!(key(&system.permute(state, &permutation)), key(&canonical));
    }
    let mut forms: BTreeMap<Vec<u8>, BTreeSet<Vec<u8>>> = BTreeMap::new();
    for state in &states {
        let representative = orbit_of(state).into_iter().next().expect("an orbit");
        let (canonical, _) = system.canonicalise(state, &full).expect("canonical");
        forms
            .entry(representative)
            .or_default()
            .insert(key(&canonical));
    }
    assert_eq!(
        forms.len(),
        6,
        "one orbit per unlinked value and pair multiset"
    );
    assert!(forms.values().all(|stored| (1..=2).contains(&stored.len())));
}

/// FR-152-AC-4: canonicalising one state twice gives byte-equal state keys
/// and equal permutations; the trivial group returns the state and the
/// identity.
#[trace("TC-569", "FR-152-AC-4")]
#[test]
fn canonicalisation_is_a_function_of_the_state_and_the_group() {
    let world = cycle(true, &[[2, 0, 1]], &[]);
    let system = world.system();
    let state = &system.initial()[0];
    let full = group(&[&["a", "b", "c"]]);
    let (once, first) = system.canonicalise(state, &full).expect("canonical");
    let (twice, second) = system.canonicalise(state, &full).expect("canonical");
    assert_eq!(
        system.state_key(&once).expect("encodes").0,
        system.state_key(&twice).expect("encodes").0
    );
    assert_eq!(first, second);
    let (same, identity) = system
        .canonicalise(state, &SymmetryGroup::default())
        .expect("canonical");
    assert_eq!(
        system.state_key(&same).expect("encodes").0,
        system.state_key(state).expect("encodes").0
    );
    assert!(identity.is_identity());
}

/// FR-158-AC-1 and the safety and deadlock-freedom rows of FR-158-AC-2,
/// over §7.3 with constraint `Low`: `always holds(x.v <= 1)` is violated at
/// the boundary state `(2,0,0)` with prefix `bump(a), bump(a)`, as
/// unconstrained; `always holds(x.v <= 2)` stores 20 states, 12 of them
/// boundary states, so 8 expanded, and settles `ConstraintReached{12}`
/// where the unconstrained run is violated at `(3,0,0)`; the
/// deadlock-freedom item settles `ConstraintReached{12}` where the
/// unconstrained run finds the deadlock `(3,3,3)` after nine steps.
#[trace("TC-580", "FR-158-AC-1")]
#[test]
fn a_constraint_reports_real_violations_and_never_proves_past_its_boundary() {
    let world = count(&[
        infinite("always holds(x.v <= 1)"),
        infinite("always holds(x.v <= 2)"),
    ]);
    let low = || vec![atom(|v| Some(v <= 1))];
    let two = || vec![atom(|v| Some(v <= 2))];
    for reductions in [constraint("Low"), Reductions::default()] {
        let outcome = run(&world, Some(0), low(), reductions);
        assert_eq!(path(&world, &outcome), [(0, 0, 0), (1, 0, 0), (2, 0, 0)]);
        assert_eq!(receivers(&world, &outcome), ["a", "a"]);
        assert_eq!(violation(&outcome).kind, CounterexampleKind::Formula);
    }
    let outcome = run(&world, Some(1), two(), constraint("Low"));
    assert_eq!(outcome.stats().product_states, 20);
    assert_eq!(
        outcome,
        ModelCheckOutcome::Reduction {
            cause: ReductionCause::ConstraintReached {
                boundary_states: 12
            },
            stats: outcome.stats(),
        }
    );
    let unconstrained = run(&world, Some(1), two(), Reductions::default());
    assert_eq!(path(&world, &unconstrained).last(), Some(&(3, 0, 0)));
    let deadlock = run(&world, None, Vec::new(), constraint("Low"));
    assert!(matches!(
        deadlock,
        ModelCheckOutcome::Reduction {
            cause: ReductionCause::ConstraintReached {
                boundary_states: 12
            },
            ..
        }
    ));
    let unconstrained = run(&world, None, Vec::new(), Reductions::default());
    assert_eq!(violation(&unconstrained).kind, CounterexampleKind::Deadlock);
    assert_eq!(violation(&unconstrained).prefix.len(), 9);
    assert_eq!(path(&world, &unconstrained).last(), Some(&(3, 3, 3)));
}

/// FR-158-AC-3: a constraint undefined at `v = 2` settles the item violated
/// with `UndefinedEvaluation` at `(2,0,0)`, whose prefix re-executes; a
/// constraint name the package does not declare refuses
/// `invalid_runtime_input`/`invalid-value` naming it; a constraint false at
/// the initial state stores that one boundary state; with no universe, the
/// constrained run still requires a bound.
#[trace("TC-581", "FR-158-AC-3")]
#[test]
fn undefined_absent_and_initially_false_constraints_and_bounds() {
    let world = count(&[infinite("always holds(x.v <= 3)")]);
    let wide = || vec![atom(|v| Some(v <= 3))];
    let outcome = run(&world, Some(0), wide(), constraint("Odd"));
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &violation(&outcome).kind else {
        panic!("expected an undefined evaluation: {outcome:?}");
    };
    assert_eq!(
        evaluation.site.locus,
        UndefinedLocus::Constraint("Odd".to_owned())
    );
    assert_eq!(
        evaluation.cause,
        UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero)
    );
    assert_eq!(path(&world, &outcome), [(0, 0, 0), (1, 0, 0), (2, 0, 0)]);
    let last = world
        .positions(violation(&outcome))
        .pop()
        .expect("a last state");
    assert_eq!(
        world.system().state_key(&last).expect("encodes").1,
        evaluation.site.state_key
    );
    match world.run(
        Some(0),
        wide(),
        constraint("Absent"),
        None,
        ModelCheckLimits::default(),
    ) {
        Err(ModelCheckRefusal::UnknownConstraint { name }) => assert_eq!(name, "Absent"),
        other => panic!("expected a constraint refusal, got {other:?}"),
    }
    let started = run(&world, Some(0), wide(), constraint("Started"));
    assert_eq!(started.stats().product_states, 1);
    assert!(matches!(
        started,
        ModelCheckOutcome::Reduction {
            cause: ReductionCause::ConstraintReached { boundary_states: 1 },
            ..
        }
    ));
    let mut unbounded = count(&[infinite("always holds(x.v <= 3)")]);
    unbounded.keys = None;
    assert!(matches!(
        unbounded.run(
            Some(0),
            wide(),
            constraint("Low"),
            None,
            ModelCheckLimits::default()
        ),
        Err(ModelCheckRefusal::RequiresBound(_))
    ));
}

/// FR-158-AC-4's determinism clause: repeating the constrained run gives
/// equal outcomes and equal boundary counts.
#[test]
fn a_constrained_run_is_deterministic() {
    let world = count(&[infinite("always holds(x.v <= 2)")]);
    let first = run(
        &world,
        Some(0),
        vec![atom(|v| Some(v <= 2))],
        constraint("Low"),
    );
    let second = run(
        &world,
        Some(0),
        vec![atom(|v| Some(v <= 2))],
        constraint("Low"),
    );
    assert_eq!(first, second);
}

/// FR-159-AC-1: over the annotated §7.2-style counters, `always[0,2]
/// holds(x.v = 0)` (TP-2) and `always (holds(x.v = 0) implies
/// eventually[0,2] holds(x.v = 1))` (TP-3 with an interval) with
/// partial-order reduction settle `ReductionNotPreserving` before any
/// expansion; under symmetry both are admitted and decided.
#[trace("TC-582", "FR-159-AC-1")]
#[test]
fn position_counting_forms_refuse_partial_order_reduction() {
    let bounded = cycle(
        true,
        &[[0, 0, 0]],
        &[Clause {
            profile: BOUNDED,
            formula: "always[0,2] holds(x.v = 0)",
        }],
    );
    let interval = cycle(
        true,
        &[[0, 0, 0]],
        &[infinite(
            "always (holds(x.v = 0) implies eventually[0,2] holds(x.v = 1))",
        )],
    );
    let zero = || vec![atom(|v| Some(v == 0))];
    let both = || vec![atom(|v| Some(v == 0)), atom(|v| Some(v == 1))];
    assert_eq!(
        settled(&run(&bounded, Some(0), zero(), partial_order())),
        ReductionCause::NotPreserving {
            reduction: ReductionKind::PartialOrder,
            form: PropertyForm::BoundedMltl,
        }
    );
    assert_eq!(
        settled(&run(&interval, Some(0), both(), partial_order())),
        ReductionCause::NotPreserving {
            reduction: ReductionKind::PartialOrder,
            form: PropertyForm::Safety,
        }
    );
    for (world, atoms) in [(&bounded, zero()), (&interval, both())] {
        let outcome = run(world, Some(0), atoms, symmetry(&[&["a", "b", "c"]]));
        assert!(
            !matches!(outcome, ModelCheckOutcome::Reduction { .. }),
            "{outcome:?}"
        );
    }
}

/// FR-159-AC-2: the preservation table's partial-order rows, its symmetry
/// and constraint rows, and a TP-2 request selecting symmetry and
/// partial-order reduction naming partial-order reduction.
#[trace("TC-582", "FR-159-AC-2")]
#[test]
fn the_preservation_table_reads_as_adr_021_states() {
    let yes = [
        TableForm::ReachableInvariant,
        TableForm::Safety { interval: false },
        TableForm::Liveness { interval: false },
        TableForm::DeadlockFreedom,
    ];
    let no = [
        TableForm::BoundedMltl,
        TableForm::Safety { interval: true },
        TableForm::Liveness { interval: true },
        TableForm::RefinementSafety,
        TableForm::RefinementLiveness,
    ];
    for form in yes {
        assert!(preserves(ReductionKind::PartialOrder, form), "{form:?}");
    }
    for form in no {
        assert!(!preserves(ReductionKind::PartialOrder, form), "{form:?}");
    }
    for form in yes.into_iter().chain(no) {
        assert!(preserves(ReductionKind::Symmetry, form), "{form:?}");
        assert!(preserves(ReductionKind::StateConstraint, form), "{form:?}");
    }
    let both = Reductions {
        partial_order: true,
        ..symmetry(&[&["a", "b", "c"]])
    };
    assert_eq!(
        not_preserving(&both, TableForm::BoundedMltl),
        Some(ReductionKind::PartialOrder)
    );
    let world = cycle(
        true,
        &[[0, 0, 0]],
        &[Clause {
            profile: BOUNDED,
            formula: "always[0,2] holds(x.v = 0)",
        }],
    );
    assert_eq!(
        settled(&run(&world, Some(0), vec![atom(|v| Some(v == 0))], both)),
        ReductionCause::NotPreserving {
            reduction: ReductionKind::PartialOrder,
            form: PropertyForm::BoundedMltl,
        }
    );
}

/// A system with no reduction hooks.
struct Bare;

impl TransitionSystem for Bare {
    type State = u8;
    type TransitionId = String;
    type Key = String;
    type Finding = ();

    fn initial(&self) -> Vec<u8> {
        vec![0]
    }

    fn key(&self, state: &u8) -> String {
        state.to_string()
    }

    fn successors(&self, _state: &u8) -> Result<Expansion<String, u8, ()>, ExpansionStop> {
        Ok(Expansion {
            successors: Vec::new(),
            findings: Vec::new(),
        })
    }
}

/// FR-159-AC-3's first sentence: over a system whose `offered_reductions`
/// is `NONE`, a request selecting partial-order reduction, symmetry or a
/// constraint is not offered, naming the reduction; over `ModelSystem`,
/// symmetry and constraints are offered and a selection of each is
/// admitted, while partial-order reduction settles V-8
/// `unsupported-requested-capability`.
#[test]
fn a_reduction_the_system_does_not_offer_settles_unsupported() {
    assert_eq!(Bare.offered_reductions(), OfferedReductions::NONE);
    assert_eq!(
        unoffered(&partial_order(), &Bare),
        Some(ReductionKind::PartialOrder)
    );
    assert_eq!(
        unoffered(&symmetry(&[&["a", "b"]]), &Bare),
        Some(ReductionKind::Symmetry)
    );
    assert_eq!(
        unoffered(&constraint("Low"), &Bare),
        Some(ReductionKind::StateConstraint)
    );
    let world = count(&[infinite("always holds(x.v <= 3)")]);
    let system = world.system();
    assert_eq!(unoffered(&constraint("Low"), &system), None);
    let outcome = run(
        &world,
        Some(0),
        vec![atom(|v| Some(v <= 3))],
        partial_order(),
    );
    assert_eq!(
        settled(&outcome),
        ReductionCause::Unsupported {
            reduction: ReductionKind::PartialOrder
        }
    );
    let record = settle(&world, &outcome);
    assert_eq!(record.label, ResultLabel::Unsupported);
    assert_eq!(record.value.category(), Category::Unsupported);
}

/// FR-162-AC-1's `ModelSystem` hooks and simulation: each hook it offers
/// answers `Some`, and `explore_model` over the annotated §7.1 subject stores
/// all 27 states, as it does unannotated.
#[test]
fn model_hooks_answer_and_simulation_stays_unreduced() {
    let world = count(&[]);
    let system = world.system();
    let offered = system.offered_reductions();
    assert!(offered.symmetry && offered.constraint);
    let state = &system.initial()[0];
    assert!(system.canonical(state, &group(&[&["a", "b"]])).is_some());
    assert_eq!(
        system.holds_constraint(state, "Low"),
        Some(Ok(ConstraintValue::Holds))
    );
    assert_eq!(
        system.holds_constraint(state, "Started"),
        Some(Ok(ConstraintValue::Boundary))
    );
    for symmetric in [true, false] {
        let world = cycle(symmetric, &[[0, 0, 0]], &[]);
        let system = world.system();
        let exploration =
            explore_model(&system, 1 << 20, Limits::default(), || false).expect("explored");
        let Outcome::Exhaustive(stats) = exploration.outcome else {
            panic!(
                "expected an exhaustive exploration: {:?}",
                exploration.outcome
            );
        };
        assert_eq!(stats.states, 27);
    }
}

/// FR-162-AC-4's limit clause: `max_states` 5 over §7.3's constrained run
/// stops `ResourceExhausted` at `MaxStates`, boundary states counted.
#[test]
fn boundary_states_count_toward_max_states() {
    let world = count(&[infinite("always holds(x.v <= 2)")]);
    let limits = ModelCheckLimits {
        limits: Limits {
            max_states: 5,
            ..Limits::default()
        },
        ..ModelCheckLimits::default()
    };
    let outcome = world
        .run(
            Some(0),
            vec![atom(|v| Some(v <= 2))],
            constraint("Low"),
            None,
            limits,
        )
        .expect("checked");
    assert_eq!(
        outcome,
        ModelCheckOutcome::Stopped {
            cause: IncompleteCause::ResourceExhausted,
            limit: Some(ReachedLimit {
                limit: ModelCheckLimit::MaxStates,
                value: 5
            }),
            stats: outcome.stats(),
        }
    );
    assert_eq!(outcome.stats().product_states, 5);
}

/// FR-127's map over a reduced outcome of `world`'s subject.
fn settle(world: &World, outcome: &ModelCheckOutcome) -> qsl_replay::ModelCheckRecord {
    settle_model_check(
        outcome,
        &ModelCheckLimits::default(),
        &DigestRecord::mint(DigestDomain::PackageSemanticV2, [0; 32]),
        &CertificateRequest {
            subject: world.subject(),
            item: ModelCheckItem::DeadlockFreedom,
        },
        &Cancel::new(),
        |_| panic!("a reduced outcome settles without a replay"),
    )
}

/// FR-160-AC-1's safety rows: §7.1's instance under `[[a, b, c]]` settles
/// `proved`, `closed-scope`, `Proved{Reduced{[Symmetry{groups: [[a, b, c]],
/// orbit: [a, b, c], stabiliser: [[b, c]]}]}, Uncertified}`, success;
/// FR-159-AC-1's TP-2 request settles `inconclusive`, `unsettled`,
/// `ReductionNotPreserving{PartialOrder, TP-2}`; FR-151-AC-3's broken
/// initial state settles `SymmetryBroken`; FR-158-AC-2's runs settle
/// `ConstraintReached{12}`.
#[test]
fn reduced_outcomes_settle_through_the_one_map() {
    let world = cycle(true, &[[0, 0, 0]], &[infinite("always holds(x.v <= 2)")]);
    let bounded = cycle(
        true,
        &[[0, 0, 0]],
        &[Clause {
            profile: BOUNDED,
            formula: "always[0,2] holds(x.v = 0)",
        }],
    );
    let outcome = run(
        &world,
        Some(0),
        vec![atom(|v| Some(v <= 2))],
        symmetry(&[&["a", "b", "c"]]),
    );
    let record = settle(&world, &outcome);
    assert_eq!(
        record.value,
        TerminalValue::Proved {
            basis: ProofBasis::Reduced {
                reductions: vec![AppliedReduction::Symmetry {
                    groups: vec![class(&counters(), &["a", "b", "c"])],
                    orbit: keys(&["a", "b", "c"]),
                    stabiliser: vec![class(&counters(), &["b", "c"])],
                }],
            },
            certification: Certification::Uncertified,
        }
    );
    assert_eq!(record.label, ResultLabel::Proved);
    assert_eq!(record.basis, SettlementBasis::ClosedScope);
    assert_eq!(record.value.category(), Category::Success);

    let refused = run(
        &bounded,
        Some(0),
        vec![atom(|v| Some(v == 0))],
        partial_order(),
    );
    let record = settle(&bounded, &refused);
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReductionNotPreserving {
            reduction: ReductionKind::PartialOrder,
            form: PropertyForm::BoundedMltl,
        })
    );
    assert_eq!(record.label, ResultLabel::Inconclusive);
    assert_eq!(record.basis, SettlementBasis::Unsettled);

    let broken = cycle(true, &[[1, 0, 0]], &[]);
    let outcome = run(&broken, None, Vec::new(), symmetry(&[&["a", "b", "c"]]));
    let record = settle(&broken, &outcome);
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::SymmetryBroken {
            population: counters(),
            cause: SymmetryBrokenCause::InitialStatesNotClosed {
                initial: 0,
                generator: swap("a", "b"),
            },
        })
    );
    assert_eq!(record.value.category(), Category::Inconclusive);

    let counted = count(&[]);
    let outcome = run(&counted, None, Vec::new(), constraint("Low"));
    assert_eq!(
        settle(&counted, &outcome).value,
        TerminalValue::Inconclusive(InconclusiveCause::ConstraintReached {
            boundary_states: 12
        })
    );
}

/// FR-160-AC-2: over §7.3, `always holds(x.v <= 3)` with constraint `Wide`
/// reaches no boundary state and settles `Proved{Exhaustive, Uncertified}`;
/// `always holds(x.v <= 2)` with `Low` and `max_depth` 2 completes depth 2,
/// reaching boundary states there, and settles `ConstraintReached`, not
/// `BoundReached`.
#[test]
fn a_constraint_only_proof_is_exhaustive_and_constraint_reached_wins_over_the_horizon() {
    let world = count(&[
        infinite("always holds(x.v <= 3)"),
        infinite("always holds(x.v <= 2)"),
    ]);
    let outcome = run(
        &world,
        Some(0),
        vec![atom(|v| Some(v <= 3))],
        constraint("Wide"),
    );
    assert_eq!(
        settle(&world, &outcome).value,
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Uncertified,
        }
    );
    let horizon = world
        .run(
            Some(1),
            vec![atom(|v| Some(v <= 2))],
            constraint("Low"),
            Some(2),
            ModelCheckLimits::default(),
        )
        .expect("checked");
    assert!(
        matches!(
            settle(&world, &horizon).value,
            TerminalValue::Inconclusive(InconclusiveCause::ConstraintReached { .. })
        ),
        "{horizon:?}"
    );
}

/// FR-160-AC-3's symmetry row: §7.1 under symmetry only with `max_depth` 1
/// settles `BoundReached{depth: 1}`.
#[test]
fn a_symmetry_run_to_its_horizon_settles_bound_reached() {
    let world = cycle(true, &[[0, 0, 0]], &[]);
    let outcome = world
        .run(
            None,
            Vec::new(),
            symmetry(&[&["a", "b", "c"]]),
            Some(1),
            ModelCheckLimits::default(),
        )
        .expect("checked");
    assert_eq!(
        settle(&world, &outcome).value,
        TerminalValue::Inconclusive(InconclusiveCause::BoundReached { depth: 1 })
    );
}

/// FR-160-AC-5's run, without its replay: over the annotated §7.1 subject
/// with `[[a, b, c]]`, `always holds(6 mod (2 - x.v) >= 0)` (the division
/// written `mod`, as `crate::model_check_safety` does) for `x = a` is
/// violated with `UndefinedEvaluation`, division by zero, where `a.v = 2`;
/// its counterexample re-executes on the subject, two steps long, as the
/// unreduced run's.
#[test]
fn an_undefined_evaluation_on_a_reduced_run_refutes_with_a_concrete_prefix() {
    let world = cycle(
        true,
        &[[0, 0, 0]],
        &[infinite("always holds(6 mod (2 - x.v) >= 0)")],
    );
    let undefined = || vec![atom(|v| (v != 2).then(|| 6 % (2 - v) >= 0))];
    let reduced = run(&world, Some(0), undefined(), symmetry(&[&["a", "b", "c"]]));
    let unreduced = run(&world, Some(0), undefined(), Reductions::default());
    for outcome in [&reduced, &unreduced] {
        let counterexample = violation(outcome);
        let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
            panic!("expected an undefined evaluation: {outcome:?}");
        };
        assert_eq!(
            evaluation.cause,
            UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero)
        );
        assert_eq!(counterexample.prefix.len(), 2);
        assert_eq!(
            counterexample
                .over_binding
                .as_ref()
                .map(|binding| binding.key.as_str()),
            Some("a")
        );
        let positions = world.positions(counterexample);
        assert_eq!(positions.last().map(|state| value(state, "a")), Some(2));
    }
}

/// FR-161's start and step rules on a prefix through moved keys: the
/// deadlock-freedom item of counters that stop at 2 under `[[a, b, c]]`
/// reaches its deadlock through canonical states, and the concretised
/// prefix is a path of the subject, six steps from `(0,0,0)` to
/// `(2,2,2)`, whose recorded digests and transitions re-execute.
#[test]
fn a_reduced_prefix_concretises_to_a_subject_path() {
    let stop = "pre Below using v on M::Counter::bump { self.v < 2 }\n\
                post Up using v on M::Counter::bump { self.v - pre(self.v) = 1 }\n";
    let world = World::new(shape(true, 2), stop, &[], &[start([0, 0, 0])], &KEYS);
    let reduced = run(&world, None, Vec::new(), symmetry(&[&["a", "b", "c"]]));
    assert_eq!(violation(&reduced).kind, CounterexampleKind::Deadlock);
    let states = path(&world, &reduced);
    assert_eq!(states.len(), 7);
    assert_eq!(states.first(), Some(&(0, 0, 0)));
    assert_eq!(states.last(), Some(&(2, 2, 2)));
    let unreduced = run(&world, None, Vec::new(), Reductions::default());
    assert_eq!(violation(&unreduced).prefix.len(), 6);
    assert!(reduced.stats().product_states < unreduced.stats().product_states);
}
