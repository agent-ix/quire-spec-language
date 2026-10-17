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

use crate::error::{Error, Result};
use crate::Outcome;

pub mod compare;
pub mod report;
pub mod versioning;

const USAGE: &str = "usage: cargo xtask refinement versioning <corpus>";

/// `cargo xtask refinement versioning <corpus>`: run the gate and return its
/// report with the exit code of its verdict (FR-344).
#[qsl_attrs::string_edge]
pub fn run(workspace_root: &Path, operands: &[OsString]) -> Result<Outcome> {
    let [gate, corpus] = operands else {
        return Err(Error::Usage(USAGE));
    };
    if gate.to_str() != Some("versioning") {
        return Err(Error::Usage(USAGE));
    }
    let report = versioning::run(Path::new(corpus), workspace_root);
    let stdout = report
        .to_json()
        .map_err(|source| Error::RefinementReport { source })?;
    Ok(Outcome {
        stdout,
        code: report.verdict.exit_code(),
    })
}
