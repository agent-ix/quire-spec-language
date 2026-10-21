// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-176, FR-177, FR-178 Outputs (ADR-023 HV-1 to HV-8): what an EN-1 run over
//! an HP-2, HP-3 or HP-6 item returns, which FR-182 settles.
//!
//! It is FR-126's [`ModelCheckOutcome`](crate::ModelCheckOutcome) for a hyper
//! item: `Violated` carries a [`HyperCounterexample`], and `Undecided` the two
//! causes FR-182 adds, `MatchUndetermined` and `VacuousMatch`. A `Holds`
//! carries no certificate yet: FR-163's product-closure certificate and its
//! core checker are not built, so a hyper `Holds` settles no `proved`.

use qsl_semantics::check::HyperForm;

use crate::model_check::hyper_counterexample::HyperCounterexample;
use crate::model_check::limits::ReachedLimit;
use crate::model_check::outcome::{RunStats, UndecidedCause};
use crate::proof_result::{AppliedReduction, IncompleteCause, ReductionKind};

/// Why a request's reduction was settled before any state was expanded
/// (FR-174 "Preservation rows").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HyperReductionCause {
    /// ADR-023 §5's row for the item's form reads "no" for the reduction
    /// (V-6 `ReductionNotPreserving`).
    NotPreserving {
        /// The first selected reduction the table refuses.
        reduction: ReductionKind,
        /// The item's form.
        form: HyperForm,
    },
    /// The row reads "yes" but this engine does not offer the reduction for
    /// a hyper item (V-8).
    Unsupported {
        /// The first selected reduction not offered.
        reduction: ReductionKind,
    },
}

/// One EN-1 run's outcome over an HP-2, HP-3 or HP-6 item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HyperOutcome {
    /// Every reachable product state was examined and no fair accepting
    /// cycle (HP-2) or fair cycle with an empty witness set (HP-3) exists
    /// (V-1), whatever certificate FR-163 will require.
    Holds {
        /// The run's counts.
        stats: RunStats,
    },
    /// Every reachable state of the product reduced by the copy group was
    /// examined and no violation exists (V-1 under ADR-023 CS-5). It carries
    /// no certificate, as [`Self::Holds`] does not.
    HoldsReduced {
        /// The applied reductions: the copy group's swap.
        reductions: Vec<AppliedReduction>,
        /// The run's counts.
        stats: RunStats,
    },
    /// The request's reductions were settled before any state was expanded
    /// (V-6, V-8).
    Reduction {
        /// Why.
        cause: HyperReductionCause,
        /// The run's counts: none.
        stats: RunStats,
    },
    /// The canonical counterexample, to be replayed before it counts (V-4).
    Violated {
        /// The counterexample.
        counterexample: Box<HyperCounterexample>,
        /// The run's counts.
        stats: RunStats,
    },
    /// The run completed every move up to its search horizon with no
    /// counterexample (V-5, ADR-023 HV-3). Only an HP-6 run sets a horizon.
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
            | Self::HoldsReduced { stats, .. }
            | Self::Reduction { stats, .. }
            | Self::Violated { stats, .. }
            | Self::BoundReached { stats, .. }
            | Self::Undecided { stats, .. }
            | Self::Stopped { stats, .. } => *stats,
        }
    }
}
