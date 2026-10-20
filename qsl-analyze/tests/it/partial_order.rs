// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-021 §7.2, §7.5 and FR-156-AC-3's ring (FR-156, FR-157, FR-162,
//! TC-576 to TC-579, TC-587, TC-617): partial-order reduction over
//! independent counters, a ring that would ignore a visible transition and
//! the `Gate` whose fairness class another operation enables.
//!
//! The ample sets are read from the model-level graph `retain_reduced`
//! builds under [`Por`] for the one instance a test names, so each state's
//! ample set is the operations of its retained edges. The verdicts and
//! product counts come from `check_model`, which runs every instance of the
//! claim.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{
    retain_reduced, Ample, Atoms, Choice, LivenessRefusal, ModelCheckRefusal, Por, Reductions,
    SymmetryDeclaration,
};
use qsl_eval::model_subject::{ModelInstance, ModelLetters};
use qsl_eval::simulation::model::{ModelFinding, ModelSimState, ModelSystem};
use qsl_eval::simulation::{
    ExpansionStop, Limits, Location, SymmetryClass, SymmetryGroup, TransitionSystem,
};
use qsl_foundation::diagnostic::Category;
use qsl_foundation::IdentityLimits;
use qsl_replay::{
    settle_model_check, AppliedReduction, CertificateRequest, Certification, CounterexampleKind,
    DigestDomain, DigestRecord, IncompleteCause, InconclusiveCause, ModelCheckItem,
    ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome, ModelTraceSettlement, ProofBasis,
    Proviso, ReachedLimit, ReductionCause, ResultLabel, SettlementBasis, TerminalValue,
};
use qsl_semantics::check::footprint::{FootprintObject, SymbolicLocation};
use quire_exact::Cancel;
use quire_semantic_value::model_transition::ModelTransition;

use crate::por_world::{self, gate, gate_state, gate_with, ring, ring_state, Fixture, Guard};
use crate::reduction_world::{
    atom, counters, start, value, values, Clause, Shape, World, INFINITE,
};

const KEYS: [&str; 3] = ["a", "b", "c"];

/// §7.2's `bump`: below 2, advance by one.
const BUMPS: &str = "pre Below using v on M::Counter::bump { self.v < 2 }\n\
     post Up using v on M::Counter::bump { self.v - pre(self.v) = 1 }\n";

/// ADR-021 §7.2's `ReachesTwo`.
pub(crate) const REACHES_TWO: &str = "eventually holds(x.v = 2)";

fn reduced() -> Reductions {
    Reductions {
        partial_order: true,
        ..Reductions::default()
    }
}

/// §7.2's subject: three counters over `Int[0, 2]`, all at 0, with `clauses`.
fn bumps(clauses: &[Clause<'_>]) -> World {
    let shape = Shape {
        symmetric: false,
        max: 2,
        link: false,
    };
    World::new(shape, BUMPS, clauses, &[start([0, 0, 0])], &KEYS)
}

fn reaches_two() -> World {
    bumps(&[Clause {
        profile: INFINITE,
        formula: REACHES_TWO,
    }])
}

/// The chooser `retain_reduced` calls: the ample set of `por`, with a
/// transition fresh when every successor through it is undiscovered.
struct ModelAmple<'p, 'r, 'a>(&'p Por<'r, 'a>);

impl<'r, 'a> Ample<ModelSystem<'a>> for ModelAmple<'_, 'r, 'a> {
    fn choose(
        &self,
        state: &ModelSimState,
        _findings: &[ModelFinding],
        successors: &[(&ModelTransition, bool)],
    ) -> Result<Choice<ModelTransition>, ExpansionStop> {
        let mut enabled: Vec<ModelTransition> = successors
            .iter()
            .map(|(transition, _)| (*transition).clone())
            .collect();
        enabled.sort();
        enabled.dedup();
        let fresh = |transition: &ModelTransition| {
            successors
                .iter()
                .filter(|(held, _)| *held == transition)
                .all(|(_, fresh)| *fresh)
        };
        let chosen = self.0.ample(state, &enabled, &fresh)?;
        Ok(if chosen.len() == enabled.len() {
            Choice::All
        } else {
            Choice::Only(chosen)
        })
    }
}

/// The operation a transition applies, by name.
fn operation_of(transition: &ModelTransition) -> String {
    let text = transition.operation();
    let node = text
        .split("\"node\":\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("a node");
    node.rsplit('/').next().expect("a name").to_owned()
}

/// The enabled transition of `state` that applies `operation`.
fn named(system: &ModelSystem<'_>, state: &ModelSimState, operation: &str) -> ModelTransition {
    system
        .successors(state)
        .expect("the state expands")
        .successors
        .into_iter()
        .map(|(transition, _)| transition)
        .find(|transition| operation_of(transition) == operation)
        .unwrap_or_else(|| panic!("{operation} is not enabled"))
}

/// Every state a reduced model-level search stores, in discovery order, each
/// with the label of every edge it retains, and the retained edge count.
fn ample_sets<'a, S: Clone>(
    system: &ModelSystem<'a>,
    por: &Por<'_, 'a>,
    summarise: impl Fn(&ModelSimState) -> S,
    label: impl Fn(&ModelSimState, &ModelSimState, &ModelTransition) -> String,
) -> (Vec<(S, Vec<String>)>, usize) {
    let exploration = retain_reduced(
        system,
        &SymmetryGroup::default(),
        Some(&ModelAmple(por)),
        Limits::default(),
        IdentityLimits::default(),
        None,
        || false,
    )
    .expect("the states encode");
    let graph = exploration.graph;
    let states = graph
        .states()
        .iter()
        .enumerate()
        .map(|(index, state)| {
            let operations = graph
                .edges()
                .iter()
                .filter(|edge| edge.source.index() == index)
                .map(|edge| {
                    let target = graph.state(edge.target).expect("a target");
                    label(&state.state, &target.state, &edge.transition)
                })
                .collect();
            (summarise(&state.state), operations)
        })
        .collect();
    (states, graph.edges().len())
}

fn counters_por<'r, 'a>(
    world: &'r World,
    system: &'r ModelSystem<'a>,
    letters: &'r (
        crate::reduction_world::Letters,
        Vec<qsl_semantics::check::FormulaNodeId>,
    ),
    instances: &'r [ModelInstance],
) -> Por<'r, 'a> {
    Por::new(
        system,
        Some(Atoms {
            letters: &letters.0,
            nodes: &letters.1,
        }),
        instances,
        world.clauses[0].fairness(),
        None,
    )
    .expect("the footprints derive")
}

