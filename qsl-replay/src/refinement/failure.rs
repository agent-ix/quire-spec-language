// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-020 RC-1 and QSpec FR-379: the failure a refinement counterexample
//! names, and the verdict of one step check (FR-141).

use qsl_eval::simulation::protocol::{ProtocolClass, ProtocolFairness, ProtocolTransition};
use quire_semantic_value::catalog::{CatalogCode, InternalFault};
use quire_semantic_value::fairness::{FairnessConstraint, FairnessGranularity, FairnessKind};
use quire_semantic_value::model_transition::ModelTransition;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::model_check::counterexample::UndefinedEvaluation;
use crate::model_check::limits::ReachedLimit;
use crate::proof_result::InconclusiveCause;
use crate::refinement::map::{MappingRowRef, MappingUndetermined};
use crate::refinement::protocol_system::ProtocolCandidate;

/// The first check that rejected an abstract transition identity (RS-4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StepRejection {
    /// The named effective precondition is false at the pre-state.
    Precondition {
        /// The first false clause, by ascending name.
        clause: String,
    },
    /// The frame decision refuses the pair of states.
    Frame {
        /// The refusal's catalog code.
        code: CatalogCode,
    },
    /// No result makes the effective postcondition true.
    Postcondition,
}

/// A refinement counterexample's failure (ADR-020 RC-1).
///
/// The first five are safety failures, a finite prefix ending at the failing
/// step; `Divergence` and `AbstractUnfair` are liveness failures, a lasso
/// that is fair under `F_C` (FR-143).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RefinementFailure {
    /// RS-2: the concrete initial state maps to no abstract initial state.
    InitialNotAbstract {
        /// The index of the initial state in the concrete subject's list.
        initial: usize,
    },
    /// RS-3: a `stutter` step changed the mapped state.
    StutterChanged {
        /// The position of the step's post-state.
        position: u64,
    },
    /// RS-4: the abstract transition identity the step row names was
    /// rejected.
    AbstractStepRejected {
        /// The position of the step's post-state.
        position: u64,
        /// The first rejected identity in canonical order.
        transition: ModelTransition,
        /// The first check that rejected it.
        cause: StepRejection,
    },
    /// RS-5: no stutter and no abstract transition matches the step, or the
    /// candidate set after it is empty.
    NoAbstractMatch {
        /// The position of the step's post-state.
        position: u64,
    },
    /// A mapping row or step-row argument evaluated undefined (RE-5;
    /// QSpec FR-379's seventh kind).
    Undefined {
        /// The position of the step's post-state; 0 for an initial state.
        position: u64,
        /// The row.
        row: MappingRowRef,
    },
    /// RS-7: an `F_A` constraint is enabled at every mapped state of a lasso
    /// whose loop has only `stutter` steps, the concrete terminal stutter
    /// included, and no step of it takes the constraint.
    Divergence {
        /// The violated constraint of `F_A`, absent for protocol
        /// observable-divergence checks that are independent of fairness.
        constraint: Option<FairnessConstraint>,
        /// The visible abstract protocol step class enabled throughout an
        /// observable-divergence loop, when this is a protocol failure.
        protocol_class: Option<String>,
    },
    /// An `F_A` constraint is enabled at every mapped state of a lasso whose
    /// loop takes abstract steps, and no step of it takes the constraint.
    AbstractUnfair {
        /// The violated constraint of `F_A`.
        constraint: FairnessConstraint,
    },
    /// FR-379: a concrete protocol refinement refuses a label that every
    /// stable abstract candidate enables.
    RefusalNotPermitted {
        /// The position of the state, equal to the concrete prefix length.
        position: u64,
        /// The canonical visible label refused by the concrete state.
        label: ObservationLabel,
    },
    /// FR-379: concrete and abstract terminal success disagree.
    TerminalSuccessMismatch {
        /// The position of the state, equal to the concrete prefix length.
        position: u64,
        /// Which side alone marks terminal success.
        direction: TerminalSuccessDirection,
    },
}

/// FR-379's full observation label for protocol refinement refusal checks.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservationLabel {
    /// The alias-free FR-432 spelling of the protocol step class.
    pub step: String,
    /// The complete binder record in effective field order.
    pub binder: Vec<ObservationField>,
}

/// One field/value entry in an [`ObservationLabel`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservationField {
    /// The checked field name.
    pub field: String,
    /// The full typed value in FR-181's canonical representation.
    pub value: JsonValue,
}

