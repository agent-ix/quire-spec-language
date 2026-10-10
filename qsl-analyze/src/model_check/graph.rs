// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-018 EN-1: canonical breadth-first discovery retaining all explored edges.

use std::collections::{BTreeMap, VecDeque};

use qsl_eval::simulation::{Limit, Limits, Outcome, Stats, TransitionSystem};
use qsl_foundation::digest::{DigestDomain, DigestRecord};
use qsl_foundation::{ByteDigest, CatalogCode, IdentityLimits};
use quire_canonical::Encode;

/// A state handle in one retained graph, assigned in discovery order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ProductStateId(pub(super) usize);

impl ProductStateId {
    /// Zero-based discovery index; meaningful only within its graph.
    pub const fn index(self) -> usize {
        self.0
    }
}

/// The unreduced edge mapping: it leaves any concrete state unchanged.
///
/// This is not the later symmetry permutation contract.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IdentityPermutation;

impl IdentityPermutation {
    /// Apply the identity mapping to the original concrete successor.
    pub fn apply<S>(self, state: S) -> S {
        state
    }
}

/// Every explored edge, including incoming, back, self and parallel edges.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedEdge<T> {
    /// Source state handle.
    pub source: ProductStateId,
    /// Actual transition identity, never a formatted display surrogate.
    pub transition: T,
    /// Coalesced target state handle.
    pub target: ProductStateId,
    /// Identity mapping, since this graph performs no symmetry reduction.
    pub permutation: IdentityPermutation,
}

/// A discovered state with its original payload and canonical identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedState<S, F> {
    /// Original state; a product payload keeps both components.
    pub state: S,
    /// Full RFC 8785 bytes used for coalescing, not merely a digest.
    pub key: Vec<u8>,
    /// FR-101 state-key digest.
    pub digest: DigestRecord,
    /// Shortest breadth-first depth.
    pub depth: usize,
    /// Findings, only when every successor was processed.
    pub findings: Vec<F>,
    /// Whether all successors were processed; false on a partial expansion.
    pub expanded: bool,
}

/// An immutable retained graph, possibly partial when exploration stopped.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedGraph<S, T, F> {
    pub(super) states: Vec<RetainedState<S, F>>,
    pub(super) edges: Vec<RetainedEdge<T>>,
    pub(super) outgoing: Vec<Vec<usize>>,
    initial: Vec<ProductStateId>,
}

impl<S, T, F> RetainedGraph<S, T, F> {
    /// States in canonical breadth-first discovery order.
    pub fn states(&self) -> &[RetainedState<S, F>] {
        &self.states
    }
    /// Edges in exploration order, including repeated edges.
    pub fn edges(&self) -> &[RetainedEdge<T>] {
        &self.edges
    }
    /// Distinct admitted initial states, in canonical key order.
    pub fn initial(&self) -> &[ProductStateId] {
        &self.initial
    }
    /// Look up a handle; a handle from another graph may be out of range.
    pub fn state(&self, id: ProductStateId) -> Option<&RetainedState<S, F>> {
        self.states.get(id.0)
    }
    /// Retained outgoing edges of a handle, in canonical successor order.
    pub fn outgoing(&self, id: ProductStateId) -> impl Iterator<Item = &RetainedEdge<T>> {
        self.outgoing
            .get(id.0)
            .into_iter()
            .flatten()
            .filter_map(|index| self.edges.get(*index))
    }

    fn insert(&mut self, state: S, key: Vec<u8>, depth: usize) -> ProductStateId {
        let id = ProductStateId(self.states.len());
        let digest = digest(&key);
        self.states.push(RetainedState {
            state,
            key,
            digest,
            depth,
            findings: Vec::new(),
            expanded: false,
        });
        self.outgoing.push(Vec::new());
        id
    }

    fn stats(&self, transitions: usize) -> Stats {
        Stats {
            states: self.states.len(),
            transitions,
            depth: self
                .states
                .iter()
                .map(|state| state.depth)
                .max()
                .unwrap_or(0),
        }
    }

    fn frontier(
        &self,
        head: ProductStateId,
        queue: &VecDeque<ProductStateId>,
    ) -> Vec<DigestRecord> {
        std::iter::once(head)
            .chain(queue.iter().copied())
            .map(|id| self.states[id.0].digest)
            .collect()
    }
}

/// FR-101 exploration outcome together with the graph it actually retained.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphExploration<S, T, F> {
    /// Retained states and edges; incomplete runs keep their explored prefix.
    pub graph: RetainedGraph<S, T, F>,
    /// Exploration status, not a temporal verdict or certificate.
    pub outcome: Outcome,
}

type RetainResult<S> = Result<
    GraphExploration<
        <S as TransitionSystem>::State,
        <S as TransitionSystem>::TransitionId,
        <S as TransitionSystem>::Finding,
    >,
    quire_canonical::Error,
>;

fn encode(
    value: &impl Encode,
    identity: IdentityLimits,
) -> Result<Vec<u8>, quire_canonical::Error> {
    quire_canonical::to_vec(value, quire_canonical::Limits::new(identity.input_bytes))
}

