// SPDX-License-Identifier: AGPL-3.0-only
//! FR-019-AC-6: complete feature discovery over already admitted static inventories.

use std::collections::{BTreeMap, BTreeSet};

use quire_contract_ir as ir;

use super::{PackageError, PackageStage, PackageUsage};
use crate::checking::{CheckedPackage, NativeType};
use crate::native_model::{NativeModel, ScalarKind};
use crate::syntax::{BinaryOp, Builtin, ClauseKind, ExprKind, UnaryOp};
use crate::Code;

pub(super) type Features = BTreeSet<&'static str>;

fn invalid_model() -> Box<PackageError> {
    Box::new(PackageError {
        code: Code::InvalidModelBinding,
        stage: PackageStage::Encode,
        path: Vec::new(),
        usage: PackageUsage::default(),
        message: "checked package contains no admitted native model interpretation",
        cause: None,
    })
}

fn formal_type(
    ty: &ir::ValueType,
    records: &BTreeMap<&ir::SymbolName, &'static str>,
    features: &mut Features,
) -> Result<(), Box<PackageError>> {
    match ty {
        ir::ValueType::Boolean => {
            features.insert("boolean");
        }
        ir::ValueType::Integer { .. } => {
            features.insert("integer");
        }
        ir::ValueType::Text => {
            features.insert("text");
        }
        ir::ValueType::Enum { .. } => {
            features.insert("enumeration");
        }
        ir::ValueType::Record { name } => {
            features.insert(records.get(name).copied().unwrap_or("structural-record"));
        }
        ir::ValueType::Option { value } => {
            features.insert("option");
            formal_type(value, records, features)?;
        }
        ir::ValueType::Collection { value } => {
            features.insert("sequence");
            formal_type(value.element(), records, features)?;
        }
        ir::ValueType::Rational { .. } => return Err(invalid_model()),
    }
    Ok(())
}

fn model_features(model: &NativeModel, features: &mut Features) -> Result<(), Box<PackageError>> {
    let mut records = BTreeMap::new();
    for object in &model.roles().objects {
        records.insert(&object.record, "object");
        records.insert(&object.reference, "reference");
        features.extend(["object", "reference"]);
    }
    for scalar in &model.roles().scalars {
        features.insert(match scalar.kind {
            ScalarKind::Integer { .. } => "integer",
            ScalarKind::Text { .. } => "text",
        });
    }
    if !model.roles().operations.is_empty() {
        features.insert("object");
    }
    // Every declaration is visited once. Named record references contribute a
    // role and are not recursively expanded, so object cycles cannot recurse.
    for declaration in model.environment().types() {
        match declaration {
            ir::TypeDeclaration::Record { declaration } => {
                features.insert(
                    records
                        .get(declaration.name())
                        .copied()
                        .unwrap_or("structural-record"),
                );
                for field in declaration.fields() {
                    formal_type(field.value_type(), &records, features)?;
                }
            }
            ir::TypeDeclaration::Enum { .. } => {
                features.insert("enumeration");
            }
        }
    }
    for value in model.environment().values() {
        formal_type(value.value_type(), &records, features)?;
    }
    Ok(())
}

fn native_type(ty: &NativeType<'_>, features: &mut Features) {
    match ty {
        NativeType::Boolean => {
            features.insert("boolean");
        }
        NativeType::Scalar { role, .. } => {
            features.insert(match role.kind {
                ScalarKind::Integer { .. } => "integer",
                ScalarKind::Text { .. } => "text",
            });
        }
        NativeType::Enumeration { .. } => {
            features.insert("enumeration");
        }
        NativeType::Record { .. } => {
            features.insert("structural-record");
        }
        NativeType::Object { .. } => {
            features.insert("object");
        }
        NativeType::Reference { .. } => {
            features.insert("reference");
        }
        NativeType::Option(value) => {
            features.insert("option");
            native_type(value, features);
        }
        NativeType::Sequence { element, .. } => {
            features.insert("sequence");
            native_type(element, features);
        }
    }
}

pub(super) fn derive(checked: &CheckedPackage<'_>) -> Result<Features, Box<PackageError>> {
    let mut features = Features::new();
    let mut visited = BTreeSet::new();
    for selected in checked.linked().models() {
        let model = selected.native_model().ok_or_else(invalid_model)?;
        if visited.insert(model.environment().owner()) {
            model_features(model, &mut features)?;
        }
    }
    for clause in checked.clauses() {
        features.insert("boolean");
        for ty in clause.expression_types() {
            native_type(ty, &mut features);
        }
    }
    for clause in checked.linked().unit().clauses() {
        match clause.kind {
            ClauseKind::Invariant => {}
            ClauseKind::Precondition => {
                features.insert("precondition");
            }
            ClauseKind::Postcondition => {
                features.insert("postcondition");
            }
        }
    }
    for expression in checked.linked().unit().expressions() {
        let feature = match expression.kind {
            ExprKind::Boolean(_) => Some("boolean"),
            ExprKind::Integer(_) => Some("integer"),
            ExprKind::Text(_) => Some("text"),
            ExprKind::EnumValue { .. } => Some("enumeration"),
            ExprKind::Unary {
                op: UnaryOp::Not, ..
            } => Some("boolean-control"),
            ExprKind::Unary {
                op: UnaryOp::Negate,
                ..
            } => Some("integer-arithmetic"),
            ExprKind::Binary { op, .. } => Some(match op {
                BinaryOp::Implies | BinaryOp::Or | BinaryOp::And => "boolean-control",
                BinaryOp::Equal
                | BinaryOp::NotEqual
                | BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual => "comparison",
                BinaryOp::Add
                | BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Divide
                | BinaryOp::Remainder => "integer-arithmetic",
            }),
            ExprKind::Call {
                builtin: Builtin::Pre,
                ..
            } => Some("pre-observation"),
            ExprKind::Call {
                builtin: Builtin::Present | Builtin::Value | Builtin::Deref | Builtin::Size,
                ..
            } => None,
            ExprKind::Let { .. } => Some("let"),
            ExprKind::If { .. } => Some("conditional"),
            ExprKind::Quantifier { .. } => Some("quantification"),
            ExprKind::Reaches { .. } => Some("reachability"),
            ExprKind::Group { .. }
            | ExprKind::Name(_)
            | ExprKind::SelfValue
            | ExprKind::ResultValue
            | ExprKind::Field { .. } => None,
        };
        if let Some(feature) = feature {
            features.insert(feature);
        }
    }
    Ok(features)
}
