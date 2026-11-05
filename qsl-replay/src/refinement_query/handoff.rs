// SPDX-License-Identifier: AGPL-3.0-or-later
//! The replay adapter for CG's existing refinement run-record byte format.

use quire_exact::Cancel;
use quire_semantic_value::catalog::InternalFault;

use crate::model_check::limits::ModelCheckLimit;
use crate::{
    CertificateCheckLimits, IncompleteCause, ProofFormat, RefinementSmtCertificate, RefutedQuery,
};

use super::{refinement_step_rows, RefinementQueryRequest};

/// Why CG's run-record handoff produced no refinement certificate.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum RefinementHandoffRefusal {
    /// Unknown tag, truncated or nonrepresentable field, or trailing bytes.
    #[error("malformed refinement SMT handoff")]
    Malformed,
    /// Missing, duplicate or out-of-range Base/Step record or absent byte field.
    #[error("refinement SMT handoff does not match the selected rows")]
    Shape,
    /// The assembly stage stopped at a configured resource or caller cancellation.
    #[error("refinement SMT handoff stopped: {0:?}")]
    Stopped(IncompleteCause),
    /// The retained request broke an established invariant.
    #[error("refinement SMT handoff invariant failed: {0:?}")]
    Fault(InternalFault),
}

struct Budget<'a> {
    limits: &'a CertificateCheckLimits,
    cancel: &'a Cancel,
}

impl Budget<'_> {
    fn poll(&self) -> Result<(), RefinementHandoffRefusal> {
        if self.cancel.poll() {
            let source = self
                .cancel
                .cause()
                .ok_or_else(|| fault("a cancelled handle has a cause"))?;
            return Err(RefinementHandoffRefusal::Stopped(
                IncompleteCause::Cancelled { source },
            ));
        }
        Ok(())
    }

    fn charge(
        &self,
        count: u64,
        bound: u64,
        limit: ModelCheckLimit,
    ) -> Result<(), RefinementHandoffRefusal> {
        self.poll()?;
        if count > bound {
            return Err(RefinementHandoffRefusal::Stopped(
                IncompleteCause::LimitReached {
                    limit,
                    value: bound,
                },
            ));
        }
        Ok(())
    }
}

fn fault(invariant: &'static str) -> RefinementHandoffRefusal {
    RefinementHandoffRefusal::Fault(InternalFault::new("refinement-smt-handoff", invariant))
}

fn exhausted() -> RefinementHandoffRefusal {
    RefinementHandoffRefusal::Stopped(IncompleteCause::ResourceExhausted)
}

struct Cursor<'a> {
    remaining: &'a [u8],
}

impl<'a> Cursor<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8], RefinementHandoffRefusal> {
        let (value, remaining) = self
            .remaining
            .split_at_checked(length)
            .ok_or(RefinementHandoffRefusal::Malformed)?;
        self.remaining = remaining;
        Ok(value)
    }

    fn number(&mut self) -> Result<u64, RefinementHandoffRefusal> {
        let octets = self
            .take(8)?
            .try_into()
            .map_err(|_| RefinementHandoffRefusal::Malformed)?;
        Ok(u64::from_le_bytes(octets))
    }

    fn field(&mut self) -> Result<&'a [u8], RefinementHandoffRefusal> {
        let length =
            usize::try_from(self.number()?).map_err(|_| RefinementHandoffRefusal::Malformed)?;
        self.take(length)
    }
}

#[derive(Clone, Copy)]
struct Part<'a> {
    query: Option<&'a [u8]>,
    proof: Option<&'a [u8]>,
}

