// SPDX-License-Identifier: AGPL-3.0-only
//! FR-025: lower decoded declarations into existing IR and native roles.

use super::decode::{DecodedEnum, DecodedModel, DecodedOperation, DecodedRecord};
use super::wire::{Field, Object, Scalar, Type, Value};
use super::{ModelSourceCause, Result};
use crate::located_json::Located;
use crate::native_model::{
    Frame, NativeRoles, ObjectRole, OperationRole, ScalarKind, ScalarRole, ScalarSite, Unit,
};
use quire_contract_ir as ir;
use std::collections::{btree_map::Entry, BTreeMap};

fn try_symbol(name: &str) -> Result<ir::SymbolName> {
    Ok(ir::SymbolName::new(name)?)
}

struct ScalarBinding {
    representation: ir::ValueType,
    role: ScalarRole,
}

/// One owner for a scalar's representation and all its native declaration sites.
struct ScalarTable {
    bindings: BTreeMap<ir::SymbolName, ScalarBinding>,
    type_depth: usize,
}

impl ScalarTable {
    fn new(declarations: Vec<Located<Scalar>>, type_depth: usize) -> Result<Self> {
        let mut bindings = BTreeMap::new();
        for declaration in declarations {
            let binding = lower_scalar(declaration)?;
            match bindings.entry(binding.role.name.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(binding);
                }
                Entry::Occupied(entry) => {
                    return Err(ModelSourceCause::DuplicateScalar {
                        name: entry.key().clone(),
                    });
                }
            }
        }
        Ok(Self {
            bindings,
            type_depth,
        })
    }

    fn lower_type(&mut self, ty: Type, site: &ScalarSite, depth: usize) -> Result<ir::ValueType> {
        if depth >= self.type_depth {
            return Err(ModelSourceCause::TypeDepth {
                actual: depth + 1,
                maximum: self.type_depth,
            });
        }
        match ty {
            Type::Boolean => Ok(ir::ValueType::Boolean),
            Type::Scalar { name } => {
                let name = try_symbol(&name)?;
                let binding = self
                    .bindings
                    .get_mut(&name)
                    .ok_or_else(|| ModelSourceCause::UnknownScalar { name: name.clone() })?;
                binding.role.sites.push(site.clone());
                Ok(binding.representation.clone())
            }
            Type::Record { name } => Ok(ir::ValueType::Record {
                name: try_symbol(&name)?,
            }),
            Type::Enum { name } => Ok(ir::ValueType::Enum {
                name: try_symbol(&name)?,
            }),
            Type::Option { value } => Ok(ir::ValueType::option(self.lower_type(
                *value,
                site,
                depth + 1,
            )?)),
            Type::Sequence { maximum, value } => {
                let element = self.lower_type(*value, site, depth + 1)?;
                Ok(ir::ValueType::collection(ir::CollectionType::new(
                    element, maximum,
                )?))
            }
        }
    }

    fn into_roles(self) -> Vec<ScalarRole> {
        self.bindings
            .into_values()
            .map(|binding| binding.role)
            .collect()
    }
}

fn lower_scalar(declaration: Located<Scalar>) -> Result<ScalarBinding> {
    let (name, kind, representation) = match declaration.value {
        Scalar::Integer {
            name,
            minimum,
            maximum,
            unit,
        } => {
            let unit = unit
                .as_deref()
                .map(try_symbol)
                .transpose()?
                .map_or(Unit::Dimensionless, Unit::Named);
            let integer = ir::IntegerType::new(
                ir::IntegerDomain::Signed,
                minimum,
                maximum,
                ir::OverflowPolicy::Reject,
            )?;
            (
                name,
                ScalarKind::Integer { unit },
                ir::ValueType::integer(integer),
            )
        }
        Scalar::Text { name, max_scalars } => {
            (name, ScalarKind::Text { max_scalars }, ir::ValueType::Text)
        }
        Scalar::Rational {
            name,
            numerator_minimum,
            numerator_maximum,
            maximum_denominator,
            unit,
        } => {
            let rational =
                ir::RationalType::new(numerator_minimum, numerator_maximum, maximum_denominator)
                    .map_err(|mut error| {
                        error.span = Some(Box::new(declaration.source.clone()));
                        error
                    })?;
            let unit = unit
                .as_deref()
                .map(try_symbol)
                .transpose()?
                .map_or(Unit::Dimensionless, Unit::Named);
            (
                name,
                ScalarKind::Rational { unit },
                ir::ValueType::rational(rational),
            )
        }
    };
    let role = ScalarRole {
        name: try_symbol(&name)?,
        source: declaration.source,
        kind,
        sites: Vec::new(),
    };
    Ok(ScalarBinding {
        representation,
        role,
    })
}

