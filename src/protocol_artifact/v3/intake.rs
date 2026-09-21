// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-054: bounded independent admission of version-3 packages.

use super::{
    wire, AdmittedPackage, Expected, ExpectedActivation, InventorySide, MappingCause, MappingField,
    Refusal, MEDIA, SCHEMA, WIRE,
};
use crate::protocol_artifact::{self as artifact, v2, work::Work};
use crate::protocol_artifact::{Candidate, Dimension, Error, Invalid, Limits, Report};
use crate::ByteDigest;

fn headers(
    package: &wire::Package,
    expected: &artifact::Expected<'_>,
    work: &mut Work,
) -> Result<(), Error> {
    let inherited = &package.inherited;
    for matches in [
        inherited.wire == WIRE,
        inherited.media == MEDIA,
        inherited.schema == SCHEMA,
        inherited.package_type == artifact::PACKAGE_TYPE,
        inherited.encoding == artifact::ENCODING,
        inherited.numeric == artifact::NUMERIC_PROFILE,
        expected.artifact.kind == artifact::wire::ArtifactKind::LinkedPackage,
        expected.artifact.wire.identity == "quire.compiled-protocol",
        expected.artifact.wire.version == "3",
    ] {
        work.visit()?;
        if !matches {
            return Err(Error::V3(Refusal::Header));
        }
    }
    artifact::intake::reference(expected.artifact)?;
    artifact::intake::same_reference(&inherited.contract, expected.contract, work)?;
    artifact::intake::same_reference(&inherited.baseline, expected.baseline, work)?;
    // The producer's binary is Producer identity, checked here as a
    // structurally valid reference equal to the caller's own expectation --
    // it is never one of the byte-sealed `dependencies`, so it is not
    // looked up there.
    artifact::intake::reference(&expected.producer.binary)?;
    artifact::intake::same_reference(&inherited.producer.binary, &expected.producer.binary, work)?;
    artifact::intake::name(&inherited.producer.implementation)?;
    artifact::intake::revision(&inherited.producer.revision)?;
    artifact::intake::name(&inherited.language.edition)?;
    if inherited.producer != *expected.producer
        || inherited.language != *expected.language
        || inherited.language.identity != "ix:native"
    {
        return Err(Error::Invalid(Invalid::Selection));
    }
    for selected in [&inherited.contract, &inherited.baseline] {
        artifact::intake::find_reference(
            &inherited.dependencies,
            selected,
            |value| &value.artifact,
            work,
        )?;
    }
    Ok(())
}

fn mapping_refusal(side: InventorySide, cause: MappingCause) -> Error {
    Error::V3(Refusal::Mapping { side, cause })
}

fn event_control(package: &wire::Package, handle: artifact::wire::Handle) -> Result<(), Error> {
    let declaration = package
        .inherited
        .declarations
        .get(usize::try_from(handle.declaration).unwrap_or(usize::MAX))
        .ok_or(Error::V3(Refusal::Control))?;
    let artifact::wire::Body::Protocol { controls, .. } = &declaration.body else {
        return Err(Error::V3(Refusal::Control));
    };
    let control = controls
        .get(usize::try_from(handle.index).unwrap_or(usize::MAX))
        .ok_or(Error::V3(Refusal::Control))?;
    if matches!(
        control.operation,
        artifact::wire::ControlOperation::Event { .. }
    ) {
        Ok(())
    } else {
        Err(Error::V3(Refusal::Control))
    }
}

fn temporal_declaration(package: &wire::Package, index: u32) -> Result<(), Error> {
    let declaration = package
        .inherited
        .declarations
        .get(usize::try_from(index).unwrap_or(usize::MAX))
        .ok_or(Error::V3(Refusal::TemporalDeclaration))?;
    if matches!(declaration.body, artifact::wire::Body::Temporal { .. }) {
        Ok(())
    } else {
        Err(Error::V3(Refusal::TemporalDeclaration))
    }
}

