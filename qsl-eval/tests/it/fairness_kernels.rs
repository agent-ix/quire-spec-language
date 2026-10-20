// SPDX-License-Identifier: AGPL-3.0-or-later
//! The fairness kernels (ADR-018 FA-1 to FA-6, CX-3; ADR-019 FS-2): SCC
//! decomposition of a subgraph, constraint membership, the fairness filter
//! with strong-fairness refinement and its witness tree, and lasso fairness.
//!
//! The fixtures are the product graphs of the worked examples the engines'
//! acceptance criteria use: ADR-018 section 6's `ConfigVersion` cycle and
//! ADR-019 section 6's mutexes, each explored here against the automaton of
//! the negated liveness claim (`support::automata::negated_liveness`). The
//! engines build the same graph from a `ModelSystem`; the kernels read it.

use std::collections::{BTreeMap, BTreeSet};

use qsl_eval::automaton::{Gba, StateId};
use qsl_eval::fairness::{
    analyze, check_loop, components, Analysis, Constraint, EdgeLabel, FairnessError, Graph,
    Identity, LassoFairness, LoopStep, Outcome, Vertex,
};
use quire_exact::{Cancel, CancelCause};

use crate::support::automata::{letter, negated_liveness};

/// The index of `q0` and `q1` in `negated_liveness`.
const Q0: usize = 1;
const Q1: usize = 2;

type Moves = fn(u8) -> Vec<(u64, u8)>;

/// A product graph, with the (model state, automaton state) each vertex
/// stands for.
struct Product {
    graph: Graph,
    nodes: Vec<(u8, StateId)>,
    index: BTreeMap<(u8, StateId), Vertex>,
    moves: Moves,
}

impl Product {
    fn vertex(&mut self, node: (u8, StateId)) -> Vertex {
        if let Some(vertex) = self.index.get(&node) {
            return *vertex;
        }
        let enabled = (self.moves)(node.0)
            .into_iter()
            .map(|(identity, _)| Identity::new(identity));
        let vertex = self.graph.add_vertex(enabled);
        self.index.insert(node, vertex);
        self.nodes.push(node);
        vertex
    }

    fn all(&self) -> BTreeSet<Vertex> {
        self.graph.vertices().collect()
    }

    /// `(model state, automaton state index)` of each vertex, ascending.
    fn labels(&self, vertices: &[Vertex]) -> Vec<(u8, usize)> {
        let mut labels: Vec<(u8, usize)> = vertices
            .iter()
            .map(|vertex| {
                let (model, automaton) = self.nodes[vertex.index()];
                (model, automaton.index())
            })
            .collect();
        labels.sort_unstable();
        labels
    }
}

/// Explore breadth-first from `initial` under `moves`, in the listed order,
/// against the negated-liveness automaton whose atom 0 is `goal`. Every
/// identity a model state lists is enabled there, whether or not the
/// automaton gives it a move (ADR-019 FS-1).
fn product(initial: u8, moves: Moves, goal: fn(u8) -> bool) -> Product {
    let (gba, _, _) = negated_liveness();
    let letter_of = |state: u8| {
        if goal(state) {
            letter(&[0])
        } else {
            letter(&[])
        }
    };
    let mut product = Product {
        graph: Graph::new(),
        nodes: Vec::new(),
        index: BTreeMap::new(),
        moves,
    };
    for automaton in gba
        .successors(Gba::INITIAL, letter_of(initial))
        .expect("the initial state exists")
    {
        product.vertex((initial, automaton));
    }
    let mut processed = 0;
    while processed < product.nodes.len() {
        let node = product.nodes[processed];
        let from = product.index[&node];
        for (identity, next) in moves(node.0) {
            for automaton in gba
                .successors(node.1, letter_of(next))
                .expect("the automaton state exists")
            {
                let target = product.vertex((next, automaton));
                product
                    .graph
                    .add_edge(from, target, EdgeLabel::Transition(Identity::new(identity)))
                    .expect("both vertices exist");
            }
        }
        processed += 1;
    }
    let accepting: Vec<Vertex> = product
        .index
        .iter()
        .filter(|(node, _)| gba.is_accepting_in(0, node.1))
        .map(|(_, vertex)| *vertex)
        .collect();
    product
        .graph
        .add_acceptance_set(accepting)
        .expect("every vertex exists");
    product
}

