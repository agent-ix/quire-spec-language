// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-177 (FR-069-AC-1): a client outside the crate, the code generator,
//! builds terminal records and reads them into envelopes through
//! `qsl_replay`'s root re-exports alone. As an integration test it reaches
//! only the crate's public API, so a re-export that is not `pub` (such as
//! `RequestIndex`, `Code` or `Std001Code`) fails to compile here.

use ix_trace_rs::trace;
use qsl_replay::{
    read_backend_provider_envelope, std001_code, BackendProviderSource, CallSiteRefusal, Category,
    Certification, Code, DeclineCode, InconclusiveCause, InternalFault, ProofBasis,
    ProofRefusalCause, ReplayRefusal, ReportedInconclusiveCause, RequestIndex, Std001Code,
    TerminalRecord, TerminalValue,
};

/// Each record is keyed by the `RequestIndex` it was built with, a declined
/// record carries its code (a QSL catalog code or a STD-001 registry code,
/// each kept in its own registry), and an inconclusive record's envelope
/// reports its own cause.
#[trace("TC-177", "FR-069-AC-1")]
#[test]
fn terminal_records_are_built_and_read_through_the_facade_alone() {
    let values = [
        TerminalValue::Declined {
            cause: ProofRefusalCause::InvalidInput,
            code: DeclineCode::Qsl(Code::MissingDeclaration),
        },
        TerminalValue::Declined {
            cause: ProofRefusalCause::Refused,
            code: DeclineCode::Std001(std001_code!("kani_corpus_identity_collision")),
        },
        TerminalValue::Declined {
            cause: ProofRefusalCause::Refused,
            code: DeclineCode::Std001(Std001Code::new("kani_proved_vacuously").unwrap()),
        },
        TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(Code::StaleDependency)),
        TerminalValue::Proved {
            basis: ProofBasis::Checks { success_checks: 0 },
            certification: Certification::Certified,
        },
    ];
    let source = BackendProviderSource {
        backend_identity: "kani-backend-1".to_owned(),
        items: values
            .iter()
            .enumerate()
            .map(|(index, value)| TerminalRecord::new(RequestIndex::new(index), value.clone()))
            .collect(),
    };
    let envelopes =
        read_backend_provider_envelope(&source, qsl_replay::ReplayLimits::default()).unwrap();

    let categories: Vec<Category> = envelopes.iter().map(|e| e.category()).collect();
    assert_eq!(
        categories,
        [
            Category::Refusal,
            Category::Refusal,
            Category::Refusal,
            Category::Inconclusive,
            Category::Inconclusive
        ]
    );
    for (index, envelope) in envelopes.iter().enumerate() {
        assert_eq!(envelope.record().request_index().get(), index);
        assert_eq!(envelope.record().value(), &values[index]);
    }
    assert_ne!(envelopes[0].record().value(), envelopes[1].record().value());
    assert_eq!(envelopes[0].inconclusive_cause(), None);
    assert_eq!(
        envelopes[3].inconclusive_cause(),
        Some(&ReportedInconclusiveCause::Cause(
            InconclusiveCause::ReplayRefused(Code::StaleDependency)
        ))
    );
    assert_eq!(
        envelopes[4].inconclusive_cause(),
        Some(&ReportedInconclusiveCause::KaniVacuousProof)
    );
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
