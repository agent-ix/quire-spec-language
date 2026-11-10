// SPDX-License-Identifier: AGPL-3.0-or-later
//! Caller admission for model identity construction, separate from transport.

use quire_exact::{CancelCause, Integer, IntegerDecimalBudget, ReservationDenied};

use crate::simulation::EncodingRefusal;

/// Cumulative caller admission shared by every identity in an operation.
///
/// Storage and work use [`IntegerDecimalBudget`]'s full requested capacity and
/// primitive work units. Denial causes stay with the caller. No reservation is
/// refunded. Integer admission is representation policy, not probability range
/// or the certificate's rational-value bit bound.
pub trait ModelKeyBudget: IntegerDecimalBudget {
    /// Admit the borrowed integer representation before inspecting/rendering it.
    fn integer(&mut self, value: &Integer) -> Result<(), ReservationDenied>;
}

/// A model identity construction stopped before the denied operation.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ModelKeyError {
    /// Caller storage, work or representation admission refused; its cause
    /// remains with the caller's budget.
    #[error("model key admission denied")]
    Denied,
    /// The original caller cancellation token stopped construction.
    #[error("model key cancelled: {0:?}")]
    Cancelled(CancelCause),
    /// The identity byte ceiling or actual allocator refused construction.
    #[error(transparent)]
    Encoding(#[from] EncodingRefusal),
    /// The value has no model identity encoding.
    #[error("no model key encoding: {0}")]
    Unsupported(&'static str),
    /// A construction size cannot be represented by this platform.
    #[error("model key size overflow")]
    SizeOverflow,
}
