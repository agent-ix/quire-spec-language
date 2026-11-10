// SPDX-License-Identifier: AGPL-3.0-or-later
//! One borrowed caller scope for model identity work, storage and representation.

use super::ProbabilityDocumentError;
use qsl_eval::simulation::model::{ModelKeyBudget, ModelKeyError};
use quire_exact::{
    Cancel, Charge, ChargePoint, Integer, IntegerDecimalBudget, LimitKind, Meter, Reservation,
    ReservationDenied,
};

/// Admission borrowed from an existing caller meter and cumulative storage count.
/// Recreating the view never resets either counter or changes the cancellation
/// handle. Identity byte limits remain the owner operation's separate argument.
pub struct ProbabilityKeyAdmission<'a> {
    meter: &'a mut Meter,
    storage: &'a mut u64,
    storage_bound: u64,
    cancel: &'a Cancel,
    stopped: Option<ProbabilityDocumentError>,
}

impl<'a> ProbabilityKeyAdmission<'a> {
    /// Borrow the real accounting state for one or more qualified key operations.
    pub fn new(
        meter: &'a mut Meter,
        storage: &'a mut u64,
        storage_bound: u64,
        cancel: &'a Cancel,
    ) -> Self {
        Self {
            meter,
            storage,
            storage_bound,
            cancel,
            stopped: None,
        }
    }

    /// Resolve an owner denial using the exact cause retained by this scope.
    pub fn refusal(&mut self, error: ModelKeyError) -> ProbabilityDocumentError {
        match error {
            ModelKeyError::Denied => self
                .stopped
                .take()
                .unwrap_or(ProbabilityDocumentError::KeyDenied),
            ModelKeyError::Cancelled(cause) => ProbabilityDocumentError::Cancelled(cause),
            ModelKeyError::Encoding(error) => ProbabilityDocumentError::KeyEncoding(error),
            ModelKeyError::Unsupported(_) => ProbabilityDocumentError::Unsupported,
            ModelKeyError::SizeOverflow => ProbabilityDocumentError::KeySizeOverflow,
        }
    }

    /// Resolve expansion failures without discarding the model owner's key cause.
    pub fn expansion_refusal(
        &mut self,
        error: qsl_eval::simulation::model::ModelKeyExpansionError,
    ) -> ProbabilityDocumentError {
        match error {
            qsl_eval::simulation::model::ModelKeyExpansionError::Key(error) => self.refusal(error),
            qsl_eval::simulation::model::ModelKeyExpansionError::Expansion(stop) => {
                match self.cancel.tripped() {
                    Some(cause) => ProbabilityDocumentError::Cancelled(cause),
                    None => ProbabilityDocumentError::Build(Box::new(
                        crate::ExactProbBuildIssue::Expansion(stop),
                    )),
                }
            }
        }
    }

    fn deny(&mut self, error: ProbabilityDocumentError) -> ReservationDenied {
        if self.stopped.is_none() {
            self.stopped = Some(error);
        }
        ReservationDenied
    }

    fn poll(&mut self) -> Result<(), ReservationDenied> {
        if let Some(cause) = self.cancel.tripped() {
            return Err(self.deny(ProbabilityDocumentError::Cancelled(cause)));
        }
        Ok(())
    }

    fn charge(&mut self, charge: Charge) -> Result<(), ReservationDenied> {
        self.poll()?;
        self.meter.charge(charge).map_err(|error| {
            let error = match self.cancel.tripped() {
                Some(cause) => ProbabilityDocumentError::Cancelled(cause),
                None => ProbabilityDocumentError::Accounting(Box::new(error)),
            };
            self.deny(error)
        })
    }
}

impl Reservation for ProbabilityKeyAdmission<'_> {
    fn reserve(&mut self, bytes: usize) -> Result<(), ReservationDenied> {
        self.poll()?;
        let bytes = u64::try_from(bytes)
            .map_err(|_| self.deny(ProbabilityDocumentError::KeySizeOverflow))?;
        let Some(actual) = self.storage.checked_add(bytes) else {
            return Err(self.deny(ProbabilityDocumentError::KeySizeOverflow));
        };
        if actual > self.storage_bound {
            return Err(self.deny(ProbabilityDocumentError::Limit {
                limit: super::ProbabilityDocumentLimit::StorageBytes,
                bound: self.storage_bound,
                actual,
            }));
        }
        *self.storage = actual;
        Ok(())
    }
}
impl IntegerDecimalBudget for ProbabilityKeyAdmission<'_> {
    fn work(&mut self, units: u64) -> Result<(), ReservationDenied> {
        self.charge(Charge::new(ChargePoint::GraphEdge).work(Integer::from(units)))
    }
}
impl ModelKeyBudget for ProbabilityKeyAdmission<'_> {
    fn integer(&mut self, value: &Integer) -> Result<(), ReservationDenied> {
        self.charge(
            Charge::new(ChargePoint::GraphResultRetain)
                .size(LimitKind::IntegerBits, value.magnitude_bits()),
        )
    }
}
