// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-142 Outputs (ADR-020 RE-1, RE-4, RE-5): what a refinement's safety
//! phase returns, which FR-144's settlement map settles.
//!
//! The safety half is not a [`crate::ModelCheckOutcome`]: that outcome's
//! `Holds` carries a closure certificate of a temporal clause's product, and
//! a refinement's product has a simulation certificate (FR-149) instead,
//! which [`SafetyOutcome::Holds`] carries.

use qsl_foundation::InternalFault;

use crate::certificate::simulation::SimulationCertificate;
use crate::model_check::counterexample::TemporalCounterexample;
use crate::model_check::limits::ReachedLimit;
use crate::model_check::outcome::{ModelCheckOutcome, RunStats};
use crate::proof_result::{IncompleteCause, InconclusiveCause, UnavailabilityCause};
use crate::refinement::failure::Undetermined;

/// Why the safety phase decided nothing (ADR-018 V-6).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SafetyUndecided {
    /// The concrete subject has no initial state (ADR-016 EX-10).
    NoInitialState,
    /// A mapping row or step-row argument was refused or incomplete
    /// (`mapping-undetermined`), or a contract conjunction of either model
    /// was undecided (`undecided-successor`).
    Undetermined(Undetermined),
}

impl SafetyUndecided {
    /// The cause the item settles `inconclusive` with.
    pub fn cause(&self) -> InconclusiveCause {
        match self {
            Self::NoInitialState => InconclusiveCause::NoInitialState,
            Self::Undetermined(undetermined) => undetermined.cause(),
        }
    }
}

/// The outcome of a refinement's safety phase (FR-142 Outputs).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SafetyOutcome {
    /// Every reachable product state was examined and every initial state
    /// and step passed (V-1).
    Holds {
        /// The simulation relation the product found, for the core to check
        /// (FR-149).
        certificate: Box<SimulationCertificate>,
        /// The run's counts.
        stats: RunStats,
    },
    /// The canonical counterexample, to be replayed before it counts (V-4):
    /// a failed initial state or step, or an undefined mapping.
    Violated {
        /// The counterexample, its `refinement` member set.
        counterexample: Box<TemporalCounterexample>,
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
        cause: SafetyUndecided,
        /// The run's counts.
        stats: RunStats,
    },
    /// A run budget or cancellation stopped the run (V-7).
    Stopped {
        /// `ResourceExhausted` for a limit, `Cancelled` for a cancellation.
        cause: IncompleteCause,
        /// The limit reached; `None` for a cancellation.
        limit: Option<ReachedLimit>,
        /// The run's counts.
        stats: RunStats,
    },
    /// An internal fault of the evaluator, or of the engine that passed
    /// `check_step` a system that is not the refinement's own. The item
    /// settles `failed` (QSpec FR-383) and no counterexample is written.
    Fault {
        /// The fault.
        fault: InternalFault,
        /// The run's counts.
        stats: RunStats,
    },
}

/// The liveness half of a refinement (FR-143 Outputs, ADR-020 RE-1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LivenessHalf {
    /// The second phase ran. It holds with `HoldsReduced` and no reduction:
    /// every behaviour of the concrete subject that is fair under `F_C`
    /// satisfies `F_A`, with no certificate, since the core has no checker
    /// for it (ADR-020 CT-4). A violation is a lasso whose failure is
    /// `Divergence` or `AbstractUnfair`.
    Checked(ModelCheckOutcome),
    /// The second phase did not run (ADR-020 AX-5).
    Unsupported(UnavailabilityCause),
}

/// FR-142's and FR-143's `RefinementOutcome`: the safety half and, when
/// `F_A` is non-empty, the liveness half.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefinementOutcome {
    /// The safety half.
    pub safety: SafetyOutcome,
    /// The liveness half: `None` with `F_A` empty. With a non-empty `F_A` it
    /// is `Unsupported` for a refinement with a hidden abstract field or a
    /// strong `F_A` constraint, whatever the safety half is; otherwise it is
    /// `None` when the safety half is other than `Holds`, and `Checked`
    /// when it holds.
    pub liveness: Option<LivenessHalf>,
}
