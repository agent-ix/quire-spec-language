// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-210: finite simulation reproducibility (FR-181).
//!
//! `Graph` is a toy `TransitionSystem` over small integer nodes: its key
//! wraps the node's big-endian `u32` bytes in `StateKey`, and its
//! transitions are declared as an explicit, ordered edge list the graph
//! filters by source node. Nothing here is sorted; successor order is
//! exactly the order edges were authored, which is what the engine
//! promises to preserve.

use ix_trace_rs::trace;
use qsl_eval::simulation::{
    explore, replay, sample, CounterSampler, EmptyInitial, Limit, Limits, Outcome, ReplayError,
    Sampler, StateKey, Stats, Step, StopReason, Trace, TransitionSystem,
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

    fn key(&self, state: &Node) -> StateKey {
        StateKey::new(state.0.to_be_bytes().to_vec())
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
fn key(n: u32) -> StateKey {
    StateKey::new(n.to_be_bytes().to_vec())
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
        Outcome::Exhaustive(Stats {
            states: 5,
            transitions: 4,
            depth: 2,
        })
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
            stats: Stats {
                states: 3,
                transitions: 2,
                depth: 1,
            },
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
        Outcome::Exhaustive(Stats {
            states: 4,
            transitions: 4,
            depth: 2,
        })
    );
}

/// A linear chain 0 -> 1 -> 2, bounded on depth. `max_depth: 2` stops one
/// state short; `max_depth: 3` completes it. This slice does not back
/// FR-181-AC-3 (the requires-bound/incomplete disposition mapping is a
/// later slice), so this test carries no AC tag.
#[trace("TC-210")]
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
            stats: Stats {
                states: 3,
                transitions: 2,
                depth: 2,
            },
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
        Outcome::Exhaustive(Stats {
            states: 3,
            transitions: 2,
            depth: 2,
        })
    );
}

/// The same chain, bounded on distinct states. `max_states: 2` stops one
/// state short; `max_states: 3` completes it. No FR-181-AC-3 tag: see the
/// depth-limit test above.
#[trace("TC-210")]
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
            stats: Stats {
                states: 2,
                transitions: 2,
                depth: 1,
            },
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
        Outcome::Exhaustive(Stats {
            states: 3,
            transitions: 2,
            depth: 2,
        })
    );
}

/// The same chain, bounded on explored transitions. `max_transitions: 1`
/// stops one edge short; `max_transitions: 2` completes it. No
/// FR-181-AC-3 tag: see the depth-limit test above.
#[trace("TC-210")]
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
            stats: Stats {
                states: 2,
                transitions: 1,
                depth: 1,
            },
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
        Outcome::Exhaustive(Stats {
            states: 3,
            transitions: 2,
            depth: 2,
        })
    );
}

/// When the state cap trips on a successor while the queue is non-empty,
/// the blocked key belongs after the queue, not spliced in front of it: 0
/// branches to 1 and 2, then 1 leads to 3; `max_states: 3` must stop with
/// frontier `[1, 2, 3]`, not `[1, 3, 2]`.
#[trace("TC-210")]
#[test]
fn state_limit_mid_successors_places_the_blocked_key_after_the_queue() {
    let system = Graph::new(vec![0], vec![(0, "a", 1), (0, "b", 2), (1, "c", 3)]);
    let outcome = explore(
        &system,
        Limits {
            max_states: 3,
            ..generous_limits()
        },
        never_cancels,
    );
    assert_eq!(
        outcome,
        Outcome::Bounded {
            stats: Stats {
                states: 3,
                transitions: 3,
                depth: 1,
            },
            frontier: vec![key(1), key(2), key(3)],
            limit: Limit::States,
        }
    );
}

/// Several distinct initial states are all admitted, in order, when the
/// state cap allows it.
#[trace("TC-210")]
#[test]
fn several_initial_states_are_all_admitted_when_the_cap_allows() {
    let system = Graph::new(vec![0, 1, 2], vec![]);
    let outcome = explore(&system, generous_limits(), never_cancels);
    assert_eq!(
        outcome,
        Outcome::Exhaustive(Stats {
            states: 3,
            transitions: 0,
            depth: 0,
        })
    );
}

