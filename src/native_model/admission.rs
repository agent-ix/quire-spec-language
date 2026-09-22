// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-015: bounded complete admission before native model construction.

use std::collections::{BTreeMap, BTreeSet};

use quire_contract_ir as ir;

use super::{
    failure, ModelLimits, NativeModelError, NativeModelProfile, NativeRoles, ScalarKind,
    ScalarRole, ScalarSite,
};
use crate::formal_source::FormalSource;
use crate::linking::{DeclarationIdentity, DeclarationKey, DeclarationLocation};
use crate::Code;

type Result<T> = std::result::Result<T, Box<NativeModelError>>;

const MAX_SEQUENCE_ITEMS: u32 = 10_000;

pub(super) fn check(
    profile: NativeModelProfile,
    source: &FormalSource,
    environment: &ir::DeclarationEnvironment,
    roles: &NativeRoles,
    limits: ModelLimits,
) -> Result<()> {
    preflight(profile, source, environment, roles, limits)?;
    check_loci(source, environment, roles)?;
    let catalog = Catalog::new(environment);
    let scalars = catalog.check_scalars(source, roles)?;
    catalog.check_objects(source, roles, &scalars)?;
    catalog.check_operations(source, roles)
}

fn spend(
    source: &FormalSource,
    remaining: &mut usize,
    count: usize,
    dimension: &str,
) -> Result<()> {
    *remaining = remaining.checked_sub(count).ok_or_else(|| {
        failure(
            source,
            Code::ResourceExhausted,
            format!("model {dimension} limit exceeded"),
        )
    })?;
    Ok(())
}

fn preflight(
    profile: NativeModelProfile,
    source: &FormalSource,
    environment: &ir::DeclarationEnvironment,
    roles: &NativeRoles,
    limits: ModelLimits,
) -> Result<()> {
    let mut roles_left = limits.roles;
    for count in [
        roles.scalars.len(),
        roles.objects.len(),
        roles.operations.len(),
    ] {
        spend(source, &mut roles_left, count, "roles")?;
    }
    let mut entries_left = limits.entries;
    for role in &roles.scalars {
        spend(source, &mut entries_left, role.sites.len(), "entries")?;
        match profile {
            NativeModelProfile::V1 => match role.kind {
                ScalarKind::Integer { .. } | ScalarKind::Text { .. } => {}
                ScalarKind::Rational { .. } => {
                    return Err(failure(
                        source,
                        Code::UnsupportedConstruct,
                        "rational roles are outside native-state-model/1",
                    ));
                }
            },
            NativeModelProfile::V2 => match role.kind {
                ScalarKind::Integer { .. }
                | ScalarKind::Rational { .. }
                | ScalarKind::Text { .. } => {}
            },
        }
    }
    for operation in &roles.operations {
        for count in [
            operation.parameters.len(),
            operation.frame.fields.len(),
            operation.frame.created.len(),
            operation.frame.deleted.len(),
        ] {
            spend(source, &mut entries_left, count, "entries")?;
        }
    }
    if !environment.functions().is_empty() {
        // A declaration form this profile's admitted package structure does
        // not include, not a fixed representability limit against an
        // otherwise-admitted declaration: reuse the existing invalid-package
        // code rather than minting a new one.
        return Err(failure(
            source,
            Code::InvalidPackage,
            "pure function signatures are outside the native model profile",
        ));
    }
    let mut nodes_left = limits.nodes;
    spend(source, &mut nodes_left, environment.types().len(), "nodes")?;
    spend(source, &mut nodes_left, environment.values().len(), "nodes")?;
    for declaration in environment.types() {
        match declaration {
            ir::TypeDeclaration::Record { declaration } => {
                spend(source, &mut nodes_left, declaration.fields().len(), "nodes")?;
                for field in declaration.fields() {
                    check_type(
                        profile,
                        source,
                        field.value_type(),
                        &mut nodes_left,
                        0,
                        limits.depth,
                    )?;
                }
            }
            ir::TypeDeclaration::Enum { declaration } => {
                spend(
                    source,
                    &mut nodes_left,
                    declaration.variants().len(),
                    "nodes",
                )?;
            }
        }
    }
    for value in environment.values() {
        check_type(
            profile,
            source,
            value.value_type(),
            &mut nodes_left,
            0,
            limits.depth,
        )?;
    }
    // String content is a lower bound on serialized size. Check it before
    // borrowed-key comparisons or metadata normalization; JSON escaping and
    // punctuation are charged exactly by the bounded artifact writer.
    super::artifact::check_string_content(source, environment, roles, limits.artifact_bytes)
}