/// The declaration key of `Counter.v`.
fn counter_field() -> qsl_semantics::model::key::DeclarationKey {
    qsl_semantics::model::key::DeclarationKey {
        package: "test/sym".to_owned(),
        node: "ix://test/sym/Counter/v".to_owned(),
    }
}

fn counter_instance(key: &str) -> [ModelInstance; 1] {
    [ModelInstance {
        population: counters(),
        key: key.to_owned(),
    }]
}

/// The values of the three counters.
type Counts = (i64, i64, i64);

/// The key of the counter an edge bumps.
fn bumped(source: &ModelSimState, target: &ModelSimState, _: &ModelTransition) -> String {
    KEYS.into_iter()
        .find(|key| value(source, key) != value(target, key))
        .expect("a bump changes its receiver")
        .to_owned()
}

/// FR-156-AC-1: for `x = a`, `bump(a)` is visible and `bump(b)` and
/// `bump(c)` are not. The ample sets are `{bump(b)}` at `(0,0,0)` and
/// `(0,1,0)`, `{bump(c)}` at `(0,2,0)` and `(0,2,1)`, `E(s)` at `(0,2,2)` and
/// `(1,2,2)`, and none at `(2,2,2)`: 7 model states and 6 model transitions
/// where the unreduced search stores 27. The run, which checks the three
/// instances, stores 6 product states for each of the three negations and
/// proves the claim under no certificate; the unreduced run stores 18 each
/// and is refused only for want of FR-339's component certificate.
#[trace("TC-576", "FR-156-AC-1")]
#[test]
fn ample_sets_reduce_independent_bumps_to_one_interleaving() {
    let world = reaches_two();
    let system = world.system();
    let letters = world.letters(0, vec![atom(|v| Some(v == 2))]);
    let instances = counter_instance("a");
    let por = counters_por(&world, &system, &letters, &instances);

    // S3 derives the atom's read footprint with `over` symbolic: `x.v`.
    assert_eq!(
        letters.0.reads(letters.1[0]).expect("a footprint"),
        [SymbolicLocation::Field {
            object: FootprintObject::Receiver,
            field: counter_field(),
        }]
        .into()
    );
    let initial = system.initial()[0].clone();
    let visible = |key: &str| {
        let transition = system
            .successors(&initial)
            .expect("expands")
            .successors
            .into_iter()
            .find(|(_, state)| value(state, key) == 1)
            .expect("each counter bumps")
            .0;
        por.visible(&initial, &transition).expect("visibility")
    };
    assert_eq!(
        (visible("a"), visible("b"), visible("c")),
        (true, false, false)
    );

    let (states, edges) = ample_sets(&system, &por, values, bumped);
    assert_eq!(edges, 6);
    let expected: [(Counts, &[&str]); 7] = [
        ((0, 0, 0), &["b"]),
        ((0, 1, 0), &["b"]),
        ((0, 2, 0), &["c"]),
        ((0, 2, 1), &["c"]),
        ((0, 2, 2), &["a"]),
        ((1, 2, 2), &["a"]),
        ((2, 2, 2), &[]),
    ];
    assert_eq!(
        states,
        expected
            .iter()
            .map(|(state, keys)| (*state, keys.iter().map(|key| (*key).to_owned()).collect()))
            .collect::<Vec<_>>()
    );

    let reduced = world
        .run(
            Some(0),
            vec![atom(|v| Some(v == 2))],
            reduced(),
            None,
            ModelCheckLimits::default(),
        )
        .expect("checks");
    let ModelCheckOutcome::HoldsReduced { reductions, stats } = reduced else {
        panic!("expected a reduced proof, got {reduced:?}");
    };
    assert_eq!(
        reductions,
        [AppliedReduction::PartialOrder {
            proviso: Proviso::BreadthFirstRevisit
        }]
    );
    assert_eq!(stats.product_states, 3 * 6);
    let unreduced = world
        .run(
            Some(0),
            vec![atom(|v| Some(v == 2))],
            Reductions::default(),
            None,
            ModelCheckLimits::default(),
        )
        .expect_err("a liveness proof is refused without FR-339's certificate");
    let ModelCheckRefusal::Liveness(LivenessRefusal::ComponentCertificateNotBuilt { stats }) =
        unreduced
    else {
        panic!("expected the missing component certificate, got {unreduced:?}");
    };
    assert_eq!(stats.product_states, 3 * 18);
}

/// FR-156-AC-2: for the deadlock-freedom item no transition is visible, the
/// reduced search reaches `(2,2,2)` and refutes with a six-step prefix, as the
/// unreduced search does.
#[trace("TC-576", "FR-156-AC-2")]
#[test]
fn the_deadlock_item_sees_no_visible_transition_and_reaches_the_terminal_state() {
    let world = reaches_two();
    let reduced = world
        .run(None, vec![], reduced(), None, ModelCheckLimits::default())
        .expect("checks");
    let ModelCheckOutcome::Violated {
        counterexample,
        stats,
    } = &reduced
    else {
        panic!("expected a deadlock, got {reduced:?}");
    };
    assert_eq!(counterexample.kind, CounterexampleKind::Deadlock);
    assert_eq!(counterexample.prefix.len(), 6);
    let positions = world.positions(counterexample);
    assert_eq!(values(&positions[6]), (2, 2, 2));
    // With nothing visible the first singleton candidate in canonical order
    // is `bump(a)`: the path is `a, a, b, b, c, c`.
    let moved: Vec<&str> = positions
        .windows(2)
        .map(|pair| {
            KEYS.into_iter()
                .find(|key| value(&pair[0], key) != value(&pair[1], key))
                .expect("a step changes its receiver")
        })
        .collect();
    assert_eq!(moved, ["a", "a", "b", "b", "c", "c"]);
    // One interleaving: 7 states, 6 edges.
    assert_eq!((stats.product_states, stats.product_edges), (7, 6));
    let unreduced = world
        .run(
            None,
            vec![],
            Reductions::default(),
            None,
            ModelCheckLimits::default(),
        )
        .expect("checks");
    let ModelCheckOutcome::Violated {
        counterexample: plain,
        ..
    } = &unreduced
    else {
        panic!("expected a deadlock, got {unreduced:?}");
    };
    assert_eq!(plain.kind, CounterexampleKind::Deadlock);
    assert_eq!(plain.prefix.len(), 6);
    assert!(matches!(
        world.replay(counterexample.as_ref().clone()),
        ModelTraceSettlement::Reproduced { .. }
    ));
}

