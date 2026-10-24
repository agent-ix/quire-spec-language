// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fixed-priority response-time analysis (ADR-026 RT-4) and `response`
//! claims.

use qsl_replay::{
    ClosedFormEvidence, ClosedFormOutcome, MissEvidence, ScheduleSubject, Task, TaskFixpoint,
};
use quire_exact::Cancel;

use super::arith::int;
use super::busy_period::{jobs, Jobs, Level, Term};
use super::Analysed;

/// The level-`i` recurrence of `task` with every budget at its highest.
fn level<'a>(subject: &'a ScheduleSubject, task: &'a Task) -> Level<'a> {
    Level {
        own: task.wcet.hi(),
        blocking: &task.blocking,
        constant: int(0),
        terms: subject
            .higher_priority(task)
            .map(|other| Term {
                period: other.arrival.interval(),
                jitter: &other.jitter,
                budget: other.wcet.hi(),
            })
            .collect(),
    }
}

/// Analyse each claimed task, highest priority first: the first to pass its
/// bound is the miss.
pub(super) fn analyse(subject: &ScheduleSubject, cancel: &Cancel) -> Analysed<ClosedFormOutcome> {
    let mut fixpoints = Vec::new();
    for task in subject.analysed() {
        match jobs(task, &level(subject, task), subject.bound(task), cancel)? {
            Jobs::Met(jobs) => fixpoints.push(TaskFixpoint {
                task: task.name.clone(),
                jobs,
            }),
            Jobs::Missed { job, iterates } => {
                return Ok(ClosedFormOutcome::Miss(MissEvidence::FixedPriority {
                    task: task.name.clone(),
                    job,
                    iterates,
                }))
            }
        }
    }
    Ok(ClosedFormOutcome::Schedulable(
        ClosedFormEvidence::FixedPoints(fixpoints),
    ))
}
