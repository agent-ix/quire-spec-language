// SPDX-License-Identifier: AGPL-3.0-or-later
//! The EDF processor-demand test with Quick Processor-demand Analysis
//! (ADR-026 RT-5; Baruah, Rosier and Howell; Zhang and Burns).

use qsl_replay::{
    ClosedFormEvidence, ClosedFormOutcome, DemandWitness, MissEvidence, ScheduleSubject,
};
use quire_exact::{Cancel, Rational};

use super::arith::{ceil, floor, int, quotient};
use super::{poll, Analysed, ScheduleLimits, Stop};

fn utilization(subject: &ScheduleSubject) -> Rational {
    subject.task_set().tasks().iter().fold(int(0), |sum, task| {
        sum.add(&quotient(task.wcet.hi(), task.arrival.interval()))
    })
}

/// `dbf(t) = sum of max(0, floor((t - D) / T) + 1) * C`.
fn demand_bound(subject: &ScheduleSubject, t: &Rational) -> Rational {
    subject.task_set().tasks().iter().fold(int(0), |sum, task| {
        let jobs = floor(&quotient(&t.sub(&task.deadline), task.arrival.interval())).add(&int(1));
        if jobs > int(0) {
            sum.add(&jobs.mul(task.wcet.hi()))
        } else {
            sum
        }
    })
}

/// The synchronous busy period: the least `w` with
/// `w = sum of ceil(w / T) * C`, which exists when utilization is at most 1.
fn busy_period(subject: &ScheduleSubject, cancel: &Cancel) -> Analysed<Rational> {
    let tasks = subject.task_set().tasks();
    let mut window = tasks
        .iter()
        .fold(int(0), |sum, task| sum.add(task.wcet.hi()));
    loop {
        poll(cancel)?;
        let next = tasks.iter().fold(int(0), |sum, task| {
            let count = ceil(&quotient(&window, task.arrival.interval()));
            sum.add(&count.mul(task.wcet.hi()))
        });
        if next == window {
            return Ok(window);
        }
        window = next;
    }
}

/// The greatest absolute deadline strictly below `t`.
fn deadline_below(subject: &ScheduleSubject, t: &Rational) -> Option<Rational> {
    subject
        .task_set()
        .tasks()
        .iter()
        .filter(|task| *t > task.deadline)
        .map(|task| {
            let before =
                ceil(&quotient(&t.sub(&task.deadline), task.arrival.interval())).sub(&int(1));
            task.deadline.add(&before.mul(task.arrival.interval()))
        })
        .max()
}

pub(super) fn analyse(
    subject: &ScheduleSubject,
    limits: &ScheduleLimits,
    cancel: &Cancel,
) -> Analysed<ClosedFormOutcome> {
    let load = utilization(subject);
    if load > int(1) {
        return Ok(ClosedFormOutcome::Miss(MissEvidence::Utilization(load)));
    }
    let length = busy_period(subject, cancel)?;
    let shortest = subject
        .task_set()
        .tasks()
        .iter()
        .map(|task| task.deadline.clone())
        .min();
    let mut sequence: Vec<Rational> = Vec::new();
    let mut next = deadline_below(subject, &length);
    while let (Some(t), Some(shortest)) = (next.take(), shortest.as_ref()) {
        poll(cancel)?;
        let checked = u64::try_from(sequence.len()).map_err(|_| Stop::DemandPoints)?;
        if checked >= limits.max_demand_points {
            return Err(Stop::DemandPoints);
        }
        let demand = demand_bound(subject, &t);
        if demand > t {
            return Ok(ClosedFormOutcome::Miss(MissEvidence::Edf(DemandWitness {
                t,
                demand,
            })));
        }
        sequence.push(t.clone());
        if demand > *shortest {
            next = if demand < t {
                Some(demand)
            } else {
                deadline_below(subject, &t)
            };
        }
    }
    Ok(ClosedFormOutcome::Schedulable(ClosedFormEvidence::Demand {
        busy_period: length,
        qpa_sequence: sequence,
    }))
}
