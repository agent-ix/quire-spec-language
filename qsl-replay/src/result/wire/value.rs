// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-269: a deciding element's typed value encoding, one tagged form per
//! kernel value kind, and the value types an `Option` or collection carries.
//!
//! Each form holds the kind's own payload exactly: integers as canonical
//! decimal strings, a rational in lowest terms, a decimal as coefficient and
//! scale, a float as its bit pattern, text as its admitted payload. The
//! reader mints no declaration identity (ADR-013 O-04, O-05): a record or
//! tuple declaration, a declared unit and an object type each resolve by
//! lookup among the identities the checked package admitted, and a name the
//! package does not hold refuses.

use qsl_package::CheckedPackage;
use quire_exact::{
    admit_text, form_collection, from_admitted_slots, CardinalityBound, CollectionKind,
    CollectionType, Decimal, DecimalType, EffectiveId, EnumMember, EnumShape, FieldValue,
    FloatType, IeeeValue, IeeeWidth, Integer, IntegerInterval, Meter, ObjectId, ObjectReference,
    OptionValue, Outcome, Presence, Quantity, Rational, RationalDomain, RoundingMode, ScalarLimits,
    TextPayload, TextProfile, TextType, UnitDomain, UnitId, UniverseId, Value, ValueType,
    VariantId,
};
use quire_semantic_value::declaration::{CompositeDeclaration, CompositeShape, TypeEnvironment};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::RawValue;

use super::{digest, hex, present, CauseCodecError};

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

/// A rational in lowest terms with a positive denominator.
#[derive(Serialize, Deserialize)]
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

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum UnitDomainWire {
    Declared,
    Compound,
}

#[derive(Serialize, Deserialize)]
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

#[derive(Clone, Copy, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Serialize, Deserialize)]
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
#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BoundWire {
    minimum: u64,
    maximum: u64,
}

/// A collection type: kind, element type and optional cardinality bound.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CollectionTypeWire {
    collection: KindWire,
    element: Box<TypeWire>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    bound: Option<BoundWire>,
}

impl CollectionTypeWire {
    fn read(self, keys: Keys<'_>) -> Result<CollectionType, CauseCodecError> {
        let bound = self
            .bound
            .map(|bound| {
                CardinalityBound::new(bound.minimum, bound.maximum)
                    .map_err(|_| CauseCodecError::Value("bound"))
            })
            .transpose()?;
        Ok(CollectionType::new(
            self.collection.read(),
            self.element.read(keys)?,
            bound,
        ))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TextTypeWire {
    min: u64,
    max: u64,
    profile: String,
}

impl TextTypeWire {
    fn of(text_type: &TextType) -> Self {
        Self {
            min: text_type.min(),
            max: text_type.max(),
            profile: text_type.profile().as_str().to_owned(),
        }
    }

    fn read(self) -> Result<TextType, CauseCodecError> {
        let profile =
            TextProfile::from_code(&self.profile).ok_or(CauseCodecError::Value("profile"))?;
        TextType::new(self.min, self.max, profile).map_err(|_| CauseCodecError::Value("text type"))
    }
}

fn rounding(text: &str) -> Result<RoundingMode, CauseCodecError> {
    RoundingMode::from_code(text).ok_or(CauseCodecError::Value("rounding"))
}

/// A value type, by its kind. [`encode_element`] writes `option` and
/// `collection` itself, over an explicit stack, and writes each other kind
/// through this type's own encoding.
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(super) enum TypeWire {
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
        min_scale: u64,
        max_scale: u64,
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
    Option {
        payload: Box<TypeWire>,
    },
    Composite {
        declaration: String,
    },
    Collection {
        #[serde(rename = "type")]
        collection_type: CollectionTypeWire,
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
        maximum: Option<u64>,
    },
}

impl TypeWire {
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
                    min_scale,
                    max_scale,
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
            Self::Option { payload } => ValueType::option(payload.read(keys)?),
            Self::Composite { declaration } => {
                ValueType::Composite(keys.composite(&declaration, "declaration")?.key())
            }
            Self::Collection { collection_type } => {
                ValueType::collection(collection_type.read(keys)?)
            }
            Self::Reference { object_type } => {
                ValueType::Reference(keys.object_type(&object_type, "object_type")?)
            }
            Self::Population { maximum } => ValueType::Population(maximum),
        })
    }
}

