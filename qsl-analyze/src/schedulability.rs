// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-7, the closed-form schedulability engine (ADR-026 RT-3 to RT-7,
//! FR-247).
//!
//! [`analyze`] decides a [`ScheduleSubject`] in exact rationals: fixed-priority
//! response-time analysis with the busy-period extension, the EDF
//! processor-demand test with Quick Processor-demand Analysis, and AMC-rtb.
//! Each fixed-priority iteration stops at its fixpoint or as soon as its
//! response time passes its bound, so it needs no iteration cap. The result
//! is a pure function of the subject and the limits; the checker in
//! `qsl-replay` recomputes its evidence with its own code (ADR-026 CF-6).

mod amc;
mod arith;
mod busy_period;
mod edf;
mod fixed_priority;

use qsl_replay::{
    ClosedFormOutcome, IncompleteCause, ScheduleClaim, ScheduleLimit, ScheduleSubject,
    SchedulingPolicy,
};
use quire_exact::{Cancel, CancelCause};

/// FR-247's published default for `max_demand_points`: 2^20.
const DEFAULT_MAX_DEMAND_POINTS: u64 = 1_048_576;

/// EN-7's caller-set limits (FR-247).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScheduleLimits {
    /// How many demand points QPA may check.
    pub max_demand_points: u64,
}

impl Default for ScheduleLimits {
    fn default() -> Self {
        Self {
            max_demand_points: DEFAULT_MAX_DEMAND_POINTS,
        }
    }
}

/// Why an analysis stopped before it decided.
enum Stop {
    Cancelled(CancelCause),
    DemandPoints,
}

type Analysed<T> = Result<T, Stop>;

fn poll(cancel: &Cancel) -> Analysed<()> {
    if cancel.poll() {
        if let Some(cause) = cancel.cause() {
            return Err(Stop::Cancelled(cause));
        }
    }
    Ok(())
}

/// Decide `subject` under `limits`, polling `cancel` as it iterates.
pub fn analyze(
    subject: &ScheduleSubject,
    limits: &ScheduleLimits,
    cancel: &Cancel,
) -> ClosedFormOutcome {
    let decided = match subject.claim() {
        ScheduleClaim::Schedulable(SchedulingPolicy::Edf) => edf::analyse(subject, limits, cancel),
        ScheduleClaim::Schedulable(SchedulingPolicy::Amc) => amc::analyse(subject, cancel),
        ScheduleClaim::Schedulable(SchedulingPolicy::FixedPriority)
        | ScheduleClaim::Response { .. } => fixed_priority::analyse(subject, cancel),
    };
    decided.unwrap_or_else(|stop| {
        let setting = ScheduleLimit::MaxDemandPoints(limits.max_demand_points);
        match stop {
            Stop::Cancelled(source) => {
                ClosedFormOutcome::Stopped(IncompleteCause::Cancelled { source }, setting)
            }
            Stop::DemandPoints => {
                ClosedFormOutcome::Stopped(IncompleteCause::ResourceExhausted, setting)
            }
        }
    })
}
