// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-042: independent exact inventories and registered definition meanings.

use super::{wire::*, work::Work, *};
use crate::linking::composed::definition_source::RegisteredDefinition as Registered;

type Key<'a> = (
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
);

pub(super) fn key(reference: &ArtifactRef) -> Key<'_> {
    (
        reference.kind.as_str(),
        &reference.authority,
        &reference.identity,
        &reference.revision.namespace,
        &reference.revision.value,
        &reference.wire.identity,
        &reference.wire.version,
    )
}

pub(super) fn name(value: &str) -> Result<(), Error> {
    if value.is_empty() || value.len() > 4096 {
        Err(Error::Invalid(Invalid::Name))
    } else {
        Ok(())
    }
}

pub(super) fn revision(value: &Revision) -> Result<(), Error> {
    name(&value.namespace)?;
    name(&value.value)
}

pub(super) fn reference(value: &ArtifactRef) -> Result<(), Error> {
    if value.ref_version != "ix.artifact-ref/3-draft" {
        return Err(Error::Unsupported(Unsupported::Wire));
    }
    for text in [
        &value.authority,
        &value.identity,
        &value.wire.identity,
        &value.wire.version,
    ] {
        name(text)?;
    }
    revision(&value.revision)
}

fn reference_bytes(value: &ArtifactRef) -> usize {
    value.ref_version.len()
        + value.kind.as_str().len()
        + value.authority.len()
        + value.identity.len()
        + value.revision.namespace.len()
        + value.revision.value.len()
        + value.wire.identity.len()
        + value.wire.version.len()
        + 71
}

fn retained_text(value: &str) -> Result<String, Error> {
    let mut retained = String::new();
    retained
        .try_reserve_exact(value.len())
        .map_err(|_| Error::Allocation)?;
    retained.push_str(value);
    Ok(retained)
}

pub(super) fn retained_reference(value: &ArtifactRef) -> Result<ArtifactRef, Error> {
    Ok(ArtifactRef {
        ref_version: retained_text(&value.ref_version)?,
        kind: value.kind,
        authority: retained_text(&value.authority)?,
        identity: retained_text(&value.identity)?,
        revision: Revision {
            namespace: retained_text(&value.revision.namespace)?,
            value: retained_text(&value.revision.value)?,
        },
        digest: value.digest,
        wire: Wire {
            identity: retained_text(&value.wire.identity)?,
            version: retained_text(&value.wire.version)?,
        },
    })
}

pub(super) fn same_reference(
    a: &ArtifactRef,
    b: &ArtifactRef,
    work: &mut Work,
) -> Result<(), Error> {
    work.bytes(reference_bytes(a).saturating_add(reference_bytes(b)))?;
    if a == b {
        Ok(())
    } else {
        Err(Error::Invalid(Invalid::Selection))
    }
}

pub(super) fn find_reference<T>(
    values: &[T],
    target: &ArtifactRef,
    get: impl Fn(&T) -> &ArtifactRef,
    work: &mut Work,
) -> Result<usize, Error> {
    let mut lower = 0;
    let mut upper = values.len();
    while lower < upper {
        work.visit()?;
        let middle = lower + (upper - lower) / 2;
        let offered = get(&values[middle]);
        work.bytes(reference_bytes(offered).saturating_add(reference_bytes(target)))?;
        match key(offered).cmp(&key(target)) {
            std::cmp::Ordering::Less => lower = middle + 1,
            std::cmp::Ordering::Greater => upper = middle,
            std::cmp::Ordering::Equal => {
                same_reference(offered, target, work)?;
                return Ok(middle);
            }
        }
    }
    Err(Error::Invalid(Invalid::Inventory))
}

fn ordered_references<T>(
    values: &[T],
    get: impl Fn(&T) -> &ArtifactRef,
    work: &mut Work,
) -> Result<(), Error> {
    for value in values {
        work.visit()?;
        reference(get(value))?;
    }
    for pair in values.windows(2) {
        work.visit()?;
        let left = get(&pair[0]);
        let right = get(&pair[1]);
        work.bytes(reference_bytes(left).saturating_add(reference_bytes(right)))?;
        if key(left) >= key(right) {
            return Err(Error::Invalid(Invalid::Order));
        }
    }
    Ok(())
}

pub(super) fn sorted_indices(values: &[u32], work: &mut Work) -> Result<(), Error> {
    for pair in values.windows(2) {
        work.visit()?;
        if pair[0] >= pair[1] {
            return Err(Error::Invalid(Invalid::Order));
        }
    }
    Ok(())
}

