// SPDX-License-Identifier: AGPL-3.0-or-later
//! Executable, replayable traces of one sampled path, and replaying one
//! against a `TransitionSystem` (FR-101, ADR-014 TR-1, TR-6, TR-7).

use qsl_foundation::digest::DigestRecord;
use qsl_foundation::selection::DefinitionRef;
use qsl_foundation::CatalogCode;

use crate::simulation::expansion::StateFindings;
use crate::simulation::explore::TransitionSystem;
use crate::simulation::key::EncodingRefusal;
use crate::simulation::order::{expand, sorted_initial, Expanded};

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
    /// The system returned an `ExpansionStop` with this cause at the
    /// trace's last state (FR-101-AC-14).
    Stopped(CatalogCode),
}

/// How a sampled trace was drawn (FR-101, ADR-014 TR-1: replaces
/// `sampler_version: String`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SampleProvenance {
    /// The run's `u64` seed.
    pub seed: u64,
    /// The trace's 0-based index within the run.
    pub trace: u64,
    /// The sampler's `DefinitionRef`, as supplied to `sample_request`.
    pub sampler: DefinitionRef,
    /// Why the run stopped.
    pub stopped: StopReason,
}

/// One executable path: the starting state's key digest, the ordered
/// transitions taken from it and the findings of the states it expanded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trace<T, F> {
    /// The `quire.simulation.state-key/v1` digest of the state the path
    /// starts from.
    pub initial: DigestRecord,
    /// The transitions taken and the resulting digest at each step, in
    /// order.
    pub steps: Vec<Step<T>>,
    /// One entry per state on the trace with at least one finding, in step
    /// order; each entry's `depth` is the state's step index, 0 for the
    /// initial state.
    pub findings: Vec<StateFindings<F>>,
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
    /// The state at step index `step` (0 for the initial state) expanded to
    /// findings other than the recorded ones.
    #[error("step {step}: recomputed findings differ from the recorded ones")]
    FindingMismatch {
        /// The state's step index.
        step: usize,
    },
    /// The state at step index `step` stopped when the trace did not stop
    /// there, did not stop when the trace did, or stopped with a different
    /// cause.
    #[error("step {step}: trace recorded stop {recorded:?}, replay stopped with {replayed:?}")]
    Stopped {
        /// The state's step index.
        step: usize,
        /// The stop cause the trace recorded at this state, if any.
        recorded: Option<CatalogCode>,
        /// The stop cause the recomputed expansion returned, if any.
        replayed: Option<CatalogCode>,
    },
    /// A state or transition identity reached during replay has no RFC
    /// 8785 encoding.
    #[error(transparent)]
    KeyEncoding(#[from] EncodingRefusal),
}

/// Re-run `trace` against `system`, refusing at the first state whose
/// expansion, successor or resulting digest differs from what the trace
/// recorded.
///
/// A trace that replays to completion is executable by `system` exactly as
/// recorded; nothing in the replay path depends on the simulator. Replay
/// expands every state on the trace again, the last included, and requires
/// the same findings and the same stop: a stop with the recorded cause at
/// the last state exactly when the trace ended `StopReason::Stopped`, and no
/// stop anywhere else (FR-101-AC-14). It recomputes each candidate state's
/// digest and compares it with the recorded digest (FR-101).
///
/// A step matches by transition identity *and* recorded digest together:
/// when several successors of the current state share a transition
/// identity, replay picks the one whose digest equals the trace's recorded
/// digest, not simply the first one the system happens to list; when none
/// does, the reported `actual` is the first match in canonical order (the
/// same order `explore` and `sample` themselves walk), not the
/// `TransitionSystem`'s own listing order.
///
/// # Errors
///
/// The first [`ReplayError`] reached, in step order.
pub fn replay<S: TransitionSystem>(
    system: &S,
    trace: &Trace<S::TransitionId, S::Finding>,
) -> Result<(), ReplayError<S::TransitionId>> {
    let mut current = sorted_initial(system)?
        .into_iter()
        .find(|item| item.digest == trace.initial)
        .map(|item| item.state)
        .ok_or(ReplayError::UnknownInitial {
            expected: trace.initial,
        })?;
    let recorded_stop = match trace.provenance.as_ref().map(|p| p.stopped) {
        Some(StopReason::Stopped(cause)) => Some(cause),
        Some(StopReason::StepLimit | StopReason::NoSuccessors) | None => None,
    };
    let last = trace.steps.len();
    let mut check = FindingsCheck::new(&trace.findings);
    let mut current_digest = trace.initial;

    for index in 0..=last {
        let recorded = if index == last { recorded_stop } else { None };
        let (successors, findings) = match expand(system, &current)? {
            Expanded::Successors {
                successors,
                findings,
            } => (successors, findings),
            Expanded::Stopped(stop) if recorded == Some(stop.cause) => break,
            Expanded::Stopped(stop) => {
                return Err(ReplayError::Stopped {
                    step: index,
                    recorded,
                    replayed: Some(stop.cause),
                });
            }
        };
        if recorded.is_some() {
            return Err(ReplayError::Stopped {
                step: index,
                recorded,
                replayed: None,
            });
        }
        check.expanded(current_digest, index, findings)?;
        let Some(step) = trace.steps.get(index) else {
            break;
        };
        let mut first_match_digest: Option<DigestRecord> = None;
        let mut matched_state = None;
        for successor in successors {
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
            (Some(next), _) => {
                current_digest = step.key;
                next
            }
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
    check.finish()
}

/// Compares the findings replay recomputes with the trace's recorded
/// `findings`, entry by entry in step order: depth, state digest and
/// findings all must match, and the recorded list must hold no other entry.
struct FindingsCheck<'a, F> {
    recorded: &'a [StateFindings<F>],
    /// How many recorded entries have matched a recomputed one.
    matched: usize,
}

impl<'a, F: PartialEq> FindingsCheck<'a, F> {
    fn new(recorded: &'a [StateFindings<F>]) -> Self {
        Self {
            recorded,
            matched: 0,
        }
    }

    /// The state at step index `depth`, with digest `state`, expanded to
    /// `findings`. Refuses at the first recorded entry, at or before
    /// `depth`, that differs from the recomputed entries.
    fn expanded<T: std::fmt::Debug>(
        &mut self,
        state: DigestRecord,
        depth: usize,
        findings: Vec<F>,
    ) -> Result<(), ReplayError<T>> {
        let next = self.recorded.get(self.matched);
        if !findings.is_empty() {
            let expected = StateFindings {
                state,
                depth,
                findings,
            };
            return match next {
                Some(entry) if *entry == expected => {
                    self.matched += 1;
                    Ok(())
                }
                Some(entry) => Err(ReplayError::FindingMismatch {
                    step: entry.depth.min(depth),
                }),
                None => Err(ReplayError::FindingMismatch { step: depth }),
            };
        }
        match next {
            // A recorded entry at or before this state that matches no
            // recomputed one: extra, duplicate or out of order.
            Some(entry) if entry.depth <= depth => {
                Err(ReplayError::FindingMismatch { step: entry.depth })
            }
            Some(_) | None => Ok(()),
        }
    }

    /// Replay reached the trace's end: any recorded entry left over matches
    /// no expanded state.
    fn finish<T: std::fmt::Debug>(self) -> Result<(), ReplayError<T>> {
        match self.recorded.get(self.matched) {
            Some(entry) => Err(ReplayError::FindingMismatch { step: entry.depth }),
            None => Ok(()),
        }
    }
}
