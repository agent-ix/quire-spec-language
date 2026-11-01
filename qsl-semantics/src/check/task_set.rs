// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL FR-246 (QSpec FR-419): S3's check of a unit's task sets and of the
//! `schedulable` and `response` claims over them.
//!
//! The grammar admits a task set with no task, a negative time value and a
//! `lo` task with a `wcet_hi`, so this stage refuses them (`invalid_timed_model`
//! / `invalid-task-set`), never the recognizer. Every defect is reported, none
//! hiding another: per task set in source order, first the task set's own
//! defects (duplicate names, an empty set, a time value of another dimension,
//! a non-positive or negative value, the budget rules), then, for each claim
//! over it in source order, that claim's own defects (priorities, `edf`'s
//! terms, `amc`'s deadline, an unknown `response` task). Within each group the
//! defects are in source order of their spans. A task set refused for its own
//! defects is still checked against each claim.
//!
//! **The unit.** The task set's unit is the unit of the first time quantity of
//! the first task, tasks taken in canonical task-name order (not source order),
//! members in the grammar's order, so that reordering the tasks leaves the unit
//! and every converted value unchanged. Every time value of the task set, and
//! the bound of every `response` claim over it, converts exactly into that
//! unit (QSpec FR-142). A task set with no time quantity (one that declares no
//! task, which is refused) has no unit; a `response` claim over it converts
//! its bound to the time dimension's root unit, and the claim is never
//! observable because the set is refused.

use qsl_foundation::source::node_table::NodeKeyDebug;
use std::collections::BTreeMap;

use qsl_forms::{
    ArrivalForm, CriticalityForm, PolicyForm, ReleaseForm, ResponseForm, SchedulableForm,
    SignedQuantityForm, TaskForm, TasksetForm,
};
use qsl_foundation::Span;
use quire_exact::{Integer, NodeKey, Rational};

use super::probabilistic::{exact_value, ExactFault};
use super::refusal::{TaskSetDefect, TaskTimeMember, TemporalCause, TimeUnitCause};
use super::time_units::{TimeScale, TimeUnits};

mod release;
pub(super) use release::check_releases;
pub use release::Release;

/// One source declaration of the task-set family, kept in source order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskSetDeclaration {
    /// `taskset S { task* }`.
    Taskset(Box<TasksetForm>),
    /// `schedulable N on S under P;`.
    Schedulable(Box<SchedulableForm>),
    /// `response N on S.t <= d under fixed_priority;`.
    Response(Box<ResponseForm>),
    /// `release S.t on M::o;` (QSL FR-249).
    Release(Box<ReleaseForm>),
}

impl TaskSetDeclaration {
    /// The construct, for the facet gate.
    pub(super) const fn construct(&self) -> super::refusal::TimedConstruct {
        use super::refusal::TimedConstruct;
        match self {
            Self::Taskset(_) => TimedConstruct::Taskset,
            Self::Schedulable(_) => TimedConstruct::Schedulable,
            Self::Response(_) => TimedConstruct::Response,
            Self::Release(_) => TimedConstruct::Release,
        }
    }

    /// The declaration's span.
    pub(super) const fn span(&self) -> Span {
        match self {
            Self::Taskset(form) => form.span,
            Self::Schedulable(form) => form.span,
            Self::Response(form) => form.span,
            Self::Release(form) => form.span,
        }
    }

    /// The declared name, for the facet gate's refusal.
    pub(super) fn owner(&self) -> String {
        match self {
            Self::Taskset(form) => form.name.name.clone(),
            Self::Schedulable(form) => form.name.name.clone(),
            Self::Response(form) => form.name.name.clone(),
            Self::Release(form) => form.operation.name.clone(),
        }
    }
}

/// How a checked task arrives (QSL FR-246).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Arrival {
    /// Periodic with this period.
    Periodic {
        /// The period.
        period: Rational,
    },
    /// Sporadic with this minimum inter-arrival time.
    Sporadic {
        /// The minimum inter-arrival time.
        min_interarrival: Rational,
    },
}

/// A checked task's budgets (QSL FR-246).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Wcet {
    /// One budget `C`.
    Single(Rational),
    /// A `hi` task's `C(LO)` and `C(HI)`.
    Dual {
        /// `C(LO)`.
        lo: Rational,
        /// `C(HI)`.
        hi: Rational,
    },
}