pub(super) fn selected<'a>(
    package: &Package,
    expected: &'a Expected<'a>,
    work: &mut Work,
) -> Result<Vec<&'a SuppliedDependency<'a>>, Error> {
    work.charge(Dimension::Dependencies, expected.dependencies.len())?;
    work.charge(Dimension::Dependencies, package.dependencies.len())?;
    if expected.dependencies.len() != package.dependencies.len() {
        return Err(Error::Invalid(Invalid::Inventory));
    }
    ordered_references(
        &package.dependencies,
        |dependency| &dependency.artifact,
        work,
    )?;
    work.charge(Dimension::Entries, package.dependencies.len())?;
    let mut supplied = vec![None; package.dependencies.len()];
    for dependency in expected.dependencies {
        work.visit()?;
        reference(dependency.artifact)?;
        let index = find_reference(
            &package.dependencies,
            dependency.artifact,
            |value| &value.artifact,
            work,
        )?;
        if supplied[index].replace(dependency).is_some() {
            return Err(Error::Invalid(Invalid::Duplicate));
        }
        work.bytes(dependency.bytes.len())?;
        if ByteDigest::of(dependency.bytes) != dependency.artifact.digest {
            return Err(Error::Invalid(Invalid::Seal));
        }
        let offered = &package.dependencies[index];
        sorted_indices(&offered.requires, work)?;
        if offered.requires.len() != dependency.requires.len() {
            return Err(Error::Invalid(Invalid::Dependency));
        }
        // Independent selectors may arrive in a different order; each must be
        // present exactly once in the canonical direct dependency set.
        work.charge(Dimension::Entries, offered.requires.len())?;
        let mut seen = vec![false; offered.requires.len()];
        for required in dependency.requires {
            let selected = find_reference(
                &package.dependencies,
                required,
                |value| &value.artifact,
                work,
            )?;
            work.visit()?;
            let position = offered
                .requires
                .binary_search(&(selected as u32))
                .map_err(|_| Error::Invalid(Invalid::Dependency))?;
            if std::mem::replace(&mut seen[position], true) {
                return Err(Error::Invalid(Invalid::Duplicate));
            }
            if selected == index {
                return Err(Error::Invalid(Invalid::Cycle));
            }
        }
    }
    // Completeness follows from equal length, exact key matches and no duplicates.
    supplied
        .into_iter()
        .map(|item| item.ok_or(Error::Invalid(Invalid::Inventory)))
        .collect()
}

fn headers(package: &Package, expected: &Expected<'_>, work: &mut Work) -> Result<(), Error> {
    if package.wire != WIRE
        || package.media != MEDIA
        || package.schema != SCHEMA
        || package.package_type != PACKAGE_TYPE
        || package.encoding != ENCODING
        || package.numeric != NUMERIC_PROFILE
    {
        return Err(Error::Unsupported(Unsupported::Wire));
    }
    reference(expected.artifact)?;
    if expected.artifact.kind != ArtifactKind::LinkedPackage
        || expected.artifact.wire.identity != "quire.compiled-protocol"
        || expected.artifact.wire.version != "1"
    {
        return Err(Error::Invalid(Invalid::Selection));
    }
    same_reference(&package.contract, expected.contract, work)?;
    same_reference(&package.baseline, expected.baseline, work)?;
    // The producer's binary is Producer identity, checked here as a
    // structurally valid reference (`reference`) equal to the caller's own
    // expectation (`same_reference`) -- it is never one of the byte-sealed
    // `dependencies`, so it is not looked up there.
    reference(&expected.producer.binary)?;
    same_reference(&package.producer.binary, &expected.producer.binary, work)?;
    name(&package.producer.implementation)?;
    revision(&package.producer.revision)?;
    name(&package.language.edition)?;
    if package.producer != *expected.producer
        || package.language != *expected.language
        || package.language.identity != "ix:native"
    {
        return Err(Error::Invalid(Invalid::Selection));
    }
    for selected in [&package.contract, &package.baseline] {
        find_reference(
            &package.dependencies,
            selected,
            |value| &value.artifact,
            work,
        )?;
    }
    Ok(())
}

