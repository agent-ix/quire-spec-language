// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests execute the graph/product APIs; they do not certify temporal formulas.

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::trace;
use qsl_analyze::model_check::{
    retain, Product, ProductFinding, ProductKey, ProductTransition, PropertyComponent,
    RetainedGraph,
};
use qsl_eval::simulation::{Expansion, ExpansionStop, Limit, Limits, Outcome, TransitionSystem};
use qsl_foundation::{CatalogCode, IdentityLimits};

type Graph = RetainedGraph<usize, String, usize>;

/// An in-memory implementation of the existing transition port. The edge
/// inventory is deliberately unsorted; all graph behavior is production code.
#[derive(Clone)]
struct Model {
    initial: Vec<usize>,
    edges: BTreeMap<usize, Vec<(String, usize)>>,
    stop: Option<usize>,
}

impl Model {
    fn new(initial: &[usize], edges: &[(usize, &str, usize)]) -> Self {
        let mut result = Self {
            initial: initial.to_vec(),
            edges: BTreeMap::new(),
            stop: None,
        };
        for &(source, transition, target) in edges {
            result
                .edges
                .entry(source)
                .or_default()
                .push((transition.to_owned(), target));
        }
        result
    }
}

const STOP: CatalogCode = CatalogCode::new("resource_exhausted", "insufficient-next-charge");

impl TransitionSystem for Model {
    type State = usize;
    type TransitionId = String;
    type Key = usize;
    type Finding = usize;

    fn initial(&self) -> Vec<usize> {
        self.initial.clone()
    }
    fn key(&self, state: &usize) -> usize {
        *state
    }
    fn successors(&self, state: &usize) -> Result<Expansion<String, usize, usize>, ExpansionStop> {
        if self.stop == Some(*state) {
            return Err(ExpansionStop { cause: STOP });
        }
        Ok(Expansion {
            successors: self.edges.get(state).cloned().unwrap_or_default(),
            findings: vec![*state],
        })
    }
}

fn run(model: &Model) -> Graph {
    let result = retain(
        model,
        Limits::default(),
        IdentityLimits::default(),
        None,
        || false,
    )
    .unwrap();
    assert!(matches!(result.outcome, Outcome::Exhaustive(_)));
    result.graph
}

fn edges(graph: &Graph) -> Vec<(usize, &str, usize)> {
    graph
        .edges()
        .iter()
        .map(|edge| {
            (
                graph.state(edge.source).unwrap().state,
                edge.transition.as_str(),
                graph.state(edge.target).unwrap().state,
            )
        })
        .collect()
}

fn partitions(graph: &Graph) -> Vec<(Vec<usize>, bool)> {
    graph
        .components()
        .into_iter()
        .map(|component| {
            (
                component
                    .states
                    .into_iter()
                    .map(|id| graph.state(id).unwrap().state)
                    .collect(),
                component.cyclic,
            )
        })
        .collect()
}

/// Retention is only the unreduced part of FR-162-AC-2.
#[trace("TC-587", "FR-162-AC-2")]
#[trace("TC-453", "FR-101-AC-1", "FR-101-AC-9")]
#[test]
fn coalescing_retains_incoming_back_self_parallel_and_repeated_edges() {
    let model = Model::new(
        &[8, 0, 0],
        &[
            (0, "z", 2),
            (0, "a", 1),
            (0, "a", 1),
            (0, "b", 1),
            (1, "merge", 2),
            (2, "back", 0),
            (2, "self", 2),
        ],
    );
    let graph = run(&model);
    assert_eq!(
        graph
            .states()
            .iter()
            .map(|state| state.state)
            .collect::<Vec<_>>(),
        [0, 8, 1, 2]
    );
    assert_eq!(
        edges(&graph),
        [
            (0, "a", 1),
            (0, "a", 1),
            (0, "b", 1),
            (0, "z", 2),
            (1, "merge", 2),
            (2, "back", 0),
            (2, "self", 2)
        ]
    );
    assert_eq!(graph.initial().len(), 2);
    for edge in graph.edges() {
        let target = &graph.state(edge.target).unwrap().state;
        assert_eq!(edge.permutation.apply(target), target);
    }
    for (index, state) in graph.states().iter().enumerate() {
        assert!(state.expanded);
        assert_eq!(state.findings, [state.state]);
        let id = graph
            .initial()
            .iter()
            .chain(graph.edges().iter().map(|edge| &edge.target))
            .find(|id| id.index() == index)
            .unwrap();
        assert_eq!(
            graph.outgoing(*id).count(),
            graph
                .edges()
                .iter()
                .filter(|edge| edge.source == *id)
                .count()
        );
    }
}

