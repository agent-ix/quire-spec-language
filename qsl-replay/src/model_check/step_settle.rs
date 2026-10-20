// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-146 (ADR-020 RE-2, ADR-018 PC-6, V-3, V-6, V-8): the settlement of a
//! per-step simulation record's SMT outcome.
//!
//! An RE-2 record never settles `refuted`: a counterexample may start from an
//! unreachable pre-state, so only an explicit-state replay (FR-145) can
//! refute a refinement. A backend counterexample settles
//! `Inconclusive(InductionNotClosed{depth: 1})`; a holding obligation settles
//! `Proved{Inductive{depth: 1}}` with basis `decisive-witness`; a record no
//! candidate settles is `unsupported` (V-8).

use crate::certificate::{CertificateLocus, CertificateRule};
use crate::model_check::settle::Truth;
use crate::proof_result::{
    Certification, InconclusiveCause, ProofBasis, SettlementBasis, TerminalValue,
    UnavailabilityCause,
};

/// The induction depth of a per-step simulation record (ADR-020 RE-2).
pub const STEP_INDUCTION_DEPTH: u64 = 1;

/// What the core's checker (FR-314) made of a backend's proof certificate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendCertificate {
    /// The backend supplied none.
    Absent,
    /// The backend supplied one the core could not verify.
    Unverifiable,
    /// The core verified it.
    Verified,
    /// The core checked it and it is wrong (ADR-018 PC-2, PC-6): the failing
    /// rule and where it failed. The record never settles `proved`.
    Rejected {
        /// The check rule that failed.
        rule: CertificateRule,
        /// Where the rule failed.
        at: CertificateLocus,
    },
}

/// What the SMT backend returned for one per-step simulation record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StepBackendOutcome {
    /// No candidate backend settled the record (V-8).
    NoCandidate,
    /// The obligation holds.
    Holds {
        /// The backend's proof certificate, as the core checked it.
        certificate: BackendCertificate,
    },
    /// The backend found a counterexample: a pre-state the invariants may
    /// not reach.
    Counterexample,
}

/// One per-step simulation record, settled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepSimulationSettlement {
    /// The terminal value.
    pub value: TerminalValue,
    /// Its QSpec FR-243 basis.
    pub basis: SettlementBasis,
    /// The truth.
    pub truth: Truth,
}

/// FR-146: settle `outcome` as one terminal value. Never `Refuted`.
pub fn settle_step_simulation(outcome: StepBackendOutcome) -> StepSimulationSettlement {
    match outcome {
        StepBackendOutcome::Holds { certificate } => match certificate {
            BackendCertificate::Rejected { rule, at } => StepSimulationSettlement {
                value: TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
                    rule,
                    at,
                }),
                basis: SettlementBasis::Unsettled,
                truth: Truth::Pending,
            },
            BackendCertificate::Absent
            | BackendCertificate::Unverifiable
            | BackendCertificate::Verified => StepSimulationSettlement {
                value: TerminalValue::Proved {
                    basis: ProofBasis::Inductive {
                        depth: STEP_INDUCTION_DEPTH,
                    },
                    certification: if certificate == BackendCertificate::Verified {
                        Certification::Certified
                    } else {
                        Certification::Uncertified
                    },
                },
                basis: SettlementBasis::DecisiveWitness,
                truth: Truth::Satisfied,
            },
        },
        StepBackendOutcome::Counterexample => StepSimulationSettlement {
            value: TerminalValue::Inconclusive(InconclusiveCause::InductionNotClosed {
                depth: STEP_INDUCTION_DEPTH,
            }),
            basis: SettlementBasis::Unsettled,
            truth: Truth::Pending,
        },
        StepBackendOutcome::NoCandidate => StepSimulationSettlement {
            value: TerminalValue::Unsupported(UnavailabilityCause::BackendAbsent),
            basis: SettlementBasis::Unavailable,
            truth: Truth::Unavailable,
        },
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    fn proved(certification: Certification) -> TerminalValue {
        TerminalValue::Proved {
            basis: ProofBasis::Inductive { depth: 1 },
            certification,
        }
    }

    /// TC-551 step 4, settlement only: a holding obligation settles
    /// `Proved{Inductive{1}}`, `Certified` only on a verified certificate.
    #[trace("TC-551", "FR-146-AC-7")]
    #[test]
    fn a_holding_record_settles_proved_with_the_certificates_standing() {
        for (certificate, certification) in [
            (BackendCertificate::Absent, Certification::Uncertified),
            (BackendCertificate::Unverifiable, Certification::Uncertified),
            (BackendCertificate::Verified, Certification::Certified),
        ] {
            let settled = settle_step_simulation(StepBackendOutcome::Holds { certificate });
            assert_eq!(settled.value, proved(certification));
            assert_eq!(settled.basis, SettlementBasis::DecisiveWitness);
            assert_eq!(settled.truth, Truth::Satisfied);
        }
    }

    /// TC-551 step 4, settlement only: a counterexample settles
    /// `Inconclusive(InductionNotClosed{1})`, never `refuted`; no candidate
    /// settles `unsupported`.
    #[trace("TC-551")]
    #[test]
    fn a_counterexample_is_inconclusive_and_no_candidate_is_unsupported() {
        let failed = settle_step_simulation(StepBackendOutcome::Counterexample);
        assert_eq!(
            failed.value,
            TerminalValue::Inconclusive(InconclusiveCause::InductionNotClosed { depth: 1 })
        );
        assert_eq!(failed.basis, SettlementBasis::Unsettled);
        assert_ne!(failed.value, TerminalValue::Refuted);
        let none = settle_step_simulation(StepBackendOutcome::NoCandidate);
        assert_eq!(
            none.value,
            TerminalValue::Unsupported(UnavailabilityCause::BackendAbsent)
        );
        assert_eq!(none.truth, Truth::Unavailable);
    }

    /// ADR-018 PC-2: a certificate the core shows wrong never settles
    /// `proved`.
    #[trace("TC-551", "FR-146-AC-7")]
    #[test]
    fn a_rejected_certificate_settles_inconclusive_never_proved() {
        let at = CertificateLocus::Query {
            part: crate::certificate::QueryPart::Step,
        };
        let settled = settle_step_simulation(StepBackendOutcome::Holds {
            certificate: BackendCertificate::Rejected {
                rule: CertificateRule::QueryMismatch,
                at: at.clone(),
            },
        });
        assert_eq!(
            settled.value,
            TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
                rule: CertificateRule::QueryMismatch,
                at,
            })
        );
        assert_eq!(settled.basis, SettlementBasis::Unsettled);
        assert_eq!(settled.truth, Truth::Pending);
    }
}
