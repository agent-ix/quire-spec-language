// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-146 "Routing and settlement" (ADR-020 RE-2): the settlement of a
//! refinement's per-step simulation records from the SMT backend that
//! decided them, and of the refinement's safety half from those records and
//! its hypothesis items (QSpec FR-468 "Settlement").
//!
//! The records route by kind to an `operation-contract` backend, which runs
//! outside the core; [`settle_step_simulations`] is the core's map of what
//! that backend returned. Each record's result becomes a verdict, the
//! backend's one certificate for the refinement is checked by FR-314's
//! `check_refinement_smt_proof` against the queries FR-315 encodes, and a
//! record settles `Proved{Inductive{1}}` `Certified` only when that check
//! returns `Verified`. A counterexample to the `Initial` record settles the
//! half `refuted` only through FR-145's replay.

use qsl_eval::smt::RefinementCertificateRequest;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_replay::{
    certify_step_records, settle_step_records, CertificateCheckLimits, RefinementSmtCertificate,
    RefinementTraceSettlement, ReplayRefusal, StepRecordVerdict, StepSimulationSettlement,
    TerminalRecord, TerminalValue,
};
use quire_exact::{Cancel, NodeKey};

/// One per-step simulation record as the routing settled it: its request
/// index and the FR-331 result its candidate backend returned, `None` when
/// no candidate settled it (V-8).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepRecordResult {
    /// The record's request index.
    pub request_index: RequestIndex,
    /// The backend's result for it.
    pub result: Option<TerminalValue>,
}

/// The replay of a counterexample to the `Initial` record (FR-145), which
/// in production is `replay_refinement_trace` over the counterexample's
/// initial state.
pub type InitialReplayFn<'a> =
    dyn FnOnce() -> Result<RefinementTraceSettlement, ReplayRefusal> + 'a;

/// A refinement's per-step simulation records, settled from the SMT backend.
pub struct StepSimulationItem<'a> {
    /// The refinement and its two subjects, for FR-315's queries.
    pub request: &'a RefinementCertificateRequest<'a>,
    /// The backend that decided the records.
    pub backend: BackendId,
    /// The `Initial` record's result, then each step row's in `steps` order.
    pub records: Vec<StepRecordResult>,
    /// The backend's certificate for the refinement, when it returned one.
    pub certificate: Option<RefinementSmtCertificate>,
    /// Each hypothesis item: the invariant clause's node and its item's
    /// result, in the records' `hypotheses` order (FR-146: every record of a
    /// refinement lists the same clauses, ascending by node key).
    pub hypotheses: Vec<(NodeKey, TerminalValue)>,
    /// The certificate checker's budgets.
    pub certificate_limits: CertificateCheckLimits,
    /// Replays a counterexample to the `Initial` record.
    pub replay_initial: Box<InitialReplayFn<'a>>,
}

/// The records and the safety half, settled.
#[derive(Debug)]
pub struct StepSimulationRecords {
    /// One terminal record per per-step simulation record, in `records`
    /// order.
    pub records: Vec<TerminalRecord>,
    /// The refinement's safety half (QSpec FR-468).
    pub safety: StepSimulationSettlement,
}

/// Why the records could not be settled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StepSimulationRefusal {
    /// The request names no refinement of its package, or one that writes
    /// no per-step simulation record (a hidden field, a history field or an
    /// `any` row: FR-146).
    NoRecords,
    /// The results are not one per record the refinement writes: the
    /// `Initial` record and one per step row.
    RecordCount {
        /// The records the refinement writes.
        expected: usize,
        /// The results given.
        given: usize,
    },
    /// The hypothesis items are not exactly the invariant clauses the
    /// refinement's records list, in their order.
    Hypotheses,
    /// The backend returned a result FR-468 gives no meaning for an
    /// `operation-contract` obligation (`tested`, `measured` or
    /// `declined`).
    Result {
        /// The record's request index.
        request_index: RequestIndex,
    },
}

/// FR-146: settle `item`'s records and the refinement's safety half under
/// `cancel`, which stops only the certificate check.
///
/// # Errors
///
/// [`StepSimulationRefusal`] when the request's refinement writes no record,
/// when `item` does not give one result per record the refinement writes or
/// one hypothesis item per clause its records list, or when a record's
/// result has no FR-468 meaning.
pub fn settle_step_simulations(
    item: StepSimulationItem<'_>,
    cancel: &Cancel,
) -> Result<StepSimulationRecords, StepSimulationRefusal> {
    let StepSimulationItem {
        request,
        backend,
        records,
        certificate,
        hypotheses,
        certificate_limits,
        replay_initial,
    } = item;
    let entries = request
        .package
        .graph()
        .refinements()
        .iter()
        .find(|refinement| {
            refinement
                .v2_emission()
                .is_ok_and(|emission| emission.identity == request.refinement)
        })
        .map(|refinement| refinement.step_simulations())
        .filter(|entries| !entries.is_empty())
        .ok_or(StepSimulationRefusal::NoRecords)?;
    if records.len() != entries.len() {
        return Err(StepSimulationRefusal::RecordCount {
            expected: entries.len(),
            given: records.len(),
        });
    }
    let clauses: Vec<_> = hypotheses
        .iter()
        .map(|(clause, _)| clause.clone())
        .collect();
    if entries
        .iter()
        .any(|entry| entry.record().hypotheses != clauses)
    {
        return Err(StepSimulationRefusal::Hypotheses);
    }
    let rows = entries.len() - 1;
    let verdicts = records
        .iter()
        .map(|record| match &record.result {
            None => Ok(StepRecordVerdict::NoCandidate),
            Some(value) => {
                StepRecordVerdict::of_backend(value).ok_or(StepSimulationRefusal::Result {
                    request_index: record.request_index,
                })
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let certification = certify_step_records(
        request,
        certificate.as_ref(),
        rows,
        &certificate_limits,
        cancel,
    );
    let settled = settle_step_records(&verdicts, &certification, &hypotheses, replay_initial);
    Ok(StepSimulationRecords {
        records: settled
            .records
            .iter()
            .zip(&records)
            .map(|(settlement, record)| {
                settlement.to_terminal(record.request_index, backend.clone())
            })
            .collect(),
        safety: settled.safety,
    })
}