#[cfg(test)]
std::thread_local! {
    static FAIL_RESERVATION_AFTER: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

fn reserve_payload(bytes: &mut Vec<u8>, additional: usize) -> Result<(), RefinementHandoffRefusal> {
    #[cfg(test)]
    if FAIL_RESERVATION_AFTER.with(|pending| match pending.get() {
        Some(0) => {
            pending.set(None);
            true
        }
        Some(remaining) => {
            pending.set(Some(remaining - 1));
            false
        }
        None => false,
    }) {
        // Vec<u8> cannot have a capacity above isize::MAX. This exercises
        // the real reservation failure before any allocator can be called.
        bytes.try_reserve(usize::MAX).map_err(|_| exhausted())?;
    }
    bytes.try_reserve_exact(additional).map_err(|_| exhausted())
}

fn owned(part: Part<'_>) -> Result<RefutedQuery, RefinementHandoffRefusal> {
    let query = part.query.ok_or(RefinementHandoffRefusal::Shape)?;
    let proof = part.proof.ok_or(RefinementHandoffRefusal::Shape)?;
    let mut query_bytes = Vec::new();
    reserve_payload(&mut query_bytes, query.len())?;
    query_bytes.extend_from_slice(query);
    let mut proof_bytes = Vec::new();
    reserve_payload(&mut proof_bytes, proof.len())?;
    proof_bytes.extend_from_slice(proof);
    Ok(RefutedQuery {
        query: query_bytes,
        proof: proof_bytes,
    })
}

pub(super) fn assemble(
    request: &RefinementQueryRequest<'_>,
    bytes: &[u8],
    limits: &CertificateCheckLimits,
    cancel: &Cancel,
) -> Result<RefinementSmtCertificate, RefinementHandoffRefusal> {
    let budget = Budget { limits, cancel };
    budget.charge(
        u64::try_from(bytes.len()).map_err(|_| exhausted())?,
        budget.limits.max_certificate_bytes,
        ModelCheckLimit::CertificateMaxBytes,
    )?;
    let rows = refinement_step_rows(request.closure.checked_package(), &request.refinement)
        .ok_or_else(|| fault("the request names a refinement of its package"))?;
    let count = rows
        .checked_add(1)
        .ok_or_else(|| fault("the selected row count fits a certificate"))?;
    let mut cursor = Cursor { remaining: bytes };
    if cursor.number()? != count {
        return Err(RefinementHandoffRefusal::Shape);
    }
    budget.charge(
        count,
        budget.limits.max_certificate_edges,
        ModelCheckLimit::CertificateMaxEdges,
    )?;
    let count = usize::try_from(count).map_err(|_| exhausted())?;
    // Every encoded record has at least a tag and two lengths. Validate that
    // many records can be framed before allocating selected-row slots.
    if count > cursor.remaining.len() / 17 {
        return Err(RefinementHandoffRefusal::Malformed);
    }
    let mut parts = Vec::new();
    parts.try_reserve_exact(count).map_err(|_| exhausted())?;
    parts.resize(count, None);
    for _ in 0..count {
        budget.poll()?;
        let tag = cursor.take(1)?[0];
        let slot = match tag {
            0 => 0,
            1 => {
                let row = cursor.number()?;
                if row >= rows {
                    return Err(RefinementHandoffRefusal::Shape);
                }
                usize::try_from(row.checked_add(1).ok_or(RefinementHandoffRefusal::Shape)?)
                    .map_err(|_| RefinementHandoffRefusal::Shape)?
            }
            _ => return Err(RefinementHandoffRefusal::Malformed),
        };
        let part = Part {
            query: Some(cursor.field()?),
            proof: Some(cursor.field()?),
        };
        let place = parts
            .get_mut(slot)
            .ok_or_else(|| fault("a record slot lies inside the part table"))?;
        if place.replace(part).is_some() {
            return Err(RefinementHandoffRefusal::Shape);
        }
    }
    if !cursor.remaining.is_empty() {
        return Err(RefinementHandoffRefusal::Malformed);
    }
    assemble_parts(&parts, &budget)
}

fn assemble_parts(
    parts: &[Option<Part<'_>>],
    budget: &Budget<'_>,
) -> Result<RefinementSmtCertificate, RefinementHandoffRefusal> {
    // Check every optional record and field before copying any payload.
    for part in parts {
        budget.poll()?;
        let part = part.ok_or(RefinementHandoffRefusal::Shape)?;
        if part.query.is_none() || part.proof.is_none() {
            return Err(RefinementHandoffRefusal::Shape);
        }
    }
    let (base, remaining) = parts.split_first().ok_or(RefinementHandoffRefusal::Shape)?;
    let base = owned(base.ok_or(RefinementHandoffRefusal::Shape)?)?;
    let mut steps = Vec::new();
    steps
        .try_reserve_exact(remaining.len())
        .map_err(|_| exhausted())?;
    for part in remaining {
        budget.poll()?;
        steps.push(owned(part.ok_or(RefinementHandoffRefusal::Shape)?)?);
    }
    Ok(RefinementSmtCertificate {
        format: ProofFormat::Alethe,
        base,
        steps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[trace("TC-1275", "FR-492-AC-9")]
    #[test]
    fn absent_record_or_field_never_assembles_a_certificate() {
        let limits = CertificateCheckLimits::default();
        let cancel = Cancel::new();
        let budget = Budget {
            limits: &limits,
            cancel: &cancel,
        };
        for part in [
            None,
            Some(Part {
                query: None,
                proof: Some(b""),
            }),
            Some(Part {
                query: Some(b""),
                proof: None,
            }),
        ] {
            assert_eq!(
                assemble_parts(&[part], &budget),
                Err(RefinementHandoffRefusal::Shape)
            );
        }
    }
    #[trace("TC-1275", "FR-492-AC-11")]
    #[test]
    fn payload_reservation_failure_never_exposes_a_certificate() {
        let limits = CertificateCheckLimits::default();
        let cancel = Cancel::new();
        let budget = Budget {
            limits: &limits,
            cancel: &cancel,
        };
        let parts = [
            Some(Part {
                query: Some(b"base query"),
                proof: Some(b"base proof"),
            }),
            Some(Part {
                query: Some(b"step query"),
                proof: Some(b"step proof"),
            }),
        ];
        let expected = assemble_parts(&parts, &budget).expect("healthy native reservations");
        // Also fail after earlier payloads have been copied, including after
        // the complete base has been assembled, without exposing any prefix.
        for successful_reservations in [0, 1, 3] {
            FAIL_RESERVATION_AFTER.with(|pending| pending.set(Some(successful_reservations)));
            assert_eq!(
                assemble_parts(&parts, &budget),
                Err(RefinementHandoffRefusal::Stopped(
                    IncompleteCause::ResourceExhausted
                ))
            );
            assert_eq!(
                FAIL_RESERVATION_AFTER.with(std::cell::Cell::get),
                None,
                "the real reservation path consumed its one-shot fault"
            );
            assert_eq!(
                assemble_parts(&parts, &budget),
                Ok(expected.clone()),
                "the next assembly must preserve every original byte"
            );
        }
    }
}
