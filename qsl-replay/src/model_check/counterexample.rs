// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126 "Counterexamples" and FR-128 Inputs (ADR-018 CX-1, CX-2, DL-4,
//! UE-2): a model counterexample, the payload of a
//! `WitnessEnvelope<TemporalCounterexample>`.
//!
//! This is the finite-prefix shape of QSpec FR-364's model arm: the safety,
//! deadlock and undefined-evaluation refutations that EN-1's first phase
//! returns. A liveness lasso's `loop` and a bounded clause's `interval` are
//! not carried yet: no engine in this crate's callers builds either.

use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::digest::{DigestRecord, WireNodeId};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_semantics::check::FormulaNodeId;
use quire_exact::{ObjectReference, Origin, Role};
use quire_semantic_value::fairness::FairnessConstraint;
use quire_semantic_value::model_transition::ModelTransition;

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

/// One step of a model behaviour: the FR-120 transition identity it took
/// and its post-state's `quire.simulation.state-key/v1` digest, which
/// selects the successor among the post-states of that identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelStep {
    /// The step's transition identity.
    pub transition: ModelTransition,
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

/// A finite-prefix counterexample over a model subject.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalCounterexample {
    /// The index of the behaviour's initial state in the subject's ordered
    /// list of initial states.
    pub initial_state: usize,
    /// The prefix's steps, in order.
    pub prefix: Vec<ModelStep>,
    /// The object the clause's `over` parameter is bound to; `None` for the
    /// deadlock-freedom item.
    pub over_binding: Option<ObjectReference>,
    /// The item's fairness set; empty for the deadlock-freedom item.
    pub fairness: Vec<FairnessConstraint>,
    /// What the counterexample refutes.
    pub kind: CounterexampleKind,
}

impl TemporalCounterexample {
    /// The counterexample's length: its number of transitions (ADR-018
    /// "Length").
    pub fn len(&self) -> usize {
        self.prefix.len()
    }

    /// Whether the counterexample has no transition: it ends at an initial
    /// state.
    pub fn is_empty(&self) -> bool {
        self.prefix.is_empty()
    }

    /// QSpec FR-364's `trace_position`: the prefix's last position for a
    /// formula and an undefined-evaluation counterexample, `None` for a
    /// deadlock.
    pub fn trace_position(&self) -> Option<u64> {
        match &self.kind {
            CounterexampleKind::Deadlock => None,
            CounterexampleKind::Formula | CounterexampleKind::UndefinedEvaluation(_) => {
                u64::try_from(self.prefix.len()).ok()
            }
        }
    }
}

impl FamilyPayload for TemporalCounterexample {
    /// The inline size plus each step's and each constraint's.
    fn measured_bytes(&self) -> usize {
        std::mem::size_of_val(self)
            .saturating_add(
                self.prefix
                    .len()
                    .saturating_mul(std::mem::size_of::<ModelStep>()),
            )
            .saturating_add(
                self.fairness
                    .len()
                    .saturating_mul(std::mem::size_of::<FairnessConstraint>()),
            )
    }
}
