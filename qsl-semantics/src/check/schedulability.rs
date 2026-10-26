// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-246: exact S3 admission for task sets and their claims.

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{
    SchedulabilityClaimForm, SchedulabilityClaimKind, TaskArrivalKind, TaskCriticality, TaskForm,
    TaskQuantityForm, TaskSetForm,
};
use qsl_foundation::diagnostic::Code;
use qsl_foundation::Span;
use quire_exact::{Integer, Rational};

use super::probabilistic::exact;
use super::refusal::TimeUnitCause;
use super::time_units::{TimeScale, TimeUnits};

/// One task's checked arrival model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckedArrival {
    /// Fixed period.
    Periodic(Rational),
    /// Minimum inter-arrival time.
    Sporadic(Rational),
}

/// Checked one or two WCET budgets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckedWcet {
    /// One budget.
    Single(Rational),
    /// Low and high criticality budgets.
    Dual {
        /// Low-mode budget.
        lo: Rational,
        /// High-mode budget.
        hi: Rational,
    },
}

/// A task after exact conversion to its task set's canonical unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedTask {
    /// Task name.
    pub name: String,
    /// Period or minimum inter-arrival time.
    pub arrival: CheckedArrival,
    /// Relative deadline.
    pub deadline: Rational,
    /// Execution budget(s).
    pub wcet: CheckedWcet,
    /// Optional priority.
    pub priority: Option<u64>,
    /// Criticality class.
    pub criticality: TaskCriticality,
    /// Release jitter, defaulting to zero.
    pub jitter: Rational,
    /// Blocking term, defaulting to zero.
    pub blocking: Rational,
    /// Source declaration span.
    pub span: Span,
}

/// A checked task set whose tasks are in Unicode scalar task-name order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedTaskSet {
    /// Task-set name.
    pub name: String,
    /// Chosen unit; absent for a task set with no task.
    pub unit: Option<String>,
    /// Canonically ordered tasks.
    pub tasks: Vec<CheckedTask>,
}

/// A claim linked to its checked task set and its FR-075 capability request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedSchedulabilityClaim {
    /// Claim name.
    pub name: String,
    /// Task-set name.
    pub task_set: String,
    /// The complete checked premise set this claim is about, including all
    /// converted time values. This is the input FR-454 emits inside the
    /// claim node from which FR-248's obligation identity is keyed.
    pub subject: CheckedTaskSet,
    /// Claim kind and exact response bound, when applicable.
    pub kind: CheckedSchedulabilityClaimKind,
    /// FR-075 request kind.
    pub capability: super::Capability,
    /// Source declaration span.
    pub span: Span,
}

/// Checked claim variant and policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckedSchedulabilityClaimKind {
    /// Decide the whole task set under one policy.
    Schedulable(CheckedPolicy),
    /// Check one task's response time against an exact bound.
    Response {
        /// Bounded task name.
        task: String,
        /// Exact converted response bound.
        bound: Rational,
        /// Unit in which `bound` is expressed.
        bound_unit: String,
    },
}

/// One admitted scheduling policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckedPolicy {
    /// Fixed priority response-time analysis.
    FixedPriority,
    /// Earliest deadline first processor-demand analysis.
    Edf,
    /// AMC response-time analysis.
    Amc,
}

/// A source-located S3 refusal while checking a schedulability declaration.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SchedulabilityRefusal {
    /// A quantity does not name a linear time unit.
    TimeUnit {
        /// Why the quantity's unit is not time.
        cause: TimeUnitCause,
        /// The quantity's source span.
        span: Span,
    },
    /// A task set or task violates a validity constraint.
    InvalidTaskSet {
        /// The offending task or task set.
        task: String,
        /// The validity rule that failed.
        reason: &'static str,
        /// The refusal's source span.
        span: Span,
    },
    /// Two task declarations share one name.
    DuplicateTask {
        /// The repeated task name.
        task: String,
        /// The duplicate name's source span.
        span: Span,
    },
    /// A response claim names no task in its task set.
    MissingTask {
        /// The unresolved task name.
        task: String,
        /// The reference source span.
        span: Span,
    },
    /// A claim names no task set.
    MissingTaskSet {
        /// The unresolved task-set name.
        task_set: String,
        /// The reference source span.
        span: Span,
    },
}

