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
//! it dispatches every `types[]`/`populations[]` entry purely by the
//! `meaning` its `kind` resolves to in the document's own `constructs[]`
//! table (FR-208), never by `kind.name`/`kind.module` directly.
//!
//! **This reader binds only what QSpec admits and refuses everything else.**
//! It never invents a value and never drops data. Four shapes FCD emits
//! today are FCD gaps (filed as FCD #199): bare-string core kinds
//! (`"scalar"`/`"alias"`/...) declared directly in `types[]`, FCD's own
//! identity form (`ix://<pkg>/type/<id>`, `ix://<pkg>/field/<Owner>-<name>`)
//! rather than QSpec's (`ix://<pkg>/<id>`, `<owner>/<name>`), a type's inline
//! `relationships[]` (FCD's `verb`/`category`/`composite` shape carries
//! neither the role nor the full direction vocabulary QSpec's own
//! Relationships row requires), and a non-empty operation `frame`. This
//! reader refuses all four rather than working around them.
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
    AllocationRecord, ComponentRecord, DomainPackageRecord, DomainPackageRef, EndpointRecord,
    Extent, FieldMemberRecord, ModelSelection, Multiplicity, ObjectTypeRecord, OperationEffect,
    OperationMemberRecord, OperationParameterRecord, OperationResult, PopulationRecord,
    PortDirection, RelationshipDirection, RelationshipEnd, RelationshipRecord,
};
use crate::model::key::{hex, jcs_bytes, DeclarationKey, SHA256_JCS_DIGEST_DOMAIN};
use crate::model::normalize::{ModelRefusal, ModelRefusalCause};
use crate::model::refusal::SourceSpan;

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

    /// Every meaning id FR-208 declares, as this reader knows it. A type's
    /// resolved meaning outside this closed list is not a real FR-208
    /// meaning at all (`invalid_model_binding`/`malformed-declaration`); one
    /// inside it but not covered by [`super::read_type_node`]'s dispatch is
    /// instead a known-but-unsupported declaration form
    /// (`unsupported_construct`/`declaration-form`) -- two different
    /// refusal causes for two different defects.
    pub const ALL: &[&str] = &[
        OBJECT_TYPE,
        VALUE_TYPE,
        RECORD_VALUE_TYPE,
        VARIANT_TYPE,
        EVENT_TYPE,
        STATE_MACHINE,
        PROCESS,
        PERSISTENCE_INTERFACE,
        NAMESPACE,
        POPULATION,
        SYSTEMS_PART,
        SYSTEMS_PORT,
        SYSTEMS_INTERFACE,
        SYSTEMS_CONNECTION,
        SYSTEMS_ALLOCATION,
    ];
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
/// declared identity/version. Check 3 (`model-complete.md:69`) is "SHA-256
/// over the package's JCS bytes equals the selected digest": when the
/// admitted bytes parse as JSON, the digest is taken over this crate's own
/// RFC 8785 JCS canonicalisation of the parsed value, not the raw bytes
/// verbatim, so two byte-different-but-JCS-equivalent encodings of the same
/// document (e.g. a whitespace variant) digest identically. Bytes that do
/// not parse as JSON at all have no JCS form to canonicalize; the digest is
/// then taken over the raw bytes, which a producer that actually selected
/// this digest could never have done for non-JSON bytes either, so this
/// still reports the real mismatch rather than fabricating a match.
///
/// Returns the admitted selection and a borrow of the package's raw bytes
/// (never cloned: the caller already owns them in `bytes_by_digest`); reads
/// its own referent past this call for [`read_records`] to read further.
pub fn admit<'a>(
    selection: &ModelSelection,
    bytes_by_digest: &'a BTreeMap<[u8; 32], Vec<u8>>,
) -> Result<(DomainPackageRef, &'a [u8]), ModelRefusal> {
    if selection.digest_domain != SHA256_JCS_DIGEST_DOMAIN {
        return Err(ModelRefusal {
            code: Code::StaleDependency,
            cause: ModelRefusalCause::DigestDomainMismatch {
                expected: SHA256_JCS_DIGEST_DOMAIN,
                actual: selection.digest_domain.clone(),
            },
            detail: format!(
                "domain package selection {}@{} names digest domain {:?}, not {SHA256_JCS_DIGEST_DOMAIN:?}",
                selection.identity, selection.version, selection.digest_domain
            ),
        });
    }
    let package_ref = DomainPackageRef {
        identity: selection.identity.clone(),
        version: selection.version.clone(),
        digest: selection.digest,
    };
    let Some(bytes) = bytes_by_digest.get(&selection.digest) else {
        return Err(ModelRefusal {
            code: Code::MissingImport,
            cause: ModelRefusalCause::MissingSelection {
                selection: package_ref.clone(),
            },
            detail: format!(
                "no package bytes supplied under digest {}",
                hex(&selection.digest)
            ),
        });
    };
    let parsed: Option<Value> = serde_json::from_slice(bytes).ok();
    let actual_digest: [u8; 32] = match &parsed {
        Some(value) => Sha256::digest(jcs_bytes(value)).into(),
        None => Sha256::digest(bytes.as_slice()).into(),
    };
    if actual_digest != selection.digest {
        return Err(ModelRefusal {
            code: Code::StaleDependency,
            cause: ModelRefusalCause::ByteDigestMismatch {
                expected: selection.digest,
                actual: actual_digest,
            },
            detail: format!(
                "domain package {}@{} bytes hash to {}, not the selected {}",
                selection.identity,
                selection.version,
                hex(&actual_digest),
                hex(&selection.digest)
            ),
        });
    }
    // The package's own declared identity/version, read defensively: bytes
    // that fail to parse or omit `package` simply supply no identity/version,
    // which check 4 below reports as a `wrong-model-selection` mismatch
    // rather than a separate malformed-package case FR-154's table does not
    // name.
    let package = parsed.as_ref().and_then(|value| value.get("package"));
    let actual_identity = package
        .and_then(|value| value.get("identity"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let actual_version = package
        .and_then(|value| value.get("version"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    if actual_identity != selection.identity || actual_version != selection.version {
        return Err(ModelRefusal {
            code: Code::InvalidModelBinding,
            cause: ModelRefusalCause::WrongModelSelection {
                selection: package_ref.clone(),
                actual_identity: actual_identity.clone(),
                actual_version: actual_version.clone(),
            },
            detail: format!(
                "domain package selection names {}@{} but the package declares {actual_identity}@{actual_version}",
                selection.identity, selection.version
            ),
        });
    }
    Ok((package_ref, bytes.as_slice()))
}

// ---------------------------------------------------------------------------
// Declaration identity form (model-complete.md's Identity row; FCD #199 gap
// 2, "the identity form"): a type-definition node's identity is exactly
// `ix://<package identity>/<artifact id>` (one segment past the package); a
// member node's identity is exactly `<owner identity>/<member name>`. An
// artifact id or member name is `^[A-Za-z][A-Za-z0-9_]*$`.
// ---------------------------------------------------------------------------

fn is_artifact_segment(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The single segment past `ix://<package_identity>/`, when `identity` has
/// exactly that form.
fn type_identity_segment<'a>(package_identity: &str, identity: &'a str) -> Option<&'a str> {
    let rest = identity
        .strip_prefix("ix://")?
        .strip_prefix(package_identity)?;
    let rest = rest.strip_prefix('/')?;
    (!rest.is_empty() && !rest.contains('/') && is_artifact_segment(rest)).then_some(rest)
}

/// The member name past `<owner_identity>/`, when `identity` has exactly
/// that form.
fn member_identity_name<'a>(owner_identity: &str, identity: &'a str) -> Option<&'a str> {
    let rest = identity.strip_prefix(owner_identity)?.strip_prefix('/')?;
    (!rest.is_empty() && !rest.contains('/') && is_artifact_segment(rest)).then_some(rest)
}

// ---------------------------------------------------------------------------
// Per-node reading context: builds FR-154 malformed-declaration/
// declaration-form refusals carrying the node's own identity, source
// artifact and span (`model-complete.md`:74-83), read from the node's own
// `identity`/`origin` -- never a caller-supplied string that could drift
// from what the node itself declares.
// ---------------------------------------------------------------------------

/// The node's own identity, exactly as the wire supplied it, or (when the
/// wire supplies none readable) the dotted path to the node itself -- still
/// a deterministic, human-locatable label, never a fabricated identity.
fn node_identity_label(value: &Value, at: &str) -> String {
    value
        .get("identity")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| at.to_owned())
}

