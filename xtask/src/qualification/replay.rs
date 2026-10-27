// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-350: adapt corpus-owned replay requests to the typed FR-071 API.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;

use qsl_foundation::digest::DigestRecord;
use qsl_replay::{
    CanonicalAssignment, DeclaredDomain, DependencyEntryWire, DomainKey, DomainKind,
    EnvironmentLabels, FiniteBound, FrameOperation, Integer, ObjectEnvironmentRef, OccurrenceKey,
    Origin, ProfileSelection, ProofBound, QualifiedName, ReplayOverBinding, ReplayRequestWire,
    ReplaySource, ReplayTargetMembers, RequestTargetWire, Role, ScalarLimits, StateEnvironment,
    SuppliedLimits, WireNodeId, Witness, WitnessField, WitnessSlot, WitnessValue,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{parse_digest, parse_unique_json, InputFile, InputKind, Vector};

type SourceDigestInput = (String, String, String, String, Option<String>, String);
type ByteProvisionInput = (Option<String>, String, Vec<u8>);

pub(super) fn execute(
    vector: &Vector,
    inputs: &[(&InputFile, Vec<u8>)],
    root: &Path,
    limits: &qsl_replay::CallerLimits,
    schema: &jsonschema::JSONSchema,
) -> Result<(Value, String), String> {
    let (request_file, request_bytes) = input_bytes(inputs, InputKind::ReplayRequest)
        .ok_or_else(|| "runtime/replay requires exactly one replay-request file".to_owned())?;
    if vector
        .input
        .files
        .iter()
        .filter(|file| file.kind == InputKind::ReplayRequest)
        .count()
        != 1
    {
        return Err("runtime/replay requires exactly one replay-request file".to_owned());
    }
    if vector
        .input
        .files
        .iter()
        .any(|file| !matches!(file.kind, InputKind::ReplayRequest | InputKind::Bytes))
    {
        return Err("runtime/replay accepts only replay-request and bytes files".to_owned());
    }
    let value = parse_unique_json(request_bytes)
        .map_err(|error| format!("{}: {error}", request_file.path))?;
    if let Err(errors) = schema.validate(&value) {
        return Err(errors
            .map(|error| format!("{}: {}", error.instance_path, error))
            .collect::<Vec<_>>()
            .join("; "));
    }
    let wire = adapt_request(&value, request_file, vector, inputs, root)?;
    let first = qsl_replay::replay(wire.clone(), limits.replay);
    let first_bytes = qsl_replay::OutcomeDocument::from_replay(&first)
        .to_bytes()
        .map_err(|error| format!("FR-286 outcome encoding failed: {error}"))?;
    let second = qsl_replay::replay(wire, limits.replay);
    let second_bytes = qsl_replay::OutcomeDocument::from_replay(&second)
        .to_bytes()
        .map_err(|error| format!("FR-286 outcome encoding failed: {error}"))?;
    if first_bytes != second_bytes {
        return Ok((
            json!({"code":"nondeterministic-replay","message":"raw FR-286 outcome bytes differ across repeated replay"}),
            String::from_utf8_lossy(&first_bytes).into_owned(),
        ));
    }
    let actual = replay_result_value(&first)?;
    let output = String::from_utf8(first_bytes)
        .map_err(|error| format!("FR-286 outcome is not UTF-8: {error}"))?;
    Ok((actual, output))
}

fn input_bytes<'a>(
    inputs: &'a [(&InputFile, Vec<u8>)],
    kind: InputKind,
) -> Option<(&'a InputFile, &'a [u8])> {
    inputs
        .iter()
        .find(|(file, _)| file.kind == kind)
        .map(|(file, bytes)| (*file, bytes.as_slice()))
}