impl SchedulabilityRefusal {
    /// The exact source site the refusal identifies.
    pub fn span(&self) -> Span {
        match self {
            Self::TimeUnit { span, .. }
            | Self::InvalidTaskSet { span, .. }
            | Self::DuplicateTask { span, .. }
            | Self::MissingTask { span, .. }
            | Self::MissingTaskSet { span, .. } => *span,
        }
    }

    /// Catalog code for this refusal.
    pub fn code(&self) -> Code {
        match self {
            Self::TimeUnit { .. } => Code::IllTyped,
            Self::InvalidTaskSet { .. } => Code::InvalidTimedModel,
            Self::DuplicateTask { .. } => Code::ConflictingDeclaration,
            Self::MissingTask { .. } | Self::MissingTaskSet { .. } => Code::MissingDeclaration,
        }
    }

    /// Stable catalog cause.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::TimeUnit { .. } => "type-mismatch",
            Self::InvalidTaskSet { .. } => "invalid-task-set",
            Self::DuplicateTask { .. } => "duplicate-declaration",
            Self::MissingTask { .. } | Self::MissingTaskSet { .. } => "missing-name",
        }
    }
}

/// The full checked task-set and claim outputs for one source unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedSchedulability {
    /// Checked task sets in declaration order.
    pub task_sets: Vec<CheckedTaskSet>,
    /// Checked claims in declaration order.
    pub claims: Vec<CheckedSchedulabilityClaim>,
}

