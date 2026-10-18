// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-246: the checked task set and the schedulability claim over it
//! (ADR-026 RT-1 to RT-3).
//!
//! [`ScheduleSubject`] is the subject EN-7 analyses and the closed-form
//! checker recomputes: a [`TaskSet`] and a [`ScheduleClaim`], admitted
//! together so that no analysis divides by a zero period, ranks a task
//! with no priority, or names a task the set does not hold. Every time
//! value is an exact rational in the task set's one unit; WCETs, blocking
//! terms and jitter are the stated premises of the claim (RT-2).

use qsl_foundation::diagnostic::Code;
use quire_exact::{Identifier, Rational};

use crate::certificate::closed_form::outcome::ClosedFormAnalysis;
use crate::identity::QualifiedName;

/// How a task's jobs arrive (RT-1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Arrival {
    /// A job every `period`.
    Periodic {
        /// The period `T`.
        period: Rational,
    },
    /// Jobs at least `min_interarrival` apart.
    Sporadic {
        /// The minimum inter-arrival time `T`.
        min_interarrival: Rational,
    },
}

impl Arrival {
    /// The period or minimum inter-arrival time `T`.
    pub fn interval(&self) -> &Rational {
        match self {
            Self::Periodic { period } => period,
            Self::Sporadic { min_interarrival } => min_interarrival,
        }
    }
}

/// A task's worst-case execution time (RT-1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Wcet {
    /// One budget `C`, for a `LO` task or a `HI` task whose budgets agree.
    Single(Rational),
    /// A `HI` task's two budgets, `C(LO) <= C(HI)`.
    Dual {
        /// `C(LO)`.
        lo: Rational,
        /// `C(HI)`.
        hi: Rational,
    },
}

impl Wcet {
    /// `C(LO)`, or the one budget.
    pub fn lo(&self) -> &Rational {
        match self {
            Self::Single(budget) | Self::Dual { lo: budget, .. } => budget,
        }
    }

    /// `C(HI)`, or the one budget.
    pub fn hi(&self) -> &Rational {
        match self {
            Self::Single(budget) | Self::Dual { hi: budget, .. } => budget,
        }
    }
}

/// A task's criticality (RT-1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Criticality {
    /// `LO`.
    Lo,
    /// `HI`.
    Hi,
}

/// One task of a task set (RT-1). A smaller `priority` is a higher
/// priority (QSpec FR-419).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Task {
    /// The task's name.
    pub name: Identifier,
    /// Its arrival and `T`.
    pub arrival: Arrival,
    /// Its relative deadline `D`.
    pub deadline: Rational,
    /// Its WCET.
    pub wcet: Wcet,
    /// Its priority, stated for fixed-priority and AMC claims.
    pub priority: Option<u64>,
    /// Its criticality.
    pub criticality: Criticality,
    /// Its release jitter `J`; zero when undeclared.
    pub jitter: Rational,
    /// Its blocking term `B`; zero when undeclared.
    pub blocking: Rational,
}

/// A scheduling policy (RT-3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedulingPolicy {
    /// `fixed-priority`.
    FixedPriority,
    /// `edf`.
    Edf,
    /// `amc`.
    Amc,
}

/// A claim over a task set (RT-3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScheduleClaim {
    /// `schedulable S under P`.
    Schedulable(SchedulingPolicy),
    /// `response S.τ <= d under fixed-priority`.
    Response {
        /// The task whose worst-case response time is bounded.
        task: Identifier,
        /// The bound `d`.
        bound: Rational,
    },
}

/// A time member a task states, for a refusal to name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeMember {
    /// The period or minimum inter-arrival time.
    Period,
    /// The relative deadline.
    Deadline,
    /// A WCET budget.
    Wcet,
    /// The release jitter.
    Jitter,
    /// The blocking term.
    Blocking,
}

