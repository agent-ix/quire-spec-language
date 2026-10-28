// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-017 RF-2: the refinement gates of the xtask repository-gate family
//! (FR-344 "Invocation and exit"). `cargo xtask refinement versioning
//! <corpus>` checks that a superseding revision of a specification admits
//! every case its prior revision admitted (FR-340).
//!
//! The module owns the three pieces FR-340, FR-343 and FR-344 name:
//! [`versioning`] reads a corpus and runs each case through the spine,
//! [`compare`] turns the two classifications of a case into its result, and
//! [`report`] lists every result, gives the run its verdict and maps the
//! verdict to its exit code.

use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::error::{Error, Result};
use crate::Outcome;

pub mod compare;
pub mod layering;
pub mod report;
pub mod versioning;

const USAGE: &str = "usage: cargo xtask refinement versioning <corpus>\n       cargo xtask refinement layering <corpus>\n       cargo xtask refinement layering-check <corpus> <cli-path|->";

/// `cargo xtask refinement versioning <corpus>`: run the gate and return its
/// report with the exit code of its verdict (FR-344).
#[ix_trace_rs::string_edge]
pub fn run(workspace_root: &Path, operands: &[OsString]) -> Result<Outcome> {
    let [gate, corpus, trailing @ ..] = operands else {
        return Err(Error::Usage(USAGE));
    };
    let path = Path::new(corpus);
    let (stdout, code) = match gate.to_str() {
        Some("versioning") if trailing.is_empty() => {
            let report = versioning::run(path, workspace_root);
            let stdout = report
                .to_json()
                .map_err(|source| Error::RefinementReport { source })?;
            (stdout, report.verdict.exit_code())
        }
        Some("layering") if trailing.is_empty() => {
            // QSL-1199 will supply the production candidate registry. Until
            // that source exists, the honest snapshot is empty; do not invent
            // candidates to make the expected-exit target pass.
            let report = layering::run(path, &layering::RegistrySnapshot::default());
            let stdout = report
                .to_json()
                .map_err(|source| Error::RefinementReport { source })?;
            (stdout, report.verdict.exit_code())
        }
        Some("layering-check") if trailing.len() == 1 => {
            let cli = if trailing[0] == "-" {
                std::env::current_exe()
                    .map_err(|source| Error::io("current xtask executable", source))?
            } else {
                Path::new(&trailing[0]).to_path_buf()
            };
            let output = Command::new(&cli)
                .arg("refinement")
                .arg("layering")
                .arg(corpus)
                .output()
                .map_err(|source| Error::io(&cli, source))?;
            let report: serde_json::Value =
                serde_json::from_slice(&output.stdout).map_err(|error| {
                    Error::LayeringExpectedReport {
                        reason: format!("CLI output is not a JSON report: {error}"),
                    }
                })?;
            if output.status.code() != Some(21) || !matches_expected_layering_report(&report) {
                return Err(Error::LayeringExpectedReport {
                    reason: format!(
                        "CLI exited {:?}; expected exit 21 and the six unresolved witness results; stderr: {}",
                        output.status.code(),
                        String::from_utf8_lossy(&output.stderr)
                    ),
                });
            }
            (
                "refinement-layering: expected unsupported report\n".to_owned(),
                0,
            )
        }
        _ => return Err(Error::Usage(USAGE)),
    };
    Ok(Outcome { stdout, code })
}

#[ix_trace_rs::string_edge]
fn matches_expected_layering_report(report: &serde_json::Value) -> bool {
    if report.get("verdict").and_then(serde_json::Value::as_str) != Some("unsupported")
        || report.get("exit").and_then(serde_json::Value::as_u64) != Some(21)
    {
        return false;
    }
    let Some(results) = report.get("results").and_then(serde_json::Value::as_array) else {
        return false;
    };
    let mut seen_layers = std::collections::BTreeSet::new();
    for result in results {
        if result.get("kind").and_then(serde_json::Value::as_str) == Some("layer") {
            let Some(layer) = result.get("layer").and_then(serde_json::Value::as_str) else {
                return false;
            };
            if !qsl_semantics::value::HeaderLayer::ALL
                .into_iter()
                .any(|known| known.identity() == layer)
                || !seen_layers.insert(layer)
                || result.get("result").and_then(serde_json::Value::as_str)
                    != Some("unresolved (unsupported)")
            {
                return false;
            }
            let Some(witnesses) = result
                .get("witnesses")
                .and_then(serde_json::Value::as_array)
            else {
                return false;
            };
            if witnesses.len() != 1
                || !witnesses.iter().all(|witness| {
                    witness.get("class").and_then(serde_json::Value::as_str) == Some("admitted")
                        && witness.get("result").and_then(serde_json::Value::as_str)
                            == Some("unresolved (unsupported)")
                        && witness.get("reason").and_then(serde_json::Value::as_str)
                            == Some("not-negotiated")
                        && witness
                            .get("candidates")
                            .and_then(serde_json::Value::as_array)
                            .is_some_and(|candidates| {
                                !candidates.is_empty()
                                    && candidates.iter().all(|candidate| {
                                        candidate
                                            .get("disposition")
                                            .and_then(serde_json::Value::as_str)
                                            == Some("not-negotiated")
                                    })
                            })
                })
            {
                return false;
            }
        } else if result.get("kind").and_then(serde_json::Value::as_str) == Some("tool_failure")
            || !matches!(
                result.get("result").and_then(serde_json::Value::as_str),
                Some("holds" | "not applicable")
            )
        {
            return false;
        }
    }
    seen_layers.len() == qsl_semantics::value::HeaderLayer::ALL.len()
}
