// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-360 item 10: encoding a value that holds a variant value as an FR-106
//! value form. The reader of the `variant` form is in `tree`, its admission in
//! `document`; this is the writer, so a decode followed by an encode yields the
//! same typed observation.

use std::collections::BTreeMap;

use quire_exact::{CollectionKind, EnumMember, FieldValue, NodeKey, Value, VariantId};
use quire_semantic_value::declaration::{CompositeShape, TypeEnvironment};

use super::{
    collection_tag, fault, model_views, AdmissionFailure, ModelNormalizationLimits, NativeKind,
    RawNativeKind, RawNode, SnapshotValue,
};
use crate::model::domain_package::DomainPackageRef;

/// Writes kernel values as snapshot value forms, naming each variant value by
/// the IR node identity of its variant type and its exact case name.
pub struct ValueEncoder {
    variants: BTreeMap<VariantId, (String, String)>,
    records: BTreeMap<NodeKey, String>,
    types: TypeEnvironment,
}

impl ValueEncoder {
    /// The encoder over the variant types of `model_selections`, re-derived
    /// from `packages` as admission re-derives them, so a value it writes names
    /// the type and case admission reads back to the same value.
    ///
    /// # Errors
    ///
    /// The model re-admission and normalization refusals of
    /// [`super::admit_current_snapshot`].
    pub fn over(
        model_selections: &[DomainPackageRef],
        types: &TypeEnvironment,
        packages: &BTreeMap<[u8; 32], Vec<u8>>,
        limits: ModelNormalizationLimits,
    ) -> Result<Self, AdmissionFailure> {
        let views = model_views(model_selections, packages, limits)?;
        let unbounded = qsl_foundation::IdentityLimits {
            input_bytes: u64::MAX,
        };
        let mut variants = BTreeMap::new();
        let mut records = BTreeMap::new();
        for view in &views {
            for record in &view.view.domain_package().records {
                if let crate::model::domain_package::DomainPackageRecord::RecordValueType(
                    declared,
                ) = record
                {
                    if let Ok(Some(key)) = crate::check::record_node_key(record, unbounded) {
                        records.insert(key, declared.key.node.clone());
                    }
                }
            }
            let declared = crate::check::variant_type_declarations(&view.view, "", unbounded)
                .map_err(|_| fault("variant-type-declaration"))?;
            for (key, declaration) in declared {
                for case in declaration.cases() {
                    variants.insert(case.variant(), (key.node.clone(), case.name().to_owned()));
                }
            }
        }
        Ok(Self {
            variants,
            records,
            types: types.clone(),
        })
    }

    /// `value` as a value form, or `None` when it holds a form this encoder
    /// does not write: an enum member of no variant type, a record of no
    /// selected record value type, or a reference, which needs the
    /// population only a snapshot knows. The encoder writes exactly the forms
    /// admission reads back from a value: a Boolean, an integer, a Text, a
    /// variant, a record, an `Option` and a `Sequence`, `Set`, `Bag` or
    /// `OrderedSet` of them.
    ///
    /// Every value inside the element of a `Set` or `Bag` is written in its
    /// canonical encoding (FR-106 "Set, bag and ordered-set values"): a Text
    /// by its retained sequence under its profile, the FR-141 compared form,
    /// so that equal elements write identically. A Text elsewhere keeps the
    /// payload it was admitted with. [`SnapshotValue::to_json`] then orders a
    /// set's or bag's elements by their encoded bytes.
    ///
    /// The walk keeps its own heap stack, so a value of any depth encodes in
    /// constant native stack.
    #[must_use]
    pub fn encode(&self, value: &Value) -> Option<SnapshotValue> {
        enum Work<'v> {
            /// Write the value; the flag is whether it sits inside the
            /// element of a set or bag.
            Read(&'v Value, bool),
            Present,
            Absent,
            Collection(CollectionKind, usize),
            Record(String, Vec<String>),
        }
        let mut work = vec![Work::Read(value, false)];
        let mut done: Vec<SnapshotValue> = Vec::new();
        while let Some(next) = work.pop() {
            match next {
                Work::Present => {
                    let inner = done.pop()?;
                    done.push(SnapshotValue::present(inner));
                }
                Work::Absent => done.push(SnapshotValue::absent()),
                Work::Record(type_identity, names) => {
                    let values = done.split_off(done.len().checked_sub(names.len())?);
                    done.push(SnapshotValue::record(
                        type_identity,
                        names.into_iter().zip(values).collect(),
                    ));
                }
                Work::Collection(kind, len) => {
                    let items = done.split_off(done.len().checked_sub(len)?);
                    done.push(SnapshotValue::collection(kind, items));
                }
                Work::Read(Value::Boolean(held), _) => done.push(SnapshotValue::boolean(*held)),
                Work::Read(Value::Integer(held), _) => {
                    done.push(SnapshotValue::integer(held.to_string()));
                }
                Work::Read(Value::Text(held), canonical) => {
                    let written = if canonical {
                        held.retained()
                    } else {
                        held.payload().as_str()
                    };
                    done.push(SnapshotValue::native(
                        RawNativeKind::Known(NativeKind::Text),
                        vec![("text".to_owned(), Some(written.to_owned()))],
                    ));
                }
                Work::Read(Value::Enum(member), _) => done.push(self.variant(member)?),
                Work::Read(Value::Option(option), canonical) => match option.payload() {
                    None => done.push(SnapshotValue::absent()),
                    Some(payload) => {
                        work.push(Work::Present);
                        work.push(Work::Read(payload, canonical));
                    }
                },
                Work::Read(Value::Collection(collection), canonical) => {
                    let kind = collection.collection_type().kind();
                    let canonical =
                        canonical || matches!(kind, CollectionKind::Set | CollectionKind::Bag);
                    let elements = collection.elements();
                    work.push(Work::Collection(kind, elements.len()));
                    // Pushed last-first, so the first element is written first.
                    work.extend(
                        elements
                            .iter()
                            .rev()
                            .map(|element| Work::Read(element, canonical)),
                    );
                }
                Work::Read(Value::Composite(composite), canonical) => {
                    let type_identity = self.records.get(&composite.declaration())?;
                    let CompositeShape::Record(declared) =
                        self.types.composite(composite.declaration())?.shape()
                    else {
                        return None;
                    };
                    let names: Vec<String> = declared
                        .iter()
                        .map(|field| field.name().to_owned())
                        .collect();
                    work.push(Work::Record(type_identity.clone(), names));
                    // Pushed last-first, so the first field is written first.
                    for slot in composite.slots().iter().rev() {
                        work.push(match slot {
                            FieldValue::Present(held) => Work::Read(held, canonical),
                            FieldValue::Absent => Work::Absent,
                            FieldValue::Null => return None,
                        });
                    }
                }
                Work::Read(_, _) => return None,
            }
        }
        done.pop()
    }

