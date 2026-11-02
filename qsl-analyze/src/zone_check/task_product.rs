// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL FR-249 and QSpec FR-419: compile the exact fixed-work gate and the
//! finite task-slot bounds shared by EN-6 and the FR-418 reader.

use super::{Bound, Dbm, DbmError};
use num_bigint::BigInt;
use num_integer::Integer as NumInteger;
use qsl_eval::simulation::model::ModelSystem;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_replay::{
    TaskAction, TaskClockCeiling, TaskSubtraction, TaskTransitionIdentity, TaskTransitionTemplate,
};
use qsl_semantics::check::{
    Arrival, CheckedScheduleClaim, CheckedTaskSets, Criticality, Release, ScheduleClaim,
    SchedulingPolicy, Task, TaskSet, Wcet,
};
use quire_contract_model::{
    CheckedLiteralRead, CheckedLiteralValue, CheckedSchedulabilityClaimRef,
    CheckedSchedulabilityKind, CheckedSchedulingPolicy, CheckedTaskArrival, CheckedTaskCriticality,
};
use quire_exact::{Integer, NodeKey, Rational};
use quire_semantic_value::model_transition::ModelTransition;
use std::cmp::Ordering;
use std::str::FromStr;

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

/// The admitted task-automaton subject selected by one checked schedulability
/// claim and source model. The model timed system is carried by the EN-6
/// request; this value binds its checked task set, release declarations and
/// policy before search begins.
#[derive(Clone, Debug)]
pub struct TaskProductSubject {
    claim: CheckedScheduleClaim,
    occurrence: OccurrenceKey,
    task_set: TaskSet,
    releases: Vec<Release>,
    route: TaskProductRoute,
}

impl TaskProductSubject {
    /// Bind an IR-admitted schedulability claim to the QSL source release
    /// declarations whose operations belong to `system`. Task parameters come
    /// from the borrowed IR view; the QSL claim is used only to cross-check
    /// structural declaration identity and select the source release family.
    pub fn new_admitted<'a>(
        checked: &'a CheckedTaskSets,
        source_claim: &'a CheckedScheduleClaim,
        admitted: CheckedSchedulabilityClaimRef<'_>,
        system: &ModelSystem<'_>,
    ) -> Result<Self, TaskProductSubjectError> {
        if admitted.kind() != CheckedSchedulabilityKind::Schedulable {
            return Err(TaskProductSubjectError::NotSchedulabilityClaim);
        }
        let admitted_node = qsl_semantics::check::wire_id::node_key(admitted.declaration())
            .map_err(|_| TaskProductSubjectError::ClaimIdentityMismatch)?;
        let Some(source_node) = source_claim.node_id() else {
            return Err(TaskProductSubjectError::ClaimIdentityMismatch);
        };
        if &admitted_node != source_node
            || admitted.occurrence().role != quire_contract_model::CheckedOccurrenceRole::Claim
            || admitted.occurrence().ordinal != source_claim.occurrence
        {
            return Err(TaskProductSubjectError::ClaimIdentityMismatch);
        }
        if !matches!(source_claim.claim, ScheduleClaim::Schedulable(_)) {
            return Err(TaskProductSubjectError::NotSchedulabilityClaim);
        }
        let source_task_set = checked
            .task_set_of(source_claim)
            .ok_or(TaskProductSubjectError::MissingTaskSet)?;
        let task_set_unit = source_task_set
            .unit
            .clone()
            .ok_or(TaskProductSubjectError::MissingTaskSetUnit)?;
        let occurrence = source_claim
            .occurrence()
            .ok_or(TaskProductSubjectError::MissingClaimIdentity)?;
        let policy = match admitted.policy() {
            CheckedSchedulingPolicy::FixedPriority => SchedulingPolicy::FixedPriority,
            CheckedSchedulingPolicy::Edf => SchedulingPolicy::Edf,
            CheckedSchedulingPolicy::Amc => SchedulingPolicy::Amc,
        };
        if source_claim.policy() != policy {
            return Err(TaskProductSubjectError::ClaimIdentityMismatch);
        }
        let tasks = admitted
            .tasks()
            .map(|task| {
                let arrival = match task.arrival() {
                    CheckedTaskArrival::Periodic => Arrival::Periodic {
                        period: checked_quantity(task.period(), &task_set_unit.node)?,
                    },
                    CheckedTaskArrival::Sporadic => Arrival::Sporadic {
                        min_interarrival: checked_quantity(task.period(), &task_set_unit.node)?,
                    },
                };
                let criticality = match task.criticality() {
                    CheckedTaskCriticality::Lo => Criticality::Lo,
                    CheckedTaskCriticality::Hi => Criticality::Hi,
                };
                let wcet = checked_quantity(task.wcet(), &task_set_unit.node)?;
                let wcet_hi = task
                    .wcet_hi()
                    .map(|value| checked_quantity(value, &task_set_unit.node))
                    .transpose()?;
                let priority = task
                    .priority()
                    .map(checked_integer)
                    .transpose()?
                    .map(|value| value.to_string().parse::<u64>())
                    .transpose()
                    .map_err(|_| TaskProductSubjectError::InvalidTypedClaim)?;
                Ok(Task {
                    name: task.name().to_owned(),
                    arrival,
                    deadline: checked_quantity(task.deadline(), &task_set_unit.node)?,
                    bcet: checked_quantity(task.bcet(), &task_set_unit.node)?,
                    wcet: match wcet_hi {
                        Some(hi) => Wcet::Dual { lo: wcet, hi },
                        None => Wcet::Single(wcet),
                    },
                    priority,
                    criticality,
                    jitter: checked_quantity(task.jitter(), &task_set_unit.node)?,
                    blocking: checked_quantity(task.blocking(), &task_set_unit.node)?,
                })
            })
            .collect::<Result<Vec<_>, TaskProductSubjectError>>()?;
        let task_set = TaskSet {
            name: source_task_set.name.clone(),
            unit: Some(task_set_unit),
            tasks,
            span: source_task_set.span,
        };
        let claim = CheckedScheduleClaim {
            name: source_claim.name.clone(),
            task_set: source_claim.task_set.clone(),
            claim: ScheduleClaim::Schedulable(policy),
            node_id: Some(admitted_node),
            occurrence: admitted.occurrence().ordinal,
            span: source_claim.span,
        };
        let releases: Vec<_> = checked
            .releases()
            .iter()
            .filter(|release| {
                release.task_set() == source_claim.task_set
                    && system
                        .operation_text(release.operation().declaring(), release.operation().name())
                        .is_some()
            })
            .cloned()
            .collect();
        let admitted_route = task_product_route(&task_set, claim.policy())
            .map_err(TaskProductSubjectError::Admission)?;
        let route = if releases.is_empty() {
            TaskProductRoute::NoRelease
        } else {
            admitted_route
        };
        Ok(Self {
            claim,
            occurrence,
            task_set,
            releases,
            route,
        })
    }

    /// The checked schedulability claim binding the task product.
    pub fn claim(&self) -> &CheckedScheduleClaim {
        &self.claim
    }

    /// The checked-package claim occurrence selected by this task product.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// The checked task set selected by the claim.
    pub fn task_set(&self) -> &TaskSet {
        &self.task_set
    }

    /// The release declarations whose source operation is admitted by the
    /// timed model.
    pub fn releases(&self) -> &[Release] {
        &self.releases
    }

    /// Exact finite-product or admitted bounded-search route.
    pub fn route(&self) -> &TaskProductRoute {
        &self.route
    }
}

