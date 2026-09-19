// SPDX-License-Identifier: AGPL-3.0-or-later
//! Executable, replayable traces of one sampled path, and replaying one
//! against a `TransitionSystem`.

use crate::simulation::explore::TransitionSystem;
use crate::simulation::frontier::StateKey;

/// One executed transition and the canonical key of the state it produced.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Step<T> {
    /// The transition taken.
    pub transition: T,
    /// The resulting state's full canonical key.
    pub key: StateKey,
}

/// Why a sampled run stopped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StopReason {
    /// `max_steps` was reached. The current state may or may not have had
    /// further successors; the run never asked.
    StepLimit,
    /// The current state had no successors to draw from.
    NoSuccessors,
}

/// How a sampled trace was drawn.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SampleProvenance {
    /// The sampler's `Sampler::seed()` at draw time.
    pub seed: u64,
    /// The sampler's `Sampler::version()` at draw time.
    pub sampler_version: String,
    /// Why the run stopped.
    pub stopped: StopReason,
}

/// One executable path: the starting canonical state key and the ordered
/// transitions taken from it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trace<T> {
    /// The canonical key of the state the path starts from.
    pub initial: StateKey,
    /// The transitions taken and the resulting key at each step, in order.
    pub steps: Vec<Step<T>>,
    /// Set for a trace `sample` drew; `None` for a trace built by hand, such
    /// as a fixture a test constructs directly.
    pub provenance: Option<SampleProvenance>,
}

/// Why a trace refused to replay.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ReplayError<T: std::fmt::Debug> {
    /// No state returned by `TransitionSystem::initial` has this key.
    #[error("no initial state has key {expected:?}")]
    UnknownInitial {
        /// The trace's recorded starting key.
        expected: StateKey,
    },
    /// At `step`, the current state offers no successor with this
    /// transition identity.
    #[error("step {step}: no successor offers transition {transition:?}")]
    MissingTransition {
        /// The zero-based index into `Trace::steps`.
        step: usize,
        /// The transition the trace expected to be available.
        transition: T,
    },
    /// At `step`, a successor with this transition identity exists, but
    /// none of them produced the trace's recorded key.
    #[error(
        "step {step}: transition {transition:?} produced {actual:?}, trace recorded {expected:?}"
    )]
    KeyMismatch {
        /// The zero-based index into `Trace::steps`.
        step: usize,
        /// The transition taken.
        transition: T,
        /// The key the trace recorded.
        expected: StateKey,
        /// The key of the first matching-transition successor the system
        /// actually produced.
        actual: StateKey,
    },
}

/// Re-run `trace` against `system`, refusing at the first step whose
/// successor or resulting key differs from what the trace recorded.
///
/// A trace that replays to completion is executable by `system` exactly as
/// recorded; nothing in the replay path depends on the simulator.
///
/// A step matches by transition identity *and* recorded key together: when
/// several successors of the current state share a transition identity,
/// replay picks the one whose key equals the trace's recorded key, not
/// simply the first one the system happens to list.
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
        let mut first_match_key: Option<StateKey> = None;
        let mut matched_state = None;
        for (transition, candidate) in system.successors(&current) {
            if transition != step.transition {
                continue;
            }
            let key = system.key(&candidate);
            if first_match_key.is_none() {
                first_match_key = Some(key.clone());
            }
            if key == step.key {
                matched_state = Some(candidate);
                break;
            }
        }
        current = match (matched_state, first_match_key) {
            (Some(next), _) => next,
            (None, Some(actual)) => {
                return Err(ReplayError::KeyMismatch {
                    step: index,
                    transition: step.transition.clone(),
                    expected: step.key.clone(),
                    actual,
                });
            }
            (None, None) => {
                return Err(ReplayError::MissingTransition {
                    step: index,
                    transition: step.transition.clone(),
                });
            }
        };
    }
    Ok(())
}