/// A task's criticality.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Criticality {
    /// `lo`.
    Lo,
    /// `hi`.
    Hi,
}

/// One checked task; every time value is in its task set's unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Task {
    /// The task's name.
    pub name: String,
    /// Its arrival.
    pub arrival: Arrival,
    /// Its relative deadline.
    pub deadline: Rational,
    /// Its budgets.
    pub wcet: Wcet,
    /// Its priority, when it declares one; a smaller value is a higher one.
    pub priority: Option<u64>,
    /// Its criticality.
    pub criticality: Criticality,
    /// Its release jitter, `0` when it declares none.
    pub jitter: Rational,
    /// Its blocking term, `0` when it declares none.
    pub blocking: Rational,
}

/// A task set's unit: the time unit of its first time quantity.
#[derive(Clone, Eq, PartialEq)]
pub struct TaskSetUnit {
    /// The unit as written.
    pub name: String,
    /// The unit's nominal node.
    pub node: NodeKey,
    /// The unit's nominal dimension node.
    pub dimension: NodeKey,
}

impl std::fmt::Debug for TaskSetUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            name,
            node,
            dimension,
        } = self;
        f.debug_struct("TaskSetUnit")
            .field("name", &name)
            .field("node", &NodeKeyDebug(node))
            .field("dimension", &NodeKeyDebug(dimension))
            .finish()
    }
}

/// A checked task set for one preemptive uniprocessor (QSL FR-246).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskSet {
    /// The task set's name.
    pub name: String,
    /// The unit every time value is in; `None` for a task set with no task.
    pub unit: Option<TaskSetUnit>,
    /// The tasks, in source order.
    pub tasks: Vec<Task>,
    /// The span of the declaration.
    pub span: Span,
}

/// A scheduling policy (QSL FR-246).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedulingPolicy {
    /// `fixed_priority`.
    FixedPriority,
    /// `edf`.
    Edf,
    /// `amc`.
    Amc,
}

/// A checked claim (QSL FR-246).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScheduleClaim {
    /// `schedulable N on S under P;`.
    Schedulable(SchedulingPolicy),
    /// `response N on S.t <= d under fixed_priority;`, `d` in the task set's
    /// unit.
    Response {
        /// The task.
        task: String,
        /// The bound `d`.
        bound: Rational,
    },
}

/// One checked claim and the task set it is over.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedScheduleClaim {
    /// The claim's name.
    pub name: String,
    /// The name of the task set it is over.
    pub task_set: String,
    /// The claim.
    pub claim: ScheduleClaim,
    /// The span of the declaration.
    pub span: Span,
}

impl CheckedScheduleClaim {
    /// The scheduling policy: a `response` claim's is `fixed_priority`.
    pub fn policy(&self) -> SchedulingPolicy {
        match &self.claim {
            ScheduleClaim::Schedulable(policy) => *policy,
            ScheduleClaim::Response { .. } => SchedulingPolicy::FixedPriority,
        }
    }

    /// The requirements FR-075's request writer routes this claim by: the
    /// FR-290 kind `schedulability` over a bounded extent (a closed form
    /// decides it, over no unbounded domain), with no evidence kind and no
    /// model subject. `claim` is the claim's node.
    pub fn requirements(&self) -> crate::family::Requirements {
        crate::family::Requirements::new(
            super::Capability::Schedulability,
            crate::family::ClaimExtent::Bounded,
        )
    }
}

/// The unit's checked task sets and the claims over them (QSL FR-246), in
/// source order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CheckedTaskSets {
    task_sets: Vec<TaskSet>,
    claims: Vec<CheckedScheduleClaim>,
    releases: Vec<Release>,
}

impl CheckedTaskSets {
    /// Every checked task set, in source order.
    pub fn task_sets(&self) -> &[TaskSet] {
        &self.task_sets
    }

    /// Every checked claim, in source order.
    pub fn claims(&self) -> &[CheckedScheduleClaim] {
        &self.claims
    }

    /// Every checked `release` declaration, in source order (QSL FR-249).
    pub fn releases(&self) -> &[Release] {
        &self.releases
    }

    /// The tasks the operation `operation` releases, in source order: empty
    /// for an operation that releases none (QSL FR-249).
    pub fn released_by<'s>(
        &'s self,
        operation: &'s quire_checked::key::DeclarationKey,
    ) -> impl Iterator<Item = &'s Release> + 's {
        self.releases
            .iter()
            .filter(move |release| release.operation().key() == operation)
    }

    /// The task set a claim is over.
    pub fn task_set_of(&self, claim: &CheckedScheduleClaim) -> Option<&TaskSet> {
        self.task_sets.iter().find(|set| set.name == claim.task_set)
    }
}

