// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-182 (ADR-023 HC-7): what an EN-3 k-induction run over an HP-2 item
//! returns, which [`settle_hyper_smt`](crate::settle_hyper_smt) settles.
//!
//! CG runs FR-315's base and step queries and constructs a
//! [`HyperSmtOutcome`] from the pair of answers. The identity it carries is
//! the one `encode_hyper_query` returned with the queries, so the settlement
//! can refuse an outcome that answers another item.

use quire_semantic_value::catalog::InternalFault;

use crate::model_check::hyper_smt_query::HyperProductIdentity;

/// The result of one EN-3 k-induction run over an HP-2 item.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HyperSmtResult {
    /// The base and the step query were both unsatisfiable at `depth`.
    Inductive {
        /// The induction depth `k`.
        depth: u64,
    },
    /// The base query was unsatisfiable and the step query satisfiable at
    /// `depth`: the induction is not closed.
    InductionNotClosed {
        /// The induction depth `k`.
        depth: u64,
    },
}

/// A k-induction run's result with the identity of the product and instance
/// its queries encoded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HyperSmtOutcome {
    /// What the run found.
    pub result: HyperSmtResult,
    /// The `product` of the queries the result answers; `Err` only when
    /// FR-315's `of` faulted (FR-315 "Outcome handoff").
    pub product: Result<HyperProductIdentity, InternalFault>,
}
