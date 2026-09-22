// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-026: construct typed JSON views of native outcomes and actual provenance.

#[cfg(feature = "quire-extraction")]
mod extraction;
mod types;

use super::{wire, LimitKind, RunCause, RunError, RunResult};
use crate::formal_source::FormalSource;
use crate::native_model::NativeModel;
use crate::runtime::{
    EvaluationOutcome, ExecutionOutcome, ExecutionReport, ImplicationEventKind, RuntimePathSegment,
    RuntimeReference, ValidationDiagnostic, ValidationStatus,
};
use crate::{ByteDigest, Diagnostic};
use serde_json::Value;

/// Immutable JSON produced from the typed native result schema.
/// Construction stays inside the output adapter; callers can inspect or serialize it.
#[derive(Debug, serde::Serialize)]
#[serde(transparent)]
pub struct NativeResult(Value);

impl NativeResult {
    /// Inspect the encoded observations without changing the admitted shape.
    pub fn as_value(&self) -> &Value {
        &self.0
    }
}

impl std::fmt::Display for NativeResult {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

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
        runtime: None,
    }
}

fn validation_diagnostic(value: &ValidationDiagnostic) -> types::Diagnostic<'_> {
    types::Diagnostic {
        runtime: Some(types::RuntimeLocation {
            artifact: reference(&value.runtime.artifact),
            observation: value.runtime.observation,
            requirement: &value.runtime.requirement,
            clause: &value.runtime.clause,
            path: value.runtime.path.iter().map(runtime_path).collect(),
        }),
        ..diagnostic(&value.diagnostic)
    }
}

fn package_path(value: &crate::package::PackagePathSegment) -> types::PackagePath<'_> {
    match value {
        crate::package::PackagePathSegment::Field(name) => types::PackagePath::Field(name),
        crate::package::PackagePathSegment::Index(index) => types::PackagePath::Index(*index),
    }
}

fn package_cause(value: &crate::package::PackageCause) -> types::PackageCause<'_> {
    match value {
        crate::package::PackageCause::Native(value) => {
            types::PackageCause::Native(diagnostic(value))
        }
        crate::package::PackageCause::Json(value) => types::PackageCause::Json {
            line: value.line(),
            column: value.column(),
            message: value.to_string(),
        },
    }
}

fn projection_location(value: super::ProjectionLocation) -> types::ProjectionLocation {
    let (span_status, span, unmapped_span) = match value {
        super::ProjectionLocation::Absent => (types::SpanStatus::Absent, None, None),
        super::ProjectionLocation::Located(span) => (types::SpanStatus::Located, Some(span), None),
        super::ProjectionLocation::Invalid(span) => (types::SpanStatus::Invalid, None, Some(span)),
    };
    types::ProjectionLocation {
        span_status,
        span,
        unmapped_span,
    }
}

