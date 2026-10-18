// SPDX-License-Identifier: AGPL-3.0-or-later
//! The closed-form checker's recomputation (ADR-026 RT-7, FR-248): every
//! fixpoint, iterate, utilization and demand point of an EN-7 outcome is
//! recomputed here, in exact rationals, from the task set. The code is the
//! checker's own and calls nothing of `qsl-analyze` (ADR-026 CF-6): it
//! verifies stated evidence step by step and never searches, so its work is
//! linear in the evidence, except that a miss for a job `q > 0` also
//! computes the completion of each earlier job.
//!
//! A chain of iterates is the least fixpoint exactly when it starts at the
//! recurrence's start and each entry is the recurrence applied to the one
//! before, because the recurrence is monotone: so "that no smaller value
//! satisfies it" is the chain check, not a second search.

use num_bigint::BigInt;
use num_integer::Integer as NumInteger;
use quire_exact::{Cancel, CancelCause, Identifier, Integer, Rational};

use crate::certificate::closed_form::outcome::{
    AmcEvidence, ClosedFormAnalysis, ClosedFormEvidence, ClosedFormOutcome, DemandWitness,
    HighModeMiss, MissEvidence, TaskFixpoint,
};
use crate::certificate::closed_form::task_set::{Criticality, ScheduleSubject, Task};

/// Why the checker disagrees with an outcome (FR-248): the first stated
/// fact that recomputation does not reproduce.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClosedFormDisagreement {
    /// The outcome's evidence is of another analysis than the claim names.
    Shape,
    /// The evidence names a task the analysis does not cover, names one
    /// twice, or omits one.
    Coverage {
        /// The task.
        task: Identifier,
    },
    /// A stated iterate is not the recurrence's start or step.
    Iterate {
        /// The task.
        task: Identifier,
        /// The job.
        job: u64,
        /// The position among the job's iterates.
        index: usize,
    },
    /// A satisfied job's iterates do not end in a repeated fixpoint.
    NotFixpoint {
        /// The task.
        task: Identifier,
        /// The job.
        job: u64,
    },
    /// A satisfied job's response time exceeds its bound.
    OverBound {
        /// The task.
        task: Identifier,
        /// The job.
        job: u64,
    },
    /// Job `job` closes the level-`i` busy period, so a later job lies past
    /// it.
    BusyPeriodClosed {
        /// The task.
        task: Identifier,
        /// The first job whose completion does not exceed the next release.
        job: u64,
    },
    /// The last job stated for a satisfied task keeps the busy period open.
    BusyPeriodOpen {
        /// The task.
        task: Identifier,
        /// The last stated job.
        job: u64,
    },
    /// A miss's iterates do not end at the first response time past the
    /// bound.
    MissBound {
        /// The task.
        task: Identifier,
        /// The job.
        job: u64,
    },
    /// A failing high-mode criterion names a task that is not `HI`.
    NotHighCriticality {
        /// The task.
        task: Identifier,
    },
    /// The stated utilization is not the task set's, or is not above 1
    /// where a miss says so, or is above 1 where a schedule says not.
    Utilization,
    /// The stated busy-period length is not the synchronous busy period's.
    BusyPeriodLength,
    /// QPA's check sequence is not the one the stepping rule gives.
    QpaSequence {
        /// The position in the sequence.
        index: usize,
    },
    /// `dbf(t)` exceeds `t` at a point a schedule checks, or does not at
    /// the point a miss names.
    Demand {
        /// The point.
        t: Rational,
    },
}

/// The checker's answer on an outcome (FR-248).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClosedFormCheck {
    /// Recomputation reproduces the outcome. A [`ClosedFormOutcome::Stopped`]
    /// holds no evidence and agrees trivially.
    Agrees,
    /// Recomputation does not reproduce the outcome.
    Disagrees(ClosedFormDisagreement),
    /// The caller cancelled the check.
    Cancelled(CancelCause),
}

enum Halt {
    Disagree(ClosedFormDisagreement),
    Cancelled(CancelCause),
}

impl From<ClosedFormDisagreement> for Halt {
    fn from(disagreement: ClosedFormDisagreement) -> Self {
        Self::Disagree(disagreement)
    }
}

type Checked<T> = Result<T, Halt>;

/// Recompute `outcome`'s evidence from `subject` in exact rationals. The
/// layer-6 entry `check_closed_form` reads `subject` from the recompiled
/// package; this is its recomputation.
pub fn recompute(
    subject: &ScheduleSubject,
    outcome: &ClosedFormOutcome,
    cancel: &Cancel,
) -> ClosedFormCheck {
    match verify(subject, outcome, cancel) {
        Ok(()) => ClosedFormCheck::Agrees,
        Err(Halt::Disagree(disagreement)) => ClosedFormCheck::Disagrees(disagreement),
        Err(Halt::Cancelled(cause)) => ClosedFormCheck::Cancelled(cause),
    }
}

