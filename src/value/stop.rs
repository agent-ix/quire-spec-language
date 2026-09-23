// SPDX-License-Identifier: AGPL-3.0-or-later
//! `value`'s internal early-exit carrier, always converted into a
//! [`quire_exact::Outcome`] by [`OutcomeStop::from_stop`], and back by
//! [`OutcomeStop::into_stop`]. Crate-private plumbing only: it is not a
//! kernel type, not a spec-owned type and never leaves `value`'s own
//! computations, which is why it holds no catalog code of its own and is not
//! part of ADR-013 O-16/O-17's outcome or refusal families. It is shared by
//! roughly two dozen `value/*.rs` computations that all convert through
//! [`OutcomeStop::from_stop`]/[`OutcomeStop::into_stop`], which is why an
//! `?`-friendly shape earns its own module rather than inline handling at
//! each call site.
//!
//! `quire_exact::Outcome<T>` is foreign to this crate (K is a leaf, ADR-011
//! §6.1), so `from_stop`/`into_stop` cannot be an inherent `impl` on it
//! (E0116, the orphan rule) the way QSL's own now-deleted `value::outcome`
//! copy could. [`OutcomeStop`] is a local trait implemented for the kernel
//! `Outcome<T>` instead: `Type::method(..)` associated-function syntax
//! still resolves a trait method when the trait is in scope, so every
//! existing `Outcome::from_stop(..)`/`.into_stop()` call site keeps its
//! shape (QSL-131 O2).
//!
//! No `Halt` variant carries a fault into a `Stop`, and no `Stop`-returning
//! helper can pass one to [`OutcomeStop::from_stop`]: `value::expression`'s
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

/// Converts between a `value` computation's `Result<T, Stop>` early exit and
/// the kernel [`Outcome<T>`] it represents.
pub(crate) trait OutcomeStop<T> {
    /// A kernel outcome, carried over variant by variant.
    fn from_stop(result: Result<T, Stop>) -> Self;
    /// The inverse of [`Self::from_stop`], for further `?` propagation.
    fn into_stop(self) -> Result<T, Stop>;
}

impl<T> OutcomeStop<T> for Outcome<T> {
    fn from_stop(result: Result<T, Stop>) -> Self {
        match result {
            Ok(value) => Self::Completed(value),
            Err(Stop::Undefined(reason)) => Self::Undefined(reason),
            Err(Stop::Refused(reason)) => Self::Refused(reason),
            Err(Stop::Incomplete(record)) => Self::Incomplete(record),
        }
    }

    fn into_stop(self) -> Result<T, Stop> {
        match self {
            Self::Completed(value) => Ok(value),
            Self::Undefined(reason) => Err(Stop::Undefined(reason)),
            Self::Refused(reason) => Err(Stop::Refused(reason)),
            Self::Incomplete(record) => Err(Stop::Incomplete(record)),
        }
    }
}
