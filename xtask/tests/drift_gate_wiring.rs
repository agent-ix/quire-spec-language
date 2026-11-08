// SPDX-License-Identifier: AGPL-3.0-or-later
//! ARCH-51 (QSL-13): every architecture-drift gate has a local command and
//! runs in `make ci`. The real `Makefile` lists each gate in `CI_CHECKS` and
//! defines its target with the gate's command as the recipe, so dropping a
//! gate from the aggregate, or repointing its recipe, fails here.

use std::fs;
use std::path::Path;

use ix_trace_rs::trace;

/// `(make target, recipe line)` for each drift gate that gates `make ci`.
const DRIFT_GATES: [(&str, &str); 5] = [
    (
        "checked-input",
        "\tcargo run --package xtask -- checked-input",
    ),
    ("string-edge", "\tcargo run --package xtask -- string-edge"),
    ("route-lint", "\tcargo run --package xtask -- route-lint"),
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
