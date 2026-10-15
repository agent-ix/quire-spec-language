// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-269: a deciding element's typed value encoding, one tagged form per
//! kernel value kind, and the value types an `Option` or collection carries.
//!
//! Each form holds the kind's own payload exactly: integers as canonical
//! decimal strings, a rational in lowest terms, a decimal as coefficient and
//! scale, a float as its bit pattern in decimal, text as its admitted
//! payload, and native String as its exact Unicode scalar sequence. Every
//! number that can exceed 2^53 is a decimal string, since the
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
    admit_text, form, from_admitted_slots, CardinalityBound, CollectionKind, CollectionType,
    Decimal, DecimalType, EffectiveId, EnumMember, EnumShape, FieldValue, FloatType, IeeeValue,
    IeeeWidth, Integer, IntegerInterval, Meter, ObjectId, ObjectReference, OptionValue, Outcome,
    Presence, Quantity, Rational, RationalDomain, RoundingMode, ScalarLimits, TextPayload,
    TextProfile, TextType, UnitDomain, UnitId, UniverseId, Value, ValueType, VariantId,
};
use quire_semantic_value::declaration::{CompositeDeclaration, CompositeShape, TypeEnvironment};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use super::{digest, hex, malformed, present, CauseCodecError};

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use quire_semantic_value::declaration::EnvironmentFailure;

    fn package() -> CheckedPackage {
        let declarations = qsl_semantics::check::PackageDeclarations::new(
            qsl_semantics::check::fixture_source(),
            qsl_foundation::IdentityLimits::default(),
        );
        CheckedPackage::link(
            declarations
                .check(quire_semantic_value::checking::CheckingLimits::default())
                .expect("the empty fixture package checks"),
        )
    }

    /// A storage failure in a membership answer is reported as the storage
    /// failure, not read as a value outside its type; a `false` before it
    /// stops the walk first, as the `all` it replaces did.
    #[test]
    fn storage_failure_is_not_a_false_membership_answer() {
        use quire_semantic_value::declaration::{StorageRequest, StorageUnit};
        let failure = EnvironmentFailure::Allocation(StorageRequest::new(4, StorageUnit::Bytes));
        let storage = || {
            Err(CauseCodecError::Storage {
                member: "slots",
                failure: failure.clone(),
            })
        };
        assert!(matches!(
            all_fit([Ok(true), storage()].into_iter()),
            Err(CauseCodecError::Storage {
                member: "slots",
                ..
            })
        ));
        assert!(matches!(
            all_fit([Ok(false), storage()].into_iter()),
            Ok(false)
        ));
        assert!(matches!(
            all_fit([Ok(true), Ok(true)].into_iter()),
            Ok(true)
        ));
    }

    #[trace("FR-056-AC-23")]
    #[test]
    fn native_string_values_round_trip_in_the_closed_canonical_form() {
        let package = package();
        for text in ["ascii", "λ雪", "", "550e8400-e29b-41d4-a716-446655440000"] {
            let encoded = encode_element(&Value::String(text.to_owned())).unwrap();
            let expected = String::from_utf8(
                quire_canonical::to_vec(
                    &serde_json::json!({"kind": "string", "text": text}),
                    Limits::new(u64::MAX),
                )
                .unwrap(),
            )
            .unwrap();
            assert_eq!(encoded.get(), expected);
            let document = quire_canonical::read(encoded.get().as_bytes(), u64::MAX).unwrap();
            validate_element_shape(document.root()).unwrap();
            match decode_element(document.root(), Keys::new(&package)).unwrap() {
                Value::String(decoded) => assert_eq!(decoded, text),
                other => panic!("decoded native String as {other:?}"),
            }
        }
    }

    #[trace("FR-056-AC-23")]
    #[test]
    fn native_string_wire_refuses_extra_members_and_lone_surrogates() {
        let extra =
            quire_canonical::read(br#"{"kind":"string","text":"x","extra":true}"#, u64::MAX)
                .unwrap();
        assert!(validate_element_shape(extra.root()).is_err());

        assert!(quire_canonical::read(br#"{"kind":"string","text":"\ud800"}"#, u64::MAX,).is_err());
    }

    #[trace("FR-056-AC-23")]
    #[test]
    fn native_string_type_uses_a_string_leaf_tag() {
        let package = package();
        let value = OptionValue::none(ValueType::String);
        let encoded = encode_element(&value).unwrap();
        assert!(encoded
            .get()
            .contains(r#""payload_type":{"kind":"string"}"#));
        let document = quire_canonical::read(encoded.get().as_bytes(), u64::MAX).unwrap();
        match decode_element(document.root(), Keys::new(&package)).unwrap() {
            Value::Option(option) => assert_eq!(option.payload_type(), &ValueType::String),
            other => panic!("decoded String type leaf in {other:?}"),
        }
    }

    /// `Money <- TaxedMoney`, each with an `amount` and the child's `tax`, and
    /// a `Pair` of two `Money`, in a checked package.
    fn money_package() -> (CheckedPackage, [quire_exact::NodeKey; 3]) {
        use quire_semantic_value::declaration::FieldDeclaration;
        let [money, taxed, pair] =
            [1, 2, 3].map(|byte| quire_exact::NodeKey::from_digest([byte; 32]));
        let field =
            |name: &str, value_type| FieldDeclaration::new(name, value_type, Presence::Required);
        let types = TypeEnvironment::new(
            [
                CompositeDeclaration::new(
                    money,
                    "Money",
                    CompositeShape::Record(vec![field("amount", ValueType::Integer)]),
                ),
                CompositeDeclaration::new(
                    taxed,
                    "TaxedMoney",
                    CompositeShape::Record(vec![
                        field("amount", ValueType::Integer),
                        field("tax", ValueType::Integer),
                    ]),
                )
                .with_supertypes(vec![money]),
                CompositeDeclaration::new(
                    pair,
                    "Pair",
                    CompositeShape::Record(vec![
                        field("first", ValueType::Composite(money)),
                        field("second", ValueType::Composite(money)),
                    ]),
                ),
            ],
            [],
        )
        .expect("the records admit");
        let package = CheckedPackage::link(
            qsl_semantics::check::PackageDeclarations {
                types,
                ..qsl_semantics::check::PackageDeclarations::new(
                    qsl_semantics::check::fixture_source(),
                    qsl_foundation::IdentityLimits::default(),
                )
            }
            .check(quire_semantic_value::checking::CheckingLimits::default())
            .expect("the package checks"),
        );
        (package, [money, taxed, pair])
    }

    /// FR-359-AC-9 (TC-1002): a deciding element holding a child record at
    /// an ancestor-typed record field, `Option` payload or collection element
    /// decodes to the same typed value, keeping its own declaration, and
    /// encodes to the same bytes. An unrelated record at the slot refuses.
    #[trace("FR-359-AC-9", "TC-1002")]
    #[test]
    fn a_child_record_at_an_ancestor_slot_round_trips() {
        let (package, [money, taxed, pair]) = money_package();
        let types = package.graph().scope().types();
        let int = |value: i64| Value::Integer(Integer::from(value));
        let plain = types
            .record(money, vec![("amount", FieldValue::Present(int(1)))])
            .unwrap();
        let child = types
            .record(
                taxed,
                vec![
                    ("amount", FieldValue::Present(int(1))),
                    ("tax", FieldValue::Present(int(2))),
                ],
            )
            .unwrap();
        let holder = types
            .record(
                pair,
                vec![
                    ("first", FieldValue::Present(child.clone())),
                    ("second", FieldValue::Present(plain.clone())),
                ],
            )
            .unwrap();
        let option = OptionValue::from_admitted(ValueType::Composite(money), Some(child.clone()));
        let collection = quire_exact::from_admitted(
            CollectionType::new(
                CollectionKind::Sequence,
                ValueType::Composite(money),
                Some(CardinalityBound::new(0, 4).unwrap()),
            ),
            vec![child.clone(), plain],
        );
        for value in [holder, option, collection] {
            let encoded = encode_element(&value).unwrap();
            let document = quire_canonical::read(encoded.get().as_bytes(), u64::MAX).unwrap();
            let decoded = decode_element(document.root(), Keys::new(&package)).unwrap();
            assert_eq!(
                quire_exact::compare_keys(&decoded, &value),
                Some(std::cmp::Ordering::Equal),
                "the decoded value is the encoded one"
            );
            let again = encode_element(&decoded).unwrap();
            assert_eq!(
                again.get(),
                encoded.get(),
                "a decode then an encode is stable"
            );
        }
        // The child's own declaration is what the document names.
        let encoded = encode_element(&child).unwrap();
        assert!(encoded.get().contains(&hex(taxed.as_bytes())));
    }

    #[trace("FR-056-AC-23")]
    #[test]
    fn existing_wire_kind_strings_remain_unchanged() {
        let encoded = encode_element(&Value::Boolean(true)).unwrap();
        assert_eq!(encoded.get(), r#"{"kind":"boolean","value":true}"#);
    }
}

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

/// Whether `value` is a member of `value_type`, with a storage failure of the
/// membership query reported as the undecided `member`, not as "no".
fn admitted(
    types: &TypeEnvironment,
    value_type: &ValueType,
    value: &Value,
    member: &'static str,
) -> Result<bool, CauseCodecError> {
    types
        .admits(value_type, value)
        .map_err(|failure| CauseCodecError::Storage { member, failure })
}

/// Whether every answer is `true`, stopping at the first `false` or failure.
fn all_fit(
    answers: impl Iterator<Item = Result<bool, CauseCodecError>>,
) -> Result<bool, CauseCodecError> {
    for answer in answers {
        if !answer? {
            return Ok(false);
        }
    }
    Ok(true)
}

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
                    && all_fit(
                        fields
                            .iter()
                            .zip(slots.iter())
                            .map(|(field, slot)| match slot {
                                FieldValue::Present(value) => {
                                    admitted(types, field.value_type(), value, "slots")
                                }
                                FieldValue::Absent | FieldValue::Null => {
                                    Ok(field.presence() == Presence::Optional)
                                }
                            }),
                    )?
            }
            CompositeShape::Tuple(positions) => {
                positions.len() == slots.len()
                    && all_fit(positions.iter().zip(slots.iter()).map(
                        |(position, slot)| match slot {
                            FieldValue::Present(value) => admitted(types, position, value, "slots"),
                            FieldValue::Absent | FieldValue::Null => Ok(false),
                        },
                    ))?
            }
            // Slots are a record's or a tuple's; a union value is not
            // built from them.
            CompositeShape::Union(_) => false,
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
    String,
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
    /// Check all leaf payloads that do not need a checked package.
    fn validate_shape(self) -> Result<(), CauseCodecError> {
        match self {
            Self::Boolean | Self::Integer => {}
            Self::Int { interval } => {
                interval.read()?;
            }
            Self::Rational {
                numerator,
                denominator,
            } => {
                RationalDomain::new(numerator.read()?, denominator.read()?)
                    .map_err(|_| CauseCodecError::Value("rational domain"))?;
            }
            Self::Decimal {
                lower,
                upper,
                min_scale,
                max_scale,
                rounding: mode,
            } => {
                DecimalType::new(
                    integer(&lower, "lower")?,
                    integer(&upper, "upper")?,
                    unsigned(&min_scale, "min_scale")?,
                    unsigned(&max_scale, "max_scale")?,
                    rounding(&mode)?,
                )
                .map_err(|_| CauseCodecError::Value("decimal type"))?;
            }
            Self::Float { rounding: mode, .. } => {
                rounding(&mode)?;
            }
            Self::Quantity { unit } => {
                digest(&unit.digest, "unit")?;
            }
            Self::Text { text_type } => {
                text_type.read()?;
            }
            Self::String => {}
            Self::Enum { ordered, variants } => {
                let variants = variants
                    .iter()
                    .map(|variant| digest(variant, "variant").map(VariantId::from_digest))
                    .collect::<Result<Vec<_>, _>>()?;
                let shape = EnumShape::new(ordered, variants.iter().copied());
                if shape.variants().count() != variants.len() {
                    return Err(CauseCodecError::Value("variants"));
                }
            }
            Self::Composite { declaration } => {
                digest(&declaration, "declaration")?;
            }
            Self::Reference { object_type } => {
                digest(&object_type, "type")?;
            }
            Self::Population { maximum } => {
                if let Some(maximum) = maximum {
                    unsigned(&maximum, "maximum")?;
                }
            }
        }
        Ok(())
    }

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
            ValueType::String => Self::String,
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
            Self::String => ValueType::String,
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
    String {
        text: String,
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
    /// Check every package-independent payload invariant before a public
    /// reader returns a witness whose package may never be supplied.
    fn validate_shape(self) -> Result<(), CauseCodecError> {
        match self {
            Self::Boolean { .. } => {}
            Self::Integer { decimal } => {
                integer(&decimal, "decimal")?;
            }
            Self::Rational { value } => {
                value.read()?;
            }
            Self::Decimal { coefficient, .. } => {
                integer(&coefficient, "coefficient")?;
            }
            Self::Float { width, bits } => {
                let bits = unsigned(&bits, "bits")?;
                if matches!(width, WidthWire::Binary32) {
                    u32::try_from(bits).map_err(|_| CauseCodecError::Value("bits"))?;
                }
            }
            Self::Quantity { magnitude, unit } => {
                magnitude.read()?;
                digest(&unit.digest, "unit")?;
            }
            Self::Text { text_type, payload } => {
                let text_type = text_type.read()?;
                let payload = TextPayload::from_utf8(payload.as_bytes())
                    .map_err(|_| CauseCodecError::Value("payload"))?;
                let mut meter = Meter::new(READER_LIMITS);
                if !matches!(
                    admit_text(&payload, &text_type, &mut meter),
                    Outcome::Completed(_)
                ) {
                    return Err(CauseCodecError::Value("payload"));
                }
            }
            Self::String { .. } => {}
            Self::Enum { variant, .. } => {
                digest(&variant, "variant")?;
            }
            Self::Reference {
                universe,
                object_type,
                object_identity,
            } => {
                digest(&universe, "universe")?;
                digest(&object_type, "type")?;
                ObjectId::new(object_identity)
                    .map_err(|_| CauseCodecError::Empty("object_identity"))?;
            }
        }
        Ok(())
    }

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
            Value::String(text) => Self::String { text: text.clone() },
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
            | Value::Union(_)
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
            Self::String { text } => Value::String(text),
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
                    // so no claim's domain holds one, and no source form
                    // builds a union.
                    Value::Population(_)
                    | Value::Uuid(_)
                    | Value::Timestamp(_)
                    | Value::Union(_) => {
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

/// Validate the package-independent typed shape of one deciding element.
/// Declaration resolution remains a consumer's responsibility because a
/// `/2` document does not carry the checked package.
pub(super) fn validate_element_shape(node: NodeRef<'_>) -> Result<(), CauseCodecError> {
    enum Check<'a> {
        Value(NodeRef<'a>),
        Type(NodeRef<'a>),
        Collection(NodeRef<'a>),
    }
    let mut work = vec![Check::Value(node)];
    while let Some(check) = work.pop() {
        match check {
            Check::Value(node) => match kind_of(node) {
                Some("option") => {
                    let fields = Fields::of(node, &["kind", "payload_type", "payload"])?;
                    if let Some(payload) = fields.get("payload") {
                        work.push(Check::Value(payload));
                    }
                    work.push(Check::Type(fields.require("payload_type")?));
                }
                Some("composite") => {
                    let fields = Fields::of(node, &["kind", "declaration", "slots"])?;
                    string_of(fields.require("declaration")?)?;
                    let Node::Array(slots) = fields.require("slots")?.node() else {
                        return Err(malformed("a composite's slots are not an array"));
                    };
                    for slot in slots {
                        let slot = Fields::of(slot, &["slot", "value"])?;
                        match (string_of(slot.require("slot")?)?, slot.get("value")) {
                            ("present", Some(value)) => work.push(Check::Value(value)),
                            ("absent" | "null", None) => {}
                            _ => return Err(malformed("a composite slot has the wrong shape")),
                        }
                    }
                }
                Some("collection") => {
                    let fields = Fields::of(node, &["kind", "type", "elements"])?;
                    let Node::Array(elements) = fields.require("elements")?.node() else {
                        return Err(malformed("a collection's elements are not an array"));
                    };
                    work.extend(elements.map(Check::Value));
                    work.push(Check::Collection(fields.require("type")?));
                }
                _ => {
                    fixed::<ScalarWire>(node)?.validate_shape()?;
                }
            },
            Check::Type(node) => match kind_of(node) {
                Some("option") => {
                    let fields = Fields::of(node, &["kind", "payload"])?;
                    work.push(Check::Type(fields.require("payload")?));
                }
                Some("collection") => {
                    let fields = Fields::of(node, &["kind", "type"])?;
                    work.push(Check::Collection(fields.require("type")?));
                }
                _ => {
                    fixed::<TypeLeafWire>(node)?.validate_shape()?;
                }
            },
            Check::Collection(node) => {
                let fields = Fields::of(node, &["collection", "element", "bound"])?;
                let _: KindWire = fixed(fields.require("collection")?)?;
                if let Some(bound) = fields.get("bound") {
                    fixed::<BoundWire>(bound)?.read()?;
                }
                work.push(Check::Type(fields.require("element")?));
            }
        }
    }
    Ok(())
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
                    // FR-359 item 5: admitted by conformance under the
                    // package's environment, so a child of the payload's
                    // record keeps its own declaration.
                    Some(payload) => {
                        if !admitted(keys.types(), &payload_type, &payload, "payload")? {
                            return Err(CauseCodecError::Value("payload"));
                        }
                        OptionValue::from_admitted(payload_type, Some(payload))
                    }
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
                // Each element is admitted by conformance (FR-359 item 5);
                // forming checks the bound, and a collapsed duplicate means
                // the document listed one twice.
                if !all_fit(elements.iter().map(|element| {
                    admitted(keys.types(), collection_type.element(), element, "elements")
                }))? {
                    return Err(CauseCodecError::Value("elements"));
                }
                match form(&collection_type, elements, &mut meter) {
                    Outcome::Completed(Value::Collection(formed))
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
