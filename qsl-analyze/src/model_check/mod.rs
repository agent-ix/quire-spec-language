// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-018 EN-1: reusable unreduced product and retained graph machinery.
//!
//! This module does not translate formulas, decide fairness, emit certificates
//! or settle a model-check verdict. The graph outcome is FR-101's exploration
//! outcome; even an exhaustive graph is not a proof of a temporal property.

mod graph;
mod product;
mod scc;

pub use graph::{
    retain, GraphExploration, IdentityPermutation, ProductStateId, RetainedEdge, RetainedGraph,
    RetainedState,
};
pub use product::{
    Product, ProductFinding, ProductKey, ProductState, ProductTransition, PropertyComponent,
};
pub use scc::Component;
