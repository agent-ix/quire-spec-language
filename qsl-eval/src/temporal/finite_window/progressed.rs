// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-504: a finite event-position or fixed-sample trace with a progress
//! assertion, read through the finite-window evaluator.
//!
//! Position `i` of such a trace has coordinate `i`: under event-position the
//! sequence index, under fixed-sample the sample index. The positions are the
//! trace's observations; under fixed-sample the samples after the last
//! observation, up to the seen horizon, are silent samples that carry no
//! observation and read every `holds` atom false. The request's progress is
//! translated once, here, into the progress the evaluator reads, and the
//! request's own assertion is kept for the report and the result identity.

use qsl_semantics::check::{CheckedTemporalClause, TemporalProfile};

use super::cover::ClockCover;
use super::run::reaches;
use super::trace::{
    LowerBoundary, TimestampedObservation, TimestampedTrace, Watermark, WindowClock, WindowProgress,
};
use crate::temporal::ledger::{ProgressClosure, ProgressCompleteness};
use crate::temporal::outcome::TemporalRefusal;
use crate::temporal::plan::{fault, Plan};

/// The progress a finite trace of event-position or fixed-sample positions
/// asserts (FR-504 Inputs).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SampleProgress {
    /// The watermark `W`.
    pub watermark: u64,
    /// The completeness assertion.
    pub completeness: ProgressCompleteness,
    /// The decision scope's closure.
    pub scope: ProgressClosure,
    /// The surrounding execution's closure.
    pub execution: ProgressClosure,
    /// The lower-boundary kind.
    pub lower_boundary: LowerBoundary,
}

/// An assertion that contradicts the trace it accompanies (FR-504
/// "Consistency"): the evaluator refuses it and never shortens it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgressedRefusal {
    /// A fixed-sample watermark below the last position's coordinate.
    WatermarkBelowLastPosition,
    /// A `Complete { through }` below the last position's coordinate.
    CompletenessBelowLastPosition,
    /// The clause is not under the event-position or fixed-sample profile.
    NotSampleProfile {
        /// The clause's profile.
        profile: TemporalProfile,
    },
}

impl TimestampedTrace {
    /// Admit the `observations` of an event-position or fixed-sample trace,
    /// in order, with the `progress` it asserts (FR-504).
    ///
    /// The seen horizon `S` is, under fixed-sample, `min(W, through)` for a
    /// `Complete { through }` and the last position for `Incomplete`; under
    /// event-position it is the last position, because the watermark does
    /// not advance the clock in silence. Fixed-sample positions after the
    /// last observation up to `S` are silent samples, kept only as far as the
    /// clause's reach can read them.
    ///
    /// # Errors
    ///
    /// A [`TemporalRefusal`] for an empty trace or a fault; a
    /// [`ProgressedRefusal`] for an assertion the trace contradicts.
    pub fn admit_progressed(
        clause: &CheckedTemporalClause,
        observations: Vec<String>,
        progress: SampleProgress,
    ) -> Result<Result<Self, ProgressedRefusal>, TemporalRefusal> {
        let profile = clause.profile();
        let fixed = match profile {
            TemporalProfile::FixedSampleFalseExtension => true,
            TemporalProfile::EventPositionFalseExtension => false,
            TemporalProfile::TimestampedEventFiniteWindow
            | TemporalProfile::InfiniteTrace
            | TemporalProfile::Timed => {
                return Ok(Err(ProgressedRefusal::NotSampleProfile { profile }))
            }
        };
        let Some(last) = observations.len().checked_sub(1) else {
            return Err(TemporalRefusal::EmptyTrace);
        };
        let last = u64::try_from(last).map_err(|_| TemporalRefusal::TraceTooLong)?;
        if fixed && progress.watermark < last {
            return Ok(Err(ProgressedRefusal::WatermarkBelowLastPosition));
        }
        if matches!(progress.completeness, ProgressCompleteness::Complete { through } if through < last)
        {
            return Ok(Err(ProgressedRefusal::CompletenessBelowLastPosition));
        }
        let closed = progress.scope == ProgressClosure::Closed;
        let seen = match (fixed, progress.completeness) {
            (true, ProgressCompleteness::Complete { through }) => progress.watermark.min(through),
            (true, ProgressCompleteness::Incomplete) | (false, _) => last,
        };
        // The evaluator establishes a window by the watermark and the
        // completeness it reads. Under fixed-sample they are the assertion's;
        // the completeness covers the seen horizon only. Under event-position
        // the watermark advances nothing: an open scope reaches the last
        // position, and a closed scope has no later observation, which a
        // complete input reads by false extension.
        let (watermark, completeness) = if fixed {
            (
                progress.watermark,
                ProgressCompleteness::Complete { through: seen },
            )
        } else if closed {
            (
                u64::MAX,
                match progress.completeness {
                    ProgressCompleteness::Complete { .. } => {
                        ProgressCompleteness::Complete { through: u64::MAX }
                    }
                    ProgressCompleteness::Incomplete => {
                        ProgressCompleteness::Complete { through: seen }
                    }
                },
            )
        } else {
            (seen, ProgressCompleteness::Complete { through: seen })
        };
        let silent = if fixed {
            let plan = Plan::new(clause.formula()).map_err(TemporalRefusal::Fault)?;
            let reach = *reaches(&plan)
                .map_err(TemporalRefusal::Fault)?
                .get(plan.root)
                .ok_or_else(|| TemporalRefusal::Fault(fault("plan-index-in-arena")))?;
            let readable = i128::from(last).saturating_add(reach);
            let horizon = i128::from(seen).min(readable);
            u64::try_from(horizon.saturating_sub(i128::from(last))).unwrap_or(0)
        } else {
            0
        };
        let observed = observations.len();
        let mut positions: Vec<TimestampedObservation> = Vec::new();
        for (index, observation) in observations.into_iter().enumerate() {
            positions.push(TimestampedObservation {
                stamp: u64::try_from(index).map_err(|_| TemporalRefusal::TraceTooLong)?,
                order_key: None,
                observation,
            });
        }
        for offset in 1..=silent {
            let stamp = last
                .checked_add(offset)
                .ok_or(TemporalRefusal::TraceTooLong)?;
            positions.push(TimestampedObservation {
                stamp,
                order_key: None,
                observation: format!("silent:{stamp}"),
            });
        }
        let asserted = WindowProgress {
            watermark: Watermark::Inclusive(progress.watermark),
            completeness: progress.completeness,
            scope: progress.scope,
            execution: progress.execution,
            lower_boundary: progress.lower_boundary,
        };
        let mut trace = Self::admit(
            clause.name(),
            WindowClock {
                unit: clause.clock().to_owned(),
                cover: ClockCover::Exact,
            },
            positions,
            WindowProgress {
                watermark: Watermark::Inclusive(watermark),
                completeness,
                ..asserted
            },
        )?;
        trace.asserted = asserted;
        trace.observed = observed;
        Ok(Ok(trace))
    }
}
