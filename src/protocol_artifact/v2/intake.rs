// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-050: bounded independent admission of version-2 packages.

use super::{
    refusal::artifact_field, wire, AdmittedPackage, BindingCause, BindingIndex, ClockField,
    DeclarationField, DefinitionField, Expected, ExpectedTemporal, HeaderField, InventorySide,
    Refusal, SelectionSide, MEDIA, SCHEMA, WIRE,
};
use crate::protocol_artifact::{self as artifact, work::Work};
use crate::protocol_artifact::{Candidate, Dimension, Error, Invalid, Limits, Report, Unsupported};
use crate::ByteDigest;

fn headers(
    package: &wire::Package,
    expected: &artifact::Expected<'_>,
    work: &mut Work,
) -> Result<(), Error> {
    let inherited = &package.inherited;
    for (matches, field) in [
        (inherited.wire == WIRE, HeaderField::Wire),
        (inherited.media == MEDIA, HeaderField::Media),
        (inherited.schema == SCHEMA, HeaderField::Schema),
        (
            inherited.package_type == artifact::PACKAGE_TYPE,
            HeaderField::PackageType,
        ),
        (
            inherited.encoding == artifact::ENCODING,
            HeaderField::Encoding,
        ),
        (
            inherited.numeric == artifact::NUMERIC_PROFILE,
            HeaderField::Numeric,
        ),
    ] {
        if !matches {
            return Err(Error::V2(Refusal::Header(field)));
        }
    }
    artifact::intake::reference(expected.artifact)?;
    for (matches, field) in [
        (
            expected.artifact.kind == artifact::wire::ArtifactKind::LinkedPackage,
            HeaderField::ArtifactKind,
        ),
        (
            expected.artifact.wire.identity == "quire.compiled-protocol",
            HeaderField::ArtifactWire,
        ),
        (
            expected.artifact.wire.version == "2",
            HeaderField::ArtifactVersion,
        ),
    ] {
        if !matches {
            return Err(Error::V2(Refusal::Header(field)));
        }
    }
    artifact::intake::same_reference(&inherited.contract, expected.contract, work)?;
    artifact::intake::same_reference(&inherited.baseline, expected.baseline, work)?;
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
    for selected in [
        &inherited.contract,
        &inherited.baseline,
        &inherited.producer.binary,
    ] {
        artifact::intake::find_reference(
            &inherited.dependencies,
            selected,
            |value| &value.artifact,
            work,
        )?;
    }
    Ok(())
}

fn numeric_bytes(number: &artifact::NumberWire, work: &mut Work) -> Result<(), Error> {
    work.visit()?;
    match number {
        artifact::NumberWire::Integer { decimal } => work.bytes(decimal.len()),
        artifact::NumberWire::Rational {
            numerator,
            denominator,
        } => {
            work.bytes(numerator.len())?;
            work.bytes(denominator.len())
        }
    }
}

pub(crate) fn reference_bytes(reference: &artifact::wire::ArtifactRef) -> usize {
    reference
        .ref_version
        .len()
        .saturating_add(reference.kind.as_str().len())
        .saturating_add(reference.authority.len())
        .saturating_add(reference.identity.len())
        .saturating_add(reference.revision.namespace.len())
        .saturating_add(reference.revision.value.len())
        .saturating_add(71)
        .saturating_add(reference.wire.identity.len())
        .saturating_add(reference.wire.version.len())
}

pub(crate) fn charge_reference_pair(
    actual: &artifact::wire::ArtifactRef,
    expected: &artifact::wire::ArtifactRef,
    work: &mut Work,
) -> Result<(), Error> {
    work.bytes(reference_bytes(actual).saturating_add(reference_bytes(expected)))
}

fn revision_bytes(revision: &artifact::wire::Revision) -> usize {
    revision
        .namespace
        .len()
        .saturating_add(revision.value.len())
}

fn requirement_bytes(requirement: &artifact::wire::Requirement) -> usize {
    requirement
        .package
        .len()
        .saturating_add(requirement.identity.len())
        .saturating_add(revision_bytes(&requirement.revision))
}

fn execution_bytes(execution: &artifact::wire::Execution) -> usize {
    match execution {
        artifact::wire::Execution::Initialization { name }
        | artifact::wire::Execution::Handler { name } => name.len(),
        artifact::wire::Execution::Pre { .. } | artifact::wire::Execution::Post { .. } => 0,
    }
}

