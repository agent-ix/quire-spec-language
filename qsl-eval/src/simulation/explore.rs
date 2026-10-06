// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic breadth-first exploration of a caller-supplied transition
//! system, in FR-101's canonical order.

use std::collections::{HashSet, VecDeque};

use qsl_foundation::diagnostic::{category_of, Category};
use qsl_foundation::digest::{DigestRecord, WireNodeId};
use qsl_foundation::CatalogCode;
use quire_canonical::Encode;
use quire_exact::ValueType;
use quire_semantic_value::declaration::TypeEnvironment;

use crate::simulation::expansion::{Expansion, ExpansionStop, StateFindings};
use crate::simulation::frontier::{Frontier, Limit};
use crate::simulation::key::{EncodingRefusal, StateKey};
use crate::simulation::not_simulated::{check_requires_bound, NotSimulated};
use crate::simulation::order::{expand, sorted_initial, Expanded};
use qsl_foundation::{IdentityLimits, Setting, SettingLimits};

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
    /// encodable to its own typed canonical form so the engine can order it.
    /// A fixed-shape identity encodes through `quire_canonical::FixedShape`;
    /// one whose depth follows its value writes `quire-canonical`'s events
    /// from an explicit stack (ADR-030 D-4.7).
    type TransitionId: Clone + Eq + std::fmt::Debug + Encode;
    /// The typed canonical view of one state, encodable so the engine can
    /// key it (FR-101), as [`Self::TransitionId`] is.
    type Key: Encode;
    /// What the system finds at an expanded state; the engine records it
    /// and never interprets it (FR-101-AC-13).
    type Finding: Clone + Eq + std::fmt::Debug;

    /// Every state the system starts from.
    fn initial(&self) -> Vec<Self::State>;

    /// The typed canonical view of `state`'s full state key. Two states are
    /// the same state to the engine exactly when their encoded key bytes are
    /// equal.
    fn key(&self, state: &Self::State) -> Self::Key;

    /// Expand `state`: its enabled transitions and its findings, or a stop.
    /// The engine calls this once per expanded state and orders the
    /// successors itself (FR-101-AC-1); the listed order is never
    /// preserved.
    ///
    /// # Errors
    ///
    /// [`ExpansionStop`] when the system cannot expand `state`; the engine
    /// stops the run there (FR-101-AC-12, FR-101-AC-14).
    fn successors(
        &self,
        state: &Self::State,
    ) -> ExpansionResult<Self::TransitionId, Self::State, Self::Finding>;
}

/// What [`TransitionSystem::successors`] returns: an [`Expansion`] or an
/// [`ExpansionStop`].
type ExpansionResult<T, S, F> = Result<Expansion<T, S, F>, ExpansionStop>;

/// Caller-raisable resource limits on one exploration run (ADR-014 B-5,
/// FR-255). The search horizon is `explore_request`'s `max_depth`, never a
/// member of this type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    /// Greatest number of distinct discovered states, by key equality.
    /// Setting `explore.states`.
    pub max_states: usize,
    /// Greatest number of transitions explored, counting every edge the
    /// engine looks at, including ones that coalesce into an already-known
    /// state. Setting `explore.transitions`.
    pub max_transitions: usize,
}

impl Default for Limits {
    /// FR-255's published defaults: `max_states` 10,000,000 and
    /// `max_transitions` 100,000,000.
    fn default() -> Self {
        Self {
            max_states: 10_000_000,
            max_transitions: 100_000_000,
        }
    }
}

impl Limits {
    /// These limits with `explore.states` set to `bound`.
    #[must_use]
    pub const fn with_max_states(mut self, bound: usize) -> Self {
        self.max_states = bound;
        self
    }

    /// These limits with `explore.transitions` set to `bound`.
    #[must_use]
    pub const fn with_max_transitions(mut self, bound: usize) -> Self {
        self.max_transitions = bound;
        self
    }
}

