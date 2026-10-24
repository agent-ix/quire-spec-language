// SPDX-License-Identifier: AGPL-3.0-or-later
//! `qsl-string-edge --base-dir <directory> --crate-root <source-directory>
//! [--crate-root <source-directory> ...] --rules <rules.json>`: FR-064's
//! command. Exit 0 for a completed clean report, 1 for a completed report
//! with violations, 2 for a request, rule, read or parse failure.
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use ix_trace_rs::string_edge;
use qsl_string_edge::{scan, EdgeRules, Error, ScanRequest};

const USAGE: &str = "usage: qsl-string-edge --base-dir <directory> --crate-root \
                     <source-directory> [--crate-root <source-directory> ...] --rules <rules.json>";

/// What a failed run says on stderr, and its exit code (always 2).
enum Failure {
    Usage,
    Scan(Error),
}

/// The parsed command line.
struct Arguments {
    base_dir: PathBuf,
    crate_roots: Vec<PathBuf>,
    rules: PathBuf,
}

/// The CLI argument-parsing edge: turns each flag spelling into a field, or
/// refuses the command line.
#[string_edge]
fn parse(arguments: &[OsString]) -> Option<Arguments> {
    let mut base_dir = None;
    let mut crate_roots = Vec::new();
    let mut rules = None;
    let mut arguments = arguments.iter();
    while let Some(flag) = arguments.next() {
        let value = PathBuf::from(arguments.next()?);
        match flag.to_str()? {
            "--base-dir" if base_dir.is_none() => base_dir = Some(value),
            "--crate-root" => crate_roots.push(value),
            "--rules" if rules.is_none() => rules = Some(value),
            _ => return None,
        }
    }
    Some(Arguments {
        base_dir: base_dir?,
        crate_roots,
        rules: rules?,
    })
}

fn run(arguments: &[OsString]) -> Result<qsl_string_edge::Report, Failure> {
    let arguments = parse(arguments).ok_or(Failure::Usage)?;
    // Relative rules paths resolve against the base (FR-064).
    let rules_path = arguments.base_dir.join(&arguments.rules);
    let text = fs::read_to_string(&rules_path).map_err(|source| {
        Failure::Scan(Error::RulesRead {
            path: rules_path,
            source,
        })
    })?;
    let rules = EdgeRules::from_json(&text).map_err(Failure::Scan)?;
    scan(&ScanRequest {
        base_dir: arguments.base_dir,
        crate_roots: arguments.crate_roots,
        rules,
    })
    .map_err(Failure::Scan)
}

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    match run(&arguments) {
        Ok(report) => match write!(io::stdout().lock(), "{}", report.render()) {
            Ok(()) if report.is_clean() => ExitCode::SUCCESS,
            Ok(()) => ExitCode::from(1),
            Err(error) => {
                let _ = writeln!(io::stderr().lock(), "io: cannot write report: {error}");
                ExitCode::from(2)
            }
        },
        Err(failure) => {
            let _ = match failure {
                Failure::Usage => writeln!(io::stderr().lock(), "{USAGE}"),
                Failure::Scan(error) => writeln!(io::stderr().lock(), "{error}"),
            };
            ExitCode::from(2)
        }
    }
}
