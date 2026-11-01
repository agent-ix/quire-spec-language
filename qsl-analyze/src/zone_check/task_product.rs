// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL FR-249 and QSpec FR-419: compile the exact fixed-work gate and the
//! finite task-slot bounds shared by EN-6 and the FR-418 reader.

use num_integer::Integer as NumInteger;
use qsl_semantics::check::{Arrival, SchedulingPolicy, TaskSet};
use quire_exact::{Integer, Rational};

/// One task type in the finite fixed-work product, in the checked task set's
/// unit. The row order is the checked task declaration order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixedWorkTask {
    /// Task-set identity.
    pub task_set: String,
    /// Task identity.
    pub task: String,
    /// `periodic` or `sporadic`.
    pub arrival: FixedWorkArrival,
    /// Enforced minimum nominal activation spacing.
    pub interarrival: Rational,
    /// Nominal relative deadline.
    pub relative_deadline: Rational,
    /// Maximum release jitter.
    pub max_jitter: Rational,
    /// The singleton actual demand selected by BCET and WCET.
    pub work: Rational,
    /// `ceil(D/T)+1`, the maximum live-job slot count for this task type.
    pub max_jobs: u64,
    /// Priority, required only by fixed-priority scheduling.
    pub priority: Option<u64>,
}

/// The enforced arrival kind of a task in the exact product.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixedWorkArrival {
    /// Exactly one nominal activation per period.
    Periodic,
    /// At least this minimum nominal inter-arrival interval.
    Sporadic,
}

/// The exact fixed-work product gate compiled from a checked task set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixedWorkTaskProduct {
    /// Task-set identity.
    pub task_set: String,
    /// Selected scheduling policy.
    pub policy: SchedulingPolicy,
    /// Tasks in declaration order.
    pub tasks: Vec<FixedWorkTask>,
    /// Sum of every task type's finite slot bound.
    pub live_job_cap: u64,
}

/// Why a task set is outside the exact fixed-work finite-product gate.
/// These cases remain admitted on FR-249's bounded-search path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixedWorkGateMiss {
    /// At least one task's actual-demand interval is not a singleton.
    VariableDemand,
    /// The fixed-priority scheduler premises are absent from the checked
    /// task set.
    MissingPriority,
    /// A task's candidate slot count or the total cap cannot be represented
    /// by FR-418's bounded count.
    SlotCapOverflow,
}

/// Compile the FR-419 exact fixed-work gate. This only computes the task
/// rows and finite cap; the reader still recompiles and explores the complete
/// source/task product before accepting a certificate.
pub fn fixed_work_task_product(
    task_set: &TaskSet,
    policy: SchedulingPolicy,
) -> Result<FixedWorkTaskProduct, FixedWorkGateMiss> {
    let mut tasks = Vec::with_capacity(task_set.tasks.len());
    let mut live_job_cap = 0_u64;
    for task in &task_set.tasks {
        let work = task
            .fixed_work()
            .cloned()
            .ok_or(FixedWorkGateMiss::VariableDemand)?;
        let (arrival, interarrival) = match &task.arrival {
            Arrival::Periodic { period } => (FixedWorkArrival::Periodic, period.clone()),
            Arrival::Sporadic { min_interarrival } => {
                (FixedWorkArrival::Sporadic, min_interarrival.clone())
            }
        };
        let ratio = task
            .deadline
            .div(&interarrival)
            .ok_or(FixedWorkGateMiss::SlotCapOverflow)?;
        let ceil = Integer::from_big(
            ratio
                .numerator()
                .as_big()
                .div_ceil(ratio.denominator().as_big()),
        );
        let max_jobs = ceil
            .add(&Integer::one())
            .to_u64()
            .filter(|count| *count > 0)
            .ok_or(FixedWorkGateMiss::SlotCapOverflow)?;
        live_job_cap = live_job_cap
            .checked_add(max_jobs)
            .ok_or(FixedWorkGateMiss::SlotCapOverflow)?;
        tasks.push(FixedWorkTask {
            task_set: task_set.name.clone(),
            task: task.name.clone(),
            arrival,
            interarrival,
            relative_deadline: task.deadline.clone(),
            max_jitter: task.jitter.clone(),
            work,
            max_jobs,
            priority: task.priority,
        });
    }
    if policy == SchedulingPolicy::FixedPriority && tasks.iter().any(|task| task.priority.is_none())
    {
        // Checked FR-246 already refuses this policy/task-set combination.
        // Treat a violation here as outside the exact gate, never invent a
        // priority ordering.
        return Err(FixedWorkGateMiss::MissingPriority);
    }
    Ok(FixedWorkTaskProduct {
        task_set: task_set.name.clone(),
        policy,
        tasks,
        live_job_cap,
    })
}
