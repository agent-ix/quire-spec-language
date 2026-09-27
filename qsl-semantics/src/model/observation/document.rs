// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-106 checks 1, 5 to 10: reading one snapshot or invocation document
//! (check 1), admitting its populations and objects (checks 5 to 8) and
//! resolving `self`, `parameters` and `result` (checks 9 and 10).

use std::collections::{BTreeMap, BTreeSet};

use quire_exact::{CollectionKind, FieldValue, Integer, ObjectReference, Value};

use super::helpers::{admission_record, object_reference};
use super::ordered_json::{OrderedJson, OrderedObject};
use super::{
    document_digest, fault, incomplete, read_raw_value, refuse, AdmissionFailure, AdmissionRecord,
    DocumentRef, ModelView, ObservationLimits, RawValue, SelectedObject,
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
    pub(super) fields: Vec<(String, RawValue)>,
}

#[derive(Clone, Debug)]
pub(super) struct RawPopulation {
    pub(super) population: String,
    pub(super) complete: bool,
    pub(super) objects: Vec<RawObject>,
}

#[derive(Clone, Debug)]
pub(super) struct SnapshotDocument {
    pub(super) observation: String,
    pub(super) anchor: Option<DocAnchor>,
    pub(super) model: ModelHeader,
    pub(super) populations: Vec<RawPopulation>,
}

#[derive(Clone, Debug)]
pub(super) enum ResultValue {
    Null,
    Value(RawValue),
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
    pub(super) parameters: Vec<(String, RawValue)>,
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
            "unavailable_observation",
            "missing-required-artifact",
        ))
    })?;
    // 1.2: input bytes.
    if bytes.len() as u64 > limits.document_bytes {
        return Err(refuse(admission_record(
            "stage_limit_exceeded",
            "input-bytes-exceeded",
        )));
    }
    let parsed: Option<OrderedJson> = serde_json::from_slice(bytes).ok();
    // 1.2: nesting depth (only meaningful once parsed).
    if let Some(value) = &parsed {
        if value.depth() > limits.nesting_depth {
            return Err(refuse(admission_record(
                "stage_limit_exceeded",
                "nesting-depth-exceeded",
            )));
        }
    }
    // 1.3: digest, before any member is read. Bytes that do not parse are
    // digested raw, and so refuse here.
    let digest = document_digest(bytes);
    if digest != selected.digest {
        return Err(refuse(admission_record(
            "stale_dependency",
            "byte-digest-mismatch",
        )));
    }
    let Some(value) = parsed else {
        return Err(fault("provisioned-bytes-not-json-after-digest-match"));
    };
    let object = value
        .as_object()
        .ok_or_else(|| fault("document-not-a-json-object"))?;

    // 1.4: format.
    let expected_format = match kind {
        DocumentKind::Snapshot => "quire.state.snapshot/v1",
        DocumentKind::Invocation => "quire.state.invocation/v1",
    };
    if object.member("format").and_then(|value| value.as_str()) != Some(expected_format) {
        return Err(refuse(admission_record("unknown_wire", "unsupported-wire")));
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
                admission_record("invalid_runtime_input", "missing-member").with("field", *member),
            ));
        }
    }
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(refuse(
                admission_record("invalid_runtime_input", "unknown-member")
                    .with("field", key.clone()),
            ));
        }
    }

    // 1.7/1.8: the document's own four labels.
    let identity_member = object
        .member("identity")
        .and_then(|value| value.as_object())
        .ok_or_else(|| fault("identity-member-not-an-object"))?;
    let label = |name: &str| {
        identity_member
            .member(name)
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_owned()
    };
    let document_identity = DocumentRef {
        authority: label("authority"),
        identity: label("identity"),
        revision_namespace: label("revision_namespace"),
        revision: label("revision"),
        digest,
    };
    if let Some(blank_label) = first_blank_label(&document_identity) {
        return Err(refuse(
            admission_record("invalid_source_identity", "blank-label").with("label", blank_label),
        ));
    }
    if !labels_match(&document_identity, selected) {
        return Err(refuse(
            admission_record("stale_dependency", "revision-mismatch")
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
        body,
    })
}

fn read_model(object: &[(String, OrderedJson)]) -> Result<ModelHeader, AdmissionFailure> {
    let model = object
        .member("model")
        .and_then(|value| value.as_object())
        .ok_or_else(|| fault("model-member-not-an-object"))?;
    Ok(ModelHeader {
        identity: model
            .member("identity")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_owned(),
        version: model
            .member("version")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_owned(),
        digest: model
            .member("digest")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .trim_start_matches("sha256-jcs:")
            .to_owned(),
    })
}

