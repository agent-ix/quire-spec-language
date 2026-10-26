// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-182 (ADR-023 HC-6, HC-7): what an EN-2 unrolling or EN-3 k-induction
//! run over an HP-2 item returns, which
//! [`settle_hyper_smt`](crate::settle_hyper_smt) settles.
//!
//! CG runs FR-315's base and step queries and constructs a
//! [`HyperSmtOutcome`] from the pair of answers. The identity it carries is
//! the one `encode_hyper_query` returned with the queries, so the settlement
//! can refuse an outcome that answers another item.

use quire_exact::CancelCause;
use quire_semantic_value::catalog::InternalFault;

use crate::model_check::hyper_counterexample::HyperCounterexample;
use crate::model_check::hyper_smt_query::HyperProductIdentity;
use crate::model_check::limits::ReachedLimit;
use qsl_eval::smt::SmtEncodingRefusal;

/// The result of one EN-2 or EN-3 SMT run over an HP-2 item.
#[derive(Clone, Debug, Eq, PartialEq)]
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
    /// The solver found a counterexample for QSL to replay itself (FR-183).
    Violated {
        /// The counterexample evidence.
        counterexample: Box<HyperCounterexample>,
        /// The query that produced it.
        method: SmtMethod,
    },
    /// EN-2 reached its requested bound without a replayable lasso.
    BoundReached {
        /// The last depth searched.
        depth: u64,
    },
    /// The solver did not decide a query, or stopped for an unnamed resource.
    Unknown {
        /// The depth at which the solver did not decide.
        depth: u64,
        /// The query that was undecided.
        method: SmtMethod,
    },
    /// The backend's own wall clock expired.
    TimedOut {
        /// The query whose backend wall clock expired.
        method: SmtMethod,
    },
    /// A request limit stopped a query.
    LimitStopped {
        /// The request limit that stopped the query.
        limit: ReachedLimit,
        /// The query that stopped.
        method: SmtMethod,
    },
    /// The solver crashed or returned an unparseable response.
    BackendFault {
        /// The query whose backend failed.
        method: SmtMethod,
    },
    /// The run's cancellation handle tripped.
    Cancelled {
        /// The cancellation handle's cause.
        source: CancelCause,
        /// The query that was cancelled.
        method: SmtMethod,
    },
    /// Encoding the EN-3 query refused.
    EncodingRefused {
        /// The reason the EN-3 query could not be encoded.
        refusal: SmtEncodingRefusal,
    },
}

/// The SMT query family that produced a result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SmtMethod {
    /// An EN-3 base or step query.
    KInduction,
    /// An EN-2 bounded-unrolling query.
    SmtUnrolling,
}

impl SmtMethod {
    /// The terminal-record spelling of this method.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::KInduction => "k-induction",
            Self::SmtUnrolling => "smt-unrolling",
        }
    }
}

/// An SMT run's result with the identity of the product and instance its
/// queries encoded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HyperSmtOutcome {
    /// What the run found.
    pub result: HyperSmtResult,
    /// The `product` of the queries the result answers; `Err` only when
    /// FR-315's `of` faulted (FR-315 "Outcome handoff").
    pub product: Result<HyperProductIdentity, InternalFault>,
}