fn adapt_request(
    value: &Value,
    request_file: &InputFile,
    vector: &Vector,
    inputs: &[(&InputFile, Vec<u8>)],
    root: &Path,
) -> Result<ReplayRequestWire, String> {
    let package = object(member(value, "package_id")?, "package_id")?;
    let (package_domain, package_digest) = digest_members(package, "domain", "digest")?;
    let source_digests = array(member(value, "source_digests")?, "source_digests")?
        .iter()
        .map(source_digest)
        .collect::<Result<Vec<_>, _>>()?;
    let dependencies = array(member(value, "dependencies")?, "dependencies")?
        .iter()
        .map(|entry| {
            let entry = object(entry, "dependency")?;
            let package = object(member(entry, "package_id")?, "dependency.package_id")?;
            let (domain, digest) = digest_members(package, "domain", "digest")?;
            Ok(DependencyEntryWire {
                identity: string(member(entry, "identity")?, "dependency.identity")?.to_owned(),
                package_id: (Some(domain), digest),
                sources: array(member(entry, "sources")?, "dependency.sources")?
                    .iter()
                    .map(source_digest)
                    .collect::<Result<Vec<_>, _>>()?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let profile_selections = array(member(value, "profile_selections")?, "profile_selections")?
        .iter()
        .map(|selection| {
            let selection = object(selection, "profile_selection")?;
            Ok(ProfileSelection::new(
                string(member(selection, "profile")?, "profile")?.to_owned(),
                string(member(selection, "value")?, "value")?.to_owned(),
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let target = adapt_target(member(value, "replay_target")?)?;
    let source = adapt_source(member(value, "source")?)?;
    let state_environment = array(member(value, "state_environment")?, "state_environment")?
        .iter()
        .map(|entry| {
            let pair = array(entry, "state_environment entry")?;
            if pair.len() != 2 {
                return Err("state_environment entries must be two-string arrays".to_owned());
            }
            Ok((
                string(&pair[0], "state_environment name")?.to_owned(),
                string(&pair[1], "state_environment value")?.to_owned(),
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let limits = adapt_scalar_limits(member(value, "accounting_limits")?)?;
    let stage_limits = object(member(value, "stage_limits")?, "stage_limits")?
        .iter()
        .map(|(name, bound)| Ok((name.clone(), unsigned(bound, "stage_limits")?)))
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let supplied_limits = if member(value, "supplied_limits")?.is_null() {
        None
    } else {
        let supplied = object(member(value, "supplied_limits")?, "supplied_limits")?;
        Some(
            SuppliedLimits::default()
                .with_admission_work_units(unsigned(
                    member(supplied, "admission_work_units")?,
                    "supplied_limits.admission_work_units",
                )?)
                .with_conversion_work_units(unsigned(
                    member(supplied, "conversion_work_units")?,
                    "supplied_limits.conversion_work_units",
                )?)
                .with_numeric_magnitude_bits(unsigned(
                    member(supplied, "numeric_magnitude_bits")?,
                    "supplied_limits.numeric_magnitude_bits",
                )?),
        )
    };
    let object_environment = if member(value, "object_environment")?.is_null() {
        None
    } else {
        Some(adapt_object_environment(member(
            value,
            "object_environment",
        )?)?)
    };
    let declared_domains = array(member(value, "declared_domains")?, "declared_domains")?
        .iter()
        .map(adapt_declared_domain)
        .collect::<Result<Vec<_>, _>>()?;
    let byte_provision = adapt_byte_provision(
        member(value, "byte_provision")?,
        request_file,
        vector,
        inputs,
        root,
    )?;
    Ok(ReplayRequestWire {
        profile_selections,
        package_id: (Some(package_domain), package_digest),
        source_digests,
        dependencies,
        replay_target: target,
        source,
        obligation_identity: parse_digest(string(
            member(value, "obligation_identity")?,
            "obligation_identity",
        )?)?,
        backend: string(member(value, "backend")?, "backend")?.to_owned(),
        state_environment: StateEnvironment::new(state_environment),
        accounting_limits: limits,
        stage_limits,
        supplied_limits,
        object_environment,
        declared_domains,
        byte_provision,
    })
}

fn digest_members(
    value: &serde_json::Map<String, Value>,
    domain_name: &str,
    digest_name: &str,
) -> Result<(String, String), String> {
    Ok((
        string(member(value, domain_name)?, domain_name)?.to_owned(),
        string(member(value, digest_name)?, digest_name)?.to_owned(),
    ))
}

fn source_digest(value: &Value) -> Result<SourceDigestInput, String> {
    let value = object(value, "source_digest")?;
    Ok((
        string(member(value, "authority")?, "authority")?.to_owned(),
        string(member(value, "identity")?, "identity")?.to_owned(),
        string(member(value, "revision_namespace")?, "revision_namespace")?.to_owned(),
        string(member(value, "revision")?, "revision")?.to_owned(),
        if member(value, "digest_domain")?.is_null() {
            None
        } else {
            Some(string(member(value, "digest_domain")?, "digest_domain")?.to_owned())
        },
        string(member(value, "digest")?, "digest")?.to_owned(),
    ))
}

fn adapt_target(value: &Value) -> Result<RequestTargetWire, String> {
    let value = object(value, "replay_target")?;
    let members: ReplayTargetMembersInput =
        serde_json::from_value(member(value, "members")?.clone())
            .map_err(|error| format!("replay_target.members: {error}"))?;
    let target = match members {
        ReplayTargetMembersInput::Function { function } => {
            ReplayTargetMembers::function(qualified_name(&function)?)
        }
        ReplayTargetMembersInput::StateClause { state_clause } => {
            ReplayTargetMembers::state_clause(qualified_name(&state_clause)?)
        }
        ReplayTargetMembersInput::FrameOperation { frame_operation } => {
            ReplayTargetMembers::frame_operation(FrameOperation {
                object: qualified_name(&frame_operation.object)?,
                operation: identifier(&frame_operation.operation)?,
            })
        }
    };
    let occurrence_key = if member(value, "occurrence_key")?.is_null() {
        None
    } else {
        let key = object(member(value, "occurrence_key")?, "occurrence_key")?;
        let node = wire_node(string(member(key, "node_id")?, "occurrence_key.node_id")?)?;
        let role = Role::new(string(member(key, "role")?, "occurrence_key.role")?);
        Some(OccurrenceKey::new(
            node,
            Origin::new(
                role,
                unsigned(member(key, "ordinal")?, "occurrence_key.ordinal")?,
            ),
        ))
    };
    let over = if member(value, "over")?.is_null() {
        None
    } else {
        let over = object(member(value, "over")?, "over")?;
        Some(ReplayOverBinding {
            population: string(member(over, "population")?, "over.population")?.to_owned(),
            key: string(member(over, "key")?, "over.key")?.to_owned(),
        })
    };
    Ok(RequestTargetWire {
        members: target,
        arguments: boolean(member(value, "arguments")?, "arguments")?,
        occurrence_key,
        over,
    })
}

fn qualified_name(value: &str) -> Result<QualifiedName, String> {
    value
        .split('.')
        .map(identifier)
        .collect::<Result<Vec<_>, _>>()
        .and_then(|segments| QualifiedName::new(segments).map_err(|error| error.to_string()))
}

fn identifier(value: &str) -> Result<qsl_replay::Identifier, String> {
    qsl_replay::Identifier::new(value).map_err(|error| error.to_string())
}

fn wire_node(value: &str) -> Result<WireNodeId, String> {
    WireNodeId::from_hex(value).ok_or_else(|| format!("invalid WireNodeId {value:?}"))
}

fn adapt_source(value: &Value) -> Result<ReplaySource, String> {
    let source: ReplaySourceInput =
        serde_json::from_value(value.clone()).map_err(|error| format!("source: {error}"))?;
    match source {
        ReplaySourceInput::Witness { transcript } => Ok(ReplaySource::Witness(
            Witness::parse(transcript).map_err(|error| error.to_string())?,
        )),
        ReplaySourceInput::Input { assignments } => {
            let assignments = assignments
                .into_iter()
                .map(|assignment| {
                    Ok(CanonicalAssignment {
                        parameter: wire_node(&assignment.parameter)?,
                        value: adapt_witness_value_input(assignment.value)?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(ReplaySource::Input(assignments))
        }
    }
}

fn adapt_witness_value_input(value: WitnessValueInput) -> Result<WitnessValue, String> {
    Ok(match value {
        WitnessValueInput::Boolean { value } => WitnessValue::Boolean(value),
        WitnessValueInput::Integer { decimal } => WitnessValue::Integer(
            decimal
                .parse()
                .map_err(|error| format!("invalid i128 decimal: {error}"))?,
        ),
        WitnessValueInput::ExactInteger { decimal } => WitnessValue::ExactInteger(
            Integer::from_str(&decimal).map_err(|error| error.to_string())?,
        ),
        WitnessValueInput::Enum {
            declaration,
            member,
        } => WitnessValue::Enum {
            declaration: wire_node(&declaration)?,
            member,
        },
        WitnessValueInput::Text { value } => WitnessValue::Text(value),
        WitnessValueInput::Uuid { value } => WitnessValue::Uuid(value),
        WitnessValueInput::Timestamp { decimal } => WitnessValue::Timestamp(decimal),
        WitnessValueInput::String { value } => WitnessValue::String(value),
        WitnessValueInput::Rational {
            numerator,
            denominator,
        } => WitnessValue::Rational {
            numerator: Integer::from_str(&numerator).map_err(|error| error.to_string())?,
            denominator: Integer::from_str(&denominator).map_err(|error| error.to_string())?,
        },
        WitnessValueInput::Decimal { coefficient, scale } => WitnessValue::Decimal {
            coefficient: Integer::from_str(&coefficient).map_err(|error| error.to_string())?,
            scale: Integer::from_str(&scale).map_err(|error| error.to_string())?,
        },
        WitnessValueInput::Float32 { bits } => WitnessValue::Float32(
            bits.try_into()
                .map_err(|error| format!("float32 bits exceed u32: {error}"))?,
        ),
        WitnessValueInput::Float64 { bits } => WitnessValue::Float64(bits),
        WitnessValueInput::Quantity { magnitude, unit } => WitnessValue::Quantity {
            magnitude: match magnitude {
                QuantityMagnitudeInput::Rational {
                    numerator,
                    denominator,
                } => qsl_replay::QuantityMagnitude::Rational {
                    numerator: Integer::from_str(&numerator).map_err(|error| error.to_string())?,
                    denominator: Integer::from_str(&denominator)
                        .map_err(|error| error.to_string())?,
                },
                QuantityMagnitudeInput::Decimal { coefficient, scale } => {
                    qsl_replay::QuantityMagnitude::Decimal {
                        coefficient: Integer::from_str(&coefficient)
                            .map_err(|error| error.to_string())?,
                        scale: Integer::from_str(&scale).map_err(|error| error.to_string())?,
                    }
                }
            },
            unit: parse_digest(&unit)?,
        },
        WitnessValueInput::Reference {
            universe,
            object_type,
            identity,
        } => WitnessValue::Reference {
            universe: parse_digest(&universe)?,
            object_type: parse_digest(&object_type)?,
            identity: hex_bytes(&identity)?,
        },
        WitnessValueInput::Option { value } => WitnessValue::Option(
            value
                .map(|value| adapt_witness_value_input(*value))
                .transpose()?
                .map(Box::new),
        ),
        WitnessValueInput::Record {
            declaration,
            fields,
        } => WitnessValue::Record {
            declaration: wire_node(&declaration)?,
            fields: fields
                .into_iter()
                .map(|field| {
                    Ok(WitnessField {
                        name: field.name,
                        slot: match field.slot {
                            WitnessSlotInput::Present { value } => {
                                WitnessSlot::Present(adapt_witness_value_input(value)?)
                            }
                            WitnessSlotInput::Absent => WitnessSlot::Absent,
                            WitnessSlotInput::Null => WitnessSlot::Null,
                        },
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
        },
        WitnessValueInput::Tuple {
            declaration,
            components,
        } => WitnessValue::Tuple {
            declaration: wire_node(&declaration)?,
            components: components
                .into_iter()
                .map(adapt_witness_value_input)
                .collect::<Result<Vec<_>, _>>()?,
        },
        WitnessValueInput::Union {
            declaration,
            member,
            components,
        } => WitnessValue::Union {
            declaration: wire_node(&declaration)?,
            member,
            components: components
                .into_iter()
                .map(adapt_witness_value_input)
                .collect::<Result<Vec<_>, _>>()?,
        },
        WitnessValueInput::Sequence { elements } => WitnessValue::Sequence(
            elements
                .into_iter()
                .map(adapt_witness_value_input)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        WitnessValueInput::OrderedSet { elements } => WitnessValue::OrderedSet(
            elements
                .into_iter()
                .map(adapt_witness_value_input)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        WitnessValueInput::Set { elements } => WitnessValue::Set(
            elements
                .into_iter()
                .map(adapt_witness_value_input)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        WitnessValueInput::Bag { elements } => WitnessValue::Bag(
            elements
                .into_iter()
                .map(adapt_witness_value_input)
                .collect::<Result<Vec<_>, _>>()?,
        ),
    })
}

fn text_from(value: &serde_json::Map<String, Value>, field: &str) -> Result<String, String> {
    string(member(value, field)?, field).map(str::to_owned)
}

fn hex_bytes(value: &str) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) {
        return Err("hex byte string must have even length".to_owned());
    }
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let text = std::str::from_utf8(pair).map_err(|error| error.to_string())?;
            u8::from_str_radix(text, 16).map_err(|error| error.to_string())
        })
        .collect()
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ReplaySourceInput {
    Witness {
        transcript: String,
    },
    Input {
        assignments: Vec<CanonicalAssignmentInput>,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalAssignmentInput {
    parameter: String,
    value: WitnessValueInput,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum WitnessValueInput {
    Boolean {
        value: bool,
    },
    Integer {
        decimal: String,
    },
    ExactInteger {
        decimal: String,
    },
    Enum {
        declaration: String,
        member: String,
    },
    Text {
        value: String,
    },
    Uuid {
        value: String,
    },
    Timestamp {
        decimal: String,
    },
    String {
        value: String,
    },
    Rational {
        numerator: String,
        denominator: String,
    },
    Decimal {
        coefficient: String,
        scale: String,
    },
    Float32 {
        bits: u64,
    },
    Float64 {
        bits: u64,
    },
    Quantity {
        magnitude: QuantityMagnitudeInput,
        unit: String,
    },
    Reference {
        universe: String,
        object_type: String,
        identity: String,
    },
    Option {
        value: Option<Box<WitnessValueInput>>,
    },
    Record {
        declaration: String,
        fields: Vec<WitnessFieldInput>,
    },
    Tuple {
        declaration: String,
        components: Vec<WitnessValueInput>,
    },
    Union {
        declaration: String,
        member: String,
        components: Vec<WitnessValueInput>,
    },
    Sequence {
        elements: Vec<WitnessValueInput>,
    },
    OrderedSet {
        elements: Vec<WitnessValueInput>,
    },
    Set {
        elements: Vec<WitnessValueInput>,
    },
    Bag {
        elements: Vec<WitnessValueInput>,
    },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum QuantityMagnitudeInput {
    Rational {
        numerator: String,
        denominator: String,
    },
    Decimal {
        coefficient: String,
        scale: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WitnessFieldInput {
    name: String,
    slot: WitnessSlotInput,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum WitnessSlotInput {
    Present { value: WitnessValueInput },
    Absent,
    Null,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ReplayTargetMembersInput {
    Function {
        function: String,
    },
    FrameOperation {
        frame_operation: FrameOperationInput,
    },
    StateClause {
        state_clause: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrameOperationInput {
    object: String,
    operation: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeclaredDomainInput {
    domain: DomainKeyInput,
    kind: Option<DomainKindInput>,
    bound: FiniteBoundInput,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum DomainKeyInput {
    Node {
        node: String,
        path: Vec<String>,
    },
    Population {
        member_type: String,
        ordinal: String,
    },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum DomainKindInput {
    Collection,
    Population,
    Integer,
    Recursive,
    Loop,
    InfiniteTrace,
    Quantity,
}

impl From<DomainKindInput> for DomainKind {
    fn from(value: DomainKindInput) -> Self {
        match value {
            DomainKindInput::Collection => Self::Collection,
            DomainKindInput::Population => Self::Population,
            DomainKindInput::Integer => Self::Integer,
            DomainKindInput::Recursive => Self::Recursive,
            DomainKindInput::Loop => Self::Loop,
            DomainKindInput::InfiniteTrace => Self::InfiniteTrace,
            DomainKindInput::Quantity => Self::Quantity,
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum FiniteBoundInput {
    Cardinality { maximum: String },
    IntegerRange { lower: String, upper: String },
    Depth { maximum: String },
    Variants { members: Vec<String> },
}

fn parse_u64(value: &str, path: &str) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
        .map_err(|error| format!("{path} is not a u64: {error}"))?;
    if parsed.to_string() != value {
        return Err(format!("{path} is not a canonical unsigned decimal"));
    }
    Ok(parsed)
}

fn adapt_scalar_limits(value: &Value) -> Result<ScalarLimits, String> {
    let value = object(value, "accounting_limits")?;
    Ok(ScalarLimits {
        integer_bits: field_u64(value, "integer_bits")?,
        decimal_digits: field_u64(value, "decimal_digits")?,
        scale_expansion: field_u64(value, "scale_expansion")?,
        text_input_bytes: field_u64(value, "text_input_bytes")?,
        text_scalars: field_u64(value, "text_scalars")?,
        normalized_scalars: field_u64(value, "normalized_scalars")?,
        unit_edges: field_u64(value, "unit_edges")?,
        value_occurrences: field_u64(value, "value_occurrences")?,
        work_units: field_u64(value, "work_units")?,
        result_units: field_u64(value, "result_units")?,
    })
}

fn field_u64(value: &serde_json::Map<String, Value>, field: &str) -> Result<u64, String> {
    unsigned(member(value, field)?, field)
}

fn adapt_object_environment(value: &Value) -> Result<ObjectEnvironmentRef, String> {
    let value = object(value, "object_environment")?;
    let labels = object(member(value, "identity")?, "object_environment.identity")?;
    let digest = string(member(value, "digest")?, "object_environment.digest")?;
    let (domain, hex) = digest
        .split_once(':')
        .ok_or_else(|| "object_environment.digest requires domain:hex".to_owned())?;
    let digest = DigestRecord::from_wire(Some(domain), hex).map_err(|error| error.to_string())?;
    ObjectEnvironmentRef::new(
        EnvironmentLabels {
            authority: text_from(labels, "authority")?,
            identity: text_from(labels, "identity")?,
            revision_namespace: text_from(labels, "revision_namespace")?,
            revision: text_from(labels, "revision")?,
        },
        digest,
    )
    .map_err(|error| error.to_string())
}

fn adapt_declared_domain(value: &Value) -> Result<DeclaredDomain, String> {
    let domain: DeclaredDomainInput = serde_json::from_value(value.clone())
        .map_err(|error| format!("declared_domain: {error}"))?;
    let key = match domain.domain {
        DomainKeyInput::Node { node, path } => DomainKey::Node {
            node: wire_node(&node)?,
            path: path
                .iter()
                .map(|item| {
                    item.parse::<u64>()
                        .map_err(|error| format!("domain path index is not u64: {error}"))?
                        .try_into()
                        .map_err(|error| format!("domain path index exceeds u32: {error}"))
                })
                .collect::<Result<Vec<_>, String>>()?,
        },
        DomainKeyInput::Population {
            member_type,
            ordinal,
        } => DomainKey::Population {
            member_type: wire_node(&member_type)?,
            ordinal: ordinal
                .parse::<u64>()
                .map_err(|error| format!("population ordinal is not u64: {error}"))?
                .try_into()
                .map_err(|error| format!("population ordinal exceeds u32: {error}"))?,
        },
    };
    let kind = domain.kind.map(Into::into);
    let bound = match domain.bound {
        FiniteBoundInput::Cardinality { maximum } => {
            FiniteBound::cardinality(parse_u64(&maximum, "bound.maximum")?)
        }
        FiniteBoundInput::IntegerRange { lower, upper } => FiniteBound::integer_range(
            Integer::from_str(&lower).map_err(|error| error.to_string())?,
            Integer::from_str(&upper).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?,
        FiniteBoundInput::Depth { maximum } => {
            FiniteBound::depth(parse_u64(&maximum, "bound.maximum")?)
                .map_err(|error| error.to_string())?
        }
        FiniteBoundInput::Variants { members } => {
            FiniteBound::variants(members).map_err(|error| error.to_string())?
        }
    };
    Ok(DeclaredDomain::new(
        ProofBound::new(key, kind, bound).map_err(|error| error.to_string())?,
    ))
}

fn adapt_byte_provision(
    value: &Value,
    request_file: &InputFile,
    vector: &Vector,
    inputs: &[(&InputFile, Vec<u8>)],
    root: &Path,
) -> Result<Vec<ByteProvisionInput>, String> {
    let parent = Path::new(&request_file.path)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let rows = array(value, "byte_provision")?;
    let mut resolved_paths = BTreeMap::<PathBuf, usize>::new();
    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let row = object(row, "byte_provision row")?;
        let bytes_path = string(member(row, "bytes_path")?, "bytes_path")?;
        let relative = Path::new(bytes_path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(format!("invalid bytes_path {bytes_path:?}"));
        }
        let corpus_relative = parent.join(relative);
        let physical = fs::canonicalize(root.join(&corpus_relative))
            .map_err(|error| format!("bytes_path {bytes_path:?}: {error}"))?;
        if !physical.starts_with(root) {
            return Err(format!("bytes_path {bytes_path:?} escapes corpus root"));
        }
        let matches = vector
            .input
            .files
            .iter()
            .enumerate()
            .filter(|(_, file)| {
                file.kind == InputKind::Bytes && Path::new(&file.path) == corpus_relative
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(format!(
                "bytes_path {bytes_path:?} must name exactly one listed bytes input; found {}",
                matches.len()
            ));
        }
        let (index, _) = matches[0];
        if resolved_paths.insert(physical, index).is_some() {
            return Err(format!("bytes_path {bytes_path:?} is repeated"));
        }
        let (_, bytes) = inputs
            .get(index)
            .ok_or_else(|| "listed bytes input was not read".to_owned())?;
        let digest_domain = if member(row, "digest_domain")?.is_null() {
            None
        } else {
            Some(string(member(row, "digest_domain")?, "digest_domain")?.to_owned())
        };
        result.push((
            digest_domain,
            string(member(row, "digest")?, "digest")?.to_owned(),
            bytes.clone(),
        ));
    }
    Ok(result)
}

fn replay_result_value(
    result: &Result<qsl_replay::ReplayResult, qsl_replay::ReplayRefusal>,
) -> Result<Value, String> {
    match result {
        Ok(qsl_replay::ReplayResult::Witness(value)) => Ok(json!({
            "results": [{"arm": {"witness": {"settlement": witness_settlement(value.settlement()), "category": value.category().as_str(), "value": evaluated_value(value.value())}}}]
        })),
        Ok(qsl_replay::ReplayResult::Input(value)) => Ok(json!({
            "results": [{"arm": {"input": {"settlement": input_settlement(value.settlement()), "category": value.category().as_str(), "value": evaluated_value(value.value())}}}]
        })),
        Ok(_) => Err(
            "FR-350 corpus adapter accepts only the FR-323 Witness/Input replay source arms"
                .to_owned(),
        ),
        Err(qsl_replay::ReplayRefusal::Fault(fault)) => {
            Err(format!("replay internal fault: {fault:?}"))
        }
        Err(refusal) => Ok(json!({"code":refusal.code().as_str(),"cause":refusal.cause()})),
    }
}

fn witness_settlement(value: qsl_replay::WitnessSettlement) -> &'static str {
    match value {
        qsl_replay::WitnessSettlement::ReproducedWithEvaluatedWitness => {
            "reproduced-with-evaluated-witness"
        }
        qsl_replay::WitnessSettlement::Inconclusive => "inconclusive",
    }
}
fn input_settlement(value: qsl_replay::InputSettlement) -> &'static str {
    match value {
        qsl_replay::InputSettlement::ReproducedWithoutWitness => "reproduced-without-witness",
        qsl_replay::InputSettlement::Inconclusive => "inconclusive",
    }
}
fn evaluated_value(value: Option<qsl_replay::EvaluatedValue>) -> Value {
    match value {
        Some(qsl_replay::EvaluatedValue::Boolean(value)) => json!({"kind":"boolean","value":value}),
        None => Value::Null,
    }
}

trait MemberObject {
    fn member(&self, key: &str) -> Option<&Value>;
}
impl MemberObject for Value {
    fn member(&self, key: &str) -> Option<&Value> {
        self.as_object()?.get(key)
    }
}
impl MemberObject for serde_json::Map<String, Value> {
    fn member(&self, key: &str) -> Option<&Value> {
        self.get(key)
    }
}
fn member<'a>(value: &'a impl MemberObject, key: &str) -> Result<&'a Value, String> {
    value
        .member(key)
        .ok_or_else(|| format!("missing member {key:?}"))
}
fn object<'a>(value: &'a Value, path: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{path} must be an object"))
}
fn array<'a>(value: &'a Value, path: &str) -> Result<&'a Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("{path} must be an array"))
}
fn string<'a>(value: &'a Value, path: &str) -> Result<&'a str, String> {
    value
        .as_str()
        .ok_or_else(|| format!("{path} must be a string"))
}
fn boolean(value: &Value, path: &str) -> Result<bool, String> {
    value
        .as_bool()
        .ok_or_else(|| format!("{path} must be a Boolean"))
}
fn unsigned(value: &Value, path: &str) -> Result<u64, String> {
    let decimal = string(value, path)?;
    let parsed = decimal
        .parse::<u64>()
        .map_err(|error| format!("{path} is not a u64: {error}"))?;
    if parsed.to_string() != decimal {
        return Err(format!("{path} is not a canonical unsigned decimal"));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qualification::{
        ConsumerType, Expected, Input, Operation, Subject, VectorApplicability,
    };

    #[test]
    fn tc_970_adapts_the_published_replay_request_fixture() {
        let Some(qspec) = std::env::var_os("QSPEC_DIR").map(PathBuf::from) else {
            return;
        };
        let fixture = qspec.join("proposals/conformance-corpus-v1/fixtures/replay-request.json");
        let schema_path = qspec.join("proposals/conformance-corpus-v1/schema.json");
        if !fixture.is_file() || !schema_path.is_file() {
            eprintln!("skipping TC-970 fixture check: selected QSpec snapshot predates the replay-request fixture");
            return;
        }
        let bytes = fs::read(&fixture).expect("QSpec replay request fixture");
        let value = parse_unique_json(&bytes).expect("unique fixture object keys");
        let schema_bytes = fs::read(schema_path).expect("QSpec corpus schema");
        let schema_value: Value = serde_json::from_slice(&schema_bytes).expect("schema JSON");
        if schema_value["$defs"]["ReplayRequest"].is_null() {
            eprintln!(
                "skipping TC-970 fixture check: selected QSpec schema predates ReplayRequest"
            );
            return;
        }
        let replay_schema_value = json!({
            "$schema": schema_value["$schema"],
            "$defs": schema_value["$defs"],
            "$ref": "#/$defs/ReplayRequest"
        });
        let schema = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .compile(&replay_schema_value)
            .expect("ReplayRequest schema");
        assert!(schema.validate(&value).is_ok());

        let request_file = InputFile {
            kind: InputKind::ReplayRequest,
            path: "proposals/conformance-corpus-v1/fixtures/replay-request.json".to_owned(),
        };
        let vector = Vector {
            id: "tc-970-fixture".to_owned(),
            capability_id: "fixture".to_owned(),
            class: "positive".to_owned(),
            mutation_of: None,
            subject: Subject {
                consumer_type: ConsumerType::Runtime,
                operation: Operation::Replay,
            },
            applicability: VectorApplicability {
                consumer_types: vec![ConsumerType::Runtime],
            },
            input: Input {
                files: vec![InputFile {
                    kind: InputKind::ReplayRequest,
                    path: request_file.path.clone(),
                }],
            },
            expected: Expected::Value { value: Value::Null },
        };
        let inputs = vec![(&request_file, bytes)];
        let wire = adapt_request(&value, &request_file, &vector, &inputs, &qspec)
            .expect("the closed corpus request maps to ReplayRequestWire");
        assert!(wire.profile_selections.is_empty());
        assert!(wire.state_environment.entries().is_empty());
        assert_eq!(wire.declared_domains.len(), 1);
        assert!(matches!(wire.source, ReplaySource::Input(_)));
        let decoded = qsl_replay::ReplayRequest::decode(wire, qsl_replay::ReplayLimits::default())
            .expect("mapped request retains FR-071 decode semantics");
        assert_eq!(decoded.backend().as_str(), "fixture.backend");
        assert!(decoded.to_wire().stage_limits.is_empty());

        let (actual, output) = execute(
            &vector,
            &inputs,
            &qspec,
            &qsl_replay::CallerLimits::default(),
            &schema,
        )
        .expect("the replay vector runs through FR-098 twice");
        assert!(actual.get("code").is_some());
        assert!(output.contains("\"operation\":\"replay\""));
    }

    #[test]
    fn bytes_paths_reject_parent_traversal_and_unlisted_files() {
        let root = tempfile::tempdir().expect("temporary corpus");
        fs::create_dir(root.path().join("vectors")).expect("vector directory");
        fs::write(root.path().join("vectors/input.bin"), b"payload").expect("bytes input");
        let vector = Vector {
            id: "bytes-path".to_owned(),
            capability_id: "fixture".to_owned(),
            class: "positive".to_owned(),
            mutation_of: None,
            subject: Subject {
                consumer_type: ConsumerType::Runtime,
                operation: Operation::Replay,
            },
            applicability: VectorApplicability {
                consumer_types: vec![ConsumerType::Runtime],
            },
            input: Input {
                files: vec![
                    InputFile {
                        kind: InputKind::ReplayRequest,
                        path: "vectors/replay.json".to_owned(),
                    },
                    InputFile {
                        kind: InputKind::Bytes,
                        path: "vectors/input.bin".to_owned(),
                    },
                ],
            },
            expected: Expected::Value { value: Value::Null },
        };
        let request = &vector.input.files[0];
        let bytes_file = &vector.input.files[1];
        let inputs = vec![(request, Vec::new()), (bytes_file, b"payload".to_vec())];
        let parent = json!([{"digest_domain":"quire.source.bytes/v1","digest":"0000000000000000000000000000000000000000000000000000000000000000","bytes_path":"../escape.bin"}]);
        assert!(adapt_byte_provision(&parent, request, &vector, &inputs, root.path()).is_err());
        let unlisted = json!([{"digest_domain":"quire.source.bytes/v1","digest":"0000000000000000000000000000000000000000000000000000000000000000","bytes_path":"other.bin"}]);
        assert!(adapt_byte_provision(&unlisted, request, &vector, &inputs, root.path()).is_err());
    }

    #[test]
    fn fr323_replay_result_envelope_keeps_the_fr072_input_arm() {
        let result =
            qsl_replay::ReplayResult::Input(qsl_replay::InputArmResult::settle_range_violation(
                Some(qsl_replay::EvaluatedValue::Boolean(false)),
                Vec::new(),
                qsl_replay::spine::default_accounting(qsl_replay::spine::DEFAULT_WORK_UNITS),
            ));
        let actual = replay_result_value(&Ok(result)).expect("typed result envelope");
        assert_eq!(
            actual["results"][0]["arm"]["input"]["settlement"],
            "reproduced-without-witness"
        );
        assert_eq!(actual["results"].as_array().map(Vec::len), Some(1));
    }
}
