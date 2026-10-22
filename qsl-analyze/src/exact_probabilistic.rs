// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-5's probabilistic product (FR-196; ADR-028 §2, SP-2, SCH-2): the finite
//! Markov model on which a probabilistic claim is decided exactly.
//!
//! [`build_product`] explores a model subject from its initial states and
//! returns, for a claim under a workload, a DTMC over FR-187's weighted
//! steps, and for a claim `under every scheduler`, an MDP over FR-187's
//! actions and draws. Each state pairs a model state with the control the
//! claim needs: FR-126's deterministic monitor for a bounded event, and an
//! accumulator for a bounded measure ([`MonitorState`], [`Accumulator`]); an
//! unbounded reachability, expected-reward or long-run form has neither, and
//! its predicates label the model states instead. Every edge keeps its
//! action, its draw and its exact probability.
//!
//! A draw with several post-states leads, over every scheduler, to an
//! intermediate state keyed by the source state, the action and the drawn
//! vector, whose edges are the post-states, each with probability 1 (SCH-2).
//! Neither the monitor nor the accumulator advances on it.
//!
//! The first state whose predicates evaluate undefined ends the build with the
//! path that reaches it ([`ProbBuild::Undefined`], ADR-028 XV-8).
//!
//! This is the untimed product. A timed subject's digital product (FR-204)
//! and the canonical byte encoding of a state key (FR-196 "Canonical
//! probability product key") belong to the checker's side and are not built
//! here.

mod build;
pub(crate) mod plan;

pub use build::build_product;

use qsl_eval::automaton::{AutomatonError, StateId};
use qsl_eval::model_subject::{ModelInstance, ModelSubject};
use qsl_eval::simulation::model::{ClauseOutcome, ScheduledIdentity};
use qsl_eval::simulation::model::{NotMarkov, UndecidedSuccessor};
use qsl_eval::simulation::{ExpansionStop, NotSimulated};
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::digest::DigestRecord;
use qsl_replay::ModelCheckLimit;
use qsl_replay::ModelCheckLimits;
use qsl_semantics::check::{CheckedProbabilisticClaim, FormulaNodeId};
use qsl_semantics::model::observation::AdmissionFailure;
use quire_exact::{CancelCause, Rational, Value};
use quire_semantic_value::catalog::InternalFault;
use std::cmp::Ordering;

/// A state handle in one product, assigned in discovery order.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProbStateId(usize);

impl ProbStateId {
    /// Zero-based discovery index; meaningful only within its product.
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Whether the product is a Markov chain or a Markov decision process.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductKind {
    /// A claim under a workload: FR-187's weighted steps.
    Dtmc,
    /// A claim `under every scheduler`: FR-187's actions and draws.
    Mdp,
}

/// What a decided bounded-event monitor concluded.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MonitorVerdict {
    /// Every continuation satisfies the event.
    Accept,
    /// No continuation satisfies the event.
    Reject,
}

/// The state of a bounded event's monitor after the position read.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FormulaState {
    /// Undecided: the monitor's state, local to the product's monitor.
    Running(StateId),
    /// Decided and absorbing.
    Decided(MonitorVerdict),
}

/// Where a bounded measure stands (FR-196 "Canonical probability product
/// key", measure control).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MeasurePhase {
    /// No position has satisfied `A` yet.
    Inactive,
    /// `A` held; `B` has not yet.
    Active,
    /// `B` ended an activated measure. Decided.
    Complete,
    /// The horizon ended an activated measure before `B`: the measure
    /// exceeds every finite threshold. Decided.
    Censored,
    /// The horizon ended with `A` never held. Decided.
    Unactivated,
}

/// The control a bounded event or measure adds to a model state.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MonitorState {
    /// A bounded event's monitor.
    Formula(FormulaState),
    /// An `accumulate` or `steps` measure: the model positions still to be
    /// read, `0..=h`, and its phase.
    Measure {
        /// The positions after this one that the window still reads.
        remaining: u64,
        /// Where the measure stands.
        phase: MeasurePhase,
    },
    /// A fraction: the included positions whose contribution is still
    /// unread, initially `h + 1`. It is complete, and decided, at 0.
    Fraction {
        /// The positions still to contribute.
        remaining: u64,
    },
}