    fn variant(&self, member: &EnumMember) -> Option<SnapshotValue> {
        let (type_identity, case) = self.variants.get(&member.variant())?;
        Some(SnapshotValue::variant(type_identity.clone(), case.clone()))
    }
}

impl SnapshotValue {
    /// This value as its wire JSON. A `variant` writes exactly its `type` and
    /// `case`. Nodes are held children first, so one pass over the arena builds
    /// every value from the values it holds, in constant native stack.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        use serde_json::{json, Map, Value as Json};
        let mut built: Vec<Option<Json>> = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let value = match node {
                RawNode::Boolean(held) => json!({"boolean": held}),
                RawNode::Integer(spelling) => json!({"integer": spelling}),
                RawNode::Native(native) => {
                    // A `kind` that was not text, and a member that was not
                    // text, are held only to be refused by admission; they
                    // are written as `null`.
                    let mut object = Map::new();
                    object.insert(
                        "kind".to_owned(),
                        match native.kind {
                            RawNativeKind::Known(NativeKind::String) => json!("string"),
                            RawNativeKind::Known(NativeKind::Uuid) => json!("uuid"),
                            RawNativeKind::Known(NativeKind::Timestamp) => json!("timestamp"),
                            RawNativeKind::Known(NativeKind::Text) => json!("text"),
                            RawNativeKind::Unknown | RawNativeKind::NotText => Json::Null,
                        },
                    );
                    for (name, text) in &native.members {
                        object.insert(name.clone(), json!(text));
                    }
                    Json::Object(object)
                }
                RawNode::Absent => json!({"absent": {}}),
                RawNode::Present(inner) => json!({"present": take(&mut built, *inner)}),
                RawNode::Reference(reference) => json!({"reference": {
                    "population": reference.population, "key": reference.key,
                }}),
                RawNode::Collection { kind, items } => {
                    let mut items: Vec<Json> =
                        items.iter().map(|item| take(&mut built, *item)).collect();
                    // A set and a bag carry no element order: the wire orders
                    // them by encoded bytes, so one value has one document.
                    if matches!(kind, CollectionKind::Set | CollectionKind::Bag) {
                        items.sort_by_cached_key(canonical_bytes);
                    }
                    let mut object = Map::new();
                    object.insert(collection_tag(*kind).to_owned(), Json::Array(items));
                    Json::Object(object)
                }
                RawNode::Record {
                    type_identity,
                    fields,
                } => {
                    let mut members = Map::new();
                    for (name, index) in fields {
                        members.insert(name.clone(), take(&mut built, *index));
                    }
                    json!({"record": {"type": type_identity, "fields": members}})
                }
                RawNode::Variant {
                    type_identity,
                    case,
                    ..
                } => {
                    let mut payload = Map::new();
                    if let Some(type_identity) = type_identity {
                        payload.insert("type".to_owned(), json!(type_identity));
                    }
                    if let Some(case) = case {
                        payload.insert("case".to_owned(), json!(case));
                    }
                    json!({"variant": payload})
                }
            };
            built.push(Some(value));
        }
        take(&mut built, self.root)
    }
}

/// The RFC 8785 bytes of `value`, the order a set's and a bag's elements are
/// written in.
fn canonical_bytes(value: &serde_json::Value) -> Vec<u8> {
    let text = value.to_string().into_bytes();
    let canonical = match quire_canonical::read(&text, u64::MAX) {
        Ok(read) => quire_canonical::to_vec(&read, quire_canonical::Limits::new(u64::MAX)).ok(),
        Err(_) => None,
    };
    canonical.unwrap_or(text)
}

/// The built value at `index`, left empty: every node is a child of at most
/// one parent.
fn take(built: &mut [Option<serde_json::Value>], index: usize) -> serde_json::Value {
    built
        .get_mut(index)
        .and_then(Option::take)
        .unwrap_or(serde_json::Value::Null)
}
