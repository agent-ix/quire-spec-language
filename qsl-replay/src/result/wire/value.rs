// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-269: a deciding element's typed value encoding, one tagged form per
//! kernel value kind, and the value types an `Option` or collection carries.
//!
//! Each form holds the kind's own payload exactly: integers as canonical
//! decimal strings, a rational in lowest terms, a decimal as coefficient and
//! scale, a float as its bit pattern in decimal, text as its admitted
//! payload. Every number that can exceed 2^53 is a decimal string, since the
//! one encoder, `quire-canonical`'s, refuses a larger JSON number. The reader
//! mints no declaration identity (ADR-013 O-04, O-05): a record or tuple
//! declaration, a declared unit and an object type each resolve by lookup
//! among the identities the checked package admitted, and a name the package
//! does not hold refuses.
//!
//! One encoder and one reader serve a value of any depth. A form that holds
//! another value or type is written from an explicit heap stack through
//! `quire-canonical`'s event [`Writer`], each form with a fixed shape through
//! its [`FixedShape`] serde encoding. The document is read by
//! `quire_canonical::read` into an arena and decoded on an explicit heap
//! stack, its fixed-shape forms through their serde derives.

use std::cell::Cell;

use qsl_package::CheckedPackage;
use quire_canonical::{FixedShape, Limits, Node, NodeRef, Sink, Writer};
use quire_exact::{
    admit_text, form_collection, from_admitted_slots, CardinalityBound, CollectionKind,
    CollectionType, Decimal, DecimalType, EffectiveId, EnumMember, EnumShape, FieldValue,
    FloatType, IeeeValue, IeeeWidth, Integer, IntegerInterval, Meter, ObjectId, ObjectReference,
    OptionValue, Outcome, Presence, Quantity, Rational, RationalDomain, RoundingMode, ScalarLimits,
    TextPayload, TextProfile, TextType, UnitDomain, UnitId, UniverseId, Value, ValueType,
    VariantId,
};
use quire_semantic_value::declaration::{CompositeDeclaration, CompositeShape, TypeEnvironment};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use super::{digest, hex, malformed, present, CauseCodecError};

/// The limits a text payload is re-admitted and a collection re-formed
/// under. Both are already bounded by the size of the document they were
/// read from, so no limit here cuts them shorter.
const READER_LIMITS: ScalarLimits = ScalarLimits {
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

/// The checked package's admitted declaration identities, which the reader
/// resolves wire digests against.
#[derive(Clone, Copy)]
pub(super) struct Keys<'a> {
    package: &'a CheckedPackage,
}

impl<'a> Keys<'a> {
    pub(super) fn new(package: &'a CheckedPackage) -> Self {
        Self { package }
    }

    /// The admitted record or tuple declaration whose key is `text`.
    fn composite(
        &self,
        text: &str,
        member: &'static str,
    ) -> Result<&'a CompositeDeclaration, CauseCodecError> {
        let bytes = digest(text, member)?;
        self.types()
            .composites()
            .find(|declaration| declaration.key().as_bytes() == &bytes)
            .ok_or(CauseCodecError::Unresolved(member))
    }

    fn types(&self) -> &'a TypeEnvironment {
        self.package.graph().scope().types()
    }

    /// `slots` as a value of `declaration`, refusing slots of another shape
    /// (QSpec FR-351-AC-5): a record's slots are its fields in order, each
    /// present and admitted by the field's type, or absent or null only for
    /// an optional field; a tuple's are its positions, each present and
    /// admitted.
    fn shaped(
        &self,
        declaration: &CompositeDeclaration,
        slots: Box<[FieldValue]>,
    ) -> Result<Value, CauseCodecError> {
        let types = self.types();
        let fits = match declaration.shape() {
            CompositeShape::Record(fields) => {
                fields.len() == slots.len()
                    && fields.iter().zip(slots.iter()).all(|(field, slot)| match slot {
                        FieldValue::Present(value) => types.admits(field.value_type(), value),
                        FieldValue::Absent | FieldValue::Null => {
                            field.presence() == Presence::Optional
                        }
                    })
            }
            CompositeShape::Tuple(positions) => {
                positions.len() == slots.len()
                    && positions.iter().zip(slots.iter()).all(|(position, slot)| {
                        matches!(slot, FieldValue::Present(value) if types.admits(position, value))
                    })
            }
        };
        if !fits {
            return Err(CauseCodecError::Value("slots"));
        }
        Ok(from_admitted_slots(declaration.key(), slots))
    }

    /// The admitted object type whose effective identity is `text`.
    pub(super) fn object_type(
        &self,
        text: &str,
        member: &'static str,
    ) -> Result<EffectiveId, CauseCodecError> {
        let bytes = digest(text, member)?;
        self.package
            .graph()
            .scope()
            .types()
            .object_types()
            .map(|declaration| declaration.key())
            .find(|key| key.as_bytes() == &bytes)
            .ok_or(CauseCodecError::Unresolved(member))
    }

    /// A declared unit resolves among the package's units; a compound unit
    /// is its digest.
    fn unit(&self, unit: &UnitWire) -> Result<UnitId, CauseCodecError> {
        let bytes = digest(&unit.digest, "unit")?;
        match unit.domain {
            UnitDomainWire::Compound => Ok(UnitId::compound(bytes)),
            UnitDomainWire::Declared => self
                .package
                .graph()
                .scope()
                .types()
                .units()
                .ids()
                .find(|id| id.domain() == UnitDomain::Declared && id.as_bytes() == &bytes)
                .ok_or(CauseCodecError::Unresolved("unit")),
        }
    }
}