// ---------------------------------------------------------------------------
// The Gate (ADR-021 §7.5, FR-157)
// ---------------------------------------------------------------------------

/// FR-157's claims: `Opens` under `fair weak go`, with no fairness, under
/// `fair weak each go` and under `fair strong go`.
pub(crate) const OPENS: [&str; 4] = [
    "fair weak M::Gate::go; eventually holds(x.done)",
    "eventually holds(x.done)",
    "fair weak each M::Gate::go; eventually holds(x.done)",
    "fair strong M::Gate::go; eventually holds(x.done)",
];

fn done() -> Vec<por_world::Atom> {
    vec![Box::new(|state, key| {
        por_world::truth(state, "gates", key, "Gate/done")
    })]
}

/// The Gate model's [`Por`] for the instance `g` of clause `clause`, over
/// `fixture`'s system, with `fairness` in place of the clause's when given.
struct GatePor<'f> {
    fixture: &'f Fixture,
    system: ModelSystem<'f>,
    letters: (por_world::Letters, Vec<qsl_semantics::check::FormulaNodeId>),
    instances: [ModelInstance; 1],
}

impl<'f> GatePor<'f> {
    fn new(fixture: &'f Fixture, clause: usize) -> Self {
        Self {
            fixture,
            system: fixture.system(),
            letters: fixture.letters(clause, done()),
            instances: [ModelInstance {
                population: por_world::key("gates"),
                key: "g".to_owned(),
            }],
        }
    }

    fn por(&self, clause: usize) -> Por<'_, 'f> {
        Por::new(
            &self.system,
            Some(Atoms {
                letters: &self.letters.0,
                nodes: &self.letters.1,
            }),
            &self.instances,
            self.fixture.clause(clause).fairness(),
            None,
        )
        .expect("the footprints derive")
    }
}

/// The operations of the transitions of `state` that are visible, by name.
fn visible_operations(
    por: &Por<'_, '_>,
    system: &ModelSystem<'_>,
    state: &ModelSimState,
) -> Vec<String> {
    let mut visible = Vec::new();
    for transition in system.transition_identities(state).expect("identities") {
        if por.visible(state, &transition).expect("visibility") {
            visible.push(operation_of(&transition));
        }
    }
    visible
}

fn labelled(_: &ModelSimState, _: &ModelSimState, transition: &ModelTransition) -> String {
    operation_of(transition)
}

/// FR-157-AC-1: under `fair weak go`, `arm` and `arm2` are visible (each
/// writes a location of `go(g)`'s enabling footprint and neither is in
/// `go`'s class), `go` is visible by the atom and `toggle` is not. The ample
/// set at `(F,F,F,F)` is `{toggle}`; at `(T,F,F,F)` the candidate `{toggle}`
/// reaches the discovered `(F,F,F,F)`, fails C3, and the state is fully
/// expanded. The reduced product keeps the cycle `(F,F,F,F)` and `(T,F,F,F)`,
/// where `go` is disabled, which passes the weak filter, and the run returns
/// the unreduced run's verdict and lasso: an empty stem and the loop
/// `toggle, toggle`.
#[trace("TC-578", "FR-157-AC-1")]
#[test]
fn fairness_visibility_keeps_the_fair_violation() {
    let fixture = gate(Guard::Pre, false, &OPENS);
    let holder = GatePor::new(&fixture, 0);
    let por = holder.por(0);
    let start = fixture.start();
    let mut visible = visible_operations(&por, &holder.system, &start);
    visible.sort();
    assert_eq!(visible, ["arm", "arm2", "go"]);

    let (states, _) = ample_sets(&holder.system, &por, gate_state, labelled);
    assert_eq!(
        states[0],
        ((false, false, false, false), vec!["toggle".to_owned()])
    );
    assert_eq!(
        states[1],
        (
            (true, false, false, false),
            vec!["arm".to_owned(), "toggle".to_owned()]
        )
    );

    let reduced_run = fixture.run(Some(0), done(), reduced()).expect("checks");
    let plain = fixture
        .run(Some(0), done(), Reductions::default())
        .expect("checks");
    for outcome in [&reduced_run, &plain] {
        let ModelCheckOutcome::Violated { counterexample, .. } = outcome else {
            panic!("expected a violation, got {outcome:?}");
        };
        let lasso = counterexample.lasso.expect("a lasso");
        assert_eq!(lasso.entry, 0, "an empty stem");
        let positions = fixture.positions(counterexample);
        assert_eq!(
            positions.iter().map(gate_state).collect::<Vec<_>>(),
            [
                (false, false, false, false),
                (true, false, false, false),
                (false, false, false, false)
            ]
        );
        assert!(matches!(
            fixture.replay(Some(0), done(), counterexample.as_ref().clone()),
            ModelTraceSettlement::Reproduced { .. }
        ));
    }
}

/// The ample set `por` chooses at `state` among the enabled transitions
/// `enabled`, by operation, when every successor is undiscovered.
fn ample_at(por: &Por<'_, '_>, system: &ModelSystem<'_>, state: &ModelSimState) -> Vec<String> {
    let mut enabled: Vec<ModelTransition> = system
        .successors(state)
        .expect("the state expands")
        .successors
        .into_iter()
        .map(|(transition, _)| transition)
        .collect();
    enabled.sort();
    enabled.dedup();
    let mut chosen: Vec<String> = por
        .ample(state, &enabled, &|_| true)
        .expect("an ample set")
        .iter()
        .map(operation_of)
        .collect();
    chosen.sort();
    chosen
}

/// FR-157-AC-2: why the rule matters. With `arm` and `arm2` invisible,
/// `{arm}` satisfies C0 to C3 at `(F,F,F,F)` and wins the tie with
/// `{toggle}` on canonical order, and the only accepting cycle left starves
/// `go` while it is enabled, so the filter would reject it and the run would
/// return `Holds`. The test computes both choices through the visibility
/// function: `{toggle}` with the fairness set and `{arm}` with the empty one.
#[trace("TC-578", "FR-157-AC-2")]
#[test]
fn without_fairness_visibility_the_arming_ample_set_would_hide_the_violation() {
    let fixture = gate(Guard::Pre, false, &OPENS);
    let holder = GatePor::new(&fixture, 0);
    let start = fixture.start();
    let weak = holder.por(0);
    let none = holder.por(1);
    assert_eq!(ample_at(&weak, &holder.system, &start), ["toggle"]);
    assert_eq!(ample_at(&none, &holder.system, &start), ["arm"]);
    let mut visible = visible_operations(&none, &holder.system, &start);
    visible.sort();
    assert_eq!(visible, ["go"]);
    // The AC-1 run, whose fairness set is the first clause's, is refuted.
    let outcome = fixture.run(Some(0), done(), reduced()).expect("checks");
    assert!(matches!(outcome, ModelCheckOutcome::Violated { .. }));
}

