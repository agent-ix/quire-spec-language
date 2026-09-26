// SPDX-License-Identifier: AGPL-3.0-or-later
//! Executable, replayable traces of one sampled path, and replaying one
//! against a `TransitionSystem` (FR-101, ADR-014 TR-1, TR-6, TR-7).

use qsl_foundation::digest::DigestRecord;
use qsl_foundation::selection::DefinitionRef;

use crate::simulation::explore::TransitionSystem;
use crate::simulation::key::EncodingRefusal;
use crate::simulation::order::{ordered_successors, sorted_initial};

/// One executed transition and the state-key digest of the state it
/// produced.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Step<T> {
    /// The transition taken.
    pub transition: T,
    /// The resulting state's `quire.simulation.state-key/v1` digest.
    pub key: DigestRecord,
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

/// How a sampled trace was drawn (FR-101, ADR-014 TR-1: replaces
/// `sampler_version: String`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SampleProvenance {
    /// The run's `u64` seed.
    pub seed: u64,
    /// The trace's 0-based index within the run.
    pub trace: u64,
    /// The pinned sampler's exact identity, revision and digest, as
    /// supplied to `sample_request`.
    pub sampler: DefinitionRef,
    /// Why the run stopped.
    pub stopped: StopReason,
}

/// One executable path: the starting state's key digest and the ordered
/// transitions taken from it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trace<T> {
    /// The `quire.simulation.state-key/v1` digest of the state the path
    /// starts from.
    pub initial: DigestRecord,
    /// The transitions taken and the resulting digest at each step, in
    /// order.
    pub steps: Vec<Step<T>>,
    /// Set for a trace `sample` drew; `None` for a trace built by hand, such
    /// as a fixture a test constructs directly.
    pub provenance: Option<SampleProvenance>,
}

/// Why a trace refused to replay.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ReplayError<T: std::fmt::Debug> {
    /// No state returned by `TransitionSystem::initial` has this digest.
    #[error("no initial state has digest {expected:?}")]
    UnknownInitial {
        /// The trace's recorded starting digest.
        expected: DigestRecord,
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
    /// none of them produced the trace's recorded digest.
    #[error(
        "step {step}: transition {transition:?} produced {actual:?}, trace recorded {expected:?}"
    )]
    KeyMismatch {
        /// The zero-based index into `Trace::steps`.
        step: usize,
        /// The transition taken.
        transition: T,
        /// The digest the trace recorded.
        expected: DigestRecord,
        /// The digest of the first matching-transition successor the system
        /// actually produced, in canonical order.
        actual: DigestRecord,
    },
    /// A state or transition identity reached during replay has no RFC
    /// 8785 encoding.
    #[error(transparent)]
    KeyEncoding(#[from] EncodingRefusal),
}

/// Re-run `trace` against `system`, refusing at the first step whose
/// successor or resulting digest differs from what the trace recorded.
///
/// A trace that replays to completion is executable by `system` exactly as
/// recorded; nothing in the replay path depends on the simulator. Replay
/// recomputes each candidate state's digest and compares it with the
/// recorded digest (FR-101).
///
/// A step matches by transition identity *and* recorded digest together:
/// when several successors of the current state share a transition
/// identity, replay picks the one whose digest equals the trace's recorded
/// digest, not simply the first one the system happens to list; when none
/// does, the reported `actual` is the first match in canonical order (the
/// same order `explore` and `sample` themselves walk), not the
/// `TransitionSystem`'s own listing order.
pub fn replay<S: TransitionSystem>(
    system: &S,
    trace: &Trace<S::TransitionId>,
) -> Result<(), ReplayError<S::TransitionId>> {
    let mut current = sorted_initial(system)?
        .into_iter()
        .find(|item| item.digest == trace.initial)
        .map(|item| item.state)
        .ok_or(ReplayError::UnknownInitial {
            expected: trace.initial,
        })?;

    for (index, step) in trace.steps.iter().enumerate() {
        let mut first_match_digest: Option<DigestRecord> = None;
        let mut matched_state = None;
        for successor in ordered_successors(system, &current)? {
            if successor.transition != step.transition {
                continue;
            }
            if first_match_digest.is_none() {
                first_match_digest = Some(successor.digest);
            }
            if successor.digest == step.key {
                matched_state = Some(successor.state);
                break;
            }
        }
        current = match (matched_state, first_match_digest) {
            (Some(next), _) => next,
            (None, Some(actual)) => {
                return Err(ReplayError::KeyMismatch {
                    step: index,
                    transition: step.transition.clone(),
                    expected: step.key,
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
