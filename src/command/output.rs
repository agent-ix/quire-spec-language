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
use qsl_foundation::{ByteDigest, Diagnostic};
use quire_contract_ir as ir;
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
        upstream: None,
        runtime: None,
    }
}

/// A native diagnostic retained by a higher-layer wrapper (`LinkingError`,
/// `CheckingError`), carrying its own structured upstream refusal separate
/// from the shared [`Diagnostic`] shape (ADR-011 §6.1).
fn diagnostic_with_upstream<'a>(
    value: &'a Diagnostic,
    upstream: Option<&'a ir::Diagnostic>,
) -> types::Diagnostic<'a> {
    types::Diagnostic {
        upstream,
        ..diagnostic(value)
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
        crate::package::PackageCause::Linking(value) => types::PackageCause::Native(
            diagnostic_with_upstream(&value.diagnostic, value.upstream.as_deref()),
        ),
        crate::package::PackageCause::Checking(value) => types::PackageCause::Native(
            diagnostic_with_upstream(&value.diagnostic, value.upstream.as_deref()),
        ),
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
        RunCause::Linking(error) => (
            types::Stage::Native(error.diagnostic.phase),
            types::Details::Native(diagnostic_with_upstream(
                &error.diagnostic,
                error.upstream.as_deref(),
            )),
        ),
        RunCause::Checking(error) => (
            types::Stage::Native(error.diagnostic.phase),
            types::Details::Native(diagnostic_with_upstream(
                &error.diagnostic,
                error.upstream.as_deref(),
            )),
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
        RunCause::CompleteSelection(_)
        | RunCause::Libraries(_)
        | RunCause::CompleteRunSelection(_)
        | RunCause::NativeRunSelection(_)
        | RunCause::MissingClauses => (types::Stage::Request, types::Details::None),
        RunCause::Spine(failure) => (
            types::Stage::Spine(failure.refusal.stage()),
            types::Details::Spine {
                source: &failure.source,
                path: &failure.path,
                span: failure.span,
            },
        ),
        RunCause::SpineRun(refusal) => (
            types::Stage::SpineRun(refusal.stage()),
            match refusal.as_ref() {
                qsl_replay::spine::RunRefusal::MissingDeclaration { function }
                | qsl_replay::spine::RunRefusal::UnsupportedResult { function } => {
                    types::Details::Function { function }
                }
                qsl_replay::spine::RunRefusal::UnknownParameter { parameter }
                | qsl_replay::spine::RunRefusal::DuplicateArgument { parameter }
                | qsl_replay::spine::RunRefusal::UnboundParameter { parameter } => {
                    types::Details::Parameter { parameter }
                }
                qsl_replay::spine::RunRefusal::WrongValueKind { position } => {
                    types::Details::Position {
                        position: *position,
                    }
                }
                qsl_replay::spine::RunRefusal::Compile(_) => types::Details::None,
                qsl_replay::spine::RunRefusal::Fault(fault) => types::Details::Invariant {
                    stage: fault.stage(),
                    invariant: fault.invariant(),
                },
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

/// FR-096/FR-100: `Evaluation.location`'s `origin`, rendered by kind.
fn spine_origin(origin: &qsl_semantics::check::Origin) -> types::SpineOrigin {
    match origin {
        qsl_semantics::check::Origin::Body { function, index } => types::SpineOrigin::Body {
            function: function.clone(),
            index: *index,
        },
        qsl_semantics::check::Origin::Measure { function, index } => types::SpineOrigin::Measure {
            function: function.clone(),
            index: *index,
        },
        qsl_semantics::check::Origin::Expression => types::SpineOrigin::Expression,
        qsl_semantics::check::Origin::TypeDeclaration { name } => {
            types::SpineOrigin::TypeDeclaration { name: name.clone() }
        }
    }
}

/// FR-096/FR-100: `Evaluation.location`, rendered as `{origin, path}`.
fn spine_location(location: &qsl_semantics::check::Location) -> types::SpineLocation {
    types::SpineLocation {
        origin: spine_origin(&location.origin),
        path: location.path.clone(),
    }
}

/// FR-096/FR-100: a record's resolved locus, rendered as
/// `{source_digest, span}`.
fn spine_locus(locus: qsl_replay::spine::CallLocus) -> types::SpineLocus {
    types::SpineLocus {
        source_digest: locus.source_digest,
        span: locus.span,
    }
}

/// A refusal's exit status: the record's or cause's catalog code exit
/// status, or 20 when the code names no `Code` (FR-100). Never used for a
/// kernel-no-record row, which is always exit 20 by the mapping table.
fn refusal_exit_code(code: qsl_foundation::diagnostic::CatalogCode) -> u8 {
    qsl_foundation::Code::from_code(code.code()).map_or(20, qsl_foundation::Code::exit_code)
}

/// FR-100: render `spine-run-result/1`, the outcome mapping's stdout
/// document and FR-301 exit status, from `qsl_replay::spine::run`'s result.
pub(super) fn spine_run_result(
    digest: ByteDigest,
    package_id: qsl_semantics::library::PackageId,
    source: &FormalSource,
    function: &str,
    outcome: qsl_replay::spine::CallOutcome,
) -> super::Result<RunResult> {
    use qsl_replay::spine::{CallOutcome, CallRefusal, CallValue};
    let (exit_code, outcome) = match outcome {
        CallOutcome::Completed(CallValue::Boolean(value)) => (
            0,
            types::SpineOutcome::Completed {
                value: types::SpineValue::Boolean { value },
            },
        ),
        CallOutcome::Completed(CallValue::Integer(value)) => (
            0,
            types::SpineOutcome::Completed {
                value: types::SpineValue::Integer {
                    decimal: value.to_string(),
                },
            },
        ),
        CallOutcome::Refused(CallRefusal::Record {
            code,
            fields,
            locus,
            location,
        }) => (
            refusal_exit_code(code),
            types::SpineOutcome::Refused {
                code: Some(code.code()),
                cause: Some(code.cause()),
                fields: Some(fields),
                locus: locus.map(spine_locus),
                location: location.as_ref().map(spine_location),
            },
        ),
        CallOutcome::Refused(CallRefusal::Family { code, location }) => (
            refusal_exit_code(code),
            types::SpineOutcome::Refused {
                code: Some(code.code()),
                cause: Some(code.cause()),
                fields: None,
                locus: None,
                location: location.as_ref().map(spine_location),
            },
        ),
        CallOutcome::Refused(CallRefusal::Kernel { location }) => (
            20,
            types::SpineOutcome::Refused {
                code: None,
                cause: None,
                fields: None,
                locus: None,
                location: location.as_ref().map(spine_location),
            },
        ),
        CallOutcome::Undefined { reason } => (20, types::SpineOutcome::Undefined { reason }),
        CallOutcome::Incomplete { limit } => (22, types::SpineOutcome::Incomplete { limit }),
    };
    let document = types::SpineRunReport {
        format: types::Format::SpineRunResult,
        request_digest: digest.to_string(),
        package_id: package_id.hex(),
        source: types::RunSource {
            identity: source.source().identity(),
            digest: source.source().digest().to_string(),
            path: source.source().path(),
        },
        function,
        outcome,
    };
    Ok(RunResult {
        exit_code,
        value: NativeResult(serde_json::to_value(document).map_err(RunCause::Output)?),
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
                .map(|diagnostic| diagnostic.diagnostic.exit_code())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formal_source::FormalSource;
    use ix_trace_rs::trace;
    use qsl_foundation::SourceIdentity;
    use qsl_replay::spine::CallOutcome;
    use qsl_semantics::library::PackageId;

    fn formal_source() -> FormalSource {
        let source = qsl_foundation::Source::read(
            SourceIdentity {
                authority: "agent-ix".into(),
                identity: "test:spine-run".into(),
                revision_namespace: "fixture".into(),
                revision: "fixture:1".into(),
            },
            "program.native",
            b"",
            1,
        )
        .unwrap();
        let identity = ir::SourceIdentity::new(
            ir::SourceDocumentId::new("SpineRun").unwrap(),
            ir::SourceRevision::new(1).unwrap(),
        );
        FormalSource::new(source, identity)
    }

    /// Assert `spine_run_result`'s whole stdout document and exit code for
    /// an `Undefined` outcome carrying `reason`, isolated from any specific
    /// kernel or family origin (FR-100-AC-9, FND-006). The renderer
    /// (`src/command/output.rs`) does not distinguish where a `reason`
    /// string came from; `qsl_replay::spine::call::convert_outcome`'s own
    /// tests cover that distinction.
    fn assert_undefined_renders(reason: &'static str) {
        let source = formal_source();
        let digest = ByteDigest::of(b"request");
        let package_id = PackageId::of_preimage(b"package");
        let result = spine_run_result(
            digest,
            package_id,
            &source,
            "seven",
            CallOutcome::Undefined { reason },
        )
        .unwrap();
        assert_eq!(result.exit_code, 20, "{reason}");
        assert_eq!(
            result.value.as_value(),
            &serde_json::json!({
                "format": "spine-run-result/1",
                "request_digest": digest.to_string(),
                "package_id": package_id.hex(),
                "source": {
                    "authority": "agent-ix",
                    "identity": "test:spine-run",
                    "revision_namespace": "fixture",
                    "revision": "fixture:1",
                    "digest": source.source().digest().to_string(),
                    "path": "program.native",
                },
                "function": "seven",
                "outcome": {"kind": "undefined", "reason": reason},
            }),
            "{reason}"
        );
    }

    /// FR-100-AC-9 (TC-452 step 4): each kernel `Undefined` reason
    /// (`qsl_replay::spine::call::kernel_undefined_reason`) renders
    /// `{"kind":"undefined","reason":...}` and exits 20 (FND-006).
    #[test]
    #[trace("TC-452", "FR-100-AC-9")]
    fn undefined_kernel_reasons_render_and_exit_20() {
        for reason in [
            "division-by-zero",
            "ieee-not-finite",
            "empty-reduction",
            "none-value",
        ] {
            assert_undefined_renders(reason);
        }
    }

    /// FR-100-AC-9 (TC-452 step 4): each family `Undefined` reason
    /// (`qsl_foundation::diagnostic::UndefinedReason::as_str`) renders
    /// `{"kind":"undefined","reason":...}` and exits 20 (FND-006).
    #[test]
    #[trace("TC-452", "FR-100-AC-9")]
    fn undefined_family_reasons_render_and_exit_20() {
        for reason in ["precondition-false", "absent-key"] {
            assert_undefined_renders(reason);
        }
    }

    fn render(outcome: CallOutcome) -> RunResult {
        spine_run_result(
            ByteDigest::of(b"request"),
            PackageId::of_preimage(b"package"),
            &formal_source(),
            "seven",
            outcome,
        )
        .unwrap()
    }

    /// FR-100-AC-9 (TC-452 step 4, FND-013): a record refusal renders every
    /// FR-096 member and exits by its catalog code's mapping -- `resource_exhausted`
    /// (`ancestor-steps`'s own code) is incomplete, so it exits 22, not the
    /// kernel-refusal row's hardcoded 20.
    #[test]
    #[trace("TC-452", "FR-100-AC-9")]
    fn refused_record_renders_and_exits_by_its_code() {
        use qsl_replay::spine::CallRefusal;
        let fields = std::collections::BTreeMap::from([
            ("from", "test/orders".to_owned()),
            ("limit", "5".to_owned()),
        ]);
        let result = render(CallOutcome::Refused(CallRefusal::Record {
            code: qsl_foundation::diagnostic::CatalogCode::new(
                "resource_exhausted",
                "ancestor-steps",
            ),
            fields: fields.clone(),
            locus: None,
            location: None,
        }));
        assert_eq!(result.exit_code, 22);
        assert_eq!(
            result.value.as_value()["outcome"],
            serde_json::json!({
                "kind": "refused",
                "code": "resource_exhausted",
                "cause": "ancestor-steps",
                "fields": fields,
            })
        );
    }

    /// FR-100-AC-9 (TC-452 step 4, FND-013): a family refusal with no FR-096
    /// record carries its code and cause but no `fields`/`locus`.
    #[test]
    #[trace("TC-452", "FR-100-AC-9")]
    fn refused_family_with_no_record_renders_code_and_cause_only() {
        use qsl_replay::spine::CallRefusal;
        let result = render(CallOutcome::Refused(CallRefusal::Family {
            code: qsl_foundation::diagnostic::CatalogCode::new("ill_typed", "type-mismatch"),
            location: None,
        }));
        assert_eq!(result.exit_code, 20);
        assert_eq!(
            result.value.as_value()["outcome"],
            serde_json::json!({"kind": "refused", "code": "ill_typed", "cause": "type-mismatch"})
        );
    }

    /// FR-100-AC-9 (TC-452 step 4, FND-013): a bare kernel refusal with no
    /// code, cause, fields or locus exits 20.
    #[test]
    #[trace("TC-452", "FR-100-AC-9")]
    fn refused_kernel_with_no_record_exits_20() {
        use qsl_replay::spine::CallRefusal;
        let result = render(CallOutcome::Refused(CallRefusal::Kernel { location: None }));
        assert_eq!(result.exit_code, 20);
        assert_eq!(
            result.value.as_value()["outcome"],
            serde_json::json!({"kind": "refused"})
        );
    }

    /// FR-100-AC-9 (TC-452 step 4, FND-013): every `location.origin` kind
    /// renders its kebab-case tag, including `type-declaration`.
    #[test]
    #[trace("TC-452", "FR-100-AC-9")]
    fn location_origin_kinds_render_kebab_case() {
        use qsl_replay::spine::CallRefusal;
        use qsl_semantics::check::{Location, Origin};
        let cases = [
            (
                Origin::Body {
                    function: "seven".to_owned(),
                    index: 0,
                },
                serde_json::json!({"kind": "body", "function": "seven", "index": 0}),
            ),
            (
                Origin::Measure {
                    function: "seven".to_owned(),
                    index: 1,
                },
                serde_json::json!({"kind": "measure", "function": "seven", "index": 1}),
            ),
            (
                Origin::Expression,
                serde_json::json!({"kind": "expression"}),
            ),
            (
                Origin::TypeDeclaration {
                    name: "Point".to_owned(),
                },
                serde_json::json!({"kind": "type-declaration", "name": "Point"}),
            ),
        ];
        for (origin, expected) in cases {
            let result = render(CallOutcome::Refused(CallRefusal::Kernel {
                location: Some(Location {
                    origin,
                    path: vec![2, 0],
                }),
            }));
            assert_eq!(
                result.value.as_value()["outcome"]["location"],
                serde_json::json!({"origin": expected, "path": [2, 0]}),
                "{expected}"
            );
        }
    }

    /// FR-100-AC-9 (TC-452 step 4, FND-019): a record refusal's `locus`
    /// renders `{source_digest, span}` exactly -- the TC-452 fixture-F
    /// values -- and `location` renders on both a record refusal and a
    /// family refusal.
    #[test]
    #[trace("TC-452", "FR-100-AC-9")]
    fn refused_locus_and_location_render() {
        use qsl_replay::spine::{CallLocus, CallRefusal};
        use qsl_semantics::check::{Location, Origin};
        let span = qsl_foundation::LocatedSpan {
            start: qsl_foundation::Position {
                byte: 225,
                line: 3,
                column: 54,
            },
            end: qsl_foundation::Position {
                byte: 226,
                line: 3,
                column: 55,
            },
        };
        let locus = CallLocus {
            source_digest:
                "sha256:5f2742391e3eaef04bc5dd7141fd639b1913dc821d14bb2f2ca618ad8598ca26".to_owned(),
            span,
        };
        let location = Location {
            origin: Origin::Body {
                function: "f".to_owned(),
                index: 0,
            },
            path: vec![1],
        };
        let fields = std::collections::BTreeMap::from([("binding", "people".to_owned())]);
        let expected_location = serde_json::json!({"origin": {"kind": "body", "function": "f", "index": 0}, "path": [1]});

        let record = render(CallOutcome::Refused(CallRefusal::Record {
            code: qsl_foundation::diagnostic::CatalogCode::new(
                "invalid_runtime_input",
                "absent-key",
            ),
            fields: fields.clone(),
            locus: Some(locus),
            location: Some(location.clone()),
        }));
        assert_eq!(
            record.value.as_value()["outcome"],
            serde_json::json!({
                "kind": "refused",
                "code": "invalid_runtime_input",
                "cause": "absent-key",
                "fields": fields,
                "locus": {
                    "source_digest": "sha256:5f2742391e3eaef04bc5dd7141fd639b1913dc821d14bb2f2ca618ad8598ca26",
                    "span": {
                        "start": {"byte": 225, "line": 3, "column": 54},
                        "end": {"byte": 226, "line": 3, "column": 55},
                    },
                },
                "location": expected_location,
            })
        );

        let family = render(CallOutcome::Refused(CallRefusal::Family {
            code: qsl_foundation::diagnostic::CatalogCode::new("ill_typed", "type-mismatch"),
            location: Some(location),
        }));
        assert_eq!(
            family.value.as_value()["outcome"],
            serde_json::json!({
                "kind": "refused",
                "code": "ill_typed",
                "cause": "type-mismatch",
                "location": expected_location,
            })
        );
    }
}