fn verify(subject: &ScheduleSubject, outcome: &ClosedFormOutcome, cancel: &Cancel) -> Checked<()> {
    let analysis = subject.analysis();
    match (outcome, analysis) {
        (ClosedFormOutcome::Stopped(..), _) => Ok(()),
        (
            ClosedFormOutcome::Schedulable(ClosedFormEvidence::FixedPoints(listed)),
            ClosedFormAnalysis::FixedPriorityRta,
        ) => fixpoints(subject, listed, Mode::Plain, &[], cancel).map(|_| ()),
        (
            ClosedFormOutcome::Schedulable(ClosedFormEvidence::Demand {
                busy_period,
                qpa_sequence,
            }),
            ClosedFormAnalysis::EdfQpa,
        ) => demand(subject, busy_period, qpa_sequence, cancel),
        (
            ClosedFormOutcome::Schedulable(ClosedFormEvidence::Amc(AmcEvidence {
                low,
                high,
                failing: None,
            })),
            ClosedFormAnalysis::AmcRtb,
        ) => amc(subject, low, high, None, cancel),
        (
            ClosedFormOutcome::SufficientTestFailed(ClosedFormEvidence::Amc(AmcEvidence {
                low,
                high,
                failing: Some(failing),
            })),
            ClosedFormAnalysis::AmcRtb,
        ) => amc(subject, low, high, Some(failing), cancel),
        (
            ClosedFormOutcome::Miss(MissEvidence::FixedPriority {
                task,
                job,
                iterates,
            }),
            ClosedFormAnalysis::FixedPriorityRta | ClosedFormAnalysis::AmcRtb,
        ) => fixed_priority_miss(subject, task, *job, iterates, cancel),
        (
            ClosedFormOutcome::Miss(MissEvidence::Utilization(stated)),
            ClosedFormAnalysis::EdfQpa,
        ) => {
            let actual = utilization(subject);
            if actual == *stated && actual > int(1) {
                Ok(())
            } else {
                Err(ClosedFormDisagreement::Utilization.into())
            }
        }
        (
            ClosedFormOutcome::Miss(MissEvidence::Edf(DemandWitness { t, demand })),
            ClosedFormAnalysis::EdfQpa,
        ) => {
            if demand_bound(subject, t) == *demand && *demand > *t && *t > int(0) {
                Ok(())
            } else {
                Err(ClosedFormDisagreement::Demand { t: t.clone() }.into())
            }
        }
        (
            ClosedFormOutcome::Schedulable(_)
            | ClosedFormOutcome::Miss(_)
            | ClosedFormOutcome::SufficientTestFailed(_),
            _,
        ) => Err(ClosedFormDisagreement::Shape.into()),
    }
}

fn poll(cancel: &Cancel) -> Checked<()> {
    if cancel.poll() {
        if let Some(cause) = cancel.cause() {
            return Err(Halt::Cancelled(cause));
        }
    }
    Ok(())
}

fn int(value: u64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

fn from_big(value: BigInt) -> Rational {
    Rational::from_integer(Integer::from_big(value))
}

/// The least integer at or above `value`; a rational's denominator is
/// positive.
fn ceil(value: &Rational) -> Rational {
    from_big(
        value
            .numerator()
            .as_big()
            .div_ceil(value.denominator().as_big()),
    )
}

/// The greatest integer at or below `value`.
fn floor(value: &Rational) -> Rational {
    from_big(
        value
            .numerator()
            .as_big()
            .div_floor(value.denominator().as_big()),
    )
}

fn ratio(numerator: &Rational, denominator: &Rational) -> Option<Rational> {
    numerator.div(denominator)
}

/// How a task's recurrence is built (ADR-026 RT-4, RT-6).
#[derive(Clone, Copy)]
enum Mode {
    /// Fixed priority: every budget is the task's highest.
    Plain,
    /// AMC-rtb low mode: `C(LO)` throughout.
    Low,
    /// AMC-rtb high mode for a `HI` task.
    High,
}

/// One higher-priority task's term of a recurrence.
struct Interferer<'a> {
    period: &'a Rational,
    jitter: &'a Rational,
    budget: &'a Rational,
}

/// `w = base + sum over varying of ceil((w + J) / T) * C`.
struct Recurrence<'a> {
    base: Rational,
    varying: Vec<Interferer<'a>>,
}

