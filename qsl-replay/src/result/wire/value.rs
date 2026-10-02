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
    FloatType, IeeeValue, IeeeWidth, Integer, IntegerInterval, Meter, NodeKey, ObjectId,
    ObjectReference, OptionValue, Outcome, Quantity, Rational, RationalDomain, RoundingMode,
    ScalarLimits, TextPayload, TextProfile, TextType, UnitDomain, UnitId, UniverseId, Value,
    ValueType, VariantId,
};
use serde::{Deserialize, Serialize};

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
    fn composite(&self, text: &str, member: &'static str) -> Result<NodeKey, CauseCodecError> {
        let bytes = digest(text, member)?;
        self.package
            .graph()
            .scope()
            .types()
            .composites()
            .map(|declaration| declaration.key())
            .find(|key| key.as_bytes() == &bytes)
            .ok_or(CauseCodecError::Unresolved(member))
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
        IntegerInterval::new(integer(&self.lower, "lower")?, integer(&self.upper, "upper")?)
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
    fn of(collection_type: &CollectionType) -> Self {
        Self {
            collection: KindWire::of(collection_type.kind()),
            element: Box::new(TypeWire::of(collection_type.element())),
            bound: collection_type.bound().map(|bound| BoundWire {
                minimum: bound.minimum(),
                maximum: bound.maximum(),
            }),
        }
    }

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

/// A value type, by its kind.
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
    fn of(value_type: &ValueType) -> Self {
        match value_type {
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
                min_scale: u64::from(decimal.min_scale()),
                max_scale: u64::from(decimal.max_scale()),
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
            ValueType::Option(payload) => Self::Option {
                payload: Box::new(Self::of(payload)),
            },
            ValueType::Composite(declaration) => Self::Composite {
                declaration: hex(declaration.as_bytes()),
            },
            ValueType::Collection(collection_type) => Self::Collection {
                collection_type: CollectionTypeWire::of(collection_type),
            },
            ValueType::Reference(object_type) => Self::Reference {
                object_type: hex(object_type.as_bytes()),
            },
            ValueType::Population(maximum) => Self::Population { maximum: *maximum },
        }
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
                ValueType::Composite(keys.composite(&declaration, "declaration")?)
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

/// A deciding element, by its value kind.
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
    pub(super) fn of(value: &Value) -> Result<Self, CauseCodecError> {
        Ok(match value {
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
                bits: float.bits(),
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
            Value::Option(option) => Self::Option {
                payload_type: TypeWire::of(option.payload_type()),
                payload: option
                    .payload()
                    .map(Self::of)
                    .transpose()?
                    .map(Box::new),
            },
            Value::Composite(composite) => Self::Composite {
                declaration: hex(composite.declaration().as_bytes()),
                slots: composite
                    .slots()
                    .iter()
                    .map(|slot| {
                        Ok(match slot {
                            FieldValue::Present(value) => SlotWire::Present {
                                value: Self::of(value)?,
                            },
                            FieldValue::Absent => SlotWire::Absent,
                            FieldValue::Null => SlotWire::Null,
                        })
                    })
                    .collect::<Result<_, CauseCodecError>>()?,
            },
            Value::Collection(collection) => Self::Collection {
                collection_type: CollectionTypeWire::of(collection.collection_type()),
                elements: collection
                    .elements()
                    .iter()
                    .map(Self::of)
                    .collect::<Result<_, _>>()?,
            },
            Value::Reference(reference) => Self::Reference {
                universe: hex(reference.universe().as_bytes()),
                object_type: hex(reference.object_type().as_bytes()),
                object_identity: reference.object().as_str().to_owned(),
            },
            // A state clause cannot name a population (ADR-016 FE-4), so no
            // claim's domain holds one.
            Value::Population(_) => return Err(CauseCodecError::UnsupportedElement),
        })
    }

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
                from_admitted_slots(declaration, slots)
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
