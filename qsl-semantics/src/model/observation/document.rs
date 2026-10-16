// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-106 checks 1, 5 to 10: reading one snapshot or invocation document
//! (check 1), admitting its populations and objects (checks 5 to 8) and
//! resolving `self`, `parameters` and `result` (checks 9 and 10).

use std::collections::{BTreeMap, BTreeSet};

use quire_exact::{
    CollectionKind, FieldValue, Integer, ObjectId, ObjectReference, UniverseId, Value,
};

use qsl_foundation::diagnostic::Code;

use super::helpers::{admission_record, object_reference};
use super::tree::{read_raw_value, Json, Object};
use super::{
    check_document_digest, fault, incomplete, population_universe, refuse, AdmissionFailure,
    AdmissionRecord, DocumentRef, ModelView, NativeKind, ObservationLimits, OutOfRange, PathStep,
    PostStateRange, RawForm, RawNative, RawNativeKind, RawValue, ReadMeter, SelectedObject,
    SnapshotValue,
};
use crate::model::key::DeclarationKey;
use crate::model::object_environment::ObjectEnvironment;
use crate::model::operation::OperationDeclaration;
use quire_semantic_value::declaration::{
    CompositeShape, FieldDeclaration, FieldRef, ObjectTypeDeclaration, TypeEnvironment,
};
use quire_semantic_value::object_closure::{
    ObjectClosure, ObjectClosureCause, ObjectClosureRefusal,
};

/// Which wire format a document is read as (FR-106 "Document forms").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DocumentKind {
    Snapshot,
    /// An invocation read whole: its call and its post side, for a
    /// postcondition or a `Frame` run.
    Invocation,
    /// An invocation read for a precondition: its call alone. Its `post`,
    /// `result`, `created` and `deleted` members are neither required nor
    /// read (FR-106 "Behavior": a precondition's input is the pre-call
    /// observation).
    InvocationCall,
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
pub enum ObservationRole {
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
    /// The invocation this snapshot says it observes, when it names one.
    pub(super) invocation: Option<InvocationLabels>,
    pub(super) model: ModelHeader,
    pub(super) populations: Vec<RawPopulation>,
    /// The snapshot's `relationships` entries, in document order (FR-106).
    pub(super) relationships: Vec<super::relationships::RawEntry>,
}

/// The four FR-001 labels of an invocation document's own `identity`: how
/// a snapshot names the invocation it observes. No digest: the
/// invocation's digest covers its `pre` and `post` references, whose
/// digests cover these snapshots.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct InvocationLabels {
    authority: String,
    identity: String,
    revision_namespace: String,
    revision: String,
}

impl InvocationLabels {
    fn names(&self, invocation: &DocumentRef) -> bool {
        self.authority == invocation.authority
            && self.identity == invocation.identity
            && self.revision_namespace == invocation.revision_namespace
            && self.revision == invocation.revision
    }
}

/// FR-106 check 5: a snapshot that names an invocation names the selected
/// invocation's own identity. A snapshot that names none is unbound.
pub(super) fn check_snapshot_invocation(
    snapshot: &SnapshotDocument,
    snapshot_identity: &DocumentRef,
    invocation: &DocumentRef,
) -> Result<(), AdmissionFailure> {
    match &snapshot.invocation {
        Some(named) if !named.names(invocation) => Err(refuse(
            admission_record(Code::WrongSnapshot, "wrong-invocation")
                .with("document", snapshot_identity.identity.clone())
                .with("field", "invocation")
                .with("required", invocation.identity.clone())
                .with("actual", named.identity.clone()),
        )),
        Some(_) | None => Ok(()),
    }
}

#[derive(Clone, Debug)]
pub(super) enum ResultValue {
    Null,
    Value(SnapshotValue),
}

/// An invocation's pre-call half: what a precondition reads of it.
#[derive(Clone, Debug)]
pub(super) struct InvocationCall {
    pub(super) model: ModelHeader,
    pub(super) context: String,
    pub(super) operation: String,
    pub(super) self_object: SelectedObject,
    pub(super) pre: DocumentRef,
    /// The invocation's own `parameters` member, in document order.
    pub(super) parameters: Vec<(String, SnapshotValue)>,
}

/// A whole invocation: its call and its post side.
#[derive(Clone, Debug)]
pub(super) struct InvocationDocument {
    pub(super) call: InvocationCall,
    pub(super) post: DocumentRef,
    pub(super) result: ResultValue,
    pub(super) created: Vec<SelectedObject>,
    pub(super) deleted: Vec<SelectedObject>,
}

