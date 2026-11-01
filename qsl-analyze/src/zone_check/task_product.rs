// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL FR-249 and QSpec FR-419: compile the exact fixed-work gate and the
//! finite task-slot bounds shared by EN-6 and the FR-418 reader.

use num_integer::Integer as NumInteger;
use qsl_eval::simulation::model::ModelSystem;
use qsl_replay::{TaskAction, TaskTransitionIdentity};
use qsl_semantics::check::{Arrival, CheckedTaskSets, Release, SchedulingPolicy, TaskSet};
use quire_exact::{Integer, Rational};
use quire_semantic_value::model_transition::ModelTransition;
use std::cmp::Ordering;

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

/// The checked task releases enabled by one source-model operation step.
/// Matching uses the subject's admitted operation declaration identity, not
/// a task or operation name inferred from the transition text.
pub fn task_releases_for_transition<'a>(
    task_sets: &'a CheckedTaskSets,
    system: &ModelSystem<'_>,
    transition: &ModelTransition,
) -> Vec<&'a Release> {
    task_sets
        .releases()
        .iter()
        .filter(|release| {
            system.operation_text(release.operation().declaring(), release.operation().name())
                == Some(transition.operation())
        })
        .collect()
}

/// A task job's discrete scheduler phase. Service and deadline time are
/// represented by auxiliary unit-rate clocks in EN-6, never by a stopwatch
/// field here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskJobPhase {
    /// Selected jitter has not elapsed; the job is not dispatchable.
    Pending,
    /// Ready for the policy-selected scheduler.
    Ready,
    /// Currently executing on the one processor.
    Running,
    /// Preempted and retained with its exact identity.
    Suspended,
    /// A deadline-miss state, which is bad for the task invariant.
    Missed,
}

/// One live job in the separate scheduler state. Its complete typed identity
/// is the QSpec FR-419 derived-transition identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskJob {
    /// Identity shared by every action for this job.
    pub identity: TaskTransitionIdentity,
    /// Current scheduler phase.
    pub phase: TaskJobPhase,
}

/// Per-source-transition occurrence counter, retained in the scheduler
/// state so repeated activations never reuse a job identity.
#[derive(Clone, Debug, Eq, PartialEq)]
struct ActivationCounter {
    task_set: String,
    task: String,
    source: ModelTransition,
    next: u64,
}

/// A bounded slot's generation counter. Reusing a finished slot does not
/// reuse the identity of a live job.
#[derive(Clone, Debug, Eq, PartialEq)]
struct SlotGeneration {
    task_set: String,
    task: String,
    slot: u64,
    next: u64,
}

/// QSpec FR-419's scheduler control, independent of `ModelSimState`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TaskSchedulerState {
    /// Live jobs in canonical task-set/task/slot order.
    jobs: Vec<TaskJob>,
    /// Which job is currently running, by its stable typed identity.
    pub running: Option<TaskTransitionIdentity>,
    activations: Vec<ActivationCounter>,
    generations: Vec<SlotGeneration>,
}

impl TaskSchedulerState {
    /// The live jobs in canonical task-set/task/slot order.
    pub fn jobs(&self) -> &[TaskJob] {
        &self.jobs
    }
}

/// Why a scheduler control operation could not be applied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskSchedulerError {
    /// The product attempted to allocate a slot beyond the reader-checked
    /// finite cap; this is a search stop, never a transition or miss.
    LiveJobCap,
    /// The selected jitter is not inside the checked `[0, J]` interval.
    InvalidJitter,
    /// No pending job has the requested identity.
    UnknownJob,
    /// The requested action is not enabled by the current scheduler phase.
    ActionNotEnabled,
    /// An exact rational counter reached the representable index limit.
    IdentityOverflow,
    /// The checked release declaration does not bind this transition to the
    /// selected task set and task.
    InvalidReleaseBinding,
}

impl TaskSchedulerState {
    /// Apply one checked QSL `release` binding for the source transition.
    /// The release is admitted only when its declaration identity resolves
    /// to the operation identity carried by `source` in this model subject.
    pub fn activate_release(
        &mut self,
        product: &FixedWorkTaskProduct,
        release: &Release,
        system: &ModelSystem<'_>,
        source: ModelTransition,
        jitter: Rational,
        nominal_activation: Rational,
    ) -> Result<TaskTransitionIdentity, TaskSchedulerError> {
        let operation =
            system.operation_text(release.operation().declaring(), release.operation().name());
        if release.task_set() != product.task_set || operation != Some(source.operation()) {
            return Err(TaskSchedulerError::InvalidReleaseBinding);
        }
        self.activate(product, release.task(), source, jitter, nominal_activation)
    }

