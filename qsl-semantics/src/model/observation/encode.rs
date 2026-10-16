// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-360 item 10: encoding a value that holds a variant value as an FR-106
//! value form. The reader of the `variant` form is in `tree`, its admission in
//! `document`; this is the writer, so a decode followed by an encode yields the
//! same typed observation.

use std::collections::BTreeMap;

use quire_exact::{CollectionKind, EnumMember, FieldValue, NodeKey, Value, VariantId};
use quire_semantic_value::declaration::{CompositeShape, TypeEnvironment};

use super::{
    fault, model_views, AdmissionFailure, ModelNormalizationLimits, NativeKind, RawNativeKind,
    RawNode, SnapshotValue,
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
    /// selected record value type, or any of Text, a reference, a `Set`, a
    /// `Bag` and an `OrderedSet`. Admission reads only `Sequence` collections
    /// (the others refuse `unsupported-feature`), and a reference needs the
    /// population only a snapshot knows, so the encoder writes exactly the
    /// forms admission reads back from a value: a Boolean, an integer, a
    /// variant, a record, an `Option` and a `Sequence` of them. The walk keeps its own heap
    /// stack, so a value of any depth encodes in constant native stack.
    #[must_use]
    pub fn encode(&self, value: &Value) -> Option<SnapshotValue> {
        enum Work<'v> {
            Read(&'v Value),
            Present,
            Absent,
            Sequence(usize),
            Record(String, Vec<String>),
        }
        let mut work = vec![Work::Read(value)];
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
                Work::Sequence(len) => {
                    let items = done.split_off(done.len().checked_sub(len)?);
                    done.push(SnapshotValue::sequence(items));
                }
                Work::Read(Value::Boolean(held)) => done.push(SnapshotValue::boolean(*held)),
                Work::Read(Value::Integer(held)) => {
                    done.push(SnapshotValue::integer(held.to_string()));
                }
                Work::Read(Value::Enum(member)) => done.push(self.variant(member)?),
                Work::Read(Value::Option(option)) => match option.payload() {
                    None => done.push(SnapshotValue::absent()),
                    Some(payload) => {
                        work.push(Work::Present);
                        work.push(Work::Read(payload));
                    }
                },
                Work::Read(Value::Collection(collection)) => {
                    if collection.collection_type().kind() != CollectionKind::Sequence {
                        return None;
                    }
                    let elements = collection.elements();
                    work.push(Work::Sequence(elements.len()));
                    // Pushed last-first, so the first element is written first.
                    work.extend(elements.iter().rev().map(Work::Read));
                }
                Work::Read(Value::Composite(composite)) => {
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
                            FieldValue::Present(held) => Work::Read(held),
                            FieldValue::Absent => Work::Absent,
                            FieldValue::Null => return None,
                        });
                    }
                }
                Work::Read(_) => return None,
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
                RawNode::Sequence(items) => {
                    let items: Vec<Json> =
                        items.iter().map(|item| take(&mut built, *item)).collect();
                    json!({ "sequence": items })
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

/// The built value at `index`, left empty: every node is a child of at most
/// one parent.
fn take(built: &mut [Option<serde_json::Value>], index: usize) -> serde_json::Value {
    built
        .get_mut(index)
        .and_then(Option::take)
        .unwrap_or(serde_json::Value::Null)
}
