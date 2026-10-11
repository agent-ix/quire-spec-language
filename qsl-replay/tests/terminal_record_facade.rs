// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-773 / FR-289-AC-3: a client outside the crate builds a terminal record
//! only from a replay result and a producing backend identity, then reads it
//! through `qsl_replay`'s root re-exports alone.

use ix_trace_rs::trace;
use qsl_foundation::BackendId;
use qsl_replay::{
    read_backend_provider_envelope, BackendProviderSource, CallSiteRefusal, Category, DeclineCode,
    EvaluatedValue, InconclusiveCause, InputArmResult, InternalFault, ProofRefusalCause,
    ReplayRefusal, ReplayResult, ReportedInconclusiveCause, RequestIndex, TerminalRecord,
    TerminalValue, Verdict, WitnessArmResult, WitnessCheck,
};

fn violating_replay() -> ReplayResult {
    ReplayResult::Witness(WitnessArmResult::settle_range_violation(
        None,
        Vec::new(),
        qsl_replay::spine::default_accounting(0),
    ))
}

/// A violation replay is the only public source of a refuted terminal record;
/// the backend identity is carried through the envelope and its round trip.
#[trace("TC-773", "FR-289-AC-3")]
#[test]
fn refuted_record_requires_replay_and_backend_evidence() {
    let replay = violating_replay();
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
    assert_eq!(first[0].record().replay_result(), Some(&replay));

    let serialized = qsl_replay::ProofResultEnvelope::to_source(&first).unwrap();
    let second =
        read_backend_provider_envelope(&serialized, qsl_replay::ReplayLimits::default()).unwrap();
    assert_eq!(second[0].record().value(), &TerminalValue::Refuted);
    assert_eq!(second[0].backend().as_str(), "replay-backend");
    assert_eq!(second[0].record().replay_result(), Some(&replay));
}

/// FR-289-AC-3 / TC-773 step 3: a replay that did not reproduce a violation
/// cannot be relabelled as `refuted`.
#[trace("TC-773", "FR-289-AC-3")]
#[test]
fn non_violating_replay_is_refused() {
    let replay = ReplayResult::Witness(WitnessArmResult::settle(
        qsl_replay::Verdict::from_category(Category::Success),
        qsl_replay::Verdict::from_category(Category::Success),
        Category::Success,
        Some(EvaluatedValue::Boolean(true)),
        qsl_replay::WitnessCheck::Agrees(None),
        Vec::new(),
        qsl_replay::spine::default_accounting(0),
    ));

    assert_eq!(
        TerminalRecord::new(
            RequestIndex::new(9),
            BackendId::new("replay-backend"),
            &replay,
        ),
        Err(qsl_replay::TerminalRecordRefusal::ReplayDidNotRefute(
            Category::Success
        ))
    );
}

/// A violation category alone cannot turn an unsettled witness replay into
/// refutation, whether parity failed or the replay completed no value.
#[trace("TC-773", "FR-289-AC-3")]
#[test]
fn inconclusive_witness_replay_cannot_refute() {
    let violation = Verdict::from_category(Category::Violation);
    let success = Verdict::from_category(Category::Success);
    for (replayed, value) in [
        (success, Some(EvaluatedValue::Boolean(false))),
        (violation, None),
    ] {
        let replay = ReplayResult::Witness(WitnessArmResult::settle(
            violation,
            replayed,
            Category::Violation,
            value,
            WitnessCheck::Agrees(None),
            Vec::new(),
            qsl_replay::spine::default_accounting(0),
        ));
        assert_eq!(
            TerminalRecord::new(RequestIndex::new(0), BackendId::new("backend"), &replay),
            Err(qsl_replay::TerminalRecordRefusal::ReplayDidNotRefute(
                Category::Inconclusive
            ))
        );
    }
}

/// An input arm can report a violation, but it cannot supply the witness
/// settlement needed for a refuted backend record.
#[trace("TC-773", "FR-289-AC-3")]
#[test]
fn violating_input_arm_cannot_refute() {
    let violation = Verdict::from_category(Category::Violation);
    let replay = ReplayResult::Input(InputArmResult::settle(
        violation,
        violation,
        Category::Violation,
        Some(EvaluatedValue::Boolean(false)),
        &WitnessCheck::Agrees(None),
        Vec::new(),
        qsl_replay::spine::default_accounting(0),
    ));
    assert_eq!(
        TerminalRecord::new(RequestIndex::new(0), BackendId::new("backend"), &replay),
        Err(qsl_replay::TerminalRecordRefusal::ReplayLacksWitness)
    );
}