/// Convert all time members to the period unit of the task with the least
/// name. Reordering source declarations therefore cannot change unit or
/// checked values (FR-246 / FR-482).
pub(super) fn check(
    task_sets: &[TaskSetForm],
    claims: &[SchedulabilityClaimForm],
    units: &TimeUnits,
    rational_bits: u64,
) -> Result<CheckedSchedulability, Vec<SchedulabilityRefusal>> {
    let mut refusals = Vec::new();
    let mut checked = Vec::with_capacity(task_sets.len());
    let mut ordered_sets = task_sets.iter().collect::<Vec<_>>();
    ordered_sets.sort_by_key(|set| set.span.start);
    for set in ordered_sets {
        let set_refusal_start = refusals.len();
        let mut canonical = set.tasks.iter().collect::<Vec<_>>();
        canonical.sort_by(|left, right| left.name.name.cmp(&right.name.name));
        let unit = canonical
            .first()
            .map(|task| task.interval.quantity.unit.name.clone());
        let target = match unit.as_deref().map(|unit| units.target(unit)).transpose() {
            Ok(target) => target.flatten(),
            Err(cause) => {
                let first = canonical[0];
                refusals.push(SchedulabilityRefusal::TimeUnit {
                    cause,
                    span: first.interval.quantity.unit.span,
                });
                None
            }
        };
        let mut seen = BTreeSet::new();
        let mut tasks = Vec::with_capacity(canonical.len());
        let mut source_order = set.tasks.iter().collect::<Vec<_>>();
        source_order.sort_by_key(|task| task.span.start);
        for task in source_order {
            if !seen.insert(task.name.name.as_str()) {
                refusals.push(SchedulabilityRefusal::DuplicateTask {
                    task: task.name.name.clone(),
                    span: task.name.span,
                });
            }
            if let Some(checked_task) =
                check_task(task, target.as_ref(), units, rational_bits, &mut refusals)
            {
                tasks.push(checked_task);
            }
        }
        tasks.sort_by(|left, right| left.name.cmp(&right.name));
        if set.tasks.is_empty() {
            refusals.push(SchedulabilityRefusal::InvalidTaskSet {
                task: set.name.name.clone(),
                reason: "empty",
                span: set.span,
            });
        }
        refusals[set_refusal_start..].sort_by_key(|refusal| refusal.span().start);
        checked.push(CheckedTaskSet {
            name: set.name.name.clone(),
            unit,
            tasks,
        });
    }
    let by_name: BTreeMap<_, _> = checked.iter().map(|set| (set.name.as_str(), set)).collect();
    let forms_by_name: BTreeMap<_, _> = task_sets
        .iter()
        .map(|set| (set.name.name.as_str(), set))
        .collect();
    let mut checked_claims = Vec::with_capacity(claims.len());
    let mut ordered_claims = claims.iter().collect::<Vec<_>>();
    ordered_claims.sort_by_key(|claim| claim.span.start);
    for claim in ordered_claims {
        let claim_refusal_start = refusals.len();
        let Some(set) = by_name.get(claim.task_set.name.as_str()).copied() else {
            refusals.push(SchedulabilityRefusal::MissingTaskSet {
                task_set: claim.task_set.name.clone(),
                span: claim.task_set.span,
            });
            continue;
        };
        let Some(set_form) = forms_by_name.get(claim.task_set.name.as_str()).copied() else {
            refusals.push(SchedulabilityRefusal::MissingTaskSet {
                task_set: claim.task_set.name.clone(),
                span: claim.task_set.span,
            });
            continue;
        };
        let kind = match &claim.kind {
            SchedulabilityClaimKind::Schedulable { policy } => {
                let checked_policy = match policy.as_str() {
                    "fixed_priority" => CheckedPolicy::FixedPriority,
                    "edf" => CheckedPolicy::Edf,
                    "amc" => CheckedPolicy::Amc,
                    _ => unreachable!("the CST grammar admits only known policies"),
                };
                check_policy_priorities(set_form, checked_policy, claim.span, &mut refusals);
                check_policy(set, checked_policy, &mut refusals);
                CheckedSchedulabilityClaimKind::Schedulable(checked_policy)
            }
            SchedulabilityClaimKind::Response { task, bound } => {
                check_policy_priorities(
                    set_form,
                    CheckedPolicy::FixedPriority,
                    claim.span,
                    &mut refusals,
                );
                check_policy(set, CheckedPolicy::FixedPriority, &mut refusals);
                if !set_form
                    .tasks
                    .iter()
                    .any(|candidate| candidate.name.name == task.name)
                {
                    refusals.push(SchedulabilityRefusal::MissingTask {
                        task: task.name.clone(),
                        span: task.span,
                    });
                }
                let magnitude = match exact::value(&bound.magnitude, rational_bits) {
                    Ok(value) => value,
                    Err(_) => {
                        refusals.push(SchedulabilityRefusal::InvalidTaskSet {
                            task: task.name.clone(),
                            reason: "exact-value-limit",
                            span: bound.span,
                        });
                        Rational::from_integer(Integer::zero())
                    }
                };
                match set
                    .unit
                    .as_deref()
                    .and_then(|unit| units.target(unit).ok().flatten())
                {
                    Some(target) => match units.convert(&magnitude, &bound.unit.name, &target) {
                        Ok(bound_value) => CheckedSchedulabilityClaimKind::Response {
                            task: task.name.clone(),
                            bound: bound_value,
                            bound_unit: set.unit.clone().unwrap_or_default(),
                        },
                        Err(cause) => {
                            refusals.push(SchedulabilityRefusal::TimeUnit {
                                cause,
                                span: bound.span,
                            });
                            CheckedSchedulabilityClaimKind::Response {
                                task: task.name.clone(),
                                bound: magnitude,
                                bound_unit: bound.unit.name.clone(),
                            }
                        }
                    },
                    None => {
                        if let Err(cause) = units.scale(&bound.unit.name) {
                            refusals.push(SchedulabilityRefusal::TimeUnit {
                                cause,
                                span: bound.span,
                            });
                        }
                        CheckedSchedulabilityClaimKind::Response {
                            task: task.name.clone(),
                            bound: magnitude,
                            bound_unit: bound.unit.name.clone(),
                        }
                    }
                }
            }
        };
        checked_claims.push(CheckedSchedulabilityClaim {
            name: claim.name.name.clone(),
            task_set: set.name.clone(),
            subject: set.clone(),
            kind,
            capability: super::Capability::Schedulability,
            span: claim.span,
        });
        refusals[claim_refusal_start..].sort_by_key(|refusal| refusal.span().start);
    }
    if refusals.is_empty() {
        Ok(CheckedSchedulability {
            task_sets: checked,
            claims: checked_claims,
        })
    } else {
        Err(refusals)
    }
}