impl Recurrence<'_> {
    fn step(&self, window: &Rational) -> Rational {
        self.varying.iter().fold(self.base.clone(), |sum, term| {
            let count = ceil(&ratio_or_zero(&window.add(term.jitter), term.period));
            sum.add(&count.mul(term.budget))
        })
    }
}

/// A period is positive by admission, so the division always has a value.
fn ratio_or_zero(numerator: &Rational, denominator: &Rational) -> Rational {
    ratio(numerator, denominator).unwrap_or_else(|| int(0))
}

/// The recurrence for job `job` of `task`. `lows` holds each task's
/// low-mode fixpoint, which high mode charges its `LO` interferers up to.
fn recurrence<'a>(
    subject: &'a ScheduleSubject,
    task: &'a Task,
    job: u64,
    mode: Mode,
    lows: &[(Identifier, Rational)],
) -> Recurrence<'a> {
    let own = match mode {
        Mode::Plain | Mode::High => task.wcet.hi(),
        Mode::Low => task.wcet.lo(),
    };
    let mut base = int(job).add(&int(1)).mul(own).add(&task.blocking);
    let mut varying = Vec::new();
    for other in subject.higher_priority(task) {
        let term = |budget| Interferer {
            period: other.arrival.interval(),
            jitter: &other.jitter,
            budget,
        };
        match (mode, other.criticality) {
            (Mode::Plain, _) => varying.push(term(other.wcet.hi())),
            (Mode::Low, _) => varying.push(term(other.wcet.lo())),
            (Mode::High, Criticality::Hi) => varying.push(term(other.wcet.hi())),
            (Mode::High, Criticality::Lo) => {
                let low = lows
                    .iter()
                    .find(|(name, _)| *name == task.name)
                    .map_or_else(|| int(0), |(_, response)| response.clone());
                let count = ceil(&ratio_or_zero(
                    &low.add(&other.jitter),
                    other.arrival.interval(),
                ));
                base = base.add(&count.mul(other.wcet.lo()));
            }
        }
    }
    Recurrence { base, varying }
}

/// `J_i - q * T_i`, which turns a job's iterate into its response time.
fn shift(task: &Task, job: u64) -> Rational {
    task.jitter.sub(&int(job).mul(task.arrival.interval()))
}

/// The time the job after `job` is released, relative to the critical
/// instant: the busy period stays open while `job` completes after it.
fn next_release(task: &Task, job: u64) -> Rational {
    int(job)
        .add(&int(1))
        .mul(task.arrival.interval())
        .sub(&task.jitter)
}

/// Check each listed task's jobs against its recurrence. Returns each
/// task's job-0 fixpoint, in listed order.
fn fixpoints(
    subject: &ScheduleSubject,
    listed: &[TaskFixpoint],
    mode: Mode,
    lows: &[(Identifier, Rational)],
    cancel: &Cancel,
) -> Checked<Vec<(Identifier, Rational)>> {
    let expected = match mode {
        Mode::Plain => subject.analysed(),
        Mode::Low => subject.by_priority(),
        Mode::High => subject
            .by_priority()
            .into_iter()
            .filter(|task| task.criticality == Criticality::Hi)
            .collect(),
    };
    covers(&expected, listed, true)?;
    listed
        .iter()
        .map(|entry| {
            let task = expected
                .iter()
                .find(|task| task.name == entry.task)
                .ok_or_else(|| coverage(&entry.task))?;
            job_fixpoints(subject, task, entry, mode, lows, cancel)
        })
        .collect()
}

fn coverage(task: &Identifier) -> Halt {
    ClosedFormDisagreement::Coverage { task: task.clone() }.into()
}

/// The listed entries name distinct tasks among `expected`, and, when
/// `exact`, every one of them.
fn covers(expected: &[&Task], listed: &[TaskFixpoint], exact: bool) -> Checked<()> {
    for (index, entry) in listed.iter().enumerate() {
        let known = expected.iter().any(|task| task.name == entry.task);
        let repeated = listed[..index].iter().any(|seen| seen.task == entry.task);
        if !known || repeated {
            return Err(coverage(&entry.task));
        }
    }
    if exact {
        if let Some(task) = expected
            .iter()
            .find(|task| !listed.iter().any(|entry| entry.task == task.name))
        {
            return Err(coverage(&task.name));
        }
    }
    Ok(())
}

