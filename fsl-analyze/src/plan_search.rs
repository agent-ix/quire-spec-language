// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-650 and FR-800 to FR-804: bounded, metered, certified plan
//! search over a native state model (FR-755, FR-1011, FR-1012).
//!
//! The search is separate from `fsl-replay`'s proof-composition planner and
//! from the verification planner. It enumerates operation plans over
//! [`ModelSystem`] up to the declared plan length in FSpec FR-801's
//! canonical sequential exploration, keys every search node by the
//! canonical state key, the policy monitor states and the step count, and
//! hands the caller-evaluated goal, objective and policy rules to the
//! [`PlanRules`] it is given. Every reported plan is replayed natively
//! ([`replay`]) and every certificate is checked by [`certificate::check`],
//! a function separate from the search, before a result claims `optimal`
//! or `infeasible`.

pub mod certificate;
pub mod replay;

use std::collections::{BTreeMap, BTreeSet};

use filament_exact::{Cancel, CancelCause, LimitKind, Rational};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::simulation::model::{ModelFinding, ModelSimState, ModelSystem};
use fsl_eval::simulation::{ExpansionLimit, ExpansionStop, TransitionSystem};

pub use certificate::{Certificate, CertificateKind, CertificateOutcome, CertificateRule};

/// One FSpec FR-801 planner limit member.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PlanLimit {
    /// `max_states`: one per distinct search node admitted.
    MaxStates,
    /// `max_work`: steps applied, monitor updates, goal and objective
    /// evaluations and frontier comparisons.
    MaxWork,
    /// `max_certificate_entries`: one per certificate entry written.
    MaxCertificateEntries,
    /// `max_check_work`: one per entry read and one per successor
    /// re-derived by the certificate checker.
    MaxCheckWork,
    /// `max_state_key_bytes`: the byte length of one canonical state key.
    MaxStateKeyBytes,
}

impl PlanLimit {
    /// Every member, in FSpec FR-801's table order.
    pub const ALL: [Self; 5] = [
        Self::MaxStates,
        Self::MaxWork,
        Self::MaxCertificateEntries,
        Self::MaxCheckWork,
        Self::MaxStateKeyBytes,
    ];

    /// The bare member name a plan request's `limits` object uses.
    pub const fn member(self) -> &'static str {
        match self {
            Self::MaxStates => "max_states",
            Self::MaxWork => "max_work",
            Self::MaxCertificateEntries => "max_certificate_entries",
            Self::MaxCheckWork => "max_check_work",
            Self::MaxStateKeyBytes => "max_state_key_bytes",
        }
    }

    /// The `plan_search.<member>` setting name (FSpec FR-461 Behavior 5).
    pub const fn setting(self) -> &'static str {
        match self {
            Self::MaxStates => "plan_search.max_states",
            Self::MaxWork => "plan_search.max_work",
            Self::MaxCertificateEntries => "plan_search.max_certificate_entries",
            Self::MaxCheckWork => "plan_search.max_check_work",
            Self::MaxStateKeyBytes => "plan_search.max_state_key_bytes",
        }
    }

    /// The member named `member`, if it is one of FR-801's.
    pub fn from_member(member: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|limit| limit.member() == member)
    }

    /// The member whose setting is `setting`, if it is one of FR-801's.
    pub fn from_setting(setting: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|limit| limit.setting() == setting)
    }

    /// FSpec FR-801's published default for this member.
    pub const fn default_value(self) -> u64 {
        match self {
            Self::MaxStates => 10_000_000,
            Self::MaxWork | Self::MaxCheckWork => 100_000_000,
            Self::MaxCertificateEntries => 16_777_216,
            Self::MaxStateKeyBytes => 1_048_576,
        }
    }
}

/// FSpec FR-801's planner limits, every member at least 1.
///
/// [`Default`] gives the published defaults; [`Self::set`] replaces one
/// member for one request and leaves the others at their values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlanSearchLimits {
    max_states: u64,
    max_work: u64,
    max_certificate_entries: u64,
    max_check_work: u64,
    max_state_key_bytes: u64,
}

impl Default for PlanSearchLimits {
    fn default() -> Self {
        Self {
            max_states: PlanLimit::MaxStates.default_value(),
            max_work: PlanLimit::MaxWork.default_value(),
            max_certificate_entries: PlanLimit::MaxCertificateEntries.default_value(),
            max_check_work: PlanLimit::MaxCheckWork.default_value(),
            max_state_key_bytes: PlanLimit::MaxStateKeyBytes.default_value(),
        }
    }
}

impl PlanSearchLimits {
    /// Replace `limit` with `value`.
    pub fn set(&mut self, limit: PlanLimit, value: std::num::NonZeroU64) {
        *self.slot(limit) = value.get();
    }

    /// The value in force for `limit`.
    pub const fn get(&self, limit: PlanLimit) -> u64 {
        match limit {
            PlanLimit::MaxStates => self.max_states,
            PlanLimit::MaxWork => self.max_work,
            PlanLimit::MaxCertificateEntries => self.max_certificate_entries,
            PlanLimit::MaxCheckWork => self.max_check_work,
            PlanLimit::MaxStateKeyBytes => self.max_state_key_bytes,
        }
    }

    /// Every member in force by setting name, defaults filled (FSpec
    /// FR-805 `limits`).
    pub fn by_setting(&self) -> BTreeMap<&'static str, u64> {
        PlanLimit::ALL
            .into_iter()
            .map(|limit| (limit.setting(), self.get(limit)))
            .collect()
    }

    fn slot(&mut self, limit: PlanLimit) -> &mut u64 {
        match limit {
            PlanLimit::MaxStates => &mut self.max_states,
            PlanLimit::MaxWork => &mut self.max_work,
            PlanLimit::MaxCertificateEntries => &mut self.max_certificate_entries,
            PlanLimit::MaxCheckWork => &mut self.max_check_work,
            PlanLimit::MaxStateKeyBytes => &mut self.max_state_key_bytes,
        }
    }
}

