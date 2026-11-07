// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-052: full-value native predecessor authority without temporal evaluation.

use std::sync::Arc;

use super::{CheckedSubject, Report, ReportRefusal};

/// Explicit finite limits for one native full-predecessor operation.
///
/// These account for the report owner's encoding, comparison and retained bytes.
/// The caller's separate `NativeJoinBudget` continues to account for IR-owned
/// admission work; a predecessor operation never resets or replaces that budget.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReportPredecessorLimits {
    /// Owner field/row visits and byte inspections or copies.
    pub work: u64,
    /// Logical temporary and retained bytes, including owned buffer handles.
    pub storage: u64,
}

impl ReportPredecessorLimits {
    /// State both original finite caller ceilings.
    #[must_use]
    pub const fn new(work: u64, storage: u64) -> Self {
        Self { work, storage }
    }
}

/// Admitted charges of a complete full-predecessor validation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReportPredecessorUsage {
    /// Field/row visits and bytes inspected or copied.
    pub work: u64,
    /// Temporary and retained logical ownership charged by the operation.
    pub storage: u64,
}

/// Resource whose first excess charge stops a predecessor operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReportPredecessorResource {
    /// Owner inspection or copy work.
    Work,
    /// Temporary or retained ownership.
    Storage,
}

struct Budget {
    limits: ReportPredecessorLimits,
    usage: ReportPredecessorUsage,
}

impl Budget {
    fn new(limits: ReportPredecessorLimits) -> Self {
        Self {
            limits,
            usage: ReportPredecessorUsage::default(),
        }
    }

    fn charge(
        &mut self,
        resource: ReportPredecessorResource,
        amount: u128,
    ) -> Result<(), ReportRefusal> {
        let (used, limit) = match resource {
            ReportPredecessorResource::Work => (&mut self.usage.work, self.limits.work),
            ReportPredecessorResource::Storage => (&mut self.usage.storage, self.limits.storage),
        };
        let actual = u128::from(*used).saturating_add(amount);
        if actual > u128::from(limit) {
            return Err(ReportRefusal::PredecessorLimit {
                resource,
                limit,
                actual,
            });
        }
        *used = u64::try_from(actual).map_err(|_| ReportRefusal::PredecessorLimit {
            resource,
            limit,
            actual,
        })?;
        Ok(())
    }

    fn work(&mut self, amount: u128) -> Result<(), ReportRefusal> {
        self.charge(ReportPredecessorResource::Work, amount)
    }

    fn storage(&mut self, amount: u128) -> Result<(), ReportRefusal> {
        self.charge(ReportPredecessorResource::Storage, amount)
    }

    fn encode(&mut self, report: &Report) -> Result<Vec<u8>, ReportRefusal> {
        // Frame lengths are read from already admitted owner members. Charge
        // each row before visiting it; the old predecessor value is flat bytes.
        self.work(1)?;
        let mut bytes = 5_u128;
        for value in &report.binding {
            self.work(1)?;
            bytes = bytes.saturating_add(9 + value.len() as u128);
        }
        bytes = bytes.saturating_add(9 + report.observation.reference.len() as u128);
        bytes = bytes.saturating_add(9 + report.observation.revision.len() as u128);
        bytes = bytes.saturating_add(9 + 3);
        let mut support = 5_u128 + 9 + 8;
        let mut temporary = 0_u128;
        let mut handles = 3_u128;
        for entry in &report.support {
            self.work(1)?;
            let row = 5_u128 + (9 + 8) + (9 + entry.key.len() as u128) + (9 + 1);
            support = support.saturating_add(9 + row);
            temporary = temporary.saturating_add(row);
            handles = handles.saturating_add(1);
        }
        bytes = bytes.saturating_add(9 + support);
        bytes = bytes.saturating_add(9 + report.identity.len() as u128);
        let mut previous = 5_u128 + 9 + 1;
        if let Some(identity) = &report.predecessor {
            previous = previous.saturating_add(9 + identity.len() as u128);
            if let Some(value) = &report.predecessor_value {
                previous = previous.saturating_add(9 + value.len() as u128);
            }
        }
        bytes = bytes.saturating_add(9 + previous);
        // The current encoder uses temporary support, row and predecessor
        // frames. Charge their bytes as well as the final output before any
        // buffer is created; a byte copied through three frames costs three.
        temporary = temporary.saturating_add(support).saturating_add(previous);
        let allocation = bytes.saturating_add(temporary);
        self.work(allocation)?;
        self.storage(allocation.saturating_add(handles))?;
        let encoded = report.to_owner_bytes();
        debug_assert_eq!(encoded.len() as u128, bytes);
        Ok(encoded)
    }
}