fn digest(key: &[u8]) -> DigestRecord {
    DigestRecord::mint(
        DigestDomain::SimulationStateKeyV1,
        ByteDigest::of(key).as_bytes(),
    )
}

/// Retain the reachable graph in FR-101's canonical breadth-first order.
///
/// This low-level graph primitive accepts an already selected system; domain
/// admission and temporal settlement belong to its caller. Limits are inclusive.
/// A depth horizon leaves states at that depth unexpanded. Cancellation polls
/// before each expansion. Coalescing full keys never discards an explored edge.
/// `Stats::transitions` counts examined transitions, including the transition
/// whose new target could not be admitted at the state cap; that edge is not
/// retained because it has no admitted target. Findings require a full expansion.
///
/// # Errors
/// The shared encoder's typed error when a state or transition identity has no
/// canonical encoding, exceeds `identity.input_bytes`, or allocation fails.
pub fn retain<S: TransitionSystem>(
    system: &S,
    limits: Limits,
    identity: IdentityLimits,
    max_depth: Option<usize>,
    mut poll: impl FnMut() -> bool,
) -> RetainResult<S> {
    let mut graph = RetainedGraph {
        states: Vec::new(),
        edges: Vec::new(),
        outgoing: Vec::new(),
        initial: Vec::new(),
    };
    let mut initial = system
        .initial()
        .into_iter()
        .map(|state| encode(&system.key(&state), identity).map(|key| (key, state)))
        .collect::<Result<Vec<_>, _>>()?;
    initial.sort_by(|(a, _), (b, _)| a.cmp(b));
    initial.dedup_by(|(a, _), (b, _)| a == b);
    let initial_frontier = initial.iter().map(|(key, _)| digest(key)).collect();
    let mut known = BTreeMap::new();
    let too_many_initial = initial.len() > limits.max_states;
    for (key, state) in initial.into_iter().take(limits.max_states) {
        let id = graph.insert(state, key.clone(), 0);
        known.insert(key, id);
        graph.initial.push(id);
    }
    if too_many_initial {
        let outcome = Outcome::Bounded {
            stats: graph.stats(0),
            frontier: initial_frontier,
            limit: Limit::States(limits.max_states),
        };
        return Ok(GraphExploration { graph, outcome });
    }
    let mut queue: VecDeque<_> = graph.initial.iter().copied().collect();
    let mut transitions = 0;
    let outcome = 'run: loop {
        let Some(source) = queue.pop_front() else {
            break Outcome::Exhaustive(graph.stats(transitions));
        };
        let depth = graph.states[source.0].depth;
        if let Some(horizon) = max_depth.filter(|horizon| depth >= *horizon) {
            break Outcome::BoundReached {
                stats: graph.stats(transitions),
                depth: horizon,
                frontier: graph.frontier(source, &queue),
            };
        }
        if poll() {
            break Outcome::Cancelled {
                stats: graph.stats(transitions),
                frontier: graph.frontier(source, &queue),
                cause: CatalogCode::new("cancelled", "caller-cancelled"),
            };
        }
        let expansion = match system.successors(&graph.states[source.0].state) {
            Ok(expansion) => expansion,
            Err(stop) => {
                break Outcome::Stopped {
                    stats: graph.stats(transitions),
                    frontier: graph.frontier(source, &queue),
                    cause: stop.cause,
                }
            }
        };
        let mut successors = expansion
            .successors
            .into_iter()
            .map(|(transition, state)| {
                let order = encode(&transition, identity)?;
                let key = encode(&system.key(&state), identity)?;
                Ok((order, key, transition, state))
            })
            .collect::<Result<Vec<_>, quire_canonical::Error>>()?;
        successors.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        for (_, key, transition, state) in successors {
            if transitions >= limits.max_transitions {
                break 'run Outcome::Bounded {
                    stats: graph.stats(transitions),
                    frontier: graph.frontier(source, &queue),
                    limit: Limit::Transitions(limits.max_transitions),
                };
            }
            transitions += 1;
            let target = if let Some(id) = known.get(&key) {
                *id
            } else {
                if graph.states.len() >= limits.max_states {
                    let mut frontier = graph.frontier(source, &queue);
                    frontier.push(digest(&key));
                    break 'run Outcome::Bounded {
                        stats: graph.stats(transitions),
                        frontier,
                        limit: Limit::States(limits.max_states),
                    };
                }
                // A new state always requires an edge from an existing depth,
                // so depth is bounded by the number of allocated state records.
                let id = graph.insert(state, key.clone(), depth.saturating_add(1));
                known.insert(key, id);
                queue.push_back(id);
                id
            };
            graph.outgoing[source.0].push(graph.edges.len());
            graph.edges.push(RetainedEdge {
                source,
                transition,
                target,
                permutation: IdentityPermutation,
            });
        }
        graph.states[source.0].findings = expansion.findings;
        graph.states[source.0].expanded = true;
    };
    Ok(GraphExploration { graph, outcome })
}
