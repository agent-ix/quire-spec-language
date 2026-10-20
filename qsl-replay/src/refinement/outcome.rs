// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-142 Outputs (ADR-020 RE-1, RE-4, RE-5): what a refinement's safety
//! phase returns, which FR-144's settlement map settles.
//!
//! The safety half is not a [`crate::ModelCheckOutcome`]: that outcome's
//! `Holds` carries a closure certificate of a temporal clause's product, and
//! a refinement's product has no such certificate until the simulation
//! certificate (FR-149) exists. [`SafetyOutcome::Holds`] therefore carries
//! none.

use qsl_foundation::InternalFault;

use crate::model_check::counterexample::TemporalCounterexample;
use crate::model_check::limits::ReachedLimit;
use crate::model_check::outcome::RunStats;
use crate::proof_result::{IncompleteCause, InconclusiveCause};
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

/// FR-142's `RefinementOutcome`: the safety half. The liveness half (FR-143)
/// is a member beside it once it is built.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefinementOutcome {
    /// The safety half.
    pub safety: SafetyOutcome,
}