/// Why a checked claim could not be bound to a task-automaton subject.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TaskProductSubjectError {
    /// A task set name on the checked claim has no checked declaration.
    #[error("the schedulability claim has no checked task set")]
    MissingTaskSet,
    /// The selected task set has no admitted time unit.
    #[error("the admitted task set has no time unit")]
    MissingTaskSetUnit,
    /// A schedulability claim must name a checked schedulable task claim.
    #[error("the checked claim is not a schedulability claim")]
    NotSchedulabilityClaim,
    /// The checked claim has not been assigned its FR-482 node occurrence.
    #[error("the checked schedulability claim has no FR-482 node occurrence")]
    MissingClaimIdentity,
    /// The IR claim identity or policy does not match the source declaration.
    #[error("the IR-admitted claim does not match the selected QSL declaration")]
    ClaimIdentityMismatch,
    /// An IR-typed task value has an unexpected kind or unit.
    #[error("the IR-admitted task claim contains a value outside its checked unit and type")]
    InvalidTypedClaim,
    /// The task set violates FR-419's admission requirements.
    #[error("the selected task set violates FR-419 admission: {0}")]
    Admission(#[from] TaskProductAdmissionError),
}

fn checked_integer(value: CheckedLiteralRead<'_>) -> Result<Integer, TaskProductSubjectError> {
    let CheckedLiteralValue::Integer(value) = value.value else {
        return Err(TaskProductSubjectError::InvalidTypedClaim);
    };
    Integer::from_str(&value.to_string()).map_err(|_| TaskProductSubjectError::InvalidTypedClaim)
}

fn checked_quantity(
    value: CheckedLiteralRead<'_>,
    unit: &NodeKey,
) -> Result<Rational, TaskProductSubjectError> {
    let value_unit = qsl_semantics::check::wire_id::node_key(value.type_id)
        .map_err(|_| TaskProductSubjectError::InvalidTypedClaim)?;
    if &value_unit != unit {
        return Err(TaskProductSubjectError::InvalidTypedClaim);
    }
    let CheckedLiteralValue::Rational {
        numerator,
        denominator,
    } = value.value
    else {
        return Err(TaskProductSubjectError::InvalidTypedClaim);
    };
    let numerator = Integer::from_str(&numerator.to_string())
        .map_err(|_| TaskProductSubjectError::InvalidTypedClaim)?;
    let denominator = Integer::from_str(&denominator.to_string())
        .map_err(|_| TaskProductSubjectError::InvalidTypedClaim)?;
    Rational::new(numerator, denominator).map_err(|_| TaskProductSubjectError::InvalidTypedClaim)
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
    /// The finite zero-based rank among this task's live jobs, ordered by
    /// source activation transition identity then exact activation order.
    /// Completion compacts the remaining ranks; exact global occurrence and
    /// slot generation stay only on `identity` for replay.
    pub rank: u64,
    /// Current scheduler phase.
    pub phase: TaskJobPhase,
}

/// The DBM location and ceiling of one live job's service clock.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskServiceClock {
    /// The job whose accumulated service is represented.
    pub job: TaskTransitionIdentity,
    /// The clock index in the task-product DBM (zero is reserved).
    pub index: usize,
    /// The certificate's stable identity for this clock.
    pub name: String,
    /// The exact positive scaled ceiling for this clock.
    pub maximum: BigInt,
}