/// EN-1 SCC support, not the whole temporal example in this criterion.
#[test]
fn components_include_disconnected_cycles_self_loops_parallel_edges_and_isolates() {
    let graph = run(&Model::new(
        &[9, 0, 6],
        &[
            (0, "a", 1),
            (1, "a", 0),
            (1, "out", 2),
            (2, "a", 3),
            (2, "parallel", 3),
            (3, "a", 2),
            (3, "out", 4),
            (4, "self", 4),
            (6, "a", 7),
            (7, "a", 6),
        ],
    ));
    assert_eq!(
        partitions(&graph),
        [
            (vec![0, 1], true),
            (vec![6, 7], true),
            (vec![9], false),
            (vec![2, 3], true),
            (vec![4], true)
        ]
    );
    assert_eq!(graph.edges().len(), 10);
}

/// Detects recursion on a production path, not a separate SCC prototype.
#[test]
fn a_deep_chain_and_cycle_decompose_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let mut model = Model::new(&[0], &[]);
            for source in 0..30_000 {
                model
                    .edges
                    .insert(source, vec![("next".to_owned(), source + 1)]);
            }
            let chain = run(&model);
            let parts = partitions(&chain);
            assert_eq!(parts.len(), 30_001);
            assert!(parts
                .iter()
                .enumerate()
                .all(|(index, (members, cyclic))| members == &[index] && !cyclic));
            model.edges.insert(30_000, vec![("back".to_owned(), 0)]);
            let cycle = run(&model);
            assert_eq!(
                partitions(&cycle),
                [((0..=30_000).collect::<Vec<_>>(), true)]
            );
            assert_eq!(cycle.edges().len(), 30_001);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[trace("TC-453", "FR-101-AC-1")]
#[test]
fn canonical_order_is_independent_of_port_inventory_order() {
    let model = Model::new(
        &[2, 0],
        &[(0, "same", 9), (0, "same", 3), (0, "a", 8), (3, "back", 0)],
    );
    let first = run(&model);
    assert_eq!(
        edges(&first),
        [(0, "a", 8), (0, "same", 3), (0, "same", 9), (3, "back", 0)]
    );
    let mut reordered = model;
    reordered.initial.reverse();
    for successors in reordered.edges.values_mut() {
        successors.reverse();
    }
    assert_eq!(first, run(&reordered));
}

#[trace("TC-455", "FR-101-AC-7")]
#[test]
fn inclusive_caps_complete_exact_graphs_and_preserve_partial_edges() {
    let model = Model::new(&[0], &[(0, "next", 1), (1, "back", 0), (1, "self", 1)]);
    let limits = Limits {
        max_states: 2,
        max_transitions: 3,
    };
    let exact = retain(&model, limits, IdentityLimits::default(), None, || false).unwrap();
    assert!(
        matches!(exact.outcome, Outcome::Exhaustive(stats) if stats.states == 2 && stats.transitions == 3 && stats.depth == 1)
    );
    let edge_cap = retain(
        &model,
        limits.with_max_transitions(2),
        IdentityLimits::default(),
        None,
        || false,
    )
    .unwrap();
    assert!(matches!(
        edge_cap.outcome,
        Outcome::Bounded {
            limit: Limit::Transitions(2),
            ..
        }
    ));
    assert_eq!(edges(&edge_cap.graph), [(0, "next", 1), (1, "back", 0)]);
    assert!(!edge_cap.graph.states()[1].expanded);
    assert!(edge_cap.graph.states()[1].findings.is_empty());
    let state_cap = retain(
        &model,
        limits.with_max_states(1),
        IdentityLimits::default(),
        None,
        || false,
    )
    .unwrap();
    assert!(
        matches!(state_cap.outcome, Outcome::Bounded { stats, limit: Limit::States(1), ref frontier } if stats.states == 1 && stats.transitions == 1 && frontier.len() == 2)
    );
    assert!(state_cap.graph.edges().is_empty());
}

