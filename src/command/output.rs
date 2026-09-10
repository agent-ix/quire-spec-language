// SPDX-License-Identifier: AGPL-3.0-only
//! FR-026: construct typed JSON views of native outcomes and actual provenance.

mod types;

use super::{wire, RunCause, RunError, RunResult};
use crate::formal_source::FormalSource;
use crate::native_model::NativeModel;
use crate::runtime::{
    EvaluationOutcome, ExecutionOutcome, ExecutionReport, ImplicationEventKind, RuntimePathSegment,
    RuntimeReference, ValidationStatus,
};
use crate::{ByteDigest, Diagnostic};
use serde_json::Value;

fn source(value: &FormalSource) -> types::Source<'_> {
    types::Source {
        identity: value.source().identity(),
        digest: value.source().digest().to_string(),
        path: value.source().path(),
        formal: value.identity(),
    }
}

fn reference(value: &RuntimeReference) -> types::Reference<'_> {
    match value {
        RuntimeReference::Snapshot(value) => types::Reference::Snapshot(value),
        RuntimeReference::Invocation(value) => types::Reference::Invocation(value),
    }
}

fn runtime_path(value: &RuntimePathSegment) -> types::RuntimePath<'_> {
    match value {
        RuntimePathSegment::Model(value) => types::RuntimePath::Model(value),
        RuntimePathSegment::Population { record, universe } => {
            types::RuntimePath::Population { record, universe }
        }
        RuntimePathSegment::Object(value) => types::RuntimePath::Object(value),
        RuntimePathSegment::State(value) => types::RuntimePath::State(value),
        RuntimePathSegment::Parameter(value) => types::RuntimePath::Parameter(value),
        RuntimePathSegment::Result => types::RuntimePath::Result(true),
        RuntimePathSegment::Field(value) => types::RuntimePath::Field(value),
        RuntimePathSegment::Index(value) => types::RuntimePath::Index(*value),
    }
}

fn diagnostic(value: &Diagnostic) -> types::Diagnostic<'_> {
    types::Diagnostic {
        phase: value.phase.as_str(),
        code: value.code.as_str(),
        message: &value.message,
        source: &value.source,
        path: &value.path,
        span: value.span,
        upstream: value.upstream.as_deref(),
        runtime: value
            .runtime
            .as_ref()
            .map(|location| types::RuntimeLocation {
                artifact: reference(&location.artifact),
                observation: location.observation,
                requirement: &location.requirement,
                clause: &location.clause,
                path: location.path.iter().map(runtime_path).collect(),
            }),
    }
}

pub(super) fn error(error: &RunError) -> Result<Value, serde_json::Error> {
    let (stage, details) = match &error.cause {
        RunCause::Io { path, error } => (
            "file",
            types::Details::Io {
                path: path.to_string_lossy(),
                os_code: error.raw_os_error(),
            },
        ),
        RunCause::Json(error) => (
            "request",
            types::Details::Json {
                line: error.line(),
                column: error.column(),
            },
        ),
        RunCause::Output(_) => ("output", types::Details::None),
        RunCause::Format => ("envelope", types::Details::None),
        RunCause::Limit(kind) => (
            "intake",
            types::Details::Limit {
                limit: kind.as_str(),
            },
        ),
        RunCause::Digest(_) => ("request", types::Details::None),
        RunCause::Identifier(error) => ("request", types::Details::Identifier(error)),
        RunCause::Native(error) => (
            error.phase.as_str(),
            types::Details::Native(diagnostic(error)),
        ),
        RunCause::Model(error) => (
            "model",
            types::Details::Model {
                source: source(error.source()),
            },
        ),
        RunCause::Package(error) => (
            "package",
            types::Details::Package {
                stage: error.stage.to_string(),
            },
        ),
        RunCause::Input(error) => (
            "input",
            types::Details::Input {
                expected: reference(&error.expected),
                stage: error.stage.as_str(),
            },
        ),
    };
    serde_json::to_value(types::Failure {
        format: types::Format::NativeResult,
        request_digest: error.request_digest.map(|value| value.to_string()),
        status: if error.cause.is_incomplete() {
            types::FailureStatus::Incomplete
        } else {
            types::FailureStatus::Refused
        },
        stage,
        code: error.cause.code().as_str(),
        message: error.to_string(),
        details,
    })
}

pub(super) fn report(
    digest: ByteDigest,
    selection: &wire::Selection,
    models: &[NativeModel],
    report: &ExecutionReport<'_, '_>,
) -> super::Result<RunResult> {
    let (exit_code, outcome) = match report.outcome() {
        ExecutionOutcome::ValidationFailed(failure) => {
            let (code, status) = match failure.status {
                ValidationStatus::Refused => (1, types::FailureStatus::Refused),
                ValidationStatus::Incomplete => (3, types::FailureStatus::Incomplete),
            };
            (
                code,
                types::Outcome::Validate {
                    status,
                    diagnostics: failure.diagnostics.iter().map(diagnostic).collect(),
                    terminal: failure.terminal.as_deref().map(diagnostic),
                },
            )
        }
        ExecutionOutcome::Evaluated {
            result,
            usage,
            events,
            cost_model,
        } => {
            let (code, result) = match result {
                EvaluationOutcome::Completed(truth) => (
                    u8::from(!truth),
                    types::Evaluation::Completed { truth: *truth },
                ),
                EvaluationOutcome::Refused(error) => (
                    1,
                    types::Evaluation::Refused {
                        diagnostic: diagnostic(error),
                    },
                ),
                EvaluationOutcome::Incomplete(error) => (
                    3,
                    types::Evaluation::Incomplete {
                        diagnostic: diagnostic(error),
                    },
                ),
            };
            let events = events
                .iter()
                .map(|event| types::Event {
                    implication: event.implication.0,
                    operand: event.operand.0,
                    span: event.span,
                    kind: match event.kind {
                        ImplicationEventKind::AntecedentEntered => {
                            types::EventKind::AntecedentEntered
                        }
                        ImplicationEventKind::AntecedentCompleted(truth) => {
                            types::EventKind::AntecedentCompleted { truth }
                        }
                        ImplicationEventKind::ConsequentEntered => {
                            types::EventKind::ConsequentEntered
                        }
                    },
                })
                .collect();
            (
                code,
                types::Outcome::Evaluate {
                    result,
                    evaluation_usage: usage,
                    events,
                    cost_model,
                },
            )
        }
    };
    let document = types::Report {
        format: types::Format::NativeResult,
        request_digest: digest.to_string(),
        package: types::Package {
            digest: report.package().digest().to_string(),
            canonical_identity: report.package().canonical_identity().to_string(),
        },
        source: source(&report.package().checked().bindings().source),
        models: models
            .iter()
            .map(|model| types::Model {
                owner: model.environment().owner(),
                digest: model.digest().to_string(),
                source: source(model.source()),
            })
            .collect(),
        inputs: types::Inputs {
            snapshots: report
                .input()
                .snapshots
                .iter()
                .map(|value| value.reference())
                .collect(),
            invocations: report
                .input()
                .invocations
                .iter()
                .map(|value| value.reference())
                .collect(),
        },
        selection,
        validation_usage: report.validation_usage(),
        outcome,
    };
    Ok(RunResult {
        exit_code,
        value: serde_json::to_value(document).map_err(RunCause::Output)?,
    })
}