/// The plan lengths the declaration's `domain` admits: an integer range
/// with both bounds and no negative value (FSpec FR-650, FR-806).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlanLengths {
    min: u64,
    max: u64,
}

impl PlanLengths {
    /// The lengths `min..=max`; `None` when `min > max`.
    pub const fn new(min: u64, max: u64) -> Option<Self> {
        if min > max {
            None
        } else {
            Some(Self { min, max })
        }
    }

    /// Whether a plan of `length` steps is a value of the domain.
    pub const fn contains(&self, length: u64) -> bool {
        self.min <= length && length <= self.max
    }

    /// `max D`, the longest plan explored.
    pub const fn max(&self) -> u64 {
        self.max
    }
}

/// The objective's direction (FSpec FR-802 "Comparison").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    /// Smaller values improve.
    Minimize,
    /// Greater values improve.
    Maximize,
}

/// FSpec FR-802's classification of the objective.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectiveKind {
    /// A sum of per-step costs (including `length`, whose cost is 1 per
    /// step): [`PlanRules::step_cost`] supplies each term.
    StepSum,
    /// Any other objective: [`PlanRules::objective`] evaluates it on a
    /// completed plan.
    General,
}

/// A policy monitor's verdict after reading one position.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MonitorStep {
    /// The prefix is still admissible; the monitor's next canonical state.
    Continue(Vec<u8>),
    /// A safety monitor rejected the prefix, which is pruned (FSpec FR-803).
    Reject,
}

/// A plan, or a plan prefix: its states from the initial state and the
/// step between each adjacent pair.
#[derive(Clone, Copy, Debug)]
pub struct PlanView<'p> {
    /// The initial state followed by each step's post-state.
    pub states: &'p [ModelSimState],
    /// The steps, in plan order.
    pub steps: &'p [ModelTransition],
}

impl PlanView<'_> {
    /// The number of steps.
    pub fn length(&self) -> usize {
        self.steps.len()
    }
}

/// Why an evaluation the search or the checker asked for stopped it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuleStop {
    /// The evaluation meter denied a charge (`accounting.<counter>`).
    Accounting {
        /// The counter, e.g. `work_units`.
        counter: &'static str,
        /// The bound in force.
        value: u64,
    },
    /// The run was cancelled.
    Cancelled(CancelCause),
}

/// The goal, objective and policy rules of one plan synthesis, evaluated
/// by the caller over its checked expressions and the evaluation meter.
///
/// Every method that evaluates an expression returns `Ok(false)` or
/// `Ok(None)` when the evaluation is undefined (FSpec FR-800 "The goal",
/// FR-802), and [`RuleStop`] when a meter or cancellation stops it.
pub trait PlanRules {
    /// FSpec FR-804's step order key: operation declaration order, then the
    /// arguments in their canonical value order.
    type StepKey: Ord + Clone;

    /// The objective's direction.
    fn direction(&self) -> Direction;

    /// The objective's FR-802 classification.
    fn objective_kind(&self) -> ObjectiveKind;

    /// Whether prefixes with equal search nodes may be merged: false when
    /// the goal or the objective reads `steps` other than through a
    /// per-step sum (FSpec FR-801 "Search nodes and completeness").
    fn merges_prefixes(&self) -> bool;

    /// The FR-804 order key of `step`.
    fn step_key(&self, step: &ModelTransition) -> Self::StepKey;

    /// The number of policy clauses, in the request's `policy` order.
    fn policy_count(&self) -> usize;

    /// Policy `policy`'s monitor after reading the initial state.
    ///
    /// # Errors
    /// [`RuleStop`] when the meter or a cancellation stops the evaluation.
    fn policy_start(
        &mut self,
        policy: usize,
        state: &ModelSimState,
    ) -> Result<MonitorStep, RuleStop>;

    /// Policy `policy`'s monitor after reading `state` from `monitor`.
    ///
    /// # Errors
    /// [`RuleStop`] when the meter or a cancellation stops the evaluation.
    fn policy_advance(
        &mut self,
        policy: usize,
        monitor: &[u8],
        state: &ModelSimState,
    ) -> Result<MonitorStep, RuleStop>;

    /// Whether policy `policy` holds on a completed plan whose monitor is
    /// `monitor` after its last state `last`, read with the terminal
    /// stutter of `last` (FSpec FR-803).
    ///
    /// # Errors
    /// [`RuleStop`] when the meter or a cancellation stops the evaluation.
    fn policy_accepts(
        &mut self,
        policy: usize,
        monitor: &[u8],
        last: &ModelSimState,
    ) -> Result<bool, RuleStop>;