/// FR-156-AC-4: the candidate seeded by `arm` holds `arm`, `arm2` and `go`
/// (`arm2` is disabled at the initial state, and the writer of its enabling
/// footprint, `arm`, and the transition that reads what `arm` writes, `go`,
/// are in it), with enabled part `{arm}`; two runs choose equal ample sets
/// and return equal outcomes.
#[trace("TC-577", "FR-156-AC-4")]
#[test]
fn the_candidate_reaches_through_a_disabled_member_and_the_choice_is_deterministic() {
    let fixture = gate(Guard::Pre, false, &OPENS);
    let holder = GatePor::new(&fixture, 0);
    let por = holder.por(0);
    let start = fixture.start();
    let enabled: Vec<ModelTransition> = {
        let mut enabled: Vec<_> = holder
            .system
            .successors(&start)
            .expect("expands")
            .successors
            .into_iter()
            .map(|(transition, _)| transition)
            .collect();
        enabled.sort();
        enabled
    };
    let arm = named(&holder.system, &start, "arm");
    let mut candidate: Vec<String> = por
        .candidate_seeded_by(&start, &enabled, &arm)
        .expect("a candidate")
        .iter()
        .map(operation_of)
        .collect();
    candidate.sort();
    assert_eq!(candidate, ["arm", "arm2", "go"]);

    let first = ample_sets(&holder.system, &por, gate_state, labelled);
    let second = ample_sets(&holder.system, &holder.por(0), gate_state, labelled);
    assert_eq!(first, second);
    assert_eq!(
        fixture.run(Some(0), done(), reduced()).expect("checks"),
        fixture.run(Some(0), done(), reduced()).expect("checks")
    );
}

/// FR-156-AC-4's closure through a disabled member, over the `Gate` with an
/// added enabled `touch()` framed `modifies self.d`: the candidate seeded by
/// `touch` holds `arm2` (dependent on it through `d`) and, through `arm2`'s
/// enabling footprint, `arm`, which writes `c` and is independent of `touch`.
/// Its enabled part is `{touch, arm}`; without the closure it is `{touch}`.
#[trace("TC-577", "FR-156-AC-4")]
#[test]
fn the_candidate_holds_the_writers_of_a_disabled_members_enabling_footprint() {
    let fixture = gate_with(Guard::Pre, false, true, &OPENS);
    let holder = GatePor::new(&fixture, 0);
    let por = holder.por(0);
    let start = fixture.start();
    let mut enabled: Vec<ModelTransition> = holder
        .system
        .successors(&start)
        .expect("expands")
        .successors
        .into_iter()
        .map(|(transition, _)| transition)
        .collect();
    enabled.sort();
    let touch = named(&holder.system, &start, "touch");
    let candidate = por
        .candidate_seeded_by(&start, &enabled, &touch)
        .expect("a candidate");
    let mut names: Vec<String> = candidate.iter().map(operation_of).collect();
    names.sort();
    assert_eq!(names, ["arm", "arm2", "go", "touch"]);
    let mut enabled_part: Vec<String> = candidate
        .iter()
        .filter(|transition| enabled.contains(transition))
        .map(operation_of)
        .collect();
    enabled_part.sort();
    assert_eq!(enabled_part, ["arm", "touch"]);
}

/// FR-155-AC-7's candidate clause and FR-162: `spawn(y)` creates an `Item`
/// and writes the membership of any item, which `touch(d)` reads, though no
/// state holds `d` yet; the candidate seeded by `spawn(y)` holds the disabled
/// `touch(d)`. `cancel(x)` deletes `x`, and `probe(y, d, x)` over the absent
/// `d` reads the membership of `x`, so the candidate seeded by `cancel(x)`
/// holds it too. Without it the initial state's ample set `{cancel(x)}`
/// would hide the deadlock the probe leads to.
#[trace("TC-1075", "FR-155-AC-7")]
#[test]
fn the_candidate_holds_transitions_over_objects_no_state_holds_yet() {
    let fixture = por_world::makers(&[]);
    let system = fixture.system();
    let start = fixture.start();
    let por = Por::new(&system, None, &[], &[], None).expect("the footprints derive");
    let mut enabled: Vec<ModelTransition> = system
        .successors(&start)
        .expect("expands")
        .successors
        .into_iter()
        .map(|(transition, _)| transition)
        .collect();
    enabled.sort();
    let candidate_of = |operation: &str| {
        let seed = enabled
            .iter()
            .find(|transition| operation_of(transition) == operation)
            .unwrap_or_else(|| panic!("{operation} is enabled"));
        let mut names: Vec<String> = por
            .candidate_seeded_by(&start, &enabled, seed)
            .expect("a candidate")
            .iter()
            .map(operation_of)
            .collect();
        names.sort();
        names
    };
    assert!(candidate_of("spawn").contains(&"touch".to_owned()));
    assert!(candidate_of("cancel").contains(&"probe".to_owned()));
}