/// Canonical DBM positions reserved for the task-derived clocks. `base_dim`
/// includes the zero reference clock and every source-model clock.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TaskClockLayout {
    arrivals: Vec<TaskArrivalClock>,
    slots: Vec<TaskSlotClock>,
    ceilings: Vec<TaskClockCeiling>,
    dim: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TaskArrivalClock {
    taskset: String,
    task: String,
    index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TaskSlotClock {
    taskset: String,
    task: String,
    slot: u64,
    jitter: Option<usize>,
    service: usize,
    deadline: usize,
}

impl TaskClockLayout {
    /// Reserve arrival, pending-jitter, service and nominal-deadline clocks in
    /// FR-418's canonical order, with exact FR-415 integer ceilings.
    pub(super) fn new(
        product: &FixedWorkTaskProduct,
        base_dim: usize,
        scale: &BigInt,
    ) -> Result<Self, TaskClockLayoutError> {
        let mut next = base_dim;
        let mut arrivals = Vec::with_capacity(product.tasks.len());
        let mut ceilings = Vec::new();
        for task in &product.tasks {
            let index = next;
            next = next
                .checked_add(1)
                .ok_or(TaskClockLayoutError::DimensionOverflow)?;
            let maximum = scale_exact(&task.interarrival, scale)?;
            arrivals.push(TaskArrivalClock {
                taskset: task.task_set.clone(),
                task: task.task.clone(),
                index,
            });
            ceilings.push(TaskClockCeiling {
                clock: format!("task:{}:{}:arrival", task.task_set, task.task),
                maximum: Integer::from_big(maximum),
            });
        }
        let mut jitter = Vec::new();
        for task in &product.tasks {
            let count = usize::try_from(task.max_jobs)
                .map_err(|_| TaskClockLayoutError::DimensionOverflow)?;
            for slot in 0..count {
                let jitter_index = if task.max_jitter.is_zero() {
                    None
                } else {
                    let index = next;
                    next = next
                        .checked_add(1)
                        .ok_or(TaskClockLayoutError::DimensionOverflow)?;
                    let maximum = scale_exact(&task.max_jitter, scale)?;
                    ceilings.push(TaskClockCeiling {
                        clock: format!("task:{}:{}:slot{}:jitter", task.task_set, task.task, slot),
                        maximum: Integer::from_big(maximum),
                    });
                    Some(index)
                };
                jitter.push((
                    task,
                    u64::try_from(slot).map_err(|_| TaskClockLayoutError::DimensionOverflow)?,
                    jitter_index,
                ));
            }
        }
        let mut slots = Vec::with_capacity(jitter.len());
        for (task, slot, jitter_index) in jitter {
            let service = next;
            next = next
                .checked_add(1)
                .ok_or(TaskClockLayoutError::DimensionOverflow)?;
            let deadline = next;
            next = next
                .checked_add(1)
                .ok_or(TaskClockLayoutError::DimensionOverflow)?;
            let service_maximum = task.work.clone().max(task.relative_deadline.clone());
            ceilings.push(TaskClockCeiling {
                clock: format!("task:{}:{}:slot{}:service", task.task_set, task.task, slot),
                maximum: Integer::from_big(scale_exact(&service_maximum, scale)?),
            });
            ceilings.push(TaskClockCeiling {
                clock: format!("task:{}:{}:slot{}:deadline", task.task_set, task.task, slot),
                maximum: Integer::from_big(scale_exact(&task.relative_deadline, scale)?),
            });
            slots.push(TaskSlotClock {
                taskset: task.task_set.clone(),
                task: task.task.clone(),
                slot,
                jitter: jitter_index,
                service,
                deadline,
            });
        }
        Ok(Self {
            arrivals,
            slots,
            ceilings,
            dim: next,
        })
    }

    pub(super) fn dim(&self) -> usize {
        self.dim
    }

    pub(super) fn arrival_index(&self, taskset: &str, task: &str) -> Option<usize> {
        self.arrivals
            .iter()
            .find(|clock| clock.taskset == taskset && clock.task == task)
            .map(|clock| clock.index)
    }

    pub(super) fn service_clock(&self, job: &TaskTransitionIdentity) -> Option<TaskServiceClock> {
        let slot = self.slots.iter().find(|slot| {
            slot.taskset == job.taskset && slot.task == job.task && slot.slot == job.slot
        })?;
        let ceiling = self.ceilings.iter().find(|ceiling| {
            ceiling.clock == format!("task:{}:{}:slot{}:service", job.taskset, job.task, job.slot)
        })?;
        Some(TaskServiceClock {
            job: job.clone(),
            index: slot.service,
            name: ceiling.clock.clone(),
            maximum: ceiling.maximum.as_big().clone(),
        })
    }

    pub(super) fn jitter_index(&self, job: &TaskTransitionIdentity) -> Option<usize> {
        self.slots
            .iter()
            .find(|slot| {
                slot.taskset == job.taskset && slot.task == job.task && slot.slot == job.slot
            })
            .and_then(|slot| slot.jitter)
    }

    pub(super) fn deadline_index(&self, job: &TaskTransitionIdentity) -> Option<usize> {
        self.slots
            .iter()
            .find(|slot| {
                slot.taskset == job.taskset && slot.task == job.task && slot.slot == job.slot
            })
            .map(|slot| slot.deadline)
    }

    pub(super) fn ceilings(&self) -> &[TaskClockCeiling] {
        &self.ceilings
    }

    /// The DBM index for a task clock named in its certificate metadata.
    pub(super) fn clock_index(&self, base_dim: usize, name: &str) -> Option<usize> {
        self.ceilings
            .iter()
            .position(|ceiling| ceiling.clock == name)
            .and_then(|offset| base_dim.checked_add(offset))
    }
}

/// Why the task-clock layout could not be represented.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(super) enum TaskClockLayoutError {
    /// The finite task slots do not fit the target address space.
    #[error("the task clock dimension overflows")]
    DimensionOverflow,
    /// A common scale does not make a task clock ceiling integral.
    #[error("the common scale does not make a task clock ceiling integral")]
    InexactScale,
}

