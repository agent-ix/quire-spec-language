// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `proved` records a process provider and a Kani run settle, read back
//! through the backend-provider envelope: the provider's own claim is
//! `trusted`, the Kani check-count record is `certified`, each naming its
//! producing backend.
//!
//! Not covered here: the `analyze` records for an EN-5 item (`certified`,
//! naming the EN-5 checker) and for a test engine with no checker
//! (`uncertified`, naming the engine). `Engines::qsl` holds only EN-1 and
//! EN-4, and no engine without a checker exists to run.

use qsl_foundation::BackendId;
use qsl_replay::{
    read_backend_provider_envelope, BackendProviderSource, Certification, PluginResultRefusal,
    ProofBasis, ReplayLimits, RequestIndex, TerminalRecord, TerminalValue,
};
use quire_semantic_value::catalog::Category;

fn read_back(backend: &str, record: TerminalRecord) -> (TerminalValue, Category, String) {
    let envelopes = read_backend_provider_envelope(
        &BackendProviderSource {
            backend_identity: backend.to_owned(),
            items: vec![record],
        },
        ReplayLimits::default(),
    )
    .unwrap();
    assert_eq!(envelopes.len(), 1);
    (
        envelopes[0].record().value().clone(),
        envelopes[0].category(),
        envelopes[0].backend().as_str().to_owned(),
    )
}

#[test]
fn plugin_proof_is_trusted_and_kani_proof_is_certified() {
    let plugin = TerminalRecord::from_plugin_proof(
        RequestIndex::new(0),
        BackendId::new("acme-prover"),
        ProofBasis::Exhaustive,
    )
    .unwrap();
    let (value, category, backend) = read_back("acme-prover", plugin);
    assert_eq!(
        value,
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Trusted,
        }
    );
    assert_eq!(category, Category::Success);
    assert_eq!(backend, "acme-prover");

    let kani = TerminalRecord::proved(RequestIndex::new(1), BackendId::new("kani-backend-1"), 3);
    let (value, category, backend) = read_back("kani-backend-1", kani);
    assert_eq!(
        value,
        TerminalValue::Proved {
            basis: ProofBasis::Checks { success_checks: 3 },
            certification: Certification::Certified,
        }
    );
    assert_eq!(category, Category::Success);
    assert_eq!(backend, "kani-backend-1");

    // The check-count basis is the core's to certify, never a plugin's.
    assert_eq!(
        TerminalRecord::from_plugin_proof(
            RequestIndex::new(0),
            BackendId::new("acme-prover"),
            ProofBasis::Checks { success_checks: 3 },
        ),
        Err(PluginResultRefusal::CoreOnlyBasis)
    );
}