/// FR-157-AC-3: with the empty fairness set no fairness visibility applies,
/// the ample sets are `{arm}` at `(F,F,F,F)` and `{arm2}` at `(F,T,F,F)`, and
/// the reduced run returns `Violated` with the stem `arm, arm2` and the loop
/// `toggle, toggle`; the unreduced run returns `Violated` with an empty stem
/// and the same loop. Both replay.
#[trace("TC-579", "FR-157-AC-3")]
#[test]
fn with_no_fairness_the_arming_ample_sets_remain_and_the_violation_is_found() {
    let fixture = gate(Guard::Pre, false, &OPENS);
    let holder = GatePor::new(&fixture, 1);
    let por = holder.por(1);
    let (states, _) = ample_sets(&holder.system, &por, gate_state, labelled);
    assert_eq!(
        states[0],
        ((false, false, false, false), vec!["arm".to_owned()])
    );
    let armed = states
        .iter()
        .find(|(state, _)| *state == (false, true, false, false))
        .expect("(F,T,F,F) is stored");
    assert_eq!(armed.1, ["arm2".to_owned()]);

    let reduced_run = fixture.run(Some(1), done(), reduced()).expect("checks");
    let plain = fixture
        .run(Some(1), done(), Reductions::default())
        .expect("checks");
    let shapes: Vec<_> = [&reduced_run, &plain]
        .into_iter()
        .map(|outcome| {
            let ModelCheckOutcome::Violated { counterexample, .. } = outcome else {
                panic!("expected a violation, got {outcome:?}");
            };
            assert!(matches!(
                fixture.replay(Some(1), done(), counterexample.as_ref().clone()),
                ModelTraceSettlement::Reproduced { .. }
            ));
            (
                counterexample.lasso.expect("a lasso").entry,
                counterexample
                    .prefix
                    .iter()
                    .map(|step| operation_of(&step.transition))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    // Reduced: the stem `arm, arm2`, then the loop `toggle, toggle`.
    assert_eq!(shapes[0].0, 2);
    assert_eq!(shapes[0].1, ["arm", "arm2", "toggle", "toggle"]);
    // Unreduced: an empty stem and the loop `toggle, toggle`.
    assert_eq!(shapes[1].0, 0);
    assert_eq!(shapes[1].1, ["toggle", "toggle"]);
}

/// FR-157-AC-5 and ADR-021 §7.6's second vector: with `go`'s guard moved
/// into its postcondition (`not self.done`, then `self.done and pre(self.c)
/// and pre(self.d)`), `go(g)`'s enabling footprint still holds `Field{g, c}`
/// and `Field{g, d}`, so `arm` and `arm2` stay visible, the ample set at
/// `(F,F,F,F)` is `{toggle}`, and the run returns `Violated` with the loop
/// `toggle, toggle`, as the unreduced run does.
#[trace("TC-617", "FR-157-AC-5")]
#[test]
fn a_guard_in_a_postcondition_keeps_the_arming_operations_visible() {
    let fixture = gate(Guard::Post, false, &OPENS);
    let holder = GatePor::new(&fixture, 0);
    let por = holder.por(0);
    let start = fixture.start();
    let mut visible = visible_operations(&por, &holder.system, &start);
    visible.sort();
    assert_eq!(visible, ["arm", "arm2", "go"]);
    assert_eq!(ample_at(&por, &holder.system, &start), ["toggle"]);
    for selection in [reduced(), Reductions::default()] {
        let outcome = fixture.run(Some(0), done(), selection).expect("checks");
        let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
            panic!("expected a violation, got {outcome:?}");
        };
        assert_eq!(counterexample.lasso.expect("a lasso").entry, 0);
        assert_eq!(
            fixture
                .positions(counterexample)
                .iter()
                .map(gate_state)
                .collect::<Vec<_>>(),
            [
                (false, false, false, false),
                (true, false, false, false),
                (false, false, false, false)
            ]
        );
    }
}

/// FR-157-AC-6: with an operation `spawn()` framed `creates Gate`, under
/// `fair weak whole go`, `spawn` writes `AnyMembership{gates}`, which meets
/// `Membership{gates, g}` in `go(g)`'s enabling footprint, so `spawn` is
/// visible.
#[trace("TC-617", "FR-157-AC-6")]
#[test]
fn a_creation_that_adds_a_member_to_a_whole_class_is_visible() {
    let fixture = gate(
        Guard::Pre,
        true,
        &["fair weak whole M::Gate::go; eventually holds(x.done)"],
    );
    let holder = GatePor::new(&fixture, 0);
    let por = holder.por(0);
    let start = fixture.start();
    let spawn = named(&holder.system, &start, "spawn");
    assert!(por.visible(&start, &spawn).expect("visibility"));
    let toggle = named(&holder.system, &start, "toggle");
    assert!(!por.visible(&start, &toggle).expect("visibility"));
}

/// The model half of FR-157-AC-4, whose `ProtocolSystem` half is not built:
/// with `fair weak each go` the classes are one identity each, and AC-1's
/// visibility and verdict are unchanged.
#[trace("TC-579")]
#[test]
fn each_granularity_leaves_the_visibility_and_verdict_unchanged() {
    let fixture = gate(Guard::Pre, false, &OPENS);
    let holder = GatePor::new(&fixture, 2);
    let por = holder.por(2);
    let start = fixture.start();
    let mut visible = visible_operations(&por, &holder.system, &start);
    visible.sort();
    assert_eq!(visible, ["arm", "arm2", "go"]);
    let outcome = fixture.run(Some(2), done(), reduced()).expect("checks");
    assert!(matches!(outcome, ModelCheckOutcome::Violated { .. }));
}

/// POR-11 holds for strong constraints as for weak: under `fair strong go`
/// the same transitions are visible and the reduced run returns the
/// unreduced run's verdict, a fair lasso that never enables `go`.
#[trace("TC-578")]
#[test]
fn a_strong_constraint_makes_the_same_writes_visible() {
    let fixture = gate(Guard::Pre, false, &OPENS);
    let holder = GatePor::new(&fixture, 3);
    let por = holder.por(3);
    let start = fixture.start();
    let mut visible = visible_operations(&por, &holder.system, &start);
    visible.sort();
    assert_eq!(visible, ["arm", "arm2", "go"]);
    assert_eq!(ample_at(&por, &holder.system, &start), ["toggle"]);
    for selection in [reduced(), Reductions::default()] {
        let outcome = fixture.run(Some(3), done(), selection).expect("checks");
        let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
            panic!("expected a violation, got {outcome:?}");
        };
        assert!(matches!(
            fixture.replay(Some(3), done(), counterexample.as_ref().clone()),
            ModelTraceSettlement::Reproduced { .. }
        ));
    }
}

// ---------------------------------------------------------------------------
// The ring (FR-156-AC-3)
// ---------------------------------------------------------------------------

fn below_two() -> Vec<por_world::Atom> {
    vec![Box::new(|state, key| {
        por_world::number(state, "counters", key, "Counter/v").map(|v| v < 2)
    })]
}

/// FR-156-AC-3: the claim `always holds(q.v < 2)` makes `bump(q)` visible.
/// The ample set is `{step(r)}` at `(r=0, q=0)` and `(1, 0)`; at `(2, 0)`
/// the candidate `{step(r)}` reaches the discovered `(0, 0)`, fails C3, and
/// the state is fully expanded, so `bump(q)` is explored. The run returns
/// `Violated` at `q.v = 2` with the six-step prefix `step, step, bump, step,
/// step, bump`, which replays; the unreduced run also returns `Violated`.
#[trace("TC-577", "FR-156-AC-3")]
#[test]
fn the_breadth_first_proviso_prevents_ignoring_a_visible_transition() {
    let fixture = ring(&["always holds(x.v < 2)"]);
    let system = fixture.system();
    let letters = fixture.letters(0, below_two());
    let instances = [ModelInstance {
        population: por_world::key("counters"),
        key: "q".to_owned(),
    }];
    let por = Por::new(
        &system,
        Some(Atoms {
            letters: &letters.0,
            nodes: &letters.1,
        }),
        &instances,
        fixture.clause(0).fairness(),
        None,
    )
    .expect("the footprints derive");
    let (states, _) = ample_sets(&system, &por, ring_state, labelled);
    let stored = |state: (i64, i64)| {
        states
            .iter()
            .find(|(held, _)| *held == state)
            .unwrap_or_else(|| panic!("{state:?} is stored"))
            .1
            .clone()
    };
    assert_eq!(stored((0, 0)), ["step"]);
    assert_eq!(stored((1, 0)), ["step"]);
    let mut full = stored((2, 0));
    full.sort();
    assert_eq!(full, ["bump", "step"]);

    let outcome = fixture
        .run(Some(0), below_two(), reduced())
        .expect("checks");
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("expected a violation, got {outcome:?}");
    };
    assert_eq!(counterexample.prefix.len(), 6);
    let names: Vec<String> = counterexample
        .prefix
        .iter()
        .map(|step| operation_of(&step.transition))
        .collect();
    assert_eq!(names, ["step", "step", "bump", "step", "step", "bump"]);
    assert!(matches!(
        fixture.replay(Some(0), below_two(), counterexample.as_ref().clone()),
        ModelTraceSettlement::Reproduced { .. }
    ));
    let plain = fixture
        .run(Some(0), below_two(), Reductions::default())
        .expect("checks");
    assert!(matches!(plain, ModelCheckOutcome::Violated { .. }));
}

