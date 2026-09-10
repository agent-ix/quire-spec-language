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

fn execute(command: Command<'_>) -> Result<(u8, String), (u8, String)> {
    match command {
        Command::Syntax {
            kind,
            identity,
            revision,
            path,
        } => syntax(kind, identity, revision, path).map(|text| (0, text)),
        Command::Run { path } => match quire_spec_language::command::run(path) {
            Ok(result) => Ok((result.exit_code, result.value.to_string())),
            Err(error) => match error.value() {
                Ok(value) => Err((error.exit_code(), value.to_string())),
                Err(output) => Err((2, format!("output failed: {output}"))),
            },
        },
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
        Ok((code, output)) => (
            code,
            writeln!(io::stdout().lock(), "{}", output.trim_end_matches('\n')),
        ),
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
