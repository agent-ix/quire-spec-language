// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-146 "Routing and settlement" (TC-551) end to end through the analyze
//! crate's settlement of a refinement's per-step simulation records: the SMT
//! backend's results for `CasRefinesCounter`'s eight records and its
//! certificate settle each record `Proved{Inductive{1}}`, `Certified` only
//! when FR-314 verifies the certificate.

use ix_trace_rs::trace;
use qsl_analyze::step_simulation::{
    settle_step_simulations, StepRecordResult, StepSimulationItem, StepSimulationRefusal,
};
use qsl_foundation::{BackendId, RequestIndex};
use qsl_replay::{
    CertificateCheckLimits, Certification, ProofBasis, SettlementBasis, TerminalValue,
};
use quire_exact::Cancel;

use crate::refinement_fixture::{cas_refines_counter, World};
use crate::refinement_proofs::{certificate, with_request};

fn proved(certification: Certification) -> TerminalValue {
    TerminalValue::Proved {
        basis: ProofBasis::Inductive { depth: 1 },
        certification,
    }
}

/// The results an SMT backend returns when every record's obligation holds.
fn holding() -> Vec<StepRecordResult> {
    (0..8)
        .map(|index| StepRecordResult {
            request_index: RequestIndex::new(index),
            result: Some(proved(Certification::Uncertified)),
        })
        .collect()
}

#[trace("TC-551")]
#[test]
fn a_verified_certificate_settles_every_record_certified() {
    let world = World::new(&cas_refines_counter());
    let backend = BackendId::new("smt");
    // The hypothesis items are the invariant clauses the records list, each
    // proved and certified.
    let entries = world.refinement().step_simulations();
    assert_eq!(entries.len(), 8);
    let hypotheses: Vec<_> = entries[0]
        .record()
        .hypotheses
        .iter()
        .map(|clause| (*clause, proved(Certification::Certified)))
        .collect();
    with_request(&world, |request| {
        let valid = certificate(request, 7);
        for (certificate, certification) in [
            (Some(valid), Certification::Certified),
            (None, Certification::Uncertified),
        ] {
            let settled = settle_step_simulations(
                StepSimulationItem {
                    request,
                    backend: backend.clone(),
                    records: holding(),
                    certificate,
                    hypotheses: hypotheses.clone(),
                    certificate_limits: CertificateCheckLimits::default(),
                    replay_initial: Box::new(|| panic!("no initial counterexample")),
                },
                &Cancel::new(),
            )
            .expect("the records settle");
            assert_eq!(settled.records.len(), 8);
            for (index, record) in settled.records.iter().enumerate() {
                assert_eq!(record.value(), &proved(certification));
                assert_eq!(record.backend(), Some(&backend));
                assert_eq!(record.request_index(), RequestIndex::new(index));
            }
            assert_eq!(settled.safety.value, proved(certification));
            assert_eq!(settled.safety.basis, SettlementBasis::DecisiveWitness);
        }

        let mut tested = holding();
        tested[3].result = Some(TerminalValue::Tested);
        let refused = settle_step_simulations(
            StepSimulationItem {
                request,
                backend: backend.clone(),
                records: tested,
                certificate: None,
                hypotheses: Vec::new(),
                certificate_limits: CertificateCheckLimits::default(),
                replay_initial: Box::new(|| panic!("no initial counterexample")),
            },
            &Cancel::new(),
        );
        assert_eq!(
            refused.map(|settled| settled.records.len()),
            Err(StepSimulationRefusal::Result {
                request_index: RequestIndex::new(3)
            })
        );
    });
}

