// SPDX-License-Identifier: AGPL-3.0-only
//! FR-050: bounded independent admission of version-2 packages.

use super::{wire, AdmittedPackage, Expected, ExpectedTemporal, MEDIA, SCHEMA, WIRE};
use crate::protocol_artifact::{self as artifact, work::Work};
use crate::protocol_artifact::{Candidate, Dimension, Error, Invalid, Limits, Report, Unsupported};
use crate::ByteDigest;

fn headers(
    package: &wire::Package,
    expected: &artifact::Expected<'_>,
    work: &mut Work,
) -> Result<(), Error> {
    let inherited = &package.inherited;
    if inherited.wire != WIRE
        || inherited.media != MEDIA
        || inherited.schema != SCHEMA
        || inherited.package_type != artifact::PACKAGE_TYPE
        || inherited.encoding != artifact::ENCODING
        || inherited.numeric != artifact::NUMERIC_PROFILE
    {
        return Err(Error::Unsupported(Unsupported::Wire));
    }
    artifact::intake::reference(expected.artifact)?;
    if expected.artifact.kind != artifact::wire::ArtifactKind::LinkedPackage
        || expected.artifact.wire.identity != "quire.compiled-protocol"
        || expected.artifact.wire.version != "2"
    {
        return Err(Error::Invalid(Invalid::Selection));
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
        ) => artifact::intake::name(sequence_authority),
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
            artifact::intake::name(unit)
        }
        (
            crate::temporal::TIMESTAMPED_WINDOW,
            wire::ClockConfiguration::TimestampedEvent { timestamp_unit },
        ) => artifact::intake::name(timestamp_unit),
        (
            crate::temporal::EVENT_POSITION
            | crate::temporal::FIXED_SAMPLE
            | crate::temporal::TIMESTAMPED_WINDOW,
            _,
        ) => Err(Error::Invalid(Invalid::Profile)),
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
    if temporal_len != package.temporal_bindings.len() || temporal_len != expected.len() {
        return Err(Error::Invalid(Invalid::Inventory));
    }
    for pair in package.temporal_bindings.windows(2) {
        work.visit()?;
        if pair[0].declaration >= pair[1].declaration {
            return Err(Error::Invalid(Invalid::Order));
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
            return Err(Error::Invalid(Invalid::Inventory));
        }
        if binding.definition != declaration.profile {
            return Err(Error::Invalid(Invalid::Profile));
        }
        let definition = inherited
            .definitions
            .get(usize::try_from(binding.definition).unwrap_or(usize::MAX))
            .ok_or(Error::Invalid(Invalid::Reference))?;
        clock(&definition.identity, &binding.clock, work)?;
        let source = inherited
            .sources
            .get(usize::try_from(declaration.locus.source).unwrap_or(usize::MAX))
            .ok_or(Error::Invalid(Invalid::Locus))?;
        let found = expected.iter().position(|candidate| {
            declaration_key(candidate)
                == (
                    &source.artifact,
                    declaration.locus.span.start,
                    declaration.locus.span.end,
                )
        });
        let found = found.ok_or_else(|| {
            if expected
                .iter()
                .any(|candidate| candidate.declaration.span == &declaration.locus.span)
            {
                Error::Invalid(Invalid::Owner)
            } else {
                Error::Invalid(Invalid::Inventory)
            }
        })?;
        if std::mem::replace(&mut seen[found], true) {
            return Err(Error::Invalid(Invalid::Duplicate));
        }
        let selected = &expected[found];
        work.bytes(declaration.name.len())?;
        work.bytes(selected.declaration.name.len())?;
        work.bytes(declaration.clause.len())?;
        work.bytes(selected.declaration.clause.len())?;
        if declaration.name != selected.declaration.name
            || declaration.locus.span != *selected.declaration.span
            || declaration.requirement != *selected.declaration.requirement
            || declaration.clause != selected.declaration.clause
            || declaration.execution != *selected.declaration.execution
        {
            return Err(Error::Invalid(Invalid::Selection));
        }
        let dependency = inherited
            .dependencies
            .get(usize::try_from(definition.artifact).unwrap_or(usize::MAX))
            .ok_or(Error::Invalid(Invalid::Reference))?;
        work.bytes(definition.identity.len())?;
        work.bytes(selected.definition.identity.len())?;
        if definition.identity != selected.definition.identity
            || definition.revision != *selected.definition.revision
            || dependency.artifact != *selected.definition.artifact
            || binding.clock != *selected.clock
        {
            return Err(Error::Invalid(Invalid::Selection));
        }
    }
    if seen.contains(&false) {
        return Err(Error::Invalid(Invalid::Inventory));
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
        let supplied =
            artifact::intake::selected(&package.inherited, &expected.inherited, &mut work)?;
        artifact::intake::sources(&package.inherited, &expected.inherited, &mut work)?;
        artifact::intake::definitions(&package.inherited, &supplied, &mut work)?;
        artifact::validate::package(&package.inherited, &mut work)?;
        artifact::models::validate(
            &package.inherited,
            expected.inherited.models,
            expected.inherited.dependencies,
            &mut work,
        )?;
        validate(&package, expected.temporal, &mut work)?;
        let canonical = artifact::encoding::bytes(&package, &mut work)?;
        work.bytes(bytes.len())?;
        work.bytes(canonical.len())?;
        if canonical != bytes {
            return Err(Error::Invalid(Invalid::Canonical));
        }
        Ok(AdmittedPackage { package, digest })
    })();
    artifact::report(work, result)
}