fn check_type(
    profile: NativeModelProfile,
    source: &FormalSource,
    ty: &ir::ValueType,
    remaining: &mut usize,
    depth: usize,
    maximum_depth: usize,
) -> Result<()> {
    if depth >= maximum_depth {
        return Err(failure(
            source,
            Code::ResourceExhausted,
            "model type depth exceeded",
        ));
    }
    spend(source, remaining, 1, "nodes")?;
    match ty {
        ir::ValueType::Integer { value } => {
            if value.domain() != ir::IntegerDomain::Signed
                || value.overflow() != ir::OverflowPolicy::Reject
            {
                return Err(failure(
                    source,
                    Code::UnrepresentableConstraint,
                    "native integers require signed reject-overflow semantics",
                ));
            }
        }
        ir::ValueType::Rational { .. } => match profile {
            NativeModelProfile::V1 => {
                return Err(failure(
                    source,
                    Code::UnsupportedConstruct,
                    "rational values are outside the native model profile",
                ))
            }
            NativeModelProfile::V2 => {}
        },
        ir::ValueType::Option { value } => {
            check_type(profile, source, value, remaining, depth + 1, maximum_depth)?
        }
        ir::ValueType::Collection { value } => {
            // MAX_SEQUENCE_ITEMS is a fixed representability ceiling of the
            // native profile itself, not a caller-configurable ModelLimits
            // budget (nodes/entries/depth, checked via `spend` and
            // `Code::ResourceExhausted` above): a declaration that exceeds
            // it is invalid on this build under any limits, so it exits 20,
            // not the 22 a caller could clear by raising a supplied limit.
            if value.maximum_items() > MAX_SEQUENCE_ITEMS {
                return Err(failure(
                    source,
                    Code::UnrepresentableConstraint,
                    format!(
                        "native sequence maximum {} exceeds {MAX_SEQUENCE_ITEMS}",
                        value.maximum_items()
                    ),
                ));
            }
            check_type(
                profile,
                source,
                value.element(),
                remaining,
                depth + 1,
                maximum_depth,
            )?;
        }
        ir::ValueType::Boolean
        | ir::ValueType::Text
        | ir::ValueType::Enum { .. }
        | ir::ValueType::Record { .. } => {}
    }
    Ok(())
}

fn locus(
    source: &FormalSource,
    environment: &ir::DeclarationEnvironment,
    key: DeclarationKey,
    span: &ir::SourceSpan,
) -> Result<()> {
    source.to_native(span).map(|_| ()).map_err(|cause| {
        let mut error = failure(
            source,
            Code::InvalidModelBinding,
            "model declaration locus does not match its exact source",
        );
        error.upstream = cause.upstream;
        error.related.push(DeclarationLocation {
            identity: DeclarationIdentity {
                owner: environment.owner().clone(),
                key,
            },
            source: span.clone(),
        });
        error
    })
}

fn check_loci(
    source: &FormalSource,
    environment: &ir::DeclarationEnvironment,
    roles: &NativeRoles,
) -> Result<()> {
    for declaration in environment.types() {
        locus(
            source,
            environment,
            DeclarationKey::Type(declaration.name().clone()),
            declaration.source(),
        )?;
        match declaration {
            ir::TypeDeclaration::Record { declaration } => {
                for field in declaration.fields() {
                    locus(
                        source,
                        environment,
                        DeclarationKey::Field {
                            record: declaration.name().clone(),
                            field: field.name().clone(),
                        },
                        field.source(),
                    )?;
                }
            }
            ir::TypeDeclaration::Enum { declaration } => {
                for variant in declaration.variants() {
                    locus(
                        source,
                        environment,
                        DeclarationKey::Variant {
                            enumeration: declaration.name().clone(),
                            variant: variant.name().clone(),
                        },
                        variant.source(),
                    )?;
                }
            }
        }
    }
    for value in environment.values() {
        locus(
            source,
            environment,
            DeclarationKey::Value(value.name().clone()),
            value.source(),
        )?;
    }
    for scalar in &roles.scalars {
        locus(
            source,
            environment,
            DeclarationKey::Scalar(scalar.name.clone()),
            &scalar.source,
        )?;
    }
    for object in &roles.objects {
        source.to_native(&object.source).map_err(|cause| {
            let mut error = failure(
                source,
                Code::InvalidModelBinding,
                "object-role locus does not match its exact source",
            );
            error.upstream = cause.upstream;
            error
        })?;
    }
    for operation in &roles.operations {
        locus(
            source,
            environment,
            DeclarationKey::Operation {
                context: operation.context.clone(),
                name: operation.name.clone(),
            },
            &operation.source,
        )?;
    }
    Ok(())
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum Site<'a> {
    Value(&'a ir::SymbolName),
    Field(&'a ir::SymbolName, &'a ir::SymbolName),
}

impl<'a> From<&'a ScalarSite> for Site<'a> {
    fn from(site: &'a ScalarSite) -> Self {
        match site {
            ScalarSite::Value { name } => Self::Value(name),
            ScalarSite::Field { record, field } => Self::Field(record, field),
        }
    }
}

