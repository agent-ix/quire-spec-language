// SPDX-License-Identifier: AGPL-3.0-or-later
//! The local QSpec vector gate. Path resolution and child environments live here,
//! so Make never interprets checkout path bytes as source.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Error, Result};

const VECTOR: &str = "proposals/checked-package-v2/node-identity-vectors.json";

struct Check {
    package: &'static str,
    target: Target,
    selection: &'static str,
    summaries: &'static [&'static [&'static str]],
    summary_mode: SummaryMode,
}

#[derive(Clone, Copy)]
enum SummaryMode {
    Exact,
    Prefix,
}

#[derive(Clone, Copy)]
enum Target {
    Lib,
    Test,
}

#[derive(Clone, Copy)]
enum OutputMarker {
    SuccessfulTest,
    Skipped,
    Other,
}

fn output_marker(line: &str) -> OutputMarker {
    let bytes = line.as_bytes();
    if bytes.starts_with(b"test result: ok. 1 passed; 0 failed; 0 ignored;") {
        OutputMarker::SuccessfulTest
    } else if bytes.starts_with(b"skipped:") {
        OutputMarker::Skipped
    } else {
        OutputMarker::Other
    }
}

impl Target {
    fn argument(self) -> &'static str {
        match self {
            Self::Lib => "--lib",
            Self::Test => "--test",
        }
    }
}

const CHECKS: &[Check] = &[
    Check { package: "qsl-semantics", target: Target::Lib, selection: "check::node_key::tests::conformance_fr322_application_keys_match_qspec_operation_vectors", summaries: &[&["conformance: ", " of ", " QSpec operation vectors match ("]], summary_mode: SummaryMode::Exact },
    Check { package: "qsl-semantics", target: Target::Lib, selection: "check::node_key::tests::conformance_fr092_nominal_enum_keys_match_qspec_vectors", summaries: &[&["conformance: 2 nominal enum vectors"]], summary_mode: SummaryMode::Exact },
    Check { package: "qsl-semantics", target: Target::Test, selection: "quantities::tc_411_compound_unit_ids_match_qspec_vectors", summaries: &[&["conformance: ", " compound-unit vectors"]], summary_mode: SummaryMode::Exact },
    Check { package: "qsl-package", target: Target::Lib, selection: "checked_v2::tests::conformance_c14_source_map_lookup_over_qspec_positive_fixtures", summaries: &[&["conformance: ", " source-map entries over ", " positive fixtures"]], summary_mode: SummaryMode::Exact },
    Check { package: "qsl-package", target: Target::Lib, selection: "checked_v2::tests::conformance_i2_read_over_qspec_checked_package_v2_fixtures", summaries: &[&["conformance: ", " positive fixtures through QSL's full I2 read"], &["conformance: ", " adverse mutations refused"]], summary_mode: SummaryMode::Prefix },
    Check { package: "qsl-package", target: Target::Lib, selection: "checked_v2::tests::conformance_fr340_frame_mutations_match_qspec_vectors", summaries: &[&["conformance: ", " frame-body mutation vectors matched"]], summary_mode: SummaryMode::Exact },
    Check { package: "qsl-package", target: Target::Lib, selection: "checked_v2::tests::conformance_dependency_selection_vectors", summaries: &[&["conformance: ", " dependency-selection entry mutations and ", " order vectors"]], summary_mode: SummaryMode::Exact },
    Check { package: "qsl-package", target: Target::Lib, selection: "emit::tests::golden::conformance_emitted_application_nodes_match_qspec_positive_fixtures", summaries: &[&["conformance: ", " emitted application nodes match QSpec's positive fixtures"]], summary_mode: SummaryMode::Exact },
    Check { package: "qsl-semantics", target: Target::Lib, selection: "check::profile::tests::conformance_fr110_profile_causes_are_listed_by_qspec_native_diagnostics", summaries: &[&["conformance: ", " profile (code, cause) pairs listed by QSpec"]], summary_mode: SummaryMode::Exact },
    Check { package: "qsl-semantics", target: Target::Lib, selection: "library::bundle_tests::conformance_fr111_resolution_causes_are_listed_by_qspec_native_diagnostics", summaries: &[&["conformance: ", " bundle (code, cause) pairs listed by QSpec"]], summary_mode: SummaryMode::Exact },
    Check { package: "qsl-semantics", target: Target::Test, selection: "complete_value_lock::conformance_catalog_matches_qspec_complete_value_lock", summaries: &[&["conformance: ", " catalog rows match QSpec's lock"]], summary_mode: SummaryMode::Exact },
    Check { package: "qsl-semantics", target: Target::Test, selection: "complete_value_lock::conformance_admit_selection_matches_qspec_selection_vectors", summaries: &[&["conformance: ", " accepted and ", " refused selection vectors"]], summary_mode: SummaryMode::Exact },
];

