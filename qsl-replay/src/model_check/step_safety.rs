// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-146 "Routing and settlement" and QSpec FR-468 "Settlement": a
//! refinement's per-step simulation records, settled from their backend
//! verdicts and the refinement's one SMT certificate, and its safety half
//! settled from those records and its hypothesis items.
//!
//! The certificate is checked once, by [`check_refinement_smt_proof`]
//! (FR-314 "Refinement certificates"). Every record is `Certified` exactly
//! when the check returns `Verified`. A check that rejects a part makes that
//! part's record (`Base` the `Initial` record, `Step { row: Some(r) }` row
//! `r`'s) `Inconclusive(CertificateRejected{rule, at})` and every other
//! record `Uncertified`. Any other result, `Unencodable` included
//! (FR-146-AC-4), leaves every record `Uncertified`.

use quire_exact::{Cancel, NodeKey};

use crate::certificate::closure::CertificateRejection;
use crate::certificate::smt::{
    check_refinement_smt_proof, RefinementSmtCertificate, SmtProofCheck,
};
use crate::certificate::zone_certificate::CertificateCheckLimits;
use crate::certificate::{CertificateLocus, QueryPart};
use crate::execute::ReplayRefusal;
use crate::model_check::refinement_replay::RefinementTraceSettlement;
use crate::model_check::settle::Row;
use crate::model_check::step_settle::{
    settle_step_simulation, BackendCertificate, OtherBackendResult, StepBackendOutcome,
    StepSimulationSettlement, STEP_INDUCTION_DEPTH,
};
use crate::outcome::ItemLabel;
use crate::proof_result::{
    Certification, IncompleteCause, InconclusiveCause, ProofBasis, SettlementBasis, TerminalValue,
};
use qsl_eval::smt::RefinementCertificateRequest;
use qsl_foundation::digest::WireNodeId;

/// What the SMT backend returned for one per-step simulation record, before
/// the refinement's certificate is checked.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StepRecordVerdict {
    /// No candidate backend settled the record (V-8).
    NoCandidate,
    /// The obligation holds.
    Holds,
    /// The backend found a counterexample.
    Counterexample,
    /// The backend settled the obligation neither way (QSpec FR-468 "Other
    /// backend results").
    Other(OtherBackendResult),
}

impl StepRecordVerdict {
    /// The verdict of the FR-331 result `value` a backend returned for a
    /// per-step simulation record: `proved` holds, `refuted` is a
    /// counterexample, and `inconclusive`, `incomplete`, `unsupported` and
    /// `failed` are other results. `None` for `tested`, `measured` and
    /// `declined`, which a backend never returns for an
    /// `operation-contract` obligation.
    pub fn of_backend(value: &TerminalValue) -> Option<Self> {
        Some(match value {
            TerminalValue::Proved { .. } => Self::Holds,
            TerminalValue::Refuted => Self::Counterexample,
            TerminalValue::Inconclusive(cause) => {
                Self::Other(OtherBackendResult::Inconclusive(cause.clone()))
            }
            TerminalValue::Incomplete(cause) => Self::Other(OtherBackendResult::Incomplete(*cause)),
            TerminalValue::Unsupported(cause) => {
                Self::Other(OtherBackendResult::Unsupported(cause.clone()))
            }
            TerminalValue::Failed => Self::Other(OtherBackendResult::Failed),
            TerminalValue::Tested | TerminalValue::Measured(_) | TerminalValue::Declined { .. } => {
                return None
            }
        })
    }
}

/// What FR-314 made of a refinement's SMT certificate, for its records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepCertification {
    /// The standing of every record the check did not reject.
    standing: BackendCertificate,
    /// The rejection, and the index of the record of its part (0 the
    /// `Initial` record, `r + 1` row `r`'s) when it has one.
    rejected: Option<(CertificateRejection, Option<usize>)>,
}