#[trace("TC-455", "FR-101-AC-7")]
#[test]
fn horizon_retains_closing_edges_from_states_below_the_bound() {
    let model = Model::new(&[0], &[(0, "next", 1), (1, "back", 0), (1, "next", 2)]);
    let result = retain(
        &model,
        Limits::default(),
        IdentityLimits::default(),
        Some(2),
        || false,
    )
    .unwrap();
    assert!(
        matches!(result.outcome, Outcome::BoundReached { depth: 2, ref frontier, .. } if frontier.len() == 1)
    );
    assert_eq!(
        edges(&result.graph),
        [(0, "next", 1), (1, "back", 0), (1, "next", 2)]
    );
    assert_eq!(
        partitions(&result.graph),
        [(vec![0, 1], true), (vec![2], false)]
    );
    assert!(!result.graph.states()[2].expanded);
    let zero = retain(
        &model,
        Limits::default(),
        IdentityLimits::default(),
        Some(0),
        || false,
    )
    .unwrap();
    assert!(matches!(
        zero.outcome,
        Outcome::BoundReached { depth: 0, .. }
    ));
    assert!(zero.graph.edges().is_empty());
}

#[trace("TC-453", "FR-101-AC-9")]
#[test]
fn initial_caps_and_empty_initial_set_are_explicit() {
    let model = Model::new(&[2, 0, 0, 1], &[]);
    for cap in [0, 2] {
        let result = retain(
            &model,
            Limits::default().with_max_states(cap),
            IdentityLimits::default(),
            None,
            || false,
        )
        .unwrap();
        assert_eq!(result.graph.states().len(), cap);
        assert!(
            matches!(result.outcome, Outcome::Bounded { ref frontier, limit: Limit::States(value), .. } if value == cap && frontier.len() == 3)
        );
    }
    let empty = retain(
        &Model::new(&[], &[]),
        Limits::default(),
        IdentityLimits::default(),
        None,
        || false,
    )
    .unwrap();
    assert!(
        matches!(empty.outcome, Outcome::Exhaustive(stats) if stats.states == 0 && stats.transitions == 0 && stats.depth == 0)
    );
    assert!(empty.graph.components().is_empty());
}

#[trace("TC-455", "FR-101-AC-6")]
#[trace("TC-474", "FR-101-AC-12", "FR-101-AC-13")]
#[test]
fn cancellation_and_model_stops_preserve_frontier_and_findings() {
    let mut model = Model::new(&[0], &[(0, "a", 1), (0, "b", 2)]);
    model.stop = Some(1);
    let stopped = retain(
        &model,
        Limits::default(),
        IdentityLimits::default(),
        None,
        || false,
    )
    .unwrap();
    let expected: Vec<_> = stopped.graph.states()[1..]
        .iter()
        .map(|state| state.digest)
        .collect();
    assert!(
        matches!(stopped.outcome, Outcome::Stopped { cause, ref frontier, .. } if cause == STOP && frontier == &expected)
    );
    assert_eq!(stopped.graph.states()[0].findings, [0]);
    assert!(stopped.graph.states()[1].findings.is_empty());
    let cancelled = retain(
        &model,
        Limits::default(),
        IdentityLimits::default(),
        None,
        || true,
    )
    .unwrap();
    assert!(
        matches!(cancelled.outcome, Outcome::Cancelled { stats, ref frontier, .. } if stats.transitions == 0 && frontier.len() == 1)
    );
    assert!(cancelled.graph.states()[0].findings.is_empty());
}

/// A genuine nondeterministic property-state component: q0 keeps searching,
/// q1 commits to seeing only letters whose second counter is not 2. This is
/// the supplied automaton from ADR-018 §6, not a formula translator/evaluator.
struct Property {
    stop: bool,
}

