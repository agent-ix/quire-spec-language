// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-042/FR-056: compiled-protocol models that name an admitted domain package.
//!
//! A domain-package `Model` selects, through `artifact`, the model-package
//! dependency whose bytes are the package's Semantic IR 2.0.0 document, and
//! names the package directly in `domain_package`: its identity, version and
//! `sha256-jcs` digest. Its exports are the package's object types and
//! Interfaces (`object`), record value types (`record`), and their field and
//! operation members, each under its artifact id, and one `population`
//! export `[object artifact id, population artifact id]` for every object
//! type an FR-153 population declaration covers. Intake keeps no per-node
//! source span yet (FR-056-AC-1), so every export's locus is the whole
//! document in that dependency.

use std::collections::{BTreeMap, BTreeSet};

use super::{wire as w, work::Work, Dimension, Error, Invalid, Unsupported};
use crate::checking::DomainType;
use crate::linking::composed::models::BoundDeclaration;
use qsl_semantics::model::admitted::{AdmittedPackage, Declaration};
use qsl_semantics::model::domain_package::{
    DomainPackageRecord, DomainPackageRef, PopulationRecord, ValueTypeRef,
};
use qsl_semantics::model::key::{DeclarationKey, SHA256_JCS_DIGEST_DOMAIN};

/// `Model.profile` of a domain-package model: the Semantic IR contract its
/// document is admitted under (FR-056).
pub const DOMAIN_PACKAGE_PROFILE: &str = "semantic-ir/2.0.0";

/// Revision namespace of a domain-package export locus: the formal revision
/// is the package's own version.
pub const DOMAIN_PACKAGE_VERSION_NAMESPACE: &str = "ix:domain-package-version";

/// Constructor-private authority for one admitted domain package, separate
/// from a wire `Model`.
#[derive(Clone, Copy, Debug)]
pub struct AdmittedDomainPackage<'a> {
    /// Exact selected model-package reference whose dependency bytes are the
    /// package's Semantic IR document.
    pub artifact: &'a w::ArtifactRef,
    /// The package, admitted under FR-056 and classified under FR-152.
    pub package: &'a AdmittedPackage,
}

/// What one domain-package export names.
#[derive(Clone, Copy, Debug)]
pub(super) enum DomainTarget<'a> {
    /// An object type, Interface or record value type.
    Type(DomainType<'a>),
    /// A field of such a type.
    Field(DomainType<'a>, &'a str),
    /// An operation of an object type or Interface, by its owning type.
    Operation(DomainType<'a>),
    /// The population input of an object type: the object type and the
    /// FR-153 population declaration covering it.
    Population(DomainType<'a>, Declaration<'a>),
}

/// One export: its kind, path and target, ascending by `(kind, path)`.
pub(super) struct DomainExport<'a> {
    pub kind: w::ExportKind,
    pub owner: &'a str,
    pub member: Option<&'a str>,
    pub target: DomainTarget<'a>,
}

/// The export kind a domain type is exported under: `object` for an object
/// type or Interface, `record` for a record value type.
pub(super) fn type_export_kind(ty: &DomainType<'_>) -> w::ExportKind {
    if ty.is_object() {
        w::ExportKind::Object
    } else {
        w::ExportKind::Record
    }
}

/// The name a member key carries after its owner's key: `<owner>/<name>`.
fn member_name<'a>(owner: &str, member: &'a str) -> Option<&'a str> {
    member.strip_prefix(owner)?.strip_prefix('/')
}

/// The artifact id a type-definition or population node is exported under:
/// its key's `ix://<package>/<artifact id>` segment.
pub(super) fn artifact_id(key: &DeclarationKey) -> &str {
    key.node
        .rsplit_once('/')
        .map_or(key.node.as_str(), |(_, artifact)| artifact)
}

