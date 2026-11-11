// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-569: the unit's own `object` declarations as one synthetic domain
//! package, so a source object type is admitted, normalized, dispatched and
//! checked by the code that handles an object type a selected domain package
//! declares.
//!
//! [`build`] checks the declarations (supertypes, identity, members,
//! redefinitions, frames and types) and builds the same normalized records
//! domain intake builds: an [`ObjectTypeRecord`] per type, a
//! [`FieldMemberRecord`] per field, an [`OperationMemberRecord`] per
//! operation and a [`PopulationRecord`] per population. The package is
//! selected under the empty alias, which the unit cannot spell, and a name
//! of that package is written without a qualifier.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use qsl_forms::{
    BuiltinType, FrameEntryForm, ObjectFieldForm, ObjectForm, ObjectMemberForm,
    ObjectOperationForm, TypeForm, TypeFormHead,
};
use qsl_foundation::Span;
use quire_checked::key::DeclarationKey;
use quire_exact::Presence;

use super::assemble::{AssemblyCause, AssemblyError, SourceObjectFault};
use super::lowering::strongly_connected;
use super::type_form::{resolve_form, TypeNames};
use crate::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Extent, FieldMemberRecord, Multiplicity,
    NativeValueType, ObjectTypeRecord, OperationEffect, OperationMemberRecord,
    OperationParameterRecord, OperationResult, PopulationRecord, ScalarTypeRecord,
    ScalarValueDomain, ValueTypeRef,
};

/// The alias the source model is selected under: empty, so no `model`
/// declaration can take it and a qualified name never reaches it.
pub(crate) const ALIAS: &str = "";

/// `member` of the model selected under `alias`, as the unit writes it: the
/// member alone for the source model, `alias::member` for a domain package.
pub(crate) fn qualified(alias: &str, member: &str) -> String {
    if is_source_alias(alias) {
        member.to_owned()
    } else {
        format!("{alias}::{member}")
    }
}

/// Whether `alias` is the source model's: the model selected under no alias,
/// so the empty alias is the absence of a qualifier, not a spelling to match.
pub(crate) fn is_source_alias(alias: &str) -> bool {
    alias.is_empty()
}

/// Whether `name` is written without a qualifier, so it names a member of the
/// source model.
pub(crate) fn is_unqualified(name: &str) -> bool {
    is_source_alias(alias_of(name))
}

/// The alias a type name is written under: its first segment when qualified,
/// the source model's empty alias when not.
pub(crate) fn alias_of(name: &str) -> &str {
    name.split_once("::").map_or(ALIAS, |(alias, _)| alias)
}

/// What the build needs to know of the rest of the unit.
pub(super) struct Context<'a> {
    /// The source model's package identity.
    pub identity: &'a str,
    /// The names of the unit's other type declarations (records, tuples,
    /// unions, enums and aliases), which an `extends` entry may name by
    /// mistake.
    pub declared_types: &'a BTreeSet<String>,
}

/// One member's position: the declaring object and the index in its forms.
#[derive(Clone, Copy)]
struct Located {
    owner: usize,
    member: usize,
}

/// The source model's object declarations, indexed.
struct Model<'a> {
    objects: &'a [ObjectForm],
    /// Each object's index by name; the first declaration of a name.
    by_name: BTreeMap<&'a str, usize>,
    /// Each object's resolved direct supertypes, by index.
    supertypes: Vec<Vec<usize>>,
    /// Each object's proper ancestors, nearest first; empty for a type in a
    /// cycle.
    ancestors: Vec<Vec<usize>>,
}

impl<'a> Model<'a> {
    fn named(&self, name: &str) -> Option<usize> {
        self.by_name.get(name).copied()
    }

    /// The fields `owner` declares or inherits, by name, nearest first.
    fn field(&self, object: usize, name: &str) -> Option<Located> {
        self.chain(object)
            .find_map(|owner| self.own_field(owner, name))
    }