/// The node's `origin.source.sourceIdentity`/`startLine`/`startColumn`,
/// honestly reflecting FCD's real wire shape (see [`SourceSpan`]'s own doc
/// comment for why this does not match QSpec's own byte-offset span shape).
/// `(None, None)` when the node's origin is `generated`, or is itself
/// absent or not the expected shape.
fn node_span(value: &Value) -> (Option<String>, Option<SourceSpan>) {
    let Some(source) = value.get("origin").and_then(|origin| origin.get("source")) else {
        return (None, None);
    };
    let artifact = source
        .get("sourceIdentity")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let span = match (
        source.get("startLine").and_then(Value::as_u64),
        source.get("startColumn").and_then(Value::as_u64),
    ) {
        (Some(start_line), Some(start_column)) => Some(SourceSpan {
            artifact: artifact.clone().unwrap_or_default(),
            start_line,
            start_column,
        }),
        _ => None,
    };
    (artifact, span)
}

fn malformed_at(value: &Value, at: &str, reason: impl std::fmt::Display) -> ModelRefusal {
    let node = node_identity_label(value, at);
    let (artifact, span) = node_span(value);
    ModelRefusal {
        code: Code::InvalidModelBinding,
        cause: ModelRefusalCause::IntakeMalformedDeclaration {
            node: node.clone(),
            artifact,
            span,
        },
        detail: format!("{at}: {reason}"),
    }
}

fn unsupported_at(value: &Value, at: &str, what: impl Into<String>) -> ModelRefusal {
    let node = node_identity_label(value, at);
    let what = what.into();
    ModelRefusal {
        code: Code::UnsupportedConstruct,
        cause: ModelRefusalCause::UnsupportedDeclarationForm {
            node,
            what: what.clone(),
        },
        detail: format!("{at}: construct meaning/capability {what:?} has no reader yet"),
    }
}

