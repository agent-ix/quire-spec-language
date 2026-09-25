// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-036/040/042: actual model exports and first-occurrence native type identities.

use std::collections::BTreeMap;

use quire_contract_ir as ir;

use crate::checking::{DomainType, NativeType};
use crate::native_model::{NativeModel, ScalarKind, Unit};
use crate::protocol_artifact::{
    domain, wire as w, work::Work, AdmittedDomainPackage, AdmittedModel, Dimension, Error, Invalid,
    NumberWire, Unsupported, DOMAIN_PACKAGE_PROFILE,
};
use qsl_semantics::model::admitted::AdmittedPackage;

pub(super) struct ValueBuilder<'a> {
    selected: Vec<Selected<'a>>,
    models: Vec<w::Model>,
    types: Vec<w::Type>,
}

/// One selected model, in dependency order.
#[derive(Clone, Copy)]
enum Selected<'a> {
    Native(AdmittedModel<'a>),
    Domain(&'a AdmittedPackage),
}

/// A model to emit: a native model, or a domain package with the length of
/// its document dependency (the extent of every export locus).
#[derive(Clone, Copy)]
pub(super) enum ModelSelection<'a> {
    Native {
        dependency: u32,
        model: AdmittedModel<'a>,
    },
    Domain {
        dependency: u32,
        package: AdmittedDomainPackage<'a>,
        document_len: usize,
    },
}

impl ModelSelection<'_> {
    pub(super) fn dependency(&self) -> u32 {
        match self {
            Self::Native { dependency, .. } | Self::Domain { dependency, .. } => *dependency,
        }
    }
}

enum Shape {
    Option,
    Sequence(u32),
    Leaf(w::Type),
}

