// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-418's task-product certificate member.

use filament_exact::{Integer, NonZeroInteger, Rational};
use filament_semantic_value::model_transition::ModelTransition;

mod key;
mod wire;

pub use key::{
    TaskArrivalPhase, TaskJobPhase, TaskLiveJob, TaskPendingBudgetChange, TaskProductKey,
    TaskProductKeyRefusal, TaskSchedulerControl,
};
pub(crate) use wire::{array, child, count, integer, member, object, refuse, source_transition, source_transition_wire};
pub use wire::{
    same_source_transition, TaskProductShapeReason, TaskProductShapeRefusal, TaskProductWireError,
};

/// A checked arrival rule copied into the FR-418 task-product certificate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskArrivalKind {
    /// A release is enabled exactly at every period.
    Periodic,
    /// Releases are separated by at least the stated interarrival time.
    Sporadic,
}

impl TaskArrivalKind {
    /// The exact FSpec FR-418 JSON spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Periodic => "periodic",
            Self::Sporadic => "sporadic",
        }
    }
}

/// The queue and work bounds for one checked task (FSpec FR-418).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskQueueBound {
    /// The task-set name.
    pub task_set: String,
    /// The task name.
    pub task: String,
    /// The declared arrival kind.
    pub arrival: TaskArrivalKind,
    /// The enforced minimum interarrival time.
    pub interarrival: Rational,
    /// The nominal relative deadline.
    pub relative_deadline: Rational,
    /// The maximum release jitter.
    pub max_jitter: Rational,
    /// The declared lower work bound.
    pub work_lower: Rational,
    /// The declared upper work bound.
    pub work_upper: Rational,
    /// The checked finite candidate slot bound.
    pub max_jobs: u64,
}

impl TaskQueueBound {
    /// FR-419's candidate per-task finite slot bound, `ceil(D / T) + 1`.
    ///
    /// The source reader still has to verify this against the checked task
    /// declaration. A missing result means the quantities are non-positive
    /// or the exact bound does not fit the wire's `u64` count.
    pub fn candidate_max_jobs(
        interarrival: &Rational,
        relative_deadline: &Rational,
    ) -> Option<u64> {
        let zero = Rational::from_integer(Integer::zero());
        if interarrival <= &zero || relative_deadline <= &zero {
            return None;
        }
        let periods = relative_deadline.div(interarrival)?;
        let denominator = NonZeroInteger::try_from(periods.denominator()).ok()?;
        let (floor, remainder) = periods.numerator().div_mod_floor(denominator);
        let ceiling = if remainder.is_zero() {
            floor
        } else {
            floor.add(&Integer::one())
        };
        ceiling.to_u64()?.checked_add(1)
    }

    /// Builds the FR-418 queue rows for one S3-checked task set, in its task
    /// source order. Every row is derived from the checked declaration; the
    /// certificate reader independently recomputes it before acceptance.
    pub fn from_checked_task_set(task_set: &fsl_semantics::check::TaskSet) -> Option<Vec<Self>> {
        task_set
            .tasks
            .iter()
            .map(|task| {
                let (arrival, interarrival) = match &task.arrival {
                    fsl_semantics::check::Arrival::Periodic { period } => {
                        (TaskArrivalKind::Periodic, period.clone())
                    }
                    fsl_semantics::check::Arrival::Sporadic { min_interarrival } => {
                        (TaskArrivalKind::Sporadic, min_interarrival.clone())
                    }
                };
                let work_upper = match (&task.wcet, task.criticality) {
                    (
                        fsl_semantics::check::Wcet::Single(wcet),
                        fsl_semantics::check::Criticality::Lo,
                    ) => wcet.clone(),
                    (
                        fsl_semantics::check::Wcet::Dual { hi, .. },
                        fsl_semantics::check::Criticality::Hi,
                    ) => hi.clone(),
                    _ => return None,
                };
                let max_jobs = Self::candidate_max_jobs(&interarrival, &task.deadline)?;
                Some(Self {
                    task_set: task_set.name.clone(),
                    task: task.name.clone(),
                    arrival,
                    interarrival,
                    relative_deadline: task.deadline.clone(),
                    max_jitter: task.jitter.clone(),
                    work_lower: task.bcet.clone(),
                    work_upper,
                    max_jobs,
                })
            })
            .collect()
    }
}