pub(super) fn sources(
    package: &Package,
    expected: &Expected<'_>,
    work: &mut Work,
) -> Result<(), Error> {
    work.charge(Dimension::Sources, expected.sources.len())?;
    work.charge(Dimension::Sources, package.sources.len())?;
    work.charge(Dimension::Declarations, package.declarations.len())?;
    if package.sources.len() != expected.sources.len() {
        return Err(Error::Invalid(Invalid::Inventory));
    }
    ordered_references(&package.sources, |source| &source.artifact, work)?;
    work.charge(Dimension::Entries, package.sources.len())?;
    let mut seen = vec![false; package.sources.len()];
    for source in &package.sources {
        if source.artifact.kind != ArtifactKind::Source {
            return Err(Error::Invalid(Invalid::Selection));
        }
        for text in [
            &source.native.identity,
            &source.native.revision,
            &source.path,
            &source.formal.document,
        ] {
            name(text)?;
        }
        revision(&source.formal.revision)?;
        work.charge(Dimension::SourceBytes, source.text.len())?;
        work.bytes(source.text.len())?;
        if ByteDigest::of(source.text.as_bytes()) != source.artifact.digest {
            return Err(Error::Invalid(Invalid::Seal));
        }
    }
    // Validate ordering before using binary searches for independent selections.
    for declaration in &package.declarations {
        work.visit()?;
        let source = package
            .sources
            .get(declaration.locus.source as usize)
            .ok_or(Error::Invalid(Invalid::Locus))?;
        span(&source.text, &declaration.locus.span)?;
    }
    for pair in package.declarations.windows(2) {
        work.visit()?;
        let (a, b) = (&pair[0].locus, &pair[1].locus);
        if (a.source, a.span.start, a.span.end) >= (b.source, b.span.start, b.span.end)
            || a.source == b.source && a.span.end > b.span.start
        {
            return Err(Error::Invalid(Invalid::Order));
        }
    }
    work.charge(Dimension::Entries, package.declarations.len())?;
    let mut declarations_seen = vec![false; package.declarations.len()];
    for source in expected.sources {
        let index = find_reference(
            &package.sources,
            source.artifact,
            |value| &value.artifact,
            work,
        )?;
        if std::mem::replace(&mut seen[index], true) {
            return Err(Error::Invalid(Invalid::Duplicate));
        }
        let offered = &package.sources[index];
        work.bytes(source.text.len().saturating_add(offered.text.len()))?;
        if source.native != &offered.native
            || source.path != offered.path
            || source.formal != &offered.formal
            || source.text != offered.text
        {
            return Err(Error::Invalid(Invalid::Selection));
        }
        work.charge(Dimension::Declarations, source.declarations.len())?;
        for declaration in source.declarations {
            let target = (index as u32, declaration.span.start, declaration.span.end);
            let mut lower = 0;
            let mut upper = package.declarations.len();
            let found = loop {
                if lower == upper {
                    return Err(Error::Invalid(Invalid::Inventory));
                }
                work.visit()?;
                let middle = lower + (upper - lower) / 2;
                let locus = &package.declarations[middle].locus;
                match (locus.source, locus.span.start, locus.span.end).cmp(&target) {
                    std::cmp::Ordering::Less => lower = middle + 1,
                    std::cmp::Ordering::Greater => upper = middle,
                    std::cmp::Ordering::Equal => break middle,
                }
            };
            if std::mem::replace(&mut declarations_seen[found], true) {
                return Err(Error::Invalid(Invalid::Duplicate));
            }
            let offered = &package.declarations[found];
            work.bytes(
                declaration
                    .name
                    .len()
                    .saturating_add(declaration.clause.len()),
            )?;
            if offered.name != declaration.name
                || &offered.requirement != declaration.requirement
                || offered.clause != declaration.clause
                || &offered.execution != declaration.execution
            {
                return Err(Error::Invalid(Invalid::Selection));
            }
        }
    }
    if declarations_seen.contains(&false) {
        return Err(Error::Invalid(Invalid::Inventory));
    }
    Ok(())
}

pub(super) fn span(text: &str, span: &Span) -> Result<(), Error> {
    let (start, end) = (span.start as usize, span.end as usize);
    if start > end
        || end > 1_048_576
        || !text.is_char_boundary(start)
        || !text.is_char_boundary(end)
    {
        Err(Error::Invalid(Invalid::Locus))
    } else {
        Ok(())
    }
}

fn definition_key(value: &Definition) -> (&str, &str, &str) {
    (
        &value.identity,
        &value.revision.namespace,
        &value.revision.value,
    )
}