fn scale_exact(value: &Rational, scale: &BigInt) -> Result<BigInt, TaskClockLayoutError> {
    let scaled = value.mul(&Rational::from_integer(Integer::from_big(scale.clone())));
    if scaled.denominator() != &Integer::one() {
        return Err(TaskClockLayoutError::InexactScale);
    }
    Ok(scaled.numerator().as_big().clone())
}

/// Why a fixed-work completion could not update scheduler and DBM state.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TaskCompletionError {
    /// The scheduler transition was not enabled.
    #[error("task completion is not enabled: {0:?}")]
    Scheduler(TaskSchedulerError),
    /// The task service-clock update is malformed.
    #[error("task service clock update failed: {0}")]
    Dbm(DbmError),
    /// The declared scale does not make the fixed demand integral.
    #[error("task demand is not integral at the selected DBM scale")]
    InexactScale,
    /// A bounded-subtraction guard has no valuation in the input zone.
    #[error("task completion has no valuation satisfying its subtraction guards")]
    EmptySuccessor,
}

/// Per-task occurrence counter shared by every checked release binding.
#[derive(Clone, Debug, Eq, PartialEq)]
struct ActivationCounter {
    task_set: String,
    task: String,
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

/// A source transition whose checked release declarations still need their
/// zero-time derived activation steps.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingTaskRelease {
    /// The checked release declaration.
    pub release: Release,
    /// The exact source transition that enabled this activation.
    pub source: ModelTransition,
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
    pending: Vec<PendingTaskRelease>,
}

impl TaskSchedulerState {
    /// The live jobs in canonical task-set/task/slot order.
    pub fn jobs(&self) -> &[TaskJob] {
        &self.jobs
    }

    /// The currently running job, when the processor is occupied.
    pub fn running(&self) -> Option<&TaskTransitionIdentity> {
        self.running.as_ref()
    }

    /// The finite rank of a live job among its task's activation order.
    pub(super) fn job_rank(&self, identity: &TaskTransitionIdentity) -> Option<u64> {
        self.jobs
            .iter()
            .find(|job| same_job(&job.identity, identity))
            .map(|job| job.rank)
    }

    /// Complete the running fixed-work job and subtract its exact work from
    /// every suspended job's unit-rate service clock. The operation is
    /// transactional across the scheduler state and DBM: a disabled guard
    /// leaves both unchanged.
    pub fn complete_with_subtractions(
        &mut self,
        product: &FixedWorkTaskProduct,
        zone: &mut Dbm,
        scale: &BigInt,
        service_clocks: &[TaskServiceClock],
        source_transition: Option<ModelTransition>,
    ) -> Result<(TaskTransitionIdentity, Vec<TaskSubtraction>), TaskCompletionError> {
        if scale <= &BigInt::from(0) {
            return Err(TaskCompletionError::InexactScale);
        }
        let running = self.running.clone().ok_or(TaskCompletionError::Scheduler(
            TaskSchedulerError::ActionNotEnabled,
        ))?;
        let running_rank = self
            .job_rank(&running)
            .ok_or(TaskCompletionError::Scheduler(
                TaskSchedulerError::UnknownJob,
            ))?;
        let work = product
            .tasks
            .iter()
            .find(|task| task.task_set == running.taskset && task.task == running.task)
            .map(|task| task.work.clone())
            .ok_or(TaskCompletionError::Scheduler(
                TaskSchedulerError::ActionNotEnabled,
            ))?;
        let scaled = work.mul(&Rational::from_integer(Integer::from_big(scale.clone())));
        if scaled.denominator() != &Integer::one() {
            return Err(TaskCompletionError::InexactScale);
        }
        let amount = scaled.numerator().as_big().clone();

        let mut next_zone = zone.clone();
        let running_clock = service_clocks
            .iter()
            .find(|clock| same_job(&clock.job, &running))
            .ok_or(TaskCompletionError::Scheduler(
                TaskSchedulerError::ActionNotEnabled,
            ))?;
        next_zone
            .constrain(0, running_clock.index, Bound::le(-&amount))
            .and_then(|()| {
                next_zone.constrain(
                    running_clock.index,
                    0,
                    Bound::le(running_clock.maximum.clone()),
                )
            })
            .map_err(TaskCompletionError::Dbm)?;
        if next_zone.is_empty() {
            return Err(TaskCompletionError::EmptySuccessor);
        }
        let mut subtractions = Vec::new();
        for job in self
            .jobs
            .iter()
            .filter(|job| job.phase == TaskJobPhase::Suspended)
        {
            let clock = service_clocks
                .iter()
                .find(|clock| same_job(&clock.job, &job.identity))
                .ok_or(TaskCompletionError::Scheduler(
                    TaskSchedulerError::ActionNotEnabled,
                ))?;
            next_zone
                .subtract_bounded(clock.index, &amount, &clock.maximum)
                .map_err(TaskCompletionError::Dbm)?;
            if next_zone.is_empty() {
                return Err(TaskCompletionError::EmptySuccessor);
            }
        }

        let mut next_scheduler = self.clone();
        let completion = next_scheduler
            .complete(source_transition)
            .map_err(TaskCompletionError::Scheduler)?;
        let template = task_transition_template(&completion, running_rank);
        next_zone
            .reset(running_clock.index, &BigInt::from(0))
            .map_err(TaskCompletionError::Dbm)?;
        for job in self
            .jobs
            .iter()
            .filter(|job| job.phase == TaskJobPhase::Suspended)
        {
            let clock = service_clocks
                .iter()
                .find(|clock| same_job(&clock.job, &job.identity))
                .ok_or(TaskCompletionError::Scheduler(
                    TaskSchedulerError::ActionNotEnabled,
                ))?;
            subtractions.push(TaskSubtraction {
                transition: template.clone(),
                clock: clock.name.clone(),
                constant: Integer::from_big(amount.clone()),
            });
        }
        *self = next_scheduler;
        *zone = next_zone;
        Ok((completion, subtractions))
    }
}