/// One scheduler-clock ceiling in FR-418's certificate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskClockCeiling {
    /// The auxiliary clock name.
    pub clock: String,
    /// Its positive ceiling in scaled clock units, of any size (FR-418
    /// writes it as an arbitrary-precision integer string).
    pub maximum: Integer,
}

/// The comparison used by an FR-418 diagonal constraint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskDiagonalOperator {
    /// `<`.
    Less,
    /// `<=`.
    LessOrEqual,
    /// `>`.
    Greater,
    /// `>=`.
    GreaterOrEqual,
}

impl TaskDiagonalOperator {
    /// The exact FSpec FR-418 JSON spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Less => "<",
            Self::LessOrEqual => "<=",
            Self::Greater => ">",
            Self::GreaterOrEqual => ">=",
        }
    }
}

/// One FSpec FR-418 diagonal constraint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskDiagonalConstraint {
    /// The left auxiliary clock.
    pub left: String,
    /// Its comparison with `right`.
    pub operator: TaskDiagonalOperator,
    /// The right auxiliary clock.
    pub right: String,
    /// The scaled integer constant.
    pub constant: Integer,
}

/// The finite identity used on task-product certificate edges and
/// bounded-subtraction rows (FSpec FR-418). Exact job occurrence and
/// generation remain on [`super::super::TaskTransitionIdentity`] in replay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskTransitionTemplate {
    /// The scheduler action.
    pub action: super::super::TaskTransitionAction,
    /// The task-set name.
    pub task_set: String,
    /// The task name.
    pub task: String,
    /// Canonical rank among this task's live jobs.
    pub job_rank: u64,
    /// The finite slot bound to `job_rank`.
    pub slot: u64,
    /// The source release operation identity.
    pub release: ModelTransition,
    /// The source operation enabling this task action, absent for autonomous
    /// scheduler actions.
    pub source_transition: Option<ModelTransition>,
    /// The declared priority, omitted for tasks without one.
    pub priority: Option<u64>,
}

/// One bounded-subtraction update in the exact finite abstraction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskSubtraction {
    /// The finite task scheduler edge.
    pub transition: TaskTransitionTemplate,
    /// The clock place updated.
    pub clock: String,
    /// The strictly positive scaled amount, of any size.
    pub constant: Integer,
}

/// The independently checked closure claim the producer supplies for
/// FR-418 to rederive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskClosureMethod {
    /// Complete finite source-plus-scheduler product.
    ClosedFiniteProduct,
    /// Complete source graph with no reachable task-release binding.
    ClosedNoReleaseProduct,
}

impl TaskClosureMethod {
    /// The exact FSpec FR-418 JSON spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClosedFiniteProduct => "closed-finite-product/v1",
            Self::ClosedNoReleaseProduct => "closed-no-release-product/v1",
        }
    }
}

/// The closure check the certificate reader must recompute (FSpec FR-418).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskClosureEvidence {
    /// Recompute every reachable product successor.
    RecomputeAllReachableSuccessors,
    /// Recompute the closed source graph and prove no task release is reachable.
    RecomputeAllSourceSuccessorsNoTaskRelease,
}

impl TaskClosureEvidence {
    /// The exact FSpec FR-418 JSON spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RecomputeAllReachableSuccessors => "recompute-all-reachable-successors/v1",
            Self::RecomputeAllSourceSuccessorsNoTaskRelease => {
                "recompute-all-source-successors-no-task-release/v1"
            }
        }
    }
}

