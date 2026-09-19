// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-154: lift a spec-bundle to Semantic IR 2.0.0 bytes, admit them as a
//! domain package selection, and read the admitted document's IR nodes into
//! [`super::domain_package::DomainPackageRecord`]s.
//!
//! Three parts. [`lift_document`] wraps
//! `agent_ix_extraction_frontend::lift::lift`, the sole entry point that
//! turns a spec bundle plus its module roots into IR 2.0.0 document bytes
//! (FCD FR-091..097). [`admit`] runs FR-154's own four-check admission
//! table (`model-complete.md:58`) over a selection and a byte map, and
//! returns the package's raw bytes. [`read_records`] is the per-node reader:
//! it dispatches every `types[]` entry purely by the `meaning` its `kind`
//! resolves to in the document's own `constructs[]` table (FR-208), never by
//! `kind.name`/`kind.module` directly.
#![allow(
    clippy::result_large_err,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline, matching state::evaluation's typed-failure precedent"
)]

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use agent_ix_extraction_frontend::lift::{lift, LiftOutcome, LiftRequest};
use agent_ix_extraction_frontend::{Diagnostic, Refusal};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::diagnostic::Code;
use crate::model::domain_package::{
    ComponentRecord, DomainPackageRecord, DomainPackageRef, EndpointRecord, FieldMemberRecord,
    Multiplicity, ObjectTypeRecord, OperationEffect, OperationMemberRecord,
    OperationParameterRecord, OperationResult, PortDirection, RelationshipDirection,
    RelationshipEnd, RelationshipRecord,
};
use crate::model::key::{hex, DeclarationKey, SHA256_JCS_DIGEST_DOMAIN};
use crate::model::normalize::{ModelRefusal, ModelRefusalCause};

/// A Quire meaning id (FR-208): the sole legitimate way to determine what a
/// construct or type definition IS. Kind names and modules are never
/// matched directly; every reader dispatches on this table's values.
pub mod meaning {
    pub const OBJECT_TYPE: &str = "quire.meaning.model.object-type/v1";
    pub const VALUE_TYPE: &str = "quire.meaning.model.value-type/v1";
    pub const RECORD_VALUE_TYPE: &str = "quire.meaning.model.record-value-type/v1";
    pub const VARIANT_TYPE: &str = "quire.meaning.model.variant-type/v1";
    pub const EVENT_TYPE: &str = "quire.meaning.model.event-type/v1";
    pub const STATE_MACHINE: &str = "quire.meaning.model.state-machine/v1";
    pub const PROCESS: &str = "quire.meaning.model.process/v1";
    pub const PERSISTENCE_INTERFACE: &str = "quire.meaning.model.persistence-interface/v1";
    pub const NAMESPACE: &str = "quire.meaning.model.namespace/v1";
    pub const POPULATION: &str = "quire.meaning.model.population/v1";
    pub const SYSTEMS_PART: &str = "quire.meaning.systems.part/v1";
    pub const SYSTEMS_PORT: &str = "quire.meaning.systems.port/v1";
    pub const SYSTEMS_INTERFACE: &str = "quire.meaning.systems.interface/v1";
    pub const SYSTEMS_CONNECTION: &str = "quire.meaning.systems.connection/v1";
    pub const SYSTEMS_ALLOCATION: &str = "quire.meaning.systems.allocation/v1";
}

