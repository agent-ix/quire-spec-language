// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-773 / FR-289-AC-3: a client outside the crate builds a terminal record
//! only from a replay result and a producing backend identity, then reads it
//! through `qsl_replay`'s root re-exports alone.

use ix_trace_rs::trace;
use qsl_foundation::BackendId;
use qsl_replay::{
    read_backend_provider_envelope, BackendProviderSource, CallSiteRefusal, Category,
    InternalFault, ReplayRefusal, ReplayResult, RequestIndex, TerminalRecord, TerminalValue,
    WitnessArmResult,
};

/// A violation replay is the only public source of a refuted terminal record;
/// the backend identity is carried through the envelope and its round trip.
#[trace("TC-773", "FR-289-AC-3")]
#[test]
fn refuted_record_requires_replay_and_backend_evidence() {
    let replay = ReplayResult::Witness(WitnessArmResult::settle_range_violation(
        None,
        Vec::new(),
        qsl_replay::spine::default_accounting(0),
    ));
    let record = TerminalRecord::new(
        RequestIndex::new(7),
        BackendId::new("replay-backend"),
        &replay,
    )
    .unwrap();
    let source = BackendProviderSource {
        backend_identity: "replay-backend".to_owned(),
        items: vec![record],
    };
    let first =
        read_backend_provider_envelope(&source, qsl_replay::ReplayLimits::default()).unwrap();
    assert_eq!(first[0].record().value(), &TerminalValue::Refuted);
    assert_eq!(first[0].backend().as_str(), "replay-backend");
    assert_eq!(
        first[0].record().backend().unwrap().as_str(),
        "replay-backend"
    );

    let serialized = qsl_replay::ProofResultEnvelope::to_source(&first).unwrap();
    let second =
        read_backend_provider_envelope(&serialized, qsl_replay::ReplayLimits::default()).unwrap();
    assert_eq!(second[0].record().value(), &TerminalValue::Refuted);
    assert_eq!(second[0].backend().as_str(), "replay-backend");
}

/// FR-121-AC-16: an `InternalFault` built through `qsl_replay::InternalFault`
/// is a `ReplayRefusal::Fault`, and `TerminalValue::from_replay_refusal`
/// settles it `Failed` (an internal failure), never `Inconclusive`.
#[trace("TC-516", "FR-121-AC-16")]
#[test]
fn a_replay_fault_built_through_the_facade_settles_failed() {
    let fault = InternalFault::new("terminal-map", "kind-total");
    assert_eq!(fault.category(), Category::InternalFailure);
    let settled = TerminalValue::from_replay_refusal(&ReplayRefusal::Fault(fault));
    assert_eq!(settled, TerminalValue::Failed);
    assert_eq!(settled.category(), Category::InternalFailure);
}

/// FR-121-AC-17: a `CallSiteRefusal::Fault` built through the facade settles
/// `Failed` by `TerminalValue::from_call_site_refusal`, never `Declined`.
#[trace("TC-516", "FR-121-AC-17")]
#[test]
fn a_call_site_fault_built_through_the_facade_settles_failed() {
    let fault = CallSiteRefusal::Fault(InternalFault::new("call-site", "function-node"));
    let settled = TerminalValue::from_call_site_refusal(&fault);
    assert_eq!(settled, TerminalValue::Failed);
    assert_eq!(settled.category(), Category::InternalFailure);
}