fn check_task(
    task: &TaskForm,
    target: Option<&TimeScale>,
    units: &TimeUnits,
    rational_bits: u64,
    refusals: &mut Vec<SchedulabilityRefusal>,
) -> Option<CheckedTask> {
    let convert = |value: &TaskQuantityForm, refusals: &mut Vec<SchedulabilityRefusal>| {
        let magnitude = match exact::value(&value.quantity.magnitude, rational_bits) {
            Ok(value) => value,
            Err(_) => {
                refusals.push(SchedulabilityRefusal::InvalidTaskSet {
                    task: task.name.name.clone(),
                    reason: "exact-value-limit",
                    span: value.span,
                });
                return None;
            }
        };
        if let Err(cause) = units.scale(&value.quantity.unit.name) {
            refusals.push(SchedulabilityRefusal::TimeUnit {
                cause,
                span: value.quantity.span,
            });
            return None;
        }
        let magnitude = if value.negative {
            Rational::from_integer(Integer::one().neg()).mul(&magnitude)
        } else {
            magnitude
        };
        let Some(target) = target else {
            return Some(magnitude);
        };
        match units.convert(&magnitude, &value.quantity.unit.name, target) {
            Ok(converted) => Some(converted),
            Err(cause) => {
                refusals.push(SchedulabilityRefusal::TimeUnit {
                    cause,
                    span: value.quantity.span,
                });
                None
            }
        }
    };

    // Check every declared quantity before returning. A bad field must not
    // hide a later type mismatch or an independent semantic defect.
    let interval = convert(&task.interval, refusals);
    let deadline = convert(&task.deadline, refusals);
    let wcet_lo = convert(&task.wcet, refusals);
    let wcet_hi = task
        .wcet_hi
        .as_ref()
        .and_then(|value| convert(value, refusals));
    let jitter = task.jitter.as_ref().map_or_else(
        || Some(Rational::from_integer(Integer::zero())),
        |value| convert(value, refusals),
    );
    let blocking = task.blocking.as_ref().map_or_else(
        || Some(Rational::from_integer(Integer::zero())),
        |value| convert(value, refusals),
    );

    check_positive_time(
        task,
        interval.as_ref(),
        "non-positive-period",
        task.interval.span,
        refusals,
    );
    check_positive_time(
        task,
        deadline.as_ref(),
        "non-positive-deadline",
        task.deadline.span,
        refusals,
    );
    check_positive_time(
        task,
        wcet_lo.as_ref(),
        "non-positive-wcet",
        task.wcet.span,
        refusals,
    );
    check_positive_time(
        task,
        wcet_hi.as_ref(),
        "non-positive-wcet-hi",
        task.wcet_hi.as_ref().map_or(task.span, |value| value.span),
        refusals,
    );

    if task.criticality == TaskCriticality::Lo && task.wcet_hi.is_some() {
        refusals.push(SchedulabilityRefusal::InvalidTaskSet {
            task: task.name.name.clone(),
            reason: "lo-dual-budget",
            span: task.wcet_hi.as_ref().map_or(task.span, |value| value.span),
        });
    }
    if task.criticality == TaskCriticality::Hi && task.wcet_hi.is_none() {
        refusals.push(SchedulabilityRefusal::InvalidTaskSet {
            task: task.name.name.clone(),
            reason: "missing-wcet-hi",
            span: task.span,
        });
    }
    if let (Some(low), Some(high)) = (wcet_lo.as_ref(), wcet_hi.as_ref()) {
        if task.criticality == TaskCriticality::Hi && low > high {
            refusals.push(SchedulabilityRefusal::InvalidTaskSet {
                task: task.name.name.clone(),
                reason: "budget-order",
                span: task.span,
            });
        }
    }
    if jitter
        .as_ref()
        .is_some_and(|value| value.numerator().is_negative())
    {
        refusals.push(SchedulabilityRefusal::InvalidTaskSet {
            task: task.name.name.clone(),
            reason: "negative-jitter",
            span: task.jitter.as_ref().map_or(task.span, |value| value.span),
        });
    }
    if blocking
        .as_ref()
        .is_some_and(|value| value.numerator().is_negative())
    {
        refusals.push(SchedulabilityRefusal::InvalidTaskSet {
            task: task.name.name.clone(),
            reason: "negative-blocking",
            span: task.blocking.as_ref().map_or(task.span, |value| value.span),
        });
    }
    let (Some(interval), Some(deadline), Some(wcet_lo), Some(jitter), Some(blocking)) =
        (interval, deadline, wcet_lo, jitter, blocking)
    else {
        return None;
    };
    let wcet = match task.criticality {
        TaskCriticality::Hi => CheckedWcet::Dual {
            lo: wcet_lo,
            hi: wcet_hi?,
        },
        TaskCriticality::Lo => CheckedWcet::Single(wcet_lo),
    };
    let arrival = match task.arrival {
        TaskArrivalKind::Periodic => CheckedArrival::Periodic(interval),
        TaskArrivalKind::Sporadic => CheckedArrival::Sporadic(interval),
    };
    Some(CheckedTask {
        name: task.name.name.clone(),
        arrival,
        deadline,
        wcet,
        priority: task.priority.map(|(priority, _)| priority),
        criticality: task.criticality,
        jitter,
        blocking,
        span: task.span,
    })
}

