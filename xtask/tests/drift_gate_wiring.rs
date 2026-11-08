// SPDX-License-Identifier: AGPL-3.0-or-later
//! ARCH-51 (QSL-13): every architecture-drift gate has a local command and
//! runs in `make ci`. The real `Makefile` lists each gate in `CI_CHECKS` and
//! defines its target with the gate's command as the recipe, so dropping a
//! gate from the aggregate, or repointing its recipe, fails here.

use std::fs;
use std::path::Path;

use ix_trace_rs::trace;

/// `(make target, recipe line)` for each drift gate that gates `make ci`.
const DRIFT_GATES: [(&str, &str); 6] = [
    (
        "checked-input",
        "\tcargo run --package xtask -- checked-input",
    ),
    ("string-edge", "\tcargo run --package xtask -- string-edge"),
    ("route-lint", "\tcargo run --package xtask -- route-lint"),
    (
        "canonical-types",
        "\tcargo run --locked --package xtask -- canonical-types",
    ),
    (
        "arch-lint-api-surface-qsl",
        "\tcargo run --locked -p arch-lint -- api-surface --qsl . --qsl-only",
    ),
    (
        "arch-lint-qualified-core",
        "\tcargo run --locked -p arch-lint -- qualified-core --qsl .",
    ),
];

#[trace("TC-745", "FR-270-AC-4")]
#[test]
fn every_drift_gate_is_a_make_ci_prerequisite_with_its_command() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives one level below the workspace root");
    let makefile = fs::read_to_string(root.join("Makefile")).expect("the Makefile reads");
    let ci = makefile
        .lines()
        .find_map(|line| line.strip_prefix("CI_CHECKS :="))
        .expect("the Makefile has the CI_CHECKS aggregate list");
    let lines: Vec<&str> = makefile.lines().collect();
    for (target, recipe) in DRIFT_GATES {
        assert!(
            ci.split_whitespace().any(|name| name == target),
            "CI_CHECKS must list {target}: {ci}"
        );
        let position = lines
            .iter()
            .position(|line| *line == format!("{target}:"))
            .unwrap_or_else(|| panic!("the Makefile must define `{target}:`"));
        assert_eq!(
            lines.get(position + 1),
            Some(&recipe),
            "the `{target}` recipe must run the gate command"
        );
    }
}

/// ADR-032 section 3: the gate register has one owner row per `CI_CHECKS`
/// drift gate and no row for anything else.
#[trace("TC-745", "FR-270-AC-4")]
#[test]
fn the_adr_032_gate_register_matches_the_drift_gates() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives one level below the workspace root");
    let decisions = root.join("spec/decisions");
    let adr = fs::read_dir(&decisions)
        .expect("the decisions directory reads")
        .map(|entry| entry.expect("a directory entry reads").path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("ADR-032-"))
        })
        .expect("ADR-032 exists");
    let text = fs::read_to_string(adr).expect("ADR-032 reads");
    let section = text
        .split("### 3. Gate register")
        .nth(1)
        .expect("ADR-032 has the gate register section")
        .split("\n## ")
        .next()
        .expect("the section has a body");
    let mut registered: Vec<(&str, &str)> = section
        .lines()
        .filter(|line| line.starts_with('|'))
        .skip(2)
        .map(|line| {
            let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
            assert_eq!(cells.len(), 2, "a register row is gate | owner: {line}");
            (cells[0], cells[1])
        })
        .collect();
    registered.sort_unstable();
    let mut expected: Vec<&str> = DRIFT_GATES.iter().map(|(target, _)| *target).collect();
    expected.sort_unstable();
    let names: Vec<&str> = registered.iter().map(|(gate, _)| *gate).collect();
    assert_eq!(
        names, expected,
        "the register rows are exactly the drift gates"
    );
    for (gate, owner) in registered {
        assert!(!owner.is_empty(), "the register names an owner for {gate}");
    }
}