    /// Whether `plan` reaches the goal.
    ///
    /// # Errors
    /// [`RuleStop`] when the meter or a cancellation stops the evaluation.
    fn goal(&mut self, plan: &PlanView<'_>) -> Result<bool, RuleStop>;

    /// The cost of step `index` of `plan`, for a [`ObjectiveKind::StepSum`]
    /// objective; `None` when it is undefined.
    ///
    /// # Errors
    /// [`RuleStop`] when the meter or a cancellation stops the evaluation.
    fn step_cost(
        &mut self,
        plan: &PlanView<'_>,
        index: usize,
    ) -> Result<Option<Rational>, RuleStop>;

    /// Add a step cost to its prefix total under the objective's numeric
    /// semantics and meter. `plan` includes the new step. An authored
    /// domain violation returns `None`; a resource denial returns a stop.
    fn accumulate_step_cost(
        &mut self,
        _plan: &PlanView<'_>,
        total: &Rational,
        cost: &Rational,
    ) -> Result<Option<Rational>, RuleStop> {
        self.add_objective_values(total, cost)
    }

    /// Add exact objective projections while constructing or checking a
    /// certificate. Evaluating implementations charge their numeric meter.
    fn add_objective_values(
        &mut self,
        left: &Rational,
        right: &Rational,
    ) -> Result<Option<Rational>, RuleStop> {
        Ok(Some(left.add(right)))
    }

    /// The constant offset of a per-step objective, evaluated independently
    /// of its steps. It is added once, including for an empty plan.
    fn objective_offset(&mut self, _plan: &PlanView<'_>) -> Result<Option<Rational>, RuleStop> {
        Ok(Some(zero()))
    }

    /// Complete a per-step objective from its accumulated costs and its
    /// constant offset. Implementations with typed arithmetic apply that
    /// arithmetic and its meter here, preserving authored domain failures.
    fn step_sum_objective(
        &mut self,
        plan: &PlanView<'_>,
        total: &Rational,
    ) -> Result<Option<Rational>, RuleStop> {
        match self.objective_offset(plan)? {
            Some(offset) => self.add_objective_values(total, &offset),
            None => Ok(None),
        }
    }

    /// The objective value of a completed `plan`, for an
    /// [`ObjectiveKind::General`] objective; `None` when it is undefined.
    ///
    /// # Errors
    /// [`RuleStop`] when the meter or a cancellation stops the evaluation.
    fn objective(&mut self, plan: &PlanView<'_>) -> Result<Option<Rational>, RuleStop>;

    /// Called before the native replay of a reported plan. The replay runs
    /// under its own allowance (FSpec FR-801, FR-650): an implementation
    /// whose evaluation meter or cancellation stopped the search restores a
    /// fresh meter for the replay's evaluations here, so a stopped search
    /// still reports its best plan. The default changes nothing.
    fn begin_replay(&mut self) {}
}

/// FR-755's two library test seams, never reachable from a request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanFault {
    /// Replace the reported plan with these operations (by name, each
    /// applied as the first enabled invocation of that operation in step
    /// order) before native replay.
    ReportPlan(Vec<String>),
    /// Remove the `successor`th successor record (zero-based, in step
    /// order) of certificate entry `entry` before the core checker reads it.
    DropSuccessor {
        /// The entry's index in the certificate.
        entry: usize,
        /// The record's index in that entry.
        successor: usize,
    },
    /// Replace the state of certificate entry `entry` with the state of
    /// entry `source`, leaving its key alone, before the core checker reads
    /// it. A library test seam for the checker's key check, beyond FR-650's
    /// two named seams.
    ForgeEntryState {
        /// The entry whose state is replaced.
        entry: usize,
        /// The entry whose state is copied.
        source: usize,
    },
    /// Point the parent record of certificate entry `entry` at the entry
    /// itself before the core checker reads it. A library test seam for the
    /// checker's parent check, beyond FR-650's two named seams.
    ParentCycle {
        /// The entry whose parent record is replaced.
        entry: usize,
    },
}

/// FSpec FR-650's result kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanStatus {
    /// A plan whose optimality certificate the core accepted.
    Optimal,
    /// A plan whose optimality is not certified.
    Feasible,
    /// No admissible plan, by a closure certificate the core accepted.
    Infeasible,
    /// A limit, a cancellation or a rejected closure certificate stopped
    /// the search before a definite answer.
    Incomplete,
}

/// Why a search ended `incomplete` (FSpec FR-801, FR-650).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanStopCause {
    /// A planner limit or an evaluation-meter counter denied a charge.
    LimitReached {
        /// The member or counter name, e.g. `max_states`.
        limit: &'static str,
        /// The bound in force.
        value: u64,
        /// Its setting name, e.g. `plan_search.max_states`.
        setting: String,
    },
    /// A caller cancellation or the request deadline.
    Cancelled(CancelCause),
    /// The core rejected the closure certificate for `infeasible`.
    CertificateRejected {
        /// The first failing rule.
        rule: CertificateRule,
        /// The entry it failed at.
        entry: usize,
    },
    /// A state was pruned because an invariant could not be decided there,
    /// so no complete exploration, and no certificate resting on it, exists.
    InvariantUndetermined {
        /// The invariant's name.
        invariant: String,
        /// The canonical key of the first such state.
        state: Vec<u8>,
    },
}

/// The search and objective-exclusion counts (FSpec FR-801, FR-802).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlanExplored {
    /// Search nodes admitted.
    pub states: u64,
    /// `max_work` units granted.
    pub work: u64,
    /// Plans excluded for a NaN objective.
    pub nan_objectives: u64,
    /// Plans excluded for an undefined objective.
    pub undefined_objectives: u64,
}

/// A natively replayed plan.
#[derive(Clone, Debug)]
pub struct ReportedPlan {
    /// The initial state followed by every post-state.
    pub states: Vec<ModelSimState>,
    /// The steps, in plan order.
    pub steps: Vec<ModelTransition>,
}

impl ReportedPlan {
    /// The plan as a [`PlanView`].
    pub fn view(&self) -> PlanView<'_> {
        PlanView {
            states: &self.states,
            steps: &self.steps,
        }
    }
}

/// One plan search's result (FSpec FR-650 "Result").
#[derive(Clone, Debug)]
pub struct PlanResult {
    /// The result kind.
    pub status: PlanStatus,
    /// The reported plan, replayed natively; none when no admissible plan
    /// is reported.
    pub plan: Option<ReportedPlan>,
    /// The reported plan's exact objective value.
    pub objective: Option<Rational>,
    /// The certificate the search wrote and the core checked, when one was.
    pub certificate: Option<CertificateReport>,
    /// Whether the core accepted the result's certificate (`certified`).
    pub certified: bool,
    /// The search counts.
    pub explored: PlanExplored,
    /// The stop cause of an `incomplete` result.
    pub cause: Option<PlanStopCause>,
    /// The index of the first negative step cost, which reclassified a
    /// per-step sum as general for this run (FSpec FR-802).
    pub negative_step: Option<usize>,
}