impl StepCertification {
    /// The certification of `check`, the result of checking a refinement's
    /// certificate against `rows` step rows; `None` when no certificate came.
    pub fn of_check(
        check: Option<Result<SmtProofCheck, CertificateRejection>>,
        rows: usize,
    ) -> Self {
        let standing = match &check {
            None => BackendCertificate::Absent,
            Some(Ok(SmtProofCheck::Verified)) => BackendCertificate::Verified,
            Some(Ok(
                SmtProofCheck::Unverifiable { .. }
                | SmtProofCheck::Unencodable { .. }
                | SmtProofCheck::Stopped(_)
                | SmtProofCheck::Fault(_),
            ))
            | Some(Err(_)) => BackendCertificate::Unverifiable,
        };
        let rejected = match check {
            Some(Err(rejection)) => {
                let record = match &rejection.at {
                    CertificateLocus::Query { part } | CertificateLocus::ProofStep { part, .. } => {
                        record_of(part, rows)
                    }
                    _ => None,
                };
                Some((rejection, record))
            }
            _ => None,
        };
        Self { standing, rejected }
    }

    /// The certificate standing of record `index` (0 the `Initial` record,
    /// `r + 1` row `r`'s).
    pub fn certificate(&self, index: usize) -> BackendCertificate {
        match &self.rejected {
            Some((rejection, Some(record))) if *record == index => BackendCertificate::Rejected {
                rule: rejection.rule,
                at: rejection.at.clone(),
            },
            _ => self.standing.clone(),
        }
    }
}

/// The record of `part` among an `Initial` record and `rows` step records.
fn record_of(part: &QueryPart, rows: usize) -> Option<usize> {
    match part {
        QueryPart::Base => Some(0),
        QueryPart::Step { row: Some(row) } => usize::try_from(*row)
            .ok()
            .filter(|row| *row < rows)
            .map(|row| row + 1),
        QueryPart::Step { row: None } | QueryPart::Unrolling => None,
    }
}

/// FR-146: check `certificate` for the refinement `request` names, whose
/// `steps` has `rows` rows; no certificate leaves every record `Uncertified`.
pub fn certify_step_records(
    request: &RefinementCertificateRequest<'_>,
    certificate: Option<&RefinementSmtCertificate>,
    rows: usize,
    limits: &CertificateCheckLimits,
    cancel: &Cancel,
) -> StepCertification {
    StepCertification::of_check(
        certificate
            .map(|certificate| check_refinement_smt_proof(request, certificate, limits, cancel)),
        rows,
    )
}

/// A refinement's per-step simulation records and its safety half, settled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepRecordsSettlement {
    /// The `Initial` record's settlement, then each row's in `steps` order.
    pub records: Vec<StepSimulationSettlement>,
    /// The safety half (QSpec FR-468 "Settlement").
    pub safety: StepSimulationSettlement,
}

/// FR-146 and QSpec FR-468: settle each record of `verdicts` (the `Initial`
/// record's, then each row's in `steps` order) under `certification`, and
/// the safety half from them and `hypotheses` (each invariant clause's node
/// and its item's result, in the records' `hypotheses` order). A
/// counterexample to the `Initial` record settles `refuted` only when
/// `replay_initial` (FR-145) reproduces it.
pub fn settle_step_records(
    verdicts: &[StepRecordVerdict],
    certification: &StepCertification,
    hypotheses: &[(NodeKey, TerminalValue)],
    replay_initial: impl FnOnce() -> Result<RefinementTraceSettlement, ReplayRefusal>,
) -> StepRecordsSettlement {
    let records: Vec<_> = verdicts
        .iter()
        .enumerate()
        .map(|(index, verdict)| {
            settle_step_simulation(match verdict {
                StepRecordVerdict::NoCandidate => StepBackendOutcome::NoCandidate,
                StepRecordVerdict::Holds => StepBackendOutcome::Holds {
                    certificate: certification.certificate(index),
                },
                StepRecordVerdict::Counterexample => StepBackendOutcome::Counterexample,
                StepRecordVerdict::Other(other) => StepBackendOutcome::Other(other.clone()),
            })
        })
        .collect();
    let safety = safety_of(
        verdicts,
        &records,
        certification,
        hypotheses,
        replay_initial,
    );
    StepRecordsSettlement { records, safety }
}

