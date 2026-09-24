// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-057 "Stage ownership": the routing step after negotiation.
//!
//! Negotiation is not a QSL stage: `quire-contract-codegen`'s `negotiate_*`
//! settles every item from its candidate set and extent (quire-specification
//! AD-016, FR-290). Routing takes those settled dispositions as input data
//! and routes an item to its one candidate only when the item is settled
//! `supported`. An item settled `requires-bound`, `unsupported` or
//! `invalid-request` gets no target; it is not turned into a refusal or a
//! hold, and it does not delay any other item's routing (FR-057 "Absence,
//! unsupported, refusal, timeout and hold").
//!
//! Routing reads nothing but its input: no registry, no registration order,
//! no display text and no ambient state, so equal inputs give equal routes.
//! It has no preference order among candidates and never chooses one:
//! FR-290 settles `supported` only for an item with exactly one candidate,
//! so [`Disposition::Supported`] carries that one candidate and routing
//! forwards it unchanged.

use crate::Candidate;

/// One item's settled disposition, as `negotiate_*` produced it (the
/// FR-331 `dispositions` values), in the item's request order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Disposition {
    /// `supported`, settled by the arm of the item's one candidate, which
    /// this variant carries.
    Supported(Candidate),
    /// `requires-bound`: the one candidate is bounded-only, the extent is
    /// unbounded and a finite bound is available.
    RequiresBound,
    /// `unsupported` (warned): for example an empty candidate set (backend
    /// absence) or an arm that does not discharge the item's IR form.
    Unsupported,
    /// `invalid-request`, with its `invalid_capability` cause.
    InvalidRequest,
}

impl Disposition {
    /// The item's routing target: its one candidate when `supported`, and
    /// no target otherwise.
    pub fn target(&self) -> Option<&Candidate> {
        match self {
            Disposition::Supported(candidate) => Some(candidate),
            Disposition::RequiresBound | Disposition::Unsupported | Disposition::InvalidRequest => {
                None
            }
        }
    }
}

/// Route every item: one entry per input disposition, at the same request
/// index, holding the item's target or `None`.
///
/// Each item's target depends only on its own disposition, so an item with
/// no target never affects another item's route.
pub fn route(dispositions: &[Disposition]) -> Vec<Option<&Candidate>> {
    dispositions.iter().map(Disposition::target).collect()
}
