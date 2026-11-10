// SPDX-License-Identifier: AGPL-3.0-or-later
//! Caller admission for model identity construction, separate from transport.

use quire_exact::{CancelCause, Integer, IntegerDecimalBudget, ReservationDenied};

use crate::simulation::EncodingRefusal;

/// Cumulative caller admission shared by raw model-state keys in an operation.
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

/// A qualified expansion either failed its raw state-key admission or stopped for
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
    fn admit(&self, bytes: usize) -> Result<(), crate::simulation::ExpansionStop>;
    fn work(&self, units: u64) -> Result<(), crate::simulation::ExpansionStop>;
    fn fail(&self, error: ModelKeyError) -> crate::simulation::ExpansionStop;
    fn compare(
        &self,
        left: &[u8],
        right: &[u8],
    ) -> Result<std::cmp::Ordering, crate::simulation::ExpansionStop> {
        for (left, right) in left.iter().zip(right) {
            self.work(1)?;
            let order = left.cmp(right);
            if !order.is_eq() {
                return Ok(order);
            }
        }
        self.work(1)?;
        Ok(left.len().cmp(&right.len()))
    }
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
    fn admit(&self, bytes: usize) -> Result<(), crate::simulation::ExpansionStop> {
        self.cancel.poll();
        if let Some(cause) = self.cancel.tripped() {
            return Err(self.fail(ModelKeyError::Cancelled(cause)));
        }
        self.budget
            .borrow_mut()
            .reserve(bytes)
            .map_err(|_| self.fail(ModelKeyError::Denied))?;
        self.cancel.poll();
        if let Some(cause) = self.cancel.tripped() {
            return Err(self.fail(ModelKeyError::Cancelled(cause)));
        }
        Ok(())
    }
    fn work(&self, units: u64) -> Result<(), crate::simulation::ExpansionStop> {
        self.cancel.poll();
        if let Some(cause) = self.cancel.tripped() {
            return Err(self.fail(ModelKeyError::Cancelled(cause)));
        }
        self.budget
            .borrow_mut()
            .work(units)
            .map_err(|_| self.fail(ModelKeyError::Denied))?;
        self.cancel.poll();
        if let Some(cause) = self.cancel.tripped() {
            return Err(self.fail(ModelKeyError::Cancelled(cause)));
        }
        Ok(())
    }
    fn fail(&self, error: ModelKeyError) -> crate::simulation::ExpansionStop {
        self.refused.borrow_mut().get_or_insert(error);
        super::walk::exhausted(crate::simulation::ExpansionLimit::EvaluationMeter(
            quire_exact::LimitKind::WorkUnits,
        ))
    }
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
            Err(error) => Err(self.fail(error)),
        }
    }
}

pub(super) fn reserve<T>(
    policy: Option<&dyn KeyPolicy>,
    values: &mut Vec<T>,
    extra: usize,
) -> Result<(), crate::simulation::ExpansionStop> {
    if let Some(policy) = policy {
        let needed = values
            .len()
            .checked_add(extra)
            .ok_or_else(|| policy.fail(ModelKeyError::SizeOverflow))?;
        if needed > values.capacity() {
            let bytes = needed
                .checked_mul(size_of::<T>())
                .ok_or_else(|| policy.fail(ModelKeyError::SizeOverflow))?;
            policy.admit(bytes)?;
            values
                .try_reserve_exact(needed - values.len())
                .map_err(|_| {
                    policy.fail(EncodingRefusal::Allocation { requested: bytes }.into())
                })?;
        }
    } else {
        values.reserve(extra);
    }
    Ok(())
}

pub(super) fn copy_bytes(
    policy: Option<&dyn KeyPolicy>,
    bytes: &[u8],
) -> Result<Vec<u8>, crate::simulation::ExpansionStop> {
    let mut copy = Vec::new();
    if let Some(policy) = policy {
        policy.work(bytes.len() as u64)?;
    }
    reserve(policy, &mut copy, bytes.len())?;
    copy.extend_from_slice(bytes);
    Ok(copy)
}

pub(super) fn sort<T>(
    policy: &dyn KeyPolicy,
    values: &mut [T],
    mut compare: impl FnMut(&T, &T) -> Result<std::cmp::Ordering, crate::simulation::ExpansionStop>,
) -> Result<(), crate::simulation::ExpansionStop> {
    for at in 1..values.len() {
        let mut next = at;
        while next > 0 {
            if !compare(&values[next - 1], &values[next])?.is_gt() {
                break;
            }
            policy.work(1)?;
            values.swap(next - 1, next);
            next -= 1;
        }
    }
    Ok(())
}