/// One IR node under reading: its own JSON value and dotted document path.
/// Every read helper reports a malformed-declaration/declaration-form
/// refusal against `value`'s own `identity`/`origin`, so a nested check
/// (a field's `typeRef`, a multiplicity's `lower`) still names the owning
/// declaration, not an anonymous sub-object with neither.
struct NodeCtx<'a> {
    value: &'a Value,
    at: String,
}

impl<'a> NodeCtx<'a> {
    fn new(value: &'a Value, at: impl Into<String>) -> Self {
        Self {
            value,
            at: at.into(),
        }
    }

    fn malformed(&self, reason: impl std::fmt::Display) -> ModelRefusal {
        malformed_at(self.value, &self.at, reason)
    }

    fn str_field(&self, field: &'static str) -> Result<&'a str, ModelRefusal> {
        self.value
            .get(field)
            .and_then(Value::as_str)
            .ok_or_else(|| self.malformed(format!("{field}: missing or not a string")))
    }

    fn opt_str_field(&self, field: &str) -> Option<&'a str> {
        self.value.get(field).and_then(Value::as_str)
    }

    /// `value[field]` as an array, or `&[]` when the member is absent
    /// entirely (every array member the readers below touch is optional at
    /// the wire level, e.g. a business type's `fields`/`operations`, a
    /// field's `subsets`).
    fn array_field(&self, field: &'static str) -> Result<&'a [Value], ModelRefusal> {
        match self.value.get(field) {
            None => Ok(&[]),
            Some(array) => array
                .as_array()
                .map(Vec::as_slice)
                .ok_or_else(|| self.malformed(format!("{field}: not an array"))),
        }
    }

    fn identity_keys(
        &self,
        package: &str,
        field: &'static str,
    ) -> Result<Vec<DeclarationKey>, ModelRefusal> {
        self.array_field(field)?
            .iter()
            .enumerate()
            .map(|(position, item)| {
                item.as_str()
                    .map(|node| declaration_key(package, node))
                    .ok_or_else(|| self.malformed(format!("{field}[{position}]: not a string")))
            })
            .collect()
    }

    /// FR-154's own multiplicity shape (`model-complete.md`:167-169):
    /// `{lower, upper, ordered, unique}`. `lower` and `upper` were already
    /// read before this PR; `ordered` and `unique` are real wire members
    /// too (FCD's own `MULTIPLICITY_MEMBERS`), and QSpec admits no default
    /// for either -- a missing one refuses, rather than silently supplying
    /// FR-113's own field/end default the way this reader once did.
    fn multiplicity(&self, field: &'static str) -> Result<Multiplicity, ModelRefusal> {
        let object = self
            .value
            .get(field)
            .ok_or_else(|| self.malformed(format!("{field}: missing multiplicity")))?;
        let lower = object.get("lower").and_then(Value::as_u64).ok_or_else(|| {
            self.malformed(format!(
                "{field}.lower: missing or not a non-negative integer"
            ))
        })?;
        let upper = match object.get("upper") {
            None | Some(Value::Null) => None,
            Some(value) => Some(value.as_u64().ok_or_else(|| {
                self.malformed(format!("{field}.upper: not a non-negative integer"))
            })?),
        };
        let ordered = object
            .get("ordered")
            .and_then(Value::as_bool)
            .ok_or_else(|| self.malformed(format!("{field}.ordered: missing or not a boolean")))?;
        let unique = object
            .get("unique")
            .and_then(Value::as_bool)
            .ok_or_else(|| self.malformed(format!("{field}.unique: missing or not a boolean")))?;
        Ok(Multiplicity {
            lower,
            upper,
            ordered,
            unique,
        })
    }
}

fn declaration_key(package: &str, node: &str) -> DeclarationKey {
    DeclarationKey {
        package: package.to_owned(),
        node: node.to_owned(),
    }
}

/// Validates `identity`'s form against the Identity row and returns the
/// node's own [`DeclarationKey`] built from `at`'s own identity string --
/// never a substituted or normalized one.
fn read_type_identity(
    package: &str,
    ctx: &NodeCtx<'_>,
) -> Result<(&'static str, DeclarationKey), ModelRefusal> {
    let identity = ctx.str_field("identity")?;
    if type_identity_segment(package, identity).is_none() {
        return Err(ctx.malformed(format!(
            "identity: {identity:?} is not ix://{package}/<artifact id>"
        )));
    }
    Ok(("", declaration_key(package, identity)))
}

fn read_operation_parameter(
    package: &str,
    owner_identity: &str,
    param: &Value,
    at: &str,
) -> Result<OperationParameterRecord, ModelRefusal> {
    let ctx = NodeCtx::new(param, at);
    let node = ctx.str_field("identity")?;
    if member_identity_name(owner_identity, node).is_none() {
        return Err(ctx.malformed(format!("identity: {node:?} is not {owner_identity}/<name>")));
    }
    let value_type = ctx.str_field("typeRef")?;
    let multiplicity = ctx.multiplicity("multiplicity")?;
    Ok(OperationParameterRecord {
        key: declaration_key(package, node),
        value_type: declaration_key(package, value_type),
        multiplicity,
    })
}

fn read_operation_member(
    package: &str,
    owner_identity: &str,
    operation: &Value,
    at: &str,
) -> Result<OperationMemberRecord, ModelRefusal> {
    let ctx = NodeCtx::new(operation, at);
    let node = ctx.str_field("identity")?;
    if member_identity_name(owner_identity, node).is_none() {
        return Err(ctx.malformed(format!("identity: {node:?} is not {owner_identity}/<name>")));
    }
    let params_at = format!("{at}.params");
    let parameters = ctx
        .array_field("params")?
        .iter()
        .enumerate()
        .map(|(position, param)| {
            read_operation_parameter(package, node, param, &format!("{params_at}[{position}]"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let result = match operation.get("returns") {
        None => None,
        Some(returns) => {
            let returns_ctx = NodeCtx::new(operation, at);
            let value_type = returns
                .get("typeRef")
                .and_then(Value::as_str)
                .ok_or_else(|| returns_ctx.malformed("returns.typeRef: missing or not a string"))?;
            let multiplicity = returns_ctx.multiplicity_of(returns, "returns.multiplicity")?;
            Some(OperationResult {
                value_type: declaration_key(package, value_type),
                multiplicity,
            })
        }
    };
    let has_own_precondition = !ctx.array_field("pre")?.is_empty();
    // H3 (FCD #199 gap 4): `frame` is a real QSpec capability
    // (model-complete.md's Frames row) this reader does not yet turn into
    // an `OperationEffect`. An absent frame, or one whose `modifies`,
    // `creates` and `deletes` are all absent or empty, carries no effect
    // data at all and reads as `OperationEffect::default()` honestly; a
    // frame that actually declares anything refuses rather than being
    // silently dropped.
    let effect = match operation.get("frame") {
        None => OperationEffect::default(),
        Some(frame) => {
            let non_empty = ["modifies", "creates", "deletes"].iter().any(|member| {
                frame
                    .get(member)
                    .and_then(Value::as_array)
                    .is_some_and(|entries| !entries.is_empty())
            });
            if non_empty {
                return Err(unsupported_at(
                    operation,
                    &format!("{at}.frame"),
                    "operation.frame",
                ));
            }
            OperationEffect::default()
        }
    };
    Ok(OperationMemberRecord {
        key: declaration_key(package, node),
        owner: declaration_key(package, owner_identity),
        parameters,
        result,
        effect,
        has_own_precondition,
        // FR-146's expression parser is out of scope for `crate::model`
        // (`domain_package::PostconditionClause`'s own module docs); this
        // reader states none rather than inventing one.
        own_postcondition_clauses: Vec::new(),
        has_body: false,
        redefines: None,
    })
}

impl<'a> NodeCtx<'a> {
    /// [`Self::multiplicity`], reading from an arbitrary sub-value (e.g. an
    /// operation's `returns` object) rather than `self.value` directly,
    /// while still reporting against `self`'s own node identity/origin.
    fn multiplicity_of(&self, container: &Value, at: &str) -> Result<Multiplicity, ModelRefusal> {
        let object = container
            .get("multiplicity")
            .ok_or_else(|| self.malformed(format!("{at}: missing multiplicity")))?;
        let lower = object.get("lower").and_then(Value::as_u64).ok_or_else(|| {
            self.malformed(format!("{at}.lower: missing or not a non-negative integer"))
        })?;
        let upper = match object.get("upper") {
            None | Some(Value::Null) => None,
            Some(value) => Some(value.as_u64().ok_or_else(|| {
                self.malformed(format!("{at}.upper: not a non-negative integer"))
            })?),
        };
        let ordered = object
            .get("ordered")
            .and_then(Value::as_bool)
            .ok_or_else(|| self.malformed(format!("{at}.ordered: missing or not a boolean")))?;
        let unique = object
            .get("unique")
            .and_then(Value::as_bool)
            .ok_or_else(|| self.malformed(format!("{at}.unique: missing or not a boolean")))?;
        Ok(Multiplicity {
            lower,
            upper,
            ordered,
            unique,
        })
    }
}

fn read_field_member(
    package: &str,
    owner_identity: &str,
    field: &Value,
    at: &str,
) -> Result<FieldMemberRecord, ModelRefusal> {
    let ctx = NodeCtx::new(field, at);
    let node = ctx.str_field("identity")?;
    if member_identity_name(owner_identity, node).is_none() {
        return Err(ctx.malformed(format!("identity: {node:?} is not {owner_identity}/<name>")));
    }
    let value_type = ctx.str_field("typeRef")?;
    let multiplicity = ctx.multiplicity("multiplicity")?;
    let subsets = ctx.identity_keys(package, "subsets")?;
    let redefines = ctx
        .opt_str_field("redefines")
        .map(|target| declaration_key(package, target));
    Ok(FieldMemberRecord {
        key: declaration_key(package, node),
        owner: declaration_key(package, owner_identity),
        value_type: declaration_key(package, value_type),
        multiplicity,
        subsets,
        redefines,
    })
}

/// Reads an `OBJECT_TYPE`/`SYSTEMS_INTERFACE` type into an
/// [`ObjectTypeRecord`] plus one [`FieldMemberRecord`]/[`OperationMemberRecord`]
/// per declared field/operation, appending all of them to `records` in the
/// type's own field-then-operation order.
fn read_object_type(
    package: &str,
    type_value: &Value,
    node: &str,
    at: &str,
    is_interface: bool,
    records: &mut Vec<DomainPackageRecord>,
) -> Result<(), ModelRefusal> {
    let ctx = NodeCtx::new(type_value, at);
    // A type's own inline `relationships[]` is QSpec's Relationship
    // declaration kind (model-complete.md's Relationships row: each end
    // names object-type-or-process, carries a role and a multiplicity, and
    // `direction` is the full source-to-target/target-to-source/
    // bidirectional/undirected vocabulary). FCD's wire shape for it
    // (`verb`/`category`/`composite`/`target`/`multiplicity`) carries
    // neither a role nor that direction vocabulary (FCD #199 gap 3); a
    // non-empty `relationships[]` refuses rather than being read as a
    // shape it is not.
    if !ctx.array_field("relationships")?.is_empty() {
        return Err(unsupported_at(
            type_value,
            &format!("{at}.relationships"),
            "relationships[]",
        ));
    }
    let supertypes = ctx.identity_keys(package, "supertypes")?;
    let abstract_type = match type_value.get("abstract") {
        None => false,
        Some(Value::Bool(value)) => *value,
        // M5: an `abstract` present but not a boolean never silently reads
        // as `false` -- that would misreport a real declaration as its own
        // opposite rather than refusing an unreadable one.
        Some(_) => return Err(ctx.malformed("abstract: not a boolean")),
    };
    let interface_features = if is_interface {
        Some(ctx.identity_keys(package, "featureOrder")?)
    } else {
        None
    };
    records.push(DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: declaration_key(package, node),
        interface_features,
        abstract_type,
        supertypes,
    }));
    let fields_at = format!("{at}.fields");
    for (position, field) in ctx.array_field("fields")?.iter().enumerate() {
        records.push(DomainPackageRecord::FieldMember(read_field_member(
            package,
            node,
            field,
            &format!("{fields_at}[{position}]"),
        )?));
    }
    let operations_at = format!("{at}.operations");
    for (position, operation) in ctx.array_field("operations")?.iter().enumerate() {
        records.push(DomainPackageRecord::OperationMember(read_operation_member(
            package,
            node,
            operation,
            &format!("{operations_at}[{position}]"),
        )?));
    }
    Ok(())
}

fn read_component(
    package: &str,
    ctx: &NodeCtx<'_>,
    node: &str,
) -> Result<ComponentRecord, ModelRefusal> {
    let owner = ctx.str_field("owner")?;
    let value_type = ctx.str_field("declaredType")?;
    let multiplicity = ctx.multiplicity("multiplicity")?;
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
    ctx: &NodeCtx<'_>,
    node: &str,
) -> Result<EndpointRecord, ModelRefusal> {
    let owner = ctx.str_field("owner")?;
    let value_type = ctx.str_field("interfaceType")?;
    let multiplicity = ctx.multiplicity("multiplicity")?;
    let direction = match ctx.opt_str_field("direction") {
        Some("in") => Some(PortDirection::In),
        Some("out") => Some(PortDirection::Out),
        Some("inout") => Some(PortDirection::InOut),
        Some(_) => return Err(ctx.malformed("direction: not in/out/inout")),
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
    ctx: &NodeCtx<'_>,
    node: &str,
) -> Result<RelationshipRecord, ModelRefusal> {
    let source_end = ctx
        .value
        .get("sourceEnd")
        .ok_or_else(|| ctx.malformed("sourceEnd: missing"))?;
    let target_end = ctx
        .value
        .get("targetEnd")
        .ok_or_else(|| ctx.malformed("targetEnd: missing"))?;
    let source_type = source_end
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| ctx.malformed("sourceEnd.type: missing or not a string"))?;
    let target_type = target_end
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| ctx.malformed("targetEnd.type: missing or not a string"))?;
    let source_multiplicity = ctx.multiplicity_of(source_end, "sourceEnd.multiplicity")?;
    let target_multiplicity = ctx.multiplicity_of(target_end, "targetEnd.multiplicity")?;
    let direction = match ctx.str_field("flowDirection")? {
        "source-to-target" => RelationshipDirection::SourceToTarget,
        "target-to-source" => RelationshipDirection::TargetToSource,
        "bidirectional" => RelationshipDirection::Bidirectional,
        _ => return Err(ctx.malformed("flowDirection: not a known direction")),
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
        direction,
    })
}

fn read_allocation(
    package: &str,
    ctx: &NodeCtx<'_>,
    node: &str,
) -> Result<AllocationRecord, ModelRefusal> {
    // model-complete.md:335: an Allocation names a source element and a
    // target element, and nothing else -- no multiplicity, no direction.
    let source = ctx.str_field("sourceElement")?;
    let target = ctx.str_field("targetElement")?;
    Ok(AllocationRecord {
        key: declaration_key(package, node),
        source_element: declaration_key(package, source),
        target_element: declaration_key(package, target),
    })
}

fn read_population(
    package: &str,
    ctx: &NodeCtx<'_>,
    node: &str,
    meanings: &HashMap<(String, String), String>,
) -> Result<PopulationRecord, ModelRefusal> {
    let kind = ctx
        .value
        .get("kind")
        .ok_or_else(|| ctx.malformed("kind: missing"))?;
    let module = kind
        .get("module")
        .and_then(Value::as_str)
        .ok_or_else(|| ctx.malformed("kind.module: missing or not a string"))?;
    let name = kind
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| ctx.malformed("kind.name: missing or not a string"))?;
    match meanings.get(&(module.to_owned(), name.to_owned())) {
        Some(resolved) if resolved == meaning::POPULATION => {}
        Some(other) => {
            return Err(ctx.malformed(format!(
                "kind: resolves to {other:?}, not {:?}",
                meaning::POPULATION
            )))
        }
        None => return Err(ctx.malformed("kind: names no constructs[] entry")),
    }
    let member_types = ctx.identity_keys(package, "members")?;
    let extent = match ctx.str_field("extent")? {
        "closed" => Extent::Closed,
        "open" => Extent::Open,
        _ => return Err(ctx.malformed("extent: not closed or open")),
    };
    Ok(PopulationRecord {
        key: declaration_key(package, node),
        member_types,
        extent,
    })
}

