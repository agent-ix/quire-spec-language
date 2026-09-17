// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-042: exact exports and value types from constructor-admitted native models.

mod populations;

use std::collections::BTreeMap;

use quire_contract_ir as ir;

use super::{
    wire as w, work::Work, AdmittedModel, Dimension, Error, Invalid, SuppliedDependency,
    Unsupported,
};
use crate::checking::{Catalog, NativeType};
use crate::native_model::{
    NativeModel, ObjectRole, OperationRole, ScalarKind, ScalarRole, ScalarSite, Unit,
};

type RefKey<'a> = (
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
);
type ExportKey = (String, Vec<String>);

#[derive(Clone, Copy)]
enum Target<'a> {
    Scalar(&'a ScalarRole, &'a ir::ValueType),
    Enum(&'a ir::EnumDeclaration),
    Variant(&'a ir::EnumDeclaration, &'a ir::EnumVariantDeclaration),
    Record(&'a ir::RecordDeclaration),
    Object(&'a ObjectRole, &'a ir::RecordDeclaration),
    Reference(&'a ObjectRole),
    Population(&'a ObjectRole),
    Field(&'a ir::RecordDeclaration, &'a ir::RecordFieldDeclaration),
    Operation(&'a OperationRole),
}

impl<'a> Target<'a> {
    fn source(self) -> Option<&'a ir::SourceSpan> {
        Some(match self {
            Self::Scalar(role, _) => &role.source,
            Self::Enum(value) => value.source(),
            Self::Variant(_, value) => value.source(),
            Self::Record(value) => value.source(),
            Self::Object(role, _) | Self::Reference(role) | Self::Population(role) => &role.source,
            Self::Field(_, value) => value.source(),
            Self::Operation(value) => &value.source,
        })
    }
}

struct View<'a> {
    catalog: Catalog<'a>,
    targets: Vec<Target<'a>>,
}

pub(super) fn validate(
    package: &w::Package,
    expected_models: &[AdmittedModel<'_>],
    expected_dependencies: &[SuppliedDependency<'_>],
    work: &mut Work,
) -> Result<Vec<NativeModel>, Error> {
    if package.models.len() != expected_models.len() {
        return Err(Error::Invalid(Invalid::Inventory));
    }
    work.charge(Dimension::Models, expected_models.len())?;
    let mut supplied = BTreeMap::new();
    let mut model_pointers = std::collections::BTreeSet::new();
    let mut owners = BTreeMap::new();
    let mut sources = BTreeMap::new();
    for model in expected_models {
        work.visit()?;
        charge_ref(model.artifact, work)?;
        work.charge(Dimension::Entries, 4)?;
        if supplied.insert(ref_key(model.artifact), model).is_some() {
            return Err(Error::Invalid(Invalid::Duplicate));
        }
        if !model_pointers.insert(std::ptr::from_ref(model.model)) {
            return Err(Error::Invalid(Invalid::Duplicate));
        }
        let owner = model.model.environment().owner();
        work.bytes(owner.package().as_str().len())?;
        work.bytes(owner.requirement().as_str().len())?;
        if owners
            .insert(owner, model.model.digest())
            .is_some_and(|digest| digest != model.model.digest())
        {
            return Err(Error::Invalid(Invalid::Model));
        }
        let formal = model.model.source();
        let source = formal.source();
        for text in [
            formal.identity().document().as_str(),
            source.identity().identity.as_str(),
            source.identity().revision.as_str(),
            source.path(),
        ] {
            work.bytes(text.len())?;
        }
        let correspondence = (source.identity(), source.path(), source.digest());
        if sources
            .insert(formal.identity(), correspondence)
            .is_some_and(|prior| prior != correspondence)
        {
            return Err(Error::Invalid(Invalid::ForeignLocus));
        }
    }
    let mut dependencies = BTreeMap::new();
    for dependency in expected_dependencies {
        work.visit()?;
        charge_ref(dependency.artifact, work)?;
        work.charge(Dimension::Entries, 1)?;
        if dependencies
            .insert(ref_key(dependency.artifact), dependency)
            .is_some()
        {
            return Err(Error::Invalid(Invalid::Duplicate));
        }
    }
    let mut views = Vec::new();
    let mut retained = Vec::new();
    retained
        .try_reserve(expected_models.len())
        .map_err(|_| Error::Allocation)?;
    let mut previous_model = None;
    for model in &package.models {
        work.visit()?;
        if previous_model.is_some_and(|previous| previous >= model.artifact) {
            return Err(Error::Invalid(Invalid::Order));
        }
        previous_model = Some(model.artifact);
        let artifact = &package
            .dependencies
            .get(model.artifact as usize)
            .ok_or(Error::Invalid(Invalid::Reference))?
            .artifact;
        charge_ref(artifact, work)?;
        let selected = supplied
            .get(&ref_key(artifact))
            .ok_or(Error::Invalid(Invalid::Model))?;
        same_ref(artifact, selected.artifact, work)?;
        work.bytes(model.profile.len())?;
        if artifact.kind != w::ArtifactKind::ModelPackage
            || artifact.digest != selected.model.digest()
            || model.profile != selected.model.profile().as_str()
        {
            return Err(Error::Invalid(Invalid::Model));
        }
        let dependency = dependencies
            .get(&ref_key(artifact))
            .ok_or(Error::Invalid(Invalid::Dependency))?;
        same_ref(artifact, dependency.artifact, work)?;
        same_bytes(
            dependency.bytes,
            selected.model.artifact_bytes(),
            work,
            Invalid::Model,
        )?;
        validate_source(selected, work)?;
        reserve_catalog(selected.model, work)?;
        let catalog = Catalog::composed(selected.model);
        let exports = exports(&catalog, work)?;
        // No admitted correspondence can authorize a model export without
        // the removed Producer 1.2 adapter (#131); a wire package claiming
        // one is always refused as unsupported.
        if model.correspondence.0.is_some() {
            return Err(Error::Unsupported(Unsupported::ProducerCorrespondence));
        }
        let mut targets = Vec::new();
        let mut previous_export: Option<ExportKey> = None;
        for export in &model.exports {
            work.visit()?;
            let key = export_key(export, work)?;
            if let Some(previous) = previous_export.as_ref() {
                work.visit()?;
                work.bytes(
                    previous
                        .0
                        .len()
                        .saturating_add(previous.1.iter().map(String::len).sum::<usize>()),
                )?;
                if previous >= &key {
                    return Err(Error::Invalid(Invalid::Order));
                }
            }
            previous_export = Some(key.clone());
            let target = match exports.get(&key).copied() {
                Some(target) => target,
                None if matches!(
                    export.kind,
                    w::ExportKind::Relationship
                        | w::ExportKind::Component
                        | w::ExportKind::Endpoint
                ) =>
                {
                    return Err(Error::Unsupported(Unsupported::Export));
                }
                None => return Err(Error::Invalid(Invalid::Model)),
            };
            let source = target.source().ok_or(Error::Invalid(Invalid::Model))?;
            validate_locus(export, source, selected, work)?;
            work.charge(Dimension::Entries, 1)?;
            targets.push(target);
        }
        work.charge(Dimension::Entries, 1)?;
        views.push(View { catalog, targets });
        retained.push(selected.model.clone());
    }
    for ty in &package.types {
        work.visit()?;
        validate_type(ty, &views, work)?;
    }
    validate_related(package, work)?;
    for declaration in &package.declarations {
        work.visit()?;
        work.locus = Some(declaration.locus.clone());
        validate_operations(package, declaration, &views, work)?;
        let populations = populations::Validation {
            package,
            declaration,
            views: &views,
        };
        populations.bindings(work)?;
        for value in &declaration.values {
            work.visit()?;
            work.locus = Some(value.locus.clone());
            match &value.operation {
                w::ValueOperation::Field { base, field } => {
                    let (view, member) = target(&views, field, work)?;
                    let Target::Field(record, field) = member else {
                        return Err(Error::Invalid(Invalid::Type));
                    };
                    let base = local_value(package, declaration, base)?;
                    let base_ty = package
                        .types
                        .get(base.value_type as usize)
                        .ok_or(Error::Invalid(Invalid::Reference))?;
                    let base_export = match base_ty {
                        w::Type::Record { export } | w::Type::Object { export } => export,
                        _ => return Err(Error::Invalid(Invalid::Type)),
                    };
                    let (base_view, base_target) = target(&views, base_export, work)?;
                    let base_record = match base_target {
                        Target::Record(record) | Target::Object(_, record) => record,
                        _ => return Err(Error::Invalid(Invalid::Type)),
                    };
                    if !same_model(view.catalog.model, base_view.catalog.model)
                        || base_record.name() != record.name()
                    {
                        return Err(Error::Invalid(Invalid::Type));
                    }
                    work.bytes(record.name().as_str().len())?;
                    work.bytes(field.name().as_str().len())?;
                    work.charge(Dimension::Entries, 1)?;
                    let site = ScalarSite::Field {
                        record: record.name().clone(),
                        field: field.name().clone(),
                    };
                    reserve_formal(field.value_type(), work)?;
                    let native = view
                        .catalog
                        .formal(field.value_type(), &site)
                        .ok_or(Error::Invalid(Invalid::Type))?;
                    matches_native(package, &views, value.value_type, &native, work)?;
                }
                w::ValueOperation::Enum { variant } => {
                    let (view, member) = target(&views, variant, work)?;
                    let Target::Variant(enumeration, _) = member else {
                        return Err(Error::Invalid(Invalid::Type));
                    };
                    let native = NativeType::Enumeration {
                        model: view.catalog.model,
                        declaration: enumeration,
                    };
                    matches_native(package, &views, value.value_type, &native, work)?;
                }
                w::ValueOperation::Reaches { .. } => {
                    populations.value(value, work)?;
                }
                // No native Parent operator is admitted by the composed frontend.
                // Population identity alone does not establish its result semantics.
                w::ValueOperation::Parent { .. } => {
                    return Err(Error::Unsupported(Unsupported::Feature));
                }
                w::ValueOperation::Boolean { .. }
                | w::ValueOperation::Number { .. }
                | w::ValueOperation::Text { .. }
                | w::ValueOperation::Read { .. }
                | w::ValueOperation::Group { .. }
                | w::ValueOperation::Unary { .. }
                | w::ValueOperation::Binary { .. }
                | w::ValueOperation::If { .. }
                | w::ValueOperation::Let { .. }
                | w::ValueOperation::Pre { .. }
                | w::ValueOperation::Call { .. }
                | w::ValueOperation::Size { .. }
                | w::ValueOperation::Contains { .. }
                | w::ValueOperation::Query { .. } => {}
            }
        }
    }
    Ok(retained)
}

// No admitted correspondence can authorize a relationship endpoint without
// the removed Producer 1.2 adapter (#131); any event occurrence that names
// a relationship is refused as unsupported, after confirming the name
// itself resolves.
fn validate_related(package: &w::Package, work: &mut Work) -> Result<(), Error> {
    for declaration in &package.declarations {
        let w::Body::Protocol {
            relationships,
            controls,
            ..
        } = &declaration.body
        else {
            continue;
        };
        for control in controls {
            let w::ControlOperation::Event { related, .. } = &control.operation else {
                continue;
            };
            let mut any = false;
            for occurrence in related {
                work.visit()?;
                relationships
                    .get(wire_index(occurrence.relationship)?)
                    .ok_or(Error::Invalid(Invalid::Reference))?;
                any = true;
            }
            if any {
                return Err(Error::Unsupported(Unsupported::Export));
            }
        }
    }
    Ok(())
}

fn wire_index(value: u32) -> Result<usize, Error> {
    usize::try_from(value).map_err(|_| Error::Invalid(Invalid::StructuralInteger))
}

fn ref_key(value: &w::ArtifactRef) -> RefKey<'_> {
    (
        value.kind.as_str(),
        &value.authority,
        &value.identity,
        &value.revision.namespace,
        &value.revision.value,
        &value.wire.identity,
        &value.wire.version,
    )
}

fn charge_ref(value: &w::ArtifactRef, work: &mut Work) -> Result<(), Error> {
    super::intake::reference(value)?;
    for text in [
        &value.ref_version,
        &value.authority,
        &value.identity,
        &value.revision.namespace,
        &value.revision.value,
        &value.wire.identity,
        &value.wire.version,
    ] {
        work.bytes(text.len())?;
    }
    work.bytes(32)
}

fn same_ref(left: &w::ArtifactRef, right: &w::ArtifactRef, work: &mut Work) -> Result<(), Error> {
    charge_ref(left, work)?;
    charge_ref(right, work)?;
    if left == right {
        Ok(())
    } else {
        Err(Error::Invalid(Invalid::Selection))
    }
}

fn same_bytes(left: &[u8], right: &[u8], work: &mut Work, cause: Invalid) -> Result<(), Error> {
    work.bytes(left.len())?;
    work.bytes(right.len())?;
    if left == right {
        Ok(())
    } else {
        Err(Error::Invalid(cause))
    }
}

fn validate_source(selected: &AdmittedModel<'_>, work: &mut Work) -> Result<(), Error> {
    let original = selected.model.source();
    let source = selected.source;
    super::intake::reference(source.artifact)?;
    super::intake::name(&source.formal.document)?;
    super::intake::revision(&source.formal.revision)?;
    work.charge(Dimension::SourceBytes, source.bytes.len())?;
    work.bytes(source.formal.document.len())?;
    work.bytes(source.formal.revision.value.len())?;
    let revision = &source.formal.revision.value;
    if source.artifact.kind != w::ArtifactKind::Source
        || source.artifact.digest != original.source().digest()
        || source.formal.document != original.identity().document().as_str()
        || revision.starts_with('0')
        || !revision.bytes().all(|byte| byte.is_ascii_digit())
        || revision.parse::<u64>().ok() != Some(original.identity().revision().get())
    {
        return Err(Error::Invalid(Invalid::ForeignLocus));
    }
    same_bytes(
        source.bytes,
        original.source().text().as_bytes(),
        work,
        Invalid::ForeignLocus,
    )
}

fn validate_locus(
    export: &w::Export,
    actual: &ir::SourceSpan,
    selected: &AdmittedModel<'_>,
    work: &mut Work,
) -> Result<(), Error> {
    super::intake::name(&export.locus.formal.document)?;
    super::intake::revision(&export.locus.formal.revision)?;
    same_ref(&export.locus.source, selected.source.artifact, work)?;
    for text in [
        &export.locus.formal.document,
        &export.locus.formal.revision.namespace,
        &export.locus.formal.revision.value,
        &selected.source.formal.document,
        &selected.source.formal.revision.namespace,
        &selected.source.formal.revision.value,
    ] {
        work.bytes(text.len())?;
    }
    if export.locus.formal != *selected.source.formal {
        return Err(Error::Invalid(Invalid::ForeignLocus));
    }
    work.visit()?;
    let span = selected
        .model
        .source()
        .to_native(actual)
        .map_err(|_| Error::Invalid(Invalid::ForeignLocus))?;
    if export.locus.span.start as usize != span.start || export.locus.span.end as usize != span.end
    {
        return Err(Error::Invalid(Invalid::ForeignLocus));
    }
    Ok(())
}

// Reserve all borrowed Catalog entries before its existing constructor allocates.
fn reserve_catalog(model: &NativeModel, work: &mut Work) -> Result<(), Error> {
    for declaration in model.environment().types() {
        work.visit()?;
        work.bytes(declaration.name().as_str().len())?;
        work.charge(Dimension::Entries, 2)?;
        match declaration {
            ir::TypeDeclaration::Record { declaration } => {
                for field in declaration.fields() {
                    work.visit()?;
                    work.bytes(field.name().as_str().len())?;
                    work.charge(Dimension::Entries, 2)?;
                }
            }
            ir::TypeDeclaration::Enum { declaration } => {
                for variant in declaration.variants() {
                    work.visit()?;
                    work.bytes(variant.name().as_str().len())?;
                    work.charge(Dimension::Entries, 1)?;
                }
            }
        }
    }
    for value in model.environment().values() {
        work.visit()?;
        work.bytes(value.name().as_str().len())?;
        work.charge(Dimension::Entries, 1)?;
    }
    for scalar in &model.roles().scalars {
        for site in &scalar.sites {
            work.visit()?;
            match site {
                ScalarSite::Value { name } => work.bytes(name.as_str().len())?,
                ScalarSite::Field { record, field } => {
                    work.bytes(record.as_str().len())?;
                    work.bytes(field.as_str().len())?;
                }
            }
            work.charge(Dimension::Entries, 1)?;
        }
    }
    for object in &model.roles().objects {
        work.visit()?;
        work.bytes(object.record.as_str().len())?;
        work.bytes(object.reference.as_str().len())?;
        work.charge(Dimension::Entries, 2)?;
    }
    for operation in &model.roles().operations {
        work.visit()?;
        work.bytes(operation.context.as_str().len())?;
        work.bytes(operation.name.as_str().len())?;
        work.charge(Dimension::Entries, 1)?;
        for (record, field) in &operation.frame.fields {
            work.visit()?;
            work.bytes(record.as_str().len())?;
            work.bytes(field.as_str().len())?;
            work.charge(Dimension::Entries, 1)?;
        }
        for name in operation
            .frame
            .created
            .iter()
            .chain(&operation.frame.deleted)
        {
            work.visit()?;
            work.bytes(name.as_str().len())?;
            work.charge(Dimension::Entries, 1)?;
        }
    }
    Ok(())
}

fn exports<'a>(
    catalog: &Catalog<'a>,
    work: &mut Work,
) -> Result<BTreeMap<ExportKey, Target<'a>>, Error> {
    let mut result = BTreeMap::new();
    for declaration in catalog.model.environment().types() {
        work.visit()?;
        match declaration {
            ir::TypeDeclaration::Record { declaration } => {
                let (kind, target) = match catalog.record_type(declaration.name()) {
                    Some(NativeType::Record { .. }) => {
                        (w::ExportKind::Record, Target::Record(declaration))
                    }
                    Some(NativeType::Object { role, .. }) => {
                        (w::ExportKind::Object, Target::Object(role, declaration))
                    }
                    Some(NativeType::Reference { role, .. }) => {
                        (w::ExportKind::Reference, Target::Reference(role))
                    }
                    _ => return Err(Error::Invalid(Invalid::Model)),
                };
                insert(
                    &mut result,
                    kind,
                    declaration.name().as_str(),
                    "",
                    target,
                    work,
                )?;
                for field in declaration.fields() {
                    insert(
                        &mut result,
                        w::ExportKind::Field,
                        declaration.name().as_str(),
                        field.name().as_str(),
                        Target::Field(declaration, field),
                        work,
                    )?;
                }
            }
            ir::TypeDeclaration::Enum { declaration } => {
                insert(
                    &mut result,
                    w::ExportKind::Enum,
                    declaration.name().as_str(),
                    "",
                    Target::Enum(declaration),
                    work,
                )?;
                for variant in declaration.variants() {
                    insert(
                        &mut result,
                        w::ExportKind::Variant,
                        declaration.name().as_str(),
                        variant.name().as_str(),
                        Target::Variant(declaration, variant),
                        work,
                    )?;
                }
            }
        }
    }
    for role in &catalog.model.roles().scalars {
        work.visit()?;
        let site = role.sites.first().ok_or(Error::Invalid(Invalid::Model))?;
        let ty = match site {
            ScalarSite::Value { name } => catalog.values.get(name).map(|value| value.value_type()),
            ScalarSite::Field { record, field } => catalog
                .fields
                .get(&(record, field))
                .map(|value| value.value_type()),
        }
        .ok_or(Error::Invalid(Invalid::Model))?;
        let primitive = primitive(ty, work)?;
        insert(
            &mut result,
            w::ExportKind::Scalar,
            role.name.as_str(),
            "",
            Target::Scalar(role, primitive),
            work,
        )?;
    }
    for operation in &catalog.model.roles().operations {
        insert(
            &mut result,
            w::ExportKind::Operation,
            operation.context.as_str(),
            operation.name.as_str(),
            Target::Operation(operation),
            work,
        )?;
    }
    for role in &catalog.model.roles().objects {
        insert(
            &mut result,
            w::ExportKind::Population,
            role.record.as_str(),
            role.universe.as_str(),
            Target::Population(role),
            work,
        )?;
    }
    Ok(result)
}

fn insert<'a>(
    index: &mut BTreeMap<ExportKey, Target<'a>>,
    kind: w::ExportKind,
    first: &'a str,
    second: &'a str,
    target: Target<'a>,
    work: &mut Work,
) -> Result<(), Error> {
    work.visit()?;
    work.bytes(first.len())?;
    work.bytes(second.len())?;
    work.charge(Dimension::Entries, 1)?;
    if index
        .insert(
            (
                kind.as_str().to_owned(),
                if second.is_empty() {
                    vec![first.to_owned()]
                } else {
                    vec![first.to_owned(), second.to_owned()]
                },
            ),
            target,
        )
        .is_some()
    {
        return Err(Error::Invalid(Invalid::Duplicate));
    }
    Ok(())
}

