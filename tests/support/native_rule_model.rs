// SPDX-License-Identifier: AGPL-3.0-only
//! IT-005 / FR-017: source-aware fixture decoding and typed native IR lowering.

use std::collections::{btree_map::Entry, BTreeMap};

use quire_contract_ir as ir;
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::native_model::{
    Frame, ModelLimits, NativeModel, NativeRoles, ObjectRole, OperationRole, ScalarKind,
    ScalarRole, ScalarSite, Unit,
};
use quire_spec_language::{Diagnostic, Source, SourceIdentity};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::value::RawValue;

mod located_json;
use located_json::Located;

/// Separately authored, licensed native rule-model input.
pub const FIXTURE: &str = include_str!("../fixtures/native-rule-model.json");
const MAX_DECLARATIONS: usize = 10_000;
const MAX_TYPE_DEPTH: usize = 64;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleModel<'a> {
    license: String,
    package: String,
    requirement: String,
    revision: u64,
    #[serde(borrow)]
    scalars: Vec<&'a RawValue>,
    #[serde(borrow)]
    records: Vec<&'a RawValue>,
    #[serde(default, borrow)]
    enums: Vec<&'a RawValue>,
    #[serde(borrow)]
    values: Vec<&'a RawValue>,
    #[serde(borrow)]
    objects: Vec<&'a RawValue>,
    #[serde(borrow)]
    operations: Vec<&'a RawValue>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Scalar {
    Integer {
        name: String,
        minimum: i64,
        maximum: i64,
        unit: Option<String>,
    },
    Text {
        name: String,
        max_scalars: u32,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record<'a> {
    name: String,
    #[serde(borrow)]
    fields: Vec<&'a RawValue>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Field {
    name: String,
    #[serde(rename = "type")]
    ty: Type,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Enumeration<'a> {
    name: String,
    #[serde(borrow)]
    variants: Vec<&'a RawValue>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Value {
    name: String,
    kind: ir::ValueDeclarationKind,
    #[serde(rename = "type")]
    ty: Type,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Type {
    Boolean,
    Scalar { name: String },
    Record { name: String },
    Enum { name: String },
    Option { value: Box<Type> },
    Sequence { maximum: u32, value: Box<Type> },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Object {
    record: String,
    reference: String,
    identity_field: String,
    universe: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Operation {
    name: String,
    context: String,
    anchor: String,
    parameters: Vec<String>,
    result: Option<String>,
    frame: FrameData,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrameData {
    fields: Vec<(String, String)>,
    created: Vec<String>,
    deleted: Vec<String>,
}

/// Fixture setup failure, distinct from a native model or checker judgment.
#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
    /// Original source could not be admitted within its input contract.
    #[error("fixture source intake failed: {0}")]
    Source(#[from] Box<Diagnostic>),
    /// Typed JSON decoding or occurrence correspondence failed.
    #[error("fixture decoding failed: {0}")]
    Decode(#[from] located_json::Error),
    /// Existing IR constructors rejected the supplied declarations.
    #[error("formal fixture construction failed: {0:?}")]
    Formal(Vec<ir::Diagnostic>),
    /// Fixture-specific identity or bounded-construction rules failed.
    #[error("invalid rule-model fixture: {0}")]
    Invalid(&'static str),
}

impl From<ir::Diagnostic> for FixtureError {
    fn from(error: ir::Diagnostic) -> Self {
        Self::Formal(vec![error])
    }
}

impl From<Vec<ir::Diagnostic>> for FixtureError {
    fn from(errors: Vec<ir::Diagnostic>) -> Self {
        Self::Formal(errors)
    }
}

type Result<T> = std::result::Result<T, FixtureError>;

/// Construct a known test symbol; fallible producer input uses try_symbol.
pub fn symbol(name: &str) -> ir::SymbolName {
    try_symbol(name).expect("valid test symbol")
}

fn try_symbol(name: &str) -> Result<ir::SymbolName> {
    Ok(ir::SymbolName::new(name)?)
}

/// Independently produced input to actual native model admission.
pub struct Parts {
    /// Immutable original fixture and explicit formal source identity.
    pub source: FormalSource,
    /// Declarations produced through the existing IR constructors.
    pub environment: ir::DeclarationEnvironment,
    /// Explicit nominal, object and operation roles from the same input.
    pub roles: NativeRoles,
}

impl Parts {
    /// Admit known valid fixture setup; adverse model tests call the API directly.
    pub fn model(self) -> NativeModel {
        NativeModel::new(
            self.source,
            self.environment,
            self.roles,
            ModelLimits::default(),
        )
        .expect("qualified rule-model setup")
    }
}

/// Read the separately authored fixture; setup failures cannot become judgments.
pub fn parts() -> Parts {
    from_text(FIXTURE, "native-rule-model.json", "draft:1")
        .expect("valid source-derived rule-model fixture")
}

/// Orchestrate source intake, occurrence-aware decoding and typed lowering.
pub fn from_text(text: &str, path: &str, revision: &str) -> Result<Parts> {
    let source = bind_source(text, path, revision)?;
    let input = decode_model(&source)?;
    let (environment, roles) = lower_model(input)?;
    Ok(Parts {
        source,
        environment,
        roles,
    })
}

fn bind_source(text: &str, path: &str, revision: &str) -> Result<FormalSource> {
    let native = Source::read(
        SourceIdentity {
            identity: "test:rule-model".into(),
            revision: revision.into(),
        },
        path,
        text.as_bytes(),
        quire_spec_language::source::MAX_SOURCE_BYTES,
    )?;
    let formal = ir::SourceIdentity::new(
        ir::SourceDocumentId::new("RuleModelSource")?,
        ir::SourceRevision::new(1)?,
    );
    Ok(FormalSource::new(native, formal))
}

struct DecodedModel {
    package: String,
    requirement: String,
    revision: u64,
    scalars: Vec<Located<Scalar>>,
    records: Vec<Located<DecodedRecord>>,
    enums: Vec<Located<DecodedEnum>>,
    values: Vec<Located<Value>>,
    objects: Vec<Located<Object>>,
    operations: Vec<Located<Operation>>,
}

struct DecodedRecord {
    name: String,
    fields: Vec<Located<Field>>,
}

struct DecodedEnum {
    name: String,
    variants: Vec<Located<String>>,
}

fn decode_items<T: DeserializeOwned>(
    source: &FormalSource,
    items: Vec<&RawValue>,
) -> Result<Vec<Located<T>>> {
    items
        .into_iter()
        .map(|raw| located_json::decode(source, raw).map_err(FixtureError::from))
        .collect()
}

fn decode_model(source: &FormalSource) -> Result<DecodedModel> {
    let input: RuleModel<'_> = located_json::read(source)?;
    if input.license != "AGPL-3.0-only" {
        return Err(FixtureError::Invalid("unexpected fixture license"));
    }
    let mut remaining = MAX_DECLARATIONS;
    for count in [
        input.scalars.len(),
        input.records.len(),
        input.enums.len(),
        input.values.len(),
        input.objects.len(),
        input.operations.len(),
    ] {
        remaining = remaining
            .checked_sub(count)
            .ok_or(FixtureError::Invalid("declaration budget exhausted"))?;
    }
    let mut records = Vec::new();
    for raw in input.records {
        let Located {
            value: record,
            source: span,
        } = located_json::decode::<Record<'_>>(source, raw)?;
        remaining = remaining
            .checked_sub(record.fields.len())
            .ok_or(FixtureError::Invalid("field budget exhausted"))?;
        records.push(Located {
            value: DecodedRecord {
                name: record.name,
                fields: decode_items(source, record.fields)?,
            },
            source: span,
        });
    }
    let mut enums = Vec::new();
    for raw in input.enums {
        let Located {
            value: enumeration,
            source: span,
        } = located_json::decode::<Enumeration<'_>>(source, raw)?;
        remaining = remaining
            .checked_sub(enumeration.variants.len())
            .ok_or(FixtureError::Invalid("variant budget exhausted"))?;
        enums.push(Located {
            value: DecodedEnum {
                name: enumeration.name,
                variants: decode_items(source, enumeration.variants)?,
            },
            source: span,
        });
    }
    Ok(DecodedModel {
        package: input.package,
        requirement: input.requirement,
        revision: input.revision,
        scalars: decode_items(source, input.scalars)?,
        records,
        enums,
        values: decode_items(source, input.values)?,
        objects: decode_items(source, input.objects)?,
        operations: decode_items(source, input.operations)?,
    })
}

struct ScalarBinding {
    representation: ir::ValueType,
    role: ScalarRole,
}

/// One owner for a scalar's representation and all its native declaration sites.
struct ScalarTable(BTreeMap<ir::SymbolName, ScalarBinding>);

impl ScalarTable {
    fn new(declarations: Vec<Located<Scalar>>) -> Result<Self> {
        let mut bindings = BTreeMap::new();
        for declaration in declarations {
            let binding = lower_scalar(declaration)?;
            match bindings.entry(binding.role.name.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(binding);
                }
                Entry::Occupied(_) => {
                    return Err(FixtureError::Invalid("duplicate scalar identity"))
                }
            }
        }
        Ok(Self(bindings))
    }

    fn lower_type(&mut self, ty: Type, site: &ScalarSite, depth: usize) -> Result<ir::ValueType> {
        if depth >= MAX_TYPE_DEPTH {
            return Err(FixtureError::Invalid("fixture type depth exhausted"));
        }
        match ty {
            Type::Boolean => Ok(ir::ValueType::Boolean),
            Type::Scalar { name } => {
                let name = try_symbol(&name)?;
                let binding = self
                    .0
                    .get_mut(&name)
                    .ok_or(FixtureError::Invalid("unknown scalar identity"))?;
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
        self.0.into_values().map(|binding| binding.role).collect()
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

fn lower_operation(declaration: Located<Operation>) -> Result<OperationRole> {
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

fn lower_model(input: DecodedModel) -> Result<(ir::DeclarationEnvironment, NativeRoles)> {
    let owner = ir::RequirementRef::new(
        ir::PackageId::new(input.package)?,
        ir::RequirementId::new(input.requirement)?,
        ir::RequirementRevision::new(input.revision)?,
    );
    let mut scalars = ScalarTable::new(input.scalars)?;
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
    Ok((environment, roles))
}