fn check_positive_time(
    task: &TaskForm,
    value: Option<&Rational>,
    reason: &'static str,
    span: Span,
    refusals: &mut Vec<SchedulabilityRefusal>,
) {
    if value.is_some_and(|value| value.numerator().is_negative() || value.is_zero()) {
        refusals.push(SchedulabilityRefusal::InvalidTaskSet {
            task: task.name.name.clone(),
            reason,
            span,
        });
    }
}

fn check_policy(
    set: &CheckedTaskSet,
    policy: CheckedPolicy,
    refusals: &mut Vec<SchedulabilityRefusal>,
) {
    for task in &set.tasks {
        let interval = match &task.arrival {
            CheckedArrival::Periodic(value) | CheckedArrival::Sporadic(value) => value,
        };
        if policy == CheckedPolicy::Edf && (!task.jitter.is_zero() || !task.blocking.is_zero()) {
            refusals.push(SchedulabilityRefusal::InvalidTaskSet {
                task: task.name.clone(),
                reason: "edf-ignores-jitter-or-blocking",
                span: task.span,
            });
        }
        if policy == CheckedPolicy::Amc && task.deadline > *interval {
            refusals.push(SchedulabilityRefusal::InvalidTaskSet {
                task: task.name.clone(),
                reason: "amc-deadline-exceeds-period",
                span: task.span,
            });
        }
    }
}

fn check_policy_priorities(
    set: &TaskSetForm,
    policy: CheckedPolicy,
    claim_span: Span,
    refusals: &mut Vec<SchedulabilityRefusal>,
) {
    if policy == CheckedPolicy::Edf {
        return;
    }
    let mut priorities: BTreeMap<u64, &TaskForm> = BTreeMap::new();
    let mut tasks = set.tasks.iter().collect::<Vec<_>>();
    tasks.sort_by_key(|task| task.span.start);
    for task in tasks {
        match task.priority {
            None => refusals.push(SchedulabilityRefusal::InvalidTaskSet {
                task: task.name.name.clone(),
                reason: "missing-priority",
                span: task.span,
            }),
            Some((priority, _)) => {
                if let Some(first) = priorities.get(&priority) {
                    refusals.push(SchedulabilityRefusal::InvalidTaskSet {
                        task: format!("{} and {}", first.name.name, task.name.name),
                        reason: "priority-clash",
                        span: claim_span,
                    });
                } else {
                    priorities.insert(priority, task);
                }
            }
        }
    }
}