/// An object reference: universe, object type and object identity.
pub(super) fn reference_of(
    keys: Keys<'_>,
    universe: &str,
    object_type: &str,
    object_identity: String,
) -> Result<ObjectReference, CauseCodecError> {
    Ok(ObjectReference::new(
        UniverseId::from_digest(digest(universe, "universe")?),
        keys.object_type(object_type, "type")?,
        ObjectId::new(object_identity).map_err(|_| CauseCodecError::Empty("object_identity"))?,
    ))
}

/// A canonical decimal integer.
fn integer(text: &str, member: &'static str) -> Result<Integer, CauseCodecError> {
    text.parse::<Integer>()
        .map_err(|_| CauseCodecError::Integer(member))
}

/// A canonical decimal `u64`.
fn unsigned(text: &str, member: &'static str) -> Result<u64, CauseCodecError> {
    let value = text
        .parse::<u64>()
        .map_err(|_| CauseCodecError::Integer(member))?;
    if value.to_string() == text {
        Ok(value)
    } else {
        Err(CauseCodecError::Integer(member))
    }
}

/// A rational in lowest terms with a positive denominator.
#[derive(Serialize, Deserialize, FixedShape)]
#[serde(deny_unknown_fields)]
pub(super) struct RationalWire {
    numerator: String,
    denominator: String,
}

impl RationalWire {
    fn of(rational: &Rational) -> Self {
        Self {
            numerator: rational.numerator().to_string(),
            denominator: rational.denominator().to_string(),
        }
    }

    fn read(self) -> Result<Rational, CauseCodecError> {
        let rational = Rational::new(
            integer(&self.numerator, "numerator")?,
            integer(&self.denominator, "denominator")?,
        )
        .map_err(|_| CauseCodecError::Value("rational"))?;
        // A rational the constructor reduced or re-signed was not written in
        // its canonical form.
        if rational.numerator().to_string() != self.numerator
            || rational.denominator().to_string() != self.denominator
        {
            return Err(CauseCodecError::Value("rational"));
        }
        Ok(rational)
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, FixedShape)]
#[serde(rename_all = "kebab-case")]
pub(super) enum UnitDomainWire {
    Declared,
    Compound,
}

#[derive(Serialize, Deserialize, FixedShape)]
#[serde(deny_unknown_fields)]
pub(super) struct UnitWire {
    domain: UnitDomainWire,
    digest: String,
}

impl UnitWire {
    fn of(unit: UnitId) -> Self {
        Self {
            domain: match unit.domain() {
                UnitDomain::Declared => UnitDomainWire::Declared,
                UnitDomain::Compound => UnitDomainWire::Compound,
            },
            digest: hex(unit.as_bytes()),
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, FixedShape)]
#[serde(rename_all = "kebab-case")]
pub(super) enum WidthWire {
    Binary32,
    Binary64,
}

impl WidthWire {
    fn of(width: IeeeWidth) -> Self {
        match width {
            IeeeWidth::Binary32 => Self::Binary32,
            IeeeWidth::Binary64 => Self::Binary64,
        }
    }

    fn read(self) -> IeeeWidth {
        match self {
            Self::Binary32 => IeeeWidth::Binary32,
            Self::Binary64 => IeeeWidth::Binary64,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, FixedShape)]
#[serde(rename_all = "kebab-case")]
pub(super) enum KindWire {
    Sequence,
    Set,
    Bag,
    OrderedSet,
}

impl KindWire {
    fn of(kind: CollectionKind) -> Self {
        match kind {
            CollectionKind::Sequence => Self::Sequence,
            CollectionKind::Set => Self::Set,
            CollectionKind::Bag => Self::Bag,
            CollectionKind::OrderedSet => Self::OrderedSet,
        }
    }

