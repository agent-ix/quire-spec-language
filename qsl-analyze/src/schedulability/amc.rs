// SPDX-License-Identifier: AGPL-3.0-or-later
//! AMC-rtb (ADR-026 RT-6; Baruah, Burns and Davis).

use qsl_replay::{
    AmcEvidence, ClosedFormEvidence, ClosedFormOutcome, Criticality, HighModeMiss, MissEvidence,
    ScheduleSubject, Task, TaskFixpoint,
};
use quire_exact::{Cancel, Identifier, Rational};

use super::arith::{ceil, int, quotient};
use super::busy_period::{jobs, Jobs, Level, Term};
use super::Analysed;

/// Low mode: every budget is `C(LO)`.
fn low_level<'a>(subject: &'a ScheduleSubject, task: &'a Task) -> Level<'a> {
    Level {
        own: task.wcet.lo(),
        blocking: &task.blocking,
        constant: int(0),
        terms: subject
            .higher_priority(task)
            .map(|other| Term {
                period: other.arrival.interval(),
                jitter: &other.jitter,
                budget: other.wcet.lo(),
            })
            .collect(),
    }
}

/// High mode of a `HI` task: `C(HI)` over higher-priority `HI` tasks, and
/// higher-priority `LO` tasks charged only up to `low_response`, the task's
/// low-mode response time.
fn high_level<'a>(
    subject: &'a ScheduleSubject,
    task: &'a Task,
    low_response: &Rational,
) -> Level<'a> {
    let mut constant = int(0);
    let mut terms = Vec::new();
    for other in subject.higher_priority(task) {
        match other.criticality {
            Criticality::Hi => terms.push(Term {
                period: other.arrival.interval(),
                jitter: &other.jitter,
                budget: other.wcet.hi(),
            }),
            Criticality::Lo => {
                let count = ceil(&quotient(
                    &low_response.add(&other.jitter),
                    other.arrival.interval(),
                ));
                constant = constant.add(&count.mul(other.wcet.lo()));
            }
        }
    }
    Level {
        own: task.wcet.hi(),
        blocking: &task.blocking,
        constant,
        terms,
    }
}

pub(super) fn analyse(subject: &ScheduleSubject, cancel: &Cancel) -> Analysed<ClosedFormOutcome> {
    let mut low = Vec::new();
    let mut low_responses: Vec<(&Identifier, Rational)> = Vec::new();
    for task in subject.by_priority() {
        match jobs(task, &low_level(subject, task), &task.deadline, cancel)? {
            Jobs::Met(jobs) => {
                if let Some(fixpoint) = jobs.first().and_then(|job| job.last()) {
                    low_responses.push((&task.name, fixpoint.clone()));
                }
                low.push(TaskFixpoint {
                    task: task.name.clone(),
                    jobs,
                });
            }
            Jobs::Missed { job, iterates } => {
                return Ok(ClosedFormOutcome::Miss(MissEvidence::FixedPriority {
                    task: task.name.clone(),
                    job,
                    iterates,
                }))
            }
        }
    }
    let mut high = Vec::new();
    for task in subject
        .by_priority()
        .into_iter()
        .filter(|task| task.criticality == Criticality::Hi)
    {
        let low_response = low_responses
            .iter()
            .find(|(name, _)| **name == task.name)
            .map_or_else(|| int(0), |(_, response)| response.clone());
        match jobs(
            task,
            &high_level(subject, task, &low_response),
            &task.deadline,
            cancel,
        )? {
            Jobs::Met(jobs) => high.push(TaskFixpoint {
                task: task.name.clone(),
                jobs,
            }),
            Jobs::Missed { iterates, .. } => {
                return Ok(ClosedFormOutcome::SufficientTestFailed(
                    ClosedFormEvidence::Amc(AmcEvidence {
                        low,
                        high,
                        failing: Some(HighModeMiss {
                            task: task.name.clone(),
                            iterates,
                        }),
                    }),
                ))
            }
        }
    }
    Ok(ClosedFormOutcome::Schedulable(ClosedFormEvidence::Amc(
        AmcEvidence {
            low,
            high,
            failing: None,
        },
    )))
}