fn export_key(export: &w::Export, work: &mut Work) -> Result<ExportKey, Error> {
    let arity = match export.kind {
        w::ExportKind::Scalar
        | w::ExportKind::Enum
        | w::ExportKind::Record
        | w::ExportKind::Object
        | w::ExportKind::Reference => 1,
        w::ExportKind::Field
        | w::ExportKind::Variant
        | w::ExportKind::Operation
        | w::ExportKind::Population => 2,
        w::ExportKind::Relationship | w::ExportKind::Component | w::ExportKind::Endpoint => 0,
    };
    if (arity != 0 && export.path.len() != arity) || export.path.is_empty() {
        return Err(Error::Invalid(Invalid::Model));
    }
    for part in &export.path {
        super::intake::name(part)?;
        work.bytes(part.len())?;
    }
    Ok((export.kind.as_str().to_owned(), export.path.clone()))
}

fn primitive<'a>(mut ty: &'a ir::ValueType, work: &mut Work) -> Result<&'a ir::ValueType, Error> {
    let mut depth = 0;
    loop {
        work.visit()?;
        depth += 1;
        work.charge(Dimension::Depth, depth)?;
        ty = match ty {
            ir::ValueType::Option { value } => value,
            ir::ValueType::Collection { value } => value.element(),
            ir::ValueType::Boolean
            | ir::ValueType::Integer { .. }
            | ir::ValueType::Rational { .. }
            | ir::ValueType::Text
            | ir::ValueType::Enum { .. }
            | ir::ValueType::Record { .. } => return Ok(ty),
        };
    }
}