/// FR-418's typed task-product certificate member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskProductCertificate {
    /// The FSpec FR-415 bounded subtraction abstraction discriminator.
    pub abstraction: &'static str,
    /// The task-specific clock ceilings.
    pub clock_ceilings: Vec<TaskClockCeiling>,
    /// The auxiliary diagonal constraints.
    pub diagonal_constraints: Vec<TaskDiagonalConstraint>,
    /// Checked task queue bounds, in source order.
    pub queue_bounds: Vec<TaskQueueBound>,
    /// Bounded subtraction updates, in canonical transition order.
    pub subtractions: Vec<TaskSubtraction>,
    /// The closure method and declared live-job cap, which FR-418 recomputes.
    pub time_progress: TaskTimeProgress,
}

impl TaskProductCertificate {
    /// FSpec FR-415/FR-418's bounded-subtraction abstraction discriminator.
    pub const ABSTRACTION: &'static str = "bounded-subtraction-diagonal/v1";

    /// Whether the task rows exactly reproduce the selected S3-checked task
    /// set. This is only the checked-input part of FR-418; a reader must also
    /// recompute task-product successors, clocks and closure evidence.
    pub fn matches_checked_task_set(&self, task_set: &fsl_semantics::check::TaskSet) -> bool {
        self.abstraction == Self::ABSTRACTION
            && TaskQueueBound::from_checked_task_set(task_set)
                .is_some_and(|rows| rows == self.queue_bounds)
    }

    /// Builds a finite-product certificate member when every checked queue
    /// row has singleton work and its finite slot cap sums without overflow.
    /// The certificate checker still recomputes the product closure.
    pub fn closed_finite_product(
        queue_bounds: Vec<TaskQueueBound>,
        clock_ceilings: Vec<TaskClockCeiling>,
        diagonal_constraints: Vec<TaskDiagonalConstraint>,
        subtractions: Vec<TaskSubtraction>,
    ) -> Option<Self> {
        if queue_bounds.is_empty()
            || queue_bounds
                .iter()
                .any(|row| row.work_lower != row.work_upper || row.max_jobs == 0)
        {
            return None;
        }
        let live_job_cap = queue_bounds
            .iter()
            .try_fold(0_u64, |sum, row| sum.checked_add(row.max_jobs))?;
        (live_job_cap > 0).then_some(Self {
            abstraction: Self::ABSTRACTION,
            queue_bounds,
            clock_ceilings,
            diagonal_constraints,
            subtractions,
            time_progress: TaskTimeProgress {
                method: TaskClosureMethod::ClosedFiniteProduct,
                live_job_cap,
                closure: TaskClosureEvidence::RecomputeAllReachableSuccessors,
            },
        })
    }

    /// Builds the certificate member for a closed source graph only when
    /// the caller has separately established that no checked task release is
    /// reachable. The FR-418 reader must independently reprove that fact.
    pub fn closed_no_release_product(queue_bounds: Vec<TaskQueueBound>) -> Option<Self> {
        if queue_bounds.is_empty() {
            return None;
        }
        Some(Self {
            abstraction: Self::ABSTRACTION,
            queue_bounds,
            clock_ceilings: Vec::new(),
            diagonal_constraints: Vec::new(),
            subtractions: Vec::new(),
            time_progress: TaskTimeProgress {
                method: TaskClosureMethod::ClosedNoReleaseProduct,
                live_job_cap: 0,
                closure: TaskClosureEvidence::RecomputeAllSourceSuccessorsNoTaskRelease,
            },
        })
    }

    /// Builds the FR-418 no-release closure candidate from the selected
    /// checked task plan. The independent reader must still prove that the
    /// source graph reaches no release binding before accepting it.
    pub fn no_release_from_plan(
        plan: &fsl_eval::simulation::model::TaskProductPlan<'_>,
    ) -> Option<Self> {
        if !plan.releases.is_empty() {
            return None;
        }
        let queue_bounds = TaskQueueBound::from_checked_task_set(plan.task_set)?;
        Self::closed_no_release_product(queue_bounds)
    }
}

/// The structured `time_progress` member of FR-418's task-product schema.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskTimeProgress {
    /// The selected finite closure rule.
    pub method: TaskClosureMethod,
    /// The claimed number of simultaneously live job slots.
    pub live_job_cap: u64,
    /// The closure witness the reader must recompute.
    pub closure: TaskClosureEvidence,
}