impl<'a> ValueBuilder<'a> {
    /// `models` ascending by dependency index.
    pub(super) fn new(models: &[ModelSelection<'a>], work: &mut Work) -> Result<Self, Error> {
        work.charge(Dimension::Models, models.len())?;
        let mut result = Self {
            selected: Vec::new(),
            models: Vec::new(),
            types: Vec::new(),
        };
        let mut previous = None;
        for selection in models {
            work.visit()?;
            let dependency = selection.dependency();
            if previous.is_some_and(|value| value >= dependency) {
                return Err(Error::Invalid(Invalid::Order));
            }
            previous = Some(dependency);
            match selection {
                ModelSelection::Native { model, .. } => {
                    if model.artifact.kind != w::ArtifactKind::ModelPackage
                        || model.artifact.digest != model.model.digest()
                    {
                        return Err(Error::Invalid(Invalid::Model));
                    }
                    let exports = model_exports(model, work)?;
                    let profile = text(model.model.profile().as_str(), work)?;
                    work.charge(Dimension::Entries, 2)?;
                    result.selected.push(Selected::Native(*model));
                    result.models.push(w::Model {
                        artifact: dependency,
                        profile,
                        exports,
                        // FR-042-AC-12: a `NativeModel` is directly admitted,
                        // never linked against an FR-056 domain package.
                        domain_package: w::Nullable(None),
                    });
                }
                ModelSelection::Domain {
                    package,
                    document_len,
                    ..
                } => {
                    if package.artifact.kind != w::ArtifactKind::ModelPackage {
                        return Err(Error::Invalid(Invalid::Model));
                    }
                    let locus = domain::locus(package, *document_len, work)?;
                    let mut exports = Vec::new();
                    for export in domain::exports(package.package, work)? {
                        work.visit()?;
                        work.charge(Dimension::Entries, 3)?;
                        let mut path = vec![text(export.owner, work)?];
                        if let Some(member) = export.member {
                            path.push(text(member, work)?);
                        }
                        exports.push(w::Export {
                            kind: export.kind,
                            path,
                            locus: locus.clone(),
                        });
                    }
                    work.charge(Dimension::Entries, 2)?;
                    result.selected.push(Selected::Domain(package.package));
                    result.models.push(w::Model {
                        artifact: dependency,
                        profile: text(DOMAIN_PACKAGE_PROFILE, work)?,
                        exports,
                        // FR-042-AC-11: the package this model was linked
                        // against, named directly.
                        domain_package: w::Nullable(Some(domain::naming(
                            package.package.selection(),
                            work,
                        )?)),
                    });
                }
            }
        }
        Ok(result)
    }

    pub(super) fn finish(self) -> (Vec<w::Type>, Vec<w::Model>) {
        (self.types, self.models)
    }

    /// Resolve only the actual admitted model, never an owner/name substitute.
    pub(super) fn export(
        &self,
        model: &NativeModel,
        kind: w::ExportKind,
        owner: &str,
        member: Option<&str>,
        work: &mut Work,
    ) -> Result<w::ExportRef, Error> {
        for (model_index, selected) in self.selected.iter().enumerate() {
            work.visit()?;
            let Selected::Native(selected) = selected else {
                continue;
            };
            if !std::ptr::eq(selected.model, model) {
                continue;
            }
            return self.find(model_index, kind, owner, member, work);
        }
        Err(Error::Invalid(Invalid::Model))
    }

    /// Resolve an export of the selected domain package `package`.
    pub(super) fn domain_export(
        &self,
        package: &AdmittedPackage,
        kind: w::ExportKind,
        owner: &str,
        member: Option<&str>,
        work: &mut Work,
    ) -> Result<w::ExportRef, Error> {
        for (model_index, selected) in self.selected.iter().enumerate() {
            work.visit()?;
            let Selected::Domain(selected) = selected else {
                continue;
            };
            if !std::ptr::eq(*selected, package) {
                continue;
            }
            return self.find(model_index, kind, owner, member, work);
        }
        Err(Error::Invalid(Invalid::Model))
    }

    /// The type export of a domain type: `object` for an object type or
    /// Interface, `record` for a record value type.
    pub(super) fn domain_type_export(
        &self,
        ty: &DomainType<'_>,
        work: &mut Work,
    ) -> Result<w::ExportRef, Error> {
        self.domain_export(
            ty.package,
            domain::type_export_kind(ty),
            ty.artifact_id(),
            None,
            work,
        )
    }

    fn find(
        &self,
        model_index: usize,
        kind: w::ExportKind,
        owner: &str,
        member: Option<&str>,
        work: &mut Work,
    ) -> Result<w::ExportRef, Error> {
        for (export_index, export) in self.models[model_index].exports.iter().enumerate() {
            work.visit()?;
            work.bytes(owner.len().saturating_add(member.map_or(0, str::len)))?;
            if export.kind == kind
                && export.path.first().is_some_and(|name| name == owner)
                && match member {
                    Some(member) => export.path.len() == 2 && export.path[1] == member,
                    None => export.path.len() == 1,
                }
            {
                return Ok(w::ExportRef {
                    model: index(model_index)?,
                    export: index(export_index)?,
                });
            }
        }
        Err(Error::Unsupported(Unsupported::Export))
    }

    /// Wrappers precede their children, exactly as the reader's first-use walk.
    /// Flat chain matching avoids recursively cloning or comparing native types.
    pub(super) fn ty(&mut self, ty: &NativeType<'_>, work: &mut Work) -> Result<u32, Error> {
        let mut shapes = Vec::new();
        let mut current = ty;
        loop {
            work.visit()?;
            work.charge(Dimension::Depth, shapes.len() + 1)?;
            work.charge(Dimension::Entries, 1)?;
            match current {
                NativeType::Option(value) => {
                    shapes.push(Shape::Option);
                    current = value;
                }
                NativeType::Sequence { element, maximum } => {
                    shapes.push(Shape::Sequence(*maximum));
                    current = element;
                }
                NativeType::Boolean
                | NativeType::Scalar { .. }
                | NativeType::Enumeration { .. }
                | NativeType::Record { .. }
                | NativeType::Object { .. }
                | NativeType::Reference { .. }
                | NativeType::Domain(_) => {
                    shapes.push(Shape::Leaf(self.leaf(current, work)?));
                    break;
                }
            }
        }
        let mut indices = Vec::new();
        let mut next = self.types.len();
        for offset in 0..shapes.len() {
            let mut found = None;
            for candidate in 0..self.types.len() {
                if self.matches(candidate, &shapes[offset..], work)? {
                    found = Some(index(candidate)?);
                    break;
                }
            }
            work.charge(Dimension::Entries, 1)?;
            indices.push(match found {
                Some(value) => value,
                None => {
                    let value = index(next)?;
                    next += 1;
                    value
                }
            });
        }
        let first = *indices.first().ok_or(Error::Invalid(Invalid::Type))?;
        for (offset, shape) in shapes.into_iter().enumerate() {
            if (indices[offset] as usize) < self.types.len() {
                continue;
            }
            if indices[offset] as usize != self.types.len() {
                return Err(Error::Invalid(Invalid::Type));
            }
            let child = || {
                indices
                    .get(offset + 1)
                    .copied()
                    .ok_or(Error::Invalid(Invalid::Type))
            };
            let value = match shape {
                Shape::Option => w::Type::Option { value: child()? },
                Shape::Sequence(maximum) => w::Type::Sequence {
                    element: child()?,
                    maximum: integer(i64::from(maximum), work)?,
                },
                Shape::Leaf(value) => value,
            };
            work.charge(Dimension::Entries, 1)?;
            self.types.push(value);
        }
        Ok(first)
    }

    fn matches(&self, mut at: usize, shapes: &[Shape], work: &mut Work) -> Result<bool, Error> {
        for shape in shapes {
            work.visit()?;
            let value = self
                .types
                .get(at)
                .ok_or(Error::Invalid(Invalid::Reference))?;
            match (shape, value) {
                (Shape::Option, w::Type::Option { value }) => at = *value as usize,
                (Shape::Sequence(expected), w::Type::Sequence { element, maximum }) => {
                    work.bytes(20)?;
                    let crate::protocol_artifact::ProtocolNumber::Integer(maximum) =
                        maximum.checked()?
                    else {
                        return Err(Error::Invalid(Invalid::Type));
                    };
                    if maximum.value() != i64::from(*expected) {
                        return Ok(false);
                    }
                    at = *element as usize;
                }
                (Shape::Leaf(expected), actual) => {
                    charge_leaf(expected, work)?;
                    return Ok(expected == actual);
                }
                _ => return Ok(false),
            }
        }
        Ok(false)
    }

    fn leaf(&self, ty: &NativeType<'_>, work: &mut Work) -> Result<w::Type, Error> {
        Ok(match ty {
            NativeType::Boolean => w::Type::Boolean {},
            NativeType::Scalar {
                model,
                role,
                representation,
            } => {
                let export =
                    self.export(model, w::ExportKind::Scalar, role.name.as_str(), None, work)?;
                let (unit, representation) = match (&role.kind, *representation) {
                    (ScalarKind::Integer { unit }, ir::ValueType::Integer { value }) => (
                        native_unit(unit, work)?,
                        w::Representation::Integer {
                            minimum: integer(value.minimum(), work)?,
                            maximum: integer(value.maximum(), work)?,
                        },
                    ),
                    (ScalarKind::Rational { unit }, ir::ValueType::Rational { value }) => (
                        native_unit(unit, work)?,
                        w::Representation::Rational {
                            numerator_minimum: integer(value.numerator_minimum(), work)?,
                            numerator_maximum: integer(value.numerator_maximum(), work)?,
                            maximum_denominator: integer(
                                i64::try_from(value.maximum_denominator())
                                    .map_err(|_| Error::Invalid(Invalid::NumericDomain))?,
                                work,
                            )?,
                        },
                    ),
                    (ScalarKind::Text { max_scalars }, ir::ValueType::Text) => (
                        w::Nullable(None),
                        w::Representation::Text {
                            maximum_scalars: integer(i64::from(*max_scalars), work)?,
                        },
                    ),
                    _ => return Err(Error::Invalid(Invalid::Type)),
                };
                w::Type::Scalar {
                    export,
                    unit,
                    representation,
                }
            }
            NativeType::Enumeration { model, declaration } => w::Type::Enum {
                export: self.export(
                    model,
                    w::ExportKind::Enum,
                    declaration.name().as_str(),
                    None,
                    work,
                )?,
            },
            NativeType::Record { model, declaration } => w::Type::Record {
                export: self.export(
                    model,
                    w::ExportKind::Record,
                    declaration.name().as_str(),
                    None,
                    work,
                )?,
            },
            NativeType::Object { model, role } => w::Type::Object {
                export: self.export(
                    model,
                    w::ExportKind::Object,
                    role.record.as_str(),
                    None,
                    work,
                )?,
            },
            NativeType::Reference { model, role } => w::Type::Reference {
                export: self.export(
                    model,
                    w::ExportKind::Reference,
                    role.reference.as_str(),
                    None,
                    work,
                )?,
                object: self.export(
                    model,
                    w::ExportKind::Object,
                    role.record.as_str(),
                    None,
                    work,
                )?,
                universe: self.export(
                    model,
                    w::ExportKind::Population,
                    role.record.as_str(),
                    Some(role.universe.as_str()),
                    work,
                )?,
            },
            NativeType::Domain(domain) => {
                let export = self.domain_type_export(domain, work)?;
                if domain.is_object() {
                    w::Type::Object { export }
                } else {
                    w::Type::Record { export }
                }
            }
            NativeType::Option(_) | NativeType::Sequence { .. } => {
                return Err(Error::Invalid(Invalid::Type))
            }
        })
    }
}

fn charge_leaf(value: &w::Type, work: &mut Work) -> Result<(), Error> {
    if let w::Type::Scalar {
        unit,
        representation,
        ..
    } = value
    {
        work.bytes(unit.0.as_ref().map_or(0, String::len))?;
        let integers: &[&w::Integer] = match representation {
            w::Representation::Integer { minimum, maximum } => &[minimum, maximum],
            w::Representation::Rational {
                numerator_minimum,
                numerator_maximum,
                maximum_denominator,
            } => &[numerator_minimum, numerator_maximum, maximum_denominator],
            w::Representation::Text { maximum_scalars } => &[maximum_scalars],
        };
        for number in integers {
            let NumberWire::Integer { decimal } = &number.0 else {
                return Err(Error::Invalid(Invalid::Type));
            };
            work.bytes(decimal.len())?;
        }
    }
    Ok(())
}

type ExportKey<'a> = (&'static str, &'a str, &'a str);