impl MonitorState {
    /// Whether the control is decided: nothing it reads can change it, so
    /// the product does not expand the state.
    pub fn is_decided(self) -> bool {
        match self {
            Self::Formula(state) => matches!(state, FormulaState::Decided(_)),
            Self::Measure { phase, .. } => matches!(
                phase,
                MeasurePhase::Complete | MeasurePhase::Censored | MeasurePhase::Unactivated
            ),
            Self::Fraction { remaining } => remaining == 0,
        }
    }
}

/// An accumulated `steps` or `accumulate` amount, normalized so that equal
/// futures have equal keys (FR-196 "Canonical probability product key").
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Amount {
    /// The amount, at most the claim's threshold; equality to the threshold
    /// is kept exact.
    Exact(Rational),
    /// Greater than the claim's threshold: every such amount is one value.
    AboveBound,
    /// The horizon ended the measure before its `B`: greater than every
    /// finite threshold.
    Censored,
}

/// The sums a bounded measure carries.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Accumulator {
    /// `steps from A until B`.
    Steps {
        /// Whether `A` has held; false entails amount zero.
        activated: bool,
        /// The steps taken since `A`.
        amount: Amount,
    },
    /// `accumulate R from A until B`.
    Reward {
        /// Whether `A` has held; false entails amount zero.
        activated: bool,
        /// The reward summed since `A`, in the reward's own unit.
        amount: Amount,
    },
    /// An unweighted fraction.
    CountFraction {
        /// The included positions at which `P` held.
        true_count: u64,
        /// The included positions read.
        position_count: u64,
    },
    /// A fraction weighted by a reward. Both sums stay exact: `(1, 2)` and
    /// `(2, 4)` are different states.
    WeightedFraction {
        /// The weight of the positions at which `P` held.
        true_sum: Rational,
        /// The weight of the positions read.
        total_sum: Rational,
    },
}

/// A draw with several post-states, awaiting its resolution (SCH-2): the
/// scheduled identity and the drawn vector.
#[derive(Clone, Debug)]
pub struct Pending {
    /// The scheduled identity.
    pub identity: ScheduledIdentity,
    /// The drawn random values, in declared order; empty for an operation
    /// with no random parameter.
    pub random: Vec<Value>,
    /// The transition's canonical bytes, which identify the pair.
    identity_bytes: Vec<u8>,
}

impl PartialEq for Pending {
    fn eq(&self, other: &Self) -> bool {
        self.identity_bytes == other.identity_bytes
    }
}

impl Eq for Pending {}

impl PartialOrd for Pending {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Pending {
    fn cmp(&self, other: &Self) -> Ordering {
        self.identity_bytes.cmp(&other.identity_bytes)
    }
}

/// A product state's identity: the complete model state, the control the
/// claim adds to it and the draw it awaits, with the `over` binding the
/// claim's predicates were read for.
///
/// The binding is part of the key because the monitor's letters depend on
/// it: the same model state and control under two bindings have different
/// futures. The canonical byte encoding of FR-196's `probabilityProductKey`
/// is the checker's, not built here.
#[derive(Clone, Debug)]
pub struct ProbKey {
    /// The `over` binding, when the claim has one.
    pub binding: Option<ModelInstance>,
    /// FR-101's complete `quire.simulation.state-key/v1` bytes of the model
    /// state.
    pub model: Vec<u8>,
    /// The monitor or measure control; `None` for an unbounded form.
    pub monitor: Option<MonitorState>,
    /// The accumulated amounts; `None` for an event and for an unbounded
    /// form.
    pub accumulator: Option<Accumulator>,
    /// The draw awaiting resolution; `None` at a model-position state.
    pub pending: Option<Pending>,
}

impl PartialEq for ProbKey {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for ProbKey {}

impl PartialOrd for ProbKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ProbKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.binding
            .cmp(&other.binding)
            .then_with(|| self.model.cmp(&other.model))
            .then_with(|| self.monitor.cmp(&other.monitor))
            .then_with(|| self.accumulator.cmp(&other.accumulator))
            .then_with(|| self.pending.cmp(&other.pending))
    }
}