/// The certificate summary an outcome document carries (FSpec FR-805).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertificateReport {
    /// `potential` or `closure`.
    pub kind: CertificateKind,
    /// The number of entries.
    pub entries: u64,
    /// The core checker's verdict.
    pub outcome: CertificateOutcome,
}

/// A plan search's inputs other than the model and its rules.
#[derive(Clone, Copy, Debug)]
pub struct PlanSearchRequest<'r> {
    /// The plan lengths the declaration's domain admits.
    pub lengths: PlanLengths,
    /// The planner limits in force.
    pub limits: PlanSearchLimits,
    /// The run's cancellation, which a deadline also trips.
    pub cancel: &'r Cancel,
    /// The test seam to apply, when a library caller passes one.
    pub fault: Option<&'r PlanFault>,
}

/// A plan search that could not produce a result.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PlanSearchError {
    /// The model has not exactly one initial state.
    #[error("a plan starts from exactly one bound initial state; the model has {0}")]
    InitialStates(usize),
    /// A canonical state key could not be encoded.
    #[error("a state key could not be encoded")]
    StateKey,
    /// A transition key could not be encoded.
    #[error("a transition key could not be encoded")]
    TransitionKey,
    /// Native expansion broke an invariant of the model system.
    #[error("native model expansion failed: {0:?}")]
    Expansion(ExpansionStop),
    /// A reported plan failed native replay: a method failure naming the
    /// first failing step (FSpec FR-650, exit 30).
    #[error("plan replay failed at step {step}: `{operation}` is not enabled there")]
    ReplayFailed {
        /// The zero-based index of the failing step.
        step: usize,
        /// The operation's name.
        operation: String,
    },
    /// A replayed plan breaks a policy or misses the goal.
    #[error("plan replay failed: {0}")]
    ReplayRejected(&'static str),
    /// An evaluation the replay asked for was stopped by the meter or a
    /// cancellation. Never a method failure: the run settles `incomplete`.
    #[error("plan replay was stopped: {0:?}")]
    RuleStopped(RuleStop),
    /// A plan passes through a state whose invariant cannot be decided: it
    /// is not admissible evidence, and the run settles `incomplete`.
    #[error("plan replay reached a state whose invariant `{0}` is undetermined")]
    InvariantUndetermined(String),
}

/// A search node's identity (FSpec FR-801 "Search nodes").
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct NodeKey {
    pub(crate) state: Vec<u8>,
    pub(crate) monitors: Vec<Vec<u8>>,
    pub(crate) depth: u64,
    /// The accumulated cost, kept when cost dominance does not apply.
    pub(crate) accumulated: Option<Rational>,
    /// A distinct prefix index, when prefixes are never merged.
    pub(crate) prefix: Option<usize>,
}

/// One admitted search node.
#[derive(Clone, Debug)]
pub(crate) struct Node<K> {
    pub(crate) key: NodeKey,
    pub(crate) state: ModelSimState,
    pub(crate) monitors: Vec<Vec<u8>>,
    pub(crate) depth: u64,
    /// The representative prefix's accumulated per-step cost.
    pub(crate) accumulated: Rational,
    /// The representative prefix: parent node and step.
    pub(crate) parent: Option<(usize, ModelTransition)>,
    /// The representative prefix's FR-804 step keys.
    pub(crate) prefix: Vec<K>,
    /// The node's goal verdict and objective value, when it was evaluated
    /// and holds.
    pub(crate) goal: Option<Rational>,
    /// Successor records in step order: step, successor node index and the
    /// step's cost (zero for a general objective).
    pub(crate) successors: Vec<(ModelTransition, usize, Rational)>,
    pub(crate) expanded: bool,
}

/// The search's internal state, shared with the certificate writer.
pub(crate) struct Search<'s, 'a, R: PlanRules> {
    pub(crate) system: &'s ModelSystem<'a>,
    pub(crate) rules: &'s mut R,
    pub(crate) request: PlanSearchRequest<'s>,
    pub(crate) kind: ObjectiveKind,
    pub(crate) dominance: bool,
    pub(crate) nodes: Vec<Node<R::StepKey>>,
    pub(crate) index: BTreeMap<NodeKey, usize>,
    pub(crate) explored: PlanExplored,
    pub(crate) negative_step: Option<usize>,
    /// The first state pruned for an undetermined invariant: its canonical
    /// key and the invariant's name.
    pub(crate) undecided: Option<(Vec<u8>, String)>,
}

/// What a state's invariants say about it.
pub(crate) enum Admission {
    /// Every invariant holds.
    Admitted,
    /// An invariant is violated.
    Violated,
    /// No invariant is violated and the named one cannot be decided.
    Undetermined(String),
}

/// Classify `state` by its invariant findings; a violation outweighs an
/// undetermined invariant.
pub(crate) fn admission(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
) -> Result<Admission, ExpansionStop> {
    let mut undetermined = None;
    for finding in system.invariant_findings_at(state)? {
        match finding {
            ModelFinding::InvariantUndetermined { clause, .. } => {
                undetermined.get_or_insert(clause);
            }
            _ => return Ok(Admission::Violated),
        }
    }
    Ok(undetermined.map_or(Admission::Admitted, Admission::Undetermined))
}

/// One candidate invocation of a state: its step, and the post-state when
/// the step is enabled.
struct Candidate<K> {
    key: K,
    step: ModelTransition,
    post: Option<ModelSimState>,
}

/// A charge or an evaluation stopped the search.
enum Stop {
    Cause(PlanStopCause),
    /// A negative per-step cost reclassifies the objective (FSpec FR-802).
    Negative(usize),
    Error(PlanSearchError),
}

impl From<RuleStop> for Stop {
    fn from(stop: RuleStop) -> Self {
        Self::Cause(stop_cause(stop))
    }
}