/// FCD `agent_ix_extraction_frontend::lift` did not produce a document.
///
/// Wraps FCD's own outcome types rather than stringifying them (matching
/// `crate::command::extraction::ExtractionError`'s `Context(Vec<SemanticFailure>)`):
/// the caller's diagnostics stay FCD-owned and typed.
#[derive(Debug, thiserror::Error)]
pub enum LiftFailure {
    /// A scratch directory for `lift`'s required output path could not be
    /// created.
    #[error("could not create a scratch directory for lift output: {0}")]
    Scratch(#[source] std::io::Error),
    /// FCD refused the bundle outright (parse, manifest or module failure).
    #[error("bundle refused: {0}")]
    Refused(Refusal),
    /// FCD produced at least one blocking diagnostic (schema or rules
    /// failure); no document was written.
    #[error("{} blocking diagnostic(s)", .0.len())]
    Blocked(Vec<Diagnostic>),
}

/// Lift one spec bundle to Semantic IR 2.0.0 document bytes.
///
/// `lift` (FCD FR-099) always writes its document to a required output
/// path; the diagnostics and provenance sidecars are skipped (`None`).
/// The scratch directory is removed when this function returns.
pub fn lift_document(bundle_root: &Path, module_roots: &[PathBuf]) -> Result<Vec<u8>, LiftFailure> {
    let scratch = tempfile::tempdir().map_err(LiftFailure::Scratch)?;
    let request = LiftRequest {
        bundle_root: bundle_root.to_path_buf(),
        module_roots: module_roots.to_vec(),
        out: scratch.path().join("document.json"),
        diagnostics: None,
        provenance: None,
    };
    match lift(&request) {
        LiftOutcome::Refused(refusal) => Err(LiftFailure::Refused(refusal)),
        LiftOutcome::Blocked { diagnostics } => Err(LiftFailure::Blocked(diagnostics)),
        LiftOutcome::Written { document, .. } => Ok(document),
    }
}

/// FR-154 Intake's four-check admission table (`model-complete.md:58-70`).
///
/// Runs the checks in table order and stops at the first failure: digest
/// domain, then byte presence, then digest equality, then the package's own
/// declared identity/version. Returns the admitted selection and the
/// package's raw bytes; [`read_records`] reads those bytes' IR nodes into
/// [`super::domain_package::DomainPackageRecord`]s as a separate, later step.
pub fn admit(
    identity: &str,
    version: &str,
    digest_domain: &str,
    digest: [u8; 32],
    bytes_by_digest: &BTreeMap<[u8; 32], Vec<u8>>,
) -> Result<(DomainPackageRef, Vec<u8>), ModelRefusal> {
    if digest_domain != SHA256_JCS_DIGEST_DOMAIN {
        return Err(ModelRefusal {
            code: Code::StaleDependency,
            cause: ModelRefusalCause::DigestDomainMismatch {
                expected: SHA256_JCS_DIGEST_DOMAIN,
                actual: digest_domain.to_owned(),
            },
            detail: format!(
                "domain package selection {identity}@{version} names digest domain \
                 {digest_domain:?}, not {SHA256_JCS_DIGEST_DOMAIN:?}"
            ),
        });
    }
    let selection = DomainPackageRef {
        identity: identity.to_owned(),
        version: version.to_owned(),
        digest,
    };
    let Some(bytes) = bytes_by_digest.get(&digest) else {
        return Err(ModelRefusal {
            code: Code::MissingImport,
            cause: ModelRefusalCause::MissingSelection {
                selection: selection.clone(),
            },
            detail: format!("no package bytes supplied under digest {}", hex(&digest)),
        });
    };
    let actual_digest: [u8; 32] = Sha256::digest(bytes.as_slice()).into();
    if actual_digest != digest {
        return Err(ModelRefusal {
            code: Code::StaleDependency,
            cause: ModelRefusalCause::ByteDigestMismatch {
                expected: digest,
                actual: actual_digest,
            },
            detail: format!(
                "domain package {identity}@{version} bytes hash to {}, not the selected {}",
                hex(&actual_digest),
                hex(&digest)
            ),
        });
    }
    // The package's own declared identity/version, read defensively: bytes
    // that fail to parse or omit `package` simply supply no identity/version,
    // which check 4 below reports as a `wrong-model-selection` mismatch
    // rather than a separate malformed-package case FR-154's table does not
    // name.
    let parsed: Option<serde_json::Value> = serde_json::from_slice(bytes).ok();
    let package = parsed.as_ref().and_then(|value| value.get("package"));
    let actual_identity = package
        .and_then(|value| value.get("identity"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let actual_version = package
        .and_then(|value| value.get("version"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    if actual_identity != identity || actual_version != version {
        return Err(ModelRefusal {
            code: Code::InvalidModelBinding,
            cause: ModelRefusalCause::WrongModelSelection {
                selection: selection.clone(),
                actual_identity: actual_identity.clone(),
                actual_version: actual_version.clone(),
            },
            detail: format!(
                "domain package selection names {identity}@{version} but the package \
                 declares {actual_identity}@{actual_version}"
            ),
        });
    }
    Ok((selection, bytes.clone()))
}

/// An admitted Semantic IR 2.0.0 document's bytes are not well-formed IR.
///
/// `admit` already checked digest and package identity/version against the
/// caller's selection; this is the separate, later failure a document that
/// passed admission but does not itself parse as the IR 2.0.0 shape raises.
#[derive(Debug, thiserror::Error)]
pub enum ReadFailure {
    /// The admitted bytes do not parse as JSON at all.
    #[error("admitted document bytes do not parse as JSON: {0}")]
    NotJson(#[source] serde_json::Error),
    /// A required member is missing, or present with the wrong shape.
    #[error("{at}: {reason}")]
    Malformed {
        /// A dotted path into the document naming the offending member.
        at: String,
        /// What was expected there.
        reason: &'static str,
    },
    /// A type's construct meaning is real (FR-208) but this reader does not
    /// yet turn it into a [`DomainPackageRecord`] — refused rather than
    /// silently dropped.
    #[error("{node}: construct meaning {meaning:?} has no reader yet")]
    UnsupportedMeaning {
        /// The type's own IR node identity.
        node: String,
        /// The unresolved meaning id.
        meaning: String,
    },
}

fn malformed(at: impl Into<String>, reason: &'static str) -> ReadFailure {
    ReadFailure::Malformed {
        at: at.into(),
        reason,
    }
}

fn declaration_key(package: &str, node: &str) -> DeclarationKey {
    DeclarationKey {
        package: package.to_owned(),
        node: node.to_owned(),
    }
}

fn str_field<'a>(value: &'a Value, at: &str, field: &'static str) -> Result<&'a str, ReadFailure> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| malformed(format!("{at}.{field}"), "missing or not a string"))
}

fn opt_str_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

/// `value[field]` as an array, or `&[]` when the member is absent entirely
/// (every array member the readers below touch is optional at the wire
/// level, e.g. a business type's `fields`/`operations`, a field's `subsets`).
fn array_field<'a>(
    value: &'a Value,
    at: &str,
    field: &'static str,
) -> Result<&'a [Value], ReadFailure> {
    match value.get(field) {
        None => Ok(&[]),
        Some(array) => array
            .as_array()
            .map(Vec::as_slice)
            .ok_or_else(|| malformed(format!("{at}.{field}"), "not an array")),
    }
}

fn identity_keys(
    package: &str,
    value: &Value,
    at: &str,
    field: &'static str,
) -> Result<Vec<DeclarationKey>, ReadFailure> {
    array_field(value, at, field)?
        .iter()
        .enumerate()
        .map(|(position, item)| {
            item.as_str()
                .map(|node| declaration_key(package, node))
                .ok_or_else(|| malformed(format!("{at}.{field}[{position}]"), "not a string"))
        })
        .collect()
}

fn read_multiplicity(
    value: &Value,
    at: &str,
    field: &'static str,
) -> Result<Multiplicity, ReadFailure> {
    let object = value
        .get(field)
        .ok_or_else(|| malformed(format!("{at}.{field}"), "missing multiplicity"))?;
    let lower = object.get("lower").and_then(Value::as_u64).ok_or_else(|| {
        malformed(
            format!("{at}.{field}.lower"),
            "missing or not a non-negative integer",
        )
    })?;
    let upper = match object.get("upper") {
        None | Some(Value::Null) => None,
        Some(value) => Some(value.as_u64().ok_or_else(|| {
            malformed(format!("{at}.{field}.upper"), "not a non-negative integer")
        })?),
    };
    Ok(Multiplicity {
        lower,
        upper,
        // Semantic IR 2.0.0's own multiplicity shape (`lower`, `upper`)
        // carries neither ordering nor uniqueness; every reader below
        // states FR-113's own field/end defaults.
        ordered: false,
        unique: true,
    })
}

fn read_operation_parameter(
    package: &str,
    owner_node: &str,
    param: &Value,
) -> Result<OperationParameterRecord, ReadFailure> {
    let node = str_field(param, owner_node, "identity")?;
    let value_type = str_field(param, node, "typeRef")?;
    let multiplicity = read_multiplicity(param, node, "multiplicity")?;
    Ok(OperationParameterRecord {
        key: declaration_key(package, node),
        value_type: declaration_key(package, value_type),
        multiplicity,
    })
}

fn read_operation_member(
    package: &str,
    owner_node: &str,
    operation: &Value,
) -> Result<OperationMemberRecord, ReadFailure> {
    let node = str_field(operation, owner_node, "identity")?;
    let parameters = array_field(operation, node, "params")?
        .iter()
        .map(|param| read_operation_parameter(package, node, param))
        .collect::<Result<Vec<_>, _>>()?;
    let result = match operation.get("returns") {
        None => None,
        Some(returns) => {
            let value_type = str_field(returns, node, "typeRef")?;
            let multiplicity = read_multiplicity(returns, node, "multiplicity")?;
            Some(OperationResult {
                value_type: declaration_key(package, value_type),
                multiplicity,
            })
        }
    };
    let has_own_precondition = !array_field(operation, node, "pre")?.is_empty();
    Ok(OperationMemberRecord {
        key: declaration_key(package, node),
        owner: declaration_key(package, owner_node),
        parameters,
        result,
        // The IR 2.0.0 operation schema (`OPERATION_MEMBERS`) carries no
        // `redefines` or body member at all, and its own `frame` names
        // dotted feature paths, not declaration identities -- resolving one
        // to a member's original key needs the qualified-name binder
        // `crate::model::systems`'s own module docs scope out of
        // `crate::model` entirely. `OperationEffect::default()` loses
        // nothing this reader could otherwise state.
        effect: OperationEffect::default(),
        has_own_precondition,
        // FR-146's expression parser is out of scope for `crate::model`
        // (`domain_package::PostconditionClause`'s own module docs); this
        // reader states none rather than inventing one.
        own_postcondition_clauses: Vec::new(),
        has_body: false,
        redefines: None,
    })
}

fn read_field_member(
    package: &str,
    owner_node: &str,
    field: &Value,
) -> Result<FieldMemberRecord, ReadFailure> {
    let node = str_field(field, owner_node, "identity")?;
    let value_type = str_field(field, node, "typeRef")?;
    let multiplicity = read_multiplicity(field, node, "multiplicity")?;
    let subsets = identity_keys(package, field, node, "subsets")?;
    let redefines =
        opt_str_field(field, "redefines").map(|target| declaration_key(package, target));
    Ok(FieldMemberRecord {
        key: declaration_key(package, node),
        owner: declaration_key(package, owner_node),
        value_type: declaration_key(package, value_type),
        multiplicity,
        subsets,
        redefines,
    })
}

/// Reads an `OBJECT_TYPE`/`RECORD_VALUE_TYPE`/`SYSTEMS_INTERFACE` type into
/// an [`ObjectTypeRecord`] plus one [`FieldMemberRecord`]/
/// [`OperationMemberRecord`] per declared field/operation, appending all of
/// them to `records` in the type's own field-then-operation order.
fn read_object_type(
    package: &str,
    type_value: &Value,
    node: &str,
    is_interface: bool,
    records: &mut Vec<DomainPackageRecord>,
) -> Result<(), ReadFailure> {
    let supertypes = identity_keys(package, type_value, node, "supertypes")?;
    let abstract_type = type_value
        .get("abstract")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let interface_features = if is_interface {
        Some(identity_keys(package, type_value, node, "featureOrder")?)
    } else {
        None
    };
    records.push(DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: declaration_key(package, node),
        interface_features,
        abstract_type,
        supertypes,
    }));
    for field in array_field(type_value, node, "fields")? {
        records.push(DomainPackageRecord::FieldMember(read_field_member(
            package, node, field,
        )?));
    }
    for operation in array_field(type_value, node, "operations")? {
        records.push(DomainPackageRecord::OperationMember(read_operation_member(
            package, node, operation,
        )?));
    }
    Ok(())
}