/// FR-289-AC-3 / TC-773: a provider envelope cannot claim a record
/// produced by a different backend.
#[trace("TC-773", "FR-289-AC-3")]
#[test]
fn record_backend_must_match_provider_identity() {
    let record = TerminalRecord::new(
        RequestIndex::new(4),
        BackendId::new("record-backend"),
        &violating_replay(),
    )
    .unwrap();
    let result = read_backend_provider_envelope(
        &BackendProviderSource {
            backend_identity: "manifest-backend".to_owned(),
            items: vec![record],
        },
        qsl_replay::ReplayLimits::default(),
    );
    assert!(matches!(
        result,
        Err(qsl_replay::ProofResultRefusal::BackendIdentityMismatch { .. })
    ));
}

/// TC-177 / FR-069-AC-1: public constructors for non-refuted values remain
/// available to an external facade client.
#[trace("TC-177", "FR-069-AC-1")]
#[test]
fn non_refuted_records_are_constructible_through_the_facade() {
    let backend = BackendId::new("provider-backend");
    let records = vec![
        TerminalRecord::declined(
            RequestIndex::new(0),
            backend.clone(),
            ProofRefusalCause::InvalidInput,
            DeclineCode::Qsl(qsl_replay::Code::MissingDeclaration),
        ),
        TerminalRecord::inconclusive(
            RequestIndex::new(1),
            backend.clone(),
            InconclusiveCause::ReplayRefused(qsl_replay::Code::StaleDependency),
        ),
        TerminalRecord::proved(RequestIndex::new(2), backend.clone(), 1),
        TerminalRecord::failed(RequestIndex::new(3), backend.clone()),
    ];
    let envelopes = read_backend_provider_envelope(
        &BackendProviderSource {
            backend_identity: backend.as_str().to_owned(),
            items: records,
        },
        qsl_replay::ReplayLimits::default(),
    )
    .unwrap();
    assert_eq!(envelopes[0].record().value().category(), Category::Refusal);
    assert_eq!(
        envelopes[1].record().value().category(),
        Category::Inconclusive
    );
    assert_eq!(
        envelopes[2].record().value(),
        &TerminalValue::Proved {
            basis: qsl_replay::ProofBasis::Checks { success_checks: 1 },
            certification: qsl_replay::Certification::Certified,
        }
    );
    assert_eq!(envelopes[3].record().value(), &TerminalValue::Failed);
}

/// FR-127-AC-2 / AC-9: the certificate rejection labels and typed query/proof-step loci
/// remain constructible and observable through the public facade.
#[trace("TC-522", "FR-127-AC-2", "FR-127-AC-9")]
#[test]
fn certificate_rejection_vocabulary_is_constructible_through_the_public_facade() {
    use qsl_replay::{CertificateLocus, CertificateRule, QueryPart};

    let backend = BackendId::new("certificate-provider");
    let values = [
        InconclusiveCause::CertificateRejected {
            rule: CertificateRule::QueryMismatch,
            at: CertificateLocus::Query {
                part: QueryPart::Unrolling,
            },
        },
        InconclusiveCause::CertificateRejected {
            rule: CertificateRule::ProofStepInvalid,
            at: CertificateLocus::ProofStep {
                part: QueryPart::Step,
                index: u64::MAX,
            },
        },
    ];
    let source = BackendProviderSource {
        backend_identity: backend.as_str().to_owned(),
        items: values
            .iter()
            .enumerate()
            .map(|(index, cause)| {
                TerminalRecord::inconclusive(
                    RequestIndex::new(index),
                    backend.clone(),
                    cause.clone(),
                )
            })
            .collect(),
    };
    let read =
        read_backend_provider_envelope(&source, qsl_replay::ReplayLimits::default()).unwrap();
    assert_eq!(read.len(), values.len());
    for (envelope, cause) in read.iter().zip(&values) {
        assert_eq!(envelope.category(), Category::Inconclusive);
        assert_eq!(
            envelope.record().value(),
            &TerminalValue::Inconclusive(cause.clone())
        );
        assert_eq!(
            envelope.inconclusive_cause(),
            Some(&ReportedInconclusiveCause::Cause(cause.clone()))
        );
    }
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