    /// Apply a source activation as one task-derived job creation. `jitter`
    /// and `nominal_activation` are exact clock values selected by the EN-6
    /// product transition; the resulting job remains pending until its
    /// jitter clock reaches the selected value.
    pub fn activate(
        &mut self,
        product: &FixedWorkTaskProduct,
        task_name: &str,
        source: ModelTransition,
        jitter: Rational,
        nominal_activation: Rational,
    ) -> Result<TaskTransitionIdentity, TaskSchedulerError> {
        if product.policy == SchedulingPolicy::Amc {
            return Err(TaskSchedulerError::ActionNotEnabled);
        }
        let task = product
            .tasks
            .iter()
            .find(|task| task.task == task_name)
            .ok_or(TaskSchedulerError::ActionNotEnabled)?;
        if product.policy == SchedulingPolicy::FixedPriority && task.priority.is_none() {
            return Err(TaskSchedulerError::ActionNotEnabled);
        }
        if jitter < Rational::from_integer(Integer::zero()) || jitter > task.max_jitter {
            return Err(TaskSchedulerError::InvalidJitter);
        }
        let live_count = self
            .jobs
            .iter()
            .filter(|job| job.identity.taskset == task.task_set && job.identity.task == task.task)
            .count();
        if live_count >= usize::try_from(task.max_jobs).unwrap_or(usize::MAX) {
            return Err(TaskSchedulerError::LiveJobCap);
        }
        let slot = (0..task.max_jobs)
            .find(|slot| {
                self.jobs.iter().all(|job| {
                    job.identity.taskset != task.task_set
                        || job.identity.task != task.task
                        || job.identity.slot != *slot
                })
            })
            .ok_or(TaskSchedulerError::LiveJobCap)?;
        let generation =
            match self.generations.iter().find(|row| {
                row.task_set == task.task_set && row.task == task.task && row.slot == slot
            }) {
                Some(row) => row.next,
                None => 0,
            };
        let activation = match self.activations.iter().find(|row| {
            row.task_set == task.task_set
                && row.task == task.task
                && same_transition(&row.source, &source)
        }) {
            Some(row) => row.next,
            None => 0,
        };
        let identity = TaskTransitionIdentity {
            action: TaskAction::Activate,
            taskset: task.task_set.clone(),
            task: task.task.clone(),
            job: activation,
            slot,
            generation,
            release: source.clone(),
            source_transition: Some(source.clone()),
            demand: task.work.clone(),
            jitter: jitter.clone(),
            deadline: nominal_activation.add(&task.relative_deadline),
            priority: task.priority,
            budget_delta: None,
        };
        if let Some(row) = self.activations.iter_mut().find(|row| {
            row.task_set == task.task_set
                && row.task == task.task
                && same_transition(&row.source, &source)
        }) {
            row.next = row
                .next
                .checked_add(1)
                .ok_or(TaskSchedulerError::IdentityOverflow)?;
        } else {
            self.activations.push(ActivationCounter {
                task_set: task.task_set.clone(),
                task: task.task.clone(),
                source,
                next: 1,
            });
        }
        self.jobs.push(TaskJob {
            identity: identity.clone(),
            phase: if jitter.is_zero() {
                TaskJobPhase::Ready
            } else {
                TaskJobPhase::Pending
            },
        });
        self.jobs.sort_by(|left, right| {
            (
                &left.identity.taskset,
                &left.identity.task,
                left.identity.slot,
            )
                .cmp(&(
                    &right.identity.taskset,
                    &right.identity.task,
                    right.identity.slot,
                ))
        });
        Ok(identity)
    }

    /// Release a pending job when its selected jitter has elapsed.
    pub fn release(
        &mut self,
        job: &TaskTransitionIdentity,
    ) -> Result<TaskTransitionIdentity, TaskSchedulerError> {
        let stored = self
            .jobs
            .iter_mut()
            .find(|candidate| same_job(&candidate.identity, job))
            .ok_or(TaskSchedulerError::UnknownJob)?;
        if stored.phase != TaskJobPhase::Pending {
            return Err(TaskSchedulerError::ActionNotEnabled);
        }
        stored.phase = TaskJobPhase::Ready;
        Ok(action_identity(&stored.identity, TaskAction::Release, None))
    }

