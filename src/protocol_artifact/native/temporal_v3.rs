// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-054: source-authorized version-3 activation mapping emission.

use super::temporal_v2::AdmissionV2;
use crate::protocol_artifact::wire::Handle;
use crate::protocol_artifact::{self as artifact, v3, Candidate, Limits, Report};

/// One exact authored event-control to temporal-declaration selection.
///
/// The selection is accepted only as a delta over an already source-authorized
/// `/2` admission. It cannot provide an independent wire-emission authority.
#[derive(Clone, Copy, Debug)]
pub struct ActivationSelection<'a> {
    /// Exact declaration-local event-control handle from the admitted graph.
    pub control: &'a Handle,
    /// Exact temporal declaration selected by that authored control relation.
    pub temporal_declaration: u32,
}

/// Canonical `/3` bytes and their constructor-private admitted source view.
#[derive(Debug)]
pub struct AdmissionV3 {
    candidate: Candidate,
    admitted: v3::AdmittedPackage,
}

impl AdmissionV3 {
    /// Complete canonical version-3 bytes.
    pub fn bytes(&self) -> &[u8] {
        self.candidate.bytes()
    }

    /// Raw-byte digest of the complete canonical version-3 payload.
    pub fn digest(&self) -> crate::ByteDigest {
        self.candidate.digest()
    }

    /// Strict source-authorized admitted view backed by these exact bytes.
    pub fn admitted(&self) -> &v3::AdmittedPackage {
        &self.admitted
    }
}

/// Append exact authored activation selections to a source-authorized `/2` package.
///
/// This preserves every `/2` member while changing only the version headers and
/// appending the canonical `/3` mapping delta. Invalid, duplicate, unordered,
/// non-event, or non-temporal selections refuse before any candidate is emitted.
pub fn admit_v3(
    inherited: &AdmissionV2,
    activations: &[ActivationSelection<'_>],
    limits: Limits,
) -> Report<AdmissionV3> {
    let mut work = artifact::work::Work::new(limits);
    let result = (|| {
        let inherited_v2 = inherited.admitted();
        let mut package_v1 = inherited_v2.package().inherited.clone();
        package_v1.wire = v3::WIRE.into();
        package_v1.media = v3::MEDIA.into();
        package_v1.schema = v3::SCHEMA.into();
        let expected: Vec<_> = activations
            .iter()
            .map(|selection| v3::ExpectedActivation {
                control: selection.control.clone(),
                temporal_declaration: selection.temporal_declaration,
            })
            .collect();
        let package = v3::wire::Package {
            inherited: package_v1,
            temporal_bindings: inherited_v2.package().temporal_bindings.clone(),
            activation_mappings: expected
                .iter()
                .map(|selection| v3::wire::ActivationMapping {
                    control: selection.control.clone(),
                    temporal_declaration: selection.temporal_declaration,
                })
                .collect(),
        };
        v3::intake::validate_mappings(&package, &expected, &mut work)?;
        work.locus = None;
        let candidate = artifact::encoding::candidate(&package, &mut work)?;
        let admitted = v3::AdmittedPackage {
            package,
            digest: candidate.digest(),
            artifact: None,
            model_schema: inherited_v2.model_schema.clone(),
        };
        Ok(AdmissionV3 {
            candidate,
            admitted,
        })
    })();
    artifact::report(work, result)
}
