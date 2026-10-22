// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-181 Outputs (ADR-023 SE-4, SE-5; ADR-022 GV-1, GV-2, GV-5, GV-7): what
//! EN-1 returns for the instances of an HP-5 item, a single-existential
//! claim decided through the possible family.
//!
//! The shapes are FR-168's [`StateGraphOutcome`](crate::StateGraphOutcome)
//! with a lasso for a witness: [`ModelLasso`] is FR-170's `WitnessPath::Lasso`.
//! A `Witnessed` outcome carries no product-closure certificate yet: FR-163's
//! certificate and its core checker are not built, so it settles no `proved`.

use std::collections::BTreeSet;

use crate::model_check::counterexample::UndefinedEvaluation;
use crate::model_check::outcome::RunStats;
use crate::model_check::state_graph::{LassoWitness, ModelPath, OpenCause, RunEnd};

/// FR-181's decision for one instance of an HP-5 item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExistentialOutcome {
    /// A witness lasso for every initial state, in initial-state order, on a
    /// run whose product exploration completed with no open node and no
    /// undefined evaluation of the body.
    Witnessed {
        /// One witness per initial state.
        lassos: Vec<LassoWitness>,
    },
    /// A witness lasso for every initial state on a run that a limit stopped
    /// or that ended with open nodes: a reachable undefined evaluation of the
    /// body is not ruled out (ADR-022 RU-5).
    WitnessedUnchecked {
        /// One witness per initial state.
        lassos: Vec<LassoWitness>,
        /// How the run ended.
        end: RunEnd,
        /// The causes of the run's open nodes.
        open: BTreeSet<OpenCause>,
    },
    /// The first initial state, in subject order, whose part of the product
    /// is closed and holds no fair accepting cycle. The stem is empty: it
    /// names the initial state alone.
    Trapped {
        /// The empty path from the initial state.
        stem: ModelPath,
    },
    /// A letter of the body has no value at a reachable product state.
    Undefined {
        /// The canonical breadth-first path to the first such product state.
        stem: ModelPath,
        /// Where and why.
        undefined: UndefinedEvaluation,
    },
    /// No decisive evidence.
    NoDecision {
        /// How the run ended.
        end: RunEnd,
        /// The causes of the run's open nodes.
        open: BTreeSet<OpenCause>,
    },
    /// The subject has no initial state.
    NoInitialState,
}

/// One instance of an HP-5 item with its decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExistentialInstanceOutcome {
    /// The instance: each object parameter of the clause and the key it is
    /// bound to, in parameter order.
    pub instance: Vec<(String, String)>,
    /// The decision.
    pub outcome: ExistentialOutcome,
    /// Phase 0's witness per initial state, in initial-state order, or none
    /// where no sampled walk gave one (FR-166); empty when phase 0 drew no
    /// walk for the instance.
    pub sampled: Vec<Option<LassoWitness>>,
    /// The phase 0 walks that `max_walk_steps` stopped before they revisited
    /// a state (FR-166).
    pub walks_stopped: u64,
    /// The counts of the instance's product exploration; zero when it
    /// explored none.
    pub stats: RunStats,
}
