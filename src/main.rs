// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-010/026: parse one command, execute it, and write one explicit outcome.
mod cli;

use cli::{Command, SyntaxCommand};
use qsl_cst::CompleteDiagnostic;
use qsl_foundation::diagnostic::Category;
use qsl_foundation::source::{render_offered, IdentityCauseFields, SourceReadCause};
use qsl_foundation::{Code, Diagnostic, LocatedSpan, Phase, SourceIdentity};
use quire_spec_language::{format::format, parse, Limits};
use serde_json::json;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::ExitCode;

/// One refusal line: the fields every syntax-command diagnostic shares.
struct Refusal<'a> {
    code: Code,
    phase: Phase,
    source: &'a SourceIdentity,
    path: &'a str,
    /// The rendered region; `None` for a refusal FR-001 locates nowhere.
    span: Option<LocatedSpan>,
    /// FR-001: the `invalid_source_identity` cause, when this is one.
    identity_cause: Option<SourceReadCause>,
    message: &'a str,
}

/// The one refusal line, typed so a field cannot be missed on one branch.
#[derive(serde::Serialize)]
struct Line<'a> {
    status: &'static str,
    phase: &'static str,
    code: &'static str,
    source: &'a SourceIdentity,
    path: &'a str,
    span: Option<serde_json::Value>,
    message: &'a str,
    /// FR-001: `cause` and `label`, on `invalid_source_identity` only.
    #[serde(flatten)]
    identity: Option<IdentityCauseFields>,
}

impl Refusal<'_> {
    fn render(&self) -> (u8, String) {
        let span = self.span.map(|span| {
            json!({
                "start": {"byte":span.start.byte,"line":span.start.line,"column":span.start.column},
                "end": {"byte":span.end.byte,"line":span.end.line,"column":span.end.column}})
        });
        let output = serde_json::to_string(&Line {
            status: if self.code.is_incomplete() {
                "incomplete"
            } else {
                "refused"
            },
            phase: self.phase.as_str(),
            code: self.code.as_str(),
            source: self.source,
            path: self.path,
            span,
            message: self.message,
            identity: self
                .identity_cause
                .and_then(SourceReadCause::identity_fields),
        })
        .unwrap_or_else(|error| format!("{{\"message\":\"unserializable refusal: {error}\"}}"));
        // FR-285: the code's category decides the exit code.
        (self.code.category().exit_code(), output)
    }
}

fn diagnostic(value: &Diagnostic) -> (u8, String) {
    Refusal {
        code: value.code,
        phase: value.phase,
        source: &value.source,
        path: &value.path,
        span: Some(value.span),
        identity_cause: value.identity_cause(),
        message: &value.message,
    }
    .render()
}

/// A complete-V1 diagnostic, its region rendered over the file's `bytes`
/// (FR-001: line and column are derived when rendered).
fn complete_diagnostic(value: &CompleteDiagnostic, bytes: &[u8]) -> (u8, String) {
    Refusal {
        code: value.code,
        phase: value.phase,
        source: &value.source,
        path: &value.path,
        span: value
            .region
            .as_ref()
            .and_then(|region| render_offered(bytes, region)),
        identity_cause: value.cause.identity_cause(),
        message: &value.message,
    }
    .render()
}

/// Read at most the source ceiling plus one byte, so an oversized file
/// reaches the parser's own size refusal without an unbounded read.
fn read_bounded(path: &Path, source_bytes: usize) -> Result<Vec<u8>, (u8, String)> {
    let display_path = path.to_string_lossy();
    let file = std::fs::File::open(path).map_err(|error| {
        (
            Category::Refusal.exit_code(),
            format!("cannot open {display_path}: {error}"),
        )
    })?;
    let mut bytes = Vec::new();
    // Unreachable on any 64-bit target: usize -> u64 cannot overflow. A
    // platform where it did would be a build/platform defect, not invalid
    // input, so an internal failure is the truer classification.
    let ceiling = u64::try_from(source_bytes).map_err(|error| {
        (
            Category::InternalFailure.exit_code(),
            format!("invalid source ceiling: {error}"),
        )
    })?;
    file.take(ceiling.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| {
            (
                Category::Refusal.exit_code(),
                format!("cannot read {display_path}: {error}"),
            )
        })?;
    Ok(bytes)
}