fn id(value: u64) -> Identity {
    Identity::new(value)
}

fn run(product: &Product, constraints: &[Constraint]) -> Result<Vec<Analysis>, FairnessError> {
    analyze(&product.graph, &product.all(), constraints, &Cancel::new())
}

// ADR-018 section 6: `ConfigVersion` with versions `(va, vb)` encoded as
// `va * 3 + vb`; `upd(a)` is identity 0 and `upd(b)` identity 1, each
// advancing its version modulo 3. The claim is `always eventually vb = 2`;
// atom 0 is `vb != 2`.
fn config_version(state: u8) -> Vec<(u64, u8)> {
    let (va, vb) = (state / 3, state % 3);
    vec![(0, ((va + 1) % 3) * 3 + vb), (1, va * 3 + (vb + 1) % 3)]
}

fn vb_is_not_two(state: u8) -> bool {
    state % 3 != 2
}

#[test]
fn the_config_version_product_has_fifteen_states_and_two_accepting_components() {
    let product = product(0, config_version, vb_is_not_two);
    assert_eq!(product.graph.vertex_count(), 15);
    let q0_count = product
        .nodes
        .iter()
        .filter(|(_, q)| q.index() == Q0)
        .count();
    assert_eq!((q0_count, product.nodes.len() - q0_count), (9, 6));

    let analyses = run(&product, &[]).expect("not cancelled");
    let sizes: Vec<usize> = analyses.iter().map(|a| a.vertices().len()).collect();
    assert_eq!(sizes, vec![9, 3, 3]);
    let accepting: Vec<Vec<(u8, usize)>> = analyses
        .iter()
        .skip(1)
        .map(|a| product.labels(a.vertices()))
        .collect();
    assert_eq!(
        accepting,
        vec![
            vec![(0, Q1), (3, Q1), (6, Q1)],
            vec![(1, Q1), (4, Q1), (7, Q1)]
        ]
    );
}

#[test]
fn weak_each_rejects_both_accepting_components_and_weak_whole_passes_the_first() {
    let product = product(0, config_version, vb_is_not_two);

    let each = [Constraint::weak([id(0)]), Constraint::weak([id(1)])];
    let analyses = run(&product, &each).expect("not cancelled");
    let outcomes: Vec<&Outcome> = analyses.iter().map(Analysis::outcome).collect();
    assert_eq!(
        outcomes,
        vec![
            &Outcome::MissingAcceptance { set: 0 },
            &Outcome::UnfairWeak { constraint: 1 },
            &Outcome::UnfairWeak { constraint: 1 },
        ]
    );
    assert_eq!(Analysis::first_fair_in(&analyses), None);

    let whole = [Constraint::weak([id(0), id(1)])];
    let analyses = run(&product, &whole).expect("not cancelled");
    let fair = Analysis::first_fair_in(&analyses).expect("upd(a) takes the whole constraint");
    assert_eq!(
        product.labels(fair.vertices()),
        vec![(0, Q1), (3, Q1), (6, Q1)]
    );
    // The loop is three `upd(a)` steps: every edge inside the component takes
    // identity 0.
    let members: BTreeSet<Vertex> = fair.vertices().iter().copied().collect();
    let inside: Vec<EdgeLabel> = fair
        .vertices()
        .iter()
        .flat_map(|vertex| product.graph.edges_from(*vertex))
        .filter(|edge| members.contains(&edge.target))
        .map(|edge| edge.label)
        .collect();
    assert_eq!(inside, vec![EdgeLabel::Transition(id(0)); 3]);
}

