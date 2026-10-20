// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-176, FR-177 Outputs (ADR-023 HV-1 to HV-8): what an EN-1 run over an
//! HP-2 or HP-3 item returns, which FR-182 settles.
//!
//! It is FR-126's [`ModelCheckOutcome`](crate::ModelCheckOutcome) for a hyper
//! item: `Violated` carries a [`HyperCounterexample`], and `Undecided` the two
//! causes FR-182 adds, `MatchUndetermined` and `VacuousMatch`. A `Holds`
//! carries no certificate yet: FR-163's product-closure certificate and its
//! core checker are not built, so a hyper `Holds` settles no `proved`.

use crate::model_check::hyper_counterexample::HyperCounterexample;
use crate::model_check::limits::ReachedLimit;
use crate::model_check::outcome::{RunStats, UndecidedCause};
use crate::proof_result::IncompleteCause;

/// One EN-1 run's outcome over an HP-2 or HP-3 item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HyperOutcome {
    /// Every reachable product state was examined and no fair accepting
    /// cycle (HP-2) or fair cycle with an empty witness set (HP-3) exists
    /// (V-1), whatever certificate FR-163 will require.
    Holds {
        /// The run's counts.
        stats: RunStats,
    },
    /// The canonical counterexample, to be replayed before it counts (V-4).
    Violated {
        /// The counterexample.
        counterexample: Box<HyperCounterexample>,
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
        /// The count a counted budget reached: the members a witness set
        /// would have held (FR-184); `None` for a limit that counts only its
        /// value and for a cancellation.
        count: Option<u64>,
        /// The run's counts.
        stats: RunStats,
    },
}

impl HyperOutcome {
    /// The run's counts.
    pub fn stats(&self) -> RunStats {
        match self {
            Self::Holds { stats }
            | Self::Violated { stats, .. }
            | Self::Undecided { stats, .. }
            | Self::Stopped { stats, .. } => *stats,
        }
    }
}