fn model_exports(selected: &AdmittedModel<'_>, work: &mut Work) -> Result<Vec<w::Export>, Error> {
    let model = selected.model;
    let mut exports = BTreeMap::new();
    for declaration in model.environment().types() {
        work.visit()?;
        match declaration {
            ir::TypeDeclaration::Record { declaration } => {
                let mut kind = w::ExportKind::Record;
                let mut source = declaration.source();
                for role in &model.roles().objects {
                    work.visit()?;
                    work.bytes(declaration.name().as_str().len())?;
                    if role.record == *declaration.name() {
                        kind = w::ExportKind::Object;
                        source = &role.source;
                        break;
                    }
                    if role.reference == *declaration.name() {
                        kind = w::ExportKind::Reference;
                        source = &role.source;
                        break;
                    }
                }
                insert_export(
                    &mut exports,
                    kind,
                    declaration.name().as_str(),
                    "",
                    source,
                    work,
                )?;
                for field in declaration.fields() {
                    insert_export(
                        &mut exports,
                        w::ExportKind::Field,
                        declaration.name().as_str(),
                        field.name().as_str(),
                        field.source(),
                        work,
                    )?;
                }
            }
            ir::TypeDeclaration::Enum { declaration } => {
                insert_export(
                    &mut exports,
                    w::ExportKind::Enum,
                    declaration.name().as_str(),
                    "",
                    declaration.source(),
                    work,
                )?;
                for variant in declaration.variants() {
                    insert_export(
                        &mut exports,
                        w::ExportKind::Variant,
                        declaration.name().as_str(),
                        variant.name().as_str(),
                        variant.source(),
                        work,
                    )?;
                }
            }
        }
    }
    for role in &model.roles().scalars {
        insert_export(
            &mut exports,
            w::ExportKind::Scalar,
            role.name.as_str(),
            "",
            &role.source,
            work,
        )?;
    }
    for role in &model.roles().objects {
        insert_export(
            &mut exports,
            w::ExportKind::Population,
            role.record.as_str(),
            role.universe.as_str(),
            &role.source,
            work,
        )?;
    }
    for role in &model.roles().operations {
        insert_export(
            &mut exports,
            w::ExportKind::Operation,
            role.context.as_str(),
            role.name.as_str(),
            &role.source,
            work,
        )?;
    }
    let mut result = Vec::new();
    for ((_, owner, member), (kind, source)) in exports {
        work.visit()?;
        let span = model
            .source()
            .to_native(source)
            .map_err(|_| Error::Invalid(Invalid::ForeignLocus))?;
        let mut path = Vec::new();
        work.charge(Dimension::Entries, if member.is_empty() { 1 } else { 2 })?;
        path.push(text(owner, work)?);
        if !member.is_empty() {
            path.push(text(member, work)?);
        }
        let source = copy_artifact(selected.source.artifact, work)?;
        let formal = copy_formal(selected.source.formal, work)?;
        work.charge(Dimension::Entries, 3)?;
        result.push(w::Export {
            kind,
            path,
            locus: w::ForeignLocus {
                source,
                formal,
                span: w::Span {
                    start: index(span.start)?,
                    end: index(span.end)?,
                },
            },
        });
    }
    Ok(result)
}

