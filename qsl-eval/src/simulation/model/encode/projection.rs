// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120 / QSpec FR-181's borrowed typed-value projection, independent of
//! retained-tree storage.

use core::convert::Infallible;
use std::collections::BTreeMap;

use qsl_semantics::model::state::{ModelState, StateField, StateModel, StateObject};
use quire_canonical::{Error, Limits, Sink, Writer};
use quire_checked::key::DeclarationKey;
use quire_exact::{
    CollectionKind, EffectiveId, FieldValue, IeeeWidth, Integer, NodeKey, ObjectReference,
    Timestamp, UnitId, Uuid, Value, VariantId,
};
use quire_semantic_value::declaration::{CompositeDeclaration, CompositeShape, TypeEnvironment};

use super::{DeclarationRecord, Enc, KeyNames, Placeholder, Typing};

/// Source scalars have not been copied or formatted when the sink receives them.
pub(super) enum Scalar<'a> {
    Null,
    Boolean(bool),
    Text(&'a str),
    Integer(&'a Integer),
    Hex {
        bits: u64,
        digits: usize,
    },
    Scale(u32),
    Uuid(&'a Uuid),
    Timestamp(&'a Timestamp),
    Node(&'a NodeKey),
    Effective(&'a EffectiveId),
    Declaration(DeclarationRecord<'a>),
    DeclarationKey(&'a DeclarationKey),
    DeclarationNode(&'a NodeKey),
    ModelReference {
        model: &'a StateModel,
        object_type: &'a DeclarationKey,
        object: &'a str,
        names: &'a KeyNames,
    },
    PopulationName {
        population: &'a DeclarationKey,
        names: &'a KeyNames,
    },
    UnitText(&'a UnitId),
    Replacement(Enc),
    Unencodable(&'static str),
}

#[derive(Clone, Copy)]
pub(super) enum ArrayOrder {
    Retained,
    CanonicalBytes,
    MapKeys,
}

/// Structural reference events are also consumed directly by the borrowed
/// canonical writer, without exposing the legacy value renderer to that path.
pub(super) trait StructuralSink {
    type Error;

    fn begin_object(&mut self) -> Result<(), Self::Error>;
    fn name(&mut self, name: &'static str) -> Result<(), Self::Error>;
    fn end_object(&mut self) -> Result<(), Self::Error>;
    fn text(&mut self, source: &str) -> Result<(), Self::Error>;
    fn node(&mut self, source: &NodeKey) -> Result<(), Self::Error>;
    fn effective(&mut self, source: &EffectiveId) -> Result<(), Self::Error>;
    fn null(&mut self) -> Result<(), Self::Error>;
}

/// Each sink owns its storage, scalar rendering, metadata lookup and ordering.
/// The projection owns the complete field schema and traversal.
pub(super) trait ValueSink: StructuralSink {
    fn pending<T>(&mut self, stack: &mut Vec<T>, task: T) -> Result<(), Self::Error>;
    fn visit(&mut self) -> Result<(), Self::Error>;
    fn scalar(&mut self, source: Scalar<'_>) -> Result<(), Self::Error>;
    fn begin_array(&mut self, order: ArrayOrder) -> Result<(), Self::Error>;
    fn end_array(&mut self) -> Result<(), Self::Error>;
    fn composite<'t>(
        &mut self,
        types: &'t TypeEnvironment,
        key: &NodeKey,
    ) -> Result<Option<&'t CompositeDeclaration>, Self::Error>;
    fn variant(&mut self, left: &VariantId, right: &VariantId) -> Result<bool, Self::Error>;
    fn fields(
        &mut self,
        model: &StateModel,
        object_type: &DeclarationKey,
    ) -> Result<Option<Vec<StateField>>, Self::Error>;
    fn cached_fields<'t>(
        &mut self,
        fields: &'t BTreeMap<DeclarationKey, Vec<StateField>>,
        object_type: &DeclarationKey,
    ) -> Result<Option<&'t [StateField]>, Self::Error>;
    fn field<'v>(
        &mut self,
        object: &'v StateObject,
        key: &DeclarationKey,
    ) -> Result<Option<&'v Value>, Self::Error>;
    fn declaration_equal(
        &mut self,
        left: &DeclarationKey,
        right: &DeclarationKey,
    ) -> Result<bool, Self::Error>;
    fn clock_count(&mut self, value: &Integer) -> Result<Option<u64>, Self::Error>;
}

enum Frame {
    Object {
        members: Vec<(&'static str, Enc)>,
        name: Option<&'static str>,
        key_bytes: Option<Vec<u8>>,
    },
    Array {
        items: Vec<Enc>,
        order: ArrayOrder,
        keyed: Vec<(Vec<u8>, Enc)>,
    },
}

/// The existing API's retained-tree sink, preserving its allocation policy.
/// Admission-aware implementations consume the projection directly.
#[derive(Default)]
struct Retained {
    stack: Vec<Frame>,
    root: Option<Enc>,
}

impl Retained {
    fn push(&mut self, value: Enc) {
        match self.stack.last_mut() {
            Some(Frame::Object { members, name, .. }) => {
                let name = name
                    .take()
                    .expect("the private projection names every object value");
                members.push((name, value));
            }
            Some(Frame::Array {
                items,
                order,
                keyed,
            }) => {
                if matches!(order, ArrayOrder::MapKeys) {
                    keyed.push((map_key_bytes(&value), value));
                } else {
                    items.push(value);
                }
            }
            None => {
                assert!(self.root.is_none(), "the private projection emits one root");
                self.root = Some(value);
            }
        }
    }

    fn push_named(&mut self, (bytes, value): (Vec<u8>, Enc)) {
        let Some(Frame::Object {
            name: Some("key"),
            key_bytes,
            ..
        }) = self.stack.last_mut()
        else {
            panic!("the private projection emits cached names only as map keys");
        };
        *key_bytes = Some(bytes);
        self.push(value);
    }

    fn finish(self) -> Enc {
        assert!(
            self.stack.is_empty(),
            "the private projection closes every container"
        );
        self.root.expect("the private projection emits a root")
    }
}

/// Select the private projection's map-entry key member for canonical encoding.
#[ix_trace_rs::string_edge]
fn map_key_bytes(value: &Enc) -> Vec<u8> {
    let Enc::Object(members) = value else {
        panic!("the private projection emits map-entry objects");
    };
    let (_, key) = members
        .iter()
        .find(|(name, _)| *name == "key")
        .expect("the private projection supplies each map entry's key");
    super::Canonicalizer::default()
        .to_vec(key, Limits::new(u64::MAX))
        .unwrap_or_default()
}

impl ValueSink for Retained {
    fn pending<T>(&mut self, stack: &mut Vec<T>, task: T) -> Result<(), Self::Error> {
        stack.push(task);
        Ok(())
    }
    fn visit(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn scalar(&mut self, source: Scalar<'_>) -> Result<(), Self::Error> {
        let value = match source {
            Scalar::Null => Enc::Null,
            Scalar::Boolean(value) => Enc::Bool(value),
            Scalar::Text(value) => Enc::text(value),
            Scalar::Integer(value) => Enc::text(value.to_string()),
            Scalar::Hex { bits, digits } => Enc::text(format!("{bits:0digits$x}")),
            Scalar::Scale(value) => Enc::text(value.to_string()),
            Scalar::Uuid(value) => Enc::text(value.to_string()),
            Scalar::Timestamp(value) => Enc::text(value.to_string()),
            Scalar::Node(value) => Enc::Node(value.clone()),
            Scalar::Effective(value) => Enc::Effective(value.clone()),
            Scalar::Declaration(value) => value
                .text(Limits::new(super::super::identity_limits().input_bytes))
                .map_or(Enc::Unencodable("a declaration identity"), Enc::Str),
            Scalar::DeclarationKey(value) => Enc::Declaration(value.clone()),
            Scalar::DeclarationNode(value) => DeclarationRecord::of_node(value)
                .and_then(|key| {
                    key.text(Limits::new(super::super::identity_limits().input_bytes))
                        .ok()
                })
                .map_or(Enc::Unencodable("a declaration identity"), Enc::Str),
            Scalar::ModelReference {
                model,
                object_type,
                object,
                names,
            } => match names.reference(model, object_type, object) {
                Some(named) => {
                    self.push_named(named);
                    return Ok(());
                }
                None => Enc::Unencodable("an object whose reference does not resolve"),
            },
            Scalar::PopulationName { population, names } => {
                self.push_named(names.population(population));
                return Ok(());
            }
            Scalar::UnitText(value) => super::unit_enc(value),
            Scalar::Replacement(value) => value,
            Scalar::Unencodable(what) => Enc::Unencodable(what),
        };
        self.push(value);
        Ok(())
    }

    fn begin_array(&mut self, order: ArrayOrder) -> Result<(), Self::Error> {
        self.stack.push(Frame::Array {
            items: Vec::new(),
            order,
            keyed: Vec::new(),
        });
        Ok(())
    }

    fn end_array(&mut self) -> Result<(), Self::Error> {
        let Some(Frame::Array {
            mut items,
            order,
            mut keyed,
        }) = self.stack.pop()
        else {
            panic!("the private projection closes an array");
        };
        if matches!(order, ArrayOrder::CanonicalBytes) {
            items.sort_by_cached_key(|value| {
                quire_canonical::to_vec(value, Limits::new(u64::MAX)).unwrap_or_default()
            });
        } else if matches!(order, ArrayOrder::MapKeys) {
            keyed.sort_by(|left, right| left.0.cmp(&right.0));
            items = keyed.into_iter().map(|(_, value)| value).collect();
        }
        self.push(Enc::Array(items));
        Ok(())
    }

    fn composite<'t>(
        &mut self,
        types: &'t TypeEnvironment,
        key: &NodeKey,
    ) -> Result<Option<&'t CompositeDeclaration>, Self::Error> {
        Ok(types.composite(key.clone()))
    }

    fn variant(&mut self, left: &VariantId, right: &VariantId) -> Result<bool, Self::Error> {
        Ok(left == right)
    }

    fn fields(
        &mut self,
        model: &StateModel,
        object_type: &DeclarationKey,
    ) -> Result<Option<Vec<StateField>>, Self::Error> {
        Ok(model.fields(object_type).ok().flatten())
    }

    fn cached_fields<'t>(
        &mut self,
        fields: &'t BTreeMap<DeclarationKey, Vec<StateField>>,
        object_type: &DeclarationKey,
    ) -> Result<Option<&'t [StateField]>, Self::Error> {
        Ok(fields.get(object_type).map(Vec::as_slice))
    }

    fn field<'v>(
        &mut self,
        object: &'v StateObject,
        key: &DeclarationKey,
    ) -> Result<Option<&'v Value>, Self::Error> {
        Ok(object.fields.get(key))
    }

    fn declaration_equal(
        &mut self,
        left: &DeclarationKey,
        right: &DeclarationKey,
    ) -> Result<bool, Self::Error> {
        Ok(left == right)
    }

    fn clock_count(&mut self, value: &Integer) -> Result<Option<u64>, Self::Error> {
        Ok(value.to_u64())
    }
}

impl StructuralSink for Retained {
    type Error = Infallible;
    fn null(&mut self) -> Result<(), Self::Error> {
        self.scalar(Scalar::Null)
    }

    fn begin_object(&mut self) -> Result<(), Self::Error> {
        self.stack.push(Frame::Object {
            members: Vec::new(),
            name: None,
            key_bytes: None,
        });
        Ok(())
    }

    fn name(&mut self, next: &'static str) -> Result<(), Self::Error> {
        let Some(Frame::Object { name, .. }) = self.stack.last_mut() else {
            panic!("the private projection names members only inside an object");
        };
        assert!(
            name.is_none(),
            "the private projection supplies every named value"
        );
        *name = Some(next);
        Ok(())
    }

    fn end_object(&mut self) -> Result<(), Self::Error> {
        let Some(Frame::Object {
            members,
            name: None,
            key_bytes,
        }) = self.stack.pop()
        else {
            panic!("the private projection closes a complete object");
        };
        let value = Enc::Object(members);
        match (key_bytes, self.stack.last_mut()) {
            (
                Some(bytes),
                Some(Frame::Array {
                    order: ArrayOrder::MapKeys,
                    keyed,
                    ..
                }),
            ) => {
                keyed.push((bytes, value));
            }
            (None, _) => self.push(value),
            (Some(_), _) => panic!("the private projection caches only map-entry keys"),
        }
        Ok(())
    }

    fn text(&mut self, source: &str) -> Result<(), Self::Error> {
        self.scalar(Scalar::Text(source))
    }

    fn node(&mut self, source: &NodeKey) -> Result<(), Self::Error> {
        self.scalar(Scalar::Node(source))
    }

    fn effective(&mut self, source: &EffectiveId) -> Result<(), Self::Error> {
        self.scalar(Scalar::Effective(source))
    }
}

struct ReferenceWriter<'w, 's, S: Sink + ?Sized> {
    writer: &'w mut Writer<'s, S>,
}

impl<S: Sink + ?Sized> StructuralSink for ReferenceWriter<'_, '_, S> {
    type Error = Error;
    fn null(&mut self) -> Result<(), Self::Error> {
        self.writer.null()
    }

    fn begin_object(&mut self) -> Result<(), Self::Error> {
        self.writer.begin_object()
    }

    fn name(&mut self, name: &'static str) -> Result<(), Self::Error> {
        self.writer.name(name)
    }

    fn end_object(&mut self) -> Result<(), Self::Error> {
        self.writer.end_object()
    }

    fn text(&mut self, source: &str) -> Result<(), Self::Error> {
        self.writer.string(source)
    }

    fn node(&mut self, source: &NodeKey) -> Result<(), Self::Error> {
        qsl_foundation::node_wire::encode_node_key(source, self.writer)
    }

    fn effective(&mut self, source: &EffectiveId) -> Result<(), Self::Error> {
        super::write_effective(self.writer, source)
    }
}

fn retained(project: impl FnOnce(&mut Retained) -> Result<(), Infallible>) -> Enc {
    let mut sink = Retained::default();
    match project(&mut sink) {
        Ok(()) => sink.finish(),
        Err(never) => match never {},
    }
}

pub(super) fn value(
    source: &Value,
    typing: Typing<'_>,
    placeholder: &Placeholder<'_>,
    normalize_decimal: bool,
) -> Enc {
    retained(|sink| project_value(sink, source, typing, Some(placeholder), normalize_decimal))
}

pub(super) fn reference(source: &ObjectReference) -> Enc {
    retained(|sink| project_reference(sink, source))
}

pub(super) fn write_reference<S: Sink + ?Sized>(
    source: &ObjectReference,
    writer: &mut Writer<'_, S>,
) -> Result<(), Error> {
    project_reference(&mut ReferenceWriter { writer }, source)
}

pub(super) fn unit_identity(source: &UnitId) -> Enc {
    retained(|sink| project_unit(sink, source))
}

pub(super) fn population_name(source: &DeclarationKey) -> Enc {
    retained(|sink| project_population(sink, source))
}

pub(super) fn sorted_map(keyed: Vec<(Vec<u8>, Enc, Enc)>) -> Enc {
    retained(|sink| {
        map(sink, ArrayOrder::Retained, |sink| {
            for (_, key, value) in keyed {
                entry(
                    sink,
                    |sink| sink.scalar(Scalar::Replacement(key)),
                    |sink| sink.scalar(Scalar::Replacement(value)),
                )?;
            }
            Ok(())
        })
    })
}

pub(super) fn semantic(model: &StateModel, state: &ModelState, typing: Typing<'_>) -> Enc {
    retained(|sink| project_semantic(sink, model, state, typing))
}

pub(super) fn object(
    model: &StateModel,
    source: &StateObject,
    typing: Typing<'_>,
    placeholder: &Placeholder<'_>,
) -> Enc {
    retained(|sink| project_object(sink, model, source, typing, Some(placeholder)))
}

pub(super) fn model_state(model: &StateModel, state: &ModelState, typing: Typing<'_>) -> Enc {
    retained(|sink| project_model_state(sink, model, state, typing))
}

fn tagged<S: StructuralSink>(
    sink: &mut S,
    tag: &str,
    fields: impl FnOnce(&mut S) -> Result<(), S::Error>,
) -> Result<(), S::Error> {
    sink.begin_object()?;
    fields(sink)?;
    sink.name("type")?;
    sink.text(tag)?;
    sink.end_object()
}

fn map<S: ValueSink>(
    sink: &mut S,
    order: ArrayOrder,
    entries: impl FnOnce(&mut S) -> Result<(), S::Error>,
) -> Result<(), S::Error> {
    tagged(sink, "map", |sink| {
        sink.name("entries")?;
        sink.begin_array(order)?;
        entries(sink)?;
        sink.end_array()
    })
}

fn entry<S: ValueSink>(
    sink: &mut S,
    key: impl FnOnce(&mut S) -> Result<(), S::Error>,
    value: impl FnOnce(&mut S) -> Result<(), S::Error>,
) -> Result<(), S::Error> {
    sink.begin_object()?;
    sink.name("key")?;
    key(sink)?;
    sink.name("value")?;
    value(sink)?;
    sink.end_object()
}

pub(super) fn project_population<S: ValueSink>(
    sink: &mut S,
    source: &DeclarationKey,
) -> Result<(), S::Error> {
    tagged(sink, "text", |sink| {
        sink.name("value")?;
        sink.scalar(Scalar::DeclarationKey(source))
    })
}

pub(super) fn project_semantic<S: ValueSink>(
    sink: &mut S,
    model: &StateModel,
    state: &ModelState,
    typing: Typing<'_>,
) -> Result<(), S::Error> {
    let fresh = KeyNames::default();
    let names = typing.names.unwrap_or(&fresh);
    map(sink, ArrayOrder::MapKeys, |sink| {
        for (population, objects) in &state.populations {
            sink.visit()?;
            entry(
                sink,
                |sink| sink.scalar(Scalar::PopulationName { population, names }),
                |sink| {
                    map(sink, ArrayOrder::MapKeys, |sink| {
                        for (key, object) in objects {
                            sink.visit()?;
                            entry(
                                sink,
                                |sink| {
                                    sink.scalar(Scalar::ModelReference {
                                        model,
                                        object_type: &object.type_identity,
                                        object: key,
                                        names,
                                    })
                                },
                                |sink| project_object(sink, model, object, typing, None),
                            )?;
                        }
                        Ok(())
                    })
                },
            )?;
        }
        if !state.extents.is_empty() {
            entry(
                sink,
                |sink| {
                    tagged(sink, "text", |sink| {
                        sink.name("value")?;
                        sink.text("relationships")
                    })
                },
                |sink| project_extents(sink, state),
            )?;
        }
        Ok(())
    })
}

fn project_extents<S: ValueSink>(sink: &mut S, state: &ModelState) -> Result<(), S::Error> {
    map(sink, ArrayOrder::MapKeys, |sink| {
        for extent in &state.extents {
            sink.visit()?;
            entry(
                sink,
                |sink| project_population(sink, &extent.relationship),
                |sink| {
                    sink.begin_object()?;
                    sink.name("complete")?;
                    sink.scalar(Scalar::Boolean(extent.complete))?;
                    sink.name("instances")?;
                    sink.begin_array(ArrayOrder::CanonicalBytes)?;
                    for (source, target) in &extent.instances {
                        sink.visit()?;
                        sink.begin_object()?;
                        sink.name("source")?;
                        project_reference(sink, source)?;
                        sink.name("target")?;
                        project_reference(sink, target)?;
                        sink.end_object()?;
                    }
                    sink.end_array()?;
                    sink.end_object()
                },
            )?;
        }
        Ok(())
    })
}

fn synthetic_field<S: ValueSink>(
    sink: &mut S,
    object_type: &DeclarationKey,
    field: &StateField,
    typing: Typing<'_>,
) -> Result<bool, S::Error> {
    for (owner, synthetic, _, _) in typing.clock_counts {
        sink.visit()?;
        if sink.declaration_equal(owner, object_type)?
            && sink.declaration_equal(&synthetic.key, &field.key)?
        {
            return Ok(true);
        }
    }
    for (owner, synthetic) in typing.clock_bits {
        sink.visit()?;
        if sink.declaration_equal(owner, object_type)?
            && sink.declaration_equal(&synthetic.key, &field.key)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn project_object<S: ValueSink>(
    sink: &mut S,
    model: &StateModel,
    object: &StateObject,
    typing: Typing<'_>,
    placeholder: Option<&Placeholder<'_>>,
) -> Result<(), S::Error> {
    sink.visit()?;
    let owned = if typing.state_fields.is_none() {
        sink.fields(model, &object.type_identity)?
    } else {
        None
    };
    let fields = match typing.state_fields {
        Some(fields) => sink.cached_fields(fields, &object.type_identity)?,
        None => owned.as_deref(),
    };
    let Some(fields) = fields else {
        return sink.scalar(Scalar::Unencodable("an object of an undeclared type"));
    };
    tagged(sink, "record", |sink| {
        sink.name("name")?;
        sink.scalar(Scalar::DeclarationKey(&object.type_identity))?;
        sink.name("fields")?;
        sink.begin_array(ArrayOrder::Retained)?;
        for field in fields {
            sink.visit()?;
            if typing.state_fields.is_some()
                && synthetic_field(sink, &object.type_identity, field, typing)?
            {
                continue;
            }
            sink.begin_object()?;
            sink.name("name")?;
            sink.text(&field.name)?;
            sink.name("value")?;
            match sink.field(object, &field.key)? {
                Some(value) => project_value(sink, value, typing, placeholder, false)?,
                None => sink.scalar(Scalar::Null)?,
            }
            sink.end_object()?;
        }
        for (owner, field, _, _) in typing.clock_counts {
            sink.visit()?;
            if !sink.declaration_equal(owner, &object.type_identity)? {
                continue;
            }
            let Some(value) = sink.field(object, &field.key)? else {
                continue;
            };
            if !matches!(value, Value::Integer(_) | Value::Collection(_)) {
                continue;
            }
            sink.begin_object()?;
            sink.name("name")?;
            sink.text(&field.name)?;
            sink.name("value")?;
            tagged(sink, "sequence", |sink| {
                sink.name("elements")?;
                sink.begin_array(ArrayOrder::Retained)?;
                match value {
                    Value::Integer(count) => {
                        let count = sink.clock_count(count)?.unwrap_or(0);
                        for _ in 0..count {
                            sink.visit()?;
                            project_clock_shell(sink)?;
                        }
                    }
                    Value::Collection(bits) => {
                        for bit in bits.elements() {
                            sink.visit()?;
                            tagged(sink, "option", |sink| {
                                sink.name("value")?;
                                if matches!(bit, Value::Boolean(true)) {
                                    project_clock_shell(sink)
                                } else {
                                    sink.scalar(Scalar::Null)
                                }
                            })?;
                        }
                    }
                    _ => {}
                }
                sink.end_array()
            })?;
            sink.end_object()?;
        }
        for (owner, field) in typing.clock_bits {
            sink.visit()?;
            if !sink.declaration_equal(owner, &object.type_identity)? {
                continue;
            }
            let present = matches!(sink.field(object, &field.key)?, Some(Value::Boolean(true)));
            sink.begin_object()?;
            sink.name("name")?;
            sink.text(&field.name)?;
            sink.name("value")?;
            tagged(sink, "option", |sink| {
                sink.name("value")?;
                if present {
                    project_clock_shell(sink)
                } else {
                    sink.scalar(Scalar::Null)
                }
            })?;
            sink.end_object()?;
        }
        sink.end_array()
    })
}

/// FR-230's present clock shell: a Clock tuple with no data components.
fn project_clock_shell<S: ValueSink>(sink: &mut S) -> Result<(), S::Error> {
    tagged(sink, "tuple", |sink| {
        sink.name("name")?;
        sink.text("Clock")?;
        sink.name("components")?;
        sink.begin_array(ArrayOrder::Retained)?;
        sink.end_array()
    })
}

pub(super) fn project_model_state<S: ValueSink>(
    sink: &mut S,
    model: &StateModel,
    state: &ModelState,
    typing: Typing<'_>,
) -> Result<(), S::Error> {
    tagged(sink, "simulation-state", |sink| {
        sink.name("semantic")?;
        project_semantic(sink, model, state, typing)?;
        for name in [
            "control",
            "queues",
            "roles",
            "observations",
            "bounds",
            "memory",
        ] {
            sink.name(name)?;
            map(sink, ArrayOrder::Retained, |_| Ok(()))?;
        }
        Ok(())
    })
}

fn declaration<S: ValueSink>(sink: &mut S, key: &NodeKey) -> Result<(), S::Error> {
    sink.scalar(Scalar::DeclarationNode(key))
}

pub(super) fn project_effective<S: StructuralSink>(
    sink: &mut S,
    id: &EffectiveId,
) -> Result<(), S::Error> {
    let chain = id.owner_chain();
    for original in chain.iter().rev() {
        sink.begin_object()?;
        sink.name("original")?;
        sink.begin_object()?;
        sink.name("node")?;
        sink.text(original.node())?;
        sink.name("package")?;
        sink.text(original.package())?;
        sink.end_object()?;
        sink.name("owner_effective_type")?;
    }
    // Structural encoders need a null event for the root owner.
    sink.null()?;
    for _ in chain {
        sink.end_object()?;
    }
    Ok(())
}

pub(super) fn write_effective<S: Sink + ?Sized>(
    writer: &mut Writer<'_, S>,
    id: &EffectiveId,
) -> Result<(), Error> {
    project_effective(&mut ReferenceWriter { writer }, id)
}

pub(super) fn project_declaration<S: StructuralSink>(
    sink: &mut S,
    key: DeclarationRecord<'_>,
) -> Result<(), S::Error> {
    sink.begin_object()?;
    for (name, value) in super::declaration_members(key) {
        sink.name(name)?;
        sink.text(value)?;
    }
    sink.end_object()
}
pub(super) fn write_declaration<S: Sink + ?Sized>(
    writer: &mut Writer<'_, S>,
    key: DeclarationRecord<'_>,
) -> Result<(), Error> {
    project_declaration(&mut ReferenceWriter { writer }, key)
}

pub(super) fn project_reference<S: StructuralSink>(
    sink: &mut S,
    source: &ObjectReference,
) -> Result<(), S::Error> {
    project_reference_parts(
        sink,
        |sink| sink.node(source.universe().root_type()),
        source.object_type(),
        source.object().as_str(),
    )
}

/// Both an admitted reference and a borrowed model-universe view use these
/// fields; the sink owns admission and emission of the universe itself.
pub(super) fn project_reference_parts<S: StructuralSink>(
    sink: &mut S,
    universe: impl FnOnce(&mut S) -> Result<(), S::Error>,
    object_type: &EffectiveId,
    identity: &str,
) -> Result<(), S::Error> {
    tagged(sink, "reference", |sink| {
        sink.name("universe")?;
        universe(sink)?;
        sink.name("object_type")?;
        sink.effective(object_type)?;
        sink.name("identity")?;
        sink.text(identity)
    })
}

pub(super) fn project_unit<S: ValueSink>(sink: &mut S, source: &UnitId) -> Result<(), S::Error> {
    sink.begin_object()?;
    if let Some(node) = source.declared_key() {
        sink.name("declared")?;
        sink.scalar(Scalar::Node(node))?;
    } else {
        sink.name("terms")?;
        sink.begin_array(ArrayOrder::Retained)?;
        for (node, exponent) in source.compound_terms().unwrap_or_default() {
            sink.visit()?;
            sink.begin_object()?;
            sink.name("unit")?;
            sink.scalar(Scalar::Node(node))?;
            sink.name("exponent")?;
            sink.scalar(Scalar::Integer(exponent))?;
            sink.end_object()?;
        }
        sink.end_array()?;
    }
    sink.end_object()
}

enum ValueTask<'a> {
    Value(&'a Value, bool),
    Scalar(Scalar<'a>),
    Name(&'static str),
    BeginObject,
    EndObject,
    BeginArray(ArrayOrder),
    EndArray,
}
fn pending_slot<'a>(value: &'a FieldValue, normalize: bool) -> ValueTask<'a> {
    match value {
        FieldValue::Present(value) => ValueTask::Value(value, normalize),
        FieldValue::Absent | FieldValue::Null => ValueTask::Scalar(Scalar::Null),
    }
}

pub(super) fn project_value<'a, S: ValueSink>(
    sink: &mut S,
    value: &'a Value,
    typing: Typing<'a>,
    placeholder: Option<&Placeholder<'_>>,
    normalize_decimal: bool,
) -> Result<(), S::Error> {
    let mut pending = Vec::new();
    sink.pending(&mut pending, ValueTask::Value(value, normalize_decimal))?;
    while let Some(task) = pending.pop() {
        sink.visit()?;
        let (value, normalize_decimal) = match task {
            ValueTask::Value(value, normalize) => (value, normalize),
            ValueTask::Scalar(value) => {
                sink.scalar(value)?;
                continue;
            }
            ValueTask::Name(name) => {
                sink.name(name)?;
                continue;
            }
            ValueTask::BeginObject => {
                sink.begin_object()?;
                continue;
            }
            ValueTask::EndObject => {
                sink.end_object()?;
                continue;
            }
            ValueTask::BeginArray(order) => {
                sink.begin_array(order)?;
                continue;
            }
            ValueTask::EndArray => {
                sink.end_array()?;
                continue;
            }
        };
        match value {
            Value::Boolean(value) => tagged(sink, "boolean", |sink| {
                sink.name("value")?;
                sink.scalar(Scalar::Boolean(*value))
            }),
            Value::Integer(value) => tagged(sink, "integer", |sink| {
                sink.name("value")?;
                sink.scalar(Scalar::Integer(value))
            }),
            Value::Rational(value) => tagged(sink, "rational", |sink| {
                sink.name("numerator")?;
                sink.scalar(Scalar::Integer(value.numerator()))?;
                sink.name("denominator")?;
                sink.scalar(Scalar::Integer(value.denominator()))
            }),
            Value::Quantity(value) => tagged(sink, "rational", |sink| {
                sink.name("numerator")?;
                sink.scalar(Scalar::Integer(value.magnitude().numerator()))?;
                sink.name("denominator")?;
                sink.scalar(Scalar::Integer(value.magnitude().denominator()))?;
                sink.name("unit")?;
                sink.scalar(Scalar::UnitText(value.unit()))
            }),
            Value::Float(value) => {
                let (tag, digits) = match value.width() {
                    IeeeWidth::Binary32 => ("float32", 8),
                    IeeeWidth::Binary64 => ("float64", 16),
                };
                tagged(sink, tag, |sink| {
                    sink.name("bits")?;
                    sink.scalar(Scalar::Hex {
                        bits: value.bits(),
                        digits,
                    })
                })
            }
            Value::Text(value) => tagged(sink, "text", |sink| {
                sink.name("value")?;
                sink.scalar(Scalar::Text(value.retained()))
            }),
            Value::String(value) => tagged(sink, "text", |sink| {
                sink.name("value")?;
                sink.scalar(Scalar::Text(value.as_str()))
            }),
            Value::Option(value) => {
                sink.pending(&mut pending, ValueTask::EndObject)?;
                sink.pending(&mut pending, ValueTask::Scalar(Scalar::Text("option")))?;
                sink.pending(&mut pending, ValueTask::Name("type"))?;
                sink.pending(
                    &mut pending,
                    value
                        .payload()
                        .map_or(ValueTask::Scalar(Scalar::Null), |value| {
                            ValueTask::Value(value, normalize_decimal)
                        }),
                )?;
                sink.pending(&mut pending, ValueTask::Name("value"))?;
                sink.pending(&mut pending, ValueTask::BeginObject)?;
                Ok(())
            }
            Value::Collection(value) => {
                let (tag, order) = match value.collection_type().kind() {
                    CollectionKind::Sequence => ("sequence", ArrayOrder::Retained),
                    CollectionKind::Set => ("set", ArrayOrder::CanonicalBytes),
                    CollectionKind::Bag => ("bag", ArrayOrder::CanonicalBytes),
                    CollectionKind::OrderedSet => ("ordered-set", ArrayOrder::Retained),
                };
                let normalize_elements =
                    normalize_decimal || matches!(order, ArrayOrder::CanonicalBytes);
                sink.pending(&mut pending, ValueTask::EndObject)?;
                sink.pending(&mut pending, ValueTask::Scalar(Scalar::Text(tag)))?;
                sink.pending(&mut pending, ValueTask::Name("type"))?;
                sink.pending(&mut pending, ValueTask::EndArray)?;
                for value in value.elements().iter().rev() {
                    sink.pending(&mut pending, ValueTask::Value(value, normalize_elements))?;
                }
                sink.pending(&mut pending, ValueTask::BeginArray(order))?;
                sink.pending(&mut pending, ValueTask::Name("elements"))?;
                sink.pending(&mut pending, ValueTask::BeginObject)?;
                Ok(())
            }

            Value::Reference(value) => match placeholder.and_then(|replace| replace(value)) {
                Some(value) => sink.scalar(Scalar::Replacement(value)),
                None => project_reference(sink, value),
            },
            Value::Decimal(value) => {
                let value = if normalize_decimal {
                    value.normalized()
                } else {
                    value.representation()
                };
                tagged(sink, "decimal", |sink| {
                    sink.name("coefficient")?;
                    sink.scalar(Scalar::Integer(value.coefficient()))?;
                    sink.name("scale")?;
                    sink.scalar(Scalar::Scale(value.scale()))
                })
            }
            Value::Uuid(value) => tagged(sink, "uuid", |sink| {
                sink.name("value")?;
                sink.scalar(Scalar::Uuid(value))
            }),
            Value::Timestamp(value) => tagged(sink, "timestamp", |sink| {
                sink.name("nanoseconds")?;
                sink.scalar(Scalar::Timestamp(value))
            }),
            Value::Enum(value) => {
                let mut found = None;
                'declarations: for declared in typing.variants {
                    sink.visit()?;
                    for case in declared.cases() {
                        if sink.variant(case.variant_ref(), value.variant())? {
                            found = Some((declared, case));
                            break 'declarations;
                        }
                    }
                }
                match found {
                    Some((declared, case)) => tagged(sink, "enum", |sink| {
                        sink.name("name")?;
                        declaration(sink, declared.node_ref())?;
                        sink.name("member")?;
                        sink.scalar(Scalar::Text(case.name()))
                    }),
                    None => sink.scalar(Scalar::Unencodable("an undeclared enum value")),
                }
            }
            Value::Population(_) => sink.scalar(Scalar::Unencodable("a population value")),
            Value::Composite(value) => {
                match match typing.types {
                    Some(types) => sink.composite(types, value.declaration())?,
                    None => None,
                }
                .map(CompositeDeclaration::shape)
                {
                    Some(CompositeShape::Record(fields)) if fields.len() == value.slots().len() => {
                        sink.pending(&mut pending, ValueTask::EndObject)?;
                        sink.pending(&mut pending, ValueTask::Scalar(Scalar::Text("record")))?;
                        sink.pending(&mut pending, ValueTask::Name("type"))?;
                        sink.pending(&mut pending, ValueTask::EndArray)?;
                        for (field, value) in fields.iter().zip(value.slots()).rev() {
                            sink.pending(&mut pending, ValueTask::EndObject)?;
                            sink.pending(&mut pending, pending_slot(value, normalize_decimal))?;
                            sink.pending(&mut pending, ValueTask::Name("value"))?;
                            sink.pending(
                                &mut pending,
                                ValueTask::Scalar(Scalar::Text(field.name())),
                            )?;
                            sink.pending(&mut pending, ValueTask::Name("name"))?;
                            sink.pending(&mut pending, ValueTask::BeginObject)?;
                        }
                        sink.pending(&mut pending, ValueTask::BeginArray(ArrayOrder::Retained))?;
                        sink.pending(&mut pending, ValueTask::Name("fields"))?;
                        sink.pending(
                            &mut pending,
                            ValueTask::Scalar(Scalar::DeclarationNode(value.declaration())),
                        )?;
                        sink.pending(&mut pending, ValueTask::Name("name"))?;
                        sink.pending(&mut pending, ValueTask::BeginObject)?;
                        Ok(())
                    }
                    Some(CompositeShape::Tuple(_)) => {
                        sink.pending(&mut pending, ValueTask::EndObject)?;
                        sink.pending(&mut pending, ValueTask::Scalar(Scalar::Text("tuple")))?;
                        sink.pending(&mut pending, ValueTask::Name("type"))?;
                        sink.pending(&mut pending, ValueTask::EndArray)?;
                        for value in value.slots().iter().rev() {
                            sink.pending(&mut pending, pending_slot(value, normalize_decimal))?;
                        }
                        sink.pending(&mut pending, ValueTask::BeginArray(ArrayOrder::Retained))?;
                        sink.pending(&mut pending, ValueTask::Name("components"))?;
                        sink.pending(
                            &mut pending,
                            ValueTask::Scalar(Scalar::DeclarationNode(value.declaration())),
                        )?;
                        sink.pending(&mut pending, ValueTask::Name("name"))?;
                        sink.pending(&mut pending, ValueTask::BeginObject)?;
                        Ok(())
                    }
                    Some(CompositeShape::Record(_)) => sink.scalar(Scalar::Unencodable(
                        "a record whose slots are not its declared fields",
                    )),
                    Some(CompositeShape::Union(_)) | None => {
                        sink.scalar(Scalar::Unencodable(if typing.types.is_some() {
                            "an undeclared record"
                        } else {
                            "a record or tuple value"
                        }))
                    }
                }
            }
            Value::Union(value) => {
                sink.pending(&mut pending, ValueTask::EndObject)?;
                sink.pending(&mut pending, ValueTask::Scalar(Scalar::Text("union")))?;
                sink.pending(&mut pending, ValueTask::Name("type"))?;
                sink.pending(&mut pending, ValueTask::EndArray)?;
                for value in value.payload().iter().rev() {
                    sink.pending(&mut pending, ValueTask::Value(value, normalize_decimal))?;
                }
                sink.pending(&mut pending, ValueTask::BeginArray(ArrayOrder::Retained))?;
                sink.pending(&mut pending, ValueTask::Name("components"))?;
                sink.pending(
                    &mut pending,
                    ValueTask::Scalar(Scalar::Text(value.member().identifier())),
                )?;
                sink.pending(&mut pending, ValueTask::Name("member"))?;
                sink.pending(
                    &mut pending,
                    ValueTask::Scalar(Scalar::DeclarationNode(value.declaration())),
                )?;
                sink.pending(&mut pending, ValueTask::Name("name"))?;
                sink.pending(&mut pending, ValueTask::BeginObject)?;
                Ok(())
            }
        }?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use quire_exact::ValueType;

    use super::*;

    fn key(node: &str) -> DeclarationKey {
        DeclarationKey {
            package: "test/clock-owner".to_owned(),
            node: node.to_owned(),
        }
    }

    fn field(node: &str) -> StateField {
        StateField {
            key: key(node),
            name: "same visible name".to_owned(),
            value_type: ValueType::Boolean,
            optional: false,
        }
    }

    #[ix_trace_rs::trace("TC-471", "TC-685", "FR-120-AC-4")]
    #[test]
    fn clock_shell_fields_match_exact_owner_and_declaration_keys() {
        let owner = key("Owner");
        let other = key("Other");
        let count = field("Owner/count");
        let bit = field("Owner/bit");
        let ordinary = field("Owner/ordinary");
        let counts = [(owner.clone(), count.clone(), 0, Some(2))];
        let bits = [(owner.clone(), bit.clone())];
        let typing = Typing::none().with_clock_members(&counts, &bits);
        let mut sink = Retained::default();
        assert!(synthetic_field(&mut sink, &owner, &count, typing).unwrap());
        assert!(synthetic_field(&mut sink, &owner, &bit, typing).unwrap());
        assert!(!synthetic_field(&mut sink, &other, &count, typing).unwrap());
        assert!(!synthetic_field(&mut sink, &other, &bit, typing).unwrap());
        assert!(!synthetic_field(&mut sink, &owner, &ordinary, typing).unwrap());
        assert_eq!(ordinary.name, count.name);
        assert_eq!(ordinary.name, bit.name);
    }
}
