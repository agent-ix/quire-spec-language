// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126 Outputs (ADR-018 LA-3, LA-5): what an EN-1 run returns, which
//! FR-127's settlement map settles.

use crate::certificate::closure::ClosureCertificate;
use crate::model_check::counterexample::TemporalCounterexample;
use crate::model_check::limits::ReachedLimit;
use crate::proof_result::{
    AppliedReduction, IncompleteCause, InconclusiveCause, MemoryBound, ReductionKind,
    SymmetryBrokenCause,
};
use qsl_eval::simulation::protocol::WeakAccessShape;
use qsl_semantics::model::key::DeclarationKey;
use quire_semantic_value::fairness::PropertyForm;
use quire_semantic_value::model_transition::ModelTransition;

/// The counts of one run, whatever ended it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RunStats {
    /// Distinct product states created.
    pub product_states: u64,
    /// Product edges explored.
    pub product_edges: u64,
    /// Distinct automaton states materialized.
    pub automaton_states: u64,
    /// The greatest depth of a created product state.
    pub depth: u64,
}

/// Why a run decided nothing (ADR-018 V-6).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UndecidedCause {
    /// A contract conjunction was refused during an expansion, with no
    /// clause false and none undefined, so the successor relation is not
    /// exactly known.
    UndecidedSuccessor,
    /// The subject has no initial state (ADR-016 EX-10).
    NoInitialState,
    /// The item has zero instances (FR-125).
    NoInstance,
    /// ADR-023 HV-4: a hyper item's `match` block was refused or incomplete
    /// at a reachable joint step (FR-176).
    MatchUndetermined,
    /// ADR-023 HM-7: a hyper item's non-empty `μ_U` is matched by no fair
    /// tuple of universal behaviours (FR-176).
    VacuousMatch,
}

impl UndecidedCause {
    /// The V-6 cause the run's settlement states (FR-127, FR-182).
    pub(super) fn inconclusive(self) -> InconclusiveCause {
        match self {
            Self::UndecidedSuccessor => InconclusiveCause::UndecidedSuccessor,
            Self::NoInitialState => InconclusiveCause::NoInitialState,
            Self::NoInstance => InconclusiveCause::NoInstance,
            Self::MatchUndetermined => InconclusiveCause::MatchUndetermined,
            Self::VacuousMatch => InconclusiveCause::VacuousMatch,
        }
    }
}

/// Why a run under reductions settles `inconclusive` (ADR-021 RV-2 to
/// RV-4, FR-160) or `unsupported` (RV-6).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReductionCause {
    /// A selected reduction's preservation-table row reads "no"; settled in
    /// the pre-check, with no state explored.
    NotPreserving {
        /// The first refused reduction.
        reduction: ReductionKind,
        /// The claim's property form.
        form: PropertyForm,
    },
    /// The initial states are not closed under a symmetry declaration's
    /// group; settled in the pre-check, with no state explored.
    SymmetryBroken {
        /// The declaration's population.
        population: DeclarationKey,
        /// Why.
        cause: SymmetryBrokenCause,
    },
    /// ADR-018 V-8: a selected reduction the system does not offer; settled
    /// `unsupported` in the pre-check, with no state explored.
    Unsupported {
        /// The first selected reduction not offered.
        reduction: ReductionKind,
    },
    /// The run completed with no counterexample and stored boundary states
    /// of its state constraint.
    ConstraintReached {
        /// The boundary states stored.
        boundary_states: u64,
    },
    /// A partial-order run completed to its search horizon with no
    /// counterexample (ADR-021 RV-5).
    Horizon {
        /// The search horizon.
        max_depth: u64,
    },
}

/// Why a run over a subject with a weak `parallel` settles by its memory
/// model (ADR-025 MV-1, MB-4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryCause {
    /// V-8: the pre-check found an access shape that is not one access, or
    /// a weak `parallel` whose join is not `all`; no state was explored.
    WeakAccessShape(Box<WeakAccessShape>),
    /// V-6: the run completed with no counterexample and at least one
    /// bound-limited state.
    BoundReached {
        /// The bound.
        bound: MemoryBound,
        /// The bound-limited states.
        states: u64,
    },
}

/// One EN-1 run's outcome (FR-126 Outputs).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelCheckOutcome<T = ModelTransition> {
    /// Every reachable product state was examined and none is bad (V-1),
    /// with the closure certificate the core checks before the proof
    /// settles (ADR-018 PC-3).
    Holds {
        /// The explored product states.
        certificate: ClosureCertificate,
        /// The run's counts.
        stats: RunStats,
    },
    /// Every reachable product state of the reduced product was examined
    /// and none is bad (ADR-021 RV-1). With no reduction listed, only a
    /// state constraint was applied and the run reached no boundary state,
    /// or it is a refinement's liveness half (FR-143), which has no
    /// reduction. No core checker covers the reduced graph or the liveness
    /// half, so it carries no certificate.
    HoldsReduced {
        /// The applied reductions, in the order symmetry, partial-order.
        reductions: Vec<AppliedReduction>,
        /// The run's counts.
        stats: RunStats,
    },
    /// A run under reductions settled `inconclusive` (ADR-021 RV-2 to RV-4)
    /// or `unsupported` (RV-6).
    Reduction {
        /// Why.
        cause: ReductionCause,
        /// The run's counts.
        stats: RunStats,
    },
    /// A run over a subject with a weak `parallel` settled by its memory
    /// model (ADR-025 MV-1, MB-4).
    Memory {
        /// Why.
        cause: MemoryCause,
        /// The run's counts.
        stats: RunStats,
    },
    /// The canonical counterexample, to be replayed before it counts (V-4).
    Violated {
        /// The counterexample.
        counterexample: Box<TemporalCounterexample<T>>,
        /// The run's counts.
        stats: RunStats,
    },
    /// The run completed its search horizon with no counterexample (V-5).
    BoundReached {
        /// The horizon `max_depth`.
        depth: u64,
        /// The run's counts.
        stats: RunStats,
    },
    /// The run decided nothing (V-6).
    Undecided {
        /// Why.
        cause: UndecidedCause,
        /// The run's counts.
        stats: RunStats,
    },
    /// A run budget or cancellation stopped the run before it completed its
    /// method (V-7).
    Stopped {
        /// `ResourceExhausted` for a limit, `Cancelled` for a cancellation.
        cause: IncompleteCause,
        /// The limit reached; `None` for a cancellation.
        limit: Option<ReachedLimit>,
        /// The run's counts.
        stats: RunStats,
    },
}

impl<T> ModelCheckOutcome<T> {
    /// The run's counts.
    pub fn stats(&self) -> RunStats {
        match self {
            Self::Holds { stats, .. }
            | Self::HoldsReduced { stats, .. }
            | Self::Reduction { stats, .. }
            | Self::Memory { stats, .. }
            | Self::Violated { stats, .. }
            | Self::BoundReached { stats, .. }
            | Self::Undecided { stats, .. }
            | Self::Stopped { stats, .. } => *stats,
        }
    }
}