/// What the stage found: the refusals it reports, and the checked model when
/// there are none.
pub(super) struct Checked {
    pub(super) model: CheckedTaskSets,
    pub(super) findings: Vec<Finding>,
}

/// One refusal of the stage.
#[derive(Debug)]
pub(super) enum Finding {
    /// A refusal that is a [`TemporalCause`].
    Cause(TemporalCause),
    /// A value written past the rational-bits ceiling.
    RationalBits {
        /// The value's span.
        span: Span,
        /// Its measure.
        bits: u64,
    },
}

impl Finding {
    fn span(&self) -> Span {
        match self {
            Self::Cause(cause) => cause.span(),
            Self::RationalBits { span, .. } => *span,
        }
    }
}

/// A task's values, in the task set's unit; `None` for a value absent or one
/// that did not convert (its own refusal is reported).
struct Values {
    interval: Option<Rational>,
    deadline: Option<Rational>,
    wcet: Option<Rational>,
    jitter: Option<Rational>,
    blocking: Option<Rational>,
    wcet_hi: Option<Rational>,
    priority: Option<u64>,
}

/// A task as the claims over its set read it.
struct TaskInfo<'f> {
    form: &'f TaskForm,
    values: Values,
}

/// A task set as the claims over it read it.
struct SetInfo<'f> {
    form: &'f TasksetForm,
    tasks: Vec<TaskInfo<'f>>,
    unit: Option<(String, TimeScale)>,
}

struct Converter<'u> {
    units: &'u TimeUnits,
    unit: Option<&'u TimeScale>,
    rational_bits: u64,
}

impl Converter<'_> {
    /// `form` in the task set's unit, or its refusal.
    fn convert(&self, form: &SignedQuantityForm) -> Result<Rational, Finding> {
        let quantity = &form.quantity;
        let magnitude =
            exact_value(&quantity.magnitude, self.rational_bits).map_err(|fault| match fault {
                ExactFault::ZeroDenominator => Finding::Cause(TemporalCause::ZeroDenominator {
                    span: quantity.magnitude.span,
                }),
                ExactFault::AboveCeiling { bits } => Finding::RationalBits {
                    span: quantity.magnitude.span,
                    bits,
                },
            })?;
        let unit = quantity.unit.name.as_str();
        let converted = match self.unit {
            Some(target) => self.units.convert(&magnitude, unit, target),
            // No time quantity of the task set resolved, so this value is not
            // one either: report why.
            None => self.units.scale(unit).map(|scale| magnitude.mul(scale)),
        }
        .map_err(|cause| time_value(cause, quantity.span))?;
        Ok(if form.negative {
            converted.neg()
        } else {
            converted
        })
    }
}

fn time_value(cause: TimeUnitCause, span: Span) -> Finding {
    Finding::Cause(TemporalCause::TimeQuantity {
        cause: Box::new(cause),
        span,
    })
}

fn invalid(set: &str, tasks: &[&str], defect: TaskSetDefect, span: Span) -> Finding {
    Finding::Cause(TemporalCause::InvalidTaskSet {
        task_set: set.to_owned(),
        tasks: tasks.iter().map(|task| (*task).to_owned()).collect(),
        defect,
        span,
    })
}