fn target<'a, 'v>(
    views: &'v [View<'a>],
    export: &w::ExportRef,
    work: &mut Work,
) -> Result<(&'v View<'a>, Target<'a>), Error> {
    work.visit()?;
    let view = views
        .get(export.model as usize)
        .ok_or(Error::Invalid(Invalid::Reference))?;
    let target = view
        .targets
        .get(export.export as usize)
        .copied()
        .ok_or(Error::Invalid(Invalid::Reference))?;
    Ok((view, target))
}

fn integer(value: &w::Integer, work: &mut Work) -> Result<i64, Error> {
    work.visit()?;
    match &value.0 {
        super::NumberWire::Integer { decimal } => work.bytes(decimal.len())?,
        super::NumberWire::Rational {
            numerator,
            denominator,
        } => {
            work.bytes(numerator.len())?;
            work.bytes(denominator.len())?;
        }
    }
    match value.checked()? {
        super::ProtocolNumber::Integer(value) => Ok(value.value()),
        super::ProtocolNumber::Rational(_) => Err(Error::Invalid(Invalid::WrongNumericKind)),
    }
}

/// The three references name one admitted role, not three matching labels.
fn reference_target<'a, 'v>(
    views: &'v [View<'a>],
    export: &w::ExportRef,
    object: &w::ExportRef,
    universe: &w::ExportRef,
    work: &mut Work,
) -> Result<(&'v View<'a>, &'a ObjectRole), Error> {
    let (view, reference) = target(views, export, work)?;
    let (_, object_target) = target(views, object, work)?;
    let (_, population) = target(views, universe, work)?;
    let (
        Target::Reference(reference),
        Target::Object(object_role, _),
        Target::Population(population),
    ) = (reference, object_target, population)
    else {
        return Err(Error::Invalid(Invalid::Type));
    };
    if export.model != object.model
        || export.model != universe.model
        || !std::ptr::eq(reference, object_role)
        || !std::ptr::eq(reference, population)
    {
        return Err(Error::Invalid(Invalid::Type));
    }
    Ok((view, reference))
}