fn positive(number: &artifact::wire::Number, work: &mut Work) -> Result<(), Error> {
    numeric_bytes(&number.0, work)?;
    let positive = match number.checked()? {
        artifact::ProtocolNumber::Integer(value) => value.value() > 0,
        artifact::ProtocolNumber::Rational(value) => value.numerator() > 0,
    };
    if positive {
        Ok(())
    } else {
        Err(Error::Invalid(Invalid::NumericDomain))
    }
}

pub(in crate::protocol_artifact) fn clock(
    profile: &str,
    clock: &wire::ClockConfiguration,
    work: &mut Work,
) -> Result<(), Error> {
    match (profile, clock) {
        (
            crate::temporal::EVENT_POSITION,
            wire::ClockConfiguration::EventPosition { sequence_authority },
        ) => {
            work.bytes(sequence_authority.len())?;
            artifact::intake::name(sequence_authority)
                .map_err(|_| Error::V2(Refusal::Clock(ClockField::SequenceAuthority)))
        }
        (
            crate::temporal::FIXED_SAMPLE,
            wire::ClockConfiguration::FixedSample {
                epoch,
                period,
                unit,
            },
        ) => {
            numeric_bytes(&epoch.0, work)?;
            epoch.checked()?;
            positive(period, work)?;
            work.bytes(unit.len())?;
            artifact::intake::name(unit).map_err(|_| Error::V2(Refusal::Clock(ClockField::Unit)))
        }
        (
            crate::temporal::TIMESTAMPED_WINDOW,
            wire::ClockConfiguration::TimestampedEvent { timestamp_unit },
        ) => {
            work.bytes(timestamp_unit.len())?;
            artifact::intake::name(timestamp_unit)
                .map_err(|_| Error::V2(Refusal::Clock(ClockField::TimestampUnit)))
        }
        (
            crate::temporal::EVENT_POSITION
            | crate::temporal::FIXED_SAMPLE
            | crate::temporal::TIMESTAMPED_WINDOW,
            _,
        ) => Err(Error::V2(Refusal::Clock(ClockField::Alternative))),
        _ => Err(Error::Unsupported(Unsupported::Profile)),
    }
}

fn declaration_key<'a>(
    value: &'a ExpectedTemporal<'a>,
) -> (&'a artifact::wire::ArtifactRef, u32, u32) {
    (
        value.source,
        value.declaration.span.start,
        value.declaration.span.end,
    )
}

fn binding_refusal(side: InventorySide, cause: BindingCause) -> Error {
    Error::V2(Refusal::Binding { side, cause })
}

fn compare_declaration(
    declaration: &artifact::wire::Declaration,
    selected: &ExpectedTemporal<'_>,
    work: &mut Work,
) -> Result<(), Error> {
    work.bytes(
        requirement_bytes(&declaration.requirement)
            .saturating_add(requirement_bytes(selected.declaration.requirement))
            .saturating_add(execution_bytes(&declaration.execution))
            .saturating_add(execution_bytes(selected.declaration.execution)),
    )?;
    let field = if declaration.name != selected.declaration.name {
        Some(DeclarationField::Name)
    } else if declaration.requirement != *selected.declaration.requirement {
        Some(DeclarationField::Requirement)
    } else if declaration.clause != selected.declaration.clause {
        Some(DeclarationField::Clause)
    } else if declaration.execution != *selected.declaration.execution {
        Some(DeclarationField::Execution)
    } else {
        None
    };
    field.map_or(Ok(()), |field| Err(Error::V2(Refusal::Declaration(field))))
}

fn compare_clock(
    actual: &wire::ClockConfiguration,
    expected: &wire::ClockConfiguration,
) -> Result<(), Error> {
    use wire::ClockConfiguration as C;
    let field = match (actual, expected) {
        (
            C::EventPosition {
                sequence_authority: actual,
            },
            C::EventPosition {
                sequence_authority: expected,
            },
        ) if actual != expected => Some(ClockField::SequenceAuthority),
        (
            C::FixedSample { epoch: actual, .. },
            C::FixedSample {
                epoch: expected, ..
            },
        ) if actual != expected => Some(ClockField::Epoch),
        (
            C::FixedSample { period: actual, .. },
            C::FixedSample {
                period: expected, ..
            },
        ) if actual != expected => Some(ClockField::Period),
        (C::FixedSample { unit: actual, .. }, C::FixedSample { unit: expected, .. })
            if actual != expected =>
        {
            Some(ClockField::Unit)
        }
        (
            C::TimestampedEvent {
                timestamp_unit: actual,
            },
            C::TimestampedEvent {
                timestamp_unit: expected,
            },
        ) if actual != expected => Some(ClockField::TimestampUnit),
        (C::EventPosition { .. }, C::EventPosition { .. })
        | (C::FixedSample { .. }, C::FixedSample { .. })
        | (C::TimestampedEvent { .. }, C::TimestampedEvent { .. }) => None,
        _ => Some(ClockField::Alternative),
    };
    field.map_or(Ok(()), |field| Err(Error::V2(Refusal::Clock(field))))
}