/// ADR-018 section 6, the shipped unit: one state with the self-loops
/// `upd(a)` and `upd(b)`.
fn shipped_unit(_state: u8) -> Vec<(u64, u8)> {
    vec![(0, 0), (1, 0)]
}

fn always_goal(_state: u8) -> bool {
    true
}

#[test]
fn the_shipped_unit_is_fair_under_weak_each_because_both_operations_loop() {
    let product = product(0, shipped_unit, always_goal);
    let each = [Constraint::weak([id(0)]), Constraint::weak([id(1)])];
    let analyses = run(&product, &each).expect("not cancelled");
    let fair = Analysis::first_fair_in(&analyses).expect("both operations are taken");
    assert_eq!(product.labels(fair.vertices()), vec![(0, Q1)]);
}

// ADR-019 section 6's mutex: owner `0..=2`, `acquire(p)` (identity `p`) when
// the owner is 0, `release` (identity 10) otherwise. The claim is `always
// eventually owner = 1`; atom 0 is `owner != 1`.
fn mutex_two(owner: u8) -> Vec<(u64, u8)> {
    if owner == 0 {
        vec![(1, 1), (2, 2)]
    } else {
        vec![(10, 0)]
    }
}

fn owner_is_not_one(owner: u8) -> bool {
    owner != 1
}

#[test]
fn the_mutex_product_has_five_states_with_no_acquire_one_edge_out_of_the_accepting_start() {
    let product = product(0, mutex_two, owner_is_not_one);
    assert_eq!(product.graph.vertex_count(), 5);
    let start_q1 = product.index[&(0, product.nodes[1].1)];
    let acquire_one = EdgeLabel::Transition(id(1));
    assert!(product
        .graph
        .edges_from(start_q1)
        .iter()
        .all(|edge| edge.label != acquire_one));
    // The identity is still enabled there: it is read from the model state.
    assert!(product
        .graph
        .enabled(start_q1)
        .expect("the vertex exists")
        .contains(&id(1)));
}

#[test]
fn strong_each_holds_where_weak_each_has_a_fair_cycle() {
    let product = product(0, mutex_two, owner_is_not_one);

    let weak = [Constraint::weak([id(1)]), Constraint::weak([id(2)])];
    let analyses = run(&product, &weak).expect("not cancelled");
    let fair = Analysis::first_fair_in(&analyses).expect("acquire(1) is disabled at owner 2");
    assert_eq!(product.labels(fair.vertices()), vec![(0, Q1), (2, Q1)]);

    let strong = [Constraint::strong([id(1)]), Constraint::strong([id(2)])];
    let analyses = run(&product, &strong).expect("not cancelled");
    assert_eq!(Analysis::first_fair_in(&analyses), None);
    assert_eq!(analyses.len(), 2);
    assert_eq!(
        analyses[0].outcome(),
        &Outcome::MissingAcceptance { set: 0 }
    );
    let Outcome::UnfairStrong { constraints, sub } = analyses[1].outcome() else {
        panic!("the accepting component fails acquire(1)");
    };
    assert_eq!(constraints, &vec![0]);
    assert_eq!(
        product.labels(analyses[1].vertices()),
        vec![(0, Q1), (2, Q1)]
    );
    assert_eq!(sub.len(), 1);
    assert_eq!(sub[0].outcome(), &Outcome::Trivial);
    assert_eq!(product.labels(sub[0].vertices()), vec![(2, Q1)]);
}

#[test]
fn strong_without_granularity_is_taken_by_either_acquire() {
    let product = product(0, mutex_two, owner_is_not_one);
    let whole = [Constraint::strong([id(1), id(2)])];
    let analyses = run(&product, &whole).expect("not cancelled");
    let fair = Analysis::first_fair_in(&analyses).expect("acquire(2) takes the constraint");
    assert_eq!(product.labels(fair.vertices()), vec![(0, Q1), (2, Q1)]);
}