enum Body {
    Snapshot(Box<SnapshotDocument>),
    Invocation(Box<InvocationDocument>),
    InvocationCall(Box<InvocationCall>),
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
            Body::Snapshot(snapshot) => Some(snapshot.as_ref()),
            Body::Invocation(_) | Body::InvocationCall(_) => None,
        }
    }

    pub(super) fn as_invocation(&self) -> Option<&InvocationDocument> {
        match &self.body {
            Body::Invocation(invocation) => Some(invocation),
            Body::Snapshot(_) | Body::InvocationCall(_) => None,
        }
    }

    /// The call half of an invocation read for a precondition
    /// ([`DocumentKind::InvocationCall`]).
    pub(super) fn as_invocation_call(&self) -> Option<&InvocationCall> {
        match &self.body {
            Body::InvocationCall(call) => Some(call),
            Body::Snapshot(_) | Body::Invocation(_) => None,
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

/// FR-106 check 1: read one document from `provision` under `selected`'s
/// digest, running the seven ordered conditions.
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
    let document_bytes = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if document_bytes > limits.document_bytes {
        return Err(super::input_bytes_exceeded(
            limits.document_bytes,
            u128::from(document_bytes),
        ));
    }
    // The one read of the document, through the shared reader at any depth
    // (FR-261): check 1.3 digests this tree, and every later check reads
    // members from it.
    let read = quire_canonical::read(bytes, limits.document_bytes);
    // 1.3: digest, before any member is read. Bytes the reader refuses are
    // digested raw, and so refuse here.
    check_document_digest(bytes, &read, selected.digest, limits)?;
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
    let Ok(document) = read else {
        return Err(unsupported_wire());
    };
    let object = Json::of(document.root())
        .as_object()
        .ok_or_else(unsupported_wire)?;
    let mut meter = ReadMeter::new(limits);

    // 1.4: format.
    let expected_format = match kind {
        DocumentKind::Snapshot => "quire.state.snapshot/v1",
        DocumentKind::Invocation | DocumentKind::InvocationCall => "quire.state.invocation/v1",
    };
    if object.member("format").and_then(|value| value.as_str()) != Some(expected_format) {
        return Err(refuse(admission_record(
            Code::UnknownWire,
            "unsupported-wire",
        )));
    }

    // 1.5/1.6: member presence, in the member order FR-106 states. An
    // invocation read for a precondition requires none of its post side
    // (`post`, `result`, `created`, `deleted`), but still names them, so
    // they are not unknown members.
    const INVOCATION_MEMBERS: &[&str] = &[
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
    ];
    let always_required: &[&str] = match kind {
        DocumentKind::Snapshot => &["format", "identity", "observation", "model", "populations"],
        DocumentKind::Invocation => INVOCATION_MEMBERS,
        DocumentKind::InvocationCall => &[
            "format",
            "identity",
            "model",
            "context",
            "operation",
            "self",
            "pre",
            "parameters",
        ],
    };
    let allowed: &[&str] = match kind {
        DocumentKind::Snapshot => &[
            "format",
            "identity",
            "observation",
            "anchor",
            "invocation",
            "model",
            "populations",
            "relationships",
        ],
        DocumentKind::Invocation | DocumentKind::InvocationCall => INVOCATION_MEMBERS,
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
        if !allowed.contains(&key) {
            return Err(refuse(
                admission_record(Code::InvalidRuntimeInput, "unknown-member")
                    .with("field", key.to_owned()),
            ));
        }
    }

    // 1.7: the document's own four labels.
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

    let model = read_model(object)?;

    let body = match kind {
        DocumentKind::Snapshot => {
            Body::Snapshot(Box::new(read_snapshot_body(object, model, &mut meter)?))
        }
        DocumentKind::Invocation => {
            Body::Invocation(Box::new(read_invocation_body(object, model, &mut meter)?))
        }
        DocumentKind::InvocationCall => {
            Body::InvocationCall(Box::new(read_invocation_call(object, model, &mut meter)?))
        }
    };
    Ok(ReadDocument {
        identity: document_identity,
        usage: super::AdmissionUsage {
            document_bytes,
            objects: meter.objects,
            values: meter.values,
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
fn read_model(object: Object<'_>) -> Result<ModelHeader, AdmissionFailure> {
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
    value: Json<'_>,
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
    value: Json<'_>,
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
    object: Object<'_>,
    model: ModelHeader,
    meter: &mut ReadMeter,
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
    let invocation = match object.member("invocation") {
        Some(value) => Some(read_invocation_labels(value)?),
        None => None,
    };
    let populations = read_populations(object, meter)?;
    let relationships = super::relationships::read_relationships(object, meter)?;
    Ok(SnapshotDocument {
        observation,
        anchor,
        invocation,
        model,
        populations,
        relationships,
    })
}

/// Reads a snapshot's `invocation` member: the invocation's four identity
/// labels.
fn read_invocation_labels(value: Json<'_>) -> Result<InvocationLabels, AdmissionFailure> {
    let object = value.as_object().ok_or_else(|| wrong_kind("invocation"))?;
    let label = |name: &'static str| {
        object
            .member(name)
            .and_then(|value| value.as_str())
            .map(str::to_owned)
            .ok_or_else(|| missing_member("invocation"))
    };
    Ok(InvocationLabels {
        authority: label("authority")?,
        identity: label("identity")?,
        revision_namespace: label("revision_namespace")?,
        revision: label("revision")?,
    })
}

fn read_populations(
    object: Object<'_>,
    meter: &mut ReadMeter,
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
            meter.object()?;
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
            let mut fields = Vec::new();
            for (name, value) in field_items.iter() {
                let raw = read_raw_value(value, &mut || meter.value())?
                    .ok_or_else(|| wrong_kind(name))?;
                fields.push((name.to_owned(), raw));
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

/// Reads an invocation's call half: `context`, `operation`, `self`, `pre`
/// and `parameters`, and nothing of its post side.
fn read_invocation_call(
    object: Object<'_>,
    model: ModelHeader,
    meter: &mut ReadMeter,
) -> Result<InvocationCall, AdmissionFailure> {
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
    let parameter_items = object
        .member("parameters")
        .and_then(|value| value.as_object())
        .ok_or_else(|| wrong_kind("parameters"))?;
    let mut parameters = Vec::new();
    for (name, value) in parameter_items.iter() {
        let raw = read_raw_value(value, &mut || meter.value())?.ok_or_else(|| wrong_kind(name))?;
        parameters.push((name.to_owned(), raw));
    }
    Ok(InvocationCall {
        model,
        context,
        operation,
        self_object,
        pre,
        parameters,
    })
}

/// Reads a whole invocation: its call half, then `post`, `result`,
/// `created` and `deleted`.
fn read_invocation_body(
    object: Object<'_>,
    model: ModelHeader,
    meter: &mut ReadMeter,
) -> Result<InvocationDocument, AdmissionFailure> {
    let call = read_invocation_call(object, model, meter)?;
    let post = object
        .member("post")
        .ok_or_else(|| missing_member("post"))
        .and_then(|value| read_document_ref(value, "post"))?;
    let result = match object.member("result") {
        Some(value) if value.is_null() => ResultValue::Null,
        Some(value) => ResultValue::Value(
            read_raw_value(value, &mut || meter.value())?.ok_or_else(|| wrong_kind("result"))?,
        ),
        None => return Err(missing_member("result")),
    };
    let created = object
        .member("created")
        .and_then(|value| value.as_array())
        .ok_or_else(|| wrong_kind("created"))?
        .map(|item| read_object_ref(item, "created"))
        .collect::<Result<Vec<_>, _>>()?;
    let deleted = object
        .member("deleted")
        .and_then(|value| value.as_array())
        .ok_or_else(|| wrong_kind("deleted"))?
        .map(|item| read_object_ref(item, "deleted"))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(InvocationDocument {
        call,
        post,
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
    /// Each integer admitted outside its declared range, in walk order
    /// (always empty for a document admitted under
    /// [`PostStateRange::Refuse`]).
    pub(super) out_of_range: Vec<OutOfRange>,
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
        (raw.form(), value_type),
        (RawForm::Boolean(_), quire_exact::ValueType::Boolean)
            | (
                RawForm::Integer(_),
                quire_exact::ValueType::Integer | quire_exact::ValueType::Int(_)
            )
            | (RawForm::Reference(_), quire_exact::ValueType::Reference(_))
            | (
                RawForm::Absent | RawForm::Present(_),
                quire_exact::ValueType::Option(_)
            )
            | (RawForm::Sequence(_), quire_exact::ValueType::Collection(_))
            | (RawForm::Record(_), quire_exact::ValueType::Composite(_))
            | (RawForm::Variant(_), quire_exact::ValueType::Enum(_))
    ) || matches!(raw.form(), RawForm::Native(_))
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
    raw: RawForm<'_>,
    value_type: &quire_exact::ValueType,
    presence: quire_exact::Presence,
) -> bool {
    // A native payload object is judged by `admit_native`, whose order puts
    // its own envelope defects before declared-type fit.
    if matches!(raw, RawForm::Native(_)) {
        return true;
    }
    if presence == quire_exact::Presence::Optional {
        return matches!(raw, RawForm::Absent | RawForm::Present(_));
    }
    matches!(
        (raw, value_type),
        (RawForm::Boolean(_), quire_exact::ValueType::Boolean)
            | (
                RawForm::Integer(_),
                quire_exact::ValueType::Integer | quire_exact::ValueType::Int(_)
            )
            | (RawForm::Reference(_), quire_exact::ValueType::Reference(_))
            | (RawForm::Sequence(_), quire_exact::ValueType::Collection(_))
            | (RawForm::Record(_), quire_exact::ValueType::Composite(_))
            | (RawForm::Variant(_), quire_exact::ValueType::Enum(_))
    )
}

/// Resolves each `{population, key}` reference value against the objects
/// one snapshot's populations actually hold, and records the ones that name
/// no admitted object. Built once per snapshot, before check 6 walks it, so
/// a reference resolves whatever the walk order of its target.
pub(super) struct References<'a> {
    views: &'a [ModelView],
    /// The checked package's type environment, whose closed supertypes
    /// graph decides whether an admitted object conforms to a reference's
    /// declared type ([`TypeEnvironment::conforms`]).
    types: &'a TypeEnvironment,
    /// Every object of the snapshot whose type and key resolve, by its wire
    /// `(population, key)`.
    admitted: BTreeMap<(&'a str, &'a str), ObjectReference>,
    /// Each listed population's `complete` flag, as check 7 reads it (the
    /// same last-entry-wins insert [`admit_population_values`] makes).
    completeness: BTreeMap<&'a str, bool>,
    /// Each reference that named no object of the snapshot, with the wire
    /// population it named, in walk order.
    unresolved: Vec<(String, ObjectReference)>,
    /// The declaration key of each selected record value type by its IR
    /// node identity, built on first use.
    record_keys: Option<BTreeMap<String, quire_exact::NodeKey>>,
    /// Each selected variant type by its IR node identity, built on first
    /// use from the same declarations the type environment holds.
    variant_keys: Option<BTreeMap<String, crate::check::VariantTypeDeclaration>>,
}

impl<'a> References<'a> {
    /// Indexes `populations`' objects. An object whose type or key does not
    /// resolve is left out: check 6 refuses it in walk order before any
    /// reference to it is used.
    pub(super) fn over(
        views: &'a [ModelView],
        types: &'a TypeEnvironment,
        populations: &'a [RawPopulation],
    ) -> Self {
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
            types,
            admitted,
            completeness,
            unresolved: Vec::new(),
            record_keys: None,
            variant_keys: None,
        }
    }

    /// The variant type the IR node identity `identity` names, or `None`
    /// when no selected package declares a variant type by that identity
    /// (another meaning or no declaration at all). It is the declaration the
    /// type environment holds, so a value admitted here equals the same case
    /// a unit's literal builds (FR-360 item 11).
    fn variant_type(&mut self, identity: &str) -> Option<&crate::check::VariantTypeDeclaration> {
        let views = self.views;
        self.variant_keys
            .get_or_insert_with(|| {
                let unbounded = qsl_foundation::IdentityLimits {
                    input_bytes: u64::MAX,
                };
                let mut found = BTreeMap::new();
                for view in views {
                    let Ok(declared) =
                        crate::check::variant_type_declarations(&view.view, "", unbounded)
                    else {
                        continue;
                    };
                    for (key, declaration) in declared {
                        found.entry(key.node).or_insert(declaration);
                    }
                }
                found
            })
            .get(identity)
    }

    /// The declaration key the record value type named by the IR node
    /// identity `identity` is declared under in the type environment, or
    /// `None` when no selected package declares a record value type by that
    /// identity (an object type, an event type or no declaration at all).
    /// It is the record's model node key (FR-359 item 9), a function of the
    /// declaring package and node alone. The keys of every selected record
    /// are computed once, on the first record value, with no byte bound: a
    /// record the unit assembled had already keyed under the unit's own
    /// identity limits, and the key does not depend on the limit.
    fn record_type(&mut self, identity: &str) -> Option<quire_exact::NodeKey> {
        let views = self.views;
        self.record_keys
            .get_or_insert_with(|| {
                let unbounded = qsl_foundation::IdentityLimits {
                    input_bytes: u64::MAX,
                };
                let mut keys = BTreeMap::new();
                for view in views {
                    for record in &view.view.domain_package().records {
                        let crate::model::domain_package::DomainPackageRecord::RecordValueType(
                            declared,
                        ) = record
                        else {
                            continue;
                        };
                        if let Ok(Some(key)) = crate::check::record_node_key(record, unbounded) {
                            keys.entry(declared.key.node.clone()).or_insert(key);
                        }
                    }
                }
                keys
            })
            .get(identity)
            .copied()
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
    /// that object, whose most-specific type must conform to `declared`
    /// (QSpec FR-151, [`TypeEnvironment::conforms`]: the same type, or a
    /// chain of declared supertypes leads from it to `declared`), else
    /// `wrong-value-kind` (check 6.5). A reference to no
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
            return if self.types.conforms(target.object_type(), declared) {
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

/// An in-document integer outside its declared `Int[lower, upper]`, kept
/// exactly: the range it broke, the value, and its position when it is a
/// sequence element.
struct RangeBreach {
    range: quire_exact::IntegerInterval,
    observed: Integer,
    index: Option<usize>,
    /// The record field names from the checked field or element down to the
    /// integer, outermost first (FR-106 check 6.5's nested-record steps).
    steps: Vec<PathStep>,
}

/// An object type's own field's admission: `presence`, not `value_type`,
/// decides whether `absent`/`present` is admitted (see
/// [`object_field_kind_matches`]'s doc). Parameters and results have no
/// `FieldValue` slot to fill (they admit straight to a [`Value`] via
/// [`admit_form`] plus [`field_kind_matches`]'s `ValueType::Option`
/// reading), so this is the one caller [`admit_populations`] needs.
fn admit_object_field(
    references: &mut References<'_>,
    raw: &SnapshotValue,
    value_type: &quire_exact::ValueType,
    presence: quire_exact::Presence,
    witness: Option<&mut Vec<RangeBreach>>,
) -> Result<FieldValue, AdmissionRecord> {
    admit_field_form(references, raw.form(), value_type, presence, witness)
}

/// [`admit_object_field`] over a value's form.
fn admit_field_form(
    references: &mut References<'_>,
    raw: RawForm<'_>,
    value_type: &quire_exact::ValueType,
    presence: quire_exact::Presence,
    witness: Option<&mut Vec<RangeBreach>>,
) -> Result<FieldValue, AdmissionRecord> {
    admit_value(references, raw, value_type, presence, witness)
}

/// A form admitted as a value of `value_type`: a boolean, an integer in its
/// declared interval, a reference to an object of a conforming type, a
/// sequence, or a record whose type conforms to the declared record type; any
/// other pairing is `wrong-value-kind`.
fn admit_form(
    references: &mut References<'_>,
    form: RawForm<'_>,
    value_type: &quire_exact::ValueType,
    witness: Option<&mut Vec<RangeBreach>>,
) -> Result<Value, AdmissionRecord> {
    match admit_value(
        references,
        form,
        value_type,
        quire_exact::Presence::Required,
        witness,
    )? {
        FieldValue::Present(value) => Ok(value),
        FieldValue::Absent | FieldValue::Null => Err(admission_record(
            Code::InvalidRuntimeInput,
            "wrong-value-kind",
        )),
    }
}

/// A scalar form admitted as a value of `value_type`: a boolean, an integer
/// in its declared interval, or a reference to an object of a conforming
/// type; any other pairing is `wrong-value-kind`. `witness` is `Some` for a
/// post-state field (FR-106 check 6.5's post-state rule): an integer outside
/// its declared range is admitted exactly and recorded there instead of
/// refused. `None` refuses it.
fn admit_leaf(
    references: &mut References<'_>,
    form: RawForm<'_>,
    value_type: &quire_exact::ValueType,
    witness: Option<&mut Vec<RangeBreach>>,
) -> Result<Value, AdmissionRecord> {
    match (form, value_type) {
        (RawForm::Boolean(value), quire_exact::ValueType::Boolean) => Ok(Value::Boolean(value)),
        (RawForm::Integer(spelling), quire_exact::ValueType::Integer) => spelling
            .parse::<Integer>()
            .map(Value::Integer)
            .map_err(|_| admission_record(Code::InvalidRuntimeInput, "invalid-value")),
        (RawForm::Integer(spelling), quire_exact::ValueType::Int(interval)) => {
            let value = spelling
                .parse::<Integer>()
                .map_err(|_| admission_record(Code::InvalidRuntimeInput, "invalid-value"))?;
            if interval.contains(&value) {
                return Ok(Value::Integer(value));
            }
            match witness {
                Some(found) => {
                    found.push(RangeBreach {
                        range: interval.clone(),
                        observed: value.clone(),
                        index: None,
                        steps: Vec::new(),
                    });
                    Ok(Value::Integer(value))
                }
                None => Err(admission_record(Code::InvalidRuntimeInput, "invalid-value")),
            }
        }
        (RawForm::Reference(reference), quire_exact::ValueType::Reference(type_identity)) => {
            references
                .resolve(reference, *type_identity)
                .map(Value::Reference)
        }
        (RawForm::Variant(variant), quire_exact::ValueType::Enum(slot)) => {
            admit_variant(references, variant, slot)
        }
        (RawForm::Native(native), value_type) => admit_native(native, value_type),
        _ => Err(admission_record(
            Code::InvalidRuntimeInput,
            "wrong-value-kind",
        )),
    }
}

/// A native payload object's kind and payload text, once its envelope is
/// sound. The first defect refuses, in FR-106's native order: a `kind` that
/// is not text (`wrong-value-kind`), a `kind` that names no native kind
/// (`invalid-value`, the members unexamined), a missing payload member
/// (`missing-member`), an unknown member (`unknown-member`, the first in
/// document order), and a payload that is not text (`wrong-value-kind`).
/// Each names the member.
#[qsl_attrs::string_edge]
fn native_payload(native: RawNative<'_>) -> Result<(NativeKind, &str), AdmissionRecord> {
    let refuse = |cause: &'static str, member: &str| {
        admission_record(Code::InvalidRuntimeInput, cause).with("member", member.to_owned())
    };
    let kind = match native.kind() {
        RawNativeKind::NotText => return Err(refuse("wrong-value-kind", "kind")),
        RawNativeKind::Unknown => return Err(refuse("invalid-value", "kind")),
        RawNativeKind::Known(kind) => kind,
    };
    let member = match kind {
        NativeKind::String | NativeKind::Uuid | NativeKind::Text => "text",
        NativeKind::Timestamp => "nanoseconds",
    };
    let Some((_, payload)) = native.members().find(|(name, _)| *name == member) else {
        return Err(refuse("missing-member", member));
    };
    if let Some((unknown, _)) = native.members().find(|(name, _)| *name != member) {
        return Err(refuse("unknown-member", unknown));
    }
    payload
        .map(|text| (kind, text))
        .ok_or_else(|| refuse("wrong-value-kind", member))
}

/// A native payload object admitted as a value of `value_type` (FR-106's
/// native order): the envelope ([`native_payload`]), then the slot's fit
/// (`wrong-value-kind`), then, for a UUID or a Timestamp, the kernel's
/// canonical constructor (`invalid-value` naming the payload member, with
/// the kernel's class) and, for a Text, its profile length ([`admit_text`]).
/// A String has every Unicode scalar sequence as a value, so it has no
/// payload refusal.
fn admit_native(
    native: RawNative<'_>,
    value_type: &quire_exact::ValueType,
) -> Result<Value, AdmissionRecord> {
    let (kind, payload) = native_payload(native)?;
    let invalid = |refusal: quire_exact::ConstructionRefusal, member: &'static str| {
        admission_record(Code::InvalidRuntimeInput, "invalid-value")
            .with("member", member)
            .kernel_class(refusal.cause)
    };
    match (kind, value_type) {
        (NativeKind::String, quire_exact::ValueType::String) => {
            Ok(Value::String(payload.to_owned()))
        }
        (NativeKind::Uuid, quire_exact::ValueType::Uuid) => {
            quire_exact::Uuid::from_canonical_text(payload)
                .map(Value::Uuid)
                .map_err(|refusal| invalid(refusal, "text"))
        }
        (NativeKind::Timestamp, quire_exact::ValueType::Timestamp) => {
            quire_exact::Timestamp::from_canonical_text(payload)
                .map(Value::Timestamp)
                .map_err(|refusal| invalid(refusal, "nanoseconds"))
        }
        (NativeKind::Text, quire_exact::ValueType::Text(declared)) => admit_text(payload, declared),
        // Type fit (check 6) refuses every other pairing before any length.
        (
            NativeKind::String | NativeKind::Uuid | NativeKind::Timestamp | NativeKind::Text,
            quire_exact::ValueType::Boolean
            | quire_exact::ValueType::Integer
            | quire_exact::ValueType::Int(_)
            | quire_exact::ValueType::Rational(_)
            | quire_exact::ValueType::Decimal(_)
            | quire_exact::ValueType::Float(_)
            | quire_exact::ValueType::Quantity(_)
            | quire_exact::ValueType::Text(_)
            | quire_exact::ValueType::String
            | quire_exact::ValueType::Uuid
            | quire_exact::ValueType::Timestamp
            | quire_exact::ValueType::Enum(_)
            | quire_exact::ValueType::Option(_)
            | quire_exact::ValueType::Composite(_)
            | quire_exact::ValueType::Collection(_)
            | quire_exact::ValueType::Reference(_)
            | quire_exact::ValueType::Population(_),
        ) => Err(admission_record(
            Code::InvalidRuntimeInput,
            "wrong-value-kind",
        )),
    }
}

/// A Text payload admitted into `declared` (FR-106 check 7): the kernel
/// retains the sequence under the declared profile, and a profile length
/// outside `[min, max]`, measured on the retained sequence, refuses
/// `invalid-value` naming `text`, the declared type (`expected`) and the
/// measured length (`actual`). Nothing is trimmed, truncated or repaired.
#[qsl_attrs::string_edge]
fn admit_text(payload: &str, declared: &quire_exact::TextType) -> Result<Value, AdmissionRecord> {
    let profile = declared.profile();
    let retained: String = match profile.normalization() {
        Some(form) => form.apply(payload).collect(),
        None => payload.to_owned(),
    };
    let measured = profile.length(&retained);
    if !(declared.min()..=declared.max()).contains(&measured) {
        return Err(admission_record(Code::InvalidRuntimeInput, "invalid-value")
            .with("member", "text")
            .with(
                "expected",
                format!(
                    "Text[{}, {}; {}]",
                    declared.min(),
                    declared.max(),
                    profile.as_str()
                ),
            )
            .with("actual", measured.to_string()));
    }
    let mut meter = quire_exact::Meter::new(crate::check::UNBOUNDED_SCALAR_LIMITS);
    quire_exact::admit_text(
        &quire_exact::TextPayload::from_utf8(payload.as_bytes())
            .map_err(|_| admission_record(Code::InvalidRuntimeInput, "invalid-value"))?,
        declared,
        &mut meter,
    )
    .completed()
    .map(Value::Text)
    .ok_or_else(|| admission_record(Code::InvalidRuntimeInput, "invalid-value"))
}

/// A `variant` form admitted as a value of the enum type `slot` (FR-360
/// item 9). The first defect found refuses, in this order: a missing `type`
/// or `case` (`type` reported first), an extra payload member, a `type` that
/// names any declaration other than the slot's variant type, and a `case`
/// that is not a declared case of it, compared byte for byte.
fn admit_variant(
    references: &mut References<'_>,
    variant: super::RawVariant<'_>,
    slot: &quire_exact::EnumShape,
) -> Result<Value, AdmissionRecord> {
    let refuse = |cause: &'static str| admission_record(Code::InvalidRuntimeInput, cause);
    let (Some(type_identity), Some(case)) = (variant.type_identity(), variant.case()) else {
        let member = if variant.type_identity().is_none() {
            "type"
        } else {
            "case"
        };
        return Err(refuse("missing-member").with("member", member));
    };
    if variant.has_extra_member() {
        return Err(refuse("unknown-member"));
    }
    let Some(declared) = references
        .variant_type(type_identity)
        .filter(|declared| &declared.shape() == slot)
    else {
        return Err(refuse("wrong-value-kind"));
    };
    declared
        .cases()
        .iter()
        .enumerate()
        .find(|(_, declared_case)| declared_case.name() == case)
        .and_then(|(rank, declared_case)| {
            Some(Value::Enum(quire_exact::EnumMember::new(
                declared_case.variant(),
                u32::try_from(rank).ok()?,
            )))
        })
        .ok_or_else(|| refuse("invalid-value"))
}

/// One unit of the explicit work stack [`admit_value`] drains.
enum Admit<'w, 'v> {
    /// Admit `form` as a value of `value_type`. `field` is set for a record
    /// field, whose kind is judged first (FR-106 check 6.5).
    Value {
        form: RawForm<'v>,
        value_type: &'w quire_exact::ValueType,
        presence: quire_exact::Presence,
        field: bool,
    },
    /// Descend one step of the value path.
    Enter(PathStep),
    /// Leave the step last entered, moving the post-state range breaches it
    /// recorded under it.
    Leave,
    /// Build the record of declaration `own` from its `count` admitted
    /// fields.
    Record {
        own: quire_exact::NodeKey,
        count: usize,
    },
    /// Wrap the admitted payload as a present value of the option type
    /// whose payload type is `payload`.
    SomeOf { payload: &'w quire_exact::ValueType },
    /// Build the sequence of type `collection` from its `count` admitted
    /// elements.
    Sequence {
        collection: &'w quire_exact::CollectionType,
        count: usize,
    },
}

/// FR-106 check 6.5 value validation, on an explicit work stack and result
/// stack, so a value of any nesting depth admits in constant native stack
/// and is bounded only by the document's byte and value limits (FR-106
/// Behavior; FR-261): a form is admitted as a value of `value_type`, with
/// `presence` deciding whether `absent`/`present` is expected. A record is
/// resolved to the record value type its `type` names, which must conform
/// to the declared record type; its fields are admitted in effective field
/// order, a sequence's elements by ascending index. A refusal names the
/// typed path of steps (record field, option value, sequence index) from the
/// checked field down to the value.
fn admit_value<'a: 'w, 'w>(
    references: &mut References<'a>,
    form: RawForm<'_>,
    value_type: &'w quire_exact::ValueType,
    presence: quire_exact::Presence,
    mut witness: Option<&mut Vec<RangeBreach>>,
) -> Result<FieldValue, AdmissionRecord> {
    let types: &'w TypeEnvironment = references.types;
    let mut work = vec![Admit::Value {
        form,
        value_type,
        presence,
        field: false,
    }];
    let mut done: Vec<FieldValue> = Vec::new();
    let mut path: Vec<PathStep> = Vec::new();
    // The breach count when each entered step was entered.
    let mut marks: Vec<usize> = Vec::new();
    while let Some(next) = work.pop() {
        let refuse_at = |cause: &'static str, path: &[PathStep]| {
            admission_record(Code::InvalidRuntimeInput, cause).at(path.to_vec())
        };
        match next {
            Admit::Enter(step) => {
                path.push(step);
                marks.push(witness.as_deref().map_or(0, Vec::len));
            }
            Admit::Leave => {
                let (Some(step), Some(first)) = (path.pop(), marks.pop()) else {
                    return Err(refuse_at("wrong-value-kind", &path));
                };
                if let Some(found) = witness.as_deref_mut() {
                    for breach in &mut found[first..] {
                        match &step {
                            // The element of the checked sequence field.
                            PathStep::Index(index) if path.is_empty() => {
                                breach.index = Some(*index);
                            }
                            _ => breach.steps.insert(0, step.clone()),
                        }
                    }
                }
            }
            Admit::Record { own, count } => {
                let Some(start) = done.len().checked_sub(count) else {
                    return Err(refuse_at("wrong-value-kind", &path));
                };
                let slots: Vec<FieldValue> = done.split_off(start);
                done.push(FieldValue::Present(quire_exact::from_admitted_slots(
                    own,
                    slots.into_boxed_slice(),
                )));
            }
            Admit::SomeOf { payload } => {
                let Some(FieldValue::Present(value)) = done.pop() else {
                    return Err(refuse_at("wrong-value-kind", &path));
                };
                done.push(FieldValue::Present(
                    quire_exact::OptionValue::from_admitted(payload.clone(), Some(value)),
                ));
            }
            Admit::Sequence { collection, count } => {
                if collection.bound().is_some_and(|bound| {
                    bound.violation(quire_exact::length_amount(count)).is_some()
                }) {
                    return Err(refuse_at("invalid-value", &path));
                }
                let Some(start) = done.len().checked_sub(count) else {
                    return Err(refuse_at("wrong-value-kind", &path));
                };
                let elements: Vec<Value> = done
                    .split_off(start)
                    .into_iter()
                    .filter_map(|element| match element {
                        FieldValue::Present(value) => Some(value),
                        FieldValue::Absent | FieldValue::Null => None,
                    })
                    .collect();
                done.push(FieldValue::Present(quire_exact::from_admitted(
                    collection.clone(),
                    elements,
                )));
            }
            Admit::Value {
                form,
                value_type,
                presence,
                field,
            } => {
                if field && !object_field_kind_matches(form, value_type, presence) {
                    return Err(refuse_at("wrong-value-kind", &path));
                }
                if presence == quire_exact::Presence::Optional {
                    match form {
                        RawForm::Absent => done.push(FieldValue::Absent),
                        // A present value is admitted as a required one of
                        // the same type, one option-value step down.
                        RawForm::Present(inner) => {
                            work.push(Admit::Leave);
                            work.push(Admit::Value {
                                form: inner.form(),
                                value_type,
                                presence: quire_exact::Presence::Required,
                                field: false,
                            });
                            work.push(Admit::Enter(PathStep::OptionValue));
                        }
                        // The native order refuses an envelope defect before
                        // the slot's fit.
                        RawForm::Native(native) => {
                            return Err(match native_payload(native) {
                                Err(record) => record.at(path.clone()),
                                Ok(_) => refuse_at("wrong-value-kind", &path),
                            });
                        }
                        _ => return Err(refuse_at("wrong-value-kind", &path)),
                    }
                    continue;
                }
                match (form, value_type) {
                    // A genuinely `Option`-typed value (an operation
                    // parameter or result).
                    (RawForm::Absent, quire_exact::ValueType::Option(payload)) => {
                        done.push(FieldValue::Present(quire_exact::OptionValue::none(
                            (**payload).clone(),
                        )));
                    }
                    (RawForm::Present(inner), quire_exact::ValueType::Option(payload)) => {
                        let payload: &'w quire_exact::ValueType = payload;
                        work.push(Admit::SomeOf { payload });
                        work.push(Admit::Leave);
                        work.push(Admit::Value {
                            form: inner.form(),
                            value_type: payload,
                            presence: quire_exact::Presence::Required,
                            field: false,
                        });
                        work.push(Admit::Enter(PathStep::OptionValue));
                    }
                    (RawForm::Sequence(items), quire_exact::ValueType::Collection(collection)) => {
                        if collection.kind() != CollectionKind::Sequence {
                            return Err(admission_record(
                                Code::UnknownRequiredFeature,
                                "unsupported-feature",
                            )
                            .at(path.clone()));
                        }
                        let collection: &'w quire_exact::CollectionType = collection;
                        work.push(Admit::Sequence {
                            collection,
                            count: items.iter().len(),
                        });
                        for (index, item) in items.iter().enumerate().rev() {
                            work.push(Admit::Leave);
                            work.push(Admit::Value {
                                form: item.form(),
                                value_type: collection.element(),
                                presence: quire_exact::Presence::Required,
                                field: false,
                            });
                            work.push(Admit::Enter(PathStep::Index(index)));
                        }
                    }
                    (RawForm::Record(record), quire_exact::ValueType::Composite(slot)) => {
                        let Some(own) = references.record_type(record.type_identity()) else {
                            return Err(refuse_at("wrong-value-kind", &path));
                        };
                        if !types.composite_conforms(own, *slot) {
                            return Err(refuse_at("wrong-value-kind", &path));
                        }
                        let Some(CompositeShape::Record(declared)) =
                            types.composite(own).map(|declaration| declaration.shape())
                        else {
                            return Err(refuse_at("wrong-value-kind", &path));
                        };
                        let nested = |name: &str| {
                            let mut nested = path.clone();
                            nested.push(PathStep::Field(name.to_owned()));
                            nested
                        };
                        if let Some(missing) = declared
                            .iter()
                            .find(|field| !record.fields().any(|(name, _)| name == field.name()))
                        {
                            return Err(admission_record(
                                Code::InvalidRuntimeInput,
                                "missing-member",
                            )
                            .at(nested(missing.name())));
                        }
                        if let Some((extra, _)) = record
                            .fields()
                            .find(|(name, _)| !declared.iter().any(|field| field.name() == *name))
                        {
                            return Err(admission_record(
                                Code::InvalidRuntimeInput,
                                "unknown-member",
                            )
                            .at(nested(extra)));
                        }
                        work.push(Admit::Record {
                            own,
                            count: declared.len(),
                        });
                        for field in declared.iter().rev() {
                            let Some((_, raw)) =
                                record.fields().find(|(name, _)| *name == field.name())
                            else {
                                return Err(refuse_at("wrong-value-kind", &nested(field.name())));
                            };
                            work.push(Admit::Leave);
                            work.push(Admit::Value {
                                form: raw.form(),
                                value_type: field.value_type(),
                                presence: field.presence(),
                                field: true,
                            });
                            work.push(Admit::Enter(PathStep::Field(field.name().to_owned())));
                        }
                    }
                    (form, value_type) => {
                        let value =
                            admit_leaf(references, form, value_type, witness.as_deref_mut())
                                .map_err(|record| record.at(path.clone()))?;
                        done.push(FieldValue::Present(value));
                    }
                }
            }
        }
    }
    done.pop()
        .ok_or_else(|| admission_record(Code::InvalidRuntimeInput, "wrong-value-kind"))
}

/// Every `(population, key)` a reference value inside `raw` names (a
/// reference can sit under `present` or inside a `sequence`), appended to
/// `into` in the value's own walk order. The walk keeps its own heap stack,
/// so a value of any depth walks in constant native stack.
fn collect_references(raw: &SnapshotValue, into: &mut Vec<(String, String)>) {
    let mut pending = vec![raw.form()];
    while let Some(form) = pending.pop() {
        match form {
            RawForm::Reference(reference) => {
                into.push((reference.population.clone(), reference.key.clone()));
            }
            RawForm::Present(inner) => pending.push(inner.form()),
            // Pushed last-first, so the first item walks first.
            RawForm::Sequence(items) => {
                pending.extend(items.iter().rev().map(RawValue::form));
            }
            RawForm::Record(record) => {
                pending.extend(record.fields().rev().map(|(_, value)| value.form()));
            }
            RawForm::Boolean(_)
            | RawForm::Integer(_)
            | RawForm::Native(_)
            | RawForm::Absent
            | RawForm::Variant(_) => {}
        }
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
    /// The entries of the snapshot's `relationships` member that passed
    /// checks 6.6 to 6.9 (FR-364).
    pub(super) extents: Vec<super::relationships::CheckedExtent>,
    /// The package facts the extent store is built from.
    facts: super::relationships::ModelFacts,
    /// Each reference that named no object of this snapshot, with the wire
    /// population it named ([`References`]).
    unresolved: Vec<(String, ObjectReference)>,
    /// Each post-state integer admitted outside its declared range, in
    /// walk order.
    out_of_range: Vec<OutOfRange>,
}

/// FR-106 check 6: admit every population's every object's every field, in
/// document order. `types` is the checked package's own effective attribute
/// set (`CheckedGraph::scope().types()`); `views` are the re-derived domain
/// package views (population declarations and identity strings, `EffectiveId`
/// conformance). The object and value ceilings (FR-106 Inputs) are not
/// charged here: the read charged them as it walked the document
/// (FR-261 B4).
pub(super) fn admit_population_values<'t>(
    views: &[ModelView],
    types: &'t TypeEnvironment,
    populations: &[RawPopulation],
    relationships: &[super::relationships::RawEntry],
    range: PostStateRange,
    ancestor_steps: u64,
) -> Result<PopulationValues<'t>, AdmissionFailure> {
    let mut objects: Vec<(ObjectReference, Vec<(&str, FieldValue)>)> = Vec::new();
    let mut listed = super::relationships::Listed::new();
    let mut completeness = BTreeMap::new();
    let mut keys_by_population: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut references = References::over(views, types, populations);
    let mut out_of_range: Vec<OutOfRange> = Vec::new();

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
            // 6.1: type/population membership, before anything else in this
            // check (SR-750 FND-005 round 2's own numbered order).
            let Some((_, object_key)) = find_declaration(views, object.type_identity.as_str())
            else {
                return Err(refuse(
                    admission_record(Code::InvalidRuntimeInput, "wrong-role-mapping")
                        .with("object", object.key.clone()),
                ));
            };
            // A reached `ancestor_steps` ceiling is the stage's limit
            // outcome, never a "does not conform" verdict.
            let mut covered = false;
            for member in &population_info.member_types {
                match population_view.view.model_index().conforms(
                    &object_key,
                    member,
                    ancestor_steps,
                    qsl_foundation::Setting::ModelAncestorSteps,
                ) {
                    Ok(true) => {
                        covered = true;
                        break;
                    }
                    Ok(false) => {}
                    Err(refusal) => {
                        return Err(match refusal.cause.denial() {
                            Some(denial) => super::caller_budget_refusal(
                                &denial,
                                refusal.cause.catalog_code().cause(),
                            ),
                            None => fault("conformance-walk-refused"),
                        });
                    }
                }
            }
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
            let mut breaches: Vec<(FieldRef, RangeBreach)> = Vec::new();
            for attribute in declared {
                let name = attribute.field().name();
                let value_type = attribute.field().value_type();
                let presence = attribute.field().presence();
                // 6.4 above already confirmed every declared field has a
                // raw value here.
                let raw = raw_field(&object.fields, name)
                    .ok_or_else(|| fault("declared-field-missing-after-check-6-4"))?;
                if !object_field_kind_matches(raw.form(), value_type, presence) {
                    return Err(refuse(
                        admission_record(Code::InvalidRuntimeInput, "wrong-value-kind")
                            .with("object", object.key.clone())
                            .with("field", name.to_owned()),
                    ));
                }
                let mut found = Vec::new();
                let witness = match range {
                    PostStateRange::Witness => Some(&mut found),
                    PostStateRange::Refuse => None,
                };
                let field_value =
                    admit_object_field(&mut references, raw, value_type, presence, witness)
                        .map_err(|record| {
                            refuse(
                                record
                                    .with("object", object.key.clone())
                                    .with("field", name.to_owned()),
                            )
                        })?;
                breaches.extend(
                    found
                        .into_iter()
                        .map(|breach| (attribute.identity(), breach)),
                );
                attributes.push((name, field_value));
            }

            // `object_reference` already refuses an empty key with FR-106's
            // own `invalid-value` (SR-750 FND-004 round 2): propagated
            // directly, never re-mapped to a `Fault`.
            let reference = object_reference(views, effective_type, &object.key)?;
            if !listed.contains_key(&entry.population) {
                listed.insert(entry.population.clone(), BTreeMap::new());
            }
            if let Some(objects) = listed.get_mut(&entry.population) {
                objects.insert(object.key.clone(), reference.clone());
            }
            out_of_range.extend(breaches.into_iter().map(|(field, breach)| OutOfRange {
                object: reference.clone(),
                field,
                index: breach.index,
                range: breach.range,
                observed: breach.observed,
                steps: breach.steps,
            }));
            objects.push((reference, attributes));
        }
    }

    // 6.6 to 6.9: the `relationships` entries, after the last object.
    let facts = super::relationships::model_facts(views, types);
    let extents = super::relationships::check_entries(types, &facts, relationships, &listed)?;

    Ok(PopulationValues {
        objects,
        completeness,
        keys_by_population,
        extents,
        facts,
        unresolved: references.unresolved,
        out_of_range,
    })
}

/// Admit one state-machine context alongside any ordinary declared
/// populations in the same snapshot. A machine context is owned by its
/// observation and therefore receives an internal observation-scoped
/// reference only for evaluator lookup; that reference is never exposed as
/// a declared-object path subject.
#[allow(clippy::too_many_arguments)]
pub(super) fn admit_state_machine_context_values<'t>(
    views: &[ModelView],
    types: &'t TypeEnvironment,
    context_view: &ModelView,
    context_name: &str,
    populations: &[RawPopulation],
    relationships: &[super::relationships::RawEntry],
    observation: &DocumentRef,
    invocation: &DocumentRef,
    range: PostStateRange,
    ancestor_steps: u64,
) -> Result<(PopulationValues<'t>, ObjectReference), AdmissionFailure> {
    let mut contexts = Vec::new();
    let mut ordinary = Vec::new();
    for (population_index, population) in populations.iter().enumerate() {
        if population.population == context_name {
            for (object_index, object) in population.objects.iter().enumerate() {
                contexts.push((population_index, object_index, object));
            }
        } else {
            ordinary.push(population.clone());
        }
    }

    let required = format!("({context_name}, {})", invocation.identity);
    let Some((_, _, context)) = contexts.first().copied() else {
        return Err(refuse(
            admission_record(Code::InvalidModelBinding, "wrong-context")
                .with("document", observation.identity.clone())
                .with("required", required)
                .with("actual", "none"),
        ));
    };
    if let Some((population_index, object_index, _)) = contexts.get(1).copied() {
        return Err(refuse(
            admission_record(Code::InvalidRuntimeInput, "duplicate-member")
                .with("document", observation.identity.clone())
                .with(
                    "path",
                    format!("/populations/{population_index}/objects/{object_index}"),
                ),
        ));
    }

    let context_key = context_view
        .view
        .type_identities()
        .keys()
        .find(|key| key.node == context_name)
        .ok_or_else(|| fault("context-type-unresolved"))?;
    if context.type_identity.as_str() != context_name {
        return Err(refuse(
            admission_record(Code::InvalidModelBinding, "wrong-context")
                .with("document", observation.identity.clone())
                .with("required", required)
                .with("actual", context.type_identity.as_str().to_owned()),
        ));
    }
    let effective_type = *context_view
        .view
        .type_identities()
        .get(context_key)
        .ok_or_else(|| fault("context-type-has-no-effective-id"))?;
    let declared = types
        .attributes(effective_type)
        .ok_or_else(|| fault("context-type-has-no-checked-attributes"))?;

    for attribute in declared {
        if let quire_exact::ValueType::Collection(collection) = attribute.field().value_type() {
            if collection.kind() != CollectionKind::Sequence {
                return Err(refuse(
                    admission_record(Code::UnknownRequiredFeature, "unsupported-feature")
                        .with("field", attribute.field().name().to_owned()),
                ));
            }
        }
    }
    for attribute in declared {
        let name = attribute.field().name();
        if raw_field(&context.fields, name).is_none() {
            return Err(refuse(
                admission_record(Code::InvalidRuntimeInput, "missing-member")
                    .with("document", observation.identity.clone())
                    .with("field", name.to_owned()),
            ));
        }
    }
    for (name, _) in &context.fields {
        if !declared
            .iter()
            .any(|attribute| attribute.field().name() == name)
        {
            return Err(refuse(
                admission_record(Code::InvalidRuntimeInput, "unknown-member")
                    .with("document", observation.identity.clone())
                    .with("field", name.clone()),
            ));
        }
    }

    let mut references = References::over(views, types, &ordinary);
    let mut attributes = Vec::with_capacity(declared.len());
    let mut breaches = Vec::new();
    for attribute in declared {
        let name = attribute.field().name();
        let value_type = attribute.field().value_type();
        let presence = attribute.field().presence();
        let raw = raw_field(&context.fields, name)
            .ok_or_else(|| fault("context-field-missing-after-check"))?;
        if !object_field_kind_matches(raw.form(), value_type, presence) {
            return Err(refuse(
                admission_record(Code::InvalidRuntimeInput, "wrong-value-kind")
                    .with("document", observation.identity.clone())
                    .with("field", name.to_owned()),
            ));
        }
        let mut found = Vec::new();
        let witness = match range {
            PostStateRange::Witness => Some(&mut found),
            PostStateRange::Refuse => None,
        };
        let value = admit_object_field(&mut references, raw, value_type, presence, witness)
            .map_err(|record| {
                refuse(
                    record
                        .with("document", observation.identity.clone())
                        .with("field", name.to_owned()),
                )
            })?;
        breaches.extend(
            found
                .into_iter()
                .map(|breach| (attribute.identity(), breach)),
        );
        attributes.push((name, value));
    }

    let object = ObjectId::new(invocation.identity.clone()).map_err(|_| {
        refuse(
            admission_record(Code::InvalidRuntimeInput, "invalid-value").with("field", "identity"),
        )
    })?;
    let reference = ObjectReference::new(
        UniverseId::from_digest(observation.digest),
        effective_type,
        object,
    );
    let mut values = admit_population_values(
        views,
        types,
        &ordinary,
        relationships,
        range,
        ancestor_steps,
    )?;
    values
        .out_of_range
        .extend(breaches.into_iter().map(|(field, breach)| OutOfRange {
            object: reference.clone(),
            field,
            index: breach.index,
            range: breach.range,
            observed: breach.observed,
            steps: breach.steps,
        }));
    values.objects.push((reference.clone(), attributes));
    Ok((values, reference))
}

/// FR-106's required populations from parameters: every population of the
/// package that a reference inside the raw value of one of `operation`'s
/// declared reference-valued parameters names, in declared parameter order
/// and then the value's walk order, once each. Read from the raw values
/// because check 7 runs before check 10 admits them. An undeclared or
/// non-reference parameter makes no population required: check 10 refuses
/// it. A reference naming no declared population is left out: check 10
/// refuses it `dangling_reference`.
pub(super) fn parameter_populations(
    views: &[ModelView],
    operation: &OperationDeclaration,
    parameters: &[(String, SnapshotValue)],
) -> Vec<String> {
    let mut named = Vec::new();
    for (name, value_type) in operation.parameters() {
        if !holds_references(value_type) {
            continue;
        }
        let Some(raw) = raw_field(parameters, name) else {
            continue;
        };
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

/// Whether a value of `value_type` can hold an object reference: a
/// reference, or an option or collection of one.
fn holds_references(value_type: &quire_exact::ValueType) -> bool {
    // A loop down the option/collection chain: any depth, constant stack.
    let mut current = value_type;
    loop {
        match current {
            quire_exact::ValueType::Reference(_) => return true,
            quire_exact::ValueType::Option(inner) => current = inner,
            quire_exact::ValueType::Collection(collection) => current = collection.element(),
            quire_exact::ValueType::Boolean
            | quire_exact::ValueType::Integer
            | quire_exact::ValueType::Int(_)
            | quire_exact::ValueType::Rational(_)
            | quire_exact::ValueType::Decimal(_)
            | quire_exact::ValueType::Float(_)
            | quire_exact::ValueType::Quantity(_)
            | quire_exact::ValueType::Text(_)
            | quire_exact::ValueType::String
            | quire_exact::ValueType::Enum(_)
            | quire_exact::ValueType::Composite(_)
            | quire_exact::ValueType::Population(_)
            | quire_exact::ValueType::Uuid
            | quire_exact::ValueType::Timestamp => return false,
        }
    }
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
    extents: &[super::relationships::CheckedExtent],
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
        // An instance end makes the population of its other end required.
        grew |= super::relationships::extend_required(&mut required, extents);
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
    // A declared population only an instance end made required follows them
    // all (check 6.8 admitted only declared populations at instance ends).
    for name in super::relationships::end_populations(extents) {
        if required.contains(name) && !walk_order.contains(&name) {
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
    super::relationships::check_unlisted_ends(extents, completeness)
}

/// FR-106 check 8: closure, skipped over an incomplete population, over
/// every reference value of every object in document walk order (not gated
/// by "required": FR-106's own text names no such restriction for this
/// check, only for check 7's completeness requirement).
pub(super) fn check_population_closure(
    populations: &[RawPopulation],
    completeness: &BTreeMap<String, bool>,
    keys_by_population: &BTreeMap<String, BTreeSet<String>>,
    extents: &[super::relationships::CheckedExtent],
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
    // After the `reference` values: the instance ends.
    super::relationships::check_dangling_ends(extents)
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
    let out_of_range = values.out_of_range;
    // The closure admits each slot against its declared type, which an
    // out-of-range post-state integer breaks by definition. Such a closure
    // is built over the declarations with `Int[lo, hi]` widened to
    // `Integer`; evaluation still resolves fields through the checked
    // package's own `types`, and reads the exact slots.
    let widened;
    let closure_types = if out_of_range.is_empty() {
        types
    } else {
        widened = widen_ranges(types)?;
        &widened
    };
    let extents = super::relationships::build_extents(
        types,
        values.facts,
        &values.extents,
        &values.completeness,
    )?;
    let environment = ObjectEnvironment::new(
        ObjectClosure::new(closure_types, values.objects, &tolerated)
            .map_err(map_environment_refusal)?,
    )
    .with_out_of_range(out_of_range.clone())
    .with_extents(extents);
    Ok(AdmittedEnvironment {
        environment,
        completeness: values.completeness,
        out_of_range,
    })
}

/// `types` with every object field declared `Int[lo, hi]` declared
/// `Integer` instead, everything else unchanged: the declaration order,
/// presence, redefinitions, supertypes, composites and units all carry
/// over, so a slot position means the same field in either environment.
fn widen_ranges(types: &TypeEnvironment) -> Result<TypeEnvironment, AdmissionFailure> {
    let object_types = types.object_types().map(|declaration| {
        let attributes = declaration
            .attributes()
            .iter()
            .map(|field| {
                let value_type = match field.value_type() {
                    quire_exact::ValueType::Int(_) => quire_exact::ValueType::Integer,
                    other => other.clone(),
                };
                let widened = FieldDeclaration::new(field.name(), value_type, field.presence());
                match field.redefines() {
                    Some(target) => widened.with_redefines(target.clone()),
                    None => widened,
                }
            })
            .collect();
        ObjectTypeDeclaration::new(declaration.key(), declaration.name(), attributes)
            .with_supertypes(declaration.supertypes().to_vec())
    });
    TypeEnvironment::new(types.composites().cloned(), object_types)
        .map(|widened| widened.with_units(types.units().clone()))
        .map_err(|_| fault("widened-type-environment-refused"))
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
    relationships: &[super::relationships::RawEntry],
    self_population: Option<&str>,
    ancestor_steps: u64,
) -> Result<AdmittedEnvironment, AdmissionFailure> {
    let values = admit_population_values(
        views,
        types,
        populations,
        relationships,
        PostStateRange::Refuse,
        ancestor_steps,
    )?;
    check_population_completeness(
        populations,
        &values.completeness,
        self_population,
        &[],
        &values.extents,
    )?;
    check_population_closure(
        populations,
        &values.completeness,
        &values.keys_by_population,
        &values.extents,
    )?;
    finish_populations(types, values)
}

fn map_environment_refusal(error: ObjectClosureRefusal) -> AdmissionFailure {
    match error.cause {
        ObjectClosureCause::DanglingReference(target) => refuse(
            admission_record(
                Code::DanglingReference,
                "absent-target-in-complete-population",
            )
            .with("object", target.object().as_str().to_owned()),
        ),
        ObjectClosureCause::DuplicateObject
        | ObjectClosureCause::UnknownObjectType
        | ObjectClosureCause::Attribute(_) => {
            fault("object-environment-refused-after-admission-checks")
        }
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
/// shares a universe, `ObjectClosure::find`'s own doc comment), and
/// `self_object.key` alone -- not a caller-narrowed type -- names the
/// object within it, the same way FR-109's `Function`-selection object
/// argument does (`ObjectClosure::find`, `population_universe`).
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
        .objects()
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
        let value = admit_form(references, raw.form(), value_type, None)
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
            let value = admit_form(references, raw.form(), value_type, None)
                .map_err(|record| refuse(record.with("field", "result")))?;
            references.check_closure(raw).map_err(refuse)?;
            Ok(Some(value))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// The value form `text` spells, read as the admission reads it.
    fn snapshot(text: &str) -> SnapshotValue {
        let document =
            quire_canonical::read(text.as_bytes(), u64::MAX).expect("test fixture is JSON");
        read_raw_value(Json::of(document.root()), &mut || Ok::<(), ()>(()))
            .expect("an uncharged read does not fail")
            .expect("test fixture is a value form")
    }

    /// SR-750 FND-002: a non-ASCII digest string must refuse, never panic
    /// on a byte-index char-boundary slice. `read_document_ref` is
    /// `hex_bytes`'s one caller reachable with untrusted input (a `pre`/
    /// `post` document reference's own digest).
    #[test]
    fn a_non_ascii_digest_refuses_rather_than_panics() {
        let text = serde_json::json!({
            "identity": {
                "authority": "a",
                "identity": "b",
                "revision_namespace": "c",
                "revision": "d",
            },
            "digest": "sha256-jcs:aé00000000000000000000000000000000000000000000000000000000000",
        })
        .to_string();
        let document =
            quire_canonical::read(text.as_bytes(), u64::MAX).expect("test fixture is JSON");
        let result = read_document_ref(Json::of(document.root()), "pre");
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
        let raw = SnapshotValue::sequence(vec![
            SnapshotValue::present(reference("p1", "k1")),
            reference("p2", "k2"),
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
        collect_references(&SnapshotValue::boolean(true), &mut found);
        collect_references(&SnapshotValue::absent(), &mut found);
        collect_references(&SnapshotValue::integer("1"), &mut found);
        assert!(found.is_empty());
    }

    fn reference(population: &str, key: &str) -> SnapshotValue {
        SnapshotValue::reference(SelectedObject {
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
        let types = TypeEnvironment::new([], []).expect("an empty environment admits");
        let references = References::over(&[], &types, &populations);

        let record = references
            .check_closure(&SnapshotValue::present(reference("complete", "k")))
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
            references.check_closure(&SnapshotValue::sequence(vec![reference("partial", "k")])),
            Ok(())
        );
        assert_eq!(
            references.check_closure(&SnapshotValue::boolean(true)),
            Ok(())
        );
    }
    /// The integer an admitted scalar holds.
    fn integer_of(admitted: Result<Value, AdmissionRecord>) -> Integer {
        match admitted {
            Ok(Value::Integer(integer)) => integer,
            other => panic!("expected an admitted integer, got {other:?}"),
        }
    }

    fn range_0_1000() -> quire_exact::ValueType {
        quire_exact::ValueType::Int(
            quire_exact::IntegerInterval::new(Integer::from(0_i64), Integer::from(1000_i64))
                .expect("a non-empty interval"),
        )
    }

    /// FR-106-AC-12: an operation argument (admitted with no witness sink)
    /// outside its declared `Int[0, 1000]` is refused `invalid-value`, a
    /// value in range is admitted.
    #[trace("TC-465", "FR-106-AC-12")]
    #[test]
    fn an_out_of_range_integer_with_no_witness_sink_refuses() {
        let types = TypeEnvironment::new([], []).expect("an empty environment admits");
        let mut references = References::over(&[], &types, &[]);
        let record = admit_form(
            &mut references,
            snapshot(r#"{"integer":"-1"}"#).form(),
            &range_0_1000(),
            None,
        )
        .expect_err("-1 is outside [0, 1000]");
        assert_eq!(record.code, Code::InvalidRuntimeInput);
        assert_eq!(record.cause, "invalid-value");
        let admitted = admit_form(
            &mut references,
            snapshot(r#"{"integer":"1000"}"#).form(),
            &range_0_1000(),
            None,
        );
        assert_eq!(integer_of(admitted), Integer::from(1000_i64));
    }

    /// FR-106-AC-12: with a witness sink the same integer is admitted
    /// exactly (never clamped) and recorded with its declared range; an
    /// in-range one records nothing.
    #[trace("TC-465", "FR-106-AC-12")]
    #[test]
    fn an_out_of_range_integer_with_a_witness_sink_is_admitted_exactly() {
        let types = TypeEnvironment::new([], []).expect("an empty environment admits");
        let mut references = References::over(&[], &types, &[]);
        let mut found = Vec::new();
        let admitted = admit_form(
            &mut references,
            snapshot(r#"{"integer":"1001"}"#).form(),
            &range_0_1000(),
            Some(&mut found),
        );
        assert_eq!(integer_of(admitted), Integer::from(1001_i64));
        assert_eq!(found.len(), 1);
        let breach = &found[0];
        assert_eq!(breach.range.lower(), &Integer::from(0_i64));
        assert_eq!(breach.range.upper(), &Integer::from(1000_i64));
        assert_eq!(breach.observed, Integer::from(1001_i64));
        assert_eq!(breach.index, None);
        let in_range = admit_form(
            &mut references,
            snapshot(r#"{"integer":"7"}"#).form(),
            &range_0_1000(),
            Some(&mut found),
        );
        assert_eq!(integer_of(in_range), Integer::from(7_i64));
        assert_eq!(found.len(), 1);
    }
    /// FR-106-AC-12: a post-state sequence field's out-of-range element is
    /// admitted exactly and reported at its index under a witness sink,
    /// and refused without one.
    #[trace("TC-465", "FR-106-AC-12")]
    #[test]
    fn an_out_of_range_sequence_element_is_witnessed_at_its_index() {
        let types = TypeEnvironment::new([], []).expect("an empty environment admits");
        let mut references = References::over(&[], &types, &[]);
        let sequence = quire_exact::ValueType::collection(quire_exact::CollectionType::new(
            CollectionKind::Sequence,
            range_0_1000(),
            None,
        ));
        let raw = snapshot(r#"{"sequence":[{"integer":"5"},{"integer":"1001"}]}"#);
        let mut found = Vec::new();
        admit_object_field(
            &mut references,
            &raw,
            &sequence,
            quire_exact::Presence::Required,
            Some(&mut found),
        )
        .expect("a witnessed sequence admits");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].index, Some(1));
        assert_eq!(found[0].observed, Integer::from(1001_i64));
        let record = admit_object_field(
            &mut references,
            &raw,
            &sequence,
            quire_exact::Presence::Required,
            None,
        )
        .expect_err("an unwitnessed sequence refuses");
        assert_eq!(record.cause, "invalid-value");
    }

    /// The raw value `text` spells, or `None` when it is not a value form.
    fn read(text: &str) -> Option<SnapshotValue> {
        let document =
            quire_canonical::read(text.as_bytes(), u64::MAX).expect("test fixture is JSON");
        read_raw_value(Json::of(document.root()), &mut || Ok::<(), ()>(()))
            .expect("an uncharged read does not fail")
    }

    /// FR-106 record form: the `record` tag reads its `type` and its fields
    /// in document order, nested records included, and equals the value built
    /// from its parts; a payload with any member besides `type` and `fields`,
    /// or either missing or of the wrong kind, is not a value form.
    #[trace("FR-106-AC-15", "FR-359-AC-9")]
    #[test]
    fn a_record_value_reads_by_its_type_and_fields_in_document_order() {
        let read_back = snapshot(
            r#"{"record":{"type":"ix://p/T","fields":{"b":{"integer":"2"},"a":{"record":{"type":"ix://p/U","fields":{}}}}}}"#,
        );
        let RawForm::Record(record) = read_back.form() else {
            panic!("a record");
        };
        assert_eq!(record.type_identity(), "ix://p/T");
        let names: Vec<&str> = record.fields().map(|(name, _)| name).collect();
        assert_eq!(names, ["b", "a"], "document order");
        assert_eq!(
            read_back,
            SnapshotValue::record(
                "ix://p/T",
                vec![
                    ("b".to_owned(), SnapshotValue::integer("2")),
                    (
                        "a".to_owned(),
                        SnapshotValue::record("ix://p/U", Vec::new())
                    ),
                ]
            )
        );
        for malformed in [
            r#"{"record":{"type":"ix://p/T"}}"#,
            r#"{"record":{"fields":{}}}"#,
            r#"{"record":{"type":"ix://p/T","fields":{},"extra":1}}"#,
            r#"{"record":{"type":7,"fields":{}}}"#,
            r#"{"record":{"type":"ix://p/T","fields":[]}}"#,
            r#"{"record":{"type":"ix://p/T","fields":{"a":{"nope":1}}}}"#,
            r#"{"record":"ix://p/T"}"#,
        ] {
            assert!(read(malformed).is_none(), "{malformed}");
        }
    }

    /// FR-360 item 9: the `variant` tag keeps the members of its payload as
    /// written, so a missing or an extra one is judged by admission in its
    /// own order; a payload that is not an object, or a `type` or `case` that
    /// is not text, is not a value form.
    #[trace("FR-360-AC-9", "FR-360-AC-10")]
    #[test]
    fn a_variant_value_reads_its_members_as_written() {
        let read_back = snapshot(r#"{"variant":{"case":"Gold","type":"ix://p/Tier"}}"#);
        let RawForm::Variant(variant) = read_back.form() else {
            panic!("a variant");
        };
        assert_eq!(variant.type_identity(), Some("ix://p/Tier"));
        assert_eq!(variant.case(), Some("Gold"));
        assert!(!variant.has_extra_member());
        assert_eq!(
            read_back,
            SnapshotValue::variant("ix://p/Tier", "Gold"),
            "member order does not change the value"
        );
        let partial = snapshot(r#"{"variant":{"x":1}}"#);
        let RawForm::Variant(variant) = partial.form() else {
            panic!("a variant");
        };
        assert_eq!((variant.type_identity(), variant.case()), (None, None));
        assert!(variant.has_extra_member());
        for malformed in [
            r#"{"variant":"Gold"}"#,
            r#"{"variant":{"type":7,"case":"Gold"}}"#,
            r#"{"variant":{"type":"ix://p/Tier","case":null}}"#,
        ] {
            assert!(read(malformed).is_none(), "{malformed}");
        }
    }

    /// A reference inside a record is found by the closure walk.
    #[trace("FR-106-AC-15", "FR-359-AC-9")]
    #[test]
    fn collect_references_walks_into_a_record() {
        let raw = snapshot(
            r#"{"record":{"type":"ix://p/T","fields":{"a":{"reference":{"population":"p","key":"k"}}}}}"#,
        );
        let mut found = Vec::new();
        collect_references(&raw, &mut found);
        assert_eq!(found, [("p".to_owned(), "k".to_owned())]);
    }

    /// A `Text[min, max; profile]` slot.
    fn text_slot(min: u64, max: u64, profile: quire_exact::TextProfile) -> quire_exact::ValueType {
        quire_exact::ValueType::Text(
            quire_exact::TextType::new(min, max, profile).expect("the bounds are not empty"),
        )
    }

    fn title() -> quire_exact::ValueType {
        text_slot(1, 8, quire_exact::TextProfile::Nfc)
    }

    fn tag() -> quire_exact::ValueType {
        text_slot(0, 4, quire_exact::TextProfile::UnicodeScalars)
    }

    fn code() -> quire_exact::ValueType {
        text_slot(0, 4, quire_exact::TextProfile::BinaryUtf8)
    }

    /// The Text object of `text`, admitted into `slot`.
    fn admit_text_object(
        text: &str,
        slot: &quire_exact::ValueType,
    ) -> Result<Value, AdmissionRecord> {
        let object = snapshot(&serde_json::json!({"kind": "text", "text": text}).to_string());
        let RawForm::Native(native) = object.form() else {
            panic!("a native object");
        };
        admit_native(native, slot)
    }

    /// The retained sequence of an admitted Text value.
    fn retained(admitted: Result<Value, AdmissionRecord>) -> String {
        match admitted {
            Ok(Value::Text(text)) => text.retained().to_owned(),
            other => panic!("a Text value, not {other:?}"),
        }
    }

    /// TC-1038 step 2 (FR-106-AC-33): each text admits into its declared
    /// type; under `nfc` U+00E9 and U+0065 U+0301 retain one sequence and
    /// sixteen supplied scalars retain eight.
    #[trace("TC-1038", "FR-106-AC-33")]
    #[test]
    fn a_text_object_admits_into_a_text_slot_under_its_profile() {
        for text in ["a", "abcdefgh", "a;b>>>c", "\u{e9}", "\u{0}", "\u{1f600}"] {
            assert_eq!(retained(admit_text_object(text, &title())), text);
        }
        assert_eq!(retained(admit_text_object("e\u{301}", &title())), "\u{e9}");
        assert_eq!(
            retained(admit_text_object(&"e\u{301}".repeat(8), &title())),
            "\u{e9}".repeat(8)
        );
        // Unicode scalars and bytes retain what was supplied.
        assert_eq!(
            retained(admit_text_object(&"\u{e9}".repeat(4), &tag())),
            "\u{e9}".repeat(4)
        );
        assert_eq!(retained(admit_text_object("", &tag())), "");
        for text in ["abcd", "\u{e9}\u{e9}", "\u{1f600}"] {
            assert_eq!(retained(admit_text_object(text, &code())), text);
        }
    }

    /// TC-1038 step 4 (FR-106-AC-34): each object defect refuses in the
    /// payload order, the first defect naming its member.
    #[trace("TC-1038", "FR-106-AC-34")]
    #[test]
    fn a_defective_text_object_refuses_in_the_payload_order() {
        for (object, cause, member) in [
            (r#"{"kind":1}"#, "wrong-value-kind", Some("kind")),
            (r#"{"kind":null}"#, "wrong-value-kind", Some("kind")),
            (r#"{"kind":{}}"#, "wrong-value-kind", Some("kind")),
            (
                r#"{"kind":"Text","text":"x"}"#,
                "invalid-value",
                Some("kind"),
            ),
            (
                r#"{"kind":"TEXT","text":"x"}"#,
                "invalid-value",
                Some("kind"),
            ),
            (
                r#"{"kind":"blob","value":"x"}"#,
                "invalid-value",
                Some("kind"),
            ),
            (
                r#"{"kind":"blob","text":"x"}"#,
                "invalid-value",
                Some("kind"),
            ),
            (r#"{"kind":"text"}"#, "missing-member", Some("text")),
            (
                r#"{"kind":"text","nanoseconds":"0"}"#,
                "missing-member",
                Some("text"),
            ),
            (
                r#"{"kind":"text","text":"Ready","extra":1}"#,
                "unknown-member",
                Some("extra"),
            ),
            (
                r#"{"kind":"text","text":"Ready","nanoseconds":"0"}"#,
                "unknown-member",
                Some("nanoseconds"),
            ),
            (
                r#"{"kind":"text","text":5}"#,
                "wrong-value-kind",
                Some("text"),
            ),
            (
                r#"{"kind":"text","text":null}"#,
                "wrong-value-kind",
                Some("text"),
            ),
            (
                r#"{"kind":"text","text":true}"#,
                "wrong-value-kind",
                Some("text"),
            ),
            (
                r#"{"kind":"text","text":["x"]}"#,
                "wrong-value-kind",
                Some("text"),
            ),
            (
                r#"{"kind":"text","text":{}}"#,
                "wrong-value-kind",
                Some("text"),
            ),
            (
                r#"{"extra":1,"kind":"text","text":5}"#,
                "unknown-member",
                Some("extra"),
            ),
        ] {
            let object_value = snapshot(object);
            let RawForm::Native(native) = object_value.form() else {
                panic!("a native object: {object}");
            };
            let Err(record) = admit_native(native, &title()) else {
                panic!("a refusal: {object}");
            };
            assert_eq!(record.code, Code::InvalidRuntimeInput, "{object}");
            assert_eq!(record.cause, cause, "{object}");
            assert_eq!(
                record.fields.get("member").map(String::as_str),
                member,
                "{object}"
            );
        }
    }

    /// TC-1038 step 5 (FR-106-AC-35): a String object in a Text slot and a
    /// Text object in any other native slot refuse `wrong-value-kind`
    /// naming no member, and a Text object longer than the Text slot's
    /// maximum placed in a String slot refuses fit, not length.
    #[trace("TC-1038", "FR-106-AC-35")]
    #[test]
    fn text_and_string_objects_fit_only_their_own_slots() {
        let string_object = snapshot(r#"{"kind":"string","text":"Ready"}"#);
        let RawForm::Native(native) = string_object.form() else {
            panic!("a native object");
        };
        let refusal = admit_native(native, &title()).expect_err("a String fits no Text slot");
        assert_eq!(refusal.code, Code::InvalidRuntimeInput);
        assert_eq!(refusal.cause, "wrong-value-kind");
        assert!(!refusal.fields.contains_key("member"));
        for slot in [
            quire_exact::ValueType::String,
            quire_exact::ValueType::Uuid,
            quire_exact::ValueType::Timestamp,
            quire_exact::ValueType::Integer,
        ] {
            for text in ["Ready", "00112233-4455-6677-8899-aabbccddeeff", "aaaaaaaaa"] {
                let refusal = admit_text_object(text, &slot).expect_err("a Text fits no such slot");
                assert_eq!(refusal.cause, "wrong-value-kind", "{slot:?} {text}");
                assert!(!refusal.fields.contains_key("member"), "{slot:?} {text}");
            }
        }
        // A defect before type fit still wins: `text` 5 in a UUID slot.
        let broken = snapshot(r#"{"kind":"text","text":5}"#);
        let RawForm::Native(native) = broken.form() else {
            panic!("a native object");
        };
        let refusal = admit_native(native, &quire_exact::ValueType::Uuid).expect_err("refused");
        assert_eq!(refusal.cause, "wrong-value-kind");
        assert_eq!(
            refusal.fields.get("member").map(String::as_str),
            Some("text")
        );
    }

    /// TC-1038 step 6 (FR-106-AC-36): the length is the profile length of the
    /// retained sequence; a text outside the declared bounds refuses
    /// `invalid-value` naming `text`, the declared type and the measured
    /// length.
    #[trace("TC-1038", "FR-106-AC-36")]
    #[test]
    fn a_text_outside_its_declared_length_refuses_with_the_measured_length() {
        let long = "a".repeat(65_536);
        let pairs = |count: usize| "e\u{301}".repeat(count);
        for (text, slot, expected, measured) in [
            ("", title(), "Text[1, 8; nfc]", 0),
            ("abcdefghi", title(), "Text[1, 8; nfc]", 9),
            (pairs(9).as_str(), title(), "Text[1, 8; nfc]", 9),
            (long.as_str(), title(), "Text[1, 8; nfc]", 65_536),
            ("abcde", tag(), "Text[0, 4; unicode-scalars]", 5),
            (pairs(3).as_str(), tag(), "Text[0, 4; unicode-scalars]", 6),
            ("\u{e9}\u{e9}\u{e9}", code(), "Text[0, 4; binary-utf8]", 6),
            ("\u{1f600}\u{1f600}", code(), "Text[0, 4; binary-utf8]", 8),
        ] {
            let refusal = admit_text_object(text, &slot).expect_err("outside the bounds");
            assert_eq!(refusal.code, Code::InvalidRuntimeInput);
            assert_eq!(refusal.cause, "invalid-value");
            let field = |name: &str| refusal.fields.get(name).map(String::as_str);
            assert_eq!(field("member"), Some("text"));
            assert_eq!(field("expected"), Some(expected));
            assert_eq!(field("actual"), Some(measured.to_string().as_str()));
        }
    }
}