impl CheckedSubject<'_> {
    pub(super) fn admit_predecessor(
        &self,
        identity: Option<&[u8]>,
        supplied: Option<&[u8]>,
    ) -> Result<Option<Arc<[u8]>>, ReportRefusal> {
        let Some(identity) = identity else {
            return if self.predecessor_input.is_none() && supplied.is_none() {
                Ok(None)
            } else {
                Err(ReportRefusal::InvalidPredecessor)
            };
        };
        let (previous, limits) = self
            .predecessor_input
            .ok_or(ReportRefusal::InvalidPredecessor)?;
        let mut budget = Budget::new(limits);
        let bytes = budget.encode(previous)?;
        budget.work(previous.identity.len() as u128)?;
        for expected in &self.binding {
            budget.work(1 + expected.len() as u128)?;
        }
        if previous.binding != self.binding || previous.identity != identity {
            return Err(ReportRefusal::InvalidPredecessor);
        }
        if let Some(supplied) = supplied {
            budget.work(supplied.len() as u128)?;
            if supplied != bytes {
                return Err(ReportRefusal::InvalidPredecessor);
            }
        }
        // Arc conversion can copy the temporary Vec; precharge both ownership
        // and the copy before it. It retains bytes, never a recursive Report.
        budget.work(bytes.len() as u128)?;
        budget.storage(1 + bytes.len() as u128)?;
        Ok(Some(Arc::from(bytes)))
    }
}

impl Report {
    /// Encode complete owner bytes within explicit finite operation limits.
    ///
    /// Unlike the convenience `to_owner_bytes` representation, this operation
    /// admits every temporary/final frame and copy before allocation. It is the
    /// bounded writer for consumers retaining full predecessor or join values.
    ///
    /// # Errors
    /// Returns the original first excess work/storage charge and no output.
    pub fn to_owner_bytes_bounded(
        &self,
        limits: ReportPredecessorLimits,
    ) -> Result<(Vec<u8>, ReportPredecessorUsage), ReportRefusal> {
        let mut budget = Budget::new(limits);
        let bytes = budget.encode(self)?;
        Ok((bytes, budget.usage))
    }

    /// Validate the actual complete direct predecessor without evaluation.
    ///
    /// An equal request/result identity alone is insufficient: every retained
    /// owner byte, including truth, outcome, support and prior relation, must
    /// equal the value bound at this report's issuance or strict admission.
    ///
    /// # Errors
    /// Refuses an original report, a different direct predecessor value, or the
    /// first excess owner work/storage charge. No evaluator or IR reader runs.
    pub fn validate_predecessor(
        &self,
        previous: &Report,
        limits: ReportPredecessorLimits,
    ) -> Result<ReportPredecessorUsage, ReportRefusal> {
        let expected = self
            .predecessor_value
            .as_deref()
            .ok_or(ReportRefusal::InvalidPredecessor)?;
        let mut budget = Budget::new(limits);
        let bytes = budget.encode(previous)?;
        budget.work(expected.len() as u128)?;
        budget.work(previous.identity.len() as u128)?;
        if self.predecessor.as_deref() != Some(previous.identity.as_slice()) || expected != bytes {
            return Err(ReportRefusal::InvalidPredecessor);
        }
        Ok(budget.usage)
    }
}
