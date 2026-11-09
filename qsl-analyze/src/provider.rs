// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-280's in-process provider boundary over the same admitted analyze request.

use crate::analyze::{
    analyze, AnalyzeLimits, AnalyzeRefusal, AnalyzeRequest, Engines, ItemCounterexample,
    NegotiatedItem, SourceProvision, SubjectBinding,
};
use qsl_foundation::diagnostic::{Code, StageFailure};
use qsl_package::CheckedPackage;
use qsl_replay::{DeclineCode, ProofRefusalCause, TerminalRecord};
use quire_exact::Cancel;
use quire_semantic_value::catalog::CatalogCode;

/// Records and reproduced counterexamples for one provider request.
#[derive(Debug)]
pub struct ProviderResults {
    /// One terminal record per requested item, in input order.
    pub records: Vec<TerminalRecord>,
    /// Exactly one reproduced counterexample for each refuted record.
    pub counterexamples: Vec<ItemCounterexample>,
}

/// Run the caller's negotiated items over its checked package, source bytes
/// and subject binding. This boundary chooses no engine and reads no file.
pub fn run(
    package: &CheckedPackage,
    binding: &SubjectBinding<'_>,
    provision: &SourceProvision,
    items: &[NegotiatedItem],
    engines: &Engines,
    limits: &AnalyzeLimits,
    cancel: &Cancel,
) -> ProviderResults {
    let request = AnalyzeRequest {
        binding: binding.clone(),
        provision: provision.clone(),
        items: items.to_vec(),
    };
    match analyze(package, &request, engines, limits, cancel) {
        Ok(staged) => {
            let outcome = staged.into_value();
            ProviderResults {
                records: outcome.records,
                counterexamples: outcome.counterexamples,
            }
        }
        Err(failure) => {
            let (cause, diagnostic) = match failure {
                StageFailure::Refused(AnalyzeRefusal::Provision(refusal)) => (
                    Some(ProofRefusalCause::IncompleteInput),
                    refusal.catalog_code(),
                ),
                StageFailure::Refused(AnalyzeRefusal::Binding(failure)) => match failure {
                    qsl_semantics::model::observation::AdmissionFailure::Refused(record) => {
                        (Some(ProofRefusalCause::InvalidInput), record.catalog)
                    }
                    qsl_semantics::model::observation::AdmissionFailure::Incomplete(record) => {
                        (Some(ProofRefusalCause::IncompleteInput), record.catalog)
                    }
                    qsl_semantics::model::observation::AdmissionFailure::Fault(fault) => {
                        (None, fault.catalog_code())
                    }
                },
                _ => (
                    None,
                    CatalogCode::new("runtime_invariant", "established-invariant-broken"),
                ),
            };
            let records = items
                .iter()
                .map(|item| match cause {
                    Some(cause) => TerminalRecord::disposition_declined(
                        item.item.request_index,
                        cause,
                        DeclineCode::Qsl(
                            Code::from_code(diagnostic.code()).unwrap_or(Code::RuntimeInvariant),
                        ),
                        diagnostic,
                    ),
                    None => TerminalRecord::disposition_failed(item.item.request_index, diagnostic),
                })
                .collect();
            ProviderResults {
                records,
                counterexamples: Vec::new(),
            }
        }
    }
}