/// The `incomplete` cause of a stopped rule evaluation.
pub(crate) fn stop_cause(stop: RuleStop) -> PlanStopCause {
    match stop {
        RuleStop::Accounting { counter, value } => PlanStopCause::LimitReached {
            limit: counter,
            value,
            setting: format!("accounting.{counter}"),
        },
        RuleStop::Cancelled(cause) => PlanStopCause::Cancelled(cause),
    }
}

impl From<PlanSearchError> for Stop {
    fn from(error: PlanSearchError) -> Self {
        Self::Error(error)
    }
}

/// Search the plan space of `system` under `rules` and `request`
/// (FSpec FR-650, FR-801's canonical sequential exploration).
///
/// # Errors
/// [`PlanSearchError`] for a model that cannot be searched and for a
/// reported plan that fails native replay (a method failure).
pub fn search<R: PlanRules>(
    system: &ModelSystem<'_>,
    rules: &mut R,
    request: PlanSearchRequest<'_>,
) -> Result<PlanResult, PlanSearchError> {
    let initial = system.initial();
    if initial.len() != 1 {
        return Err(PlanSearchError::InitialStates(initial.len()));
    }
    let kind = rules.objective_kind();
    let mut dominance = kind == ObjectiveKind::StepSum && rules.merges_prefixes();
    let mut negative_step = None;
    loop {
        let mut search = Search {
            system,
            rules: &mut *rules,
            request,
            kind,
            dominance,
            nodes: Vec::new(),
            index: BTreeMap::new(),
            explored: PlanExplored::default(),
            negative_step,
            undecided: None,
        };
        match search.explore() {
            Ok(()) => return search.settle(),
            Err(Stop::Negative(step)) if negative_step.is_none() => {
                // FSpec FR-802: a negative step cost reclassifies the
                // objective as general for this run; the search restarts
                // keeping the accumulated value in every node.
                negative_step = Some(step);
                dominance = false;
            }
            Err(Stop::Negative(_)) => return search.settle(),
            Err(Stop::Cause(cause)) => return search.stopped(cause),
            Err(Stop::Error(error)) => return Err(error),
        }
    }
}

impl<R: PlanRules> Search<'_, '_, R> {
    /// FSpec FR-801 "Charge order" step 5: grant one `limit` unit or deny.
    fn charge(&mut self, limit: PlanLimit) -> Result<(), Stop> {
        if let Some(cause) = self.request.cancel.cause() {
            return Err(Stop::Cause(PlanStopCause::Cancelled(cause)));
        }
        let bound = self.request.limits.get(limit);
        let count = match limit {
            PlanLimit::MaxStates => &mut self.explored.states,
            PlanLimit::MaxWork => &mut self.explored.work,
            PlanLimit::MaxCertificateEntries
            | PlanLimit::MaxCheckWork
            | PlanLimit::MaxStateKeyBytes => {
                return Ok(());
            }
        };
        if *count >= bound {
            return Err(Stop::Cause(limit_reached(limit, bound)));
        }
        *count += 1;
        Ok(())
    }

    fn state_key(&self, state: &ModelSimState) -> Result<Vec<u8>, Stop> {
        let (key, _) = self
            .system
            .state_key(state)
            .map_err(|_| Stop::Error(PlanSearchError::StateKey))?;
        let bound = self.request.limits.get(PlanLimit::MaxStateKeyBytes);
        if u64::try_from(key.len()).map_or(true, |length| length > bound) {
            return Err(Stop::Cause(limit_reached(
                PlanLimit::MaxStateKeyBytes,
                bound,
            )));
        }
        Ok(key)
    }

    fn explore(&mut self) -> Result<(), Stop> {
        let Some(start) = self.system.initial().into_iter().next() else {
            return Err(Stop::Error(PlanSearchError::InitialStates(0)));
        };
        // FSpec FR-801 "Charge order" step 4: the initial node's admission
        // is charged before its monitors are read.
        self.charge(PlanLimit::MaxStates)?;
        if !self.admissible(&start)? {
            // The initial state breaks an invariant (FSpec FR-650): no plan
            // is admissible and the space is empty.
            return Ok(());
        }
        let mut monitors = Vec::with_capacity(self.rules.policy_count());
        for policy in 0..self.rules.policy_count() {
            self.charge(PlanLimit::MaxWork)?;
            match self.rules.policy_start(policy, &start)? {
                MonitorStep::Continue(state) => monitors.push(state),
                // The initial state breaks a safety policy: no plan is
                // admissible and the space is empty.
                MonitorStep::Reject => return Ok(()),
            }
        }
        let key = NodeKey {
            state: self.state_key(&start)?,
            monitors: monitors.clone(),
            depth: 0,
            accumulated: (!self.dominance && self.kind == ObjectiveKind::StepSum).then(zero),
            prefix: (!self.rules.merges_prefixes()).then_some(0),
        };
        self.index.insert(key.clone(), 0);
        self.nodes.push(Node {
            key,
            state: start,
            monitors,
            depth: 0,
            accumulated: zero(),
            parent: None,
            prefix: Vec::new(),
            goal: None,
            successors: Vec::new(),
            expanded: false,
        });
        self.evaluate_goal(0)?;
        let mut layer = vec![0];
        for _ in 0..self.request.lengths.max() {
            self.order_layer(&mut layer);
            let mut next = Vec::new();
            for &node in &layer {
                self.expand(node, &mut next)?;
            }
            if next.is_empty() {
                break;
            }
            layer = next;
        }
        Ok(())
    }

    /// FSpec FR-801 "Order within a layer": accumulated value for a
    /// per-step sum, then the representative prefix in step order.
    pub(crate) fn order_layer(&self, layer: &mut [usize]) {
        let direction = self.rules.direction();
        let sum = self.kind == ObjectiveKind::StepSum;
        layer.sort_by(|left, right| {
            let left = &self.nodes[*left];
            let right = &self.nodes[*right];
            let by_value = if sum {
                match direction {
                    Direction::Minimize => left.accumulated.cmp(&right.accumulated),
                    Direction::Maximize => right.accumulated.cmp(&left.accumulated),
                }
            } else {
                std::cmp::Ordering::Equal
            };
            by_value.then_with(|| left.prefix.cmp(&right.prefix))
        });
    }

