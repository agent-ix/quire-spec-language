// SPDX-License-Identifier: AGPL-3.0-only
//! FR-010/FR-033: native CLI with OS paths and explicit phase/target selection.
use quire_spec_language::lowering::ProjectionTarget;
use quire_spec_language::{format::format, parse, Diagnostic, Limits, SourceIdentity};
use serde_json::json;
use std::ffi::OsString;
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

fn run(arguments: &[OsString]) -> Result<String, (u8, String)> {
    let [command, identity, revision, path] = arguments else {
        return Err((
            2,
            "usage: quire-spec <parse|format> <source-id> <source-revision> <file> | quire-spec <run|compile|lower> <request-file> [--target <boolean-oracle/v1|integer-ir/v1> (lower only)]".into(),
        ));
    };
    let Some(command) = command.to_str() else {
        return Err((2, "command must be UTF-8".into()));
    };
    let Some(identity) = identity.to_str() else {
        return Err((2, "source identity must be UTF-8".into()));
    };
    let Some(revision) = revision.to_str() else {
        return Err((2, "source revision must be UTF-8".into()));
    };
    if !matches!(command, "parse" | "format") {
        return Err((2, "command must be parse or format".into()));
    }
    let limits = Limits::default();
    let path = Path::new(path);
    let display_path = path.to_string_lossy();
    let file = std::fs::File::open(path)
        .map_err(|error| (2, format!("cannot open {display_path}: {error}")))?;
    let mut bytes = Vec::new();
    file.take(limits.source_bytes as u64 + 1)
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
    if command == "format" {
        return format(&unit).map_err(|error| diagnostic(&error));
    }
    Ok(
        json!({"status":"parsed", "source":{"identity":identity,"revision":revision,"digest":unit.source().digest().to_string()},
        "path":display_path, "imports":unit.imports().len(), "clauses":unit.clauses().len() })
        .to_string(),
    )
}

fn command_error(error: &quire_spec_language::command::RunError) -> ExitCode {
    match writeln!(io::stderr().lock(), "{}", error.value()) {
        Ok(()) => ExitCode::from(error.exit_code()),
        Err(output) if output.kind() == io::ErrorKind::BrokenPipe => {
            ExitCode::from(error.exit_code())
        }
        Err(_) => ExitCode::from(2),
    }
}

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).take(5).collect();
    let export = match arguments.as_slice() {
        [command, path] if command == "compile" => {
            Some(quire_spec_language::command::compile(Path::new(path)))
        }
        [command, path] if command == "lower" => {
            Some(quire_spec_language::command::lower(Path::new(path)))
        }
        [command, path, option, target] if command == "lower" && option == "--target" => {
            let target = match target.to_str() {
                Some("boolean-oracle/v1") => ProjectionTarget::BooleanOracleV1,
                Some("integer-ir/v1") => ProjectionTarget::IntegerIrV1,
                _ => {
                    let _ = writeln!(io::stderr().lock(), "unknown lowering target");
                    return ExitCode::from(2);
                }
            };
            Some(quire_spec_language::command::lower_for(
                Path::new(path),
                target,
            ))
        }
        _ => None,
    };
    if let Some(export) = export {
        return match export {
            Ok(bytes) => match io::stdout().lock().write_all(&bytes) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
                Err(error) => {
                    let _ = writeln!(io::stderr().lock(), "output failed: {error}");
                    ExitCode::from(2)
                }
            },
            Err(error) => command_error(&error),
        };
    }
    if let [command, path] = arguments.as_slice() {
        if command == "run" {
            return match quire_spec_language::command::run(Path::new(path)) {
                Ok(result) => match writeln!(io::stdout().lock(), "{}", result.value) {
                    Ok(()) => ExitCode::from(result.exit_code),
                    Err(error) if error.kind() == io::ErrorKind::BrokenPipe => {
                        ExitCode::from(result.exit_code)
                    }
                    Err(error) => {
                        let _ = writeln!(io::stderr().lock(), "output failed: {error}");
                        ExitCode::from(2)
                    }
                },
                Err(error) => command_error(&error),
            };
        }
    }
    match run(&arguments) {
        Ok(output) => match writeln!(io::stdout().lock(), "{}", output.trim_end_matches('\n')) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
            Err(error) => {
                let _ = writeln!(io::stderr().lock(), "output failed: {error}");
                ExitCode::from(2)
            }
        },
        Err((code, output)) => {
            let _ = writeln!(io::stderr().lock(), "{output}");
            ExitCode::from(code)
        }
    }
}