pub(super) fn error(error: &RunError) -> Result<Value, serde_json::Error> {
    let (stage, details) = match &error.cause {
        #[cfg(feature = "quire-extraction")]
        RunCause::Extraction(error) => {
            let (stage, details) = extraction::failure(error);
            (stage, types::Details::Extraction(details))
        }
        RunCause::Io { path, error } => (
            types::Stage::File,
            types::Details::Io {
                path: path.to_string_lossy(),
                os_code: error.raw_os_error(),
            },
        ),
        RunCause::Json(error) => (
            types::Stage::Request,
            types::Details::Json {
                line: error.line(),
                column: error.column(),
            },
        ),
        RunCause::Output(_) => (types::Stage::Output, types::Details::None),
        RunCause::Format => (types::Stage::Envelope, types::Details::None),
        RunCause::Limit(kind) => (
            types::Stage::Intake,
            match kind {
                LimitKind::FileBytes => types::Details::FileBytes {
                    limit: kind.as_str(),
                },
                LimitKind::SelectedFiles {
                    category,
                    requested,
                    remaining,
                    maximum,
                } => types::Details::FileCount {
                    limit: kind.as_str(),
                    category: category.as_str(),
                    requested: *requested,
                    remaining: *remaining,
                    maximum: *maximum,
                },
            },
        ),
        RunCause::Digest(_) => (types::Stage::Request, types::Details::None),
        RunCause::Identifier(error) => (types::Stage::Request, types::Details::Identifier(error)),
        RunCause::Native(error) => (
            types::Stage::Native(error.phase),
            types::Details::Native(diagnostic(error)),
        ),
        RunCause::Model(error) => (
            types::Stage::Model,
            types::Details::Model {
                source: source(error.source()),
            },
        ),
        RunCause::Package(error) => (
            types::Stage::Package,
            types::Details::Package {
                stage: error.stage.to_string(),
            },
        ),
        RunCause::Input(error) => (
            types::Stage::Input,
            types::Details::Input {
                expected: reference(&error.expected),
                stage: error.stage.as_str(),
            },
        ),
        RunCause::SelectedPackage {
            file,
            expected,
            error,
        } => (
            types::Stage::SelectedPackage,
            types::Details::SelectedPackage {
                file,
                expected: types::PackageReference {
                    format: expected.format(),
                    digest: expected.digest().to_string(),
                },
                stage: error.stage.to_string(),
                path: error.path.iter().map(package_path).collect(),
                cause: error.cause.as_ref().map(package_cause),
            },
        ),
        RunCause::Lowering {
            target,
            package,
            program,
            location,
            error,
        } => (
            types::Stage::Lower,
            types::Details::Lowering {
                profile: target.name(),
                package: types::PackageReference {
                    format: package.format(),
                    digest: package.digest().to_string(),
                },
                source: types::Source {
                    identity: &program.identity,
                    digest: program.digest.to_string(),
                    path: &program.path,
                    formal: &program.formal,
                },
                clause: error.clause.as_ref(),
                location: projection_location(*location),
                upstream: &error.upstream,
            },
        ),
    };
    serde_json::to_value(types::Failure {
        format: types::Format::RunResult,
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
    _package: &super::compilation::RunPackage<'_>,
    report: &ExecutionReport<'_, '_>,
) -> super::Result<RunResult> {
    let (exit_code, outcome) = match report.outcome() {
        ExecutionOutcome::ValidationFailed(failure) => {
            // FR-301's exit ladder is the highest-severity code present
            // across every retained diagnostic (including the terminal one,
            // when present): invalid (20) outranks unsupported (21)
            // outranks incomplete (22). `Code::exit_code()`'s range is
            // exactly {20, 21, 22} (asserted over `Code::all()` in
            // tests/native_boundaries.rs), and FR-301's ordering over that
            // three-code range happens to coincide with ascending numeric
            // order, so the minimum over `Diagnostic::exit_code` is exactly
            // the highest-severity code present; a single unsupported
            // diagnostic never promotes a report that also holds an invalid
            // one. This is not general — FR-301's full order (tool failure,
            // invalid, unsupported, incomplete, violation, success) is not
            // ascending-numeric across 0/10/20/21/22/30, only within the
            // three codes a diagnostic can actually carry here. ValidationStatus
            // only carries the wire-schema's binary refused/incomplete
            // distinction and does not drive the exit code.
            let status = match failure.status {
                ValidationStatus::Refused => types::FailureStatus::Refused,
                ValidationStatus::Incomplete => types::FailureStatus::Incomplete,
            };
            let code = failure
                .diagnostics
                .iter()
                .chain(failure.terminal.as_deref())
                .map(|diagnostic| diagnostic.exit_code())
                .min()
                .unwrap_or(match failure.status {
                    ValidationStatus::Incomplete => 22,
                    ValidationStatus::Refused => 20,
                });
            (
                code,
                types::Outcome::Validate {
                    status,
                    diagnostics: failure
                        .diagnostics
                        .iter()
                        .map(validation_diagnostic)
                        .collect(),
                    terminal: failure.terminal.as_deref().map(validation_diagnostic),
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
                    if *truth { 0 } else { 10 },
                    types::Evaluation::Completed { truth: *truth },
                ),
                EvaluationOutcome::Refused(error) => (
                    error.exit_code(),
                    types::Evaluation::Refused {
                        diagnostic: diagnostic(error),
                    },
                ),
                EvaluationOutcome::Incomplete(error) => (
                    22,
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
        #[cfg(feature = "quire-extraction")]
        extraction: extraction::context(_package),
        format: types::Format::RunResult,
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
        value: NativeResult(serde_json::to_value(document).map_err(RunCause::Output)?),
    })
}
