// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-098: a replay argument as a kernel value of its parameter's declared
//! type.
//!
//! A [`WitnessValue`] tree is walked against the recompiled package's type
//! environment on an explicit heap stack, never the native stack in
//! proportion to its depth (ADR-030 D-1), and each node is built through the
//! kernel's value constructors. A value the declared type does not admit, in
//! shape, declaration, member, arity, range, length, scale or cardinality,
//! refuses `WrongValueKind` naming the parameter's position, before any
//! call and with no charge: the conversion runs over its own unlimited meter.
//! The request's accounting limits bound it instead: a decimal quantity's
//! scale expansion is checked against `scale_expansion`, and the caller
//! checks the converted occurrence and node counts against
//! `value_occurrences` and `work_units`.

use qsl_foundation::diagnostic::InternalFault;
use qsl_package::CheckedPackage;
use qsl_semantics::check::NominalNode;
use qsl_semantics::value::enumeration::mint_variant_id;
use quire_exact::{
    admit_text, form_collection, from_admitted_slots, ChargePoint, CollectionKind, CollectionType,
    Decimal, EnumMember, FieldValue, IeeeValue, IeeeWidth, Incomplete, Integer, LimitKind, Meter,
    NodeKey, ObjectId, ObjectReference, OptionValue, Outcome, Presence, Quantity, Rational,
    ScalarLimits, TextPayload, UniverseId, Value, ValueType,
};
use quire_semantic_value::call::InputRefusal;
use quire_semantic_value::declaration::{CompositeShape, TypeEnvironment};

use super::ReplayRefusal;
use crate::witness::{QuantityMagnitude, WitnessSlot, WitnessValue};

/// The meter a conversion builds values under: no counter limits it.
pub(super) const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

/// Why a conversion stopped.
pub(super) enum Stopped {
    /// The value is refused input, or a broken invariant.
    Refusal(Box<ReplayRefusal>),
    /// An accounting limit of the request stopped it.
    Limit(Box<Incomplete>),
}

impl From<ReplayRefusal> for Stopped {
    fn from(refusal: ReplayRefusal) -> Self {
        Self::Refusal(Box::new(refusal))
    }
}

/// The converted arguments and the count of nodes converted.
pub(super) struct Converted {
    /// One kernel value per argument, in parameter order.
    pub(super) values: Vec<Value>,
    /// The nodes converted over every argument.
    pub(super) nodes: u64,
}

/// An enum declaration of the checked graph: its node key and its members.
pub(super) struct EnumDeclaration {
    key: NodeKey,
    members: Vec<String>,
}

impl EnumDeclaration {
    /// The declared member identifiers.
    pub(super) fn members(&self) -> &[String] {
        &self.members
    }
}

/// Every enum declaration of `package`'s checked graph.
pub(super) fn enum_declarations(package: &CheckedPackage) -> Vec<EnumDeclaration> {
    package
        .graph()
        .semantic_graph()
        .nodes()
        .filter_map(|node| match node.nominal() {
            Some(NominalNode::EnumDeclaration(preimage)) => Some(EnumDeclaration {
                key: node.key(),
                members: preimage.members().to_vec(),
            }),
            _ => None,
        })
        .collect()
}

/// The declaration whose members are exactly `shape`'s variants.
pub(super) fn declaration_of<'e>(
    enums: &'e [EnumDeclaration],
    shape: &quire_exact::EnumShape,
) -> Option<&'e EnumDeclaration> {
    enums.iter().find(|candidate| {
        candidate.members.len() == shape.variants().count()
            && candidate
                .members
                .iter()
                .map(|case| mint_variant_id(candidate.key, case))
                .eq(shape.variants())
    })
}

/// One slot's state while a record is assembled.
#[derive(Clone, Copy)]
enum Slot {
    Present,
    Absent,
    Null,
}

/// One pending step of the conversion.
enum Step<'a> {
    Convert {
        value: &'a WitnessValue,
        declared: &'a ValueType,
    },
    Option {
        payload_type: &'a ValueType,
        present: bool,
    },
    Composite {
        key: NodeKey,
        slots: Vec<Slot>,
    },
    Collection {
        declared: &'a CollectionType,
        count: usize,
    },
}

/// The conversion of one package's arguments.
struct Converter<'a> {
    types: &'a TypeEnvironment,
    enums: Vec<EnumDeclaration>,
    limits: &'a ScalarLimits,
    nodes: u64,
}