/// One record or tuple slot.
#[derive(Serialize, Deserialize)]
#[serde(tag = "slot", rename_all = "kebab-case", deny_unknown_fields)]
pub(super) enum SlotWire {
    Present { value: ValueWire },
    Absent,
    Null,
}

/// A deciding element, by its value kind. [`encode_element`] writes the
/// forms that hold another value or type itself, over an explicit stack, and
/// writes each other form through this type's own encoding.
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(super) enum ValueWire {
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
        bits: u64,
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
    Option {
        payload_type: TypeWire,
        #[serde(
            default,
            deserialize_with = "present",
            skip_serializing_if = "Option::is_none"
        )]
        payload: Option<Box<ValueWire>>,
    },
    Composite {
        declaration: String,
        slots: Vec<SlotWire>,
    },
    Collection {
        #[serde(rename = "type")]
        collection_type: CollectionTypeWire,
        elements: Vec<ValueWire>,
    },
    Reference {
        universe: String,
        #[serde(rename = "type")]
        object_type: String,
        object_identity: String,
    },
}

impl ValueWire {
    pub(super) fn read(self, keys: Keys<'_>) -> Result<Value, CauseCodecError> {
        Ok(match self {
            Self::Boolean { value } => Value::Boolean(value),
            Self::Integer { decimal } => Value::Integer(integer(&decimal, "decimal")?),
            Self::Rational { value } => Value::Rational(value.read()?),
            Self::Decimal { coefficient, scale } => {
                Value::Decimal(Decimal::new(integer(&coefficient, "coefficient")?, scale))
            }
            Self::Float { width, bits } => Value::Float(match width {
                WidthWire::Binary32 => IeeeValue::binary32(
                    u32::try_from(bits).map_err(|_| CauseCodecError::Value("bits"))?,
                ),
                WidthWire::Binary64 => IeeeValue::binary64(bits),
            }),
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
            Self::Option {
                payload_type,
                payload,
            } => {
                let payload_type = payload_type.read(keys)?;
                match payload {
                    None => OptionValue::none(payload_type),
                    Some(payload) => OptionValue::present(payload_type, payload.read(keys)?)
                        .map_err(|_| CauseCodecError::Value("payload"))?,
                }
            }
            Self::Composite { declaration, slots } => {
                let declaration = keys.composite(&declaration, "declaration")?;
                let slots = slots
                    .into_iter()
                    .map(|slot| {
                        Ok(match slot {
                            SlotWire::Present { value } => FieldValue::Present(value.read(keys)?),
                            SlotWire::Absent => FieldValue::Absent,
                            SlotWire::Null => FieldValue::Null,
                        })
                    })
                    .collect::<Result<Box<[_]>, CauseCodecError>>()?;
                keys.shaped(declaration, slots)?
            }
            Self::Collection {
                collection_type,
                elements,
            } => {
                let collection_type = collection_type.read(keys)?;
                let elements = elements
                    .into_iter()
                    .map(|element| element.read(keys))
                    .collect::<Result<Vec<_>, _>>()?;
                let count = elements.len();
                let mut meter = Meter::new(READER_LIMITS);
                // Forming checks each element's type and the bound; a
                // collapsed duplicate means the document listed one twice.
                match form_collection(&collection_type, elements, &mut meter) {
                    Ok(Outcome::Completed(Value::Collection(formed)))
                        if formed.elements().len() == count =>
                    {
                        Value::Collection(formed)
                    }
                    _ => return Err(CauseCodecError::Value("elements")),
                }
            }
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

/// A deciding element's FR-269 encoding: written as the JSON
/// [`encode_element`] renders from the value, read as its typed form.
pub(super) enum ElementWire {
    /// Rendered from a value, to be written.
    Written(Box<RawValue>),
    /// Read from a cause document.
    Read(ValueWire),
}

impl ElementWire {
    pub(super) fn of(value: &Value) -> Result<Self, CauseCodecError> {
        encode_element(value).map(Self::Written)
    }

    pub(super) fn read(self, keys: Keys<'_>) -> Result<Value, CauseCodecError> {
        match self {
            Self::Read(wire) => wire.read(keys),
            Self::Written(raw) => serde_json::from_str::<ValueWire>(raw.get())?.read(keys),
        }
    }
}

impl Serialize for ElementWire {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Written(raw) => raw.serialize(serializer),
            Self::Read(_) => Err(serde::ser::Error::custom(
                "a deciding element read from a document is not written again",
            )),
        }
    }
}

impl<'de> Deserialize<'de> for ElementWire {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        ValueWire::deserialize(deserializer).map(Self::Read)
    }
}

/// One step of [`encode_element`].
enum Encode<'v> {
    /// Write this JSON text.
    Text(&'static str),
    /// Write this rendered JSON.
    Json(String),
    /// Encode a value.
    Value(&'v Value),
    /// Encode a value type.
    Type(&'v ValueType),
    /// Encode a collection type.
    Collection(&'v CollectionType),
}

/// `wire`'s JSON, for a form that holds no other value or type.
fn json<T: Serialize>(wire: &T) -> Result<String, CauseCodecError> {
    Ok(serde_json::to_string(wire)?)
}

/// FR-269: `value`'s typed encoding. An `option`, `composite` or
/// `collection` value and an `option` or `collection` type are written here,
/// their members kept on an explicit heap stack, so a value or type of any
/// depth encodes in constant host stack; every other form is written by its
/// own wire type.
pub(super) fn encode_element(value: &Value) -> Result<Box<RawValue>, CauseCodecError> {
    let mut out = String::new();
    let mut pending = vec![Encode::Value(value)];
    while let Some(step) = pending.pop() {
        let mut next: Vec<Encode<'_>> = Vec::new();
        match step {
            Encode::Text(text) => out.push_str(text),
            Encode::Json(text) => out.push_str(&text),
            Encode::Value(value) => match value {
                Value::Boolean(value) => {
                    out.push_str(&json(&ValueWire::Boolean { value: *value })?)
                }
                Value::Integer(integer) => out.push_str(&json(&ValueWire::Integer {
                    decimal: integer.to_string(),
                })?),
                Value::Rational(rational) => out.push_str(&json(&ValueWire::Rational {
                    value: RationalWire::of(rational),
                })?),
                Value::Decimal(decimal) => out.push_str(&json(&ValueWire::Decimal {
                    coefficient: decimal.representation().coefficient().to_string(),
                    scale: decimal.representation().scale(),
                })?),
                Value::Float(float) => out.push_str(&json(&ValueWire::Float {
                    width: WidthWire::of(float.width()),
                    bits: float.bits(),
                })?),
                Value::Quantity(quantity) => out.push_str(&json(&ValueWire::Quantity {
                    magnitude: RationalWire::of(quantity.magnitude()),
                    unit: UnitWire::of(quantity.unit()),
                })?),
                Value::Text(text) => out.push_str(&json(&ValueWire::Text {
                    text_type: TextTypeWire::of(text.text_type()),
                    payload: text.payload().as_str().to_owned(),
                })?),
                Value::Enum(member) => out.push_str(&json(&ValueWire::Enum {
                    variant: hex(member.variant().as_bytes()),
                    rank: member.rank(),
                })?),
                Value::Reference(reference) => out.push_str(&json(&ValueWire::Reference {
                    universe: hex(reference.universe().as_bytes()),
                    object_type: hex(reference.object_type().as_bytes()),
                    object_identity: reference.object().as_str().to_owned(),
                })?),
                Value::Option(option) => {
                    next.push(Encode::Text(r#"{"kind":"option","payload_type":"#));
                    next.push(Encode::Type(option.payload_type()));
                    if let Some(payload) = option.payload() {
                        next.push(Encode::Text(r#","payload":"#));
                        next.push(Encode::Value(payload));
                    }
                    next.push(Encode::Text("}"));
                }
                Value::Composite(composite) => {
                    next.push(Encode::Text(r#"{"kind":"composite","declaration":"#));
                    next.push(Encode::Json(json(&hex(composite
                        .declaration()
                        .as_bytes()))?));
                    next.push(Encode::Text(r#","slots":["#));
                    for (at, slot) in composite.slots().iter().enumerate() {
                        if at > 0 {
                            next.push(Encode::Text(","));
                        }
                        match slot {
                            FieldValue::Present(value) => {
                                next.push(Encode::Text(r#"{"slot":"present","value":"#));
                                next.push(Encode::Value(value));
                                next.push(Encode::Text("}"));
                            }
                            FieldValue::Absent => next.push(Encode::Text(r#"{"slot":"absent"}"#)),
                            FieldValue::Null => next.push(Encode::Text(r#"{"slot":"null"}"#)),
                        }
                    }
                    next.push(Encode::Text("]}"));
                }
                Value::Collection(collection) => {
                    next.push(Encode::Text(r#"{"kind":"collection","type":"#));
                    next.push(Encode::Collection(collection.collection_type()));
                    next.push(Encode::Text(r#","elements":["#));
                    for (at, element) in collection.elements().iter().enumerate() {
                        if at > 0 {
                            next.push(Encode::Text(","));
                        }
                        next.push(Encode::Value(element));
                    }
                    next.push(Encode::Text("]}"));
                }
                // A state clause cannot name a population (ADR-016 FE-4), so
                // no claim's domain holds one.
                Value::Population(_) => return Err(CauseCodecError::UnsupportedElement),
            },
            Encode::Type(value_type) => match value_type {
                ValueType::Boolean => out.push_str(&json(&TypeWire::Boolean)?),
                ValueType::Integer => out.push_str(&json(&TypeWire::Integer)?),
                ValueType::Int(interval) => out.push_str(&json(&TypeWire::Int {
                    interval: IntervalWire::of(interval),
                })?),
                ValueType::Rational(domain) => out.push_str(&json(&TypeWire::Rational {
                    numerator: IntervalWire::of(domain.numerator()),
                    denominator: IntervalWire::of(domain.denominator()),
                })?),
                ValueType::Decimal(decimal) => out.push_str(&json(&TypeWire::Decimal {
                    lower: decimal.lower().to_string(),
                    upper: decimal.upper().to_string(),
                    min_scale: u64::from(decimal.min_scale()),
                    max_scale: u64::from(decimal.max_scale()),
                    rounding: decimal.rounding().as_str().to_owned(),
                })?),
                ValueType::Float(float) => out.push_str(&json(&TypeWire::Float {
                    width: WidthWire::of(float.width()),
                    rounding: float.rounding().as_str().to_owned(),
                })?),
                ValueType::Quantity(unit) => out.push_str(&json(&TypeWire::Quantity {
                    unit: UnitWire::of(*unit),
                })?),
                ValueType::Text(text_type) => out.push_str(&json(&TypeWire::Text {
                    text_type: TextTypeWire::of(text_type),
                })?),
                ValueType::Enum(shape) => out.push_str(&json(&TypeWire::Enum {
                    ordered: shape.is_ordered(),
                    variants: shape
                        .variants()
                        .map(|variant| hex(variant.as_bytes()))
                        .collect(),
                })?),
                ValueType::Composite(declaration) => out.push_str(&json(&TypeWire::Composite {
                    declaration: hex(declaration.as_bytes()),
                })?),
                ValueType::Reference(object_type) => out.push_str(&json(&TypeWire::Reference {
                    object_type: hex(object_type.as_bytes()),
                })?),
                ValueType::Population(maximum) => {
                    out.push_str(&json(&TypeWire::Population { maximum: *maximum })?)
                }
                ValueType::Option(payload) => {
                    next.push(Encode::Text(r#"{"kind":"option","payload":"#));
                    next.push(Encode::Type(payload));
                    next.push(Encode::Text("}"));
                }
                ValueType::Collection(collection_type) => {
                    next.push(Encode::Text(r#"{"kind":"collection","type":"#));
                    next.push(Encode::Collection(collection_type));
                    next.push(Encode::Text("}"));
                }
            },
            Encode::Collection(collection_type) => {
                next.push(Encode::Text(r#"{"collection":"#));
                next.push(Encode::Json(json(&KindWire::of(collection_type.kind()))?));
                next.push(Encode::Text(r#","element":"#));
                next.push(Encode::Type(collection_type.element()));
                if let Some(bound) = collection_type.bound() {
                    next.push(Encode::Text(r#","bound":"#));
                    next.push(Encode::Json(json(&BoundWire {
                        minimum: bound.minimum(),
                        maximum: bound.maximum(),
                    })?));
                }
                next.push(Encode::Text("}"));
            }
        }
        pending.extend(next.into_iter().rev());
    }
    Ok(RawValue::from_string(out)?)
}