    /// Whether `state` is admissible (FSpec FR-650): every state clause and
    /// invariant of the model holds. A violated invariant makes it
    /// inadmissible. An invariant that cannot be decided is not a
    /// violation: the state is not expanded, the first such prune is
    /// recorded, and the run can then only settle `incomplete`.
    fn admissible(&mut self, state: &ModelSimState) -> Result<bool, Stop> {
        match admission(self.system, state).map_err(|stop| expansion_stop(stop, self.system))? {
            Admission::Admitted => Ok(true),
            Admission::Violated => Ok(false),
            Admission::Undetermined(invariant) => {
                if self.undecided.is_none() {
                    let key = self.state_key(state)?;
                    self.undecided = Some((key, invariant));
                }
                Ok(false)
            }
        }
    }

    /// Every candidate invocation of `state`, enabled or not, in FSpec
    /// FR-804's step order.
    fn candidates(&self, state: &ModelSimState) -> Result<Vec<Candidate<R::StepKey>>, Stop> {
        let expansion = self
            .system
            .successors(state)
            .map_err(|stop| expansion_stop(stop, self.system))?;
        let identities = self
            .system
            .transition_identities(state)
            .map_err(|stop| expansion_stop(stop, self.system))?;
        let mut candidates: Vec<_> = expansion
            .successors
            .into_iter()
            .map(|(step, post)| Candidate {
                key: self.rules.step_key(&step),
                step,
                post: Some(post),
            })
            .collect();
        let enabled: BTreeSet<ModelTransition> = candidates
            .iter()
            .map(|candidate| candidate.step.clone())
            .collect();
        for step in identities {
            if !enabled.contains(&step) {
                candidates.push(Candidate {
                    key: self.rules.step_key(&step),
                    step,
                    post: None,
                });
            }
        }
        candidates.sort_by(|left, right| left.key.cmp(&right.key));
        Ok(candidates)
    }

    fn expand(&mut self, parent: usize, next: &mut Vec<usize>) -> Result<(), Stop> {
        let candidates = self.candidates(&self.nodes[parent].state)?;
        self.nodes[parent].expanded = true;
        let depth = self.nodes[parent].depth + 1;
        for Candidate {
            key: step_key,
            step,
            post,
        } in candidates
        {
            // Applying the step: precondition, post-state and admission. A
            // disabled step is charged and stops here (FSpec FR-801).
            self.charge(PlanLimit::MaxWork)?;
            let Some(state) = post else {
                continue;
            };
            if !self.admissible(&state)? {
                continue;
            }
            let mut monitors = Vec::with_capacity(self.nodes[parent].monitors.len());
            let mut rejected = false;
            for policy in 0..self.rules.policy_count() {
                self.charge(PlanLimit::MaxWork)?;
                let previous = self.nodes[parent].monitors[policy].clone();
                match self.rules.policy_advance(policy, &previous, &state)? {
                    MonitorStep::Continue(monitor) => monitors.push(monitor),
                    MonitorStep::Reject => {
                        rejected = true;
                        break;
                    }
                }
            }
            if rejected {
                continue;
            }
            let mut prefix = self.nodes[parent].prefix.clone();
            prefix.push(step_key);
            let cost = match self.kind {
                ObjectiveKind::StepSum => {
                    let (states, steps) = self.prefix_with(parent, &step, &state);
                    let view = PlanView {
                        states: &states,
                        steps: &steps,
                    };
                    let Some(cost) = self.rules.step_cost(&view, steps.len() - 1)? else {
                        // An undefined step cost makes every completion's
                        // objective undefined: the prefix is not admissible.
                        self.explored.undefined_objectives += 1;
                        continue;
                    };
                    if cost < zero() && self.negative_step.is_none() {
                        return Err(Stop::Negative(steps.len() - 1));
                    }
                    cost
                }
                ObjectiveKind::General => zero(),
            };
            let accumulated = if self.kind == ObjectiveKind::StepSum {
                let (states, steps) = self.prefix_with(parent, &step, &state);
                let view = PlanView {
                    states: &states,
                    steps: &steps,
                };
                let Some(total) = self.rules.accumulate_step_cost(
                    &view,
                    &self.nodes[parent].accumulated,
                    &cost,
                )?
                else {
                    self.explored.undefined_objectives += 1;
                    continue;
                };
                total
            } else {
                zero()
            };
            let key = NodeKey {
                state: self.state_key(&state)?,
                monitors: monitors.clone(),
                depth,
                accumulated: (!self.dominance && self.kind == ObjectiveKind::StepSum)
                    .then(|| accumulated.clone()),
                prefix: (!self.rules.merges_prefixes()).then_some(self.nodes.len()),
            };
            if let Some(&existing) = self.index.get(&key) {
                // Frontier comparison with the admitted node.
                self.charge(PlanLimit::MaxWork)?;
                self.nodes[parent]
                    .successors
                    .push((step.clone(), existing, cost.clone()));
                if self.dominance && self.better(&accumulated, &self.nodes[existing].accumulated) {
                    // Cost dominance: the strictly better prefix becomes the
                    // representative. The goal depends on the node alone, so
                    // only a goal node's per-step sum value changes.
                    let was_goal = self.nodes[existing].goal.is_some();
                    let node = &mut self.nodes[existing];
                    node.accumulated = accumulated;
                    node.parent = Some((parent, step));
                    node.prefix = prefix;
                    if was_goal {
                        self.evaluate_goal_value(existing)?;
                    }
                }
                continue;
            }
            self.charge(PlanLimit::MaxStates)?;
            let index = self.nodes.len();
            self.index.insert(key.clone(), index);
            self.nodes.push(Node {
                key,
                state,
                monitors,
                depth,
                accumulated,
                parent: Some((parent, step.clone())),
                prefix,
                goal: None,
                successors: Vec::new(),
                expanded: false,
            });
            self.nodes[parent].successors.push((step, index, cost));
            self.evaluate_goal(index)?;
            if depth < self.request.lengths.max() {
                next.push(index);
            }
        }
        Ok(())
    }

