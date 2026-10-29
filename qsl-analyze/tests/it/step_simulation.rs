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
use quire_exact::{Cancel, NodeKey};

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
    let hypothesis = NodeKey::from_digest([7; 32]);
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
                    hypotheses: vec![(hypothesis, proved(Certification::Certified))],
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