/// Which side alone has terminal success at a product state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalSuccessDirection {
    /// The concrete model is terminal success; no abstract candidate is.
    ConcreteOnly,
    /// Every abstract candidate is terminal success; concrete is not.
    AbstractOnly,
}

impl RefinementFailure {
    /// The position the failure is at: 0 for a failed initial state, and
    /// `None` for a liveness failure, which is a lasso and no one position.
    pub fn position(&self) -> Option<u64> {
        match self {
            Self::InitialNotAbstract { .. } => Some(0),
            Self::StutterChanged { position }
            | Self::AbstractStepRejected { position, .. }
            | Self::NoAbstractMatch { position }
            | Self::Undefined { position, .. }
            | Self::RefusalNotPermitted { position, .. }
            | Self::TerminalSuccessMismatch { position, .. } => Some(*position),
            Self::Divergence { .. } | Self::AbstractUnfair { .. } => None,
        }
    }
}

/// Which abstract steps a passing concrete step took (FR-141 `Taken`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Taken {
    /// The `stutter` rule admitted it (an initial state takes no step and
    /// reads the same).
    Stutter,
    /// Every accepted abstract transition identity, in canonical order.
    Abstract(Vec<ModelTransition>),
    /// Abstract protocol transitions accepted by FR-148.
    Protocol(Vec<ProtocolTransition>),
    /// Candidate-specific protocol paths, including internal closure steps.
    /// `candidates[n]` is the state reached by `transitions[n]`.
    ProtocolTraces(Vec<ProtocolTrace>),
}

/// One admitted path through an abstract protocol candidate graph.
#[derive(Clone, Debug)]
pub struct ProtocolTrace {
    /// The candidate before the first transition.
    pub start: ProtocolCandidate,
    /// Canonical identity of the starting candidate.
    pub start_key: Vec<u8>,
    /// The transitions taken from `start`.
    pub transitions: Vec<ProtocolTransition>,
    /// The candidate after each matching transition.
    pub candidates: Vec<ProtocolCandidate>,
    /// Canonical identity of each reached candidate.
    pub candidate_keys: Vec<Vec<u8>>,
}

impl PartialEq for ProtocolTrace {
    fn eq(&self, other: &Self) -> bool {
        self.start_key == other.start_key
            && self.transitions == other.transitions
            && self.candidate_keys == other.candidate_keys
    }
}

impl Eq for ProtocolTrace {}

/// Why a step check is undetermined (ADR-020 RE-4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Undetermined {
    /// A mapping row or step-row argument was refused or incomplete.
    Mapping(Box<MappingUndetermined>),
    /// A contract conjunction of the abstract model could not be decided.
    UndecidedSuccessor,
    /// FR-148 could not establish its environment-assumption relation.
    ProtocolAssumptionNotEstablished(Box<ProtocolAssumptionWitness>),
}

impl Undetermined {
    /// The cause the item settles `inconclusive` with.
    pub fn cause(&self) -> InconclusiveCause {
        match self {
            Self::Mapping(_) => InconclusiveCause::MappingUndetermined,
            Self::UndecidedSuccessor => InconclusiveCause::UndecidedSuccessor,
            Self::ProtocolAssumptionNotEstablished(witness) => {
                InconclusiveCause::ProtocolAssumptionNotEstablished((**witness).clone())
            }
        }
    }
}

/// FR-148's typed reason for an assumption relation that cannot be established.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolAssumptionReason {
    /// A paired lasso satisfies the specification assumptions and violates
    /// an implementation assumption.
    Strengthened,
    /// An identified source fairness predicate is undetermined at a reachable
    /// paired state, and no violating lasso is available.
    RelationUndetermined,
}

/// Which admitted system owns one environment assumption.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssumptionSide {
    /// Concrete implementation assumptions (`assume`).
    Implementation,
    /// Abstract protocol assumptions.
    Specification,
}

/// The source of one typed environment assumption.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssumptionOrigin {
    /// An authored checked fairness constraint.
    Authored,
    /// Derived protocol scheduler fairness.
    Scheduler,
    /// Derived protocol memory fairness.
    Memory,
}

/// One assumption's checked target and optional selected member identity.
/// The canonical bytes preserve the source identity without replacing it with
/// a display name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssumptionIdentity {
    /// Which side owns the assumption.
    pub side: AssumptionSide,
    /// How the source supplied it.
    pub origin: AssumptionOrigin,
    /// Weak or strong fairness.
    pub kind: FairnessKind,
    /// Whole-class or each-member fairness.
    pub granularity: FairnessGranularity,
    /// The exact source-checked target and fairness identity.
    pub target: AssumptionTarget,
    /// The selected application or thread identity for an `each` member.
    pub selected: Option<AssumptionSelectedMember>,
}