fn resolve(input: Option<&OsStr>, workspace: &Path) -> Result<PathBuf> {
    let raw = input
        .filter(|value| !value.is_empty())
        .ok_or_else(|| Error::ConformanceInput {
            path: PathBuf::new(),
            reason: "set QSPEC_DIR to a quire-specification checkout".to_owned(),
        })?;
    let path = Path::new(raw);
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace.join(path)
    };
    let absolute = std::fs::canonicalize(candidate).map_err(|error| Error::ConformanceInput {
        path: path.to_path_buf(),
        reason: error.to_string(),
    })?;
    if !absolute.join(VECTOR).is_file() {
        return Err(Error::ConformanceInput {
            path: path.to_path_buf(),
            reason: format!("missing {VECTOR}"),
        });
    }
    Ok(absolute)
}

fn matches_summary(line: &str, pieces: &[&str], mode: SummaryMode) -> bool {
    let Some(mut remainder) = line.strip_prefix(pieces[0]) else {
        return false;
    };
    if pieces.len() == 1 {
        return true;
    }
    for (index, piece) in pieces.iter().enumerate().skip(1) {
        let digits = remainder.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 || remainder.as_bytes()[0] == b'0' {
            return false;
        }
        remainder = &remainder[digits..];
        let Some(rest) = remainder.strip_prefix(piece) else {
            return false;
        };
        remainder = rest;
        if index == pieces.len() - 1 && !piece.ends_with('(') && matches!(mode, SummaryMode::Exact)
        {
            return remainder.is_empty();
        }
    }
    true
}

fn check_output(check: &Check, output: &std::process::Output) -> Result<()> {
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    print!("{combined}");
    if !output.status.success() {
        return Err(Error::ConformanceCheck {
            selection: check.selection,
            reason: format!("cargo exited with {}", output.status),
        });
    }
    if !combined
        .lines()
        .any(|line| matches!(output_marker(line), OutputMarker::SuccessfulTest))
    {
        return Err(Error::ConformanceCheck {
            selection: check.selection,
            reason: "required test did not execute successfully".to_owned(),
        });
    }
    if combined
        .lines()
        .any(|line| matches!(output_marker(line), OutputMarker::Skipped))
    {
        return Err(Error::ConformanceCheck {
            selection: check.selection,
            reason: "required vector check skipped".to_owned(),
        });
    }
    if !check.summaries.iter().all(|pattern| {
        combined
            .lines()
            .any(|line| matches_summary(line, pattern, check.summary_mode))
    }) {
        return Err(Error::ConformanceCheck {
            selection: check.selection,
            reason: "required vector summary did not appear".to_owned(),
        });
    }
    Ok(())
}

fn run_with(
    cargo: &Path,
    workspace: &Path,
    input: Option<&OsStr>,
    child_env: &[(&str, &OsStr)],
) -> Result<String> {
    let checkout = resolve(input, workspace)?;
    for check in CHECKS {
        let mut command = Command::new(cargo);
        command.current_dir(workspace).args([
            "test",
            "--locked",
            "-p",
            check.package,
            check.target.argument(),
        ]);
        if matches!(check.target, Target::Test) {
            command.arg("it");
        }
        let output = command
            .args([
                "--",
                "--exact",
                check.selection,
                "--format",
                "terse",
                "--nocapture",
            ])
            .env("QSPEC_DIR", &checkout)
            .envs(child_env.iter().copied())
            .output()
            .map_err(|source| Error::ConformanceSpawn {
                selection: check.selection,
                source,
            })?;
        check_output(check, &output)?;
    }
    Ok(String::new())
}

/// Validate the checkout and run every required exact QSpec vector selection.
pub fn run(workspace: &Path) -> Result<String> {
    let input: Option<OsString> = std::env::var_os("QSPEC_DIR");
    run_with(Path::new("cargo"), workspace, input.as_deref(), &[])
}

/// Validate the checkout before other local CI lanes begin.
pub fn check_input(workspace: &Path) -> Result<String> {
    let input = std::env::var_os("QSPEC_DIR");
    resolve(input.as_deref(), workspace)?;
    Ok(String::new())
}

fn run_ci_with(
    make: &Path,
    workspace: &Path,
    input: Option<&OsStr>,
    args: &[&str],
    child_env: &[(&str, &OsStr)],
) -> Result<String> {
    let checkout = resolve(input, workspace)?;
    let status = Command::new(make)
        .current_dir(workspace)
        .arg("--no-print-directory")
        .arg("ci-internal")
        .args(args)
        .env("QSPEC_DIR", checkout)
        .env_remove("MAKEFLAGS")
        .env_remove("MFLAGS")
        .envs(child_env.iter().copied())
        .status()
        .map_err(|source| Error::ConformanceCiSpawn { source })?;
    if !status.success() {
        return Err(Error::ConformanceCiFailed { status });
    }
    Ok(String::new())
}

/// Launch the entire local CI graph with one validated absolute QSpec checkout.
pub fn run_ci(workspace: &Path) -> Result<String> {
    let input = std::env::var_os("QSPEC_DIR");
    run_ci_with(Path::new("make"), workspace, input.as_deref(), &[], &[])
}

#[cfg(test)]
mod tests;