    /// Dispatch the highest-priority ready or suspended job when the
    /// processor is idle. Fixed-priority tie ordering is by task name, job
    /// occurrence, then bounded slot, each in ascending order.
    pub fn dispatch(
        &mut self,
        product: &FixedWorkTaskProduct,
    ) -> Result<TaskTransitionIdentity, TaskSchedulerError> {
        if self.running.is_some() {
            return Err(TaskSchedulerError::ActionNotEnabled);
        }
        let mut selected: Option<usize> = None;
        for (index, job) in self.jobs.iter().enumerate() {
            if !matches!(job.phase, TaskJobPhase::Ready | TaskJobPhase::Suspended) {
                continue;
            }
            let Some(held) = selected else {
                selected = Some(index);
                continue;
            };
            if scheduler_order(product.policy, &job.identity, &self.jobs[held].identity)?
                == Ordering::Less
            {
                selected = Some(index);
            }
        }
        let selected = selected.ok_or(TaskSchedulerError::ActionNotEnabled)?;
        let stored = &mut self.jobs[selected];
        let action = if stored.phase == TaskJobPhase::Suspended {
            TaskAction::Resume
        } else {
            TaskAction::Dispatch
        };
        stored.phase = TaskJobPhase::Running;
        let identity = action_identity(&stored.identity, action, None);
        self.running = Some(stored.identity.clone());
        Ok(identity)
    }

    /// Preempt the running job when a ready job has strictly higher
    /// fixed-priority rank. Dispatch remains a separate zero-time action.
    pub fn preempt(
        &mut self,
        product: &FixedWorkTaskProduct,
    ) -> Result<TaskTransitionIdentity, TaskSchedulerError> {
        let running = self
            .running
            .as_ref()
            .ok_or(TaskSchedulerError::ActionNotEnabled)?;
        let mut higher_ready = false;
        for job in self
            .jobs
            .iter()
            .filter(|job| job.phase == TaskJobPhase::Ready)
        {
            if scheduler_order(product.policy, &job.identity, running)? == Ordering::Less {
                higher_ready = true;
                break;
            }
        }
        if !higher_ready {
            return Err(TaskSchedulerError::ActionNotEnabled);
        }
        let selected = self
            .jobs
            .iter_mut()
            .find(|job| same_job(&job.identity, running))
            .ok_or(TaskSchedulerError::UnknownJob)?;
        selected.phase = TaskJobPhase::Suspended;
        let identity = action_identity(&selected.identity, TaskAction::Preempt, None);
        self.running = None;
        Ok(identity)
    }

    /// Complete the running job after the DBM guard has established that its
    /// fixed service demand has been reached.
    pub fn complete(
        &mut self,
        source_transition: Option<ModelTransition>,
    ) -> Result<TaskTransitionIdentity, TaskSchedulerError> {
        let running = self
            .running
            .clone()
            .ok_or(TaskSchedulerError::ActionNotEnabled)?;
        let index = self
            .jobs
            .iter()
            .position(|job| same_job(&job.identity, &running))
            .ok_or(TaskSchedulerError::UnknownJob)?;
        let completed = self.jobs.get(index).ok_or(TaskSchedulerError::UnknownJob)?;
        let next_generation = completed
            .identity
            .generation
            .checked_add(1)
            .ok_or(TaskSchedulerError::IdentityOverflow)?;
        let prior_generation = self.generations.iter().find(|row| {
            row.task_set == completed.identity.taskset
                && row.task == completed.identity.task
                && row.slot == completed.identity.slot
        });
        if prior_generation.is_some_and(|row| row.next.checked_add(1).is_none()) {
            return Err(TaskSchedulerError::IdentityOverflow);
        }
        let completed = self.jobs.remove(index);
        self.running = None;
        if let Some(row) = self.generations.iter_mut().find(|row| {
            row.task_set == completed.identity.taskset
                && row.task == completed.identity.task
                && row.slot == completed.identity.slot
        }) {
            row.next += 1;
        } else {
            self.generations.push(SlotGeneration {
                task_set: completed.identity.taskset.clone(),
                task: completed.identity.task.clone(),
                slot: completed.identity.slot,
                next: next_generation,
            });
        }
        Ok(action_identity(
            &completed.identity,
            TaskAction::Complete,
            source_transition,
        ))
    }