impl PropertyComponent<Model> for Property {
    type State = u8;
    type Key = u8;
    type Finding = &'static str;
    fn initial(&self, model: &usize) -> Vec<u8> {
        if model % 3 == 2 {
            vec![0]
        } else {
            vec![1, 0]
        }
    }
    fn key(&self, state: &u8) -> u8 {
        *state
    }
    fn advance(
        &self,
        state: &u8,
        _source: &usize,
        _transition: &ProductTransition<String>,
        target: &usize,
    ) -> Result<Vec<u8>, ExpansionStop> {
        if self.stop {
            return Err(ExpansionStop { cause: STOP });
        }
        Ok(match (*state, target % 3 == 2) {
            (0, false) => vec![1, 0],
            (0, true) => vec![0],
            (_, false) => vec![1],
            (_, true) => vec![],
        })
    }
    fn findings(&self, state: &u8, _model: &usize) -> Result<Vec<&'static str>, ExpansionStop> {
        Ok(if *state == 1 {
            vec!["committed"]
        } else {
            vec![]
        })
    }
    fn close(&self, state: &u8, _model: &usize) -> Result<Vec<&'static str>, ExpansionStop> {
        Ok(if *state == 1 {
            vec!["closed-unmet"]
        } else {
            vec!["closed-search"]
        })
    }
}

fn counters() -> Model {
    let mut model = Model::new(&[0], &[]);
    for x in 0..3 {
        for y in 0..3 {
            model.edges.insert(
                x * 3 + y,
                vec![
                    ("b".to_owned(), x * 3 + (y + 1) % 3),
                    ("a".to_owned(), ((x + 1) % 3) * 3 + y),
                ],
            );
        }
    }
    model
}

#[trace("TC-587", "FR-162-AC-2")]
#[test]
fn product_preserves_distinct_property_states_and_all_model_edges() {
    let model = counters();
    let property = Property { stop: false };
    let product = Product::infinite(&model, &property);
    let result = retain(
        &product,
        Limits {
            max_states: 15,
            max_transitions: 39,
        },
        IdentityLimits::default(),
        None,
        || false,
    )
    .unwrap();
    let graph = &result.graph;
    assert!(
        matches!(result.outcome, Outcome::Exhaustive(stats) if stats.states == 15 && stats.transitions == 39)
    );
    let pairs: BTreeSet<_> = graph
        .states()
        .iter()
        .map(|state| (state.state.model, state.state.property))
        .collect();
    let expected: BTreeSet<_> = (0..9)
        .flat_map(|model| {
            if model % 3 == 2 {
                vec![(model, 0)]
            } else {
                vec![(model, 0), (model, 1)]
            }
        })
        .collect();
    assert_eq!(pairs, expected);
    assert_eq!(graph.initial().len(), 2);
    let mut actual_edges = BTreeMap::new();
    for edge in graph.edges() {
        let source = &graph.state(edge.source).unwrap().state;
        let target = &graph.state(edge.target).unwrap().state;
        let ProductTransition::Model(identity) = &edge.transition else {
            panic!("no terminal state in the counters");
        };
        *actual_edges
            .entry((
                (source.model, source.property),
                identity.clone(),
                (target.model, target.property),
            ))
            .or_insert(0) += 1;
        assert_eq!(edge.permutation.apply(target), target);
    }
    // Independently authored transition relation for the supplied automaton.
    let mut expected_edges = BTreeMap::new();
    for &(m, q) in &expected {
        let x = m / 3;
        let y = m % 3;
        for (identity, target) in [("a", ((x + 1) % 3) * 3 + y), ("b", x * 3 + (y + 1) % 3)] {
            if q == 0 {
                expected_edges.insert(((m, q), identity.to_owned(), (target, 0)), 1);
            }
            if target % 3 != 2 {
                expected_edges.insert(((m, q), identity.to_owned(), (target, 1)), 1);
            }
        }
    }
    assert_eq!(actual_edges, expected_edges);
    let mut components: Vec<_> = graph
        .components()
        .into_iter()
        .map(|component| {
            let mut members: Vec<_> = component
                .states
                .into_iter()
                .map(|id| {
                    let state = &graph.state(id).unwrap().state;
                    (state.model, state.property)
                })
                .collect();
            members.sort_unstable();
            assert!(component.cyclic);
            members
        })
        .collect();
    components.sort();
    let mut expected_components = vec![
        (0..9).map(|m| (m, 0)).collect::<Vec<_>>(),
        vec![(0, 1), (3, 1), (6, 1)],
        vec![(1, 1), (4, 1), (7, 1)],
    ];
    expected_components.sort();
    assert_eq!(components, expected_components);
    assert_eq!(
        result,
        retain(
            &product,
            Limits::default(),
            IdentityLimits::default(),
            None,
            || false
        )
        .unwrap()
    );
}

