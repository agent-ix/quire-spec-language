// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-018 EN-1: the explicit-state model checker.
//!
//! [`check_model`] decides a reachable invariant (TP-1) or the
//! deadlock-freedom item over every behaviour of a model subject (FR-126's
//! first phase) and a liveness clause (TP-4) under weak fairness (its second
//! phase, in `liveness`); `qsl-replay` settles its outcome (FR-127). The
//! retained graph, the generic product and the SCC decomposition are the
//! reusable machinery; a graph outcome alone is FR-101's exploration
//! outcome, never a proof of a temporal property.

mod check;
mod graph;
mod liveness;
mod product;
mod scc;

pub use check::{check_model, ModelCheckRefusal, ModelCheckRequest};
pub use graph::{
    retain, GraphExploration, IdentityPermutation, ProductStateId, RetainedEdge, RetainedGraph,
    RetainedState,
};
pub use liveness::LivenessRefusal;
pub use product::{
    Product, ProductFinding, ProductKey, ProductState, ProductTransition, PropertyComponent,
};
pub use scc::Component;