/// The settlement of `row`.
fn settled(row: Row) -> StepSimulationSettlement {
    StepSimulationSettlement {
        value: row.value,
        basis: row.basis,
        truth: row.truth,
    }
}

/// QSpec FR-468's rules, the first that applies: an initial counterexample,
/// a hypothesis item not `proved`, a step counterexample, a certificate shown
/// wrong, then another backend result (no candidate included), the first
/// record's in record order taking the half, then the proof.
fn safety_of(
    verdicts: &[StepRecordVerdict],
    records: &[StepSimulationSettlement],
    certification: &StepCertification,
    hypotheses: &[(NodeKey, TerminalValue)],
    replay_initial: impl FnOnce() -> Result<RefinementTraceSettlement, ReplayRefusal>,
) -> StepSimulationSettlement {
    if let Some(row) = decisive_row(verdicts, certification, hypotheses, replay_initial) {
        return settled(row);
    }
    let other = verdicts.iter().zip(records).find(|(verdict, _)| {
        matches!(
            verdict,
            StepRecordVerdict::NoCandidate | StepRecordVerdict::Other(_)
        )
    });
    match other {
        Some((_, record)) => record.clone(),
        None => settled(proof_row(records, hypotheses)),
    }
}

/// The first of FR-468's rules before "Other backend results" that applies.
fn decisive_row(
    verdicts: &[StepRecordVerdict],
    certification: &StepCertification,
    hypotheses: &[(NodeKey, TerminalValue)],
    replay_initial: impl FnOnce() -> Result<RefinementTraceSettlement, ReplayRefusal>,
) -> Option<Row> {
    if verdicts.first() == Some(&StepRecordVerdict::Counterexample) {
        return Some(match replay_initial() {
            Ok(RefinementTraceSettlement::Reproduced { .. }) => {
                Row::refuted(SettlementBasis::DecisiveCounterexample)
            }
            Ok(RefinementTraceSettlement::Inconclusive(cause)) => {
                Row::inconclusive(InconclusiveCause::ReplayParity(cause))
            }
            Ok(RefinementTraceSettlement::Stopped(reached)) => {
                Row::stopped_run(IncompleteCause::ResourceExhausted, Some(reached))
            }
            Err(refusal) => Row::refused(&refusal),
        });
    }
    if let Some((hypothesis, result)) = hypotheses
        .iter()
        .map(|(hypothesis, value)| (hypothesis, ItemLabel::of(value)))
        .find(|(_, result)| *result != ItemLabel::Proved)
    {
        return Some(Row::inconclusive(
            InconclusiveCause::HypothesesNotEstablished {
                hypothesis: WireNodeId::from_digest(*hypothesis.as_bytes()),
                result,
            },
        ));
    }
    if verdicts.contains(&StepRecordVerdict::Counterexample) {
        return Some(Row::inconclusive(InconclusiveCause::InductionNotClosed {
            depth: STEP_INDUCTION_DEPTH,
        }));
    }
    if let Some((rejection, _)) = &certification.rejected {
        return Some(Row::inconclusive(InconclusiveCause::CertificateRejected {
            rule: rejection.rule,
            at: rejection.at.clone(),
        }));
    }
    None
}

/// The proof: `Certified` only when every record and hypothesis item is.
fn proof_row(records: &[StepSimulationSettlement], hypotheses: &[(NodeKey, TerminalValue)]) -> Row {
    let certified = records
        .iter()
        .map(|record| &record.value)
        .chain(hypotheses.iter().map(|(_, value)| value))
        .all(|value| {
            matches!(
                value,
                TerminalValue::Proved {
                    certification: Certification::Certified,
                    ..
                }
            )
        });
    Row::proof(
        ProofBasis::Inductive {
            depth: STEP_INDUCTION_DEPTH,
        },
        if certified {
            Certification::Certified
        } else {
            Certification::Uncertified
        },
        SettlementBasis::DecisiveWitness,
    )
}