    fn read(self) -> CollectionKind {
        match self {
            Self::Sequence => CollectionKind::Sequence,
            Self::Set => CollectionKind::Set,
            Self::Bag => CollectionKind::Bag,
            Self::OrderedSet => CollectionKind::OrderedSet,
        }
    }
}

/// An inclusive integer interval.
#[derive(Serialize, Deserialize, FixedShape)]
#[serde(deny_unknown_fields)]
pub(super) struct IntervalWire {
    lower: String,
    upper: String,
}

impl IntervalWire {
    fn of(interval: &IntegerInterval) -> Self {
        Self {
            lower: interval.lower().to_string(),
            upper: interval.upper().to_string(),
        }
    }

    fn read(self) -> Result<IntegerInterval, CauseCodecError> {
        IntegerInterval::new(
            integer(&self.lower, "lower")?,
            integer(&self.upper, "upper")?,
        )
        .map_err(|_| CauseCodecError::Value("interval"))
    }
}

/// A collection's cardinality bound, each end a decimal string.
#[derive(Serialize, Deserialize, FixedShape)]
#[serde(deny_unknown_fields)]
pub(super) struct BoundWire {
    minimum: String,
    maximum: String,
}

impl BoundWire {
    fn of(bound: &CardinalityBound) -> Self {
        Self {
            minimum: bound.minimum().to_string(),
            maximum: bound.maximum().to_string(),
        }
    }

    fn read(self) -> Result<CardinalityBound, CauseCodecError> {
        CardinalityBound::new(
            unsigned(&self.minimum, "minimum")?,
            unsigned(&self.maximum, "maximum")?,
        )
        .map_err(|_| CauseCodecError::Value("bound"))
    }
}

#[derive(Serialize, Deserialize, FixedShape)]
#[serde(deny_unknown_fields)]
pub(super) struct TextTypeWire {
    min: String,
    max: String,
    profile: String,
}

impl TextTypeWire {
    fn of(text_type: &TextType) -> Self {
        Self {
            min: text_type.min().to_string(),
            max: text_type.max().to_string(),
            profile: text_type.profile().as_str().to_owned(),
        }
    }

    fn read(self) -> Result<TextType, CauseCodecError> {
        let profile =
            TextProfile::from_code(&self.profile).ok_or(CauseCodecError::Value("profile"))?;
        TextType::new(
            unsigned(&self.min, "min")?,
            unsigned(&self.max, "max")?,
            profile,
        )
        .map_err(|_| CauseCodecError::Value("text type"))
    }
}

fn rounding(text: &str) -> Result<RoundingMode, CauseCodecError> {
    RoundingMode::from_code(text).ok_or(CauseCodecError::Value("rounding"))
}

/// A value type that holds no other type, by its kind. The forms that hold
/// one, `option` and `collection`, are written and read by
/// [`ElementEncode`] and [`Decoder`].
#[derive(Serialize, Deserialize, FixedShape)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(super) enum TypeLeafWire {
    Boolean,
    Integer,
    Int {
        interval: IntervalWire,
    },
    Rational {
        numerator: IntervalWire,
        denominator: IntervalWire,
    },
    Decimal {
        lower: String,
        upper: String,
        min_scale: String,
        max_scale: String,
        rounding: String,
    },
    Float {
        width: WidthWire,
        rounding: String,
    },
    Quantity {
        unit: UnitWire,
    },
    Text {
        #[serde(rename = "type")]
        text_type: TextTypeWire,
    },
    Enum {
        ordered: bool,
        variants: Vec<String>,
    },
    Composite {
        declaration: String,
    },
    Reference {
        object_type: String,
    },
    Population {
        #[serde(
            default,
            deserialize_with = "present",
            skip_serializing_if = "Option::is_none"
        )]
        maximum: Option<String>,
    },
}

impl TypeLeafWire {
    fn of(value_type: &ValueType) -> Option<Self> {
        Some(match value_type {
            ValueType::Boolean => Self::Boolean,
            ValueType::Integer => Self::Integer,
            ValueType::Int(interval) => Self::Int {
                interval: IntervalWire::of(interval),
            },
            ValueType::Rational(domain) => Self::Rational {
                numerator: IntervalWire::of(domain.numerator()),
                denominator: IntervalWire::of(domain.denominator()),
            },
            ValueType::Decimal(decimal) => Self::Decimal {
                lower: decimal.lower().to_string(),
                upper: decimal.upper().to_string(),
                min_scale: decimal.min_scale().to_string(),
                max_scale: decimal.max_scale().to_string(),
                rounding: decimal.rounding().as_str().to_owned(),
            },
            ValueType::Float(float) => Self::Float {
                width: WidthWire::of(float.width()),
                rounding: float.rounding().as_str().to_owned(),
            },
            ValueType::Quantity(unit) => Self::Quantity {
                unit: UnitWire::of(*unit),
            },
            ValueType::Text(text_type) => Self::Text {
                text_type: TextTypeWire::of(text_type),
            },
            ValueType::Enum(shape) => Self::Enum {
                ordered: shape.is_ordered(),
                variants: shape
                    .variants()
                    .map(|variant| hex(variant.as_bytes()))
                    .collect(),
            },
            ValueType::Composite(declaration) => Self::Composite {
                declaration: hex(declaration.as_bytes()),
            },
            ValueType::Reference(object_type) => Self::Reference {
                object_type: hex(object_type.as_bytes()),
            },
            ValueType::Population(maximum) => Self::Population {
                maximum: maximum.map(|maximum| maximum.to_string()),
            },
            ValueType::Option(_)
            | ValueType::Collection(_)
            | ValueType::Uuid
            | ValueType::Timestamp => return None,
        })
    }