fn read_component(
    package: &str,
    type_value: &Value,
    node: &str,
) -> Result<ComponentRecord, ReadFailure> {
    let owner = str_field(type_value, node, "owner")?;
    let value_type = str_field(type_value, node, "declaredType")?;
    let multiplicity = read_multiplicity(type_value, node, "multiplicity")?;
    Ok(ComponentRecord {
        key: declaration_key(package, node),
        owning_type: declaration_key(package, owner),
        value_type: declaration_key(package, value_type),
        multiplicity,
        // FR-152's Part kind is exactly this meaning (`meaning::SYSTEMS_PART`);
        // there is no separate wire capability to read.
        has_part_signature: true,
    })
}

fn read_endpoint(
    package: &str,
    type_value: &Value,
    node: &str,
) -> Result<EndpointRecord, ReadFailure> {
    let owner = str_field(type_value, node, "owner")?;
    let value_type = str_field(type_value, node, "interfaceType")?;
    let multiplicity = read_multiplicity(type_value, node, "multiplicity")?;
    let direction = match opt_str_field(type_value, "direction") {
        Some("in") => Some(PortDirection::In),
        Some("out") => Some(PortDirection::Out),
        Some("inout") => Some(PortDirection::InOut),
        Some(_) => return Err(malformed(format!("{node}.direction"), "not in/out/inout")),
        None => None,
    };
    Ok(EndpointRecord {
        key: declaration_key(package, node),
        owning_component: declaration_key(package, owner),
        value_type: declaration_key(package, value_type),
        direction,
        multiplicity,
    })
}