/// When the state cap trips during initial seeding, the frontier is the
/// admitted initials, then the refused one, then the rest — every initial
/// state the scan reached, none dropped.
#[trace("TC-210")]
#[test]
fn state_limit_on_initial_states_orders_admitted_then_blocked_then_remaining() {
    let system = Graph::new(vec![0, 1, 2], vec![]);
    let outcome = explore(
        &system,
        Limits {
            max_states: 1,
            ..generous_limits()
        },
        never_cancels,
    );
    assert_eq!(
        outcome,
        Outcome::Bounded {
            stats: Stats {
                states: 1,
                transitions: 0,
                depth: 0,
            },
            frontier: vec![key(0), key(1), key(2)],
            limit: Limit::States,
        }
    );
}

/// A duplicate initial state coalesces with its first occurrence, exactly
/// like a duplicate discovered by traversal.
#[trace("TC-210")]
#[test]
fn duplicate_initial_states_coalesce_to_one_state() {
    let system = Graph::new(vec![0, 0, 1], vec![]);
    let outcome = explore(&system, generous_limits(), never_cancels);
    assert_eq!(
        outcome,
        Outcome::Exhaustive(Stats {
            states: 2,
            transitions: 0,
            depth: 0,
        })
    );
}

/// `max_states: 0` admits nothing at all; every initial state lands in the
/// frontier, in order.
#[trace("TC-210")]
#[test]
fn max_states_zero_admits_no_initial_state() {
    let system = Graph::new(vec![0, 1], vec![]);
    let outcome = explore(
        &system,
        Limits {
            max_states: 0,
            ..generous_limits()
        },
        never_cancels,
    );
    assert_eq!(
        outcome,
        Outcome::Bounded {
            stats: Stats {
                states: 0,
                transitions: 0,
                depth: 0,
            },
            frontier: vec![key(0), key(1)],
            limit: Limit::States,
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
            stats: Stats {
                states: 3,
                transitions: 2,
                depth: 1,
            },
            frontier: vec![key(1), key(2)],
        }
    );
}

/// FR-181-AC-2: the same seed, driven through a fresh sampler, reproduces
/// an identical trace end to end, including its recorded provenance. Both
/// branches of this graph dead-end after one more step, so a generous
/// `max_steps` still stops on `NoSuccessors`, not the step limit.
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
    let trace_a = sample(&system, 5, &mut first).expect("system has an initial state");
    let trace_b = sample(&system, 5, &mut second).expect("system has an initial state");
    assert_eq!(trace_a, trace_b);
    assert_eq!(trace_a.initial, key(0));
    assert_eq!(trace_a.steps.len(), 2);
    let provenance = trace_a.provenance.as_ref().expect("sampled trace");
    assert_eq!(provenance.seed, 42);
    assert_eq!(
        provenance.sampler_version,
        "quire.simulation.counter-sampler/1"
    );
    assert_eq!(provenance.stopped, StopReason::NoSuccessors);
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
        let run = sample(&system, 2, &mut sampler).expect("system has an initial state");
        first_steps.insert(run.steps[0].key.clone());
    }
    assert!(
        first_steps.len() > 1,
        "20 seeds across a 2-way branch produced only one first step"
    );
}

/// `max_steps` shorter than the available path stops the run on the step
/// limit, not on running out of successors: the chain has a second edge
/// this run never gets to draw.
#[trace("TC-210")]
#[test]
fn max_steps_shorter_than_the_path_stops_on_the_step_limit() {
    let system = Graph::new(vec![0], vec![(0, "t1", 1), (1, "t2", 2)]);
    let mut sampler = CounterSampler::new(0);
    let trace = sample(&system, 1, &mut sampler).expect("system has an initial state");
    assert_eq!(trace.steps.len(), 1);
    assert_eq!(trace.steps[0].key, key(1));
    let provenance = trace.provenance.as_ref().expect("sampled trace");
    assert_eq!(provenance.stopped, StopReason::StepLimit);
}

/// `sample` cannot draw from a system with no initial states; it reports
/// that as an error, not a panic.
#[trace("TC-210")]
#[test]
fn sample_on_a_system_with_no_initial_states_returns_an_error() {
    let system = Graph::new(vec![], vec![]);
    let mut sampler = CounterSampler::new(0);
    assert_eq!(sample(&system, 3, &mut sampler), Err(EmptyInitial));
}

