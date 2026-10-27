// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126 "Counterexamples" and FR-128 Inputs (ADR-018 CX-1, CX-2, DL-4,
//! UE-2): a model counterexample, the payload of a
//! `WitnessEnvelope<TemporalCounterexample>`.
//!
//! This is QSpec FR-364's model arm: the finite-prefix safety, deadlock and
//! undefined-evaluation refutations that EN-1's first phase returns, and the
//! lasso a liveness refutation returns from its second phase ([`Lasso`]). A
//! bounded clause's `interval` is not carried yet: no engine in this crate's
//! callers builds one.

use qsl_eval::model_subject::ModelInstance;
use qsl_eval::simulation::model::ModelFinding;
use qsl_eval::simulation::protocol::{BlockedThread, ThreadPath, WaitCause};
use qsl_eval::simulation::Location;
use qsl_eval::simulation::SubjectTransition;
use qsl_eval::temporal::TemporalPosition;
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::bound::TemporalInterval;
use qsl_foundation::digest::{DigestRecord, WireNodeId};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_semantics::check::FormulaNodeId;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{Origin, Rational, Role};
use quire_semantic_value::fairness::FairnessConstraint;
use quire_semantic_value::model_transition::ModelTransition;

use crate::refinement::{MappingRowRef, ObjectKey, RefinementFailure};
use crate::witness::FamilyPayload;

/// The occurrence role of the derived deadlock-freedom item.
const DEADLOCK_FREEDOM_ROLE: &str = "deadlock-freedom";

/// The envelope identity of the deadlock-freedom item of the package
/// `package_id` (FR-128 "The envelope names the clause, or the
/// deadlock-freedom item, by its clause node and occurrence key"): the
/// item has no authored clause node, so its node is the package's own
/// identity, and its occurrence key is that node with the role
/// `deadlock-freedom` and ordinal 0. No authored clause has this node,
/// since a clause node is the digest of a clause's checked content and
/// never a `package_id`. The subject is bound by the envelope's obligation
/// identity (FR-124), not here.
pub fn deadlock_freedom_clause(package_id: &DigestRecord) -> (WireNodeId, OccurrenceKey) {
    let node = WireNodeId::from_digest(*package_id.as_bytes());
    let occurrence = OccurrenceKey::new(node, Origin::new(Role::new(DEADLOCK_FREEDOM_ROLE), 0));
    (node, occurrence)
}

/// The occurrence role of the derived time-lock-freedom item.
const TIME_LOCK_FREEDOM_ROLE: &str = "time-lock-freedom";

/// The envelope identity of the time-lock-freedom item of the package
/// `package_id` (FR-232), as [`deadlock_freedom_clause`] names the
/// deadlock-freedom item's: its node is the package's own identity and its
/// occurrence key is that node with the role `time-lock-freedom` and
/// ordinal 0.
pub fn time_lock_freedom_clause(package_id: &DigestRecord) -> (WireNodeId, OccurrenceKey) {
    let node = WireNodeId::from_digest(*package_id.as_bytes());
    let occurrence = OccurrenceKey::new(node, Origin::new(Role::new(TIME_LOCK_FREEDOM_ROLE), 0));
    (node, occurrence)
}

/// The occurrence role of the derived race-freedom item.
const RACE_FREEDOM_ROLE: &str = "race-freedom";

/// The envelope identity of the race-freedom item of the package
/// `package_id`, as [`deadlock_freedom_clause`] names the deadlock-freedom
/// item's: its node is the package's own identity and its occurrence key is
/// that node with the role `race-freedom` and ordinal 0. The subject and its
/// resolved models are bound by the envelope's obligation identity
/// (FR-224), not here.
pub fn race_freedom_clause(package_id: &DigestRecord) -> (WireNodeId, OccurrenceKey) {
    let node = WireNodeId::from_digest(*package_id.as_bytes());
    let occurrence = OccurrenceKey::new(node, Origin::new(Role::new(RACE_FREEDOM_ROLE), 0));
    (node, occurrence)
}

/// One step of a behaviour: the transition identity it took (FR-120's for a
/// model subject, ADR-027 TS-2's for a protocol subject) and its
/// post-state's `quire.simulation.state-key/v1` digest, which selects the
/// successor among the post-states of that identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelStep<T = ModelTransition> {
    /// The step's transition identity.
    pub transition: T,
    /// The post-state's state-key digest.
    pub post_state: DigestRecord,
    /// QSpec FR-181's `delay` (FR-236): over a timed subject the exact time
    /// that elapses before the step, 0 under a tick source; `None` over an
    /// untimed subject.
    pub delay: Option<Rational>,
}

