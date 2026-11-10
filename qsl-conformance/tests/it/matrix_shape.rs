// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1394: the published cases give their expected documents, stable and
//! self-contained.

use ix_trace_rs::trace;
use qsl_conformance::{
    lifecycle_matrix, run_case, CaseInput, CaseStage, Input, MatrixCell, OutcomeKind, Report, Verb,
};

/// FR-564's n/a cells.
const NOT_APPLICABLE: [(Verb, OutcomeKind); 8] = [
    (Verb::Check, OutcomeKind::Violation),
    (Verb::Check, OutcomeKind::ToolFailure),
    (Verb::Package, OutcomeKind::Violation),
    (Verb::Package, OutcomeKind::Refusal),
    (Verb::Package, OutcomeKind::ToolFailure),
    (Verb::Execute, OutcomeKind::Violation),
    (Verb::Analyze, OutcomeKind::ToolFailure),
    (Verb::Monitor, OutcomeKind::ToolFailure),
];

/// FR-565-AC-1: the cells are in verb then kind order, one per (verb, kind)
/// pair, and the n/a cells sit at FR-564's n/a positions.
#[trace("TC-1394", "FR-565-AC-1")]
#[test]
fn cells_are_in_fr564_order_with_not_applicable_at_its_positions() {
    let cells = lifecycle_matrix();
    let positions: Vec<_> = cells
        .iter()
        .map(|cell| (cell.verb(), cell.kind()))
        .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "{positions:?}"
    );
    let not_applicable: Vec<_> = cells
        .iter()
        .filter(|cell| matches!(cell, MatrixCell::NotApplicable { .. }))
        .map(|cell| (cell.verb(), cell.kind()))
        .collect();
    assert_eq!(not_applicable, NOT_APPLICABLE);
    // Every pair of the six verbs has exactly one cell.
    assert_eq!(positions.len(), Verb::ALL.len() * OutcomeKind::ALL.len());
    // Each n/a cell states its reason, and the 48 cells hold 40 cases.
    assert!(cells.iter().all(|cell| match cell {
        MatrixCell::NotApplicable { reason, .. } => !reason.is_empty(),
        MatrixCell::Case(_) => true,
    }));
    assert_eq!(cells.len(), 48);
    assert_eq!(
        cells
            .iter()
            .filter(|cell| matches!(cell, MatrixCell::Case(_)))
            .count(),
        40
    );
}

/// FR-565-AC-2 and FR-564-AC-7: every `Case` gives its expected category and
/// exit code, the exit is the one FR-285's function gives the category, two
/// runs give equal outcomes, the document names its verb, and every source
/// identity is the case's explicitly supplied authored label.
#[trace("TC-1394", "FR-565-AC-2")]
#[trace("TC-1394", "FR-564-AC-7")]
#[test]
fn every_case_gives_its_expected_stable_document() {
    for cell in lifecycle_matrix() {
        let MatrixCell::Case(case) = cell else {
            continue;
        };
        let label = format!("{:?}/{:?}", case.verb, case.kind);
        let first = run_case(case).unwrap_or_else(|error| panic!("{label}: {error}"));
        let second = run_case(case).unwrap_or_else(|error| panic!("{label}: {error}"));
        crate::support::assert_same_outcome(&first, &second, &label);
        // FR-564-AC-7: serializing each document twice gives equal bytes.
        let bytes = |outcome: &qsl_conformance::CaseOutcome| {
            serde_json::to_vec(&crate::support::written(outcome)).expect("an outcome is JSON")
        };
        assert_eq!(bytes(&first), bytes(&first), "{label}");
        assert_eq!(bytes(&first), bytes(&second), "{label}");
        assert_eq!(first.exit_code(), case.expected_exit, "{label}");
        let category = match first.report() {
            Report::Document(document) => document.category(),
            Report::Item(item) => item.category(),
        };
        assert_eq!(category, case.expected_category, "{label}");
        assert_eq!(category.exit_code(), case.expected_exit, "{label}");

        let written = crate::support::written(&first);
        assert_eq!(written["category"], category.as_str(), "{label}");
        if matches!(first.report(), Report::Document(_)) {
            let operation = match case.verb {
                Verb::Check => "check",
                Verb::Package => "package",
                Verb::Execute => "execute",
                Verb::Analyze => "analyze",
                Verb::Monitor => "monitor",
                Verb::Prove => panic!("a prove case is an item"),
            };
            assert_eq!(written["operation"], operation, "{label}");
        }
        let mut found = Vec::new();
        collect_source_identities(&written, &mut found);
        for (authority, identity) in &found {
            assert_eq!(authority, case.parameters.source_authority, "{label}");
            assert_eq!(identity, case.parameters.source_identity, "{label}");
        }
        if matches!(case.expected_exit, 20 | 21) && matches!(case.verb, Verb::Check) {
            assert!(!found.is_empty(), "{label}: a source-located diagnostic");
        }
    }
}