/// Settle `records` of `request` with `certificate` and `hypotheses`.
fn settle(
    request: &qsl_eval::smt::RefinementCertificateRequest<'_>,
    records: Vec<StepRecordResult>,
    certificate: Option<qsl_replay::RefinementSmtCertificate>,
    hypotheses: Vec<(quire_exact::NodeKey, TerminalValue)>,
) -> Result<qsl_analyze::step_simulation::StepSimulationRecords, StepSimulationRefusal> {
    settle_step_simulations(
        StepSimulationItem {
            request,
            backend: BackendId::new("smt"),
            records,
            certificate,
            hypotheses,
            certificate_limits: CertificateCheckLimits::default(),
            replay_initial: Box::new(|| panic!("no initial counterexample")),
        },
        &Cancel::new(),
    )
}

/// SR-1821001 FND-001 (TC-551): the records are checked against the
/// refinement. With the `Initial` record alone, with or without a valid
/// certificate, and with nine records, `settle_step_simulations` refuses
/// `RecordCount`, never settling the half `proved` over step obligations
/// that have no result.
#[trace("TC-551")]
#[test]
fn records_that_are_not_the_refinements_are_refused() {
    let world = World::new(&cas_refines_counter());
    with_request(&world, |request| {
        let valid = certificate(request, 7);
        for certificate in [None, Some(valid)] {
            let mut initial = holding();
            initial.truncate(1);
            assert_eq!(
                settle(request, initial, certificate, Vec::new()).map(|_| ()),
                Err(StepSimulationRefusal::RecordCount {
                    expected: 8,
                    given: 1
                })
            );
        }
        let mut nine = holding();
        nine.push(StepRecordResult {
            request_index: RequestIndex::new(8),
            result: Some(proved(Certification::Uncertified)),
        });
        assert_eq!(
            settle(request, nine, None, Vec::new()).map(|_| ()),
            Err(StepSimulationRefusal::RecordCount {
                expected: 8,
                given: 9
            })
        );
    });
}

const CAS_INV: &str = "invariant CasInv using inf on Impl::Counter at current { \
     self.busyA or self.tmpA <= self.value }\n";

/// SR-1821001 FND-001 and FR-146-AC-8 (TC-551): over `CasRefinesCounter`
/// with `CasInv`, an empty hypothesis list is refused rather than dropping
/// the hypotheses rule; an unproved `CasInv` item settles the half
/// `inconclusive`, `hypotheses-not-established` naming `CasInv`, although
/// every record holds; an `Uncertified` `CasInv` item settles it `proved`,
/// `Uncertified`.
#[trace("TC-551", "FR-146-AC-8")]
#[test]
fn the_hypothesis_items_decide_the_half() {
    let world = World::new(&format!("{}\n{CAS_INV}", cas_refines_counter()));
    let clauses = world.refinement().step_simulations()[0]
        .record()
        .hypotheses
        .clone();
    let [cas_inv] = clauses.as_slice() else {
        panic!("CasInv is the one hypothesis");
    };
    with_request(&world, |request| {
        assert_eq!(
            settle(request, holding(), None, Vec::new()).map(|_| ()),
            Err(StepSimulationRefusal::Hypotheses)
        );
        let unproved =
            TerminalValue::Inconclusive(qsl_replay::InconclusiveCause::InductionNotClosed {
                depth: 1,
            });
        let settled = settle(request, holding(), None, vec![(*cas_inv, unproved)])
            .expect("the records settle");
        assert_eq!(
            settled.safety.value,
            TerminalValue::Inconclusive(qsl_replay::InconclusiveCause::HypothesesNotEstablished {
                hypothesis: qsl_foundation::digest::WireNodeId::from_digest(*cas_inv.as_bytes()),
                result: qsl_replay::ItemLabel::Inconclusive,
            })
        );
        assert!(settled
            .records
            .iter()
            .all(|record| record.value() == &proved(Certification::Uncertified)));

        let settled = settle(
            request,
            holding(),
            None,
            vec![(*cas_inv, proved(Certification::Uncertified))],
        )
        .expect("the records settle");
        assert_eq!(settled.safety.value, proved(Certification::Uncertified));
        assert_eq!(settled.safety.basis, SettlementBasis::DecisiveWitness);
    });
}