/// The `{module, name} -> meaning` lookup a document's own `constructs[]`
/// table states (FR-208): the sole legitimate way a type's `kind` resolves
/// to what it IS.
fn meaning_index(document: &Value) -> Result<HashMap<(String, String), String>, ModelRefusal> {
    let ctx = NodeCtx::new(document, "$.constructs");
    let constructs = document
        .get("constructs")
        .and_then(Value::as_array)
        .ok_or_else(|| ctx.malformed("missing or not an array"))?;
    let mut index = HashMap::new();
    for (position, entry) in constructs.iter().enumerate() {
        let entry_ctx = NodeCtx::new(entry, format!("$.constructs[{position}]"));
        let kind = entry
            .get("kind")
            .ok_or_else(|| entry_ctx.malformed("kind: missing"))?;
        let module = kind
            .get("module")
            .and_then(Value::as_str)
            .ok_or_else(|| entry_ctx.malformed("kind.module: missing or not a string"))?;
        let name = kind
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| entry_ctx.malformed("kind.name: missing or not a string"))?;
        let meaning = entry
            .get("construct")
            .and_then(|construct| construct.get("meaning"))
            .and_then(Value::as_str)
            .ok_or_else(|| entry_ctx.malformed("construct.meaning: missing or not a string"))?;
        index.insert((module.to_owned(), name.to_owned()), meaning.to_owned());
    }
    Ok(index)
}