fn read_connection(
    package: &str,
    type_value: &Value,
    node: &str,
) -> Result<RelationshipRecord, ReadFailure> {
    let source_end = type_value
        .get("sourceEnd")
        .ok_or_else(|| malformed(format!("{node}.sourceEnd"), "missing"))?;
    let target_end = type_value
        .get("targetEnd")
        .ok_or_else(|| malformed(format!("{node}.targetEnd"), "missing"))?;
    let source_type = str_field(source_end, node, "type")?;
    let target_type = str_field(target_end, node, "type")?;
    let source_multiplicity = read_multiplicity(source_end, node, "multiplicity")?;
    let target_multiplicity = read_multiplicity(target_end, node, "multiplicity")?;
    let direction = match str_field(type_value, node, "flowDirection")? {
        "source-to-target" => RelationshipDirection::SourceToTarget,
        "target-to-source" => RelationshipDirection::TargetToSource,
        "bidirectional" => RelationshipDirection::Bidirectional,
        _ => {
            return Err(malformed(
                format!("{node}.flowDirection"),
                "not a known direction",
            ))
        }
    };
    Ok(RelationshipRecord {
        key: declaration_key(package, node),
        source: RelationshipEnd {
            type_identity: declaration_key(package, source_type),
            multiplicity: source_multiplicity,
        },
        target: RelationshipEnd {
            type_identity: declaration_key(package, target_type),
            multiplicity: target_multiplicity,
        },
        // Semantic IR 2.0.0's Connection-kind type node carries no
        // `semantics.category` member of its own; the resolved construct
        // meaning (`meaning::SYSTEMS_CONNECTION`) already establishes this
        // is a connection, so this reader states that fact directly rather
        // than reading a field the wire format does not carry here.
        category: "connection".to_owned(),
        direction,
    })
}