fn temporal(
    package: &wire::Package,
    expected: &[ExpectedTemporal<'_>],
    work: &mut Work,
) -> Result<(), Error> {
    let inherited = &package.inherited;
    let temporal_len = inherited
        .declarations
        .iter()
        .filter(|declaration| matches!(declaration.body, artifact::wire::Body::Temporal { .. }))
        .count();
    work.charge(Dimension::Entries, temporal_len)?;
    work.charge(Dimension::Entries, package.temporal_bindings.len())?;
    work.charge(Dimension::Entries, expected.len())?;
    if package.temporal_bindings.len() < temporal_len {
        return Err(binding_refusal(InventorySide::Offer, BindingCause::Missing));
    }
    if package.temporal_bindings.len() > temporal_len {
        return Err(binding_refusal(InventorySide::Offer, BindingCause::Surplus));
    }
    if expected.len() < temporal_len {
        return Err(binding_refusal(
            InventorySide::Expected,
            BindingCause::Missing,
        ));
    }
    if expected.len() > temporal_len {
        return Err(binding_refusal(
            InventorySide::Expected,
            BindingCause::Surplus,
        ));
    }
    for pair in package.temporal_bindings.windows(2) {
        work.visit()?;
        if pair[0].declaration == pair[1].declaration {
            return Err(binding_refusal(
                InventorySide::Offer,
                BindingCause::Duplicate,
            ));
        }
        if pair[0].declaration > pair[1].declaration {
            return Err(Error::V2(Refusal::OfferOrder));
        }
    }
    let mut seen = Vec::new();
    seen.try_reserve_exact(temporal_len)
        .map_err(|_| Error::Allocation)?;
    seen.resize(temporal_len, false);
    for ((declaration_index, declaration), binding) in inherited
        .declarations
        .iter()
        .enumerate()
        .filter(|(_, declaration)| {
            matches!(declaration.body, artifact::wire::Body::Temporal { .. })
        })
        .zip(&package.temporal_bindings)
    {
        work.visit()?;
        let declaration_u32 = u32::try_from(declaration_index)
            .map_err(|_| Error::Invalid(Invalid::StructuralInteger))?;
        if binding.declaration != declaration_u32 {
            return Err(Error::V2(Refusal::OfferIndex(BindingIndex::Declaration)));
        }
        if binding.definition != declaration.profile {
            return Err(Error::V2(Refusal::OfferIndex(BindingIndex::Definition)));
        }
        let definition = inherited
            .definitions
            .get(usize::try_from(binding.definition).unwrap_or(usize::MAX))
            .ok_or(Error::Invalid(Invalid::Reference))?;
        let source = inherited
            .sources
            .get(usize::try_from(declaration.locus.source).unwrap_or(usize::MAX))
            .ok_or(Error::Invalid(Invalid::Locus))?;
        let mut found = None;
        let mut foreign_owner = false;
        for (index, candidate) in expected.iter().enumerate() {
            work.visit()?;
            if candidate.declaration.span == &declaration.locus.span {
                charge_reference_pair(candidate.source, &source.artifact, work)?;
                if declaration_key(candidate)
                    == (
                        &source.artifact,
                        declaration.locus.span.start,
                        declaration.locus.span.end,
                    )
                {
                    if found.replace(index).is_some() {
                        return Err(binding_refusal(
                            InventorySide::Expected,
                            BindingCause::Duplicate,
                        ));
                    }
                } else {
                    foreign_owner = true;
                }
            }
        }
        let found = found.ok_or_else(|| {
            if foreign_owner {
                Error::V2(Refusal::ForeignOwner(SelectionSide::Expected))
            } else {
                binding_refusal(InventorySide::Expected, BindingCause::Missing)
            }
        })?;
        if std::mem::replace(&mut seen[found], true) {
            return Err(binding_refusal(
                InventorySide::Expected,
                BindingCause::Duplicate,
            ));
        }
        let selected = &expected[found];
        work.bytes(declaration.name.len())?;
        work.bytes(selected.declaration.name.len())?;
        work.bytes(declaration.clause.len())?;
        work.bytes(selected.declaration.clause.len())?;
        compare_declaration(declaration, selected, work)?;
        let dependency = inherited
            .dependencies
            .get(usize::try_from(definition.artifact).unwrap_or(usize::MAX))
            .ok_or(Error::Invalid(Invalid::Reference))?;
        work.bytes(definition.identity.len())?;
        work.bytes(selected.definition.identity.len())?;
        if definition.identity != selected.definition.identity {
            return Err(Error::V2(Refusal::Definition(DefinitionField::Identity)));
        }
        work.bytes(
            revision_bytes(&definition.revision)
                .saturating_add(revision_bytes(selected.definition.revision)),
        )?;
        if definition.revision != *selected.definition.revision {
            return Err(Error::V2(Refusal::Definition(DefinitionField::Revision)));
        }
        charge_reference_pair(&dependency.artifact, selected.definition.artifact, work)?;
        if let Some(field) = artifact_field(&dependency.artifact, selected.definition.artifact) {
            return Err(Error::V2(Refusal::Definition(DefinitionField::Artifact(
                field,
            ))));
        }
        clock(&definition.identity, &binding.clock, work)?;
        clock(&definition.identity, selected.clock, work)?;
        compare_clock(&binding.clock, selected.clock)?;
    }
    if seen.contains(&false) {
        return Err(binding_refusal(
            InventorySide::Expected,
            BindingCause::Missing,
        ));
    }
    Ok(())
}

pub(super) fn validate(
    package: &wire::Package,
    expected: &[ExpectedTemporal<'_>],
    work: &mut Work,
) -> Result<(), Error> {
    temporal(package, expected, work)
}

fn numbers(package: &wire::Package, work: &mut Work) -> Result<(), Error> {
    artifact::validate::numbers(&package.inherited, work)?;
    for binding in &package.temporal_bindings {
        work.visit()?;
        let definition = package
            .inherited
            .definitions
            .get(usize::try_from(binding.definition).unwrap_or(usize::MAX))
            .ok_or(Error::Invalid(Invalid::Reference))?;
        clock(&definition.identity, &binding.clock, work)?;
    }
    Ok(())
}

/// Encode a typed untrusted version-2 candidate without granting admission.
pub fn encode_candidate(package: &wire::Package, limits: Limits) -> Report<Candidate> {
    let mut work = Work::new(limits);
    let result = (|| {
        numbers(package, &mut work)?;
        work.locus = None;
        artifact::encoding::candidate(package, &mut work)
    })();
    artifact::report(work, result)
}

/// Read exact bounded version-2 bytes against independent complete selections.
pub fn read(bytes: &[u8], expected: &Expected<'_>, limits: Limits) -> Report<AdmittedPackage> {
    read_with_producers(bytes, expected, &[], limits)
}

/// Read exact bounded version-2 bytes with admitted producer views.
pub fn read_with_producers(
    bytes: &[u8],
    expected: &Expected<'_>,
    producers: &[artifact::ExpectedProducerModel<'_>],
    limits: Limits,
) -> Report<AdmittedPackage> {
    let mut work = Work::new(limits);
    let result = (|| {
        work.charge(Dimension::PayloadBytes, bytes.len())?;
        work.bytes(bytes.len())?;
        let digest = ByteDigest::of(bytes);
        if digest != expected.inherited.artifact.digest {
            return Err(Error::Invalid(Invalid::Seal));
        }
        let package: wire::Package = artifact::decode::bounded(bytes, &mut work)?;
        headers(&package, &expected.inherited, &mut work)?;
        validate(&package, expected.temporal, &mut work)?;
        let supplied =
            artifact::intake::selected(&package.inherited, &expected.inherited, &mut work)?;
        artifact::intake::sources(&package.inherited, &expected.inherited, &mut work)?;
        artifact::intake::definitions(&package.inherited, &supplied, &mut work)?;
        artifact::validate::package(&package.inherited, &mut work)?;
        let model_schema = artifact::models::validate_with_producers(
            &package.inherited,
            expected.inherited.models,
            expected.inherited.dependencies,
            producers,
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
                expected.inherited.artifact,
            )?),
            model_schema,
        })
    })();
    artifact::report(work, result)
}
