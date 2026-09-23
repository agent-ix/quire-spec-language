// SPDX-License-Identifier: AGPL-3.0-or-later
//! `value`'s internal early-exit carrier, always converted into a
//! [`quire_exact::Outcome`] by [`outcome_from_stop`], and back by
//! [`outcome_into_stop`]. Crate-private plumbing only: it is not a kernel
//! type, not a spec-owned type and never leaves `value`'s own computations,
//! which is why it holds no catalog code of its own and is not part of
//! ADR-013 O-16/O-17's outcome or refusal families. It is shared by roughly
//! two dozen `value/*.rs` computations that all convert through
//! [`outcome_from_stop`]/[`outcome_into_stop`], which is why an
//! `?`-friendly shape earns its own module rather than inline handling at
//! each call site.
//!
//! `quire_exact::Outcome<T>` is foreign to this crate (K is a leaf, ADR-011
//! §6.1), so `outcome_from_stop`/`outcome_into_stop` cannot be an inherent
//! `impl` on it (E0116, the orphan rule) the way QSL's own now-deleted
//! `value::outcome` copy could. They are plain crate-private functions
//! instead: a single-implementation trait would add a name to learn with no
//! seam or polymorphism to justify it (QSL-131 O2).
//!
//! No `Halt` variant carries a fault into a `Stop`, and no `Stop`-returning
//! helper can pass one to [`outcome_from_stop`]: `value::expression`'s
//! `Machine` carries its own crate-private `Halt` for an S6a invariant break
//! (PR #334), and no `From<Halt> for Stop` conversion exists here, so a
//! fault stays unrepresentable in `Stop` by construction, not by convention.

use quire_exact::{Incomplete, Outcome, Refusal, Undefined};

/// Why a `value` computation stopped without a value: the kernel's own
/// [`Undefined`]/[`Refusal`] reasons, or a meter [`Incomplete`] charge
/// denial. Holds kernel causes only.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Stop {
    Undefined(Undefined),
    Refused(Refusal),
    Incomplete(Incomplete),
}

impl From<Incomplete> for Stop {
    fn from(record: Incomplete) -> Self {
        Self::Incomplete(record)
    }
}

/// Converts a `value` computation's `Result<T, Stop>` early exit into the
/// kernel [`Outcome<T>`] it represents, variant by variant.
pub(crate) fn outcome_from_stop<T>(result: Result<T, Stop>) -> Outcome<T> {
    match result {
        Ok(value) => Outcome::Completed(value),
        Err(Stop::Undefined(reason)) => Outcome::Undefined(reason),
        Err(Stop::Refused(reason)) => Outcome::Refused(reason),
        Err(Stop::Incomplete(record)) => Outcome::Incomplete(record),
    }
}

/// The inverse of [`outcome_from_stop`], for further `?` propagation.
pub(crate) fn outcome_into_stop<T>(outcome: Outcome<T>) -> Result<T, Stop> {
    match outcome {
        Outcome::Completed(value) => Ok(value),
        Outcome::Undefined(reason) => Err(Stop::Undefined(reason)),
        Outcome::Refused(reason) => Err(Stop::Refused(reason)),
        Outcome::Incomplete(record) => Err(Stop::Incomplete(record)),
    }
}