    /// The operation `object` declares or inherits under `name`, nearest
    /// first.
    fn operation(&self, object: usize, name: &str) -> Option<Located> {
        self.chain(object)
            .find_map(|owner| self.own_operation(owner, name))
    }

    /// The field `owner` itself declares under `name`.
    fn own_field(&self, owner: usize, name: &str) -> Option<Located> {
        self.objects[owner]
            .members
            .iter()
            .position(|member| matches!(member, ObjectMemberForm::Field(field) if field.name.name == name))
            .map(|member| Located { owner, member })
    }

    /// The operation `owner` itself declares under `name`.
    fn own_operation(&self, owner: usize, name: &str) -> Option<Located> {
        self.objects[owner]
            .members
            .iter()
            .position(|member| matches!(member, ObjectMemberForm::Operation(operation) if operation.name.name == name))
            .map(|member| Located { owner, member })
    }

    /// `object` and its ancestors, nearest first.
    fn chain(&self, object: usize) -> impl Iterator<Item = usize> + '_ {
        std::iter::once(object).chain(self.ancestors[object].iter().copied())
    }

    fn type_key(&self, identity: &str, object: usize) -> DeclarationKey {
        node_key(identity, &self.objects[object].name.name)
    }
}

fn node_key(identity: &str, node: &str) -> DeclarationKey {
    DeclarationKey {
        package: identity.to_owned(),
        node: format!("ix://{identity}/{node}"),
    }
}

fn member_key(identity: &str, owner: &str, member: &str) -> DeclarationKey {
    node_key(identity, &format!("{owner}/{member}"))
}

fn one() -> Multiplicity {
    Multiplicity {
        lower: 1,
        upper: Some(1),
        ordered: false,
        unique: true,
    }
}

fn missing(name: &str, span: Span) -> AssemblyError {
    AssemblyError {
        cause: AssemblyCause::UnresolvedTypeName {
            name: name.to_owned(),
        },
        span,
    }
}

/// A field or operation of an object type that the type neither declares nor
/// inherits.
fn missing_member(name: &str, span: Span) -> AssemblyError {
    fault(
        SourceObjectFault::MissingMember {
            name: name.to_owned(),
        },
        span,
    )
}

fn ambiguous(name: &str, candidates: Vec<Span>, span: Span) -> AssemblyError {
    AssemblyError {
        cause: AssemblyCause::AmbiguousTypeName {
            name: name.to_owned(),
            candidates,
            native: false,
        },
        span,
    }
}

fn fault(fault: SourceObjectFault, span: Span) -> AssemblyError {
    AssemblyError {
        cause: AssemblyCause::SourceObject(fault),
        span,
    }
}

