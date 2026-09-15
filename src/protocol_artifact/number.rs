// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-038: exact tagged decimal strings, with no wire normalization or rounding.

use std::fmt;

use serde::{de, ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};

/// Selected numeric wire profile; independent of artifact and model admission.
pub const NUMERIC_PROFILE: &str = "quire.protocol.numeric/1";

/// Component whose decimal spelling or signed-64 domain was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumberComponent {
    /// The integer's decimal value.
    Decimal,
    /// The rational's signed numerator.
    Numerator,
    /// The rational's positive signed-64 denominator.
    Denominator,
}

impl fmt::Display for NumberComponent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Decimal => "decimal",
            Self::Numerator => "numerator",
            Self::Denominator => "denominator",
        })
    }
}

/// Numeric validation failure, classified without interpreting diagnostic text.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum NumberError {
    /// A component is not the unique ASCII decimal spelling of an integer.
    #[error("{component} is not a canonical signed decimal")]
    NonCanonicalDecimal {
        /// The refused component.
        component: NumberComponent,
    },
    /// A canonically spelled integer exceeds the signed-64 component domain.
    #[error("{component} is outside the signed-64 domain")]
    ComponentOutOfRange {
        /// The refused component.
        component: NumberComponent,
    },
    /// Rational denominators must be strictly positive.
    #[error("rational denominator must be positive")]
    NonPositiveDenominator,
    /// The offered components are not coprime; the reader must not reduce them.
    #[error("rational components are not reduced")]
    UnreducedRational,
}

/// An exact integer; fitting this type does not establish a selected model bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ExactInteger(i64);

impl ExactInteger {
    /// Retain a signed-64 value without any conversion to floating point.
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    /// Validate an offered canonical decimal without repairing its spelling.
    ///
    /// # Errors
    /// Returns the typed decimal-spelling or component-domain failure.
    pub fn from_decimal(decimal: &str) -> Result<Self, NumberError> {
        parse_component(decimal, NumberComponent::Decimal).map(Self)
    }

    /// The exact admitted integer.
    pub const fn value(self) -> i64 {
        self.0
    }
}

/// A reduced rational with a signed-64 numerator and positive signed-64 denominator.
///
/// Construction validates already normalized components; it never reduces them.
/// Private fields prevent bypassing that validation:
///
/// ```compile_fail,E0451
/// use quire_spec_language::protocol_artifact::ExactRational;
/// let invalid = ExactRational { numerator: 1, denominator: 0 };
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ExactRational {
    numerator: i64,
    denominator: i64,
}

impl ExactRational {
    /// Admit coprime components with a positive denominator, including only 0/1 for zero.
    ///
    /// # Errors
    /// Refuses nonpositive denominators and unreduced components without repair.
    pub fn new(numerator: i64, denominator: i64) -> Result<Self, NumberError> {
        if denominator <= 0 {
            return Err(NumberError::NonPositiveDenominator);
        }
        // unsigned_abs also represents the magnitude of i64::MIN exactly.
        if gcd(numerator.unsigned_abs(), denominator.unsigned_abs()) != 1 {
            return Err(NumberError::UnreducedRational);
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    /// Validate both offered decimal strings and their unrepaired rational relation.
    ///
    /// # Errors
    /// Returns a typed component, denominator, or reduction failure.
    pub fn from_decimals(numerator: &str, denominator: &str) -> Result<Self, NumberError> {
        Self::new(
            parse_component(numerator, NumberComponent::Numerator)?,
            parse_component(denominator, NumberComponent::Denominator)?,
        )
    }

    /// The exact admitted numerator.
    pub const fn numerator(self) -> i64 {
        self.numerator
    }

    /// The exact positive denominator.
    pub const fn denominator(self) -> i64 {
        self.denominator
    }
}

/// An admitted exact wire number; integer 1 remains distinct from rational 1/1.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ProtocolNumber {
    /// An exact signed-64 integer.
    Integer(ExactInteger),
    /// An exact reduced rational.
    Rational(ExactRational),
}

/// Shape-checked but numerically unvalidated fields from the selected wire profile.
///
/// Decode this type inside the enclosing artifact's bounded reader, then use
/// [`ProtocolNumber::try_from`] to retain a typed [`NumberError`]. A successful
/// shape decode does not admit a number. This type intentionally has no encoder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NumberWire {
    /// Untrusted integer spelling.
    Integer {
        /// Offered decimal text.
        decimal: String,
    },
    /// Untrusted rational spellings.
    Rational {
        /// Offered numerator text.
        numerator: String,
        /// Offered denominator text.
        denominator: String,
    },
}