    /// Mark a live job missed at its nominal deadline. The search treats this
    /// state as bad; it is not a settled counterexample until FR-237/FR-364
    /// replay a divergent continuation.
    pub fn deadline_miss(
        &mut self,
        job: &TaskTransitionIdentity,
    ) -> Result<TaskTransitionIdentity, TaskSchedulerError> {
        let stored = self
            .jobs
            .iter_mut()
            .find(|candidate| same_job(&candidate.identity, job))
            .ok_or(TaskSchedulerError::UnknownJob)?;
        if stored.phase == TaskJobPhase::Missed {
            return Err(TaskSchedulerError::ActionNotEnabled);
        }
        stored.phase = TaskJobPhase::Missed;
        if self
            .running
            .as_ref()
            .is_some_and(|running| same_job(running, job))
        {
            self.running = None;
        }
        Ok(action_identity(
            &stored.identity,
            TaskAction::DeadlineMiss,
            None,
        ))
    }
}

fn same_job(left: &TaskTransitionIdentity, right: &TaskTransitionIdentity) -> bool {
    left.taskset == right.taskset
        && left.task == right.task
        && left.job == right.job
        && left.slot == right.slot
        && left.generation == right.generation
        && same_transition(&left.release, &right.release)
}

fn same_transition(left: &ModelTransition, right: &ModelTransition) -> bool {
    left.operation() == right.operation()
        && left.receiver() == right.receiver()
        && left.arguments() == right.arguments()
}

fn scheduler_order(
    policy: SchedulingPolicy,
    left: &TaskTransitionIdentity,
    right: &TaskTransitionIdentity,
) -> Result<Ordering, TaskSchedulerError> {
    let order = match policy {
        SchedulingPolicy::FixedPriority => left
            .priority
            .ok_or(TaskSchedulerError::ActionNotEnabled)?
            .cmp(&right.priority.ok_or(TaskSchedulerError::ActionNotEnabled)?),
        SchedulingPolicy::Edf => left
            .deadline
            .cmp(&right.deadline)
            .then_with(|| left.task.as_bytes().cmp(right.task.as_bytes()))
            .then_with(|| transition_order(&left.release, &right.release))
            .then_with(|| left.job.cmp(&right.job)),
        SchedulingPolicy::Amc => return Err(TaskSchedulerError::ActionNotEnabled),
    };
    Ok(order
        .then_with(|| left.task.as_bytes().cmp(right.task.as_bytes()))
        .then_with(|| left.job.cmp(&right.job))
        .then_with(|| left.slot.cmp(&right.slot)))
}

fn transition_order(left: &ModelTransition, right: &ModelTransition) -> Ordering {
    (left.operation(), left.receiver(), left.arguments()).cmp(&(
        right.operation(),
        right.receiver(),
        right.arguments(),
    ))
}

fn action_identity(
    job: &TaskTransitionIdentity,
    action: TaskAction,
    source_transition: Option<ModelTransition>,
) -> TaskTransitionIdentity {
    let mut identity = job.clone();
    identity.action = action;
    identity.source_transition = source_transition;
    identity
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
    /// The fixed-priority scheduler has duplicate priorities.
    PriorityClash,
    /// The caller supplied task data that does not satisfy FR-419 task-set
    /// premises. Checked graphs normally refuse these before this point.
    InvalidTaskSet,
    /// The policy is not a fixed-work policy supported by this exact route.
    UnsupportedPolicy,
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
    if policy == SchedulingPolicy::Amc {
        return Err(FixedWorkGateMiss::UnsupportedPolicy);
    }
    let mut tasks = Vec::with_capacity(task_set.tasks.len());
    let mut live_job_cap = 0_u64;
    let mut priorities = std::collections::BTreeSet::new();
    for task in &task_set.tasks {
        if task.deadline <= Rational::from_integer(Integer::zero())
            || task.jitter < Rational::from_integer(Integer::zero())
            || task.blocking < Rational::from_integer(Integer::zero())
        {
            return Err(FixedWorkGateMiss::InvalidTaskSet);
        }
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
        if interarrival <= Rational::from_integer(Integer::zero()) {
            return Err(FixedWorkGateMiss::InvalidTaskSet);
        }
        match policy {
            SchedulingPolicy::FixedPriority => {
                let priority = task.priority.ok_or(FixedWorkGateMiss::MissingPriority)?;
                if !priorities.insert(priority) {
                    return Err(FixedWorkGateMiss::PriorityClash);
                }
            }
            SchedulingPolicy::Edf if !task.jitter.is_zero() || !task.blocking.is_zero() => {
                return Err(FixedWorkGateMiss::InvalidTaskSet);
            }
            SchedulingPolicy::Edf => {}
            SchedulingPolicy::Amc => return Err(FixedWorkGateMiss::UnsupportedPolicy),
        }
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
    Ok(FixedWorkTaskProduct {
        task_set: task_set.name.clone(),
        policy,
        tasks,
        live_job_cap,
    })
}
