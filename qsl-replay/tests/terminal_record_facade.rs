// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-177 (FR-069-AC-1): a client outside the crate, the code generator,
//! builds terminal records and reads them into envelopes through
//! `qsl_replay`'s root re-exports alone. As an integration test it reaches
//! only the crate's public API, so a re-export that is not `pub` (such as
//! `RequestIndex` or `Code`) fails to compile here.

use ix_trace_rs::trace;
use qsl_replay::{
    read_backend_provider_envelope, BackendProviderSource, Category, Code, InconclusiveCause,
    ProofRefusalCause, ReportedInconclusiveCause, RequestIndex, TerminalRecord, TerminalValue,
};

/// Each record is keyed by the `RequestIndex` it was built with, a declined
/// record carries its code, and an inconclusive record's envelope reports
/// its own cause.
#[trace("TC-177", "FR-069-AC-1")]
#[test]
fn terminal_records_are_built_and_read_through_the_facade_alone() {
    let values = [
        TerminalValue::Declined {
            cause: ProofRefusalCause::InvalidInput,
            code: Code::MissingDeclaration,
        },
        TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(Code::StaleDependency)),
        TerminalValue::Proved { success_checks: 0 },
    ];
    let source = BackendProviderSource {
        backend_identity: "kani-backend-1".to_owned(),
        items: values
            .iter()
            .enumerate()
            .map(|(index, value)| TerminalRecord::new(RequestIndex::new(index), value.clone()))
            .collect(),
    };
    let envelopes = read_backend_provider_envelope(&source).unwrap();

    let categories: Vec<Category> = envelopes.iter().map(|e| e.category()).collect();
    assert_eq!(
        categories,
        [
            Category::Refusal,
            Category::Inconclusive,
            Category::Inconclusive
        ]
    );
    for (index, envelope) in envelopes.iter().enumerate() {
        assert_eq!(envelope.record().request_index().get(), index);
        assert_eq!(envelope.record().value(), &values[index]);
    }
    assert_eq!(envelopes[0].inconclusive_cause(), None);
    assert_eq!(
        envelopes[1].inconclusive_cause(),
        Some(&ReportedInconclusiveCause::Cause(
            InconclusiveCause::ReplayRefused(Code::StaleDependency)
        ))
    );
    assert_eq!(
        envelopes[2].inconclusive_cause(),
        Some(&ReportedInconclusiveCause::KaniVacuousProof)
    );
}
