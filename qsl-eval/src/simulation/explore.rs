// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic breadth-first exploration of a caller-supplied transition
//! system, in FR-101's canonical order.

use std::collections::{HashSet, VecDeque};

use qsl_foundation::diagnostic::Category;
use qsl_foundation::digest::{DigestRecord, WireNodeId};
use qsl_foundation::CatalogCode;
use qsl_semantics::value::declaration::TypeEnvironment;
use quire_exact::ValueType;
use serde::Serialize;

use crate::simulation::frontier::{Frontier, Limit};
use crate::simulation::key::{EncodingRefusal, StateKey};
use crate::simulation::not_simulated::{check_requires_bound, NotSimulated};
use crate::simulation::order::{ordered_successors, sorted_initial};

/// A finite-branching transition system the engine explores.
///
/// An implementation owns the typed canonical view of its own state and
/// transition identities (QSpec FR-181's exploration contract); the engine
/// encodes and hashes them itself (FR-101) and never otherwise interprets
/// them.
pub trait TransitionSystem {
    /// One point in the system's state space.
    type State;
    /// The identity of one authored transition, stable across states, and
    /// serializable to its own typed canonical form so the engine can order
    /// it.
    type TransitionId: Clone + Eq + std::fmt::Debug + Serialize;
    /// The typed canonical view of one state, serializable so the engine can
    /// key it (FR-101).
    type Key: Serialize;

    /// Every state the system starts from.
    fn initial(&self) -> Vec<Self::State>;

    /// The typed canonical view of `state`'s full state key. Two states are
    /// the same state to the engine exactly when their encoded key bytes are
    /// equal.
    fn key(&self, state: &Self::State) -> Self::Key;

    /// The transitions enabled from `state`. The engine orders them itself
    /// (FR-101-AC-1); this order is never preserved.
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
        /// The unexpanded state-key digests, in the order the run would
        /// have expanded them next.
        frontier: Frontier,
        /// The bound that stopped the run.
        limit: Limit,
    },
    /// The poll callback requested cancellation before the frontier went
    /// empty.
    Cancelled {
        /// Counts at the moment the run stopped.
        stats: Stats,
        /// The unexpanded state-key digests, in the order the run would
        /// have expanded them next.
        frontier: Frontier,
        /// Always `CatalogCode::new("cancelled", "caller-cancelled")`
        /// (FR-101, ADR-014 TR-7).
        cause: CatalogCode,
    },
}

/// `Outcome::Cancelled`'s one cause (FR-101).
const CANCELLED_CAUSE: CatalogCode = CatalogCode::new("cancelled", "caller-cancelled");

impl Outcome {
    /// The ADR-013 O-16 category of this run (ADR-014 §7): `Exhaustive` is
    /// success; `Bounded` and `Cancelled` are incomplete, each keeping its
    /// frontier. A stopped run never reports success or a verdict (QSpec
    /// FR-181).
    pub fn category(&self) -> Category {
        match self {
            Self::Exhaustive(_) => Category::Success,
            Self::Bounded { .. } | Self::Cancelled { .. } => Category::Incomplete,
        }
    }
}

/// One queued, not-yet-expanded discovered state.
struct Queued<S> {
    state: S,
    digest: DigestRecord,
    depth: usize,
}

/// `head`, then every still-queued state's digest, in queue order.
fn frontier_of<S>(head: DigestRecord, queue: &VecDeque<Queued<S>>) -> Frontier {
    std::iter::once(head)
        .chain(queue.iter().map(|item| item.digest))
        .collect()
}

