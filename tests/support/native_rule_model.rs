// SPDX-License-Identifier: AGPL-3.0-only
//! IT-005: source-derived, domain-specific Rust qualification producer.

use std::collections::BTreeMap;

use quire_contract_ir as ir;
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::native_model::{
    Frame, ModelLimits, NativeModel, NativeRoles, ObjectRole, OperationRole, ScalarKind,
    ScalarRole, ScalarSite, Unit,
};
use quire_spec_language::{Source, SourceIdentity, Span};
use serde::Deserialize;

pub const FIXTURE: &str = include_str!("../fixtures/native-rule-model.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleModel {
    license: String,
    package: String,
    requirement: String,
    revision: u64,
    scalars: Vec<Scalar>,
    records: Vec<Record>,
    values: Vec<Value>,
    objects: Vec<Object>,
    operations: Vec<Operation>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Scalar {
    Integer { name: String, minimum: i64, maximum: i64, unit: Option<String> },
    Text { name: String, max_scalars: u32 },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record { name: String, fields: Vec<Field> }

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Field { name: String, #[serde(rename = "type")] ty: Type }

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Value { name: String, kind: ir::ValueDeclarationKind, #[serde(rename = "type")] ty: Type }

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Type {
    Boolean,
    Scalar { name: String },
    Record { name: String },
    Option { value: Box<Type> },
    Sequence { maximum: u32, value: Box<Type> },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Object { record: String, reference: String, identity_field: String, universe: String }

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Operation {
    name: String, context: String, anchor: String, parameters: Vec<String>,
    result: Option<String>, frame: FrameData,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrameData { fields: Vec<(String, String)>, created: Vec<String>, deleted: Vec<String> }

pub fn symbol(name: &str) -> ir::SymbolName { ir::SymbolName::new(name).unwrap() }

pub struct Parts {
    pub source: FormalSource,
    pub environment: ir::DeclarationEnvironment,
    pub roles: NativeRoles,
}

impl Parts {
    pub fn model(self) -> NativeModel {
        NativeModel::new(self.source, self.environment, self.roles, ModelLimits::default()).unwrap()
    }
}

fn locus(source: &FormalSource, needle: &str) -> ir::SourceSpan {
    let start = source.source().text().find(needle).expect("fixture source locus");
    source.to_ir(source.source(), Span { start, end: start + needle.len() }).unwrap()
}

fn named_locus(source: &FormalSource, name: &str) -> ir::SourceSpan {
    locus(source, &format!("\"name\": \"{name}\""))
}

fn formal_type(ty: &Type, types: &BTreeMap<String, ir::ValueType>, depth: usize) -> (ir::ValueType, Option<String>) {
    assert!(depth < 64, "qualification input type depth");
    match ty {
        Type::Boolean => (ir::ValueType::Boolean, None),
        Type::Scalar { name } => (types.get(name).expect("authored scalar").clone(), Some(name.clone())),
        Type::Record { name } => (ir::ValueType::Record { name: symbol(name) }, None),
        Type::Option { value } => {
            let (ty, scalar) = formal_type(value, types, depth + 1);
            (ir::ValueType::option(ty), scalar)
        }
        Type::Sequence { maximum, value } => {
            let (ty, scalar) = formal_type(value, types, depth + 1);
            (ir::ValueType::collection(ir::CollectionType::new(ty, *maximum).unwrap()), scalar)
        }
    }
}

pub fn parts() -> Parts {
    from_text(FIXTURE, "native-rule-model.json", "draft:1")
}

pub fn from_text(text: &str, path: &str, revision: &str) -> Parts {
    let native = Source::read(SourceIdentity { identity: "test:rule-model".into(), revision: revision.into() }, path, text.as_bytes(), 1_048_576).unwrap();
    let source = FormalSource::new(native, ir::SourceIdentity::new(ir::SourceDocumentId::new("RuleModelSource").unwrap(), ir::SourceRevision::new(1).unwrap()));
    let input: RuleModel = serde_json::from_str(source.source().text()).unwrap();
    assert_eq!(input.license, "AGPL-3.0-only");
    assert!(input.scalars.len() + input.records.len() + input.values.len() + input.objects.len() + input.operations.len() <= 10_000);
    let mut types = BTreeMap::new();
    let mut scalars = BTreeMap::new();
    for scalar in input.scalars {
        let (name, kind, ty) = match scalar {
            Scalar::Integer { name, minimum, maximum, unit } => (name, ScalarKind::Integer { unit: unit.map_or(Unit::Dimensionless, |name| Unit::Named(symbol(&name))) }, ir::ValueType::integer(ir::IntegerType::new(ir::IntegerDomain::Signed, minimum, maximum, ir::OverflowPolicy::Reject).unwrap())),
            Scalar::Text { name, max_scalars } => (name, ScalarKind::Text { max_scalars }, ir::ValueType::Text),
        };
        let role = ScalarRole { name: symbol(&name), source: named_locus(&source, &name), kind, sites: Vec::new() };
        assert!(types.insert(name.clone(), ty).is_none());
        assert!(scalars.insert(name, role).is_none());
    }
    let mut declarations = Vec::new();
    for record in input.records {
        assert!(record.fields.len() <= 10_000);
        let mut fields = Vec::new();
        for field in record.fields {
            let (ty, scalar) = formal_type(&field.ty, &types, 0);
            if let Some(scalar) = scalar { scalars.get_mut(&scalar).unwrap().sites.push(ScalarSite::Field { record: symbol(&record.name), field: symbol(&field.name) }); }
            fields.push(ir::RecordFieldDeclaration::new(symbol(&field.name), ty, named_locus(&source, &field.name)));
        }
        declarations.push(ir::TypeDeclaration::Record { declaration: ir::RecordDeclaration::new(symbol(&record.name), named_locus(&source, &record.name), fields).unwrap() });
    }
    let mut values = Vec::new();
    for value in input.values {
        let (ty, scalar) = formal_type(&value.ty, &types, 0);
        if let Some(scalar) = scalar { scalars.get_mut(&scalar).unwrap().sites.push(ScalarSite::Value { name: symbol(&value.name) }); }
        values.push(ir::ValueDeclaration::new(symbol(&value.name), value.kind, ty, named_locus(&source, &value.name)));
    }
    let objects = input.objects.into_iter().map(|object| ObjectRole {
        source: locus(&source, &format!("\"record\": \"{}\", \"reference\": \"{}\"", object.record, object.reference)),
        record: symbol(&object.record), reference: symbol(&object.reference), identity_field: symbol(&object.identity_field), universe: symbol(&object.universe),
    }).collect();
    let operations = input.operations.into_iter().map(|operation| OperationRole {
        source: named_locus(&source, &operation.name), context: symbol(&operation.context), name: symbol(&operation.name), anchor: ir::AnchorName::new(operation.anchor).unwrap(),
        parameters: operation.parameters.iter().map(|name| symbol(name)).collect(), result: operation.result.map(|name| symbol(&name)),
        frame: Frame { fields: operation.frame.fields.into_iter().map(|(record, field)| (symbol(&record), symbol(&field))).collect(), created: operation.frame.created.iter().map(|name| symbol(name)).collect(), deleted: operation.frame.deleted.iter().map(|name| symbol(name)).collect() },
    }).collect();
    let owner = ir::RequirementRef::new(ir::PackageId::new(input.package).unwrap(), ir::RequirementId::new(input.requirement).unwrap(), ir::RequirementRevision::new(input.revision).unwrap());
    let environment = ir::DeclarationEnvironment::new(owner, declarations, values, Vec::new()).unwrap();
    Parts { source, environment, roles: NativeRoles { scalars: scalars.into_values().collect(), objects, operations } }
}
