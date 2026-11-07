// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-042: exact exports and value types from admitted domain-package models.

mod populations;

use std::collections::{BTreeMap, BTreeSet};

use super::domain::{self, DomainTarget};
use super::{
    wire as w, work::Work, AdmittedDomainPackage, Dimension, Error, Invalid, SuppliedDependency,
    Unsupported,
};
use crate::checking::{DomainField, NativeType};
use crate::number::{NumberWire, ProtocolNumber};
use qsl_semantics::model::domain_package::DOMAIN_PACKAGE_PROFILE;

type RefKey<'a> = (&'a str, &'a str, &'a str, &'a str, &'a str);
type ExportKey = (String, Vec<String>);

/// One selected domain-package model's export targets.
struct View<'a> {
    targets: Vec<DomainTarget<'a>>,
}

/// Checks every wire `Model` against its independently admitted domain package.
pub(super) fn validate(
    package: &w::Package,
    expected_domains: &[AdmittedDomainPackage<'_>],
    expected_dependencies: &[SuppliedDependency<'_>],
    work: &mut Work,
) -> Result<(), Error> {
    if package.models.len() != expected_domains.len() {
        return Err(Error::Invalid(Invalid::Inventory));
    }
    work.charge(Dimension::Models, expected_domains.len())?;
    let mut domains = BTreeMap::new();
    let mut domain_pointers = BTreeSet::new();
    for domain in expected_domains {
        work.visit()?;
        charge_ref(domain.artifact, work)?;
        work.charge(Dimension::Entries, 2)?;
        let key = ref_key(domain.artifact);
        if domains.insert(key, domain).is_some()
            || !domain_pointers.insert(std::ptr::from_ref(domain.package))
        {
            return Err(Error::Invalid(Invalid::Duplicate));
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
        let domain = domains
            .get(&ref_key(artifact))
            .ok_or(Error::Invalid(Invalid::Model))?;
        let targets = domain_model(model, artifact, domain, &dependencies, work)?;
        work.charge(Dimension::Entries, 1)?;
        views.push(View { targets });
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
                    let DomainTarget::Field(owner, name) = target(&views, field, work)? else {
                        return Err(Error::Invalid(Invalid::Type));
                    };
                    let native =
                        domain_field(package, &views, declaration, base, owner, name, work)?;
                    matches_native(package, &views, value.value_type, &native, work)?;
                }
                w::ValueOperation::Enum { variant } => {
                    target(&views, variant, work)?;
                    return Err(Error::Invalid(Invalid::Type));
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
    Ok(())
}

/// FR-042-AC-11: a wire `Model` selecting a domain package names exactly the
/// package FR-056 admitted at linking, selects that package's document
/// dependency, and carries only exports of that package, each located at
/// the whole document. Returns its export targets.
#[ix_trace_rs::string_edge]
fn domain_model<'a>(
    model: &w::Model,
    artifact: &w::ArtifactRef,
    domain: &AdmittedDomainPackage<'a>,
    dependencies: &BTreeMap<RefKey<'_>, &SuppliedDependency<'_>>,
    work: &mut Work,
) -> Result<Vec<DomainTarget<'a>>, Error> {
    same_ref(artifact, domain.artifact, work)?;
    work.bytes(model.profile.len())?;
    let selection = domain.package.selection();
    let Some(naming) = &model.domain_package.0 else {
        return Err(Error::Invalid(Invalid::Model));
    };
    work.bytes(naming.identity.len())?;
    if artifact.kind != w::ArtifactKind::ModelPackage
        || model.profile != DOMAIN_PACKAGE_PROFILE
        || naming.identity != selection.identity
    {
        return Err(Error::Invalid(Invalid::Model));
    }
    let dependency = dependencies
        .get(&ref_key(artifact))
        .ok_or(Error::Invalid(Invalid::Dependency))?;
    same_ref(artifact, dependency.artifact, work)?;
    let intake = work.limits.intake;
    domain::verify_document(selection, dependency.bytes, intake, work)?;
    let locus = domain::locus(domain, dependency.bytes.len(), work)?;
    let mut exports = BTreeMap::new();
    for export in domain::exports(domain.package, work)? {
        let mut path = vec![export.owner.to_owned()];
        path.extend(export.member.map(str::to_owned));
        exports.insert((export.kind.as_str().to_owned(), path), export.target);
    }
    let mut targets = Vec::new();
    let mut previous_export: Option<ExportKey> = None;
    for export in &model.exports {
        work.visit()?;
        let key = export_key(export, work)?;
        if let Some(previous) = previous_export.as_ref() {
            work.visit()?;
            if previous >= &key {
                return Err(Error::Invalid(Invalid::Order));
            }
        }
        previous_export = Some(key.clone());
        let target = exports
            .get(&key)
            .copied()
            .ok_or(Error::Invalid(Invalid::Model))?;
        charge_ref(&export.locus.source, work)?;
        work.bytes(
            export
                .locus
                .formal
                .document
                .len()
                .saturating_add(export.locus.formal.revision.value.len()),
        )?;
        if export.locus != locus {
            return Err(Error::Invalid(Invalid::ForeignLocus));
        }
        work.charge(Dimension::Entries, 1)?;
        targets.push(target);
    }
    Ok(targets)
}

// No admitted model carries a relationship authority; any event occurrence
// that names a relationship is refused as unsupported, after confirming the
// name itself resolves. The bounds check above is real and reachable from
// untrusted wire bytes (see the out-of-range `Related` mutant in
// tests/protocol_artifact.rs); the `Unsupported::Export` refusal below it is
// not, in this build: a relationship export already refuses as
// `Unsupported::Export` in the model loop above this function's caller, and
// `Graph::protocol` (validate/control.rs) independently refuses every
// relationship binding requirement, so a `relationships[i]` entry that
// resolves here can never exist. Kept, not deleted, because it is
// reachable data-in/data-out logic over the wire schema, not compiler
// state, and a future relaxation of either check would change that.
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
            for occurrence in related {
                work.visit()?;
                relationships
                    .get(wire_index(occurrence.relationship)?)
                    .ok_or(Error::Invalid(Invalid::Reference))?;
            }
            if !related.is_empty() {
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
        &value.wire.identity,
        &value.wire.version,
    ] {
        work.bytes(text.len())?;
    }
    Ok(())
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

fn target<'a>(
    views: &[View<'a>],
    export: &w::ExportRef,
    work: &mut Work,
) -> Result<DomainTarget<'a>, Error> {
    work.visit()?;
    let view = views
        .get(export.model as usize)
        .ok_or(Error::Invalid(Invalid::Reference))?;
    let target = view
        .targets
        .get(export.export as usize)
        .copied()
        .ok_or(Error::Invalid(Invalid::Reference))?;
    Ok(target)
}

fn integer(value: &w::Integer, work: &mut Work) -> Result<i64, Error> {
    work.visit()?;
    match &value.0 {
        NumberWire::Integer { decimal } => work.bytes(decimal.len())?,
        NumberWire::Rational {
            numerator,
            denominator,
        } => {
            work.bytes(numerator.len())?;
            work.bytes(denominator.len())?;
        }
    }
    match value.checked()? {
        ProtocolNumber::Integer(value) => Ok(value.value()),
        ProtocolNumber::Rational(_) => Err(Error::Invalid(Invalid::WrongNumericKind)),
    }
}

fn validate_type(ty: &w::Type, views: &[View<'_>], work: &mut Work) -> Result<(), Error> {
    let valid = match ty {
        w::Type::Boolean {} | w::Type::Option { .. } => true,
        w::Type::Sequence { maximum, .. } => {
            (1..=i64::from(u32::MAX)).contains(&integer(maximum, work)?)
        }
        // A domain package exports no scalar, enumeration or reference type.
        w::Type::Scalar { export, .. } | w::Type::Enum { export } => {
            target(views, export, work)?;
            false
        }
        w::Type::Record { export } => match target(views, export, work)? {
            DomainTarget::Type(ty) => !ty.is_object(),
            _ => false,
        },
        w::Type::Object { export } => match target(views, export, work)? {
            DomainTarget::Type(ty) => ty.is_object(),
            _ => false,
        },
        w::Type::Reference {
            export,
            object,
            universe,
        } => {
            target(views, export, work)?;
            target(views, object, work)?;
            target(views, universe, work)?;
            false
        }
    };
    if valid {
        Ok(())
    } else {
        Err(Error::Invalid(Invalid::Type))
    }
}

/// Whether two domain types are one declaration under one selection.
fn same_domain_type(
    left: &crate::checking::DomainType<'_>,
    right: &crate::checking::DomainType<'_>,
    work: &mut Work,
) -> Result<bool, Error> {
    work.bytes(
        left.declaration
            .key
            .node
            .len()
            .saturating_add(right.declaration.key.node.len()),
    )?;
    Ok(left.package.selection() == right.package.selection()
        && left.declaration.key == right.declaration.key)
}

/// The type of domain field `name` of `owner`, read through `base`, whose
/// wire type must be that same domain type.
fn domain_field<'a>(
    package: &w::Package,
    views: &[View<'a>],
    declaration: &w::Declaration,
    base: &w::Handle,
    owner: crate::checking::DomainType<'a>,
    name: &str,
    work: &mut Work,
) -> Result<NativeType<'a>, Error> {
    let base = local_value(package, declaration, base)?;
    let base_export = match package
        .types
        .get(base.value_type as usize)
        .ok_or(Error::Invalid(Invalid::Reference))?
    {
        w::Type::Record { export } | w::Type::Object { export } => export,
        _ => return Err(Error::Invalid(Invalid::Type)),
    };
    let DomainTarget::Type(base_type) = target(views, base_export, work)? else {
        return Err(Error::Invalid(Invalid::Type));
    };
    if !same_domain_type(&base_type, &owner, work)? {
        return Err(Error::Invalid(Invalid::Type));
    }
    work.bytes(name.len())?;
    work.charge(Dimension::Entries, 1)?;
    match owner.field(name) {
        DomainField::Typed(native) => Ok(native),
        DomainField::Unrepresented => Err(Error::Unsupported(Unsupported::Export)),
        DomainField::Missing => Err(Error::Invalid(Invalid::Type)),
    }
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
            (
                w::Type::Record { export } | w::Type::Object { export },
                NativeType::Domain(expected),
            ) => {
                let selected = target(views, export, work)?;
                matches!(wire, w::Type::Object { .. }) == expected.is_object()
                    && matches!(selected, DomainTarget::Type(actual)
                        if same_domain_type(&actual, expected, work)?)
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
            match target(views, operation, work)? {
                DomainTarget::Operation(_) => {}
                _ => return Err(Error::Invalid(Invalid::Type)),
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
                match target(views, &role.model, work)? {
                    DomainTarget::Type(ty) if ty.is_object() => {}
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
    let context_index = context.model;
    let operation_index = operation.model;
    let context = target(views, context, work)?;
    let operation = target(views, operation, work)?;
    // A domain operation's context is exactly its owning object type,
    // in the same domain-package model.
    match (context, operation) {
        (DomainTarget::Type(context), DomainTarget::Operation(owner)) => {
            if context_index != operation_index || !same_domain_type(&context, &owner, work)? {
                return Err(Error::Invalid(Invalid::Type));
            }
        }
        _ => return Err(Error::Invalid(Invalid::Type)),
    }
    Ok(())
}
