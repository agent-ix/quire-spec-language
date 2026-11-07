// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-513-AC-15 (TC-1308 row 25): `quire matrix` backs every retained
//! protocol-artifact criterion with a tagged test, and deleting the module
//! that holds the only tests of a criterion leaves that criterion untagged.
//!
//! The test runs the installed `quire matrix` over a fixture tree built from
//! this repository's own requirements and tag-bearing tests, and reads the
//! status the CLI computes for each criterion. It never computes or edits a
//! matrix itself.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use ix_trace_rs::trace;

/// The criteria FR-513-AC-15 retains.
const RETAINED: [&str; 9] = [
    "FR-050-AC-2",
    "FR-050-AC-3",
    "FR-054-AC-1",
    "FR-054-AC-2",
    "FR-054-AC-3",
    "FR-054-AC-4",
    "FR-049-AC-2",
    "FR-049-AC-5",
    "FR-513-AC-17",
];

/// The requirement documents of the retained criteria.
const REQUIREMENTS: [&str; 4] = ["FR-049-", "FR-050-", "FR-054-", "FR-513-"];

/// The tests that back the retained criteria.
const TESTS: [&str; 4] = [
    "assessment_artifact_kinds.rs",
    "compiled_protocol_v2.rs",
    "protocol_artifact_versions.rs",
    "v2_refusal_code_bridge.rs",
];

fn copy_fixture(root: &Path) {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
    let functional = root.join("spec/functional");
    let tests = root.join("tests/it");
    std::fs::create_dir_all(&functional).expect("fixture spec dir");
    std::fs::create_dir_all(&tests).expect("fixture tests dir");
    for entry in std::fs::read_dir(repo.join("spec/functional")).expect("repo spec dir") {
        let path = entry.expect("spec entry").path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if REQUIREMENTS.iter().any(|prefix| name.starts_with(prefix)) {
            std::fs::copy(&path, functional.join(name)).expect("copy requirement");
        }
    }
    for test in TESTS {
        std::fs::copy(repo.join("tests/it").join(test), tests.join(test)).expect("copy test");
    }
}

/// The status `quire matrix` computes for each criterion of the fixture.
fn matrix(root: &Path) -> BTreeMap<String, String> {
    let output = Command::new("quire")
        .args(["matrix", "--format", "tsv", "--scope"])
        .arg(root)
        .output()
        .expect("native quire is required on PATH");
    assert!(
        output.status.success(),
        "quire matrix failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("tsv output is text")
        .lines()
        .filter_map(|line| {
            let mut columns = line.split('\t');
            let (_document, criterion, status) =
                (columns.next()?, columns.next()?, columns.next()?);
            Some((criterion.to_owned(), status.to_owned()))
        })
        .collect()
}

fn untagged(matrix: &BTreeMap<String, String>) -> Vec<&str> {
    RETAINED
        .iter()
        .copied()
        .filter(|criterion| matrix.get(*criterion).map(String::as_str) != Some("tagged"))
        .collect()
}

#[trace("TC-1308", "FR-513-AC-15")]
#[test]
fn deleting_a_retained_module_without_a_successor_test_leaves_its_criteria_untagged() {
    let fixture = tempfile::tempdir().expect("fixture dir");
    let root = fixture.path();
    copy_fixture(root);

    let base = matrix(root);
    assert_eq!(
        untagged(&base),
        Vec::<&str>::new(),
        "every retained criterion is tagged"
    );

    // Each deletion removes the only tests of its criteria.
    for (deleted, expected) in [
        ("v2_refusal_code_bridge.rs", vec!["FR-513-AC-17"]),
        (
            "protocol_artifact_versions.rs",
            vec!["FR-054-AC-1", "FR-054-AC-2", "FR-054-AC-3", "FR-054-AC-4"],
        ),
        (
            "assessment_artifact_kinds.rs",
            vec!["FR-049-AC-2", "FR-049-AC-5"],
        ),
    ] {
        let kept = root.join("tests/it").join(deleted);
        let held = std::fs::read(&kept).expect("fixture test");
        std::fs::remove_file(&kept).expect("delete the module's test");
        assert_eq!(
            untagged(&matrix(root)),
            expected,
            "after deleting {deleted}"
        );
        std::fs::write(&kept, held).expect("restore the test");
    }
}
