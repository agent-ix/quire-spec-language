// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-018 LA-2, PC-6; FR-315: the one SMT-LIB 2 encoding of a model
//! subject's transition relation and of a temporal item's proof queries,
//! V-2's bounded unrolling and EN-3's `k`-induction base and step cases.
//!
//! - [`script`] is the canonical printer: [`script::Term`] and
//!   [`script::Script`].
//! - [`layout`] declares the state copies: one constant per field of each
//!   universe object, named `s<i>.<object>.<field>`.
//! - [`expression`] reads a checked clause body as a linear term and the
//!   condition under which it is defined (ADR-018 UE-1, UE-4, UE-6).
//! - [`relation`] builds the step relation of one subject between two state
//!   copies: the disjuncts of FR-315's transition assertion, operations first
//!   and the terminal stutter last. A caller encoding a product of several
//!   subjects takes one component disjunct from this relation per subject.
//! - [`query`] assembles a script for an item and a [`SmtQueryKind`].

mod expression;
mod kind;
mod layout;
mod monitor;
mod query;
mod refusal;
pub(crate) mod relation;
pub mod script;

pub use kind::SmtQueryKind;
pub use query::{encode_smt_query, SmtItem};
pub use refusal::{ExpressionLocus, SmtEncodingRefusal};