/// The typed checked declaration that owns an assumption.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssumptionTarget {
    /// An authored concrete `assume` row.
    Model(FairnessConstraint),
    /// An authored or derived protocol fairness member.
    Protocol(ProtocolFairness),
    /// An authored operation-fairness row on a directly required temporal
    /// clause. The key and checked row retain the clause and operation
    /// identity that supplied the protocol-side assumption.
    RequiredTemporal {
        /// The required temporal clause's emitted node identity.
        clause: quire_exact::NodeKey,
        /// The clause's checked operation fairness row.
        constraint: FairnessConstraint,
        /// The corresponding operation class over protocol transitions.
        fairness: ProtocolFairness,
    },
}

/// A selected class identity for an `each` fairness member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssumptionSelectedMember {
    /// A concrete operation application identity.
    Model(ModelTransition),
    /// A protocol application or thread identity.
    Protocol(ProtocolClass),
}

/// One edge of a paired implementation/protocol path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairedAssumptionStep {
    /// The concrete model transition, or `None` for its terminal stutter.
    pub concrete: Option<ModelTransition>,
    /// The concrete post-state's canonical key.
    pub concrete_post: qsl_eval::simulation::StateKey,
    /// The protocol transition's canonical identity, or `None` for an
    /// internal concrete step.
    pub protocol: Option<Vec<u8>>,
    /// The protocol post-state's canonical state key.
    pub protocol_post: Vec<u8>,
}

/// Paired path evidence for FR-148's assumption relation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairedAssumptionPath {
    /// The concrete initial-state key.
    pub concrete_initial: qsl_eval::simulation::StateKey,
    /// The protocol initial-state key.
    pub protocol_initial: Vec<u8>,
    /// Steps after the initial states.
    pub steps: Vec<PairedAssumptionStep>,
    /// The loop entry for a strengthened-assumption lasso, absent for finite
    /// relation-undetermined prefixes.
    pub loop_entry: Option<usize>,
}

/// The typed product outcome when assumption weakening is not established.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolAssumptionWitness {
    /// Why the relation was not established.
    pub reason: ProtocolAssumptionReason,
    /// Exact source assumptions involved in the decision.
    pub identities: Vec<AssumptionIdentity>,
    /// The paired path or lasso that establishes the outcome.
    pub path: PairedAssumptionPath,
    /// Canonical checked predicate locus for relation-undetermined, if any.
    pub undecided_locus: Option<String>,
}

/// What `check_step` returns: the verdict and the number of abstract
/// transition identities it judged or expanded, which FR-142 adds to the
/// run's `max_transitions`.
#[derive(Clone, Debug)]
pub struct StepResult {
    /// The verdict.
    pub verdict: StepVerdict,
    /// The abstract transitions judged or expanded.
    pub abstract_successors: u64,
}

/// What deciding one concrete initial state or step gave (FR-141).
#[derive(Clone, Debug)]
pub enum StepVerdict {
    /// The initial state or step is an abstract one.
    Passes {
        /// The position after it.
        post: crate::refinement::RefinementPosition,
        /// Which abstract steps it took.
        taken: Taken,
    },
    /// The refinement does not hold at it.
    Fails(RefinementFailure),
    /// A mapping row or step-row argument evaluated undefined: the
    /// refinement is refuted here (ADR-020 RE-5).
    Undefined(UndefinedEvaluation),
    /// The check could not decide.
    Undetermined(Undetermined),
    /// A protocol transition's checked contract was undetermined while
    /// closing an admitted protocol step; retain the candidate-specific
    /// trace so the product can build a paired relation witness.
    ProtocolContractUndetermined {
        /// The candidate whose successor contract could not be evaluated.
        candidate: ProtocolCandidate,
        /// The operation transition whose checked contract was undetermined.
        transition: ModelTransition,
        /// The exact checked contract clause that was undefined.
        clause: String,
        /// The candidate-specific path from the product's protocol candidate.
        trace: ProtocolTrace,
    },
    /// A run budget stopped the check: the evaluation meter, the candidate
    /// bound, or a result domain too large to order. It is no fault of QSL
    /// and no verdict; the run stops (FR-142).
    Stopped(ReachedLimit),
    /// An internal fault of the evaluator, or of the calling engine code
    /// that passed a system that is not the refinement's own. It is not a
    /// verdict: the item settles `failed` (QSpec FR-383).
    Fault(InternalFault),
}
