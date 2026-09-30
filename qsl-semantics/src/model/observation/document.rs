// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-106 checks 1, 5 to 10: reading one snapshot or invocation document
//! (check 1), admitting its populations and objects (checks 5 to 8) and
//! resolving `self`, `parameters` and `result` (checks 9 and 10).

use std::collections::{BTreeMap, BTreeSet};

use quire_exact::{CollectionKind, FieldValue, Integer, ObjectReference, Value};

use qsl_foundation::diagnostic::Code;

use super::helpers::{admission_record, object_reference};
use super::ordered_json::{OrderedJson, OrderedObject};
use super::{
    check_document_digest, fault, incomplete, population_universe, read_raw_value, refuse,
    AdmissionFailure, AdmissionRecord, DocumentRef, ModelView, ObservationLimits, SelectedObject,
    SnapshotValue,
};
use crate::model::key::DeclarationKey;
use crate::model::object_environment::{ObjectEnvironment, ObjectEnvironmentCause};
use crate::value::declaration::{OperationDeclaration, TypeEnvironment};

/// Which wire format a document is read as (FR-106 "Document forms").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DocumentKind {
    Snapshot,
    Invocation,
}

#[derive(Clone, Debug)]
pub(super) struct ModelHeader {
    pub(super) identity: String,
    pub(super) version: String,
    pub(super) digest: String,
}

#[derive(Clone, Debug)]
pub(super) struct DocAnchor {
    pub(super) kind: super::AnchorKind,
    pub(super) name: String,
}

/// An object's own `"type"` member: a raw producer identity string, not
/// yet resolved against a re-derived view (`find_declaration` does that
/// resolution, matching a [`DeclarationKey`](crate::model::key::DeclarationKey)'s
/// `node` half only -- this document reader never learns the matching
/// `package` half, only the domain package's own intake does, TC-260's
/// own typestate rule (FR-088-AC-8): no "type"-named field under `model`
/// is a bare `String`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RawTypeIdentity(String);

impl RawTypeIdentity {
    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub(super) struct RawObject {
    pub(super) key: String,
    pub(super) type_identity: RawTypeIdentity,
    /// The object's own `fields` member, in document order (FR-106 check
    /// 6.4's "unknown-member" detection needs "the first ... in walk
    /// order").
    pub(super) fields: Vec<(String, SnapshotValue)>,
}

#[derive(Clone, Debug)]
pub(super) struct RawPopulation {
    pub(super) population: String,
    pub(super) complete: bool,
    pub(super) objects: Vec<RawObject>,
}

/// The snapshot's own `observation` member (FR-106 "Document forms"): a
/// wire-reading edge (ADR-012 §9, mirroring [`super::AnchorKind`]'s own
/// pattern) converts it once, at read (`read_snapshot_body`), so no caller
/// compares `SnapshotDocument::observation` against a string literal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ObservationRole {
    /// `current`.
    Current,
    /// `pre`.
    Pre,
    /// `post`.
    Post,
}

#[derive(Clone, Debug)]
pub(super) struct SnapshotDocument {
    pub(super) observation: ObservationRole,
    pub(super) anchor: Option<DocAnchor>,
    pub(super) model: ModelHeader,
    pub(super) populations: Vec<RawPopulation>,
}

#[derive(Clone, Debug)]
pub(super) enum ResultValue {
    Null,
    Value(SnapshotValue),
}

#[derive(Clone, Debug)]
pub(super) struct InvocationDocument {
    pub(super) model: ModelHeader,
    pub(super) context: String,
    pub(super) operation: String,
    pub(super) self_object: SelectedObject,
    pub(super) pre: DocumentRef,
    pub(super) post: DocumentRef,
    /// The invocation's own `parameters` member, in document order.
    pub(super) parameters: Vec<(String, SnapshotValue)>,
    pub(super) result: ResultValue,
    pub(super) created: Vec<SelectedObject>,
    pub(super) deleted: Vec<SelectedObject>,
}

enum Body {
    Snapshot(SnapshotDocument),
    Invocation(Box<InvocationDocument>),
}

pub(super) struct ReadDocument {
    pub(super) identity: DocumentRef,
    /// FR-109 Outputs' admission usage: this document's own byte length
    /// and nesting depth (SR-751 FND-002 round 2).
    pub(super) usage: super::AdmissionUsage,
    body: Body,
}

impl ReadDocument {
    pub(super) fn as_snapshot(&self) -> Option<&SnapshotDocument> {
        match &self.body {
            Body::Snapshot(snapshot) => Some(snapshot),
            Body::Invocation(_) => None,
        }
    }

    pub(super) fn as_invocation(&self) -> Option<&InvocationDocument> {
        match &self.body {
            Body::Invocation(invocation) => Some(invocation),
            Body::Snapshot(_) => None,
        }
    }
}

fn blank(text: &str) -> bool {
    text.is_empty() || text.chars().all(char::is_whitespace)
}

/// Check 1.7: the first blank label of `identity`, in the order `authority`,
/// `identity`, `revision_namespace`, `revision`.
fn first_blank_label(identity: &DocumentRef) -> Option<&'static str> {
    if blank(&identity.authority) {
        return Some("authority");
    }
    if blank(&identity.identity) {
        return Some("identity");
    }
    if blank(&identity.revision_namespace) {
        return Some("revision_namespace");
    }
    if blank(&identity.revision) {
        return Some("revision");
    }
    None
}

fn labels_match(document: &DocumentRef, selection: &DocumentRef) -> bool {
    document.authority == selection.authority
        && document.identity == selection.identity
        && document.revision_namespace == selection.revision_namespace
        && document.revision == selection.revision
}

