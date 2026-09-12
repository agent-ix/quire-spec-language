// SPDX-License-Identifier: AGPL-3.0-only
//! FR-050: source-authorized version-2 temporal selection and emission.

use super::Selections;
use crate::checking::composed::proofs::ProofReport;
use crate::protocol_artifact::{self as artifact, v2, wire as w};
use crate::protocol_artifact::{Candidate, Dimension, Error, Invalid, Limits, Report};

fn binding_refusal(cause: v2::BindingCause) -> Error {
    Error::V2(v2::Refusal::Binding {
        side: v2::InventorySide::Producer,
        cause,
    })
}

/// One independently selected temporal definition and clock configuration.
#[derive(Clone, Copy, Debug)]
pub struct TemporalSelection<'a> {
    /// Exact source artifact containing the declaration.
    pub source: &'a w::ArtifactRef,
    /// Exact declaration byte span in the original source.
    pub span: &'a w::Span,
    /// Exact registered definition identity.
    pub definition_identity: &'a str,
    /// Exact registered definition revision.
    pub definition_revision: &'a w::Revision,
    /// Exact original definition artifact.
    pub definition_artifact: &'a w::ArtifactRef,
    /// Explicit clock configuration selected for this declaration.
    pub clock: &'a v2::wire::ClockConfiguration,
}

/// Canonical bytes and the constructor-private admitted view they came from.
#[derive(Debug)]
pub struct AdmissionV2 {
    candidate: Candidate,
    admitted: v2::AdmittedPackage,
}

impl AdmissionV2 {
    /// Complete canonical version-2 bytes.
    pub fn bytes(&self) -> &[u8] {
        self.candidate.bytes()
    }

    /// Raw-byte digest of the complete canonical version-2 payload.
    pub fn digest(&self) -> crate::ByteDigest {
        self.candidate.digest()
    }

    /// Strict admitted view backed by the same source-authorized package.
    pub fn admitted(&self) -> &v2::AdmittedPackage {
        &self.admitted
    }
}

fn bindings(
    package: &w::Package,
    selections: &[TemporalSelection<'_>],
    work: &mut artifact::work::Work,
) -> Result<Vec<v2::wire::TemporalBinding>, Error> {
    let temporal_len = package
        .declarations
        .iter()
        .filter(|declaration| matches!(declaration.body, w::Body::Temporal { .. }))
        .count();
    work.charge(Dimension::Entries, temporal_len)?;
    work.charge(Dimension::Entries, selections.len())?;
    if selections.len() < temporal_len {
        return Err(binding_refusal(v2::BindingCause::Missing));
    }
    if selections.len() > temporal_len {
        return Err(binding_refusal(v2::BindingCause::Surplus));
    }
    let mut result = Vec::new();
    result
        .try_reserve_exact(temporal_len)
        .map_err(|_| Error::Allocation)?;
    for (declaration_index, declaration) in package
        .declarations
        .iter()
        .enumerate()
        .filter(|(_, declaration)| matches!(declaration.body, w::Body::Temporal { .. }))
    {
        work.visit()?;
        let emitted_source = package
            .sources
            .get(usize::try_from(declaration.locus.source).unwrap_or(usize::MAX))
            .ok_or(Error::Invalid(Invalid::Locus))?;
        let mut selected = None;
        let mut foreign_owner = false;
        for candidate in selections {
            work.visit()?;
            if candidate.span == &declaration.locus.span {
                if candidate.source == &emitted_source.artifact {
                    if selected.replace(candidate).is_some() {
                        return Err(binding_refusal(v2::BindingCause::Duplicate));
                    }
                } else {
                    foreign_owner = true;
                }
            }
        }
        let selected = selected.ok_or_else(|| {
            binding_refusal(if foreign_owner {
                v2::BindingCause::ForeignOwner
            } else {
                v2::BindingCause::Missing
            })
        })?;
        let definition = package
            .definitions
            .get(usize::try_from(declaration.profile).unwrap_or(usize::MAX))
            .ok_or(Error::Invalid(Invalid::Reference))?;
        let dependency = package
            .dependencies
            .get(usize::try_from(definition.artifact).unwrap_or(usize::MAX))
            .ok_or(Error::Invalid(Invalid::Reference))?;
        work.bytes(definition.identity.len())?;
        work.bytes(selected.definition_identity.len())?;
        if definition.identity != selected.definition_identity {
            return Err(Error::V2(v2::Refusal::Definition(
                v2::DefinitionField::Identity,
            )));
        }
        if definition.revision != *selected.definition_revision {
            return Err(Error::V2(v2::Refusal::Definition(
                v2::DefinitionField::Revision,
            )));
        }
        if let Some(field) =
            v2::refusal::artifact_field(&dependency.artifact, selected.definition_artifact)
        {
            return Err(Error::V2(v2::Refusal::Definition(
                v2::DefinitionField::Artifact(field),
            )));
        }
        v2::intake::clock(&definition.identity, selected.clock, work)?;
        work.charge(Dimension::Entries, 1)?;
        result.push(v2::wire::TemporalBinding {
            declaration: u32::try_from(declaration_index)
                .map_err(|_| Error::Invalid(Invalid::StructuralInteger))?,
            definition: declaration.profile,
            clock: selected.clock.clone(),
        });
    }
    Ok(result)
}

/// Compile, bind and canonically emit one strict version-2 package.
pub fn admit_v2(
    proofs: &ProofReport<'_, '_, '_>,
    selections: &Selections<'_>,
    temporal: &[TemporalSelection<'_>],
    limits: Limits,
) -> Report<AdmissionV2> {
    let mut work = artifact::work::Work::new(limits);
    let result = (|| {
        let mut inherited = super::metadata::lower(proofs, selections, &mut work)?;
        inherited.wire = v2::WIRE.into();
        inherited.media = v2::MEDIA.into();
        inherited.schema = v2::SCHEMA.into();
        artifact::validate::package(&inherited, &mut work)?;
        let temporal_bindings = bindings(&inherited, temporal, &mut work)?;
        let package = v2::wire::Package {
            inherited,
            temporal_bindings,
        };
        let candidate = artifact::encoding::candidate(&package, &mut work)?;
        let admitted = v2::AdmittedPackage {
            digest: candidate.digest(),
            package,
        };
        Ok(AdmissionV2 {
            candidate,
            admitted,
        })
    })();
    artifact::report(work, result)
}
