// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-017 RF-2: the refinement gates of the xtask repository-gate family
//! (FR-344 "Invocation and exit"). `cargo xtask refinement versioning
//! <corpus>` checks that a superseding revision of a specification admits
//! every case its prior revision admitted (FR-340); `layering <corpus>` runs
//! the FR-345 gate over the linked production registry (FR-489).
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
pub mod layering;
pub mod report;
pub mod versioning;

const USAGE: &str = "usage: cargo xtask refinement versioning <corpus> | layering <corpus>";

/// `cargo xtask refinement <gate> <corpus>`: run the selected gate and return
/// its report with the exit code of its verdict (FR-344).
#[ix_trace_rs::string_edge]
pub fn run(workspace_root: &Path, operands: &[OsString]) -> Result<Outcome> {
    let [gate, corpus] = operands else {
        return Err(Error::Usage(USAGE));
    };
    let path = Path::new(corpus);
    let (stdout, code) = match gate.to_str() {
        Some("versioning") => {
            let report = versioning::run(path, workspace_root);
            let stdout = report
                .to_json()
                .map_err(|source| Error::RefinementReport { source })?;
            (stdout, report.verdict.exit_code())
        }
        Some("layering") => {
            let outcome = run_layering(path, layering::RegistrySnapshot::production())?;
            (outcome.stdout, outcome.code)
        }
        _ => return Err(Error::Usage(USAGE)),
    };
    Ok(Outcome { stdout, code })
}

fn run_layering(
    corpus: &Path,
    registry: std::result::Result<layering::RegistrySnapshot, layering::RegistryBuildError>,
) -> Result<Outcome> {
    let registry = registry.map_err(|source| Error::RefinementRegistryBuild { source })?;
    let report = layering::run(corpus, &registry);
    let stdout = report
        .to_json()
        .map_err(|source| Error::RefinementReport { source })?;
    Ok(Outcome {
        stdout,
        code: report.exit,
    })
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_foundation::BackendId;
    use qsl_route::{descriptor_from_manifest, Candidate, ManifestDigest, ProviderOrigin};

    use super::*;

    /// FR-489-AC-6: a typed production registry failure is a FR-344 tool
    /// failure (exit 30), not the expected unsupported result (exit 21).
    #[trace("TC-1272", "FR-489-AC-6")]
    #[test]
    fn a_registry_build_failure_maps_to_tool_failure_exit_30() {
        let refusal = descriptor_from_manifest(
            Candidate::new(BackendId::new("bad-provider")),
            ProviderOrigin::Linked,
            ManifestDigest::from_digest([0; 32]),
            [(Some("unknown-kind"), Some("bounded"))],
            qsl_route::ManifestMembers::default(),
        )
        .expect_err("unknown capability label is refused");
        let error = layering::RegistryBuildError::DescriptorSource(refusal);

        let result = run_layering(Path::new("unused"), Err(error));
        let Err(error) = result else {
            panic!("the registry build error must propagate");
        };

        assert_eq!(error.exit_code(), 30,);
    }
}
