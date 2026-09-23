// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-010/026: parse one command, execute it, and write one explicit outcome.
mod cli;

use cli::{Command, SyntaxCommand};
use qsl_cst::CompleteDiagnostic;
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
    span: &'a LocatedSpan,
    message: &'a str,
}

impl Refusal<'_> {
    fn render(&self) -> (u8, String) {
        let span = self.span;
        let output =
            json!({ "status": if self.code.is_incomplete() { "incomplete" } else { "refused" },
            "phase": self.phase.as_str(), "code": self.code.as_str(),
            "source": {"identity": self.source.identity, "revision": self.source.revision},
            "path": self.path, "span": {
                "start": {"byte":span.start.byte,"line":span.start.line,"column":span.start.column},
                "end": {"byte":span.end.byte,"line":span.end.line,"column":span.end.column}},
            "message": self.message })
            .to_string();
        // FR-301's contract, via Code::exit_code(): a recognized construct this
        // profile does not admit is unsupported (21); other incomplete work is
        // 22; a refused syntax request is otherwise invalid input (20).
        (self.code.exit_code(), output)
    }
}

fn diagnostic(value: &Diagnostic) -> (u8, String) {
    Refusal {
        code: value.code,
        phase: value.phase,
        source: &value.source,
        path: &value.path,
        span: &value.span,
        message: &value.message,
    }
    .render()
}

fn complete_diagnostic(value: &CompleteDiagnostic) -> (u8, String) {
    Refusal {
        code: value.code,
        phase: value.phase,
        source: &value.source,
        path: &value.path,
        span: &value.span,
        message: &value.message,
    }
    .render()
}

/// Read at most the source ceiling plus one byte, so an oversized file
/// reaches the parser's own size refusal without an unbounded read.
fn read_bounded(path: &Path, source_bytes: usize) -> Result<Vec<u8>, (u8, String)> {
    let display_path = path.to_string_lossy();
    let file = std::fs::File::open(path)
        .map_err(|error| (20, format!("cannot open {display_path}: {error}")))?;
    let mut bytes = Vec::new();
    // Unreachable on any 64-bit target: usize -> u64 cannot overflow. A
    // platform where it did would be a build/platform defect, not invalid
    // input, so FR-301's tool-failure code (30) is the truer classification.
    let ceiling = u64::try_from(source_bytes)
        .map_err(|error| (30, format!("invalid source ceiling: {error}")))?;
    file.take(ceiling.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| (20, format!("cannot read {display_path}: {error}")))?;
    Ok(bytes)
}

fn syntax(
    command: SyntaxCommand,
    identity: &str,
    revision: &str,
    path: &Path,
) -> Result<String, (u8, String)> {
    let source_identity = SourceIdentity {
        identity: identity.into(),
        revision: revision.into(),
    };
    let display_path = path.to_string_lossy();
    match command {
        // FR-003: `format` reads the S1 lossless CST (ADR-011 §6.2, §7.3 M-6a).
        SyntaxCommand::Format => {
            let limits = qsl_cst::Limits::default();
            let bytes = read_bounded(path, limits.source_bytes)?;
            let parsed = qsl_cst::parse(source_identity, display_path.as_ref(), &bytes, limits)
                .map_err(|error| complete_diagnostic(&error))?;
            format(&parsed).map_err(|refusal| complete_diagnostic(refusal.diagnostic()))
        }
        SyntaxCommand::Parse => {
            let limits = Limits::default();
            let bytes = read_bounded(path, limits.source_bytes)?;
            let unit = parse(source_identity, display_path.as_ref(), &bytes, limits)
                .map_err(|error| diagnostic(&error))?;
            Ok(
                json!({"status":"parsed", "source":{"identity":identity,"revision":revision,"digest":unit.source().digest().to_string()},
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
        Ok(value) => (error.exit_code(), value.to_string()),
        // FR-301's contract: serializing the outcome is a tool failure (30),
        // distinct from the request-level disposition it failed to encode.
        Err(output) => (30, format!("output failed: {output}")),
    }
}

fn execute(command: Command<'_>) -> Result<(u8, Output), (u8, String)> {
    match command {
        Command::Syntax {
            kind,
            identity,
            revision,
            path,
        } => syntax(kind, identity, revision, path).map(|text| (0, Output::Line(text))),
        Command::Run { path } => quire_spec_language::command::run(path)
            .map(|result| (result.exit_code, Output::Line(result.value.to_string())))
            .map_err(|error| command_error(&error)),
        Command::Compile { path } => quire_spec_language::command::compile(path)
            .map(|bytes| (0, Output::Artifact(bytes)))
            .map_err(|error| command_error(&error)),
        Command::Lower { path, target } => quire_spec_language::command::lower_for(path, target)
            .map(|bytes| (0, Output::Artifact(bytes)))
            .map_err(|error| command_error(&error)),
    }
}

fn main() -> ExitCode {
    // Four operands including the command are admitted; retain one extra to
    // reject excess arguments without collecting an unbounded process argument list.
    let arguments: Vec<_> = std::env::args_os().skip(1).take(5).collect();
    let outcome = Command::try_from(arguments.as_slice())
        .map_err(|error| {
            // FR-301's contract: an unrecognized lowering target names a
            // real, catalogued capability this build does not implement,
            // unsupported (21); every other usage failure is invalid
            // input (20).
            let code = if matches!(error, cli::UsageError::UnknownTarget(_)) {
                21
            } else {
                20
            };
            (code, error.to_string())
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
            // FR-301's contract: a stdout/stderr write failure is a tool
            // failure (30), not a request-level disposition.
            ExitCode::from(30)
        }
    }
}