/// The expression an undefined evaluation names (QSpec FR-364 `where.locus`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UndefinedLocus {
    /// A `holds` atom of the authored clause.
    Atom(FormulaNodeId),
    /// The `when` condition of the authored clause's guarded `on each`
    /// activation.
    Activation,
    /// The initializer of a capture of the authored clause (FR-337 "Capture
    /// observation"), by the capture's name.
    Capture(String),
    /// A `pre` or `post` clause of an application's contract conjunction,
    /// by its name.
    Contract(String),
    /// The `terminal when` predicate declared for the model alias.
    Terminal(String),
    /// The request's state constraint (ADR-021 SC-2), by its clause name.
    Constraint(String),
    /// A protocol step's constraint (ADR-027 TS-4), by its node's static
    /// declaration path.
    Protocol(String),
    /// A refinement's mapping row or step-row argument (ADR-020 RE-5).
    Mapping(Box<MappingSite>),
    /// A state predicate of a probabilistic claim's bounded measure,
    /// fraction or long-run fraction (ADR-024 PF-2, PF-6), by its role;
    /// a `holds` atom of a claim's temporal event is an [`Self::Atom`].
    ClaimPredicate(ClaimPredicate),
}

/// Which state predicate of a probabilistic claim's measure an undefined
/// evaluation names (ADR-024 PF-2, PF-6).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ClaimPredicate {
    /// `A`: an `accumulate` or `steps` measure's activation.
    From,
    /// `B`: an `accumulate` or `steps` measure's end.
    Until,
    /// `P`: a fraction's or a long-run fraction's predicate.
    Target,
}

/// The row and object a refinement's undefined mapping names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MappingSite {
    /// The row.
    pub row: MappingRowRef,
    /// The concrete object the row was evaluated for, when it is evaluated
    /// for one.
    pub object: Option<ObjectKey>,
}

/// QSpec FR-364's `where` of an undefined evaluation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UndefinedSite {
    /// The trace position: the prefix's last position.
    pub position: u64,
    /// The state-key digest of the state at `position`.
    pub state_key: DigestRecord,
    /// The expression that has no value.
    pub locus: UndefinedLocus,
    /// For an undefined guard only: the transition identity it guards.
    pub transition: Option<ModelTransition>,
    /// Over a timed subject, the position's time stamp and exact clock
    /// valuation (ADR-026 TV-1); `None` over an untimed subject.
    pub time: Option<Box<TimedPoint>>,
}

/// A position's time over a timed subject: its time stamp and the exact
/// value of each present clock place, in the subject's place order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimedPoint {
    /// The time stamp.
    pub stamp: Rational,
    /// The clock valuation.
    pub clocks: Vec<Rational>,
}

/// ADR-018 UE-2: where an evaluation was undefined, and why.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UndefinedEvaluation {
    /// Where.
    pub site: UndefinedSite,
    /// The undefined value's cause.
    pub cause: UndefinedCause,
}

/// ADR-025 DR-5: the data race a race-freedom counterexample reports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Race {
    /// The shared location both accesses touch.
    pub location: Location,
    /// The position of the earlier conflicting access: the position the
    /// trace reaches by the step that made it.
    pub earlier: u64,
    /// The position of the step that completes the race: the prefix's last.
    pub later: u64,
}

/// What a counterexample refutes (ADR-018 DL-4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CounterexampleKind {
    /// An authored clause on which the evaluator returns false.
    Formula,
    /// The deadlock-freedom item: the prefix ends at a deadlocked state.
    Deadlock,
    /// The race-freedom item (FR-224): the prefix ends at the step that
    /// completes a data race.
    Race(Race),
    /// An expression undefined at the prefix's last position.
    UndefinedEvaluation(UndefinedEvaluation),
    /// The time-lock-freedom item (FR-232, ADR-026 TD-1): after the prefix
    /// and [`TemporalCounterexample::final_delay`], the state admits no
    /// time-divergent continuation.
    TimeLock,
}

/// The loop of a liveness refutation (ADR-018 CX-1): where the behaviour
/// re-enters, and whether the loop is the terminal stutter step.
///
/// The steps of the lasso are [`TemporalCounterexample::prefix`]: the stem,
/// then the loop. The loop re-enters at the position `entry`, so its steps
/// are the prefix's steps from `entry` on and the last post-state of the
/// prefix is the state at position `entry`. A loop that is the terminal
/// stutter step (ADR-018 SM-4) has no model step: `entry` is the prefix's
/// length, the position of the terminal state, and the marker is set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Lasso {
    /// The position the loop re-enters, in `0..=prefix.len()`.
    pub entry: usize,
    /// Whether the loop is the one terminal stutter step.
    pub terminal_stutter: bool,
}