// ---------------------------------------------------------------------------
// ADR-021 §7.6's vectors (FR-156-AC-5 and AC-6, FR-155-AC-6)
// ---------------------------------------------------------------------------

/// Run `fixture`'s claim unreduced and under partial-order reduction: both
/// return `Violated` with the same operations, the same kind of
/// counterexample and the same number of product states (§7.6: the reduced
/// run refutes "as the unreduced run does", where a footprint without the
/// membership, referrer or navigation reads would prove it), and each
/// counterexample replays or, for an undefined evaluation, re-executes.
fn vector(
    fixture: &Fixture,
    atoms: fn() -> Vec<por_world::Atom>,
) -> (Vec<String>, CounterexampleKind) {
    let runs: Vec<(Vec<String>, CounterexampleKind, u64)> = [reduced(), Reductions::default()]
        .into_iter()
        .map(|selection| {
            let outcome = fixture.run(Some(0), atoms(), selection).expect("checks");
            let ModelCheckOutcome::Violated {
                counterexample,
                stats,
            } = outcome
            else {
                panic!("expected a violation, got {outcome:?}");
            };
            // FR-128 refuses an authored undefined evaluation; its prefix
            // re-executes on the subject instead.
            if matches!(
                counterexample.kind,
                CounterexampleKind::UndefinedEvaluation(_)
            ) {
                assert_eq!(
                    fixture.positions(&counterexample).len(),
                    counterexample.prefix.len() + 1
                );
            } else {
                assert!(matches!(
                    fixture.replay(Some(0), atoms(), counterexample.as_ref().clone()),
                    ModelTraceSettlement::Reproduced { .. }
                ));
            }
            (
                counterexample
                    .prefix
                    .iter()
                    .map(|step| operation_of(&step.transition))
                    .collect(),
                counterexample.kind,
                stats.product_states,
            )
        })
        .collect();
    assert_eq!(runs[0], runs[1], "the reduced run is the unreduced run");
    (runs[0].0.clone(), runs[0].1.clone())
}

/// FR-156-AC-5 and FR-155-AC-5's guard in a postcondition: the initial state
/// is fully expanded because `boom`'s enabling footprint holds the field of
/// any cell its postcondition reads, so `set` joins the candidate seeded by
/// `disarm`; the run returns `Violated` with the prefix `set, boom`.
#[trace("TC-589", "FR-156-AC-5")]
#[test]
fn a_guard_in_a_postcondition_expands_the_initial_state() {
    let (operations, kind) = vector(&por_world::guard_in_postcondition(), por_world::no_alarm);
    assert_eq!(operations, ["set", "boom"]);
    assert!(matches!(kind, CounterexampleKind::Formula));
}

/// FR-156-AC-6: deleting the receiver. `cancel(j)` writes the membership of
/// `j`, which `boom(j)` reads, so `{cancel(j)}` is not an ample set and the
/// run returns `Violated` by `boom(j)`.
#[trace("TC-589", "FR-156-AC-6")]
#[test]
fn deleting_the_receiver_is_dependent_on_the_transitions_that_read_it() {
    let (operations, _) = vector(&por_world::deleting_the_receiver(), por_world::not_bad);
    assert_eq!(operations, ["boom"]);
}

/// FR-156-AC-6: a type-wide delete. `cancel(m)` writes the membership of any
/// job, which meets `boom(j)`'s receiver membership, so `{cancel(m)}` is not
/// an ample set and the run returns `Violated` by `boom(j)`, never `Holds`.
#[trace("TC-589", "FR-156-AC-6")]
#[test]
fn a_type_wide_delete_is_dependent_on_every_job() {
    let (operations, _) = vector(&por_world::deleting_any_job(), por_world::not_bad);
    assert_eq!(operations, ["boom"]);
}

/// FR-156-AC-6: a delete blocked by a reference. `unlink(j)` writes the
/// referrer field `cancel(k)`'s enabling footprint holds, so `{lock(k)}` is
/// not an ample set and the run returns `Violated` with `UndefinedEvaluation`
/// after `unlink, cancel`.
#[trace("TC-589", "FR-156-AC-6")]
#[test]
fn a_delete_blocked_by_a_reference_reads_the_referrer_field() {
    let (operations, kind) = vector(&por_world::blocked_delete(), por_world::zero_x);
    assert_eq!(operations, ["unlink", "cancel"]);
    assert!(matches!(kind, CounterexampleKind::UndefinedEvaluation(_)));
}