/// The unit's object declarations as a synthetic domain package, or every
/// refusal the declarations earn.
pub(super) fn build(
    objects: &[ObjectForm],
    context: &Context<'_>,
) -> Result<DomainPackage, Vec<AssemblyError>> {
    let mut errors = Vec::new();
    let mut by_name = BTreeMap::new();
    for (index, object) in objects.iter().enumerate() {
        by_name.entry(object.name.name.as_str()).or_insert(index);
    }
    let mut model = Model {
        objects,
        by_name,
        supertypes: vec![Vec::new(); objects.len()],
        ancestors: vec![Vec::new(); objects.len()],
    };
    resolve_supertypes(&mut model, context, &mut errors);
    for index in 0..objects.len() {
        check_object(&model, index, &mut errors);
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut records = Vec::new();
    let mut scalars = Vec::new();
    for index in 0..objects.len() {
        records_of(
            &model,
            index,
            context,
            &mut records,
            &mut scalars,
            &mut errors,
        );
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    records.extend(scalars);
    Ok(DomainPackage::new(
        DomainPackageRef {
            identity: context.identity.to_owned(),
        },
        records,
    ))
}

/// Each object's `extends` entries resolved to source object types, the
/// supertype cycles refused, and each acyclic type's ancestors.
fn resolve_supertypes(
    model: &mut Model<'_>,
    context: &Context<'_>,
    errors: &mut Vec<AssemblyError>,
) {
    let mut edges: Vec<Vec<usize>> = vec![Vec::new(); model.objects.len()];
    for (index, object) in model.objects.iter().enumerate() {
        for supertype in &object.extends {
            if supertype.is_qualified() || context.declared_types.contains(&supertype.name) {
                errors.push(fault(
                    SourceObjectFault::NotAnObjectType {
                        name: supertype.name.clone(),
                    },
                    supertype.span,
                ));
            } else if let Some(target) = model.named(&supertype.name) {
                if !edges[index].contains(&target) {
                    edges[index].push(target);
                }
            } else {
                errors.push(missing(&supertype.name, supertype.span));
            }
        }
    }
    let mut cyclic = BTreeSet::new();
    for component in strongly_connected(&edges) {
        let loops = component.len() > 1
            || component
                .first()
                .is_some_and(|member| edges[*member].contains(member));
        if !loops {
            continue;
        }
        let members: BTreeSet<usize> = component.iter().copied().collect();
        let mut cycle = Vec::new();
        for &member in &component {
            cyclic.insert(member);
            for &target in &edges[member] {
                if members.contains(&target) {
                    cycle.push((
                        model.objects[member].name.name.clone(),
                        model.objects[target].name.name.clone(),
                    ));
                }
            }
        }
        cycle.sort();
        cycle.dedup();
        let first = component.iter().min().copied().unwrap_or_default();
        errors.push(AssemblyError {
            cause: AssemblyCause::SourceObject(SourceObjectFault::SupertypeCycle { edges: cycle }),
            span: model.objects[first]
                .extends
                .first()
                .map_or(model.objects[first].name.span, |entry| entry.span),
        });
    }
    for index in 0..model.objects.len() {
        if cyclic.contains(&index) {
            continue;
        }
        let mut ancestors = Vec::new();
        let mut seen = BTreeSet::from([index]);
        let mut frontier: VecDeque<usize> = edges[index].iter().copied().collect();
        while let Some(next) = frontier.pop_front() {
            if cyclic.contains(&next) || !seen.insert(next) {
                continue;
            }
            ancestors.push(next);
            frontier.extend(edges[next].iter().copied());
        }
        model.ancestors[index] = ancestors;
    }
    model.supertypes = edges;
}

/// The checks of one object declaration that need no other record: its
/// identity, its members' names, its redefinitions and its frames.
fn check_object(model: &Model<'_>, index: usize, errors: &mut Vec<AssemblyError>) {
    let object = &model.objects[index];
    let mut seen: BTreeMap<&str, Span> = BTreeMap::new();
    for member in &object.members {
        let (name, span) = match member {
            ObjectMemberForm::Field(field) => (&field.name.name, field.name.span),
            ObjectMemberForm::Population(population) => {
                (&population.name.name, population.name.span)
            }
            ObjectMemberForm::Operation(operation) => (&operation.name.name, operation.name.span),
        };
        match seen.get(name.as_str()) {
            Some(first) => errors.push(ambiguous(name, vec![*first, span], span)),
            None => {
                seen.insert(name, span);
            }
        }
    }
    for name in &object.identity {
        if model.field(index, &name.name).is_none() {
            errors.push(missing_member(&name.name, name.span));
        }
    }
    for member in &object.members {
        match member {
            ObjectMemberForm::Field(field) => {
                if let Some(inherited) = model.ancestors[index]
                    .iter()
                    .find_map(|ancestor| model.own_field(*ancestor, &field.name.name))
                {
                    let first = field_span(model, inherited);
                    errors.push(ambiguous(
                        &field.name.name,
                        vec![first, field.name.span],
                        field.name.span,
                    ));
                }
            }
            ObjectMemberForm::Population(_) => {}
            ObjectMemberForm::Operation(operation) => {
                check_operation(model, index, operation, errors);
            }
        }
    }
}

fn field_span(model: &Model<'_>, located: Located) -> Span {
    match &model.objects[located.owner].members[located.member] {
        ObjectMemberForm::Field(field) => field.name.span,
        ObjectMemberForm::Population(population) => population.name.span,
        ObjectMemberForm::Operation(operation) => operation.name.span,
    }
}

fn check_operation(
    model: &Model<'_>,
    index: usize,
    operation: &ObjectOperationForm,
    errors: &mut Vec<AssemblyError>,
) {
    let inherited = model.ancestors[index]
        .iter()
        .find_map(|ancestor| model.own_operation(*ancestor, &operation.name.name));
    match (&operation.redefines, inherited) {
        (None, Some(inherited)) => {
            let first = field_span(model, inherited);
            errors.push(ambiguous(
                &operation.name.name,
                vec![first, operation.name.span],
                operation.name.span,
            ));
        }
        (Some(redefines), _) => {
            let declaring = model.named(&redefines.declaring.name);
            match declaring {
                Some(declaring) if model.ancestors[index].contains(&declaring) => {
                    if model.operation(declaring, &redefines.member.name).is_none() {
                        errors.push(missing_member(
                            &redefines.member.name,
                            redefines.member.span,
                        ));
                    }
                }
                _ => errors.push(missing(&redefines.declaring.name, redefines.declaring.span)),
            }
        }
        (None, None) => {}
    }
    let parameters: BTreeMap<&str, &TypeForm> = operation
        .parameters
        .iter()
        .map(|parameter| (parameter.name.name.as_str(), &parameter.value_type))
        .collect();
    for entry in &operation.frame {
        match entry {
            FrameEntryForm::ModifiesReceiver { field }
            | FrameEntryForm::ModifiesField { field } => {
                if model.field(index, &field.name).is_none() {
                    errors.push(missing_member(&field.name, field.span));
                }
            }
            FrameEntryForm::ModifiesTypeField { object, field } => {
                match model.named(&object.name) {
                    Some(target) => {
                        if model.field(target, &field.name).is_none() {
                            errors.push(missing_member(&field.name, field.span));
                        }
                    }
                    None => errors.push(missing(&object.name, object.span)),
                }
            }
            FrameEntryForm::Creates(object) => {
                if model.named(&object.name).is_none() {
                    errors.push(missing(&object.name, object.span));
                }
            }
            FrameEntryForm::DeletesReceiver(_) => {}
            FrameEntryForm::Deletes(name) => {
                let parameter = parameters.get(name.name.as_str());
                match (parameter, model.named(&name.name)) {
                    (Some(_), Some(_)) => {
                        errors.push(ambiguous(&name.name, vec![name.span], name.span))
                    }
                    (Some(form), None) => {
                        if !is_reference(form) {
                            errors.push(fault(
                                SourceObjectFault::NotAReference {
                                    name: name.name.clone(),
                                },
                                name.span,
                            ));
                        }
                    }
                    (None, Some(_)) => {}
                    (None, None) => errors.push(missing(&name.name, name.span)),
                }
            }
        }
    }
}

fn is_reference(form: &TypeForm) -> bool {
    matches!(form.head, TypeFormHead::Builtin(BuiltinType::Reference))
}

/// The records of one object: its type, fields, operations and populations.
fn records_of(
    model: &Model<'_>,
    index: usize,
    context: &Context<'_>,
    records: &mut Vec<DomainPackageRecord>,
    scalars: &mut Vec<DomainPackageRecord>,
    errors: &mut Vec<AssemblyError>,
) {
    let identity = context.identity;
    let object = &model.objects[index];
    let owner = model.type_key(identity, index);
    records.push(DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: owner.clone(),
        interface_features: None,
        abstract_type: object.abstract_span.is_some(),
        supertypes: model.supertypes[index]
            .iter()
            .map(|supertype| model.type_key(identity, *supertype))
            .collect(),
    }));
    let mut position = 0;
    for member in &object.members {
        match member {
            ObjectMemberForm::Field(field) => {
                let key = member_key(identity, &object.name.name, &field.name.name);
                match value_type(
                    model,
                    identity,
                    &object.name.name,
                    &field.name.name,
                    &field.type_form,
                    scalars,
                ) {
                    Ok(value_type) => records.push(DomainPackageRecord::FieldMember(field_record(
                        field, key, &owner, value_type, position,
                    ))),
                    Err(error) => errors.push(error),
                }
                position += 1;
            }
            ObjectMemberForm::Population(population) => {
                records.push(DomainPackageRecord::Population(PopulationRecord {
                    key: node_key(identity, &population.name.name),
                    member_types: vec![owner.clone()],
                    extent: Extent::Closed,
                    symmetric: population.symmetric,
                }));
            }
            ObjectMemberForm::Operation(operation) => {
                match operation_record(model, index, operation, context, scalars) {
                    Ok(record) => records.push(DomainPackageRecord::OperationMember(record)),
                    Err(mut found) => errors.append(&mut found),
                }
            }
        }
    }
}