/// FR-255: the one mapping from each field to its setting.
impl SettingLimits for Limits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        let widen = |bound: usize| u64::try_from(bound).unwrap_or(u64::MAX);
        vec![
            (Setting::ExploreStates, widen(self.max_states)),
            (Setting::ExploreTransitions, widen(self.max_transitions)),
        ]
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        let bound = usize::try_from(bound).unwrap_or(usize::MAX);
        match setting {
            Setting::ExploreStates => self.max_states = bound,
            Setting::ExploreTransitions => self.max_transitions = bound,
            _ => return false,
        }
        true
    }
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
    /// The frontier went empty: every reachable state was expanded.
    Exhaustive(Stats),
    /// Every reachable state below the `max_depth` horizon was expanded and
    /// the frontier holds only states at the horizon.
    BoundReached {
        /// Counts at the moment the run stopped.
        stats: Stats,
        /// The horizon the run used.
        depth: usize,
        /// The horizon states, in the order the run would have expanded
        /// them next.
        frontier: Frontier,
    },
    /// A resource limit stopped the run before the frontier went empty.
    Bounded {
        /// Counts at the moment the run stopped.
        stats: Stats,
        /// The unexpanded state-key digests, in the order the run would
        /// have expanded them next.
        frontier: Frontier,
        /// The limit that stopped the run, with its value.
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
    /// The system returned an [`ExpansionStop`] for a state.
    Stopped {
        /// Counts at the moment the run stopped.
        stats: Stats,
        /// The state whose expansion stopped, then the queued states in
        /// next-expansion order.
        frontier: Frontier,
        /// The stop's cause.
        cause: CatalogCode,
    },
}

/// `Outcome::Cancelled`'s one cause (FR-101).
const CANCELLED_CAUSE: CatalogCode = CatalogCode::new("cancelled", "caller-cancelled");

impl Outcome {
    /// The ADR-013 O-16 category of this run (FR-097-AC-5): `Exhaustive` is
    /// success; `BoundReached` is inconclusive; `Bounded` and `Cancelled`
    /// are incomplete; `Stopped` takes its cause's category. A stopped run
    /// never reports success (QSpec FR-181).
    pub fn category(&self) -> Category {
        match self {
            Self::Exhaustive(_) => Category::Success,
            Self::BoundReached { .. } => Category::Inconclusive,
            Self::Bounded { .. } | Self::Cancelled { .. } => Category::Incomplete,
            Self::Stopped { cause, .. } => stop_category(cause),
        }
    }
}

/// The O-16 category of an expansion stop's cause. `resource_exhausted` is
/// an exhausted execution budget, incomplete (ADR-014 B-2); every other code
/// takes the catalog's category, and a code the catalog does not define is
/// an internal failure: the system stopped for a reason the engine cannot
/// classify. The only site that reads a stop cause's code spelling.
#[qsl_attrs::string_edge]
fn stop_category(cause: &CatalogCode) -> Category {
    if cause.code() == "resource_exhausted" {
        return Category::Incomplete;
    }
    category_of(cause).unwrap_or(Category::InternalFailure)
}