#[test]
fn bounded_terminal_closes_while_infinite_terminal_advances_property() {
    let model = Model::new(&[0], &[]);
    let property = Property { stop: false };
    let bounded = retain(
        &Product::bounded(&model, &property),
        Limits::default(),
        IdentityLimits::default(),
        None,
        || false,
    )
    .unwrap();
    assert!(bounded.graph.edges().is_empty());
    assert_eq!(bounded.graph.states().len(), 2);
    assert_eq!(
        bounded.graph.states()[1].findings,
        [
            ProductFinding::Model(0),
            ProductFinding::Property("committed"),
            ProductFinding::Property("closed-unmet")
        ]
    );
    assert!(bounded
        .graph
        .components()
        .iter()
        .all(|component| !component.cyclic));
    let infinite = retain(
        &Product::infinite(&model, &property),
        Limits::default(),
        IdentityLimits::default(),
        None,
        || false,
    )
    .unwrap();
    assert_eq!(infinite.graph.edges().len(), 3);
    assert!(infinite
        .graph
        .edges()
        .iter()
        .all(|edge| edge.transition == ProductTransition::TerminalStutter));
    let moves: BTreeSet<_> = infinite
        .graph
        .edges()
        .iter()
        .map(|edge| {
            (
                infinite.graph.state(edge.source).unwrap().state.property,
                infinite.graph.state(edge.target).unwrap().state.property,
            )
        })
        .collect();
    assert_eq!(moves, BTreeSet::from([(0, 0), (0, 1), (1, 1)]));
    assert!(infinite
        .graph
        .components()
        .iter()
        .all(|component| component.cyclic));
    assert!(!infinite
        .graph
        .states()
        .iter()
        .flat_map(|state| &state.findings)
        .any(|finding| matches!(
            finding,
            ProductFinding::Property("closed-unmet" | "closed-search")
        )));
}