    fn read(self, keys: Keys<'_>) -> Result<ValueType, CauseCodecError> {
        Ok(match self {
            Self::Boolean => ValueType::Boolean,
            Self::Integer => ValueType::Integer,
            Self::Int { interval } => ValueType::Int(interval.read()?),
            Self::Rational {
                numerator,
                denominator,
            } => ValueType::Rational(
                RationalDomain::new(numerator.read()?, denominator.read()?)
                    .map_err(|_| CauseCodecError::Value("rational domain"))?,
            ),
            Self::Decimal {
                lower,
                upper,
                min_scale,
                max_scale,
                rounding: mode,
            } => ValueType::Decimal(
                DecimalType::new(
                    integer(&lower, "lower")?,
                    integer(&upper, "upper")?,
                    unsigned(&min_scale, "min_scale")?,
                    unsigned(&max_scale, "max_scale")?,
                    rounding(&mode)?,
                )
                .map_err(|_| CauseCodecError::Value("decimal type"))?,
            ),
            Self::Float {
                width,
                rounding: mode,
            } => ValueType::Float(FloatType::new(width.read(), rounding(&mode)?)),
            Self::Quantity { unit } => ValueType::Quantity(keys.unit(&unit)?),
            Self::Text { text_type } => ValueType::Text(text_type.read()?),
            Self::Enum { ordered, variants } => {
                let variants = variants
                    .iter()
                    .map(|variant| digest(variant, "variant").map(VariantId::from_digest))
                    .collect::<Result<Vec<_>, _>>()?;
                let shape = EnumShape::new(ordered, variants.iter().copied());
                // A repeated variant collapses in the shape: it was not one
                // variant set.
                if shape.variants().count() != variants.len() {
                    return Err(CauseCodecError::Value("variants"));
                }
                ValueType::Enum(shape)
            }
            Self::Composite { declaration } => {
                ValueType::Composite(keys.composite(&declaration, "declaration")?.key())
            }
            Self::Reference { object_type } => {
                ValueType::Reference(keys.object_type(&object_type, "object_type")?)
            }
            Self::Population { maximum } => ValueType::Population(
                maximum
                    .map(|maximum| unsigned(&maximum, "maximum"))
                    .transpose()?,
            ),
        })
    }
}

/// A value that holds no other value or type, by its kind. The forms that
/// hold one, `option`, `composite` and `collection`, are written and read by
/// [`ElementEncode`] and [`Decoder`].
#[derive(Serialize, Deserialize, FixedShape)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(super) enum ScalarWire {
    Boolean {
        value: bool,
    },
    Integer {
        decimal: String,
    },
    Rational {
        value: RationalWire,
    },
    Decimal {
        coefficient: String,
        scale: u32,
    },
    Float {
        width: WidthWire,
        bits: String,
    },
    Quantity {
        magnitude: RationalWire,
        unit: UnitWire,
    },
    Text {
        #[serde(rename = "type")]
        text_type: TextTypeWire,
        payload: String,
    },
    Enum {
        variant: String,
        rank: u32,
    },
    Reference {
        universe: String,
        #[serde(rename = "type")]
        object_type: String,
        object_identity: String,
    },
}

impl ScalarWire {
    fn of(value: &Value) -> Option<Self> {
        Some(match value {
            Value::Boolean(value) => Self::Boolean { value: *value },
            Value::Integer(integer) => Self::Integer {
                decimal: integer.to_string(),
            },
            Value::Rational(rational) => Self::Rational {
                value: RationalWire::of(rational),
            },
            Value::Decimal(decimal) => Self::Decimal {
                coefficient: decimal.representation().coefficient().to_string(),
                scale: decimal.representation().scale(),
            },
            Value::Float(float) => Self::Float {
                width: WidthWire::of(float.width()),
                bits: float.bits().to_string(),
            },
            Value::Quantity(quantity) => Self::Quantity {
                magnitude: RationalWire::of(quantity.magnitude()),
                unit: UnitWire::of(quantity.unit()),
            },
            Value::Text(text) => Self::Text {
                text_type: TextTypeWire::of(text.text_type()),
                payload: text.payload().as_str().to_owned(),
            },
            Value::Enum(member) => Self::Enum {
                variant: hex(member.variant().as_bytes()),
                rank: member.rank(),
            },
            Value::Reference(reference) => Self::Reference {
                universe: hex(reference.universe().as_bytes()),
                object_type: hex(reference.object_type().as_bytes()),
                object_identity: reference.object().as_str().to_owned(),
            },
            Value::Option(_)
            | Value::Composite(_)
            | Value::Collection(_)
            | Value::Population(_)
            | Value::Uuid(_)
            | Value::Timestamp(_) => return None,
        })
    }

