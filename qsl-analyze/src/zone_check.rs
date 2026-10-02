// SPDX-License-Identifier: AGPL-3.0-or-later
//! The native zone engine EN-6 (ADR-026 §8).
//!
//! [`Dbm`] is its zone representation (EZ-2, FR-238): a difference-bound
//! matrix over arbitrary-precision integer bounds, after every constant of
//! the model and the claim has been scaled once to an integer by
//! [`scale_constants`].

mod dbm;
mod scale;

pub use dbm::{Bound, Dbm, DbmError};
pub use scale::{scale_constants, ScaledConstants};