/// Reads one `types[]` entry, dispatching purely by the `meaning` its `kind`
/// resolves to in the document's own `constructs[]` table (FR-208) -- never
/// by `kind.name`/`kind.module` directly. Checks report in FR-154's own
/// order: the node's own identity form first (the object-id check), then
/// its `kind`/meaning, then the meaning-specific shape.
fn read_type_node(
    package: &str,
    meanings: &HashMap<(String, String), String>,
    type_value: &Value,
    at: &str,
) -> Result<Vec<DomainPackageRecord>, ModelRefusal> {
    let ctx = NodeCtx::new(type_value, at);
    let (_, key) = read_type_identity(package, &ctx)?;
    let node = key.node.as_str();
    let Some(kind) = type_value.get("kind").filter(|kind| kind.is_object()) else {
        // M3 (FCD #199 gap 1): a bare-string kind (`"scalar"`, `"alias"`,
        // `"record"`, ...) is a core kind FCD's own schema admits, but
        // QSpec's declaration-kinds table has no row for a package
        // declaring one of these directly in `types[]` -- every declared
        // type resolves via a `constructs[]` entry to a real FR-208
        // meaning instead. A reference TO such a node (e.g. a field's
        // `typeRef` naming a native scalar) is untouched here: this reader
        // never checks that reference, so a bare-string-kind node being
        // refused as a top-level declaration does not stop it resolving as
        // a reference target.
        return Err(ctx.malformed("kind: a bare-string core kind is not a declared FR-208 meaning"));
    };
    let module = kind
        .get("module")
        .and_then(Value::as_str)
        .ok_or_else(|| ctx.malformed("kind.module: missing or not a string"))?;
    let name = kind
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| ctx.malformed("kind.name: missing or not a string"))?;
    let Some(construct_meaning) = meanings.get(&(module.to_owned(), name.to_owned())) else {
        return Err(ctx.malformed("kind: names no constructs[] entry"));
    };
    let mut records = Vec::new();
    match construct_meaning.as_str() {
        meaning::OBJECT_TYPE => {
            read_object_type(package, type_value, node, at, false, &mut records)?
        }
        meaning::SYSTEMS_INTERFACE => {
            read_object_type(package, type_value, node, at, true, &mut records)?
        }
        meaning::SYSTEMS_PART => {
            records.push(DomainPackageRecord::Component(read_component(
                package, &ctx, node,
            )?));
        }
        meaning::SYSTEMS_PORT => {
            records.push(DomainPackageRecord::Endpoint(read_endpoint(
                package, &ctx, node,
            )?));
        }
        meaning::SYSTEMS_CONNECTION => {
            records.push(DomainPackageRecord::Relationship(read_connection(
                package, &ctx, node,
            )?));
        }
        meaning::SYSTEMS_ALLOCATION => {
            records.push(DomainPackageRecord::Allocation(read_allocation(
                package, &ctx, node,
            )?));
        }
        other if meaning::ALL.contains(&other) => {
            // H4: a real FR-208 meaning (e.g. RECORD_VALUE_TYPE) with no
            // QSL record shape yet -- refused as a known-but-unsupported
            // declaration form, never silently folded into `ObjectTypeRecord`.
            return Err(unsupported_at(type_value, at, other));
        }
        other => {
            return Err(ctx.malformed(format!("kind: resolves to {other:?}, outside FR-208")));
        }
    }
    Ok(records)
}

