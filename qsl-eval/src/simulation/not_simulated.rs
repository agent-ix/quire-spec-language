// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-101: the dispositions returned before any state is explored or
//! sampled.

use qsl_foundation::digest::WireNodeId;
use qsl_foundation::selection::DefinitionRef;
use qsl_semantics::family::{classify_extent, ClaimExtent, ClassifyFailure, UnboundedDomains};
use qsl_semantics::value::declaration::TypeEnvironment;
use quire_exact::ValueType;

/// Every unbounded domain a `requires-bound` request named, by key, with its
/// kind (ADR-014 §4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequiresBound {
    /// The request's unbounded domains.
    pub domains: UnboundedDomains,
}

/// Why `explore_request` or `sample_request` returned before exploring or
/// sampling anything (FR-101).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum NotSimulated {
    /// The request's extent is unbounded (ADR-014 §4). No `TransitionSystem`
    /// method was called.
    #[error("the request has an unbounded domain and requires a proof bound")]
    RequiresBound(RequiresBound),
    /// `classify_extent` stopped: a node-count stage limit or an internal
    /// fault. Neither `RequiresBound` nor an exploration.
    #[error("the request's extent could not be classified: {0:?}")]
    Extent(ClassifyFailure),
    /// `sampler` names a generator other than
    /// `quire.simulation.sampler/v1` `1-draft.1`, the only one
    /// `sample_request` runs. Refused before any draw;
    /// `explore_request` never returns this.
    #[error("sampler {supplied:?} is not the pinned quire.simulation.sampler/v1 generator")]
    GeneratorMismatch {
        /// The `DefinitionRef` the caller supplied.
        supplied: DefinitionRef,
    },
    /// `system.initial()` returned no state to sample from.
    /// `explore_request` never returns this.
    #[error("the system has no initial state to sample from")]
    EmptyInitial,
}

/// Classify `domains`' extent (ADR-014 §4) before any `TransitionSystem`
/// method is called, shared by `explore_request` and `sample_request`
/// (FR-101).
pub(crate) fn check_requires_bound(
    domains: &[(WireNodeId, &ValueType)],
    types: &TypeEnvironment,
    position_limit: u64,
) -> Result<(), NotSimulated> {
    match classify_extent(domains, types, position_limit) {
        Ok(ClaimExtent::Bounded) => Ok(()),
        Ok(ClaimExtent::Unbounded(domains)) => {
            Err(NotSimulated::RequiresBound(RequiresBound { domains }))
        }
        Err(failure) => Err(NotSimulated::Extent(failure)),
    }
}
