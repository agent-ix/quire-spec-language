// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-492: the facade's refinement-query entry (FR-315 "Refinement queries").
//! A consumer encodes the `Initial` or the `Step { row }` simulation query of
//! a refinement with [`encode_refinement_query`], and reads which source row
//! each `steps` row is with [`refinement_step_sources`], through the
//! `qsl_replay` root and without naming [`crate::spine`] or `qsl_eval`
//! (ADR-011 FB-05, T-12 rule (a)).
//!
//! The refinement node's `steps` array is the rows in the order the V2 wire
//! sorts them, which is not the order they were written in. A `Step { row }`
//! query names a position of that array; [`refinement_step_sources`] gives,
//! for each position, the zero-based position of the same row among the
//! refinement's rows as written, so a consumer that holds source rows can map
//! them to query rows and check that the mapping is a permutation.

use qsl_eval::smt::{RefinementCertificateRequest, SmtEncodingRefusal};

/// The source row of each row of the refinement `request` names, in `steps`
/// order: entry `k` is the position, among the rows as written, of the row at
/// position `k` of the refinement node's `steps` array. The entries are the
/// numbers `0..n` in some order.
///
/// # Errors
///
/// A fault [`SmtEncodingRefusal`] when the request names no refinement of its
/// package.
pub fn refinement_step_sources(
    request: &RefinementCertificateRequest<'_>,
) -> Result<Vec<u64>, SmtEncodingRefusal> {
    request
        .package
        .graph()
        .refinements()
        .iter()
        .find_map(|refinement| {
            let emission = refinement.v2_emission().ok()?;
            (emission.identity == request.refinement).then_some(emission.step_sources)
        })
        .ok_or_else(|| SmtEncodingRefusal::fault("the request names a refinement of its package"))?
        .into_iter()
        .map(|source| {
            u64::try_from(source)
                .map_err(|_| SmtEncodingRefusal::fault("a source row position fits a u64"))
        })
        .collect()
}
