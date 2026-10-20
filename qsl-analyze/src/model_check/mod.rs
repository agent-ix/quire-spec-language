// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-018 EN-1: the explicit-state model checker.
//!
//! [`check_model`] decides a reachable invariant (TP-1) or the
//! deadlock-freedom item over every behaviour of a model subject (FR-126's
//! first phase) and a liveness clause (TP-4) under weak fairness (its second
//! phase, in `liveness`); `qsl-replay` settles its outcome (FR-127). A
//! universal hyperproperty (HP-2, FR-176) is decided over the self-composition
//! product by [`check_hyper`]. The retained graph, the generic product and
//! the SCC decomposition are the reusable machinery; a graph outcome alone is
//! FR-101's exploration outcome, never a proof of a temporal property.

mod ample;
mod check;
mod concretise;
mod fair_walk;
mod graph;
mod hyper;
mod liveness;
mod product;
mod quotient;
mod reduce;
mod refine;
mod scc;
mod state_graph;

pub use ample::{Atoms, Por};
pub use check::{
    check_model, check_protocol, ModelCheckRefusal, ModelCheckRequest, ProtocolCheckRequest,
};
pub use graph::{
    retain, retain_reduced, Ample, Choice, GraphExploration, ProductStateId, RetainedEdge,
    RetainedGraph, RetainedState,
};
pub use hyper::{check_hyper, HyperCheckRequest, HyperRefusal};
pub use liveness::LivenessRefusal;
pub use product::{
    Product, ProductFinding, ProductKey, ProductState, ProductTransition, PropertyComponent,
};
pub use reduce::{
    admit, closure, generators, instance_groups, not_preserving, preserves, unoffered,
    ClosureRefusal, InstanceGroup, Reductions, SymmetryDeclaration, SymmetryRefusal, TableForm,
};
pub use refine::{check_refinement, RefinementCheckRequest};
pub use scc::Component;
pub use state_graph::{
    check_state_graph, ExploredStateGraph, GraphEdge, GraphNode, GraphStats, NodeIndex,
    PredicateLabel, StateGraphItem, StateGraphRequest, StateGraphRun,
};
