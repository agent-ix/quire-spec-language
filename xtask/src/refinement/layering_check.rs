// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-345-AC-9 and AC-13: the expected-exit check `make refinement-layering`
//! runs. With no negotiation function the CLI requests no witness item, so
//! its expected run exits 21 (FR-344 `unsupported`), every layer result is
//! `unresolved (unsupported)` with reason `not-negotiated` over a non-empty
//! candidate set, and every case and edge holds or is not applicable. The
//! check runs the CLI as a child process, reads its report and passes only
//! that run.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use qsl_semantics::value::HeaderLayer;
use serde::Deserialize;

use super::compare::CaseResult;
use super::report::Verdict;
use crate::error::{Error, Result};

/// The CLI exit code of FR-344's `unsupported` verdict.
const UNSUPPORTED_EXIT: i32 = 21;
/// The reason a witness that was not requested is unresolved.
const NOT_NEGOTIATED: &str = "not-negotiated";

/// Why the CLI's run is not the expected unsupported run.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum Mismatch {
    /// The CLI exited with another code, or was stopped by a signal.
    #[error("the CLI exited {found:?}, not 21")]
    Exit {
        /// The exit code, `None` for a signal.
        found: Option<i32>,
    },
    /// The CLI's stdout is no FR-344 report.
    #[error("the CLI output is no report: {detail}")]
    NotAReport {
        /// The decoding failure.
        detail: String,
    },
    /// The report's verdict or exit member is not `unsupported`/21.
    #[error("the report's verdict is {verdict:?} with exit {exit}, not unsupported with exit 21")]
    Verdict {
        /// The report's verdict.
        verdict: Verdict,
        /// The report's exit member.
        exit: u8,
    },
    /// A case or edge result is neither `holds` nor `not applicable`.
    #[error("a {kind} result is {result:?}")]
    Result {
        /// `case` or `edge`.
        kind: &'static str,
        /// Its result.
        result: CaseResult,
    },
    /// The report holds a tool-failure result.
    #[error("tool failure at {entry}")]
    ToolFailure {
        /// The entry it names.
        entry: String,
    },
    /// A layer result names no header-selectable layer, or names one twice.
    #[error("the layer result {layer} is unknown or repeated")]
    Layer {
        /// The layer the result names.
        layer: String,
    },
    /// A layer's result is not one unresolved, not-negotiated witness of an
    /// admitted unit over a non-empty candidate set.
    #[error("the layer {layer} is not one unrequested admitted witness")]
    Witness {
        /// The layer.
        layer: String,
    },
    /// Some layer has no result.
    #[error("the report has {found} layer results, not 6")]
    Layers {
        /// The number of layer results.
        found: usize,
    },
}