#[test]
fn a_strong_constraint_beside_a_weak_one_on_the_same_operation_decides() {
    let product = product(0, mutex_two, owner_is_not_one);
    let both = [
        Constraint::weak([id(1)]),
        Constraint::weak([id(2)]),
        Constraint::strong([id(1)]),
        Constraint::strong([id(2)]),
    ];
    let analyses = run(&product, &both).expect("not cancelled");
    assert_eq!(Analysis::first_fair_in(&analyses), None);
    let Outcome::UnfairStrong { constraints, .. } = analyses[1].outcome() else {
        panic!("the strong constraint on acquire(1) fails");
    };
    assert_eq!(constraints, &vec![2]);
}

// ADR-019 FR-130-AC-2's `Handoff` mutex: owner `0..=3`; `acquire(p)` for `p`
// in `1..=3` when the owner is 0; `release` (identity 10) when it is not;
// `pass` (identity 11) from owner 2 or 3 to `5 - owner`.
fn handoff(owner: u8) -> Vec<(u64, u8)> {
    match owner {
        0 => vec![(1, 1), (2, 2), (3, 3)],
        2 | 3 => vec![(10, 0), (11, 5 - owner)],
        _ => vec![(10, 0)],
    }
}

#[test]
fn refinement_removes_the_start_state_and_the_rest_passes() {
    let product = product(0, handoff, owner_is_not_one);
    let strong = [
        Constraint::strong([id(1)]),
        Constraint::strong([id(2)]),
        Constraint::strong([id(3)]),
    ];
    let analyses = run(&product, &strong).expect("not cancelled");
    let accepting = analyses
        .iter()
        .find(|a| matches!(a.outcome(), Outcome::UnfairStrong { .. }))
        .expect("the accepting component fails acquire(1)");
    assert_eq!(
        product.labels(accepting.vertices()),
        vec![(0, Q1), (2, Q1), (3, Q1)]
    );
    let Outcome::UnfairStrong { constraints, sub } = accepting.outcome() else {
        panic!("matched above");
    };
    assert_eq!(constraints, &vec![0]);
    assert_eq!(sub.len(), 1);
    assert_eq!(sub[0].outcome(), &Outcome::Fair);
    assert_eq!(product.labels(sub[0].vertices()), vec![(2, Q1), (3, Q1)]);
    assert_eq!(Analysis::first_fair_in(&analyses), Some(&sub[0]));
}

// FR-130-AC-5's three-process mutex: owner `0..=3`, `acquire(p)` for `p` in
// `1..=3` when the owner is 0, `release` otherwise. The claim is `always
// eventually owner = 1 or owner = 2`; atom 0 is `owner = 0 or owner = 3`.
fn mutex_three(owner: u8) -> Vec<(u64, u8)> {
    if owner == 0 {
        vec![(1, 1), (2, 2), (3, 3)]
    } else {
        vec![(10, 0)]
    }
}

fn owner_is_not_one_or_two(owner: u8) -> bool {
    owner == 0 || owner == 3
}

#[test]
fn two_strong_constraints_failing_on_one_component_are_refined_together() {
    let product = product(0, mutex_three, owner_is_not_one_or_two);
    let strong = [
        Constraint::strong([id(1)]),
        Constraint::strong([id(2)]),
        Constraint::strong([id(3)]),
    ];
    let analyses = run(&product, &strong).expect("not cancelled");
    assert_eq!(Analysis::first_fair_in(&analyses), None);
    let accepting = analyses
        .iter()
        .find(|a| matches!(a.outcome(), Outcome::UnfairStrong { .. }))
        .expect("the accepting component fails acquire(1) and acquire(2)");
    assert_eq!(product.labels(accepting.vertices()), vec![(0, Q1), (3, Q1)]);
    let Outcome::UnfairStrong { constraints, sub } = accepting.outcome() else {
        panic!("matched above");
    };
    assert_eq!(constraints, &vec![0, 1]);
    assert_eq!(sub.len(), 1);
    assert_eq!(sub[0].outcome(), &Outcome::Trivial);
    assert_eq!(product.labels(sub[0].vertices()), vec![(3, Q1)]);
}