/// The case's authored source label survives a physical input rename.
#[trace("TC-1394")]
#[test]
fn source_labels_are_explicit_and_independent_of_file_names() {
    let mut case = crate::support::case_of(Verb::Check, OutcomeKind::Refusal);
    case.parameters.source_authority = "conformance-author";
    case.parameters.source_identity = "rules/check-refusal";
    let first = run_case(&case).expect("the refusal case runs");
    let CaseInput::Check(input) = &mut case.input else {
        panic!("a check case holds a check input");
    };
    input.unit.file_name = "renamed-input.native";
    let second = run_case(&case).expect("the renamed refusal case runs");
    assert_eq!(
        first, second,
        "physical path does not replace the source label"
    );
    let mut labels = Vec::new();
    collect_source_identities(&crate::support::written(&second), &mut labels);
    assert!(
        !labels.is_empty(),
        "the refusal carries a source diagnostic"
    );
    for (authority, identity) in labels {
        assert_eq!(authority, "conformance-author");
        assert_eq!(identity, "rules/check-refusal");
    }
}

/// FR-565-AC-2: a case with an input removed returns `Err` whose stage is
/// `CaseStage::Read`, and no outcome.
#[trace("TC-1394", "FR-565-AC-2")]
#[test]
fn a_case_with_a_removed_input_fails_at_the_read_stage() {
    let mut case = crate::support::case_of(Verb::Check, OutcomeKind::Success);
    let CaseInput::Check(input) = &mut case.input else {
        panic!("a check case holds a check input");
    };
    input.unit.bytes = &[];
    let error = run_case(&case).expect_err("an empty unit is not a unit");
    assert_eq!(error.stage, CaseStage::Read);

    let mut case = crate::support::case_of(Verb::Check, OutcomeKind::Success);
    let CaseInput::Check(input) = &mut case.input else {
        panic!("a check case holds a check input");
    };
    static REMOVED: [Input; 1] = [Input {
        file_name: "removed.json",
        bytes: &[],
    }];
    input.packages = &REMOVED;
    let error = run_case(&case).expect_err("an empty package is not a package");
    assert_eq!(error.stage, CaseStage::Read);
}

/// Every `source` object (an `{authority, identity}` pair) in the document.
fn collect_source_identities(value: &serde_json::Value, found: &mut Vec<(String, String)>) {
    match value {
        serde_json::Value::Object(members) => {
            for (name, member) in members {
                if name == "source" && member.is_object() {
                    found.push((
                        member["authority"].as_str().unwrap_or_default().to_owned(),
                        member["identity"].as_str().unwrap_or_default().to_owned(),
                    ));
                }
                collect_source_identities(member, found);
            }
        }
        serde_json::Value::Array(members) => {
            for member in members {
                collect_source_identities(member, found);
            }
        }
        _ => {}
    }
}

/// Set in the child process of the FR-565-AC-3 test.
const CHILD: &str = "QSL_CONFORMANCE_EMPTY_WORKING_DIRECTORY";

/// The serialized outcome of every `Case`, one line each.
fn serialized_cases() -> Vec<String> {
    lifecycle_matrix()
        .iter()
        .filter_map(|cell| match cell {
            MatrixCell::Case(case) => {
                let outcome = run_case(case).expect("the case runs");
                Some(format!(
                    "CASE {:?}/{:?} {} {}",
                    case.verb,
                    case.kind,
                    outcome.exit_code(),
                    serde_json::to_string(&crate::support::written(&outcome))
                        .expect("an outcome is JSON")
                ))
            }
            _ => None,
        })
        .collect()
}

/// FR-565-AC-3: run with the process's working directory set to an empty
/// directory, every `Case` gives the outcome it gives here. The cases run in
/// a child process of this test binary, so no test in the parallel binary
/// changes its own working directory.
#[trace("TC-1394", "FR-565-AC-3")]
#[test]
fn every_case_gives_the_same_outcome_in_an_empty_working_directory() {
    if std::env::var_os(CHILD).is_some() {
        for line in serialized_cases() {
            println!("{line}");
        }
        return;
    }
    let empty = tempfile::tempdir().expect("a scratch directory");
    let output = std::process::Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            "--exact",
            "matrix_shape::every_case_gives_the_same_outcome_in_an_empty_working_directory",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD, "1")
        .current_dir(empty.path())
        .output()
        .expect("the child test binary runs");
    assert!(output.status.success(), "{output:?}");
    let child: Vec<String> = String::from_utf8(output.stdout)
        .expect("UTF-8 output")
        .lines()
        .filter_map(|line| line.find("CASE ").map(|start| line[start..].to_owned()))
        .collect();
    assert_eq!(child, serialized_cases());
}