/// One document node under the combined `types[]`/`populations[]` ascending
/// sort ("ascending by declaration key", `model-complete.md`:74). Carries
/// its own array position (rather than re-deriving it by searching `types`/
/// `populations` back for a pointer match) so the read loop below reports
/// against the same `$.types[N]`/`$.populations[N]` path this node was
/// built from.
enum DocumentNode<'a> {
    Type(&'a Value, usize, String),
    Population(&'a Value, usize, String),
}

/// Reads an admitted Semantic IR 2.0.0 document's `types[]` and
/// `populations[]` into [`DomainPackageRecord`]s under `package_identity`.
/// Reads every node -- ascending by its own declaration key -- and collects
/// every refusal rather than stopping at the first (FR-154,
/// `model-complete.md`:74-83): `Ok` only when every node read cleanly,
/// `Err` with every node's refusal, in that same ascending order, otherwise.
pub fn read_records(
    package_identity: &str,
    document: &[u8],
) -> Result<Vec<DomainPackageRecord>, Vec<ModelRefusal>> {
    let parsed: Value = match serde_json::from_slice(document) {
        Ok(parsed) => parsed,
        Err(err) => {
            return Err(vec![ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::IntakeMalformedDeclaration {
                    node: "$".to_owned(),
                    artifact: None,
                    span: None,
                },
                detail: format!("admitted document bytes do not parse as JSON: {err}"),
            }]);
        }
    };
    let meanings = match meaning_index(&parsed) {
        Ok(meanings) => meanings,
        Err(refusal) => return Err(vec![refusal]),
    };
    let types = match parsed.get("types").and_then(Value::as_array) {
        Some(types) => types,
        None => {
            return Err(vec![malformed_at(
                &parsed,
                "$.types",
                "missing or not an array",
            )])
        }
    };
    let populations: &[Value] = match parsed.get("populations") {
        None => &[],
        Some(populations) => match populations.as_array() {
            Some(populations) => populations,
            None => return Err(vec![malformed_at(&parsed, "$.populations", "not an array")]),
        },
    };

    let mut nodes: Vec<DocumentNode<'_>> = Vec::with_capacity(types.len() + populations.len());
    for (position, type_value) in types.iter().enumerate() {
        let at = format!("$.types[{position}]");
        let label = node_identity_label(type_value, &at);
        nodes.push(DocumentNode::Type(type_value, position, label));
    }
    for (position, population) in populations.iter().enumerate() {
        let at = format!("$.populations[{position}]");
        let label = node_identity_label(population, &at);
        nodes.push(DocumentNode::Population(population, position, label));
    }
    nodes.sort_by(|a, b| {
        let label = |node: &DocumentNode<'_>| match node {
            DocumentNode::Type(_, _, label) | DocumentNode::Population(_, _, label) => {
                label.clone()
            }
        };
        label(a).cmp(&label(b))
    });

    let mut records = Vec::new();
    let mut refusals = Vec::new();
    for node in nodes {
        match node {
            DocumentNode::Type(type_value, position, _) => {
                let at = format!("$.types[{position}]");
                match read_type_node(package_identity, &meanings, type_value, &at) {
                    Ok(mut new_records) => records.append(&mut new_records),
                    Err(refusal) => refusals.push(refusal),
                }
            }
            DocumentNode::Population(population, position, _) => {
                let at = format!("$.populations[{position}]");
                let ctx = NodeCtx::new(population, at.clone());
                let result = read_type_identity(package_identity, &ctx).and_then(|(_, key)| {
                    read_population(package_identity, &ctx, &key.node, &meanings)
                });
                match result {
                    Ok(record) => records.push(DomainPackageRecord::Population(record)),
                    Err(refusal) => refusals.push(refusal),
                }
            }
        }
    }
    if refusals.is_empty() {
        Ok(records)
    } else {
        Err(refusals)
    }
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

    fn selection(
        identity: &str,
        version: &str,
        digest_domain: &str,
        digest: [u8; 32],
    ) -> ModelSelection {
        ModelSelection {
            identity: identity.to_owned(),
            version: version.to_owned(),
            digest_domain: digest_domain.to_owned(),
            digest,
        }
    }

    #[test]
    fn admits_matching_selection() {
        let bytes = package_bytes("acme/orders", "1");
        let digest = digest_of(&bytes);
        let mut map = BTreeMap::new();
        map.insert(digest, bytes.clone());
        let (package_ref, admitted_bytes) = admit(
            &selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest),
            &map,
        )
        .unwrap();
        assert_eq!(package_ref.identity, "acme/orders");
        assert_eq!(package_ref.version, "1");
        assert_eq!(package_ref.digest, digest);
        assert_eq!(admitted_bytes, bytes.as_slice());
    }

    #[test]
    fn admits_a_whitespace_variant_under_the_same_jcs_digest() {
        // M1: the digest is taken over JCS bytes, not the raw bytes
        // verbatim, so a byte-different-but-JCS-equivalent re-encoding of
        // the same document (extra whitespace here) still admits under the
        // digest recorded for the compact encoding.
        let compact = package_bytes("acme/orders", "1");
        let digest = digest_of(&jcs_bytes(&serde_json::from_slice(&compact).unwrap()));
        let padded = b"  \n\t"
            .iter()
            .chain(compact.iter())
            .copied()
            .collect::<Vec<u8>>();
        assert_ne!(padded, compact, "the two encodings are byte-different");
        let mut map = BTreeMap::new();
        map.insert(digest, padded.clone());
        let (package_ref, admitted_bytes) = admit(
            &selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest),
            &map,
        )
        .unwrap();
        assert_eq!(package_ref.digest, digest);
        assert_eq!(admitted_bytes, padded.as_slice());
    }

    #[test]
    fn refuses_foreign_digest_domain() {
        let map = BTreeMap::new();
        let refusal = admit(&selection("acme/orders", "1", "sha1", [0; 32]), &map).unwrap_err();
        assert_eq!(refusal.code, Code::StaleDependency);
        assert_eq!(refusal.cause.as_str(), "digest-domain-mismatch");
    }

    #[test]
    fn refuses_missing_bytes() {
        let map = BTreeMap::new();
        let refusal = admit(
            &selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, [0; 32]),
            &map,
        )
        .unwrap_err();
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
            &selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, wrong_digest),
            &map,
        )
        .unwrap_err();
        assert_eq!(refusal.code, Code::StaleDependency);
        assert_eq!(refusal.cause.as_str(), "byte-digest-mismatch");
    }

    #[test]
    fn refuses_wrong_package_identity() {
        let bytes = package_bytes("acme/other", "1");
        let digest = digest_of(&jcs_bytes(&serde_json::from_slice(&bytes).unwrap()));
        let mut map = BTreeMap::new();
        map.insert(digest, bytes);
        let refusal = admit(
            &selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest),
            &map,
        )
        .unwrap_err();
        assert_eq!(refusal.code, Code::InvalidModelBinding);
        assert_eq!(refusal.cause.as_str(), "wrong-model-selection");
    }

    #[test]
    fn refuses_non_json_bytes_as_wrong_selection() {
        let bytes = b"not json".to_vec();
        let digest = digest_of(&bytes);
        let mut map = BTreeMap::new();
        map.insert(digest, bytes);
        let refusal = admit(
            &selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest),
            &map,
        )
        .unwrap_err();
        assert_eq!(refusal.cause.as_str(), "wrong-model-selection");
    }

    #[test]
    fn refuses_a_document_that_is_not_json() {
        let refusals = read_records("acme/orders", b"not json").unwrap_err();
        assert_eq!(refusals.len(), 1);
        assert_eq!(refusals[0].code, Code::InvalidModelBinding);
        assert_eq!(refusals[0].cause.as_str(), "malformed-declaration");
    }

    #[test]
    fn refuses_a_bare_string_kind_as_malformed_declaration() {
        let document = serde_json::json!({
            "constructs": [],
            "types": [
                {"identity": "ix://acme/orders/Count", "kind": "scalar"},
            ],
        })
        .to_string();
        let refusals = read_records("acme/orders", document.as_bytes()).unwrap_err();
        assert_eq!(refusals.len(), 1);
        assert_eq!(refusals[0].cause.as_str(), "malformed-declaration");
        assert!(refusals[0].detail.contains("bare-string core kind"));
    }

    #[test]
    fn refuses_fcd_own_identity_form_as_malformed_declaration() {
        let document = serde_json::json!({
            "constructs": [
                {
                    "kind": {"module": "acme/orders", "name": "order"},
                    "moduleVersion": "1.0.0",
                    "manifestDigest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
                    "construct": {"meaning": meaning::OBJECT_TYPE},
                },
            ],
            "types": [
                {
                    "identity": "ix://acme/orders/type/Order",
                    "kind": {"module": "acme/orders", "name": "order"},
                    "supertypes": [],
                    "fields": [],
                    "operations": [],
                },
            ],
        })
        .to_string();
        let refusals = read_records("acme/orders", document.as_bytes()).unwrap_err();
        assert_eq!(refusals.len(), 1);
        assert_eq!(refusals[0].cause.as_str(), "malformed-declaration");
        assert!(refusals[0].detail.contains("identity"));
    }
}