pub(crate) fn validate_mappings(
    package: &wire::Package,
    expected: &[ExpectedActivation],
    work: &mut Work,
) -> Result<(), Error> {
    work.charge(Dimension::Entries, package.activation_mappings.len())?;
    work.charge(Dimension::Entries, expected.len())?;
    let mut seen = Vec::new();
    seen.try_reserve_exact(expected.len())
        .map_err(|_| Error::Allocation)?;
    seen.resize(expected.len(), false);
    for pair in package.activation_mappings.windows(2) {
        work.visit()?;
        if pair[0].control == pair[1].control {
            return Err(mapping_refusal(
                InventorySide::Offer,
                MappingCause::Duplicate,
            ));
        }
        if (pair[0].control.declaration, pair[0].control.index)
            > (pair[1].control.declaration, pair[1].control.index)
        {
            return Err(Error::V3(Refusal::OfferOrder));
        }
    }
    if package.activation_mappings.len() < expected.len() {
        return Err(mapping_refusal(InventorySide::Offer, MappingCause::Missing));
    }
    if package.activation_mappings.len() > expected.len() {
        return Err(mapping_refusal(InventorySide::Offer, MappingCause::Surplus));
    }
    for mapping in &package.activation_mappings {
        work.visit()?;
        event_control(package, mapping.control.clone())?;
        temporal_declaration(package, mapping.temporal_declaration)?;
        let mut found = None;
        for (index, selected) in expected.iter().enumerate() {
            work.visit()?;
            if selected.control == mapping.control && found.replace(index).is_some() {
                return Err(mapping_refusal(
                    InventorySide::Expected,
                    MappingCause::Duplicate,
                ));
            }
        }
        let Some(index) = found else {
            return Err(mapping_refusal(
                InventorySide::Expected,
                MappingCause::Missing,
            ));
        };
        if std::mem::replace(&mut seen[index], true) {
            return Err(mapping_refusal(
                InventorySide::Expected,
                MappingCause::Duplicate,
            ));
        }
        let selected = &expected[index];
        if selected.control != mapping.control {
            return Err(Error::V3(Refusal::Expected(MappingField::Control)));
        }
        if selected.temporal_declaration != mapping.temporal_declaration {
            return Err(Error::V3(Refusal::Expected(
                MappingField::TemporalDeclaration,
            )));
        }
    }
    if seen.contains(&false) {
        return Err(mapping_refusal(
            InventorySide::Expected,
            MappingCause::Missing,
        ));
    }
    Ok(())
}

fn v2_package(package: &wire::Package) -> v2::wire::Package {
    v2::wire::Package {
        inherited: package.inherited.clone(),
        temporal_bindings: package.temporal_bindings.clone(),
    }
}

fn numbers(package: &wire::Package, work: &mut Work) -> Result<(), Error> {
    let v2 = v2_package(package);
    artifact::validate::numbers(&package.inherited, work)?;
    for binding in &v2.temporal_bindings {
        work.visit()?;
        let definition = package
            .inherited
            .definitions
            .get(usize::try_from(binding.definition).unwrap_or(usize::MAX))
            .ok_or(Error::Invalid(Invalid::Reference))?;
        v2::intake::clock(&definition.identity, &binding.clock, work)?;
    }
    Ok(())
}

/// Encode a typed untrusted v3 candidate without granting admission.
pub fn encode_candidate(package: &wire::Package, limits: Limits) -> Report<Candidate> {
    let mut work = Work::new(limits);
    let result = (|| {
        numbers(package, &mut work)?;
        work.locus = None;
        artifact::encoding::candidate(package, &mut work)
    })();
    artifact::report(work, result)
}

/// Read exact bounded v3 bytes against independently selected inputs.
pub fn read(bytes: &[u8], expected: &Expected<'_>, limits: Limits) -> Report<AdmittedPackage> {
    let mut work = Work::new(limits);
    let result = (|| {
        work.charge(Dimension::PayloadBytes, bytes.len())?;
        work.bytes(bytes.len())?;
        let digest = ByteDigest::of(bytes);
        if digest != expected.inherited.inherited.artifact.digest {
            return Err(Error::Invalid(Invalid::Seal));
        }
        let package: wire::Package = artifact::decode::bounded(bytes, &mut work)?;
        headers(&package, &expected.inherited.inherited, &mut work)?;
        v2::intake::validate(
            &v2_package(&package),
            expected.inherited.temporal,
            &mut work,
        )?;
        validate_mappings(&package, expected.activations, &mut work)?;
        let supplied = artifact::intake::selected(
            &package.inherited,
            &expected.inherited.inherited,
            &mut work,
        )?;
        artifact::intake::sources(&package.inherited, &expected.inherited.inherited, &mut work)?;
        artifact::intake::definitions(&package.inherited, &supplied, &mut work)?;
        artifact::validate::package(&package.inherited, &mut work)?;
        let model_schema = artifact::models::validate(
            &package.inherited,
            expected.inherited.inherited.models,
            expected.inherited.inherited.dependencies,
            &mut work,
        )?;
        let canonical = artifact::encoding::bytes(&package, &mut work)?;
        work.bytes(bytes.len())?;
        work.bytes(canonical.len())?;
        if canonical != bytes {
            return Err(Error::Invalid(Invalid::Canonical));
        }
        Ok(AdmittedPackage {
            package,
            digest,
            artifact: Some(artifact::intake::retained_reference(
                expected.inherited.inherited.artifact,
            )?),
            model_schema,
        })
    })();
    artifact::report(work, result)
}