fn leaf(mut ty: &ir::ValueType) -> &ir::ValueType {
    loop {
        match ty {
            ir::ValueType::Option { value } => ty = value,
            ir::ValueType::Collection { value } => ty = value.element(),
            ir::ValueType::Boolean
            | ir::ValueType::Integer { .. }
            | ir::ValueType::Rational { .. }
            | ir::ValueType::Text
            | ir::ValueType::Enum { .. }
            | ir::ValueType::Record { .. } => return ty,
        }
    }
}

struct Catalog<'a> {
    records: BTreeMap<&'a ir::SymbolName, &'a ir::RecordDeclaration>,
    values: BTreeMap<&'a ir::SymbolName, &'a ir::ValueDeclaration>,
    sites: BTreeMap<Site<'a>, &'a ir::ValueType>,
}

impl<'a> Catalog<'a> {
    fn new(environment: &'a ir::DeclarationEnvironment) -> Self {
        let mut records = BTreeMap::new();
        let mut sites = BTreeMap::new();
        for declaration in environment.types() {
            if let ir::TypeDeclaration::Record { declaration } = declaration {
                records.insert(declaration.name(), declaration);
                for field in declaration.fields() {
                    sites.insert(
                        Site::Field(declaration.name(), field.name()),
                        leaf(field.value_type()),
                    );
                }
            }
        }
        let values = environment
            .values()
            .iter()
            .map(|value| {
                sites.insert(Site::Value(value.name()), leaf(value.value_type()));
                (value.name(), value)
            })
            .collect();
        Self {
            records,
            values,
            sites,
        }
    }

    fn check_scalars(
        &self,
        source: &FormalSource,
        roles: &'a NativeRoles,
    ) -> Result<BTreeMap<Site<'a>, &'a ScalarRole>> {
        let mut names = BTreeSet::new();
        let mut assignments = BTreeMap::new();
        for scalar in &roles.scalars {
            if !names.insert(&scalar.name) || scalar.sites.is_empty() {
                return Err(failure(
                    source,
                    Code::InvalidModelBinding,
                    "scalar identities must be unique and have declaration sites",
                ));
            }
            let mut integer = None;
            let mut rational = None;
            for site in &scalar.sites {
                let site = Site::from(site);
                if assignments.insert(site, scalar).is_some() {
                    return Err(failure(
                        source,
                        Code::InvalidModelBinding,
                        "primitive declaration site has multiple scalar roles",
                    ));
                }
                let representation = self.sites.get(&site).copied();
                let valid = match &scalar.kind {
                    ScalarKind::Integer { .. } => {
                        if let Some(ir::ValueType::Integer { value }) = representation {
                            let same = integer.is_none_or(|prior| prior == value);
                            integer = Some(value);
                            same
                        } else {
                            false
                        }
                    }
                    ScalarKind::Rational { .. } => {
                        if let Some(ir::ValueType::Rational { value }) = representation {
                            let same = rational.is_none_or(|prior| prior == value);
                            rational = Some(value);
                            same
                        } else {
                            false
                        }
                    }
                    ScalarKind::Text { max_scalars } => {
                        matches!(representation, Some(ir::ValueType::Text))
                            && *max_scalars <= ir::MAX_TEXT_LENGTH
                    }
                };
                if !valid {
                    return Err(failure(
                        source,
                        Code::InvalidModelBinding,
                        "scalar role disagrees with its formal primitive representation",
                    ));
                }
            }
        }
        for (site, ty) in &self.sites {
            if matches!(
                ty,
                ir::ValueType::Integer { .. }
                    | ir::ValueType::Rational { .. }
                    | ir::ValueType::Text
            ) && !assignments.contains_key(site)
            {
                return Err(failure(
                    source,
                    Code::InvalidModelBinding,
                    "primitive declaration site has no native scalar role",
                ));
            }
        }
        Ok(assignments)
    }