/// What an MDP chooses at a state, or the one move of a DTMC state.
#[derive(Clone, Debug)]
pub enum ProductAction {
    /// A scheduled identity (FR-187): the operation, its receiver and its
    /// non-random arguments.
    Identity(ScheduledIdentity),
    /// The resolution of a draw with several post-states: the complete
    /// target key (SCH-2).
    PostState(Box<ProbKey>),
    /// The untimed terminal stutter step (ADR-018 SM-4).
    TerminalStutter,
}

/// One product edge, with its exact probability.
#[derive(Clone, Debug)]
pub struct ProbEdge {
    /// The source state.
    pub from: ProbStateId,
    /// The choice the edge belongs to.
    pub choice: ProductAction,
    /// The edge's operation projection: the scheduled identity, none for a
    /// resolution or a stutter.
    pub action: Option<ScheduledIdentity>,
    /// The drawn random values, in declared order. Over every scheduler,
    /// draws of one action that reach the same post-state with the same
    /// rewards are one edge and carry none.
    pub draw: Option<Vec<Value>>,
    /// The target state.
    pub to: ProbStateId,
    /// The probability: the step probability under a workload, the draw's
    /// (summed over draws merged into the edge) over every scheduler, and 1
    /// at a resolution.
    pub probability: Rational,
    /// Every reward of the model for the step, in ascending name order. A
    /// resolution edge carries each reward at 0: the draw's rewards are on the
    /// edge that took the draw, once.
    pub rewards: Vec<(quire_exact::Identifier, Value)>,
}

/// A built product.
#[derive(Clone, Debug)]
pub struct ProbProduct {
    /// The states, in canonical discovery order.
    pub states: Vec<ProbKey>,
    /// Whether the product is a DTMC or an MDP.
    pub kind: ProductKind,
    /// The edges, grouped by source state in discovery order and within a
    /// source in FR-187's canonical order. A decided state has none.
    pub edges: Vec<ProbEdge>,
    /// Each initial state of the subject, per `over` binding, with the
    /// product state it starts at.
    pub initial: Vec<(u32, Option<ModelInstance>, ProbStateId)>,
    /// The states a bounded event's monitor decided, with the verdict. The
    /// control of a decided measure is in its key.
    pub decided: Vec<(ProbStateId, MonitorVerdict)>,
    /// For an unbounded form, per state, the truth of the claim's
    /// predicates at the state's model state, in claim order: `P`, `A` then
    /// `B` of a reachability claim, `B` of an expected reward, `P` of a
    /// long-run fraction. Empty for a bounded form.
    pub labels: Vec<Vec<bool>>,
}

impl ProbProduct {
    /// Every state's handle, in discovery order.
    pub fn ids(&self) -> impl Iterator<Item = ProbStateId> {
        (0..self.states.len()).map(ProbStateId)
    }

    /// The edges leaving `state`, in order.
    pub fn outgoing(&self, state: ProbStateId) -> &[ProbEdge] {
        let start = self.edges.partition_point(|edge| edge.from < state);
        let end = self.edges.partition_point(|edge| edge.from <= state);
        &self.edges[start..end]
    }
}

/// The predicate of a claim an undefined reading names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UndefinedRead {
    /// A `holds` atom of the claim's event.
    Atom(FormulaNodeId),
    /// A state predicate of the claim's measure or unbounded form.
    Predicate(PredicateRole),
}

/// Which predicate of a claim a state predicate is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PredicateRole {
    /// `A`: a measure's activation, or the left of an `until`.
    From,
    /// `B`: a measure's end, an `until`'s right, or an expected reward's
    /// target.
    Until,
    /// `P`: a fraction's, an `eventually`'s, an `always`'s or a long-run
    /// fraction's predicate.
    Target,
}