fn read_allocation(
    package: &str,
    type_value: &Value,
    node: &str,
) -> Result<RelationshipRecord, ReadFailure> {
    let source = str_field(type_value, node, "sourceElement")?;
    let target = str_field(type_value, node, "targetElement")?;
    // FR-152's Allocation names only a source and a target element key
    // (`FR-152-bind-systems-model-structures.md`: "Allocation compatibility
    // has no other rule"); the wire node carries no multiplicity or
    // direction for either end. `check_allocation` never inspects either
    // field (only `check_connection`, gated on `Kind::Connection`, does), so
    // a fixed 1..1 multiplicity and `Undirected` -- a direction
    // `check_connection`'s own port-direction table never admits, and which
    // never runs for an Allocation-classified relationship -- stand in as
    // this reader's own defaults rather than invented data a caller could
    // mistake for something the producer declared.
    let unstated_multiplicity = Multiplicity {
        lower: 1,
        upper: Some(1),
        ordered: false,
        unique: true,
    };
    Ok(RelationshipRecord {
        key: declaration_key(package, node),
        source: RelationshipEnd {
            type_identity: declaration_key(package, source),
            multiplicity: unstated_multiplicity,
        },
        target: RelationshipEnd {
            type_identity: declaration_key(package, target),
            multiplicity: unstated_multiplicity,
        },
        category: "allocation".to_owned(),
        direction: RelationshipDirection::Undirected,
    })
}