/// FR-106 check 1: read one document from `provision` under `selected`'s
/// digest, running the eight ordered conditions.
pub(super) fn read_document(
    kind: DocumentKind,
    provision: &BTreeMap<[u8; 32], Vec<u8>>,
    selected: &DocumentRef,
    limits: ObservationLimits,
) -> Result<ReadDocument, AdmissionFailure> {
    // 1.1
    let bytes = provision.get(&selected.digest).ok_or_else(|| {
        incomplete(admission_record(
            Code::UnavailableObservation,
            "missing-required-artifact",
        ))
    })?;
    // 1.2: input bytes.
    if bytes.len() as u64 > limits.document_bytes {
        return Err(refuse(admission_record(
            Code::StageLimitExceeded,
            "input-bytes-exceeded",
        )));
    }
    let document_bytes = bytes.len() as u64;
    let parsed: Option<OrderedJson> = serde_json::from_slice(bytes).ok();
    // 1.2: nesting depth (only meaningful once parsed).
    let nesting_depth = parsed.as_ref().map_or(0, OrderedJson::depth);
    if nesting_depth > limits.nesting_depth {
        return Err(refuse(admission_record(
            Code::StageLimitExceeded,
            "nesting-depth-exceeded",
        )));
    }
    // 1.3: digest, before any member is read. Bytes that do not parse are
    // digested raw, and so refuse here.
    check_document_digest(bytes, selected.digest)?;
    let digest = selected.digest;
    // Bytes that matched the expected digest raw (never parsed as JSON at
    // all) or that parsed but are not a JSON object trivially hold no
    // `format` member at all: absent is not the expected format string, so
    // check 1.4 (which comes before 1.5's member-presence check, FR-106's
    // own order) refuses it here, never 1.5's `missing-member` (SR-750
    // FND-015 round 2) and never an `AdmissionFailure::Fault` (SR-750
    // FND-004): FR-106 settles every input defect at admission, and this
    // input is untrusted, not an internal invariant.
    let unsupported_wire = || refuse(admission_record(Code::UnknownWire, "unsupported-wire"));
    let Some(value) = parsed else {
        return Err(unsupported_wire());
    };
    let object = value.as_object().ok_or_else(unsupported_wire)?;

    // 1.4: format.
    let expected_format = match kind {
        DocumentKind::Snapshot => "quire.state.snapshot/v1",
        DocumentKind::Invocation => "quire.state.invocation/v1",
    };
    if object.member("format").and_then(|value| value.as_str()) != Some(expected_format) {
        return Err(refuse(admission_record(
            Code::UnknownWire,
            "unsupported-wire",
        )));
    }

    // 1.5/1.6: member presence, in the member order FR-106 states.
    let always_required: &[&str] = match kind {
        DocumentKind::Snapshot => &["format", "identity", "observation", "model", "populations"],
        DocumentKind::Invocation => &[
            "format",
            "identity",
            "model",
            "context",
            "operation",
            "self",
            "pre",
            "post",
            "parameters",
            "result",
            "created",
            "deleted",
        ],
    };
    let allowed: &[&str] = match kind {
        DocumentKind::Snapshot => &[
            "format",
            "identity",
            "observation",
            "anchor",
            "model",
            "populations",
        ],
        DocumentKind::Invocation => always_required,
    };
    for member in always_required {
        if !object.contains_key(member) {
            return Err(refuse(
                admission_record(Code::InvalidRuntimeInput, "missing-member")
                    .with("field", *member),
            ));
        }
    }
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(refuse(
                admission_record(Code::InvalidRuntimeInput, "unknown-member")
                    .with("field", key.clone()),
            ));
        }
    }

    // 1.7/1.8: the document's own four labels.
    let identity_member = object
        .member("identity")
        .and_then(|value| value.as_object())
        .ok_or_else(|| {
            refuse(
                admission_record(Code::InvalidRuntimeInput, "wrong-value-kind")
                    .with("field", "identity"),
            )
        })?;
    let label = |name: &'static str| {
        identity_member
            .member(name)
            .and_then(|value| value.as_str())
            .map(str::to_owned)
            .ok_or_else(|| {
                refuse(
                    admission_record(Code::InvalidRuntimeInput, "missing-member")
                        .with("field", name),
                )
            })
    };
    let document_identity = DocumentRef {
        authority: label("authority")?,
        identity: label("identity")?,
        revision_namespace: label("revision_namespace")?,
        revision: label("revision")?,
        digest,
    };
    if let Some(blank_label) = first_blank_label(&document_identity) {
        return Err(refuse(
            admission_record(Code::InvalidSourceIdentity, "blank-label").with("label", blank_label),
        ));
    }
    if !labels_match(&document_identity, selected) {
        return Err(refuse(
            admission_record(Code::StaleDependency, "revision-mismatch")
                .with("required", format!("{selected:?}"))
                .with("supplied", format!("{document_identity:?}")),
        ));
    }

    let model = read_model(object)?;

    let body = match kind {
        DocumentKind::Snapshot => Body::Snapshot(read_snapshot_body(object, model)?),
        DocumentKind::Invocation => {
            Body::Invocation(Box::new(read_invocation_body(object, model)?))
        }
    };
    Ok(ReadDocument {
        identity: document_identity,
        usage: super::AdmissionUsage {
            document_bytes,
            nesting_depth,
            objects: 0,
            values: 0,
        },
        body,
    })
}

/// Reads the document's own `model` member into a [`ModelHeader`]: a
/// wire-reading edge (ADR-012 §9), converting `identity`/`version`/`digest`
/// once, here, into the typed header [`check_model`] compares against the
/// package's own model selection. A missing `identity`/`version`/`digest`
/// member refuses `invalid_runtime_input`/`missing-member` at that member
/// (check 1.5's own cause, generalized to a nested member); `digest` not
/// spelled with the exact `sha256-jcs:` prefix refuses
/// `invalid_runtime_input`/`wrong-value-kind` at `digest` -- `strip_prefix`,
/// not `trim_start_matches`, so a wrong prefix refuses rather than silently
/// keeping the whole unstripped string. Before this fix, a missing or
/// malformed member silently became `""` (`unwrap_or_default`), which
/// [`check_model`] would then merely fail to match against any package's
/// model selection, misreporting `wrong-model-selection` for what is
/// actually a missing- or malformed-member document defect.
#[qsl_attrs::string_edge]
fn read_model(object: &[(String, OrderedJson)]) -> Result<ModelHeader, AdmissionFailure> {
    let model = object
        .member("model")
        .ok_or_else(|| missing_member("model"))?
        .as_object()
        .ok_or_else(|| wrong_kind("model"))?;
    let missing_member = |field: &'static str| {
        refuse(admission_record(Code::InvalidRuntimeInput, "missing-member").with("field", field))
    };
    let identity = model
        .member("identity")
        .and_then(|value| value.as_str())
        .ok_or_else(|| missing_member("identity"))?
        .to_owned();
    let version = model
        .member("version")
        .and_then(|value| value.as_str())
        .ok_or_else(|| missing_member("version"))?
        .to_owned();
    let digest_raw = model
        .member("digest")
        .and_then(|value| value.as_str())
        .ok_or_else(|| missing_member("digest"))?;
    let digest = digest_raw
        .strip_prefix("sha256-jcs:")
        .ok_or_else(|| {
            refuse(
                admission_record(Code::InvalidRuntimeInput, "wrong-value-kind")
                    .with("field", "digest"),
            )
        })?
        .to_owned();
    Ok(ModelHeader {
        identity,
        version,
        digest,
    })
}

fn wrong_kind(field: impl Into<String>) -> AdmissionFailure {
    refuse(admission_record(Code::InvalidRuntimeInput, "wrong-value-kind").with("field", field))
}

fn missing_member(field: impl Into<String>) -> AdmissionFailure {
    refuse(admission_record(Code::InvalidRuntimeInput, "missing-member").with("field", field))
}

/// Reads a `{population, key}` object reference at `field`: a document
/// defect (a missing or wrong-kind member) is `invalid_runtime_input`,
/// never `AdmissionFailure::Fault` (SR-750 FND-004).
fn read_object_ref(
    value: &OrderedJson,
    field: &'static str,
) -> Result<SelectedObject, AdmissionFailure> {
    let object = value.as_object().ok_or_else(|| wrong_kind(field))?;
    let population = object
        .member("population")
        .and_then(|value| value.as_str())
        .ok_or_else(|| missing_member(field))?
        .to_owned();
    let key = object
        .member("key")
        .and_then(|value| value.as_str())
        .ok_or_else(|| missing_member(field))?
        .to_owned();
    Ok(SelectedObject { population, key })
}