/// The result of one exploration run: its outcome and the findings of every
/// expanded state that had any, in expansion order (FR-101-AC-13).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Exploration<F> {
    /// How the run ended.
    pub outcome: Outcome,
    /// One entry per expanded state with at least one finding.
    pub findings: Vec<StateFindings<F>>,
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
/// With `max_depth = Some(k)`, states at depth `>= k` are not expanded.
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
    identity: IdentityLimits,
    max_depth: Option<usize>,
    mut poll: impl FnMut() -> bool,
) -> Result<Exploration<S::Finding>, EncodingRefusal> {
    let mut findings = Vec::new();
    let initial = sorted_initial(system, identity)?;
    if initial.len() > limits.max_states {
        let frontier: Frontier = initial.iter().map(|item| item.digest).collect();
        return Ok(Exploration {
            outcome: Outcome::Bounded {
                stats: Stats {
                    states: limits.max_states,
                    transitions: 0,
                    depth: 0,
                },
                frontier,
                limit: Limit::States(limits.max_states),
            },
            findings,
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

    let outcome = 'run: loop {
        let Some(Queued {
            state,
            digest,
            depth,
        }) = queue.pop_front()
        else {
            break Outcome::Exhaustive(Stats {
                states,
                transitions,
                depth: depth_reached,
            });
        };
        // Breadth-first order queues every state below the horizon before
        // the first state at it, so the first horizon state popped means
        // every state below it is expanded and the queue holds only
        // horizon states.
        if let Some(horizon) = max_depth.filter(|horizon| depth >= *horizon) {
            break Outcome::BoundReached {
                stats: Stats {
                    states,
                    transitions,
                    depth: depth_reached,
                },
                depth: horizon,
                frontier: frontier_of(digest, &queue),
            };
        }
        if poll() {
            break Outcome::Cancelled {
                stats: Stats {
                    states,
                    transitions,
                    depth: depth_reached,
                },
                frontier: frontier_of(digest, &queue),
                cause: CANCELLED_CAUSE,
            };
        }
        let (successors, state_findings) = match expand(system, &state, identity)? {
            Expanded::Successors {
                successors,
                findings,
            } => (successors, findings),
            Expanded::Stopped(stop) => {
                break Outcome::Stopped {
                    stats: Stats {
                        states,
                        transitions,
                        depth: depth_reached,
                    },
                    frontier: frontier_of(digest, &queue),
                    cause: stop.cause,
                };
            }
        };
        for successor in successors {
            if transitions >= limits.max_transitions {
                break 'run Outcome::Bounded {
                    stats: Stats {
                        states,
                        transitions,
                        depth: depth_reached,
                    },
                    frontier: frontier_of(digest, &queue),
                    limit: Limit::Transitions(limits.max_transitions),
                };
            }
            transitions += 1;
            if visited.contains(&successor.key) {
                continue;
            }
            if states >= limits.max_states {
                let mut frontier = frontier_of(digest, &queue);
                frontier.push(successor.digest);
                break 'run Outcome::Bounded {
                    stats: Stats {
                        states,
                        transitions,
                        depth: depth_reached,
                    },
                    frontier,
                    limit: Limit::States(limits.max_states),
                };
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
        // Recorded only once every successor is queued: a state a limit put
        // back in the frontier is unexpanded and has no entry.
        if !state_findings.is_empty() {
            findings.push(StateFindings {
                state: digest,
                depth,
                findings: state_findings,
            });
        }
    };

    Ok(Exploration { outcome, findings })
}

/// Explore `system`, first refusing an unbounded `domains` request before any
/// `TransitionSystem` method is called (FR-101).
///
/// `max_depth` is the search horizon: `Some(k)` expands no state at depth
/// `>= k` and reports `Outcome::BoundReached` when it is the only thing that
/// stopped the run; `None` sets no horizon.
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
#[allow(
    clippy::too_many_arguments,
    reason = "FR-101 pins this exact signature; the parameters are the request's own fields, not an accretion of unrelated flags"
)]
pub fn explore_request<S: TransitionSystem>(
    system: &S,
    domains: &[(WireNodeId, &ValueType)],
    types: &TypeEnvironment,
    position_limit: u64,
    limits: Limits,
    identity: IdentityLimits,
    max_depth: Option<usize>,
    poll: impl FnMut() -> bool,
) -> Result<Exploration<S::Finding>, NotSimulated> {
    check_requires_bound(domains, types, position_limit)?;
    explore(system, limits, identity, max_depth, poll).map_err(NotSimulated::from)
}

#[cfg(test)]
mod stop_category_tests {
    use super::{stop_category, CatalogCode, Category};
    use ix_trace_rs::trace;

    /// FR-101-AC-12: a stop for an unavailable observation or an incomplete
    /// population is incomplete, as is a stop at a work budget; a refusal
    /// code is not.
    #[trace("TC-474", "FR-101-AC-12")]
    #[test]
    fn a_stop_for_missing_evidence_or_a_budget_is_incomplete() {
        for code in [
            "unavailable_observation",
            "incomplete_population",
            "resource_exhausted",
        ] {
            assert_eq!(
                stop_category(&CatalogCode::new(code, "x")),
                Category::Incomplete,
                "{code}"
            );
        }
        assert_eq!(
            stop_category(&CatalogCode::new("wrong_snapshot", "x")),
            Category::Refusal
        );
    }
}
