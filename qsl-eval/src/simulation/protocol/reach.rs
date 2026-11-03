// SPDX-License-Identifier: AGPL-3.0-or-later
//! Every reachable state of a [`ProtocolSystem`], breadth-first from its
//! initial states, each state once by its full canonical state key and its
//! successors in transition-identity order.
//!
//! Each breadth-first level expands on up to a caller-given number of worker
//! threads and is admitted in frontier order, so the state indices and steps
//! equal those of a sequential FIFO search whatever the worker count.
//!
//! The search is a method of each memory model this crate implements
//! ([`Sc`], [`Tso`] and [`Ra`]) rather than generic over [`MemoryModel`]: a
//! generic function is compiled in the crate that instantiates it, at that
//! crate's optimization level, while these methods compile the whole
//! expansion here.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::{MemoryModel, ProtocolState, ProtocolSystem, ProtocolTransition, Ra, Sc, Tso};
use crate::simulation::expansion::ExpansionStop;
use crate::simulation::explore::TransitionSystem;
use crate::simulation::key::{EncodingRefusal, StateKey};
use crate::simulation::subject::SubjectSystem;

/// One reachable state: the state, its canonical state key, and each of its
/// successors' transition and state index, in transition-identity order.
#[derive(Clone, Debug)]
pub struct ReachedState<M: MemoryModel> {
    /// The state.
    pub state: ProtocolState<M>,
    /// The state's canonical state key.
    pub key: StateKey,
    /// Each successor's transition and its index in the reachable states.
    pub steps: Vec<(ProtocolTransition, usize)>,
}

/// The reachable states of a protocol system in breadth-first admission
/// order, and the greatest number of workers one level expanded on.
#[derive(Clone, Debug)]
pub struct Reachable<M: MemoryModel> {
    /// Every reachable state, the initial states first.
    pub states: Vec<ReachedState<M>>,
    /// The greatest number of worker threads spawned for one level: 1 when
    /// every level expanded on the calling thread.
    pub workers: usize,
}

/// Why the reachable states could not be computed.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ReachRefusal {
    /// A state's expansion stopped.
    #[error("the expansion stopped: {0:?}")]
    Expansion(ExpansionStop),
    /// A state or transition has no canonical encoding.
    #[error(transparent)]
    Encoding(#[from] EncodingRefusal),
}

impl From<ExpansionStop> for ReachRefusal {
    fn from(stop: ExpansionStop) -> Self {
        Self::Expansion(stop)
    }
}

macro_rules! reachable_under {
    ($($model:ty),+) => {$(
        impl ProtocolSystem<'_, $model> {
            /// Every reachable state, breadth-first from the initial states,
            /// each once by its canonical state key, with successors in
            /// transition-identity order. Each level expands on up to
            /// `worker_limit` threads and is admitted in frontier order, so
            /// the result does not depend on the worker count.
            ///
            /// The search is unbounded: it ends when the frontier is empty.
            ///
            /// # Errors
            ///
            /// [`ReachRefusal::Expansion`] when a state's expansion stops and
            /// [`ReachRefusal::Encoding`] when a state or transition has no
            /// canonical encoding: the first such refusal in breadth-first
            /// order.
            ///
            /// # Panics
            ///
            /// Re-raises a panic of an expansion worker.
            // Never inlined, so that the expansion is instantiated in this
            // crate rather than in a caller's.
            #[inline(never)]
            pub fn reachable(
                &self,
                worker_limit: NonZeroUsize,
            ) -> Result<Reachable<$model>, ReachRefusal> {
                reachable(self, worker_limit)
            }
        }
    )+};
}

reachable_under!(Sc, Tso, Ra);

/// The successors of `state` sorted by transition identity, each with its
/// canonical state key.
fn expand<M: MemoryModel>(
    system: &ProtocolSystem<'_, M>,
    state: &ProtocolState<M>,
) -> Expanded<ProtocolTransition, ProtocolState<M>> {
    let mut successors = system
        .successors(state)?
        .successors
        .into_iter()
        .map(|(transition, next)| Ok((system.transition_key(&transition)?, transition, next)))
        .collect::<Result<Vec<_>, ReachRefusal>>()?;
    successors.sort_by(|left, right| left.0.cmp(&right.0));
    successors
        .into_iter()
        .map(|(_, transition, next)| {
            let key = system.canonical_state_key(&next)?;
            Ok((transition, next, key))
        })
        .collect()
}

/// [`ProtocolSystem::reachable`] for any memory model.
fn reachable<M>(
    system: &ProtocolSystem<'_, M>,
    worker_limit: NonZeroUsize,
) -> Result<Reachable<M>, ReachRefusal>
where
    M: MemoryModel + Sync,
    M::Component: Send + Sync,
{
    let initial = system
        .initial()
        .into_iter()
        .map(|state| Ok((system.canonical_state_key(&state)?, state)));
    let (nodes, workers) = search(initial, |state| expand(system, state), worker_limit)?;
    let states = nodes
        .into_iter()
        .map(|node| ReachedState {
            state: node.state,
            key: node.key,
            steps: node.steps,
        })
        .collect();
    Ok(Reachable { states, workers })
}