fn insert_export<'a>(
    exports: &mut BTreeMap<ExportKey<'a>, (w::ExportKind, &'a ir::SourceSpan)>,
    kind: w::ExportKind,
    owner: &'a str,
    member: &'a str,
    source: &'a ir::SourceSpan,
    work: &mut Work,
) -> Result<(), Error> {
    work.visit()?;
    // Charge the borrowed key supplied to this logical index operation.
    work.bytes(owner.len().saturating_add(member.len()))?;
    work.charge(Dimension::Entries, 1)?;
    if exports
        .insert((kind.as_str(), owner, member), (kind, source))
        .is_some()
    {
        return Err(Error::Invalid(Invalid::Duplicate));
    }
    Ok(())
}

fn native_unit(unit: &Unit, work: &mut Work) -> Result<w::Nullable<String>, Error> {
    Ok(w::Nullable(match unit {
        Unit::Dimensionless => None,
        Unit::Named(name) => Some(text(name.as_str(), work)?),
    }))
}

pub(super) fn index(value: usize) -> Result<u32, Error> {
    if value > 1_048_576 {
        return Err(Error::Invalid(Invalid::StructuralInteger));
    }
    u32::try_from(value).map_err(|_| Error::Invalid(Invalid::StructuralInteger))
}

pub(super) fn text(value: &str, work: &mut Work) -> Result<String, Error> {
    work.bytes(value.len())?;
    work.charge(Dimension::ContentBytes, value.len())?;
    Ok(value.to_owned())
}