    fn better(&self, candidate: &Rational, kept: &Rational) -> bool {
        match self.rules.direction() {
            Direction::Minimize => candidate < kept,
            Direction::Maximize => candidate > kept,
        }
    }

    /// The representative prefix of `parent` extended by one step.
    fn prefix_with(
        &self,
        parent: usize,
        step: &ModelTransition,
        state: &ModelSimState,
    ) -> (Vec<ModelSimState>, Vec<ModelTransition>) {
        let (mut states, mut steps) = self.prefix_of(parent);
        states.push(state.clone());
        steps.push(step.clone());
        (states, steps)
    }

    /// The representative prefix that admitted node `index`.
    pub(crate) fn prefix_of(&self, index: usize) -> (Vec<ModelSimState>, Vec<ModelTransition>) {
        prefix_of(&self.nodes, index)
    }

    /// The goal evaluation of a node whose step count is in `D`, and its
    /// objective evaluation when the goal holds.
    fn evaluate_goal(&mut self, index: usize) -> Result<(), Stop> {
        if !self.request.lengths.contains(self.nodes[index].depth) {
            return Ok(());
        }
        self.charge(PlanLimit::MaxWork)?;
        if !self.goal_holds(index)? {
            return Ok(());
        }
        self.charge(PlanLimit::MaxWork)?;
        self.evaluate_goal_value(index)
    }