    fn read(self, keys: Keys<'_>) -> Result<Value, CauseCodecError> {
        Ok(match self {
            Self::Boolean { value } => Value::Boolean(value),
            Self::Integer { decimal } => Value::Integer(integer(&decimal, "decimal")?),
            Self::Rational { value } => Value::Rational(value.read()?),
            Self::Decimal { coefficient, scale } => {
                Value::Decimal(Decimal::new(integer(&coefficient, "coefficient")?, scale))
            }
            Self::Float { width, bits } => {
                let bits = unsigned(&bits, "bits")?;
                Value::Float(match width {
                    WidthWire::Binary32 => IeeeValue::binary32(
                        u32::try_from(bits).map_err(|_| CauseCodecError::Value("bits"))?,
                    ),
                    WidthWire::Binary64 => IeeeValue::binary64(bits),
                })
            }
            Self::Quantity { magnitude, unit } => {
                Value::Quantity(Quantity::new(magnitude.read()?, keys.unit(&unit)?))
            }
            Self::Text { text_type, payload } => {
                let text_type = text_type.read()?;
                let payload = TextPayload::from_utf8(payload.as_bytes())
                    .map_err(|_| CauseCodecError::Value("payload"))?;
                let mut meter = Meter::new(READER_LIMITS);
                match admit_text(&payload, &text_type, &mut meter) {
                    Outcome::Completed(text) => Value::Text(text),
                    _ => return Err(CauseCodecError::Value("payload")),
                }
            }
            Self::Enum { variant, rank } => Value::Enum(EnumMember::new(
                VariantId::from_digest(digest(&variant, "variant")?),
                rank,
            )),
            Self::Reference {
                universe,
                object_type,
                object_identity,
            } => Value::Reference(reference_of(
                keys,
                &universe,
                &object_type,
                object_identity,
            )?),
        })
    }
}

/// A deciding element's FR-269 encoding, as the JSON the cause document
/// holds in its place: written by [`encode_element`], and refused when a
/// `quire-canonical` limit or number bound is reached.
pub(super) fn encode_element(value: &Value) -> Result<Box<RawValue>, CauseCodecError> {
    let encode = ElementEncode {
        value,
        unsupported: Cell::new(false),
    };
    match quire_canonical::to_vec(&encode, Limits::new(u64::MAX)) {
        Ok(bytes) => {
            let text =
                String::from_utf8(bytes).map_err(|_| CauseCodecError::Value("canonical text"))?;
            Ok(RawValue::from_string(text)?)
        }
        Err(_) if encode.unsupported.get() => Err(CauseCodecError::UnsupportedElement),
        Err(error) => Err(CauseCodecError::Canonical(error)),
    }
}

/// A deciding element as `quire-canonical` encodes it: forms with a fixed
/// shape through their serde encoding, the forms that hold another value or
/// type from an explicit heap stack of pending steps.
struct ElementEncode<'v> {
    value: &'v Value,
    /// Set when the value holds a kind with no encoding.
    unsupported: Cell<bool>,
}