/// `values[i]` as a kernel value of `declared[i]`, for every parameter.
pub(super) fn convert_arguments(
    package: &CheckedPackage,
    declared: &[ValueType],
    values: &[WitnessValue],
    limits: &ScalarLimits,
) -> Result<Converted, Stopped> {
    let enums = enum_declarations(package);
    let mut converter = Converter {
        types: package.graph().scope().types(),
        enums,
        limits,
        nodes: 0,
    };
    let converted = values
        .iter()
        .zip(declared)
        .enumerate()
        .map(|(parameter, (value, declared))| converter.convert(parameter, value, declared))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Converted {
        values: converted,
        nodes: converter.nodes,
    })
}

fn wrong_kind(parameter: usize) -> Stopped {
    ReplayRefusal::Input(InputRefusal::WrongValueKind { parameter }).into()
}

fn invariant(what: &'static str) -> Stopped {
    ReplayRefusal::Fault(InternalFault::new("replay", what)).into()
}

/// An integer leaf: the scalar `i128` form or an exact integer.
fn integer_of(value: &WitnessValue) -> Option<Integer> {
    match value {
        WitnessValue::Integer(number) => Some(Integer::from(*number)),
        WitnessValue::ExactInteger(number) => Some(number.clone()),
        _ => None,
    }
}

/// A decimal scale as the kernel's `u32`.
fn scale_of(scale: &Integer) -> Option<u32> {
    scale.to_u64().and_then(|scale| u32::try_from(scale).ok())
}

impl<'a> Converter<'a> {
    /// Convert argument `parameter`.
    fn convert(
        &mut self,
        parameter: usize,
        root: &'a WitnessValue,
        declared: &'a ValueType,
    ) -> Result<Value, Stopped> {
        let mut pending = vec![Step::Convert {
            value: root,
            declared,
        }];
        let mut values: Vec<Value> = Vec::new();
        while let Some(step) = pending.pop() {
            match step {
                Step::Convert { value, declared } => {
                    self.nodes = self.nodes.saturating_add(1);
                    self.node(parameter, value, declared, &mut pending, &mut values)?;
                }
                Step::Option {
                    payload_type,
                    present,
                } => {
                    let payload = if present {
                        Some(values.pop().ok_or_else(|| invariant("converted-payload"))?)
                    } else {
                        None
                    };
                    values.push(match payload {
                        None => OptionValue::none(payload_type.clone()),
                        Some(payload) => OptionValue::present(payload_type.clone(), payload)
                            .map_err(|_| wrong_kind(parameter))?,
                    });
                }
                Step::Composite { key, slots } => {
                    let present = slots
                        .iter()
                        .filter(|slot| matches!(slot, Slot::Present))
                        .count();
                    let start = values
                        .len()
                        .checked_sub(present)
                        .ok_or_else(|| invariant("converted-slots"))?;
                    let mut built = values.split_off(start).into_iter();
                    let slots = slots
                        .into_iter()
                        .map(|slot| match slot {
                            Slot::Present => built.next().map(FieldValue::Present),
                            Slot::Absent => Some(FieldValue::Absent),
                            Slot::Null => Some(FieldValue::Null),
                        })
                        .collect::<Option<Box<[_]>>>()
                        .ok_or_else(|| invariant("converted-slots"))?;
                    values.push(from_admitted_slots(key, slots));
                }
                Step::Collection { declared, count } => {
                    let start = values
                        .len()
                        .checked_sub(count)
                        .ok_or_else(|| invariant("converted-elements"))?;
                    let elements = values.split_off(start);
                    let mut meter = Meter::new(UNLIMITED);
                    // Forming checks each element's type and the declared
                    // cardinality, and collapses a duplicate the kernel's
                    // equality relates, which a set or ordered set refuses.
                    match form_collection(declared, elements, &mut meter) {
                        Ok(Outcome::Completed(Value::Collection(formed))) => {
                            let unique = matches!(
                                declared.kind(),
                                CollectionKind::Set | CollectionKind::OrderedSet
                            );
                            if unique && formed.elements().len() != count {
                                return Err(wrong_kind(parameter));
                            }
                            values.push(Value::Collection(formed));
                        }
                        _ => return Err(wrong_kind(parameter)),
                    }
                }
            }
        }
        match (values.pop(), values.is_empty()) {
            (Some(value), true) => Ok(value),
            _ => Err(invariant("one-converted-argument")),
        }
    }