/// Why a scheduler control operation could not be applied.
#[derive(Clone, Debug, Eq, PartialEq)]
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
    /// The common DBM scale does not make the task interarrival exact.
    InexactArrivalScale,
    /// The task arrival-clock guard or reset failed.
    ArrivalZone(DbmError),
    /// A source transition exceeded the canonical identity encoding bound.
    IdentityEncoding,
}

impl TaskSchedulerState {
    /// The scheduler's finite discrete key.
    ///
    /// Stable byte grammar, all integers big-endian:
    ///
    /// - `qsl-task-control-v1\0`;
    /// - a u64 job count, then each job sorted by `(taskset, task, rank)`:
    ///   length-prefixed UTF-8 taskset and task, u64 live job rank,
    ///   u64 slot, length-prefixed
    ///   canonical `ModelTransition` bytes (limits `1 << 20`), phase byte
    ///   (`Pending=0`, `Ready=1`, `Running=2`, `Suspended=3`, `Missed=4`),
    ///   length-prefixed UTF-8 exact-demand text, and priority tag (`0` for
    ///   absent, `1` followed by u64 for present);
    /// - running tag (`0` absent, `1` then taskset, task, slot, and the
    ///   length-prefixed canonical release transition bytes);
    /// - a u64 activated-task count, then sorted taskset/task pairs and byte
    ///   `1` for each established arrival phase;
    /// - a u64 pending-release count, then pending declaration-order rows of
    ///   taskset, task, and length-prefixed canonical source transition bytes.
    ///
    /// Each string or byte sequence is encoded as u64 byte length then bytes.
    /// Jitter and deadline values live in the task-clock DBM, not this key.
    /// Occurrence and slot generation are concrete provenance and are
    /// intentionally normalized from the finite control key.
    pub fn canonical_control_key(&self) -> Result<Vec<u8>, TaskSchedulerError> {
        let mut key = b"qsl-task-control-v1\0".to_vec();
        put_count(&mut key, self.jobs.len());
        let mut jobs: Vec<_> = self.jobs.iter().collect();
        jobs.sort_by(|left, right| {
            (&left.identity.taskset, &left.identity.task, left.rank).cmp(&(
                &right.identity.taskset,
                &right.identity.task,
                right.rank,
            ))
        });
        for job in jobs {
            let identity = &job.identity;
            put_text(&mut key, &identity.taskset);
            put_text(&mut key, &identity.task);
            key.extend_from_slice(&job.rank.to_be_bytes());
            key.extend_from_slice(&identity.slot.to_be_bytes());
            let transition = identity
                .release
                .canonical_bytes(quire_canonical::Limits::new(1 << 20))
                .map_err(|_| TaskSchedulerError::IdentityEncoding)?;
            put_bytes(&mut key, &transition);
            key.push(match job.phase {
                TaskJobPhase::Pending => 0,
                TaskJobPhase::Ready => 1,
                TaskJobPhase::Running => 2,
                TaskJobPhase::Suspended => 3,
                TaskJobPhase::Missed => 4,
            });
            put_text(&mut key, &identity.demand.to_string());
            match identity.priority {
                Some(priority) => {
                    key.push(1);
                    key.extend_from_slice(&priority.to_be_bytes());
                }
                None => key.push(0),
            }
        }
        match &self.running {
            Some(running) => {
                key.push(1);
                put_text(&mut key, &running.taskset);
                put_text(&mut key, &running.task);
                key.extend_from_slice(&running.slot.to_be_bytes());
                let transition = running
                    .release
                    .canonical_bytes(quire_canonical::Limits::new(1 << 20))
                    .map_err(|_| TaskSchedulerError::IdentityEncoding)?;
                put_bytes(&mut key, &transition);
            }
            None => key.push(0),
        }
        let mut activations: Vec<_> = self.activations.iter().collect();
        activations.sort_by(|left, right| {
            (&left.task_set, &left.task).cmp(&(&right.task_set, &right.task))
        });
        put_count(&mut key, activations.len());
        for row in activations {
            put_text(&mut key, &row.task_set);
            put_text(&mut key, &row.task);
            key.push(1);
        }
        put_count(&mut key, self.pending.len());
        for pending in &self.pending {
            put_text(&mut key, pending.release.task_set());
            put_text(&mut key, pending.release.task());
            let transition = pending
                .source
                .canonical_bytes(quire_canonical::Limits::new(1 << 20))
                .map_err(|_| TaskSchedulerError::IdentityEncoding)?;
            put_bytes(&mut key, &transition);
        }
        Ok(key)
    }

    /// Queue every checked release bound to one source transition, preserving
    /// checked declaration order. The transition is performed once by the
    /// source model; subsequent task activations are distinct zero-time edges.
    pub fn enqueue_releases(
        &mut self,
        releases: impl IntoIterator<Item = Release>,
        source: ModelTransition,
    ) {
        self.pending
            .extend(releases.into_iter().map(|release| PendingTaskRelease {
                release,
                source: source.clone(),
            }));
    }