pub(super) fn integer(value: i64, work: &mut Work) -> Result<w::Integer, Error> {
    let magnitude = value.unsigned_abs();
    let digits =
        magnitude.checked_ilog10().map_or(1, |log| log as usize + 1) + usize::from(value < 0);
    work.bytes(digits)?;
    work.charge(Dimension::ContentBytes, digits)?;
    Ok(w::Integer(NumberWire::Integer {
        decimal: value.to_string(),
    }))
}

fn copy_formal(value: &w::Formal, work: &mut Work) -> Result<w::Formal, Error> {
    work.charge(Dimension::Entries, 2)?;
    Ok(w::Formal {
        document: text(&value.document, work)?,
        revision: w::Revision {
            namespace: text(&value.revision.namespace, work)?,
            value: text(&value.revision.value, work)?,
        },
    })
}

fn copy_artifact(value: &w::ArtifactRef, work: &mut Work) -> Result<w::ArtifactRef, Error> {
    work.charge(Dimension::Entries, 3)?;
    Ok(w::ArtifactRef {
        ref_version: text(&value.ref_version, work)?,
        kind: value.kind,
        authority: text(&value.authority, work)?,
        identity: text(&value.identity, work)?,
        revision: w::Revision {
            namespace: text(&value.revision.namespace, work)?,
            value: text(&value.revision.value, work)?,
        },
        digest: value.digest,
        wire: w::Wire {
            identity: text(&value.wire.identity, work)?,
            version: text(&value.wire.version, work)?,
        },
    })
}