/// FR-246's refusals of a task set or a claim over it, each with the
/// catalog code and cause it carries. The span a frontend attaches is the
/// frontend's.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TaskSetRefusal {
    /// The task set declares no task.
    #[error("invalid_model_binding/malformed-declaration: the task set declares no task")]
    Empty,
    /// A period, minimum inter-arrival time, deadline or WCET is not
    /// positive.
    #[error("invalid_model_binding/malformed-declaration: task {} states a non-positive {member:?}", .task.as_str())]
    NotPositive {
        /// The task.
        task: Identifier,
        /// The member.
        member: TimeMember,
    },
    /// A jitter or blocking term is negative.
    #[error("invalid_model_binding/malformed-declaration: task {} states a negative {member:?}", .task.as_str())]
    Negative {
        /// The task.
        task: Identifier,
        /// The member.
        member: TimeMember,
    },
    /// A `HI` task's `C(LO)` exceeds its `C(HI)`.
    #[error("invalid_model_binding/malformed-declaration: task {} has C(LO) above C(HI)", .task.as_str())]
    BudgetOrder {
        /// The task.
        task: Identifier,
    },
    /// A `LO` task states dual budgets.
    #[error("invalid_model_binding/malformed-declaration: LO task {} states dual budgets", .task.as_str())]
    LoDualBudgets {
        /// The task.
        task: Identifier,
    },
    /// Two tasks share a name.
    #[error("conflicting_declaration/duplicate-declaration: two tasks are named {}", .task.as_str())]
    DuplicateTask {
        /// The shared name.
        task: Identifier,
    },
    /// A fixed-priority or AMC claim over a task with no priority.
    #[error("invalid_model_binding/malformed-declaration: task {} states no priority", .task.as_str())]
    MissingPriority {
        /// The task.
        task: Identifier,
    },
    /// A fixed-priority or AMC claim over two tasks of one priority.
    #[error("invalid_model_binding/malformed-declaration: tasks {} and {} share priority {priority}", .first.as_str(), .second.as_str())]
    PriorityClash {
        /// The earlier task.
        first: Identifier,
        /// The later task.
        second: Identifier,
        /// The shared priority.
        priority: u64,
    },
    /// A response claim names a task the set does not hold.
    #[error("missing_declaration/missing-name: the task set holds no task {}", .task.as_str())]
    UnknownTask {
        /// The named task.
        task: Identifier,
    },
    /// An AMC claim over a task whose deadline exceeds its period, which
    /// AMC-rtb does not cover.
    #[error("invalid_model_binding/malformed-declaration: AMC-rtb covers no deadline beyond the period, task {} states one", .task.as_str())]
    AmcDeadlineBeyondPeriod {
        /// The task.
        task: Identifier,
    },
}

impl TaskSetRefusal {
    /// The catalog code of this refusal.
    pub fn code(&self) -> Code {
        match self {
            Self::DuplicateTask { .. } => Code::ConflictingDeclaration,
            Self::UnknownTask { .. } => Code::MissingDeclaration,
            Self::Empty
            | Self::NotPositive { .. }
            | Self::Negative { .. }
            | Self::BudgetOrder { .. }
            | Self::LoDualBudgets { .. }
            | Self::MissingPriority { .. }
            | Self::PriorityClash { .. }
            | Self::AmcDeadlineBeyondPeriod { .. } => Code::InvalidModelBinding,
        }
    }

    /// The catalog cause under [`Self::code`].
    pub fn cause(&self) -> &'static str {
        match self {
            Self::DuplicateTask { .. } => "duplicate-declaration",
            Self::UnknownTask { .. } => "missing-name",
            Self::Empty
            | Self::NotPositive { .. }
            | Self::Negative { .. }
            | Self::BudgetOrder { .. }
            | Self::LoDualBudgets { .. }
            | Self::MissingPriority { .. }
            | Self::PriorityClash { .. }
            | Self::AmcDeadlineBeyondPeriod { .. } => "malformed-declaration",
        }
    }
}

/// A task set for one preemptive uniprocessor whose tasks passed FR-246's
/// policy-independent checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskSet {
    name: QualifiedName,
    tasks: Vec<Task>,
}

impl TaskSet {
    /// Admit `tasks` as the task set `name`.
    pub fn new(name: QualifiedName, tasks: Vec<Task>) -> Result<Self, TaskSetRefusal> {
        if tasks.is_empty() {
            return Err(TaskSetRefusal::Empty);
        }
        for (index, task) in tasks.iter().enumerate() {
            if tasks[..index].iter().any(|seen| seen.name == task.name) {
                return Err(TaskSetRefusal::DuplicateTask {
                    task: task.name.clone(),
                });
            }
            check_task(task)?;
        }
        Ok(Self { name, tasks })
    }

    /// The task set's name.
    pub fn name(&self) -> &QualifiedName {
        &self.name
    }

    /// The tasks, in declared order.
    pub fn tasks(&self) -> &[Task] {
        &self.tasks
    }

    /// The task named `name`.
    pub fn task(&self, name: &Identifier) -> Option<&Task> {
        self.tasks.iter().find(|task| &task.name == name)
    }
}

fn check_task(task: &Task) -> Result<(), TaskSetRefusal> {
    let zero = Rational::from_integer(quire_exact::Integer::zero());
    let positive = |value: &Rational, member| {
        if *value > zero {
            Ok(())
        } else {
            Err(TaskSetRefusal::NotPositive {
                task: task.name.clone(),
                member,
            })
        }
    };
    let non_negative = |value: &Rational, member| {
        if *value >= zero {
            Ok(())
        } else {
            Err(TaskSetRefusal::Negative {
                task: task.name.clone(),
                member,
            })
        }
    };
    positive(task.arrival.interval(), TimeMember::Period)?;
    positive(&task.deadline, TimeMember::Deadline)?;
    positive(task.wcet.lo(), TimeMember::Wcet)?;
    positive(task.wcet.hi(), TimeMember::Wcet)?;
    non_negative(&task.jitter, TimeMember::Jitter)?;
    non_negative(&task.blocking, TimeMember::Blocking)?;
    match (&task.wcet, task.criticality) {
        (Wcet::Dual { .. }, Criticality::Lo) => Err(TaskSetRefusal::LoDualBudgets {
            task: task.name.clone(),
        }),
        (Wcet::Dual { lo, hi }, Criticality::Hi) if lo > hi => Err(TaskSetRefusal::BudgetOrder {
            task: task.name.clone(),
        }),
        _ => Ok(()),
    }
}