impl<'de> Deserialize<'de> for NumberWire {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
        enum Fields {
            Integer {
                decimal: String,
            },
            Rational {
                numerator: String,
                denominator: String,
            },
        }
        // A tagged Serde struct must not acquire a positional-array alternative.
        let fields: Fields = crate::serde_object::from_object(deserializer)?;
        Ok(match fields {
            Fields::Integer { decimal } => Self::Integer { decimal },
            Fields::Rational {
                numerator,
                denominator,
            } => Self::Rational {
                numerator,
                denominator,
            },
        })
    }
}

impl TryFrom<&NumberWire> for ProtocolNumber {
    type Error = NumberError;

    fn try_from(wire: &NumberWire) -> Result<Self, Self::Error> {
        match wire {
            NumberWire::Integer { decimal } => {
                ExactInteger::from_decimal(decimal).map(Self::Integer)
            }
            NumberWire::Rational {
                numerator,
                denominator,
            } => ExactRational::from_decimals(numerator, denominator).map(Self::Rational),
        }
    }
}

impl TryFrom<NumberWire> for ProtocolNumber {
    type Error = NumberError;

    fn try_from(wire: NumberWire) -> Result<Self, Self::Error> {
        Self::try_from(&wire)
    }
}

impl Serialize for ProtocolNumber {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Integer(value) => {
                let mut object = serializer.serialize_struct("ProtocolNumber", 2)?;
                object.serialize_field("kind", "integer")?;
                object.serialize_field("decimal", &value.value().to_string())?;
                object.end()
            }
            Self::Rational(value) => {
                let mut object = serializer.serialize_struct("ProtocolNumber", 3)?;
                object.serialize_field("kind", "rational")?;
                object.serialize_field("numerator", &value.numerator().to_string())?;
                object.serialize_field("denominator", &value.denominator().to_string())?;
                object.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for ProtocolNumber {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::try_from(NumberWire::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

impl Serialize for ExactInteger {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ProtocolNumber::Integer(*self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ExactInteger {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match ProtocolNumber::deserialize(deserializer)? {
            ProtocolNumber::Integer(value) => Ok(value),
            ProtocolNumber::Rational(_) => Err(de::Error::invalid_value(
                de::Unexpected::Str("rational"),
                &"an integer kind",
            )),
        }
    }
}

impl Serialize for ExactRational {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ProtocolNumber::Rational(*self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ExactRational {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match ProtocolNumber::deserialize(deserializer)? {
            ProtocolNumber::Rational(value) => Ok(value),
            ProtocolNumber::Integer(_) => Err(de::Error::invalid_value(
                de::Unexpected::Str("integer"),
                &"a rational kind",
            )),
        }
    }
}

fn parse_component(text: &str, component: NumberComponent) -> Result<i64, NumberError> {
    let digits = text.strip_prefix('-').unwrap_or(text).as_bytes();
    let canonical = text == "0"
        || (matches!(digits.first(), Some(b'1'..=b'9')) && digits.iter().all(u8::is_ascii_digit));
    if !canonical {
        return Err(NumberError::NonCanonicalDecimal { component });
    }
    text.parse()
        .map_err(|_| NumberError::ComponentOutOfRange { component })
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        (left, right) = (right, left % right);
    }
    left
}