/// Reads a `{identity: {...}, digest: "sha256-jcs:<hex>"}` document
/// reference at `field`: a wire-reading edge (ADR-012 §9), same as
/// [`read_model`]. `strip_prefix`, not `trim_start_matches` (SR-750
/// FND-003): a digest with no `sha256-jcs:` prefix, or a doubled one,
/// refuses rather than being silently accepted or kept whole.
#[qsl_attrs::string_edge]
fn read_document_ref(
    value: &OrderedJson,
    field: &'static str,
) -> Result<DocumentRef, AdmissionFailure> {
    let object = value.as_object().ok_or_else(|| wrong_kind(field))?;
    let identity = object
        .member("identity")
        .and_then(|value| value.as_object())
        .ok_or_else(|| missing_member(field))?;
    let label = |name: &'static str| {
        identity
            .member(name)
            .and_then(|value| value.as_str())
            .map(str::to_owned)
            .ok_or_else(|| missing_member(field))
    };
    let digest_raw = object
        .member("digest")
        .and_then(|value| value.as_str())
        .ok_or_else(|| missing_member(field))?;
    let digest_hex = digest_raw
        .strip_prefix("sha256-jcs:")
        .ok_or_else(|| wrong_kind(field))?;
    let bytes = hex_bytes(digest_hex).ok_or_else(|| wrong_kind(field))?;
    if bytes.len() != 32 {
        return Err(wrong_kind(field));
    }
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&bytes);
    Ok(DocumentRef {
        authority: label("authority")?,
        identity: label("identity")?,
        revision_namespace: label("revision_namespace")?,
        revision: label("revision")?,
        digest,
    })
}

/// Decodes `text` as ASCII hex, or `None` for anything else -- including
/// non-ASCII text, where byte-index slicing would otherwise panic on a
/// char boundary (an untrusted digest such as `"sha256-jcs:aé…"`, reached
/// after the invocation's own digest check passes). Checked one byte at a
/// time over `text.as_bytes()`, never `&text[at..at + 2]` (which slices by
/// byte offset but assumes every two offsets fall on a char boundary).
fn hex_bytes(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(2) || !bytes.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16)?;
            let low = (pair[1] as char).to_digit(16)?;
            Some((high * 16 + low) as u8)
        })
        .collect()
}

/// Reads the snapshot's own body (`observation`, `anchor`, `populations`),
/// converting `observation` to [`ObservationRole`] once, here (a
/// wire-reading edge, ADR-012 §9): an unrecognized value refuses
/// `wrong_snapshot`/`wrong-observation`, the same cause every caller of
/// this document already reports for the "wrong role for this call" case,
/// since an unrecognized spelling is exactly that, one level earlier.
#[qsl_attrs::string_edge]
fn read_snapshot_body(
    object: &[(String, OrderedJson)],
    model: ModelHeader,
) -> Result<SnapshotDocument, AdmissionFailure> {
    let observation = match object
        .member("observation")
        .and_then(|value| value.as_str())
    {
        Some("current") => ObservationRole::Current,
        Some("pre") => ObservationRole::Pre,
        Some("post") => ObservationRole::Post,
        _ => {
            return Err(refuse(AdmissionRecord::new(
                Code::WrongSnapshot,
                "wrong-observation",
            )))
        }
    };
    let anchor = match object.member("anchor") {
        Some(value) => {
            let object = value.as_object().ok_or_else(|| wrong_kind("anchor"))?;
            let kind = match object.member("kind").and_then(|value| value.as_str()) {
                Some("initialization") => super::AnchorKind::Initialization,
                Some("handler") => super::AnchorKind::Handler,
                _ => return Err(wrong_kind("anchor")),
            };
            let name = object
                .member("name")
                .and_then(|value| value.as_str())
                .ok_or_else(|| missing_member("anchor"))?
                .to_owned();
            Some(DocAnchor { kind, name })
        }
        None => None,
    };
    let populations = read_populations(object)?;
    Ok(SnapshotDocument {
        observation,
        anchor,
        model,
        populations,
    })
}

fn read_populations(
    object: &[(String, OrderedJson)],
) -> Result<Vec<RawPopulation>, AdmissionFailure> {
    let items = object
        .member("populations")
        .and_then(|value| value.as_array())
        .ok_or_else(|| wrong_kind("populations"))?;
    let mut populations = Vec::with_capacity(items.len());
    for item in items {
        let entry = item.as_object().ok_or_else(|| wrong_kind("populations"))?;
        let population = entry
            .member("population")
            .and_then(|value| value.as_str())
            .ok_or_else(|| missing_member("population"))?
            .to_owned();
        let complete = entry
            .member("complete")
            .and_then(|value| value.as_bool())
            .ok_or_else(|| missing_member("complete"))?;
        let object_items = entry
            .member("objects")
            .and_then(|value| value.as_array())
            .ok_or_else(|| wrong_kind("objects"))?;
        let mut objects = Vec::with_capacity(object_items.len());
        for object_item in object_items {
            let object_entry = object_item
                .as_object()
                .ok_or_else(|| wrong_kind("objects"))?;
            let key = object_entry
                .member("key")
                .and_then(|value| value.as_str())
                .ok_or_else(|| missing_member("key"))?
                .to_owned();
            let type_identity = RawTypeIdentity(
                object_entry
                    .member("type")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| missing_member("type"))?
                    .to_owned(),
            );
            let field_items = object_entry
                .member("fields")
                .and_then(|value| value.as_object())
                .ok_or_else(|| wrong_kind("fields"))?;
            let mut fields = Vec::with_capacity(field_items.len());
            for (name, value) in field_items {
                let raw = read_raw_value(&value.clone().into_value())
                    .ok_or_else(|| wrong_kind(name.clone()))?;
                fields.push((name.clone(), raw));
            }
            objects.push(RawObject {
                key,
                type_identity,
                fields,
            });
        }
        populations.push(RawPopulation {
            population,
            complete,
            objects,
        });
    }
    Ok(populations)
}

fn read_invocation_body(
    object: &[(String, OrderedJson)],
    model: ModelHeader,
) -> Result<InvocationDocument, AdmissionFailure> {
    let context = object
        .member("context")
        .and_then(|value| value.as_str())
        .ok_or_else(|| missing_member("context"))?
        .to_owned();
    let operation = object
        .member("operation")
        .and_then(|value| value.as_str())
        .ok_or_else(|| missing_member("operation"))?
        .to_owned();
    let self_object = object
        .member("self")
        .ok_or_else(|| missing_member("self"))
        .and_then(|value| read_object_ref(value, "self"))?;
    let pre = object
        .member("pre")
        .ok_or_else(|| missing_member("pre"))
        .and_then(|value| read_document_ref(value, "pre"))?;
    let post = object
        .member("post")
        .ok_or_else(|| missing_member("post"))
        .and_then(|value| read_document_ref(value, "post"))?;
    let parameter_items = object
        .member("parameters")
        .and_then(|value| value.as_object())
        .ok_or_else(|| wrong_kind("parameters"))?;
    let mut parameters = Vec::with_capacity(parameter_items.len());
    for (name, value) in parameter_items {
        let raw =
            read_raw_value(&value.clone().into_value()).ok_or_else(|| wrong_kind(name.clone()))?;
        parameters.push((name.clone(), raw));
    }
    let result = match object.member("result") {
        Some(OrderedJson::Null) => ResultValue::Null,
        Some(value) => ResultValue::Value(
            read_raw_value(&value.clone().into_value()).ok_or_else(|| wrong_kind("result"))?,
        ),
        None => return Err(missing_member("result")),
    };
    let created = object
        .member("created")
        .and_then(|value| value.as_array())
        .ok_or_else(|| wrong_kind("created"))?
        .iter()
        .map(|item| read_object_ref(item, "created"))
        .collect::<Result<Vec<_>, _>>()?;
    let deleted = object
        .member("deleted")
        .and_then(|value| value.as_array())
        .ok_or_else(|| wrong_kind("deleted"))?
        .iter()
        .map(|item| read_object_ref(item, "deleted"))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(InvocationDocument {
        model,
        context,
        operation,
        self_object,
        pre,
        post,
        parameters,
        result,
        created,
        deleted,
    })
}

