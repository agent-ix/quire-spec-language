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

/// One expanded state's successors in transition-identity order, each with
/// its canonical state key.
type Expanded<M> = Vec<(ProtocolTransition, ProtocolState<M>, StateKey)>;

/// The successors of `state` sorted by transition identity, each with its
/// canonical state key.
fn expand<M: MemoryModel>(
    system: &ProtocolSystem<'_, M>,
    state: &ProtocolState<M>,
) -> Result<Expanded<M>, ReachRefusal> {
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

/// Expands every state of `frontier` on up to `worker_limit` threads,
/// returning each expansion at its frontier position and the number of
/// workers used.
fn expand_level<M>(
    system: &ProtocolSystem<'_, M>,
    states: &[ReachedState<M>],
    frontier: &[usize],
    worker_limit: NonZeroUsize,
) -> Result<(Vec<Expanded<M>>, usize), ReachRefusal>
where
    M: MemoryModel + Sync,
    M::Component: Send + Sync,
{
    let workers = worker_limit.get().min(frontier.len());
    if workers <= 1 {
        let expanded = frontier
            .iter()
            .map(|&at| expand(system, &states[at].state))
            .collect::<Result<_, _>>()?;
        return Ok((expanded, 1));
    }
    let next = AtomicUsize::new(0);
    let mut expanded: Vec<Option<Result<Expanded<M>, ReachRefusal>>> =
        std::iter::repeat_with(|| None)
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
                        done.push((position, expand(system, &states[at].state)));
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

/// [`ProtocolSystem::reachable`] for any memory model.
fn reachable<M>(
    system: &ProtocolSystem<'_, M>,
    worker_limit: NonZeroUsize,
) -> Result<Reachable<M>, ReachRefusal>
where
    M: MemoryModel + Sync,
    M::Component: Send + Sync,
{
    // State identity is the full canonical key, so equality cannot collapse
    // distinct states through a key hash collision; an ordered map compares
    // keys with `memcmp` rather than hashing every byte of every key.
    let mut known: BTreeMap<StateKey, usize> = BTreeMap::new();
    let mut states: Vec<ReachedState<M>> = Vec::new();
    let mut frontier = Vec::new();
    let mut workers = 1;
    let mut admit = |state: ProtocolState<M>,
                     key: StateKey,
                     states: &mut Vec<ReachedState<M>>,
                     frontier: &mut Vec<usize>|
     -> usize {
        if let Some(&index) = known.get(&key) {
            return index;
        }
        let index = states.len();
        known.insert(key.clone(), index);
        states.push(ReachedState {
            state,
            key,
            steps: Vec::new(),
        });
        frontier.push(index);
        index
    };
    for state in system.initial() {
        let key = system.canonical_state_key(&state)?;
        admit(state, key, &mut states, &mut frontier);
    }
    while !frontier.is_empty() {
        let level = std::mem::take(&mut frontier);
        let (expanded, used) = expand_level(system, &states, &level, worker_limit)?;
        workers = workers.max(used);
        for (at, successors) in level.into_iter().zip(expanded) {
            for (transition, state, key) in successors {
                let next = admit(state, key, &mut states, &mut frontier);
                states[at].steps.push((transition, next));
            }
        }
    }
    Ok(Reachable { states, workers })
}