fn validate_type(ty: &w::Type, views: &[View<'_>], work: &mut Work) -> Result<(), Error> {
    let valid = match ty {
        w::Type::Boolean {} | w::Type::Option { .. } => true,
        w::Type::Sequence { maximum, .. } => {
            (1..=i64::from(u32::MAX)).contains(&integer(maximum, work)?)
        }
        w::Type::Scalar {
            export,
            unit,
            representation,
        } => {
            let (_, target) = target(views, export, work)?;
            let Target::Scalar(role, actual) = target else {
                return Err(Error::Invalid(Invalid::Type));
            };
            scalar(role, actual, unit.0.as_deref(), representation, work)?
        }
        w::Type::Enum { export } => matches!(target(views, export, work)?.1, Target::Enum(_)),
        w::Type::Record { export } => matches!(target(views, export, work)?.1, Target::Record(_)),
        w::Type::Object { export } => matches!(target(views, export, work)?.1, Target::Object(..)),
        w::Type::Reference {
            export,
            object,
            universe,
        } => {
            reference_target(views, export, object, universe, work)?;
            true
        }
    };
    if valid {
        Ok(())
    } else {
        Err(Error::Invalid(Invalid::Type))
    }
}

fn scalar(
    role: &ScalarRole,
    actual: &ir::ValueType,
    unit: Option<&str>,
    representation: &w::Representation,
    work: &mut Work,
) -> Result<bool, Error> {
    let expected_unit = match &role.kind {
        ScalarKind::Integer { unit } | ScalarKind::Rational { unit } => match unit {
            Unit::Dimensionless => None,
            Unit::Named(name) => Some(name.as_str()),
        },
        ScalarKind::Text { .. } => None,
    };
    work.bytes(unit.map_or(0, str::len))?;
    work.bytes(expected_unit.map_or(0, str::len))?;
    if unit != expected_unit {
        return Ok(false);
    }
    Ok(match (actual, &role.kind, representation) {
        (
            ir::ValueType::Integer { value },
            ScalarKind::Integer { .. },
            w::Representation::Integer { minimum, maximum },
        ) => {
            integer(minimum, work)? == value.minimum() && integer(maximum, work)? == value.maximum()
        }
        (
            ir::ValueType::Rational { value },
            ScalarKind::Rational { .. },
            w::Representation::Rational {
                numerator_minimum,
                numerator_maximum,
                maximum_denominator,
            },
        ) => {
            integer(numerator_minimum, work)? == value.numerator_minimum()
                && integer(numerator_maximum, work)? == value.numerator_maximum()
                && u64::try_from(integer(maximum_denominator, work)?).ok()
                    == Some(value.maximum_denominator())
        }
        (
            ir::ValueType::Text,
            ScalarKind::Text { max_scalars },
            w::Representation::Text { maximum_scalars },
        ) => integer(maximum_scalars, work)? == i64::from(*max_scalars),
        _ => false,
    })
}

fn reserve_formal(mut ty: &ir::ValueType, work: &mut Work) -> Result<(), Error> {
    let mut depth = 0;
    loop {
        work.visit()?;
        depth += 1;
        work.charge(Dimension::Depth, depth)?;
        work.charge(Dimension::Entries, 1)?;
        ty = match ty {
            ir::ValueType::Option { value } => value,
            ir::ValueType::Collection { value } => value.element(),
            ir::ValueType::Boolean
            | ir::ValueType::Integer { .. }
            | ir::ValueType::Rational { .. }
            | ir::ValueType::Text
            | ir::ValueType::Enum { .. }
            | ir::ValueType::Record { .. } => return Ok(()),
        };
    }
}

fn same_model(left: &NativeModel, right: &NativeModel) -> bool {
    left.environment().owner() == right.environment().owner() && left.digest() == right.digest()
}

fn matches_native(
    package: &w::Package,
    views: &[View<'_>],
    mut index: u32,
    mut native: &NativeType<'_>,
    work: &mut Work,
) -> Result<(), Error> {
    let mut depth = 0;
    loop {
        work.visit()?;
        depth += 1;
        work.charge(Dimension::Depth, depth)?;
        let wire = package
            .types
            .get(index as usize)
            .ok_or(Error::Invalid(Invalid::Reference))?;
        let valid = match (wire, native) {
            (w::Type::Option { value }, NativeType::Option(inner)) => {
                index = *value;
                native = inner;
                continue;
            }
            (
                w::Type::Sequence { element, maximum },
                NativeType::Sequence {
                    element: inner,
                    maximum: actual,
                },
            ) => {
                if integer(maximum, work)? != i64::from(*actual) {
                    return Err(Error::Invalid(Invalid::Type));
                }
                index = *element;
                native = inner;
                continue;
            }
            (w::Type::Boolean {}, NativeType::Boolean) => true,
            (w::Type::Scalar { export, .. }, NativeType::Scalar { model, role, .. }) => {
                let (view, selected) = target(views, export, work)?;
                matches!(selected, Target::Scalar(actual, _) if same_model(view.catalog.model, model) && actual.name == role.name)
            }
            (w::Type::Enum { export }, NativeType::Enumeration { model, declaration }) => {
                let (view, selected) = target(views, export, work)?;
                matches!(selected, Target::Enum(actual) if same_model(view.catalog.model, model) && actual.name() == declaration.name())
            }
            (w::Type::Record { export }, NativeType::Record { model, declaration }) => {
                let (view, selected) = target(views, export, work)?;
                matches!(selected, Target::Record(actual) if same_model(view.catalog.model, model) && actual.name() == declaration.name())
            }
            (w::Type::Object { export }, NativeType::Object { model, role }) => {
                let (view, selected) = target(views, export, work)?;
                matches!(selected, Target::Object(actual, _) if same_model(view.catalog.model, model) && actual == *role)
            }
            (
                w::Type::Reference {
                    export,
                    object,
                    universe,
                },
                NativeType::Reference { model, role },
            ) => {
                let (view, actual) = reference_target(views, export, object, universe, work)?;
                same_model(view.catalog.model, model) && actual == *role
            }
            _ => false,
        };
        return if valid {
            Ok(())
        } else {
            Err(Error::Invalid(Invalid::Type))
        };
    }
}

fn local_value<'a>(
    package: &'a w::Package,
    declaration: &w::Declaration,
    handle: &w::Handle,
) -> Result<&'a w::Value, Error> {
    let owner = package
        .declarations
        .get(handle.declaration as usize)
        .ok_or(Error::Invalid(Invalid::Reference))?;
    if !std::ptr::eq(owner, declaration) {
        return Err(Error::Invalid(Invalid::Owner));
    }
    owner
        .values
        .get(handle.index as usize)
        .ok_or(Error::Invalid(Invalid::Reference))
}

