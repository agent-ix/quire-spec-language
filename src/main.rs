// SPDX-License-Identifier: AGPL-3.0-only
//! FR-010/026: parse one command, execute it, and write one explicit outcome.
mod cli;

use cli::{Command, SyntaxCommand};
use quire_spec_language::{format::format, parse, Diagnostic, Limits, SourceIdentity};
use serde_json::json;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::ExitCode;

fn diagnostic(value: &Diagnostic) -> (u8, String) {
    let incomplete = value.is_incomplete();
    let span = value.span;
    let output = json!({ "status": if incomplete { "incomplete" } else { "refused" },
        "phase": value.phase.as_str(), "code": value.code.as_str(),
        "source": {"identity": value.source.identity, "revision": value.source.revision},
        "path": value.path, "span": {
            "start": {"byte":span.start.byte,"line":span.start.line,"column":span.start.column},
            "end": {"byte":span.end.byte,"line":span.end.line,"column":span.end.column}},
        "message": value.message })
    .to_string();
    (if incomplete { 3 } else { 1 }, output)
}

fn syntax(
    command: SyntaxCommand,
    identity: &str,
    revision: &str,
    path: &Path,
) -> Result<String, (u8, String)> {
    let limits = Limits::default();
    let display_path = path.to_string_lossy();
    let file = std::fs::File::open(path)
        .map_err(|error| (2, format!("cannot open {display_path}: {error}")))?;
    let mut bytes = Vec::new();
    let ceiling = u64::try_from(limits.source_bytes)
        .map_err(|error| (2, format!("invalid source ceiling: {error}")))?;
    file.take(ceiling.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| (2, format!("cannot read {display_path}: {error}")))?;
    let unit = parse(
        SourceIdentity {
            identity: identity.into(),
            revision: revision.into(),
        },
        display_path.as_ref(),
        &bytes,
        limits,
    )
    .map_err(|error| diagnostic(&error))?;
    if command == SyntaxCommand::Format {
        return format(&unit).map_err(|error| diagnostic(&error));
    }
    Ok(
        json!({"status":"parsed", "source":{"identity":identity,"revision":revision,"digest":unit.source().digest().to_string()},
        "path":display_path, "imports":unit.imports().len(), "clauses":unit.clauses().len() })
        .to_string(),
    )
}

enum Output {
    Line(String),
    Artifact(Vec<u8>),
}

fn command_error(error: &quire_spec_language::command::RunError) -> (u8, String) {
    match error.value() {
        Ok(value) => (error.exit_code(), value.to_string()),
        Err(output) => (2, format!("output failed: {output}")),
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
    }
}

fn main() -> ExitCode {
    // Four operands including the command are admitted; retain one extra to
    // reject excess arguments without collecting an unbounded process argument list.
    let arguments: Vec<_> = std::env::args_os().skip(1).take(5).collect();
    let outcome = Command::try_from(arguments.as_slice())
        .map_err(|error| (2, error.to_string()))
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
            ExitCode::from(2)
        }
    }
}
