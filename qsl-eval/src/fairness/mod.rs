// SPDX-License-Identifier: AGPL-3.0-or-later
//! The fairness kernels (ADR-018 §4, ADR-019 §3; ADR-011 §6.1 layer 5): what
//! the model checker's second phase, the scheduler-fairness filter, the
//! possible family's lasso search and the certificate checkers share, so
//! none of them depends on `qsl-analyze`.
//!
//! The kernels work on a [`Graph`] the caller builds: vertices in the order
//! the caller discovered them, edges in canonical successor order, each edge
//! labelled with the transition identity it takes (or the terminal stutter),
//! each vertex with the set of identities enabled there, and the acceptance
//! sets of the property automaton as sets of vertices.
//!
//! - [`components`] is iterative SCC decomposition of a given subgraph.
//! - [`Constraint`] is membership: which identities a fairness constraint
//!   covers, when it is enabled, and when an edge takes it.
//! - [`analyze`] is ADR-019 FS-2's `fair` function: it decomposes a vertex set
//!   into SCCs and classifies each, refining a component that fails a strong
//!   constraint, and keeps every witness the certificate checker reads.
//! - [`check_loop`] is lasso fairness: whether the loop of a given lasso is
//!   fair, as replay reads it (ADR-018 CX-3).

mod constraint;
mod error;
mod graph;
mod lasso;
mod refine;
mod scc;

pub use constraint::{Constraint, Strength};
pub use error::FairnessError;
pub use graph::{Edge, EdgeLabel, Graph, Identity, Vertex};
pub use lasso::{check_loop, LassoFairness, LoopStep};
pub use refine::{analyze, Analysis, Outcome};
pub use scc::{components, Component};