/// One step of [`ElementEncode`].
enum Emit<'v> {
    /// Encode a value.
    Value(&'v Value),
    /// Encode a value type.
    Type(&'v ValueType),
    /// Encode a collection type.
    Collection(&'v CollectionType),
    /// Encode a cardinality bound.
    Bound(CardinalityBound),
    /// Encode a composite's slot.
    Slot(&'v FieldValue),
    /// A member name of the open object.
    Name(&'static str),
    /// Open an array.
    BeginArray,
    /// Close the open array.
    EndArray,
    /// Close the open object.
    EndObject,
}

impl ElementEncode<'_> {
    /// Open an object holding `kind`; its remaining members are `next`.
    fn open_kind<'v, S: Sink + ?Sized>(
        writer: &mut Writer<'_, S>,
        kind: &str,
        next: impl IntoIterator<Item = Emit<'v>>,
        pending: &mut Vec<Emit<'v>>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_object()?;
        writer.name("kind")?;
        writer.string(kind)?;
        let steps: Vec<Emit<'v>> = next.into_iter().collect();
        pending.extend(steps.into_iter().rev());
        Ok(())
    }
}

impl quire_canonical::Encode for ElementEncode<'_> {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        let mut pending = vec![Emit::Value(self.value)];
        while let Some(step) = pending.pop() {
            match step {
                Emit::Name(name) => writer.name(name)?,
                Emit::BeginArray => writer.begin_array()?,
                Emit::EndArray => writer.end_array()?,
                Emit::EndObject => writer.end_object()?,
                Emit::Bound(bound) => writer.serialize(&BoundWire::of(&bound))?,
                Emit::Slot(FieldValue::Absent) => {
                    writer.begin_object()?;
                    writer.name("slot")?;
                    writer.string("absent")?;
                    writer.end_object()?;
                }
                Emit::Slot(FieldValue::Null) => {
                    writer.begin_object()?;
                    writer.name("slot")?;
                    writer.string("null")?;
                    writer.end_object()?;
                }
                Emit::Slot(FieldValue::Present(value)) => {
                    writer.begin_object()?;
                    writer.name("slot")?;
                    writer.string("present")?;
                    pending.extend([Emit::EndObject, Emit::Value(value), Emit::Name("value")]);
                }
                Emit::Value(value) => match value {
                    Value::Option(option) => {
                        let mut next = vec![
                            Emit::Name("payload_type"),
                            Emit::Type(option.payload_type()),
                        ];
                        if let Some(payload) = option.payload() {
                            next.extend([Emit::Name("payload"), Emit::Value(payload)]);
                        }
                        next.push(Emit::EndObject);
                        Self::open_kind(writer, "option", next, &mut pending)?;
                    }
                    Value::Composite(composite) => {
                        writer.begin_object()?;
                        writer.name("kind")?;
                        writer.string("composite")?;
                        writer.name("declaration")?;
                        writer.string(&hex(composite.declaration().as_bytes()))?;
                        writer.name("slots")?;
                        writer.begin_array()?;
                        let mut next: Vec<Emit<'_>> =
                            composite.slots().iter().map(Emit::Slot).collect();
                        next.extend([Emit::EndArray, Emit::EndObject]);
                        pending.extend(next.into_iter().rev());
                    }
                    Value::Collection(collection) => {
                        let mut next = vec![
                            Emit::Name("type"),
                            Emit::Collection(collection.collection_type()),
                            Emit::Name("elements"),
                            Emit::BeginArray,
                        ];
                        next.extend(collection.elements().iter().map(Emit::Value));
                        next.extend([Emit::EndArray, Emit::EndObject]);
                        Self::open_kind(writer, "collection", next, &mut pending)?;
                    }
                    // A state clause cannot name a population (ADR-016 FE-4),
                    // so no claim's domain holds one.
                    Value::Population(_) | Value::Uuid(_) | Value::Timestamp(_) => {
                        self.unsupported.set(true);
                        return Err(quire_canonical::Error::Serialize(
                            "this value kind has no deciding-element encoding".to_owned(),
                        ));
                    }
                    scalar => match ScalarWire::of(scalar) {
                        Some(wire) => writer.serialize(&wire)?,
                        None => {
                            return Err(quire_canonical::Error::Internal {
                                invariant: "a value is a scalar or one of the forms matched above",
                            })
                        }
                    },
                },
                Emit::Type(value_type) => match value_type {
                    ValueType::Uuid | ValueType::Timestamp => {
                        self.unsupported.set(true);
                        return Err(quire_canonical::Error::Serialize(
                            "the selected type has no deciding-element encoding".to_owned(),
                        ));
                    }
                    ValueType::Option(payload) => Self::open_kind(
                        writer,
                        "option",
                        [Emit::Name("payload"), Emit::Type(payload), Emit::EndObject],
                        &mut pending,
                    )?,
                    ValueType::Collection(collection_type) => Self::open_kind(
                        writer,
                        "collection",
                        [
                            Emit::Name("type"),
                            Emit::Collection(collection_type),
                            Emit::EndObject,
                        ],
                        &mut pending,
                    )?,
                    leaf => match TypeLeafWire::of(leaf) {
                        Some(wire) => writer.serialize(&wire)?,
                        None => {
                            return Err(quire_canonical::Error::Internal {
                                invariant: "a type is a leaf or one of the forms matched above",
                            })
                        }
                    },
                },
                Emit::Collection(collection_type) => {
                    writer.begin_object()?;
                    writer.name("collection")?;
                    writer.serialize(&KindWire::of(collection_type.kind()))?;
                    writer.name("element")?;
                    let mut next = vec![Emit::Type(collection_type.element())];
                    if let Some(bound) = collection_type.bound() {
                        next.extend([Emit::Name("bound"), Emit::Bound(bound)]);
                    }
                    next.push(Emit::EndObject);
                    pending.extend(next.into_iter().rev());
                }
            }
        }
        Ok(())
    }
}

