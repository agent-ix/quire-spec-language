// SPDX-License-Identifier: AGPL-3.0-or-later
//! Executable, replayable traces of one explored or sampled path.

use crate::simulation::explore::TransitionSystem;

/// One executed transition and the canonical key of the state it produced.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Step<T> {
    /// The transition taken.
    pub transition: T,
    /// The resulting state's full canonical key.
    pub key: Vec<u8>,
}

/// How a sampled trace was drawn.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SampleProvenance {
    /// The seed the sampler was constructed with.
    pub seed: u64,
    /// The sampler's `Sampler::version()` at draw time.
    pub sampler_version: String,
}

/// One executable path: the starting canonical state key and the ordered
/// transitions taken from it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trace<T> {
    /// The canonical key of the state the path starts from.
    pub initial: Vec<u8>,
    /// The transitions taken and the resulting key at each step, in order.
    pub steps: Vec<Step<T>>,
    /// Set when this trace was drawn by a `Sampler`; `None` for a trace
    /// built directly from an exploration run.
    pub provenance: Option<SampleProvenance>,
}

/// Why a trace refused to replay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayError<T> {
    /// No state returned by `TransitionSystem::initial` has this key.
    UnknownInitial {
        /// The trace's recorded starting key.
        expected: Vec<u8>,
    },
    /// At `step`, the current state offers no successor with this
    /// transition identity.
    MissingTransition {
        /// The zero-based index into `Trace::steps`.
        step: usize,
        /// The transition the trace expected to be available.
        transition: T,
    },
    /// At `step`, the transition exists but its successor's key does not
    /// match the trace.
    KeyMismatch {
        /// The zero-based index into `Trace::steps`.
        step: usize,
        /// The transition taken.
        transition: T,
        /// The key the trace recorded.
        expected: Vec<u8>,
        /// The key the system actually produced.
        actual: Vec<u8>,
    },
}

/// Re-run `trace` against `system`, refusing at the first step whose
/// successor or resulting key differs from what the trace recorded.
///
/// A trace that replays to completion is executable by `system` exactly as
/// recorded; nothing in the replay path depends on the simulator.
pub fn replay<S: TransitionSystem>(
    system: &S,
    trace: &Trace<S::TransitionId>,
) -> Result<(), ReplayError<S::TransitionId>> {
    let mut current = system
        .initial()
        .into_iter()
        .find(|state| system.key(state) == trace.initial)
        .ok_or_else(|| ReplayError::UnknownInitial {
            expected: trace.initial.clone(),
        })?;

    for (index, step) in trace.steps.iter().enumerate() {
        let successors = system.successors(&current);
        let found = successors
            .into_iter()
            .find(|(transition, _)| *transition == step.transition);
        let (_, next) = found.ok_or_else(|| ReplayError::MissingTransition {
            step: index,
            transition: step.transition.clone(),
        })?;
        let actual = system.key(&next);
        if actual != step.key {
            return Err(ReplayError::KeyMismatch {
                step: index,
                transition: step.transition.clone(),
                expected: step.key.clone(),
                actual,
            });
        }
        current = next;
    }
    Ok(())
}
