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

/// A qualified expansion either failed its identity admission or stopped for
/// one of the existing semantic expansion reasons.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ModelKeyExpansionError {
    /// Identity construction or ordering was refused.
    #[error(transparent)]
    Key(#[from] ModelKeyError),
    /// The model's ordinary expansion stopped.
    #[error("model expansion stopped: {0:?}")]
    Expansion(crate::simulation::ExpansionStop),
}

pub(super) trait KeyPolicy {
    fn key(
        &self,
        system: &super::ModelSystem<'_>,
        state: &qsl_semantics::model::state::ModelState,
    ) -> Result<crate::simulation::StateKey, crate::simulation::ExpansionStop>;
}

pub(super) struct KeyScope<'a> {
    pub identity: qsl_foundation::IdentityLimits,
    pub budget: std::cell::RefCell<&'a mut dyn ModelKeyBudget>,
    pub cancel: &'a quire_exact::Cancel,
    pub refused: std::cell::RefCell<Option<ModelKeyError>>,
}
impl KeyScope<'_> {
    pub fn finish<T>(
        &self,
        result: Result<T, crate::simulation::ExpansionStop>,
    ) -> Result<T, ModelKeyExpansionError> {
        if let Some(error) = self.refused.borrow_mut().take() {
            return Err(error.into());
        }
        self.cancel.poll();
        if let Some(cause) = self.cancel.tripped() {
            return Err(ModelKeyError::Cancelled(cause).into());
        }
        result.map_err(ModelKeyExpansionError::Expansion)
    }
}
impl KeyPolicy for KeyScope<'_> {
    fn key(
        &self,
        system: &super::ModelSystem<'_>,
        state: &qsl_semantics::model::state::ModelState,
    ) -> Result<crate::simulation::StateKey, crate::simulation::ExpansionStop> {
        match super::encode::bounded::model_bytes(
            &system.model,
            state,
            system.typing(),
            self.identity,
            &mut **self.budget.borrow_mut(),
            self.cancel,
        ) {
            Ok(bytes) => Ok(crate::simulation::StateKey::from_canonical_bytes(bytes)),
            Err(error) => {
                *self.refused.borrow_mut() = Some(error);
                Err(super::walk::exhausted(
                    crate::simulation::ExpansionLimit::EvaluationMeter(
                        quire_exact::LimitKind::WorkUnits,
                    ),
                ))
            }
        }
    }
}