fn job_fixpoints(
    subject: &ScheduleSubject,
    task: &Task,
    entry: &TaskFixpoint,
    mode: Mode,
    lows: &[(Identifier, Rational)],
    cancel: &Cancel,
) -> Checked<(Identifier, Rational)> {
    let bound = subject.bound(task);
    let fault = |job: u64| ClosedFormDisagreement::NotFixpoint {
        task: task.name.clone(),
        job,
    };
    let mut first = None;
    for (number, job) in (0_u64..).zip(&entry.jobs) {
        poll(cancel)?;
        let recurrence = recurrence(subject, task, number, mode, lows);
        chain(task, number, &recurrence, &job.0)?;
        let [.., before, last] = job.0.as_slice() else {
            return Err(fault(number).into());
        };
        if before != last {
            return Err(fault(number).into());
        }
        if last.add(&shift(task, number)) > *bound {
            return Err(ClosedFormDisagreement::OverBound {
                task: task.name.clone(),
                job: number,
            }
            .into());
        }
        let open = *last > next_release(task, number);
        let final_job = usize::try_from(number).is_ok_and(|n| n + 1 == entry.jobs.len());
        if open == final_job {
            let task_name = task.name.clone();
            return Err(if final_job {
                ClosedFormDisagreement::BusyPeriodOpen {
                    task: task_name,
                    job: number,
                }
            } else {
                ClosedFormDisagreement::BusyPeriodClosed {
                    task: task_name,
                    job: number,
                }
            }
            .into());
        }
        if first.is_none() {
            first = Some(last.clone());
        }
    }
    first
        .map(|fixpoint| (task.name.clone(), fixpoint))
        .ok_or_else(|| fault(0).into())
}

/// The first iterate is the recurrence's start and each next one is the
/// recurrence applied to the one before.
fn chain(task: &Task, job: u64, recurrence: &Recurrence<'_>, iterates: &[Rational]) -> Checked<()> {
    let wrong = |index| ClosedFormDisagreement::Iterate {
        task: task.name.clone(),
        job,
        index,
    };
    match iterates.first() {
        Some(start) if *start == recurrence.base => {}
        _ => return Err(wrong(0).into()),
    }
    for (index, pair) in iterates.windows(2).enumerate() {
        if let [before, after] = pair {
            if recurrence.step(before) != *after {
                return Err(wrong(index + 1).into());
            }
        }
    }
    Ok(())
}

/// A chain that misses: only its last iterate gives a response time above
/// `bound`.
fn missing_chain(
    task: &Task,
    job: u64,
    recurrence: &Recurrence<'_>,
    iterates: &[Rational],
    bound: &Rational,
) -> Checked<()> {
    chain(task, job, recurrence, iterates)?;
    let shift = shift(task, job);
    let Some((last, earlier)) = iterates.split_last() else {
        return Err(ClosedFormDisagreement::Iterate {
            task: task.name.clone(),
            job,
            index: 0,
        }
        .into());
    };
    let exceeds = |iterate: &Rational| iterate.add(&shift) > *bound;
    if earlier.iter().any(exceeds) || !exceeds(last) {
        return Err(ClosedFormDisagreement::MissBound {
            task: task.name.clone(),
            job,
        }
        .into());
    }
    Ok(())
}

fn fixed_priority_miss(
    subject: &ScheduleSubject,
    named: &Identifier,
    job: u64,
    iterates: &[Rational],
    cancel: &Cancel,
) -> Checked<()> {
    let mode = match subject.analysis() {
        ClosedFormAnalysis::AmcRtb => Mode::Low,
        ClosedFormAnalysis::FixedPriorityRta | ClosedFormAnalysis::EdfQpa => Mode::Plain,
    };
    let task = subject
        .analysed()
        .into_iter()
        .find(|task| task.name == *named)
        .ok_or_else(|| coverage(named))?;
    missing_chain(
        task,
        job,
        &recurrence(subject, task, job, mode, &[]),
        iterates,
        subject.bound(task),
    )?;
    // Job `q` is in the busy period only if every earlier job keeps it
    // open; otherwise its recurrence overstates its completion.
    for earlier in 0..job {
        poll(cancel)?;
        let recurrence = recurrence(subject, task, earlier, mode, &[]);
        let release = next_release(task, earlier);
        let mut window = recurrence.base.clone();
        loop {
            // The iterates rise, so passing the release proves the
            // completion is past it without finding the fixpoint.
            if window > release {
                break;
            }
            let next = recurrence.step(&window);
            if next == window {
                return Err(ClosedFormDisagreement::BusyPeriodClosed {
                    task: task.name.clone(),
                    job: earlier,
                }
                .into());
            }
            window = next;
            poll(cancel)?;
        }
    }
    Ok(())
}