// ---------------------------------------------------------------------------
// Model-view lookups
// ---------------------------------------------------------------------------

/// The re-derived view whose domain package declares an object type or
/// population named `identity` (a raw producer identity string), with that
/// declaration's key.
pub(super) fn find_declaration<'v>(
    views: &'v [ModelView],
    identity: &str,
) -> Option<(&'v ModelView, DeclarationKey)> {
    views.iter().find_map(|view| {
        view.view
            .type_identities()
            .keys()
            .find(|key| key.node == identity)
            .cloned()
            .map(|key| (view, key))
    })
}

struct PopulationInfo {
    member_types: Vec<DeclarationKey>,
}

fn find_population<'v>(
    views: &'v [ModelView],
    identity: &str,
) -> Option<(&'v ModelView, PopulationInfo)> {
    for view in views {
        for record in &view.view.domain_package().records {
            if let crate::model::domain_package::DomainPackageRecord::Population(population) =
                record
            {
                if population.key.node == identity {
                    return Some((
                        view,
                        PopulationInfo {
                            member_types: population.member_types.clone(),
                        },
                    ));
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Checks 6 to 8: populations and values, completeness, closure
// ---------------------------------------------------------------------------

pub(super) struct AdmittedEnvironment {
    pub(super) environment: ObjectEnvironment,
    pub(super) completeness: BTreeMap<String, bool>,
    /// FR-109 Outputs' admission usage: this document's own admitted
    /// object and field-value counts (SR-751 FND-002 round 2); the
    /// caller merges in `ReadDocument::usage`'s byte length and nesting
    /// depth, which this check-6-to-8 pass never sees.
    pub(super) usage: super::AdmissionUsage,
}

/// A model object-type field's raw kind test, keyed off `value_type` alone
/// (used for operation parameters and results, whose `(String, ValueType)`
/// signature carries no separate presence: an optional operation parameter
/// or result is itself typed `ValueType::Option(_)`).
/// The first value named `name` in `fields` (document order, first
/// occurrence for a duplicate key), or `None`.
pub(super) fn raw_field<'a>(
    fields: &'a [(String, SnapshotValue)],
    name: &str,
) -> Option<&'a SnapshotValue> {
    fields
        .iter()
        .find(|(field, _)| field == name)
        .map(|(_, value)| value)
}

fn field_kind_matches(raw: &SnapshotValue, value_type: &quire_exact::ValueType) -> bool {
    matches!(
        (raw, value_type),
        (SnapshotValue::Boolean(_), quire_exact::ValueType::Boolean)
            | (
                SnapshotValue::Integer(_),
                quire_exact::ValueType::Integer | quire_exact::ValueType::Int(_)
            )
            | (
                SnapshotValue::Reference(_),
                quire_exact::ValueType::Reference(_)
            )
            | (
                SnapshotValue::Absent | SnapshotValue::Present(_),
                quire_exact::ValueType::Option(_)
            )
            | (
                SnapshotValue::Sequence(_),
                quire_exact::ValueType::Collection(_)
            )
    )
}

/// An object type's own field's raw kind test: presence
/// (`Presence::Required`/`Presence::Optional`) is a declaration-level
/// property distinct from its `value_type()` (`FieldDeclaration` keeps them
/// as two fields, not one wrapping the other): an optional `Reference<T>`
/// field's `value_type()` is the bare `ValueType::Reference(_)`, not
/// `ValueType::Option(_)` -- `ValueType::Option` names a genuinely
/// `Option`-typed *value* (a function's own parameter or result type, see
/// [`field_kind_matches`]), never model field presence. FR-106's
/// `absent`/`present` object-field forms key off `presence`, not off
/// matching `value_type` against `ValueType::Option`.
fn object_field_kind_matches(
    raw: &SnapshotValue,
    value_type: &quire_exact::ValueType,
    presence: quire_exact::Presence,
) -> bool {
    if presence == quire_exact::Presence::Optional {
        return matches!(raw, SnapshotValue::Absent | SnapshotValue::Present(_));
    }
    matches!(
        (raw, value_type),
        (SnapshotValue::Boolean(_), quire_exact::ValueType::Boolean)
            | (
                SnapshotValue::Integer(_),
                quire_exact::ValueType::Integer | quire_exact::ValueType::Int(_)
            )
            | (
                SnapshotValue::Reference(_),
                quire_exact::ValueType::Reference(_)
            )
            | (
                SnapshotValue::Sequence(_),
                quire_exact::ValueType::Collection(_)
            )
    )
}

/// Resolves each `{population, key}` reference value against the objects
/// one snapshot's populations actually hold, and records the ones that name
/// no admitted object. Built once per snapshot, before check 6 walks it, so
/// a reference resolves whatever the walk order of its target.
pub(super) struct References<'a> {
    views: &'a [ModelView],
    /// Every object of the snapshot whose type and key resolve, by its wire
    /// `(population, key)`.
    admitted: BTreeMap<(&'a str, &'a str), ObjectReference>,
    /// Each listed population's `complete` flag, as check 7 reads it (the
    /// same last-entry-wins insert [`admit_population_values`] makes).
    completeness: BTreeMap<&'a str, bool>,
    /// Each reference that named no object of the snapshot, with the wire
    /// population it named, in walk order.
    unresolved: Vec<(String, ObjectReference)>,
}

impl<'a> References<'a> {
    /// Indexes `populations`' objects. An object whose type or key does not
    /// resolve is left out: check 6 refuses it in walk order before any
    /// reference to it is used.
    pub(super) fn over(views: &'a [ModelView], populations: &'a [RawPopulation]) -> Self {
        let mut admitted = BTreeMap::new();
        let mut completeness = BTreeMap::new();
        for population in populations {
            completeness.insert(population.population.as_str(), population.complete);
            for object in &population.objects {
                let Some((view, key)) = find_declaration(views, object.type_identity.as_str())
                else {
                    continue;
                };
                let Some(effective) = view.view.type_identities().get(&key) else {
                    continue;
                };
                let Ok(reference) = object_reference(views, *effective, &object.key) else {
                    continue;
                };
                admitted
                    .entry((population.population.as_str(), object.key.as_str()))
                    .or_insert(reference);
            }
        }
        Self {
            views,
            admitted,
            completeness,
            unresolved: Vec::new(),
        }
    }

    /// FR-106 check 8 over one check-10 value (a parameter or the result):
    /// every reference inside `raw` must name an object of this snapshot,
    /// unless the snapshot lists its population as incomplete (check 7
    /// skips the dangling check there). A population the snapshot does not
    /// list is not tolerated, the same rule [`finish_populations`] applies
    /// to object fields (SR-750 FND-019). Object fields get this check from
    /// [`check_population_closure`]; this is the same rule for the values
    /// check 10 admits (SR-771 FND-001).
    fn check_closure(&self, raw: &SnapshotValue) -> Result<(), AdmissionRecord> {
        let mut refs = Vec::new();
        collect_references(raw, &mut refs);
        for (population, key) in refs {
            if self
                .admitted
                .contains_key(&(population.as_str(), key.as_str()))
                || self.completeness.get(population.as_str()).copied() == Some(false)
            {
                continue;
            }
            return Err(admission_record(
                Code::DanglingReference,
                "absent-target-in-complete-population",
            )
            .with("population", population)
            .with("object", key));
        }
        Ok(())
    }

    /// The typed reference `reference` names, for a value of declared type
    /// `Reference<declared>`. A reference to an admitted object resolves to
    /// that object, which the declared type must admit
    /// ([`quire_exact::ValueType::admits`]: the object's most-specific type
    /// is `declared`), else `wrong-value-kind` (check 6.5). A reference to no
    /// admitted object -- a dangling target, which check 8 refuses in a
    /// complete population and skips in an incomplete one -- is typed by
    /// `declared` and recorded.
    fn resolve(
        &mut self,
        reference: &super::SelectedObject,
        declared: quire_exact::EffectiveId,
    ) -> Result<ObjectReference, AdmissionRecord> {
        let wire = (reference.population.as_str(), reference.key.as_str());
        if let Some(target) = self.admitted.get(&wire) {
            let value = Value::Reference(target.clone());
            return if quire_exact::ValueType::Reference(declared).admits(&value) {
                Ok(target.clone())
            } else {
                Err(admission_record(
                    Code::InvalidRuntimeInput,
                    "wrong-value-kind",
                ))
            };
        }
        let target = object_reference(self.views, declared, &reference.key)
            .map_err(|_| admission_record(Code::InvalidRuntimeInput, "wrong-value-kind"))?;
        self.unresolved
            .push((reference.population.clone(), target.clone()));
        Ok(target)
    }
}

fn admit_scalar(
    references: &mut References<'_>,
    raw: &SnapshotValue,
    value_type: &quire_exact::ValueType,
) -> Result<Value, AdmissionRecord> {
    match (raw, value_type) {
        (SnapshotValue::Boolean(value), quire_exact::ValueType::Boolean) => {
            Ok(Value::Boolean(*value))
        }
        (SnapshotValue::Integer(spelling), quire_exact::ValueType::Integer) => spelling
            .parse::<Integer>()
            .map(Value::Integer)
            .map_err(|_| admission_record(Code::InvalidRuntimeInput, "invalid-value")),
        (SnapshotValue::Integer(spelling), quire_exact::ValueType::Int(interval)) => {
            let value = spelling
                .parse::<Integer>()
                .map_err(|_| admission_record(Code::InvalidRuntimeInput, "invalid-value"))?;
            if interval.contains(&value) {
                Ok(Value::Integer(value))
            } else {
                Err(admission_record(Code::InvalidRuntimeInput, "invalid-value"))
            }
        }
        (SnapshotValue::Reference(reference), quire_exact::ValueType::Reference(type_identity)) => {
            references
                .resolve(reference, *type_identity)
                .map(Value::Reference)
        }
        _ => Err(admission_record(
            Code::InvalidRuntimeInput,
            "wrong-value-kind",
        )),
    }
}

/// An object type's own field's admission: `presence`, not `value_type`,
/// decides whether `absent`/`present` is admitted (see
/// [`object_field_kind_matches`]'s doc). Parameters and results have no
/// `FieldValue` slot to fill (they admit straight to a [`Value`] via
/// [`admit_scalar`] plus [`field_kind_matches`]'s `ValueType::Option`
/// reading), so this is the one caller [`admit_populations`] needs.
fn admit_object_field(
    references: &mut References<'_>,
    raw: &SnapshotValue,
    value_type: &quire_exact::ValueType,
    presence: quire_exact::Presence,
) -> Result<FieldValue, AdmissionRecord> {
    if presence == quire_exact::Presence::Optional {
        return match raw {
            SnapshotValue::Absent => Ok(FieldValue::Absent),
            SnapshotValue::Present(inner) => {
                admit_scalar(references, inner, value_type).map(FieldValue::Present)
            }
            // `object_field_kind_matches` already refused every other raw
            // form for an optional field before this is called.
            SnapshotValue::Boolean(_)
            | SnapshotValue::Integer(_)
            | SnapshotValue::Reference(_)
            | SnapshotValue::Sequence(_) => Err(admission_record(
                Code::InvalidRuntimeInput,
                "wrong-value-kind",
            )),
        };
    }
    match (raw, value_type) {
        (SnapshotValue::Sequence(items), quire_exact::ValueType::Collection(collection)) => {
            if collection.kind() != CollectionKind::Sequence {
                return Err(admission_record(
                    Code::UnknownRequiredFeature,
                    "unsupported-feature",
                ));
            }
            let mut elements = Vec::with_capacity(items.len());
            for item in items {
                elements.push(admit_scalar(references, item, collection.element())?);
            }
            Ok(FieldValue::Present(quire_exact::from_admitted(
                (**collection).clone(),
                elements,
            )))
        }
        (raw, value_type) => admit_scalar(references, raw, value_type).map(FieldValue::Present),
    }
}

/// Every `(population, key)` a reference value inside `raw` names,
/// recursively (a reference can sit under `present` or inside a
/// `sequence`), appended to `into` in the value's own walk order.
fn collect_references(raw: &SnapshotValue, into: &mut Vec<(String, String)>) {
    match raw {
        SnapshotValue::Reference(reference) => {
            into.push((reference.population.clone(), reference.key.clone()))
        }
        SnapshotValue::Present(inner) => collect_references(inner, into),
        SnapshotValue::Sequence(items) => {
            for item in items {
                collect_references(item, into);
            }
        }
        SnapshotValue::Boolean(_) | SnapshotValue::Integer(_) | SnapshotValue::Absent => {}
    }
}

/// Check 6's own admitted output, carried between checks 6, 7 and 8: an
/// invocation's pre and post snapshots each get their own [`PopulationValues`]
/// from [`admit_population_values`], so a caller can run check 6 on both,
/// then check 7 on both, then check 8 on both -- FR-106's own numbered-check
/// order, across two documents, never fully finishing one document's checks
/// 6 to 8 before starting the other's check 6 (SR-750 FND-005).
pub(super) struct PopulationValues<'t> {
    objects: Vec<(ObjectReference, Vec<(&'t str, FieldValue)>)>,
    pub(super) completeness: BTreeMap<String, bool>,
    pub(super) keys_by_population: BTreeMap<String, BTreeSet<String>>,
    /// Each reference that named no object of this snapshot, with the wire
    /// population it named ([`References`]).
    unresolved: Vec<(String, ObjectReference)>,
    /// FR-109 Outputs' admission usage: this document's own admitted
    /// object and field-value counts (SR-751 FND-002 round 2).
    pub(super) objects_admitted: u64,
    pub(super) values_admitted: u64,
}

/// FR-106 check 6: admit every population's every object's every field, in
/// document order. `types` is the checked package's own effective attribute
/// set (`CheckedGraph::scope().types()`); `views` are the re-derived domain
/// package views (population declarations and identity strings, `EffectiveId`
/// conformance). `limits`' `objects_per_document`/`values_per_document`
/// ceilings (FR-106 Inputs, `FR-106-admit-snapshots-and-invocations.md:52-54`)
/// are enforced here, walk order, alongside check 1.2's own bytes/depth
/// ceilings: `stage_limit_exceeded`/`objects-exceeded` once this document's
/// object count would exceed `objects_per_document`, `values-exceeded` once
/// its admitted field-value count would exceed `values_per_document` (SR-750
/// FND-011: previously declared but never read).
pub(super) fn admit_population_values<'t>(
    views: &[ModelView],
    types: &'t TypeEnvironment,
    populations: &[RawPopulation],
    limits: ObservationLimits,
) -> Result<PopulationValues<'t>, AdmissionFailure> {
    let mut objects: Vec<(ObjectReference, Vec<(&str, FieldValue)>)> = Vec::new();
    let mut completeness = BTreeMap::new();
    let mut keys_by_population: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut object_count: u64 = 0;
    let mut value_count: u64 = 0;
    let mut references = References::over(views, populations);

    for entry in populations {
        completeness.insert(entry.population.clone(), entry.complete);
        let Some((population_view, population_info)) = find_population(views, &entry.population)
        else {
            return Err(refuse(
                admission_record(Code::InvalidRuntimeInput, "wrong-role-mapping")
                    .with("population", entry.population.clone()),
            ));
        };
        let seen = keys_by_population
            .entry(entry.population.clone())
            .or_default();
        for object in &entry.objects {
            object_count += 1;
            if object_count > limits.objects_per_document {
                return Err(refuse(admission_record(
                    Code::StageLimitExceeded,
                    "objects-exceeded",
                )));
            }
            // 6.1: type/population membership, before anything else in this
            // check (SR-750 FND-005 round 2's own numbered order).
            let Some((_, object_key)) = find_declaration(views, object.type_identity.as_str())
            else {
                return Err(refuse(
                    admission_record(Code::InvalidRuntimeInput, "wrong-role-mapping")
                        .with("object", object.key.clone()),
                ));
            };
            let covered = population_info.member_types.iter().any(|member| {
                population_view
                    .view
                    .model_index()
                    .conforms(&object_key, member, 64)
                    .unwrap_or(false)
            });
            if !covered {
                return Err(refuse(
                    admission_record(Code::InvalidRuntimeInput, "wrong-role-mapping")
                        .with("object", object.key.clone()),
                ));
            }
            let effective_type = *population_view
                .view
                .type_identities()
                .get(&object_key)
                .ok_or_else(|| fault("object-type-has-no-effective-id"))?;

            let declared = types.attributes(effective_type).ok_or_else(|| {
                AdmissionFailure::Fault(qsl_foundation::diagnostic::InternalFault::new(
                    "observation-admission",
                    "object-type-has-no-checked-attributes",
                ))
            })?;
            // 6.2: a set/bag/ordered-set field.
            for attribute in declared {
                if let quire_exact::ValueType::Collection(collection) =
                    attribute.field().value_type()
                {
                    if collection.kind() != CollectionKind::Sequence {
                        return Err(refuse(
                            admission_record(Code::UnknownRequiredFeature, "unsupported-feature")
                                .with("field", attribute.field().name().to_owned()),
                        ));
                    }
                }
            }
            // 6.3: duplicate key. `seen.insert` still runs first (its
            // result feeds `keys_by_population`, check 8's own input,
            // whichever way this object's own admission ends), but the
            // refusal is raised only after 6.1 and 6.2 have already passed
            // for this object.
            let is_duplicate = !seen.insert(object.key.clone());
            if is_duplicate {
                return Err(refuse(
                    admission_record(Code::InvalidRuntimeInput, "conflicting-identity")
                        .with("object", object.key.clone()),
                ));
            }
            // 6.4: a declared field missing (`missing-member`), then an
            // undeclared field present (`unknown-member`), in the order
            // FR-106 lists them -- both before any 6.5 value check.
            for attribute in declared {
                let name = attribute.field().name();
                if raw_field(&object.fields, name).is_none() {
                    return Err(refuse(
                        admission_record(Code::InvalidRuntimeInput, "missing-member")
                            .with("object", object.key.clone())
                            .with("field", name.to_owned()),
                    ));
                }
            }
            for (name, _) in &object.fields {
                if !declared
                    .iter()
                    .any(|attribute| attribute.field().name() == name)
                {
                    return Err(refuse(
                        admission_record(Code::InvalidRuntimeInput, "unknown-member")
                            .with("object", object.key.clone())
                            .with("field", name.clone()),
                    ));
                }
            }
            // 6.5: value checks, per declared field, in declared order.
            let mut attributes: Vec<(&str, FieldValue)> = Vec::with_capacity(declared.len());
            for attribute in declared {
                let name = attribute.field().name();
                let value_type = attribute.field().value_type();
                let presence = attribute.field().presence();
                // 6.4 above already confirmed every declared field has a
                // raw value here.
                let raw = raw_field(&object.fields, name)
                    .ok_or_else(|| fault("declared-field-missing-after-check-6-4"))?;
                if !object_field_kind_matches(raw, value_type, presence) {
                    return Err(refuse(
                        admission_record(Code::InvalidRuntimeInput, "wrong-value-kind")
                            .with("object", object.key.clone())
                            .with("field", name.to_owned()),
                    ));
                }
                let field_value = admit_object_field(&mut references, raw, value_type, presence)
                    .map_err(|record| {
                        refuse(
                            record
                                .with("object", object.key.clone())
                                .with("field", name.to_owned()),
                        )
                    })?;
                value_count += 1;
                if value_count > limits.values_per_document {
                    return Err(refuse(admission_record(
                        Code::StageLimitExceeded,
                        "values-exceeded",
                    )));
                }
                attributes.push((name, field_value));
            }

            // `object_reference` already refuses an empty key with FR-106's
            // own `invalid-value` (SR-750 FND-004 round 2): propagated
            // directly, never re-mapped to a `Fault`.
            let reference = object_reference(views, effective_type, &object.key)?;
            objects.push((reference, attributes));
        }
    }

    Ok(PopulationValues {
        objects,
        completeness,
        keys_by_population,
        unresolved: references.unresolved,
        objects_admitted: object_count,
        values_admitted: value_count,
    })
}

/// FR-106's required populations from parameters: every population of the
/// package that a reference inside a parameter's raw value names, in
/// parameter order and then the value's walk order, once each. Read from
/// the raw values because check 7 runs before check 10 admits them. A
/// reference naming no declared population is left out: check 10 refuses
/// it `dangling_reference`.
pub(super) fn parameter_populations(
    views: &[ModelView],
    parameters: &[(String, SnapshotValue)],
) -> Vec<String> {
    let mut named = Vec::new();
    for (_, raw) in parameters {
        let mut refs = Vec::new();
        collect_references(raw, &mut refs);
        for (population, _) in refs {
            if !named.contains(&population) && find_population(views, &population).is_some() {
                named.push(population);
            }
        }
    }
    named
}

/// FR-106 check 7: a required population -- the population holding `self`,
/// and, repeatedly, every population a reference field of any object of a
/// required population names -- must be complete (FR-106's own recursive
/// definition). `self_population` is the population holding `self` for this
/// particular observation, when this observation must resolve `self` at all
/// (FR-106 check 9: always for the current snapshot and an invocation's pre
/// snapshot; only for a postcondition's post snapshot) -- `None` skips this
/// check's unconditional self-population requirement, never skipping it
/// entirely. `parameter_populations` are the declared populations a
/// reference-valued parameter names ([`parameter_populations`]): required
/// in the pre snapshot of an invocation or a `PreCall` selection, and
/// empty everywhere else.
pub(super) fn check_population_completeness(
    populations: &[RawPopulation],
    completeness: &BTreeMap<String, bool>,
    self_population: Option<&str>,
    parameter_populations: &[String],
) -> Result<(), AdmissionFailure> {
    // Built from the wire-declared `reference.population` (SR-750 FND-001):
    // a dangling reference (whose target this call never admits into
    // `objects`) still makes its named population required, or check 8
    // below could never see it, and it still makes `self`'s own population
    // required even when `self` is never itself the target of any
    // reference (SR-750 FND-001, "self's population is never required at
    // all").
    let mut required: BTreeSet<String> = BTreeSet::new();
    if let Some(name) = self_population {
        required.insert(name.to_owned());
    }
    required.extend(parameter_populations.iter().cloned());
    loop {
        let mut grew = false;
        for name in required.clone() {
            let Some(entry) = populations.iter().find(|entry| entry.population == name) else {
                continue;
            };
            for object in &entry.objects {
                for (_, raw) in &object.fields {
                    let mut refs = Vec::new();
                    collect_references(raw, &mut refs);
                    for (population, _) in refs {
                        if required.insert(population) {
                            grew = true;
                        }
                    }
                }
            }
        }
        if !grew {
            break;
        }
    }
    // Report the first required-but-incomplete population, in the
    // document's own population order; `self`'s population and then each
    // parameter-named population, when the document never lists it, are
    // checked last (there is no document-order position for a population
    // the document never names).
    let mut walk_order: Vec<&str> = populations
        .iter()
        .map(|entry| entry.population.as_str())
        .collect();
    for name in self_population
        .into_iter()
        .chain(parameter_populations.iter().map(String::as_str))
    {
        if !walk_order.contains(&name) {
            walk_order.push(name);
        }
    }
    for name in walk_order {
        if required.contains(name) && completeness.get(name).copied() != Some(true) {
            return Err(incomplete(
                admission_record(Code::IncompletePopulation, "incomplete-scope")
                    .with("population", name.to_owned()),
            ));
        }
    }
    Ok(())
}

/// FR-106 check 8: closure, skipped over an incomplete population, over
/// every reference value of every object in document walk order (not gated
/// by "required": FR-106's own text names no such restriction for this
/// check, only for check 7's completeness requirement).
pub(super) fn check_population_closure(
    populations: &[RawPopulation],
    completeness: &BTreeMap<String, bool>,
    keys_by_population: &BTreeMap<String, BTreeSet<String>>,
) -> Result<(), AdmissionFailure> {
    for entry in populations {
        for object in &entry.objects {
            for (_, raw) in &object.fields {
                let mut refs = Vec::new();
                collect_references(raw, &mut refs);
                for (population, key) in refs {
                    if completeness.get(&population).copied() != Some(true) {
                        continue;
                    }
                    let known = keys_by_population
                        .get(&population)
                        .is_some_and(|keys| keys.contains(&key));
                    if known {
                        continue;
                    }
                    return Err(refuse(
                        admission_record(
                            Code::DanglingReference,
                            "absent-target-in-complete-population",
                        )
                        .with("population", population)
                        .with("object", key),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Builds the final [`AdmittedEnvironment`] from `values`, once checks 6, 7
/// and 8 have all passed for this observation.
pub(super) fn finish_populations(
    types: &TypeEnvironment,
    values: PopulationValues<'_>,
) -> Result<AdmittedEnvironment, AdmissionFailure> {
    // The dangling targets check 8 tolerated: references naming no object
    // of this snapshot, into a population the snapshot lists as incomplete
    // (FR-106 check 7: "skip the dangling check over an incomplete
    // population"). A reference into a population the snapshot does not
    // list is not tolerated (SR-750 FND-019): every reference other than
    // the tolerated ones must name an admitted object, so the closure check
    // refuses it `dangling_reference`.
    let tolerated: Vec<ObjectReference> = values
        .unresolved
        .into_iter()
        .filter(|(population, _)| values.completeness.get(population).copied() == Some(false))
        .map(|(_, reference)| reference)
        .collect();
    let usage = super::AdmissionUsage {
        document_bytes: 0,
        nesting_depth: 0,
        objects: values.objects_admitted,
        values: values.values_admitted,
    };
    let environment = ObjectEnvironment::new(types, values.objects, &tolerated)
        .map_err(map_environment_refusal)?;
    Ok(AdmittedEnvironment {
        environment,
        completeness: values.completeness,
        usage,
    })
}

/// FR-106 checks 6 to 8 over one observation (a single document, not a
/// pre/post pair): admits every population's every object's every field
/// (check 6), then completeness (7), then closure (8), in that order. An
/// invocation's pre and post snapshots instead call
/// [`admit_population_values`], [`check_population_completeness`] and
/// [`check_population_closure`] directly, so the two documents' checks
/// interleave by check number rather than one document finishing all of
/// 6-8 before the other starts check 6 (SR-750 FND-005).
pub(super) fn admit_populations(
    views: &[ModelView],
    types: &TypeEnvironment,
    populations: &[RawPopulation],
    self_population: Option<&str>,
    limits: ObservationLimits,
) -> Result<AdmittedEnvironment, AdmissionFailure> {
    let values = admit_population_values(views, types, populations, limits)?;
    check_population_completeness(populations, &values.completeness, self_population, &[])?;
    check_population_closure(
        populations,
        &values.completeness,
        &values.keys_by_population,
    )?;
    finish_populations(types, values)
}

fn map_environment_refusal(
    error: crate::model::object_environment::ObjectEnvironmentRefusal,
) -> AdmissionFailure {
    match error.cause {
        ObjectEnvironmentCause::DanglingReference(target) => refuse(
            admission_record(
                Code::DanglingReference,
                "absent-target-in-complete-population",
            )
            .with("object", target.object().as_str().to_owned()),
        ),
        _ => fault("object-environment-refused-after-admission-checks"),
    }
}

/// Resolves `self` by conformance (QSL-277's ruling): the admitted object's
/// *actual* type -- whatever the snapshot declared it, not necessarily
/// `context_name` itself -- must conform to the clause's context type
/// (itself or a subtype). The returned reference carries that actual type,
/// so a later frame check (check 11) sees the object as it was really
/// admitted, including a retype across an invocation's pre/post pair.
///
/// Resolving by conformance, not exact type, is FR-106 check 9's own rule,
/// not an approximation of it: `context_name`'s effective type only locates
/// the population's universe (every type in one population's hierarchy
/// shares a universe, `ObjectEnvironment::find`'s own doc comment), and
/// `self_object.key` alone -- not a caller-narrowed type -- names the
/// object within it, the same way FR-109's `Function`-selection object
/// argument does (`ObjectEnvironment::find`, `population_universe`).
pub(super) fn resolve_self(
    views: &[ModelView],
    types: &TypeEnvironment,
    context_view: &ModelView,
    context_name: &str,
    environment: &ObjectEnvironment,
    self_object: &SelectedObject,
) -> Result<ObjectReference, AdmissionFailure> {
    let context_effective = context_view
        .view
        .type_identities()
        .iter()
        .find(|(key, _)| key.node == context_name)
        .map(|(_, effective)| *effective)
        .ok_or_else(|| fault("context-type-unresolved"))?;
    let universe = population_universe(views, context_effective)?;
    let wrong_role_mapping = || {
        refuse(admission_record(
            Code::InvalidRuntimeInput,
            "wrong-role-mapping",
        ))
    };
    let reference = environment
        .find(universe, &self_object.key)
        .ok_or_else(wrong_role_mapping)?;
    if types.conforms(reference.object_type(), context_effective) {
        Ok(reference.clone())
    } else {
        Err(wrong_role_mapping())
    }
}

/// FR-106 check 10 over the parameters, with check 8's closure rule applied
/// to every reference value it admits ([`References::check_closure`]): the
/// operation's parameter bindings in declared order. A parameter resolves
/// against the pre snapshot (`references`): the operation reads its
/// arguments before it runs.
pub(super) fn admit_parameters(
    references: &mut References<'_>,
    operation: &OperationDeclaration,
    parameters: &[(String, SnapshotValue)],
) -> Result<Vec<(String, Value)>, AdmissionFailure> {
    let mut admitted = Vec::with_capacity(operation.parameters().len());
    for (name, value_type) in operation.parameters() {
        let raw = raw_field(parameters, name).ok_or_else(|| {
            refuse(
                admission_record(Code::InvalidRuntimeInput, "missing-member")
                    .with("field", name.clone()),
            )
        })?;
        if !field_kind_matches(raw, value_type) {
            return Err(refuse(
                admission_record(Code::InvalidRuntimeInput, "wrong-value-kind")
                    .with("field", name.clone()),
            ));
        }
        let value = admit_scalar(references, raw, value_type)
            .map_err(|record| refuse(record.with("field", name.clone())))?;
        references.check_closure(raw).map_err(refuse)?;
        admitted.push((name.clone(), value));
    }
    for (name, _) in parameters {
        if !operation
            .parameters()
            .iter()
            .any(|(declared, _)| declared == name)
        {
            return Err(refuse(
                admission_record(Code::InvalidRuntimeInput, "unknown-member")
                    .with("field", name.clone()),
            ));
        }
    }
    Ok(admitted)
}

/// FR-106 check 10 over an invocation's result: `None` for an operation
/// with no declared result. The result resolves against the post snapshot
/// (`references`): it is observed after the operation, so it can name an
/// object the operation created.
pub(super) fn admit_result(
    references: &mut References<'_>,
    operation: &OperationDeclaration,
    result: &ResultValue,
) -> Result<Option<Value>, AdmissionFailure> {
    match (operation.result(), result) {
        (None, ResultValue::Null) => Ok(None),
        (None, ResultValue::Value(_)) => Err(refuse(admission_record(
            Code::InvalidRuntimeInput,
            "unknown-member",
        ))),
        (Some(_), ResultValue::Null) => Err(refuse(admission_record(
            Code::InvalidRuntimeInput,
            "missing-member",
        ))),
        (Some(value_type), ResultValue::Value(raw)) => {
            if !field_kind_matches(raw, value_type) {
                return Err(refuse(admission_record(
                    Code::InvalidRuntimeInput,
                    "wrong-value-kind",
                )));
            }
            let value = admit_scalar(references, raw, value_type).map_err(refuse)?;
            references.check_closure(raw).map_err(refuse)?;
            Ok(Some(value))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SR-750 FND-002: a non-ASCII digest string must refuse, never panic
    /// on a byte-index char-boundary slice. `read_document_ref` is
    /// `hex_bytes`'s one caller reachable with untrusted input (a `pre`/
    /// `post` document reference's own digest).
    #[test]
    fn a_non_ascii_digest_refuses_rather_than_panics() {
        let value: OrderedJson = serde_json::from_value(serde_json::json!({
            "identity": {
                "authority": "a",
                "identity": "b",
                "revision_namespace": "c",
                "revision": "d",
            },
            "digest": "sha256-jcs:aé00000000000000000000000000000000000000000000000000000000000",
        }))
        .expect("test fixture is JSON");
        let result = read_document_ref(&value, "pre");
        assert!(
            matches!(result, Err(AdmissionFailure::Refused(_))),
            "expected Err(Refused(..)), got {result:?}"
        );
    }

    /// `hex_bytes` decodes a well-formed 32-byte digest.
    #[test]
    fn hex_bytes_decodes_a_well_formed_digest() {
        let hex = "00".repeat(32);
        let bytes = hex_bytes(&hex).expect("32 zero bytes decode");
        assert_eq!(bytes, vec![0u8; 32]);
    }

    /// `hex_bytes` refuses an odd-length string before ever indexing it.
    #[test]
    fn hex_bytes_refuses_an_odd_length_string() {
        assert_eq!(hex_bytes("abc"), None);
    }

    /// `collect_references` finds a reference nested under `present` and
    /// inside a `sequence`, in walk order.
    #[test]
    fn collect_references_walks_present_and_sequence() {
        let raw = SnapshotValue::Sequence(vec![
            SnapshotValue::Present(Box::new(SnapshotValue::Reference(SelectedObject {
                population: "p1".to_owned(),
                key: "k1".to_owned(),
            }))),
            SnapshotValue::Reference(SelectedObject {
                population: "p2".to_owned(),
                key: "k2".to_owned(),
            }),
        ]);
        let mut found = Vec::new();
        collect_references(&raw, &mut found);
        assert_eq!(
            found,
            vec![
                ("p1".to_owned(), "k1".to_owned()),
                ("p2".to_owned(), "k2".to_owned()),
            ]
        );
    }

    /// `collect_references` finds nothing under a scalar or absent value.
    #[test]
    fn collect_references_finds_nothing_in_a_scalar() {
        let mut found = Vec::new();
        collect_references(&SnapshotValue::Boolean(true), &mut found);
        collect_references(&SnapshotValue::Absent, &mut found);
        collect_references(&SnapshotValue::Integer("1".to_owned()), &mut found);
        assert!(found.is_empty());
    }

    fn reference(population: &str, key: &str) -> SnapshotValue {
        SnapshotValue::Reference(SelectedObject {
            population: population.to_owned(),
            key: key.to_owned(),
        })
    }

    fn empty_population(population: &str, complete: bool) -> RawPopulation {
        RawPopulation {
            population: population.to_owned(),
            complete,
            objects: Vec::new(),
        }
    }

    /// SR-771 FND-001: `References::check_closure` (check 8 over a check-10
    /// value) refuses a reference naming no object of a complete population
    /// and of a population the snapshot does not list, and tolerates one
    /// into a population listed incomplete -- the same rule
    /// `check_population_closure` plus `finish_populations` apply to object
    /// fields. The refusal names the reference's population and key, and a
    /// reference nested under `present` or a `sequence` is found.
    #[test]
    fn check_closure_refuses_dangling_and_tolerates_incomplete() {
        let populations = vec![
            empty_population("complete", true),
            empty_population("partial", false),
        ];
        let references = References::over(&[], &populations);

        let record = references
            .check_closure(&SnapshotValue::Present(Box::new(reference(
                "complete", "k",
            ))))
            .expect_err("a key absent from a complete population refuses");
        assert_eq!(record.code, Code::DanglingReference);
        assert_eq!(record.cause, "absent-target-in-complete-population");
        assert_eq!(
            record.fields.get("population").map(String::as_str),
            Some("complete")
        );
        assert_eq!(record.fields.get("object").map(String::as_str), Some("k"));

        let record = references
            .check_closure(&reference("unlisted", "k"))
            .expect_err("a population the snapshot does not list refuses");
        assert_eq!(record.code, Code::DanglingReference);

        assert_eq!(
            references.check_closure(&SnapshotValue::Sequence(vec![reference("partial", "k")])),
            Ok(())
        );
        assert_eq!(
            references.check_closure(&SnapshotValue::Boolean(true)),
            Ok(())
        );
    }
}
