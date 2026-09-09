// SPDX-License-Identifier: AGPL-3.0-only
//! FR-025: source-aware rule-model decoding and existing IR declaration lowering.

use std::collections::{btree_map::Entry, BTreeMap};

use crate::formal_source::FormalSource;
use crate::native_model::{
    Frame, ModelLimits, NativeModel, NativeRoles, ObjectRole, OperationRole, ScalarKind,
    ScalarRole, ScalarSite, Unit,
};
use crate::serde_object::{
    deserialize_empty_object, from_object as deserialize_object, Object as JsonObject,
};
use crate::{Code, Diagnostic};
use quire_contract_ir as ir;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::value::RawValue;

use crate::located_json::{self, Located};

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
    #[serde(deserialize_with = "deserialize_object")]
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
    #[serde(deserialize_with = "deserialize_object")]
    ty: Type,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Type {
    #[serde(deserialize_with = "deserialize_empty_object")]
    Boolean,
    Scalar {
        name: String,
    },
    Record {
        name: String,
    },
    Enum {
        name: String,
    },
    Option {
        #[serde(deserialize_with = "deserialize_object")]
        value: Box<Type>,
    },
    Sequence {
        maximum: u32,
        #[serde(deserialize_with = "deserialize_object")]
        value: Box<Type>,
    },
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
    #[serde(deserialize_with = "deserialize_object")]
    frame: FrameData,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrameData {
    fields: Vec<(String, String)>,
    created: Vec<String>,
    deleted: Vec<String>,
}

/// Explicit authoring profile for the existing rule-model JSON syntax.
pub const FORMAT: &str = "native-rule-model/1";

/// Inclusive frontend limits, independently clamped to their defaults.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelSourceLimits {
    /// Original source UTF-8 bytes, at most 1 MiB.
    pub source_bytes: usize,
    /// Declaration, field, variant, parameter and frame entries, at most 10,000.
    pub entries: usize,
    /// Active nested type levels, at most 64.
    pub type_depth: usize,
}

impl Default for ModelSourceLimits {
    fn default() -> Self {
        Self {
            source_bytes: 1_048_576,
            entries: 10_000,
            type_depth: 64,
        }
    }
}

impl ModelSourceLimits {
    fn bounded(self) -> Self {
        let hard = Self::default();
        Self {
            source_bytes: self.source_bytes.min(hard.source_bytes),
            entries: self.entries.min(hard.entries),
            type_depth: self.type_depth.min(hard.type_depth),
        }
    }
}

