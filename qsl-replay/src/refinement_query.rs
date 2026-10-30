// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-492: the facade's refinement-query entry (FR-315 "Refinement queries").
//! A consumer encodes the `Initial` or the `Step { row }` simulation query of
//! a refinement with [`encode_refinement_query`], and reads which source row
//! each `steps` row is with [`refinement_step_sources`], through the
//! `qsl_replay` root without naming [`crate::spine`] or `qsl_eval`
//! (ADR-011 FB-05, T-12 rule (a)).
//!
//! The types here are replay-owned wrappers, not re-exports (FR-100-AC-8: no
//! `qsl_replay` re-export names a `qsl_eval` path). They add no digest, hash
//! or revision and compute none: the request holds the checked package and
//! subjects it was built from, unchanged.
//!
//! The refinement node's `steps` array is the rows in the order the V2 wire
//! sorts them, which is not the order they were written in. A `Step { row }`
//! query names a position of that array; [`refinement_step_sources`] gives,
//! for each position, the zero-based position of the same row among the
//! refinement's rows as written.

use qsl_eval::smt::{self, RefinementCertificateRequest};

use crate::SmtEncodingRefusal;

/// The request of a refinement query: the refinement of a recompiled package
/// and its concrete and abstract subjects (FR-314's request of a refinement
/// item).
pub struct RefinementQueryRequest<'a> {
    inner: RefinementCertificateRequest<'a>,
}

impl<'a> From<RefinementCertificateRequest<'a>> for RefinementQueryRequest<'a> {
    fn from(inner: RefinementCertificateRequest<'a>) -> Self {
        Self { inner }
    }
}

/// Which refinement query to encode.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum RefinementQueryKind {
    /// The initial query: every concrete initial state maps to an abstract
    /// initial state.
    Initial,
    /// The step query of the row at this zero-based position of the
    /// refinement node's `steps` array.
    Step {
        /// The row's position.
        row: u64,
    },
}

impl From<RefinementQueryKind> for smt::RefinementQueryKind {
    fn from(kind: RefinementQueryKind) -> Self {
        match kind {
            RefinementQueryKind::Initial => Self::Initial,
            RefinementQueryKind::Step { row } => Self::Step { row },
        }
    }
}

/// Encode the refinement query `kind` of `request` as an SMT-LIB 2 script in
/// FR-315's canonical printing.
///
/// # Errors
///
/// [`SmtEncodingRefusal`] as FR-315 names it; no script is written.
pub fn encode_refinement_query(
    request: &RefinementQueryRequest<'_>,
    kind: RefinementQueryKind,
) -> Result<Vec<u8>, SmtEncodingRefusal> {
    smt::encode_refinement_query(&request.inner, kind.into())
}

/// The source row of each row of the refinement `request` names, in `steps`
/// order: entry `k` is the position, among the rows as written, of the row at
/// position `k` of the refinement node's `steps` array. The entries are the
/// numbers `0..n` in some order.
///
/// # Errors
///
/// A fault [`SmtEncodingRefusal`] naming why the refinement has no emitted
/// rows, or that the request names no refinement of its package.
pub fn refinement_step_sources(
    request: &RefinementQueryRequest<'_>,
) -> Result<Vec<u64>, SmtEncodingRefusal> {
    smt::refinement_step_sources(&request.inner)?
        .into_iter()
        .map(|source| {
            u64::try_from(source)
                .map_err(|_| SmtEncodingRefusal::fault("a source row position fits a u64"))
        })
        .collect()
}