#[test]
fn a_cancelled_handle_stops_the_filter_before_any_decomposition() {
    let product = product(0, mutex_two, owner_is_not_one);
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let strong = [Constraint::strong([id(1)])];
    assert_eq!(
        analyze(&product.graph, &product.all(), &strong, &cancel),
        Err(FairnessError::Cancelled)
    );
    assert_eq!(
        components(&product.graph, &product.all(), &cancel),
        Err(FairnessError::Cancelled)
    );
}

#[test]
fn each_analysis_is_a_function_of_the_graph_and_the_constraints() {
    let strong = [Constraint::strong([id(1)]), Constraint::strong([id(2)])];
    let first = run(&product(0, mutex_two, owner_is_not_one), &strong);
    let second = run(&product(0, mutex_two, owner_is_not_one), &strong);
    assert_eq!(first, second);
}

/// A graph of `count` vertices with no edges.
fn vertices(count: usize) -> (Graph, Vec<Vertex>) {
    let mut graph = Graph::new();
    let all: Vec<Vertex> = (0..count)
        .map(|_| graph.add_vertex(std::iter::empty()))
        .collect();
    (graph, all)
}

fn link(graph: &mut Graph, from: Vertex, to: Vertex) {
    graph
        .add_edge(from, to, EdgeLabel::Transition(id(0)))
        .expect("both vertices exist");
}

#[test]
fn components_are_found_without_recursion_on_a_long_ring() {
    let (mut graph, all) = vertices(200_000);
    for (from, to) in all.iter().zip(all.iter().cycle().skip(1)) {
        link(&mut graph, *from, *to);
    }
    let within: BTreeSet<Vertex> = all.iter().copied().collect();
    let found = components(&graph, &within, &Cancel::new()).expect("not cancelled");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].vertices(), all.as_slice());
    assert!(found[0].has_edge());
}

#[test]
fn a_subgraph_cuts_a_ring_into_trivial_components_in_discovery_order() {
    let (mut graph, all) = vertices(4);
    for (from, to) in all.iter().zip(all.iter().cycle().skip(1)) {
        link(&mut graph, *from, *to);
    }
    let whole: BTreeSet<Vertex> = all.iter().copied().collect();
    let ring = components(&graph, &whole, &Cancel::new()).expect("not cancelled");
    assert_eq!(ring.len(), 1);
    assert!(!ring[0].is_trivial());

    let cut: BTreeSet<Vertex> = all.iter().copied().take(3).collect();
    let chain = components(&graph, &cut, &Cancel::new()).expect("not cancelled");
    let shapes: Vec<(Vec<Vertex>, bool)> = chain
        .iter()
        .map(|c| (c.vertices().to_vec(), c.is_trivial()))
        .collect();
    assert_eq!(
        shapes,
        vec![
            (vec![all[0]], true),
            (vec![all[1]], true),
            (vec![all[2]], true)
        ]
    );
}

#[test]
fn a_self_loop_is_an_edge_and_components_follow_their_least_vertex() {
    let (mut graph, all) = vertices(5);
    // Cycles (0,2) and (1,3) interleaved, and a self-loop on 4.
    for (from, to) in [(0, 2), (2, 0), (1, 3), (3, 1), (4, 4)] {
        link(&mut graph, all[from], all[to]);
    }
    let within: BTreeSet<Vertex> = all.iter().copied().collect();
    let found = components(&graph, &within, &Cancel::new()).expect("not cancelled");
    let shapes: Vec<(Vec<usize>, bool)> = found
        .iter()
        .map(|c| {
            (
                c.vertices().iter().map(|v| v.index()).collect(),
                c.has_edge(),
            )
        })
        .collect();
    assert_eq!(
        shapes,
        vec![(vec![0, 2], true), (vec![1, 3], true), (vec![4], true)]
    );

    let (lone, single) = vertices(1);
    let alone = components(&lone, &single.iter().copied().collect(), &Cancel::new())
        .expect("not cancelled");
    assert!(alone[0].is_trivial());
}