/// Original frontend or admission failure; no message parsing determines its kind.
#[derive(Debug, thiserror::Error)]
pub enum ModelSourceCause {
    /// JSON decoding or original occurrence correspondence failed.
    #[error("{0}")]
    Decode(#[from] located_json::Error),
    /// Existing IR constructors rejected the declarations.
    #[error("formal model construction failed: {0:?}")]
    Formal(Vec<ir::Diagnostic>),
    /// The source's declaration relationships are invalid.
    #[error("{0}")]
    Invalid(&'static str),
    /// A caller-lowered or implementation ceiling stopped lowering.
    #[error("{0}")]
    Resource(&'static str),
    /// The caller selected an unknown source profile.
    #[error("unsupported rule-model source profile")]
    UnknownFormat,
    /// Actual native role admission failed.
    #[error("{0}")]
    Admission(#[from] Box<Diagnostic>),
}

impl From<ir::Diagnostic> for ModelSourceCause {
    fn from(error: ir::Diagnostic) -> Self {
        Self::Formal(vec![error])
    }
}

impl From<Vec<ir::Diagnostic>> for ModelSourceCause {
    fn from(errors: Vec<ir::Diagnostic>) -> Self {
        Self::Formal(errors)
    }
}

/// A refused or incomplete source read retains the exact original document.
#[derive(Debug, thiserror::Error)]
#[error("{cause}")]
pub struct ModelSourceError {
    binding: FormalSource,
    /// Original typed failure.
    #[source]
    pub cause: ModelSourceCause,
}

impl ModelSourceError {
    /// Exact original native/formal source, including retained bytes.
    pub fn source(&self) -> &FormalSource {
        &self.binding
    }
    /// Stable native classification for the actual failing stage.
    pub fn code(&self) -> Code {
        match &self.cause {
            ModelSourceCause::Decode(located_json::Error::Source(error))
            | ModelSourceCause::Admission(error) => error.code,
            ModelSourceCause::Decode(located_json::Error::ForeignOccurrence) => {
                Code::InvalidSourceMap
            }
            ModelSourceCause::Resource(_) => Code::ResourceExhausted,
            ModelSourceCause::UnknownFormat => Code::UnknownWire,
            ModelSourceCause::Formal(errors)
                if errors.iter().any(|error| {
                    matches!(
                        error.code,
                        ir::DiagnosticCode::SemanticInputTooLarge
                            | ir::DiagnosticCode::CanonicalizationResourceExhausted
                    )
                }) =>
            {
                Code::ResourceExhausted
            }
            ModelSourceCause::Decode(located_json::Error::Json(_))
            | ModelSourceCause::Formal(_)
            | ModelSourceCause::Invalid(_) => Code::InvalidModelBinding,
        }
    }
    /// A source, lowering or admission budget prevented completion.
    pub fn is_incomplete(&self) -> bool {
        self.code() == Code::ResourceExhausted
    }
}

type Result<T> = std::result::Result<T, ModelSourceCause>;

/// Source-derived inputs to native admission, not an already admitted model.
#[derive(Debug)]
pub struct ModelDraft {
    /// Immutable original document and explicitly supplied formal identity.
    pub source: FormalSource,
    /// Existing IR declarations constructed from this source.
    pub environment: ir::DeclarationEnvironment,
    /// Explicit source-derived nominal, object and operation roles.
    pub roles: NativeRoles,
    /// Original declared license string, retained as metadata without interpretation.
    pub declared_license: String,
}

impl ModelDraft {
    /// Apply the existing native admission checks under independent limits.
    pub fn admit(
        self,
        limits: ModelLimits,
    ) -> std::result::Result<NativeModel, Box<ModelSourceError>> {
        let binding = self.source.clone();
        NativeModel::new(self.source, self.environment, self.roles, limits).map_err(|cause| {
            Box::new(ModelSourceError {
                binding,
                cause: ModelSourceCause::Admission(cause),
            })
        })
    }
}

/// Decode and lower the selected rule-model profile with exact original loci.
pub fn read(
    source: FormalSource,
    format: &str,
    limits: ModelSourceLimits,
) -> std::result::Result<ModelDraft, Box<ModelSourceError>> {
    let lower = || {
        let limits = limits.bounded();
        if format != FORMAT {
            return Err(ModelSourceCause::UnknownFormat);
        }
        if source.source().text().len() > limits.source_bytes {
            return Err(ModelSourceCause::Resource(
                "model source byte limit exceeded",
            ));
        }
        lower_model(decode_model(&source, limits)?, limits.type_depth)
    };
    match lower() {
        Ok((environment, roles, declared_license)) => Ok(ModelDraft {
            source,
            environment,
            roles,
            declared_license,
        }),
        Err(cause) => Err(Box::new(ModelSourceError {
            binding: source,
            cause,
        })),
    }
}

fn try_symbol(name: &str) -> Result<ir::SymbolName> {
    Ok(ir::SymbolName::new(name)?)
}

struct DecodedModel {
    license: String,
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
        .map(|raw| decode_record(source, raw))
        .collect()
}

fn decode_model(source: &FormalSource, limits: ModelSourceLimits) -> Result<DecodedModel> {
    let JsonObject(input): JsonObject<RuleModel<'_>> = located_json::read(source)?;
    let mut remaining = limits.entries;
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
            .ok_or(ModelSourceCause::Resource(
                "declaration entry limit exceeded",
            ))?;
    }
    let mut records = Vec::new();
    for raw in input.records {
        let Located {
            value: record,
            source: span,
        } = decode_record::<Record<'_>>(source, raw)?;
        remaining = remaining
            .checked_sub(record.fields.len())
            .ok_or(ModelSourceCause::Resource("field entry limit exceeded"))?;
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
        } = decode_record::<Enumeration<'_>>(source, raw)?;
        remaining = remaining
            .checked_sub(enumeration.variants.len())
            .ok_or(ModelSourceCause::Resource("variant entry limit exceeded"))?;
        enums.push(Located {
            value: DecodedEnum {
                name: enumeration.name,
                variants: enumeration
                    .variants
                    .into_iter()
                    .map(|raw| located_json::decode(source, raw).map_err(ModelSourceCause::from))
                    .collect::<Result<_>>()?,
            },
            source: span,
        });
    }
    let operations: Vec<Located<Operation>> = decode_items(source, input.operations)?;
    for operation in &operations {
        let value = &operation.value;
        for count in [
            value.parameters.len(),
            value.frame.fields.len(),
            value.frame.created.len(),
            value.frame.deleted.len(),
        ] {
            remaining = remaining
                .checked_sub(count)
                .ok_or(ModelSourceCause::Resource("operation entry limit exceeded"))?;
        }
    }
    Ok(DecodedModel {
        license: input.license,
        package: input.package,
        requirement: input.requirement,
        revision: input.revision,
        scalars: decode_items(source, input.scalars)?,
        records,
        enums,
        values: decode_items(source, input.values)?,
        objects: decode_items(source, input.objects)?,
        operations,
    })
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
                Entry::Occupied(_) => {
                    return Err(ModelSourceCause::Invalid("duplicate scalar identity"))
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
            return Err(ModelSourceCause::Resource(
                "model type depth limit exceeded",
            ));
        }
        match ty {
            Type::Boolean => Ok(ir::ValueType::Boolean),
            Type::Scalar { name } => {
                let name = try_symbol(&name)?;
                let binding = self
                    .bindings
                    .get_mut(&name)
                    .ok_or(ModelSourceCause::Invalid("unknown scalar identity"))?;
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

fn lower_model(
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

fn decode_record<'de, T: Deserialize<'de>>(
    source: &FormalSource,
    raw: &'de RawValue,
) -> Result<Located<T>> {
    let Located {
        value: JsonObject(value),
        source,
    } = located_json::decode::<JsonObject<T>>(source, raw)?;
    Ok(Located { value, source })
}
