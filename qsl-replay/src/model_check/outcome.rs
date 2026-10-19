// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126 Outputs (ADR-018 LA-3, LA-5): what an EN-1 run returns, which
//! FR-127's settlement map settles.

use crate::model_check::counterexample::TemporalCounterexample;
use crate::model_check::limits::ReachedLimit;
use crate::proof_result::IncompleteCause;

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
}

/// One EN-1 run's outcome (FR-126 Outputs).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelCheckOutcome {
    /// Every reachable product state was examined and none is bad (V-1).
    Holds {
        /// The run's counts.
        stats: RunStats,
    },
    /// The canonical counterexample, to be replayed before it counts (V-4).
    Violated {
        /// The counterexample.
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

impl ModelCheckOutcome {
    /// The run's counts.
    pub fn stats(&self) -> RunStats {
        match self {
            Self::Holds { stats }
            | Self::Violated { stats, .. }
            | Self::BoundReached { stats, .. }
            | Self::Undecided { stats, .. }
            | Self::Stopped { stats, .. } => *stats,
        }
    }
}