/// FR-156-AC-6: a navigation through a rewritten reference. `point` writes
/// the intermediate field `boom` reads, so `{disarm}` is not an ample set and
/// the run returns `Violated` by `point, boom`.
#[trace("TC-589", "FR-156-AC-6")]
#[test]
fn a_navigation_reads_the_intermediate_reference_field() {
    let (operations, _) = vector(&por_world::navigation(), por_world::no_alarm);
    assert_eq!(operations, ["point", "boom"]);
}

/// FR-155-AC-6: a postcondition reading through `result` reads the field on
/// any object and the any-object membership of the result's population, so
/// `set` joins the candidate seeded by `disarm` and the run returns
/// `Violated` by `set, pick`.
#[trace("TC-589", "FR-155-AC-6")]
#[test]
fn a_postcondition_reading_through_result_reads_the_result_population() {
    let (operations, _) = vector(&por_world::result_read(), por_world::no_alarm);
    assert_eq!(operations, ["set", "pick"]);
}

// ---------------------------------------------------------------------------
// Partial-order reduction with symmetry (FR-162-AC-3)
// ---------------------------------------------------------------------------

/// §7.2's subject with `counters` declared `symmetric`.
fn symmetric_bumps() -> World {
    let shape = Shape {
        symmetric: true,
        max: 2,
        link: false,
    };
    World::new(
        shape,
        BUMPS,
        &[Clause {
            profile: INFINITE,
            formula: REACHES_TWO,
        }],
        &[start([0, 0, 0])],
        &KEYS,
    )
}

fn both() -> Reductions {
    Reductions {
        symmetry: vec![SymmetryDeclaration {
            population: counters(),
            classes: vec![KEYS.iter().map(|key| (*key).to_owned()).collect()],
        }],
        partial_order: true,
        ..Reductions::default()
    }
}

/// FR-162-AC-3: with `counters` annotated `symmetric`, `[[a, b, c]]` and
/// partial-order reduction, the instance `x = a` (group `{id, (b c)}`) stores
/// 7 model states, `(0,0,0)`, `(0,0,1)`, `(0,1,1)`, `(0,1,2)`, `(0,2,2)`,
/// `(1,2,2)` and `(2,2,2)`, with every edge's permutation mapping the
/// concrete successor to its canonical target; the run proves the claim and
/// settles `Proved{Reduced{[Symmetry{..}, PartialOrder{BreadthFirstRevisit}]},
/// Uncertified}`. Two runs retain equal edge lists.
#[trace("TC-587", "FR-162-AC-3", "FR-162-AC-4")]
#[test]
fn symmetry_and_partial_order_reduce_together_on_canonical_states() {
    let world = symmetric_bumps();
    let system = world.system();
    let letters = world.letters(0, vec![atom(|v| Some(v == 2))]);
    let instances = counter_instance("a");
    let por = counters_por(&world, &system, &letters, &instances);
    let stabiliser = SymmetryGroup {
        classes: vec![SymmetryClass {
            population: counters(),
            keys: vec!["b".to_owned(), "c".to_owned()],
        }],
    };
    let retain = || {
        retain_reduced(
            &system,
            &stabiliser,
            Some(&ModelAmple(&por)),
            Limits::default(),
            IdentityLimits::default(),
            None,
            || false,
        )
        .expect("the states encode")
        .graph
    };
    let graph = retain();
    assert_eq!(
        graph
            .states()
            .iter()
            .map(|state| values(&state.state))
            .collect::<Vec<_>>(),
        [
            (0, 0, 0),
            (0, 0, 1),
            (0, 1, 1),
            (0, 1, 2),
            (0, 2, 2),
            (1, 2, 2),
            (2, 2, 2)
        ]
    );
    let again = retain();
    assert_eq!(graph.edges(), again.edges());
    // Every target is canonical: a concrete successor sorts its `b` and `c`.
    for edge in graph.edges() {
        let (_, b, c) = values(&graph.state(edge.target).expect("a target").state);
        assert!(b <= c, "{b} <= {c}");
    }

    let outcome = world
        .run(
            Some(0),
            vec![atom(|v| Some(v == 2))],
            both(),
            None,
            ModelCheckLimits::default(),
        )
        .expect("checks");
    let ModelCheckOutcome::HoldsReduced { reductions, .. } = &outcome else {
        panic!("expected a reduced proof, got {outcome:?}");
    };
    assert!(matches!(
        reductions.as_slice(),
        [
            AppliedReduction::Symmetry { .. },
            AppliedReduction::PartialOrder {
                proviso: Proviso::BreadthFirstRevisit
            }
        ]
    ));
}

// ---------------------------------------------------------------------------
// Settling a partial-order run (FR-160)
// ---------------------------------------------------------------------------

/// FR-127's map over an outcome of `world`'s subject.
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

fn run_reaches_two(
    world: &World,
    selection: Reductions,
    max_depth: Option<usize>,
    limits: ModelCheckLimits,
) -> ModelCheckOutcome {
    world
        .run(
            Some(0),
            vec![atom(|v| Some(v == 2))],
            selection,
            max_depth,
            limits,
        )
        .expect("checks")
}

/// FR-160-AC-1's partial-order row and FR-160-AC-7's completed proof: §7.2's
/// instance settles `proved`, `closed-scope`, `Proved{Reduced{[PartialOrder{
/// BreadthFirstRevisit}]}, Uncertified}`, success.
#[trace("TC-584", "FR-160-AC-1", "FR-160-AC-7")]
#[test]
fn a_completed_partial_order_proof_settles_as_a_reduced_proof() {
    let world = reaches_two();
    let outcome = run_reaches_two(&world, reduced(), None, ModelCheckLimits::default());
    let record = settle(&world, &outcome);
    assert_eq!(
        record.value,
        TerminalValue::Proved {
            basis: ProofBasis::Reduced {
                reductions: vec![AppliedReduction::PartialOrder {
                    proviso: Proviso::BreadthFirstRevisit,
                }],
            },
            certification: Certification::Uncertified,
        }
    );
    assert_eq!(record.label, ResultLabel::Proved);
    assert_eq!(record.basis, SettlementBasis::ClosedScope);
    assert_eq!(record.value.category(), Category::Success);
}