    /// The pending checked source-release activations in declaration order.
    pub fn pending_releases(&self) -> &[PendingTaskRelease] {
        &self.pending
    }

    /// Remove one pending activation after its derived activation edge is
    /// successfully constructed.
    pub fn remove_pending_release(&mut self, index: usize) -> Option<PendingTaskRelease> {
        (index < self.pending.len()).then(|| self.pending.remove(index))
    }

    /// Apply the checked task's shared periodic or sporadic arrival rule to
    /// its unit-rate arrival clock, then reset that clock for the new
    /// activation. The first activation establishes the phase. All release
    /// bindings for a task consult the same counter, so changing source
    /// operation cannot bypass the interarrival constraint.
    pub fn apply_arrival_guard(
        &self,
        product: &FixedWorkTaskProduct,
        task_name: &str,
        clock: usize,
        scale: &BigInt,
        zone: &mut Dbm,
    ) -> Result<bool, TaskSchedulerError> {
        let mut successor = zone.clone();
        if !self.constrain_arrival_guard(product, task_name, clock, scale, &mut successor)? {
            return Ok(false);
        }
        successor
            .assign_clocks(&[], &[(clock, BigInt::from(0))])
            .map_err(TaskSchedulerError::ArrivalZone)?;
        *zone = successor;
        Ok(true)
    }