    /// Convert one node: a leaf is pushed as a value; a composite pushes the
    /// steps that convert its parts and then assemble them.
    fn node(
        &mut self,
        parameter: usize,
        value: &'a WitnessValue,
        declared: &'a ValueType,
        pending: &mut Vec<Step<'a>>,
        values: &mut Vec<Value>,
    ) -> Result<(), Stopped> {
        let refuse = || wrong_kind(parameter);
        match (declared, value) {
            (ValueType::Boolean, WitnessValue::Boolean(flag)) => {
                values.push(Value::Boolean(*flag));
            }
            (ValueType::Integer, _) => {
                values.push(Value::Integer(integer_of(value).ok_or_else(refuse)?));
            }
            (ValueType::Int(interval), _) => {
                let number = integer_of(value).ok_or_else(refuse)?;
                if !interval.contains(&number) {
                    return Err(refuse());
                }
                values.push(Value::Integer(number));
            }
            (
                ValueType::Rational(domain),
                WitnessValue::Rational {
                    numerator,
                    denominator,
                },
            ) => {
                let rational =
                    Rational::new(numerator.clone(), denominator.clone()).map_err(|_| refuse())?;
                if !domain.contains(&rational) {
                    return Err(refuse());
                }
                values.push(Value::Rational(rational));
            }
            (ValueType::Decimal(decimal_type), WitnessValue::Decimal { coefficient, scale }) => {
                let decimal =
                    Decimal::new(coefficient.clone(), scale_of(scale).ok_or_else(refuse)?);
                if !decimal_type.contains(&decimal) {
                    return Err(refuse());
                }
                values.push(Value::Decimal(decimal));
            }
            (ValueType::Float(float_type), WitnessValue::Float32(bits)) => {
                if float_type.width() != IeeeWidth::Binary32 {
                    return Err(refuse());
                }
                values.push(Value::Float(IeeeValue::binary32(*bits)));
            }
            (ValueType::Float(float_type), WitnessValue::Float64(bits)) => {
                if float_type.width() != IeeeWidth::Binary64 {
                    return Err(refuse());
                }
                values.push(Value::Float(IeeeValue::binary64(*bits)));
            }
            (
                ValueType::Quantity(unit),
                WitnessValue::Quantity {
                    magnitude,
                    unit: wire,
                },
            ) => {
                if wire != unit.as_bytes() {
                    return Err(refuse());
                }
                let magnitude = self.magnitude(parameter, magnitude)?;
                values.push(Value::Quantity(Quantity::new(magnitude, *unit)));
            }
            (ValueType::Text(text_type), WitnessValue::Text(text)) => {
                let payload = TextPayload::from_utf8(text.as_bytes()).map_err(|_| refuse())?;
                let mut meter = Meter::new(UNLIMITED);
                match admit_text(&payload, text_type, &mut meter) {
                    Outcome::Completed(admitted) => values.push(Value::Text(admitted)),
                    _ => return Err(refuse()),
                }
            }
            (
                ValueType::Enum(shape),
                WitnessValue::Enum {
                    declaration,
                    member,
                },
            ) => {
                let declaration_key = declaration.as_bytes();
                let owner = declaration_of(&self.enums, shape)
                    .filter(|owner| owner.key.as_bytes() == declaration_key)
                    .ok_or_else(refuse)?;
                if !owner.members.contains(member) {
                    return Err(refuse());
                }
                let variant = mint_variant_id(owner.key, member);
                let rank = shape.rank(variant).ok_or_else(refuse)?;
                values.push(Value::Enum(EnumMember::new(variant, rank)));
            }
            (
                ValueType::Reference(object_type),
                WitnessValue::Reference {
                    universe,
                    object_type: wire_type,
                    identity,
                },
            ) => {
                if wire_type != object_type.as_bytes() {
                    return Err(refuse());
                }
                let identity = String::from_utf8(identity.clone()).map_err(|_| refuse())?;
                let object = ObjectId::new(identity).map_err(|_| refuse())?;
                values.push(Value::Reference(ObjectReference::new(
                    UniverseId::from_digest(*universe),
                    *object_type,
                    object,
                )));
            }
            (ValueType::Option(payload_type), WitnessValue::Option(inner)) => {
                pending.push(Step::Option {
                    payload_type,
                    present: inner.is_some(),
                });
                if let Some(inner) = inner {
                    pending.push(Step::Convert {
                        value: inner,
                        declared: payload_type,
                    });
                }
            }
            (
                ValueType::Composite(key),
                WitnessValue::Record {
                    declaration,
                    fields,
                },
            ) => {
                let types: &'a TypeEnvironment = self.types;
                let record = types.composite(*key).ok_or_else(refuse)?;
                let CompositeShape::Record(declared_fields) = record.shape() else {
                    return Err(refuse());
                };
                if declaration.as_bytes() != key.as_bytes() || fields.len() != declared_fields.len()
                {
                    return Err(refuse());
                }
                let mut slots = Vec::with_capacity(fields.len());
                let mut present = Vec::new();
                for (field, declared_field) in fields.iter().zip(declared_fields) {
                    if field.name != declared_field.name() {
                        return Err(refuse());
                    }
                    let optional = declared_field.presence() == Presence::Optional;
                    match &field.slot {
                        WitnessSlot::Present(inner) => {
                            slots.push(Slot::Present);
                            present.push(Step::Convert {
                                value: inner,
                                declared: declared_field.value_type(),
                            });
                        }
                        WitnessSlot::Absent if optional => slots.push(Slot::Absent),
                        WitnessSlot::Null if optional => slots.push(Slot::Null),
                        WitnessSlot::Absent | WitnessSlot::Null => return Err(refuse()),
                    }
                }
                pending.push(Step::Composite { key: *key, slots });
                pending.extend(present.into_iter().rev());
            }
            (
                ValueType::Composite(key),
                WitnessValue::Tuple {
                    declaration,
                    components,
                },
            ) => {
                let types: &'a TypeEnvironment = self.types;
                let tuple = types.composite(*key).ok_or_else(refuse)?;
                let CompositeShape::Tuple(positions) = tuple.shape() else {
                    return Err(refuse());
                };
                if declaration.as_bytes() != key.as_bytes() || components.len() != positions.len() {
                    return Err(refuse());
                }
                pending.push(Step::Composite {
                    key: *key,
                    slots: vec![Slot::Present; components.len()],
                });
                pending.extend(
                    components
                        .iter()
                        .zip(positions)
                        .rev()
                        .map(|(value, declared)| Step::Convert { value, declared }),
                );
            }
            (ValueType::Collection(declared), _) => {
                let (kind, elements) = match value {
                    WitnessValue::Sequence(elements) => (CollectionKind::Sequence, elements),
                    WitnessValue::OrderedSet(elements) => (CollectionKind::OrderedSet, elements),
                    WitnessValue::Set(elements) => (CollectionKind::Set, elements),
                    WitnessValue::Bag(elements) => (CollectionKind::Bag, elements),
                    _ => return Err(refuse()),
                };
                if declared.kind() != kind {
                    return Err(refuse());
                }
                pending.push(Step::Collection {
                    declared,
                    count: elements.len(),
                });
                pending.extend(elements.iter().rev().map(|value| Step::Convert {
                    value,
                    declared: declared.element(),
                }));
            }
            // A declared type no witness value is a value of (a population),
            // and a value no declared type of this package admits (a union:
            // the checker has no union type), refuse; so does every pairing
            // of a declared type with a value of another form.
            _ => return Err(refuse()),
        }
        Ok(())
    }

    /// A quantity's magnitude as the kernel's exact rational. A decimal
    /// magnitude `c`/`10^s` expands a power of ten, so its scale is checked
    /// against the request's `scale_expansion` limit first.
    fn magnitude(
        &self,
        parameter: usize,
        magnitude: &QuantityMagnitude,
    ) -> Result<Rational, Stopped> {
        match magnitude {
            QuantityMagnitude::Rational {
                numerator,
                denominator,
            } => Rational::new(numerator.clone(), denominator.clone())
                .map_err(|_| wrong_kind(parameter)),
            QuantityMagnitude::Decimal { coefficient, scale } => {
                let exponent = scale.to_u64().ok_or_else(|| wrong_kind(parameter))?;
                if exponent > self.limits.scale_expansion {
                    return Err(Stopped::Limit(Box::new(Incomplete {
                        limit_kind: LimitKind::ScaleExpansion,
                        limit: self.limits.scale_expansion,
                        consumed: 0,
                        next_charge: Integer::from(exponent),
                        charge_point: ChargePoint::DecimalScaleExpansion,
                    })));
                }
                Rational::new(coefficient.clone(), Integer::power_of_ten(exponent))
                    .map_err(|_| wrong_kind(parameter))
            }
        }
    }
}