/// One expanded state's successors, each with its canonical state key, in
/// the order they are admitted.
type Expanded<T, S> = Result<Successors<T, S>, ReachRefusal>;

/// Successors, each with its transition and canonical state key.
type Successors<T, S> = Vec<(T, S, StateKey)>;

/// One reached state of [`search`].
struct Node<S, T> {
    state: S,
    key: StateKey,
    steps: Vec<(T, usize)>,
}

/// The breadth-first search behind [`ProtocolSystem::reachable`] over any
/// state and transition type: `initial` gives the initial states with their
/// keys and `expand` a state's successors in admission order. Returns the
/// reached states and the greatest number of workers one level used.
fn search<S, T, F>(
    initial: impl IntoIterator<Item = Result<(StateKey, S), ReachRefusal>>,
    expand: F,
    worker_limit: NonZeroUsize,
) -> Result<(Vec<Node<S, T>>, usize), ReachRefusal>
where
    S: Sync + Send,
    T: Send + Sync,
    F: Fn(&S) -> Expanded<T, S> + Sync,
{
    // State identity is the full canonical key, so equality cannot collapse
    // distinct states through a key hash collision; an ordered map compares
    // keys with `memcmp` rather than hashing every byte of every key.
    let mut known: BTreeMap<StateKey, usize> = BTreeMap::new();
    let mut nodes: Vec<Node<S, T>> = Vec::new();
    let mut frontier = Vec::new();
    let mut workers = 1;
    let mut admit =
        |state: S, key: StateKey, nodes: &mut Vec<Node<S, T>>, frontier: &mut Vec<usize>| {
            if let Some(&index) = known.get(&key) {
                return index;
            }
            let index = nodes.len();
            known.insert(key.clone(), index);
            nodes.push(Node {
                state,
                key,
                steps: Vec::new(),
            });
            frontier.push(index);
            index
        };
    for entry in initial {
        let (key, state) = entry?;
        admit(state, key, &mut nodes, &mut frontier);
    }
    while !frontier.is_empty() {
        let level = std::mem::take(&mut frontier);
        let (expanded, used) = expand_level(&expand, &nodes, &level, worker_limit)?;
        workers = workers.max(used);
        for (at, successors) in level.into_iter().zip(expanded) {
            for (transition, state, key) in successors {
                let next = admit(state, key, &mut nodes, &mut frontier);
                nodes[at].steps.push((transition, next));
            }
        }
    }
    Ok((nodes, workers))
}

/// Expands every state of `frontier` on up to `worker_limit` threads,
/// returning each expansion at its frontier position and the number of
/// workers used: the first refusal in frontier order, whichever worker
/// met it first.
fn expand_level<S, T, F>(
    expand: &F,
    nodes: &[Node<S, T>],
    frontier: &[usize],
    worker_limit: NonZeroUsize,
) -> Result<(Vec<Successors<T, S>>, usize), ReachRefusal>
where
    S: Sync + Send,
    T: Send + Sync,
    F: Fn(&S) -> Expanded<T, S> + Sync,
{
    let workers = worker_limit.get().min(frontier.len());
    if workers <= 1 {
        let expanded = frontier
            .iter()
            .map(|&at| expand(&nodes[at].state))
            .collect::<Result<_, _>>()?;
        return Ok((expanded, 1));
    }
    let next = AtomicUsize::new(0);
    let mut expanded: Vec<Option<Expanded<T, S>>> = std::iter::repeat_with(|| None)
        .take(frontier.len())
        .collect();
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let position = next.fetch_add(1, Ordering::Relaxed);
                        let Some(&at) = frontier.get(position) else {
                            return done;
                        };
                        done.push((position, expand(&nodes[at].state)));
                    }
                })
            })
            .collect();
        for handle in handles {
            let done = handle
                .join()
                .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
            for (position, successors) in done {
                expanded[position] = Some(successors);
            }
        }
    });
    // Every position below the frontier's length was claimed by exactly one
    // worker, which ran to completion; the first refusal in frontier order
    // is the result.
    let expanded = expanded
        .into_iter()
        .map(|successors| successors.unwrap_or_else(|| Err(broken())))
        .collect::<Result<_, _>>()?;
    Ok((expanded, workers))
}

/// The refusal of a frontier position no worker expanded: a broken
/// invariant of the level's work distribution.
fn broken() -> ReachRefusal {
    ReachRefusal::Expansion(super::broken())
}

#[cfg(test)]
mod tests {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::time::Duration;

    use ix_trace_rs::trace;

