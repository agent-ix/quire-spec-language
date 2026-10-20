// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-020 RC-1 and QSpec FR-379: the failure a refinement counterexample
//! names, and the verdict of one step check (FR-141).

use qsl_eval::simulation::ExpansionStop;
use qsl_foundation::{CatalogCode, InternalFault};
use quire_semantic_value::model_transition::ModelTransition;

use crate::model_check::counterexample::UndefinedEvaluation;
use crate::proof_result::InconclusiveCause;
use crate::refinement::map::{MappingRowRef, MappingUndetermined};

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
/// The safety kinds are here; the liveness kinds, `Divergence` and
/// `AbstractUnfair`, arrive with FR-143.
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
}

impl RefinementFailure {
    /// The position the failure is at: 0 for a failed initial state.
    pub fn position(&self) -> u64 {
        match self {
            Self::InitialNotAbstract { .. } => 0,
            Self::StutterChanged { position }
            | Self::AbstractStepRejected { position, .. }
            | Self::NoAbstractMatch { position }
            | Self::Undefined { position, .. } => *position,
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
}

/// Why a step check is undetermined (ADR-020 RE-4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Undetermined {
    /// A mapping row or step-row argument was refused or incomplete.
    Mapping(Box<MappingUndetermined>),
    /// A contract conjunction of the abstract model could not be decided.
    UndecidedSuccessor,
}

impl Undetermined {
    /// The cause the item settles `inconclusive` with.
    pub fn cause(&self) -> InconclusiveCause {
        match self {
            Self::Mapping(_) => InconclusiveCause::MappingUndetermined,
            Self::UndecidedSuccessor => InconclusiveCause::UndecidedSuccessor,
        }
    }
}

/// What `check_step` returns: the verdict and the number of abstract
/// transition identities it judged or expanded, which FR-142 adds to the
/// run's `max_transitions`.
#[derive(Clone, Debug)]
pub struct StepOutcome {
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
    /// A run budget stopped the check: the evaluation meter, the candidate
    /// bound, or a result domain too large to order. It is no fault of QSL
    /// and no verdict; the run stops (FR-142).
    Stopped(ExpansionStop),
    /// An internal fault of the evaluator, or of the calling engine code
    /// that passed a system that is not the refinement's own. It is not a
    /// verdict: the item settles `failed` (QSpec FR-383).
    Fault(InternalFault),
}