    fn goal_holds(&mut self, index: usize) -> Result<bool, Stop> {
        let (states, steps) = self.prefix_of(index);
        let view = PlanView {
            states: &states,
            steps: &steps,
        };
        if !self.rules.goal(&view)? {
            return Ok(false);
        }
        let monitors = self.nodes[index].monitors.clone();
        for (policy, monitor) in monitors.iter().enumerate() {
            if !self
                .rules
                .policy_accepts(policy, monitor, &self.nodes[index].state)?
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn evaluate_goal_value(&mut self, index: usize) -> Result<(), Stop> {
        let value = match self.kind {
            ObjectiveKind::StepSum => {
                let (states, steps) = self.prefix_of(index);
                self.rules.step_sum_objective(
                    &PlanView {
                        states: &states,
                        steps: &steps,
                    },
                    &self.nodes[index].accumulated,
                )?
            }
            ObjectiveKind::General => {
                let (states, steps) = self.prefix_of(index);
                self.rules.objective(&PlanView {
                    states: &states,
                    steps: &steps,
                })?
            }
        };
        match value {
            Some(value) => self.nodes[index].goal = Some(value),
            None => self.explored.undefined_objectives += 1,
        }
        Ok(())
    }

    /// The best goal node in FSpec FR-804's plan order: objective, then
    /// length, then the steps.
    pub(crate) fn best(&self) -> Option<usize> {
        let direction = self.rules.direction();
        (0..self.nodes.len())
            .filter(|index| self.nodes[*index].goal.is_some())
            .min_by(|left, right| {
                let (left, right) = (&self.nodes[*left], &self.nodes[*right]);
                let by_value = match direction {
                    Direction::Minimize => left.goal.cmp(&right.goal),
                    Direction::Maximize => right.goal.cmp(&left.goal),
                };
                by_value
                    .then_with(|| left.depth.cmp(&right.depth))
                    .then_with(|| left.prefix.cmp(&right.prefix))
            })
    }

    /// The best plan found so far, replayed, for an `incomplete` result.
    fn stopped(mut self, cause: PlanStopCause) -> Result<PlanResult, PlanSearchError> {
        let best = self.best();
        let (plan, objective) = match best {
            Some(index) => match self.replayed(index) {
                Ok((plan, value)) => (Some(plan), Some(value)),
                // A plan that cannot be replayed even under the replay's own
                // allowance is not reported; the stop is the search's own.
                Err(
                    PlanSearchError::RuleStopped(_) | PlanSearchError::InvariantUndetermined(_),
                ) => (None, None),
                Err(error) => return Err(error),
            },
            None => (None, None),
        };
        Ok(PlanResult {
            status: PlanStatus::Incomplete,
            plan,
            objective,
            certificate: None,
            certified: false,
            explored: self.explored,
            cause: Some(cause),
            negative_step: self.negative_step,
        })
    }

    /// Replay the plan of node `index`, or the substituted plan.
    ///
    /// The replay runs under its own allowance ([`PlanRules::begin_replay`])
    /// and recomputes the plan's objective; a value that differs from the
    /// one the search found for the node is a method failure.
    fn replayed(&mut self, index: usize) -> Result<(ReportedPlan, Rational), PlanSearchError> {
        self.rules.begin_replay();
        let lengths = self.request.lengths;
        let (plan, value) = match self.request.fault {
            Some(PlanFault::ReportPlan(operations)) => replay::substitute(
                self.system,
                &mut *self.rules,
                operations,
                lengths,
                self.kind,
            )?,
            _ => {
                let (states, steps) = self.prefix_of(index);
                let plan = ReportedPlan { states, steps };
                let value =
                    replay::replay(self.system, &mut *self.rules, &plan, lengths, self.kind)?;
                (plan, value)
            }
        };
        if self.nodes[index].goal.as_ref() != Some(&value) {
            return Err(PlanSearchError::ReplayRejected(
                "the replayed objective differs from the certified value",
            ));
        }
        Ok((plan, value))
    }

    /// Build the result of a complete exploration.
    fn settle(mut self) -> Result<PlanResult, PlanSearchError> {
        if let Some((state, invariant)) = self.undecided.take() {
            // A state was pruned for an undetermined invariant, so the
            // exploration is not known complete: never `optimal` or
            // `infeasible`; a plan found is carried as the best plan.
            return self.stopped(PlanStopCause::InvariantUndetermined { invariant, state });
        }
        let best = self.best();
        let potential = self.kind == ObjectiveKind::StepSum
            && self.rules.direction() == Direction::Minimize
            && self.negative_step.is_none();
        let kind = if best.is_some() && potential {
            CertificateKind::Potential
        } else {
            CertificateKind::Closure
        };
        let mut written = match certificate::write(&mut self, kind) {
            Ok(written) => written,
            Err(limit) => return self.stopped(limit),
        };
        match self.request.fault {
            Some(PlanFault::DropSuccessor { entry, successor }) => {
                written.drop_successor(*entry, *successor);
            }
            Some(PlanFault::ForgeEntryState { entry, source }) => {
                written.forge_state(*entry, *source);
            }
            Some(PlanFault::ParentCycle { entry }) => written.parent_cycle(*entry),
            Some(PlanFault::ReportPlan(_)) | None => {}
        }
        let entries = written.entries();
        let goal_value = best.and_then(|index| self.nodes[index].goal.clone());
        let outcome = match certificate::check(
            self.system,
            &mut *self.rules,
            &written,
            self.request.lengths,
            self.request.limits,
            self.request.cancel,
            goal_value.as_ref(),
        ) {
            Ok(outcome) => outcome,
            Err(stop) => return self.stopped(stop),
        };
        let report = CertificateReport {
            kind,
            entries,
            outcome: outcome.clone(),
        };
        let accepted = outcome == CertificateOutcome::Accepted;
        let Some(best) = best else {
            if accepted {
                return Ok(PlanResult {
                    status: PlanStatus::Infeasible,
                    plan: None,
                    objective: None,
                    certificate: Some(report),
                    certified: true,
                    explored: self.explored,
                    cause: None,
                    negative_step: self.negative_step,
                });
            }
            let CertificateOutcome::Rejected { rule, entry } = outcome else {
                unreachable!("an unaccepted certificate is rejected");
            };
            return Ok(PlanResult {
                status: PlanStatus::Incomplete,
                plan: None,
                objective: None,
                certificate: Some(report),
                certified: false,
                explored: self.explored,
                cause: Some(PlanStopCause::CertificateRejected { rule, entry }),
                negative_step: self.negative_step,
            });
        };
        let (plan, value) = match self.replayed(best) {
            Ok(replayed) => replayed,
            // FSpec FR-801: a stop is `incomplete`, never a method failure.
            Err(PlanSearchError::RuleStopped(stop)) => {
                return Ok(PlanResult {
                    status: PlanStatus::Incomplete,
                    plan: None,
                    objective: None,
                    certificate: Some(report),
                    certified: false,
                    explored: self.explored,
                    cause: Some(stop_cause(stop)),
                    negative_step: self.negative_step,
                });
            }
            Err(PlanSearchError::InvariantUndetermined(invariant)) => {
                return self.stopped(PlanStopCause::InvariantUndetermined {
                    invariant,
                    state: Vec::new(),
                });
            }
            Err(error) => return Err(error),
        };
        let objective = Some(value);
        Ok(PlanResult {
            status: if accepted {
                PlanStatus::Optimal
            } else {
                PlanStatus::Feasible
            },
            plan: Some(plan),
            objective,
            certificate: Some(report),
            certified: accepted,
            explored: self.explored,
            cause: None,
            negative_step: self.negative_step,
        })
    }
}

pub(crate) fn prefix_of<K>(
    nodes: &[Node<K>],
    mut index: usize,
) -> (Vec<ModelSimState>, Vec<ModelTransition>) {
    let mut states = vec![nodes[index].state.clone()];
    let mut steps = Vec::new();
    while let Some((parent, step)) = &nodes[index].parent {
        steps.push(step.clone());
        index = *parent;
        states.push(nodes[index].state.clone());
    }
    states.reverse();
    steps.reverse();
    (states, steps)
}

pub(crate) fn zero() -> Rational {
    Rational::from_integer(filament_exact::Integer::from(0_i64))
}

pub(crate) fn limit_reached(limit: PlanLimit, value: u64) -> PlanStopCause {
    PlanStopCause::LimitReached {
        limit: limit.member(),
        value,
        setting: limit.setting().to_owned(),
    }
}

/// An expansion stop: an evaluation-meter denial names its counter; any
/// other stop is a failure of the model system.
fn expansion_stop(stop: ExpansionStop, system: &ModelSystem<'_>) -> Stop {
    match stop.limit {
        Some(ExpansionLimit::EvaluationMeter(kind)) => {
            let counter = kind.as_str();
            let limits = system.meter_limits();
            let value = match kind {
                LimitKind::IntegerBits => limits.integer_bits,
                LimitKind::DecimalDigits => limits.decimal_digits,
                LimitKind::ScaleExpansion => limits.scale_expansion,
                LimitKind::TextInputBytes => limits.text_input_bytes,
                LimitKind::TextScalars => limits.text_scalars,
                LimitKind::NormalizedScalars => limits.normalized_scalars,
                LimitKind::UnitEdges => limits.unit_edges,
                LimitKind::ValueOccurrences => limits.value_occurrences,
                LimitKind::WorkUnits => limits.work_units,
                LimitKind::ResultUnits => limits.result_units,
                // A session high-water limit with no request setting.
                LimitKind::LiveOccurrences => u64::MAX,
            };
            Stop::Cause(PlanStopCause::LimitReached {
                limit: counter,
                value,
                setting: format!("accounting.{counter}"),
            })
        }
        Some(ExpansionLimit::MaxCandidates) | None => Stop::Error(PlanSearchError::Expansion(stop)),
    }
}