/// Fields of one document object: its members, with every name checked
/// against the form's own.
struct Fields<'d> {
    members: Vec<(&'d str, NodeRef<'d>)>,
}

impl<'d> Fields<'d> {
    /// `node`'s members, refusing a node that is no object and any member
    /// not in `allowed`.
    fn of(node: NodeRef<'d>, allowed: &[&str]) -> Result<Self, CauseCodecError> {
        let Node::Object(members) = node.node() else {
            return Err(malformed("a deciding-element form is not an object"));
        };
        let members: Vec<_> = members.collect();
        if members.iter().any(|(name, _)| !allowed.contains(name)) {
            return Err(malformed("a deciding-element form holds an unknown member"));
        }
        Ok(Self { members })
    }

    fn get(&self, name: &str) -> Option<NodeRef<'d>> {
        self.members
            .iter()
            .find(|(member, _)| *member == name)
            .map(|(_, node)| *node)
    }

    fn require(&self, name: &str) -> Result<NodeRef<'d>, CauseCodecError> {
        self.get(name)
            .ok_or_else(|| malformed("a deciding-element form is missing a member"))
    }
}

/// `node` as a string.
fn string_of<'d>(node: NodeRef<'d>) -> Result<&'d str, CauseCodecError> {
    match node.node() {
        Node::String(text) => Ok(text),
        _ => Err(malformed("a deciding-element member is not a string")),
    }
}

/// `node`'s `kind` member, when it has a string one.
fn kind_of<'d>(node: NodeRef<'d>) -> Option<&'d str> {
    match node.get("kind")?.node() {
        Node::String(kind) => Some(kind),
        _ => None,
    }
}