/// Explore `system` breadth-first, level by level, in FR-101's canonical
/// order, coalescing states only when their full key bytes are equal.
///
/// `poll` is called once per state the engine attempts to expand; returning
/// `true` cancels the run before that state is touched.
///
/// `explore_request` is the public entry that calls this; a caller reaches
/// `explore` only through it, so the requires-bound check always runs first.
///
/// # Errors
///
/// [`EncodingRefusal`] when a state or transition identity reached during
/// the walk has no RFC 8785 encoding.
pub(crate) fn explore<S: TransitionSystem>(
    system: &S,
    limits: Limits,
    mut poll: impl FnMut() -> bool,
) -> Result<Outcome, EncodingRefusal> {
    let initial = sorted_initial(system)?;
    if initial.len() > limits.max_states {
        let frontier: Frontier = initial.iter().map(|item| item.digest).collect();
        return Ok(Outcome::Bounded {
            stats: Stats {
                states: limits.max_states,
                transitions: 0,
                depth: 0,
            },
            frontier,
            limit: Limit::States,
        });
    }

    let mut visited: HashSet<StateKey> = initial.iter().map(|item| item.key.clone()).collect();
    let mut queue: VecDeque<Queued<S::State>> = initial
        .into_iter()
        .map(|item| Queued {
            state: item.state,
            digest: item.digest,
            depth: 0,
        })
        .collect();
    let mut states = queue.len();
    let mut transitions = 0usize;
    let mut depth_reached = 0usize;

    while let Some(Queued {
        state,
        digest,
        depth,
    }) = queue.pop_front()
    {
        if poll() {
            return Ok(Outcome::Cancelled {
                stats: Stats {
                    states,
                    transitions,
                    depth: depth_reached,
                },
                frontier: frontier_of(digest, &queue),
                cause: CANCELLED_CAUSE,
            });
        }
        if depth >= limits.max_depth {
            return Ok(Outcome::Bounded {
                stats: Stats {
                    states,
                    transitions,
                    depth: depth_reached,
                },
                frontier: frontier_of(digest, &queue),
                limit: Limit::Depth,
            });
        }
        for successor in ordered_successors(system, &state)? {
            if transitions >= limits.max_transitions {
                return Ok(Outcome::Bounded {
                    stats: Stats {
                        states,
                        transitions,
                        depth: depth_reached,
                    },
                    frontier: frontier_of(digest, &queue),
                    limit: Limit::Transitions,
                });
            }
            transitions += 1;
            if visited.contains(&successor.key) {
                continue;
            }
            if states >= limits.max_states {
                let mut frontier = frontier_of(digest, &queue);
                frontier.push(successor.digest);
                return Ok(Outcome::Bounded {
                    stats: Stats {
                        states,
                        transitions,
                        depth: depth_reached,
                    },
                    frontier,
                    limit: Limit::States,
                });
            }
            visited.insert(successor.key);
            states += 1;
            let successor_depth = depth + 1;
            depth_reached = depth_reached.max(successor_depth);
            queue.push_back(Queued {
                state: successor.state,
                digest: successor.digest,
                depth: successor_depth,
            });
        }
    }

    Ok(Outcome::Exhaustive(Stats {
        states,
        transitions,
        depth: depth_reached,
    }))
}

/// Explore `system`, first refusing an unbounded `domains` request before any
/// `TransitionSystem` method is called (FR-101).
///
/// `explore` and `Sampler` stay `pub(crate)`; this and `sample_request` are
/// the only entries that explore or sample, so no caller skips the
/// requires-bound check.
///
/// # Errors
///
/// [`NotSimulated::RequiresBound`] when `domains` has an unbounded domain
/// under the ADR-014 §4 extent rule; [`NotSimulated::Extent`] when
/// `classify_extent` itself stops at a stage limit or an internal fault;
/// [`NotSimulated::KeyEncoding`] when a state or transition identity reached
/// during the walk has no RFC 8785 encoding. Never
/// [`NotSimulated::GeneratorMismatch`] or [`NotSimulated::EmptyInitial`].
pub fn explore_request<S: TransitionSystem>(
    system: &S,
    domains: &[(WireNodeId, &ValueType)],
    types: &TypeEnvironment,
    position_limit: u64,
    limits: Limits,
    poll: impl FnMut() -> bool,
) -> Result<Outcome, NotSimulated> {
    check_requires_bound(domains, types, position_limit)?;
    explore(system, limits, poll).map_err(NotSimulated::from)
}
