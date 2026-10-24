// SPDX-License-Identifier: AGPL-3.0-or-later
//! Workspace `xtask`: `cargo xtask seam-probe`, `cargo xtask string-edge`,
//! `cargo xtask route-lint`, `cargo xtask checked-input` and `cargo xtask
//! test-suites`, the gates
//! `make ci` runs standalone (the `Makefile` targets of the same names), and
//! `cargo xtask canonical-types`, run on demand, and the local `conformance`
//! vector gate.
//! `refinement` holds the refinement gates (`cargo xtask refinement versioning`).
//! `import_graph`, `definition_scan` and `typestate_scan` are plain library
//! modules backing their own `#[cfg(test)]` suites.
#![forbid(unsafe_code)]

pub mod canonical_types;
pub mod checked_input;
pub mod conformance;
pub mod definition_scan;
pub mod error;
pub mod feature_scope;
pub mod import_graph;
pub mod refinement;
pub mod route_lint;
pub mod seam_probe;
pub mod string_edge;
pub mod test_suites;
pub mod typestate_scan;

/// Test helpers shared by the unit tests.
#[cfg(test)]
mod test_support {
    /// A temporary directory whose path is already canonical. macOS reports
    /// `$TMPDIR` under the `/var` symlink while the tools under test print
    /// the resolved `/private/var` path, so comparing paths needs the
    /// resolved one from the start.
    pub(crate) fn tempdir() -> tempfile::TempDir {
        let base = std::env::temp_dir()
            .canonicalize()
            .expect("canonical temp dir");
        tempfile::tempdir_in(base).expect("tempdir")
    }
}

#[cfg(test)]
mod ci_clean_build_tests;

#[cfg(test)]
mod ci_feature_lane_tests;

#[cfg(test)]
mod ci_spec_validation_tests;
pub use error::{Error, Result};

/// What a command produced: the text it writes to stdout and the exit code
/// it ends with. `main`'s one exit function turns it into the process exit.
pub struct Outcome {
    /// The text written to stdout.
    pub stdout: String,
    /// The process exit code.
    pub code: u8,
}

impl Outcome {
    /// A gate that passed: its summary and exit code 0.
    pub fn success(stdout: String) -> Self {
        Self { stdout, code: 0 }
    }
}