/// A counterexample over a model subject, or over a protocol subject with
/// protocol transition identities (ADR-027 PX-1): a finite prefix, or with
/// a [`Lasso`] the stem and loop of a liveness refutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalCounterexample<T: SubjectTransition = ModelTransition> {
    /// The index of the behaviour's initial state in the subject's ordered
    /// list of initial states.
    pub initial_state: usize,
    /// The prefix's steps, in order.
    pub prefix: Vec<ModelStep<T>>,
    /// The instance the clause's `over` parameter is bound to, named by
    /// population and key (FR-125 Instancing); `None` for the
    /// deadlock-freedom item.
    pub over_binding: Option<ModelInstance>,
    /// The item's fairness set; empty for the deadlock-freedom item. Over a
    /// protocol subject each member carries its target and origin (FR-212).
    pub fairness: Vec<T::Fairness>,
    /// What the counterexample refutes.
    pub kind: CounterexampleKind,
    /// The loop of a liveness refutation; `None` for a finite prefix.
    pub lasso: Option<Lasso>,
    /// For a deadlock over a protocol subject, the waiting threads of the
    /// last state (ADR-027 PD-4); empty otherwise.
    pub blocked: Vec<BlockedThread>,
    /// The failure of a refinement item (ADR-020 RC-1); `None` for every
    /// other item.
    pub refinement: Option<RefinementFailure>,
    /// QSpec FR-417's `final_delay` of a time-lock counterexample (FR-236):
    /// the delay after the last step that reaches the time-locked state;
    /// `None` on every other kind.
    pub final_delay: Option<Rational>,
}

/// ADR-018 UE-6 and ADR-027 TS-4: the first finding of an expansion that
/// evaluated undefined, in the order the expansion records them: a contract
/// conjunction, with the transition its guard was evaluated for, or a
/// protocol step's constraint, by its node.
pub fn undefined_finding(
    findings: &[ModelFinding],
) -> Option<(UndefinedLocus, Option<ModelTransition>, UndefinedCause)> {
    findings.iter().find_map(|finding| {
        if let Some(contract) = finding.undefined_contract() {
            return Some((
                UndefinedLocus::Contract(contract.clause),
                contract.guard_of,
                contract.cause,
            ));
        }
        finding
            .undefined_protocol_expression()
            .map(|(node, cause)| (UndefinedLocus::Protocol(node), None, cause))
    })
}

impl<T: SubjectTransition> TemporalCounterexample<T> {
    /// The counterexample's length: its number of transitions, stem and loop
    /// together (ADR-018 "Length"). A terminal stutter loop is one transition
    /// (ADR-018 SM-4) that [`Self::prefix`] does not list.
    pub fn len(&self) -> usize {
        self.prefix.len().saturating_add(usize::from(
            self.lasso.is_some_and(|lasso| lasso.terminal_stutter),
        ))
    }

    /// Whether the counterexample has no transition: it ends at an initial
    /// state.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// QSpec FR-364's `trace_position`: the prefix's last position for a
    /// formula and an undefined-evaluation counterexample, `None` for a
    /// deadlock and a time-lock. A formula lasso is evaluated from its
    /// origin (ADR-018 §6), so its position is 0.
    pub fn trace_position(&self) -> Option<u64> {
        match (&self.kind, self.lasso) {
            (CounterexampleKind::Deadlock | CounterexampleKind::TimeLock, _) => None,
            (CounterexampleKind::Formula, Some(_)) => Some(0),
            (
                CounterexampleKind::Formula
                | CounterexampleKind::Race(_)
                | CounterexampleKind::UndefinedEvaluation(_),
                None,
            )
            | (CounterexampleKind::Race(_) | CounterexampleKind::UndefinedEvaluation(_), Some(_)) => {
                u64::try_from(self.prefix.len()).ok()
            }
        }
    }
}

