// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-031: typed extraction provenance inside native command output.

use super::{diagnostic, diagnostic_with_upstream, source, types};
use crate::command::compilation::RunPackage;
use crate::command::extraction::{ExtractionError, ExtractionMode, JoinCause, JoinFailure};
use qsl_foundation::{Diagnostic, LocatedSpan, SourceIdentity, Span};
use qsl_source::{
    ClauseRef, ClausesOutcome, KindAvailability, SemanticDiagnostic, SemanticFailure,
    CONTRACT_VERSION, SEMANTIC_CORE_VERSION,
};
use quire_contract_ir as ir;
use serde::{Serialize, Serializer};
use std::collections::BTreeMap;

pub(super) enum Stage {
    Selection,
    Context,
    Quire,
}

impl Stage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Selection => "extraction-selection",
            Self::Context => "quire-context",
            Self::Quire => "quire",
        }
    }
}

#[derive(Serialize)]
struct Outcome<'a> {
    availability: &'a KindAvailability,
    clauses: &'a Option<Vec<ClauseRef>>,
    clause_text: &'a BTreeMap<String, String>,
    diagnostics: &'a [SemanticDiagnostic],
}

impl<'a> From<&'a ClausesOutcome> for Outcome<'a> {
    fn from(value: &'a ClausesOutcome) -> Self {
        Self {
            availability: &value.availability,
            clauses: &value.clauses,
            clause_text: &value.clause_text,
            diagnostics: &value.diagnostics,
        }
    }
}

#[derive(Serialize)]
struct Segment {
    body: Span,
    original: Span,
}

#[derive(Serialize)]
struct Mapping {
    region: Span,
    segments: Vec<Segment>,
    drop_final_newline: bool,
}

#[derive(Serialize)]
pub(super) struct Extracted<'a> {
    original: types::Source<'a>,
    outcome: Outcome<'a>,
    mapping: Mapping,
}

pub(super) fn context<'a>(package: &'a RunPackage<'_>) -> Option<Extracted<'a>> {
    match package {
        RunPackage::Native(_) => None,
        RunPackage::Extracted(value) => {
            let mapping = value.package.mapped().mapping();
            Some(Extracted {
                original: source(&value.original),
                outcome: value.package.extraction().into(),
                mapping: Mapping {
                    region: mapping.region(),
                    segments: mapping
                        .segments()
                        .iter()
                        .map(|segment| Segment {
                            body: segment.body,
                            original: segment.original,
                        })
                        .collect(),
                    drop_final_newline: mapping.layout().drop_final_newline,
                },
            })
        }
    }
}

#[derive(Serialize)]
struct Original<'a> {
    #[serde(flatten)]
    identity: &'a SourceIdentity,
    path: &'a str,
    digest: String,
}

#[derive(Serialize)]
struct Selection<'a> {
    name: &'a str,
    requirement: &'a ir::RequirementRef,
    clause: &'a ir::ClauseId,
    execution_point: &'a ir::ExecutionPoint,
}

struct MappedSpans(Result<Option<Vec<LocatedSpan>>, Box<Diagnostic>>);

impl Serialize for MappedSpans {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct MappingError<'a> {
            mapping_error: types::Diagnostic<'a>,
        }
        match &self.0 {
            Ok(spans) => spans.serialize(serializer),
            Err(error) => MappingError {
                mapping_error: diagnostic(error),
            }
            .serialize(serializer),
        }
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum Cause<'a> {
    Preflight {
        preflight: &'a qsl_source::PreflightFailure,
    },
    Join(types::Diagnostic<'a>),
    Compile {
        diagnostic: Option<types::Diagnostic<'a>>,
        original_spans: MappedSpans,
    },
}

#[derive(Serialize)]
pub(super) struct ConsumerFailure<'a> {
    original: Original<'a>,
    selection: Selection<'a>,
    outcome: Option<Outcome<'a>>,
    cause: Cause<'a>,
}

fn consumer_failure(value: &JoinFailure) -> ConsumerFailure<'_> {
    let original = value.original();
    let binding = value.binding();
    let cause = match &value.cause {
        JoinCause::Preflight(error) => Cause::Preflight { preflight: error },
        JoinCause::Join(error) => Cause::Join(diagnostic(error)),
        JoinCause::Compile(error) => Cause::Compile {
            diagnostic: error
                .native_diagnostic()
                .map(|value| diagnostic_with_upstream(value, error.upstream_diagnostic())),
            original_spans: MappedSpans(error.original_spans()),
        },
    };
    ConsumerFailure {
        original: Original {
            identity: original.identity(),
            path: original.path(),
            digest: original.digest().to_string(),
        },
        selection: Selection {
            name: &binding.name,
            requirement: &binding.requirement,
            clause: &binding.clause,
            execution_point: &binding.execution_point,
        },
        outcome: value.extraction().map(Into::into),
        cause,
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub(super) enum Failure<'a> {
    Mode(&'a ExtractionMode),
    Context {
        contract_version: &'static str,
        semantic_core: &'static str,
        diagnostics: &'a [SemanticFailure],
    },
    Consumer(Box<ConsumerFailure<'a>>),
}

pub(super) fn failure(value: &ExtractionError) -> (types::Stage, Failure<'_>) {
    let (stage, details) = match value {
        ExtractionError::Mode(mode) => (Stage::Selection, Failure::Mode(mode)),
        ExtractionError::Context(diagnostics) => (
            Stage::Context,
            Failure::Context {
                contract_version: CONTRACT_VERSION,
                semantic_core: SEMANTIC_CORE_VERSION,
                diagnostics,
            },
        ),
        ExtractionError::Compile(error) => (
            Stage::Quire,
            Failure::Consumer(Box::new(consumer_failure(error))),
        ),
    };
    (types::Stage::Extraction(stage), details)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{RunCause, RunError};
    use ix_trace_rs::trace;

    #[test]
    #[trace("TC-109", "FR-031-AC-2")]
    fn rejected_quire_context_keeps_producer_diagnostics_and_selected_versions() {
        let diagnostics = qsl_source::read_semantic_block(
            &serde_json::json!({"contract_version":"future", "semantic_core":SEMANTIC_CORE_VERSION,
                "package":"example/rules", "exports":[], "targets":["markdown"]}),
            &[],
            &|_| false,
        )
        .unwrap_err();
        assert!(!diagnostics.is_empty());
        let expected = serde_json::to_value(&diagnostics).unwrap();
        let error = RunError {
            request_digest: None,
            cause: RunCause::Extraction(Box::new(ExtractionError::Context(diagnostics))),
        };
        assert_eq!(error.exit_code(), 20);
        let value = error.value().unwrap();
        assert_eq!(value["stage"], "quire-context");
        assert_eq!(value["code"], "invalid-quire-context");
        assert_eq!(value["details"]["contract_version"], "1.0.0");
        assert_eq!(value["details"]["semantic_core"], "0.1.0");
        assert_eq!(value["details"]["diagnostics"], expected);
        assert!(value.get("truth").is_none());
    }
}