pub(super) fn definitions(
    package: &Package,
    supplied: &[&SuppliedDependency<'_>],
    work: &mut Work,
) -> Result<(), Error> {
    work.charge(Dimension::Definitions, package.definitions.len())?;
    for pair in package.definitions.windows(2) {
        work.visit()?;
        if definition_key(&pair[0]) >= definition_key(&pair[1]) {
            return Err(Error::Invalid(Invalid::Order));
        }
    }
    work.charge(Dimension::Entries, package.definitions.len())?;
    let mut registered = Vec::with_capacity(package.definitions.len());
    for definition in &package.definitions {
        work.visit()?;
        name(&definition.identity)?;
        revision(&definition.revision)?;
        let selected = supplied
            .get(definition.artifact as usize)
            .ok_or(Error::Invalid(Invalid::Reference))?;
        if selected.artifact.kind != ArtifactKind::Source {
            return Err(Error::Invalid(Invalid::Definition));
        }
        let mut found = None;
        for candidate in Registered::all() {
            work.visit()?;
            work.bytes(
                definition
                    .identity
                    .len()
                    .saturating_add(definition.revision.value.len()),
            )?;
            if definition.identity == candidate.identity()
                && definition.revision.value == candidate.revision()
            {
                found = Some(*candidate);
                break;
            }
        }
        registered.push(found.ok_or(Error::Unsupported(Unsupported::Definition))?);
    }
    for (index, definition) in package.definitions.iter().enumerate() {
        let selected = registered[index];
        sorted_indices(&definition.requires, work)?;
        sorted_indices(&definition.rules, work)?;
        if definition.requires.len() != selected.requirements().len()
            || definition.rules.len() != selected.rules().len()
        {
            return Err(Error::Invalid(Invalid::Definition));
        }
        for required in &definition.requires {
            work.visit()?;
            let target = registered
                .get(*required as usize)
                .ok_or(Error::Invalid(Invalid::Reference))?;
            if !selected.requirements().contains(target) {
                return Err(Error::Invalid(Invalid::Definition));
            }
        }
        work.charge(Dimension::Entries, selected.rules().len())?;
        let mut seen = vec![false; selected.rules().len()];
        for rule in &definition.rules {
            work.visit()?;
            let supplied = supplied
                .get(*rule as usize)
                .ok_or(Error::Invalid(Invalid::Reference))?;
            if supplied.artifact.kind != ArtifactKind::Source {
                return Err(Error::Invalid(Invalid::Definition));
            }
            let mut found = false;
            for (position, rule) in selected.rules().iter().enumerate() {
                work.visit()?;
                work.bytes(
                    rule.path
                        .len()
                        .saturating_add(supplied.artifact.identity.len()),
                )?;
                if supplied.artifact.identity == rule.path {
                    if std::mem::replace(&mut seen[position], true) {
                        return Err(Error::Invalid(Invalid::Duplicate));
                    }
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(Error::Invalid(Invalid::Definition));
            }
        }
    }
    if registered.get(package.package_definition as usize) != Some(&Registered::Package)
        || !registered.contains(&Registered::Protocol)
        || !registered.contains(&Registered::Edition)
        || package.language.edition != "1-draft"
    {
        return Err(Error::Invalid(Invalid::Definition));
    }
    Ok(())
}

/// Read exact bounded bytes without parsing any embedded native source text.
/// The returned admission is scoped to the independently supplied selections.
pub fn read(bytes: &[u8], expected: &Expected<'_>, limits: Limits) -> Report<AdmittedPackage> {
    let mut work = Work::new(limits);
    let result = (|| {
        work.charge(Dimension::PayloadBytes, bytes.len())?;
        work.bytes(bytes.len())?;
        let digest = ByteDigest::of(bytes);
        if digest != expected.artifact.digest {
            return Err(Error::Invalid(Invalid::Seal));
        }
        let package = super::decode::package(bytes, &mut work)?;
        headers(&package, expected, &mut work)?;
        let supplied = selected(&package, expected, &mut work)?;
        sources(&package, expected, &mut work)?;
        definitions(&package, &supplied, &mut work)?;
        super::validate::package(&package, &mut work)?;
        let model_schema =
            super::models::validate(&package, expected.models, expected.dependencies, &mut work)?;
        let canonical = super::encoding::bytes(&package, &mut work)?;
        work.bytes(bytes.len().saturating_add(canonical.len()))?;
        if canonical != bytes {
            return Err(Error::Invalid(Invalid::Canonical));
        }
        Ok(AdmittedPackage {
            package,
            digest,
            artifact: retained_reference(expected.artifact)?,
            model_schema,
        })
    })();
    report(work, result)
}