#[trace("TC-453", "FR-101-AC-2")]
#[test]
fn product_keys_are_unambiguous_and_stutter_cannot_alias_model_identity() {
    let key = |model: &str, property: &str| {
        quire_canonical::to_vec(
            &ProductKey {
                model: model.to_owned(),
                property: property.to_owned(),
            },
            quire_canonical::Limits::new(100),
        )
        .unwrap()
    };
    assert_eq!(key("ab", "c"), br#"["ab","c"]"#);
    assert_ne!(key("ab", "c"), key("a", "bc"));
    assert_ne!(key("a", "q0"), key("a", "q1"));
    let encode_step =
        |step| quire_canonical::to_vec(&step, quire_canonical::Limits::new(100)).unwrap();
    assert_ne!(
        encode_step(ProductTransition::Model("terminal-stutter".to_owned())),
        encode_step(ProductTransition::<String>::TerminalStutter)
    );
}

#[trace("TC-474", "FR-101-AC-12")]
#[test]
fn property_stops_propagate_without_retaining_a_partial_product_expansion() {
    let model = Model::new(&[0], &[(0, "a", 1)]);
    let property = Property { stop: true };
    let result = retain(
        &Product::infinite(&model, &property),
        Limits::default(),
        IdentityLimits::default(),
        None,
        || false,
    )
    .unwrap();
    assert!(
        matches!(result.outcome, Outcome::Stopped { cause, ref frontier, .. } if cause == STOP && frontier.len() == 2)
    );
    assert!(result.graph.edges().is_empty());
    assert!(result
        .graph
        .states()
        .iter()
        .all(|state| !state.expanded && state.findings.is_empty()));
}

#[trace("TC-453", "FR-101-AC-11")]
#[test]
fn canonical_encoding_failures_are_typed_errors() {
    let model = Model::new(&[0], &[(0, "identity-too-large", 1)]);
    let error = retain(
        &model,
        Limits::default(),
        IdentityLimits::default().with_input_bytes(2),
        None,
        || false,
    )
    .unwrap_err();
    assert!(matches!(error, quire_canonical::Error::Limit(_)));
    let invalid = Model::new(&[usize::MAX], &[]);
    assert!(retain(
        &invalid,
        Limits::default(),
        IdentityLimits::default(),
        None,
        || false
    )
    .is_err());
}

/// An independent transitive-closure oracle covers every three-node directed
/// graph, including edges into a previously finished component.
#[test]
fn component_partition_agrees_with_mutual_reachability_on_all_three_node_graphs() {
    for mask in 0u16..512 {
        let mut model = Model::new(&[0, 1, 2], &[]);
        let mut reachable = [[false; 3]; 3];
        for (source, row) in reachable.iter_mut().enumerate() {
            for (target, cell) in row.iter_mut().enumerate() {
                *cell = mask & (1 << (source * 3 + target)) != 0;
                if *cell {
                    model
                        .edges
                        .entry(source)
                        .or_default()
                        .push(("edge".to_owned(), target));
                }
            }
            row[source] = true;
        }
        for via in 0..3 {
            for source in 0..3 {
                for target in 0..3 {
                    reachable[source][target] |= reachable[source][via] && reachable[via][target];
                }
            }
        }
        let graph = run(&model);
        let components = graph.components();
        for (source, row) in reachable.iter().enumerate() {
            for (target, &forward) in row.iter().enumerate() {
                let same_component = components.iter().any(|component| {
                    component
                        .states
                        .iter()
                        .any(|id| graph.state(*id).unwrap().state == source)
                        && component
                            .states
                            .iter()
                            .any(|id| graph.state(*id).unwrap().state == target)
                });
                assert_eq!(
                    same_component,
                    forward && reachable[target][source],
                    "mask={mask}, {source}->{target}"
                );
            }
        }
        for component in components {
            let source = graph.state(component.states[0]).unwrap().state;
            assert_eq!(
                component.cyclic,
                component.states.len() > 1 || mask & (1 << (source * 3 + source)) != 0,
                "mask={mask}"
            );
        }
        assert_eq!(
            graph.edges().len(),
            usize::try_from(mask.count_ones()).unwrap()
        );
    }
}

/// A property adapter tracks the actual step anchor as its own state.
struct AnchoredProperty;

impl PropertyComponent<Model> for AnchoredProperty {
    type State = String;
    type Key = String;
    type Finding = ();
    fn initial(&self, _model: &usize) -> Vec<String> {
        vec!["initialization".to_owned()]
    }
    fn key(&self, state: &String) -> String {
        state.clone()
    }
    fn advance(
        &self,
        _state: &String,
        _source: &usize,
        transition: &ProductTransition<String>,
        _target: &usize,
    ) -> Result<Vec<String>, ExpansionStop> {
        Ok(vec![match transition {
            ProductTransition::Model(identity) => identity.clone(),
            ProductTransition::TerminalStutter => "stutter".to_owned(),
        }])
    }
    fn findings(&self, _state: &String, _model: &usize) -> Result<Vec<()>, ExpansionStop> {
        Ok(vec![])
    }
    fn close(&self, _state: &String, _model: &usize) -> Result<Vec<()>, ExpansionStop> {
        Ok(vec![])
    }
}

#[test]
fn model_step_anchors_reach_the_property_component_even_at_the_same_model_state() {
    let model = Model::new(&[0], &[(0, "b", 0), (0, "a", 0)]);
    let property = AnchoredProperty;
    let graph = retain(
        &Product::infinite(&model, &property),
        Limits::default(),
        IdentityLimits::default(),
        None,
        || false,
    )
    .unwrap()
    .graph;
    assert_eq!(
        graph
            .states()
            .iter()
            .map(|state| (state.state.model, state.state.property.as_str()))
            .collect::<Vec<_>>(),
        [(0, "initialization"), (0, "a"), (0, "b")]
    );
    assert_eq!(graph.edges().len(), 6);
    for edge in graph.edges() {
        let ProductTransition::Model(identity) = &edge.transition else {
            panic!("a model operation, not stutter");
        };
        assert_eq!(&graph.state(edge.target).unwrap().state.property, identity);
    }
}