/// The `{module, name} -> meaning` lookup a document's own `constructs[]`
/// table states (FR-208): the sole legitimate way a type's `kind` resolves
/// to what it IS.
fn meaning_index(document: &Value) -> Result<HashMap<(String, String), String>, ReadFailure> {
    let constructs = document
        .get("constructs")
        .and_then(Value::as_array)
        .ok_or_else(|| malformed("$.constructs", "missing or not an array"))?;
    let mut index = HashMap::new();
    for (position, entry) in constructs.iter().enumerate() {
        let at = format!("$.constructs[{position}]");
        let kind = entry
            .get("kind")
            .ok_or_else(|| malformed(format!("{at}.kind"), "missing"))?;
        let module = str_field(kind, &at, "module")?;
        let name = str_field(kind, &at, "name")?;
        let meaning = entry
            .get("construct")
            .and_then(|construct| construct.get("meaning"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                malformed(format!("{at}.construct.meaning"), "missing or not a string")
            })?;
        index.insert((module.to_owned(), name.to_owned()), meaning.to_owned());
    }
    Ok(index)
}

/// Reads an admitted Semantic IR 2.0.0 document's `types[]` into
/// [`DomainPackageRecord`]s under `package_identity`, dispatching each type
/// purely by the `meaning` its `kind` resolves to in the document's own
/// `constructs[]` table (FR-208) -- never by `kind.name`/`kind.module`
/// directly.
///
/// Covers the meanings a domain-object/systems-model bundle actually
/// carries: object and record-value types, systems interfaces, parts,
/// ports, connections and allocations. A type whose resolved meaning names
/// a kind this reader does not yet cover refuses with
/// [`ReadFailure::UnsupportedMeaning`] rather than silently dropping the
/// node. A bare-string-kind type (`scalar`, `alias`, ...) names no
/// `constructs[]` entry at all and is skipped: none of the readers above
/// need a scalar's own bounds to resolve a field's, parameter's or result's
/// `value_type` reference, since [`super::normalize::validate_references`]
/// does not check that reference either.
pub fn read_records(
    package_identity: &str,
    document: &[u8],
) -> Result<Vec<DomainPackageRecord>, ReadFailure> {
    let parsed: Value = serde_json::from_slice(document).map_err(ReadFailure::NotJson)?;
    let meanings = meaning_index(&parsed)?;
    let types = parsed
        .get("types")
        .and_then(Value::as_array)
        .ok_or_else(|| malformed("$.types", "missing or not an array"))?;
    let mut records = Vec::new();
    for (position, type_value) in types.iter().enumerate() {
        let at = format!("$.types[{position}]");
        let node = str_field(type_value, &at, "identity")?;
        let Some(kind) = type_value.get("kind").filter(|kind| kind.is_object()) else {
            // A bare-string kind (`"scalar"`, `"alias"`, ...): a core kind,
            // not a constructs-table entry, and out of this reader's scope.
            continue;
        };
        let module = str_field(kind, node, "module")?;
        let name = str_field(kind, node, "name")?;
        let Some(construct_meaning) = meanings.get(&(module.to_owned(), name.to_owned())) else {
            return Err(malformed(
                format!("{node}.kind"),
                "names no constructs[] entry",
            ));
        };
        match construct_meaning.as_str() {
            meaning::OBJECT_TYPE | meaning::RECORD_VALUE_TYPE => {
                read_object_type(package_identity, type_value, node, false, &mut records)?;
            }
            meaning::SYSTEMS_INTERFACE => {
                read_object_type(package_identity, type_value, node, true, &mut records)?;
            }
            meaning::SYSTEMS_PART => {
                records.push(DomainPackageRecord::Component(read_component(
                    package_identity,
                    type_value,
                    node,
                )?));
            }
            meaning::SYSTEMS_PORT => {
                records.push(DomainPackageRecord::Endpoint(read_endpoint(
                    package_identity,
                    type_value,
                    node,
                )?));
            }
            meaning::SYSTEMS_CONNECTION => {
                records.push(DomainPackageRecord::Relationship(read_connection(
                    package_identity,
                    type_value,
                    node,
                )?));
            }
            meaning::SYSTEMS_ALLOCATION => {
                records.push(DomainPackageRecord::Relationship(read_allocation(
                    package_identity,
                    type_value,
                    node,
                )?));
            }
            other => {
                return Err(ReadFailure::UnsupportedMeaning {
                    node: node.to_owned(),
                    meaning: other.to_owned(),
                });
            }
        }
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest_of(bytes: &[u8]) -> [u8; 32] {
        Sha256::digest(bytes).into()
    }

    fn package_bytes(identity: &str, version: &str) -> Vec<u8> {
        serde_json::json!({
            "contractVersion": "2.0.0",
            "package": {"identity": identity, "version": version},
        })
        .to_string()
        .into_bytes()
    }

    #[test]
    fn admits_matching_selection() {
        let bytes = package_bytes("acme/orders", "1");
        let digest = digest_of(&bytes);
        let mut map = BTreeMap::new();
        map.insert(digest, bytes.clone());
        let (selection, admitted_bytes) =
            admit("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest, &map).unwrap();
        assert_eq!(selection.identity, "acme/orders");
        assert_eq!(selection.version, "1");
        assert_eq!(selection.digest, digest);
        assert_eq!(admitted_bytes, bytes);
    }

    #[test]
    fn refuses_foreign_digest_domain() {
        let map = BTreeMap::new();
        let refusal = admit("acme/orders", "1", "sha1", [0; 32], &map).unwrap_err();
        assert_eq!(refusal.code, Code::StaleDependency);
        assert_eq!(refusal.cause.as_str(), "digest-domain-mismatch");
    }

    #[test]
    fn refuses_missing_bytes() {
        let map = BTreeMap::new();
        let refusal =
            admit("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, [0; 32], &map).unwrap_err();
        assert_eq!(refusal.code, Code::MissingImport);
        assert_eq!(refusal.cause.as_str(), "missing-selection");
    }

    #[test]
    fn refuses_byte_digest_mismatch() {
        let bytes = package_bytes("acme/orders", "1");
        let wrong_digest = digest_of(b"not the package");
        let mut map = BTreeMap::new();
        map.insert(wrong_digest, bytes);
        let refusal = admit(
            "acme/orders",
            "1",
            SHA256_JCS_DIGEST_DOMAIN,
            wrong_digest,
            &map,
        )
        .unwrap_err();
        assert_eq!(refusal.code, Code::StaleDependency);
        assert_eq!(refusal.cause.as_str(), "byte-digest-mismatch");
    }

    #[test]
    fn refuses_wrong_package_identity() {
        let bytes = package_bytes("acme/other", "1");
        let digest = digest_of(&bytes);
        let mut map = BTreeMap::new();
        map.insert(digest, bytes);
        let refusal =
            admit("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest, &map).unwrap_err();
        assert_eq!(refusal.code, Code::InvalidModelBinding);
        assert_eq!(refusal.cause.as_str(), "wrong-model-selection");
    }

    #[test]
    fn refuses_non_json_bytes_as_wrong_selection() {
        let bytes = b"not json".to_vec();
        let digest = digest_of(&bytes);
        let mut map = BTreeMap::new();
        map.insert(digest, bytes);
        let refusal =
            admit("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest, &map).unwrap_err();
        assert_eq!(refusal.cause.as_str(), "wrong-model-selection");
    }
}
