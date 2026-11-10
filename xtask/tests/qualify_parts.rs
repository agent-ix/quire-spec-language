// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1396: the `qualify` gate's verdict covers the corpus, the lifecycle
//! matrix and the parity of the linked backends (FR-567).

use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use serde_json::Value;
use xtask::qualify::{self, QualifyInputs};

fn qspec_dir() -> PathBuf {
    std::env::var_os("QSPEC_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("xtask is under the workspace root");
            Some(workspace.parent()?.join("quire-specification"))
        })
        .expect("QSpec checkout path")
}

fn inputs() -> QualifyInputs {
    let fixtures = qspec_dir().join("proposals/conformance-corpus-v1/fixtures");
    QualifyInputs {
        qspec: qspec_dir(),
        operands: vec!["--corpus".into(), fixtures.into_os_string()],
        #[cfg(feature = "test-support")]
        matrix: None,
    }
}

fn report(outcome: &xtask::Outcome) -> Value {
    serde_json::from_str(&outcome.stdout).expect("the report is JSON")
}

fn part<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["parts"]
        .as_array()
        .and_then(|parts| parts.iter().find(|part| part["part"] == name))
        .unwrap_or_else(|| panic!("the report names the part {name}: {report}"))
}

/// The no-backend half of FR-567-AC-4 and the passing run of AC-1 without a
/// linked backend: not tagged, because AC-1 needs a linked delegating backend
/// and AC-4 a reference fault, and the FR-563 runner they need is not in this
/// repository yet. Over the passing fixture corpus
/// with every matrix case passing, the report names the three parts, the
/// `parity` part `no-backend`, `complete-v1: qualified`, exit 0.
#[trace("TC-1396")]
#[test]
fn a_passing_run_names_the_three_parts_and_qualifies() {
    let outcome = qualify::run(&inputs()).expect("the gate runs");
    assert_eq!(outcome.code, 0, "{}", outcome.stdout);
    let report = report(&outcome);
    assert_eq!(report["verdict"], "qualified");
    assert_eq!(report["qualification"], "complete-v1");
    assert_eq!(part(&report, "corpus")["result"], "passed");
    assert_eq!(part(&report, "lifecycle-matrix")["result"], "passed");
    assert_eq!(part(&report, "parity")["result"], "no-backend");
}

/// FR-567-AC-5, first half only (so the test carries no `FR-567-AC-5` tag): the report's QSL version is the
/// `quire-spec-language` root package's own `CARGO_PKG_VERSION`, not xtask's,
/// and it is the version the root manifest declares. The scratch-copy half
/// (a copy whose version is `9.9.9` reports `9.9.9`) needs a rebuild of the
/// copy and is not run here (docs/outcome-parity.md, Known gaps).
#[trace("TC-1396")]
#[test]
fn the_report_names_the_root_package_version() {
    let outcome = qualify::run(&inputs()).expect("the gate runs");
    let version = report(&outcome)["qsl_version"].clone();
    assert_eq!(version, quire_spec_language::VERSION);
    assert_ne!(
        version,
        env!("CARGO_PKG_VERSION"),
        "xtask's own version is not the QSL version"
    );
    let manifest =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml"))
            .expect("the root manifest");
    assert!(
        manifest.contains(&format!("version = \"{}\"", quire_spec_language::VERSION)),
        "the root manifest declares the reported version"
    );
}

/// FR-567-AC-2: with the matrix replaced by one whose `check`/`refusal` case
/// expects exit 21, the run is `not-qualified`, names
/// `lifecycle-matrix/check/refusal` and exits 1, although the corpus passes.
#[cfg(feature = "test-support")]
#[trace("TC-1396", "FR-567-AC-2")]
#[test]
fn a_failing_matrix_case_fails_the_gate_by_name() {
    use qsl_conformance::{lifecycle_matrix, MatrixCell, OutcomeKind, Verb};
    let cells: Vec<MatrixCell> = lifecycle_matrix()
        .iter()
        .map(|cell| match cell {
            MatrixCell::Case(case)
                if case.verb == Verb::Check && case.kind == OutcomeKind::Refusal =>
            {
                MatrixCell::Case(qsl_conformance::MatrixCase {
                    expected_exit: 21,
                    ..*case
                })
            }
            other => *other,
        })
        .collect();
    let mut gate = inputs();
    gate.matrix = Some(cells);
    let outcome = qualify::run(&gate).expect("the gate runs");
    assert_eq!(outcome.code, 1, "{}", outcome.stdout);
    let report = report(&outcome);
    assert_eq!(report["verdict"], "not-qualified");
    assert_eq!(part(&report, "corpus")["result"], "passed");
    assert_eq!(
        part(&report, "lifecycle-matrix")["failures"],
        serde_json::json!(["lifecycle-matrix/check/refusal"])
    );
}