/// The members of FR-344's layering report the check reads.
#[derive(Deserialize)]
struct CliReport {
    results: Vec<CliResult>,
    verdict: Verdict,
    exit: u8,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CliResult {
    Case {
        result: CaseResult,
    },
    Edge {
        result: CaseResult,
    },
    Layer {
        layer: String,
        result: CaseResult,
        witnesses: Vec<CliWitness>,
    },
    ToolFailure {
        entry: String,
    },
}

#[derive(Deserialize)]
struct CliWitness {
    class: String,
    candidates: Vec<CliCandidate>,
    reason: Option<String>,
    result: CaseResult,
}

#[derive(Deserialize)]
struct CliCandidate {
    disposition: String,
}

/// FR-345-AC-9: whether the CLI's exit `code` and `stdout` are the
/// expected no-negotiation run.
///
/// # Errors
/// The first [`Mismatch`] found.
// Intake proof: the_expected_unsupported_run_passes. The CLI report's
// witness class, reason and dispositions are read as its wire words.
#[ix_trace_rs::string_edge]
pub fn expected_run(code: Option<i32>, stdout: &[u8]) -> std::result::Result<(), Mismatch> {
    if code != Some(UNSUPPORTED_EXIT) {
        return Err(Mismatch::Exit { found: code });
    }
    let report: CliReport =
        serde_json::from_slice(stdout).map_err(|error| Mismatch::NotAReport {
            detail: error.to_string(),
        })?;
    if report.verdict != Verdict::Unsupported || i32::from(report.exit) != UNSUPPORTED_EXIT {
        return Err(Mismatch::Verdict {
            verdict: report.verdict,
            exit: report.exit,
        });
    }
    let mut layers = BTreeSet::new();
    for result in report.results {
        match result {
            CliResult::Case { result } | CliResult::Edge { result }
                if matches!(result, CaseResult::Holds | CaseResult::NotApplicable) => {}
            CliResult::Case { result } => {
                return Err(Mismatch::Result {
                    kind: "case",
                    result,
                })
            }
            CliResult::Edge { result } => {
                return Err(Mismatch::Result {
                    kind: "edge",
                    result,
                })
            }
            CliResult::ToolFailure { entry } => return Err(Mismatch::ToolFailure { entry }),
            CliResult::Layer {
                layer,
                result,
                witnesses,
            } => {
                if HeaderLayer::from_identity(&layer).is_none() || !layers.insert(layer.clone()) {
                    return Err(Mismatch::Layer { layer });
                }
                let unrequested = |witness: &CliWitness| {
                    witness.class == "admitted"
                        && witness.result == CaseResult::UnresolvedUnsupported
                        && witness.reason.as_deref() == Some(NOT_NEGOTIATED)
                        && !witness.candidates.is_empty()
                        && witness
                            .candidates
                            .iter()
                            .all(|candidate| candidate.disposition == NOT_NEGOTIATED)
                };
                if result != CaseResult::UnresolvedUnsupported
                    || witnesses.len() != 1
                    || !witnesses.iter().all(unrequested)
                {
                    return Err(Mismatch::Witness { layer });
                }
            }
        }
    }
    if layers.len() != HeaderLayer::ALL.len() {
        return Err(Mismatch::Layers {
            found: layers.len(),
        });
    }
    Ok(())
}

/// Run `cli refinement layering <corpus>` and check its run by
/// [`expected_run`]. `make refinement-layering` passes the running xtask
/// executable; a test passes a stub executable.
///
/// # Errors
/// [`Error::Io`] when the CLI cannot be started, and
/// [`Error::LayeringUnexpectedRun`] when its run is not the expected one.
pub fn check(cli: &Path, corpus: &Path) -> Result<()> {
    let output = Command::new(cli)
        .arg("refinement")
        .arg("layering")
        .arg(corpus)
        .output()
        .map_err(|source| Error::io(cli, source))?;
    expected_run(output.status.code(), &output.stdout)
        .map_err(|mismatch| Error::LayeringUnexpectedRun { mismatch })
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use serde_json::{json, Value};

    use super::*;

    /// The expected report, with the layer results of `layers`.
    fn report(layers: &[HeaderLayer]) -> Value {
        let mut results = vec![
            json!({"kind": "case", "result": "holds"}),
            json!({"kind": "case", "result": "not applicable"}),
            json!({"kind": "edge", "result": "holds"}),
        ];
        results.extend(layers.iter().map(|layer| {
            json!({
                "kind": "layer",
                "layer": layer.identity(),
                "result": "unresolved (unsupported)",
                "witnesses": [{
                    "class": "admitted",
                    "codes": [],
                    "candidates": [{"backend": "backend-a", "disposition": "not-negotiated"}],
                    "reason": "not-negotiated",
                    "result": "unresolved (unsupported)",
                }],
            })
        }));
        json!({"results": results, "verdict": "unsupported", "exit": 21})
    }

    fn bytes(value: &Value) -> Vec<u8> {
        serde_json::to_vec(value).unwrap()
    }

    /// FR-345-AC-13: exit 21 with all six layers unresolved, not negotiated,
    /// and every case and edge holding or not applicable is the expected
    /// run.
    #[trace("TC-868", "FR-345-AC-13")]
    #[test]
    fn the_expected_unsupported_run_passes() {
        assert_eq!(
            expected_run(Some(21), &bytes(&report(&HeaderLayer::ALL))),
            Ok(())
        );
    }

    /// FR-345-AC-13: child exits 0, 1 and 10 fail, and so does exit 21 with
    /// one layer result missing, with an empty candidate set, or with a
    /// regressed case.
    #[trace("TC-868", "FR-345-AC-13")]
    #[test]
    fn any_other_run_fails() {
        let expected = bytes(&report(&HeaderLayer::ALL));
        for code in [0, 1, 10] {
            assert_eq!(
                expected_run(Some(code), &expected),
                Err(Mismatch::Exit { found: Some(code) })
            );
        }
        assert_eq!(
            expected_run(Some(21), &bytes(&report(&HeaderLayer::ALL[1..]))),
            Err(Mismatch::Layers { found: 5 })
        );
        let mut empty = report(&HeaderLayer::ALL);
        empty["results"][3]["witnesses"][0]["candidates"] = json!([]);
        assert_eq!(
            expected_run(Some(21), &bytes(&empty)),
            Err(Mismatch::Witness {
                layer: HeaderLayer::ALL[0].identity().to_owned()
            })
        );
        let mut regressed = report(&HeaderLayer::ALL);
        regressed["results"][0]["result"] = json!("regression");
        assert_eq!(
            expected_run(Some(21), &bytes(&regressed)),
            Err(Mismatch::Result {
                kind: "case",
                result: CaseResult::Regression
            })
        );
        assert!(matches!(
            expected_run(Some(21), b"not json"),
            Err(Mismatch::NotAReport { .. })
        ));
    }

    /// FR-345-AC-13 through the CLI seam: a stub executable, passed
    /// directly, that prints the expected report passes only when it exits
    /// 21.
    #[cfg(unix)]
    #[trace("TC-868", "FR-345-AC-13")]
    #[test]
    fn the_check_runs_the_cli_it_is_given() {
        use std::os::unix::fs::PermissionsExt;
        let temp = crate::test_support::tempdir();
        let report_path = temp.path().join("report.json");
        std::fs::write(&report_path, bytes(&report(&HeaderLayer::ALL))).unwrap();
        let stub = |code: i32| {
            let path = temp.path().join(format!("stub-{code}"));
            std::fs::write(
                &path,
                format!("#!/bin/sh\ncat '{}'\nexit {code}\n", report_path.display()),
            )
            .unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            path
        };
        assert!(check(&stub(21), temp.path()).is_ok());
        for code in [0, 1, 10] {
            assert!(matches!(
                check(&stub(code), temp.path()),
                Err(Error::LayeringUnexpectedRun {
                    mismatch: Mismatch::Exit { .. }
                })
            ));
        }
    }
}