    fn check_objects(
        &self,
        source: &FormalSource,
        roles: &NativeRoles,
        scalars: &BTreeMap<Site<'_>, &ScalarRole>,
    ) -> Result<()> {
        let mut objects = BTreeSet::new();
        let mut references = BTreeSet::new();
        for object in &roles.objects {
            if !objects.insert(&object.record)
                || !references.insert(&object.reference)
                || !self.records.contains_key(&object.record)
            {
                return Err(failure(
                    source,
                    Code::InvalidModelBinding,
                    "object and reference roles must have unique record owners",
                ));
            }
            let carrier = self.records.get(&object.reference).ok_or_else(|| {
                failure(
                    source,
                    Code::InvalidModelBinding,
                    "reference carrier record is absent",
                )
            })?;
            let [field] = carrier.fields() else {
                return Err(failure(
                    source,
                    Code::InvalidModelBinding,
                    "reference carrier must have exactly one ID field",
                ));
            };
            let bounded_id = scalars
                .get(&Site::Field(&object.reference, &object.identity_field))
                .is_some_and(|scalar| match scalar.kind {
                    ScalarKind::Text { max_scalars } => max_scalars > 0,
                    ScalarKind::Integer { .. } | ScalarKind::Rational { .. } => false,
                });
            if field.name() != &object.identity_field
                || field.value_type() != &ir::ValueType::Text
                || !bounded_id
            {
                return Err(failure(
                    source,
                    Code::InvalidModelBinding,
                    "reference IDs require one nonoptional positively bounded text field",
                ));
            }
        }
        if !objects.is_disjoint(&references) {
            return Err(failure(
                source,
                Code::InvalidModelBinding,
                "object payload and reference carrier records must be disjoint",
            ));
        }
        Ok(())
    }

    fn check_operations(&self, source: &FormalSource, roles: &NativeRoles) -> Result<()> {
        let objects: BTreeSet<_> = roles.objects.iter().map(|object| &object.record).collect();
        let mut operations = BTreeSet::new();
        let mut inputs = BTreeSet::new();
        for operation in &roles.operations {
            if !objects.contains(&operation.context)
                || !operations.insert((&operation.context, &operation.name))
            {
                return Err(failure(
                    source,
                    Code::InvalidModelBinding,
                    "operation needs a unique name and an explicit object context",
                ));
            }
            let mut selected = BTreeSet::new();
            for name in operation.parameters.iter().chain(operation.result.iter()) {
                if !selected.insert(name)
                    || !self
                        .values
                        .get(name)
                        .is_some_and(|value| value.kind() == ir::ValueDeclarationKind::Input)
                {
                    return Err(failure(source, Code::InvalidModelBinding, "operation parameters and result must select distinct IR Input declarations"));
                }
                inputs.insert(name);
            }
            let mut fields = BTreeSet::new();
            for (record, field) in &operation.frame.fields {
                if !fields.insert((record, field))
                    || !objects.contains(record)
                    || !self.sites.contains_key(&Site::Field(record, field))
                {
                    return Err(failure(
                        source,
                        Code::InvalidModelBinding,
                        "frame field must be a unique actual object field",
                    ));
                }
            }
            for inventory in [&operation.frame.created, &operation.frame.deleted] {
                let mut seen = BTreeSet::new();
                if inventory
                    .iter()
                    .any(|name| !seen.insert(name) || !objects.contains(name))
                {
                    return Err(failure(
                        source,
                        Code::InvalidModelBinding,
                        "frame object types must be unique and explicitly mapped",
                    ));
                }
            }
        }
        if self.values.values().any(|value| {
            value.kind() == ir::ValueDeclarationKind::Input && !inputs.contains(value.name())
        }) {
            return Err(failure(
                source,
                Code::InvalidModelBinding,
                "IR Input declaration has no native operation role",
            ));
        }
        Ok(())
    }
}
