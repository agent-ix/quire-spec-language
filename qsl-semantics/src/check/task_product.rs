// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared typed identities for QSL's checked task-product automaton.
//!
//! The analyzer and independent readers use these immutable values at their
//! boundary. Each consumer still compiles and explores its own product; this
//! module contains no compiler, scheduler search, DBM operation, or replay
//! acceptance logic.

use quire_exact::{Integer, Rational};
use quire_semantic_value::model_transition::ModelTransition;

/// A task-derived scheduler action (QSL FR-419).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskAction {
    /// A source activation creates the task job identity.
    Activate,
    /// The pending job becomes ready after its selected jitter.
    Release,
    /// The scheduler selects a ready job.
    Dispatch,
    /// The scheduler suspends a running job.
    Preempt,
    /// The scheduler resumes a suspended job.
    Resume,
    /// The selected demand has received its required service.
    Complete,
    /// A source mode change increases a HI service allowance.
    BudgetChange,
    /// A live job reaches its nominal deadline before completion.
    DeadlineMiss,
}

impl TaskAction {
    /// The FR-419 wire spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Activate => "activate",
            Self::Release => "release",
            Self::Dispatch => "dispatch",
            Self::Preempt => "preempt",
            Self::Resume => "resume",
            Self::Complete => "complete",
            Self::BudgetChange => "budget-change",
            Self::DeadlineMiss => "deadline-miss",
        }
    }
}

/// QSL FR-419's exact identity for one derived scheduler action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskTransitionIdentity {
    /// The action name.
    pub action: TaskAction,
    /// The checked task-set name.
    pub taskset: String,
    /// The checked task name.
    pub task: String,
    /// The zero-based activation occurrence for this source transition.
    pub job: u64,
    /// The task's bounded storage slot.
    pub slot: u64,
    /// The slot generation, preventing identity reuse.
    pub generation: u64,
    /// The source transition whose occurrence identifies this job.
    pub release: ModelTransition,
    /// The source transition enabling this scheduler action, if any.
    pub source_transition: Option<ModelTransition>,
    /// The exact demand selected for this job.
    pub demand: Rational,
    /// The job's selected release jitter.
    pub jitter: Rational,
    /// The job's absolute deadline.
    pub deadline: Rational,
    /// The checked task's declared priority, when present. EDF ignores this
    /// value for queue order; tasks without a declaration omit it.
    pub priority: Option<u64>,
    /// The HI budget increase, present only for a budget-change action.
    pub budget_delta: Option<Rational>,
}

/// QSpec FR-418's finite identity for a task-product certificate edge or
/// bounded subtraction. Concrete occurrence, generation, demand, jitter and
/// deadline remain in [`TaskTransitionIdentity`] for replay/search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskTransitionTemplate {
    /// The scheduler action.
    pub action: TaskAction,
    /// The checked task-set name.
    pub taskset: String,
    /// The checked task name.
    pub task: String,
    /// The zero-based canonical rank among this task's live jobs.
    pub job_rank: u64,
    /// The finite queue slot bound to the ranked live job.
    pub slot: u64,
    /// The source transition identifying the activation.
    pub release: ModelTransition,
    /// The source operation enabling this scheduler action, if any.
    pub source_transition: Option<ModelTransition>,
    /// The checked task's declared priority, when present.
    pub priority: Option<u64>,
}

/// One exact bounded subtraction of a service clock by task demand.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskSubtraction {
    /// The finite typed scheduler transition template.
    pub transition: TaskTransitionTemplate,
    /// The service clock being updated.
    pub clock: String,
    /// The exact positive scaled subtraction constant.
    pub constant: Integer,
}