/// One model step of a path.
#[derive(Clone, Debug)]
pub struct PathStep {
    /// The scheduled identity; none for the terminal stutter step.
    pub action: Option<ScheduledIdentity>,
    /// The drawn random values; none for the terminal stutter step.
    pub draw: Option<Vec<Value>>,
    /// The step's probability.
    pub probability: Rational,
    /// Over every scheduler, when the draw has several post-states: the
    /// model state the scheduler took, as its complete state-key bytes. The
    /// path names one state only with it (SCH-2).
    pub resolved: Option<Vec<u8>>,
}

/// A path to a product state at which the claim reads an undefined value
/// (ADR-028 XV-8, ADR-018 UE-1): the refutation's evidence.
#[derive(Clone, Debug)]
pub struct UndefinedPath {
    /// The model steps from the initial state, in order.
    pub steps: Vec<PathStep>,
    /// The product of the steps' probabilities.
    pub probability: Rational,
    /// The trace position of the state: the number of steps.
    pub position: u64,
    /// The `quire.simulation.state-key/v1` digest of the model state.
    pub state_key: DigestRecord,
    /// The predicate that has no value.
    pub read: UndefinedRead,
    /// Why it has none.
    pub cause: UndefinedCause,
}

/// What a completed build found.
#[derive(Clone, Debug)]
pub enum ProbBuild {
    /// The product.
    Built(ProbProduct),
    /// The first product state, in breadth-first order, at which the claim
    /// reads an undefined value.
    Undefined(UndefinedPath),
}

/// The budget or cancellation that ended a build.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductStop {
    /// A FR-126 budget: `max_states`, `max_transitions` or
    /// `max_automaton_states`, with the value that was reached.
    Limit {
        /// The budget.
        limit: ModelCheckLimit,
        /// Its value.
        value: u64,
    },
    /// The request was cancelled.
    Cancelled(CancelCause),
}

/// Why a build produced no product and no refutation.
#[derive(Debug, thiserror::Error)]
pub enum ProductRefusal {
    /// The subject does not admit (FR-120).
    #[error("the subject does not admit: {0:?}")]
    Admission(AdmissionFailure),
    /// The `over` parameter's instances are not bounded (FR-125).
    #[error("the claim's instances are not bounded: {0:?}")]
    NotSimulated(NotSimulated),
    /// A state's expansion stopped (FR-120).
    #[error("expansion stopped: {0:?}")]
    Expansion(ExpansionStop),
    /// A drawn vector has no post-state, or several under a workload
    /// (ADR-024 PM-3).
    #[error("not a Markov chain: {0:?}")]
    NotMarkov(Box<NotMarkov>),
    /// A contract conjunction was undecided at a state (FR-187).
    #[error("a successor is undecided: {0:?}")]
    UndecidedSuccessor(UndecidedSuccessor),
    /// A claim predicate's evaluation was refused with no value undefined.
    #[error("a predicate was refused: {0:?}")]
    PredicateRefused(ClauseOutcome),
    /// The claim's event does not translate to a monitor.
    #[error("the event has no monitor: {0}")]
    Translation(AutomatonError),
    /// A budget or a cancellation ended the build.
    #[error("the build stopped: {0:?}")]
    Stopped(ProductStop),
    /// A broken internal invariant.
    #[error("internal fault: {0:?}")]
    Internal(InternalFault),
}

/// A product request (FR-196 `ExactProbRequest`, over FR-126's budgets until
/// FR-203's `ExactProbLimits` exists).
pub struct ProductRequest<'a> {
    /// The model subject (FR-125).
    pub subject: ModelSubject<'a>,
    /// The checked claim (FR-186, FR-195).
    pub claim: &'a CheckedProbabilisticClaim,
    /// `max_states`, `max_transitions` and `max_automaton_states`.
    pub limits: ModelCheckLimits,
}