/// Every export of `package`, ascending by `(kind, owner, member)` exactly
/// as the wire orders exports.
pub(super) fn exports<'a>(
    package: &'a AdmittedPackage,
    work: &mut Work,
) -> Result<Vec<DomainExport<'a>>, Error> {
    let mut result = BTreeMap::new();
    for declaration in package.declarations() {
        work.visit()?;
        let (kind, owner) = match declaration.record {
            DomainPackageRecord::FieldMember(field) => (w::ExportKind::Field, &field.owner),
            DomainPackageRecord::OperationMember(operation) => {
                (w::ExportKind::Operation, &operation.owner)
            }
            DomainPackageRecord::ObjectType(_) | DomainPackageRecord::RecordValueType(_) => {
                let Some(ty) = DomainType::new(package, declaration) else {
                    return Err(Error::Invalid(Invalid::Model));
                };
                let kind = type_export_kind(&ty);
                work.bytes(ty.artifact_id().len())?;
                work.charge(Dimension::Entries, 1)?;
                if result
                    .insert(
                        (kind.as_str(), ty.artifact_id(), ""),
                        (kind, DomainTarget::Type(ty)),
                    )
                    .is_some()
                {
                    return Err(Error::Invalid(Invalid::Duplicate));
                }
                continue;
            }
            DomainPackageRecord::Population(population) => {
                for covered in package.declarations() {
                    work.visit()?;
                    let Some(object) = DomainType::new(package, covered) else {
                        continue;
                    };
                    if !object.is_object() || !covers(population, &object, work)? {
                        continue;
                    }
                    let name = artifact_id(declaration.key);
                    work.bytes(object.artifact_id().len().saturating_add(name.len()))?;
                    work.charge(Dimension::Entries, 1)?;
                    if result
                        .insert(
                            (
                                w::ExportKind::Population.as_str(),
                                object.artifact_id(),
                                name,
                            ),
                            (
                                w::ExportKind::Population,
                                DomainTarget::Population(object, declaration),
                            ),
                        )
                        .is_some()
                    {
                        return Err(Error::Invalid(Invalid::Duplicate));
                    }
                }
                continue;
            }
            DomainPackageRecord::ScalarType(_)
            | DomainPackageRecord::Component(_)
            | DomainPackageRecord::Endpoint(_)
            | DomainPackageRecord::Relationship(_)
            | DomainPackageRecord::Allocation(_) => continue,
        };
        let owner = package
            .declaration_by_key(owner)
            .and_then(|owner| DomainType::new(package, owner))
            .ok_or(Error::Invalid(Invalid::Model))?;
        let name = member_name(&owner.declaration.key.node, &declaration.key.node)
            .ok_or(Error::Invalid(Invalid::Model))?;
        work.bytes(owner.artifact_id().len().saturating_add(name.len()))?;
        work.charge(Dimension::Entries, 1)?;
        let target = match declaration.record {
            DomainPackageRecord::FieldMember(_) => DomainTarget::Field(owner, name),
            _ => DomainTarget::Operation(owner),
        };
        if result
            .insert((kind.as_str(), owner.artifact_id(), name), (kind, target))
            .is_some()
        {
            return Err(Error::Invalid(Invalid::Duplicate));
        }
    }
    Ok(result
        .into_iter()
        .map(|((_, owner, member), (kind, target))| DomainExport {
            kind,
            owner,
            member: (!member.is_empty()).then_some(member),
            target,
        })
        .collect())
}

/// The locus every export of the package names: the whole document in the
/// model's own dependency, under the package identity and version.
pub(super) fn locus(
    selected: &AdmittedDomainPackage<'_>,
    document_bytes: usize,
    work: &mut Work,
) -> Result<w::ForeignLocus, Error> {
    let selection = selected.package.selection();
    work.charge(Dimension::Entries, 3)?;
    work.bytes(
        selection
            .identity
            .len()
            .saturating_add(selection.version.len()),
    )?;
    Ok(w::ForeignLocus {
        source: selected.artifact.clone(),
        formal: w::Formal {
            document: selection.identity.clone(),
            revision: w::Revision {
                namespace: DOMAIN_PACKAGE_VERSION_NAMESPACE.to_owned(),
                value: selection.version.clone(),
            },
        },
        span: w::Span {
            start: 0,
            end: structural_index(document_bytes)?,
        },
    })
}

/// A span offset as a wire structural integer, bounded as every other index.
fn structural_index(value: usize) -> Result<u32, Error> {
    if value > 1_048_576 {
        return Err(Error::Invalid(Invalid::StructuralInteger));
    }
    u32::try_from(value).map_err(|_| Error::Invalid(Invalid::StructuralInteger))
}

/// `Model.domain_package` for `selection`.
pub(super) fn naming(
    selection: &DomainPackageRef,
    work: &mut Work,
) -> Result<w::DomainPackage, Error> {
    work.charge(Dimension::Entries, 3)?;
    work.bytes(
        selection
            .identity
            .len()
            .saturating_add(selection.version.len()),
    )?;
    Ok(w::DomainPackage {
        identity: selection.identity.clone(),
        version: selection.version.clone(),
        digest: w::JcsDigest(selection.digest),
    })
}