fn field_record(
    field: &ObjectFieldForm,
    key: DeclarationKey,
    owner: &DeclarationKey,
    value_type: ValueTypeRef,
    position: usize,
) -> FieldMemberRecord {
    FieldMemberRecord {
        key,
        owner: owner.clone(),
        value_type,
        multiplicity: one(),
        presence: if field.optional {
            Presence::Optional
        } else {
            Presence::Required
        },
        subsets: Vec::new(),
        redefines: None,
        position,
    }
}

fn operation_record(
    model: &Model<'_>,
    index: usize,
    operation: &ObjectOperationForm,
    context: &Context<'_>,
    scalars: &mut Vec<DomainPackageRecord>,
) -> Result<OperationMemberRecord, Vec<AssemblyError>> {
    let identity = context.identity;
    let object = &model.objects[index];
    let owner = model.type_key(identity, index);
    let key = member_key(identity, &object.name.name, &operation.name.name);
    let mut errors = Vec::new();
    let mut parameters = Vec::new();
    for parameter in &operation.parameters {
        let site = format!("{}-{}", operation.name.name, parameter.name.name);
        match value_type(
            model,
            identity,
            &object.name.name,
            &site,
            &parameter.value_type,
            scalars,
        ) {
            Ok(value_type) => parameters.push(OperationParameterRecord {
                key: member_key(
                    identity,
                    &format!("{}/{}", object.name.name, operation.name.name),
                    &parameter.name.name,
                ),
                value_type,
                multiplicity: one(),
            }),
            Err(error) => errors.push(error),
        }
    }
    let result = match &operation.result {
        None => None,
        Some(form) => {
            let site = format!("{}-result", operation.name.name);
            match value_type(model, identity, &object.name.name, &site, form, scalars) {
                Ok(value_type) => Some(OperationResult {
                    value_type,
                    multiplicity: one(),
                }),
                Err(error) => {
                    errors.push(error);
                    None
                }
            }
        }
    };
    if !errors.is_empty() {
        return Err(errors);
    }
    let redefines = operation.redefines.as_ref().and_then(|redefines| {
        let declaring = model.named(&redefines.declaring.name)?;
        let located = model.operation(declaring, &redefines.member.name)?;
        Some(member_key(
            identity,
            &model.objects[located.owner].name.name,
            &redefines.member.name,
        ))
    });
    Ok(OperationMemberRecord {
        key,
        owner,
        parameters,
        result,
        effect: effect(model, index, operation, identity),
        requires: Vec::new(),
        ensures: Vec::new(),
        has_body: false,
        redefines,
    })
}