fn amc(
    subject: &ScheduleSubject,
    low: &[TaskFixpoint],
    high: &[TaskFixpoint],
    failing: Option<&HighModeMiss>,
    cancel: &Cancel,
) -> Checked<()> {
    let lows = fixpoints(subject, low, Mode::Low, &[], cancel)?;
    match failing {
        None => {
            fixpoints(subject, high, Mode::High, &lows, cancel)?;
            Ok(())
        }
        Some(failing) => {
            let high_tasks: Vec<&Task> = subject
                .by_priority()
                .into_iter()
                .filter(|task| task.criticality == Criticality::Hi)
                .collect();
            covers(&high_tasks, high, false)?;
            for entry in high {
                let task = high_tasks
                    .iter()
                    .find(|task| task.name == entry.task)
                    .ok_or_else(|| coverage(&entry.task))?;
                job_fixpoints(subject, task, entry, Mode::High, &lows, cancel)?;
            }
            let task = subject
                .task_set()
                .task(&failing.task)
                .ok_or_else(|| coverage(&failing.task))?;
            if task.criticality != Criticality::Hi {
                return Err(ClosedFormDisagreement::NotHighCriticality {
                    task: task.name.clone(),
                }
                .into());
            }
            if high.iter().any(|entry| entry.task == failing.task) {
                return Err(coverage(&failing.task));
            }
            missing_chain(
                task,
                0,
                &recurrence(subject, task, 0, Mode::High, &lows),
                &failing.iterates,
                &task.deadline,
            )
        }
    }
}

fn utilization(subject: &ScheduleSubject) -> Rational {
    subject.task_set().tasks().iter().fold(int(0), |sum, task| {
        sum.add(&ratio_or_zero(task.wcet.hi(), task.arrival.interval()))
    })
}

/// `dbf(t) = sum of max(0, floor((t - D) / T) + 1) * C`.
fn demand_bound(subject: &ScheduleSubject, t: &Rational) -> Rational {
    subject.task_set().tasks().iter().fold(int(0), |sum, task| {
        let jobs = floor(&ratio_or_zero(
            &t.sub(&task.deadline),
            task.arrival.interval(),
        ))
        .add(&int(1));
        if jobs > int(0) {
            sum.add(&jobs.mul(task.wcet.hi()))
        } else {
            sum
        }
    })
}

/// The synchronous busy period: the least `w` with
/// `w = sum of ceil(w / T) * C`.
fn busy_period(subject: &ScheduleSubject, cancel: &Cancel) -> Checked<Rational> {
    let tasks = subject.task_set().tasks();
    let mut window = tasks
        .iter()
        .fold(int(0), |sum, task| sum.add(task.wcet.hi()));
    loop {
        poll(cancel)?;
        let next = tasks.iter().fold(int(0), |sum, task| {
            let count = ceil(&ratio_or_zero(&window, task.arrival.interval()));
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
            let before = ceil(&ratio_or_zero(
                &t.sub(&task.deadline),
                task.arrival.interval(),
            ))
            .sub(&int(1));
            task.deadline.add(&before.mul(task.arrival.interval()))
        })
        .max()
}

fn demand(
    subject: &ScheduleSubject,
    stated_busy_period: &Rational,
    sequence: &[Rational],
    cancel: &Cancel,
) -> Checked<()> {
    if utilization(subject) > int(1) {
        return Err(ClosedFormDisagreement::Utilization.into());
    }
    let length = busy_period(subject, cancel)?;
    if length != *stated_busy_period {
        return Err(ClosedFormDisagreement::BusyPeriodLength.into());
    }
    let Some(shortest) = subject
        .task_set()
        .tasks()
        .iter()
        .map(|task| task.deadline.clone())
        .min()
    else {
        return Ok(());
    };
    let mut expected = deadline_below(subject, &length);
    for (index, stated) in sequence.iter().enumerate() {
        poll(cancel)?;
        if expected.as_ref() != Some(stated) {
            return Err(ClosedFormDisagreement::QpaSequence { index }.into());
        }
        let demand = demand_bound(subject, stated);
        if demand > *stated {
            return Err(ClosedFormDisagreement::Demand { t: stated.clone() }.into());
        }
        let last = index + 1 == sequence.len();
        if demand <= shortest {
            return if last {
                Ok(())
            } else {
                Err(ClosedFormDisagreement::QpaSequence { index: index + 1 }.into())
            };
        }
        expected = if demand < *stated {
            Some(demand)
        } else {
            deadline_below(subject, stated)
        };
    }
    // The sequence ended with a point still to check, or began with none
    // while a deadline lies below the busy period's end.
    match expected {
        None => Ok(()),
        Some(_) => Err(ClosedFormDisagreement::QpaSequence {
            index: sequence.len(),
        }
        .into()),
    }
}
