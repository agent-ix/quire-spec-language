// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cumulative candidate identity admission shared by quantitative proof profiles.

use super::*;
use qsl_replay::ProbabilityKeyAdmission;

pub(super) fn rational_bytes(value: &Rational) -> usize {
    usize::try_from(
        value
            .numerator()
            .magnitude_bits()
            .saturating_add(value.denominator().magnitude_bits())
            .div_ceil(8),
    )
    .unwrap_or(usize::MAX)
}

pub(super) struct CandidateBudget<'a> {
    pub(super) meter: quire_exact::Meter,
    retained: u64,
    storage_bound: u64,
    pub(super) identity: qsl_foundation::IdentityLimits,
    pub(super) cancel: &'a Cancel,
}
impl<'a> CandidateBudget<'a> {
    pub(super) fn new(limits: ProbabilityDocumentLimits, cancel: &'a Cancel) -> Self {
        Self {
            meter: quire_exact::Meter::new(limits.accounting).with_cancel(cancel.clone()),
            retained: 0,
            storage_bound: limits.storage_bytes,
            identity: qsl_foundation::IdentityLimits::default(),
            cancel,
        }
    }
    pub(super) fn admission(&mut self) -> ProbabilityKeyAdmission<'_> {
        ProbabilityKeyAdmission::new(
            &mut self.meter,
            &mut self.retained,
            self.storage_bound,
            self.cancel,
        )
    }
    pub(super) fn reserve(&mut self, bytes: usize) -> Result<(), ProbabilityDocumentError> {
        let mut admission = self.admission();
        quire_exact::Reservation::reserve(&mut admission, bytes)
            .map_err(|_| admission.refusal(qsl_eval::simulation::model::ModelKeyError::Denied))
    }
    pub(super) fn key(
        &mut self,
        binding: Option<&ObjectReference>,
        raw: &[u8],
    ) -> Result<ProbabilityProductKey, ProbabilityDocumentError> {
        let identity = self.identity;
        let cancel = self.cancel;
        let mut admission = self.admission();
        ProbabilityProductKey::for_model_state_with_budget(
            binding,
            raw,
            identity,
            &mut admission,
            cancel,
        )
        .map_err(|error| admission.refusal(error))
    }
    pub(super) fn retain_row(
        &mut self,
        row: &ExactProbabilityValue,
    ) -> Result<(), ProbabilityDocumentError> {
        self.reserve(
            std::mem::size_of::<ExactProbabilityValue>()
                .saturating_add(row.state.as_bytes().len())
                .saturating_add(rational_bytes(&row.value)),
        )?;
        let mut admission = self.admission();
        quire_exact::IntegerDecimalBudget::work(
            &mut admission,
            u64::try_from(
                row.state
                    .as_bytes()
                    .len()
                    .saturating_add(rational_bytes(&row.value)),
            )
            .unwrap_or(u64::MAX),
        )
        .map_err(|_| admission.refusal(qsl_eval::simulation::model::ModelKeyError::Denied))
    }
}

pub(super) fn values(
    product: &ProbProduct,
    binding: Option<&ObjectReference>,
    values: &[ExtRational],
    budget: &mut CandidateBudget<'_>,
) -> Result<ExactProbabilityCertificate, ProbabilityDocumentError> {
    if product.states.len() != values.len() {
        return Err(ProbabilityDocumentError::Malformed);
    }
    budget.reserve(
        values
            .len()
            .saturating_mul(std::mem::size_of::<ExactProbabilityValue>()),
    )?;
    let mut rows = Vec::with_capacity(values.len());
    for (state, value) in product.states.iter().zip(values) {
        let ExtRational::Finite(value) = value else {
            return Err(ProbabilityDocumentError::Unsupported);
        };
        budget.reserve(rational_bytes(value))?;
        rows.push(ExactProbabilityValue {
            state: budget.key(binding, &state.model)?,
            value: value.clone(),
        });
    }
    sort(
        &mut rows,
        |row| &row.state,
        &mut budget.meter,
        budget.cancel,
    )?;
    if rows.windows(2).any(|pair| pair[0].state == pair[1].state) {
        return Err(ProbabilityDocumentError::Malformed);
    }
    Ok(ExactProbabilityCertificate { values: rows })
}

// In-place insertion gives a fallible, allocation-free comparison boundary.
pub(super) fn sort<T>(
    rows: &mut [T],
    key: impl Fn(&T) -> &ProbabilityProductKey,
    meter: &mut quire_exact::Meter,
    cancel: &Cancel,
) -> Result<(), ProbabilityDocumentError> {
    for index in 1..rows.len() {
        let mut position = index;
        while position > 0 {
            if let Some(cause) = cancel.tripped() {
                return Err(ProbabilityDocumentError::Cancelled(cause));
            }
            let bytes = key(&rows[position])
                .as_bytes()
                .len()
                .min(key(&rows[position - 1]).as_bytes().len())
                .saturating_add(1);
            meter
                .charge(
                    quire_exact::Charge::new(quire_exact::ChargePoint::GraphEdge)
                        .work(quire_exact::Integer::from(bytes as u64)),
                )
                .map_err(|error| match cancel.tripped() {
                    Some(cause) => ProbabilityDocumentError::Cancelled(cause),
                    None => ProbabilityDocumentError::Accounting(Box::new(error)),
                })?;
            if key(&rows[position - 1]) <= key(&rows[position]) {
                break;
            }
            rows.swap(position - 1, position);
            position -= 1;
        }
    }
    Ok(())
}