/// The operation's frame as an `OperationEffect` (FR-154).
fn effect(
    model: &Model<'_>,
    index: usize,
    operation: &ObjectOperationForm,
    identity: &str,
) -> OperationEffect {
    let field_key = |object: usize, name: &str| {
        model
            .field(object, name)
            .map(|located| member_key(identity, &model.objects[located.owner].name.name, name))
    };
    let parameters: BTreeSet<&str> = operation
        .parameters
        .iter()
        .map(|parameter| parameter.name.name.as_str())
        .collect();
    let mut effect = OperationEffect::default();
    for entry in &operation.frame {
        match entry {
            FrameEntryForm::ModifiesReceiver { field } => {
                effect
                    .receiver_modifies
                    .extend(field_key(index, &field.name));
            }
            FrameEntryForm::ModifiesField { field } => {
                effect.modifies.extend(field_key(index, &field.name));
            }
            FrameEntryForm::ModifiesTypeField { object, field } => {
                if let Some(target) = model.named(&object.name) {
                    effect.modifies.extend(field_key(target, &field.name));
                }
            }
            FrameEntryForm::Creates(object) => {
                if let Some(target) = model.named(&object.name) {
                    effect.creates.push(model.type_key(identity, target));
                }
            }
            FrameEntryForm::DeletesReceiver(_) => effect.deletes_receiver = true,
            FrameEntryForm::Deletes(name) => {
                if parameters.contains(name.name.as_str()) {
                    effect.deletes_arguments.push(name.name.clone());
                } else if let Some(target) = model.named(&name.name) {
                    effect.deletes.push(model.type_key(identity, target));
                }
            }
        }
    }
    effect
}