impl<T: SubjectTransition> FamilyPayload for TemporalCounterexample<T> {
    /// The inline size plus each step's, each constraint's and each blocked
    /// thread's, its paths and names included. A delay counts only where
    /// it is present, so an untimed counterexample measures what it did
    /// before FR-236 gave steps a `delay` (SR-1598001 FND-003).
    fn measured_bytes(&self) -> usize {
        let delay = std::mem::size_of::<Option<Rational>>();
        let present = |value: &Option<Rational>| value.as_ref().map_or(0, |_| delay);
        std::mem::size_of_val(self)
            .saturating_sub(delay)
            .saturating_add(present(&self.final_delay))
            .saturating_add(self.prefix.iter().fold(0_usize, |bytes, step| {
                bytes.saturating_add(
                    std::mem::size_of::<ModelStep<T>>()
                        .saturating_sub(delay)
                        .saturating_add(present(&step.delay)),
                )
            }))
            .saturating_add(
                self.fairness
                    .len()
                    .saturating_mul(std::mem::size_of::<T::Fairness>()),
            )
            .saturating_add(
                self.blocked
                    .iter()
                    .map(blocked_bytes)
                    .fold(0, usize::saturating_add),
            )
    }
}

/// One step in QSpec FR-364's observed temporal counterexample. The
/// `document` is a digest-addressed admitted snapshot; its bytes are
/// supplied by the replay request's byte provision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservedTemporalStep {
    /// The observed snapshot at this trace position.
    pub document: DocumentRef,
}

/// The refutation kind carried by an observed temporal counterexample.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservedTemporalKind {
    /// A formula evaluated false.
    Formula,
    /// An undefined formula evaluation, naming its first undefined atom.
    UndefinedEvaluation {
        /// The expression that had no value.
        locus: FormulaNodeId,
        /// Why that expression had no value.
        cause: UndefinedCause,
    },
}

/// QSpec FR-364's observed arm of the typed temporal-counterexample
/// payload (FR-331). It shares the envelope with the model arm while keeping
/// its distinct trace representation: observed document references, never
/// model states or transitions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservedTemporalCounterexample {
    /// The finite prefix, in trace-position order.
    pub prefix: Vec<ObservedTemporalStep>,
    /// The loop's first represented position, or `None` for a finite trace.
    pub loop_start: Option<u64>,
    /// The checked clause's fairness set, in canonical order.
    pub fairness: Vec<FairnessConstraint>,
    /// The failing operator interval for a formula counterexample.
    pub interval: Option<TemporalInterval>,
    /// What the packet claims the evaluator found.
    pub kind: ObservedTemporalKind,
    /// The position whose value the packet reports.
    pub trace_position: TemporalPosition,
}

impl FamilyPayload for ObservedTemporalCounterexample {
    fn measured_bytes(&self) -> usize {
        std::mem::size_of_val(self)
            .saturating_add(
                self.prefix
                    .len()
                    .saturating_mul(std::mem::size_of::<ObservedTemporalStep>()),
            )
            .saturating_add(
                self.fairness
                    .len()
                    .saturating_mul(std::mem::size_of::<FairnessConstraint>()),
            )
            .saturating_add(self.prefix.iter().fold(0_usize, |bytes, step| {
                bytes
                    .saturating_add(step.document.authority.len())
                    .saturating_add(step.document.identity.len())
                    .saturating_add(step.document.revision_namespace.len())
                    .saturating_add(step.document.revision.len())
            }))
            .saturating_add(self.fairness.iter().fold(0_usize, |bytes, constraint| {
                bytes.saturating_add(constraint.operation().name().len())
            }))
    }
}

/// One blocked thread's measured size: its inline size, its path, its rest
/// point's node and the paths its wait cause names.
fn blocked_bytes(thread: &BlockedThread) -> usize {
    let cause = match &thread.cause {
        WaitCause::JoinWaiting { threads } | WaitCause::FinishWaiting { threads } => threads
            .iter()
            .map(path_bytes)
            .fold(0, usize::saturating_add),
        WaitCause::Precondition { clause } => clause.len(),
        WaitCause::EmptyQueue { channel }
        | WaitCause::HeadBlocked { channel }
        | WaitCause::FullQueue { channel }
        | WaitCause::Displaced { channel } => channel.len(),
        WaitCause::CheckFalse { node } => node.len(),
        WaitCause::CommitClosed { template } => template.len(),
        WaitCause::NoRoleInstance { role } => role.len(),
        WaitCause::NoPostState | WaitCause::Constraint | WaitCause::Gate => 0,
    };
    std::mem::size_of::<BlockedThread>()
        .saturating_add(path_bytes(&thread.path))
        .saturating_add(thread.rest_point.node.as_ref().map_or(0, String::len))
        .saturating_add(cause)
}

/// A thread path's measured size: its inline size and each step's.
fn path_bytes(path: &ThreadPath) -> usize {
    path.steps
        .iter()
        .fold(std::mem::size_of::<ThreadPath>(), |bytes, step| {
            bytes
                .saturating_add(std::mem::size_of_val(step))
                .saturating_add(step.text_len())
        })
}