/// Whether `bytes` are the selected package's document: FR-154 admission of
/// `bytes` under the selection's own identity, version and `sha256-jcs`
/// digest.
pub(super) fn verify_document(
    selection: &DomainPackageRef,
    bytes: &[u8],
    work: &mut Work,
) -> Result<(), Error> {
    work.bytes(bytes.len())?;
    let offered = BTreeMap::from([(selection.digest, bytes.to_vec())]);
    qsl_semantics::model::intake::admit(selection, SHA256_JCS_DIGEST_DOMAIN, &offered)
        .map(|_| ())
        .map_err(|_| Error::Invalid(Invalid::Model))
}

/// Whether `population` covers the object type `object` (FR-153): one of
/// its member types is `object` itself or a type `object` conforms to
/// through its declared supertypes, walked transitively, each type once.
fn covers(
    population: &PopulationRecord,
    object: &DomainType<'_>,
    work: &mut Work,
) -> Result<bool, Error> {
    let mut seen = BTreeSet::new();
    let mut pending = vec![object.declaration.key];
    while let Some(key) = pending.pop() {
        work.visit()?;
        if population.member_types.contains(key) {
            return Ok(true);
        }
        if !seen.insert(key) {
            continue;
        }
        work.charge(Dimension::Entries, 1)?;
        if let Some(DomainPackageRecord::ObjectType(record)) = object
            .package
            .declaration_by_key(key)
            .map(|declaration| declaration.record)
        {
            work.charge(Dimension::Entries, record.supertypes.len())?;
            pending.extend(&record.supertypes);
        }
    }
    Ok(false)
}

/// The one population declaration of `object`'s package that covers the
/// object type `object`: its population input binds that declaration.
/// With no covering declaration there is no population export to bind, and
/// with more than one the source selects none of them, so both refuse as
/// `Unsupported::Export`.
pub(super) fn population<'a>(
    object: &DomainType<'a>,
    work: &mut Work,
) -> Result<Declaration<'a>, Error> {
    let mut found = None;
    for declaration in object.package.declarations() {
        work.visit()?;
        let DomainPackageRecord::Population(population) = declaration.record else {
            continue;
        };
        if covers(population, object, work)? && found.replace(declaration).is_some() {
            return Err(Error::Unsupported(Unsupported::Export));
        }
    }
    found.ok_or(Error::Unsupported(Unsupported::Export))
}

/// The object types a value of domain type `ty` reaches, each once: `ty`
/// itself when it is an object type or Interface, and every object type
/// reached through the declared value types of its fields, transitively,
/// whatever their multiplicity or native representation. Each reached
/// object type needs a population input.
pub(super) fn reached_objects<'a>(
    ty: &DomainType<'a>,
    work: &mut Work,
) -> Result<Vec<DomainType<'a>>, Error> {
    let mut seen = BTreeSet::new();
    let mut reached = Vec::new();
    let mut pending = vec![*ty];
    while let Some(ty) = pending.pop() {
        work.visit()?;
        if !seen.insert(ty.declaration.key) {
            continue;
        }
        work.charge(Dimension::Entries, 1)?;
        if ty.is_object() {
            reached.push(ty);
        }
        for declaration in ty.package.declarations() {
            work.visit()?;
            let DomainPackageRecord::FieldMember(field) = declaration.record else {
                continue;
            };
            if &field.owner != ty.declaration.key {
                continue;
            }
            let ValueTypeRef::Package(key) = &field.value_type else {
                continue;
            };
            let target = ty
                .package
                .declaration_by_key(key)
                .ok_or(Error::Invalid(Invalid::Model))?;
            // A domain value type carries no object.
            if let Some(inner) = DomainType::new(ty.package, target) {
                work.charge(Dimension::Entries, 1)?;
                pending.push(inner);
            }
        }
    }
    Ok(reached)
}

/// The owning type and name of a bound domain operation, `None` when
/// `bound` is not an operation member.
pub(super) fn operation<'a>(
    bound: &BoundDeclaration<'a>,
) -> Result<Option<(DomainType<'a>, &'a str)>, Error> {
    let Some((owner, name)) = bound.operation() else {
        return Ok(None);
    };
    let package = bound.package();
    let owner = package
        .declaration_by_key(owner)
        .and_then(|owner| DomainType::new(package, owner))
        .ok_or(Error::Invalid(Invalid::Model))?;
    Ok(Some((owner, name)))
}