#[test]
fn a_graph_refuses_a_vertex_it_does_not_have() {
    let (mut graph, all) = vertices(2);
    let (_, larger) = vertices(5);
    let foreign = larger[4];
    assert_eq!(
        graph.add_edge(all[0], foreign, EdgeLabel::Stutter),
        Err(FairnessError::UnknownVertex { vertex: foreign })
    );
    assert_eq!(
        graph.add_acceptance_set([foreign]),
        Err(FairnessError::UnknownVertex { vertex: foreign })
    );
    let within = BTreeSet::from([foreign]);
    assert_eq!(
        components(&graph, &within, &Cancel::new()),
        Err(FairnessError::UnknownVertex { vertex: foreign })
    );
}

#[test]
fn membership_covers_identities_and_the_stutter_belongs_to_none() {
    let whole = Constraint::weak([id(1), id(2)]);
    assert!(whole.covers(id(2)));
    assert!(!whole.covers(id(3)));
    assert!(whole.is_taken_by(EdgeLabel::Transition(id(1))));
    assert!(!whole.is_taken_by(EdgeLabel::Transition(id(3))));
    assert!(!whole.is_taken_by(EdgeLabel::Stutter));
    assert!(whole.is_enabled_in(&BTreeSet::from([id(2), id(9)])));
    assert!(!whole.is_enabled_in(&BTreeSet::from([id(9)])));
    assert!(!whole.is_enabled_in(&BTreeSet::new()));
}

fn loop_step(enabled: &[u64], label: EdgeLabel) -> LoopStep {
    LoopStep::new(enabled.iter().copied().map(id), label)
}

#[test]
fn replay_reads_the_three_upd_a_loop_as_fair_under_whole_and_unfair_under_each() {
    let steps = vec![loop_step(&[0, 1], EdgeLabel::Transition(id(0))); 3];
    let whole = [Constraint::weak([id(0), id(1)])];
    assert_eq!(check_loop(&steps, &whole), Ok(LassoFairness::Fair));
    let each = [Constraint::weak([id(0)]), Constraint::weak([id(1)])];
    assert_eq!(
        check_loop(&steps, &each),
        Ok(LassoFairness::Unfair { constraint: 1 })
    );
}

#[test]
fn the_shipped_unit_loop_taking_both_operations_is_fair_under_each() {
    let steps = [
        loop_step(&[0, 1], EdgeLabel::Transition(id(0))),
        loop_step(&[0, 1], EdgeLabel::Transition(id(1))),
    ];
    let each = [Constraint::weak([id(0)]), Constraint::weak([id(1)])];
    assert_eq!(check_loop(&steps, &each), Ok(LassoFairness::Fair));
}

#[test]
fn a_strong_constraint_enabled_in_the_loop_must_be_taken_in_it() {
    let strong = [Constraint::strong([id(5)])];
    let weak = [Constraint::weak([id(5)])];
    let starved = [
        loop_step(&[5, 6], EdgeLabel::Transition(id(6))),
        loop_step(&[6], EdgeLabel::Transition(id(6))),
    ];
    assert_eq!(
        check_loop(&starved, &strong),
        Ok(LassoFairness::Unfair { constraint: 0 })
    );
    assert_eq!(check_loop(&starved, &weak), Ok(LassoFairness::Fair));

    let never_enabled = [loop_step(&[6], EdgeLabel::Transition(id(6)))];
    assert_eq!(check_loop(&never_enabled, &strong), Ok(LassoFairness::Fair));

    let taken = [
        loop_step(&[5], EdgeLabel::Transition(id(5))),
        loop_step(&[5, 6], EdgeLabel::Transition(id(6))),
    ];
    assert_eq!(check_loop(&taken, &strong), Ok(LassoFairness::Fair));
}