/// A task set and a claim over it, admitted together (FR-246): the subject
/// of EN-7's analysis and of the closed-form checker's recomputation. Its
/// obligation identity binds every premise (RT-2); CG mints that digest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleSubject {
    task_set: TaskSet,
    claim: ScheduleClaim,
}

impl ScheduleSubject {
    /// Admit `claim` over `task_set`: a fixed-priority, AMC or response
    /// claim needs a distinct priority on every task, a response claim
    /// names a task of the set, and an `edf` claim reads no priority.
    pub fn new(task_set: TaskSet, claim: ScheduleClaim) -> Result<Self, TaskSetRefusal> {
        match &claim {
            ScheduleClaim::Schedulable(SchedulingPolicy::Edf) => {}
            ScheduleClaim::Schedulable(policy) => {
                check_priorities(&task_set)?;
                if *policy == SchedulingPolicy::Amc {
                    if let Some(task) = task_set
                        .tasks()
                        .iter()
                        .find(|task| task.deadline > *task.arrival.interval())
                    {
                        return Err(TaskSetRefusal::AmcDeadlineBeyondPeriod {
                            task: task.name.clone(),
                        });
                    }
                }
            }
            ScheduleClaim::Response { task, .. } => {
                check_priorities(&task_set)?;
                if task_set.task(task).is_none() {
                    return Err(TaskSetRefusal::UnknownTask { task: task.clone() });
                }
            }
        }
        Ok(Self { task_set, claim })
    }

    /// The task set.
    pub fn task_set(&self) -> &TaskSet {
        &self.task_set
    }

    /// The claim.
    pub fn claim(&self) -> &ScheduleClaim {
        &self.claim
    }

    /// The closed-form analysis the claim names (RT-7).
    pub fn analysis(&self) -> ClosedFormAnalysis {
        match &self.claim {
            ScheduleClaim::Schedulable(SchedulingPolicy::Edf) => ClosedFormAnalysis::EdfQpa,
            ScheduleClaim::Schedulable(SchedulingPolicy::Amc) => ClosedFormAnalysis::AmcRtb,
            ScheduleClaim::Schedulable(SchedulingPolicy::FixedPriority)
            | ScheduleClaim::Response { .. } => ClosedFormAnalysis::FixedPriorityRta,
        }
    }

    /// The tasks a fixed-priority or AMC analysis visits, highest priority
    /// first. Every task of such a subject has a priority.
    pub fn by_priority(&self) -> Vec<&Task> {
        let mut tasks: Vec<&Task> = self.task_set.tasks().iter().collect();
        tasks.sort_by_key(|task| task.priority);
        tasks
    }

    /// The tasks of higher priority than `task`.
    pub fn higher_priority<'a>(&'a self, task: &'a Task) -> impl Iterator<Item = &'a Task> {
        self.task_set
            .tasks()
            .iter()
            .filter(move |other| other.priority < task.priority)
    }

    /// The bound a task's response time is held to: the claim's `d` for the
    /// task a response claim names, the task's deadline otherwise.
    pub fn bound<'a>(&'a self, task: &'a Task) -> &'a Rational {
        match &self.claim {
            ScheduleClaim::Response { task: named, bound } if *named == task.name => bound,
            ScheduleClaim::Response { .. } | ScheduleClaim::Schedulable(_) => &task.deadline,
        }
    }

    /// The tasks the claim's fixed-priority analysis covers: the named task
    /// for a response claim, every task otherwise, highest priority first.
    pub fn analysed(&self) -> Vec<&Task> {
        match &self.claim {
            ScheduleClaim::Response { task, .. } => self.task_set.task(task).into_iter().collect(),
            ScheduleClaim::Schedulable(_) => self.by_priority(),
        }
    }
}

fn check_priorities(task_set: &TaskSet) -> Result<(), TaskSetRefusal> {
    let mut seen: Vec<(u64, &Identifier)> = Vec::new();
    for task in task_set.tasks() {
        let Some(priority) = task.priority else {
            return Err(TaskSetRefusal::MissingPriority {
                task: task.name.clone(),
            });
        };
        if let Some((_, first)) = seen.iter().find(|(held, _)| *held == priority) {
            return Err(TaskSetRefusal::PriorityClash {
                first: (*first).clone(),
                second: task.name.clone(),
                priority,
            });
        }
        seen.push((priority, &task.name));
    }
    Ok(())
}
