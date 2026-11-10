// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120 / QSpec FR-181's borrowed typed-value projection, independent of
//! retained-tree storage.

use core::convert::Infallible;

use quire_canonical::{Error, Limits, Sink, Writer};
use quire_exact::{
    CollectionKind, CompositeValue, EffectiveId, FieldValue, IeeeWidth, Integer, NodeKey,
    ObjectReference, Timestamp, UnitId, Uuid, Value, VariantId,
};
use quire_semantic_value::declaration::{CompositeDeclaration, CompositeShape, TypeEnvironment};

use super::{DeclarationRecord, Enc, Placeholder, Typing};

/// Source scalars have not been copied or formatted when the sink receives them.
pub(super) enum Scalar<'a> {
    Null,
    Boolean(bool),
    Text(&'a str),
    Integer(&'a Integer),
    Hex { bits: u64, digits: usize },
    Scale(u32),
    Uuid(&'a Uuid),
    Timestamp(&'a Timestamp),
    Node(&'a NodeKey),
    Effective(&'a EffectiveId),
    Declaration(DeclarationRecord<'a>),
    UnitText(&'a UnitId),
    Replacement(Enc),
    Unencodable(&'static str),
}

#[derive(Clone, Copy)]
pub(super) enum ArrayOrder {
    Retained,
    CanonicalBytes,
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
}

/// Each sink owns its storage, scalar rendering, metadata lookup and ordering.
/// The projection owns the complete field schema and traversal.
pub(super) trait ValueSink: StructuralSink {
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
}

enum Frame {
    Object {
        members: Vec<(&'static str, Enc)>,
        name: Option<&'static str>,
    },
    Array {
        items: Vec<Enc>,
        order: ArrayOrder,
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
            Some(Frame::Object { members, name }) => {
                let name = name
                    .take()
                    .expect("the private projection names every object value");
                members.push((name, value));
            }
            Some(Frame::Array { items, .. }) => items.push(value),
            None => {
                assert!(self.root.is_none(), "the private projection emits one root");
                self.root = Some(value);
            }
        }
    }

    fn finish(self) -> Enc {
        assert!(
            self.stack.is_empty(),
            "the private projection closes every container"
        );
        self.root.expect("the private projection emits a root")
    }
}

impl ValueSink for Retained {
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
        });
        Ok(())
    }

    fn end_array(&mut self) -> Result<(), Self::Error> {
        let Some(Frame::Array { mut items, order }) = self.stack.pop() else {
            panic!("the private projection closes an array");
        };
        if matches!(order, ArrayOrder::CanonicalBytes) {
            items.sort_by_cached_key(|value| {
                quire_canonical::to_vec(value, Limits::new(u64::MAX)).unwrap_or_default()
            });
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
}

impl StructuralSink for Retained {
    type Error = Infallible;

    fn begin_object(&mut self) -> Result<(), Self::Error> {
        self.stack.push(Frame::Object {
            members: Vec::new(),
            name: None,
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
        }) = self.stack.pop()
        else {
            panic!("the private projection closes a complete object");
        };
        self.push(Enc::Object(members));
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

fn declaration<S: ValueSink>(sink: &mut S, key: &NodeKey) -> Result<(), S::Error> {
    sink.scalar(DeclarationRecord::of_node(key).map_or(
        Scalar::Unencodable("a declaration identity"),
        Scalar::Declaration,
    ))
}

pub(super) fn project_reference<S: StructuralSink>(
    sink: &mut S,
    source: &ObjectReference,
) -> Result<(), S::Error> {
    tagged(sink, "reference", |sink| {
        sink.name("universe")?;
        sink.node(source.universe().root_type())?;
        sink.name("object_type")?;
        sink.effective(source.object_type())?;
        sink.name("identity")?;
        sink.text(source.object().as_str())
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

fn slot<S: ValueSink>(
    sink: &mut S,
    value: &FieldValue,
    typing: Typing<'_>,
    placeholder: Option<&Placeholder<'_>>,
    normalize_decimal: bool,
) -> Result<(), S::Error> {
    match value {
        FieldValue::Present(inner) => {
            project_value(sink, inner, typing, placeholder, normalize_decimal)
        }
        FieldValue::Absent | FieldValue::Null => sink.scalar(Scalar::Null),
    }
}

fn composite<S: ValueSink>(
    sink: &mut S,
    value: &CompositeValue,
    types: &TypeEnvironment,
    typing: Typing<'_>,
    placeholder: Option<&Placeholder<'_>>,
    normalize_decimal: bool,
) -> Result<(), S::Error> {
    match sink
        .composite(types, value.declaration())?
        .map(CompositeDeclaration::shape)
    {
        Some(CompositeShape::Record(fields)) if fields.len() == value.slots().len() => {
            tagged(sink, "record", |sink| {
                sink.name("name")?;
                declaration(sink, value.declaration())?;
                sink.name("fields")?;
                sink.begin_array(ArrayOrder::Retained)?;
                for (field, value) in fields.iter().zip(value.slots()) {
                    sink.visit()?;
                    sink.begin_object()?;
                    sink.name("name")?;
                    sink.scalar(Scalar::Text(field.name()))?;
                    sink.name("value")?;
                    slot(sink, value, typing, placeholder, normalize_decimal)?;
                    sink.end_object()?;
                }
                sink.end_array()
            })
        }
        Some(CompositeShape::Tuple(_)) => tagged(sink, "tuple", |sink| {
            sink.name("name")?;
            declaration(sink, value.declaration())?;
            sink.name("components")?;
            sink.begin_array(ArrayOrder::Retained)?;
            for value in value.slots() {
                slot(sink, value, typing, placeholder, normalize_decimal)?;
            }
            sink.end_array()
        }),
        Some(CompositeShape::Record(_)) => sink.scalar(Scalar::Unencodable(
            "a record whose slots are not its declared fields",
        )),
        Some(CompositeShape::Union(_)) | None => {
            sink.scalar(Scalar::Unencodable("an undeclared record"))
        }
    }
}

pub(super) fn project_value<S: ValueSink>(
    sink: &mut S,
    value: &Value,
    typing: Typing<'_>,
    placeholder: Option<&Placeholder<'_>>,
    normalize_decimal: bool,
) -> Result<(), S::Error> {
    sink.visit()?;
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
        Value::Option(value) => tagged(sink, "option", |sink| {
            sink.name("value")?;
            match value.payload() {
                Some(value) => project_value(sink, value, typing, placeholder, normalize_decimal),
                None => sink.scalar(Scalar::Null),
            }
        }),
        Value::Collection(value) => {
            let (tag, order) = match value.collection_type().kind() {
                CollectionKind::Sequence => ("sequence", ArrayOrder::Retained),
                CollectionKind::Set => ("set", ArrayOrder::CanonicalBytes),
                CollectionKind::Bag => ("bag", ArrayOrder::CanonicalBytes),
                CollectionKind::OrderedSet => ("ordered-set", ArrayOrder::Retained),
            };
            let normalize_elements =
                normalize_decimal || matches!(order, ArrayOrder::CanonicalBytes);
            tagged(sink, tag, |sink| {
                sink.name("elements")?;
                sink.begin_array(order)?;
                for value in value.elements() {
                    project_value(sink, value, typing, placeholder, normalize_elements)?;
                }
                sink.end_array()
            })
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
        Value::Composite(value) => match typing.types {
            Some(types) => composite(sink, value, types, typing, placeholder, normalize_decimal),
            None => sink.scalar(Scalar::Unencodable("a record or tuple value")),
        },
        Value::Union(value) => tagged(sink, "union", |sink| {
            sink.name("name")?;
            declaration(sink, value.declaration())?;
            sink.name("member")?;
            sink.scalar(Scalar::Text(value.member().identifier()))?;
            sink.name("components")?;
            sink.begin_array(ArrayOrder::Retained)?;
            for value in value.payload() {
                project_value(sink, value, typing, placeholder, normalize_decimal)?;
            }
            sink.end_array()
        }),
    }
}