/// A seeded run through a state with two same-id, different-target
/// transitions still replays: replay must pick the successor whose key
/// matches the recorded step, not merely the first one with a matching
/// transition id.
#[trace("TC-210")]
#[test]
fn sample_then_replay_round_trips_with_duplicate_transition_ids() {
    let system = Graph::new(vec![0], vec![(0, "recv", 1), (0, "recv", 2)]);
    let mut sampler = CounterSampler::new(0);
    let sampled = sample(&system, 1, &mut sampler).expect("system has an initial state");
    // Pinned by the seed: this draw selects the second "recv" edge, to
    // 2, not the first, to 1.
    assert_eq!(sampled.steps[0].transition, "recv");
    assert_eq!(sampled.steps[0].key, key(2));
    assert_eq!(replay(&system, &sampled), Ok(()));
}

/// Sampling from several initial states picks the one the sampler's draw
/// selects, and records that state's key as the trace's start — not
/// necessarily the first one the system declares.
#[trace("TC-210")]
#[test]
fn multi_initial_sample_picks_the_sampler_selected_start() {
    let system = Graph::new(vec![10, 20, 30], vec![]);
    let mut sampler = CounterSampler::new(1);
    let trace = sample(&system, 3, &mut sampler).expect("system has an initial state");
    // Pinned by the seed: this draw selects the second declared initial
    // state, 20, not the first, 10.
    assert_eq!(trace.initial, key(20));
    assert_eq!(trace.steps.len(), 0);
    let provenance = trace.provenance.as_ref().expect("sampled trace");
    assert_eq!(provenance.stopped, StopReason::NoSuccessors);
}

/// Replay must locate the initial state whose key matches the trace, not
/// simply take whichever initial state the system lists first.
#[trace("TC-210")]
#[test]
fn multi_initial_replay_matches_the_recorded_initial_key() {
    let system = Graph::new(vec![10, 20], vec![(10, "from10", 99), (20, "from20", 99)]);
    let trace: Trace<&'static str> = Trace {
        initial: key(20),
        steps: vec![Step {
            transition: "from20",
            key: key(99),
        }],
        provenance: None,
    };
    assert_eq!(replay(&system, &trace), Ok(()));
}

/// A trace whose recorded initial key matches no state the system offers
/// is refused with `UnknownInitial`, not a panic or a silent wrong start.
#[trace("TC-210")]
#[test]
fn replay_refuses_a_trace_whose_initial_key_matches_no_state() {
    let system = Graph::new(vec![0], vec![]);
    let trace: Trace<&'static str> = Trace {
        initial: key(999),
        steps: vec![],
        provenance: None,
    };
    assert_eq!(
        replay(&system, &trace),
        Err(ReplayError::UnknownInitial { expected: key(999) })
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

/// `CounterSampler` is deterministic: at a fixed seed, its draw sequence is
/// pinned. This is a regression guard on the mixer and the counter advance,
/// not a claim about which indices are "right".
#[trace("TC-210")]
#[test]
fn counter_sampler_produces_a_pinned_index_sequence() {
    let mut sampler = CounterSampler::new(7);
    let indices: Vec<usize> = (0..4).map(|_| sampler.next_index(5)).collect();
    assert_eq!(indices, vec![4, 2, 4, 1]);
}

/// TC-439 (ADR-014 §7, §10 scenario 4): an exhaustive run is O-16 success;
/// a run a bound stops or the caller cancels is incomplete, and keeps its
/// frontier and the bound that stopped it.
#[trace("TC-439", "FR-097-AC-5")]
#[test]
fn tc_439_explore_outcomes_map_to_their_o16_category() {
    use qsl_foundation::diagnostic::Category;
    // 0 -a-> 1 -b-> 2
    let system = Graph::new(vec![0], vec![(0, "a", 1), (1, "b", 2)]);
    let exhaustive = explore(&system, generous_limits(), never_cancels);
    assert!(matches!(exhaustive, Outcome::Exhaustive(_)));
    assert_eq!(exhaustive.category(), Category::Success);

    let bounded = explore(
        &system,
        Limits {
            max_depth: 1,
            ..generous_limits()
        },
        never_cancels,
    );
    let Outcome::Bounded {
        ref frontier,
        limit,
        ..
    } = bounded
    else {
        panic!("expected Bounded, got {bounded:?}");
    };
    assert_eq!(limit, Limit::Depth);
    assert_eq!(frontier, &vec![key(1)]);
    assert_eq!(bounded.category(), Category::Incomplete);

    let cancelled = explore(&system, generous_limits(), || true);
    let Outcome::Cancelled { ref frontier, .. } = cancelled else {
        panic!("expected Cancelled, got {cancelled:?}");
    };
    assert_eq!(frontier, &vec![key(0)]);
    assert_eq!(cancelled.category(), Category::Incomplete);
}