/// No names: a field, parameter or result of a source object type names no
/// declaration the build does not resolve itself.
struct NoNames;

impl TypeNames for NoNames {
    fn named_types(&self, _name: &str) -> &[quire_exact::ValueType] {
        &[]
    }

    fn object_type_named(&self, _name: &str) -> Option<quire_exact::EffectiveId> {
        None
    }

    fn unit_named(&self, _name: &str) -> Option<quire_exact::UnitId> {
        None
    }
}

/// The package value type of `form`, adding the scalar record a bounded
/// type needs under an artifact name no source name can spell.
fn value_type(
    model: &Model<'_>,
    identity: &str,
    owner: &str,
    site: &str,
    form: &TypeForm,
    scalars: &mut Vec<DomainPackageRecord>,
) -> Result<ValueTypeRef, AssemblyError> {
    let unsupported = || fault(SourceObjectFault::UnsupportedType, form.span);
    match &form.head {
        TypeFormHead::Builtin(BuiltinType::Boolean) => {
            Ok(ValueTypeRef::Native(NativeValueType::Boolean))
        }
        TypeFormHead::Builtin(BuiltinType::Integer) => {
            Ok(ValueTypeRef::Native(NativeValueType::Integer))
        }
        TypeFormHead::Builtin(BuiltinType::Reference) => {
            let [target] = form.arguments.as_slice() else {
                return Err(unsupported());
            };
            let TypeFormHead::Name(name) = &target.head else {
                return Err(unsupported());
            };
            match (name.is_qualified(), model.named(&name.name)) {
                (false, Some(object)) => {
                    Ok(ValueTypeRef::Package(model.type_key(identity, object)))
                }
                _ => Err(missing(&name.name, target.span)),
            }
        }
        TypeFormHead::Builtin(BuiltinType::Int | BuiltinType::Text) => {
            let resolved = resolve_form(&NoNames, form).map_err(|error| AssemblyError {
                cause: AssemblyCause::IllFormedBounds(error.fault),
                span: error.span,
            })?;
            let domain = match resolved {
                quire_exact::ValueType::Int(_) => {
                    let bound = |at: usize| {
                        form.bounds
                            .get(at)
                            .and_then(|text| text.parse::<i128>().ok())
                    };
                    match (bound(0), bound(1)) {
                        (Some(lower), Some(upper)) => ScalarValueDomain::Integer { lower, upper },
                        _ => return Err(unsupported()),
                    }
                }
                quire_exact::ValueType::Text(text) => ScalarValueDomain::Text(text),
                _ => return Err(unsupported()),
            };
            let key = node_key(identity, &format!("{owner}-{site}-type"));
            scalars.push(DomainPackageRecord::ScalarType(ScalarTypeRecord {
                key: key.clone(),
                domain,
            }));
            Ok(ValueTypeRef::Package(key))
        }
        _ => Err(unsupported()),
    }
}