fn lower_field(
    declaration: Located<Field>,
    record: &ir::SymbolName,
    scalars: &mut ScalarTable,
) -> Result<ir::RecordFieldDeclaration> {
    let name = try_symbol(&declaration.value.name)?;
    let site = ScalarSite::Field {
        record: record.clone(),
        field: name.clone(),
    };
    let ty = scalars.lower_type(declaration.value.ty, &site, 0)?;
    Ok(ir::RecordFieldDeclaration::new(
        name,
        ty,
        declaration.source,
    ))
}

fn lower_record(
    declaration: Located<DecodedRecord>,
    scalars: &mut ScalarTable,
) -> Result<ir::TypeDeclaration> {
    let name = try_symbol(&declaration.value.name)?;
    let fields = declaration
        .value
        .fields
        .into_iter()
        .map(|field| lower_field(field, &name, scalars))
        .collect::<Result<Vec<_>>>()?;
    let record = ir::RecordDeclaration::new(name, declaration.source, fields)?;
    Ok(ir::TypeDeclaration::Record {
        declaration: record,
    })
}

fn lower_value(
    declaration: Located<Value>,
    scalars: &mut ScalarTable,
) -> Result<ir::ValueDeclaration> {
    let name = try_symbol(&declaration.value.name)?;
    let ty = scalars.lower_type(
        declaration.value.ty,
        &ScalarSite::Value { name: name.clone() },
        0,
    )?;
    Ok(ir::ValueDeclaration::new(
        name,
        declaration.value.kind,
        ty,
        declaration.source,
    ))
}

fn lower_enum(declaration: Located<DecodedEnum>) -> Result<ir::TypeDeclaration> {
    let variants = declaration
        .value
        .variants
        .into_iter()
        .map(|variant| {
            Ok(ir::EnumVariantDeclaration::new(
                try_symbol(&variant.value)?,
                variant.source,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(ir::TypeDeclaration::Enum {
        declaration: ir::EnumDeclaration::new(
            try_symbol(&declaration.value.name)?,
            declaration.source,
            variants,
        )?,
    })
}

fn lower_object(declaration: Located<Object>) -> Result<ObjectRole> {
    let object = declaration.value;
    Ok(ObjectRole {
        record: try_symbol(&object.record)?,
        reference: try_symbol(&object.reference)?,
        identity_field: try_symbol(&object.identity_field)?,
        universe: try_symbol(&object.universe)?,
        source: declaration.source,
    })
}

fn symbols(names: &[String]) -> Result<Vec<ir::SymbolName>> {
    names.iter().map(|name| try_symbol(name)).collect()
}

fn lower_operation(declaration: Located<DecodedOperation>) -> Result<OperationRole> {
    let operation = declaration.value;
    let frame = Frame {
        fields: operation
            .frame
            .fields
            .into_iter()
            .map(|(record, field)| Ok((try_symbol(&record)?, try_symbol(&field)?)))
            .collect::<Result<Vec<_>>>()?,
        created: symbols(&operation.frame.created)?,
        deleted: symbols(&operation.frame.deleted)?,
    };
    Ok(OperationRole {
        context: try_symbol(&operation.context)?,
        name: try_symbol(&operation.name)?,
        anchor: ir::AnchorName::new(operation.anchor)?,
        source: declaration.source,
        parameters: symbols(&operation.parameters)?,
        result: operation.result.as_deref().map(try_symbol).transpose()?,
        frame,
    })
}

pub(super) fn lower_model(
    input: DecodedModel,
    type_depth: usize,
) -> Result<(ir::DeclarationEnvironment, NativeRoles, String)> {
    let owner = ir::RequirementRef::new(
        ir::PackageId::new(input.package)?,
        ir::RequirementId::new(input.requirement)?,
        ir::RequirementRevision::new(input.revision)?,
    );
    let mut scalars = ScalarTable::new(input.scalars, type_depth)?;
    let mut declarations = input
        .records
        .into_iter()
        .map(|record| lower_record(record, &mut scalars))
        .collect::<Result<Vec<_>>>()?;
    declarations.extend(
        input
            .enums
            .into_iter()
            .map(lower_enum)
            .collect::<Result<Vec<_>>>()?,
    );
    let values = input
        .values
        .into_iter()
        .map(|value| lower_value(value, &mut scalars))
        .collect::<Result<Vec<_>>>()?;
    let roles = NativeRoles {
        scalars: scalars.into_roles(),
        objects: input
            .objects
            .into_iter()
            .map(lower_object)
            .collect::<Result<_>>()?,
        operations: input
            .operations
            .into_iter()
            .map(lower_operation)
            .collect::<Result<_>>()?,
    };
    let environment = ir::DeclarationEnvironment::new(owner, declarations, values, Vec::new())?;
    Ok((environment, roles, input.license))
}