fn syntax(
    command: SyntaxCommand,
    source_identity: SourceIdentity,
    path: &Path,
    parse_limits: Limits,
) -> Result<String, (u8, String)> {
    let display_path = path.to_string_lossy();
    // FR-010: a blank label refuses as `invalid_source_identity` before the
    // file is opened, so it is never reported as a file error.
    if let Some(label) = source_identity.first_blank_label() {
        return Err(Refusal {
            code: Code::InvalidSourceIdentity,
            phase: Phase::Source,
            source: &source_identity,
            path: display_path.as_ref(),
            span: None,
            identity_cause: Some(SourceReadCause::BlankLabel { label }),
            message: "source authority, identity, revision namespace and revision must be explicit",
        }
        .render());
    }
    match command {
        // FR-003: `format` reads the S1 lossless CST (ADR-011 §6.2, §7.3 M-6a).
        SyntaxCommand::Format => {
            let limits = qsl_cst::Limits::default();
            let bytes = read_bounded(path, limits.source_bytes)?;
            let parsed = qsl_cst::parse(source_identity, display_path.as_ref(), &bytes, limits)
                .map_err(|error| complete_diagnostic(&error, &bytes))?;
            format(&parsed).map_err(|refusal| complete_diagnostic(refusal.diagnostic(), &bytes))
        }
        SyntaxCommand::Parse => {
            let limits = parse_limits;
            let bytes = read_bounded(path, limits.source_bytes)?;
            let unit = parse(source_identity, display_path.as_ref(), &bytes, limits)
                .map_err(|error| diagnostic(&error))?;
            // FR-010: the source is reported as its `RawSourceRef`.
            Ok(
                json!({"status":"parsed", "source": unit.source().reference(),
                "path":display_path, "imports":unit.imports().len(), "clauses":unit.clauses().len() })
                .to_string(),
            )
        }
    }
}

enum Output {
    Line(String),
    Artifact(Vec<u8>),
}

fn command_error(error: &quire_spec_language::command::RunError) -> (u8, String) {
    match error.value() {
        Ok(value) => (error.category().exit_code(), value.to_string()),
        // Serializing the outcome is an internal failure, distinct from the
        // request-level disposition it failed to encode.
        Err(output) => (
            Category::InternalFailure.exit_code(),
            format!("output failed: {output}"),
        ),
    }
}

fn execute(command: Command<'_>) -> Result<(u8, Output), (u8, String)> {
    match command {
        Command::Syntax {
            kind,
            source,
            path,
            limits,
        } => syntax(kind, source, path, limits)
            .map(|text| (Category::Success.exit_code(), Output::Line(text))),
        Command::Run { path } => quire_spec_language::command::run(path)
            .map(|result| (result.exit_code, Output::Line(result.value.to_string())))
            .map_err(|error| command_error(&error)),
        Command::Compile { path } => quire_spec_language::command::compile(path)
            .map(|bytes| (Category::Success.exit_code(), Output::Artifact(bytes)))
            .map_err(|error| command_error(&error)),
        Command::Lower { path, target } => quire_spec_language::command::lower_for(path, target)
            .map(|bytes| (Category::Success.exit_code(), Output::Artifact(bytes)))
            .map_err(|error| command_error(&error)),
    }
}

fn main() -> ExitCode {
    // The longest form is `parse` with five operands and three limit options
    // (twelve including the command); retain one extra to reject excess
    // arguments without collecting an unbounded process argument list.
    let arguments: Vec<_> = std::env::args_os().skip(1).take(13).collect();
    let outcome = Command::try_from(arguments.as_slice())
        .map_err(|error| {
            // An unrecognized lowering target names a real, catalogued
            // capability this build does not implement (unsupported);
            // every other usage failure is invalid input (refusal).
            let category = if matches!(error, cli::UsageError::UnknownTarget(_)) {
                Category::Unsupported
            } else {
                Category::Refusal
            };
            (category.exit_code(), error.to_string())
        })
        .and_then(execute);
    let (code, result) = match outcome {
        Ok((code, output)) => {
            let mut stdout = io::stdout().lock();
            let written = match output {
                Output::Line(text) => writeln!(stdout, "{}", text.trim_end_matches('\n')),
                Output::Artifact(bytes) => stdout.write_all(&bytes),
            };
            (code, written)
        }
        Err((code, output)) => (code, writeln!(io::stderr().lock(), "{output}")),
    };
    match result {
        Ok(()) => ExitCode::from(code),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::from(code),
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "output failed: {error}");
            // A stdout/stderr write failure is an internal failure, not a
            // request-level disposition.
            ExitCode::from(Category::InternalFailure.exit_code())
        }
    }
}