fn read_object_ref(value: &OrderedJson) -> Option<SelectedObject> {
    let object = value.as_object()?;
    Some(SelectedObject {
        population: object.member("population")?.as_str()?.to_owned(),
        key: object.member("key")?.as_str()?.to_owned(),
    })
}

fn read_document_ref(value: &OrderedJson) -> Option<DocumentRef> {
    let object = value.as_object()?;
    let identity = object.member("identity")?.as_object()?;
    let label = |name: &str| identity.member(name)?.as_str().map(str::to_owned);
    let digest_hex = object
        .member("digest")?
        .as_str()?
        .trim_start_matches("sha256-jcs:");
    let bytes = hex_bytes(digest_hex)?;
    if bytes.len() != 32 {
        return None;
    }
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&bytes);
    Some(DocumentRef {
        authority: label("authority")?,
        identity: label("identity")?,
        revision_namespace: label("revision_namespace")?,
        revision: label("revision")?,
        digest,
    })
}

fn hex_bytes(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).ok())
        .collect()
}

fn read_snapshot_body(
    object: &[(String, OrderedJson)],
    model: ModelHeader,
) -> Result<SnapshotDocument, AdmissionFailure> {
    let observation = object
        .member("observation")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_owned();
    let anchor = match object.member("anchor") {
        Some(value) => {
            let object = value
                .as_object()
                .ok_or_else(|| fault("anchor-member-not-an-object"))?;
            let kind = match object.member("kind").and_then(|value| value.as_str()) {
                Some("initialization") => super::AnchorKind::Initialization,
                Some("handler") => super::AnchorKind::Handler,
                _ => return Err(fault("unrecognized-anchor-kind")),
            };
            let name = object
                .member("name")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
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
        .ok_or_else(|| fault("populations-member-not-an-array"))?;
    let mut populations = Vec::with_capacity(items.len());
    for item in items {
        let entry = item
            .as_object()
            .ok_or_else(|| fault("population-entry-not-an-object"))?;
        let population = entry
            .member("population")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_owned();
        let complete = entry
            .member("complete")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let object_items = entry
            .member("objects")
            .and_then(|value| value.as_array())
            .ok_or_else(|| fault("objects-member-not-an-array"))?;
        let mut objects = Vec::with_capacity(object_items.len());
        for object_item in object_items {
            let object_entry = object_item
                .as_object()
                .ok_or_else(|| fault("object-entry-not-an-object"))?;
            let key = object_entry
                .member("key")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_owned();
            let type_identity = RawTypeIdentity(
                object_entry
                    .member("type")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_owned(),
            );
            let field_items = object_entry
                .member("fields")
                .and_then(|value| value.as_object())
                .ok_or_else(|| fault("fields-member-not-an-object"))?;
            let mut fields = Vec::with_capacity(field_items.len());
            for (name, value) in field_items {
                let raw = read_raw_value(&value.clone().into_value())
                    .ok_or_else(|| fault("field-value-not-a-recognized-form"))?;
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
        .unwrap_or_default()
        .to_owned();
    let operation = object
        .member("operation")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_owned();
    let self_object = object
        .member("self")
        .and_then(read_object_ref)
        .ok_or_else(|| fault("self-member-not-an-object-ref"))?;
    let pre = object
        .member("pre")
        .and_then(read_document_ref)
        .ok_or_else(|| fault("pre-member-not-a-document-ref"))?;
    let post = object
        .member("post")
        .and_then(read_document_ref)
        .ok_or_else(|| fault("post-member-not-a-document-ref"))?;
    let parameter_items = object
        .member("parameters")
        .and_then(|value| value.as_object())
        .ok_or_else(|| fault("parameters-member-not-an-object"))?;
    let mut parameters = Vec::with_capacity(parameter_items.len());
    for (name, value) in parameter_items {
        let raw = read_raw_value(&value.clone().into_value())
            .ok_or_else(|| fault("parameter-value-not-a-recognized-form"))?;
        parameters.push((name.clone(), raw));
    }
    let result = match object.member("result") {
        Some(OrderedJson::Null) => ResultValue::Null,
        Some(value) => ResultValue::Value(
            read_raw_value(&value.clone().into_value())
                .ok_or_else(|| fault("result-value-not-a-recognized-form"))?,
        ),
        None => return Err(fault("result-member-absent")),
    };
    let created = object
        .member("created")
        .and_then(|value| value.as_array())
        .ok_or_else(|| fault("created-member-not-an-array"))?
        .iter()
        .map(|item| read_object_ref(item).ok_or_else(|| fault("created-entry-not-an-object-ref")))
        .collect::<Result<Vec<_>, _>>()?;
    let deleted = object
        .member("deleted")
        .and_then(|value| value.as_array())
        .ok_or_else(|| fault("deleted-member-not-an-array"))?
        .iter()
        .map(|item| read_object_ref(item).ok_or_else(|| fault("deleted-entry-not-an-object-ref")))
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
}

/// A model object-type field's raw kind test, keyed off `value_type` alone
/// (used for operation parameters and results, whose `(String, ValueType)`
/// signature carries no separate presence: an optional operation parameter
/// or result is itself typed `ValueType::Option(_)`).
/// The first value named `name` in `fields` (document order, first
/// occurrence for a duplicate key), or `None`.
pub(super) fn raw_field<'a>(fields: &'a [(String, RawValue)], name: &str) -> Option<&'a RawValue> {
    fields
        .iter()
        .find(|(field, _)| field == name)
        .map(|(_, value)| value)
}

fn field_kind_matches(raw: &RawValue, value_type: &quire_exact::ValueType) -> bool {
    matches!(
        (raw, value_type),
        (RawValue::Boolean(_), quire_exact::ValueType::Boolean)
            | (
                RawValue::Integer(_),
                quire_exact::ValueType::Integer | quire_exact::ValueType::Int(_)
            )
            | (RawValue::Reference(_), quire_exact::ValueType::Reference(_))
            | (
                RawValue::Absent | RawValue::Present(_),
                quire_exact::ValueType::Option(_)
            )
            | (RawValue::Sequence(_), quire_exact::ValueType::Collection(_))
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
    raw: &RawValue,
    value_type: &quire_exact::ValueType,
    presence: quire_exact::Presence,
) -> bool {
    if presence == quire_exact::Presence::Optional {
        return matches!(raw, RawValue::Absent | RawValue::Present(_));
    }
    matches!(
        (raw, value_type),
        (RawValue::Boolean(_), quire_exact::ValueType::Boolean)
            | (
                RawValue::Integer(_),
                quire_exact::ValueType::Integer | quire_exact::ValueType::Int(_)
            )
            | (RawValue::Reference(_), quire_exact::ValueType::Reference(_))
            | (RawValue::Sequence(_), quire_exact::ValueType::Collection(_))
    )
}

fn admit_scalar(
    raw: &RawValue,
    value_type: &quire_exact::ValueType,
) -> Result<Value, AdmissionRecord> {
    match (raw, value_type) {
        (RawValue::Boolean(value), quire_exact::ValueType::Boolean) => Ok(Value::Boolean(*value)),
        (RawValue::Integer(spelling), quire_exact::ValueType::Integer) => spelling
            .parse::<Integer>()
            .map(Value::Integer)
            .map_err(|_| admission_record("invalid_runtime_input", "invalid-value")),
        (RawValue::Integer(spelling), quire_exact::ValueType::Int(interval)) => {
            let value = spelling
                .parse::<Integer>()
                .map_err(|_| admission_record("invalid_runtime_input", "invalid-value"))?;
            if interval.contains(&value) {
                Ok(Value::Integer(value))
            } else {
                Err(admission_record("invalid_runtime_input", "invalid-value"))
            }
        }
        (RawValue::Reference(reference), quire_exact::ValueType::Reference(type_identity)) => {
            let target = object_reference(&reference.population, *type_identity, &reference.key)
                .map_err(|_| admission_record("invalid_runtime_input", "wrong-value-kind"))?;
            Ok(Value::Reference(target))
        }
        _ => Err(admission_record(
            "invalid_runtime_input",
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
    raw: &RawValue,
    value_type: &quire_exact::ValueType,
    presence: quire_exact::Presence,
) -> Result<FieldValue, AdmissionRecord> {
    if presence == quire_exact::Presence::Optional {
        return match raw {
            RawValue::Absent => Ok(FieldValue::Absent),
            RawValue::Present(inner) => admit_scalar(inner, value_type).map(FieldValue::Present),
            // `object_field_kind_matches` already refused every other raw
            // form for an optional field before this is called.
            RawValue::Boolean(_)
            | RawValue::Integer(_)
            | RawValue::Reference(_)
            | RawValue::Sequence(_) => Err(admission_record(
                "invalid_runtime_input",
                "wrong-value-kind",
            )),
        };
    }
    match (raw, value_type) {
        (RawValue::Sequence(items), quire_exact::ValueType::Collection(collection)) => {
            if collection.kind() != CollectionKind::Sequence {
                return Err(admission_record(
                    "unknown_required_feature",
                    "unsupported-feature",
                ));
            }
            let mut elements = Vec::with_capacity(items.len());
            for item in items {
                elements.push(admit_scalar(item, collection.element())?);
            }
            Ok(FieldValue::Present(quire_exact::from_admitted(
                (**collection).clone(),
                elements,
            )))
        }
        (raw, value_type) => admit_scalar(raw, value_type).map(FieldValue::Present),
    }
}

/// FR-106 checks 6 to 8: admit every population's every object's every
/// field, in document order, then completeness (7) then closure (8).
/// `types` is the checked package's own effective attribute set
/// (`CheckedGraph::scope().types()`); `views` are the re-derived domain
/// package views (population declarations and identity strings, `EffectiveId`
/// conformance).
pub(super) fn admit_populations(
    views: &[ModelView],
    types: &TypeEnvironment,
    populations: &[RawPopulation],
) -> Result<AdmittedEnvironment, AdmissionFailure> {
    let mut objects: Vec<(ObjectReference, Vec<(&str, FieldValue)>)> = Vec::new();
    let mut completeness = BTreeMap::new();
    let mut keys_by_population: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut population_of: BTreeMap<ObjectReference, String> = BTreeMap::new();

    for entry in populations {
        completeness.insert(entry.population.clone(), entry.complete);
        let Some((population_view, population_info)) = find_population(views, &entry.population)
        else {
            return Err(refuse(
                admission_record("invalid_runtime_input", "wrong-role-mapping")
                    .with("population", entry.population.clone()),
            ));
        };
        let seen = keys_by_population
            .entry(entry.population.clone())
            .or_default();
        for object in &entry.objects {
            if !seen.insert(object.key.clone()) {
                return Err(refuse(
                    admission_record("invalid_runtime_input", "conflicting-identity")
                        .with("object", object.key.clone()),
                ));
            }
            let Some((_, object_key)) = find_declaration(views, object.type_identity.as_str())
            else {
                return Err(refuse(
                    admission_record("invalid_runtime_input", "wrong-role-mapping")
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
                    admission_record("invalid_runtime_input", "wrong-role-mapping")
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
            for attribute in declared {
                if let quire_exact::ValueType::Collection(collection) =
                    attribute.field().value_type()
                {
                    if collection.kind() != CollectionKind::Sequence {
                        return Err(refuse(
                            admission_record("unknown_required_feature", "unsupported-feature")
                                .with("field", attribute.field().name().to_owned()),
                        ));
                    }
                }
            }
            let mut attributes: Vec<(&str, FieldValue)> = Vec::with_capacity(declared.len());
            for attribute in declared {
                let name = attribute.field().name();
                let value_type = attribute.field().value_type();
                let presence = attribute.field().presence();
                let Some(raw) = raw_field(&object.fields, name) else {
                    return Err(refuse(
                        admission_record("invalid_runtime_input", "missing-member")
                            .with("object", object.key.clone())
                            .with("field", name.to_owned()),
                    ));
                };
                if !object_field_kind_matches(raw, value_type, presence) {
                    return Err(refuse(
                        admission_record("invalid_runtime_input", "wrong-value-kind")
                            .with("object", object.key.clone())
                            .with("field", name.to_owned()),
                    ));
                }
                let field_value =
                    admit_object_field(raw, value_type, presence).map_err(|record| {
                        refuse(
                            record
                                .with("object", object.key.clone())
                                .with("field", name.to_owned()),
                        )
                    })?;
                attributes.push((name, field_value));
            }
            for (name, _) in &object.fields {
                if !declared
                    .iter()
                    .any(|attribute| attribute.field().name() == name)
                {
                    return Err(refuse(
                        admission_record("invalid_runtime_input", "unknown-member")
                            .with("object", object.key.clone())
                            .with("field", name.clone()),
                    ));
                }
            }

            let reference = object_reference(&entry.population, effective_type, &object.key)
                .map_err(|_| fault("empty-object-identity"))?;
            population_of.insert(reference.clone(), entry.population.clone());
            objects.push((reference, attributes));
        }
    }

    // Check 7: a required population -- the one holding `self`, and every
    // population any reference field names -- must be complete. `self`'s
    // own population is checked by the caller (check 9 runs after this).
    let mut required: BTreeSet<String> = BTreeSet::new();
    for (_, attributes) in &objects {
        for (_, value) in attributes {
            if let FieldValue::Present(Value::Reference(reference)) = value {
                if let Some(name) = population_of.get(reference) {
                    required.insert(name.clone());
                }
            }
        }
    }
    for name in &required {
        if completeness.get(name).copied() != Some(true) {
            return Err(incomplete(
                admission_record("incomplete_population", "incomplete-scope")
                    .with("population", name.clone()),
            ));
        }
    }

    // Check 8: closure, skipped over an incomplete population.
    for (_, attributes) in &objects {
        for (_, value) in attributes {
            if let FieldValue::Present(Value::Reference(target)) = value {
                let Some(name) = population_of.get(target) else {
                    continue;
                };
                if completeness.get(name).copied() != Some(true) {
                    continue;
                }
                if !objects.iter().any(|(reference, _)| reference == target) {
                    return Err(refuse(
                        admission_record(
                            "dangling_reference",
                            "absent-target-in-complete-population",
                        )
                        .with("population", name.clone())
                        .with("object", target.object().as_str().to_owned()),
                    ));
                }
            }
        }
    }

    let environment = ObjectEnvironment::new(types, objects).map_err(map_environment_refusal)?;

    Ok(AdmittedEnvironment {
        environment,
        completeness,
    })
}

fn map_environment_refusal(
    error: crate::model::object_environment::ObjectEnvironmentRefusal,
) -> AdmissionFailure {
    match error.cause {
        ObjectEnvironmentCause::DanglingReference(target) => refuse(
            admission_record("dangling_reference", "absent-target-in-complete-population")
                .with("object", target.object().as_str().to_owned()),
        ),
        _ => fault("object-environment-refused-after-admission-checks"),
    }
}

pub(super) fn resolve_self(
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
    let reference = object_reference(&self_object.population, context_effective, &self_object.key)
        .map_err(|_| fault("empty-object-identity"))?;
    if environment.contains(&reference) {
        Ok(reference)
    } else {
        Err(refuse(admission_record(
            "invalid_runtime_input",
            "wrong-role-mapping",
        )))
    }
}

/// Check 10's admitted output: the operation's parameter bindings in
/// declared order, and its admitted result value (`None` for an operation
/// with no declared result).
pub(super) type AdmittedParametersAndResult = (Vec<(String, Value)>, Option<Value>);

pub(super) fn admit_parameters_and_result(
    operation: &OperationDeclaration,
    parameters: &[(String, RawValue)],
    result: &ResultValue,
) -> Result<AdmittedParametersAndResult, AdmissionFailure> {
    let mut admitted = Vec::with_capacity(operation.parameters().len());
    for (name, value_type) in operation.parameters() {
        let raw = raw_field(parameters, name).ok_or_else(|| {
            refuse(
                admission_record("invalid_runtime_input", "missing-member")
                    .with("field", name.clone()),
            )
        })?;
        if !field_kind_matches(raw, value_type) {
            return Err(refuse(
                admission_record("invalid_runtime_input", "wrong-value-kind")
                    .with("field", name.clone()),
            ));
        }
        let value = admit_scalar(raw, value_type)
            .map_err(|record| refuse(record.with("field", name.clone())))?;
        admitted.push((name.clone(), value));
    }
    for (name, _) in parameters {
        if !operation
            .parameters()
            .iter()
            .any(|(declared, _)| declared == name)
        {
            return Err(refuse(
                admission_record("invalid_runtime_input", "unknown-member")
                    .with("field", name.clone()),
            ));
        }
    }

    let result_value = match (operation.result(), result) {
        (None, ResultValue::Null) => None,
        (None, ResultValue::Value(_)) => {
            return Err(refuse(admission_record(
                "invalid_runtime_input",
                "unknown-member",
            )))
        }
        (Some(_), ResultValue::Null) => {
            return Err(refuse(admission_record(
                "invalid_runtime_input",
                "missing-member",
            )))
        }
        (Some(value_type), ResultValue::Value(raw)) => {
            if !field_kind_matches(raw, value_type) {
                return Err(refuse(admission_record(
                    "invalid_runtime_input",
                    "wrong-value-kind",
                )));
            }
            Some(admit_scalar(raw, value_type).map_err(refuse)?)
        }
    };
    Ok((admitted, result_value))
}