/// FR-160-AC-3: §7.2's TP-4 instance with partial-order reduction and
/// `max_depth` 2 settles `inconclusive`, `unsettled`, `ReductionHorizon{
/// max_depth: 2}`, wire cause `reduction-horizon`; with symmetry only over
/// §7.1 and `max_depth` 1 it settles `BoundReached{depth: 1}`.
#[trace("TC-584", "FR-160-AC-3")]
#[test]
fn a_partial_order_run_to_its_horizon_settles_reduction_horizon() {
    let world = reaches_two();
    let outcome = run_reaches_two(&world, reduced(), Some(2), ModelCheckLimits::default());
    assert!(matches!(
        outcome,
        ModelCheckOutcome::Reduction {
            cause: ReductionCause::Horizon { max_depth: 2 },
            ..
        }
    ));
    let record = settle(&world, &outcome);
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::ReductionHorizon { max_depth: 2 })
    );
    assert_eq!(record.label, ResultLabel::Inconclusive);
    assert_eq!(record.basis, SettlementBasis::Unsettled);
    assert_eq!(
        InconclusiveCause::ReductionHorizon { max_depth: 2 }.as_str(),
        "reduction-horizon"
    );

    let configs = symmetric_bumps();
    let outcome = run_reaches_two(
        &configs,
        Reductions {
            symmetry: both().symmetry,
            ..Reductions::default()
        },
        Some(1),
        ModelCheckLimits::default(),
    );
    let record = settle(&configs, &outcome);
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(InconclusiveCause::BoundReached { depth: 1 })
    );
}

/// FR-160-AC-7: §7.2's TP-4 instance with partial-order reduction and
/// `max_states` 3 stops before completing its method and settles V-7,
/// `incomplete`, never `failed`.
#[trace("TC-584", "FR-160-AC-7")]
#[test]
fn a_partial_order_run_a_limit_stops_settles_as_incomplete() {
    let world = reaches_two();
    let limits = ModelCheckLimits {
        limits: Limits {
            max_states: 3,
            ..Limits::default()
        },
        ..ModelCheckLimits::default()
    };
    let outcome = run_reaches_two(&world, reduced(), None, limits);
    assert!(matches!(
        outcome,
        ModelCheckOutcome::Stopped {
            cause: IncompleteCause::ResourceExhausted,
            limit: Some(ReachedLimit {
                limit: ModelCheckLimit::MaxStates,
                value: 3
            }),
            ..
        }
    ));
    let record = settle(&world, &outcome);
    assert_eq!(
        record.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: 3,
        })
    );
    assert_eq!(ModelCheckLimit::MaxStates.setting(), "max_states");
}

/// FR-156 "Visibility": a transition is also visible when its write footprint
/// meets a location the state constraint reads, whatever the atoms read. §7.3's
/// `Low` reads `v` of every counter, so every bump is visible under it and
/// none is without it.
#[trace("TC-576")]
#[test]
fn a_state_constraint_makes_the_transitions_it_reads_visible() {
    let shape = Shape {
        symmetric: false,
        max: 3,
        link: false,
    };
    let body = "pre Below using v on M::Counter::bump { self.v < 3 }\n\
         post Up using v on M::Counter::bump { self.v - pre(self.v) = 1 }\n\
         invariant Low using v on M::Counter at current { self.v <= 1 }\n";
    let world = World::new(shape, body, &[], &[start([0, 0, 0])], &KEYS);
    let system = world.system();
    // The constraint holds for every counter, so its reads are on any counter
    // and on the membership of any object of `counters`.
    assert_eq!(
        system.constraint_reads("Low").expect("a footprint"),
        [
            Location::AnyField {
                field: counter_field()
            },
            Location::AnyMembership {
                population: counters()
            }
        ]
        .into()
    );
    let initial = system.initial()[0].clone();
    let bumps: Vec<ModelTransition> = system
        .successors(&initial)
        .expect("expands")
        .successors
        .into_iter()
        .map(|(transition, _)| transition)
        .collect();
    assert_eq!(bumps.len(), 3);
    for (constraint, expected) in [(Some("Low"), true), (None, false)] {
        let por = Por::new(&system, None, &[], &[], constraint).expect("the footprints derive");
        for bump in &bumps {
            assert_eq!(por.visible(&initial, bump).expect("visibility"), expected);
        }
    }
    // Every bump visible, nothing is left out: the run is the constrained run.
    let constrained = |partial_order| {
        world
            .run(
                None,
                vec![],
                Reductions {
                    partial_order,
                    constraint: Some("Low".to_owned()),
                    ..Reductions::default()
                },
                None,
                ModelCheckLimits::default(),
            )
            .expect("checks")
    };
    assert_eq!(constrained(true), constrained(false));
}

/// ADR-021 §7.6's operation with no footprint rule: `spin()` calls a function
/// in its precondition. The unreduced run refutes the claim; partial-order
/// reduction refuses before it expands a state, naming the clause, instead of
/// stopping an expansion as a broken invariant.
#[trace("TC-587")]
#[test]
fn an_operation_with_no_footprint_rule_refuses_partial_order_reduction() {
    let fixture = por_world::call_in_a_precondition();
    let plain = fixture
        .run(Some(0), por_world::no_f(), Reductions::default())
        .expect("checks");
    assert!(matches!(plain, ModelCheckOutcome::Violated { .. }));
    let refused = fixture
        .run(Some(0), por_world::no_f(), reduced())
        .expect_err("a footprint-less operation refuses");
    assert!(
        matches!(
            refused,
            ModelCheckRefusal::Footprint(qsl_eval::simulation::model::FootprintError::Refused(_))
        ),
        "{refused:?}"
    );
}

/// FR-162's absent objects, end to end: reduced and unreduced runs agree on
/// the deadlock-freedom item and on a safety claim of a subject whose probe
/// is over an item no state holds yet.
#[trace("TC-587")]
#[test]
fn a_probe_over_an_absent_object_keeps_the_verdicts() {
    let fixture = por_world::makers(&["always holds(not x.f)"]);
    for clause in [None, Some(0)] {
        let atoms = || {
            if clause.is_some() {
                por_world::no_f()
            } else {
                Vec::new()
            }
        };
        let violated = |selection: Reductions| {
            matches!(
                fixture.run(clause, atoms(), selection),
                Ok(ModelCheckOutcome::Violated { .. })
            )
        };
        assert_eq!(violated(Reductions::default()), violated(reduced()));
    }
}
