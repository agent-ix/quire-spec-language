// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-210: finite simulation reproducibility (FR-181).
//!
//! `Graph` is a toy `TransitionSystem` over small integer nodes: its key is
//! the node's big-endian `u32` bytes, and its transitions are declared as
//! an explicit, ordered edge list the graph filters by source node. Nothing
//! here is sorted; successor order is exactly the order edges were
//! authored, which is what the engine promises to preserve.

use ix_trace_rs::trace;
use quire_spec_language::simulation::{
    explore, replay, sample, CounterSampler, Limit, Limits, Outcome, ReplayError, Step, Trace,
    TransitionSystem,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Node(u32);

struct Graph {
    initial: Vec<u32>,
    edges: Vec<(u32, &'static str, u32)>,
}

impl Graph {
    fn new(initial: Vec<u32>, edges: Vec<(u32, &'static str, u32)>) -> Self {
        Self { initial, edges }
    }
}

impl TransitionSystem for Graph {
    type State = Node;
    type TransitionId = &'static str;

    fn initial(&self) -> Vec<Node> {
        self.initial.iter().copied().map(Node).collect()
    }

    fn key(&self, state: &Node) -> Vec<u8> {
        state.0.to_be_bytes().to_vec()
    }

    fn successors(&self, state: &Node) -> Vec<(&'static str, Node)> {
        self.edges
            .iter()
            .filter(|(from, _, _)| *from == state.0)
            .map(|&(_, transition, to)| (transition, Node(to)))
            .collect()
    }
}

/// The canonical key `Graph` computes for node `n`.
fn key(n: u32) -> Vec<u8> {
    n.to_be_bytes().to_vec()
}

fn generous_limits() -> Limits {
    Limits {
        max_states: usize::MAX,
        max_depth: usize::MAX,
        max_transitions: usize::MAX,
    }
}

fn never_cancels() -> bool {
    false
}

/// FR-181-AC-1: a small branching graph with no coalescing, so every node
/// is its own state and every edge is its own transition.
#[trace("TC-210", "FR-181-AC-1")]
#[test]
fn exhaustive_small_graph_reports_exact_state_and_transition_counts() {
    // 0 -a-> 1 -c-> 3
    // 0 -b-> 2 -d-> 4
    let system = Graph::new(
        vec![0],
        vec![(0, "a", 1), (0, "b", 2), (1, "c", 3), (2, "d", 4)],
    );
    let outcome = explore(&system, generous_limits(), never_cancels);
    assert_eq!(
        outcome,
        Outcome::Exhaustive {
            states: 5,
            transitions: 4,
            depth: 2,
        }
    );
}

/// FR-181-AC-1: successors are visited in the graph's own authored order,
/// not sorted by key. Node 0 declares `9` before `3`; a depth-1 bound stops
/// before either is expanded, and the frontier preserves discovery order.
#[trace("TC-210", "FR-181-AC-1")]
#[test]
fn canonical_order_is_the_systems_authored_successor_order() {
    let system = Graph::new(vec![0], vec![(0, "z", 9), (0, "a", 3)]);
    let outcome = explore(
        &system,
        Limits {
            max_depth: 1,
            ..generous_limits()
        },
        never_cancels,
    );
    assert_eq!(
        outcome,
        Outcome::Bounded {
            states: 3,
            transitions: 2,
            depth: 1,
            frontier: vec![key(9), key(3)],
            limit: Limit::Depth,
        }
    );
}

/// FR-181-AC-4: two distinct paths from the root reach the same key. The
/// engine must coalesce them into one state, not two.
#[trace("TC-210", "FR-181-AC-4")]
#[test]
fn key_equal_coalescing_merges_two_paths_to_the_same_state() {
    // 0 -left-> 1 -merge-> 3
    // 0 -right-> 2 -merge-> 3
    let system = Graph::new(
        vec![0],
        vec![
            (0, "left", 1),
            (0, "right", 2),
            (1, "merge", 3),
            (2, "merge", 3),
        ],
    );
    let outcome = explore(&system, generous_limits(), never_cancels);
    // 4 states, not 5: node 3 is discovered once even though two edges lead
    // to it. All 4 edges are still explored, so transitions stays at 4.
    assert_eq!(
        outcome,
        Outcome::Exhaustive {
            states: 4,
            transitions: 4,
            depth: 2,
        }
    );
}

/// FR-181-AC-3: a linear chain 0 -> 1 -> 2, bounded on depth. `max_depth: 2`
/// stops one state short; `max_depth: 3` completes it.
#[trace("TC-210", "FR-181-AC-3")]
#[test]
fn depth_limit_n_stops_bounded_and_n_plus_one_is_exhaustive() {
    let system = Graph::new(vec![0], vec![(0, "t1", 1), (1, "t2", 2)]);

    let stopped = explore(
        &system,
        Limits {
            max_depth: 2,
            ..generous_limits()
        },
        never_cancels,
    );
    assert_eq!(
        stopped,
        Outcome::Bounded {
            states: 3,
            transitions: 2,
            depth: 2,
            frontier: vec![key(2)],
            limit: Limit::Depth,
        }
    );

    let completed = explore(
        &system,
        Limits {
            max_depth: 3,
            ..generous_limits()
        },
        never_cancels,
    );
    assert_eq!(
        completed,
        Outcome::Exhaustive {
            states: 3,
            transitions: 2,
            depth: 2,
        }
    );
}

/// FR-181-AC-3: the same chain, bounded on distinct states. `max_states: 2`
/// stops one state short; `max_states: 3` completes it.
#[trace("TC-210", "FR-181-AC-3")]
#[test]
fn state_limit_n_stops_bounded_and_n_plus_one_is_exhaustive() {
    let system = Graph::new(vec![0], vec![(0, "t1", 1), (1, "t2", 2)]);

    let stopped = explore(
        &system,
        Limits {
            max_states: 2,
            ..generous_limits()
        },
        never_cancels,
    );
    assert_eq!(
        stopped,
        Outcome::Bounded {
            states: 2,
            transitions: 2,
            depth: 1,
            frontier: vec![key(1), key(2)],
            limit: Limit::States,
        }
    );

    let completed = explore(
        &system,
        Limits {
            max_states: 3,
            ..generous_limits()
        },
        never_cancels,
    );
    assert_eq!(
        completed,
        Outcome::Exhaustive {
            states: 3,
            transitions: 2,
            depth: 2,
        }
    );
}

/// FR-181-AC-3: the same chain, bounded on explored transitions.
/// `max_transitions: 1` stops one edge short; `max_transitions: 2` completes
/// it.
#[trace("TC-210", "FR-181-AC-3")]
#[test]
fn transition_limit_n_stops_bounded_and_n_plus_one_is_exhaustive() {
    let system = Graph::new(vec![0], vec![(0, "t1", 1), (1, "t2", 2)]);

    let stopped = explore(
        &system,
        Limits {
            max_transitions: 1,
            ..generous_limits()
        },
        never_cancels,
    );
    assert_eq!(
        stopped,
        Outcome::Bounded {
            states: 2,
            transitions: 1,
            depth: 1,
            frontier: vec![key(1)],
            limit: Limit::Transitions,
        }
    );

    let completed = explore(
        &system,
        Limits {
            max_transitions: 2,
            ..generous_limits()
        },
        never_cancels,
    );
    assert_eq!(
        completed,
        Outcome::Exhaustive {
            states: 3,
            transitions: 2,
            depth: 2,
        }
    );
}

/// A run cancelled mid-expansion returns `Cancelled` with the exact
/// frontier, never `Exhaustive` or `Bounded`.
#[trace("TC-210")]
#[test]
fn cancellation_stops_the_run_and_returns_the_frontier() {
    // 0 -a-> 1 -c-> 3
    // 0 -b-> 2 -d-> 4
    let system = Graph::new(
        vec![0],
        vec![(0, "a", 1), (0, "b", 2), (1, "c", 3), (2, "d", 4)],
    );
    let mut polls = 0usize;
    let outcome = explore(&system, generous_limits(), || {
        polls += 1;
        // Let state 0 expand; cancel just before state 1 would expand.
        polls == 2
    });
    assert_eq!(
        outcome,
        Outcome::Cancelled {
            states: 3,
            transitions: 2,
            depth: 1,
            frontier: vec![key(1), key(2)],
        }
    );
}

/// FR-181-AC-2: the same seed, driven through a fresh sampler, reproduces
/// an identical trace end to end, including its recorded provenance.
#[trace("TC-210", "FR-181-AC-2")]
#[test]
fn seeded_sampling_reproduces_the_same_trace() {
    // 0 -a-> 1 -c-> 3
    // 0 -b-> 2 -d-> 4
    let system = Graph::new(
        vec![0],
        vec![(0, "a", 1), (0, "b", 2), (1, "c", 3), (2, "d", 4)],
    );
    let mut first = CounterSampler::new(42);
    let mut second = CounterSampler::new(42);
    let trace_a = sample(&system, 42, 2, &mut first);
    let trace_b = sample(&system, 42, 2, &mut second);
    assert_eq!(trace_a, trace_b);
    assert_eq!(trace_a.initial, key(0));
    assert_eq!(trace_a.steps.len(), 2);
    let provenance = trace_a.provenance.as_ref().expect("sampled trace");
    assert_eq!(provenance.seed, 42);
    assert_eq!(
        provenance.sampler_version,
        "quire.simulation.counter-sampler/1"
    );
}

/// FR-181-AC-2: across a spread of seeds, at least two produce different
/// first steps — sampling is not silently ignoring the seed.
#[trace("TC-210", "FR-181-AC-2")]
#[test]
fn different_seeds_can_sample_different_traces() {
    let system = Graph::new(
        vec![0],
        vec![(0, "a", 1), (0, "b", 2), (1, "c", 3), (2, "d", 4)],
    );
    let mut first_steps = std::collections::BTreeSet::new();
    for seed in 0u64..20 {
        let mut sampler = CounterSampler::new(seed);
        let run = sample(&system, seed, 2, &mut sampler);
        first_steps.insert(run.steps[0].key.clone());
    }
    assert!(
        first_steps.len() > 1,
        "20 seeds across a 2-way branch produced only one first step"
    );
}

/// FR-181-AC-5: a trace replays cleanly against the system that produced
/// it, and refuses at the exact step whichever part of it is tampered with.
#[trace("TC-210", "FR-181-AC-5")]
#[test]
fn replay_accepts_a_good_trace_and_refuses_a_tampered_one() {
    let system = Graph::new(vec![0], vec![(0, "t1", 1), (1, "t2", 2)]);
    let good: Trace<&'static str> = Trace {
        initial: key(0),
        steps: vec![
            Step {
                transition: "t1",
                key: key(1),
            },
            Step {
                transition: "t2",
                key: key(2),
            },
        ],
        provenance: None,
    };
    assert_eq!(replay(&system, &good), Ok(()));

    let mut wrong_key = good.clone();
    wrong_key.steps[1].key = key(999);
    assert_eq!(
        replay(&system, &wrong_key),
        Err(ReplayError::KeyMismatch {
            step: 1,
            transition: "t2",
            expected: key(999),
            actual: key(2),
        })
    );

    let mut wrong_transition = good.clone();
    wrong_transition.steps[0].transition = "bogus";
    assert_eq!(
        replay(&system, &wrong_transition),
        Err(ReplayError::MissingTransition {
            step: 0,
            transition: "bogus",
        })
    );
}
