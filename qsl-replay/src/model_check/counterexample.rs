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
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::digest::{DigestRecord, WireNodeId};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_semantics::check::FormulaNodeId;
use quire_exact::{Origin, Role};
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
}

/// The expression an undefined evaluation names (QSpec FR-364 `where.locus`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UndefinedLocus {
    /// A `holds` atom of the authored clause.
    Atom(FormulaNodeId),
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
}

/// ADR-018 UE-2: where an evaluation was undefined, and why.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UndefinedEvaluation {
    /// Where.
    pub site: UndefinedSite,
    /// The undefined value's cause.
    pub cause: UndefinedCause,
}

/// What a counterexample refutes (ADR-018 DL-4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CounterexampleKind {
    /// An authored clause on which the evaluator returns false.
    Formula,
    /// The deadlock-freedom item: the prefix ends at a deadlocked state.
    Deadlock,
    /// An expression undefined at the prefix's last position.
    UndefinedEvaluation(UndefinedEvaluation),
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
pub struct TemporalCounterexample<T = ModelTransition> {
    /// The index of the behaviour's initial state in the subject's ordered
    /// list of initial states.
    pub initial_state: usize,
    /// The prefix's steps, in order.
    pub prefix: Vec<ModelStep<T>>,
    /// The instance the clause's `over` parameter is bound to, named by
    /// population and key (FR-125 Instancing); `None` for the
    /// deadlock-freedom item.
    pub over_binding: Option<ModelInstance>,
    /// The item's fairness set; empty for the deadlock-freedom item.
    pub fairness: Vec<FairnessConstraint>,
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

impl<T> TemporalCounterexample<T> {
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
    /// deadlock. A formula lasso is evaluated from its origin (ADR-018 §6),
    /// so its position is 0.
    pub fn trace_position(&self) -> Option<u64> {
        match (&self.kind, self.lasso) {
            (CounterexampleKind::Deadlock, _) => None,
            (CounterexampleKind::Formula, Some(_)) => Some(0),
            (CounterexampleKind::Formula | CounterexampleKind::UndefinedEvaluation(_), None)
            | (CounterexampleKind::UndefinedEvaluation(_), Some(_)) => {
                u64::try_from(self.prefix.len()).ok()
            }
        }
    }
}

impl<T: Clone + std::fmt::Debug + Eq> FamilyPayload for TemporalCounterexample<T> {
    /// The inline size plus each step's, each constraint's and each blocked
    /// thread's, its paths and names included.
    fn measured_bytes(&self) -> usize {
        std::mem::size_of_val(self)
            .saturating_add(
                self.prefix
                    .len()
                    .saturating_mul(std::mem::size_of::<ModelStep<T>>()),
            )
            .saturating_add(
                self.fairness
                    .len()
                    .saturating_mul(std::mem::size_of::<FairnessConstraint>()),
            )
            .saturating_add(
                self.blocked
                    .iter()
                    .map(blocked_bytes)
                    .fold(0, usize::saturating_add),
            )
    }
}

/// One blocked thread's measured size: its inline size, its path, its rest
/// point's node and the paths its wait cause names.
fn blocked_bytes(thread: &BlockedThread) -> usize {
    let cause = match &thread.cause {
        WaitCause::JoinWaiting { threads } => threads
            .iter()
            .map(path_bytes)
            .fold(0, usize::saturating_add),
        WaitCause::Precondition { clause } => clause.len(),
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
                .saturating_add(step.node.len())
                .saturating_add(step.branch.len())
        })
}
