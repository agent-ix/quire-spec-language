// SPDX-License-Identifier: AGPL-3.0-or-later
//! The native zone engine EN-6 (ADR-026 §8).
//!
//! [`Dbm`] is its zone representation (EZ-2, FR-238): a difference-bound
//! matrix over arbitrary-precision integer bounds, after every constant of
//! the model and the claim has been scaled once to an integer by
//! [`scale_constants`]. [`check_timed_model`] is its search (FR-239), with
//! FR-240's liveness phase on a symbolic product, and counterexamples,
//! finite or lassos, concretised to exact delays (FR-241). A proof
//! carries `fsl_replay`'s [`fsl_replay::ZoneCertificate`] (FR-244), which
//! the core checks with its own code (FR-245). [`check_timed_refinement`]
//! decides the safety half of a refinement whose concrete model is timed
//! on the same search, with FR-142's discrete product (FSpec FR-430).

mod certificate;
pub(crate) use certificate::state_graph_certificate;
mod concretise;
mod dbm;
mod divergence;
mod lasso;
mod live_certificate;
mod liveness;
mod refine;
mod scale;
mod search;
mod settle;
mod state_graph;
mod task_product;
mod task_certificate;
mod task_symbolic;
mod time_lock;

pub use certificate::certificate_item;
pub use refine::{
    check_free_delay_refinement, check_timed_refinement, TimedLivenessHalf, TimedLivenessOutcome,
    TimedRefinementOutcome, TimedRefinementRefusal, TimedRefinementRequest, TimedRefinementStop,
    TimedRefinementUndecided, TimedSafetyOutcome,
};
pub mod manifest;
pub use dbm::{Bound, Dbm, DbmError};
pub use scale::{scale_constants, ScaledConstants};
pub use task_certificate::{read_back_unchanged, task_product_certificate, TaskCertificateRefusal};
pub use task_symbolic::{
    explore as explore_task_symbolic, SymbolicEdge, SymbolicGraph, SymbolicIdentity, SymbolicStop,
    SymbolicVertex,
};
pub use search::{
    canonical_symbolic_state, check_timed_model, concretise_timed_state_graph_path,
    explore_timed_state_graph, TimedStateGraph, TimedStateGraphEdge, TimedStateGraphNode,
    ZoneCheckItem, ZoneCheckLimit, ZoneCheckLimits, ZoneCheckOutcome, ZoneCheckRefusal,
    ZoneCheckRequest, ZoneReduction, ZoneStats, ZoneStop, ZoneUndecided,
};
pub use settle::{
    settle_timed_refinement, settle_zone_check, ZoneCheckRecord, ZoneItem, ZONE_CERTIFIED,
};
pub use state_graph::{decide_timed_state_graph, TimedStateGraphDecision};
pub use task_product::{
    check_task_product, check_task_product_with_cancel, TaskProductSearchFrontier,
    TaskProductSearchLimits, TaskProductSearchOutcome, TaskProductSearchRefusal,
    TaskProductSearchRefutation, TaskProductSearchStop,
};