fn validate_operations(
    package: &w::Package,
    declaration: &w::Declaration,
    views: &[View<'_>],
    work: &mut Work,
) -> Result<(), Error> {
    work.locus = Some(declaration.locus.clone());
    match &declaration.execution {
        w::Execution::Pre { operation } | w::Execution::Post { operation } => {
            if !matches!(target(views, operation, work)?.1, Target::Operation(_)) {
                return Err(Error::Invalid(Invalid::Type));
            }
        }
        w::Execution::Initialization { .. } | w::Execution::Handler { .. } => {}
    }
    match &declaration.body {
        w::Body::State {
            context, operation, ..
        } => {
            if let Some(operation) = &operation.0 {
                operation_context(views, context, operation, work)?;
            }
        }
        w::Body::Protocol {
            roles,
            controls,
            compensations,
            ..
        } => {
            for role in roles {
                work.visit()?;
                work.locus = Some(role.locus.clone());
                match target(views, &role.model, work)?.1 {
                    Target::Object(..) => {}
                    _ => return Err(Error::Invalid(Invalid::Type)),
                }
            }
            for control in controls {
                work.visit()?;
                work.locus = Some(control.locus.clone());
                match &control.operation {
                    w::ControlOperation::Event { event, .. } => match event {
                        w::Event::Attempt {
                            owner, operation, ..
                        } => {
                            let role = role(package, declaration, roles, owner, work)?;
                            operation_context(views, &role.model, operation, work)?;
                        }
                        w::Event::Send { .. }
                        | w::Event::Receive { .. }
                        | w::Event::Effect { .. }
                        | w::Event::Event { .. } => {}
                    },
                    w::ControlOperation::Sequence { .. }
                    | w::ControlOperation::Choice { .. }
                    | w::ControlOperation::Parallel { .. }
                    | w::ControlOperation::Repeat { .. }
                    | w::ControlOperation::Await { .. }
                    | w::ControlOperation::Check { .. }
                    | w::ControlOperation::Commit { .. } => {}
                }
            }
            for compensation in compensations {
                work.visit()?;
                work.locus = Some(compensation.locus.clone());
                let role = role(package, declaration, roles, &compensation.owner, work)?;
                operation_context(views, &role.model, &compensation.operation, work)?;
            }
        }
        w::Body::Predicate { .. } | w::Body::Temporal { .. } => {}
    }
    Ok(())
}

fn role<'a>(
    package: &w::Package,
    declaration: &w::Declaration,
    roles: &'a [w::Role],
    handle: &w::Handle,
    work: &mut Work,
) -> Result<&'a w::Role, Error> {
    work.visit()?;
    let owner = package
        .declarations
        .get(handle.declaration as usize)
        .ok_or(Error::Invalid(Invalid::Reference))?;
    if !std::ptr::eq(owner, declaration) {
        return Err(Error::Invalid(Invalid::Owner));
    }
    roles
        .get(handle.index as usize)
        .ok_or(Error::Invalid(Invalid::Reference))
}

fn operation_context(
    views: &[View<'_>],
    context: &w::ExportRef,
    operation: &w::ExportRef,
    work: &mut Work,
) -> Result<(), Error> {
    let (context_model, context) = target(views, context, work)?;
    let (operation_model, operation) = target(views, operation, work)?;
    let (Target::Object(_, context), Target::Operation(operation)) = (context, operation) else {
        return Err(Error::Invalid(Invalid::Type));
    };
    work.bytes(context.name().as_str().len())?;
    work.bytes(operation.context.as_str().len())?;
    if !same_model(context_model.catalog.model, operation_model.catalog.model)
        || context.name() != &operation.context
    {
        return Err(Error::Invalid(Invalid::Type));
    }
    Ok(())
}