#[test]
fn the_terminal_stutter_loop_is_fair_because_it_enables_nothing() {
    let steps = [loop_step(&[], EdgeLabel::Stutter)];
    let constraints = [Constraint::weak([id(0)]), Constraint::strong([id(1)])];
    assert_eq!(check_loop(&steps, &constraints), Ok(LassoFairness::Fair));
}

#[test]
fn a_lasso_with_no_loop_step_is_refused() {
    assert_eq!(
        check_loop(&[], &[Constraint::weak([id(0)])]),
        Err(FairnessError::EmptyLoop)
    );
}

// ADR-025 MF-2: visibility fairness for one (thread, location) pair. Thread
// `b`'s loads of `l` are identity 10 (depth 1, a stale message) and 11
// (depth 0, the mo-last message); identity 20 is another thread's step.
const STALE: u64 = 10;
const FRESH: u64 = 11;
const OTHER: u64 = 20;

fn visibility() -> Constraint {
    Constraint::visibility([id(STALE), id(FRESH)], [id(FRESH)])
}

/// A two-vertex cycle whose edges take `labels`: the first from `a` to `b`,
/// every other from `b` back to `a`. Every listed identity is enabled at
/// both vertices, so enabledness alone discharges nothing.
fn cycle(labels: &[u64]) -> Graph {
    let mut graph = Graph::new();
    let enabled = [id(STALE), id(FRESH), id(OTHER)];
    let a = graph.add_vertex(enabled);
    let b = graph.add_vertex(enabled);
    for (position, label) in labels.iter().enumerate() {
        let (from, to) = if position == 0 { (a, b) } else { (b, a) };
        graph
            .add_edge(from, to, EdgeLabel::Transition(id(*label)))
            .expect("both vertices exist");
    }
    graph
}

fn filter(graph: &Graph) -> Vec<Outcome> {
    analyze(
        graph,
        &graph.vertices().collect(),
        &[visibility()],
        &Cancel::new(),
    )
    .expect("the filter runs")
    .iter()
    .map(|analysis| analysis.outcome().clone())
    .collect()
}

/// FR-228-AC-3: an SCC on which `b` loads `l` only from a message that is
/// not mo-last fails visibility fairness; one that also holds a load of the
/// mo-last message passes; one that never loads `l` owes nothing.
#[ix_trace_rs::trace("TC-673", "FR-228-AC-3")]
#[test]
fn visibility_fairness_rejects_an_scc_that_loads_only_stale_messages() {
    assert_eq!(
        filter(&cycle(&[STALE, OTHER])),
        [Outcome::UnfairWeak { constraint: 0 }]
    );
    assert_eq!(filter(&cycle(&[STALE, OTHER, FRESH])), [Outcome::Fair]);
    assert_eq!(filter(&cycle(&[OTHER, OTHER])), [Outcome::Fair]);
}

/// ADR-025 MF-2 on a lasso: the loop must load the mo-last message of every
/// pair it loads.
#[ix_trace_rs::trace("TC-673", "FR-228-AC-3")]
#[test]
fn a_loop_that_keeps_loading_a_stale_message_is_unfair_under_visibility() {
    let enabled = [STALE, FRESH, OTHER];
    let stale = [
        loop_step(&enabled, EdgeLabel::Transition(id(STALE))),
        loop_step(&enabled, EdgeLabel::Transition(id(OTHER))),
    ];
    assert_eq!(
        check_loop(&stale, &[visibility()]),
        Ok(LassoFairness::Unfair { constraint: 0 })
    );
    let fresh = [
        loop_step(&enabled, EdgeLabel::Transition(id(STALE))),
        loop_step(&enabled, EdgeLabel::Transition(id(FRESH))),
    ];
    assert_eq!(check_loop(&fresh, &[visibility()]), Ok(LassoFairness::Fair));
    let unloaded = [loop_step(&enabled, EdgeLabel::Transition(id(OTHER)))];
    assert_eq!(
        check_loop(&unloaded, &[visibility()]),
        Ok(LassoFairness::Fair)
    );
}