/// A task's time members in the grammar's order.
fn members(task: &TaskForm) -> Vec<&SignedQuantityForm> {
    [
        Some(&task.interval),
        Some(&task.deadline),
        Some(&task.wcet),
        task.jitter.as_ref(),
        task.blocking.as_ref(),
        task.wcet_hi.as_ref(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The task set's unit: the first time quantity of the first task in
/// canonical task-name order.
fn unit_of(set: &TasksetForm, units: &TimeUnits) -> Option<(String, TimeScale)> {
    let mut order: Vec<&TaskForm> = set.tasks.iter().collect();
    // Stable, so equal names keep source order.
    order.sort_by(|left, right| left.name.name.cmp(&right.name.name));
    order.into_iter().flat_map(members).find_map(|member| {
        let name = member.quantity.unit.name.as_str();
        match units.target(name) {
            Ok(Some(target)) => Some((name.to_owned(), target)),
            Ok(None) | Err(_) => None,
        }
    })
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

/// One task set: its checked form, its tasks' values and its own defects.
fn analyse<'f>(
    form: &'f TasksetForm,
    units: &TimeUnits,
    rational_bits: u64,
) -> (SetInfo<'f>, Vec<Finding>) {
    let unit = unit_of(form, units);
    let converter = Converter {
        units,
        unit: unit.as_ref().map(|(_, scale)| scale),
        rational_bits,
    };
    let set = form.name.name.as_str();
    let mut findings = Vec::new();
    if form.tasks.is_empty() {
        findings.push(invalid(set, &[], TaskSetDefect::NoTask, form.span));
    }
    let mut tasks: Vec<TaskInfo<'f>> = Vec::new();
    for task in &form.tasks {
        let name = task.name.name.as_str();
        if let Some(first) = tasks.iter().find(|seen| seen.form.name.name == name) {
            findings.push(Finding::Cause(TemporalCause::DuplicateTask {
                task_set: set.to_owned(),
                task: name.to_owned(),
                first: first.form.name.span,
                span: task.name.span,
            }));
        }
        let mut value = |form: Option<&SignedQuantityForm>| -> Option<Rational> {
            match converter.convert(form?) {
                Ok(value) => Some(value),
                Err(finding) => {
                    findings.push(finding);
                    None
                }
            }
        };
        let interval = value(Some(&task.interval));
        let deadline = value(Some(&task.deadline));
        let wcet = value(Some(&task.wcet));
        let jitter = value(task.jitter.as_ref());
        let blocking = value(task.blocking.as_ref());
        let wcet_hi = value(task.wcet_hi.as_ref());
        let priority = task
            .priority
            .as_ref()
            .and_then(|priority| match priority.value.to_u64() {
                Some(value) => Some(value),
                None => {
                    findings.push(invalid(
                        set,
                        &[name],
                        TaskSetDefect::PriorityOutOfRange,
                        priority.span,
                    ));
                    None
                }
            });
        let zero = zero();
        for (member, quantity, positive) in [
            (TaskTimeMember::Period, &interval, true),
            (TaskTimeMember::Deadline, &deadline, true),
            (TaskTimeMember::Wcet, &wcet, true),
            (TaskTimeMember::WcetHi, &wcet_hi, true),
            (TaskTimeMember::Jitter, &jitter, false),
            (TaskTimeMember::Blocking, &blocking, false),
        ] {
            let Some(quantity) = quantity else { continue };
            let span = match member {
                TaskTimeMember::Period => task.interval.span,
                TaskTimeMember::Deadline => task.deadline.span,
                TaskTimeMember::Wcet => task.wcet.span,
                TaskTimeMember::WcetHi => task.wcet_hi.as_ref().map_or(task.span, |q| q.span),
                TaskTimeMember::Jitter => task.jitter.as_ref().map_or(task.span, |q| q.span),
                TaskTimeMember::Blocking => task.blocking.as_ref().map_or(task.span, |q| q.span),
            };
            if positive && *quantity <= zero {
                findings.push(invalid(
                    set,
                    &[name],
                    TaskSetDefect::NotPositive(member),
                    span,
                ));
            } else if !positive && *quantity < zero {
                findings.push(invalid(set, &[name], TaskSetDefect::Negative(member), span));
            }
        }
        match (task.criticality, &task.wcet_hi, &wcet, &wcet_hi) {
            (CriticalityForm::Lo, Some(budget), ..) => findings.push(invalid(
                set,
                &[name],
                TaskSetDefect::LoDualBudgets,
                budget.span,
            )),
            (CriticalityForm::Hi, Some(_), Some(lo), Some(hi)) if lo > hi => findings.push(
                invalid(set, &[name], TaskSetDefect::BudgetOrder, task.wcet.span),
            ),
            _ => {}
        }
        tasks.push(TaskInfo {
            form: task,
            values: Values {
                interval,
                deadline,
                wcet,
                jitter,
                blocking,
                wcet_hi,
                priority,
            },
        });
    }
    sort_by_span(&mut findings);
    (SetInfo { form, tasks, unit }, findings)
}

/// The checked task set, when every task's values converted. A task whose
/// value did not convert has a refusal among the set's findings, and the set
/// is then not built: a task is never dropped from a set that is.
fn built(info: &SetInfo<'_>) -> Option<TaskSet> {
    let tasks = info
        .tasks
        .iter()
        .map(|task| {
            let values = &task.values;
            let interval = values.interval.clone()?;
            let wcet = values.wcet.clone()?;
            // A member that is written but did not convert is not defaulted.
            if (task.form.wcet_hi.is_some() && values.wcet_hi.is_none())
                || (task.form.jitter.is_some() && values.jitter.is_none())
                || (task.form.blocking.is_some() && values.blocking.is_none())
            {
                return None;
            }
            let (criticality, wcet) = match task.form.criticality {
                CriticalityForm::Lo => (Criticality::Lo, Wcet::Single(wcet)),
                CriticalityForm::Hi => (
                    Criticality::Hi,
                    match values.wcet_hi.clone() {
                        Some(hi) => Wcet::Dual { lo: wcet, hi },
                        None => Wcet::Single(wcet),
                    },
                ),
            };
            Some(Task {
                name: task.form.name.name.clone(),
                arrival: match task.form.arrival {
                    ArrivalForm::Periodic => Arrival::Periodic { period: interval },
                    ArrivalForm::Sporadic => Arrival::Sporadic {
                        min_interarrival: interval,
                    },
                },
                deadline: values.deadline.clone()?,
                wcet,
                priority: values.priority,
                criticality,
                jitter: values.jitter.clone().unwrap_or_else(zero),
                blocking: values.blocking.clone().unwrap_or_else(zero),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(TaskSet {
        name: info.form.name.name.clone(),
        unit: info.unit.as_ref().map(|(name, scale)| TaskSetUnit {
            name: name.clone(),
            node: scale.unit(),
            dimension: scale.dimension(),
        }),
        tasks,
        span: info.form.span,
    })
}

/// One claim over `info`: its checked form, when its bound converts, and its
/// own defects.
fn check_claim(
    claim: ClaimRef<'_>,
    info: &SetInfo<'_>,
    units: &TimeUnits,
    rational_bits: u64,
) -> (Option<CheckedScheduleClaim>, Vec<Finding>) {
    let set = info.form.name.name.as_str();
    let mut findings = Vec::new();
    let (policy, name, span) = match &claim {
        ClaimRef::Schedulable(form) => (
            match form.policy {
                PolicyForm::FixedPriority => SchedulingPolicy::FixedPriority,
                PolicyForm::Edf => SchedulingPolicy::Edf,
                PolicyForm::Amc => SchedulingPolicy::Amc,
            },
            &form.name.name,
            form.span,
        ),
        ClaimRef::Response(form) => (SchedulingPolicy::FixedPriority, &form.name.name, form.span),
    };
    let zero = zero();
    match policy {
        SchedulingPolicy::Edf => {
            for task in &info.tasks {
                for (member, value, form) in [
                    (
                        TaskTimeMember::Jitter,
                        &task.values.jitter,
                        &task.form.jitter,
                    ),
                    (
                        TaskTimeMember::Blocking,
                        &task.values.blocking,
                        &task.form.blocking,
                    ),
                ] {
                    if let (Some(value), Some(form)) = (value, form) {
                        if *value != zero {
                            findings.push(invalid(
                                set,
                                &[task.form.name.name.as_str()],
                                TaskSetDefect::EdfTerm(member),
                                form.span,
                            ));
                        }
                    }
                }
            }
        }
        SchedulingPolicy::FixedPriority | SchedulingPolicy::Amc => {
            let mut seen: Vec<(u64, &str)> = Vec::new();
            for task in &info.tasks {
                let task_name = task.form.name.name.as_str();
                match task.values.priority {
                    None if task.form.priority.is_none() => findings.push(invalid(
                        set,
                        &[task_name],
                        TaskSetDefect::MissingPriority,
                        task.form.name.span,
                    )),
                    None => {}
                    Some(priority) => {
                        if let Some((_, first)) = seen.iter().find(|(held, _)| *held == priority) {
                            findings.push(invalid(
                                set,
                                &[first, task_name],
                                TaskSetDefect::PriorityClash { priority },
                                task.form.name.span,
                            ));
                        } else {
                            seen.push((priority, task_name));
                        }
                    }
                }
                if policy == SchedulingPolicy::Amc {
                    if let (Some(interval), Some(deadline)) =
                        (&task.values.interval, &task.values.deadline)
                    {
                        if deadline > interval {
                            findings.push(invalid(
                                set,
                                &[task_name],
                                TaskSetDefect::AmcDeadlineBeyondPeriod,
                                task.form.deadline.span,
                            ));
                        }
                    }
                }
            }
        }
    }
    let claim = match &claim {
        ClaimRef::Schedulable(_) => Some(ScheduleClaim::Schedulable(policy)),
        ClaimRef::Response(form) => {
            if !info
                .tasks
                .iter()
                .any(|task| task.form.name.name == form.task.name)
            {
                findings.push(Finding::Cause(TemporalCause::UnknownResponseTask {
                    claim: form.name.name.clone(),
                    task_set: set.to_owned(),
                    task: form.task.name.clone(),
                    span: form.task.span,
                }));
            }
            let bound = Converter {
                units,
                unit: info.unit.as_ref().map(|(_, scale)| scale),
                rational_bits,
            }
            .convert(&SignedQuantityForm {
                negative: false,
                quantity: form.bound.clone(),
                span: form.bound.span,
            });
            match bound {
                Ok(bound) => Some(ScheduleClaim::Response {
                    task: form.task.name.clone(),
                    bound,
                }),
                Err(finding) => {
                    findings.push(finding);
                    None
                }
            }
        }
    };
    sort_by_span(&mut findings);
    (
        claim.map(|claim| CheckedScheduleClaim {
            name: name.clone(),
            task_set: set.to_owned(),
            claim,
            span,
        }),
        findings,
    )
}

fn sort_by_span(findings: &mut [Finding]) {
    // Stable: defects at one span keep the order they were found in.
    findings.sort_by_key(|finding| finding.span().start);
}

enum ClaimRef<'f> {
    Schedulable(&'f SchedulableForm),
    Response(&'f ResponseForm),
}

/// Check a unit's task-set family.
pub(super) fn check(
    declarations: &[TaskSetDeclaration],
    units: &TimeUnits,
    rational_bits: u64,
) -> Checked {
    let mut model = CheckedTaskSets::default();
    let mut findings = Vec::new();
    let mut first_of: BTreeMap<&str, Span> = BTreeMap::new();
    for declaration in declarations {
        let TaskSetDeclaration::Taskset(form) = declaration else {
            continue;
        };
        let (info, mut own) = analyse(form, units, rational_bits);
        let duplicate = first_of.get(form.name.name.as_str()).copied();
        if let Some(first) = duplicate {
            own.insert(
                0,
                Finding::Cause(TemporalCause::DuplicateTaskSet {
                    name: form.name.name.clone(),
                    first,
                    span: form.name.span,
                }),
            );
            sort_by_span(&mut own);
        } else {
            first_of.insert(form.name.name.as_str(), form.name.span);
        }
        findings.append(&mut own);
        model.task_sets.extend(built(&info));
        if duplicate.is_some() {
            // Its claims resolve to the first task set of the name.
            continue;
        }
        for declaration in declarations {
            let claim = match declaration {
                TaskSetDeclaration::Schedulable(claim) if claim.task_set.name == form.name.name => {
                    ClaimRef::Schedulable(claim)
                }
                TaskSetDeclaration::Response(claim) if claim.task_set.name == form.name.name => {
                    ClaimRef::Response(claim)
                }
                TaskSetDeclaration::Taskset(_)
                | TaskSetDeclaration::Schedulable(_)
                | TaskSetDeclaration::Response(_)
                | TaskSetDeclaration::Release(_) => continue,
            };
            let (checked, mut defects) = check_claim(claim, &info, units, rational_bits);
            findings.append(&mut defects);
            model.claims.extend(checked);
        }
    }
    // A claim over a task set the unit does not declare.
    for declaration in declarations {
        let (claim, set) = match declaration {
            TaskSetDeclaration::Schedulable(claim) => (&claim.name.name, &claim.task_set),
            TaskSetDeclaration::Response(claim) => (&claim.name.name, &claim.task_set),
            TaskSetDeclaration::Taskset(_) | TaskSetDeclaration::Release(_) => continue,
        };
        if !first_of.contains_key(set.name.as_str()) {
            findings.push(Finding::Cause(TemporalCause::MissingTaskSet {
                claim: claim.clone(),
                name: set.name.clone(),
                span: set.span,
            }));
        }
    }
    model.claims.sort_by_key(|claim| claim.span.start);
    Checked { model, findings }
}