/// A fixed-shape form read from its document node: the node's canonical
/// text through the form's serde derive, whose depth the form fixes.
fn fixed<T: serde::de::DeserializeOwned>(node: NodeRef<'_>) -> Result<T, CauseCodecError> {
    let bytes = quire_canonical::to_vec(&node, Limits::new(u64::MAX))?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// One pending step of [`decode_element`].
enum Decode<'d, 'k> {
    /// Decode a value.
    Value(NodeRef<'d>),
    /// Decode a value type.
    Type(NodeRef<'d>),
    /// Decode a collection type.
    Collection(NodeRef<'d>),
    /// Build an `option` value from its decoded payload type and payload.
    OptionValue { present: bool },
    /// Build a record or tuple value from its decoded present slots.
    CompositeValue {
        declaration: &'k CompositeDeclaration,
        slots: Vec<SlotKind>,
    },
    /// Build a collection value from its decoded type and elements.
    CollectionValue { count: usize },
    /// Build an `option` type from its decoded payload type.
    OptionType,
    /// Build a collection type from its decoded element type.
    CollectionType {
        kind: CollectionKind,
        bound: Option<CardinalityBound>,
    },
    /// Make the decoded collection type a value type.
    WrapCollection,
}

/// Which slot of a composite a document names.
#[derive(Clone, Copy)]
enum SlotKind {
    Present,
    Absent,
    Null,
}

/// FR-269: the deciding element `node` holds, decoded on an explicit heap
/// stack so a value or type of any depth reads in constant host stack. A
/// form missing a member, holding a member it does not define or holding one
/// of the wrong kind refuses, as the serde-read forms do.
pub(super) fn decode_element(node: NodeRef<'_>, keys: Keys<'_>) -> Result<Value, CauseCodecError> {
    let mut pending = vec![Decode::Value(node)];
    let mut values: Vec<Value> = Vec::new();
    let mut types: Vec<ValueType> = Vec::new();
    let mut collections: Vec<CollectionType> = Vec::new();
    let underflow = || malformed("a deciding-element form lacks a part it needs");
    while let Some(step) = pending.pop() {
        match step {
            Decode::Value(node) => match kind_of(node) {
                Some("option") => {
                    let fields = Fields::of(node, &["kind", "payload_type", "payload"])?;
                    let payload = fields.get("payload");
                    pending.push(Decode::OptionValue {
                        present: payload.is_some(),
                    });
                    if let Some(payload) = payload {
                        pending.push(Decode::Value(payload));
                    }
                    pending.push(Decode::Type(fields.require("payload_type")?));
                }
                Some("composite") => {
                    let fields = Fields::of(node, &["kind", "declaration", "slots"])?;
                    let declaration =
                        keys.composite(string_of(fields.require("declaration")?)?, "declaration")?;
                    let Node::Array(items) = fields.require("slots")?.node() else {
                        return Err(malformed("a composite's slots are not an array"));
                    };
                    let mut kinds = Vec::new();
                    let mut present = Vec::new();
                    for slot in items {
                        let slot_fields = Fields::of(slot, &["slot", "value"])?;
                        let value = slot_fields.get("value");
                        match (string_of(slot_fields.require("slot")?)?, value) {
                            ("present", Some(value)) => {
                                kinds.push(SlotKind::Present);
                                present.push(value);
                            }
                            ("absent", None) => kinds.push(SlotKind::Absent),
                            ("null", None) => kinds.push(SlotKind::Null),
                            _ => {
                                return Err(malformed(
                                    "a composite slot is not present, absent or null",
                                ))
                            }
                        }
                    }
                    pending.push(Decode::CompositeValue {
                        declaration,
                        slots: kinds,
                    });
                    pending.extend(present.into_iter().rev().map(Decode::Value));
                }
                Some("collection") => {
                    let fields = Fields::of(node, &["kind", "type", "elements"])?;
                    let Node::Array(elements) = fields.require("elements")?.node() else {
                        return Err(malformed("a collection's elements are not an array"));
                    };
                    let elements: Vec<_> = elements.collect();
                    pending.push(Decode::CollectionValue {
                        count: elements.len(),
                    });
                    pending.extend(elements.into_iter().rev().map(Decode::Value));
                    pending.push(Decode::Collection(fields.require("type")?));
                }
                _ => values.push(fixed::<ScalarWire>(node)?.read(keys)?),
            },
            Decode::Type(node) => match kind_of(node) {
                Some("option") => {
                    let fields = Fields::of(node, &["kind", "payload"])?;
                    pending.push(Decode::OptionType);
                    pending.push(Decode::Type(fields.require("payload")?));
                }
                Some("collection") => {
                    let fields = Fields::of(node, &["kind", "type"])?;
                    pending.push(Decode::WrapCollection);
                    pending.push(Decode::Collection(fields.require("type")?));
                }
                _ => types.push(fixed::<TypeLeafWire>(node)?.read(keys)?),
            },
            Decode::Collection(node) => {
                let fields = Fields::of(node, &["collection", "element", "bound"])?;
                let kind = fixed::<KindWire>(fields.require("collection")?)?.read();
                let bound = fields
                    .get("bound")
                    .map(|bound| fixed::<BoundWire>(bound)?.read())
                    .transpose()?;
                pending.push(Decode::CollectionType { kind, bound });
                pending.push(Decode::Type(fields.require("element")?));
            }
            Decode::OptionType => {
                let payload = types.pop().ok_or_else(underflow)?;
                types.push(ValueType::option(payload));
            }
            Decode::CollectionType { kind, bound } => {
                let element = types.pop().ok_or_else(underflow)?;
                collections.push(CollectionType::new(kind, element, bound));
            }
            Decode::WrapCollection => {
                let collection = collections.pop().ok_or_else(underflow)?;
                types.push(ValueType::collection(collection));
            }
            Decode::OptionValue { present } => {
                let payload = if present {
                    Some(values.pop().ok_or_else(underflow)?)
                } else {
                    None
                };
                let payload_type = types.pop().ok_or_else(underflow)?;
                values.push(match payload {
                    None => OptionValue::none(payload_type),
                    Some(payload) => OptionValue::present(payload_type, payload)
                        .map_err(|_| CauseCodecError::Value("payload"))?,
                });
            }
            Decode::CompositeValue { declaration, slots } => {
                let present = slots
                    .iter()
                    .filter(|slot| matches!(slot, SlotKind::Present))
                    .count();
                let mut decoded = values
                    .split_off(values.len().checked_sub(present).ok_or_else(underflow)?)
                    .into_iter();
                let slots = slots
                    .into_iter()
                    .map(|slot| match slot {
                        SlotKind::Present => decoded.next().map(FieldValue::Present),
                        SlotKind::Absent => Some(FieldValue::Absent),
                        SlotKind::Null => Some(FieldValue::Null),
                    })
                    .collect::<Option<Box<[_]>>>()
                    .ok_or_else(underflow)?;
                values.push(keys.shaped(declaration, slots)?);
            }
            Decode::CollectionValue { count } => {
                let collection_type = collections.pop().ok_or_else(underflow)?;
                let elements =
                    values.split_off(values.len().checked_sub(count).ok_or_else(underflow)?);
                let mut meter = Meter::new(READER_LIMITS);
                // Forming checks each element's type and the bound; a
                // collapsed duplicate means the document listed one twice.
                match form_collection(&collection_type, elements, &mut meter) {
                    Ok(Outcome::Completed(Value::Collection(formed)))
                        if formed.elements().len() == count =>
                    {
                        values.push(Value::Collection(formed));
                    }
                    _ => return Err(CauseCodecError::Value("elements")),
                }
            }
        }
    }
    match (values.pop(), values.is_empty()) {
        (Some(value), true) => Ok(value),
        _ => Err(underflow()),
    }
}
