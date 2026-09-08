// SPDX-License-Identifier: AGPL-3.0-only
use quire_spec_language::{format::format, parse, Diagnostic, Limits, SourceIdentity};
use serde_json::json;
use std::io::{self, Read, Write};
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

fn run() -> Result<String, (u8, String)> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let [command, identity, revision, path] = arguments.as_slice() else {
        return Err((
            2,
            "usage: quire-spec <parse|format> <source-id> <source-revision> <file>".into(),
        ));
    };
    if !matches!(command.as_str(), "parse" | "format") {
        return Err((2, "command must be parse or format".into()));
    }
    let limits = Limits::default();
    let file =
        std::fs::File::open(path).map_err(|error| (2, format!("cannot open {path}: {error}")))?;
    let mut bytes = Vec::new();
    file.take(limits.source_bytes as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| (2, format!("cannot read {path}: {error}")))?;
    let unit = parse(
        SourceIdentity {
            identity: identity.clone(),
            revision: revision.clone(),
        },
        path,
        &bytes,
        limits,
    )
    .map_err(|error| diagnostic(&error))?;
    if command == "format" {
        return format(&unit).map_err(|error| diagnostic(&error));
    }
    Ok(
        json!({"status":"parsed", "source":{"identity":identity,"revision":revision},
        "path":path, "imports":unit.imports().len(), "clauses":unit.clauses().len() })
        .to_string(),
    )
}

fn main() -> ExitCode {
    match run() {
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
