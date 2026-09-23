// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic breadth-first exploration of a caller-supplied transition
//! system.

use std::collections::{HashSet, VecDeque};

use crate::simulation::frontier::{Frontier, Limit, StateKey};

/// A finite-branching transition system the engine explores without
/// interpreting.
///
/// An implementation owns the encoding of its own state key; the engine
/// never inspects state beyond calling `key` and `successors`.
pub trait TransitionSystem {
    /// One point in the system's state space.
    type State;
    /// The identity of one authored transition, stable across states.
    type TransitionId: Clone + Eq + std::fmt::Debug;

    /// Every state the system starts from, in the order enumeration should
    /// consider them.
    fn initial(&self) -> Vec<Self::State>;

    /// The full canonical state key. Two states are the same state to the
    /// engine exactly when their keys are equal.
    fn key(&self, state: &Self::State) -> StateKey;

    /// The transitions enabled from `state`, in the order the system wants
    /// them explored. The engine preserves this order exactly.
    fn successors(&self, state: &Self::State) -> Vec<(Self::TransitionId, Self::State)>;
}

/// Caller-supplied ceilings on one exploration run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    /// Greatest number of distinct discovered states, by key equality.
    pub max_states: usize,
    /// States at depth `>= max_depth` are not expanded.
    pub max_depth: usize,
    /// Greatest number of transitions explored, counting every edge the
    /// engine looks at, including ones that coalesce into an already-known
    /// state.
    pub max_transitions: usize,
}

/// Aggregate counts for one exploration run, whatever stopped it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Stats {
    /// Distinct states discovered.
    pub states: usize,
    /// Transitions explored.
    pub transitions: usize,
    /// Deepest state discovered.
    pub depth: usize,
}

/// The result of one exploration run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Outcome {
    /// The frontier went empty: every declared choice and schedule was
    /// visited.
    Exhaustive(Stats),
    /// An explicit bound stopped the run before the frontier went empty.
    Bounded {
        /// Counts at the moment the run stopped.
        stats: Stats,
        /// The unexpanded keys, in the order the run would have expanded
        /// them next.
        frontier: Frontier,
        /// The bound that stopped the run.
        limit: Limit,
    },
    /// The poll callback requested cancellation before the frontier went
    /// empty.
    Cancelled {
        /// Counts at the moment the run stopped.
        stats: Stats,
        /// The unexpanded keys, in the order the run would have expanded
        /// them next.
        frontier: Frontier,
    },
}

/// One queued, not-yet-expanded discovered state.
struct Queued<S> {
    state: S,
    key: StateKey,
    depth: usize,
}

/// Explore `system` breadth-first, level by level, in `system`'s own
/// successor order, coalescing states only when their full key bytes are
/// equal.
///
/// `poll` is called once per state the engine attempts to expand; returning
/// `true` cancels the run before that state is touched.
pub fn explore<S: TransitionSystem>(
    system: &S,
    limits: Limits,
    mut poll: impl FnMut() -> bool,
) -> Outcome {
    let mut visited: HashSet<StateKey> = HashSet::new();
    let mut queue: VecDeque<Queued<S::State>> = VecDeque::new();
    let mut states = 0usize;

    // Admit as many initial states as `max_states` allows, in the system's
    // own order, deduplicating by key. A state the cap refuses still
    // belongs in the frontier — it was discovered, just never queued — so
    // the scan never stops early: it always sees every initial state before
    // deciding whether the run is bounded.
    let mut blocked_initial: Vec<StateKey> = Vec::new();
    for state in system.initial() {
        let key = system.key(&state);
        if visited.contains(&key) {
            continue;
        }
        if states >= limits.max_states {
            visited.insert(key.clone());
            blocked_initial.push(key);
            continue;
        }
        visited.insert(key.clone());
        states += 1;
        queue.push_back(Queued {
            state,
            key,
            depth: 0,
        });
    }
    if !blocked_initial.is_empty() {
        let mut frontier: Frontier = queue.iter().map(|item| item.key.clone()).collect();
        frontier.extend(blocked_initial);
        return Outcome::Bounded {
            stats: Stats {
                states,
                transitions: 0,
                depth: 0,
            },
            frontier,
            limit: Limit::States,
        };
    }

    let mut transitions = 0usize;
    let mut depth_reached = 0usize;

    let frontier_of = |head: Vec<StateKey>, queue: &VecDeque<Queued<S::State>>| -> Frontier {
        let mut frontier = head;
        frontier.extend(queue.iter().map(|item| item.key.clone()));
        frontier
    };

    while let Some(Queued { state, key, depth }) = queue.pop_front() {
        if poll() {
            let frontier = frontier_of(vec![key], &queue);
            return Outcome::Cancelled {
                stats: Stats {
                    states,
                    transitions,
                    depth: depth_reached,
                },
                frontier,
            };
        }
        if depth >= limits.max_depth {
            let frontier = frontier_of(vec![key], &queue);
            return Outcome::Bounded {
                stats: Stats {
                    states,
                    transitions,
                    depth: depth_reached,
                },
                frontier,
                limit: Limit::Depth,
            };
        }
        for (_, successor) in system.successors(&state) {
            if transitions >= limits.max_transitions {
                let frontier = frontier_of(vec![key.clone()], &queue);
                return Outcome::Bounded {
                    stats: Stats {
                        states,
                        transitions,
                        depth: depth_reached,
                    },
                    frontier,
                    limit: Limit::Transitions,
                };
            }
            transitions += 1;
            let successor_key = system.key(&successor);
            if visited.contains(&successor_key) {
                continue;
            }
            if states >= limits.max_states {
                // `key` (still unexpanded) comes first, then whatever was
                // already queued ahead of it, then this successor — it was
                // the last of the three to be discovered.
                let mut frontier = frontier_of(vec![key.clone()], &queue);
                frontier.push(successor_key);
                return Outcome::Bounded {
                    stats: Stats {
                        states,
                        transitions,
                        depth: depth_reached,
                    },
                    frontier,
                    limit: Limit::States,
                };
            }
            visited.insert(successor_key.clone());
            states += 1;
            let successor_depth = depth + 1;
            depth_reached = depth_reached.max(successor_depth);
            queue.push_back(Queued {
                state: successor,
                key: successor_key,
                depth: successor_depth,
            });
        }
    }

    Outcome::Exhaustive(Stats {
        states,
        transitions,
        depth: depth_reached,
    })
}