    use super::*;
    use crate::simulation::expansion::ExpansionLimit;
    use crate::simulation::model::EXHAUSTED;

    /// The states of the test system: a binary tree over `0..15` whose
    /// every state also steps back to `0`.
    const STATES: u64 = 15;

    fn key(state: u64) -> StateKey {
        StateKey::from_canonical_bytes(format!("\"{state:02}\"").into_bytes())
    }

    fn initial() -> Vec<Result<(StateKey, u64), ReachRefusal>> {
        vec![Ok((key(0), 0))]
    }

    /// `state`'s successors in admission order, each transition named by
    /// its target.
    fn successors(state: u64) -> Expanded<u64, u64> {
        Ok([2 * state + 1, 2 * state + 2, 0]
            .into_iter()
            .filter(|next| *next < STATES)
            .map(|next| (next, next, key(next)))
            .collect())
    }

    fn workers(limit: usize) -> NonZeroUsize {
        NonZeroUsize::new(limit).expect("a positive worker count")
    }

    /// Each reached state's key and successor indices.
    fn graph(nodes: &[Node<u64, u64>]) -> Vec<(StateKey, Vec<usize>)> {
        nodes
            .iter()
            .map(|node| {
                let steps = node.steps.iter().map(|(_, next)| *next).collect();
                (node.key.clone(), steps)
            })
            .collect()
    }

    /// The search admits states breadth-first in frontier order, coalesces
    /// equal keys, and gives the same graph on one worker and on several.
    #[trace("TC-668")]
    #[test]
    fn the_search_is_breadth_first_and_independent_of_the_worker_count() {
        let (sequential, used) =
            search(initial(), |state| successors(*state), workers(1)).expect("the system expands");
        assert_eq!(used, 1);
        let order: Vec<u64> = sequential.iter().map(|node| node.state).collect();
        assert_eq!(order, (0..STATES).collect::<Vec<_>>());
        assert_eq!(sequential[1].steps, vec![(3, 3), (4, 4), (0, 0)]);
        assert_eq!(sequential[14].steps, vec![(0, 0)]);
        for limit in 2..=8 {
            let (parallel, used) = search(initial(), |state| successors(*state), workers(limit))
                .expect("the system expands");
            // The widest level holds 8 states.
            assert_eq!(used, limit);
            assert_eq!(graph(&parallel), graph(&sequential), "{limit} workers");
        }
    }

    /// Two states of one level refuse, and a state of a later level would:
    /// the result is the refusal at the earlier frontier position, on one
    /// worker and on several, although the earlier state's expansion
    /// finishes last.
    #[trace("TC-668")]
    #[test]
    fn the_first_refusal_in_breadth_first_order_is_the_result() {
        let stop = ExpansionStop {
            cause: EXHAUSTED,
            limit: Some(ExpansionLimit::MaxCandidates),
        };
        let refusing = |state: &u64| match *state {
            4 => {
                std::thread::sleep(Duration::from_millis(50));
                Err(ReachRefusal::Expansion(stop))
            }
            5 => Err(EncodingRefusal::NoEncoding("state 5".to_owned()).into()),
            9 => Err(EncodingRefusal::NoEncoding("state 9".to_owned()).into()),
            other => successors(other),
        };
        for limit in 1..=4 {
            let refusal = search(initial(), refusing, workers(limit))
                .err()
                .expect("the search refuses");
            assert_eq!(refusal, ReachRefusal::Expansion(stop), "{limit} workers");
        }
        let encoding = |state: &u64| match *state {
            5 => Err(EncodingRefusal::NoEncoding("state 5".to_owned()).into()),
            other => successors(other),
        };
        for limit in 1..=4 {
            assert_eq!(
                search(initial(), encoding, workers(limit)).err(),
                Some(ReachRefusal::Encoding(EncodingRefusal::NoEncoding(
                    "state 5".to_owned()
                ))),
                "{limit} workers"
            );
        }
        let unkeyed = vec![Err(EncodingRefusal::NoEncoding("initial".to_owned()).into())];
        assert_eq!(
            search(unkeyed, |state| successors(*state), workers(2)).err(),
            Some(ReachRefusal::Encoding(EncodingRefusal::NoEncoding(
                "initial".to_owned()
            )))
        );
    }

    /// A worker's panic reaches the caller with its own payload.
    #[trace("TC-668")]
    #[test]
    fn a_worker_panic_is_re_raised_with_its_payload() {
        let panicking = |state: &u64| {
            if *state == 5 {
                std::panic::panic_any("expanding state 5");
            }
            successors(*state)
        };
        let payload = catch_unwind(AssertUnwindSafe(|| {
            search(initial(), panicking, workers(4)).map(|_| ())
        }))
        .expect_err("the worker's panic is re-raised");
        assert_eq!(payload.downcast_ref::<&str>(), Some(&"expanding state 5"));
    }
}