    /// Intersect `zone` with the shared inter-arrival rule without resetting
    /// the arrival clock. A source release edge uses this guard before the
    /// operation; its following zero-time activation resets the clock.
    pub fn constrain_arrival_guard(
        &self,
        product: &FixedWorkTaskProduct,
        task_name: &str,
        clock: usize,
        scale: &BigInt,
        zone: &mut Dbm,
    ) -> Result<bool, TaskSchedulerError> {
        if scale <= &BigInt::from(0) {
            return Err(TaskSchedulerError::InexactArrivalScale);
        }
        let task = product
            .tasks
            .iter()
            .find(|task| task.task == task_name)
            .ok_or(TaskSchedulerError::ActionNotEnabled)?;
        let scaled = task
            .interarrival
            .mul(&Rational::from_integer(Integer::from_big(scale.clone())));
        if scaled.denominator() != &Integer::one() {
            return Err(TaskSchedulerError::InexactArrivalScale);
        }
        let interarrival = scaled.numerator().as_big().clone();
        let has_activated = self
            .activations
            .iter()
            .any(|row| row.task_set == task.task_set && row.task == task.task);
        let mut successor = zone.clone();
        if has_activated {
            successor
                .constrain(0, clock, Bound::le(-&interarrival))
                .map_err(TaskSchedulerError::ArrivalZone)?;
            if task.arrival == FixedWorkArrival::Periodic {
                successor
                    .constrain(clock, 0, Bound::le(interarrival))
                    .map_err(TaskSchedulerError::ArrivalZone)?;
            }
        }
        if successor.is_empty() {
            return Ok(false);
        }
        *zone = successor;
        Ok(true)
    }

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
        let activation = match self
            .activations
            .iter()
            .find(|row| row.task_set == task.task_set && row.task == task.task)
        {
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
        if let Some(row) = self
            .activations
            .iter_mut()
            .find(|row| row.task_set == task.task_set && row.task == task.task)
        {
            row.next = row
                .next
                .checked_add(1)
                .ok_or(TaskSchedulerError::IdentityOverflow)?;
        } else {
            self.activations.push(ActivationCounter {
                task_set: task.task_set.clone(),
                task: task.task.clone(),
                next: 1,
            });
        }
        let rank = u64::try_from(live_count).map_err(|_| TaskSchedulerError::IdentityOverflow)?;
        self.jobs.push(TaskJob {
            identity: identity.clone(),
            rank,
            phase: if jitter.is_zero() {
                TaskJobPhase::Ready
            } else {
                TaskJobPhase::Pending
            },
        });
        self.jobs.sort_by(task_job_order);
        self.normalize_job_ranks()?;
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
            if scheduler_order(product.policy, job, &self.jobs[held])? == Ordering::Less {
                selected = Some(index);
            }
        }
        let selected = selected.ok_or(TaskSchedulerError::ActionNotEnabled)?;
        let identity = self.jobs[selected].identity.clone();
        self.dispatch_selected(&identity)
    }

    /// Dispatch `identity`, after the caller has selected it by the
    /// scheduler's current policy region.
    pub(super) fn dispatch_selected(
        &mut self,
        identity: &TaskTransitionIdentity,
    ) -> Result<TaskTransitionIdentity, TaskSchedulerError> {
        if self.running.is_some() {
            return Err(TaskSchedulerError::ActionNotEnabled);
        }
        let selected = self
            .jobs
            .iter()
            .position(|job| {
                same_job(&job.identity, identity)
                    && matches!(job.phase, TaskJobPhase::Ready | TaskJobPhase::Suspended)
            })
            .ok_or(TaskSchedulerError::ActionNotEnabled)?;
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
        let running_job = self
            .jobs
            .iter()
            .find(|job| same_job(&job.identity, running))
            .ok_or(TaskSchedulerError::UnknownJob)?;
        let mut higher_ready = false;
        for job in self
            .jobs
            .iter()
            .filter(|job| job.phase == TaskJobPhase::Ready)
        {
            if scheduler_order(product.policy, job, running_job)? == Ordering::Less {
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

    /// Preempt the running job because `ready` has been proved by the caller's
    /// policy-region guard to precede it in the scheduling order.
    pub(super) fn preempt_for(
        &mut self,
        ready: &TaskTransitionIdentity,
    ) -> Result<TaskTransitionIdentity, TaskSchedulerError> {
        let running = self
            .running
            .as_ref()
            .ok_or(TaskSchedulerError::ActionNotEnabled)?;
        if !self
            .jobs
            .iter()
            .any(|job| same_job(&job.identity, ready) && job.phase == TaskJobPhase::Ready)
        {
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
        self.normalize_job_ranks()?;
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

    /// Record the deadline-miss action and retire its live slot. The exact
    /// miss identity remains on the typed transition edge for replay; freeing
    /// the slot lets later releases continue and makes a post-miss lasso a
    /// genuine cycle of scheduler control rather than a growing miss log.
    pub fn deadline_miss(
        &mut self,
        job: &TaskTransitionIdentity,
    ) -> Result<TaskTransitionIdentity, TaskSchedulerError> {
        let index = self
            .jobs
            .iter()
            .position(|candidate| same_job(&candidate.identity, job))
            .ok_or(TaskSchedulerError::UnknownJob)?;
        let stored = self.jobs.get(index).ok_or(TaskSchedulerError::UnknownJob)?;
        if stored.phase == TaskJobPhase::Missed {
            return Err(TaskSchedulerError::ActionNotEnabled);
        }
        let next_generation = stored
            .identity
            .generation
            .checked_add(1)
            .ok_or(TaskSchedulerError::IdentityOverflow)?;
        if self.generations.iter().any(|row| {
            row.task_set == stored.identity.taskset
                && row.task == stored.identity.task
                && row.slot == stored.identity.slot
                && row.next.checked_add(1).is_none()
        }) {
            return Err(TaskSchedulerError::IdentityOverflow);
        }
        let missed = self.jobs.remove(index);
        self.normalize_job_ranks()?;
        if self
            .running
            .as_ref()
            .is_some_and(|running| same_job(running, job))
        {
            self.running = None;
        }
        if let Some(row) = self.generations.iter_mut().find(|row| {
            row.task_set == missed.identity.taskset
                && row.task == missed.identity.task
                && row.slot == missed.identity.slot
        }) {
            row.next += 1;
        } else {
            self.generations.push(SlotGeneration {
                task_set: missed.identity.taskset.clone(),
                task: missed.identity.task.clone(),
                slot: missed.identity.slot,
                next: next_generation,
            });
        }
        Ok(action_identity(
            &missed.identity,
            TaskAction::DeadlineMiss,
            None,
        ))
    }

    fn normalize_job_ranks(&mut self) -> Result<(), TaskSchedulerError> {
        self.jobs.sort_by(task_job_order);
        let mut current_task: Option<(&str, &str)> = None;
        let mut rank = 0_u64;
        for job in &mut self.jobs {
            let task = (job.identity.taskset.as_str(), job.identity.task.as_str());
            if current_task != Some(task) {
                current_task = Some(task);
                rank = 0;
            }
            job.rank = rank;
            rank = rank
                .checked_add(1)
                .ok_or(TaskSchedulerError::IdentityOverflow)?;
        }
        Ok(())
    }
}

/// Frame the complete source model wire key with the finite task scheduler
/// key for FR-418's single `CertNode.state_key` field.
fn put_count(target: &mut Vec<u8>, count: usize) {
    target.extend_from_slice(&u64::try_from(count).unwrap_or(u64::MAX).to_be_bytes());
}

fn put_text(target: &mut Vec<u8>, text: &str) {
    put_bytes(target, text.as_bytes());
}

fn put_bytes(target: &mut Vec<u8>, bytes: &[u8]) {
    put_count(target, bytes.len());
    target.extend_from_slice(bytes);
}

fn same_job(left: &TaskTransitionIdentity, right: &TaskTransitionIdentity) -> bool {
    left.taskset == right.taskset
        && left.task == right.task
        && left.job == right.job
        && left.slot == right.slot
        && left.generation == right.generation
        && same_transition(&left.release, &right.release)
}

fn task_job_order(left: &TaskJob, right: &TaskJob) -> Ordering {
    left.identity
        .taskset
        .cmp(&right.identity.taskset)
        .then_with(|| left.identity.task.cmp(&right.identity.task))
        .then_with(|| transition_order(&left.identity.release, &right.identity.release))
        .then_with(|| left.identity.job.cmp(&right.identity.job))
        .then_with(|| left.identity.slot.cmp(&right.identity.slot))
}

pub(super) fn edf_tie_order(
    left: &TaskTransitionIdentity,
    right: &TaskTransitionIdentity,
) -> Ordering {
    left.task
        .cmp(&right.task)
        .then_with(|| transition_order(&left.release, &right.release))
        .then_with(|| left.job.cmp(&right.job))
}

pub(super) fn task_transition_template(
    identity: &TaskTransitionIdentity,
    job_rank: u64,
) -> TaskTransitionTemplate {
    TaskTransitionTemplate {
        action: identity.action,
        taskset: identity.taskset.clone(),
        task: identity.task.clone(),
        job_rank,
        slot: identity.slot,
        release: identity.release.clone(),
        source_transition: identity.source_transition.clone(),
        priority: identity.priority,
    }
}

fn same_transition(left: &ModelTransition, right: &ModelTransition) -> bool {
    left.operation() == right.operation()
        && left.receiver() == right.receiver()
        && left.arguments() == right.arguments()
}

fn scheduler_order(
    policy: SchedulingPolicy,
    left: &TaskJob,
    right: &TaskJob,
) -> Result<Ordering, TaskSchedulerError> {
    let left_identity = &left.identity;
    let right_identity = &right.identity;
    let order = match policy {
        SchedulingPolicy::FixedPriority => left_identity
            .priority
            .ok_or(TaskSchedulerError::ActionNotEnabled)?
            .cmp(
                &right_identity
                    .priority
                    .ok_or(TaskSchedulerError::ActionNotEnabled)?,
            ),
        SchedulingPolicy::Edf => left_identity
            .deadline
            .cmp(&right_identity.deadline)
            .then_with(|| {
                left_identity
                    .task
                    .as_bytes()
                    .cmp(right_identity.task.as_bytes())
            })
            .then_with(|| transition_order(&left_identity.release, &right_identity.release))
            .then_with(|| left.rank.cmp(&right.rank)),
        SchedulingPolicy::Amc => return Err(TaskSchedulerError::ActionNotEnabled),
    };
    Ok(order
        .then_with(|| {
            left_identity
                .task
                .as_bytes()
                .cmp(right_identity.task.as_bytes())
        })
        .then_with(|| left.rank.cmp(&right.rank))
        .then_with(|| left_identity.slot.cmp(&right_identity.slot)))
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
    /// Fixed-priority blocking is not modeled by this exact task product;
    /// the admitted task remains eligible for bounded exploration.
    BoundedSearchRequired,
    /// A task's candidate slot count or the total cap cannot be represented
    /// by FR-418's bounded count.
    SlotCapOverflow,
}

/// Why an admitted task product uses FR-249's bounded-search route.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskSearchReason {
    /// Actual demand is not a singleton.
    VariableDemand,
    /// The current exact certificate carrier cannot encode this valid task
    /// product's blocking or concrete selected-jitter identity finitely.
    BoundedProductData,
    /// The admitted scheduler policy has no current exact task-product gate.
    PolicyOutsideExactGate,
    /// A candidate finite slot cap cannot be represented.
    SlotCapOverflow,
}

/// The route selected after applying FR-419 admission and the FR-418 exact
/// finite-product gate. Bounded is an admitted analysis route, never an
/// unsupported outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskProductRoute {
    /// The checked task set has no release binding in the selected source
    /// model, so only the complete source graph must be closed.
    NoRelease,
    /// The product may be submitted for exact finite closure.
    Exact(FixedWorkTaskProduct),
    /// The product remains admitted and must use replayable bounded search.
    Bounded(TaskSearchReason),
}

/// Why the task product fails FR-419's explicit task-set admission rules.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TaskProductAdmissionError {
    /// The task's checked data or policy-specific restrictions are invalid.
    #[error("the checked task set violates FR-419 admission rules")]
    InvalidTaskSet,
    /// Fixed-priority claims require a priority on every task.
    #[error("fixed-priority scheduling requires a priority for each task")]
    MissingPriority,
    /// Fixed-priority priorities must be distinct.
    #[error("fixed-priority task priorities must be distinct")]
    PriorityClash,
}

/// Select exact closure or admitted bounded search for a checked task set.
/// Only FR-419 admission failures are returned as errors; every other exact
/// gate miss remains an analyzable bounded task product.
pub fn task_product_route(
    task_set: &TaskSet,
    policy: SchedulingPolicy,
) -> Result<TaskProductRoute, TaskProductAdmissionError> {
    match fixed_work_task_product(task_set, policy) {
        Ok(product) => Ok(TaskProductRoute::Exact(product)),
        Err(FixedWorkGateMiss::VariableDemand) => {
            Ok(TaskProductRoute::Bounded(TaskSearchReason::VariableDemand))
        }
        Err(FixedWorkGateMiss::BoundedSearchRequired) => Ok(TaskProductRoute::Bounded(
            TaskSearchReason::BoundedProductData,
        )),
        Err(FixedWorkGateMiss::UnsupportedPolicy) => Ok(TaskProductRoute::Bounded(
            TaskSearchReason::PolicyOutsideExactGate,
        )),
        Err(FixedWorkGateMiss::SlotCapOverflow) => {
            Ok(TaskProductRoute::Bounded(TaskSearchReason::SlotCapOverflow))
        }
        Err(FixedWorkGateMiss::InvalidTaskSet) => Err(TaskProductAdmissionError::InvalidTaskSet),
        Err(FixedWorkGateMiss::MissingPriority) => Err(TaskProductAdmissionError::MissingPriority),
        Err(FixedWorkGateMiss::PriorityClash) => Err(TaskProductAdmissionError::PriorityClash),
    }
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
                // Fixed-priority jitter is admitted by FR-419, but the
                // current typed certificate edge identifies its selected
                // jitter by a concrete Rational. Keep this valid behavior on
                // bounded search until closure has a symbolic region label.
                if !task.jitter.is_zero() {
                    return Err(FixedWorkGateMiss::BoundedSearchRequired);
                }
                if !task.blocking.is_zero() {
                    return Err(FixedWorkGateMiss::BoundedSearchRequired);
                }
                let priority = task.priority.ok_or(FixedWorkGateMiss::MissingPriority)?;
                if !priorities.insert(priority) {
                    return Err(FixedWorkGateMiss::PriorityClash);
                }
            }
            // FR-419-AC-10: EDF task sets with jitter or blocking are invalid.
            SchedulingPolicy::Edf if !task.blocking.is_zero() || !task.jitter.is_zero() => {
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
